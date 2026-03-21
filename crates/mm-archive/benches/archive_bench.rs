use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use mm_archive::ArchiveReader;
use std::io::Write;

/// Create a ZIP with N JPEG images.
fn make_test_zip(count: usize) -> tempfile::NamedTempFile {
    let tmp = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
    let mut writer =
        zip::ZipWriter::new(std::io::BufWriter::new(tmp.as_file().try_clone().unwrap()));

    for i in 0..count {
        let img = image::RgbImage::from_fn(200, 300, |x, y| {
            image::Rgb([
                ((x + i as u32 * 37) % 256) as u8,
                ((y + i as u32 * 73) % 256) as u8,
                128,
            ])
        });
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            200,
            300,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer
            .start_file(format!("page{i:04}.jpg"), options)
            .unwrap();
        writer.write_all(&buf).unwrap();
    }
    writer.finish().unwrap();
    tmp
}

fn bench_zip_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_open");

    for &count in &[10, 50, 100] {
        let zip = make_test_zip(count);
        group.bench_with_input(BenchmarkId::new("zip", count), &zip, |b, zip| {
            b.iter(|| {
                let reader =
                    mm_archive::zip_reader::ZipArchiveReader::open(black_box(zip.path())).unwrap();
                assert_eq!(reader.entry_count(), count);
            })
        });
    }
    group.finish();
}

fn bench_zip_read_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_read_all");

    for &count in &[10, 50] {
        let zip = make_test_zip(count);
        let reader = mm_archive::zip_reader::ZipArchiveReader::open(zip.path()).unwrap();
        group.bench_with_input(BenchmarkId::new("zip", count), &reader, |b, reader| {
            b.iter(|| {
                let entries = reader.read_all().unwrap();
                assert_eq!(entries.len(), count);
                black_box(entries);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_zip_open, bench_zip_read_all);
criterion_main!(benches);
