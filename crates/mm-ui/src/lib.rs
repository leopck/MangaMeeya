/// MangaMeeya UI built on egui.
/// Phase 2 implementation - currently a stub.

/// UI state for the viewer application.
pub struct ViewerUi {
    pub show_toolbar: bool,
    pub show_folder_tree: bool,
    pub show_thumbnails: bool,
    pub show_bookmarks: bool,
    pub show_settings: bool,
}

impl Default for ViewerUi {
    fn default() -> Self {
        Self {
            show_toolbar: true,
            show_folder_tree: false,
            show_thumbnails: false,
            show_bookmarks: false,
            show_settings: false,
        }
    }
}
