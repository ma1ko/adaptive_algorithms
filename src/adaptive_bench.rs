
pub use crate::steal;

pub trait Benchable<'a, R>: Send + Sync {
    fn start(&mut self); // run the test
    fn name(&self) -> &'static str; // give it a nice name
                                    // fn id(&self) -> BenchmarkId; // not required, we create one directly
    fn verify(&self, _result: &R) -> bool {
        true
    } // if you want to verify for correctness
    fn get_result(&self) -> R {
        unimplemented!() // If you don't want to check for the result
    }
    fn get_thread_pool(&self, num_threads: usize, backoffs: Option<usize>) -> rayon::ThreadPool {
        let mut pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads);
        if let Some(backoffs) = backoffs {
            if backoffs == 0 {
                pool = pool.steal_callback(move |x| steal::optimized_steal(x))
            } else {
                pool = pool.steal_callback(move |x| steal::steal(backoffs, x));
            }
        }
        pool.build().unwrap()
    }
    // reset the test, will get called after every test so we can reuse it.
    fn reset(&mut self);
}

use criterion::BenchmarkGroup;
use criterion::*;
type Group<'a> = BenchmarkGroup<'a, criterion::measurement::WallTime>;
pub struct Tester<'a, R> {
    result: Option<R>,
    tests: Vec<TestConfig<'a, R>>,
    group: Group<'a>,
}
impl<'a, R> Tester<'a, R>
where
    R: std::fmt::Debug,
{
    pub fn verify(result: &R, test: Box<dyn Benchable<R> + 'a>) {
        assert!(test.verify(result))
    }
    pub fn new(tests: Vec<TestConfig<'a, R>>, group: Group<'a>, result: Option<R>) -> Self {
        Tester {
            result,
            tests,
            group,
        }
    }
    pub fn run(&mut self) {
        for test in &mut self.tests {
            let group = &mut self.group;
            // let checksum = self.checksum;
            group.bench_with_input(
                BenchmarkId::new(test.name(), test.num_cpus),
                &self.result,
                |b, result| {
                    let pool = test.get_thread_pool();
                    b.iter_batched(
                        || (),
                        |_| {
                            test.test.reset();
                            pool.install(|| test.test.start());
                            // Optional verification
                            if let Some(result) = result {
                                assert!(test.test.verify(result));
                            }
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }
}

pub struct TestConfig<'a, R> {
    pub len: usize,
    pub num_cpus: usize,
    pub backoff: Option<usize>,
    pub test: Box<dyn Benchable<'a, R> + 'a>,
}
impl<'a, R> TestConfig<'a, R> {
    pub fn new(
        len: usize,
        num_cpus: usize,
        backoff: Option<usize>,
        test: Box<dyn Benchable<'a, R> + 'a>,
    ) -> TestConfig<'a, R> {
        TestConfig {
            len,
            num_cpus,
            backoff,
            test,
        }
    }
    pub fn get_thread_pool(&self) -> rayon::ThreadPool {
        self.test.get_thread_pool(self.num_cpus, self.backoff)
    }
    pub fn name(&self) -> String {
        let backoff = if let Some(backoff) = self.backoff {
            if backoff == 0 {
                "optimized".to_string() + "/"
            } else {
                backoff.to_string() + "/"
            }
        } else {
            "".to_string()
        };
        self.test.name().to_string() + "/" + &backoff + &self.len.to_string()
    }
}
