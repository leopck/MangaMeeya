use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A saved reading position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub path: PathBuf,
    pub page: usize,
    pub timestamp: u64,
    pub label: Option<String>,
}

/// Manages reading bookmarks with JSON persistence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookmarkStore {
    pub bookmarks: Vec<Bookmark>,
}

impl BookmarkStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: PathBuf, page: usize, label: Option<String>) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Remove existing bookmark for same path+page
        self.bookmarks
            .retain(|b| !(b.path == path && b.page == page));
        self.bookmarks.push(Bookmark {
            path,
            page,
            timestamp: ts,
            label,
        });
    }

    pub fn remove(&mut self, path: &Path, page: usize) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks
            .retain(|b| !(b.path == path && b.page == page));
        self.bookmarks.len() < before
    }

    pub fn find(&self, path: &Path) -> Vec<&Bookmark> {
        self.bookmarks.iter().filter(|b| b.path == path).collect()
    }

    pub fn is_bookmarked(&self, path: &Path, page: usize) -> bool {
        self.bookmarks
            .iter()
            .any(|b| b.path == path && b.page == page)
    }

    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
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
    fn test_add_bookmark() {
        let mut store = BookmarkStore::new();
        store.add(PathBuf::from("/manga/vol1.zip"), 10, None);
        assert_eq!(store.len(), 1);
        assert!(store.is_bookmarked(Path::new("/manga/vol1.zip"), 10));
    }

    #[test]
    fn test_remove_bookmark() {
        let mut store = BookmarkStore::new();
        store.add(PathBuf::from("/manga/vol1.zip"), 10, None);
        assert!(store.remove(Path::new("/manga/vol1.zip"), 10));
        assert!(store.is_empty());
    }

    #[test]
    fn test_no_duplicates() {
        let mut store = BookmarkStore::new();
        store.add(PathBuf::from("/test.zip"), 5, None);
        store.add(PathBuf::from("/test.zip"), 5, Some("label".into()));
        assert_eq!(store.len(), 1);
        assert_eq!(store.bookmarks[0].label, Some("label".into()));
    }

    #[test]
    fn test_find_by_path() {
        let mut store = BookmarkStore::new();
        store.add(PathBuf::from("/a.zip"), 1, None);
        store.add(PathBuf::from("/a.zip"), 5, None);
        store.add(PathBuf::from("/b.zip"), 3, None);
        assert_eq!(store.find(Path::new("/a.zip")).len(), 2);
        assert_eq!(store.find(Path::new("/b.zip")).len(), 1);
        assert_eq!(store.find(Path::new("/c.zip")).len(), 0);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bookmarks.json");
        let mut store = BookmarkStore::new();
        store.add(PathBuf::from("/test.zip"), 42, Some("ch3".into()));
        store.save(&path).unwrap();
        let loaded = BookmarkStore::load(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.bookmarks[0].page, 42);
        assert_eq!(loaded.bookmarks[0].label, Some("ch3".into()));
    }

    #[test]
    fn test_load_missing_file() {
        let store = BookmarkStore::load(Path::new("/nonexistent/bookmarks.json"));
        assert!(store.is_empty());
    }
}
