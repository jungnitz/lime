/// Measure time of `func` and print it if `do_print_timings` is set
#[macro_export]
macro_rules! measure_time {
    ($func:expr, $label:expr, $do_print_timings:expr) => {{
        let start_time = Instant::now();
        let result = $func;
        let t_runtime = start_time.elapsed().as_secs_f64();

        if $do_print_timings {
            println!("{}: {:.6}sec", $label, t_runtime);
        }
        result
    }};
}
