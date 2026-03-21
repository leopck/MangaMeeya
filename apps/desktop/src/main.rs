use eframe::egui;
use mm_archive::ArchiveReader;
use mm_image::ColorSpace;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Accept file path as CLI argument (like original MangaMeeya)
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
        Box::new(move |cc| Ok(Box::new(MangaMeeyaApp::new(cc, initial_path)))),
    )
}

struct MangaMeeyaApp {
    // Archive state
    archive: Option<Box<dyn ArchiveReader>>,
    source_path: Option<PathBuf>,
    entry_names: Vec<String>,

    // Page state
    current_page: usize,
    total_pages: usize,
    dual_page: bool,

    // Decoded images as egui textures
    page_textures: Vec<Option<egui::TextureHandle>>,

    // UI state
    error_message: Option<String>,
    show_page_bar: bool,
    fit_mode: FitMode,
    zoom: f32,
    drag_offset: egui::Vec2,
}

#[derive(Clone, Copy, PartialEq)]
enum FitMode {
    FitScreen,
    FitWidth,
    FitHeight,
    Original,
}

impl MangaMeeyaApp {
    fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        let mut app = Self {
            archive: None,
            source_path: None,
            entry_names: Vec::new(),
            current_page: 0,
            total_pages: 0,
            dual_page: false,
            page_textures: Vec::new(),
            error_message: None,
            show_page_bar: true,
            fit_mode: FitMode::FitScreen,
            zoom: 1.0,
            drag_offset: egui::Vec2::ZERO,
        };
        if let Some(path) = initial_path {
            app.open_path(path, &cc.egui_ctx);
        }
        app
    }

    fn open_path(&mut self, path: PathBuf, ctx: &egui::Context) {
        match mm_archive::open(&path) {
            Ok(reader) => {
                self.entry_names = reader.entry_names();
                self.total_pages = reader.entry_count();
                self.current_page = 0;
                self.page_textures = vec![None; self.total_pages];
                self.source_path = Some(path);
                self.archive = Some(reader);
                self.error_message = None;
                self.drag_offset = egui::Vec2::ZERO;
                self.zoom = 1.0;
                // Decode first page immediately
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
        if page >= self.total_pages {
            return;
        }
        if self.page_textures[page].is_some() {
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
                Err(e) => {
                    tracing::warn!("Failed to decode page {page}: {e}");
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read page {page}: {e}");
            }
        }
    }

    fn goto_page(&mut self, page: usize, ctx: &egui::Context) {
        if self.total_pages == 0 {
            return;
        }
        self.current_page = page.min(self.total_pages - 1);
        self.drag_offset = egui::Vec2::ZERO;
        self.ensure_page_decoded(self.current_page, ctx);
        if self.dual_page && self.current_page + 1 < self.total_pages {
            self.ensure_page_decoded(self.current_page + 1, ctx);
        }
        // Prefetch next few pages
        for i in 1..=3 {
            if self.current_page + i < self.total_pages {
                self.ensure_page_decoded(self.current_page + i, ctx);
            }
        }
    }

    fn next_page(&mut self, ctx: &egui::Context) {
        let step = if self.dual_page { 2 } else { 1 };
        let new_page = (self.current_page + step).min(self.total_pages.saturating_sub(1));
        self.goto_page(new_page, ctx);
    }

    fn prev_page(&mut self, ctx: &egui::Context) {
        let step = if self.dual_page { 2 } else { 1 };
        let new_page = self.current_page.saturating_sub(step);
        self.goto_page(new_page, ctx);
    }

    fn title(&self) -> String {
        if let Some(path) = &self.source_path {
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
        } else {
            "MangaMeeya".to_string()
        }
    }

    fn calculate_image_size(
        &self,
        img_w: f32,
        img_h: f32,
        avail_w: f32,
        avail_h: f32,
    ) -> egui::Vec2 {
        match self.fit_mode {
            FitMode::FitScreen => {
                let scale = (avail_w / img_w).min(avail_h / img_h).min(1.0) * self.zoom;
                egui::vec2(img_w * scale, img_h * scale)
            }
            FitMode::FitWidth => {
                let scale = (avail_w / img_w) * self.zoom;
                egui::vec2(img_w * scale, img_h * scale)
            }
            FitMode::FitHeight => {
                let scale = (avail_h / img_h) * self.zoom;
                egui::vec2(img_w * scale, img_h * scale)
            }
            FitMode::Original => egui::vec2(img_w * self.zoom, img_h * self.zoom),
        }
    }
}

fn to_rgba_pixels(img: &mm_image::ImageBuffer) -> Vec<egui::Color32> {
    let pixel_count = img.width as usize * img.height as usize;
    let mut pixels = Vec::with_capacity(pixel_count);
    match img.color_space {
        ColorSpace::Rgb8 => {
            for i in 0..pixel_count {
                let offset = i * 3;
                pixels.push(egui::Color32::from_rgb(
                    img.data[offset],
                    img.data[offset + 1],
                    img.data[offset + 2],
                ));
            }
        }
        ColorSpace::Rgba8 => {
            for i in 0..pixel_count {
                let offset = i * 4;
                pixels.push(egui::Color32::from_rgba_premultiplied(
                    img.data[offset],
                    img.data[offset + 1],
                    img.data[offset + 2],
                    img.data[offset + 3],
                ));
            }
        }
        ColorSpace::Grayscale => {
            for i in 0..pixel_count {
                let v = img.data[i];
                pixels.push(egui::Color32::from_rgb(v, v, v));
            }
        }
    }
    pixels
}

impl eframe::App for MangaMeeyaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update window title
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));

        // Handle keyboard input
        let mut action = None;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown) {
                action = Some(Action::NextPage);
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp) {
                action = Some(Action::PrevPage);
            }
            if i.key_pressed(egui::Key::Home) {
                action = Some(Action::FirstPage);
            }
            if i.key_pressed(egui::Key::End) {
                action = Some(Action::LastPage);
            }
            if i.key_pressed(egui::Key::Space) {
                action = Some(Action::NextPage);
            }
            if i.key_pressed(egui::Key::Backspace) {
                action = Some(Action::PrevPage);
            }
            if i.key_pressed(egui::Key::D) {
                action = Some(Action::ToggleDual);
            }
            if i.key_pressed(egui::Key::F11) || i.key_pressed(egui::Key::Enter) {
                action = Some(Action::ToggleFullscreen);
            }
            if i.key_pressed(egui::Key::Escape) {
                action = Some(Action::ExitFullscreen);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                action = Some(Action::OpenFile);
            }
            if i.key_pressed(egui::Key::Minus) {
                action = Some(Action::ZoomOut);
            }
            if i.key_pressed(egui::Key::Plus)
                || (i.modifiers.shift && i.key_pressed(egui::Key::Equals))
            {
                action = Some(Action::ZoomIn);
            }
            if i.key_pressed(egui::Key::Num0) {
                action = Some(Action::ZoomReset);
            }
        });

        // Execute actions
        match action {
            Some(Action::NextPage) => self.next_page(ctx),
            Some(Action::PrevPage) => self.prev_page(ctx),
            Some(Action::FirstPage) => self.goto_page(0, ctx),
            Some(Action::LastPage) => {
                let last = self.total_pages.saturating_sub(1);
                self.goto_page(last, ctx);
            }
            Some(Action::ToggleDual) => {
                self.dual_page = !self.dual_page;
                self.goto_page(self.current_page, ctx);
            }
            Some(Action::ToggleFullscreen) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            Some(Action::ExitFullscreen) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
            Some(Action::OpenFile) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Archives", &["zip", "cbz"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open_path(path, ctx);
                }
            }
            Some(Action::ZoomIn) => self.zoom = (self.zoom * 1.2).min(10.0),
            Some(Action::ZoomOut) => self.zoom = (self.zoom / 1.2).max(0.1),
            Some(Action::ZoomReset) => self.zoom = 1.0,
            None => {}
        }

        // Handle dropped files
        let dropped_path = ctx.input(|i| i.raw.dropped_files.first().and_then(|d| d.path.clone()));
        if let Some(path) = dropped_path {
            self.open_path(path, ctx);
        }

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open (Ctrl+O)").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Archives", &["zip", "cbz"])
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            self.open_path(path, ctx);
                        }
                    }
                    if ui.button("Open Folder").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.open_path(path, ctx);
                        }
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::FitScreen, "Fit Screen")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::FitWidth, "Fit Width")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::FitHeight, "Fit Height")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(&mut self.fit_mode, FitMode::Original, "Original Size")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.checkbox(&mut self.dual_page, "Dual Page (D)").clicked() {
                        ui.close_menu();
                        self.goto_page(self.current_page, ctx);
                    }
                    ui.checkbox(&mut self.show_page_bar, "Page Bar");
                });
                ui.menu_button("Help", |ui| {
                    ui.label("Keyboard shortcuts:");
                    ui.label("  Right/PageDown/Space: Next page");
                    ui.label("  Left/PageUp/Backspace: Prev page");
                    ui.label("  Home/End: First/Last page");
                    ui.label("  D: Toggle dual page");
                    ui.label("  F11/Enter: Fullscreen");
                    ui.label("  +/-/0: Zoom in/out/reset");
                    ui.label("  Ctrl+O: Open file");
                    ui.label("  Drag & drop: Open file");
                });

                // Page info on the right
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

        // Page slider at bottom
        if self.show_page_bar && self.total_pages > 1 {
            egui::TopBottomPanel::bottom("page_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("|<").clicked() {
                        self.goto_page(0, ctx);
                    }
                    if ui.button("<").clicked() {
                        self.prev_page(ctx);
                    }

                    let mut page_f = self.current_page as f32;
                    let slider =
                        egui::Slider::new(&mut page_f, 0.0..=(self.total_pages as f32 - 1.0))
                            .show_value(false)
                            .text(format!("{}/{}", self.current_page + 1, self.total_pages));
                    if ui.add(slider).changed() {
                        self.goto_page(page_f as usize, ctx);
                    }

                    if ui.button(">").clicked() {
                        self.next_page(ctx);
                    }
                    if ui.button(">|").clicked() {
                        let last = self.total_pages.saturating_sub(1);
                        self.goto_page(last, ctx);
                    }
                });
            });
        }

        // Main image area
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                if self.archive.is_none() {
                    // Welcome screen
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(ui.available_height() / 3.0);
                            ui.heading("MangaMeeya");
                            ui.add_space(10.0);
                            ui.label("Drop a ZIP/CBZ file or folder here to start reading");
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

                // Handle mouse scroll for page navigation on the image area
                let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                if scroll_delta.y > 5.0 {
                    self.prev_page(ctx);
                } else if scroll_delta.y < -5.0 {
                    self.next_page(ctx);
                }

                // Handle mouse drag for panning
                let response = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());
                if response.dragged() {
                    self.drag_offset += response.drag_delta();
                }
                if response.double_clicked() {
                    self.drag_offset = egui::Vec2::ZERO;
                    self.zoom = 1.0;
                }

                // Click left/right thirds for page navigation
                if response.clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let rect = response.rect;
                    let relative_x = (pos.x - rect.left()) / rect.width();
                    if relative_x < 0.33 {
                        self.prev_page(ctx);
                    } else if relative_x > 0.67 {
                        self.next_page(ctx);
                    }
                }

                // Draw pages
                if self.dual_page {
                    self.draw_dual_page(ui, avail);
                } else {
                    self.draw_single_page(ui, avail);
                }
            });
    }
}

impl MangaMeeyaApp {
    fn draw_single_page(&self, ui: &mut egui::Ui, avail: egui::Vec2) {
        if let Some(Some(tex)) = self.page_textures.get(self.current_page) {
            let img_size = tex.size_vec2();
            let display_size = self.calculate_image_size(img_size.x, img_size.y, avail.x, avail.y);
            let center = ui.max_rect().center() + self.drag_offset;
            let rect = egui::Rect::from_center_size(center, display_size);
            ui.painter().image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
        }
    }

    fn draw_dual_page(&self, ui: &mut egui::Ui, avail: egui::Vec2) {
        let left_page = self.current_page;
        let right_page = self.current_page + 1;
        let half_w = avail.x / 2.0;

        let left_tex = self.page_textures.get(left_page).and_then(|t| t.as_ref());
        let right_tex = if right_page < self.total_pages {
            self.page_textures.get(right_page).and_then(|t| t.as_ref())
        } else {
            None
        };

        let center = ui.max_rect().center() + self.drag_offset;
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

        if let Some(tex) = left_tex {
            let img_size = tex.size_vec2();
            let display_size =
                self.calculate_image_size(img_size.x, img_size.y, half_w - 4.0, avail.y);
            let left_center = egui::pos2(center.x - display_size.x / 2.0 - 2.0, center.y);
            let rect = egui::Rect::from_center_size(left_center, display_size);
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        }

        if let Some(tex) = right_tex {
            let img_size = tex.size_vec2();
            let display_size =
                self.calculate_image_size(img_size.x, img_size.y, half_w - 4.0, avail.y);
            let right_center = egui::pos2(center.x + display_size.x / 2.0 + 2.0, center.y);
            let rect = egui::Rect::from_center_size(right_center, display_size);
            ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        }

        if left_tex.is_none() && right_tex.is_none() {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
        }
    }
}

enum Action {
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
}
