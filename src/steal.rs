use crossbeam_utils::{Backoff, CachePadded};
use num_cpus;
use rayon::current_num_threads;
use std::sync::atomic::{AtomicUsize, Ordering};

lazy_static! {
    static ref NUM_THREADS: usize = num_cpus::get();
    static ref V: Vec<CachePadded<AtomicUsize>> = (0..*NUM_THREADS)
        .map(|_| CachePadded::new(AtomicUsize::new(0)))
        .collect();
}

pub fn optimized_steal(victim: usize) -> Option<()> {
    let thread_index = rayon::current_thread_index().unwrap();
    let thread_index = 1 << thread_index;
    let num_threads = rayon::current_num_threads();
    let backoffs = match num_threads {
        1 => panic!("Can't steal from myself"), // What are we even doing here?
        2..=8 => 6,
        9..=16 => 4,
        _ => 2,
    };
    V[victim].fetch_or(thread_index, Ordering::Relaxed);

    let backoff = Backoff::new();
    let mut c: usize;
    for _ in 0..backoffs{
        backoff.spin(); // spin or snooze()?

        c = V[victim].load(Ordering::Relaxed);
        if c == 0 {
            return Some(());
        }
    }
    V[victim].fetch_and(!thread_index, Ordering::Relaxed);
    None
}

pub fn steal(backoffs: usize, victim: usize) -> Option<()> {
    let thread_index = rayon::current_thread_index().unwrap();
    let thread_index = 1 << thread_index;
    V[victim].fetch_or(thread_index, Ordering::Relaxed);
    //V[victim].fetch_add(1, Ordering::Relaxed);

    let backoff = Backoff::new();
    let mut c: usize;
    for _ in 0..backoffs {
        backoff.spin(); // spin or snooze()?

        // wait until the victim has taken the value, check regularly
        c = V[victim].load(Ordering::Relaxed);
        if c == 0 {
            return Some(());
        }
    }

    V[victim].fetch_and(!thread_index, Ordering::Relaxed);
    //let i = V[victim].fetch_sub(1, Ordering::Relaxed);

    //let _ = V[victim].compare_exchange_weak(c, c - 1, Ordering::Relaxed, Ordering::Relaxed);

    None
}
pub fn get_my_steal_count() -> usize {
    if let Some(thread_index) = rayon::current_thread_index() {
        let steal_counter = V[thread_index].load(Ordering::Relaxed);
        let steal_counter = steal_counter.count_ones() as usize;
        let steal_counter = std::cmp::min(steal_counter, *NUM_THREADS - 1);
        steal_counter
    } else {
        0
    }
}
pub fn reset_my_steal_count() {
    if let Some(thread_index) = rayon::current_thread_index() {
        V[thread_index].store(0, Ordering::Relaxed);
    }
}
