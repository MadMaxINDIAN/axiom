use axum::{extract::{Extension, State}, http::StatusCode, Json};
use serde_json::{json, Value};
use axiom_core::EvaluationRequest;

use crate::auth::Identity;
use crate::metrics;
use crate::state::AppState;

/// POST /v1/evaluate
pub async fn evaluate(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(req): Json<EvaluationRequest>,
) -> (StatusCode, Json<Value>) {
    // Rate limit: 1 evaluation = 1 token
    if let Err(retry) = state.check_rate_limit(&identity.key_id, 1) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "rate limit exceeded", "retry_after": retry })),
        );
    }

    let registry = state.registry_read().await;

    match registry.evaluate_full(&req) {
        Ok((resp, triggered)) => {
            // Record metrics
            let strategy = format!("{:?}", req.strategy).to_lowercase();
            let ruleset  = req.ruleset.as_deref().unwrap_or("");
            metrics::record_eval(
                &strategy, ruleset,
                resp.duration_us, &resp.matched_rules, resp.trace.timed_out,
            );
            drop(registry);

            // Dispatch webhooks (detached tasks — no I/O block here)
            if !triggered.is_empty() {
                let payload = serde_json::to_value(&resp.output_context).unwrap_or_default();
                state.webhooks().dispatch_all(triggered, payload);
            }

            (StatusCode::OK, Json(json!(resp)))
        }
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

/// POST /v1/evaluate/batch  (max 1,000 contexts, §6.1)
pub async fn evaluate_batch(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(requests): Json<Vec<EvaluationRequest>>,
) -> (StatusCode, Json<Value>) {
    if requests.len() > 1_000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "batch size exceeds maximum of 1,000" })),
        );
    }

    // Rate limit counts as N evaluations
    let n = requests.len() as u32;
    if let Err(retry) = state.check_rate_limit(&identity.key_id, n) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "rate limit exceeded", "retry_after": retry })),
        );
    }

    // Evaluate all requests under one read lock (each eval is µs-scale).
    // The concurrency bound (§6.1) is enforced by the rate limiter above.
    let registry = state.registry_read().await;
    let mut final_results = Vec::with_capacity(requests.len());

    for req in &requests {
        let val = match registry.evaluate_full(req) {
            Ok((resp, triggered)) => {
                let strategy = format!("{:?}", req.strategy).to_lowercase();
                let ruleset  = req.ruleset.as_deref().unwrap_or("");
                metrics::record_eval(&strategy, ruleset, resp.duration_us, &resp.matched_rules, resp.trace.timed_out);
                if !triggered.is_empty() {
                    let payload = serde_json::to_value(&resp.output_context).unwrap_or_default();
                    state.webhooks().dispatch_all(triggered, payload);
                }
                json!(resp)
            }
            Err(e) => json!({ "error": e.to_string() }),
        };
        final_results.push(val);
    }
    (StatusCode::OK, Json(json!({ "results": final_results })))
}
