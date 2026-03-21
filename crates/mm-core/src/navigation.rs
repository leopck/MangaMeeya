use crate::state::{PageMode, ViewSession};

/// Page navigation logic.
pub struct Navigator;

impl Navigator {
    /// Advance to the next page(s). Returns the new page index.
    pub fn next_page(session: &ViewSession) -> usize {
        if session.is_last_page() {
            return session.current_page;
        }

        let step = session.page_step();
        let new_page = session.current_page + step;
        new_page.min(session.total_pages.saturating_sub(1))
    }

    /// Go to the previous page(s). Returns the new page index.
    pub fn prev_page(session: &ViewSession) -> usize {
        if session.is_first_page() {
            return 0;
        }

        let step = session.page_step();
        session.current_page.saturating_sub(step)
    }

    /// Jump to a specific page, clamping to valid range.
    pub fn jump_to(session: &ViewSession, page: usize) -> usize {
        if session.total_pages == 0 {
            return 0;
        }
        page.min(session.total_pages - 1)
    }

    /// Go to first page.
    pub fn first_page() -> usize {
        0
    }

    /// Go to last page.
    pub fn last_page(session: &ViewSession) -> usize {
        session.total_pages.saturating_sub(1)
    }

    /// Determine if a page should be displayed as dual based on image dimensions.
    /// Original MangaMeeya formula: `@height > @width` means portrait -> use dual.
    pub fn should_use_dual(img_width: u32, img_height: u32) -> bool {
        img_height > img_width
    }

    /// Get the pages to display for the current position in dual mode.
    /// Returns (left_page, optional_right_page) accounting for reading direction
    /// and DualWithCover mode.
    pub fn dual_pages(session: &ViewSession) -> (usize, Option<usize>) {
        let page = session.current_page;

        if session.page_mode == PageMode::Single {
            return (page, None);
        }

        // DualWithCover: page 0 is shown alone
        if session.page_mode == PageMode::DualWithCover && page == 0 {
            return (0, None);
        }

        let right_page = if page + 1 < session.total_pages {
            Some(page + 1)
        } else {
            None
        };

        (page, right_page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ReadingDirection, ScaleMode};
    use std::path::PathBuf;

    fn make_session(pages: usize, current: usize, mode: PageMode) -> ViewSession {
        ViewSession {
            source_path: PathBuf::from("/test"),
            current_page: current,
            total_pages: pages,
            page_mode: mode,
            scale_mode: ScaleMode::FitScreen,
            reading_direction: ReadingDirection::RightToLeft,
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }

    #[test]
    fn test_next_page_single() {
        let session = make_session(10, 0, PageMode::Single);
        assert_eq!(Navigator::next_page(&session), 1);
    }

    #[test]
    fn test_next_page_dual() {
        let session = make_session(10, 0, PageMode::Dual);
        assert_eq!(Navigator::next_page(&session), 2);
    }

    #[test]
    fn test_next_page_at_end() {
        let session = make_session(10, 9, PageMode::Single);
        assert_eq!(Navigator::next_page(&session), 9);
    }

    #[test]
    fn test_next_page_dual_at_end() {
        let session = make_session(10, 9, PageMode::Dual);
        assert_eq!(Navigator::next_page(&session), 9); // Can't go past last
    }

    #[test]
    fn test_prev_page_single() {
        let session = make_session(10, 5, PageMode::Single);
        assert_eq!(Navigator::prev_page(&session), 4);
    }

    #[test]
    fn test_prev_page_dual() {
        let session = make_session(10, 4, PageMode::Dual);
        assert_eq!(Navigator::prev_page(&session), 2);
    }

    #[test]
    fn test_prev_page_at_start() {
        let session = make_session(10, 0, PageMode::Single);
        assert_eq!(Navigator::prev_page(&session), 0);
    }

    #[test]
    fn test_jump_to_page() {
        let session = make_session(100, 0, PageMode::Single);
        assert_eq!(Navigator::jump_to(&session, 50), 50);
    }

    #[test]
    fn test_jump_out_of_range() {
        let session = make_session(100, 0, PageMode::Single);
        assert_eq!(Navigator::jump_to(&session, 9999), 99);
    }

    #[test]
    fn test_first_page() {
        assert_eq!(Navigator::first_page(), 0);
    }

    #[test]
    fn test_last_page() {
        let session = make_session(100, 0, PageMode::Single);
        assert_eq!(Navigator::last_page(&session), 99);
    }

    #[test]
    fn test_should_use_dual_portrait() {
        assert!(Navigator::should_use_dual(800, 1200));
    }

    #[test]
    fn test_should_use_dual_landscape() {
        assert!(!Navigator::should_use_dual(1200, 800));
    }

    #[test]
    fn test_should_use_dual_square() {
        assert!(!Navigator::should_use_dual(800, 800));
    }

    #[test]
    fn test_dual_pages_normal() {
        let session = make_session(10, 2, PageMode::Dual);
        let (left, right) = Navigator::dual_pages(&session);
        assert_eq!(left, 2);
        assert_eq!(right, Some(3));
    }

    #[test]
    fn test_dual_pages_last() {
        let session = make_session(10, 9, PageMode::Dual);
        let (left, right) = Navigator::dual_pages(&session);
        assert_eq!(left, 9);
        assert_eq!(right, None);
    }

    #[test]
    fn test_dual_with_cover_page0() {
        let session = make_session(10, 0, PageMode::DualWithCover);
        let (left, right) = Navigator::dual_pages(&session);
        assert_eq!(left, 0);
        assert_eq!(right, None);
    }

    #[test]
    fn test_dual_with_cover_page1() {
        let session = make_session(10, 1, PageMode::DualWithCover);
        let (left, right) = Navigator::dual_pages(&session);
        assert_eq!(left, 1);
        assert_eq!(right, Some(2));
    }

    #[test]
    fn test_single_mode_no_dual() {
        let session = make_session(10, 5, PageMode::Single);
        let (left, right) = Navigator::dual_pages(&session);
        assert_eq!(left, 5);
        assert_eq!(right, None);
    }

    #[test]
    fn test_empty_archive_navigation() {
        let session = make_session(0, 0, PageMode::Single);
        assert_eq!(Navigator::next_page(&session), 0);
        assert_eq!(Navigator::prev_page(&session), 0);
        assert_eq!(Navigator::jump_to(&session, 5), 0);
        assert_eq!(Navigator::last_page(&session), 0);
    }
}
