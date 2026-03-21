pub mod manager;
pub mod traits;

pub use manager::PluginManager;
pub use traits::*;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies what kind of plugin this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginType {
    ImageFilter,
    ArchiveFormat,
    ContentSource,
    UiOverlay,
    Payment,
    Event,
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginType::ImageFilter => write!(f, "Image Filter"),
            PluginType::ArchiveFormat => write!(f, "Archive Format"),
            PluginType::ContentSource => write!(f, "Content Source"),
            PluginType::UiOverlay => write!(f, "UI Overlay"),
            PluginType::Payment => write!(f, "Payment"),
            PluginType::Event => write!(f, "Event"),
        }
    }
}

/// Permissions a plugin can request. Enforced by the plugin manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    NetworkAccess,
    FileSystemRead,
    FileSystemWrite,
    UiOverlay,
    PaymentProcessing,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permission::NetworkAccess => write!(f, "Network Access"),
            Permission::FileSystemRead => write!(f, "File System Read"),
            Permission::FileSystemWrite => write!(f, "File System Write"),
            Permission::UiOverlay => write!(f, "UI Overlay"),
            Permission::PaymentProcessing => write!(f, "Payment Processing"),
        }
    }
}

/// Metadata describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub plugin_type: PluginType,
    pub permissions: Vec<Permission>,
}

/// Plugin error type.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Permission denied: plugin '{plugin}' requires {permission}")]
    PermissionDenied {
        plugin: String,
        permission: Permission,
    },
    #[error("Plugin load error: {0}")]
    LoadError(String),
    #[error("Plugin runtime error: {0}")]
    RuntimeError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PluginResult<T> = Result<T, PluginError>;

/// Where a UI overlay plugin should be placed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlayPlacement {
    BottomBanner { height: f32 },
    BetweenPages { frequency: usize },
}

/// A search result from a content source plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub author: Option<String>,
}

/// A chapter from a content source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub number: f32,
}

/// A page URL from a content source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageUrl {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_display() {
        assert_eq!(PluginType::ImageFilter.to_string(), "Image Filter");
        assert_eq!(PluginType::ContentSource.to_string(), "Content Source");
    }

    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::NetworkAccess.to_string(), "Network Access");
    }

    #[test]
    fn test_metadata_serialize() {
        let meta = PluginMetadata {
            id: "com.test.filter".into(),
            name: "Test Filter".into(),
            version: "1.0.0".into(),
            author: "Test".into(),
            description: "A test filter".into(),
            plugin_type: PluginType::ImageFilter,
            permissions: vec![],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: PluginMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "com.test.filter");
        assert_eq!(parsed.plugin_type, PluginType::ImageFilter);
    }

    #[test]
    fn test_metadata_with_permissions() {
        let meta = PluginMetadata {
            id: "com.test.downloader".into(),
            name: "Downloader".into(),
            version: "0.1.0".into(),
            author: "Test".into(),
            description: "Downloads manga".into(),
            plugin_type: PluginType::ContentSource,
            permissions: vec![Permission::NetworkAccess, Permission::FileSystemWrite],
        };
        assert_eq!(meta.permissions.len(), 2);
        assert!(meta.permissions.contains(&Permission::NetworkAccess));
    }
}
