use crate::rayon;
use crate::steal;

// if you can't use None because of typing errors, use Nothing
// #[derive(Copy, Clone)]
pub struct Dummy;
pub const NOTHING: Option<&mut Dummy> = None;

pub type NoTask<'a> = Option<&'a mut Dummy>;
impl Task for Dummy {
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
    println!("Successful Steals: {}", steals);
    println!("Steal Counter: {}", total);
    println!("Average Steal Counter: {}", total as f64 / steals as f64);
    println!("Successes: {}", successes);
    println!("Fails: {}", fails);
}

pub trait Task: Send + Sized {
    // run self *and* me, or return false if you can't
    fn run_(&mut self) {
        self.run(NOTHING)
    }
    fn run(&mut self, mut f: Option<&mut impl Task>) {
        let work = self.work();
        let mut run_loop = || {
            while !self.is_finished() {
                let steal_counter = steal::get_my_steal_count();
                if steal_counter != 0 && self.can_split() {
                    self.split_run(steal_counter, f.take());
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
    fn split_run_(&mut self) {
        self.split_run(1, NOTHING)
    }

    // Both Task and SimpleTask runner have the (almost) same implementation, we can do sth maybe?
    fn runner(tasks: &mut Vec<&mut Self>) {
        if tasks.len() > 1 {
            // get the first task (take from the front so we can fuse correctly in the end
            let task = tasks.remove(0);
            // run it
            rayon::join(|| Self::runner(tasks), || task.run_());

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
                task.run_();
            }
        }
    }

    fn split_run(&mut self, steal_counter: usize, f: Option<&mut impl Task>) {
        #[cfg(feature = "statistics")]
        SUCCESSFUL_STEALS.fetch_add(1, Relaxed);
        #[cfg(feature = "statistics")]
        TOTAL_STEAL_COUNTER.fetch_add(steal_counter, Relaxed);

        if let Some(f) = f {
            if f.can_split() {
                rayon::join(|| self.run_(), || f.run_());
                return;
            }
        }
        self.split(Self::runner, steal_counter);
    }
    fn check_(&mut self) {
        self.check(NOTHING);
    }

    fn check(&mut self, mut f: Option<&mut impl Task>) {
        let steal_counter = steal::get_my_steal_count();
        if steal_counter != 0 && self.can_split() {
            self.split_run(steal_counter, f.take());
        }
    }
    fn can_split(&self) -> bool;
    fn work(&self) -> Option<(&'static str, usize)> {
        None
    }
    fn is_finished(&self) -> bool;
    fn split(&mut self, runner: impl FnMut(&mut Vec<&mut Self>), steal_counter: usize);
    fn fuse(&mut self, _other: &mut Self);
}

pub trait SimpleTask: Send {
    fn run(&mut self) {
        let work = self.work();
        let mut run_loop = || {
            while !self.is_finished() {
                let steal_counter = steal::get_my_steal_count();
                if steal_counter != 0 && self.can_split() {
                    self.split_run(steal_counter);
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
    fn check(&mut self) {
        let steal_counter = steal::get_my_steal_count();
        if steal_counter != 0 && self.can_split() {
            self.split_run(steal_counter);
        }
    }
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

    fn split_run(&mut self, steal_counter: usize) {
        #[cfg(feature = "statistics")]
        SUCCESSFUL_STEALS.fetch_add(1, Relaxed);
        #[cfg(feature = "statistics")]
        TOTAL_STEAL_COUNTER.fetch_add(steal_counter, Relaxed);
        self.split(Self::runner, steal_counter);
    }
    fn can_split(&self) -> bool;
    fn is_finished(&self) -> bool;
    fn work(&self) -> Option<(&'static str, usize)> {
        None
    }
    fn split(&mut self, runner: impl FnMut(&mut Vec<&mut Self>), steal_counter: usize);
    fn fuse(&mut self, _other: &mut Self);
}
