use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A playlist entry - an archive/folder to read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaylistEntry {
    pub path: PathBuf,
    pub label: Option<String>,
}

/// Ordered reading queue with persistence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Playlist {
    pub name: String,
    pub entries: Vec<PlaylistEntry>,
    pub current_index: usize,
}

impl Playlist {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entries: Vec::new(),
            current_index: 0,
        }
    }

    pub fn add(&mut self, path: PathBuf, label: Option<String>) {
        self.entries.push(PlaylistEntry { path, label });
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.entries.len() {
            self.entries.remove(index);
            if self.current_index >= self.entries.len() && !self.entries.is_empty() {
                self.current_index = self.entries.len() - 1;
            }
            true
        } else {
            false
        }
    }

    pub fn move_entry(&mut self, from: usize, to: usize) {
        if from < self.entries.len() && to < self.entries.len() && from != to {
            let entry = self.entries.remove(from);
            self.entries.insert(to, entry);
            // Adjust current_index
            if self.current_index == from {
                self.current_index = to;
            } else if from < self.current_index && to >= self.current_index {
                self.current_index -= 1;
            } else if from > self.current_index && to <= self.current_index {
                self.current_index += 1;
            }
        }
    }

    pub fn current(&self) -> Option<&PlaylistEntry> {
        self.entries.get(self.current_index)
    }

    pub fn advance(&mut self) -> Option<&PlaylistEntry> {
        if self.current_index + 1 < self.entries.len() {
            self.current_index += 1;
            self.current()
        } else {
            None
        }
    }

    pub fn go_back(&mut self) -> Option<&PlaylistEntry> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current()
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has_next(&self) -> bool {
        self.current_index + 1 < self.entries.len()
    }

    pub fn has_prev(&self) -> bool {
        self.current_index > 0
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
    fn test_add_entries() {
        let mut pl = Playlist::new("Reading List");
        pl.add(PathBuf::from("/a.zip"), None);
        pl.add(PathBuf::from("/b.zip"), Some("Vol 2".into()));
        assert_eq!(pl.len(), 2);
        assert_eq!(pl.entries[1].label, Some("Vol 2".into()));
    }

    #[test]
    fn test_navigate() {
        let mut pl = Playlist::new("test");
        pl.add(PathBuf::from("/a.zip"), None);
        pl.add(PathBuf::from("/b.zip"), None);
        pl.add(PathBuf::from("/c.zip"), None);

        assert_eq!(pl.current().unwrap().path, PathBuf::from("/a.zip"));
        assert!(pl.has_next());
        assert!(!pl.has_prev());

        pl.advance();
        assert_eq!(pl.current().unwrap().path, PathBuf::from("/b.zip"));

        pl.advance();
        assert_eq!(pl.current().unwrap().path, PathBuf::from("/c.zip"));
        assert!(!pl.has_next());

        pl.go_back();
        assert_eq!(pl.current().unwrap().path, PathBuf::from("/b.zip"));
    }

    #[test]
    fn test_remove() {
        let mut pl = Playlist::new("test");
        pl.add(PathBuf::from("/a.zip"), None);
        pl.add(PathBuf::from("/b.zip"), None);
        pl.remove(0);
        assert_eq!(pl.len(), 1);
        assert_eq!(pl.entries[0].path, PathBuf::from("/b.zip"));
    }

    #[test]
    fn test_move_entry() {
        let mut pl = Playlist::new("test");
        pl.add(PathBuf::from("/a.zip"), None);
        pl.add(PathBuf::from("/b.zip"), None);
        pl.add(PathBuf::from("/c.zip"), None);
        pl.move_entry(0, 2);
        assert_eq!(pl.entries[0].path, PathBuf::from("/b.zip"));
        assert_eq!(pl.entries[2].path, PathBuf::from("/a.zip"));
    }

    #[test]
    fn test_empty_playlist() {
        let pl = Playlist::new("empty");
        assert!(pl.is_empty());
        assert!(pl.current().is_none());
        assert!(!pl.has_next());
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("playlist.json");
        let mut pl = Playlist::new("My List");
        pl.add(PathBuf::from("/vol1.zip"), Some("Volume 1".into()));
        pl.add(PathBuf::from("/vol2.zip"), None);
        pl.advance();
        pl.save(&path).unwrap();
        let loaded = Playlist::load(&path);
        assert_eq!(loaded.name, "My List");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.current_index, 1);
    }
}
