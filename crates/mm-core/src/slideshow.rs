/// Slideshow auto-advance timer.
#[derive(Debug, Clone)]
pub struct Slideshow {
    pub active: bool,
    pub interval_secs: f32,
    elapsed: f32,
    paused_by_input: bool,
    pause_timeout: f32,
    pause_elapsed: f32,
}

impl Slideshow {
    pub fn new(interval_secs: f32) -> Self {
        Self {
            active: false,
            interval_secs,
            elapsed: 0.0,
            paused_by_input: false,
            pause_timeout: 3.0,
            pause_elapsed: 0.0,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        self.elapsed = 0.0;
        self.paused_by_input = false;
    }

    pub fn start(&mut self) {
        self.active = true;
        self.elapsed = 0.0;
        self.paused_by_input = false;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Call when user provides input (mouse move, key press).
    /// Pauses slideshow temporarily.
    pub fn on_user_input(&mut self) {
        if self.active {
            self.paused_by_input = true;
            self.pause_elapsed = 0.0;
        }
    }

    /// Tick the timer. Returns true if it's time to advance to the next page.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.active {
            return false;
        }

        if self.paused_by_input {
            self.pause_elapsed += dt;
            if self.pause_elapsed >= self.pause_timeout {
                self.paused_by_input = false;
                self.elapsed = 0.0;
            }
            return false;
        }

        self.elapsed += dt;
        if self.elapsed >= self.interval_secs {
            self.elapsed -= self.interval_secs;
            true
        } else {
            false
        }
    }

    pub fn is_paused(&self) -> bool {
        self.active && self.paused_by_input
    }

    pub fn progress(&self) -> f32 {
        if !self.active || self.interval_secs <= 0.0 {
            return 0.0;
        }
        (self.elapsed / self.interval_secs).clamp(0.0, 1.0)
    }
}

impl Default for Slideshow {
    fn default() -> Self {
        Self::new(5.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inactive_no_advance() {
        let mut ss = Slideshow::new(5.0);
        assert!(!ss.active);
        assert!(!ss.tick(10.0));
    }

    #[test]
    fn test_advance_after_interval() {
        let mut ss = Slideshow::new(2.0);
        ss.start();
        assert!(!ss.tick(1.0)); // 1s elapsed
        assert!(!ss.tick(0.5)); // 1.5s elapsed
        assert!(ss.tick(1.0)); // 2.5s -> triggers at 2.0
    }

    #[test]
    fn test_pause_on_input() {
        let mut ss = Slideshow::new(2.0);
        ss.start();
        ss.tick(1.0); // 1s
        ss.on_user_input();
        assert!(ss.is_paused());
        assert!(!ss.tick(5.0)); // Would have triggered but paused
    }

    #[test]
    fn test_resume_after_pause_timeout() {
        let mut ss = Slideshow::new(1.0);
        ss.pause_timeout = 1.0;
        ss.start();
        ss.on_user_input();
        assert!(ss.is_paused());
        assert!(!ss.tick(0.5)); // Still paused
        assert!(!ss.tick(0.6)); // Pause ends (1.1s > 1.0s timeout), timer resets
        assert!(!ss.is_paused());
        assert!(ss.tick(1.1)); // Now timer runs: 1.1s > 1.0 interval -> advance
    }

    #[test]
    fn test_toggle() {
        let mut ss = Slideshow::new(5.0);
        ss.toggle();
        assert!(ss.active);
        ss.toggle();
        assert!(!ss.active);
    }

    #[test]
    fn test_progress() {
        let mut ss = Slideshow::new(4.0);
        ss.start();
        ss.tick(2.0);
        assert!((ss.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_multiple_advances() {
        let mut ss = Slideshow::new(1.0);
        ss.start();
        assert!(ss.tick(1.5)); // First advance at 1.0, 0.5 remaining
        assert!(!ss.tick(0.3)); // 0.8 total
        assert!(ss.tick(0.3)); // 1.1 total -> second advance
    }
}
