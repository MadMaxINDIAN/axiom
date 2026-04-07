use axum::{
    extract::{Extension, State},
    http::{StatusCode, HeaderMap},
    Json,
};
use serde_json::{json, Value};
use axiom_core::parser;

use crate::auth::{Identity, require_write};
use crate::state::AppState;

/// POST /v1/import — import YAML/JSON bundle
pub async fn import_bundle(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let parse_result = if content_type.contains("yaml") {
        parser::parse_bundle_yaml(&body)
    } else {
        // Try YAML first (supersets JSON), then JSON
        parser::parse_bundle_yaml(&body)
    };

    let (rules, rulesets) = match parse_result {
        Ok(r)  => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))),
    };

    let mut imported_rules   = 0usize;
    let mut imported_rulesets = 0usize;

    for rule in rules {
        if let Err(e) = state.store().upsert_rule(rule).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
        }
        imported_rules += 1;
    }
    for rs in rulesets {
        if let Err(e) = state.store().upsert_ruleset(rs).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
        }
        imported_rulesets += 1;
    }

    // Reload in-memory registry from store
    reload_registry(&state).await;

    (StatusCode::OK, Json(json!({
        "imported_rules":    imported_rules,
        "imported_rulesets": imported_rulesets,
    })))
}

/// GET /v1/export — export all rules + rulesets as a YAML bundle
pub async fn export_bundle(State(state): State<AppState>) -> (StatusCode, String) {
    let rules    = state.store().list_rules(Default::default()).await;
    let rulesets = state.store().list_rulesets().await;

    match (rules, rulesets) {
        (Ok(r), Ok(rs)) => {
            let bundle = serde_json::json!({ "rules": r, "rulesets": rs });
            match serde_yaml::to_string(&bundle) {
                Ok(yaml) => (StatusCode::OK, yaml),
                Err(e)   => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            }
        }
        (Err(e), _) | (_, Err(e)) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

async fn reload_registry(state: &AppState) {
    let store_rules    = state.store().list_rules(Default::default()).await.unwrap_or_default();
    let store_rulesets = state.store().list_rulesets().await.unwrap_or_default();
    let mut reg = state.registry_write().await;
    let _ = reg.load_rules(store_rules);
    for rs in store_rulesets {
        reg.upsert_ruleset(rs);
    }
}
