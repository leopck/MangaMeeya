use criterion::{Criterion, criterion_group, criterion_main};

fn bench_decode_jpeg(_c: &mut Criterion) {
    // TODO: Implement with generated test data
}

criterion_group!(benches, bench_decode_jpeg);
criterion_main!(benches);
