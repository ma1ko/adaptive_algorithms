
use adaptive_algorithms::points::*;
use adaptive_algorithms::rayon::*;

use adaptive_algorithms::adaptive_bench::*;
use rayon_logs::prelude::*;
use rayon_logs::Logged;
use rayon::prelude::*;
fn main() {
    let points = Point::create_random_points(50000);
    let pool = rayon_logs::ThreadPoolBuilder::new().num_threads(4).build().unwrap();

    // let mut s = RayonPoints::new(&points);
    

    // #[cfg(feature = "logs")]
    {
        let (_, log) = pool.logging_install(|| {
           let iter = Logged::new(points.par_iter())
            // .enumerate()
            .map(|( a)| {
                let inner_iter = points[0 + 1..].iter().map(|b| a.distance_to(b));
                inner_iter.fold(1.0f64, |x, y| x.min(y))
            })
            .collect::<Vec<f64>>();
        Some(iter.iter().fold(1.0f64, |x, y| x.min(*y)))

        });
        log.save_svg("log.svg").unwrap();
    }

    // #[cfg(not(feature = "logs"))]
    // {
    //     pool.install(|| s.run());
    // }
    // #[cfg(feature = "statistics")]
    // adaptive_algorithms::task::print_statistics();
}
