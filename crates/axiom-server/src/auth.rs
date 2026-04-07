use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::state::AppState;

/// Hash a plaintext API key to `hex(SHA-256(key))`.
pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify a plaintext key against a stored `"sha256:<hex>"` hash.
pub fn verify_key(plaintext: &str, stored_hash: &str) -> bool {
    if let Some(hex_part) = stored_hash.strip_prefix("sha256:") {
        hash_key(plaintext) == hex_part
    } else {
        false
    }
}

/// RBAC roles (§6.4, §9.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role { Admin, Editor, Viewer }

impl Role {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin"  => Some(Role::Admin),
            "editor" => Some(Role::Editor),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin  => "admin",
            Role::Editor => "editor",
            Role::Viewer => "viewer",
        }
    }

    pub fn can_write(&self) -> bool { matches!(self, Role::Admin | Role::Editor) }
    pub fn can_admin(&self) -> bool { matches!(self, Role::Admin) }
}

/// The resolved identity extracted from the `X-Axiom-Key` header.
#[derive(Debug, Clone)]
pub struct Identity {
    pub key_id: String,
    pub role:   Role,
}

/// Axum middleware: authenticate every request using `X-Axiom-Key`.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Allow health/ready/metrics without auth
    let path = req.uri().path();
    if path == "/health" || path == "/ready" || path == "/metrics" {
        return Ok(next.run(req).await);
    }

    let key_header = req
        .headers()
        .get("X-Axiom-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let identity = state.authenticate(key_header).ok_or(StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(identity);
    Ok(next.run(req).await)
}

/// Require at least write (editor/admin) access.
pub fn require_write(identity: &Identity) -> Result<(), StatusCode> {
    if identity.role.can_write() {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Require admin access.
pub fn require_admin(identity: &Identity) -> Result<(), StatusCode> {
    if identity.role.can_admin() {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
