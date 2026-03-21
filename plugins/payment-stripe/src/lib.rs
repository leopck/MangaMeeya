use mm_plugin::{
    Permission, PluginMetadata, PluginResult, PluginType,
    traits::{PaymentPlugin, Plugin},
};
use serde::{Deserialize, Serialize};

/// Stripe-based payment plugin for tips and subscriptions.
/// All payment processing happens server-side — this plugin only
/// provides links and checks status. No payment data stored locally.
pub struct StripePaymentPlugin {
    meta: PluginMetadata,
    config: PaymentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    /// Base URL of the payment server.
    pub server_url: String,
    /// Whether the user has an active subscription.
    pub subscription_active: bool,
}

impl Default for PaymentConfig {
    fn default() -> Self {
        Self {
            server_url: "https://api.mangameeya.com/payments".into(),
            subscription_active: false,
        }
    }
}

impl StripePaymentPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "mm.payment.stripe".into(),
                name: "Stripe Payments".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                author: "MangaMeeya".into(),
                description: "Support MangaMeeya with tips or subscribe for premium features"
                    .into(),
                plugin_type: PluginType::Payment,
                permissions: vec![Permission::PaymentProcessing, Permission::NetworkAccess],
            },
            config: PaymentConfig::default(),
        }
    }

    pub fn with_config(mut self, config: PaymentConfig) -> Self {
        self.config = config;
        self
    }

    pub fn config(&self) -> &PaymentConfig {
        &self.config
    }
}

impl Default for StripePaymentPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for StripePaymentPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }
}

impl PaymentPlugin for StripePaymentPlugin {
    fn check_subscription(&self) -> PluginResult<bool> {
        // In production: call server API to verify subscription status
        // For now: return local cached status
        Ok(self.config.subscription_active)
    }

    fn get_payment_url(&self, amount_cents: u64, description: &str) -> PluginResult<String> {
        // In production: call server to create a Stripe Checkout session
        // and return the session URL. For now: construct a placeholder URL.
        let encoded_desc = description.replace(' ', "+");
        Ok(format!(
            "{}/checkout?amount={}&desc={}",
            self.config.server_url, amount_cents, encoded_desc
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let p = StripePaymentPlugin::new();
        assert_eq!(p.metadata().id, "mm.payment.stripe");
        assert_eq!(p.metadata().plugin_type, PluginType::Payment);
    }

    #[test]
    fn test_permissions() {
        let p = StripePaymentPlugin::new();
        assert!(
            p.metadata()
                .permissions
                .contains(&Permission::PaymentProcessing)
        );
        assert!(
            p.metadata()
                .permissions
                .contains(&Permission::NetworkAccess)
        );
    }

    #[test]
    fn test_check_subscription_default() {
        let p = StripePaymentPlugin::new();
        assert!(!p.check_subscription().unwrap());
    }

    #[test]
    fn test_check_subscription_active() {
        let config = PaymentConfig {
            server_url: "https://test.com".into(),
            subscription_active: true,
        };
        let p = StripePaymentPlugin::new().with_config(config);
        assert!(p.check_subscription().unwrap());
    }

    #[test]
    fn test_get_payment_url() {
        let p = StripePaymentPlugin::new();
        let url = p.get_payment_url(500, "Monthly Pro").unwrap();
        assert!(url.contains("amount=500"));
        assert!(url.contains("Monthly+Pro"));
    }

    #[test]
    fn test_get_payment_url_custom_server() {
        let config = PaymentConfig {
            server_url: "https://custom.server.com/pay".into(),
            subscription_active: false,
        };
        let p = StripePaymentPlugin::new().with_config(config);
        let url = p.get_payment_url(1000, "Tip").unwrap();
        assert!(url.starts_with("https://custom.server.com/pay"));
    }

    #[test]
    fn test_default_config() {
        let p = StripePaymentPlugin::new();
        assert!(p.config().server_url.contains("mangameeya.com"));
        assert!(!p.config().subscription_active);
    }
}
