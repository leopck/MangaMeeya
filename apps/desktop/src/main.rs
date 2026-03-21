#![windows_subsystem = "windows"]

use eframe::egui;
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

    // View state
    fit_mode: FitMode,
    page_mode: PageMode,
    reading_dir: ReadingDir,
    zoom: f32,
    scroll: ScrollState,

    // Features
    loupe: Loupe,
    slideshow: Slideshow,
    bookmarks: BookmarkStore,
    history: HistoryStore,
    playlist: Playlist,

    // UI toggles
    show_page_bar: bool,
    show_settings: bool,
    show_bookmarks: bool,
    show_history: bool,
    show_playlist: bool,

    config: mm_config::Config,
    last_frame_time: std::time::Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum FitMode {
    FitScreen,
    FitWidth,
    FitHeight,
    Original,
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

        let mut app = Self {
            archive: None,
            source_path: None,
            current_page: 0,
            total_pages: 0,
            page_textures: Vec::new(),
            error_message: None,
            fit_mode: FitMode::FitScreen,
            page_mode: PageMode::Single,
            reading_dir: ReadingDir::Rtl,
            zoom: 1.0,
            scroll,
            loupe: Loupe::new(),
            slideshow: Slideshow::new(5.0),
            bookmarks,
            history,
            playlist,
            show_page_bar: true,
            show_settings: false,
            show_bookmarks: false,
            show_history: false,
            show_playlist: false,
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
                self.goto_page(start_page, ctx);
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to open: {e}"));
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
        } else if self.playlist.has_next()
            && let Some(entry) = self.playlist.advance().cloned()
        {
            self.open_path(entry.path, ctx);
        }
    }

    fn prev_page(&mut self, ctx: &egui::Context) {
        let new = self.current_page.saturating_sub(self.page_step());
        if new != self.current_page {
            self.goto_page(new, ctx);
        } else if self.playlist.has_prev()
            && let Some(entry) = self.playlist.go_back().cloned()
        {
            self.open_path(entry.path, ctx);
        }
    }

    fn title(&self) -> String {
        match &self.source_path {
            Some(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if self.total_pages > 0 {
                    let mode = match self.page_mode {
                        PageMode::Single => "",
                        PageMode::Dual => " [Dual]",
                        PageMode::DualWithCover => " [Dual+Cover]",
                    };
                    let dir = if self.reading_dir == ReadingDir::Rtl {
                        " RTL"
                    } else {
                        ""
                    };
                    format!(
                        "MangaMeeya - {} [{}/{}{}{}]",
                        name,
                        self.current_page + 1,
                        self.total_pages,
                        mode,
                        dir
                    )
                } else {
                    format!("MangaMeeya - {name}")
                }
            }
            None => "MangaMeeya".to_string(),
        }
    }

    fn calc_image_size(&self, img_w: f32, img_h: f32, avail_w: f32, avail_h: f32) -> (f32, f32) {
        let z = self.zoom;
        match self.fit_mode {
            FitMode::FitScreen => {
                let s = (avail_w / img_w).min(avail_h / img_h).min(1.0) * z;
                (img_w * s, img_h * s)
            }
            FitMode::FitWidth => {
                let s = (avail_w / img_w) * z;
                (img_w * s, img_h * s)
            }
            FitMode::FitHeight => {
                let s = (avail_h / img_h) * z;
                (img_w * s, img_h * s)
            }
            FitMode::Original => (img_w * z, img_h * z),
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
            None => {}
        }

        // --- Dropped files ---
        let dropped = ctx.input(|i| i.raw.dropped_files.first().and_then(|d| d.path.clone()));
        if let Some(path) = dropped {
            self.open_path(path, ctx);
        }

        // --- Side panels (bookmarks/history/playlist) ---
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

        // --- Settings window ---
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

                    ui.separator();
                    if ui.button("Save config").clicked() {
                        self.save_state();
                    }
                });
            if !open {
                self.show_settings = false;
            }
        }

        // --- Menu bar ---
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open (Ctrl+O)").clicked() {
                        ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("Archives", &["zip", "cbz"])
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            self.open_path(p, ctx);
                        }
                    }
                    if ui.button("Open Folder").clicked() {
                        ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.open_path(p, ctx);
                        }
                    }
                    if ui.button("Add to Playlist").clicked() {
                        ui.close_menu();
                        if let Some(p) = &self.source_path {
                            self.playlist.add(p.clone(), None);
                        }
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        self.save_state();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.menu_button("Fit Mode", |ui| {
                        for (mode, label) in [
                            (FitMode::FitScreen, "Fit Screen"),
                            (FitMode::FitWidth, "Fit Width"),
                            (FitMode::FitHeight, "Fit Height"),
                            (FitMode::Original, "Original Size"),
                        ] {
                            if ui.radio_value(&mut self.fit_mode, mode, label).clicked() {
                                self.scroll.reset();
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Page Mode (D)", |ui| {
                        for (mode, label) in [
                            (PageMode::Single, "Single"),
                            (PageMode::Dual, "Dual"),
                            (PageMode::DualWithCover, "Dual + Cover"),
                        ] {
                            if ui.radio_value(&mut self.page_mode, mode, label).clicked() {
                                self.goto_page(self.current_page, ctx);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Reading Dir (R)", |ui| {
                        if ui
                            .radio_value(&mut self.reading_dir, ReadingDir::Rtl, "RTL (Manga)")
                            .clicked()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .radio_value(&mut self.reading_dir, ReadingDir::Ltr, "LTR (Comics)")
                            .clicked()
                        {
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.checkbox(&mut self.show_page_bar, "Page Bar");
                    if ui.checkbox(&mut self.loupe.enabled, "Loupe (L)").clicked() {
                        ui.close_menu();
                    }
                    let mut ss = self.slideshow.active;
                    if ui.checkbox(&mut ss, "Slideshow (P)").clicked() {
                        self.slideshow.toggle();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Bookmarks").clicked() {
                        self.show_bookmarks = true;
                        self.show_history = false;
                        self.show_playlist = false;
                        ui.close_menu();
                    }
                    if ui.button("History").clicked() {
                        self.show_history = true;
                        self.show_bookmarks = false;
                        self.show_playlist = false;
                        ui.close_menu();
                    }
                    if ui.button("Playlist").clicked() {
                        self.show_playlist = true;
                        self.show_bookmarks = false;
                        self.show_history = false;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Settings...").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.label("Right/PgDn/Space: Next page");
                    ui.label("Left/PgUp/Bksp: Prev page");
                    ui.label("Up/Down: Scroll");
                    ui.label("D: Cycle page mode");
                    ui.label("R: Toggle RTL/LTR");
                    ui.label("B: Toggle bookmark");
                    ui.label("L: Toggle loupe");
                    ui.label("P: Toggle slideshow");
                    ui.label("F11: Fullscreen");
                    ui.label("+/-/0: Zoom");
                });

                // Right-side status
                if self.total_pages > 0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.slideshow.active {
                            ui.label(if self.slideshow.is_paused() {
                                "▮▮"
                            } else {
                                "▶"
                            });
                        }
                        let bookmarked = self
                            .source_path
                            .as_ref()
                            .is_some_and(|p| self.bookmarks.is_bookmarked(p, self.current_page));
                        if bookmarked {
                            ui.label("★");
                        }
                        ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                    });
                }
            });
        });

        // --- Page bar ---
        if self.show_page_bar && self.total_pages > 1 {
            egui::TopBottomPanel::bottom("pagebar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("|<").clicked() {
                        self.goto_page(0, ctx);
                    }
                    if ui.button("<").clicked() {
                        self.prev_page(ctx);
                    }
                    let mut pf = self.current_page as f32;
                    let slider = egui::Slider::new(&mut pf, 0.0..=(self.total_pages as f32 - 1.0))
                        .show_value(false)
                        .text(format!("{}/{}", self.current_page + 1, self.total_pages));
                    if ui.add(slider).changed() {
                        self.goto_page(pf as usize, ctx);
                    }
                    if ui.button(">").clicked() {
                        self.next_page(ctx);
                    }
                    if ui.button(">|").clicked() {
                        self.goto_page(self.total_pages.saturating_sub(1), ctx);
                    }
                });
            });
        }

        // --- Main image area ---
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

                // Compute display size and update scroll dimensions
                if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
                    let img_sz = tex.size_vec2();
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
                    // RTL inverts click zones
                    let (prev_zone, next_zone) = if self.reading_dir == ReadingDir::Rtl {
                        (0.67, 0.33) // right=prev, left=next for manga
                    } else {
                        (0.33, 0.67) // left=prev, right=next for comics
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
                } else {
                    self.draw_single(ui, origin, px, py, avail, uv);
                }

                // --- Draw loupe overlay ---
                if self.loupe.enabled {
                    self.draw_loupe(ui, origin, px, py, avail);
                }
            });
    }
}

impl App {
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
            (self.current_page + 1, self.current_page) // RTL: right page first
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
            // Draw zoomed region
            ui.painter()
                .image(tex.id(), loupe_rect, loupe_uv, egui::Color32::WHITE);
            // Draw border
            ui.painter().rect_stroke(
                loupe_rect,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(2.0, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );
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
                    "▶ "
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
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ScrollUp,
    ScrollDown,
    ToggleBookmark,
    ToggleLoupe,
    ToggleSlideshow,
}
