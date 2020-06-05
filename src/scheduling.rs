//! you need to compile me with a nightly compiler to enable the atomic_min_max feature.
//! we try to parallelize a brute force and branch and bound algorithm for
//! the independant tasks scheduling problem (P||Cmax).
// #![feature(integer_atomics)]
use crate::steal;
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

const P: usize = 3; // the number of processors we simulate

use crate::task::SimpleTask;
use std::ops::Range;
// use crate::task::NOTHING;

pub struct Scheduling {
    pub remaining_times: Vec<u64>,
    // pub index: usize,
    pub best: u64,
    pub procs: Vec<u64>,
    pub decisions: Vec<Range<usize>>,
}
impl Scheduling {
    pub fn new(remaining_times: &Vec<u64>, procs: &Vec<u64>) -> Self {
        // procs[0] += remaining_times[0];
        let mut s = Scheduling {
            remaining_times: remaining_times.clone(),
            // index: 0,
            best: std::u64::MAX,
            procs: procs.clone(),
            decisions: Vec::new(),
        };
        // Do the first step, else isFinished reports it's finished without doing anything :O
        s.decisions.push(Range {
            start: 0,
            end: procs.len(),
        });
        s.procs[0] += s.remaining_times[0];
        s
    }
    fn print(&mut self) {
        // println!("-----------");
        // println!("Times     : {:?}", self.remaining_times);
        // println!("Decisions : {:?}", self.decisions);
        // println!("Procs     : {:?}", self.procs);
    }
    pub fn redo_tree(&mut self) {
        self.procs.iter_mut().for_each(|p| *p = 0);
        // self.decisions
        //     .iter()
        //     .zip(&self.remaining_times)
        //     .for_each(|(d, t)| self.procs[d.start] += t);
        for i in 0..self.decisions.len() {
            self.procs[self.decisions[i].start] += self.remaining_times[i];
        }
    }
}
impl Scheduling {
    fn next(&mut self) {
        self.print();
        // println!("^ Next())");
        if let Some(mut d) = self.decisions.pop() {
            self.procs[d.start] -= self.remaining_times[self.decisions.len()];
            if d.start == d.end - 1 {
                self.next();
            } else {
                d.start += 1;
                self.procs[d.start] += 
                    self.remaining_times[self.decisions.len()];
                self.decisions.push(d);
            }
        }
    }
    fn split_range(range: &mut Range<usize>) -> Range<usize> {
        assert!(range.start < range.end - 1); // needs to be splittable

        let mid = (range.end + range.start) / 2;
        let other = Range {
            start: mid,
            end: range.end,
        };
        range.end = mid;
        other
    }
}
impl SimpleTask for Scheduling {
    fn step(&mut self) {
        self.print();
        // println!("Depth: {}, decisions: {:?}", self.index, self.decisions);
        // Sequential cut-off
        if self.remaining_times.len() - self.decisions.len() <= 8 {
            self.best = self.best.min(brute_force_rec(
                &mut self.procs,
                &mut self.remaining_times[self.decisions.len()..],
            ));
            // println!("{}", self.best);
            self.next();
            return;
        }
        self.decisions.push(Range {
            start: 0,
            end: self.procs.len(),
        });
        self.procs[0] += self.remaining_times[self.decisions.len() - 1];
    }
    fn can_split(&self) -> bool {
        true
    }

    fn split(&mut self, runner: impl Fn(&mut Self, &mut Self)) {
        // println!("Splitting");
        // println!("Trees: {:?}", self.decisions);
        let mut other = Scheduling {
            remaining_times: self.remaining_times.clone(),
            best: self.best,
            procs: self.procs.clone(),
            decisions: self.decisions.clone(),
        };
        for i in 0..self.decisions.len() {
            if self.decisions[i].start < self.decisions[i].end - 1 {
                let other_range = Scheduling::split_range(&mut self.decisions[i]);
                // self.decisions[i] = my_range;
                other.decisions[i] = other_range;
                other.decisions.truncate(i + 1);
                other.redo_tree();
                break;
            }
        }
        // println!("New Trees: {:?} vs {:?}", self.decisions, other.decisions);
        // assert!(self.decisions != other.decisions); // we should have a different choice somewhere
        if self.decisions == other.decisions {
            // We couldn't split
            return;

        }
        runner(self, &mut other);
    }
    fn fuse(&mut self, other: &mut Self) {
        self.best = other.best.min(self.best);
    }
    fn is_finished(&self) -> bool {
        self.decisions.is_empty()
    }
}

#[test]
fn test_scheduling() {
    let times: Vec<u64> = std::iter::repeat_with(|| rand::random::<u64>() % 10_000)
        .take(14)
        .collect();
    let procs: Vec<u64> = std::iter::repeat(0).take(P).collect();

    let mut s = Scheduling::new(&times, &procs);
    s.start();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(3)
        .steal_callback(|x| steal::steal(8, x))
        .build()
        .unwrap();
    let mut s = Scheduling::new(&times, &procs);
    pool.install(|| s.start());
    let mut b = BruteForcePar::new(times.clone());
    b.start();
    assert_eq!(s.get_result(), b.get_result());
    s.verify(&b.get_result());
}

use crate::adaptive_bench::Benchable;
impl<'a> Benchable<'a, u64> for Scheduling {
    fn start(&mut self) -> () {
        self.run()
    }
    fn name(&self) -> &'static str {
        "Adaptive_Scheduling"
    }
    fn get_result(&self) -> u64 {
        self.best
    }
    fn reset(&mut self) {
        self.procs.iter_mut().for_each(|p| *p = 0);
        *self = Self::new(&self.remaining_times, &self.procs);
        // self.best = std::u64::MAX;
    }
    fn verify(&self, result: &u64) -> bool {
        assert_eq!(*result, self.best);
        true
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
    fn verify(&self, result: &u64) -> bool {
        *result == self.result
    }
}

/*
fn greedy_scheduling(times: &[u64]) -> u64 {
    let procs: Vec<u64> = std::iter::repeat(0).take(P).collect(); // processors state (load)
    let procs = times.iter().fold(procs, |mut procs, time| {
        let min_index = (0..P).min_by_key(|&i| procs[i]).unwrap();
        procs[min_index] += time;
        procs
    });
    procs.iter().max().cloned().unwrap()
}
*/

// this is slooooooow
// fn brute_force_rec(procs: &mut Vec<u64>, times: &[u64]) -> u64 {
//     times
//         .split_first()
//         .map(|(time, remaining_times)| {
//             (0..P)
//                 .map(|i| {
//                     procs[i] += time;
//                     let r = brute_force_rec(procs, remaining_times);
//                     procs[i] -= time;
//                     r
//                 })
//                 .min()
//                 .unwrap()
//         })
//         .unwrap_or_else(|| *procs.iter().max().unwrap())
// }
fn brute_force_rec(procs: &mut Vec<u64>, times: &[u64]) -> u64 {
    if times.is_empty() {
        return *procs.iter().max().unwrap();
    }
    let (time, remaining_times) = times.split_first().unwrap();

    let mut best = std::u64::MAX;
    for i in 0..P {
        procs[i] += time;
        let r = brute_force_rec(procs, remaining_times);
        procs[i] -= time;
        best = best.min(r);
    }
    return best;
}

fn brute_force(times: &[u64]) -> u64 {
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    brute_force_rec(&mut procs, times)
}

fn brute_force_par(times: &[u64]) -> u64 {
    // START_REPLACING
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    brute_force_rec_par(&mut procs, times, 4)
    // END_COMMENTING
}

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
    let mut procs: Vec<u64> = std::iter::repeat(0).take(P).collect();
    let best_value = AtomicU64::new(initial_solution);
    branch_and_bound_rec_par(&mut procs, times, &best_value);
    best_value.load(Ordering::SeqCst)
    // END_COMMENTING
}

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
                // let value = procs.iter().max().cloned().unwrap();
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
                // let value = procs.iter().max().cloned().unwrap();
                // best_solution.fetch_min(value, Ordering::SeqCst);
            });
    }
}

fn compute_lower_bound(times: &[u64], times_sum: u64) -> u64 {
    debug_assert_eq!(times.iter().sum::<u64>(), times_sum);
    std::cmp::max(
        times.iter().max().cloned().unwrap_or(0),
        times_sum / P as u64,
    )
}


