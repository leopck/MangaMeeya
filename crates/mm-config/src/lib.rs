pub mod error;
pub mod ini_import;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use error::ConfigError;
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub window: WindowConfig,
    pub viewing: ViewingConfig,
    pub scrolling: ScrollConfig,
    pub cache: CacheConfig,
    pub image: ImageConfig,
    pub paths: PathConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
    pub save_position: bool,
    pub top_most: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewingConfig {
    pub page_mode: PageModeConfig,
    pub scale_mode: ScaleModeConfig,
    pub reading_direction: ReadingDirectionConfig,
    pub auto_dual_detect: bool,
    pub centering: bool,
    pub hide_cursor: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PageModeConfig {
    Single,
    Dual,
    DualWithCover,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScaleModeConfig {
    FitWidth,
    FitHeight,
    FitScreen,
    Original,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadingDirectionConfig {
    RightToLeft,
    LeftToRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollConfig {
    pub smooth_scroll: bool,
    pub smooth_scroll_speed: u32,
    pub smooth_scroll_accel: u32,
    pub keyboard_h_speed: u32,
    pub keyboard_v_speed: u32,
    pub mouse_wheel_h_speed: u32,
    pub mouse_wheel_v_speed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    pub prepage_forward: usize,
    pub prepage_backward: usize,
    pub cache_num: usize,
    pub file_cache_size_mb: usize,
    pub gc_limit_mb: usize,
    pub resize_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageConfig {
    pub scale_filter: u32,
    pub default_save_format: String,
    pub jpeg_quality: u8,
    pub png_compression: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct PathConfig {
    pub last_open_folder: Option<PathBuf>,
    pub save_folder: Option<PathBuf>,
}

// === Default implementations ===

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            maximized: false,
            fullscreen: false,
            save_position: true,
            top_most: false,
        }
    }
}

impl Default for ViewingConfig {
    fn default() -> Self {
        Self {
            page_mode: PageModeConfig::Dual,
            scale_mode: ScaleModeConfig::FitScreen,
            reading_direction: ReadingDirectionConfig::RightToLeft,
            auto_dual_detect: true,
            centering: false,
            hide_cursor: false,
        }
    }
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            smooth_scroll: true,
            smooth_scroll_speed: 1300,
            smooth_scroll_accel: 50,
            keyboard_h_speed: 350,
            keyboard_v_speed: 350,
            mouse_wheel_h_speed: 150,
            mouse_wheel_v_speed: 150,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            prepage_forward: 2,
            prepage_backward: 2,
            cache_num: 200,
            file_cache_size_mb: 200,
            gc_limit_mb: 300,
            resize_cache: true,
        }
    }
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            scale_filter: 6,
            default_save_format: "jpg".into(),
            jpeg_quality: 100,
            png_compression: 9,
        }
    }
}

impl Config {
    /// Load config from a TOML file, using defaults for missing fields.
    pub fn load(path: &Path) -> ConfigResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save config to a TOML file.
    pub fn save(&self, path: &Path) -> ConfigResult<()> {
        let contents = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.window.width, 640);
        assert_eq!(config.window.height, 480);
        assert_eq!(config.cache.prepage_forward, 2);
        assert_eq!(config.cache.prepage_backward, 2);
        assert_eq!(config.cache.cache_num, 200);
        assert_eq!(config.cache.file_cache_size_mb, 200);
        assert_eq!(config.cache.gc_limit_mb, 300);
        assert!(config.scrolling.smooth_scroll);
        assert_eq!(config.scrolling.smooth_scroll_speed, 1300);
        assert_eq!(config.viewing.page_mode, PageModeConfig::Dual);
        assert_eq!(
            config.viewing.reading_direction,
            ReadingDirectionConfig::RightToLeft
        );
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = Config::default();
        config.window.width = 1920;
        config.cache.cache_num = 500;
        config.save(&path).unwrap();

        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.window.width, 1920);
        assert_eq!(loaded.cache.cache_num, 500);
    }

    #[test]
    fn test_load_missing_file() {
        let config = Config::load(Path::new("/nonexistent/config.toml")).unwrap();
        assert_eq!(config.window.width, 640); // defaults
    }

    #[test]
    fn test_load_partial_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[window]\nwidth = 1024\n").unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.window.width, 1024);
        assert_eq!(config.window.height, 480); // default
        assert_eq!(config.cache.cache_num, 200); // default
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.window.width, config.window.width);
        assert_eq!(parsed.cache.gc_limit_mb, config.cache.gc_limit_mb);
    }
}
