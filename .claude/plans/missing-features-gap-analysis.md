# MangaMeeya: Gap Analysis & Implementation Plan

Comprehensive comparison of original MangaMeeya (screenshots 31-37) vs current Rust implementation.

Legend: ✅ = Implemented | ⚠️ = Partially | ❌ = Missing

---

## 1. FILE MENU (Screenshot 31)

| Feature | Status | Notes |
|---------|--------|-------|
| Open File | ✅ | Ctrl+O, file dialog |
| Open Folder | ✅ | Folder dialog |
| Open URL (H)... | ❌ | Command enum exists (`OpenUrl`) but no UI/network fetch |
| Open With External Program (I) | ❌ | Command exists (`ExternalEditor`) but no launcher |
| External Program Settings (K)... | ❌ | No config for external programs |
| Save (Z) | ⚠️ | `SaveImage` command exists, no save-current-page impl |
| Quick Save submenu (1-4) | ⚠️ | `QuickSave1-4` commands exist, no submenu UI or actual save logic |
| Quick Save Settings... | ❌ | No dialog to configure quick save paths/formats |
| Close (C) | ⚠️ | `CloseFile` command exists, may not reset state properly |
| Refresh All (N) | ⚠️ | `Refresh` exists, but "Refresh All" may re-read archive too |
| Edit (E) submenu | ❌ | Entire submenu missing (likely image crop/rotate operations) |
| Import Configuration File (R)... | ⚠️ | INI import exists in mm-config but no UI file picker |
| Export Configuration File (D)... | ❌ | No config export |
| Paste From Clipboard (A) | ❌ | No clipboard paste support |
| Copy To Clipboard (B) | ⚠️ | `CopyImage` command exists, no actual clipboard integration |
| Page Setup (U)... | ❌ | No print page setup |
| Print (R)... | ❌ | No printing support |
| Recent Files list | ❌ | History exists but not shown at bottom of File menu |
| Exit | ✅ | Quit command |

---

## 2. VIEW MENU (Screenshot 32)

| Feature | Status | Notes |
|---------|--------|-------|
| 1 Page View / 2 Page View | ✅ | Single, Dual, Dual+Cover |
| Page Open Direction submenu | ⚠️ | Reading direction exists (RTL/LTR) but no submenu UI |
| Viewing Position submenu | ❌ | Initial scroll position after page change (top-left, top-right, etc.) — commands exist (`OriginTopLeft` etc.) but no menu |
| **Auto-Split Images** | ❌ | Single Page View that auto-splits wide/tall images into 2 pages |
| Auto-Split Horizontal | ❌ | Split wide images into left/right halves |
| Auto-Split Vertical | ❌ | Split tall images into top/bottom halves |
| Split Settings... | ❌ | Configure split thresholds, overlap |
| **Thumbnail View (Single Page)** | ❌ | Grid of page thumbnails (1 column) |
| **Thumbnail View (8 columns)** | ❌ | Grid of page thumbnails (8 columns) |
| **Thumbnail View (Custom)** | ❌ | Configurable column count |
| **Thumbnails Of All Files/Folders** | ❌ | Browse multiple archives as thumbnails |
| Thumbnails Settings... | ❌ | Thumbnail size, caching, etc. |
| Full Screen | ✅ | F11/Enter toggle |
| Show Lens Tool (Loupe) | ✅ | L key toggle |
| **Show Tool Bar** | ⚠️ | Bottom page bar exists, but not the icon toolbar from original |
| **Show Folder Tree Sidebar** | ❌ | Command exists, no folder tree UI |
| Show Bookmarks Sidebar | ✅ | Right panel tab |
| **Show Bookshelf** | ❌ | Visual bookshelf with book spine images |
| **Show Table of Contents** | ❌ | Chapter/section markers within a book |
| **Full Screen Settings submenu** | ❌ | Configure fullscreen behavior (hide cursor, hide taskbar, etc.) |
| **Show Additional Window** | ❌ | Open second viewer window |

---

## 3. SCALING MENU (Screenshot 33)

| Feature | Status | Notes |
|---------|--------|-------|
| Fit Width | ✅ | FitWidth scale mode |
| Fit To Window | ✅ | FitScreen scale mode |
| Original 100% Size | ✅ | Original scale mode |
| **Fit Window To Image** | ❌ | Resize the window to match image dimensions |
| **Fit To Window (2 Pages)** | ❌ | Scale for dual-page spread to fit window |
| **Manually Adjust Size...** | ❌ | Dialog for custom percentage/dimensions |
| **Custom Scale (Default)** | ❌ | Remembered custom scale factor |
| **Use Downscale-Resizing Only** | ❌ | Only scale down, never scale up |
| **HALFTONE Filter** | ❌ | Dithering/halftone resize algorithm |
| **Pixel Averaging Algorithm** | ❌ | Area-based averaging for downscale |
| **Lanczos Filter** | ❌ | High-quality sinc-based resampling |
| **Bicubic Interpolation** | ❌ | Cubic spline interpolation |
| Bilinear Interpolation | ✅ | Implemented in mm-image |
| Nearest Neighbor | ✅ | Implemented in mm-image |
| **Pixel Avg + Weak Sharpening** | ❌ | Combined filter pipeline preset |
| **Pixel Avg + Strong Sharpening** | ❌ | Combined filter pipeline preset |

**Key gap**: Only 2 of 7+ resize algorithms implemented. Lanczos and Bicubic are essential for quality.

---

## 4. JUMP MENU (Screenshot 34)

| Feature | Status | Notes |
|---------|--------|-------|
| Next / Back (1 page) | ✅ | NextPage/PrevPage |
| **Jump 10 Pages** | ❌ | No N-page jump |
| **Jump 20 Pages** | ❌ | No N-page jump |
| **Jump 50 Pages** | ❌ | No N-page jump |
| Last Page / Start | ✅ | FirstPage/LastPage |
| **Next Sub-Folder** | ❌ | Navigate to next archive in parent subfolder |
| **Next Folder** | ❌ | Navigate to next sibling folder |
| **Previous Sub-Folder** | ❌ | Navigate to previous archive in parent subfolder |
| **Previous Folder** | ❌ | Navigate to previous sibling folder |
| **Up One Level (Thumbnails View)** | ❌ | Go to parent dir in thumbnail browser |
| **Go To Page... dialog** | ❌ | Command exists but no popup dialog with page number input |
| **Show File Index** | ❌ | List all files in archive with index numbers |
| **No-Repeat Browsing Mode** | ❌ | Stop at last page |
| **Repeat Browsing Mode** | ❌ | Loop back to first page |
| **Continuous Browsing Mode** | ❌ | Auto-open next folder/archive when reaching end |

**Key gap**: Folder/archive navigation (next/prev folder) is a core workflow feature for reading series.

---

## 5. BOOK LIST MENU (Screenshot 35)

| Feature | Status | Notes |
|---------|--------|-------|
| View | ❌ | Unknown - possibly show booklist panel |
| **Mark reading position on toolbar** | ❌ | Quick bookmark shown in toolbar |
| **Go To Marked Position** | ❌ | Jump to marked position |
| **Add/Remove from Table of Contents** | ❌ | TOC management per book |
| Add to Bookmark List | ✅ | Bookmark system exists |
| **Add a Cover...** | ❌ | Assign cover image to book |
| **Add Image As Book Spine To Bookshelf** | ❌ | Bookshelf spine feature |
| Start Slideshow | ✅ | Slideshow implemented |
| **Playlist Settings... dialog** | ❌ | No playlist config dialog |
| **Show Thumbnails From submenu** | ❌ | Source for thumbnail display |
| Use History List | ✅ | History tracking works |
| Save Playlist... | ⚠️ | Playlist persists as JSON but no "Save As" file dialog |
| **Save Favorites...** | ❌ | Separate favorites list (vs bookmarks) |
| Save History... | ⚠️ | Auto-persists but no export dialog |
| Clear History List | ✅ | History clear function |
| Clear Bookmarks | ✅ | Bookmark clear function |

---

## 6. TOOLS MENU (Screenshot 36)

| Feature | Status | Notes |
|---------|--------|-------|
| **Pre-Cache Files toggle** | ⚠️ | Cache/prefetch exists in mm-core but no UI toggle |
| **Pre-Cache All Files** | ❌ | Decode all pages in archive upfront |
| **Page Flip Effect** | ❌ | Animated page-turn animation |
| **Use Overlay** | ❌ | Overlay rendering (semi-transparent UI layer) |
| **Sort Files submenu** | ❌ | Sort by name/date/size/type within archive |
| **Sort Folders submenu** | ❌ | Sort folder listing |
| Enable Avisynth | ❌ | Not applicable for Rust (replace with modern filter framework) |
| Avisynth Settings | ❌ | → Replace with image filter pipeline config |
| **Customize submenu** | ❌ | Toolbar/keyboard/mouse customization UI |
| **Registry Settings** | ❌ | File associations (Windows-specific) |
| **Options dialog** | ⚠️ | Settings dialog exists but is minimal |

---

## 7. HELP MENU (Screenshot 37)

| Feature | Status | Notes |
|---------|--------|-------|
| **Image Information dialog** | ❌ | Show format, dimensions, file size, color depth, DPI |
| **Cache Information dialog** | ❌ | Show cache stats, memory usage, decoded pages |
| **Version Information dialog** | ❌ | About dialog with version, build info |

---

## 8. BOTTOM STATUS BAR & TOOLBAR (All Screenshots)

| Feature | Status | Notes |
|---------|--------|-------|
| **Icon Toolbar** (bottom-left) | ❌ | Row of small icon buttons for common actions |
| **Full-width Page Slider** | ⚠️ | Page bar exists but may not match original's long slider design |
| **Image Dimensions Display** | ❌ | "2560 x 965" shown at top-left corner |
| **Title Bar Info** | ⚠️ | Shows file path but not dimensions or filter info |
| **Page position in slider** | ⚠️ | Basic slider exists, original has a more prominent design |

---

## 9. ARCHIVE FORMAT SUPPORT

| Format | Status | Notes |
|--------|--------|-------|
| ZIP/CBZ | ✅ | Implemented |
| **RAR/CBR** | ❌ | Very common for manga/comics |
| **7z/CB7** | ❌ | Growing in popularity |
| **TAR/TAR.GZ** | ❌ | Unix archives |
| **PDF** | ❌ | Common for scanlations |
| Folder | ✅ | Implemented |

---

## 10. TITLE BAR FORMAT

Original shows: `C:\...\filename.zip : Off fpJX`
- Full file path
- Filter status ("Off" = no filter applied)
- "fpJX" = internal codec/format info

Current shows just the file name. Missing the detailed status info.

---

# IMPLEMENTATION PLAN

## Phase 4: Essential Missing Features (HIGH PRIORITY)

These are the most impactful features that real users need:

### 4A. Scaling Algorithms & Menu
- [ ] Implement Lanczos resampling in mm-image
- [ ] Implement Bicubic interpolation in mm-image
- [ ] Implement Pixel Averaging (area-based) downscaler
- [ ] Add Halftone filter
- [ ] Add combined filter presets (Pixel Avg + Weak/Strong Sharpening)
- [ ] Add "Fit Window To Image" scale mode
- [ ] Add "Fit To Window (2 Pages)" scale mode
- [ ] Add "Manually Adjust Size" dialog
- [ ] Add "Custom Scale" with remembered factor
- [ ] Add "Downscale Only" toggle
- [ ] Build full Scaling menu in UI

### 4B. Jump / Navigation Enhancements
- [ ] Add N-page jump (10, 20, 50) forward/back
- [ ] Implement folder/archive navigation (next/prev folder, next/prev subfolder)
- [ ] Add "Go To Page" dialog (popup with number input)
- [ ] Add "Show File Index" panel (list of all files in archive)
- [ ] Implement Browsing Modes: No-Repeat, Repeat, Continuous
- [ ] Build full Jump menu in UI

### 4C. Auto-Split Wide/Tall Images
- [ ] Detect wide images (width > 1.3x height or configurable threshold)
- [ ] Split into left/right halves for "Single Page View (auto-split)"
- [ ] Split horizontally (H) and vertically (V) variants
- [ ] Split Settings dialog (threshold, overlap percentage)
- [ ] Respect reading direction for split order (RTL: right-first)

### 4D. Thumbnail Grid View
- [ ] Thumbnail generation (downscaled page previews)
- [ ] Single-column thumbnail view
- [ ] Multi-column grid (8 columns, custom)
- [ ] Thumbnail cache for performance
- [ ] Click-to-jump from thumbnail
- [ ] Thumbnail settings (size, columns, cache)
- [ ] "All Files and Folders" thumbnail browser

### 4E. Full-Width Bottom Toolbar & Slider
- [ ] Icon toolbar strip at bottom-left (open, save, prev, next, zoom, etc.)
- [ ] Full-width page slider bar spanning entire window bottom
- [ ] Image dimensions display (top-left or status bar)
- [ ] Enhanced title bar: `filepath : filter_status format_info`
- [ ] Configurable toolbar (show/hide, icon selection)

---

## Phase 5: Important Missing Features (MEDIUM PRIORITY)

### 5A. Additional Archive Formats
- [ ] RAR/CBR support (via `unrar` or `sevenz-rust` crate)
- [ ] 7z/CB7 support (via `sevenz-rust` crate)
- [ ] PDF support (via `pdf-extract` or `pdfium` crate)
- [ ] TAR/TAR.GZ support (via `tar` + `flate2` crates)

### 5B. Folder Tree Sidebar
- [ ] Tree widget showing directory structure
- [ ] Expand/collapse folders
- [ ] Click to open folder/archive
- [ ] Current file highlighted
- [ ] Configurable root path

### 5C. File Operations
- [ ] Copy current image to clipboard (platform clipboard crate)
- [ ] Paste image from clipboard
- [ ] Print support (platform print dialog)
- [ ] Page Setup dialog
- [ ] Save As dialog with format selection
- [ ] Quick Save to preset paths (4 slots with configurable paths)
- [ ] Quick Save Settings dialog

### 5D. Table of Contents System
- [ ] Mark pages as chapter/section boundaries
- [ ] Add/remove TOC entries with labels
- [ ] TOC sidebar panel
- [ ] Jump to TOC entry
- [ ] Persist TOC per book (JSON)

### 5E. Recent Files
- [ ] Track last N opened files (separate from history)
- [ ] Show at bottom of File menu with numbered shortcuts
- [ ] Clear recent files option

### 5F. Import/Export Config
- [ ] Export current config to file (TOML)
- [ ] Import config from file picker
- [ ] Import from original MangaMeeya.ini (UI for existing mm-config feature)

### 5G. Sort Options
- [ ] Sort files within archive: Name (natural), Date, Size, Type
- [ ] Sort folders: Name, Date, Size
- [ ] Persist sort preference per-folder or globally

---

## Phase 6: Polish & Advanced Features (LOWER PRIORITY)

### 6A. Bookshelf View
- [ ] Visual bookshelf with book spine images
- [ ] Assign cover images to books
- [ ] Assign spine images
- [ ] Grid/shelf layout
- [ ] Click to open

### 6B. Page Flip Animation
- [ ] Animated page-turn effect (3D curl or slide)
- [ ] Toggle on/off
- [ ] Configurable animation speed

### 6C. Multi-Window Support
- [ ] "Show Additional Window" opens second viewer
- [ ] Independent navigation per window
- [ ] Useful for reference/comparison

### 6D. External Program Integration
- [ ] Configure list of external programs (editors, converters)
- [ ] "Open With External Program" sends current image
- [ ] External Program Settings dialog

### 6E. Fullscreen Settings
- [ ] Auto-hide cursor timeout
- [ ] Background color in fullscreen
- [ ] Auto-hide toolbar in fullscreen
- [ ] Multi-monitor fullscreen behavior

### 6F. Overlay System
- [ ] Semi-transparent info overlay
- [ ] Customizable overlay content (page info, time, filename)
- [ ] Overlay position and opacity settings

### 6G. Viewing Position System
- [ ] Configure initial scroll position after page change
- [ ] Options: Top-Left, Top-Right, Bottom-Left, Bottom-Right, Center
- [ ] Reading-direction-aware defaults

### 6H. Customize Dialog
- [ ] Keyboard shortcut editor
- [ ] Mouse button/gesture editor
- [ ] Toolbar button editor
- [ ] Click zone editor (resize zones, assign actions)

### 6I. Image Information Dialog
- [ ] File name, path, size on disk
- [ ] Image dimensions (pixels)
- [ ] Color depth, color space
- [ ] DPI/PPI
- [ ] Format/codec info
- [ ] Archive position (page X of Y)

### 6J. Cache Information Dialog
- [ ] Total cached pages
- [ ] Memory usage (decoded images)
- [ ] Cache hit/miss stats
- [ ] Prefetch queue status

### 6K. Version/About Dialog
- [ ] App name, version, build date
- [ ] Rust version, platform info
- [ ] Credits, license
- [ ] Link to repository

### 6L. Options Dialog (Comprehensive)
- [ ] Consolidate all settings into tabbed dialog
- [ ] Tabs: General, Display, Scaling, Navigation, Cache, Keyboard, Mouse, Advanced
- [ ] Currently the Settings window is minimal

### 6M. File Association (Windows)
- [ ] Register as handler for .cbz, .cbr, .cb7, .zip, .rar, .7z, .pdf
- [ ] Windows right-click "Open with MangaMeeya"
- [ ] Registry integration

---

# SUMMARY OF GAPS BY SEVERITY

## Critical (core reading experience):
1. **Lanczos/Bicubic resize** — image quality is noticeably worse without these
2. **Continuous browsing mode** — auto-open next archive/folder is essential for series
3. **Next/Prev folder navigation** — navigating between volumes
4. **RAR/CBR support** — huge portion of manga/comic archives use RAR
5. **Full-width page slider** — the bottom slider bar is a signature MangaMeeya UX element

## Important (expected features):
6. **Thumbnail grid view** — quick page overview
7. **Auto-split wide images** — common for scanned double-page spreads
8. **Go To Page dialog** — direct page entry
9. **N-page jump** (10/20/50) — fast skipping
10. **Folder tree sidebar** — browse filesystem
11. **Image info display** — dimensions in title/status bar
12. **Sort options** — control file ordering
13. **Downscale-only mode** — prevent upscaling small images

## Nice to have:
14. Bookshelf view
15. Page flip animation
16. Multi-window
17. Clipboard paste
18. Print support
19. External program integration
20. Table of Contents
21. Customize dialog
22. About/Version dialog
