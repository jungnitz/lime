mod compilation;
mod extraction;
mod optimization;
mod program;
mod rows;

use std::sync::LazyLock;
use std::time::Instant;

use self::compilation::compile;
use self::extraction::CompilingCostFunction;

use crate::opt_extractor::{OptExtractionNetwork, OptExtractor};
use eggmock::egg::{rewrite, EGraph, Rewrite, Runner};
use eggmock::{Mig, MigLanguage, MigReceiverFFI, Network, Receiver, ReceiverFFI};
use program::*;
use rows::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BitwiseOperand {
    T(u8),
    DCC { inverted: bool, index: u8 },
}

#[derive(Clone, Debug)]
pub struct Architecture {
    maj_ops: Vec<usize>,
    multi_activations: Vec<Vec<BitwiseOperand>>,
    num_dcc: u8,
}

impl Architecture {
    pub fn new(multi_activations: Vec<Vec<BitwiseOperand>>, num_dcc: u8) -> Self {
        let maj_ops = multi_activations
            .iter()
            .enumerate()
            .filter(|(_, ops)| ops.len() == 3)
            .map(|(i, _)| i)
            .collect();
        Self {
            maj_ops,
            multi_activations,
            num_dcc,
        }
    }
}

static ARCHITECTURE: LazyLock<Architecture> = LazyLock::new(|| {
    use BitwiseOperand::*;
    Architecture::new(
        vec![
            // 2 rows
            vec![
                DCC {
                    index: 0,
                    inverted: true,
                },
                T(0),
            ],
            vec![
                DCC {
                    inverted: true,
                    index: 1,
                },
                T(1),
            ],
            vec![T(2), T(3)],
            vec![T(0), T(3)],
            // 3 rows
            vec![T(0), T(1), T(2)],
            vec![T(1), T(2), T(3)],
            vec![
                DCC {
                    index: 1,
                    inverted: false,
                },
                T(1),
                T(2),
            ],
            vec![
                DCC {
                    index: 1,
                    inverted: false,
                },
                T(0),
                T(3),
            ],
        ],
        2,
    )
});

static REWRITE_RULES: LazyLock<Vec<Rewrite<MigLanguage, ()>>> = LazyLock::new(|| {
    let mut rules = vec![
        rewrite!("commute_1"; "(maj ?a ?b ?c)" => "(maj ?b ?a ?c)"),
        rewrite!("commute_2"; "(maj ?a ?b ?c)" => "(maj ?a ?c ?b)"),
        rewrite!("not_not"; "(! (! ?a))" => "?a"),
        rewrite!("maj_1"; "(maj ?a ?a ?b)" => "?a"),
        rewrite!("maj_2"; "(maj ?a (! ?a) ?b)" => "?b"),
        rewrite!("associativity"; "(maj ?a ?b (maj ?c ?b ?d))" => "(maj ?d ?b (maj ?c ?b ?a))"),
    ];
    rules.extend(rewrite!("invert"; "(! (maj ?a ?b ?c))" <=> "(maj (! ?a) (! ?b) (! ?c))"));
    rules.extend(rewrite!("distributivity"; "(maj ?a ?b (maj ?c ?d ?e))" <=> "(maj (maj ?a ?b ?c) (maj ?a ?b ?d) ?e)"));
    rules
});

impl BitwiseOperand {
    pub fn row(&self) -> BitwiseRow {
        match self {
            BitwiseOperand::T(t) => BitwiseRow::T(*t),
            BitwiseOperand::DCC { index, .. } => BitwiseRow::DCC(*index),
        }
    }
    pub fn is_dcc(&self) -> bool {
        matches!(self, BitwiseOperand::DCC { .. })
    }
    pub fn inverted(&self) -> bool {
        matches!(self, BitwiseOperand::DCC { inverted: true, .. })
    }
}

struct CompilingReceiverResult<'a> {
    output: CompilerOutput<'a>,

    t_runner: u128,
    t_extractor: u128,
    t_compiler: u128,
}

#[ouroboros::self_referencing]
struct CompilerOutput<'a> {
    graph: EGraph<MigLanguage, ()>,
    #[borrows(graph)]
    #[covariant]
    ntk: OptExtractionNetwork<OptExtractor<'this, CompilingCostFunction<'a>, MigLanguage, ()>>,
    #[borrows(ntk)]
    program: Program<'a>,
}

fn compiling_receiver<'a>(
    architecture: &'a Architecture,
    rules: &'a [Rewrite<MigLanguage, ()>],
    settings: CompilerSettings,
) -> impl Receiver<Result = CompilingReceiverResult<'a>, Node = Mig> + 'a {
    EGraph::<MigLanguage, _>::new(()).map(move |(mut graph, outputs)| {
        let t_runner = if settings.rewrite {
            let t_runner = std::time::Instant::now();
            let runner = Runner::default().with_egraph(graph).run(rules);
            let t_runner = t_runner.elapsed().as_millis();
            if settings.verbose {
                println!("== Runner Report");
                runner.print_report();
            }
            graph = runner.egraph;
            t_runner
        } else {
            0
        };

        let mut t_extractor = 0;
        let mut t_compiler = 0;

        let output = CompilerOutput::new(
            graph,
            |graph| {
                let start_time = Instant::now();
                let extractor = OptExtractor::new(graph, CompilingCostFunction { architecture });
                t_extractor = start_time.elapsed().as_millis();
                OptExtractionNetwork(extractor, outputs)
            },
            |ntk| {
                let start_time = Instant::now();
                let program = compile(architecture, &ntk.with_backward_edges())
                    .expect("network should be compilable");
                t_compiler = start_time.elapsed().as_millis();
                if settings.print_program || settings.verbose {
                    if settings.verbose {
                        println!("== Program")
                    }
                    println!("{program}");
                }
                program
            },
        );
        if settings.verbose {
            println!("== Timings");
            println!("t_runner: {t_runner}ms");
            println!("t_extractor: {t_extractor}ms");
            println!("t_compiler: {t_compiler}ms");
        }
        CompilingReceiverResult {
            output,
            t_runner,
            t_extractor,
            t_compiler,
        }
    })
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct CompilerSettings {
    print_program: bool,
    verbose: bool,
    rewrite: bool,
}

#[repr(C)]
struct CompilerStatistics {
    egraph_classes: u64,
    egraph_nodes: u64,
    egraph_size: u64,

    instruction_count: u64,

    t_runner: u64,
    t_extractor: u64,
    t_compiler: u64,
}

#[no_mangle]
extern "C" fn ambit_rewrite_ffi(
    settings: CompilerSettings,
    receiver: MigReceiverFFI<()>,
) -> MigReceiverFFI<CompilerStatistics> {
    let receiver =
        compiling_receiver(&ARCHITECTURE, REWRITE_RULES.as_slice(), settings).map(|res| {
            res.output.borrow_ntk().send(receiver);
            CompilerStatistics::from_result(res)
        });
    MigReceiverFFI::new(receiver)
}

#[no_mangle]
extern "C" fn ambit_compile_ffi(settings: CompilerSettings) -> MigReceiverFFI<CompilerStatistics> {
    let receiver = compiling_receiver(&ARCHITECTURE, REWRITE_RULES.as_slice(), settings)
        .map(CompilerStatistics::from_result);
    MigReceiverFFI::new(receiver)
}

impl CompilerStatistics {
    fn from_result(res: CompilingReceiverResult) -> Self {
        let graph = res.output.borrow_graph();
        CompilerStatistics {
            egraph_classes: graph.number_of_classes() as u64,
            egraph_nodes: graph.total_number_of_nodes() as u64,
            egraph_size: graph.total_size() as u64,
            instruction_count: res.output.borrow_program().instructions.len() as u64,
            t_runner: res.t_runner as u64,
            t_extractor: res.t_extractor as u64,
            t_compiler: res.t_compiler as u64,
        }
    }
}
