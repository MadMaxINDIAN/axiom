use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::{Arc, RwLock};

use axiom_core::{
    parser,
    registry::Registry,
    schema::EvaluationRequest,
};

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

fn to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// AxiomEngine — synchronous Python class
// ---------------------------------------------------------------------------

/// Synchronous Axiom evaluation engine.
///
/// Create a single instance and reuse it across calls; the internal registry
/// is protected by a `RwLock` so it is safe to share across threads.
///
/// Example::
///
///     from axiom_rules import AxiomEngine
///     engine = AxiomEngine()
///     engine.load_rule_yaml(open("loan.yaml").read())
///     result = engine.evaluate({"context": {"annual_income": 60000}})
#[pyclass]
pub struct AxiomEngine {
    registry: Arc<RwLock<Registry>>,
}

#[pymethods]
impl AxiomEngine {
    /// Create a new, empty engine.
    #[new]
    pub fn new() -> Self {
        AxiomEngine {
            registry: Arc::new(RwLock::new(Registry::new())),
        }
    }

    // ── Rule loading ──────────────────────────────────────────────────────

    /// Load a rule from an ARS YAML string.
    pub fn load_rule_yaml(&self, yaml: &str) -> PyResult<()> {
        let rule = parser::parse_rule_yaml_str(yaml).map_err(to_py)?;
        self.registry.write().unwrap().upsert_rule(rule).map_err(to_py)
    }

    /// Load a rule from an ARS JSON string.
    pub fn load_rule_json(&self, json: &str) -> PyResult<()> {
        let rule = parser::parse_rule_json_str(json).map_err(to_py)?;
        self.registry.write().unwrap().upsert_rule(rule).map_err(to_py)
    }

    /// Load a rule from a file path (YAML or JSON, detected by extension).
    pub fn load_rule_file(&self, path: &str) -> PyResult<()> {
        let bytes = std::fs::read(path)
            .map_err(|e| PyValueError::new_err(format!("{path}: {e}")))?;
        let rule = if path.ends_with(".json") {
            parser::parse_rule_json(&bytes)
        } else {
            parser::parse_rule_yaml(&bytes)
        }
        .map_err(to_py)?;
        self.registry.write().unwrap().upsert_rule(rule).map_err(to_py)
    }

    /// Load a bundle file (YAML with ``rules:`` and optional ``rulesets:``).
    pub fn load_bundle(&self, path: &str) -> PyResult<()> {
        let bytes = std::fs::read(path)
            .map_err(|e| PyValueError::new_err(format!("{path}: {e}")))?;
        let (rules, rulesets) = parser::parse_bundle_yaml(&bytes).map_err(to_py)?;
        let mut reg = self.registry.write().unwrap();
        reg.load_rules(rules).map_err(to_py)?;
        for rs in rulesets {
            reg.upsert_ruleset(rs);
        }
        Ok(())
    }

    // ── Evaluation ────────────────────────────────────────────────────────

    /// Evaluate a request dict against all loaded rules.
    ///
    /// The ``request`` dict follows :class:`EvaluationRequest`::
    ///
    ///     result = engine.evaluate({
    ///         "context": {"annual_income": 60000},
    ///         "strategy": "all_match",
    ///     })
    ///
    /// Returns a dict with ``matched``, ``matched_rules``, ``tags``,
    /// ``output_context``, ``duration_us``, ``trace``.
    pub fn evaluate<'py>(&self, py: Python<'py>, request: &Bound<'py, PyDict>) -> PyResult<PyObject> {
        // Convert Python dict → serde_json::Value (GIL held).
        let json_val: serde_json::Value = pythonize::depythonize(request)
            .map_err(|e| PyValueError::new_err(format!("request error: {e}")))?;
        let req: EvaluationRequest = serde_json::from_value(json_val).map_err(to_py)?;

        // Run the evaluation off the GIL.
        let resp_json = py.allow_threads(|| -> PyResult<String> {
            let reg = self.registry.read().unwrap();
            let resp = reg.evaluate(&req).map_err(to_py)?;
            serde_json::to_string(&resp).map_err(to_py)
        })?;

        // Convert JSON response → Python dict (GIL re-acquired).
        let resp_val: serde_json::Value = serde_json::from_str(&resp_json).map_err(to_py)?;
        let obj = pythonize::pythonize(py, &resp_val)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(obj.into())
    }

    /// Validate ARS YAML or JSON without loading.
    ///
    /// Returns ``None`` if valid, or an error string.
    pub fn validate_rule(&self, source: &str) -> Option<String> {
        parser::parse_rule_yaml_str(source)
            .err()
            .map(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Module-level validate function
// ---------------------------------------------------------------------------

/// Parse and validate ARS YAML or JSON.
///
/// Returns ``None`` if valid, or an error message string.
#[pyfunction]
pub fn validate_rule(yaml_or_json: &str) -> Option<String> {
    parser::parse_rule_yaml_str(yaml_or_json)
        .err()
        .map(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

#[pymodule]
fn axiom_rules(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AxiomEngine>()?;
    m.add_function(wrap_pyfunction!(validate_rule, m)?)?;
    Ok(())
}
