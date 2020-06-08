use adaptive_algorithms::adaptive_bench::*;
use adaptive_algorithms::scheduling::*;
use criterion::*;
extern crate rand;


fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scheduling");
    group.warm_up_time(std::time::Duration::new(1, 0));
    group.measurement_time(std::time::Duration::new(3, 0));
    group.sample_size(10);
    group.nresamples(10);

    let cpus: Vec<usize> = vec![1, 2, 3, 4, 8, 16, 24, 32]
        .iter()
        .filter(|&&i| i <= num_cpus::get())
        .cloned()
        .collect();

    let n = 22;
    let times: Vec<u64> = std::iter::repeat_with(|| rand::random::<u64>() % 10_000)
        .take(n)
        .collect();
    // two process scheduling
    let procs: Vec<u64> = std::iter::repeat(0).take(2).collect();
    let mut test: Vec<TestConfig<u64>> = vec![];
    // Baseline (single-core)
    let t = TestConfig {
        len: times.len(),
        num_cpus: 1,
        backoff: None,
        test: Box::new(BruteForce::new(times.clone())),
    };
    test.push(t);
    for i in &cpus {
        for s in vec![6, 8] {
            let t = TestConfig {
                len: times.len(),
                num_cpus: *i,
                backoff: Some(s),
                test: Box::new(Scheduling::new(
                    &times,
                    &procs,
                )),
            };
            test.push(t);
        }
        let t = TestConfig {
            len: times.len(),
            num_cpus: *i,
            backoff: None,
            test: Box::new(BruteForcePar::new(times.clone())),
        };
        test.push(t);
    }

    let mut b = BruteForce::new(times.clone());
    b.start();

    let mut t = Tester::new(test, group, Some(b.get_result()));
    t.run();

    // group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
