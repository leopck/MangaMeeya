use criterion::{Criterion, criterion_group, criterion_main};

fn bench_texture_upload(_c: &mut Criterion) {
    // TODO: Implement with wgpu test harness
}

criterion_group!(benches, bench_texture_upload);
criterion_main!(benches);
