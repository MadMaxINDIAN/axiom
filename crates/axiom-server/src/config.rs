use serde::{Deserialize, Serialize};
use crate::webhook::WebhookConfig;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub storage: StorageConfig,
    #[serde(default)]
    pub keys: Vec<ConfigKey>,
    #[serde(default = "default_poll_secs")]
    pub rule_poll_secs: u64,
    #[serde(default)]
    pub dead_letter_path: Option<String>,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_sec: u32,
    #[serde(default)]
    pub log_level: Option<String>,
    /// Outbound webhook registrations for `trigger` actions.
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,
    /// Optional directory to watch for YAML rule files (§6.5 hot-reload).
    #[serde(default)]
    pub rules_dir: Option<String>,
}

fn default_host()       -> String { "0.0.0.0".into() }
fn default_port()       -> u16    { 8080 }
fn default_poll_secs()  -> u64    { 10 }
fn default_rate_limit() -> u32    { 1_000 }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StorageConfig {
    #[serde(default = "default_backend")]
    pub backend: StorageBackend,
    #[serde(default = "default_sqlite_path")]
    pub path: String,
    #[serde(default)]
    pub postgres_url: Option<String>,
}

fn default_backend()     -> StorageBackend { StorageBackend::Sqlite }
fn default_sqlite_path() -> String { "/data/axiom.db".into() }

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend { Sqlite, Postgres }

/// An API key entry from the config file (§6.2).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigKey {
    pub id:          String,
    pub role:        String,
    pub hash:        String,
    #[serde(default)]
    pub description: Option<String>,
}

impl ServerConfig {
    pub fn load() -> anyhow::Result<Self> {
        let cfg: ServerConfig = config::Config::builder()
            .add_source(config::File::with_name("axiom").required(false))
            .add_source(config::Environment::with_prefix("AXIOM").separator("_"))
            .build()?
            .try_deserialize()?;
        Ok(cfg)
    }

    /// Build a minimal default config for cases where loading fails.
    pub fn default_config() -> Self {
        serde_json::from_str(r#"{
            "storage": { "backend": "sqlite", "path": "/data/axiom.db" }
        }"#).expect("default config")
    }
}
