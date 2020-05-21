use rayon;

pub use crate::steal;

pub trait Benchable<'a, R>  : Send + Sync{
    fn start(&mut self) -> ();
    fn name(&self) -> &'static str;
    // fn id(&self) -> BenchmarkId;
    fn verify(&self, _result: &R) -> bool { return true }
    fn get_result(&self) -> R;
    fn get_thread_pool(&self, num_threads: usize, backoffs: Option<usize>) -> rayon::ThreadPool{
        let mut pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads);
        if let Some(backoffs) = backoffs {
            pool = pool.steal_callback(move |x| steal::steal(backoffs, x));
        }
        pool.build().unwrap()
    }
    fn reset(&mut self) {}
}

use criterion::BenchmarkGroup;
use criterion::*;
type Group<'a> = BenchmarkGroup<'a, criterion::measurement::WallTime>;
pub struct Tester<'a, R>
where R: std::fmt::Debug
{
    // data : T,
    result: R,
    tests: Vec<TestConfig<'a, R>>,
    group: Group<'a>,
}
impl<'a, R> Tester<'a, R>
where R: std::fmt::Debug
{
    pub fn verify(result: &R, test: Box<dyn Benchable<R> + 'a>) {
        assert!(test.verify(result))
    }
    pub fn new(mut tests: Vec<TestConfig<'a, R>>, group: Group<'a>) -> Self {
        // tests[0].test.start();
        Tester {
            // data ,
            result: tests[0].test.get_result(),
            tests,
            group,
        }
    }
    pub fn run(&mut self) {
        for test in &mut self.tests {
            let group = &mut self.group;
            // let checksum = self.checksum;
            group.bench_with_input(BenchmarkId::new(test.name(), test.num_cpus), &(), |b, _| {
                let pool = test.get_thread_pool();
                b.iter_batched(
                    || (),
                    |_| {
                        test.test.reset();
                        pool.install(|| test.test.start());
                        // test.test.verify(&self.result);
                    },
                    BatchSize::SmallInput,
                );
            });
        }
    }
}
trait Test<T> {
    fn run(&self, numbers: &mut Vec<T>) -> ();
    fn name(&self) -> &'static str;
    fn id(&self) -> BenchmarkId;
}

pub struct TestConfig<'a, R> {
    pub len: usize,
    pub num_cpus: usize,
    pub backoff: Option<usize>,
    pub test: Box<dyn Benchable<'a, R> + 'a>,
}
impl<'a, R> TestConfig<'a, R> {
    pub fn get_thread_pool(&self) -> rayon::ThreadPool {
        self.test.get_thread_pool(self.num_cpus, self.backoff)
    }
    pub fn name(&self) -> String {
        let backoff = if let Some(backoff) = self.backoff {
            backoff.to_string() + "/"
        } else {
            "".to_string()
        };
        self.test.name().to_string()
            + "/"
            + &backoff
            + &self.len.to_string()
    }
}


