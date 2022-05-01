use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{Rng, thread_rng};
use rand::distributions::{Distribution, Standard};
use rand::seq::SliceRandom;
use sortedcontainers::sorted_containers::SortedContainers;


fn sorted_containers_random(input: &Vec<i32>) {
    let mut vec: SortedContainers<i32> = SortedContainers::default();
    for el in input {
        vec.insert(el.clone());
    }
}


fn criterion_benchmark(c: &mut Criterion) {
    let mut input: Vec<i32> = (0..100_000).collect();
    let mut rng = thread_rng();
    c.bench_function("sorted_containers random insert",
                     |b| b.iter(|| {
                         input.shuffle(&mut rng);
                         sorted_containers_random(&input);
                     }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
