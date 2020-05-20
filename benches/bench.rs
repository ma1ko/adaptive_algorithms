use adaptive_algorithms::adaptive_bench::Benchable;
use criterion::BenchmarkGroup;
use criterion::*;
extern crate rand;

type Group<'a> = BenchmarkGroup<'a, criterion::measurement::WallTime>;
struct Tester<'a, R>
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
    fn verify(result: &R, test: Box<dyn Benchable<R> + 'a>) {
        assert!(test.verify(result))
    }
    fn new(mut tests: Vec<TestConfig<'a, R>>, group: Group<'a>) -> Self {
        tests[0].test.start();
        Tester {
            // data ,
            result: tests[0].test.get_result(),
            tests,
            group,
        }
    }
    fn run(&mut self) {
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

struct TestConfig<'a, R> {
    len: usize,
    num_cpus: usize,
    backoff: Option<usize>,
    test: Box<dyn Benchable<'a, R> + 'a>,
}
impl<'a, R> TestConfig<'a, R> {
    fn get_thread_pool(&self) -> rayon::ThreadPool {
        self.test.get_thread_pool(self.num_cpus, self.backoff)
    }
    fn name(&self) -> String {
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

use adaptive_algorithms::points::*;

fn bench(c: &mut Criterion) {
    let data = Point::create_random_points(5000);
    let mut group = c.benchmark_group("NearestNeighbor");
    group.warm_up_time(std::time::Duration::new(1, 0));
    group.measurement_time(std::time::Duration::new(3,0));
    group.sample_size(10);
    group.nresamples(10);

    let cpus: Vec<usize> = vec![1, 2, 3, 4, 8, 16, 24, 32]
        .iter()
        .filter(|&&i| i <= num_cpus::get())
        .cloned()
        .collect();

    let mut test: Vec<TestConfig<f64>> = vec![];
    for i in &cpus {
        for s in vec![6, 8] {
            let t = TestConfig {
                len: data.len(),
                num_cpus: *i,
                backoff: Some(s),
                test: Box::new(Searcher::new(&data)),
            };
            test.push(t);
        }
        let t = TestConfig {
            len: data.len(),
            num_cpus: *i,
            backoff: None,
            test: Box::new(RayonPoints::new(&data)),
        };
        test.push(t);
    }
    // let x: Box<dyn Test<u32>> = Box::new(Single::new(1));
    // test.push(x
    let mut t = Tester::new(test, group);
    // let mut t = Tester::new(v_21, test, group);
    t.run();

    // group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
