/// Loupe (magnifier) state and calculation logic.
#[derive(Debug, Clone)]
pub struct Loupe {
    pub enabled: bool,
    pub radius: f32,
    pub magnification: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
}

/// Input parameters for loupe region computation.
pub struct LoupeInput {
    pub screen_x: f32,
    pub screen_y: f32,
    pub img_rect_x: f32,
    pub img_rect_y: f32,
    pub img_rect_w: f32,
    pub img_rect_h: f32,
}

/// The computed region to sample and display for the loupe.
#[derive(Debug, Clone)]
pub struct LoupeRegion {
    pub screen_x: f32,
    pub screen_y: f32,
    pub screen_radius: f32,
    pub uv_center_x: f32,
    pub uv_center_y: f32,
    pub uv_radius_x: f32,
    pub uv_radius_y: f32,
}

impl Loupe {
    pub fn new() -> Self {
        Self {
            enabled: false,
            radius: 80.0,
            magnification: 3.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn update_cursor(&mut self, x: f32, y: f32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn compute_region(&self, input: &LoupeInput) -> Option<LoupeRegion> {
        if !self.enabled {
            return None;
        }

        if input.screen_x < input.img_rect_x
            || input.screen_x > input.img_rect_x + input.img_rect_w
            || input.screen_y < input.img_rect_y
            || input.screen_y > input.img_rect_y + input.img_rect_h
        {
            return None;
        }

        let norm_x = (input.screen_x - input.img_rect_x) / input.img_rect_w;
        let norm_y = (input.screen_y - input.img_rect_y) / input.img_rect_h;
        let uv_radius_x = (self.radius / input.img_rect_w) / self.magnification;
        let uv_radius_y = (self.radius / input.img_rect_h) / self.magnification;

        Some(LoupeRegion {
            screen_x: input.screen_x,
            screen_y: input.screen_y,
            screen_radius: self.radius,
            uv_center_x: norm_x.clamp(uv_radius_x, 1.0 - uv_radius_x),
            uv_center_y: norm_y.clamp(uv_radius_y, 1.0 - uv_radius_y),
            uv_radius_x,
            uv_radius_y,
        })
    }
}

impl Default for Loupe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(sx: f32, sy: f32) -> LoupeInput {
        LoupeInput {
            screen_x: sx,
            screen_y: sy,
            img_rect_x: 0.0,
            img_rect_y: 0.0,
            img_rect_w: 800.0,
            img_rect_h: 600.0,
        }
    }

    #[test]
    fn test_loupe_disabled_returns_none() {
        let loupe = Loupe::new();
        assert!(loupe.compute_region(&input(100.0, 100.0)).is_none());
    }

    #[test]
    fn test_loupe_enabled_returns_region() {
        let mut loupe = Loupe::new();
        loupe.toggle();
        let r = loupe.compute_region(&input(400.0, 300.0)).unwrap();
        assert_eq!(r.screen_x, 400.0);
        assert_eq!(r.screen_y, 300.0);
        assert_eq!(r.screen_radius, 80.0);
    }

    #[test]
    fn test_loupe_outside_image_returns_none() {
        let mut loupe = Loupe::new();
        loupe.toggle();
        assert!(loupe.compute_region(&input(900.0, 300.0)).is_none());
    }

    #[test]
    fn test_loupe_uv_center() {
        let mut loupe = Loupe::new();
        loupe.toggle();
        loupe.magnification = 2.0;
        let r = loupe.compute_region(&input(400.0, 300.0)).unwrap();
        assert!((r.uv_center_x - 0.5).abs() < 0.01);
        assert!((r.uv_center_y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_toggle() {
        let mut loupe = Loupe::new();
        assert!(!loupe.enabled);
        loupe.toggle();
        assert!(loupe.enabled);
        loupe.toggle();
        assert!(!loupe.enabled);
    }
}
