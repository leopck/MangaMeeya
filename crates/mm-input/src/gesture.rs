use crate::GestureDirection;
use std::time::Instant;

/// Recognizes mouse gesture patterns from a sequence of mouse movements.
pub struct GestureRecognizer {
    pub unit_length: u32,
    pub split_time_ms: u32,
    pub threshold_degrees: u32,

    segments: Vec<GestureDirection>,
    last_pos: Option<(f64, f64)>,
    last_time: Option<Instant>,
    accumulated_dx: f64,
    accumulated_dy: f64,
}

impl GestureRecognizer {
    pub fn new(unit_length: u32, split_time_ms: u32, threshold_degrees: u32) -> Self {
        Self {
            unit_length,
            split_time_ms,
            threshold_degrees,
            segments: Vec::new(),
            last_pos: None,
            last_time: None,
            accumulated_dx: 0.0,
            accumulated_dy: 0.0,
        }
    }

    /// Default recognizer matching original MangaMeeya settings.
    pub fn default_config() -> Self {
        Self::new(30, 100, 60)
    }

    /// Start tracking a new gesture.
    pub fn begin(&mut self, x: f64, y: f64) {
        self.segments.clear();
        self.last_pos = Some((x, y));
        self.last_time = Some(Instant::now());
        self.accumulated_dx = 0.0;
        self.accumulated_dy = 0.0;
    }

    /// Update with a new mouse position during gesture tracking.
    pub fn update(&mut self, x: f64, y: f64) {
        let Some((lx, ly)) = self.last_pos else {
            return;
        };

        let now = Instant::now();

        // Check for time-based split
        if let Some(last_time) = self.last_time {
            if now.duration_since(last_time).as_millis() > self.split_time_ms as u128 {
                self.accumulated_dx = 0.0;
                self.accumulated_dy = 0.0;
            }
        }

        self.accumulated_dx += x - lx;
        self.accumulated_dy += y - ly;
        self.last_pos = Some((x, y));
        self.last_time = Some(now);

        let dist = (self.accumulated_dx.powi(2) + self.accumulated_dy.powi(2)).sqrt();
        if dist >= self.unit_length as f64 {
            if let Some(dir) = classify_direction(self.accumulated_dx, self.accumulated_dy, self.threshold_degrees) {
                // Avoid duplicate consecutive segments
                if self.segments.last() != Some(&dir) {
                    self.segments.push(dir);
                }
            }
            self.accumulated_dx = 0.0;
            self.accumulated_dy = 0.0;
        }
    }

    /// End the gesture and return the recognized pattern.
    pub fn end(&mut self) -> Vec<GestureDirection> {
        let result = self.segments.clone();
        self.segments.clear();
        self.last_pos = None;
        self.last_time = None;
        self.accumulated_dx = 0.0;
        self.accumulated_dy = 0.0;
        result
    }
}

fn classify_direction(dx: f64, dy: f64, _threshold: u32) -> Option<GestureDirection> {
    if dx.abs() > dy.abs() {
        if dx > 0.0 {
            Some(GestureDirection::Right)
        } else {
            Some(GestureDirection::Left)
        }
    } else if dy.abs() > dx.abs() {
        if dy > 0.0 {
            Some(GestureDirection::Down)
        } else {
            Some(GestureDirection::Up)
        }
    } else {
        None // Diagonal / ambiguous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gesture_right() {
        let mut g = GestureRecognizer::new(10, 1000, 60);
        g.begin(0.0, 0.0);
        g.update(50.0, 0.0);
        let result = g.end();
        assert_eq!(result, vec![GestureDirection::Right]);
    }

    #[test]
    fn test_gesture_left() {
        let mut g = GestureRecognizer::new(10, 1000, 60);
        g.begin(50.0, 0.0);
        g.update(0.0, 0.0);
        let result = g.end();
        assert_eq!(result, vec![GestureDirection::Left]);
    }

    #[test]
    fn test_gesture_up() {
        let mut g = GestureRecognizer::new(10, 1000, 60);
        g.begin(0.0, 50.0);
        g.update(0.0, 0.0);
        let result = g.end();
        assert_eq!(result, vec![GestureDirection::Up]);
    }

    #[test]
    fn test_gesture_down() {
        let mut g = GestureRecognizer::new(10, 1000, 60);
        g.begin(0.0, 0.0);
        g.update(0.0, 50.0);
        let result = g.end();
        assert_eq!(result, vec![GestureDirection::Down]);
    }

    #[test]
    fn test_gesture_below_threshold() {
        let mut g = GestureRecognizer::new(100, 1000, 60);
        g.begin(0.0, 0.0);
        g.update(5.0, 0.0); // Less than unit_length
        let result = g.end();
        assert!(result.is_empty());
    }

    #[test]
    fn test_gesture_up_down() {
        let mut g = GestureRecognizer::new(10, 1000, 60);
        g.begin(0.0, 50.0);
        g.update(0.0, 0.0);  // Up
        g.update(0.0, 50.0); // Down
        let result = g.end();
        assert_eq!(result, vec![GestureDirection::Up, GestureDirection::Down]);
    }

    #[test]
    fn test_gesture_no_duplicate_segments() {
        let mut g = GestureRecognizer::new(10, 1000, 60);
        g.begin(0.0, 0.0);
        g.update(20.0, 0.0);  // Right
        g.update(40.0, 0.0);  // Still right (should not duplicate)
        let result = g.end();
        assert_eq!(result, vec![GestureDirection::Right]);
    }
}
