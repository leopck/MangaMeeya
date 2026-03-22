/// End-to-end tests for Phase 4 features:
/// Scaling algorithms, navigation, auto-split detection, thumbnails,
/// browsing modes, config roundtrip, bookmarks+history, filter pipelines,
/// and the full user workflow from open to read to navigate to close.
use mm_config::{BrowsingMode, Config, ScaleFilterConfig, SortMode};
use mm_core::{BookmarkStore, HistoryStore, ImageCache, Navigator, Playlist, ScrollState, Slideshow, ViewSession};
use mm_image::filter::resize::{ResizeAlgorithm, ResizeFilter};
use mm_image::filter::sharpen::SharpenFilter;
use mm_image::filter::{ImageFilter, ImagePipeline};
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

fn make_session(pages: usize, current: usize, mode: mm_core::PageMode) -> ViewSession {
    let mut s = ViewSession::new(PathBuf::from("/test.zip"), pages);
    s.current_page = current;
    s.page_mode = mode;
    s
}

// ── E2E TEST 1 ──────────────────────────────────────────────────────────────
// Use case: User opens a manga ZIP, reads through in single-page mode,
// bookmarks a page, closes and reopens, resumes from history.

#[test]
fn e2e_01_open_read_bookmark_resume() {
    let zip = create_test_zip(20, 200, 300);
    let archive = mm_archive::open(&zip).unwrap();
    assert_eq!(archive.entry_count(), 20);

    // Simulate reading: navigate to page 12
    let mut session = make_session(20, 0, mm_core::PageMode::Single);
    for _ in 0..12 {
        session.current_page = Navigator::next_page(&session);
    }
    assert_eq!(session.current_page, 12);

    // Bookmark page 12
    let dir = tempfile::TempDir::new().unwrap();
    let mut bookmarks = BookmarkStore::load(&dir.path().join("bm.json"));
    bookmarks.add(PathBuf::from("/manga.zip"), 12, Some("Chapter 3 start".into()));
    assert!(bookmarks.is_bookmarked(Path::new("/manga.zip"), 12));
    bookmarks.save(&dir.path().join("bm.json")).unwrap();

    // Record history
    let mut history = HistoryStore::load(&dir.path().join("hist.json"));
    history.record(PathBuf::from("/manga.zip"), 12, 20);
    history.save(&dir.path().join("hist.json")).unwrap();

    // "Close" and "reopen" - load persisted state
    let bookmarks2 = BookmarkStore::load(&dir.path().join("bm.json"));
    let history2 = HistoryStore::load(&dir.path().join("hist.json"));
    assert!(bookmarks2.is_bookmarked(Path::new("/manga.zip"), 12));
    assert_eq!(history2.last_page_for(Path::new("/manga.zip")), Some(12));
}

// ── E2E TEST 2 ──────────────────────────────────────────────────────────────
// Use case: All 6 resize algorithms applied to a real decoded image from ZIP.

#[test]
fn e2e_02_all_resize_algorithms_on_real_image() {
    let zip = create_test_zip(1, 400, 600);
    let archive = mm_archive::open(&zip).unwrap();
    let entry = archive.read_entry(0).unwrap();
    let img = mm_image::codec::decode(&entry.data).unwrap();
    assert_eq!(img.width, 400);
    assert_eq!(img.height, 600);

    let algorithms = [
        ResizeAlgorithm::Nearest,
        ResizeAlgorithm::Bilinear,
        ResizeAlgorithm::Lanczos3,
        ResizeAlgorithm::Bicubic,
        ResizeAlgorithm::PixelAveraging,
        ResizeAlgorithm::Halftone,
    ];

    for algo in &algorithms {
        let filter = ResizeFilter::new(200, 300, *algo);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 200, "Width mismatch for {algo:?}");
        assert_eq!(result.height, 300, "Height mismatch for {algo:?}");
        assert!(result.size_bytes() > 0, "Empty result for {algo:?}");
        // Verify no panics and pixel data is sane
        assert!(
            result.data.iter().any(|&v| v > 0),
            "All-zero output for {algo:?}"
        );
    }
}

// ── E2E TEST 3 ──────────────────────────────────────────────────────────────
// Use case: N-page jump navigation (10, 20, 50 pages forward/back).

#[test]
fn e2e_03_n_page_jump_navigation() {
    let mut session = make_session(200, 0, mm_core::PageMode::Single);

    // Jump forward 10
    session.current_page = Navigator::jump_to(&session, session.current_page + 10);
    assert_eq!(session.current_page, 10);

    // Jump forward 20
    session.current_page = Navigator::jump_to(&session, session.current_page + 20);
    assert_eq!(session.current_page, 30);

    // Jump forward 50
    session.current_page = Navigator::jump_to(&session, session.current_page + 50);
    assert_eq!(session.current_page, 80);

    // Jump backward 10
    session.current_page = Navigator::jump_to(&session, session.current_page.saturating_sub(10));
    assert_eq!(session.current_page, 70);

    // Jump backward 20
    session.current_page = Navigator::jump_to(&session, session.current_page.saturating_sub(20));
    assert_eq!(session.current_page, 50);

    // Jump backward 50
    session.current_page = Navigator::jump_to(&session, session.current_page.saturating_sub(50));
    assert_eq!(session.current_page, 0);

    // Jump forward 50 from near end (should clamp to last page)
    session.current_page = 190;
    session.current_page = Navigator::jump_to(&session, session.current_page + 50);
    assert_eq!(session.current_page, 199);

    // Jump backward 50 from near start (should clamp to 0)
    session.current_page = 10;
    session.current_page = Navigator::jump_to(&session, session.current_page.saturating_sub(50));
    assert_eq!(session.current_page, 0);
}

// ── E2E TEST 4 ──────────────────────────────────────────────────────────────
// Use case: Auto-split detection for wide (landscape) images.

#[test]
fn e2e_04_auto_split_wide_image_detection() {
    // Portrait image: should NOT be split (W/H < 1.3)
    let portrait = ImageBuffer::blank(800, 1200, ColorSpace::Rgb8);
    let ratio = portrait.width as f32 / portrait.height as f32;
    assert!(ratio < 1.3, "Portrait should not trigger split");
    assert!(portrait.is_portrait());

    // Landscape image: SHOULD be split (W/H > 1.3)
    let landscape = ImageBuffer::blank(2400, 1200, ColorSpace::Rgb8);
    let ratio = landscape.width as f32 / landscape.height as f32;
    assert!(ratio > 1.3, "Wide landscape should trigger split");
    assert!(landscape.is_landscape());

    // Exactly at threshold: 1300 x 1000 = 1.3 ratio -> NOT split (needs > 1.3)
    let borderline = ImageBuffer::blank(1300, 1000, ColorSpace::Rgb8);
    let ratio = borderline.width as f32 / borderline.height as f32;
    assert!(
        (ratio - 1.3).abs() < 0.01,
        "Borderline should be near threshold"
    );

    // When split, left half and right half should each be valid sub-images
    // Simulate split by cropping
    use mm_image::filter::crop::CropFilter;
    let wide = ImageBuffer::blank(2400, 1200, ColorSpace::Rgb8);
    let left_half = CropFilter::new(0, 0, 1200, 1200).apply(&wide).unwrap();
    let right_half = CropFilter::new(1200, 0, 1200, 1200).apply(&wide).unwrap();
    assert_eq!(left_half.width, 1200);
    assert_eq!(right_half.width, 1200);
    assert_eq!(left_half.height, 1200);
    assert_eq!(right_half.height, 1200);
}

// ── E2E TEST 5 ──────────────────────────────────────────────────────────────
// Use case: Config roundtrip with all new Phase 4 fields.

#[test]
fn e2e_05_config_roundtrip_phase4_fields() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("config.toml");

    let mut config = Config::default();
    // Set all new Phase 4 fields
    config.viewing.auto_split = true;
    config.viewing.auto_split_threshold = 1.5;
    config.viewing.downscale_only = true;
    config.viewing.thumbnail_columns = 4;
    config.viewing.thumbnail_size = 150;
    config.image.scale_filter = ScaleFilterConfig::Lanczos;
    config.navigation.browsing_mode = BrowsingMode::Continuous;
    config.navigation.sort_files = SortMode::Size;
    config.navigation.sort_folders = SortMode::Date;

    config.save(&path).unwrap();
    let loaded = Config::load(&path).unwrap();

    assert!(loaded.viewing.auto_split);
    assert_eq!(loaded.viewing.auto_split_threshold, 1.5);
    assert!(loaded.viewing.downscale_only);
    assert_eq!(loaded.viewing.thumbnail_columns, 4);
    assert_eq!(loaded.viewing.thumbnail_size, 150);
    assert_eq!(loaded.image.scale_filter, ScaleFilterConfig::Lanczos);
    assert_eq!(loaded.navigation.browsing_mode, BrowsingMode::Continuous);
    assert_eq!(loaded.navigation.sort_files, SortMode::Size);
    assert_eq!(loaded.navigation.sort_folders, SortMode::Date);
}

// ── E2E TEST 6 ──────────────────────────────────────────────────────────────
// Use case: Filter pipeline combining PixelAveraging + Sharpening (the
// "Pixel Averaging + Weak/Strong Sharpening" presets from Scaling menu).

#[test]
fn e2e_06_filter_pipeline_pixel_avg_plus_sharpen() {
    let zip = create_test_zip(1, 400, 600);
    let archive = mm_archive::open(&zip).unwrap();
    let entry = archive.read_entry(0).unwrap();
    let img = mm_image::codec::decode(&entry.data).unwrap();

    // Pipeline: Downscale with PixelAveraging, then apply weak sharpen
    let mut pipeline = ImagePipeline::new();
    pipeline.add(Box::new(ResizeFilter::new(200, 300, ResizeAlgorithm::PixelAveraging)));
    pipeline.add(Box::new(SharpenFilter::new(0.5, 1, 0))); // Weak sharpen

    let result = pipeline.apply(&img).unwrap();
    assert_eq!(result.width, 200);
    assert_eq!(result.height, 300);
    assert!(result.data.iter().any(|&v| v > 0));

    // Pipeline: Downscale with PixelAveraging, then apply strong sharpen
    let mut pipeline2 = ImagePipeline::new();
    pipeline2.add(Box::new(ResizeFilter::new(200, 300, ResizeAlgorithm::PixelAveraging)));
    pipeline2.add(Box::new(SharpenFilter::new(2.0, 2, 0))); // Strong sharpen

    let result2 = pipeline2.apply(&img).unwrap();
    assert_eq!(result2.width, 200);
    assert_eq!(result2.height, 300);

    // Strong sharpen should produce different results than weak sharpen
    assert_ne!(result.data, result2.data, "Weak and strong sharpen should differ");
}

// ── E2E TEST 7 ──────────────────────────────────────────────────────────────
// Use case: Browsing mode "Repeat" wraps to first page at end,
// and "NoRepeat" stays at last page.

#[test]
fn e2e_07_browsing_modes_repeat_and_no_repeat() {
    let mut session = make_session(10, 0, mm_core::PageMode::Single);

    // Navigate to last page
    session.current_page = Navigator::last_page(&session);
    assert_eq!(session.current_page, 9);
    assert!(session.is_last_page());

    // In NoRepeat mode: next_page at end stays at end
    let next = Navigator::next_page(&session);
    assert_eq!(next, 9, "NoRepeat: should stay at last page");

    // Simulate Repeat mode: when at end, wrap to 0
    // (this logic lives in the app, not Navigator, but we verify the building blocks)
    let repeat_next = if session.is_last_page() { 0 } else { Navigator::next_page(&session) };
    assert_eq!(repeat_next, 0, "Repeat: should wrap to first page");

    // At first page, going back should wrap to last in Repeat mode
    session.current_page = 0;
    assert!(session.is_first_page());
    let prev = Navigator::prev_page(&session);
    assert_eq!(prev, 0, "NoRepeat: prev at start stays at start");
    let repeat_prev = if session.is_first_page() {
        Navigator::last_page(&session)
    } else {
        Navigator::prev_page(&session)
    };
    assert_eq!(repeat_prev, 9, "Repeat: should wrap to last page");
}

// ── E2E TEST 8 ──────────────────────────────────────────────────────────────
// Use case: Playlist sequential playback across multiple archives.

#[test]
fn e2e_08_playlist_sequential_playback() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut playlist = Playlist::new("Reading Session");

    // Add 3 archives to playlist
    let paths: Vec<PathBuf> = (1..=3)
        .map(|i| dir.path().join(format!("vol{i}.zip")))
        .collect();
    for p in &paths {
        playlist.add(p.clone(), None);
    }

    assert_eq!(playlist.entries.len(), 3);
    assert_eq!(playlist.current_index, 0);

    // Advance through playlist
    assert!(playlist.has_next());
    let entry1 = playlist.advance().cloned().unwrap();
    assert_eq!(entry1.path, paths[1]);
    assert_eq!(playlist.current_index, 1);

    let entry2 = playlist.advance().cloned().unwrap();
    assert_eq!(entry2.path, paths[2]);
    assert_eq!(playlist.current_index, 2);

    // No more next
    assert!(!playlist.has_next());

    // Go back
    assert!(playlist.has_prev());
    let entry_back = playlist.go_back().cloned().unwrap();
    assert_eq!(entry_back.path, paths[1]);

    // Save and reload
    playlist.save(&dir.path().join("pl.json")).unwrap();
    let loaded = Playlist::load(&dir.path().join("pl.json"));
    assert_eq!(loaded.entries.len(), 3);
}

// ── E2E TEST 9 ──────────────────────────────────────────────────────────────
// Use case: Full pipeline - open ZIP, decode, apply Lanczos resize,
// cache pages, navigate dual mode, verify scroll state.

#[test]
fn e2e_09_full_pipeline_zip_decode_resize_cache_navigate() {
    let zip = create_test_zip(10, 800, 1200);
    let archive = mm_archive::open(&zip).unwrap();
    assert_eq!(archive.entry_count(), 10);

    let mut cache = ImageCache::new(10, 500);
    let mut scroll = ScrollState::new();
    scroll.smooth = false;

    // Decode and resize all pages with Lanczos3
    for i in 0..archive.entry_count() {
        let entry = archive.read_entry(i).unwrap();
        let img = mm_image::codec::decode(&entry.data).unwrap();
        assert_eq!(img.width, 800);
        assert_eq!(img.height, 1200);

        // Apply Lanczos3 resize (fit to 400x600 viewport)
        let resized = ResizeFilter::fit(400, 600, img.width, img.height, ResizeAlgorithm::Lanczos3);
        let scaled = resized.apply(&img).unwrap();
        assert_eq!(scaled.width, 400);
        assert_eq!(scaled.height, 600);

        cache.insert(i, scaled);
    }

    assert_eq!(cache.len(), 10);

    // Navigate in dual mode
    let mut session = make_session(10, 0, mm_core::PageMode::Dual);

    // Set scroll dimensions: dual pages side by side in 1024x768 viewport
    scroll.set_dimensions(800.0, 600.0, 1024.0, 768.0);
    assert!(!scroll.can_scroll_vertically());

    // Navigate forward in dual steps
    session.current_page = Navigator::next_page(&session); // 0 -> 2
    assert_eq!(session.current_page, 2);
    session.current_page = Navigator::next_page(&session); // 2 -> 4
    assert_eq!(session.current_page, 4);

    // Verify dual page pairs
    let (left, right) = Navigator::dual_pages(&session);
    assert_eq!(left, 4);
    assert_eq!(right, Some(5));

    // Both pages should be in cache
    assert!(cache.contains(4));
    assert!(cache.contains(5));
}

// ── E2E TEST 10 ─────────────────────────────────────────────────────────────
// Use case: Complete user workflow - open archive, navigate, bookmark,
// use slideshow, check history, verify all features interact correctly.

#[test]
fn e2e_10_complete_user_workflow() {
    let zip = create_test_zip(50, 200, 300);
    let archive = mm_archive::open(&zip).unwrap();
    let dir = tempfile::TempDir::new().unwrap();

    // 1. Open archive, record history
    let mut history = HistoryStore::load(&dir.path().join("h.json"));
    let mut bookmarks = BookmarkStore::load(&dir.path().join("b.json"));
    let mut session = make_session(50, 0, mm_core::PageMode::Single);
    let path = PathBuf::from("/comics/manga_vol1.zip");
    history.record(path.clone(), 0, 50);

    // 2. Navigate to page 25
    session.current_page = Navigator::jump_to(&session, 25);
    assert_eq!(session.current_page, 25);
    history.record(path.clone(), 25, 50);

    // 3. Bookmark this page
    bookmarks.add(path.clone(), 25, Some("Good scene".into()));
    assert!(bookmarks.is_bookmarked(&path, 25));

    // 4. Verify image can be decoded at this page
    let entry = archive.read_entry(25).unwrap();
    let img = mm_image::codec::decode(&entry.data).unwrap();
    assert_eq!(img.width, 200);
    assert_eq!(img.height, 300);

    // 5. Apply all resize algorithms to verify no crashes
    for algo in [
        ResizeAlgorithm::Nearest,
        ResizeAlgorithm::Bilinear,
        ResizeAlgorithm::Lanczos3,
        ResizeAlgorithm::Bicubic,
        ResizeAlgorithm::PixelAveraging,
        ResizeAlgorithm::Halftone,
    ] {
        let filter = ResizeFilter::new(100, 150, algo);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 150);
    }

    // 6. Slideshow: verify tick behavior
    let mut slideshow = Slideshow::new(5.0);
    slideshow.toggle(); // start
    assert!(slideshow.active);
    assert!(!slideshow.tick(1.0)); // not enough time
    assert!(!slideshow.tick(3.0)); // 4 seconds total
    assert!(slideshow.tick(2.0)); // 6 seconds total -> should advance

    // 7. User input pauses slideshow
    slideshow.on_user_input();
    assert!(slideshow.is_paused());

    // 8. Continue navigation
    session.current_page = Navigator::next_page(&session);
    assert_eq!(session.current_page, 26);
    history.record(path.clone(), 26, 50);

    // 9. Jump forward 20
    session.current_page = Navigator::jump_to(&session, session.current_page + 20);
    assert_eq!(session.current_page, 46);

    // 10. Navigate to end
    session.current_page = Navigator::last_page(&session);
    assert_eq!(session.current_page, 49);
    assert!(session.is_last_page());

    // 11. Save all state
    bookmarks.save(&dir.path().join("b.json")).unwrap();
    history.save(&dir.path().join("h.json")).unwrap();

    // 12. Verify persistence
    let h2 = HistoryStore::load(&dir.path().join("h.json"));
    let b2 = BookmarkStore::load(&dir.path().join("b.json"));
    assert_eq!(h2.last_page_for(&path), Some(26)); // last recorded
    assert!(b2.is_bookmarked(&path, 25));

    // 13. Verify config roundtrip with Phase 4 fields
    let mut config = Config::default();
    config.image.scale_filter = ScaleFilterConfig::Lanczos;
    config.navigation.browsing_mode = BrowsingMode::Continuous;
    config.viewing.auto_split = true;
    config.save(&dir.path().join("c.toml")).unwrap();
    let c2 = Config::load(&dir.path().join("c.toml")).unwrap();
    assert_eq!(c2.image.scale_filter, ScaleFilterConfig::Lanczos);
    assert_eq!(c2.navigation.browsing_mode, BrowsingMode::Continuous);
    assert!(c2.viewing.auto_split);
}
