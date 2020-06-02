use adaptive_algorithms::adaptive_bench::*;
use criterion::BenchmarkGroup;
use criterion::*;
extern crate rand;




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
    let mut r = RayonPoints::new(&data);
    r.start();
    let mut t = Tester::new(test, group, Some(r.get_result()));
    t.run();

    // group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
