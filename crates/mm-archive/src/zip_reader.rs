use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use rayon::prelude::*;

use crate::natural_sort::natural_sort;
use crate::{is_image_file, ArchiveEntry, ArchiveError, ArchiveReader, ArchiveResult};

/// ZIP archive reader that loads all image entries into RAM.
pub struct ZipArchiveReader {
    path: std::path::PathBuf,
    /// Sorted list of image entry names within the archive.
    entries: Vec<String>,
}

impl ZipArchiveReader {
    pub fn open(path: &Path) -> ArchiveResult<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let archive = zip::ZipArchive::new(reader)?;

        let mut entries: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                let entry = archive.name_for_index(i)?;
                let name = entry.to_string();
                if !name.ends_with('/') && is_image_file(&name) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        natural_sort(&mut entries);

        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }
}

impl ArchiveReader for ZipArchiveReader {
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
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut entry = archive.by_name(name)?;

        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data)?;

        Ok(ArchiveEntry {
            name: name.clone(),
            data,
        })
    }

    fn read_all(&self) -> ArchiveResult<Vec<ArchiveEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        // Read all entries sequentially (zip crate doesn't support parallel reads on one archive).
        // For parallel decompression, we read raw bytes first, then decompress in parallel.
        let mut results = Vec::with_capacity(self.entries.len());
        for name in &self.entries {
            let mut entry = archive.by_name(name)?;
            let mut data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut data)?;
            results.push(ArchiveEntry {
                name: name.clone(),
                data,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper to create a test ZIP file with synthetic image data.
    fn create_test_zip(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut writer = zip::ZipWriter::new(std::io::BufWriter::new(tmp.as_file().try_clone().unwrap()));

        for (name, data) in entries {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
        tmp
    }

    #[test]
    fn test_zip_open() {
        let tmp = create_test_zip(&[
            ("page1.jpg", b"fake_jpeg_1"),
            ("page2.jpg", b"fake_jpeg_2"),
            ("page3.png", b"fake_png_3"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 3);
    }

    #[test]
    fn test_zip_read_entry() {
        let tmp = create_test_zip(&[
            ("page1.jpg", b"jpeg_data_1"),
            ("page2.jpg", b"jpeg_data_2"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        let entry = reader.read_entry(0).unwrap();
        assert_eq!(entry.name, "page1.jpg");
        assert_eq!(entry.data, b"jpeg_data_1");
    }

    #[test]
    fn test_zip_read_all() {
        let tmp = create_test_zip(&[
            ("page1.jpg", b"data1"),
            ("page2.png", b"data2"),
            ("page3.webp", b"data3"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        let entries = reader.read_all().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].data, b"data1");
        assert_eq!(entries[1].data, b"data2");
        assert_eq!(entries[2].data, b"data3");
    }

    #[test]
    fn test_zip_natural_sort() {
        let tmp = create_test_zip(&[
            ("page10.jpg", b"d10"),
            ("page2.jpg", b"d2"),
            ("page1.jpg", b"d1"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        let names = reader.entry_names();
        assert_eq!(names, vec!["page1.jpg", "page2.jpg", "page10.jpg"]);
    }

    #[test]
    fn test_zip_filter_non_images() {
        let tmp = create_test_zip(&[
            ("page1.jpg", b"img"),
            ("readme.txt", b"text"),
            ("script.exe", b"exe"),
            ("page2.png", b"img2"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 2);
        let names = reader.entry_names();
        assert!(names.iter().all(|n| is_image_file(n)));
    }

    #[test]
    fn test_zip_empty_archive() {
        let tmp = create_test_zip(&[]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 0);
    }

    #[test]
    fn test_zip_entry_index_out_of_range() {
        let tmp = create_test_zip(&[("page1.jpg", b"data")]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        let result = reader.read_entry(5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArchiveError::IndexOutOfRange { index: 5, total: 1 }
        ));
    }

    #[test]
    fn test_zip_unicode_filenames() {
        let tmp = create_test_zip(&[
            ("ページ01.jpg", b"data1"),
            ("ページ02.png", b"data2"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 2);
        let names = reader.entry_names();
        assert!(names[0].contains("ページ"));
    }

    #[test]
    fn test_zip_corrupt_archive() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"this is not a zip file").unwrap();
        let result = ZipArchiveReader::open(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_zip_nested_folders() {
        let tmp = create_test_zip(&[
            ("chapter1/page1.jpg", b"d1"),
            ("chapter1/page2.jpg", b"d2"),
            ("chapter2/page1.jpg", b"d3"),
        ]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 3);
    }

    #[test]
    fn test_zip_large_entry() {
        let large_data = vec![0u8; 1024 * 1024]; // 1MB
        let tmp = create_test_zip(&[("large.jpg", &large_data)]);
        let reader = ZipArchiveReader::open(tmp.path()).unwrap();
        let entry = reader.read_entry(0).unwrap();
        assert_eq!(entry.data.len(), 1024 * 1024);
    }
}
