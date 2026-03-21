/// Mouse click zone detection for page navigation.
/// Divides the viewport into regions that trigger different commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickZone {
    Left,
    Center,
    Right,
}

/// Determine which zone a click falls into based on viewport width.
pub fn detect_click_zone(click_x: f64, viewport_width: f64) -> ClickZone {
    let ratio = click_x / viewport_width;
    if ratio < 0.33 {
        ClickZone::Left
    } else if ratio > 0.67 {
        ClickZone::Right
    } else {
        ClickZone::Center
    }
}

/// Double-click detection state.
pub struct DoubleClickDetector {
    last_click_time: Option<std::time::Instant>,
    interval_ms: u32,
}

impl DoubleClickDetector {
    pub fn new(interval_ms: u32) -> Self {
        Self {
            last_click_time: None,
            interval_ms,
        }
    }

    /// Returns true if this click constitutes a double-click.
    pub fn click(&mut self) -> bool {
        let now = std::time::Instant::now();
        let is_double = self
            .last_click_time
            .map(|t| now.duration_since(t).as_millis() < self.interval_ms as u128)
            .unwrap_or(false);

        if is_double {
            self.last_click_time = None; // Reset after detecting
        } else {
            self.last_click_time = Some(now);
        }

        is_double
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_zone_left() {
        assert_eq!(detect_click_zone(100.0, 1000.0), ClickZone::Left);
    }

    #[test]
    fn test_click_zone_center() {
        assert_eq!(detect_click_zone(500.0, 1000.0), ClickZone::Center);
    }

    #[test]
    fn test_click_zone_right() {
        assert_eq!(detect_click_zone(800.0, 1000.0), ClickZone::Right);
    }

    #[test]
    fn test_click_zone_boundaries() {
        assert_eq!(detect_click_zone(329.0, 1000.0), ClickZone::Left);
        assert_eq!(detect_click_zone(330.0, 1000.0), ClickZone::Center);
        assert_eq!(detect_click_zone(670.0, 1000.0), ClickZone::Center);
        assert_eq!(detect_click_zone(671.0, 1000.0), ClickZone::Right);
    }

    #[test]
    fn test_double_click_fast() {
        let mut det = DoubleClickDetector::new(500);
        assert!(!det.click()); // First click
        assert!(det.click()); // Quick second click = double
    }

    #[test]
    fn test_double_click_resets() {
        let mut det = DoubleClickDetector::new(500);
        det.click();
        det.click(); // double
        assert!(!det.click()); // Should be a new first click
    }
}
