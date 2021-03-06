use adaptive_algorithms::adaptive_bench::*;
use criterion::*;
extern crate rand;

use adaptive_algorithms::points::*;

fn bench(c: &mut Criterion) {
    let data = Point::create_random_points(5000);
    let mut group = c.benchmark_group("NearestNeighbor");
    group.warm_up_time(std::time::Duration::new(1, 0));
    group.measurement_time(std::time::Duration::new(3, 0));
    group.sample_size(10);
    group.nresamples(10);

    let cpus: Vec<usize> = vec![1, 2, 3, 4, 8, 16, 24, 32]
        .iter()
        .filter(|&&i| i <= num_cpus::get())
        .cloned()
        .collect();

    let mut test: Vec<TestConfig<f64>> = vec![];
    for i in &cpus {
        for s in vec![0, 6, 8] {
            let t = TestConfig::new(data.len(), *i, Some(s), Searcher::new(&data));
            test.push(t);
        }
        let t = TestConfig::new(data.len(), *i, None, RayonPoints::new(&data));
        test.push(t);
        let t = TestConfig::new(data.len(), *i, None, FlatMapPoints::new(&data));
        test.push(t);
    }
    let mut r = RayonPoints::new(&data);
    let mut t = Tester::new(test, group, r.start());
    t.run();

    // group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
