/// End-to-end tests for Phase 5 features:
/// TAR/GZ archives, Table of Contents, file save/clipboard,
/// folder tree navigation, sort options, and full workflow.
use mm_config::{BrowsingMode, Config, ScaleFilterConfig, SortMode};
use mm_core::{BookmarkStore, HistoryStore, Navigator, Playlist, TocStore, ViewSession};
use mm_image::filter::resize::{ResizeAlgorithm, ResizeFilter};
use mm_image::filter::ImageFilter;
use mm_image::{ColorSpace, ImageBuffer};
use std::io::Write;
use std::path::{Path, PathBuf};

// ── helpers ──────────────────────────────────────────────────────────────────

fn create_test_zip(count: usize, width: u32, height: u32) -> tempfile::TempPath {
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

fn create_test_tar(count: usize) -> tempfile::TempPath {
    let tmp = tempfile::Builder::new().suffix(".tar").tempfile().unwrap();
    let file = tmp.as_file().try_clone().unwrap();
    let mut builder = tar::Builder::new(file);
    for i in 0..count {
        let data = vec![128u8; 100]; // fake image data
        let name = format!("page{:04}.jpg", i + 1);
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, &name, &data[..])
            .unwrap();
    }
    builder.finish().unwrap();
    tmp.into_temp_path()
}

fn create_test_tar_gz(count: usize) -> tempfile::TempPath {
    let tmp = tempfile::Builder::new().suffix(".tar.gz").tempfile().unwrap();
    let file = tmp.as_file().try_clone().unwrap();
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
    let mut builder = tar::Builder::new(gz);
    for i in 0..count {
        let data = vec![200u8; 50];
        let name = format!("img{:03}.png", i + 1);
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, &name, &data[..])
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap();
    tmp.into_temp_path()
}

fn make_session(pages: usize, current: usize) -> ViewSession {
    let mut s = ViewSession::new(PathBuf::from("/test.zip"), pages);
    s.current_page = current;
    s.page_mode = mm_core::PageMode::Single;
    s
}

// ── E2E TEST 1 ──────────────────────────────────────────────────────────────
// Use case: Open a TAR archive, read entries, verify natural sort.

#[test]
fn e2e_p5_01_open_tar_archive() {
    let tar = create_test_tar(10);
    let archive = mm_archive::open(&tar).unwrap();
    assert_eq!(archive.entry_count(), 10);

    let names = archive.entry_names();
    assert_eq!(names[0], "page0001.jpg");
    assert_eq!(names[9], "page0010.jpg");

    // Read first and last entry
    let first = archive.read_entry(0).unwrap();
    assert_eq!(first.name, "page0001.jpg");
    assert_eq!(first.data.len(), 100);

    let last = archive.read_entry(9).unwrap();
    assert_eq!(last.name, "page0010.jpg");
}

// ── E2E TEST 2 ──────────────────────────────────────────────────────────────
// Use case: Open a TAR.GZ archive, read all entries.

#[test]
fn e2e_p5_02_open_tar_gz_archive() {
    let tgz = create_test_tar_gz(5);
    let archive = mm_archive::open(&tgz).unwrap();
    assert_eq!(archive.entry_count(), 5);

    let all = archive.read_all().unwrap();
    assert_eq!(all.len(), 5);
    for (i, entry) in all.iter().enumerate() {
        assert_eq!(entry.name, format!("img{:03}.png", i + 1));
        assert_eq!(entry.data.len(), 50);
    }
}

// ── E2E TEST 3 ──────────────────────────────────────────────────────────────
// Use case: Table of Contents - add chapters, navigate between them.

#[test]
fn e2e_p5_03_toc_chapter_navigation() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut toc = TocStore::new();
    let book = PathBuf::from("/manga/vol1.zip");

    // Add chapters
    toc.add_entry(&book, 0, "Cover".into());
    toc.add_entry(&book, 5, "Chapter 1: Beginning".into());
    toc.add_entry(&book, 25, "Chapter 2: Middle".into());
    toc.add_entry(&book, 50, "Chapter 3: Climax".into());
    toc.add_entry(&book, 90, "Chapter 4: End".into());

    assert_eq!(toc.get_entries(&book).len(), 5);

    // Navigate forward from page 0
    let next = toc.next_entry(&book, 0).unwrap();
    assert_eq!(next.page, 5);
    assert_eq!(next.label, "Chapter 1: Beginning");

    // Navigate forward from chapter 2
    let next = toc.next_entry(&book, 25).unwrap();
    assert_eq!(next.page, 50);

    // Navigate backward from chapter 3
    let prev = toc.prev_entry(&book, 50).unwrap();
    assert_eq!(prev.page, 25);

    // No chapter before cover
    assert!(toc.prev_entry(&book, 0).is_none());

    // No chapter after last
    assert!(toc.next_entry(&book, 90).is_none());

    // Save and reload
    toc.save(&dir.path().join("toc.json")).unwrap();
    let loaded = TocStore::load(&dir.path().join("toc.json"));
    assert_eq!(loaded.get_entries(&book).len(), 5);

    // Remove a chapter
    toc.remove_entry(&book, 25);
    assert_eq!(toc.get_entries(&book).len(), 4);
    assert!(!toc.has_entry(&book, 25));
}

// ── E2E TEST 4 ──────────────────────────────────────────────────────────────
// Use case: TOC across multiple books are independent.

#[test]
fn e2e_p5_04_toc_multiple_books() {
    let mut toc = TocStore::new();
    let book1 = PathBuf::from("/manga/vol1.zip");
    let book2 = PathBuf::from("/manga/vol2.zip");

    toc.add_entry(&book1, 0, "Ch1".into());
    toc.add_entry(&book1, 10, "Ch2".into());
    toc.add_entry(&book2, 0, "Intro".into());
    toc.add_entry(&book2, 20, "Part 2".into());
    toc.add_entry(&book2, 40, "Part 3".into());

    assert_eq!(toc.get_entries(&book1).len(), 2);
    assert_eq!(toc.get_entries(&book2).len(), 3);

    // Removing from book1 doesn't affect book2
    toc.remove_entry(&book1, 0);
    assert_eq!(toc.get_entries(&book1).len(), 1);
    assert_eq!(toc.get_entries(&book2).len(), 3);
}

// ── E2E TEST 5 ──────────────────────────────────────────────────────────────
// Use case: Save image to disk via file_ops (using ImageBuffer directly).

#[test]
fn e2e_p5_05_save_image_to_disk() {
    let zip = create_test_zip(1, 100, 100);
    let archive = mm_archive::open(&zip).unwrap();
    let entry = archive.read_entry(0).unwrap();
    let img = mm_image::codec::decode(&entry.data).unwrap();

    let dir = tempfile::TempDir::new().unwrap();

    // Save as JPEG
    let jpg_path = dir.path().join("test_output.jpg");
    let dyn_img = match img.color_space {
        ColorSpace::Rgb8 => {
            let rgb = image::RgbImage::from_raw(img.width, img.height, img.data.clone()).unwrap();
            image::DynamicImage::ImageRgb8(rgb)
        }
        ColorSpace::Rgba8 => {
            let rgba = image::RgbaImage::from_raw(img.width, img.height, img.data.clone()).unwrap();
            image::DynamicImage::ImageRgba8(rgba)
        }
        _ => panic!("Unexpected color space"),
    };
    dyn_img.save(&jpg_path).unwrap();
    assert!(jpg_path.exists());
    let saved_size = std::fs::metadata(&jpg_path).unwrap().len();
    assert!(saved_size > 0);

    // Save as PNG
    let png_path = dir.path().join("test_output.png");
    dyn_img.save(&png_path).unwrap();
    assert!(png_path.exists());
    assert!(std::fs::metadata(&png_path).unwrap().len() > 0);

    // Verify saved file can be decoded back
    let reloaded = image::open(&jpg_path).unwrap();
    assert_eq!(reloaded.width(), 100);
    assert_eq!(reloaded.height(), 100);
}

// ── E2E TEST 6 ──────────────────────────────────────────────────────────────
// Use case: Folder navigation - find sibling archives.

#[test]
fn e2e_p5_06_sibling_archive_navigation() {
    let dir = tempfile::TempDir::new().unwrap();

    // Create fake archive files (don't need to be real zips for path-based test)
    for name in ["vol01.zip", "vol02.zip", "vol03.zip", "vol10.zip"] {
        std::fs::write(dir.path().join(name), b"fake").unwrap();
    }

    // List and sort siblings
    let mut siblings: Vec<PathBuf> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "zip")
        })
        .collect();
    siblings.sort_by(|a, b| {
        mm_archive::natural_sort::natural_cmp(
            &a.to_string_lossy(),
            &b.to_string_lossy(),
        )
    });

    assert_eq!(siblings.len(), 4);
    // Natural sort: vol01 < vol02 < vol03 < vol10
    assert!(siblings[0].to_string_lossy().contains("vol01"));
    assert!(siblings[1].to_string_lossy().contains("vol02"));
    assert!(siblings[2].to_string_lossy().contains("vol03"));
    assert!(siblings[3].to_string_lossy().contains("vol10"));

    // Simulate "next" from vol02
    let current_idx = siblings
        .iter()
        .position(|p| p.to_string_lossy().contains("vol02"))
        .unwrap();
    let next = siblings.get(current_idx + 1).unwrap();
    assert!(next.to_string_lossy().contains("vol03"));

    // Simulate "prev" from vol03
    let current_idx = siblings
        .iter()
        .position(|p| p.to_string_lossy().contains("vol03"))
        .unwrap();
    let prev = siblings.get(current_idx - 1).unwrap();
    assert!(prev.to_string_lossy().contains("vol02"));
}

// ── E2E TEST 7 ──────────────────────────────────────────────────────────────
// Use case: Config with all Phase 5 fields persists correctly.

#[test]
fn e2e_p5_07_config_roundtrip_all_fields() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("config.toml");

    let mut config = Config::default();
    config.navigation.browsing_mode = BrowsingMode::Repeat;
    config.navigation.sort_files = SortMode::Extension;
    config.navigation.sort_folders = SortMode::Size;
    config.image.scale_filter = ScaleFilterConfig::Bicubic;
    config.viewing.auto_split = true;
    config.viewing.thumbnail_columns = 6;
    config.viewing.thumbnail_size = 300;

    config.save(&path).unwrap();
    let loaded = Config::load(&path).unwrap();

    assert_eq!(loaded.navigation.browsing_mode, BrowsingMode::Repeat);
    assert_eq!(loaded.navigation.sort_files, SortMode::Extension);
    assert_eq!(loaded.navigation.sort_folders, SortMode::Size);
    assert_eq!(loaded.image.scale_filter, ScaleFilterConfig::Bicubic);
    assert!(loaded.viewing.auto_split);
    assert_eq!(loaded.viewing.thumbnail_columns, 6);
    assert_eq!(loaded.viewing.thumbnail_size, 300);
}

// ── E2E TEST 8 ──────────────────────────────────────────────────────────────
// Use case: TAR archive natural sort with mixed numbering.

#[test]
fn e2e_p5_08_tar_natural_sort_mixed_names() {
    let tmp = tempfile::Builder::new().suffix(".tar").tempfile().unwrap();
    let file = tmp.as_file().try_clone().unwrap();
    let mut builder = tar::Builder::new(file);

    // Insert in wrong order
    for name in ["img10.jpg", "img2.jpg", "img1.jpg", "img20.jpg", "img3.jpg"] {
        let data = vec![0u8; 10];
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, name, &data[..])
            .unwrap();
    }
    builder.finish().unwrap();

    let archive = mm_archive::open(tmp.path()).unwrap();
    let names = archive.entry_names();
    assert_eq!(
        names,
        vec!["img1.jpg", "img2.jpg", "img3.jpg", "img10.jpg", "img20.jpg"]
    );
}

// ── E2E TEST 9 ──────────────────────────────────────────────────────────────
// Use case: Full pipeline with TAR.GZ - open, cache, navigate, resize.

#[test]
fn e2e_p5_09_full_pipeline_tar_gz() {
    // Create a tar.gz with real JPEG images
    let tmp = tempfile::Builder::new().suffix(".tar.gz").tempfile().unwrap();
    let file = tmp.as_file().try_clone().unwrap();
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
    let mut builder = tar::Builder::new(gz);

    for i in 0..5 {
        let img = image::RgbImage::from_fn(80, 120, |x, y| {
            image::Rgb([
                ((x + i * 37) % 256) as u8,
                ((y + i * 73) % 256) as u8,
                128,
            ])
        });
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            80,
            120,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();

        let name = format!("page{:03}.jpg", i + 1);
        let mut header = tar::Header::new_gnu();
        header.set_size(buf.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, &name, &buf[..])
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap();

    // Open and decode
    let archive = mm_archive::open(tmp.path()).unwrap();
    assert_eq!(archive.entry_count(), 5);

    for i in 0..5 {
        let entry = archive.read_entry(i).unwrap();
        let img = mm_image::codec::decode(&entry.data).unwrap();
        assert_eq!(img.width, 80);
        assert_eq!(img.height, 120);

        // Apply Lanczos resize
        let filter = ResizeFilter::new(40, 60, ResizeAlgorithm::Lanczos3);
        let resized = filter.apply(&img).unwrap();
        assert_eq!(resized.width, 40);
        assert_eq!(resized.height, 60);
    }
}

// ── E2E TEST 10 ─────────────────────────────────────────────────────────────
// Use case: Complete workflow with TOC, bookmarks, history, playlist
// across multiple archive formats.

#[test]
fn e2e_p5_10_complete_multi_format_workflow() {
    let dir = tempfile::TempDir::new().unwrap();

    // Create a ZIP and a TAR
    let zip = create_test_zip(20, 100, 150);
    let tar = create_test_tar(10);

    // Open ZIP, verify it works
    let zip_archive = mm_archive::open(&zip).unwrap();
    assert_eq!(zip_archive.entry_count(), 20);

    // Open TAR, verify it works
    let tar_archive = mm_archive::open(&tar).unwrap();
    assert_eq!(tar_archive.entry_count(), 10);

    // Create history entries for both
    let mut history = HistoryStore::load(&dir.path().join("h.json"));
    history.record(PathBuf::from("manga.zip"), 10, 20);
    history.record(PathBuf::from("manga.tar"), 5, 10);

    // Create bookmarks
    let mut bookmarks = BookmarkStore::load(&dir.path().join("b.json"));
    bookmarks.add(PathBuf::from("manga.zip"), 10, Some("Good scene".into()));
    bookmarks.add(PathBuf::from("manga.tar"), 3, None);

    // Create TOC entries
    let mut toc = TocStore::new();
    toc.add_entry(Path::new("manga.zip"), 0, "Cover".into());
    toc.add_entry(Path::new("manga.zip"), 5, "Ch1".into());
    toc.add_entry(Path::new("manga.zip"), 15, "Ch2".into());
    toc.add_entry(Path::new("manga.tar"), 0, "Start".into());
    toc.add_entry(Path::new("manga.tar"), 5, "Middle".into());

    // Create playlist with both
    let mut playlist = Playlist::new("Multi-format");
    playlist.add(PathBuf::from("manga.zip"), Some("ZIP vol".into()));
    playlist.add(PathBuf::from("manga.tar"), Some("TAR vol".into()));

    // Save everything
    history.save(&dir.path().join("h.json")).unwrap();
    bookmarks.save(&dir.path().join("b.json")).unwrap();
    toc.save(&dir.path().join("toc.json")).unwrap();
    playlist.save(&dir.path().join("pl.json")).unwrap();

    // Reload and verify
    let h2 = HistoryStore::load(&dir.path().join("h.json"));
    let b2 = BookmarkStore::load(&dir.path().join("b.json"));
    let toc2 = TocStore::load(&dir.path().join("toc.json"));
    let pl2 = Playlist::load(&dir.path().join("pl.json"));

    assert_eq!(h2.last_page_for(Path::new("manga.zip")), Some(10));
    assert_eq!(h2.last_page_for(Path::new("manga.tar")), Some(5));
    assert!(b2.is_bookmarked(Path::new("manga.zip"), 10));
    assert!(b2.is_bookmarked(Path::new("manga.tar"), 3));
    assert_eq!(toc2.get_entries(Path::new("manga.zip")).len(), 3);
    assert_eq!(toc2.get_entries(Path::new("manga.tar")).len(), 2);
    assert_eq!(pl2.entries.len(), 2);

    // Navigate through TOC
    let next_ch = toc2.next_entry(Path::new("manga.zip"), 0).unwrap();
    assert_eq!(next_ch.page, 5);
    assert_eq!(next_ch.label, "Ch1");
}
