//! you need to compile me with a nightly compiler to enable the atomic_min_max feature.
//! we try to parallelize a brute force and branch and bound algorithm for
//! the independant tasks scheduling problem (P||Cmax).
// #![feature(integer_atomics)]
// use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

// const P: usize = 2; // the number of processors we simulate

use crate::rayon::{get_adaptive_thread_pool, subgraph};
use crate::task::SimpleTask;
use rayon::prelude::*;
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
        println!("-----------");
        println!("Times     : {:?}", self.remaining_times);
        println!("Decisions : {:?}", self.decisions);
        println!("Procs     : {:?}", self.procs);
    }
    pub fn redo_tree(&mut self) {
        self.procs.iter_mut().for_each(|p| *p = 0);
        // TODO: this has borrowing issues, so we just do the regular loop for now
        // self.decisions .iter()
        //     .zip(&self.remaining_times)
        //     .for_each(|(d, t)| self.procs[d.start] += t);
        for i in 0..self.decisions.len() {
            self.procs[self.decisions[i].start] += self.remaining_times[i];
        }
    }
    fn next(&mut self) {
        // self.print();
        if let Some(mut d) = self.decisions.pop() {
            self.procs[d.start] -= self.remaining_times[self.decisions.len()];
            if d.start == d.end - 1 {
                self.next();
            } else {
                d.start += 1;
                self.procs[d.start] += self.remaining_times[self.decisions.len()];
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
        // self.print();
        // println!("Depth: {}, decisions: {:?}", self.index, self.decisions);
        // Sequential cut-off
        if self.remaining_times.len() - self.decisions.len() <= 5 {
            // subgraph("Cut-off", 1, || {
            self.best = self.best.min(brute_force_rec(
                &mut self.procs,
                &mut self.remaining_times[self.decisions.len()..],
            ));
            // });
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
        // We need a tree that has a choice left (meaning 2 branches, one that is currently
        // executing and one we can steal
        self.decisions.iter().any(|r| r.end - r.start >= 2)
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
        let mut split = false;
        for i in 0..self.decisions.len() {
            if self.decisions[i].end - self.decisions[i].start >= 2 {
                split = true;
                let other_range = Scheduling::split_range(&mut self.decisions[i]);
                // self.decisions[i] = my_range;
                other.decisions[i] = other_range;
                other.decisions.truncate(i + 1);
                other.redo_tree();
                break;
            }
        }
        if !split {
            // We couldn't split
            assert!(false, "Couldn't split");
            // println!("Couldn't split");
            // println!("New Trees: {:?} vs {:?}", self.decisions, other.decisions);
            return;
        }
        assert!(self.decisions != other.decisions); // we should have a different choice somewhere

        // println!("Actually split");
        // println!("New Trees: {:?} vs {:?}", self.decisions, other.decisions);
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
        .take(12)
        .collect();
    let procs: Vec<u64> = std::iter::repeat(0).take(3).collect();

    let mut s = Scheduling::new(&times, &procs);
    s.start();
    let pool = get_adaptive_thread_pool();
    let mut s = Scheduling::new(&times, &procs);
    pool.install(|| s.start());
    // s.start();
    let mut b = BruteForcePar::new(times.clone(), procs.clone());
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
    procs: Vec<u64>,
}
impl BruteForcePar {
    pub fn new(times: Vec<u64>, procs: Vec<u64>) -> Self {
        BruteForcePar {
            times,
            result: std::u64::MAX,
            procs,
        }
    }
}
pub struct BruteForce {
    times: Vec<u64>,
    result: u64,
    procs: Vec<u64>,
}
impl BruteForce {
    pub fn new(times: Vec<u64>, procs: Vec<u64>) -> Self {
        BruteForce {
            times,
            result: std::u64::MAX,
            procs,
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
        self.result = brute_force(&self.times, self.procs.clone())
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
        self.result = brute_force_par(&self.times, self.procs.clone())
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
    for i in 0..procs.len() {
        procs[i] += time;
        let r = brute_force_rec(procs, remaining_times);
        procs[i] -= time;
        best = best.min(r);
    }
    return best;
}

fn brute_force(times: &[u64], mut procs: Vec<u64>) -> u64 {
    brute_force_rec(&mut procs, times)
}

fn brute_force_par(times: &[u64], mut procs: Vec<u64>) -> u64 {
    // let mut procs: Vec<u64> = std::iter::repeat(0).take(2).collect();
    brute_force_rec_par(&mut procs, times, 4)
}

fn brute_force_rec_par(procs: &mut Vec<u64>, times: &[u64], levels: usize) -> u64 {
    if levels == 0 {
        return brute_force_rec(procs, times);
    }
    times
        .split_first()
        .map(|(time, remaining_times)| {
            (0..procs.len())
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

pub fn branch_and_bound(times: &[u64], initial_solution: u64) -> u64 {
    let mut procs: Vec<u64> = std::iter::repeat(0).take(3).collect();
    branch_and_bound_rec(&mut procs, times, initial_solution)
}

pub fn branch_and_bound_rec(procs: &mut Vec<u64>, times: &[u64], mut best_solution: u64) -> u64 {
    if procs.iter().max().cloned().unwrap() >= best_solution {
        best_solution
    } else {
        times
            .split_first()
            .map(|(time, remaining_times)| {
                for i in 0..procs.len() {
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

pub fn branch_and_bound_par(times: &[u64], initial_solution: u64) -> u64 {
    let mut procs: Vec<u64> = std::iter::repeat(0).take(3).collect();
    let best_value = AtomicU64::new(initial_solution);
    branch_and_bound_rec_par(&mut procs, times, &best_value);
    best_value.load(Ordering::SeqCst)
    // END_COMMENTING
}

pub fn branch_and_bound_rec_par(procs: &mut Vec<u64>, times: &[u64], best_solution: &AtomicU64) {
    if procs.iter().max().cloned().unwrap() < best_solution.load(Ordering::SeqCst) {
        times
            .split_first()
            .map(|(time, remaining_times)| {
                (0..procs.len()).into_par_iter().for_each_init(
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

pub fn branch_and_bound_rec_fallback(
    procs: &mut Vec<u64>,
    times: &[u64],
    best_solution: &AtomicU64,
) {
    if procs.iter().max().cloned().unwrap() < best_solution.load(Ordering::SeqCst) {
        times
            .split_first()
            .map(|(time, remaining_times)| {
                for i in 0..procs.len() {
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

fn _compute_lower_bound(times: &[u64], times_sum: u64, P: usize) -> u64 {
    debug_assert_eq!(times.iter().sum::<u64>(), times_sum);
    std::cmp::max(
        times.iter().max().cloned().unwrap_or(0),
        times_sum / P as u64,
    )
}
