use mm_plugin::{
    Permission, PluginError, PluginMetadata, PluginResult, PluginType,
    traits::{ArchiveFormatPlugin, Plugin},
};
use std::path::Path;

/// 7z/CB7 archive format plugin.
pub struct SevenZipPlugin {
    meta: PluginMetadata,
}

impl SevenZipPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "mm.archive.7z".into(),
                name: "7z Archive Support".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                author: "MangaMeeya".into(),
                description: "Adds 7z/CB7 archive support".into(),
                plugin_type: PluginType::ArchiveFormat,
                permissions: vec![Permission::FileSystemRead],
            },
        }
    }
}

impl Default for SevenZipPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SevenZipPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

impl ArchiveFormatPlugin for SevenZipPlugin {
    fn supported_extensions(&self) -> Vec<String> {
        vec!["7z".into(), "cb7".into()]
    }

    fn can_open(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("7z") || e.eq_ignore_ascii_case("cb7"))
    }

    fn entry_count(&self, path: &Path) -> PluginResult<usize> {
        let archive = sevenz_rust::SevenZReader::open(path, sevenz_rust::Password::empty())
            .map_err(|e| PluginError::RuntimeError(format!("7z open error: {e}")))?;
        let count = archive
            .archive()
            .files
            .iter()
            .filter(|f| !f.is_directory() && is_image_name(f.name()))
            .count();
        Ok(count)
    }

    fn read_entry(&self, path: &Path, index: usize) -> PluginResult<Vec<u8>> {
        let mut archive = sevenz_rust::SevenZReader::open(path, sevenz_rust::Password::empty())
            .map_err(|e| PluginError::RuntimeError(format!("7z open error: {e}")))?;

        // Collect image file names
        let image_names: Vec<String> = archive
            .archive()
            .files
            .iter()
            .filter(|f| !f.is_directory() && is_image_name(f.name()))
            .map(|f| f.name().to_string())
            .collect();

        let target_name = image_names
            .get(index)
            .ok_or_else(|| PluginError::RuntimeError(format!("Index {index} out of range")))?
            .clone();

        let mut result: Option<Vec<u8>> = None;
        archive
            .for_each_entries(|entry, reader| {
                if entry.name() == target_name {
                    let mut buf = Vec::new();
                    std::io::Read::read_to_end(reader, &mut buf)?;
                    result = Some(buf);
                }
                Ok(true)
            })
            .map_err(|e| PluginError::RuntimeError(format!("7z extract error: {e}")))?;

        result.ok_or_else(|| PluginError::RuntimeError(format!("Entry '{target_name}' not found")))
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
        let p = SevenZipPlugin::new();
        assert_eq!(p.metadata().id, "mm.archive.7z");
        assert_eq!(p.metadata().plugin_type, PluginType::ArchiveFormat);
    }

    #[test]
    fn test_supported_extensions() {
        let p = SevenZipPlugin::new();
        let exts = p.supported_extensions();
        assert!(exts.contains(&"7z".to_string()));
        assert!(exts.contains(&"cb7".to_string()));
    }

    #[test]
    fn test_can_open() {
        let p = SevenZipPlugin::new();
        assert!(p.can_open(Path::new("manga.7z")));
        assert!(p.can_open(Path::new("comic.CB7")));
        assert!(!p.can_open(Path::new("archive.zip")));
    }

    #[test]
    fn test_entry_count_missing_file() {
        let p = SevenZipPlugin::new();
        assert!(p.entry_count(Path::new("/nonexistent.7z")).is_err());
    }
}
