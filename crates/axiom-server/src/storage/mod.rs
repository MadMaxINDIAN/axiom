use async_trait::async_trait;
use axiom_core::{Rule, Ruleset};
use serde_json::Value;

pub mod sqlite;
pub mod postgres;

// ---------------------------------------------------------------------------
// RuleStore trait (§7.2, extended for Phase 2 key management)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait RuleStore: Send + Sync {
    // ── Rules ─────────────────────────────────────────────────────────────
    async fn get_rule(&self, id: &str)              -> anyhow::Result<Option<Rule>>;
    async fn list_rules(&self, filter: StoreFilter) -> anyhow::Result<Vec<Rule>>;
    async fn upsert_rule(&self, rule: Rule)          -> anyhow::Result<Rule>;
    async fn disable_rule(&self, id: &str)           -> anyhow::Result<()>;
    async fn list_versions(&self, id: &str)          -> anyhow::Result<Vec<u32>>;

    // ── Rulesets ──────────────────────────────────────────────────────────
    async fn get_ruleset(&self, name: &str)          -> anyhow::Result<Option<Ruleset>>;
    async fn upsert_ruleset(&self, rs: Ruleset)      -> anyhow::Result<Ruleset>;
    async fn list_rulesets(&self)                    -> anyhow::Result<Vec<Ruleset>>;

    // ── API keys (Phase 2) ────────────────────────────────────────────────
    async fn list_api_keys(&self)                    -> anyhow::Result<Vec<Value>>;
    async fn create_api_key(
        &self, id: &str, role: &str, hash: &str,
        description: Option<&str>, created_by: &str,
    ) -> anyhow::Result<()>;
    async fn revoke_api_key(&self, id: &str)         -> anyhow::Result<()>;
    /// Verify a plaintext key against stored hash (O(1) via index). Returns role if found.
    async fn verify_api_key(&self, hash: &str)       -> anyhow::Result<Option<(String, String)>>;

    // ── Health ────────────────────────────────────────────────────────────
    async fn ping(&self) -> bool;
}

#[derive(Debug, Default, Clone)]
pub struct StoreFilter {
    pub tag:     Option<String>,
    pub enabled: Option<bool>,
}
