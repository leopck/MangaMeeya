use criterion::{criterion_group, criterion_main, Criterion};

fn bench_decode_jpeg(_c: &mut Criterion) {
    // TODO: Implement with generated test data
}

criterion_group!(benches, bench_decode_jpeg);
criterion_main!(benches);
