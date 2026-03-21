use crate::{Chapter, OverlayPlacement, PageUrl, PluginMetadata, PluginResult, SearchResult};
use mm_image::{ImageBuffer, ImageResult};

/// Base trait all plugins must implement.
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn on_load(&mut self) -> PluginResult<()> {
        Ok(())
    }
    fn on_unload(&mut self) -> PluginResult<()> {
        Ok(())
    }
}

/// Image processing filter plugin.
pub trait ImageFilterPlugin: Plugin {
    fn filter_name(&self) -> &str;
    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer>;
}

/// Archive format reader plugin.
pub trait ArchiveFormatPlugin: Plugin {
    fn supported_extensions(&self) -> Vec<String>;
    fn can_open(&self, path: &std::path::Path) -> bool;
    fn entry_count(&self, path: &std::path::Path) -> PluginResult<usize>;
    fn read_entry(&self, path: &std::path::Path, index: usize) -> PluginResult<Vec<u8>>;
}

/// Content source plugin (manga downloader).
pub trait ContentSourcePlugin: Plugin {
    fn source_name(&self) -> &str;
    fn search(&self, query: &str) -> PluginResult<Vec<SearchResult>>;
    fn get_chapters(&self, manga_id: &str) -> PluginResult<Vec<Chapter>>;
    fn get_pages(&self, chapter_id: &str) -> PluginResult<Vec<PageUrl>>;
    fn download_page(&self, url: &PageUrl) -> PluginResult<Vec<u8>>;
}

/// UI overlay plugin (ads, banners, notifications).
/// Ethical constraints enforced by PluginManager:
/// - NEVER covers reading area during active reading
/// - User can ALWAYS dismiss
/// - Frequency is user-configurable
/// - No audio, no popups, no redirects
pub trait UiOverlayPlugin: Plugin {
    fn placement(&self) -> OverlayPlacement;
    fn is_closeable(&self) -> bool {
        true
    }
    fn render_content(&self) -> String;
}

/// Payment/monetization plugin.
pub trait PaymentPlugin: Plugin {
    fn check_subscription(&self) -> PluginResult<bool>;
    fn get_payment_url(&self, amount_cents: u64, description: &str) -> PluginResult<String>;
}

/// Event/analytics plugin (opt-in only).
pub trait EventPlugin: Plugin {
    fn on_page_view(&self, page: usize, total: usize);
    fn on_archive_open(&self, path: &str);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Permission, PluginType};

    struct TestFilterPlugin {
        meta: PluginMetadata,
    }

    impl TestFilterPlugin {
        fn new() -> Self {
            Self {
                meta: PluginMetadata {
                    id: "test.grayscale".into(),
                    name: "Grayscale".into(),
                    version: "1.0.0".into(),
                    author: "Test".into(),
                    description: "Converts to grayscale".into(),
                    plugin_type: PluginType::ImageFilter,
                    permissions: vec![],
                },
            }
        }
    }

    impl Plugin for TestFilterPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }
    }

    impl ImageFilterPlugin for TestFilterPlugin {
        fn filter_name(&self) -> &str {
            "Grayscale"
        }
        fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
            use mm_image::filter::ImageFilter as _;
            mm_image::filter::grayscale::GrayscaleFilter.apply(image)
        }
    }

    #[test]
    fn test_filter_plugin_metadata() {
        let p = TestFilterPlugin::new();
        assert_eq!(p.metadata().id, "test.grayscale");
        assert_eq!(p.metadata().plugin_type, PluginType::ImageFilter);
    }

    #[test]
    fn test_filter_plugin_apply() {
        let p = TestFilterPlugin::new();
        let img = ImageBuffer::blank(10, 10, mm_image::ColorSpace::Rgb8);
        let result = p.apply(&img).unwrap();
        assert_eq!(result.color_space, mm_image::ColorSpace::Grayscale);
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut p = TestFilterPlugin::new();
        assert!(p.on_load().is_ok());
        assert!(p.on_unload().is_ok());
    }

    // Test a content source plugin stub
    struct MockDownloader {
        meta: PluginMetadata,
    }

    impl MockDownloader {
        fn new() -> Self {
            Self {
                meta: PluginMetadata {
                    id: "test.downloader".into(),
                    name: "Mock Downloader".into(),
                    version: "0.1.0".into(),
                    author: "Test".into(),
                    description: "Mock manga source".into(),
                    plugin_type: PluginType::ContentSource,
                    permissions: vec![Permission::NetworkAccess, Permission::FileSystemWrite],
                },
            }
        }
    }

    impl Plugin for MockDownloader {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }
    }

    impl ContentSourcePlugin for MockDownloader {
        fn source_name(&self) -> &str {
            "Mock Source"
        }
        fn search(&self, query: &str) -> PluginResult<Vec<SearchResult>> {
            Ok(vec![SearchResult {
                id: "manga-1".into(),
                title: format!("Results for: {query}"),
                cover_url: None,
                author: Some("Author".into()),
            }])
        }
        fn get_chapters(&self, _manga_id: &str) -> PluginResult<Vec<Chapter>> {
            Ok(vec![
                Chapter {
                    id: "ch1".into(),
                    title: "Chapter 1".into(),
                    number: 1.0,
                },
                Chapter {
                    id: "ch2".into(),
                    title: "Chapter 2".into(),
                    number: 2.0,
                },
            ])
        }
        fn get_pages(&self, _chapter_id: &str) -> PluginResult<Vec<PageUrl>> {
            Ok(vec![PageUrl {
                url: "https://example.com/page1.jpg".into(),
                headers: vec![],
            }])
        }
        fn download_page(&self, _url: &PageUrl) -> PluginResult<Vec<u8>> {
            Ok(vec![0xFF, 0xD8, 0xFF]) // fake JPEG header
        }
    }

    #[test]
    fn test_content_source_search() {
        let p = MockDownloader::new();
        let results = p.search("naruto").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("naruto"));
    }

    #[test]
    fn test_content_source_chapters() {
        let p = MockDownloader::new();
        let chapters = p.get_chapters("manga-1").unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].number, 1.0);
    }

    #[test]
    fn test_content_source_download() {
        let p = MockDownloader::new();
        let pages = p.get_pages("ch1").unwrap();
        let data = p.download_page(&pages[0]).unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_content_source_permissions() {
        let p = MockDownloader::new();
        assert!(
            p.metadata()
                .permissions
                .contains(&Permission::NetworkAccess)
        );
        assert!(
            p.metadata()
                .permissions
                .contains(&Permission::FileSystemWrite)
        );
    }
}
