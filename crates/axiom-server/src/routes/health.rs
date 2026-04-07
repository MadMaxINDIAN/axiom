use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::state::AppState;

/// GET /health — liveness probe (200 if process alive)
pub async fn health() -> StatusCode {
    StatusCode::OK
}

/// GET /ready — readiness probe (200 when storage reachable; 503 otherwise)
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    if state.store().ping().await {
        (StatusCode::OK, Json(json!({ "status": "ready" })))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "status": "storage_unavailable" })))
    }
}

/// GET /metrics — Prometheus text format
pub async fn metrics() -> (StatusCode, String) {
    use prometheus::Encoder;
    let encoder  = prometheus::TextEncoder::new();
    let families = prometheus::gather();
    let mut buf  = Vec::new();
    if encoder.encode(&families, &mut buf).is_ok() {
        (StatusCode::OK, String::from_utf8_lossy(&buf).into_owned())
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, String::new())
    }
}
