mod auth;
mod config;
mod metrics;
mod rate_limit;
mod routes;
mod state;
mod storage;
mod watch;
mod webhook;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    middleware,
    routing::{delete, get, post, put, patch},
    Router,
};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use axiom_core::Registry;
use config::{ServerConfig, StorageBackend};
use state::AppState;
use storage::RuleStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Structured JSON logging (§11.3) ───────────────────────────────────
    let log_level = std::env::var("AXIOM_LOG_LEVEL").unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("axiom={log_level},tower_http=warn")))
        )
        .json()
        .init();

    // ── Config ────────────────────────────────────────────────────────────
    let cfg = ServerConfig::load().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Config load failed, using defaults");
        ServerConfig::default_config()
    });

    info!(host = %cfg.host, port = %cfg.port, backend = ?cfg.storage.backend, "Starting axiom-server");

    // ── Storage ───────────────────────────────────────────────────────────
    let store: Arc<dyn RuleStore> = match cfg.storage.backend {
        StorageBackend::Sqlite => {
            Arc::new(storage::sqlite::SqliteStore::new(&cfg.storage.path).await?)
        }
        StorageBackend::Postgres => {
            let url = cfg.storage.postgres_url.as_deref()
                .expect("AXIOM_STORAGE_POSTGRES_URL required for postgres backend");
            Arc::new(storage::postgres::PostgresStore::new(url).await?)
        }
    };

    // ── Bootstrap registry ────────────────────────────────────────────────
    let mut registry = Registry::new();
    let stored_rules    = store.list_rules(Default::default()).await.unwrap_or_default();
    let stored_rulesets = store.list_rulesets().await.unwrap_or_default();
    let rule_count      = stored_rules.len();
    let _ = registry.load_rules(stored_rules);
    for rs in stored_rulesets { registry.upsert_ruleset(rs); }
    metrics::set_rules_loaded(rule_count);

    // ── App state ─────────────────────────────────────────────────────────
    let state = AppState::new(registry, Arc::clone(&store), cfg.clone());

    // ── Router ────────────────────────────────────────────────────────────
    let app = Router::new()
        // Health / observability
        .route("/health",  get(routes::health::health))
        .route("/ready",   get(routes::health::ready))
        .route("/metrics", get(routes::health::metrics))
        // Rules
        .route("/v1/rules",
            get(routes::rules::list_rules).post(routes::rules::create_rule))
        .route("/v1/rules/:id",
            get(routes::rules::get_rule)
                .put(routes::rules::update_rule)
                .patch(routes::rules::patch_rule)
                .delete(routes::rules::delete_rule))
        .route("/v1/rules/:id/versions", get(routes::rules::list_versions))
        // Rulesets
        .route("/v1/rulesets",
            get(routes::rulesets::list_rulesets).post(routes::rulesets::create_ruleset))
        .route("/v1/rulesets/:name",
            get(routes::rulesets::get_ruleset).put(routes::rulesets::update_ruleset))
        // Evaluate
        .route("/v1/evaluate",       post(routes::evaluate::evaluate))
        .route("/v1/evaluate/batch", post(routes::evaluate::evaluate_batch))
        // API key management (Phase 2)
        .route("/v1/keys",
            get(routes::keys::list_keys).post(routes::keys::create_key))
        .route("/v1/keys/:id", delete(routes::keys::revoke_key))
        // Import / export
        .route("/v1/import", post(routes::import_export::import_bundle))
        .route("/v1/export", get(routes::import_export::export_bundle))
        // Auth middleware
        .layer(middleware::from_fn_with_state(state.clone(), auth::auth_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    // ── Background poll (§6.5) ────────────────────────────────────────────
    {
        let ps = state.clone();
        let interval = cfg.rule_poll_secs;
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(interval));
            loop { tick.tick().await; reload_from_store(&ps).await; }
        });
    }

    // ── Filesystem hot-reload (§6.5) ──────────────────────────────────────
    if let Some(ref rules_dir) = cfg.rules_dir {
        watch::spawn_watch(state.clone(), rules_dir);
    }

    // ── Listen ────────────────────────────────────────────────────────────
    let addr     = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn reload_from_store(state: &AppState) {
    if !state.store().ping().await {
        tracing::warn!("Storage unreachable during poll — serving from cache");
        return;
    }
    let rules    = state.store().list_rules(Default::default()).await.unwrap_or_default();
    let rulesets = state.store().list_rulesets().await.unwrap_or_default();
    let count    = rules.len();
    let mut reg  = state.registry_write().await;
    let _ = reg.load_rules(rules);
    for rs in rulesets { reg.upsert_ruleset(rs); }
    metrics::set_rules_loaded(count);
}
