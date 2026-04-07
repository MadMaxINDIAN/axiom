/// /v1/keys — API key management (§6.2, [R2-new]).
///
/// Phase 1: config-file keys listed read-only.
/// Phase 2: POST creates DB-backed keys; DELETE revokes them.
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use rand::RngCore;

use crate::auth::{Identity, require_admin};
use crate::state::AppState;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub role:        String,
    #[serde(default)]
    pub description: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /v1/keys — list all keys (config-file + DB). Admin only.
/// Never returns the raw key value.
pub async fn list_keys(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_admin(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    // Config-file keys
    let mut keys: Vec<Value> = state.config().keys.iter().map(|k| json!({
        "id":          k.id,
        "role":        k.role,
        "description": k.description,
        "source":      "config",
        "revoked_at":  null
    })).collect();

    // DB-backed keys
    match state.store().list_api_keys().await {
        Ok(db_keys) => keys.extend(db_keys),
        Err(e)      => tracing::warn!(error = %e, "failed to list DB API keys"),
    }

    (StatusCode::OK, Json(json!(keys)))
}

/// POST /v1/keys — create a new API key.  Returns the plaintext value **once**.
/// Admin only. (§6.2)
pub async fn create_key(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(body): Json<CreateKeyRequest>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_admin(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    if !["admin","editor","viewer"].contains(&body.role.as_str()) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "invalid role" })));
    }

    // Generate key: 32 random bytes → 64 hex chars
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let plaintext = hex::encode(raw);

    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    let hash = format!("sha256:{}", hex::encode(hasher.finalize()));

    // Derive a slug ID from first 8 chars of hash
    let key_id = format!("key-{}", &plaintext[..8]);

    match state.store().create_api_key(&key_id, &body.role, &hash, body.description.as_deref(), &identity.key_id).await {
        Ok(()) => {
            // Return the plaintext key value ONCE
            (StatusCode::CREATED, Json(json!({
                "id":          key_id,
                "role":        body.role,
                "description": body.description,
                "key":         plaintext,   // shown exactly once
                "hash":        hash,
            })))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

/// DELETE /v1/keys/:id — revoke a key. Admin only.
/// Cannot revoke the last admin key (HTTP 409).
/// Cannot revoke config-file keys via API.
pub async fn revoke_key(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(key_id): Path<String>,
) -> (StatusCode, Json<Value>) {
    if let Err(s) = require_admin(&identity) {
        return (s, Json(json!({ "error": "forbidden" })));
    }

    // Refuse to revoke config-file keys
    if state.config().keys.iter().any(|k| k.id == key_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "config-file keys cannot be revoked via API; remove from axiom.yaml and restart" })),
        );
    }

    // Guard: must not revoke the last admin key
    match guard_last_admin(&state, &key_id).await {
        Err(msg) => return (StatusCode::CONFLICT, Json(json!({ "error": msg }))),
        Ok(())   => {}
    }

    match state.store().revoke_api_key(&key_id).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "id": key_id, "status": "revoked" }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}

async fn guard_last_admin(state: &AppState, revoking_id: &str) -> Result<(), String> {
    let config_admin_count = state.config().keys.iter()
        .filter(|k| k.role == "admin")
        .count();
    let db_keys = state.store().list_api_keys().await.unwrap_or_default();
    let db_admin_active = db_keys.iter()
        .filter(|k| {
            k.get("role").and_then(|v| v.as_str()) == Some("admin")
                && k.get("revoked_at").map(|v| v.is_null()).unwrap_or(true)
                && k.get("id").and_then(|v| v.as_str()) != Some(revoking_id)
        })
        .count();
    if config_admin_count + db_admin_active == 0 {
        return Err("cannot revoke the last admin key".to_string());
    }
    Ok(())
}
