use async_trait::async_trait;
use sqlx::{PgPool, Row};
use axiom_core::{Rule, Ruleset};

use super::{RuleStore, StoreFilter};

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(url).await?;
        let store = PostgresStore { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rules (
                id          TEXT        NOT NULL,
                version     INTEGER     NOT NULL,
                ars_version INTEGER     NOT NULL DEFAULT 1,
                enabled     BOOLEAN     NOT NULL DEFAULT true,
                priority    INTEGER     NOT NULL DEFAULT 0,
                tags        JSONB       NOT NULL DEFAULT '[]',
                definition  TEXT        NOT NULL,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_by  TEXT,
                PRIMARY KEY (id, version)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS rules_tags_gin ON rules USING GIN (tags)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rulesets (
                name        TEXT PRIMARY KEY,
                rule_ids    JSONB       NOT NULL DEFAULT '[]',
                description TEXT,
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id          TEXT PRIMARY KEY,
                role        TEXT        NOT NULL,
                hash        TEXT        NOT NULL UNIQUE,
                description TEXT,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                created_by  TEXT,
                revoked_at  TIMESTAMPTZ
            );
            CREATE UNIQUE INDEX IF NOT EXISTS api_keys_hash_idx ON api_keys (hash);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl RuleStore for PostgresStore {
    async fn get_rule(&self, id: &str) -> anyhow::Result<Option<Rule>> {
        let row = sqlx::query(
            "SELECT definition FROM rules WHERE id = $1 ORDER BY version DESC LIMIT 1"
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
        let tags_json  = serde_json::to_string(&rule.tags)?;

        sqlx::query(
            r#"
            INSERT INTO rules (id, version, ars_version, enabled, priority, tags, definition)
            VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7)
            ON CONFLICT (id, version) DO UPDATE SET
                ars_version = EXCLUDED.ars_version,
                enabled     = EXCLUDED.enabled,
                priority    = EXCLUDED.priority,
                tags        = EXCLUDED.tags,
                definition  = EXCLUDED.definition,
                updated_at  = now()
            "#,
        )
        .bind(&rule.id)
        .bind(rule.version as i32)
        .bind(rule.ars_version as i32)
        .bind(rule.enabled)
        .bind(rule.priority)
        .bind(&tags_json)
        .bind(&definition)
        .execute(&self.pool)
        .await?;

        Ok(rule)
    }

    async fn disable_rule(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE rules SET enabled = false, updated_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_ruleset(&self, name: &str) -> anyhow::Result<Option<Ruleset>> {
        let row = sqlx::query("SELECT name, rule_ids, description FROM rulesets WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => {
                let ids_raw: serde_json::Value = r.get("rule_ids");
                let rule_ids: Vec<String> = serde_json::from_value(ids_raw)?;
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
            INSERT INTO rulesets (name, rule_ids, description)
            VALUES ($1, $2::jsonb, $3)
            ON CONFLICT (name) DO UPDATE SET
                rule_ids    = EXCLUDED.rule_ids,
                description = EXCLUDED.description,
                updated_at  = now()
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
            let ids_raw: serde_json::Value = row.get("rule_ids");
            let rule_ids: Vec<String> = serde_json::from_value(ids_raw)?;
            rulesets.push(Ruleset {
                name:        row.get("name"),
                rule_ids,
                description: row.get("description"),
            });
        }
        Ok(rulesets)
    }

    async fn list_versions(&self, id: &str) -> anyhow::Result<Vec<u32>> {
        let rows = sqlx::query("SELECT version FROM rules WHERE id = $1 ORDER BY version ASC")
            .bind(id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(|r| r.get::<i32, _>("version") as u32).collect())
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
            "INSERT INTO api_keys (id, role, hash, description, created_by) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id).bind(role).bind(hash).bind(description).bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_api_key(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE api_keys SET revoked_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn verify_api_key(&self, hash: &str) -> anyhow::Result<Option<(String, String)>> {
        let row = sqlx::query(
            "SELECT id, role FROM api_keys WHERE hash = $1 AND revoked_at IS NULL"
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
