use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use mm_core::ImageCache;
use mm_image::{ColorSpace, ImageBuffer};

fn make_image(width: u32, height: u32) -> ImageBuffer {
    let size = width as usize * height as usize * 3;
    let data = vec![128u8; size];
    ImageBuffer::new(data, width, height, ColorSpace::Rgb8)
}

fn bench_cache_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_insert");

    for &count in &[10, 50, 200] {
        group.bench_with_input(BenchmarkId::new("pages", count), &count, |b, &count| {
            b.iter(|| {
                let mut cache = ImageCache::new(count, 500);
                for i in 0..count {
                    cache.insert(i, make_image(200, 300));
                }
                black_box(&cache);
            })
        });
    }

    group.finish();
}

fn bench_cache_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_get");

    let mut cache = ImageCache::new(200, 500);
    for i in 0..200 {
        cache.insert(i, make_image(200, 300));
    }

    group.bench_function("hit", |b| {
        b.iter(|| {
            let img = cache.get(black_box(100));
            assert!(img.is_some());
        })
    });

    group.bench_function("miss", |b| {
        b.iter(|| {
            let img = cache.get(black_box(999));
            assert!(img.is_none());
        })
    });

    group.finish();
}

fn bench_cache_eviction(c: &mut Criterion) {
    c.bench_function("cache_eviction/insert_300_into_200", |b| {
        b.iter(|| {
            let mut cache = ImageCache::new(200, 500);
            for i in 0..300 {
                cache.insert(i, make_image(100, 100));
            }
            assert_eq!(cache.len(), 200);
            black_box(&cache);
        })
    });
}

criterion_group!(
    benches,
    bench_cache_insert,
    bench_cache_get,
    bench_cache_eviction
);
criterion_main!(benches);
