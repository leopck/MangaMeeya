use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use mm_image::filter::{
    ImageFilter, crop::CropFilter, flip::FlipHorizontal, grayscale::GrayscaleFilter,
    resize::ResizeAlgorithm, resize::ResizeFilter, rotate::RotateFilter, rotate::Rotation,
    sharpen::SharpenFilter,
};
use mm_image::{ColorSpace, ImageBuffer};

/// Create a test JPEG of given dimensions, return raw bytes.
fn make_jpeg(width: u32, height: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([
            ((x * 3 + y * 7) % 256) as u8,
            ((x * 5 + y * 11) % 256) as u8,
            ((x * 7 + y * 3) % 256) as u8,
        ])
    });
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    buf
}

/// Create a test PNG of given dimensions.
fn make_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([((x * 3) % 256) as u8, ((y * 5) % 256) as u8, 128, 255])
    });
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();
    buf
}

fn make_test_buffer(width: u32, height: u32) -> ImageBuffer {
    let size = width as usize * height as usize * 3;
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    ImageBuffer::new(data, width, height, ColorSpace::Rgb8)
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    for &(w, h, label) in &[
        (640, 480, "480p"),
        (1280, 720, "720p"),
        (1920, 1080, "1080p"),
    ] {
        let jpeg_data = make_jpeg(w, h);
        group.bench_with_input(BenchmarkId::new("jpeg", label), &jpeg_data, |b, data| {
            b.iter(|| mm_image::codec::decode(black_box(data)).unwrap())
        });

        let png_data = make_png(w, h);
        group.bench_with_input(BenchmarkId::new("png", label), &png_data, |b, data| {
            b.iter(|| mm_image::codec::decode(black_box(data)).unwrap())
        });
    }

    group.finish();
}

fn bench_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize");

    let img_1080p = make_test_buffer(1920, 1080);

    group.bench_function("1080p->720p/nearest", |b| {
        let filter = ResizeFilter::new(1280, 720, ResizeAlgorithm::Nearest);
        b.iter(|| filter.apply(black_box(&img_1080p)).unwrap())
    });

    group.bench_function("1080p->720p/bilinear", |b| {
        let filter = ResizeFilter::new(1280, 720, ResizeAlgorithm::Bilinear);
        b.iter(|| filter.apply(black_box(&img_1080p)).unwrap())
    });

    group.bench_function("1080p->480p/nearest", |b| {
        let filter = ResizeFilter::new(640, 480, ResizeAlgorithm::Nearest);
        b.iter(|| filter.apply(black_box(&img_1080p)).unwrap())
    });

    group.bench_function("1080p->480p/bilinear", |b| {
        let filter = ResizeFilter::new(640, 480, ResizeAlgorithm::Bilinear);
        b.iter(|| filter.apply(black_box(&img_1080p)).unwrap())
    });

    group.finish();
}

fn bench_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("filters");
    let img = make_test_buffer(1920, 1080);

    group.bench_function("grayscale/1080p", |b| {
        b.iter(|| GrayscaleFilter.apply(black_box(&img)).unwrap())
    });

    group.bench_function("flip_h/1080p", |b| {
        b.iter(|| FlipHorizontal.apply(black_box(&img)).unwrap())
    });

    group.bench_function("rotate90/1080p", |b| {
        let filter = RotateFilter::new(Rotation::Cw90);
        b.iter(|| filter.apply(black_box(&img)).unwrap())
    });

    group.bench_function("crop/1080p->720p", |b| {
        let filter = CropFilter::new(0, 0, 1280, 720);
        b.iter(|| filter.apply(black_box(&img)).unwrap())
    });

    group.bench_function("sharpen/1080p", |b| {
        let filter = SharpenFilter::new(1.0, 1, 0);
        b.iter(|| filter.apply(black_box(&img)).unwrap())
    });

    group.finish();
}

criterion_group!(benches, bench_decode, bench_resize, bench_filters);
criterion_main!(benches);
