use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A record of a previously opened file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub last_page: usize,
    pub total_pages: usize,
    pub timestamp: u64,
}

/// Manages reading history with JSON persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStore {
    pub entries: Vec<HistoryEntry>,
    pub max_entries: usize,
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 100,
        }
    }
}

impl HistoryStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Record or update a history entry. Moves existing entries to the top.
    pub fn record(&mut self, path: PathBuf, last_page: usize, total_pages: usize) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Remove existing entry for this path
        self.entries.retain(|e| e.path != path);
        // Add to front (most recent)
        self.entries.insert(
            0,
            HistoryEntry {
                path,
                last_page,
                total_pages,
                timestamp: ts,
            },
        );
        // Trim to max
        self.entries.truncate(self.max_entries);
    }

    /// Get the last page read for a given path.
    pub fn last_page_for(&self, path: &Path) -> Option<usize> {
        self.entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.last_page)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
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
    fn test_record_history() {
        let mut h = HistoryStore::new(100);
        h.record(PathBuf::from("/a.zip"), 10, 50);
        assert_eq!(h.len(), 1);
        assert_eq!(h.last_page_for(Path::new("/a.zip")), Some(10));
    }

    #[test]
    fn test_update_existing() {
        let mut h = HistoryStore::new(100);
        h.record(PathBuf::from("/a.zip"), 10, 50);
        h.record(PathBuf::from("/a.zip"), 25, 50);
        assert_eq!(h.len(), 1);
        assert_eq!(h.last_page_for(Path::new("/a.zip")), Some(25));
    }

    #[test]
    fn test_most_recent_first() {
        let mut h = HistoryStore::new(100);
        h.record(PathBuf::from("/a.zip"), 1, 10);
        h.record(PathBuf::from("/b.zip"), 5, 20);
        assert_eq!(h.entries[0].path, PathBuf::from("/b.zip"));
    }

    #[test]
    fn test_max_entries() {
        let mut h = HistoryStore::new(3);
        h.record(PathBuf::from("/a.zip"), 1, 10);
        h.record(PathBuf::from("/b.zip"), 1, 10);
        h.record(PathBuf::from("/c.zip"), 1, 10);
        h.record(PathBuf::from("/d.zip"), 1, 10);
        assert_eq!(h.len(), 3);
        assert!(h.last_page_for(Path::new("/a.zip")).is_none()); // evicted
        assert!(h.last_page_for(Path::new("/d.zip")).is_some());
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("history.json");
        let mut h = HistoryStore::new(100);
        h.record(PathBuf::from("/test.zip"), 42, 100);
        h.save(&path).unwrap();
        let loaded = HistoryStore::load(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.entries[0].last_page, 42);
    }

    #[test]
    fn test_clear() {
        let mut h = HistoryStore::new(100);
        h.record(PathBuf::from("/a.zip"), 1, 10);
        h.clear();
        assert!(h.is_empty());
    }
}
