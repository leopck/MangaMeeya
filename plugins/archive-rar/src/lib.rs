use mm_plugin::{
    Permission, PluginError, PluginMetadata, PluginResult, PluginType,
    traits::{ArchiveFormatPlugin, Plugin},
};
use std::path::Path;

/// RAR/CBR archive format plugin.
///
/// Uses the system's `unrar` library if available.
/// On systems without unrar, falls back to calling the `unrar` command-line tool.
pub struct RarPlugin {
    meta: PluginMetadata,
}

impl RarPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "mm.archive.rar".into(),
                name: "RAR Archive Support".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                author: "MangaMeeya".into(),
                description: "Adds RAR/CBR archive support".into(),
                plugin_type: PluginType::ArchiveFormat,
                permissions: vec![Permission::FileSystemRead],
            },
        }
    }
}

impl Default for RarPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RarPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

impl ArchiveFormatPlugin for RarPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["rar".into(), "cbr".into()]
    }

    fn can_open(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("rar") || e.eq_ignore_ascii_case("cbr"))
    }

    fn entry_count(&self, path: &Path) -> PluginResult<usize> {
        // Use unrar CLI to list entries
        let output = std::process::Command::new("unrar")
            .args(["lb", &path.to_string_lossy()])
            .output()
            .map_err(|e| {
                PluginError::RuntimeError(format!(
                    "unrar not found. Install unrar to read RAR files: {e}"
                ))
            })?;

        if !output.status.success() {
            return Err(PluginError::RuntimeError(format!(
                "unrar failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        let count = listing.lines().filter(|l| is_image_name(l)).count();
        Ok(count)
    }

    fn read_entry(&self, path: &Path, index: usize) -> PluginResult<Vec<u8>> {
        // List entries to find the target name
        let output = std::process::Command::new("unrar")
            .args(["lb", &path.to_string_lossy()])
            .output()
            .map_err(|e| PluginError::RuntimeError(format!("unrar not found: {e}")))?;

        let listing = String::from_utf8_lossy(&output.stdout);
        let images: Vec<&str> = listing.lines().filter(|l| is_image_name(l)).collect();
        let name = images
            .get(index)
            .ok_or_else(|| PluginError::RuntimeError(format!("Index {index} out of range")))?;

        // Extract single file to stdout
        let extract = std::process::Command::new("unrar")
            .args(["p", "-inul", &path.to_string_lossy(), name])
            .output()
            .map_err(|e| PluginError::RuntimeError(format!("unrar extract error: {e}")))?;

        if !extract.status.success() {
            return Err(PluginError::RuntimeError(format!(
                "unrar extract failed: {}",
                String::from_utf8_lossy(&extract.stderr)
            )));
        }

        Ok(extract.stdout)
    }
}

fn is_image_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif", "tiff"]
        .iter()
        .any(|ext| lower.ends_with(&format!(".{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let p = RarPlugin::new();
        assert_eq!(p.metadata().id, "mm.archive.rar");
        assert_eq!(p.metadata().plugin_type, PluginType::ArchiveFormat);
    }

    #[test]
    fn test_supported_extensions() {
        let p = RarPlugin::new();
        let exts = p.supported_extensions();
        assert!(exts.contains(&"rar".to_string()));
        assert!(exts.contains(&"cbr".to_string()));
    }

    #[test]
    fn test_can_open() {
        let p = RarPlugin::new();
        assert!(p.can_open(Path::new("manga.rar")));
        assert!(p.can_open(Path::new("comic.CBR")));
        assert!(!p.can_open(Path::new("archive.zip")));
    }

    #[test]
    fn test_is_image_name() {
        assert!(is_image_name("page01.jpg"));
        assert!(is_image_name("image.PNG"));
        assert!(!is_image_name("readme.txt"));
    }
}
