use async_trait::async_trait;
use sqlx::{SqlitePool, Row};
use axiom_core::{Rule, Ruleset};

use super::{RuleStore, StoreFilter};

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let url = format!("sqlite://{}?mode=rwc", path);
        let pool = SqlitePool::connect(&url).await?;
        let store = SqliteStore { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rules (
                id          TEXT    NOT NULL,
                version     INTEGER NOT NULL,
                ars_version INTEGER NOT NULL DEFAULT 1,
                enabled     BOOLEAN NOT NULL DEFAULT 1,
                priority    INTEGER NOT NULL DEFAULT 0,
                tags        TEXT    NOT NULL DEFAULT '[]',
                definition  TEXT    NOT NULL,
                created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_by  TEXT,
                PRIMARY KEY (id, version)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rulesets (
                name        TEXT PRIMARY KEY,
                rule_ids    TEXT NOT NULL DEFAULT '[]',
                description TEXT,
                updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id          TEXT PRIMARY KEY,
                role        TEXT NOT NULL,
                hash        TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at  TEXT NOT NULL DEFAULT (datetime('now')),
                created_by  TEXT,
                revoked_at  TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl RuleStore for SqliteStore {
    async fn get_rule(&self, id: &str) -> anyhow::Result<Option<Rule>> {
        let row = sqlx::query(
            "SELECT definition FROM rules WHERE id = ? ORDER BY version DESC LIMIT 1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let def: String = r.get("definition");
                Ok(Some(serde_json::from_str(&def)?))
            }
            None => Ok(None),
        }
    }

    async fn list_rules(&self, filter: StoreFilter) -> anyhow::Result<Vec<Rule>> {
        // Fetch the latest version of each rule
        let rows = sqlx::query(
            r#"
            SELECT r.definition
            FROM rules r
            INNER JOIN (
                SELECT id, MAX(version) AS max_ver FROM rules GROUP BY id
            ) latest ON r.id = latest.id AND r.version = latest.max_ver
            ORDER BY r.priority DESC, r.id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            let def: String = row.get("definition");
            let rule: Rule = serde_json::from_str(&def)?;
            if let Some(enabled) = filter.enabled {
                if rule.enabled != enabled { continue; }
            }
            if let Some(ref tag) = filter.tag {
                if !rule.tags.contains(tag) { continue; }
            }
            rules.push(rule);
        }
        Ok(rules)
    }

    async fn upsert_rule(&self, rule: Rule) -> anyhow::Result<Rule> {
        let definition = serde_json::to_string(&rule)?;
        let tags       = serde_json::to_string(&rule.tags)?;

        sqlx::query(
            r#"
            INSERT INTO rules (id, version, ars_version, enabled, priority, tags, definition, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(id, version) DO UPDATE SET
                ars_version = excluded.ars_version,
                enabled     = excluded.enabled,
                priority    = excluded.priority,
                tags        = excluded.tags,
                definition  = excluded.definition,
                updated_at  = excluded.updated_at
            "#,
        )
        .bind(&rule.id)
        .bind(rule.version as i64)
        .bind(rule.ars_version as i64)
        .bind(rule.enabled)
        .bind(rule.priority)
        .bind(&tags)
        .bind(&definition)
        .execute(&self.pool)
        .await?;

        Ok(rule)
    }

    async fn disable_rule(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE rules SET enabled = 0, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_ruleset(&self, name: &str) -> anyhow::Result<Option<Ruleset>> {
        let row = sqlx::query("SELECT name, rule_ids, description FROM rulesets WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => {
                let rule_ids: Vec<String> = serde_json::from_str(r.get("rule_ids"))?;
                Ok(Some(Ruleset {
                    name:        r.get("name"),
                    rule_ids,
                    description: r.get("description"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn upsert_ruleset(&self, rs: Ruleset) -> anyhow::Result<Ruleset> {
        let ids = serde_json::to_string(&rs.rule_ids)?;
        sqlx::query(
            r#"
            INSERT INTO rulesets (name, rule_ids, description, updated_at)
            VALUES (?, ?, ?, datetime('now'))
            ON CONFLICT(name) DO UPDATE SET
                rule_ids    = excluded.rule_ids,
                description = excluded.description,
                updated_at  = excluded.updated_at
            "#,
        )
        .bind(&rs.name)
        .bind(&ids)
        .bind(&rs.description)
        .execute(&self.pool)
        .await?;
        Ok(rs)
    }

    async fn list_rulesets(&self) -> anyhow::Result<Vec<Ruleset>> {
        let rows = sqlx::query("SELECT name, rule_ids, description FROM rulesets ORDER BY name")
            .fetch_all(&self.pool)
            .await?;

        let mut rulesets = Vec::new();
        for row in rows {
            let rule_ids: Vec<String> = serde_json::from_str(row.get("rule_ids"))?;
            rulesets.push(Ruleset {
                name:        row.get("name"),
                rule_ids,
                description: row.get("description"),
            });
        }
        Ok(rulesets)
    }

    async fn list_versions(&self, id: &str) -> anyhow::Result<Vec<u32>> {
        let rows = sqlx::query("SELECT version FROM rules WHERE id = ? ORDER BY version ASC")
            .bind(id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(|r| r.get::<i64, _>("version") as u32).collect())
    }

    async fn list_api_keys(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT id, role, description, created_at, created_by, revoked_at FROM api_keys ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| serde_json::json!({
            "id":          r.get::<String, _>("id"),
            "role":        r.get::<String, _>("role"),
            "description": r.get::<Option<String>, _>("description"),
            "created_at":  r.get::<String, _>("created_at"),
            "created_by":  r.get::<Option<String>, _>("created_by"),
            "revoked_at":  r.get::<Option<String>, _>("revoked_at"),
            "source":      "db",
        })).collect())
    }

    async fn create_api_key(
        &self, id: &str, role: &str, hash: &str,
        description: Option<&str>, created_by: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO api_keys (id, role, hash, description, created_by) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(id).bind(role).bind(hash).bind(description).bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_api_key(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE api_keys SET revoked_at = datetime('now') WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn verify_api_key(&self, hash: &str) -> anyhow::Result<Option<(String, String)>> {
        let row = sqlx::query(
            "SELECT id, role FROM api_keys WHERE hash = ? AND revoked_at IS NULL"
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| (r.get::<String, _>("id"), r.get::<String, _>("role"))))
    }

    async fn ping(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }
}
