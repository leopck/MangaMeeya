use criterion::{Criterion, criterion_group, criterion_main};

fn bench_zip_open(_c: &mut Criterion) {
    // TODO: Implement with generated test data
}

criterion_group!(benches, bench_zip_open);
criterion_main!(benches);
