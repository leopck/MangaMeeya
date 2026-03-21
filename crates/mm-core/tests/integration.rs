/// End-to-end integration tests that exercise the full pipeline:
/// archive -> decode -> cache -> navigate
use mm_core::{ImageCache, Navigator, PageMode, ViewSession};
use mm_image::ColorSpace;
use std::io::Write;
use std::path::PathBuf;

/// Create a real ZIP with actual JPEG images.
fn create_test_zip_with_images(count: usize, width: u32, height: u32) -> tempfile::TempPath {
    let tmp = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
    let mut writer =
        zip::ZipWriter::new(std::io::BufWriter::new(tmp.as_file().try_clone().unwrap()));

    for i in 0..count {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
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
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer
            .start_file(format!("page{:04}.jpg", i + 1), options)
            .unwrap();
        writer.write_all(&buf).unwrap();
    }
    writer.finish().unwrap();
    tmp.into_temp_path()
}

#[test]
fn e2e_open_zip_decode_all_pages() {
    let zip = create_test_zip_with_images(10, 200, 300);
    let archive = mm_archive::open(&zip).unwrap();

    assert_eq!(archive.entry_count(), 10);

    // Decode every single page and verify dimensions
    for i in 0..archive.entry_count() {
        let entry = archive.read_entry(i).unwrap();
        let img = mm_image::codec::decode(&entry.data).unwrap();
        assert_eq!(img.width, 200, "Page {i} width mismatch");
        assert_eq!(img.height, 300, "Page {i} height mismatch");
        assert!(
            img.color_space == ColorSpace::Rgb8 || img.color_space == ColorSpace::Rgba8,
            "Page {i} unexpected color space"
        );
        assert!(img.size_bytes() > 0, "Page {i} empty data");
    }
}

#[test]
fn e2e_open_zip_read_all_at_once() {
    let zip = create_test_zip_with_images(20, 100, 150);
    let archive = mm_archive::open(&zip).unwrap();
    let entries = archive.read_all().unwrap();

    assert_eq!(entries.len(), 20);
    for (i, entry) in entries.iter().enumerate() {
        assert!(
            !entry.data.is_empty(),
            "Entry {i} ({}) has empty data",
            entry.name
        );
        let img = mm_image::codec::decode(&entry.data).unwrap();
        assert_eq!(img.width, 100);
        assert_eq!(img.height, 150);
    }
}

#[test]
fn e2e_navigate_through_all_pages() {
    let zip = create_test_zip_with_images(50, 80, 120);
    let archive = mm_archive::open(&zip).unwrap();

    let mut session = ViewSession::new(PathBuf::from("test.zip"), archive.entry_count());
    session.page_mode = PageMode::Single;

    // Navigate forward through all pages
    let mut visited = [false; 50];
    visited[0] = true;
    for _ in 0..49 {
        let next = Navigator::next_page(&session);
        assert_ne!(next, session.current_page, "Navigation stuck");
        session.current_page = next;
        visited[session.current_page] = true;
    }
    assert!(visited.iter().all(|&v| v), "Not all pages visited");

    // Should be at last page now
    assert_eq!(session.current_page, 49);
    assert!(session.is_last_page());

    // Navigate all the way back
    for _ in 0..49 {
        let prev = Navigator::prev_page(&session);
        session.current_page = prev;
    }
    assert_eq!(session.current_page, 0);
    assert!(session.is_first_page());
}

#[test]
fn e2e_dual_page_navigation() {
    let zip = create_test_zip_with_images(10, 80, 120);
    let archive = mm_archive::open(&zip).unwrap();

    let mut session = ViewSession::new(PathBuf::from("test.zip"), archive.entry_count());
    session.page_mode = PageMode::Dual;

    assert_eq!(session.page_step(), 2);
    session.current_page = Navigator::next_page(&session);
    assert_eq!(session.current_page, 2);
    session.current_page = Navigator::next_page(&session);
    assert_eq!(session.current_page, 4);
}

#[test]
fn e2e_cache_decode_and_evict() {
    let zip = create_test_zip_with_images(10, 100, 100);
    let archive = mm_archive::open(&zip).unwrap();

    // Cache that holds max 5 pages
    let mut cache = ImageCache::new(5, 100);

    // Load all 10 pages through cache
    for i in 0..10 {
        let entry = archive.read_entry(i).unwrap();
        let img = mm_image::codec::decode(&entry.data).unwrap();
        cache.insert(i, img);
    }

    // Only 5 should remain (LRU eviction)
    assert_eq!(cache.len(), 5);

    // Most recent 5 pages should be cached
    for i in 5..10 {
        assert!(cache.contains(i), "Page {i} should be in cache");
    }
    // Oldest should be evicted
    for i in 0..5 {
        assert!(!cache.contains(i), "Page {i} should have been evicted");
    }
}

#[test]
fn e2e_open_folder_decode_images() {
    let dir = tempfile::TempDir::new().unwrap();

    // Create real JPEG files in a folder
    for i in 0..5 {
        let img = image::RgbImage::from_fn(100, 100, |x, y| {
            image::Rgb([((x + i * 50) % 256) as u8, ((y + i * 30) % 256) as u8, 0])
        });
        img.save(dir.path().join(format!("page{:02}.jpg", i + 1)))
            .unwrap();
    }

    let archive = mm_archive::open(dir.path()).unwrap();
    assert_eq!(archive.entry_count(), 5);

    for i in 0..5 {
        let entry = archive.read_entry(i).unwrap();
        let img = mm_image::codec::decode(&entry.data).unwrap();
        assert_eq!(img.width, 100);
        assert_eq!(img.height, 100);
    }
}

#[test]
fn e2e_natural_sort_order_in_archive() {
    let tmp = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
    let mut writer =
        zip::ZipWriter::new(std::io::BufWriter::new(tmp.as_file().try_clone().unwrap()));

    // Insert in wrong order
    let names = [
        "page10.jpg",
        "page2.jpg",
        "page1.jpg",
        "page20.jpg",
        "page3.jpg",
    ];
    for name in &names {
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file(*name, options).unwrap();
        // Write minimal valid JPEG
        let img = image::RgbImage::from_pixel(1, 1, image::Rgb([128, 128, 128]));
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            1,
            1,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
        writer.write_all(&buf).unwrap();
    }
    writer.finish().unwrap();

    let archive = mm_archive::open(tmp.path()).unwrap();
    let sorted_names = archive.entry_names();
    assert_eq!(
        sorted_names,
        vec![
            "page1.jpg",
            "page2.jpg",
            "page3.jpg",
            "page10.jpg",
            "page20.jpg"
        ]
    );
}

#[test]
fn e2e_corrupt_image_does_not_panic() {
    let tmp = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
    let mut writer =
        zip::ZipWriter::new(std::io::BufWriter::new(tmp.as_file().try_clone().unwrap()));

    // One valid, one corrupt
    let img = image::RgbImage::from_pixel(10, 10, image::Rgb([255, 0, 0]));
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        10,
        10,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();

    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("01_good.jpg", options).unwrap();
    writer.write_all(&buf).unwrap();

    writer
        .start_file("02_bad.jpg", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"NOT A JPEG").unwrap();

    writer.finish().unwrap();

    let archive = mm_archive::open(tmp.path()).unwrap();
    assert_eq!(archive.entry_count(), 2);

    // First entry (01_good.jpg) decodes fine
    let good = archive.read_entry(0).unwrap();
    assert!(
        mm_image::codec::decode(&good.data).is_ok(),
        "Good image should decode: {}",
        good.name
    );

    // Second entry (02_bad.jpg) returns error, does NOT panic
    let bad = archive.read_entry(1).unwrap();
    assert!(
        mm_image::codec::decode(&bad.data).is_err(),
        "Bad image should fail: {}",
        bad.name
    );
}
