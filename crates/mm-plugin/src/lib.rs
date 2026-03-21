use mm_image::{ImageBuffer, ImageResult};

/// Trait for dynamically loaded image filter plugins.
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}

/// A plugin that provides an image filter.
pub trait ImageFilterPlugin: Plugin {
    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer>;
}

/// Plugin manager (stub). Will handle loading/unloading of .dll/.so/.dylib plugins.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
