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

    fn split(&mut self, _runner: impl FnMut(Vec<&mut Self>), _steal_counter: usize) {
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
    println!("Successful Steals: {}", steals);
    println!("Steal Counter: {}", total);
    println!("Average Steal Counter: {}", total as f64 / steals as f64);
}

pub trait Task: Send + Sync + Sized {
    // run self *and* me, or return false if you can't
    fn run_(&mut self) {
        self.run(NOTHING)
    }
    fn run(&mut self, mut f: Option<&mut impl Task>) {
        while !self.is_finished() {
            let steal_counter = steal::get_my_steal_count();
            if steal_counter != 0 && self.can_split() {
                self.split_run(steal_counter, f.take());
                continue;
            }
            self.step();
        }
    }
    fn step(&mut self);
    fn split_run_(&mut self) {
        self.split_run(1, NOTHING)
    }

    fn _runner<T: Task>(&mut self, left: &mut T, mut right: &mut T) {
        rayon::join(
            || {
                steal::reset_my_steal_count();
                self.run_();
                left.run_()
            },
            || right.run_(),
        );
        left.fuse(&mut right);
        return;
    }
    fn runner(f: Option<&mut impl Task>, mut tasks: Vec<&mut Self>) {
        if let Some(f) = f {
            rayon::join(|| Self::runner(NOTHING, tasks), || f.run_());
        } else if let Some(task) = tasks.pop() {
            rayon::join(|| Self::runner(NOTHING, tasks), || task.run_());
        } else {
            steal::reset_my_steal_count();
        }
    }

    fn split_run(&mut self, steal_counter: usize, mut f: Option<&mut impl Task>) {
        #[cfg(feature = "statistics")]
        SUCCESSFUL_STEALS.fetch_add(1, Relaxed);
        #[cfg(feature = "statistics")]
        TOTAL_STEAL_COUNTER.fetch_add(steal_counter, Relaxed);

        let mut f = f.take();
        self.split(move |x| Self::runner(f.take(), x), steal_counter);
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
    fn is_finished(&self) -> bool;
    fn split(&mut self, runner: impl FnMut(Vec<&mut Self>), steal_counter: usize);
    fn fuse(&mut self, _other: &mut Self);
}

pub trait SimpleTask: Send + Sync {
    fn run(&mut self) {
        while !self.is_finished() {
            let steal_counter = steal::get_my_steal_count();
            if steal_counter != 0 && self.can_split() {
                self.split_run(steal_counter);
                continue;
            }
            self.step();
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
        // if let Some(task) = tasks.pop() {
        if !tasks.is_empty() {
            let task = tasks.remove(0);
            rayon::join(|| Self::runner(tasks), || task.run());

            if let Some(other) = tasks.pop() {
                task.fuse(other);
            }
            tasks.push(task);
        } else {
            steal::reset_my_steal_count();
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
    fn split(&mut self, runner: impl FnMut(&mut Vec<&mut Self>), steal_counter: usize);
    fn fuse(&mut self, _other: &mut Self);
}
