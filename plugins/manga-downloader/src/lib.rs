pub mod queue;

use mm_plugin::{
    Chapter, PageUrl, Permission, PluginMetadata, PluginResult, PluginType, SearchResult,
    traits::{ContentSourcePlugin, Plugin},
};

/// A manga downloader plugin with a download queue.
/// This is a framework - actual source backends are registered separately.
pub struct MangaDownloaderPlugin {
    meta: PluginMetadata,
    sources: Vec<Box<dyn MangaSource>>,
}

/// Trait for individual manga source backends.
/// Each source connects to a different manga website/API.
pub trait MangaSource: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &str) -> PluginResult<Vec<SearchResult>>;
    fn get_chapters(&self, manga_id: &str) -> PluginResult<Vec<Chapter>>;
    fn get_pages(&self, chapter_id: &str) -> PluginResult<Vec<PageUrl>>;
    fn download_page(&self, url: &PageUrl) -> PluginResult<Vec<u8>>;
}

impl MangaDownloaderPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "mm.content.downloader".into(),
                name: "Manga Downloader".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                author: "MangaMeeya".into(),
                description: "Download manga from various sources".into(),
                plugin_type: PluginType::ContentSource,
                permissions: vec![Permission::NetworkAccess, Permission::FileSystemWrite],
            },
            sources: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: Box<dyn MangaSource>) {
        self.sources.push(source);
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    pub fn source_names(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.name()).collect()
    }
}

impl Default for MangaDownloaderPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MangaDownloaderPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

impl ContentSourcePlugin for MangaDownloaderPlugin {
    fn source_name(&self) -> &str {
        "Manga Downloader"
    }

    fn search(&self, query: &str) -> PluginResult<Vec<SearchResult>> {
        let mut all = Vec::new();
        for source in &self.sources {
            match source.search(query) {
                Ok(results) => all.extend(results),
                Err(e) => tracing::warn!("Search error from {}: {e}", source.name()),
            }
        }
        Ok(all)
    }

    fn get_chapters(&self, manga_id: &str) -> PluginResult<Vec<Chapter>> {
        // manga_id format: "source_index:actual_id"
        let (src_idx, actual_id) = parse_source_id(manga_id);
        if let Some(source) = self.sources.get(src_idx) {
            source.get_chapters(actual_id)
        } else {
            Err(mm_plugin::PluginError::RuntimeError(
                "Unknown source".into(),
            ))
        }
    }

    fn get_pages(&self, chapter_id: &str) -> PluginResult<Vec<PageUrl>> {
        let (src_idx, actual_id) = parse_source_id(chapter_id);
        if let Some(source) = self.sources.get(src_idx) {
            source.get_pages(actual_id)
        } else {
            Err(mm_plugin::PluginError::RuntimeError(
                "Unknown source".into(),
            ))
        }
    }

    fn download_page(&self, url: &PageUrl) -> PluginResult<Vec<u8>> {
        // Try all sources (the URL should only match one)
        for source in &self.sources {
            match source.download_page(url) {
                Ok(data) => return Ok(data),
                Err(_) => continue,
            }
        }
        Err(mm_plugin::PluginError::RuntimeError(
            "No source could download this page".into(),
        ))
    }
}

fn parse_source_id(id: &str) -> (usize, &str) {
    if let Some((idx_str, rest)) = id.split_once(':') {
        (idx_str.parse().unwrap_or(0), rest)
    } else {
        (0, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSource;

    impl MangaSource for MockSource {
        fn name(&self) -> &str {
            "MockManga"
        }
        fn search(&self, query: &str) -> PluginResult<Vec<SearchResult>> {
            Ok(vec![SearchResult {
                id: "0:mock-1".into(),
                title: format!("Mock: {query}"),
                cover_url: None,
                author: Some("Author".into()),
            }])
        }
        fn get_chapters(&self, _manga_id: &str) -> PluginResult<Vec<Chapter>> {
            Ok(vec![
                Chapter {
                    id: "0:ch1".into(),
                    title: "Chapter 1".into(),
                    number: 1.0,
                },
                Chapter {
                    id: "0:ch2".into(),
                    title: "Chapter 2".into(),
                    number: 2.0,
                },
                Chapter {
                    id: "0:ch3".into(),
                    title: "Chapter 3".into(),
                    number: 3.0,
                },
            ])
        }
        fn get_pages(&self, _chapter_id: &str) -> PluginResult<Vec<PageUrl>> {
            Ok(vec![
                PageUrl {
                    url: "https://mock.com/p1.jpg".into(),
                    headers: vec![],
                },
                PageUrl {
                    url: "https://mock.com/p2.jpg".into(),
                    headers: vec![],
                },
            ])
        }
        fn download_page(&self, _url: &PageUrl) -> PluginResult<Vec<u8>> {
            // Return fake JPEG data
            Ok(vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10])
        }
    }

    #[test]
    fn test_metadata() {
        let p = MangaDownloaderPlugin::new();
        assert_eq!(p.metadata().id, "mm.content.downloader");
        assert_eq!(p.metadata().plugin_type, PluginType::ContentSource);
        assert!(
            p.metadata()
                .permissions
                .contains(&Permission::NetworkAccess)
        );
    }

    #[test]
    fn test_add_source() {
        let mut p = MangaDownloaderPlugin::new();
        p.add_source(Box::new(MockSource));
        assert_eq!(p.source_count(), 1);
        assert_eq!(p.source_names(), vec!["MockManga"]);
    }

    #[test]
    fn test_search() {
        let mut p = MangaDownloaderPlugin::new();
        p.add_source(Box::new(MockSource));
        let results = p.search("naruto").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("naruto"));
    }

    #[test]
    fn test_get_chapters() {
        let mut p = MangaDownloaderPlugin::new();
        p.add_source(Box::new(MockSource));
        let chapters = p.get_chapters("0:mock-1").unwrap();
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].number, 1.0);
        assert_eq!(chapters[2].number, 3.0);
    }

    #[test]
    fn test_get_pages() {
        let mut p = MangaDownloaderPlugin::new();
        p.add_source(Box::new(MockSource));
        let pages = p.get_pages("0:ch1").unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].url.contains("p1.jpg"));
    }

    #[test]
    fn test_download_page() {
        let mut p = MangaDownloaderPlugin::new();
        p.add_source(Box::new(MockSource));
        let pages = p.get_pages("0:ch1").unwrap();
        let data = p.download_page(&pages[0]).unwrap();
        assert!(!data.is_empty());
        assert_eq!(data[0], 0xFF); // JPEG magic
    }

    #[test]
    fn test_no_sources() {
        let p = MangaDownloaderPlugin::new();
        let results = p.search("anything").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_source_id() {
        assert_eq!(parse_source_id("0:manga-1"), (0, "manga-1"));
        assert_eq!(parse_source_id("2:ch5"), (2, "ch5"));
        assert_eq!(parse_source_id("no-prefix"), (0, "no-prefix"));
    }
}
