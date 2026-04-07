use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use axiom_core::Ruleset;

use crate::auth::{Identity, require_write};
use crate::state::AppState;

/// GET /v1/rulesets
pub async fn list_rulesets(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.store().list_rulesets().await {
        Ok(rs)  => (StatusCode::OK, Json(json!(rs))),
        Err(e)  => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// POST /v1/rulesets
pub async fn create_ruleset(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }
    let rs: Ruleset = match serde_json::from_value(body) {
        Ok(r)  => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))),
    };
    match state.store().upsert_ruleset(rs).await {
        Ok(created) => {
            state.registry_write().await.upsert_ruleset(created.clone());
            (StatusCode::CREATED, Json(json!(created)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// GET /v1/rulesets/:name
pub async fn get_ruleset(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Value>) {
    match state.store().get_ruleset(&name).await {
        Ok(Some(rs)) => (StatusCode::OK, Json(json!(rs))),
        Ok(None)     => (StatusCode::NOT_FOUND, Json(json!({ "error": "ruleset not found" }))),
        Err(e)       => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// PUT /v1/rulesets/:name
pub async fn update_ruleset(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(name): Path<String>,
    Json(mut body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_write(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }
    body["name"] = json!(name);
    let rs: Ruleset = match serde_json::from_value(body) {
        Ok(r)  => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))),
    };
    match state.store().upsert_ruleset(rs).await {
        Ok(updated) => {
            state.registry_write().await.upsert_ruleset(updated.clone());
            (StatusCode::OK, Json(json!(updated)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}
