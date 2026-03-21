use serde::{Deserialize, Serialize};

/// All commands that can be triggered by user input.
/// Matches the original MangaMeeya's ~175 CMD entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    // Navigation
    NextPage,
    PrevPage,
    NextPageDual,
    PrevPageDual,
    FirstPage,
    LastPage,
    JumpToPage,

    // Scrolling
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    ScrollPageUp,
    ScrollPageDown,
    ScrollToTop,
    ScrollToBottom,

    // Zoom
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomFitWidth,
    ZoomFitHeight,
    ZoomFitScreen,
    ZoomOriginal,

    // View modes
    ToggleFullscreen,
    ToggleDualPage,
    ToggleSinglePage,
    ToggleDualWithCover,
    ToggleReadingDirection,

    // File operations
    OpenFile,
    OpenFolder,
    OpenUrl,
    CloseFile,
    SaveImage,
    QuickSave1,
    QuickSave2,
    QuickSave3,
    QuickSave4,

    // Bookmarks & Lists
    ToggleBookmark,
    NextBookmark,
    PrevBookmark,
    ShowPlaylist,
    ShowBookmarks,
    ShowHistory,
    ShowBookRack,

    // UI
    ToggleToolbar,
    ToggleFolderTree,
    ToggleScrollBar,
    ToggleFileView,
    ToggleThumbnail,
    ToggleBookContents,
    TogglePageInfo,
    ShowSettings,

    // Image processing
    RotateCw90,
    RotateCcw90,
    Rotate180,
    FlipHorizontal,
    FlipVertical,
    ApplyFilter,
    ToggleLoupe,

    // Slide show
    SlideShowStart,
    SlideShowStop,
    SlideShowToggle,

    // System
    Quit,
    Refresh,
    ExternalEditor,
    DeleteFile,
    CopyImage,
    PrintImage,

    // Scroll position
    OriginTopLeft,
    OriginTopRight,
    OriginBottomLeft,
    OriginBottomRight,
}

impl Command {
    /// Returns a human-readable name for this command.
    pub fn display_name(self) -> &'static str {
        match self {
            Command::NextPage => "Next Page",
            Command::PrevPage => "Previous Page",
            Command::FirstPage => "First Page",
            Command::LastPage => "Last Page",
            Command::ToggleFullscreen => "Toggle Fullscreen",
            Command::ToggleDualPage => "Toggle Dual Page",
            Command::OpenFile => "Open File",
            Command::Quit => "Quit",
            Command::ZoomIn => "Zoom In",
            Command::ZoomOut => "Zoom Out",
            _ => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_display_name() {
        assert_eq!(Command::NextPage.display_name(), "Next Page");
        assert_eq!(Command::Quit.display_name(), "Quit");
    }

    #[test]
    fn test_command_equality() {
        assert_eq!(Command::NextPage, Command::NextPage);
        assert_ne!(Command::NextPage, Command::PrevPage);
    }

    #[test]
    fn test_command_clone() {
        let cmd = Command::ToggleFullscreen;
        let cloned = cmd;
        assert_eq!(cmd, cloned);
    }
}
