use crossbeam_utils::{Backoff, CachePadded};
use std::sync::atomic::{AtomicUsize, Ordering};

lazy_static! {
    static ref NUM_THREADS: usize = num_cpus::get();
    static ref V: Vec<CachePadded<AtomicUsize>> = (0..*NUM_THREADS)
        .map(|_| CachePadded::new(AtomicUsize::new(0)))
        .collect();
}
#[cfg(feature = "statistics")]
lazy_static! {
    pub static ref STEAL_SUCCESS: AtomicUsize = AtomicUsize::new(0);
    pub static ref STEAL_FAIL: AtomicUsize = AtomicUsize::new(0);
}
#[cfg(feature = "statistics")]
use std::cell::RefCell;
thread_local! {
#[cfg(feature = "statistics")]
    pub static LAST_VICTIM: RefCell<usize>= RefCell::new(0);
}

pub fn optimized_steal(victim: usize) -> Option<()> {
    let num_threads = rayon::current_num_threads();
    let backoffs = match num_threads {
        1 => panic!("Can't steal from myself"), // What are we even doing here?
        2..=8 => 6,
        9..=12 => 4,
        _ => 1,
    };
    steal(backoffs, victim)
}

pub fn steal(mut backoffs: usize, victim: usize) -> Option<()> {
    #[cfg(feature = "statistics")]
    LAST_VICTIM.with(|v| {
        *v.borrow_mut() = victim;
    });
    let thread_index = rayon::current_thread_index().unwrap();
    let thread_index = 1 << thread_index;
    V[victim].fetch_or(thread_index, Ordering::Relaxed);

    if backoffs == 0 {
        let num_threads = rayon::current_num_threads();
        backoffs = match num_threads {
            1 => panic!("Can't steal from myself"), // What are we even doing here?
            2..=8 => 6,
            9..=12 => 4,
            _ => 1,
        };
    }

    let backoff = Backoff::new();
    let mut c: usize;
    for _ in 0..backoffs {
        backoff.spin(); // spin or snooze()?

        // wait until the victim has taken the value, check regularly
        c = V[victim].load(Ordering::Relaxed);
        if c == 0 {
            #[cfg(feature = "statistics")]
            STEAL_SUCCESS.fetch_add(1, Ordering::Relaxed);

            return Some(());
        }
    }

    V[victim].fetch_and(!thread_index, Ordering::Relaxed);

    #[cfg(feature = "statistics")]
    STEAL_FAIL.fetch_add(1, Ordering::Relaxed);

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
