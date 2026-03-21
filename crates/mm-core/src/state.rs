use std::path::PathBuf;

/// Top-level application state.
#[derive(Debug, Clone, Default)]
pub enum AppState {
    #[default]
    Idle,
    Loading {
        path: PathBuf,
        progress: f32,
    },
    Viewing {
        session: ViewSession,
    },
    Error {
        message: String,
    },
}

/// Active viewing session state.
#[derive(Debug, Clone)]
pub struct ViewSession {
    pub source_path: PathBuf,
    pub current_page: usize,
    pub total_pages: usize,
    pub page_mode: PageMode,
    pub scale_mode: ScaleMode,
    pub reading_direction: ReadingDirection,
    pub scroll_x: f64,
    pub scroll_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMode {
    Single,
    Dual,
    DualWithCover,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleMode {
    FitWidth,
    FitHeight,
    FitScreen,
    Original,
    Custom(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingDirection {
    RightToLeft,
    LeftToRight,
}

impl ViewSession {
    pub fn new(source_path: PathBuf, total_pages: usize) -> Self {
        Self {
            source_path,
            current_page: 0,
            total_pages,
            page_mode: PageMode::Dual,
            scale_mode: ScaleMode::FitScreen,
            reading_direction: ReadingDirection::RightToLeft,
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }

    /// Returns true if currently on the last page.
    pub fn is_last_page(&self) -> bool {
        if self.total_pages == 0 {
            return true;
        }
        self.current_page >= self.total_pages - 1
    }

    /// Returns true if currently on the first page.
    pub fn is_first_page(&self) -> bool {
        self.current_page == 0
    }

    /// Page step size based on current mode.
    pub fn page_step(&self) -> usize {
        match self.page_mode {
            PageMode::Single => 1,
            PageMode::Dual | PageMode::DualWithCover => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_session_new() {
        let session = ViewSession::new(PathBuf::from("/test.zip"), 100);
        assert_eq!(session.current_page, 0);
        assert_eq!(session.total_pages, 100);
        assert!(session.is_first_page());
        assert!(!session.is_last_page());
    }

    #[test]
    fn test_is_last_page() {
        let mut session = ViewSession::new(PathBuf::from("/test"), 10);
        session.current_page = 9;
        assert!(session.is_last_page());
    }

    #[test]
    fn test_is_first_page() {
        let session = ViewSession::new(PathBuf::from("/test"), 10);
        assert!(session.is_first_page());
    }

    #[test]
    fn test_page_step_single() {
        let mut session = ViewSession::new(PathBuf::from("/test"), 10);
        session.page_mode = PageMode::Single;
        assert_eq!(session.page_step(), 1);
    }

    #[test]
    fn test_page_step_dual() {
        let session = ViewSession::new(PathBuf::from("/test"), 10);
        assert_eq!(session.page_step(), 2);
    }

    #[test]
    fn test_empty_archive() {
        let session = ViewSession::new(PathBuf::from("/empty"), 0);
        assert!(session.is_first_page());
        assert!(session.is_last_page());
    }

    #[test]
    fn test_default_app_state() {
        let state = AppState::default();
        assert!(matches!(state, AppState::Idle));
    }
}
