use std::path::Path;
use axiom_core::{parser, Registry, EvaluationRequest, Strategy};

/// Evaluate a local rule file against a JSON context string.
pub fn run_local(
    rule_path:       &Path,
    context_json:    &str,
    strategy:        Strategy,
    fail_on_no_match: bool,
) -> anyhow::Result<bool> {
    let bytes = std::fs::read(rule_path)?;
    let rule = if rule_path.extension().and_then(|e| e.to_str()) == Some("json") {
        parser::parse_rule_json(&bytes)?
    } else {
        parser::parse_rule_yaml(&bytes)?
    };

    let context: serde_json::Value = serde_json::from_str(context_json)?;
    let rule_id = rule.id.clone();

    let mut registry = Registry::new();
    registry.upsert_rule(rule).map_err(|e| anyhow::anyhow!("{e}"))?;

    let req = EvaluationRequest {
        rule_id:    Some(rule_id),
        strategy,
        context,
        ..Default::default()
    };

    let resp = registry.evaluate(&req)?;
    let out  = serde_json::to_string_pretty(&resp)?;
    println!("{out}");

    if fail_on_no_match && !resp.matched {
        anyhow::bail!("no rule matched (--fail-on-no-match)");
    }

    Ok(resp.matched)
}

/// Evaluate via a remote Axiom server.
pub async fn run_remote(
    server_url:   &str,
    api_key:      &str,
    rule_id:      &str,
    context_json: &str,
    strategy:     Strategy,
) -> anyhow::Result<bool> {
    let context: serde_json::Value = serde_json::from_str(context_json)?;

    let req = EvaluationRequest {
        rule_id:  Some(rule_id.to_string()),
        strategy,
        context,
        ..Default::default()
    };

    let client = reqwest::Client::new();
    let url    = format!("{server_url}/v1/evaluate");
    let resp   = client.post(&url)
        .header("X-Axiom-Key", api_key)
        .json(&req)
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&body)?);

    if !status.is_success() {
        anyhow::bail!("server returned {status}");
    }

    Ok(body.get("matched").and_then(|v| v.as_bool()).unwrap_or(false))
}
