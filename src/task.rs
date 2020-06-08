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

    fn split(&mut self, _runner: impl FnMut(&mut Self, &mut Self)) {
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

    fn runner<T: Task>(&mut self, left: &mut T, mut right: &mut T) {
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
    fn split_run(&mut self, steal_counter: usize, mut f: Option<&mut impl Task>) {
        // run the parent task
        if let Some(f) = f.take() {
            if f.can_split() {
                f.split(move |left, right| self.runner(left, right));
                return;
            }
        }
        let runner = |left: &mut Self, right: &mut Self| {
            if steal_counter < 2 || !left.can_split() || !right.can_split() {
                rayon::join(
                    || {
                        steal::reset_my_steal_count();
                        left.run(NOTHING)
                    },
                    || right.run(NOTHING),
                );
                left.fuse(right);
            } else {
                rayon::join(
                    || left.split_run(steal_counter / 2, NOTHING),
                    || right.split_run(steal_counter / 2, NOTHING),
                );
                left.fuse(right);
            }
        };
        self.split(runner);
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
    fn split(&mut self, runner: impl FnMut(&mut Self, &mut Self));
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
    fn split_run(&mut self, steal_counter: usize) {
        let runner = |left: &mut Self, right: &mut Self| {
            if steal_counter < 2 || !left.can_split() || !right.can_split() {
                rayon::join(
                    || {
                        steal::reset_my_steal_count();
                        left.run()
                    },
                    || right.run(),
                );
                left.fuse(right);
            } else {
                rayon::join(
                    || left.split_run(steal_counter / 2),
                    || right.split_run(steal_counter / 2),
                );
                left.fuse(right);
            }
        };
        self.split(runner);
    }
    fn can_split(&self) -> bool;
    fn is_finished(&self) -> bool;
    fn split(&mut self, runner: impl Fn(&mut Self, &mut Self));
    fn fuse(&mut self, _other: &mut Self);
}
