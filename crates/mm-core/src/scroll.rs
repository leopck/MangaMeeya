/// Scroll state machine for the image viewport.
///
/// Handles vertical/horizontal scrolling when images overflow the viewport,
/// smooth scroll animation, and page-turn-at-boundary logic.
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current scroll offset (pixels). Negative = scrolled down/right.
    pub offset_x: f32,
    pub offset_y: f32,
    /// Image display size (after fit/zoom).
    content_width: f32,
    content_height: f32,
    /// Available viewport size.
    viewport_width: f32,
    viewport_height: f32,
    /// Smooth scroll animation target.
    smooth_target_x: f32,
    smooth_target_y: f32,
    /// Whether smooth scrolling is enabled.
    pub smooth: bool,
    /// Smooth scroll speed (pixels per second).
    pub speed: f32,
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            viewport_width: 0.0,
            viewport_height: 0.0,
            smooth_target_x: 0.0,
            smooth_target_y: 0.0,
            smooth: true,
            speed: 1300.0,
        }
    }

    /// Update the content and viewport dimensions. Call when image or window size changes.
    pub fn set_dimensions(
        &mut self,
        content_width: f32,
        content_height: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.content_width = content_width;
        self.content_height = content_height;
        self.viewport_width = viewport_width;
        self.viewport_height = viewport_height;
        self.clamp();
    }

    /// Whether vertical scrolling is possible (content taller than viewport).
    pub fn can_scroll_vertically(&self) -> bool {
        self.content_height > self.viewport_height + 0.5
    }

    /// Whether horizontal scrolling is possible.
    pub fn can_scroll_horizontally(&self) -> bool {
        self.content_width > self.viewport_width + 0.5
    }

    /// Whether we're scrolled to the very top.
    pub fn at_top(&self) -> bool {
        self.offset_y >= -0.5
    }

    /// Whether we're scrolled to the very bottom.
    pub fn at_bottom(&self) -> bool {
        if !self.can_scroll_vertically() {
            return true;
        }
        let max_scroll = self.content_height - self.viewport_height;
        self.offset_y <= -(max_scroll - 0.5)
    }

    /// Whether we're scrolled to the very left.
    pub fn at_left(&self) -> bool {
        self.offset_x >= -0.5
    }

    /// Whether we're scrolled to the very right.
    pub fn at_right(&self) -> bool {
        if !self.can_scroll_horizontally() {
            return true;
        }
        let max_scroll = self.content_width - self.viewport_width;
        self.offset_x <= -(max_scroll - 0.5)
    }

    /// Scroll vertically by delta pixels. Positive = scroll up (content moves down).
    pub fn scroll_y(&mut self, delta: f32) {
        if self.smooth {
            self.smooth_target_y += delta;
            self.smooth_target_y = self.clamp_value_y(self.smooth_target_y);
        } else {
            self.offset_y += delta;
            self.clamp();
        }
    }

    /// Scroll horizontally by delta pixels.
    pub fn scroll_x(&mut self, delta: f32) {
        if self.smooth {
            self.smooth_target_x += delta;
            self.smooth_target_x = self.clamp_value_x(self.smooth_target_x);
        } else {
            self.offset_x += delta;
            self.clamp();
        }
    }

    /// Animate smooth scrolling. Call each frame with dt in seconds.
    pub fn animate(&mut self, dt: f32) {
        if !self.smooth {
            return;
        }
        let factor = (self.speed * dt / 100.0).min(1.0);
        self.offset_x += (self.smooth_target_x - self.offset_x) * factor;
        self.offset_y += (self.smooth_target_y - self.offset_y) * factor;
        // Snap when close enough
        if (self.offset_x - self.smooth_target_x).abs() < 0.5 {
            self.offset_x = self.smooth_target_x;
        }
        if (self.offset_y - self.smooth_target_y).abs() < 0.5 {
            self.offset_y = self.smooth_target_y;
        }
    }

    /// Whether smooth scroll animation is still in progress.
    pub fn is_animating(&self) -> bool {
        self.smooth
            && ((self.offset_x - self.smooth_target_x).abs() > 0.5
                || (self.offset_y - self.smooth_target_y).abs() > 0.5)
    }

    /// Reset scroll to top-left.
    pub fn reset(&mut self) {
        self.offset_x = 0.0;
        self.offset_y = 0.0;
        self.smooth_target_x = 0.0;
        self.smooth_target_y = 0.0;
    }

    /// Clamp scroll offsets to valid range.
    fn clamp(&mut self) {
        self.offset_x = self.clamp_value_x(self.offset_x);
        self.offset_y = self.clamp_value_y(self.offset_y);
        self.smooth_target_x = self.clamp_value_x(self.smooth_target_x);
        self.smooth_target_y = self.clamp_value_y(self.smooth_target_y);
    }

    fn clamp_value_y(&self, val: f32) -> f32 {
        if !self.can_scroll_vertically() {
            return 0.0;
        }
        let max_scroll = self.content_height - self.viewport_height;
        val.clamp(-max_scroll, 0.0)
    }

    fn clamp_value_x(&self, val: f32) -> f32 {
        if !self.can_scroll_horizontally() {
            return 0.0;
        }
        let max_scroll = self.content_width - self.viewport_width;
        val.clamp(-max_scroll, 0.0)
    }

    /// Get the top-left position for painting the image.
    /// When content fits viewport: centered.
    /// When content overflows: offset from top-left.
    pub fn image_position(&self) -> (f32, f32) {
        let x = if self.can_scroll_horizontally() {
            self.offset_x
        } else {
            (self.viewport_width - self.content_width) / 2.0
        };
        let y = if self.can_scroll_vertically() {
            self.offset_y
        } else {
            (self.viewport_height - self.content_height) / 2.0
        };
        (x, y)
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_scroll_when_fits() {
        let mut s = ScrollState::new();
        s.set_dimensions(800.0, 600.0, 1024.0, 768.0);
        assert!(!s.can_scroll_vertically());
        assert!(!s.can_scroll_horizontally());
        assert!(s.at_top());
        assert!(s.at_bottom());
    }

    #[test]
    fn test_vertical_scroll_when_tall() {
        let mut s = ScrollState::new();
        s.smooth = false;
        s.set_dimensions(800.0, 2000.0, 800.0, 600.0);
        assert!(s.can_scroll_vertically());
        assert!(s.at_top());
        assert!(!s.at_bottom());

        s.scroll_y(-500.0);
        assert!(!s.at_top());
        assert!(!s.at_bottom());

        // Scroll to bottom
        s.scroll_y(-10000.0);
        assert!(s.at_bottom());
    }

    #[test]
    fn test_scroll_clamped_to_bounds() {
        let mut s = ScrollState::new();
        s.smooth = false;
        s.set_dimensions(800.0, 1000.0, 800.0, 600.0);

        // Can't scroll past top
        s.scroll_y(1000.0);
        assert_eq!(s.offset_y, 0.0);

        // Can't scroll past bottom
        s.scroll_y(-10000.0);
        assert_eq!(s.offset_y, -400.0); // 1000 - 600
    }

    #[test]
    fn test_reset() {
        let mut s = ScrollState::new();
        s.smooth = false;
        s.set_dimensions(800.0, 2000.0, 800.0, 600.0);
        s.scroll_y(-300.0);
        assert_ne!(s.offset_y, 0.0);
        s.reset();
        assert_eq!(s.offset_y, 0.0);
    }

    #[test]
    fn test_image_position_centered_when_fits() {
        let mut s = ScrollState::new();
        s.set_dimensions(400.0, 300.0, 800.0, 600.0);
        let (x, y) = s.image_position();
        assert_eq!(x, 200.0); // (800-400)/2
        assert_eq!(y, 150.0); // (600-300)/2
    }

    #[test]
    fn test_image_position_top_left_when_overflows() {
        let mut s = ScrollState::new();
        s.smooth = false;
        s.set_dimensions(800.0, 2000.0, 800.0, 600.0);
        let (x, y) = s.image_position();
        assert_eq!(x, 0.0); // centered horizontally (same width)
        assert_eq!(y, 0.0); // at top
    }

    #[test]
    fn test_at_boundary_detection() {
        let mut s = ScrollState::new();
        s.smooth = false;
        s.set_dimensions(800.0, 1200.0, 800.0, 600.0);

        assert!(s.at_top());
        assert!(!s.at_bottom());

        s.scroll_y(-600.0); // scroll to exact bottom (1200-600=600)
        assert!(!s.at_top());
        assert!(s.at_bottom());
    }

    #[test]
    fn test_smooth_scroll_converges() {
        let mut s = ScrollState::new();
        s.smooth = true;
        s.speed = 1000.0;
        s.set_dimensions(800.0, 2000.0, 800.0, 600.0);

        s.scroll_y(-200.0);
        assert_eq!(s.offset_y, 0.0); // hasn't moved yet, only target set

        // Animate for many frames
        for _ in 0..100 {
            s.animate(0.016); // ~60fps
        }
        // Should have converged to target
        assert!((s.offset_y - (-200.0)).abs() < 1.0);
    }

    #[test]
    fn test_horizontal_scroll() {
        let mut s = ScrollState::new();
        s.smooth = false;
        s.set_dimensions(2000.0, 600.0, 800.0, 600.0);
        assert!(s.can_scroll_horizontally());
        assert!(s.at_left());
        s.scroll_x(-500.0);
        assert!(!s.at_left());
    }

    #[test]
    fn test_no_scroll_identical_size() {
        let mut s = ScrollState::new();
        s.set_dimensions(800.0, 600.0, 800.0, 600.0);
        assert!(!s.can_scroll_vertically());
        assert!(!s.can_scroll_horizontally());
    }
}
