use crate::{Permission, PluginError, PluginMetadata, PluginResult, PluginType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Configuration for which plugins are enabled and their granted permissions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    pub enabled: HashMap<String, bool>,
    pub granted_permissions: HashMap<String, Vec<Permission>>,
}

/// Manages plugin discovery, registration, permissions, and lifecycle.
pub struct PluginManager {
    plugins_dir: PathBuf,
    registered: Vec<RegisteredPlugin>,
    config: PluginConfig,
    /// Permissions the user has granted to each plugin.
    granted: HashMap<String, HashSet<Permission>>,
}

/// A plugin registered with the manager (not necessarily loaded).
struct RegisteredPlugin {
    metadata: PluginMetadata,
    enabled: bool,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        let config_path = plugins_dir.join("plugins.json");
        let config = Self::load_config(&config_path);
        let granted = config
            .granted_permissions
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().copied().collect()))
            .collect();
        Self {
            plugins_dir,
            registered: Vec::new(),
            config,
            granted,
        }
    }

    /// Register a plugin with the manager.
    pub fn register(&mut self, metadata: PluginMetadata) {
        let enabled = self
            .config
            .enabled
            .get(&metadata.id)
            .copied()
            .unwrap_or(true);
        self.registered.push(RegisteredPlugin { metadata, enabled });
    }

    /// Check if a plugin has a specific permission granted.
    pub fn has_permission(&self, plugin_id: &str, permission: Permission) -> bool {
        self.granted
            .get(plugin_id)
            .is_some_and(|perms| perms.contains(&permission))
    }

    /// Grant a permission to a plugin.
    pub fn grant_permission(&mut self, plugin_id: &str, permission: Permission) {
        self.granted
            .entry(plugin_id.to_string())
            .or_default()
            .insert(permission);
    }

    /// Revoke a permission from a plugin.
    pub fn revoke_permission(&mut self, plugin_id: &str, permission: Permission) {
        if let Some(perms) = self.granted.get_mut(plugin_id) {
            perms.remove(&permission);
        }
    }

    /// Check all requested permissions for a plugin. Returns Err if any are missing.
    pub fn check_permissions(&self, metadata: &PluginMetadata) -> PluginResult<()> {
        for perm in &metadata.permissions {
            if !self.has_permission(&metadata.id, *perm) {
                return Err(PluginError::PermissionDenied {
                    plugin: metadata.id.clone(),
                    permission: *perm,
                });
            }
        }
        Ok(())
    }

    /// Grant all permissions a plugin requests.
    pub fn grant_all_requested(&mut self, metadata: &PluginMetadata) {
        for perm in &metadata.permissions {
            self.grant_permission(&metadata.id, *perm);
        }
    }

    /// Enable or disable a plugin by ID.
    pub fn set_enabled(&mut self, plugin_id: &str, enabled: bool) {
        for p in &mut self.registered {
            if p.metadata.id == plugin_id {
                p.enabled = enabled;
            }
        }
        self.config.enabled.insert(plugin_id.to_string(), enabled);
    }

    /// Check if a plugin is enabled.
    pub fn is_enabled(&self, plugin_id: &str) -> bool {
        self.registered
            .iter()
            .find(|p| p.metadata.id == plugin_id)
            .is_some_and(|p| p.enabled)
    }

    /// Get all registered plugin metadata.
    pub fn list_plugins(&self) -> Vec<(&PluginMetadata, bool)> {
        self.registered
            .iter()
            .map(|p| (&p.metadata, p.enabled))
            .collect()
    }

    /// Get plugins of a specific type.
    pub fn plugins_of_type(&self, plugin_type: PluginType) -> Vec<&PluginMetadata> {
        self.registered
            .iter()
            .filter(|p| p.enabled && p.metadata.plugin_type == plugin_type)
            .map(|p| &p.metadata)
            .collect()
    }

    /// Number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.registered.len()
    }

    /// Number of enabled plugins.
    pub fn enabled_count(&self) -> usize {
        self.registered.iter().filter(|p| p.enabled).count()
    }

    /// Save plugin configuration.
    pub fn save_config(&self) -> std::io::Result<()> {
        let mut config = self.config.clone();
        for (id, perms) in &self.granted {
            config
                .granted_permissions
                .insert(id.clone(), perms.iter().copied().collect());
        }
        let path = self.plugins_dir.join("plugins.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(path, json)
    }

    fn load_config(path: &Path) -> PluginConfig {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata(id: &str, ptype: PluginType, perms: Vec<Permission>) -> PluginMetadata {
        PluginMetadata {
            id: id.into(),
            name: id.into(),
            version: "1.0".into(),
            author: "Test".into(),
            description: "Test plugin".into(),
            plugin_type: ptype,
            permissions: perms,
        }
    }

    #[test]
    fn test_register_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.register(test_metadata("p1", PluginType::ImageFilter, vec![]));
        assert_eq!(mgr.plugin_count(), 1);
        assert!(mgr.is_enabled("p1"));
    }

    #[test]
    fn test_disable_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.register(test_metadata("p1", PluginType::ImageFilter, vec![]));
        mgr.set_enabled("p1", false);
        assert!(!mgr.is_enabled("p1"));
        assert_eq!(mgr.enabled_count(), 0);
    }

    #[test]
    fn test_grant_check_permissions() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        let meta = test_metadata(
            "dl",
            PluginType::ContentSource,
            vec![Permission::NetworkAccess],
        );
        mgr.register(meta.clone());

        // Permission not granted yet
        assert!(!mgr.has_permission("dl", Permission::NetworkAccess));
        assert!(mgr.check_permissions(&meta).is_err());

        // Grant it
        mgr.grant_permission("dl", Permission::NetworkAccess);
        assert!(mgr.has_permission("dl", Permission::NetworkAccess));
        assert!(mgr.check_permissions(&meta).is_ok());
    }

    #[test]
    fn test_revoke_permission() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.grant_permission("p1", Permission::NetworkAccess);
        assert!(mgr.has_permission("p1", Permission::NetworkAccess));
        mgr.revoke_permission("p1", Permission::NetworkAccess);
        assert!(!mgr.has_permission("p1", Permission::NetworkAccess));
    }

    #[test]
    fn test_grant_all_requested() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        let meta = test_metadata(
            "dl",
            PluginType::ContentSource,
            vec![Permission::NetworkAccess, Permission::FileSystemWrite],
        );
        mgr.grant_all_requested(&meta);
        assert!(mgr.has_permission("dl", Permission::NetworkAccess));
        assert!(mgr.has_permission("dl", Permission::FileSystemWrite));
        assert!(!mgr.has_permission("dl", Permission::PaymentProcessing));
    }

    #[test]
    fn test_plugins_of_type() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.register(test_metadata("f1", PluginType::ImageFilter, vec![]));
        mgr.register(test_metadata("f2", PluginType::ImageFilter, vec![]));
        mgr.register(test_metadata("d1", PluginType::ContentSource, vec![]));
        assert_eq!(mgr.plugins_of_type(PluginType::ImageFilter).len(), 2);
        assert_eq!(mgr.plugins_of_type(PluginType::ContentSource).len(), 1);
        assert_eq!(mgr.plugins_of_type(PluginType::Payment).len(), 0);
    }

    #[test]
    fn test_disabled_plugin_excluded_from_type_query() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.register(test_metadata("f1", PluginType::ImageFilter, vec![]));
        mgr.register(test_metadata("f2", PluginType::ImageFilter, vec![]));
        mgr.set_enabled("f1", false);
        assert_eq!(mgr.plugins_of_type(PluginType::ImageFilter).len(), 1);
    }

    #[test]
    fn test_save_load_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.register(test_metadata("p1", PluginType::ImageFilter, vec![]));
        mgr.set_enabled("p1", false);
        mgr.grant_permission("p1", Permission::NetworkAccess);
        mgr.save_config().unwrap();

        // Reload
        let mgr2 = PluginManager::new(dir.path().to_path_buf());
        // Config persists enabled state
        assert_eq!(mgr2.config.enabled.get("p1").copied(), Some(false));
        // Config persists permissions
        assert!(mgr2.has_permission("p1", Permission::NetworkAccess));
    }

    #[test]
    fn test_list_plugins() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut mgr = PluginManager::new(dir.path().to_path_buf());
        mgr.register(test_metadata("a", PluginType::ImageFilter, vec![]));
        mgr.register(test_metadata("b", PluginType::Payment, vec![]));
        let list = mgr.list_plugins();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|(_, enabled)| *enabled));
    }

    #[test]
    fn test_permission_denied_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let mgr = PluginManager::new(dir.path().to_path_buf());
        let meta = test_metadata(
            "bad",
            PluginType::ContentSource,
            vec![Permission::NetworkAccess, Permission::PaymentProcessing],
        );
        let err = mgr.check_permissions(&meta).unwrap_err();
        match err {
            PluginError::PermissionDenied { plugin, permission } => {
                assert_eq!(plugin, "bad");
                assert_eq!(permission, Permission::NetworkAccess);
            }
            _ => panic!("Expected PermissionDenied"),
        }
    }

    #[test]
    fn test_empty_manager() {
        let dir = tempfile::TempDir::new().unwrap();
        let mgr = PluginManager::new(dir.path().to_path_buf());
        assert_eq!(mgr.plugin_count(), 0);
        assert_eq!(mgr.enabled_count(), 0);
        assert!(mgr.list_plugins().is_empty());
    }
}
