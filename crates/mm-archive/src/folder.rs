use std::fs;
use std::path::{Path, PathBuf};

use crate::natural_sort::natural_sort;
use crate::{ArchiveEntry, ArchiveError, ArchiveReader, ArchiveResult, is_image_file};

/// Reads image files from a directory, treating it as an archive.
pub struct FolderReader {
    root: PathBuf,
    entries: Vec<String>,
}

impl FolderReader {
    pub fn open(path: &Path) -> ArchiveResult<Self> {
        if !path.is_dir() {
            return Err(ArchiveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Not a directory: {}", path.display()),
            )));
        }

        let mut entries: Vec<String> = Vec::new();
        collect_images(path, path, &mut entries, 0, 14)?;
        natural_sort(&mut entries);

        Ok(Self {
            root: path.to_path_buf(),
            entries,
        })
    }
}

/// Recursively collect image files up to max_depth.
fn collect_images(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<String>,
    depth: usize,
    max_depth: usize,
) -> ArchiveResult<()> {
    if depth > max_depth {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_images(root, &path, entries, depth + 1, max_depth)?;
        } else if let Some(name) = path
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .filter(|name| is_image_file(name))
        {
            entries.push(name.replace('\\', "/"));
        }
    }

    Ok(())
}

impl ArchiveReader for FolderReader {
    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn entry_names(&self) -> Vec<String> {
        self.entries.clone()
    }

    fn read_entry(&self, index: usize) -> ArchiveResult<ArchiveEntry> {
        if index >= self.entries.len() {
            return Err(ArchiveError::IndexOutOfRange {
                index,
                total: self.entries.len(),
            });
        }

        let name = &self.entries[index];
        let full_path = self.root.join(name);
        let data = fs::read(&full_path)?;

        Ok(ArchiveEntry {
            name: name.clone(),
            data,
        })
    }

    fn read_all(&self) -> ArchiveResult<Vec<ArchiveEntry>> {
        self.entries
            .iter()
            .map(|name| {
                let full_path = self.root.join(name);
                let data = fs::read(&full_path)?;
                Ok(ArchiveEntry {
                    name: name.clone(),
                    data,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_folder(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        for (name, data) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut f = fs::File::create(&path).unwrap();
            f.write_all(data).unwrap();
        }
        dir
    }

    #[test]
    fn test_folder_open() {
        let dir = create_test_folder(&[("page1.jpg", b"img1"), ("page2.png", b"img2")]);
        let reader = FolderReader::open(dir.path()).unwrap();
        assert_eq!(reader.entry_count(), 2);
    }

    #[test]
    fn test_folder_read_entry() {
        let dir = create_test_folder(&[("page1.jpg", b"img_data")]);
        let reader = FolderReader::open(dir.path()).unwrap();
        let entry = reader.read_entry(0).unwrap();
        assert_eq!(entry.data, b"img_data");
    }

    #[test]
    fn test_folder_read_all() {
        let dir = create_test_folder(&[("page1.jpg", b"d1"), ("page2.jpg", b"d2")]);
        let reader = FolderReader::open(dir.path()).unwrap();
        let entries = reader.read_all().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_folder_filter_non_images() {
        let dir = create_test_folder(&[
            ("page1.jpg", b"img"),
            ("readme.txt", b"text"),
            ("page2.png", b"img2"),
        ]);
        let reader = FolderReader::open(dir.path()).unwrap();
        assert_eq!(reader.entry_count(), 2);
    }

    #[test]
    fn test_folder_natural_sort() {
        let dir = create_test_folder(&[
            ("page10.jpg", b"d"),
            ("page2.jpg", b"d"),
            ("page1.jpg", b"d"),
        ]);
        let reader = FolderReader::open(dir.path()).unwrap();
        let names = reader.entry_names();
        assert_eq!(names, vec!["page1.jpg", "page2.jpg", "page10.jpg"]);
    }

    #[test]
    fn test_folder_nested() {
        let dir = create_test_folder(&[
            ("ch1/page1.jpg", b"d1"),
            ("ch1/page2.jpg", b"d2"),
            ("ch2/page1.jpg", b"d3"),
        ]);
        let reader = FolderReader::open(dir.path()).unwrap();
        assert_eq!(reader.entry_count(), 3);
    }

    #[test]
    fn test_folder_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let reader = FolderReader::open(dir.path()).unwrap();
        assert_eq!(reader.entry_count(), 0);
    }

    #[test]
    fn test_folder_index_out_of_range() {
        let dir = create_test_folder(&[("page1.jpg", b"d")]);
        let reader = FolderReader::open(dir.path()).unwrap();
        let result = reader.read_entry(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_folder_not_a_directory() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = FolderReader::open(tmp.path());
        assert!(result.is_err());
    }
}
