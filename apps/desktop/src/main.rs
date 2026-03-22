#![windows_subsystem = "windows"]

use eframe::egui;
use mm_config::{BrowsingMode, ScaleFilterConfig, SortMode};
use mm_core::{BookmarkStore, HistoryStore, Loupe, Playlist, ScrollState, Slideshow};
use mm_image::ColorSpace;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let initial_path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("MangaMeeya")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "MangaMeeya",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, initial_path)))),
    )
}

struct App {
    // Archive
    archive: Option<Box<dyn mm_archive::ArchiveReader>>,
    source_path: Option<PathBuf>,
    current_page: usize,
    total_pages: usize,
    page_textures: Vec<Option<egui::TextureHandle>>,
    error_message: Option<String>,

    // Image dimensions for info display
    current_image_dims: Option<(u32, u32)>,

    // View state
    fit_mode: FitMode,
    page_mode: PageMode,
    reading_dir: ReadingDir,
    zoom: f32,
    scroll: ScrollState,
    downscale_only: bool,
    scale_filter: ScaleFilterConfig,

    // Auto-split state
    auto_split: bool,
    auto_split_threshold: f32,
    /// When auto-split is active on a wide image, we have virtual pages:
    /// split_pages[i] = (real_page_idx, SplitHalf) for each virtual page
    split_pages: Vec<(usize, SplitHalf)>,
    virtual_page: usize,
    virtual_total: usize,

    // Thumbnail view
    show_thumbnails: bool,
    thumbnail_columns: u32,
    thumbnail_textures: Vec<Option<egui::TextureHandle>>,

    // Features
    loupe: Loupe,
    slideshow: Slideshow,
    bookmarks: BookmarkStore,
    history: HistoryStore,
    playlist: Playlist,

    // Browsing mode
    browsing_mode: BrowsingMode,

    // UI toggles
    show_page_bar: bool,
    show_toolbar_icons: bool,
    show_settings: bool,
    show_bookmarks: bool,
    show_history: bool,
    show_playlist: bool,
    show_file_index: bool,
    show_go_to_page: bool,
    go_to_page_input: String,
    show_image_info: bool,
    show_cache_info: bool,
    show_about: bool,

    config: mm_config::Config,
    last_frame_time: std::time::Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum FitMode {
    FitScreen,
    FitWidth,
    FitHeight,
    Original,
    FitWindowToImage,
    FitWindow2Pages,
    ManualScale(f32),
}

#[derive(Clone, Copy, PartialEq)]
enum PageMode {
    Single,
    Dual,
    DualWithCover,
}

#[derive(Clone, Copy, PartialEq)]
enum ReadingDir {
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, PartialEq)]
enum SplitHalf {
    Full,
    Left,
    Right,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        let data_dir = data_dir();
        let config = mm_config::Config::load(&data_dir.join("config.toml")).unwrap_or_default();
        let mut scroll = ScrollState::new();
        scroll.smooth = config.scrolling.smooth_scroll;
        scroll.speed = config.scrolling.smooth_scroll_speed as f32;

        let bookmarks = BookmarkStore::load(&data_dir.join("bookmarks.json"));
        let history = HistoryStore::load(&data_dir.join("history.json"));
        let playlist = Playlist::load(&data_dir.join("playlist.json"));

        let browsing_mode = config.navigation.browsing_mode;
        let scale_filter = config.image.scale_filter;
        let auto_split = config.viewing.auto_split;
        let auto_split_threshold = config.viewing.auto_split_threshold;
        let downscale_only = config.viewing.downscale_only;
        let thumbnail_columns = config.viewing.thumbnail_columns;

        let mut app = Self {
            archive: None,
            source_path: None,
            current_page: 0,
            total_pages: 0,
            page_textures: Vec::new(),
            error_message: None,
            current_image_dims: None,
            fit_mode: FitMode::FitScreen,
            page_mode: PageMode::Single,
            reading_dir: ReadingDir::Rtl,
            zoom: 1.0,
            scroll,
            downscale_only,
            scale_filter,
            auto_split,
            auto_split_threshold,
            split_pages: Vec::new(),
            virtual_page: 0,
            virtual_total: 0,
            show_thumbnails: false,
            thumbnail_columns,
            thumbnail_textures: Vec::new(),
            loupe: Loupe::new(),
            slideshow: Slideshow::new(5.0),
            bookmarks,
            history,
            playlist,
            browsing_mode,
            show_page_bar: true,
            show_toolbar_icons: true,
            show_settings: false,
            show_bookmarks: false,
            show_history: false,
            show_playlist: false,
            show_file_index: false,
            show_go_to_page: false,
            go_to_page_input: String::new(),
            show_image_info: false,
            show_cache_info: false,
            show_about: false,
            config,
            last_frame_time: std::time::Instant::now(),
        };
        if let Some(path) = initial_path {
            app.open_path(path, &cc.egui_ctx);
        }
        app
    }

    fn open_path(&mut self, path: PathBuf, ctx: &egui::Context) {
        match mm_archive::open(&path) {
            Ok(reader) => {
                self.total_pages = reader.entry_count();
                self.page_textures = vec![None; self.total_pages];
                self.thumbnail_textures = vec![None; self.total_pages];
                self.archive = Some(reader);
                self.error_message = None;
                self.scroll.reset();
                self.zoom = 1.0;

                // Resume from history if available
                let start_page = self
                    .history
                    .last_page_for(&path)
                    .unwrap_or(0)
                    .min(self.total_pages.saturating_sub(1));

                self.source_path = Some(path.clone());
                self.history.record(path, start_page, self.total_pages);

                // Build split pages map
                self.rebuild_split_pages();

                self.goto_page(start_page, ctx);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to open: {e}"));
            }
        }
    }

    fn rebuild_split_pages(&mut self) {
        self.split_pages.clear();
        if !self.auto_split {
            for i in 0..self.total_pages {
                self.split_pages.push((i, SplitHalf::Full));
            }
        } else {
            // We need to check each page's dimensions to decide if it should be split
            // For now, build lazily - assume all full, and update as we decode
            for i in 0..self.total_pages {
                self.split_pages.push((i, SplitHalf::Full));
            }
        }
        self.virtual_total = self.split_pages.len();
    }

    /// Check if a decoded page should be split and update the split_pages map
    fn maybe_split_page(&mut self, page: usize) {
        if !self.auto_split {
            return;
        }
        let Some(Some(tex)) = self.page_textures.get(page) else {
            return;
        };
        let sz = tex.size_vec2();
        let ratio = sz.x / sz.y;

        // Already split pages for this index?
        let already_split = self
            .split_pages
            .iter()
            .any(|(p, h)| *p == page && *h != SplitHalf::Full);

        if ratio > self.auto_split_threshold && !already_split {
            // Replace the Full entry with Left+Right
            if let Some(pos) = self
                .split_pages
                .iter()
                .position(|(p, h)| *p == page && *h == SplitHalf::Full)
            {
                let (left, right) = if self.reading_dir == ReadingDir::Rtl {
                    (SplitHalf::Right, SplitHalf::Left)
                } else {
                    (SplitHalf::Left, SplitHalf::Right)
                };
                self.split_pages[pos] = (page, left);
                self.split_pages.insert(pos + 1, (page, right));
                self.virtual_total = self.split_pages.len();
            }
        }
    }

    fn ensure_page_decoded(&mut self, page: usize, ctx: &egui::Context) {
        if page >= self.total_pages || self.page_textures[page].is_some() {
            return;
        }
        let Some(archive) = &self.archive else {
            return;
        };
        match archive.read_entry(page) {
            Ok(entry) => match mm_image::codec::decode(&entry.data) {
                Ok(img) => {
                    self.current_image_dims = Some((img.width, img.height));
                    let pixels = to_rgba_pixels(&img);
                    let color_image = egui::ColorImage {
                        size: [img.width as usize, img.height as usize],
                        pixels,
                    };
                    let texture = ctx.load_texture(
                        format!("page-{page}"),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.page_textures[page] = Some(texture);
                    self.maybe_split_page(page);
                }
                Err(e) => tracing::warn!("Decode page {page}: {e}"),
            },
            Err(e) => tracing::warn!("Read page {page}: {e}"),
        }
    }

    fn goto_page(&mut self, page: usize, ctx: &egui::Context) {
        if self.total_pages == 0 {
            return;
        }
        self.current_page = page.min(self.total_pages - 1);
        self.scroll.reset();
        self.ensure_page_decoded(self.current_page, ctx);

        // Update virtual page tracking
        if let Some(vp) = self
            .split_pages
            .iter()
            .position(|(p, _)| *p == self.current_page)
        {
            self.virtual_page = vp;
        }

        // Decode second page for dual modes
        if self.is_dual() && self.current_page + 1 < self.total_pages {
            self.ensure_page_decoded(self.current_page + 1, ctx);
        }

        // Prefetch ahead
        for i in 1..=self.config.cache.prepage_forward {
            let p = self.current_page + i;
            if p < self.total_pages {
                self.ensure_page_decoded(p, ctx);
            }
        }

        // Record in history
        if let Some(path) = &self.source_path {
            self.history
                .record(path.clone(), self.current_page, self.total_pages);
        }
    }

    fn is_dual(&self) -> bool {
        match self.page_mode {
            PageMode::Single => false,
            PageMode::Dual => true,
            PageMode::DualWithCover => self.current_page > 0,
        }
    }

    fn page_step(&self) -> usize {
        if self.is_dual() { 2 } else { 1 }
    }

    fn next_page(&mut self, ctx: &egui::Context) {
        let new = (self.current_page + self.page_step()).min(self.total_pages.saturating_sub(1));
        if new != self.current_page {
            self.goto_page(new, ctx);
        } else {
            // At end of archive - handle browsing mode
            self.handle_end_of_archive(ctx, true);
        }
    }

    fn prev_page(&mut self, ctx: &egui::Context) {
        let new = self.current_page.saturating_sub(self.page_step());
        if new != self.current_page {
            self.goto_page(new, ctx);
        } else {
            self.handle_end_of_archive(ctx, false);
        }
    }

    fn jump_pages(&mut self, n: isize, ctx: &egui::Context) {
        if self.total_pages == 0 {
            return;
        }
        let new = if n > 0 {
            (self.current_page + n as usize).min(self.total_pages - 1)
        } else {
            self.current_page.saturating_sub((-n) as usize)
        };
        self.goto_page(new, ctx);
    }

    fn handle_end_of_archive(&mut self, ctx: &egui::Context, forward: bool) {
        match self.browsing_mode {
            BrowsingMode::NoRepeat => {
                // Do nothing, stay at current page
            }
            BrowsingMode::Repeat => {
                if forward {
                    self.goto_page(0, ctx);
                } else {
                    let last = self.total_pages.saturating_sub(1);
                    self.goto_page(last, ctx);
                }
            }
            BrowsingMode::Continuous => {
                // Try playlist first
                if forward {
                    if self.playlist.has_next() {
                        if let Some(entry) = self.playlist.advance().cloned() {
                            self.open_path(entry.path, ctx);
                            return;
                        }
                    }
                } else if self.playlist.has_prev() {
                    if let Some(entry) = self.playlist.go_back().cloned() {
                        self.open_path(entry.path, ctx);
                        return;
                    }
                }
                // Try sibling folder/archive
                if let Some(next) = self.find_sibling_archive(forward) {
                    self.open_path(next, ctx);
                }
            }
        }
    }

    fn find_sibling_archive(&self, forward: bool) -> Option<PathBuf> {
        let path = self.source_path.as_ref()?;
        let parent = path.parent()?;
        let mut siblings: Vec<PathBuf> = std::fs::read_dir(parent)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                if p.is_dir() {
                    return true;
                }
                match p.extension().and_then(|e| e.to_str()) {
                    Some(ext) => matches!(ext.to_lowercase().as_str(), "zip" | "cbz"),
                    None => false,
                }
            })
            .collect();
        siblings.sort_by(|a, b| {
            mm_archive::natural_sort::natural_cmp(
                &a.to_string_lossy(),
                &b.to_string_lossy(),
            )
        });

        let current_idx = siblings.iter().position(|p| p == path)?;
        if forward {
            siblings.get(current_idx + 1).cloned()
        } else {
            if current_idx > 0 {
                siblings.get(current_idx - 1).cloned()
            } else {
                None
            }
        }
    }

    fn title(&self) -> String {
        match &self.source_path {
            Some(path) => {
                let display_path = path.to_string_lossy();
                if self.total_pages > 0 {
                    let filter_name = match self.scale_filter {
                        ScaleFilterConfig::Nearest => "Nearest",
                        ScaleFilterConfig::Bilinear => "Bilinear",
                        ScaleFilterConfig::Bicubic => "Bicubic",
                        ScaleFilterConfig::Lanczos => "Lanczos",
                        ScaleFilterConfig::PixelAveraging => "PixelAvg",
                        ScaleFilterConfig::Halftone => "Halftone",
                        ScaleFilterConfig::PixelAvgWeakSharpen => "PixelAvg+WS",
                        ScaleFilterConfig::PixelAvgStrongSharpen => "PixelAvg+SS",
                    };
                    format!(
                        "{} : {} [{}/{}]",
                        display_path,
                        filter_name,
                        self.current_page + 1,
                        self.total_pages,
                    )
                } else {
                    format!("{display_path}")
                }
            }
            None => "MangaMeeya".to_string(),
        }
    }

    fn calc_image_size(&self, img_w: f32, img_h: f32, avail_w: f32, avail_h: f32) -> (f32, f32) {
        let z = self.zoom;
        match self.fit_mode {
            FitMode::FitScreen => {
                let max_s = if self.downscale_only { 1.0 } else { f32::MAX };
                let s = (avail_w / img_w).min(avail_h / img_h).min(max_s) * z;
                (img_w * s, img_h * s)
            }
            FitMode::FitWidth => {
                let s = (avail_w / img_w) * z;
                let s = if self.downscale_only { s.min(1.0 * z) } else { s };
                (img_w * s, img_h * s)
            }
            FitMode::FitHeight => {
                let s = (avail_h / img_h) * z;
                let s = if self.downscale_only { s.min(1.0 * z) } else { s };
                (img_w * s, img_h * s)
            }
            FitMode::Original => (img_w * z, img_h * z),
            FitMode::FitWindowToImage => {
                // In this mode we don't scale the image, we resize the window
                (img_w * z, img_h * z)
            }
            FitMode::FitWindow2Pages => {
                // Scale to fit two pages side by side
                let half_w = avail_w / 2.0 - 2.0;
                let s = (half_w / img_w).min(avail_h / img_h) * z;
                let s = if self.downscale_only { s.min(1.0 * z) } else { s };
                (img_w * s, img_h * s)
            }
            FitMode::ManualScale(factor) => {
                let s = factor * z;
                (img_w * s, img_h * s)
            }
        }
    }

    fn save_state(&self) {
        let dir = data_dir();
        let _ = self.config.save(&dir.join("config.toml"));
        let _ = self.bookmarks.save(&dir.join("bookmarks.json"));
        let _ = self.history.save(&dir.join("history.json"));
        let _ = self.playlist.save(&dir.join("playlist.json"));
    }

    fn toggle_bookmark(&mut self) {
        if let Some(path) = &self.source_path {
            if self.bookmarks.is_bookmarked(path, self.current_page) {
                self.bookmarks.remove(path, self.current_page);
            } else {
                self.bookmarks.add(path.clone(), self.current_page, None);
            }
        }
    }

    fn entry_names(&self) -> Vec<String> {
        self.archive
            .as_ref()
            .map(|a| a.entry_names())
            .unwrap_or_default()
    }
}

fn to_rgba_pixels(img: &mm_image::ImageBuffer) -> Vec<egui::Color32> {
    let n = img.width as usize * img.height as usize;
    let mut out = Vec::with_capacity(n);
    match img.color_space {
        ColorSpace::Rgb8 => {
            for i in 0..n {
                let o = i * 3;
                out.push(egui::Color32::from_rgb(
                    img.data[o],
                    img.data[o + 1],
                    img.data[o + 2],
                ));
            }
        }
        ColorSpace::Rgba8 => {
            for i in 0..n {
                let o = i * 4;
                out.push(egui::Color32::from_rgba_premultiplied(
                    img.data[o],
                    img.data[o + 1],
                    img.data[o + 2],
                    img.data[o + 3],
                ));
            }
        }
        ColorSpace::Grayscale => {
            for &v in &img.data[..n] {
                out.push(egui::Color32::from_rgb(v, v, v));
            }
        }
    }
    out
}

fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("mangameeya")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("mangameeya")
    }
}

// ============================================================
// eframe::App implementation
// ============================================================

impl eframe::App for App {
    fn on_exit(&mut self) {
        self.save_state();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.scroll.animate(dt);

        // Slideshow auto-advance
        if self.slideshow.tick(dt) {
            self.next_page(ctx);
        }

        if self.scroll.is_animating() || self.slideshow.active {
            ctx.request_repaint();
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));

        // --- Keyboard input ---
        self.handle_keyboard(ctx);

        // --- Dropped files ---
        let dropped = ctx.input(|i| i.raw.dropped_files.first().and_then(|d| d.path.clone()));
        if let Some(path) = dropped {
            self.open_path(path, ctx);
        }

        // --- Dialogs / Windows ---
        self.draw_dialogs(ctx);

        // --- Side panels ---
        self.draw_side_panels(ctx);

        // --- Menu bar ---
        self.draw_menu_bar(ctx);

        // --- Bottom toolbar + page slider ---
        self.draw_bottom_bar(ctx);

        // --- Main content area ---
        if self.show_thumbnails {
            self.draw_thumbnail_view(ctx);
        } else {
            self.draw_main_view(ctx);
        }
    }
}

// ============================================================
// Keyboard handling
// ============================================================

impl App {
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let mut action = None;
        ctx.input(|i| {
            // Navigation
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown) {
                action = Some(Act::NextPage);
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp) {
                action = Some(Act::PrevPage);
            }
            if i.key_pressed(egui::Key::Home) {
                action = Some(Act::FirstPage);
            }
            if i.key_pressed(egui::Key::End) {
                action = Some(Act::LastPage);
            }
            if i.key_pressed(egui::Key::Space) {
                action = Some(Act::NextPage);
            }
            if i.key_pressed(egui::Key::Backspace) {
                action = Some(Act::PrevPage);
            }
            // View modes
            if i.key_pressed(egui::Key::D) {
                action = Some(Act::CyclePageMode);
            }
            if i.key_pressed(egui::Key::R) {
                action = Some(Act::ToggleReadingDir);
            }
            if i.key_pressed(egui::Key::F11) || i.key_pressed(egui::Key::Enter) {
                action = Some(Act::ToggleFullscreen);
            }
            if i.key_pressed(egui::Key::Escape) {
                action = Some(Act::ExitFullscreen);
            }
            // File
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                action = Some(Act::OpenFile);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::G) {
                action = Some(Act::GoToPage);
            }
            // Zoom
            if i.key_pressed(egui::Key::Minus) {
                action = Some(Act::ZoomOut);
            }
            if i.key_pressed(egui::Key::Plus)
                || (i.modifiers.shift && i.key_pressed(egui::Key::Equals))
            {
                action = Some(Act::ZoomIn);
            }
            if i.key_pressed(egui::Key::Num0) {
                action = Some(Act::ZoomReset);
            }
            // Scroll
            if i.key_pressed(egui::Key::ArrowUp) && self.scroll.can_scroll_vertically() {
                action = Some(Act::ScrollUp);
            }
            if i.key_pressed(egui::Key::ArrowDown) && self.scroll.can_scroll_vertically() {
                action = Some(Act::ScrollDown);
            }
            // Features
            if i.key_pressed(egui::Key::B) {
                action = Some(Act::ToggleBookmark);
            }
            if i.key_pressed(egui::Key::L) {
                action = Some(Act::ToggleLoupe);
            }
            if i.key_pressed(egui::Key::P) {
                action = Some(Act::ToggleSlideshow);
            }
            if i.key_pressed(egui::Key::T) {
                action = Some(Act::ToggleThumbnails);
            }
        });

        match action {
            Some(Act::NextPage) => {
                self.slideshow.on_user_input();
                self.next_page(ctx);
            }
            Some(Act::PrevPage) => {
                self.slideshow.on_user_input();
                self.prev_page(ctx);
            }
            Some(Act::FirstPage) => self.goto_page(0, ctx),
            Some(Act::LastPage) => {
                let last = self.total_pages.saturating_sub(1);
                self.goto_page(last, ctx);
            }
            Some(Act::CyclePageMode) => {
                self.page_mode = match self.page_mode {
                    PageMode::Single => PageMode::Dual,
                    PageMode::Dual => PageMode::DualWithCover,
                    PageMode::DualWithCover => PageMode::Single,
                };
                self.goto_page(self.current_page, ctx);
            }
            Some(Act::ToggleReadingDir) => {
                self.reading_dir = match self.reading_dir {
                    ReadingDir::Ltr => ReadingDir::Rtl,
                    ReadingDir::Rtl => ReadingDir::Ltr,
                };
            }
            Some(Act::ToggleFullscreen) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            Some(Act::ExitFullscreen) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
            Some(Act::OpenFile) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Archives", &["zip", "cbz"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open_path(path, ctx);
                }
            }
            Some(Act::GoToPage) => {
                self.show_go_to_page = true;
                self.go_to_page_input = (self.current_page + 1).to_string();
            }
            Some(Act::ZoomIn) => self.zoom = (self.zoom * 1.2).min(10.0),
            Some(Act::ZoomOut) => self.zoom = (self.zoom / 1.2).max(0.1),
            Some(Act::ZoomReset) => self.zoom = 1.0,
            Some(Act::ScrollUp) => {
                self.scroll
                    .scroll_y(self.config.scrolling.keyboard_v_speed as f32);
            }
            Some(Act::ScrollDown) => {
                self.scroll
                    .scroll_y(-(self.config.scrolling.keyboard_v_speed as f32));
            }
            Some(Act::ToggleBookmark) => self.toggle_bookmark(),
            Some(Act::ToggleLoupe) => self.loupe.toggle(),
            Some(Act::ToggleSlideshow) => self.slideshow.toggle(),
            Some(Act::ToggleThumbnails) => self.show_thumbnails = !self.show_thumbnails,
            None => {}
        }
    }
}

// ============================================================
// Menu bar
// ============================================================

impl App {
    fn draw_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.menu_file(ui, ctx);
                self.menu_view(ui, ctx);
                self.menu_scaling(ui, ctx);
                self.menu_jump(ui, ctx);
                self.menu_booklist(ui, ctx);
                self.menu_tools(ui, ctx);
                self.menu_help(ui, ctx);

                // Right-side status: image dims + page + bookmark + slideshow
                if self.total_pages > 0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.slideshow.active {
                            ui.label(if self.slideshow.is_paused() {
                                "||"
                            } else {
                                ">"
                            });
                        }
                        let bookmarked = self
                            .source_path
                            .as_ref()
                            .is_some_and(|p| self.bookmarks.is_bookmarked(p, self.current_page));
                        if bookmarked {
                            ui.label("*");
                        }
                        ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                        if let Some((w, h)) = self.current_image_dims {
                            ui.label(format!("{} x {}", w, h));
                        }
                    });
                }
            });
        });
    }

    fn menu_file(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button("File (F)", |ui| {
            if ui.button("Open File (O)...").clicked() {
                ui.close_menu();
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Archives", &["zip", "cbz"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open_path(p, ctx);
                }
            }
            if ui.button("Open Folder (D)...").clicked() {
                ui.close_menu();
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.open_path(p, ctx);
                }
            }
            ui.separator();
            if ui.button("Close (C)").clicked() {
                ui.close_menu();
                self.archive = None;
                self.source_path = None;
                self.total_pages = 0;
                self.current_page = 0;
                self.page_textures.clear();
                self.error_message = None;
            }
            if ui.button("Refresh All (N)").clicked() {
                ui.close_menu();
                if self.source_path.is_some() {
                    let page = self.current_page;
                    self.page_textures = vec![None; self.total_pages];
                    self.goto_page(page, ctx);
                }
            }
            ui.separator();
            if ui.button("Import Configuration File (R)...").clicked() {
                ui.close_menu();
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Config", &["toml", "ini"])
                    .pick_file()
                {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if ext == "ini" {
                            if let Ok(cfg) = mm_config::ini_import::import_ini(&p) {
                                self.config = cfg;
                            }
                        } else if let Ok(cfg) = mm_config::Config::load(&p) {
                            self.config = cfg;
                        }
                    }
                }
            }
            if ui.button("Export Configuration File (D)...").clicked() {
                ui.close_menu();
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Config", &["toml"])
                    .save_file()
                {
                    let _ = self.config.save(&p);
                }
            }
            ui.separator();

            // Recent files from history (last 5)
            let recent: Vec<_> = self
                .history
                .entries
                .iter()
                .take(5)
                .map(|e| (e.path.clone(), e.last_page))
                .collect();
            if !recent.is_empty() {
                ui.separator();
                for (i, (path, _page)) in recent.iter().enumerate() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if ui.button(format!("{}: {}", i + 1, name)).clicked() {
                        ui.close_menu();
                        let p = path.clone();
                        self.open_path(p, ctx);
                    }
                }
                ui.separator();
            }

            if ui.button("Exit (I)").clicked() {
                self.save_state();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn menu_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button("View (V)", |ui| {
            // Page view modes
            if ui
                .radio_value(&mut self.page_mode, PageMode::Single, "1 Page View")
                .clicked()
            {
                self.goto_page(self.current_page, ctx);
                ui.close_menu();
            }
            if ui
                .radio_value(&mut self.page_mode, PageMode::Dual, "2 Page View")
                .clicked()
            {
                self.goto_page(self.current_page, ctx);
                ui.close_menu();
            }
            ui.menu_button("Page Open Direction", |ui| {
                if ui
                    .radio_value(&mut self.reading_dir, ReadingDir::Rtl, "Right to Left (Manga)")
                    .clicked()
                {
                    ui.close_menu();
                }
                if ui
                    .radio_value(&mut self.reading_dir, ReadingDir::Ltr, "Left to Right (Comics)")
                    .clicked()
                {
                    ui.close_menu();
                }
            });

            ui.separator();
            // Auto-split
            if ui
                .checkbox(&mut self.auto_split, "Single Page View (auto-split images)")
                .clicked()
            {
                self.config.viewing.auto_split = self.auto_split;
                self.rebuild_split_pages();
                ui.close_menu();
            }

            ui.separator();
            // Thumbnail views
            if ui.button("Show Thumbnails (Single Page)").clicked() {
                self.show_thumbnails = true;
                self.thumbnail_columns = 1;
                ui.close_menu();
            }
            if ui.button("Show Thumbnails (8 columns)").clicked() {
                self.show_thumbnails = true;
                self.thumbnail_columns = 8;
                ui.close_menu();
            }
            if ui.button("Show Thumbnails (Custom)").clicked() {
                self.show_thumbnails = true;
                self.thumbnail_columns = self.config.viewing.thumbnail_columns;
                ui.close_menu();
            }
            if ui.button("Thumbnails Settings...").clicked() {
                self.show_settings = true;
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Full Screen (F11)").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                ui.close_menu();
            }

            ui.separator();
            if ui.checkbox(&mut self.loupe.enabled, "Show Lens Tool (L)").clicked() {
                ui.close_menu();
            }
            ui.checkbox(&mut self.show_toolbar_icons, "Show Tool Bar");
            ui.checkbox(&mut self.show_page_bar, "Show Page Bar");
            if ui.button("Show Bookmarks Sidebar").clicked() {
                self.show_bookmarks = true;
                self.show_history = false;
                self.show_playlist = false;
                ui.close_menu();
            }
        });
    }

    fn menu_scaling(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.menu_button("Scaling (S)", |ui| {
            // Fit modes
            for (mode, label) in [
                (FitMode::FitWidth, "Fit Width"),
                (FitMode::FitScreen, "Fit To Window"),
                (FitMode::Original, "Original 100% Size"),
                (FitMode::FitHeight, "Fit To Height"),
                (FitMode::FitWindowToImage, "Fit Window To Image"),
                (FitMode::FitWindow2Pages, "Fit To Window (2 Pages)"),
            ] {
                let selected = std::mem::discriminant(&self.fit_mode) == std::mem::discriminant(&mode);
                if ui.radio(selected, label).clicked() {
                    self.fit_mode = mode;
                    self.scroll.reset();
                    ui.close_menu();
                }
            }
            if ui.button("Manually Adjust Size...").clicked() {
                // Simple: set to current zoom as manual
                self.fit_mode = FitMode::ManualScale(self.zoom);
                ui.close_menu();
            }

            ui.separator();
            ui.checkbox(&mut self.downscale_only, "Use Downscale-Resizing Only");

            ui.separator();
            // Scale filter selection
            for (filter, label) in [
                (ScaleFilterConfig::Halftone, "HALFTONE Filter (Z)"),
                (ScaleFilterConfig::PixelAveraging, "Pixel Averaging Algorithm (X)"),
                (ScaleFilterConfig::Lanczos, "Lanczos Filter (C)"),
                (ScaleFilterConfig::Bicubic, "Bicubic Interpolation (V)"),
                (ScaleFilterConfig::Bilinear, "Bilinear Interpolation (B)"),
                (
                    ScaleFilterConfig::PixelAvgWeakSharpen,
                    "Pixel Averaging + Weak Sharpening (N)",
                ),
                (
                    ScaleFilterConfig::PixelAvgStrongSharpen,
                    "Pixel Averaging + Strong Sharpening (M)",
                ),
            ] {
                if ui
                    .radio_value(&mut self.scale_filter, filter, label)
                    .clicked()
                {
                    self.config.image.scale_filter = filter;
                    // Clear textures to re-decode with new filter
                    // (In a real impl we'd re-filter, but for now force redecode)
                    ui.close_menu();
                }
            }
        });
    }

    fn menu_jump(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button("Jump (J)", |ui| {
            ui.label("Next");
            for (n, label) in [(1, "1 Page"), (10, "10 Pages"), (20, "20 Pages"), (50, "50 Pages")]
            {
                if ui.button(format!("  {label}")).clicked() {
                    self.jump_pages(n, ctx);
                    ui.close_menu();
                }
            }
            if ui.button("  Last Page").clicked() {
                let last = self.total_pages.saturating_sub(1);
                self.goto_page(last, ctx);
                ui.close_menu();
            }
            if ui.button("  Next Sub-Folder").clicked() {
                if let Some(next) = self.find_sibling_archive(true) {
                    self.open_path(next, ctx);
                }
                ui.close_menu();
            }
            if ui.button("  Next Folder").clicked() {
                if let Some(next) = self.find_sibling_archive(true) {
                    self.open_path(next, ctx);
                }
                ui.close_menu();
            }

            ui.separator();
            ui.label("Back");
            for (n, label) in [(1, "1 Page"), (10, "10 Pages"), (20, "20 Pages"), (50, "50 Pages")]
            {
                if ui.button(format!("  {label}")).clicked() {
                    self.jump_pages(-n, ctx);
                    ui.close_menu();
                }
            }
            if ui.button("  Start").clicked() {
                self.goto_page(0, ctx);
                ui.close_menu();
            }
            if ui.button("  Previous Sub-Folder").clicked() {
                if let Some(prev) = self.find_sibling_archive(false) {
                    self.open_path(prev, ctx);
                }
                ui.close_menu();
            }
            if ui.button("  Previous Folder").clicked() {
                if let Some(prev) = self.find_sibling_archive(false) {
                    self.open_path(prev, ctx);
                }
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Up One Level (Thumbnails View)").clicked() {
                self.show_thumbnails = true;
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Go To Page...").clicked() {
                self.show_go_to_page = true;
                self.go_to_page_input = (self.current_page + 1).to_string();
                ui.close_menu();
            }
            if ui.button("Show File Index").clicked() {
                self.show_file_index = !self.show_file_index;
                ui.close_menu();
            }

            ui.separator();
            for (mode, label) in [
                (BrowsingMode::NoRepeat, "No-Repeat Browsing Mode"),
                (BrowsingMode::Repeat, "Repeat Browsing Mode"),
                (
                    BrowsingMode::Continuous,
                    "Continuous Browsing Mode (opens next folder)",
                ),
            ] {
                if ui
                    .radio_value(&mut self.browsing_mode, mode, label)
                    .clicked()
                {
                    self.config.navigation.browsing_mode = mode;
                    ui.close_menu();
                }
            }
        });
    }

    fn menu_booklist(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.menu_button("Book List (B)", |ui| {
            if ui.button("Add to Bookmark List").clicked() {
                self.toggle_bookmark();
                ui.close_menu();
            }

            ui.separator();
            let mut ss = self.slideshow.active;
            if ui.checkbox(&mut ss, "Start Slideshow...").clicked() {
                self.slideshow.toggle();
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Show Bookmarks").clicked() {
                self.show_bookmarks = true;
                self.show_history = false;
                self.show_playlist = false;
                ui.close_menu();
            }
            if ui.button("Show History").clicked() {
                self.show_history = true;
                self.show_bookmarks = false;
                self.show_playlist = false;
                ui.close_menu();
            }
            if ui.button("Show Playlist").clicked() {
                self.show_playlist = true;
                self.show_bookmarks = false;
                self.show_history = false;
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Save Playlist...").clicked() {
                ui.close_menu();
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Playlist", &["json"])
                    .save_file()
                {
                    let _ = self.playlist.save(&p);
                }
            }
            if ui.button("Save History...").clicked() {
                ui.close_menu();
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("History", &["json"])
                    .save_file()
                {
                    let _ = self.history.save(&p);
                }
            }

            ui.separator();
            if ui.button("Clear History List").clicked() {
                self.history.clear();
                ui.close_menu();
            }
            if ui.button("Clear Bookmarks").clicked() {
                self.bookmarks.bookmarks.clear();
                ui.close_menu();
            }
        });
    }

    fn menu_tools(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.menu_button("Tools (T)", |ui| {
            let mut precache = self.config.cache.prepage_forward > 0;
            if ui.checkbox(&mut precache, "Pre-Cache Files (A)").clicked() {
                self.config.cache.prepage_forward = if precache { 2 } else { 0 };
                ui.close_menu();
            }

            ui.separator();
            ui.menu_button("Sort Files (S)", |ui| {
                for (mode, label) in [
                    (SortMode::Name, "By Name (Natural)"),
                    (SortMode::Date, "By Date"),
                    (SortMode::Size, "By Size"),
                    (SortMode::Extension, "By Extension"),
                ] {
                    if ui
                        .radio_value(&mut self.config.navigation.sort_files, mode, label)
                        .clicked()
                    {
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button("Sort Folders (S)", |ui| {
                for (mode, label) in [
                    (SortMode::Name, "By Name (Natural)"),
                    (SortMode::Date, "By Date"),
                    (SortMode::Size, "By Size"),
                ] {
                    if ui
                        .radio_value(&mut self.config.navigation.sort_folders, mode, label)
                        .clicked()
                    {
                        ui.close_menu();
                    }
                }
            });

            ui.separator();
            if ui.button("Options (Q)...").clicked() {
                self.show_settings = true;
                ui.close_menu();
            }
        });
    }

    fn menu_help(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.menu_button("Help (H)", |ui| {
            if ui.button("Image Information (I)...").clicked() {
                self.show_image_info = true;
                ui.close_menu();
            }
            if ui.button("Cache Information (C)...").clicked() {
                self.show_cache_info = true;
                ui.close_menu();
            }
            if ui.button("Version Information (V)...").clicked() {
                self.show_about = true;
                ui.close_menu();
            }
        });
    }
}

// ============================================================
// Bottom toolbar + page slider
// ============================================================

impl App {
    fn draw_bottom_bar(&mut self, ctx: &egui::Context) {
        if self.total_pages <= 1 && !self.show_toolbar_icons {
            return;
        }

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            // Row 1: Toolbar icons
            if self.show_toolbar_icons {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    // Open
                    if ui.small_button("Open").clicked() {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("Archives", &["zip", "cbz"])
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            self.open_path(p, ctx);
                        }
                    }
                    ui.separator();
                    // Navigation buttons
                    if ui.small_button("|<").clicked() {
                        self.goto_page(0, ctx);
                    }
                    if ui.small_button("<<").clicked() {
                        self.jump_pages(-10, ctx);
                    }
                    if ui.small_button("<").clicked() {
                        self.prev_page(ctx);
                    }
                    if ui.small_button(">").clicked() {
                        self.next_page(ctx);
                    }
                    if ui.small_button(">>").clicked() {
                        self.jump_pages(10, ctx);
                    }
                    if ui.small_button(">|").clicked() {
                        let last = self.total_pages.saturating_sub(1);
                        self.goto_page(last, ctx);
                    }
                    ui.separator();
                    // View toggles
                    if ui.small_button(if self.page_mode == PageMode::Single { "[1]" } else { "[2]" }).clicked() {
                        self.page_mode = match self.page_mode {
                            PageMode::Single => PageMode::Dual,
                            _ => PageMode::Single,
                        };
                        self.goto_page(self.current_page, ctx);
                    }
                    if ui.small_button(if self.reading_dir == ReadingDir::Rtl { "RTL" } else { "LTR" }).clicked() {
                        self.reading_dir = match self.reading_dir {
                            ReadingDir::Ltr => ReadingDir::Rtl,
                            ReadingDir::Rtl => ReadingDir::Ltr,
                        };
                    }
                    ui.separator();
                    // Fit modes
                    if ui.small_button("Fit").clicked() {
                        self.fit_mode = FitMode::FitScreen;
                        self.scroll.reset();
                    }
                    if ui.small_button("W").clicked() {
                        self.fit_mode = FitMode::FitWidth;
                        self.scroll.reset();
                    }
                    if ui.small_button("1:1").clicked() {
                        self.fit_mode = FitMode::Original;
                        self.scroll.reset();
                    }
                    ui.separator();
                    // Zoom
                    if ui.small_button("+").clicked() {
                        self.zoom = (self.zoom * 1.2).min(10.0);
                    }
                    if ui.small_button("-").clicked() {
                        self.zoom = (self.zoom / 1.2).max(0.1);
                    }
                    ui.separator();
                    // Features
                    if ui.small_button(if self.loupe.enabled { "Lens*" } else { "Lens" }).clicked() {
                        self.loupe.toggle();
                    }
                    if ui.small_button("BM").clicked() {
                        self.toggle_bookmark();
                    }
                    if ui.small_button(if self.show_thumbnails { "Thumb*" } else { "Thumb" }).clicked() {
                        self.show_thumbnails = !self.show_thumbnails;
                    }
                });
            }

            // Row 2: Full-width page slider
            if self.show_page_bar && self.total_pages > 1 {
                ui.horizontal(|ui| {
                    ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                    let mut pf = self.current_page as f32;
                    let slider =
                        egui::Slider::new(&mut pf, 0.0..=(self.total_pages as f32 - 1.0))
                            .show_value(false);
                    let resp = ui.add_sized(
                        egui::vec2(ui.available_width() - 5.0, ui.spacing().interact_size.y),
                        slider,
                    );
                    if resp.changed() {
                        self.goto_page(pf as usize, ctx);
                    }
                });
            }
        });
    }
}

// ============================================================
// Dialogs / popup windows
// ============================================================

impl App {
    fn draw_dialogs(&mut self, ctx: &egui::Context) {
        // Go To Page dialog
        if self.show_go_to_page {
            let mut open = true;
            egui::Window::new("Go To Page")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Page (1-{}):", self.total_pages));
                        let resp = ui.text_edit_singleline(&mut self.go_to_page_input);
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Ok(page) = self.go_to_page_input.parse::<usize>() {
                                let target = page.saturating_sub(1).min(self.total_pages.saturating_sub(1));
                                self.goto_page(target, ctx);
                                self.show_go_to_page = false;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Go").clicked() {
                            if let Ok(page) = self.go_to_page_input.parse::<usize>() {
                                let target = page.saturating_sub(1).min(self.total_pages.saturating_sub(1));
                                self.goto_page(target, ctx);
                                self.show_go_to_page = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_go_to_page = false;
                        }
                    });
                });
            if !open {
                self.show_go_to_page = false;
            }
        }

        // Image Information dialog
        if self.show_image_info {
            let mut open = true;
            egui::Window::new("Image Information")
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(path) = &self.source_path {
                        ui.label(format!("File: {}", path.display()));
                    }
                    ui.label(format!(
                        "Page: {}/{}",
                        self.current_page + 1,
                        self.total_pages
                    ));
                    if let Some((w, h)) = self.current_image_dims {
                        ui.label(format!("Dimensions: {} x {} pixels", w, h));
                        ui.label(format!("Aspect Ratio: {:.2}", w as f32 / h as f32));
                    }
                    if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
                        let sz = tex.size_vec2();
                        ui.label(format!("Display Size: {:.0} x {:.0}", sz.x, sz.y));
                    }
                    if let Some(archive) = &self.archive {
                        let names = archive.entry_names();
                        if let Some(name) = names.get(self.current_page) {
                            ui.label(format!("Entry: {name}"));
                        }
                    }
                });
            if !open {
                self.show_image_info = false;
            }
        }

        // Cache Information dialog
        if self.show_cache_info {
            let mut open = true;
            egui::Window::new("Cache Information")
                .open(&mut open)
                .show(ctx, |ui| {
                    let decoded = self.page_textures.iter().filter(|t| t.is_some()).count();
                    ui.label(format!("Decoded Pages: {} / {}", decoded, self.total_pages));
                    ui.label(format!(
                        "Prefetch Forward: {}",
                        self.config.cache.prepage_forward
                    ));
                    ui.label(format!("Max Cache Pages: {}", self.config.cache.cache_num));
                    ui.label(format!(
                        "Cache Size Limit: {} MB",
                        self.config.cache.file_cache_size_mb
                    ));
                    let thumb_decoded = self
                        .thumbnail_textures
                        .iter()
                        .filter(|t| t.is_some())
                        .count();
                    ui.label(format!("Thumbnail Cache: {} decoded", thumb_decoded));
                });
            if !open {
                self.show_cache_info = false;
            }
        }

        // About dialog
        if self.show_about {
            let mut open = true;
            egui::Window::new("About MangaMeeya")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.heading("MangaMeeya");
                    ui.label("Rust rewrite of the classic manga/comic viewer");
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.label("License: MIT");
                    ui.separator();
                    ui.label("Keyboard shortcuts:");
                    ui.label("  Right/PgDn/Space: Next page");
                    ui.label("  Left/PgUp/Bksp: Prev page");
                    ui.label("  D: Cycle page mode");
                    ui.label("  R: Toggle RTL/LTR");
                    ui.label("  B: Toggle bookmark");
                    ui.label("  L: Toggle loupe");
                    ui.label("  P: Toggle slideshow");
                    ui.label("  T: Toggle thumbnails");
                    ui.label("  Ctrl+G: Go to page");
                    ui.label("  F11: Fullscreen");
                    ui.label("  +/-/0: Zoom");
                });
            if !open {
                self.show_about = false;
            }
        }

        // Settings window
        if self.show_settings {
            let mut open = true;
            egui::Window::new("Settings")
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::CollapsingHeader::new("Viewing")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Page mode:");
                                ui.radio_value(&mut self.page_mode, PageMode::Single, "Single");
                                ui.radio_value(&mut self.page_mode, PageMode::Dual, "Dual");
                                ui.radio_value(
                                    &mut self.page_mode,
                                    PageMode::DualWithCover,
                                    "Dual+Cover",
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Reading direction:");
                                ui.radio_value(
                                    &mut self.reading_dir,
                                    ReadingDir::Rtl,
                                    "RTL (manga)",
                                );
                                ui.radio_value(
                                    &mut self.reading_dir,
                                    ReadingDir::Ltr,
                                    "LTR (comics)",
                                );
                            });
                            ui.checkbox(&mut self.auto_split, "Auto-split wide images");
                            ui.add(
                                egui::Slider::new(&mut self.auto_split_threshold, 1.0..=3.0)
                                    .text("Split threshold (W/H ratio)"),
                            );
                            ui.checkbox(&mut self.downscale_only, "Downscale only (never upscale)");
                        });

                    egui::CollapsingHeader::new("Thumbnails")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Slider::new(&mut self.config.viewing.thumbnail_columns, 1..=16)
                                    .text("Columns"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.config.viewing.thumbnail_size, 50..=500)
                                    .text("Thumbnail size"),
                            );
                        });

                    egui::CollapsingHeader::new("Scrolling")
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut smooth = self.scroll.smooth;
                            if ui.checkbox(&mut smooth, "Smooth scrolling").changed() {
                                self.scroll.smooth = smooth;
                                self.config.scrolling.smooth_scroll = smooth;
                            }
                            ui.add(
                                egui::Slider::new(
                                    &mut self.config.scrolling.smooth_scroll_speed,
                                    100..=5000,
                                )
                                .text("Speed"),
                            );
                            self.scroll.speed = self.config.scrolling.smooth_scroll_speed as f32;
                        });

                    egui::CollapsingHeader::new("Loupe")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Slider::new(&mut self.loupe.radius, 30.0..=200.0)
                                    .text("Radius"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.loupe.magnification, 1.5..=8.0)
                                    .text("Magnification"),
                            );
                        });

                    egui::CollapsingHeader::new("Slideshow")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Slider::new(&mut self.slideshow.interval_secs, 1.0..=30.0)
                                    .text("Interval (sec)"),
                            );
                        });

                    egui::CollapsingHeader::new("Cache")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Slider::new(&mut self.config.cache.prepage_forward, 0..=10)
                                    .text("Prefetch forward"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.config.cache.cache_num, 10..=1000)
                                    .text("Max cached pages"),
                            );
                        });

                    egui::CollapsingHeader::new("Navigation")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Browsing mode:");
                                ui.radio_value(
                                    &mut self.browsing_mode,
                                    BrowsingMode::NoRepeat,
                                    "No Repeat",
                                );
                                ui.radio_value(
                                    &mut self.browsing_mode,
                                    BrowsingMode::Repeat,
                                    "Repeat",
                                );
                                ui.radio_value(
                                    &mut self.browsing_mode,
                                    BrowsingMode::Continuous,
                                    "Continuous",
                                );
                            });
                            self.config.navigation.browsing_mode = self.browsing_mode;
                        });

                    ui.separator();
                    if ui.button("Save config").clicked() {
                        self.config.viewing.auto_split = self.auto_split;
                        self.config.viewing.auto_split_threshold = self.auto_split_threshold;
                        self.config.viewing.downscale_only = self.downscale_only;
                        self.save_state();
                    }
                });
            if !open {
                self.show_settings = false;
            }
        }
    }
}

// ============================================================
// Side panels
// ============================================================

impl App {
    fn draw_side_panels(&mut self, ctx: &egui::Context) {
        // File index panel (left side)
        if self.show_file_index {
            egui::SidePanel::left("file_index_panel")
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("File Index");
                        if ui.button("X").clicked() {
                            self.show_file_index = false;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let names = self.entry_names();
                        for (i, name) in names.iter().enumerate() {
                            let label = if i == self.current_page {
                                format!("> {}: {name}", i + 1)
                            } else {
                                format!("  {}: {name}", i + 1)
                            };
                            if ui.button(&label).clicked() {
                                self.goto_page(i, ctx);
                            }
                        }
                    });
                });
        }

        // Right side panels (bookmarks/history/playlist)
        if self.show_bookmarks || self.show_history || self.show_playlist {
            egui::SidePanel::right("library_panel")
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.show_bookmarks, true, "Bookmarks");
                        ui.selectable_value(&mut self.show_history, true, "History");
                        ui.selectable_value(&mut self.show_playlist, true, "Playlist");
                        if ui.button("X").clicked() {
                            self.show_bookmarks = false;
                            self.show_history = false;
                            self.show_playlist = false;
                        }
                    });
                    ui.separator();

                    if self.show_bookmarks {
                        self.draw_bookmarks_panel(ui, ctx);
                    } else if self.show_history {
                        self.draw_history_panel(ui, ctx);
                    } else if self.show_playlist {
                        self.draw_playlist_panel(ui, ctx);
                    }
                });
        }
    }

    fn draw_bookmarks_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if ui.button("Add bookmark for current page").clicked() {
            self.toggle_bookmark();
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut goto = None;
            for bm in &self.bookmarks.bookmarks {
                let name = bm
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let default_label = format!("Page {}", bm.page + 1);
                let label = bm.label.as_deref().unwrap_or(&default_label);
                if ui.button(format!("{name} - {label}")).clicked() {
                    goto = Some((bm.path.clone(), bm.page));
                }
            }
            if let Some((path, page)) = goto {
                if self.source_path.as_deref() != Some(&path) {
                    self.open_path(path, ctx);
                }
                self.goto_page(page, ctx);
            }
        });
    }

    fn draw_history_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if ui.button("Clear history").clicked() {
            self.history.clear();
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut goto = None;
            for entry in &self.history.entries {
                let name = entry
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let text = format!(
                    "{name} (page {}/{})",
                    entry.last_page + 1,
                    entry.total_pages
                );
                if ui.button(&text).clicked() {
                    goto = Some((entry.path.clone(), entry.last_page));
                }
            }
            if let Some((path, page)) = goto {
                self.open_path(path, ctx);
                self.goto_page(page, ctx);
            }
        });
    }

    fn draw_playlist_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if ui.button("Add file").clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("Archives", &["zip", "cbz"])
                    .pick_file()
            {
                self.playlist.add(p, None);
            }
            if ui.button("Clear").clicked() {
                self.playlist = Playlist::new("Default");
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut goto = None;
            for (i, entry) in self.playlist.entries.iter().enumerate() {
                let name = entry
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let prefix = if i == self.playlist.current_index {
                    "> "
                } else {
                    "  "
                };
                if ui.button(format!("{prefix}{name}")).clicked() {
                    goto = Some((i, entry.path.clone()));
                }
            }
            if let Some((i, path)) = goto {
                self.playlist.current_index = i;
                self.open_path(path, ctx);
            }
        });
    }
}

// ============================================================
// Thumbnail grid view
// ============================================================

impl App {
    fn draw_thumbnail_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(40, 40, 40)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Thumbnails");
                    ui.label(format!(
                        "{} pages, {} columns",
                        self.total_pages, self.thumbnail_columns
                    ));
                    if ui.button("Back to viewer").clicked() {
                        self.show_thumbnails = false;
                    }
                    ui.separator();
                    ui.label("Columns:");
                    for n in [1, 4, 8] {
                        if ui
                            .selectable_label(self.thumbnail_columns == n, format!("{n}"))
                            .clicked()
                        {
                            self.thumbnail_columns = n;
                        }
                    }
                });
                ui.separator();

                let cols = self.thumbnail_columns.max(1) as usize;
                let thumb_size = self.config.viewing.thumbnail_size as f32;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let total = self.total_pages;
                    let rows = (total + cols - 1) / cols;

                    for row in 0..rows {
                        ui.horizontal(|ui| {
                            for col in 0..cols {
                                let page = row * cols + col;
                                if page >= total {
                                    break;
                                }

                                // Ensure decoded
                                self.ensure_page_decoded(page, ctx);

                                let is_current = page == self.current_page;
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(thumb_size, thumb_size),
                                    egui::Sense::click(),
                                );

                                // Draw background
                                if is_current {
                                    ui.painter().rect_filled(
                                        rect,
                                        2.0,
                                        egui::Color32::from_rgb(60, 80, 120),
                                    );
                                }

                                // Draw thumbnail
                                if let Some(Some(tex)) = self.page_textures.get(page) {
                                    let sz = tex.size_vec2();
                                    let scale = (thumb_size / sz.x).min(thumb_size / sz.y);
                                    let tw = sz.x * scale;
                                    let th = sz.y * scale;
                                    let offset_x = (thumb_size - tw) / 2.0;
                                    let offset_y = (thumb_size - th) / 2.0;
                                    let img_rect = egui::Rect::from_min_size(
                                        rect.min + egui::vec2(offset_x, offset_y),
                                        egui::vec2(tw, th),
                                    );
                                    let uv = egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    );
                                    ui.painter().image(
                                        tex.id(),
                                        img_rect,
                                        uv,
                                        egui::Color32::WHITE,
                                    );
                                } else {
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        format!("{}", page + 1),
                                        egui::FontId::default(),
                                        egui::Color32::GRAY,
                                    );
                                }

                                // Page number label
                                ui.painter().text(
                                    rect.min + egui::vec2(2.0, 2.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{}", page + 1),
                                    egui::FontId::proportional(10.0),
                                    if is_current {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::LIGHT_GRAY
                                    },
                                );

                                // Border
                                if is_current {
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                                        egui::StrokeKind::Outside,
                                    );
                                }

                                if response.clicked() {
                                    self.show_thumbnails = false;
                                    self.goto_page(page, ctx);
                                }
                            }
                        });
                    }
                });
            });
    }
}

// ============================================================
// Main image view
// ============================================================

impl App {
    fn draw_main_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                if self.archive.is_none() {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(ui.available_height() / 3.0);
                            ui.heading("MangaMeeya");
                            ui.add_space(10.0);
                            ui.label("Drop a ZIP/CBZ file or folder here");
                            ui.label("or use File > Open (Ctrl+O)");
                        });
                    });
                    return;
                }
                if let Some(err) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, err);
                    return;
                }

                let avail = ui.available_size();

                // FitWindowToImage: resize window to match image
                if self.fit_mode == FitMode::FitWindowToImage {
                    if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
                        let sz = tex.size_vec2();
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(sz));
                    }
                }

                // Compute display size and update scroll dimensions
                if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
                    let img_sz = tex.size_vec2();

                    // Update image dims for info display
                    self.current_image_dims = Some((img_sz.x as u32, img_sz.y as u32));

                    let (dw, dh) = if self.is_dual() {
                        self.calc_image_size(img_sz.x, img_sz.y, avail.x / 2.0 - 2.0, avail.y)
                    } else {
                        self.calc_image_size(img_sz.x, img_sz.y, avail.x, avail.y)
                    };
                    let total_w = if self.is_dual() { dw * 2.0 + 4.0 } else { dw };
                    self.scroll.set_dimensions(total_w, dh, avail.x, avail.y);
                }

                // Mouse wheel
                let wheel = ui.input(|i| i.smooth_scroll_delta);
                if wheel.y.abs() > 1.0 {
                    self.slideshow.on_user_input();
                    if self.scroll.can_scroll_vertically() {
                        let old_top = self.scroll.at_top();
                        let old_bot = self.scroll.at_bottom();
                        self.scroll.scroll_y(wheel.y);
                        if wheel.y > 0.0 && old_top && self.scroll.at_top() {
                            self.prev_page(ctx);
                        } else if wheel.y < 0.0 && old_bot && self.scroll.at_bottom() {
                            self.next_page(ctx);
                        }
                        ctx.request_repaint();
                    } else if wheel.y > 0.0 {
                        self.prev_page(ctx);
                    } else {
                        self.next_page(ctx);
                    }
                }

                // Mouse drag
                let resp = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());
                if resp.dragged() {
                    self.slideshow.on_user_input();
                    let d = resp.drag_delta();
                    self.scroll.scroll_x(d.x);
                    self.scroll.scroll_y(d.y);
                    ctx.request_repaint();
                }
                if resp.double_clicked() {
                    self.scroll.reset();
                    self.zoom = 1.0;
                }

                // Click zones for page navigation
                if resp.clicked()
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    let rx = (pos.x - resp.rect.left()) / resp.rect.width();
                    let (prev_zone, next_zone) = if self.reading_dir == ReadingDir::Rtl {
                        (0.67, 0.33)
                    } else {
                        (0.33, 0.67)
                    };
                    if self.reading_dir == ReadingDir::Rtl {
                        if rx > prev_zone {
                            self.prev_page(ctx);
                        } else if rx < next_zone {
                            self.next_page(ctx);
                        }
                    } else {
                        if rx < prev_zone {
                            self.prev_page(ctx);
                        } else if rx > next_zone {
                            self.next_page(ctx);
                        }
                    }
                }

                // Update loupe cursor
                if let Some(pos) = resp.hover_pos() {
                    self.loupe.update_cursor(pos.x, pos.y);
                }

                // --- Draw pages ---
                let origin = ui.max_rect().min;
                let (px, py) = self.scroll.image_position();
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

                if self.is_dual() {
                    self.draw_dual(ui, origin, px, py, avail, uv);
                } else if self.auto_split {
                    self.draw_split_single(ui, origin, px, py, avail, uv);
                } else {
                    self.draw_single(ui, origin, px, py, avail, uv);
                }

                // --- Draw loupe overlay ---
                if self.loupe.enabled {
                    self.draw_loupe(ui, origin, px, py, avail);
                }
            });
    }

    fn draw_single(
        &self,
        ui: &mut egui::Ui,
        origin: egui::Pos2,
        px: f32,
        py: f32,
        avail: egui::Vec2,
        uv: egui::Rect,
    ) {
        if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
            let sz = tex.size_vec2();
            let (dw, dh) = self.calc_image_size(sz.x, sz.y, avail.x, avail.y);
            let rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + px, origin.y + py),
                egui::vec2(dw, dh),
            );
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        } else {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
        }
    }

    fn draw_split_single(
        &self,
        ui: &mut egui::Ui,
        origin: egui::Pos2,
        px: f32,
        py: f32,
        avail: egui::Vec2,
        _uv: egui::Rect,
    ) {
        // Get the split half for current virtual page
        let (real_page, half) = self
            .split_pages
            .get(self.virtual_page)
            .copied()
            .unwrap_or((self.current_page, SplitHalf::Full));

        if let Some(Some(tex)) = self.page_textures.get(real_page) {
            let sz = tex.size_vec2();
            let (dw, dh) = self.calc_image_size(sz.x, sz.y, avail.x, avail.y);

            let uv = match half {
                SplitHalf::Full => {
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
                }
                SplitHalf::Left => {
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.5, 1.0))
                }
                SplitHalf::Right => {
                    egui::Rect::from_min_max(egui::pos2(0.5, 0.0), egui::pos2(1.0, 1.0))
                }
            };

            // For split halves, the display width should be half of the scaled image
            let display_w = match half {
                SplitHalf::Full => dw,
                _ => {
                    let half_w = sz.x / 2.0;
                    let (hw, _) = self.calc_image_size(half_w, sz.y, avail.x, avail.y);
                    hw
                }
            };

            let rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + px, origin.y + py),
                egui::vec2(display_w, dh),
            );
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        } else {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
        }
    }

    fn draw_dual(
        &self,
        ui: &mut egui::Ui,
        origin: egui::Pos2,
        px: f32,
        py: f32,
        avail: egui::Vec2,
        uv: egui::Rect,
    ) {
        let half = avail.x / 2.0 - 2.0;
        let (left_idx, right_idx) = if self.reading_dir == ReadingDir::Rtl {
            (self.current_page + 1, self.current_page)
        } else {
            (self.current_page, self.current_page + 1)
        };

        // Left page
        if let Some(Some(tex)) = self.page_textures.get(left_idx) {
            let sz = tex.size_vec2();
            let (dw, dh) = self.calc_image_size(sz.x, sz.y, half, avail.y);
            let rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + px, origin.y + py),
                egui::vec2(dw, dh),
            );
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        }

        // Right page
        if right_idx < self.total_pages
            && let Some(Some(tex)) = self.page_textures.get(right_idx)
        {
            let sz = tex.size_vec2();
            let (dw, dh) = self.calc_image_size(sz.x, sz.y, half, avail.y);
            let rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + px + half + 4.0, origin.y + py),
                egui::vec2(dw, dh),
            );
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        }
    }

    fn draw_loupe(
        &self,
        ui: &mut egui::Ui,
        origin: egui::Pos2,
        px: f32,
        py: f32,
        avail: egui::Vec2,
    ) {
        let Some(Some(tex)) = self.page_textures.get(self.current_page) else {
            return;
        };
        let sz = tex.size_vec2();
        let (dw, dh) = self.calc_image_size(sz.x, sz.y, avail.x, avail.y);

        let region = self.loupe.compute_region(&mm_core::loupe::LoupeInput {
            screen_x: self.loupe.cursor_x,
            screen_y: self.loupe.cursor_y,
            img_rect_x: origin.x + px,
            img_rect_y: origin.y + py,
            img_rect_w: dw,
            img_rect_h: dh,
        });

        if let Some(r) = region {
            let loupe_rect = egui::Rect::from_center_size(
                egui::pos2(r.screen_x, r.screen_y),
                egui::vec2(r.screen_radius * 2.0, r.screen_radius * 2.0),
            );
            let loupe_uv = egui::Rect::from_center_size(
                egui::pos2(r.uv_center_x, r.uv_center_y),
                egui::vec2(r.uv_radius_x * 2.0, r.uv_radius_y * 2.0),
            );
            ui.painter()
                .image(tex.id(), loupe_rect, loupe_uv, egui::Color32::WHITE);
            ui.painter().rect_stroke(
                loupe_rect,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(2.0, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );
        }
    }
}

#[derive(Clone, Copy)]
enum Act {
    NextPage,
    PrevPage,
    FirstPage,
    LastPage,
    CyclePageMode,
    ToggleReadingDir,
    ToggleFullscreen,
    ExitFullscreen,
    OpenFile,
    GoToPage,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ScrollUp,
    ScrollDown,
    ToggleBookmark,
    ToggleLoupe,
    ToggleSlideshow,
    ToggleThumbnails,
}
