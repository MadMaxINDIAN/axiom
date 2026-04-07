use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Schema validation error at '{field}': {message}")]
    Schema { field: String, message: String },

    #[error("Unsupported ARS version {got}, expected {expected}")]
    ArsVersion { got: u32, expected: u32 },

    #[error("'not' group must have exactly one child; use 'none' for multi-child NOR")]
    NotGroupArray,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Rule not found: {0}")]
    NotFound(String),

    #[error("Ruleset not found: {0}")]
    RulesetNotFound(String),

    #[error("Cyclic rule dependency detected: {cycle}")]
    CyclicRuleDependency { cycle: String },

    #[error("Unresolved rule reference in call_rule: '{0}'")]
    UnresolvedRuleReference(String),
}

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),

    #[error("Resolver error at path '{path}': {message}")]
    Resolver { path: String, message: String },

    #[error("Expression error: {0}")]
    Expression(String),

    #[error("Arithmetic overflow in expression")]
    Overflow,

    #[error("call_rule depth limit ({limit}) exceeded")]
    CallDepthExceeded { limit: usize },

    #[error("Evaluation timed out after {budget_ms} ms")]
    Timeout { budget_ms: u64 },

    #[error("Action error: {0}")]
    Action(String),
}

pub type ParseResult<T>  = Result<T, ParseError>;
pub type EvalResult<T>   = Result<T, EvaluationError>;
