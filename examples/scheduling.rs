use adaptive_algorithms::rayon::*;
use adaptive_algorithms::scheduling::*;
use adaptive_algorithms::task::*;

fn main() {
    let n = 30;
    let times: Vec<u64> = std::iter::repeat_with(|| rand::random::<u64>() % 10_000)
        .take(n)
        .collect();
    // two process scheduling

    let procs: Vec<u64> = std::iter::repeat(0).take(2).collect();

    let pool = get_adaptive_thread_pool();

    let mut s = Scheduling::new(&times, &procs);

    #[cfg(feature = "logs")]
    {
        let (_, log) = pool.logging_install(|| s.run());
        log.save("log.json").unwrap();
        log.save_svg("log.svg").unwrap();
    }

    #[cfg(not(feature = "logs"))]
    {
        pool.install(|| s.run());
    }
}
