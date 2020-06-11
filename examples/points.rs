use adaptive_algorithms::points::*;
use adaptive_algorithms::rayon::*;

fn main() {
    let points = Point::create_random_points(50000);
    let pool = get_adaptive_thread_pool();

    let mut s = Searcher::new(&points);

    #[cfg(feature = "logs")]
    {
        let (_, log) = pool.logging_install(|| s.run_());
        log.save("log.json").unwrap();
        log.save_svg("log.svg").unwrap();
    }

    #[cfg(not(feature = "logs"))]
    {
        pool.install(|| s.run_());
    }
}
