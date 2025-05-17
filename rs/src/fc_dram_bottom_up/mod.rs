use log::{info, warn, error, debug, trace};

// NEXT TODO: make `compiling_receiver` work
// fn compiling_receiver<'a, A: FCDRAMArchitecture>(
//     architecture: &'a A,
//     rules: &'a [Rewrite<MigLanguage, ()>],
//     settings: CompilerSettings,
// ) -> impl Receiver<Result = CompilingReceiverResult<'a, A>, Node = Mig> + 'a {
//     EGraph::<MigLanguage, _>::new(()).map(move |(graph, outputs)| {
//         let t_runner = std::time::Instant::now();
//         let runner = Runner::default().with_egraph(graph).run(rules);
//         let t_runner = t_runner.elapsed().as_millis();
//         if settings.verbose {
//             println!("== Runner Report");
//             runner.print_report();
//         }
//         let graph = runner.egraph;
//
//         let mut t_extractor = 0;
//         let mut t_compiler = 0;
//
//         let output = CompilerOutput::new(
//             graph,
//             |graph| {
//                 let start_time = Instant::now();
//                 let extractor = Extractor::new(
//                     &graph,
//                     CompilingCostFunction {
//                         architecture: architecture,
//                     },
//                 );
//                 t_extractor = start_time.elapsed().as_millis();
//                 (extractor, outputs)
//             },
//             |ntk| {
//                 let start_time = Instant::now();
//                 let program = compile(architecture, &ntk.with_backward_edges());
//                 t_compiler = start_time.elapsed().as_millis();
//                 if settings.print_program || settings.verbose {
//                     if settings.verbose {
//                         println!("== Program")
//                     }
//                     println!("{program}");
//                 }
//                 program
//             },
//         );
//         if settings.verbose {
//             println!("== Timings");
//             println!("t_runner: {t_runner}ms");
//             println!("t_extractor: {t_extractor}ms");
//             println!("t_compiler: {t_compiler}ms");
//         }
//         CompilingReceiverResult {
//             output,
//             t_runner,
//             t_extractor,
//             t_compiler,
//         }
//     })
// }

// #[no_mangle]
// extern "C" fn fcdram_rewriter(settings: CompilerSettings) -> MigReceiverFFI<RewriterFFI<Mig>> {
//     info!("Called fcdram_rewriter");
//     RewriterFFI::new(FCDramRewriter(settings))
// }

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

/// Main functions called by `main.cpp`
#[no_mangle]
// extern "C" fn fcdram_compile(settings: CompilerSettings) -> MigReceiverFFI<CompilerStatistics> {
extern "C" fn fcdram_compile() {
    info!("Called fcdram_compile");
    todo!()
    // TODO: create example `ARCHITECTURE` implementing `FCDRAMArchitecture`
    // let receiver =
    //     compiling_receiver(&*&ARCHITECTURE, REWRITE_RULES.as_slice(), settings).map(|res| {
    //         let graph = res.output.borrow_graph();
    //         CompilerStatistics {
    //             egraph_classes: graph.number_of_classes() as u64,
    //             egraph_nodes: graph.total_number_of_nodes() as u64,
    //             egraph_size: graph.total_size() as u64,
    //             instruction_count: res.output.borrow_program().instructions.len() as u64,
    //             t_runner: res.t_runner as u64,
    //             t_extractor: res.t_extractor as u64,
    //             t_compiler: res.t_compiler as u64,
    //         }
    //     });
    // MigReceiverFFI::new(receiver)
}
