use std::collections::HashMap;

/// GPU texture cache (stub). Will hold wgpu::Texture references.
pub struct TextureCache {
    max_entries: usize,
    entries: HashMap<usize, TextureEntry>,
    access_order: Vec<usize>,
}

struct TextureEntry {
    width: u32,
    height: u32,
    // In the real implementation: wgpu::Texture
}

impl TextureCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    pub fn contains(&self, page: usize) -> bool {
        self.entries.contains_key(&page)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(&mut self, page: usize, width: u32, height: u32) {
        if self.entries.len() >= self.max_entries && !self.access_order.is_empty() {
            let oldest = self.access_order.remove(0);
            self.entries.remove(&oldest);
        }
        self.entries.insert(page, TextureEntry { width, height });
        self.access_order.retain(|&p| p != page);
        self.access_order.push(page);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_cache_insert() {
        let mut cache = TextureCache::new(10);
        cache.insert(0, 1920, 1080);
        assert!(cache.contains(0));
    }

    #[test]
    fn test_texture_cache_eviction() {
        let mut cache = TextureCache::new(2);
        cache.insert(0, 100, 100);
        cache.insert(1, 100, 100);
        cache.insert(2, 100, 100);
        assert!(!cache.contains(0));
        assert!(cache.contains(1));
        assert!(cache.contains(2));
    }

    #[test]
    fn test_texture_cache_clear() {
        let mut cache = TextureCache::new(10);
        cache.insert(0, 100, 100);
        cache.clear();
        assert!(cache.is_empty());
    }
}
