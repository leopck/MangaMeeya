pub mod error;
pub mod folder;
pub mod natural_sort;
pub mod zip_reader;

use std::path::Path;

pub use error::ArchiveError;

/// A single entry extracted from an archive, held entirely in memory.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub data: Vec<u8>,
}

/// Result type for archive operations.
pub type ArchiveResult<T> = Result<T, ArchiveError>;

/// Trait for reading archives of various formats.
/// All implementations load data into RAM for maximum read performance.
pub trait ArchiveReader: Send + Sync {
    /// Returns the number of image entries in the archive.
    fn entry_count(&self) -> usize;

    /// Returns the sorted list of entry names.
    fn entry_names(&self) -> Vec<String>;

    /// Read a single entry by index.
    fn read_entry(&self, index: usize) -> ArchiveResult<ArchiveEntry>;

    /// Load all entries into memory at once.
    fn read_all(&self) -> ArchiveResult<Vec<ArchiveEntry>>;
}

/// Known image file extensions.
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "avif", "jxl", "gif", "bmp", "tiff", "tif", "psd", "jp2",
];

/// Check if a filename has an image extension.
pub fn is_image_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{ext}")))
}

/// Open an archive or folder from a path, returning the appropriate reader.
pub fn open(path: &Path) -> ArchiveResult<Box<dyn ArchiveReader>> {
    if path.is_dir() {
        let reader = folder::FolderReader::open(path)?;
        Ok(Box::new(reader))
    } else {
        match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) {
            Some(ext) if ext == "zip" || ext == "cbz" => {
                let reader = zip_reader::ZipArchiveReader::open(path)?;
                Ok(Box::new(reader))
            }
            Some(ext) => Err(ArchiveError::UnsupportedFormat(ext)),
            None => Err(ArchiveError::UnsupportedFormat("(no extension)".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("page01.jpg"));
        assert!(is_image_file("page01.JPEG"));
        assert!(is_image_file("page01.png"));
        assert!(is_image_file("page01.webp"));
        assert!(is_image_file("page01.avif"));
        assert!(is_image_file("page01.gif"));
        assert!(is_image_file("page01.bmp"));
        assert!(!is_image_file("readme.txt"));
        assert!(!is_image_file("script.exe"));
        assert!(!is_image_file("data.xml"));
    }
}
