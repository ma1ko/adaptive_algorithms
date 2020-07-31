//! you need to compile me with a nightly compiler to enable the atomic_min_max feature.
//! we try to parallelize a brute force and branch and bound algorithm for
//! the independant tasks scheduling problem (P||Cmax).
// #![feature(integer_atomics)]
// use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

// const P: usize = 2; // the number of processors we simulate

use crate::task::*;
use rayon::prelude::*;
use std::ops::Range;
// use crate::task::NOTHING;

use smallvec::SmallVec;
#[derive(Debug, Clone)]
pub struct Scheduling<'a> {
    pub times: &'a [u64],
    pub best: u64,
    pub procs: Vec<u64>,
    pub counter: usize,
    pub sum: u64,
    pub pred: Option<&'a mut Scheduling<'a>>,
}
impl<'a> Scheduling<'a> {
    pub fn new(times: &'a Vec<u64>, procs: &Vec<u64>) -> Self {
        // procs[0] += remaining_times[0];
        let sum = times.iter().sum();
        let s = Scheduling {
            sum, // debugging
            times: &times,
            best: std::u64::MAX,
            // procs: SmallVec::from_vec(*procs),
            procs: procs.clone(),
            counter: 0,
            pred: None,
        };

        s
    }
    fn verify(&self) {
        assert_eq!(
            self.sum,
            self.times.iter().sum::<u64>() + self.procs.iter().sum::<u64>()
        );
    }
}
impl<'a> Task for Scheduling<'a> {
    fn step(&mut self) {
        // println!("Level: {}", self.times.len());
        // println!("{:?}", self);
        self.verify();
        // self.print();
        // println!("Depth: {}, decisions: {:?}", self.index, self.decisions);
        // Sequential cut-off
        if self.times.len() <= 5 {
            // subgraph("Cut-off", 1, || {
            // println!("Before {:?}", self);
            self.best = self
                .best
                .min(brute_force_rec(&mut self.procs, &mut self.times));
            // println!("After {:?}", self);
            return;
        }
        let (time, rest) = self.times.split_first().unwrap();
        // let before = std::mem::replace(&mut self.times, rest);

        // self.next();

        for i in self.counter..self.procs.len() {
            self.counter += 1;
            let mut scheduling = self.clone();
            scheduling.times = rest;
            scheduling.best = std::u64::MAX;
            scheduling.procs[i] += time;
            scheduling.counter = 0;
            scheduling.pred = Some(self);

            // scheduling.run_with(self);
            scheduling.run();

            scheduling.procs[i] -= time;
            self.best = self.best.min(scheduling.best);
        }
        self.counter = 0;
        // let _ = std::mem::replace(&mut self.times, before);
    }
    fn can_split(&self) -> bool {
        self.times.len() > 5
    }

    fn split(&mut self, mut runner: impl FnMut(&mut Vec<&mut Self>), _steal_counter: usize) {
        let mut splits = Vec::new();
        let (time, rest) = self.times.split_first().unwrap();
        // let before = std::mem::replace(&mut self.times, rest);

        for i in self.counter..self.procs.len() {
            self.counter += 1;
            let mut scheduling = self.clone();
            scheduling.times = rest;
            scheduling.best = std::u64::MAX;
            scheduling.procs[i] += time;
            scheduling.counter = 0;
            splits.push(scheduling);
        }
        // self.procs[0] += time;
        let mut splits = splits.iter_mut().collect::<Vec<&mut Self>>();
        // splits.insert(0, self);
        if splits.is_empty() {
            println!("Anc splitting");
            if let Some(mut pred) = self.pred {
                println!("Anc splitting");
                pred.split(runner, 1);
            }
            return;
        };
        if splits.len() == 1 {
            println!("Splitting: Sub");
            splits[0].split(runner, 1);
        } else {
            println!("Splitting: {}", splits.len());
            runner(&mut splits);
        }
        // self.procs[0] -= time;
        // let _ = std::mem::replace(&mut self.times, before);
        self.fuse(&mut splits[0]);
    }

    fn fuse(&mut self, other: &mut Self) {
        self.best = other.best.min(self.best);
    }
    fn is_finished(&self) -> bool {
        // self.decisions.is_empty()
        self.best != std::u64::MAX
    }
    fn work(&self) -> Option<(&'static str, usize)> {
        Some(("Scheduling", self.procs.len().pow(self.times.len() as u32)))
    }
}

#[test]
fn test_scheduling() {
    use crate::rayon::get_thread_pool;
    let times: Vec<u64> = std::iter::repeat_with(|| rand::random::<u64>() % 10_000)
        .take(16)
        .collect();
    let procs: Vec<u64> = std::iter::repeat(0).take(3).collect();

    let mut s = Scheduling::new(&times, &procs);
    // let my_result = s.start();
    let pool = get_thread_pool();
    let my_result = pool.install(|| s.start());
    // let my_result = s.start();
    let mut b = BruteForcePar::new(times.clone(), procs.clone());
    let other_result = b.start();
    assert_eq!(my_result, other_result);
    // s.verify(&b.get_result());
}

use crate::adaptive_bench::Benchable;
impl<'a> Benchable<'a, u64> for Scheduling<'a> {
    fn start(&mut self) -> Option<u64> {
        // self.procs.iter_mut().for_each(|p| *p = 0);
        self.best = std::u64::MAX;
        // *self = Self::new(&self.times, &self.procs);
        self.run();
        Some(self.best)
    }
    fn name(&self) -> &'static str {
        "Adaptive"
    }
}

pub struct BruteForcePar {
    times: Vec<u64>,
    procs: Vec<u64>,
}
impl BruteForcePar {
    pub fn new(times: Vec<u64>, procs: Vec<u64>) -> Self {
        BruteForcePar { times, procs }
    }
}
pub struct BruteForce {
    times: Vec<u64>,
    procs: Vec<u64>,
}
impl BruteForce {
    pub fn new(times: Vec<u64>, procs: Vec<u64>) -> Self {
        BruteForce { times, procs }
    }
}
impl<'a> Benchable<'a, u64> for BruteForce {
    fn name(&self) -> &'static str {
        "BruteForce-Sequential"
    }
    fn start(&mut self) -> Option<u64> {
        Some(brute_force(&self.times, self.procs.clone()))
    }
}
impl<'a> Benchable<'a, u64> for BruteForcePar {
    fn name(&self) -> &'static str {
        "BruteForce"
    }
    fn start(&mut self) -> Option<u64> {
        Some(brute_force_par(&self.times, self.procs.clone()))
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
    if procs.len() == 2 {
        // Really only works for two processors...
        return brute_force_rec_par_split(procs, times);
    }
    let levels = (rayon::current_num_threads() as f64).log2().ceil() * 2.0;
    // let mut procs: Vec<u64> = std::iter::repeat(0).take(2).collect();
    brute_force_rec_par(&mut procs, times, levels as usize)
}

fn brute_force_rec_par_split(procs: Vec<u64>, times: &[u64]) -> u64 {
    rayon::iter::split((procs, times), |(mut procs, times)| {
        if let Some((first, rest)) = times.split_first() {
            // let times2 = rest.clone();
            let mut procs2 = procs.clone();
            // if procs.len() == 2 {
            procs[0] += first;
            procs2[1] += first;
            ((procs, rest), Some((procs2, rest)))
        } else {
            ((procs, times), None)
        }
    })
    .map(|(mut procs, times)| brute_force_rec(&mut procs, times))
    .min()
    .unwrap()
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

fn _compute_lower_bound(times: &[u64], times_sum: u64, p: usize) -> u64 {
    debug_assert_eq!(times.iter().sum::<u64>(), times_sum);
    std::cmp::max(
        times.iter().max().cloned().unwrap_or(0),
        times_sum / p as u64,
    )
}
