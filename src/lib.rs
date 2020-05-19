extern crate rand;
use std::time::Instant;

// use rayon::prelude::*;
pub mod points;
pub mod adaptive_bench;
pub mod steal;
pub mod task;
pub mod rayon;
use points::Searcher;
#[macro_use]
extern crate lazy_static;


pub fn main() {
    // println!("Running");

    // let points = points::Point::create_random_points(50000);
    // let now = Instant::now();

    // let pool = rayon::ThreadPoolBuilder::new()
    //     .num_threads(4)
    //     .build()
    //     .unwrap();
    // let min = pool.install(|| {
    //     let iter = points
    //         .par_iter()
    //         .enumerate()
    //         .map(|(i, a)| {
    //             let inner_iter = points[i + 1..].iter().map(|b| a.distance_to(b));
    //             inner_iter.fold(1.0f64, |x, y| x.min(y))
    //         })
    //         .collect::<Vec<f64>>();
    //     let min = iter.iter().fold(1.0f64, |x, y| x.min(*y));
    //     min
    // });
    // let new_now = Instant::now();
    // println!("{:?}", new_now.duration_since(now));
    // println!("Closest points have a distance of {}", min);

    // println!("My Algo");

    // let pool = rayon::ThreadPoolBuilder::new()
    //     .num_threads(4)
    //     .steal_callback(|x| mergesort::steal::steal(6, x))
    //     .build()
    //     .unwrap();
    // let mut s = Searcher::new(&points);
    // pool.install(|| s.run_());
    // let now = Instant::now();
    // assert_eq!(min, s.min);
    // println!("{:?}", now.duration_since(new_now));
    // println!("My result: {}", s.min);
}





