#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;

use axiom_core::{
    parser,
    registry::Registry,
    schema::{EvaluationRequest, Strategy},
};

// ---------------------------------------------------------------------------
// AxiomEngine — main entry point exposed to JS/TS
// ---------------------------------------------------------------------------

/// Thread-safe Axiom evaluation engine.
/// Share a single instance across requests — the registry is protected
/// internally by a `RwLock`.
#[napi]
pub struct AxiomEngine {
    registry: std::sync::Arc<std::sync::RwLock<Registry>>,
}

#[napi]
impl AxiomEngine {
    /// Create a new, empty engine instance.
    #[napi(constructor)]
    pub fn new() -> Self {
        AxiomEngine {
            registry: std::sync::Arc::new(std::sync::RwLock::new(Registry::new())),
        }
    }

    // ── Rule loading ──────────────────────────────────────────────────────

    /// Load a rule from an ARS YAML string.
    #[napi]
    pub fn load_rule_yaml(&self, yaml: String) -> napi::Result<()> {
        let rule = parser::parse_rule_yaml_str(&yaml)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.registry.write().unwrap()
            .upsert_rule(rule)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Load a rule from an ARS JSON string or plain JS object (JSON-serialised).
    #[napi]
    pub fn load_rule_json(&self, json: String) -> napi::Result<()> {
        let rule = parser::parse_rule_json_str(&json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.registry.write().unwrap()
            .upsert_rule(rule)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Load a rule from a file path (auto-detects YAML/JSON by extension).
    #[napi]
    pub fn load_rule_file(&self, path: String) -> napi::Result<()> {
        let bytes = std::fs::read(&path)
            .map_err(|e| napi::Error::from_reason(format!("{path}: {e}")))?;
        let rule = if path.ends_with(".json") {
            parser::parse_rule_json(&bytes)
        } else {
            parser::parse_rule_yaml(&bytes)
        }.map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.registry.write().unwrap()
            .upsert_rule(rule)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Load a bundle YAML file (rules + rulesets).
    #[napi]
    pub fn load_bundle(&self, path: String) -> napi::Result<()> {
        let bytes = std::fs::read(&path)
            .map_err(|e| napi::Error::from_reason(format!("{path}: {e}")))?;
        let (rules, rulesets) = parser::parse_bundle_yaml(&bytes)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let mut reg = self.registry.write().unwrap();
        reg.load_rules(rules)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        for rs in rulesets { reg.upsert_ruleset(rs); }
        Ok(())
    }

    // ── Evaluation ────────────────────────────────────────────────────────

    /// Evaluate a context (JSON string) against all loaded rules.
    /// Returns the evaluation response as a JSON string.
    #[napi]
    pub fn evaluate(&self, request_json: String) -> napi::Result<String> {
        let req: EvaluationRequest = serde_json::from_str(&request_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let reg = self.registry.read().unwrap();
        let resp = reg.evaluate(&req)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        serde_json::to_string(&resp)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Validate an ARS YAML/JSON string without loading it.
    /// Returns an error message string, or null if valid.
    #[napi]
    pub fn validate_rule(&self, source: String, is_json: bool) -> Option<String> {
        let result = if is_json {
            parser::parse_rule_json_str(&source).map(|_| ())
        } else {
            parser::parse_rule_yaml_str(&source).map(|_| ())
        };
        result.err().map(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Standalone validate function (§8.2)
// ---------------------------------------------------------------------------

/// Parse and validate ARS YAML or JSON. Returns null if valid, error string if not.
#[napi]
pub fn validate_rule(yaml_or_json: String) -> Option<String> {
    // Try YAML first (superset of JSON)
    let result = parser::parse_rule_yaml_str(&yaml_or_json).map(|_| ());
    result.err().map(|e| e.to_string())
}
