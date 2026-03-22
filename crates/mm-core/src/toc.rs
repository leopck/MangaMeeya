use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A chapter/section marker within a book.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TocEntry {
    pub page: usize,
    pub label: String,
    pub timestamp: u64,
}

/// Table of Contents for a specific book/archive.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableOfContents {
    pub path: PathBuf,
    pub entries: Vec<TocEntry>,
}

/// Manages TOC for multiple books with JSON persistence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TocStore {
    pub books: Vec<TableOfContents>,
}

impl TocStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entry(&mut self, path: &Path, page: usize, label: String) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Find or create the book's TOC
        let toc = if let Some(toc) = self.books.iter_mut().find(|b| b.path == path) {
            toc
        } else {
            self.books.push(TableOfContents {
                path: path.to_path_buf(),
                entries: Vec::new(),
            });
            self.books.last_mut().unwrap()
        };

        // Remove existing entry for same page (no duplicates)
        toc.entries.retain(|e| e.page != page);

        // Add new entry
        toc.entries.push(TocEntry {
            page,
            label,
            timestamp: ts,
        });
    }

    pub fn remove_entry(&mut self, path: &Path, page: usize) -> bool {
        if let Some(toc) = self.books.iter_mut().find(|b| b.path == path) {
            let before = toc.entries.len();
            toc.entries.retain(|e| e.page != page);
            toc.entries.len() < before
        } else {
            false
        }
    }

    pub fn get_entries(&self, path: &Path) -> Vec<&TocEntry> {
        if let Some(toc) = self.books.iter().find(|b| b.path == path) {
            let mut entries: Vec<&TocEntry> = toc.entries.iter().collect();
            entries.sort_by_key(|e| e.page);
            entries
        } else {
            Vec::new()
        }
    }

    pub fn has_entry(&self, path: &Path, page: usize) -> bool {
        self.books
            .iter()
            .find(|b| b.path == path)
            .map(|toc| toc.entries.iter().any(|e| e.page == page))
            .unwrap_or(false)
    }

    pub fn next_entry(&self, path: &Path, current_page: usize) -> Option<&TocEntry> {
        let entries = self.get_entries(path);
        entries
            .into_iter()
            .find(|e| e.page > current_page)
    }

    pub fn prev_entry(&self, path: &Path, current_page: usize) -> Option<&TocEntry> {
        let entries = self.get_entries(path);
        entries
            .into_iter()
            .rev()
            .find(|e| e.page < current_page)
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry() {
        let mut store = TocStore::new();
        store.add_entry(
            Path::new("/manga/vol1.zip"),
            10,
            "Chapter 1".to_string(),
        );
        assert_eq!(store.books.len(), 1);
        assert!(store.has_entry(Path::new("/manga/vol1.zip"), 10));
    }

    #[test]
    fn test_remove_entry() {
        let mut store = TocStore::new();
        store.add_entry(
            Path::new("/manga/vol1.zip"),
            10,
            "Chapter 1".to_string(),
        );
        assert!(store.remove_entry(Path::new("/manga/vol1.zip"), 10));
        assert!(!store.has_entry(Path::new("/manga/vol1.zip"), 10));
    }

    #[test]
    fn test_no_duplicate_pages() {
        let mut store = TocStore::new();
        store.add_entry(
            Path::new("/test.zip"),
            5,
            "First Label".to_string(),
        );
        store.add_entry(
            Path::new("/test.zip"),
            5,
            "Updated Label".to_string(),
        );
        let entries = store.get_entries(Path::new("/test.zip"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "Updated Label");
    }

    #[test]
    fn test_get_entries_sorted() {
        let mut store = TocStore::new();
        store.add_entry(Path::new("/manga.zip"), 50, "Chapter 3".to_string());
        store.add_entry(Path::new("/manga.zip"), 10, "Chapter 1".to_string());
        store.add_entry(Path::new("/manga.zip"), 30, "Chapter 2".to_string());

        let entries = store.get_entries(Path::new("/manga.zip"));
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].page, 10);
        assert_eq!(entries[1].page, 30);
        assert_eq!(entries[2].page, 50);
    }

    #[test]
    fn test_has_entry() {
        let mut store = TocStore::new();
        store.add_entry(Path::new("/test.zip"), 15, "Chapter".to_string());
        assert!(store.has_entry(Path::new("/test.zip"), 15));
        assert!(!store.has_entry(Path::new("/test.zip"), 20));
        assert!(!store.has_entry(Path::new("/other.zip"), 15));
    }

    #[test]
    fn test_next_entry() {
        let mut store = TocStore::new();
        store.add_entry(Path::new("/manga.zip"), 10, "Chapter 1".to_string());
        store.add_entry(Path::new("/manga.zip"), 30, "Chapter 2".to_string());
        store.add_entry(Path::new("/manga.zip"), 50, "Chapter 3".to_string());

        let next = store.next_entry(Path::new("/manga.zip"), 5);
        assert_eq!(next.unwrap().page, 10);

        let next = store.next_entry(Path::new("/manga.zip"), 10);
        assert_eq!(next.unwrap().page, 30);

        let next = store.next_entry(Path::new("/manga.zip"), 25);
        assert_eq!(next.unwrap().page, 30);

        let next = store.next_entry(Path::new("/manga.zip"), 50);
        assert!(next.is_none());
    }

    #[test]
    fn test_prev_entry() {
        let mut store = TocStore::new();
        store.add_entry(Path::new("/manga.zip"), 10, "Chapter 1".to_string());
        store.add_entry(Path::new("/manga.zip"), 30, "Chapter 2".to_string());
        store.add_entry(Path::new("/manga.zip"), 50, "Chapter 3".to_string());

        let prev = store.prev_entry(Path::new("/manga.zip"), 60);
        assert_eq!(prev.unwrap().page, 50);

        let prev = store.prev_entry(Path::new("/manga.zip"), 50);
        assert_eq!(prev.unwrap().page, 30);

        let prev = store.prev_entry(Path::new("/manga.zip"), 35);
        assert_eq!(prev.unwrap().page, 30);

        let prev = store.prev_entry(Path::new("/manga.zip"), 10);
        assert!(prev.is_none());
    }

    #[test]
    fn test_multiple_books() {
        let mut store = TocStore::new();
        store.add_entry(Path::new("/book1.zip"), 10, "B1 Ch1".to_string());
        store.add_entry(Path::new("/book2.zip"), 20, "B2 Ch1".to_string());
        store.add_entry(Path::new("/book1.zip"), 30, "B1 Ch2".to_string());

        assert_eq!(store.books.len(), 2);
        assert_eq!(store.get_entries(Path::new("/book1.zip")).len(), 2);
        assert_eq!(store.get_entries(Path::new("/book2.zip")).len(), 1);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("toc.json");

        let mut store = TocStore::new();
        store.add_entry(
            Path::new("/manga.zip"),
            42,
            "Chapter 3".to_string(),
        );
        store.add_entry(
            Path::new("/manga.zip"),
            10,
            "Chapter 1".to_string(),
        );
        store.save(&path).unwrap();

        let loaded = TocStore::load(&path);
        assert_eq!(loaded.books.len(), 1);
        let entries = loaded.get_entries(Path::new("/manga.zip"));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].page, 10);
        assert_eq!(entries[0].label, "Chapter 1");
        assert_eq!(entries[1].page, 42);
        assert_eq!(entries[1].label, "Chapter 3");
    }

    #[test]
    fn test_load_missing_file() {
        let store = TocStore::load(Path::new("/nonexistent/toc.json"));
        assert!(store.books.is_empty());
    }
}
