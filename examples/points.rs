use adaptive_algorithms::points::*;
use adaptive_algorithms::rayon::*;

fn main() {
    let points = Point::create_random_points(10000);
    // let pool = get_custom_thread_pool(3,6);
    let pool = get_thread_pool();


    for _ in 0..100 {
    let mut s = Searcher::new(&points);
        #[cfg(feature = "logs")]
        {
            let (_, log) = pool.logging_install(|| s.run());
            log.save_svg("log.svg").unwrap();
        }

        #[cfg(not(feature = "logs"))]
        {
            pool.install(|| s.run());
        }
        // #[cfg(feature = "statistics")]
        // adaptive_algorithms::task::print_statistics();
    }
}
