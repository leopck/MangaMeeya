#![windows_subsystem = "windows"]

use eframe::egui;
use mm_core::ScrollState;
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
    archive: Option<Box<dyn mm_archive::ArchiveReader>>,
    source_path: Option<PathBuf>,
    current_page: usize,
    total_pages: usize,
    dual_page: bool,
    page_textures: Vec<Option<egui::TextureHandle>>,
    error_message: Option<String>,
    show_page_bar: bool,
    fit_mode: FitMode,
    zoom: f32,
    scroll: ScrollState,
    show_settings: bool,
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

impl App {
    fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        let config = mm_config::Config::load(&config_path()).unwrap_or_default();
        let mut scroll = ScrollState::new();
        scroll.smooth = config.scrolling.smooth_scroll;
        scroll.speed = config.scrolling.smooth_scroll_speed as f32;

        let mut app = Self {
            archive: None,
            source_path: None,
            current_page: 0,
            total_pages: 0,
            dual_page: false,
            page_textures: Vec::new(),
            error_message: None,
            show_page_bar: true,
            fit_mode: FitMode::FitScreen,
            zoom: 1.0,
            scroll,
            show_settings: false,
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
                self.current_page = 0;
                self.page_textures = vec![None; self.total_pages];
                self.source_path = Some(path);
                self.archive = Some(reader);
                self.error_message = None;
                self.scroll.reset();
                self.zoom = 1.0;
                self.ensure_page_decoded(0, ctx);
                if self.dual_page && self.total_pages > 1 {
                    self.ensure_page_decoded(1, ctx);
                }
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
        if self.dual_page && self.current_page + 1 < self.total_pages {
            self.ensure_page_decoded(self.current_page + 1, ctx);
        }
        for i in 1..=3 {
            let fwd = self.current_page + i;
            if fwd < self.total_pages {
                self.ensure_page_decoded(fwd, ctx);
            }
        }
    }

    fn next_page(&mut self, ctx: &egui::Context) {
        let step = if self.dual_page { 2 } else { 1 };
        let new = (self.current_page + step).min(self.total_pages.saturating_sub(1));
        self.goto_page(new, ctx);
    }

    fn prev_page(&mut self, ctx: &egui::Context) {
        let step = if self.dual_page { 2 } else { 1 };
        self.goto_page(self.current_page.saturating_sub(step), ctx);
    }

    fn title(&self) -> String {
        match &self.source_path {
            Some(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if self.total_pages > 0 {
                    format!(
                        "MangaMeeya - {} [{}/{}]",
                        name,
                        self.current_page + 1,
                        self.total_pages
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

fn config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("mangameeya")
            .join("config.toml")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("mangameeya")
            .join("config.toml")
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Delta time for smooth scroll animation
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.scroll.animate(dt);

        // Request repaint if smooth scroll is animating
        if self.scroll.is_animating() {
            ctx.request_repaint();
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));

        // --- Input handling ---
        let mut action = None;
        ctx.input(|i| {
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
            if i.key_pressed(egui::Key::D) {
                action = Some(Act::ToggleDual);
            }
            if i.key_pressed(egui::Key::F11) || i.key_pressed(egui::Key::Enter) {
                action = Some(Act::ToggleFullscreen);
            }
            if i.key_pressed(egui::Key::Escape) {
                action = Some(Act::ExitFullscreen);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                action = Some(Act::OpenFile);
            }
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
            // Arrow up/down for scrolling
            if i.key_pressed(egui::Key::ArrowUp) && self.scroll.can_scroll_vertically() {
                action = Some(Act::ScrollUp);
            }
            if i.key_pressed(egui::Key::ArrowDown) && self.scroll.can_scroll_vertically() {
                action = Some(Act::ScrollDown);
            }
        });

        match action {
            Some(Act::NextPage) => self.next_page(ctx),
            Some(Act::PrevPage) => self.prev_page(ctx),
            Some(Act::FirstPage) => self.goto_page(0, ctx),
            Some(Act::LastPage) => {
                let last = self.total_pages.saturating_sub(1);
                self.goto_page(last, ctx);
            }
            Some(Act::ToggleDual) => {
                self.dual_page = !self.dual_page;
                self.goto_page(self.current_page, ctx);
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
            None => {}
        }

        // --- Dropped files ---
        let dropped = ctx.input(|i| i.raw.dropped_files.first().and_then(|d| d.path.clone()));
        if let Some(path) = dropped {
            self.open_path(path, ctx);
        }

        // --- Settings window ---
        if self.show_settings {
            let mut open = true;
            egui::Window::new("Settings")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.heading("Viewing");
                    ui.checkbox(&mut self.dual_page, "Dual page mode");
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
                        .text("Scroll speed"),
                    );
                    self.scroll.speed = self.config.scrolling.smooth_scroll_speed as f32;

                    ui.heading("Cache");
                    ui.add(
                        egui::Slider::new(&mut self.config.cache.prepage_forward, 0..=10)
                            .text("Prefetch forward"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.config.cache.prepage_backward, 0..=10)
                            .text("Prefetch backward"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.config.cache.cache_num, 10..=1000)
                            .text("Max cached pages"),
                    );

                    ui.separator();
                    if ui.button("Save").clicked() {
                        let _ = self.config.save(&config_path());
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
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        let _ = self.config.save(&config_path());
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::FitScreen, "Fit Screen")
                        .clicked()
                    {
                        self.scroll.reset();
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::FitWidth, "Fit Width")
                        .clicked()
                    {
                        self.scroll.reset();
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::FitHeight, "Fit Height")
                        .clicked()
                    {
                        self.scroll.reset();
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::Original, "Original Size")
                        .clicked()
                    {
                        self.scroll.reset();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.checkbox(&mut self.dual_page, "Dual Page (D)").clicked() {
                        ui.close_menu();
                        self.goto_page(self.current_page, ctx);
                    }
                    ui.checkbox(&mut self.show_page_bar, "Page Bar");
                    ui.separator();
                    if ui.button("Settings...").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.label("Right/PgDn/Space: Next page");
                    ui.label("Left/PgUp/Bksp: Prev page");
                    ui.label("Home/End: First/Last page");
                    ui.label("Up/Down: Scroll (if image overflows)");
                    ui.label("Mouse wheel: Scroll or page turn");
                    ui.label("D: Toggle dual page");
                    ui.label("F11/Enter: Fullscreen");
                    ui.label("+/-/0: Zoom");
                    ui.label("Ctrl+O: Open file");
                    ui.label("Drag & drop files to open");
                });

                if self.total_pages > 0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "Page {}/{}",
                            self.current_page + 1,
                            self.total_pages
                        ));
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

                // Get current page texture to compute scroll dimensions
                if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
                    let img_sz = tex.size_vec2();
                    let (dw, dh) = self.calc_image_size(img_sz.x, img_sz.y, avail.x, avail.y);
                    self.scroll.set_dimensions(dw, dh, avail.x, avail.y);
                }

                // Mouse wheel: scroll if image overflows, otherwise page turn
                let wheel = ui.input(|i| i.smooth_scroll_delta);
                if wheel.y.abs() > 1.0 {
                    if self.scroll.can_scroll_vertically() {
                        // Scroll the image
                        let old_at_top = self.scroll.at_top();
                        let old_at_bottom = self.scroll.at_bottom();
                        self.scroll.scroll_y(wheel.y);

                        // Page turn at boundaries
                        if wheel.y > 0.0 && old_at_top && self.scroll.at_top() {
                            self.prev_page(ctx);
                        } else if wheel.y < 0.0 && old_at_bottom && self.scroll.at_bottom() {
                            self.next_page(ctx);
                        }
                        ctx.request_repaint();
                    } else {
                        // No scroll needed, just page turn
                        if wheel.y > 0.0 {
                            self.prev_page(ctx);
                        } else {
                            self.next_page(ctx);
                        }
                    }
                }

                // Mouse drag for panning
                let resp = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());
                if resp.dragged() {
                    let d = resp.drag_delta();
                    self.scroll.scroll_x(d.x);
                    self.scroll.scroll_y(d.y);
                    ctx.request_repaint();
                }
                if resp.double_clicked() {
                    self.scroll.reset();
                    self.zoom = 1.0;
                }

                // Click left/right third for page navigation
                if resp.clicked()
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    let rx = (pos.x - resp.rect.left()) / resp.rect.width();
                    if rx < 0.33 {
                        self.prev_page(ctx);
                    } else if rx > 0.67 {
                        self.next_page(ctx);
                    }
                }

                // Draw page(s)
                let origin = ui.max_rect().min;
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

                if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
                    let img_sz = tex.size_vec2();
                    let (dw, dh) = self.calc_image_size(img_sz.x, img_sz.y, avail.x, avail.y);
                    let (px, py) = self.scroll.image_position();
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
            });
    }
}

enum Act {
    NextPage,
    PrevPage,
    FirstPage,
    LastPage,
    ToggleDual,
    ToggleFullscreen,
    ExitFullscreen,
    OpenFile,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ScrollUp,
    ScrollDown,
}
