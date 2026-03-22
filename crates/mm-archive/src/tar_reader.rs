use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::{ArchiveEntry, ArchiveError, ArchiveReader, ArchiveResult, is_image_file};

/// TAR/TAR.GZ archive reader that loads all image entries into RAM.
///
/// Since tar is a sequential-access format (unlike zip), we load all image
/// data into memory when opening the archive.
pub struct TarArchiveReader {
    /// Pre-loaded (name, data) pairs, sorted naturally.
    entries: Vec<(String, Vec<u8>)>,
}

impl TarArchiveReader {
    pub fn open(path: &Path) -> ArchiveResult<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Try to open as gzip first, fall back to raw tar
        let entries = match Self::read_tar_entries_gzip(reader) {
            Ok(entries) => entries,
            Err(_) => {
                // Retry as raw tar
                let file = File::open(path)?;
                let reader = BufReader::new(file);
                Self::read_tar_entries_raw(reader)?
            }
        };

        Ok(Self { entries })
    }

    /// Try to read tar entries assuming gzip compression.
    fn read_tar_entries_gzip(reader: BufReader<File>) -> ArchiveResult<Vec<(String, Vec<u8>)>> {
        let decoder = flate2::read::GzDecoder::new(reader);
        let mut archive = tar::Archive::new(decoder);
        Self::extract_image_entries(&mut archive)
    }

    /// Read tar entries without decompression (raw tar).
    fn read_tar_entries_raw(reader: BufReader<File>) -> ArchiveResult<Vec<(String, Vec<u8>)>> {
        let mut archive = tar::Archive::new(reader);
        Self::extract_image_entries(&mut archive)
    }

    /// Extract all image entries from a tar archive.
    fn extract_image_entries<R: Read>(
        archive: &mut tar::Archive<R>,
    ) -> ArchiveResult<Vec<(String, Vec<u8>)>> {
        let mut image_entries = Vec::new();

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;

            // Get the entry path
            let path = entry.path()?;
            let name = path.to_string_lossy().to_string();

            // Skip directories and non-image files
            if entry.header().entry_type().is_dir() || !is_image_file(&name) {
                continue;
            }

            // Read entry data into memory
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;

            image_entries.push((name, data));
        }

        // Natural sort by filename
        image_entries.sort_by(|a, b| crate::natural_sort::natural_cmp(&a.0, &b.0));

        Ok(image_entries)
    }
}

impl ArchiveReader for TarArchiveReader {
    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn entry_names(&self) -> Vec<String> {
        self.entries.iter().map(|(name, _)| name.clone()).collect()
    }

    fn read_entry(&self, index: usize) -> ArchiveResult<ArchiveEntry> {
        if index >= self.entries.len() {
            return Err(ArchiveError::IndexOutOfRange {
                index,
                total: self.entries.len(),
            });
        }

        let (name, data) = &self.entries[index];
        Ok(ArchiveEntry {
            name: name.clone(),
            data: data.clone(),
        })
    }

    fn read_all(&self) -> ArchiveResult<Vec<ArchiveEntry>> {
        Ok(self
            .entries
            .iter()
            .map(|(name, data)| ArchiveEntry {
                name: name.clone(),
                data: data.clone(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test TAR file with synthetic image data.
    fn create_test_tar(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut builder = tar::Builder::new(tmp.as_file().try_clone().unwrap());

        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, *name, *data).unwrap();
        }

        builder.finish().unwrap();
        tmp
    }

    /// Helper to create a test TAR.GZ file with synthetic image data.
    fn create_test_tar_gz(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let encoder = flate2::write::GzEncoder::new(
            tmp.as_file().try_clone().unwrap(),
            flate2::Compression::default(),
        );
        let mut builder = tar::Builder::new(encoder);

        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, *name, *data).unwrap();
        }

        builder.finish().unwrap();
        tmp
    }

    #[test]
    fn test_tar_open() {
        let tmp = create_test_tar(&[
            ("page1.jpg", b"fake_jpeg_1"),
            ("page2.jpg", b"fake_jpeg_2"),
            ("page3.png", b"fake_png_3"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 3);
    }

    #[test]
    fn test_tar_gz_open() {
        let tmp = create_test_tar_gz(&[
            ("page1.jpg", b"fake_jpeg_1"),
            ("page2.jpg", b"fake_jpeg_2"),
            ("page3.png", b"fake_png_3"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 3);
    }

    #[test]
    fn test_tar_read_entry() {
        let tmp = create_test_tar(&[("page1.jpg", b"jpeg_data_1"), ("page2.jpg", b"jpeg_data_2")]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        let entry = reader.read_entry(0).unwrap();
        assert_eq!(entry.name, "page1.jpg");
        assert_eq!(entry.data, b"jpeg_data_1");
    }

    #[test]
    fn test_tar_natural_sort() {
        let tmp = create_test_tar(&[
            ("page10.jpg", b"d10"),
            ("page2.jpg", b"d2"),
            ("page1.jpg", b"d1"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        let names = reader.entry_names();
        assert_eq!(names, vec!["page1.jpg", "page2.jpg", "page10.jpg"]);
    }

    #[test]
    fn test_tar_filter_non_images() {
        let tmp = create_test_tar(&[
            ("page1.jpg", b"img"),
            ("readme.txt", b"text"),
            ("script.exe", b"exe"),
            ("page2.png", b"img2"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 2);
        let names = reader.entry_names();
        assert!(names.iter().all(|n| is_image_file(n)));
    }

    #[test]
    fn test_tar_empty() {
        let tmp = create_test_tar(&[]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 0);
    }

    #[test]
    fn test_tar_read_all() {
        let tmp = create_test_tar(&[
            ("page1.jpg", b"data1"),
            ("page2.png", b"data2"),
            ("page3.webp", b"data3"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        let entries = reader.read_all().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].data, b"data1");
        assert_eq!(entries[1].data, b"data2");
        assert_eq!(entries[2].data, b"data3");
    }

    #[test]
    fn test_tar_entry_index_out_of_range() {
        let tmp = create_test_tar(&[("page1.jpg", b"data")]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        let result = reader.read_entry(5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArchiveError::IndexOutOfRange { index: 5, total: 1 }
        ));
    }

    #[test]
    fn test_tar_unicode_filenames() {
        let tmp = create_test_tar(&[("ページ01.jpg", b"data1"), ("ページ02.png", b"data2")]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 2);
        let names = reader.entry_names();
        assert!(names[0].contains("ページ"));
    }

    #[test]
    fn test_tar_nested_folders() {
        let tmp = create_test_tar(&[
            ("chapter1/page1.jpg", b"d1"),
            ("chapter1/page2.jpg", b"d2"),
            ("chapter2/page1.jpg", b"d3"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        assert_eq!(reader.entry_count(), 3);
    }

    #[test]
    fn test_tar_gz_read_entry() {
        let tmp = create_test_tar_gz(&[("page1.jpg", b"jpeg_data_1"), ("page2.jpg", b"jpeg_data_2")]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        let entry = reader.read_entry(1).unwrap();
        assert_eq!(entry.name, "page2.jpg");
        assert_eq!(entry.data, b"jpeg_data_2");
    }

    #[test]
    fn test_tar_gz_natural_sort() {
        let tmp = create_test_tar_gz(&[
            ("page10.jpg", b"d10"),
            ("page2.jpg", b"d2"),
            ("page1.jpg", b"d1"),
        ]);
        let reader = TarArchiveReader::open(tmp.path()).unwrap();
        let names = reader.entry_names();
        assert_eq!(names, vec!["page1.jpg", "page2.jpg", "page10.jpg"]);
    }
}
