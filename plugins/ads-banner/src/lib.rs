use mm_plugin::{
    OverlayPlacement, Permission, PluginMetadata, PluginType,
    traits::{Plugin, UiOverlayPlugin},
};
use serde::{Deserialize, Serialize};

/// Non-intrusive banner ad plugin.
///
/// Ethical constraints (enforced by both plugin and PluginManager):
/// - NEVER covers the reading area during active reading
/// - User can ALWAYS close/dismiss the banner
/// - Frequency is user-configurable (can be disabled entirely)
/// - No audio, no popups, no redirects
/// - No tracking without explicit opt-in
pub struct AdsBannerPlugin {
    meta: PluginMetadata,
    config: AdsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdsConfig {
    /// Whether ads are enabled at all.
    pub enabled: bool,
    /// Height of the bottom banner in pixels.
    pub banner_height: f32,
    /// Minimum seconds of idle before showing a banner.
    pub idle_threshold_secs: f32,
    /// Whether the user has opted into usage analytics.
    pub analytics_opt_in: bool,
}

impl Default for AdsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            banner_height: 50.0,
            idle_threshold_secs: 30.0,
            analytics_opt_in: false,
        }
    }
}

impl AdsBannerPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "mm.overlay.ads-banner".into(),
                name: "Non-Intrusive Ads".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                author: "MangaMeeya".into(),
                description:
                    "Non-intrusive bottom banner ads. Always dismissible, never blocks reading."
                        .into(),
                plugin_type: PluginType::UiOverlay,
                permissions: vec![Permission::UiOverlay, Permission::NetworkAccess],
            },
            config: AdsConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AdsConfig) -> Self {
        self.config = config;
        self
    }

    pub fn config(&self) -> &AdsConfig {
        &self.config
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Default for AdsBannerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for AdsBannerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

impl UiOverlayPlugin for AdsBannerPlugin {
    fn placement(&self) -> OverlayPlacement {
        OverlayPlacement::BottomBanner {
            height: self.config.banner_height,
        }
    }

    fn is_closeable(&self) -> bool {
        true // ALWAYS closeable
    }

    fn render_content(&self) -> String {
        if !self.config.enabled {
            return String::new();
        }
        // In a real implementation, this would fetch ad content from a server.
        // For now, return a placeholder.
        "Support MangaMeeya - Consider upgrading to Pro!".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let p = AdsBannerPlugin::new();
        assert_eq!(p.metadata().id, "mm.overlay.ads-banner");
        assert_eq!(p.metadata().plugin_type, PluginType::UiOverlay);
    }

    #[test]
    fn test_always_closeable() {
        let p = AdsBannerPlugin::new();
        assert!(p.is_closeable());
    }

    #[test]
    fn test_placement() {
        let p = AdsBannerPlugin::new();
        match p.placement() {
            OverlayPlacement::BottomBanner { height } => {
                assert_eq!(height, 50.0);
            }
            _ => panic!("Expected BottomBanner"),
        }
    }

    #[test]
    fn test_render_content() {
        let p = AdsBannerPlugin::new();
        let content = p.render_content();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_disabled_returns_empty() {
        let mut p = AdsBannerPlugin::new();
        p.set_enabled(false);
        assert!(p.render_content().is_empty());
    }

    #[test]
    fn test_custom_config() {
        let config = AdsConfig {
            enabled: true,
            banner_height: 80.0,
            idle_threshold_secs: 60.0,
            analytics_opt_in: false,
        };
        let p = AdsBannerPlugin::new().with_config(config);
        assert_eq!(p.config().banner_height, 80.0);
        assert_eq!(p.config().idle_threshold_secs, 60.0);
    }

    #[test]
    fn test_default_config() {
        let p = AdsBannerPlugin::new();
        assert!(p.is_enabled());
        assert_eq!(p.config().banner_height, 50.0);
        assert_eq!(p.config().idle_threshold_secs, 30.0);
        assert!(!p.config().analytics_opt_in);
    }

    #[test]
    fn test_permissions() {
        let p = AdsBannerPlugin::new();
        assert!(p.metadata().permissions.contains(&Permission::UiOverlay));
        assert!(
            p.metadata()
                .permissions
                .contains(&Permission::NetworkAccess)
        );
    }
}
