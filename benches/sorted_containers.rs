use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::distributions::{Distribution, Standard};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use sortedcontainers::sorted_containers::SortedContainers;

criterion_main!(benches);

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = sorted_containers_benchmark
}

fn sorted_containers_benchmark(c: &mut Criterion) {
    let mut rng = thread_rng();
    let mut group = c.benchmark_group("random insert sorted containers up to 100_000");
    for len in (10_000..100_000).step_by(10_000) {
        group.throughput(Throughput::Elements(len as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &len, |b, &len| {
            let mut input: Vec<i32> = (-len..len).collect();
            input.shuffle(&mut rng);
            b.iter(|| {
                insert_in_sorted_containers(&input);
            })
        });
    }
    for len in (100_000..=1_000_000).step_by(100_000) {
        group.throughput(Throughput::Elements(len as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &len, |b, &len| {
            let mut input: Vec<i32> = (-len..len).collect();
            input.shuffle(&mut rng);
            b.iter(|| {
                insert_in_sorted_containers(&input);
            })
        });
    }
    group.finish();
}

fn insert_in_sorted_containers(input: &Vec<i32>) {
    let mut vec: SortedContainers<i32> = SortedContainers::default();
    for el in input {
        vec.insert(el.clone());
    }
}
