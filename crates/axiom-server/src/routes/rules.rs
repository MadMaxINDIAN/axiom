use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use axiom_core::Rule;

use crate::auth::{Identity, require_write};
use crate::state::AppState;
use crate::storage::StoreFilter;

// ── Query params ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct RuleListQuery {
    pub tag:     Option<String>,
    pub enabled: Option<bool>,
    pub ruleset: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /v1/rules
pub async fn list_rules(
    State(state): State<AppState>,
    Query(q): Query<RuleListQuery>,
) -> (StatusCode, Json<Value>) {
    let filter = StoreFilter { tag: q.tag.clone(), enabled: q.enabled };
    match state.store().list_rules(filter).await {
        Ok(rules) => (StatusCode::OK, Json(json!(rules))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// POST /v1/rules
pub async fn create_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    // Parse as ARS (supports JSON body)
    let rule: Rule = match serde_json::from_value(body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))),
    };

    // Validate ars_version
    if rule.ars_version != axiom_core::schema::ARS_VERSION {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("unsupported ars_version: {}", rule.ars_version) })),
        );
    }

    match state.store().upsert_rule(rule).await {
        Ok(created) => {
            // Sync into in-memory registry
            let mut reg = state.registry_write().await;
            if let Err(e) = reg.upsert_rule(created.clone()) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
            }
            (StatusCode::CREATED, Json(json!(created)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// GET /v1/rules/:id
pub async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    match state.store().get_rule(&id).await {
        Ok(Some(rule)) => (StatusCode::OK, Json(json!(rule))),
        Ok(None)       => (StatusCode::NOT_FOUND, Json(json!({ "error": "rule not found" }))),
        Err(e)         => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// GET /v1/rules/:id/versions
pub async fn list_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    match state.store().list_versions(&id).await {
        Ok(versions) => (StatusCode::OK, Json(json!({ "id": id, "versions": versions }))),
        Err(e)       => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// PUT /v1/rules/:id — full replace (increments version)
pub async fn update_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(id): Path<String>,
    Json(mut body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    // Force the id from the URL, increment version
    let current_version = state.store().list_versions(&id).await
        .ok().and_then(|v| v.last().copied()).unwrap_or(0);

    body["id"]      = json!(id);
    body["version"] = json!(current_version + 1);

    let rule: Rule = match serde_json::from_value(body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))),
    };

    match state.store().upsert_rule(rule).await {
        Ok(updated) => {
            let mut reg = state.registry_write().await;
            if let Err(e) = reg.upsert_rule(updated.clone()) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
            }
            (StatusCode::OK, Json(json!(updated)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// PATCH /v1/rules/:id — partial update (e.g. { "enabled": false })
pub async fn patch_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    let existing = match state.store().get_rule(&id).await {
        Ok(Some(r)) => r,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({ "error": "rule not found" }))),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    };

    // Merge patch into existing rule JSON
    let mut rule_val = serde_json::to_value(&existing).unwrap();
    if let (Value::Object(ref mut base), Value::Object(delta)) = (&mut rule_val, patch) {
        for (k, v) in delta { base.insert(k, v); }
    }

    let current_version = state.store().list_versions(&id).await
        .ok().and_then(|v| v.last().copied()).unwrap_or(existing.version);
    rule_val["version"] = json!(current_version + 1);

    let rule: Rule = match serde_json::from_value(rule_val) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))),
    };

    match state.store().upsert_rule(rule).await {
        Ok(patched) => {
            let mut reg = state.registry_write().await;
            if let Err(e) = reg.upsert_rule(patched.clone()) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
            }
            (StatusCode::OK, Json(json!(patched)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// DELETE /v1/rules/:id — soft delete (disables all versions)
pub async fn delete_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    match state.store().disable_rule(&id).await {
        Ok(()) => {
            state.registry_write().await.disable_rule(&id);
            (StatusCode::OK, Json(json!({ "id": id, "status": "disabled" })))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}
