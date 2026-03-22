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

    // Navigation - N-page jumps
    JumpForward10,
    JumpForward20,
    JumpForward50,
    JumpBackward10,
    JumpBackward20,
    JumpBackward50,

    // Folder navigation
    NextFolder,
    PrevFolder,
    NextSubFolder,
    PrevSubFolder,
    UpOneLevel,

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
    ToggleThumbnailView,
    ToggleAutoSplit,
    ShowFileIndex,
    ShowGoToPageDialog,
    ShowImageInfo,
    ShowCacheInfo,
    ShowAbout,

    // Browsing modes
    SetBrowsingNoRepeat,
    SetBrowsingRepeat,
    SetBrowsingContinuous,

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
    ToggleToolbarIcons,
    ShowSettings,

    // Image processing
    RotateCw90,
    RotateCcw90,
    Rotate180,
    FlipHorizontal,
    FlipVertical,
    ApplyFilter,
    ToggleLoupe,

    // Scaling
    SetScaleFitWindowToImage,
    SetScaleFitWindow2Pages,
    SetScaleManual,
    SetScaleCustom,
    ToggleDownscaleOnly,
    SetFilterHalftone,
    SetFilterPixelAvg,
    SetFilterLanczos,
    SetFilterBicubic,
    SetFilterBilinear,
    SetFilterPixelAvgWeakSharpen,
    SetFilterPixelAvgStrongSharpen,

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
            Command::JumpForward10 => "Jump Forward 10 Pages",
            Command::JumpForward20 => "Jump Forward 20 Pages",
            Command::JumpForward50 => "Jump Forward 50 Pages",
            Command::JumpBackward10 => "Jump Backward 10 Pages",
            Command::JumpBackward20 => "Jump Backward 20 Pages",
            Command::JumpBackward50 => "Jump Backward 50 Pages",
            Command::NextFolder => "Next Folder",
            Command::PrevFolder => "Previous Folder",
            Command::ToggleFullscreen => "Toggle Fullscreen",
            Command::ToggleDualPage => "Toggle Dual Page",
            Command::ToggleThumbnailView => "Toggle Thumbnail View",
            Command::ToggleAutoSplit => "Toggle Auto Split",
            Command::SetBrowsingContinuous => "Continuous Browsing",
            Command::SetBrowsingRepeat => "Repeat Browsing",
            Command::SetBrowsingNoRepeat => "No Repeat Browsing",
            Command::OpenFile => "Open File",
            Command::Quit => "Quit",
            Command::ZoomIn => "Zoom In",
            Command::ZoomOut => "Zoom Out",
            Command::SetScaleFitWindowToImage => "Fit Window to Image",
            Command::SetScaleFitWindow2Pages => "Fit Window to 2 Pages",
            Command::ToggleDownscaleOnly => "Toggle Downscale Only",
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
        assert_eq!(Command::JumpForward10.display_name(), "Jump Forward 10 Pages");
        assert_eq!(Command::ToggleAutoSplit.display_name(), "Toggle Auto Split");
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
