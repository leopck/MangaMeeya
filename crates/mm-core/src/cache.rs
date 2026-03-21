use std::collections::HashMap;
use mm_image::ImageBuffer;

/// LRU image cache that holds decoded images in memory.
pub struct ImageCache {
    entries: HashMap<usize, CacheEntry>,
    access_order: Vec<usize>,
    max_entries: usize,
    max_size_bytes: usize,
    current_size_bytes: usize,
}

struct CacheEntry {
    image: ImageBuffer,
}

impl ImageCache {
    pub fn new(max_entries: usize, max_size_mb: usize) -> Self {
        Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size_bytes: 0,
        }
    }

    /// Get a cached image by page index. Updates LRU order.
    pub fn get(&mut self, page: usize) -> Option<&ImageBuffer> {
        if self.entries.contains_key(&page) {
            // Move to end of access order (most recently used)
            self.access_order.retain(|&p| p != page);
            self.access_order.push(page);
            Some(&self.entries[&page].image)
        } else {
            None
        }
    }

    /// Check if a page is in the cache without updating LRU.
    pub fn contains(&self, page: usize) -> bool {
        self.entries.contains_key(&page)
    }

    /// Insert an image into the cache, evicting old entries if necessary.
    pub fn insert(&mut self, page: usize, image: ImageBuffer) {
        let entry_size = image.size_bytes();

        // Remove existing entry for this page if present
        if let Some(old) = self.entries.remove(&page) {
            self.current_size_bytes -= old.image.size_bytes();
            self.access_order.retain(|&p| p != page);
        }

        // Evict until we have room
        while (self.entries.len() >= self.max_entries
            || self.current_size_bytes + entry_size > self.max_size_bytes)
            && !self.access_order.is_empty()
        {
            self.evict_oldest();
        }

        self.current_size_bytes += entry_size;
        self.entries.insert(page, CacheEntry { image });
        self.access_order.push(page);
    }

    /// Evict the least recently used entry.
    fn evict_oldest(&mut self) {
        if let Some(oldest) = self.access_order.first().copied() {
            self.access_order.remove(0);
            if let Some(entry) = self.entries.remove(&oldest) {
                self.current_size_bytes -= entry.image.size_bytes();
            }
        }
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.current_size_bytes = 0;
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current total size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.current_size_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mm_image::ColorSpace;

    fn make_image(size: usize) -> ImageBuffer {
        // Create a 10x10 RGB image = 300 bytes, or custom size
        let side = ((size / 3) as f64).sqrt() as u32;
        let actual_side = side.max(1);
        ImageBuffer::blank(actual_side, actual_side, ColorSpace::Rgb8)
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = ImageCache::new(100, 100);
        let img = make_image(300);
        cache.insert(0, img.clone());
        assert!(cache.contains(0));
        let retrieved = cache.get(0).unwrap();
        assert_eq!(retrieved.width, img.width);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ImageCache::new(100, 100);
        assert!(cache.get(0).is_none());
        assert!(!cache.contains(0));
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = ImageCache::new(3, 100); // max 3 entries
        cache.insert(0, make_image(300));
        cache.insert(1, make_image(300));
        cache.insert(2, make_image(300));
        assert_eq!(cache.len(), 3);

        cache.insert(3, make_image(300)); // Should evict page 0
        assert_eq!(cache.len(), 3);
        assert!(!cache.contains(0));
        assert!(cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn test_cache_lru_access_updates_order() {
        let mut cache = ImageCache::new(3, 100);
        cache.insert(0, make_image(300));
        cache.insert(1, make_image(300));
        cache.insert(2, make_image(300));

        // Access page 0, making it most recently used
        cache.get(0);

        cache.insert(3, make_image(300)); // Should evict page 1 (least recently used)
        assert!(cache.contains(0));  // Was accessed, so not evicted
        assert!(!cache.contains(1)); // Evicted
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn test_cache_size_limit() {
        // Each 10x10 RGB image = 300 bytes. Max 1MB = plenty of room.
        // But with max_entries = 2:
        let mut cache = ImageCache::new(2, 1);
        cache.insert(0, make_image(300));
        cache.insert(1, make_image(300));
        cache.insert(2, make_image(300));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ImageCache::new(100, 100);
        cache.insert(0, make_image(300));
        cache.insert(1, make_image(300));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.size_bytes(), 0);
    }

    #[test]
    fn test_cache_replace_same_page() {
        let mut cache = ImageCache::new(100, 100);
        let img1 = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        let img2 = ImageBuffer::blank(20, 20, ColorSpace::Rgb8);
        cache.insert(0, img1);
        cache.insert(0, img2);
        assert_eq!(cache.len(), 1);
        let retrieved = cache.get(0).unwrap();
        assert_eq!(retrieved.width, 20);
    }

    #[test]
    fn test_cache_size_tracking() {
        let mut cache = ImageCache::new(100, 100);
        let img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8); // 300 bytes
        cache.insert(0, img);
        assert_eq!(cache.size_bytes(), 300);

        let img2 = ImageBuffer::blank(20, 20, ColorSpace::Rgb8); // 1200 bytes
        cache.insert(1, img2);
        assert_eq!(cache.size_bytes(), 1500);

        cache.clear();
        assert_eq!(cache.size_bytes(), 0);
    }
}
