//! NOTE: currently FCDRAM has only been shown to work with HK Sync Modules
//!
//! # Literature
//!
//! - [1] Functionally-Complete Boolean Logic in Real DRAM Chips: Experimental Characterization and Analysis, 2024
//! - [2] FracDRAM: Fractional Values in Off-the-Shelf DRAM, 2022
//! - [3] PULSAR: Simultaneous Many-Row Activation for Reliable and High-Performance Computing in Off-the-Shelf DRAM Chips, 2024
//!
//! # Submodules
//!
//! - [`architecture`] - defines Instructions (and performance-metrics of Instructions in that architecture) used in FC-DRAM
//! - [`compiler`] - compiles given LogicNetwork for FC-DRAM architecture
//! - [`generator`] - Generates output code or reports based on analysis. (TODO)
//! - [`optimization`] - applies architecture-specific optimizations to generated program (TODO: don't use here but in MLIR instead)
//! - [ ] [`program`]
//! - [`utils`] - utilities (helper macros/...)
mod architecture;
mod compiler;
mod egraph_extraction;
mod optimization;
mod program;
mod utils;

use std::sync::LazyLock;
use std::time::Instant;

use crate::measure_time;

use self::compiler::compile;
use self::egraph_extraction::CompilingCostFunction;

use eggmock::egg::{rewrite, EGraph, Extractor, Id, Rewrite, Runner};
use eggmock::{
    Aig, AigLanguage, AigReceiverFFI, Network, Receiver, ReceiverFFI, Rewriter,
    RewriterFFI, // TODO: add AOIG-rewrite (bc FC-DRAM supports AND&OR natively)?
};
use program::*;
use architecture::*;

/// Rewrite rules to use in E-Graph Rewriting (see [egg](https://egraphs-good.github.io/))
/// TODO: adjust rewriting rules to FCDRAM (=AND/OR related rewrites like De-Morgan?)
static REWRITE_RULES: LazyLock<Vec<Rewrite<AigLanguage, ()>>> = LazyLock::new(|| {
    let mut rules = vec![
        // TODO: add "or" - and De-Morgan ?
        rewrite!("commute-and"; "(and ?a ?b)" => "(and ?b ?a)"),
        rewrite!("and-1"; "(and ?a 1)" => "?a"),
        rewrite!("and-0"; "(and ?a 0)" => "0"),
        rewrite!("and-same"; "(and ?a ?a)" => "?a"),
        rewrite!("not_not"; "(! (! ?a))" => "?a"),
        // rewrite!("maj_1"; "(maj ?a ?a ?b)" => "?a"),
        // rewrite!("maj_2"; "(maj ?a (! ?a) ?b)" => "?b"),
        // rewrite!("associativity"; "(maj ?a ?b (maj ?c ?b ?d))" => "(maj ?d ?b (maj ?c ?b ?a))"),
    ];
    // rules.extend(rewrite!("invert"; "(! (maj ?a ?b ?c))" <=> "(maj (! ?a) (! ?b) (! ?c))"));
    // rules.extend(rewrite!("distributivity"; "(maj ?a ?b (maj ?c ?d ?e))" <=> "(maj (maj ?a ?b ?c) (maj ?a ?b ?d) ?e)"));
    rules
});

/// Compilation result (program + E-Graph)
#[ouroboros::self_referencing]
struct CompilerOutput {
    /// Result E-Graph
    graph: EGraph<AigLanguage, ()>,
    /// (, output-nodes)
    #[borrows(graph)]
    #[covariant]
    ntk: (
        Extractor<'this, CompilingCostFunction, AigLanguage, ()>, // `'this`=self-reference
        Vec<Id>,
    ),
    /// Compiled Program
    /// Program is compiled using previously (EGraph-)extracted `ntk`
    #[borrows(ntk)]
    program: Program,
}

/// Initiates compilation and prints compilation-statistics (and program if `settings.verbose=true`
/// - returned receiver allows converting result-graph in both directions (C++ <=> Rust)
/// - `settings`: compiler-options
fn compiling_receiver<'a>(
    rules: &'a [Rewrite<AigLanguage, ()>],
    settings: CompilerSettings,
) -> impl Receiver<Result = CompilerOutput, Node = Aig> + use<'a> {
    // REMINDER: EGraph implements `Receiver`
    EGraph::<AigLanguage, _>::new(())
        .map(move |(graph, outputs)| { // `.map()` of `Provider`-trait:

            // 1. Create E-Graph: run equivalence saturation
            let runner = measure_time!(Runner::default().with_egraph(graph).run(rules),  "t_runner", settings.print_compilation_stats );


            if settings.verbose {
                println!("== Runner Report");
                runner.print_report();
            }

            let graph = runner.egraph;


            CompilerOutput::new(
                graph,
                |graph| {
            // 2. Given E-Graph: Retrieve best graph using custom `CompilingCostFunction`
                    let extractor = measure_time!(
                        Extractor::new(
                            graph,
                            CompilingCostFunction {},
                        ), // TODO: provide CostFunction !!
                        "t_extractor", settings.print_compilation_stats
                    );
                    (extractor, outputs) // produce `ntk`
                },
                |ntk| {
                    // ===== MAIN CALL (actual compilation) =====
            // 3. Compile program using extracted network
                    let program = measure_time!(
                        compile(&ntk.with_backward_edges()), "t_compiler", settings.print_compilation_stats
                    );
                    // =====================
                                                                                                                                                // print program if compiler-setting is set
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
/// Compiler options
/// - TODO: add flags like minimal success-rate for program
struct CompilerSettings {
    /// Whether to print the compiled program
    print_program: bool,
    /// Whether to enable verbose output
    verbose: bool,
    /// Whether to print stats like runtimes of individual compiler-stages during compilation
    print_compilation_stats: bool,
    /// Minimal success rate to be guaranteed for success compiled program
    /// REMINDER: FCDRAM-operations dont have a 100%-success rate to create the correct results
    min_success_rate: u64,
    // /// Location to config-file holding fcdram-specific configs
    // fcdram_config_file: Path,
}

struct FCDramRewriter(CompilerSettings);

impl Rewriter for FCDramRewriter {
    type Node = Aig;
    type Intermediate = CompilerOutput;

    fn create_receiver(
        &mut self,
    ) -> impl Receiver<Node = Aig, Result = CompilerOutput> + 'static {
        compiling_receiver(REWRITE_RULES.as_slice(), self.0)
    }

    fn rewrite(
        self,
        result: CompilerOutput,
        output: impl Receiver<Node = Aig, Result = ()>,
    ) {
        result.borrow_ntk().send(output);
    }
}

/// ?? (maybe FFI for rewriting graph using mockturtle?)
#[no_mangle]
extern "C" fn fcdram_rewriter(settings: CompilerSettings) -> AigReceiverFFI<RewriterFFI<Aig>> {
    RewriterFFI::new(FCDramRewriter(settings))
}

/// Statistic results about Compilation-Process
#[repr(C)]
struct CompilerStatistics {
    egraph_classes: u64,
    egraph_nodes: u64,
    egraph_size: u64,

    instruction_count: u64,
}

/// Main function called from `.cpp()` file - receives compiler settings
/// - `settings`: settings to use when running compiler
#[no_mangle]
extern "C" fn fcdram_compile(settings: CompilerSettings) -> AigReceiverFFI<CompilerStatistics> {
    // todo!()
    // TODO: create example `ARCHITECTURE` implementing `FCDRAMArchitecture`
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
    AigReceiverFFI::new(receiver)
}
