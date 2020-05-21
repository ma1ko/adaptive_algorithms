//! you need to compile me with a nightly compiler to enable the atomic_min_max feature.
//! we try to parallelize a brute force and branch and bound algorithm for
//! the independant tasks scheduling problem (P||Cmax).
// #![feature(integer_atomics)]
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

const P: usize = 2; // the number of processors we simulate

use crate::task::Task;
use crate::task::NOTHING;

pub struct Scheduling {
    pub remaining_times: Vec<u64>,
    pub best: u64,
    pub procs: Vec<u64>,
    pub splits: Vec<u64>,
}
impl Scheduling {
    pub fn new(remaining_times: Vec<u64>, procs: Vec<u64>) -> Self {
        Scheduling {
            remaining_times,
            best: std::u64::MAX,
            procs,
            splits: Vec::new(),
        }
    }
}
impl Task for Scheduling {
    fn step(&mut self) {
        if self.remaining_times.len() == 10 {
            self.check(NOTHING);
        }
        if let Some(time) = self.remaining_times.pop() {
            (0..P)
                .into_iter()
                .map(|i| {
                    self.procs[i] += time;
                    self.step();
                    self.procs[i] -= time;
                })
                .min()
                .unwrap();
            self.remaining_times.push(time);
        } else {
            self.best = self.best.min(*self.procs.iter().max().unwrap());
        }
    }
    fn can_split(&self) -> bool {
        true
    }
    fn split(&mut self) -> Self {
        assert_eq!(self.procs.len(), 2);
        let time = self.remaining_times.pop().unwrap();
        let mut other = Scheduling {
            remaining_times: self.remaining_times.clone(),
            best: self.best,
            procs: self.procs.clone(),
            splits: Vec::new(),
        };
        self.procs[0] += time;
        other.procs[1] += time;
        self.splits.push(time);
        return other;
    }
    fn fuse(&mut self, other: Self) {
        let time = self.splits.pop().unwrap();
        self.procs[0] -= time;
        self.remaining_times.push(time);
        self.best = other.best.min(self.best);
    }
    fn is_finished(&self) -> bool {
        self.best != std::u64::MAX
    }
}

use crate::adaptive_bench::Benchable;
impl<'a> Benchable<'a, u64> for Scheduling {
    fn start(&mut self) -> () {
        self.run_()
    }
    fn name(&self) -> &'static str {
        "Adaptive_Scheduling"
    }
    fn get_result(&self) -> u64 {
        self.best
    }
    fn reset(&mut self) {
        self.best = std::u64::MAX;
    }
}

pub struct BruteForcePar {
    times: Vec<u64>,
    result: u64,
}
impl BruteForcePar {
    pub fn new(times: Vec<u64>) -> Self {
        BruteForcePar {
            times,
            result: std::u64::MAX,
        }
    }
}
pub struct BruteForce {
    times: Vec<u64>,
    result: u64,
}
impl BruteForce {
    pub fn new(times: Vec<u64>) -> Self {
        BruteForce {
            times,
            result: std::u64::MAX,
        }
    }
}
impl<'a> Benchable<'a, u64> for BruteForce {
    fn name(&self) -> &'static str {
        "Brute Force Single"
    }
    fn get_result(&self) -> u64 {
        self.result
    }
    fn start(&mut self) -> () {
        self.result = brute_force(&self.times)
    }
    fn reset(&mut self) {
        self.result = std::u64::MAX;
    }
}
impl<'a> Benchable<'a, u64> for BruteForcePar {
    fn name(&self) -> &'static str {
        "Brute Force Parallel"
    }
    fn get_result(&self) -> u64 {
        self.result
    }
    fn start(&mut self) -> () {
        self.result = brute_force_par(&self.times)
    }
    fn reset(&mut self) {
        self.result = std::u64::MAX;
    }
}

fn greedy_scheduling(times: &[u64]) -> u64 {
    let procs: Vec<u64> = std::iter::repeat(0).take(P).collect(); // processors state (load)
    let procs = times.iter().fold(procs, |mut procs, time| {
        let min_index = (0..P).min_by_key(|&i| procs[i]).unwrap();
        procs[min_index] += time;
        procs
    });
    procs.iter().max().cloned().unwrap()
}

fn brute_force_rec(procs: &mut Vec<u64>, times: &[u64]) -> u64 {
    times
        .split_first()
        .map(|(time, remaining_times)| {
            (0..P)
                .map(|i| {
                    procs[i] += time;
                    let r = brute_force_rec(procs, remaining_times);
                    procs[i] -= time;
                    r
                })
                .min()
                .unwrap()
        })
        .unwrap_or_else(|| procs.iter().max().cloned().unwrap())
}

fn brute_force(times: &[u64]) -> u64 {
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    brute_force_rec(&mut procs, times)
}

fn brute_force_par(times: &[u64]) -> u64 {
    // START_REPLACING
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    brute_force_rec_par(&mut procs, times, 3)
    // END_COMMENTING
}

// START_COMMENTING
fn brute_force_rec_par(procs: &mut Vec<u64>, times: &[u64], levels: usize) -> u64 {
    if levels == 0 {
        return brute_force_rec(procs, times);
    }
    times
        .split_first()
        .map(|(time, remaining_times)| {
            (0..P)
                .into_par_iter()
                .map_init(
                    || procs.clone(),
                    |mut procs, i| {
                        procs[i] += time;
                        let r = brute_force_rec_par(&mut procs, remaining_times, levels - 1);
                        procs[i] -= time;
                        r
                    },
                )
                .min()
                .unwrap()
        })
        .unwrap_or_else(|| procs.iter().max().cloned().unwrap())
}
// END_COMMENTING

fn branch_and_bound(times: &[u64], initial_solution: u64) -> u64 {
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    branch_and_bound_rec(&mut procs, times, initial_solution)
}

fn branch_and_bound_rec(procs: &mut Vec<u64>, times: &[u64], mut best_solution: u64) -> u64 {
    if procs.iter().max().cloned().unwrap() >= best_solution {
        best_solution
    } else {
        times
            .split_first()
            .map(|(time, remaining_times)| {
                for i in 0..P {
                    procs[i] += time;
                    let r = branch_and_bound_rec(procs, remaining_times, best_solution);
                    if r < best_solution {
                        best_solution = r
                    }
                    procs[i] -= time;
                }
                best_solution
            })
            .unwrap_or_else(|| procs.iter().max().cloned().unwrap())
    }
}

fn branch_and_bound_par(times: &[u64], initial_solution: u64) -> u64 {
    // START_REPLACING
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    let best_value = AtomicU64::new(initial_solution);
    branch_and_bound_rec_par(&mut procs, times, &best_value);
    best_value.load(Ordering::SeqCst)
    // END_COMMENTING
}

// START_COMMENTING
fn branch_and_bound_rec_par(procs: &mut Vec<u64>, times: &[u64], best_solution: &AtomicU64) {
    if procs.iter().max().cloned().unwrap() < best_solution.load(Ordering::SeqCst) {
        times
            .split_first()
            .map(|(time, remaining_times)| {
                (0..P).into_par_iter().for_each_init(
                    || procs.clone(),
                    |procs, i| {
                        procs[i] += time;
                        branch_and_bound_rec_fallback(procs, remaining_times, best_solution);
                        procs[i] -= time;
                    },
                )
            })
            .unwrap_or_else(|| {
                let value = procs.iter().max().cloned().unwrap();
                // best_solution.fetch_min(value, Ordering::SeqCst);
            });
    }
}

fn branch_and_bound_rec_fallback(procs: &mut Vec<u64>, times: &[u64], best_solution: &AtomicU64) {
    if procs.iter().max().cloned().unwrap() < best_solution.load(Ordering::SeqCst) {
        times
            .split_first()
            .map(|(time, remaining_times)| {
                for i in 0..P {
                    procs[i] += time;
                    branch_and_bound_rec_fallback(procs, remaining_times, best_solution);
                    procs[i] -= time;
                }
            })
            .unwrap_or_else(|| {
                let value = procs.iter().max().cloned().unwrap();
                // best_solution.fetch_min(value, Ordering::SeqCst);
            });
    }
}
// END_COMMENTING

fn compute_lower_bound(times: &[u64], times_sum: u64) -> u64 {
    debug_assert_eq!(times.iter().sum::<u64>(), times_sum);
    std::cmp::max(
        times.iter().max().cloned().unwrap_or(0),
        times_sum / P as u64,
    )
}

pub fn main() {
    // let n: usize = std::env::args()
    //     .nth(1)
    //     .and_then(|arg| arg.parse().ok())
    //     .expect("give number of tasks");
    let n = 18;
    let times: Vec<u64> = std::iter::repeat_with(|| rand::random::<u64>() % 10_000)
        //.enumerate()
        //.map(|(i, e)| e / (i as u64 + 1))
        .take(n)
        .collect();

    println!("times are {:?}", times);

    let lower_bound = compute_lower_bound(&times, times.iter().sum());
    println!("lower bound is {}", lower_bound);

    let start = std::time::Instant::now();
    println!(
        "brute force : {} in {}",
        brute_force(&times),
        std::time::Instant::now().duration_since(start).as_millis()
    );

    let start = std::time::Instant::now();
    println!(
        "brute force par : {} in {}",
        brute_force_par(&times),
        std::time::Instant::now().duration_since(start).as_millis()
    );

    let start = std::time::Instant::now();
    println!(
        "b&b : {} in {}",
        branch_and_bound(&times, std::u64::MAX),
        std::time::Instant::now().duration_since(start).as_millis()
    );

    let start = std::time::Instant::now();
    println!(
        "b&b par : {} in {}",
        branch_and_bound_par(&times, std::u64::MAX),
        std::time::Instant::now().duration_since(start).as_millis()
    );
}
