pub mod texture_cache;

/// Placeholder renderer. Will use wgpu for GPU-accelerated rendering.
/// Phase 2 implementation.
pub struct Renderer {
    _private: (),
}

impl Renderer {
    /// Create a new renderer (stub).
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
