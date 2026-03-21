use crate::{Config, ConfigResult};
use std::collections::HashMap;
use std::path::Path;

/// Import settings from an original MangaMeeya INI file.
pub fn import_ini(path: &Path) -> ConfigResult<Config> {
    let contents = std::fs::read_to_string(path)?;
    let sections = parse_ini(&contents);

    let mut config = Config::default();

    if let Some(cfg) = sections.get("Config") {
        // Window
        if let Some(v) = cfg.get("StartUpWidth") {
            config.window.width = v.parse().unwrap_or(config.window.width);
        }
        if let Some(v) = cfg.get("StartUpHeight") {
            config.window.height = v.parse().unwrap_or(config.window.height);
        }
        if let Some(v) = cfg.get("FullScreen") {
            config.window.fullscreen = v == "1";
        }
        if let Some(v) = cfg.get("SaveWindowPosSize") {
            config.window.save_position = v == "1";
        }
        if let Some(v) = cfg.get("TopMost") {
            config.window.top_most = v == "1";
        }

        // Viewing
        if let Some(v) = cfg.get("PageMode") {
            config.viewing.page_mode = match v.as_str() {
                "1" => crate::PageModeConfig::Dual,
                _ => crate::PageModeConfig::Single,
            };
        }
        if let Some(v) = cfg.get("Centering") {
            config.viewing.centering = v == "1";
        }
        if let Some(v) = cfg.get("HideCursor") {
            config.viewing.hide_cursor = v == "1";
        }

        // Scrolling
        if let Some(v) = cfg.get("SmoothScroll") {
            config.scrolling.smooth_scroll = v == "1";
        }
        if let Some(v) = cfg.get("SmoothScrollSpeed") {
            config.scrolling.smooth_scroll_speed =
                v.parse().unwrap_or(config.scrolling.smooth_scroll_speed);
        }
        if let Some(v) = cfg.get("SmoothScrollAccel") {
            config.scrolling.smooth_scroll_accel =
                v.parse().unwrap_or(config.scrolling.smooth_scroll_accel);
        }
        if let Some(v) = cfg.get("KeyBoardHScrollSpeed") {
            config.scrolling.keyboard_h_speed =
                v.parse().unwrap_or(config.scrolling.keyboard_h_speed);
        }
        if let Some(v) = cfg.get("KeyBoardVScrollSpeed") {
            config.scrolling.keyboard_v_speed =
                v.parse().unwrap_or(config.scrolling.keyboard_v_speed);
        }
        if let Some(v) = cfg.get("MouseWheelHScrollSpeed") {
            config.scrolling.mouse_wheel_h_speed =
                v.parse().unwrap_or(config.scrolling.mouse_wheel_h_speed);
        }
        if let Some(v) = cfg.get("MouseWheelVScrollSpeed") {
            config.scrolling.mouse_wheel_v_speed =
                v.parse().unwrap_or(config.scrolling.mouse_wheel_v_speed);
        }

        // Cache
        if let Some(v) = cfg.get("PrepageForwardNum") {
            config.cache.prepage_forward = v.parse().unwrap_or(config.cache.prepage_forward);
        }
        if let Some(v) = cfg.get("PrepageBackwardNum") {
            config.cache.prepage_backward = v.parse().unwrap_or(config.cache.prepage_backward);
        }
        if let Some(v) = cfg.get("CacheNum") {
            config.cache.cache_num = v.parse().unwrap_or(config.cache.cache_num);
        }
        if let Some(v) = cfg.get("FileCacheSize") {
            config.cache.file_cache_size_mb = v.parse().unwrap_or(config.cache.file_cache_size_mb);
        }
        if let Some(v) = cfg.get("GCLimitSize") {
            config.cache.gc_limit_mb = v.parse().unwrap_or(config.cache.gc_limit_mb);
        }
        if let Some(v) = cfg.get("ResizeCache") {
            config.cache.resize_cache = v == "1";
        }

        // Image
        if let Some(v) = cfg.get("ScaleFilter") {
            config.image.scale_filter = v.parse().unwrap_or(config.image.scale_filter);
        }
        if let Some(v) = cfg.get("SaveJpgQuality") {
            config.image.jpeg_quality = v.parse().unwrap_or(config.image.jpeg_quality);
        }
        if let Some(v) = cfg.get("SavePngCompress") {
            config.image.png_compression = v.parse().unwrap_or(config.image.png_compression);
        }
    }

    Ok(config)
}

/// Simple INI parser: returns section -> key -> value map.
fn parse_ini(contents: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            sections.entry(current_section.clone()).or_default();
        } else if let Some((key, value)) = line.split_once('=') {
            sections
                .entry(current_section.clone())
                .or_default()
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ini_basic() {
        let ini = "[Config]\nWidth=640\nHeight=480\n";
        let sections = parse_ini(ini);
        assert_eq!(sections["Config"]["Width"], "640");
        assert_eq!(sections["Config"]["Height"], "480");
    }

    #[test]
    fn test_parse_ini_comments() {
        let ini = ";comment\n[Config]\n#also comment\nKey=Value\n";
        let sections = parse_ini(ini);
        assert_eq!(sections["Config"]["Key"], "Value");
        assert_eq!(sections["Config"].len(), 1);
    }

    #[test]
    fn test_parse_ini_multiple_sections() {
        let ini = "[A]\nX=1\n[B]\nY=2\n";
        let sections = parse_ini(ini);
        assert_eq!(sections["A"]["X"], "1");
        assert_eq!(sections["B"]["Y"], "2");
    }

    #[test]
    fn test_import_ini_from_original() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("MangaMeeya.ini");
        std::fs::write(
            &path,
            ";MangaMeeya v7.4\n\
             [Config]\n\
             StartUpWidth=640\n\
             StartUpHeight=480\n\
             FullScreen=1\n\
             SmoothScroll=1\n\
             SmoothScrollSpeed=1300\n\
             SmoothScrollAccel=50\n\
             PrepageForwardNum=2\n\
             PrepageBackwardNum=2\n\
             CacheNum=200\n\
             FileCacheSize=200\n\
             GCLimitSize=300\n\
             PageMode=1\n\
             ScaleFilter=6\n\
             SaveJpgQuality=100\n\
             SavePngCompress=9\n",
        )
        .unwrap();

        let config = import_ini(&path).unwrap();
        assert_eq!(config.window.width, 640);
        assert_eq!(config.window.height, 480);
        assert!(config.window.fullscreen);
        assert!(config.scrolling.smooth_scroll);
        assert_eq!(config.scrolling.smooth_scroll_speed, 1300);
        assert_eq!(config.cache.prepage_forward, 2);
        assert_eq!(config.cache.cache_num, 200);
        assert_eq!(config.cache.file_cache_size_mb, 200);
        assert_eq!(config.cache.gc_limit_mb, 300);
        assert_eq!(config.image.jpeg_quality, 100);
    }
}
