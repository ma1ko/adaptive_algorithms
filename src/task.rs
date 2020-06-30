use crate::rayon;
use crate::steal;

// if you can't use None because of typing errors, use Nothing
pub struct Dummy;
pub const NOTHING: Option<&mut Dummy> = None;

impl Task for Dummy {
    fn run(&mut self) {
        assert!(false);
    }
    fn step(&mut self) {
        assert!(false);
    }
    fn can_split(&self) -> bool {
        assert!(false);
        false
    }
    fn split(&mut self, _runner: impl FnMut(&mut Vec<&mut Self>), _steal_counter: usize) {
        assert!(false);
    }
    fn is_finished(&self) -> bool {
        assert!(false);
        true
    }
    fn fuse(&mut self, _other: &mut Self) {
        assert!(false);
    }
}

#[cfg(feature = "statistics")]
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
#[cfg(feature = "statistics")]
lazy_static! {
    pub static ref SUCCESSFUL_STEALS: AtomicUsize = AtomicUsize::new(0);
    pub static ref TOTAL_STEAL_COUNTER: AtomicUsize = AtomicUsize::new(0);
}

#[cfg(feature = "statistics")]
pub fn print_statistics() {
    let steals = SUCCESSFUL_STEALS.load(Relaxed);
    let total = TOTAL_STEAL_COUNTER.load(Relaxed);
    let successes = steal::STEAL_SUCCESS.load(Relaxed);
    let fails = steal::STEAL_FAIL.load(Relaxed);
    println!("Sum of steals: {}", steals);
    println!("Steal Counter: {}", total);
    println!("Average Steal Counter: {}", total as f64 / steals as f64);
    println!("Successful steals: {}", successes);
    println!("Failed steals: {}", fails);
}

pub trait Task: Sized + Send {
    // Both Task and SimpleTask runner have the (almost) same implementation, we can do sth maybe?
    fn runner(tasks: &mut Vec<&mut Self>) {
        if tasks.len() > 1 {
            // get the first task (take from the front so we can fuse correctly in the end
            let task = tasks.remove(0);
            // run it
            rayon::join(|| Self::runner(tasks), || task.run());

            // Finished doing all tasks, we need to fuse here
            // Grab the successor from the vector and fuse
            if let Some(other) = tasks.pop() {
                task.fuse(other);
            }
            // Push ourselves on the queue so the predecessor can fuse
            tasks.push(task);
        } else {
            // last task, reset counter so stealers know they can steal now
            steal::reset_my_steal_count();
            // just run the last task if it exists
            if let Some(task) = tasks.first_mut() {
                task.run();
            }
        }
    }
    fn runner_with<T: Task>(f: &mut impl Task, tasks: &mut Vec<&mut T>) {
        if tasks.len() == 1 {
            rayon::join(|| f.run(), || tasks[0].run());
            return;
        }
        let mut right = tasks.pop().unwrap();
        let left = tasks.pop().unwrap();
        rayon::join(
            || {
                steal::reset_my_steal_count();
                f.run();
                left.run()
            },
            || right.run(),
        );
        left.fuse(&mut right);
        if !tasks.is_empty() {
            tasks.push(left);
            T::runner(tasks);
        }
    }
    fn run(&mut self) {
        self.run_with(NOTHING)
    }
    fn run_with(&mut self, mut f: Option<&mut impl Task>) {
        let work = self.work();
        let mut run_loop = || {
            while !self.is_finished() {
                let steal_counter = steal::get_my_steal_count();
                if steal_counter != 0 && (f.as_ref().map_or(false, |x| x.can_split()) || self.can_split()) {
                    self.split_run_with(steal_counter, f.take());
                    continue;
                }
                self.step();
            }
        };
        if let Some((work_type, work_amount)) = work {
            rayon::subgraph(work_type, work_amount, || run_loop())
        } else {
            run_loop()
        }
    }
    fn step(&mut self);
    fn split_run_with(&mut self, steal_counter: usize, f: Option<&mut impl Task>) {
        if let Some(f) = f {
            if f.can_split() {
                f.split(|x| Self::runner_with(self, x), 1);

                return;
            }
        }
        self.split_run(steal_counter);
    }

    fn split_run(&mut self, steal_counter: usize) {
        #[cfg(feature = "statistics")]
        SUCCESSFUL_STEALS.fetch_add(1, Relaxed);
        #[cfg(feature = "statistics")]
        TOTAL_STEAL_COUNTER.fetch_add(steal_counter, Relaxed);

        #[cfg(feature = "multisplit")]
        self.split(Self::runner, steal_counter);
        #[cfg(not(feature = "multisplit"))]
        self.split(Self::runner, 1);
    }
    fn check_(&mut self) {
        self.check(NOTHING);
    }

    fn check(&mut self, mut f: Option<&mut impl Task>) {
        let steal_counter = steal::get_my_steal_count();
        if steal_counter != 0 && self.can_split() {
            self.split_run_with(steal_counter, f.take());
        }
    }
    fn can_split(&self) -> bool;
    fn work(&self) -> Option<(&'static str, usize)> {
        None
    }
    fn is_finished(&self) -> bool;
    fn split(&mut self, runner: impl FnMut(&mut Vec<&mut Self>), steal_counter: usize);
    fn fuse(&mut self, other: &mut Self);
}
