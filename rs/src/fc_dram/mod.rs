//! NOTE: currently FCDRAM has only been shown to work with HK Sync Modules
//!
//! # Literature
//!
//! - [1] Functionally-Complete Boolean Logic in Real DRAM Chips: Experimental Characterization and Analysis, 2024
//! - [2] FracDRAM: Fractional Values in Off-the-Shelf DRAM, 2022
//! - [3] PULSAR: Simultaneous Many-Row Activation for Reliable and High-Performance Computing in Off-the-Shelf DRAM Chips, 2024
//! - [4] RowClone: fast and energy-efficient in-DRAM bulk data copy and initialization, 2013
//! - [5] Design-Induced Latency Variation in Modern DRAM Chips: Characterization, Analysis, and Latency Reduction Mechanisms, 2017
//!     - explains why distance of rows to sense-amps influence success-rate of executed op
//!
//! # Submodules
//!
//! - [`architecture`] - defines Instructions (and performance-metrics of Instructions in that architecture) used in FC-DRAM
//! - [`compiler`] - compiles given LogicNetwork for FC-DRAM architecture
//! - [`generator`] - Generates output code or reports based on analysis. (TODO)
//! - [`optimization`] - applies architecture-specific optimizations to generated program (TODO: don't use here but in MLIR instead)
//! - [ ] [`program`]
//! - [`utils`] - utilities (helper macros/...)
pub mod architecture;
pub mod compiler;
pub mod cost_estimation;
pub mod optimization;
pub mod program;
pub mod utils;

use std::sync::LazyLock;
use std::time::Instant;

use crate::measure_time;

use self::compiler::Compiler;
use self::cost_estimation::CompilingCostFunction;

use eggmock::egg::{rewrite, EGraph, Extractor, Id, Rewrite, Runner};
use eggmock::{
    AigReceiverFFI, Aoig, AoigLanguage, Network, NetworkWithBackwardEdges, Receiver, ReceiverFFI, Signal
};
use log::debug;
use program::*;
use architecture::*;

/// Rewrite rules to use in E-Graph Rewriting (see [egg](https://egraphs-good.github.io/))
/// TODO: adjust rewriting rules to FCDRAM (=AND/OR related rewrites like De-Morgan?)
static REWRITE_RULES: LazyLock<Vec<Rewrite<AoigLanguage, ()>>> = LazyLock::new(|| {
    let rules = vec![
        // TODO: sth is wrong with these rewrite rules - they produce a non-equivalent logic network !
        // rewrite!("commute-and"; "(and ?a ?b)" => "(and ?b ?a)"),
        rewrite!("and-1"; "(and ?a (! f))" => "?a"),
        rewrite!("and-0"; "(and ?a f)" => "f"),
        rewrite!("or-1"; "(or ?a (! f))" => "(! f)"),
        rewrite!("or-0"; "(or ?a f)" => "?a"),

        rewrite!("and-or"; "(! (or (! ?a) (! ?b)))" => "(and ?a ?b)"), // (De-Morgan)
        rewrite!("or-and"; "(! (and (! ?a) (! ?b)))" => "(or ?a ?b)" ), // (De-Morgan)
        // rewrite!("and-or-more-not"; "(and ?a ?b)" => "(! (or (! ?a) (! ?b)))"), // (De-Morgan)
        // rewrite!("or-and-more-not"; "(or ?a ?b)" => "(! (and (! ?a) (! ?b)))"), // (De-Morgan)
        rewrite!("and-same"; "(and ?a ?a)" => "?a"),
        rewrite!("not_not"; "(! (! ?a))" => "?a"),

        // in general more operands are better for AND/OR (see [1])
        rewrite!("and2_to_4"; "(and (and ?a ?b) (and ?c ?d))" => "(and4 ?a ?b ?c ?d)"),
        rewrite!("and4_to_8"; "(and (and4 ?a ?b ?c ?d) (and4 ?e ?f ?g ?h))" => "(and8 ?a ?b ?c ?d ?e ?f ?g ?h)"),
        rewrite!("and8_to_16"; "(and (and8 ?a ?b ?c ?d ?e ?f ?g ?h) (and8 ?i ?j ?k ?l ?m ?n ?o ?p))" => "(and16 ?a ?b ?c ?d ?e ?f ?g ?h ?i ?j ?k ?l ?m ?n ?o ?p)"),
        rewrite!("or2_to_4"; "(or (or ?a ?b) (or ?c ?d))" => "(or4 ?a ?b ?c ?d)"),
        rewrite!("or4_to_8"; "(or (or4 ?a ?b ?c ?d) (or4 ?e ?f ?g ?h))" => "(or8 ?a ?b ?c ?d ?e ?f ?g ?h)"),
        rewrite!("or8_to_16"; "(or (or8 ?a ?b ?c ?d ?e ?f ?g ?h) (or8 ?i ?j ?k ?l ?m ?n ?o ?p))" => "(or16 ?a ?b ?c ?d ?e ?f ?g ?h ?i ?j ?k ?l ?m ?n ?o ?p)"),
        // no use for NOT with multiple dsts (for now)
    ];
    rules
});

/// Compilation result (program + E-Graph)
#[ouroboros::self_referencing]
struct CompilerOutput {
    /// Result E-Graph
    graph: EGraph<AoigLanguage, ()>,
    /// (, output-nodes)
    #[borrows(graph)]
    #[covariant]
    /// A network consists of nodes (accessed via `Extractor` and separately stored `outputs` (`Vec<Id>`)
    ntk: (
        Extractor<'this, CompilingCostFunction, AoigLanguage, ()>, // `'this`=self-reference, used to extract best-node from `E-Class` of `AoigLanguage`-nodes based on `CompilingCostFunction`
        Vec<Id>, // vector of outputs
    ),
    /// Compiled Program Program is compiled using previously (EGraph-)extracted `ntk`
    #[borrows(ntk)]
    program: Program,
}

/// Initiates compilation and prints compilation-statistics (and program if `settings.verbose=true`
/// - returned receiver allows converting result-graph in both directions (C++ <=> Rust)
/// - `settings`: compiler-options
fn compiling_receiver<'a>(
    rules: &'a [Rewrite<AoigLanguage, ()>],
    settings: CompilerSettings,
) -> impl Receiver<Result = CompilerOutput, Node = Aoig> + use<'a> {
    // REMINDER: EGraph implements `Receiver`
    let mut compiler = Compiler::new(settings.clone());
    EGraph::<AoigLanguage, _>::new(())
        .map(move |(graph, outputs)| { // `.map()` of `Provider`-trait!, outputs=vector of EClasses

            debug!("Input EGraph nodes: {:?}", graph.nodes());
            debug!("Input EGraph's EClasses : {:?}", graph.classes()
                .map(|eclass| (eclass.id, &eclass.nodes) )
                .collect::<Vec<(Id,&Vec<AoigLanguage>)>>()
            );
            // 1. Create E-Graph: run equivalence saturation
            debug!("Running equivalence saturation...");
            let runner = measure_time!(
                Runner::default().with_egraph(graph).run(rules),  "t_runner", settings.print_compilation_stats
            );

            if settings.verbose {
                println!("== Runner Report");
                runner.print_report();
            }

            let graph = runner.egraph;

            CompilerOutput::new(
                graph,
                |graph| {
            // 2. Given E-Graph: Retrieve best graph using custom `CompilingCostFunction`
                    debug!("Extracting...");
                    let extractor = measure_time!(
                        Extractor::new(
                            graph,
                            CompilingCostFunction {},
                        ),
                        "t_extractor", settings.print_compilation_stats
                    );
                    debug!("Outputs: {outputs:?}");
                    (extractor, outputs) // produce `ntk`
                },
                |ntk| {
                    // ===== MAIN CALL (actual compilation) =====
            // 3. Compile program using extracted network

                    debug!("Compiling...");
                    debug!("Network outputs: {:?}", ntk.outputs().collect::<Vec<Signal>>());
                    ntk.dump();
                    let ntk_with_backward_edges = ntk.with_backward_edges();
                    debug!("Network Leaves: {:?}", ntk_with_backward_edges.leafs().collect::<Vec<eggmock::Id>>());
                    debug!("Network Outputs of first leaf: {:?}",
                        ntk_with_backward_edges.node_outputs(
                            ntk_with_backward_edges.leafs().next().unwrap()
                        ).collect::<Vec<eggmock::Id>>()
                    );

                    let program = measure_time!(
                        compiler.compile(&ntk_with_backward_edges), "t_compiler", settings.print_compilation_stats
                    );
                    // =====================

                    // print program if compiler-setting is set
                    // TOOD: write program to output-file instead !!
                    if settings.print_program || settings.verbose {
                        if settings.verbose {
                            println!("== Program")
                        }
                        println!("{program}");
                    }
                    program
                },
            )
    })
}

#[derive(Debug, Clone)]
#[repr(C)]
/// Compiler options
/// - TODO: add flags like minimal success-rate for program
pub struct CompilerSettings {
    /// Whether to print the compiled program
    print_program: bool,
    /// Whether to enable verbose output
    verbose: bool,
    /// Whether to print stats like runtimes of individual compiler-stages during compilation
    print_compilation_stats: bool,
    /// Minimal success rate to be guaranteed for success compiled program
    /// REMINDER: FCDRAM-operations dont have a 100%-success rate to create the correct results
    /// TODO: not used yet by compiler
    min_success_rate: f64,
    // /// Location to config-file holding fcdram-specific configs
    // fcdram_config_file: Path,

    /// How many times to issue FracOps to store `V_{DD}/2` in one of the activated rows for AND/OR
    repetition_fracops: u64,
    /// Nr of rows to use as a safe space for operands per subarray
    /// - REMINDER: after `AND`/`OR`-ops the src-operands are overwritten by the op-result, so to reuse operands they're put into specially designated rows (="safe-space") which won't be overwritten
    /// - Ops reusing those operands have to clone the values from the safe-space prior to issuing the Op
    /// - NOTE: rows which are used as safe-space are determined by analyzing patterns in Simultaneous-row activation for the specific architecture (to ensure that safe-space rows won't be activated on any combination of row-addresses)
    ///
    /// TODO: if `config_file` is passed, make sure nr safe-space-rows is equal to nr of rows detailed in config-file
    ///
    /// DEPRECATED: current implementation select compute rows instead
    safe_space_rows_per_subarray: u8,
    /// Location of config-file (to which to write the compiled configs) - if this config file doesn't exist then a new one is generated under this given path
    config_file: *const i8,
    /// Whether to save the configuration file (for used safe-space rows, placement of constant 0s&1s, ..)
    do_save_config: bool,
}

// TODO: this will be needed once E-Graph Validation is added (=once we want to transfer the E-Graph back to mockturtle)
// /// ?? (maybe FFI for rewriting graph using mockturtle?)
// #[no_mangle]
// extern "C" fn fcdram_rewriter(settings: CompilerSettings) -> AigReceiverFFI<RewriterFFI<Aig>> {
//     RewriterFFI::new(FCDramRewriter(settings))
// }

/// Statistic results about Compilation-Process
#[repr(C)]
struct CompilerStatistics {
    egraph_classes: u64,
    egraph_nodes: u64,
    egraph_size: u64,

    instruction_count: u64,
}

/// Entry point for cpp-code
/// - `settings`: settings to use when running compiler
#[no_mangle]
extern "C" fn fcdram_compile(settings: CompilerSettings) -> AigReceiverFFI<CompilerStatistics> {

    env_logger::init(); // needed for `export RUST_LOG=debug` to work
    let receiver =
        compiling_receiver(REWRITE_RULES.as_slice(), settings)
            .map(|output| {
                let graph = output.borrow_graph();
                CompilerStatistics {
                    egraph_classes: graph.number_of_classes() as u64,
                    egraph_nodes: graph.total_number_of_nodes() as u64,
                    egraph_size: graph.total_size() as u64,
                    instruction_count: output.borrow_program().instructions.len() as u64,
            }
        });
    // return graph back to calling cpp-code
    AigReceiverFFI::new(receiver.adapt(Into::into))
}
