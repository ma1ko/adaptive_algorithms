extern crate rand;
use std::time::Instant;

// use rayon::prelude::*;
pub mod adaptive_bench;
pub mod points;
pub mod rayon;
pub mod scheduling;
pub mod steal;
pub mod task;
#[macro_use]
extern crate lazy_static;

use crate::adaptive_bench::Benchable;
use crate::scheduling::*;
use crate::task::Task;
pub fn main() {
    // let remaining_times = vec![
    //     1, 2, 5, 20, 09, 20, 42, 13, 4, 20, 64, 6, 84, 20, 01, 91, 100, 5, 42, 25, 65, 39, 62, 35, 60, 25, 29, 53
    // ];

    // let mut x = Scheduling {
    //     remaining_times: remaining_times.clone(),
    //     best: std::u64::MAX,
    //     procs: vec![0, 0],
    // };
    // x.start();
    // println!("{}", x.best);

    // let mut b = BruteForce::new(remaining_times.clone());

    // b.start();
    // println!("{}", b.get_result());
}
