/// Outbound webhook dispatcher for `trigger` actions (§4.6, [AR-9, R3-2]).
///
/// Retry policy: 3 attempts, exponential backoff — 1 s, 4 s, 16 s.
/// On all retries exhausted: write to dead-letter directory as a JSON file.
/// HMAC-SHA256 signature in the `X-Axiom-Signature` request header.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::Sha256;
use hmac::{Hmac, Mac};
use tracing::{info, warn, error};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// One entry in the `webhooks:` section of `axiom.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookConfig {
    pub event:  String,
    pub url:    String,
    #[serde(default)]
    pub secret: Option<String>,
    /// Override max retries (default: 3).
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}
fn default_retries() -> u32 { 3 }

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WebhookDispatcher {
    /// event name → config
    hooks:           Arc<HashMap<String, WebhookConfig>>,
    dead_letter_dir: PathBuf,
    client:          reqwest::Client,
}

impl WebhookDispatcher {
    pub fn new(hooks: Vec<WebhookConfig>, dead_letter_dir: impl Into<PathBuf>) -> Self {
        let map: HashMap<String, WebhookConfig> = hooks.into_iter()
            .map(|h| (h.event.clone(), h))
            .collect();
        WebhookDispatcher {
            hooks:           Arc::new(map),
            dead_letter_dir: dead_letter_dir.into(),
            client:          reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    /// Dispatch a list of triggered event names from a single evaluation.
    /// Each dispatch is launched as a detached tokio task.
    pub fn dispatch_all(&self, events: Vec<String>, payload: serde_json::Value) {
        for event in events {
            if let Some(hook) = self.hooks.get(&event).cloned() {
                let dispatcher = self.clone();
                let payload    = payload.clone();
                tokio::spawn(async move {
                    dispatcher.dispatch_with_retry(hook, payload).await;
                });
            } else {
                info!(event = %event, "trigger fired but no webhook registered — no-op");
            }
        }
    }

    async fn dispatch_with_retry(&self, hook: WebhookConfig, payload: serde_json::Value) {
        let body      = serde_json::to_string(&payload).unwrap_or_default();
        let delays_s  = [1u64, 4, 16];
        let max       = hook.max_retries.min(3) as usize;

        for attempt in 0..=max {
            match self.send_once(&hook, &body).await {
                Ok(status) if status.is_success() => {
                    info!(event = %hook.event, url = %hook.url, attempt, "webhook delivered");
                    return;
                }
                Ok(status) => {
                    warn!(event = %hook.event, url = %hook.url, attempt,
                          status = %status, "webhook non-2xx response");
                }
                Err(e) => {
                    warn!(event = %hook.event, url = %hook.url, attempt, error = %e, "webhook error");
                }
            }

            if attempt < max {
                tokio::time::sleep(Duration::from_secs(delays_s[attempt])).await;
            }
        }

        // All retries exhausted — write dead-letter
        self.write_dead_letter(&hook.event, &body).await;
    }

    async fn send_once(
        &self,
        hook:  &WebhookConfig,
        body:  &str,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let mut req = self.client
            .post(&hook.url)
            .header("Content-Type", "application/json")
            .body(body.to_string());

        if let Some(ref secret) = hook.secret {
            let sig = hmac_sign(secret, body);
            req = req.header("X-Axiom-Signature", format!("sha256={sig}"));
        }

        let resp = req.send().await?;
        Ok(resp.status())
    }

    async fn write_dead_letter(&self, event: &str, body: &str) {
        let dir = &self.dead_letter_dir;
        if let Err(e) = tokio::fs::create_dir_all(dir).await {
            error!(dir = %dir.display(), error = %e, "cannot create dead-letter dir");
            return;
        }
        let filename = format!("{event}-{}.json", Uuid::new_v4());
        let path     = dir.join(&filename);
        let entry    = serde_json::json!({ "event": event, "body": body, "failed_at": chrono::Utc::now().to_rfc3339() });
        let content  = serde_json::to_string_pretty(&entry).unwrap_or_default();
        match tokio::fs::write(&path, content).await {
            Ok(())  => error!(event, file = %path.display(), "trigger dead-lettered after all retries"),
            Err(e)  => error!(event, error = %e, "failed to write dead-letter file"),
        }
    }
}

fn hmac_sign(secret: &str, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key size");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
