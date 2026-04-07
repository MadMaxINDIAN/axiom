use std::sync::Arc;
use tokio::sync::RwLock;

use axiom_core::Registry;
use crate::auth::{verify_key, Identity, Role};
use crate::config::{ConfigKey, ServerConfig};
use crate::rate_limit::RateLimiter;
use crate::storage::RuleStore;
use crate::webhook::WebhookDispatcher;

/// Shared application state passed to every Axum handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    registry:    RwLock<Registry>,
    store:       Arc<dyn RuleStore>,
    config_keys: Vec<ConfigKey>,
    config:      ServerConfig,
    rate_limiter: RateLimiter,
    webhooks:    WebhookDispatcher,
}

impl AppState {
    pub fn new(
        registry: Registry,
        store:    Arc<dyn RuleStore>,
        config:   ServerConfig,
    ) -> Self {
        let config_keys  = config.keys.clone();
        let rate_limit   = config.rate_limit_per_sec;
        let dead_letter  = config.dead_letter_path.clone()
            .unwrap_or_else(|| "/data/dead-letter".to_string());
        let webhooks     = WebhookDispatcher::new(config.webhooks.clone(), &dead_letter);

        AppState {
            inner: Arc::new(Inner {
                registry:     RwLock::new(registry),
                store,
                config_keys,
                config,
                rate_limiter: RateLimiter::new(rate_limit),
                webhooks,
            }),
        }
    }

    // ── Registry ──────────────────────────────────────────────────────────

    pub async fn registry_read(&self) -> tokio::sync::RwLockReadGuard<'_, Registry> {
        self.inner.registry.read().await
    }

    pub async fn registry_write(&self) -> tokio::sync::RwLockWriteGuard<'_, Registry> {
        self.inner.registry.write().await
    }

    // ── Storage ───────────────────────────────────────────────────────────

    pub fn store(&self) -> &Arc<dyn RuleStore> { &self.inner.store }

    // ── Config ────────────────────────────────────────────────────────────

    pub fn config(&self) -> &ServerConfig { &self.inner.config }

    // ── Auth ──────────────────────────────────────────────────────────────

    pub fn authenticate(&self, plaintext_key: &str) -> Option<Identity> {
        for ck in &self.inner.config_keys {
            if verify_key(plaintext_key, &ck.hash) {
                let role = Role::from_str(&ck.role)?;
                return Some(Identity { key_id: ck.id.clone(), role });
            }
        }
        None
    }

    // ── Rate limiting ─────────────────────────────────────────────────────

    /// Returns `Ok(())` or `Err(retry_after_secs)`.
    pub fn check_rate_limit(&self, key_id: &str, n: u32) -> Result<(), u64> {
        self.inner.rate_limiter.check(key_id, n)
    }

    // ── Webhooks ──────────────────────────────────────────────────────────

    pub fn webhooks(&self) -> &WebhookDispatcher { &self.inner.webhooks }
}
