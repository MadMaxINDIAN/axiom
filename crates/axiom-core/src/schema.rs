use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ARS_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Rule — top-level document
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub ars_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: u32,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub extends: Option<String>,
    pub conditions: ConditionGroup,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

fn default_enabled() -> bool { true }

// ---------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------

/// A condition group is an object with exactly one logical-operator key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionGroup {
    All(AllGroup),
    Any(AnyGroup),
    None(NoneGroup),
    Not(NotGroup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllGroup  { pub all:  Vec<ConditionNode> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnyGroup  { pub any:  Vec<ConditionNode> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoneGroup { pub none: Vec<ConditionNode> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotGroup  { pub not:  Box<ConditionNode> }

/// A condition node is either a leaf comparison or a nested group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionNode {
    Group(ConditionGroup),
    Leaf(LeafCondition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafCondition {
    pub field:    String,
    #[serde(alias = "op")]
    pub operator: Operator,
    /// Static value for most operators.
    #[serde(default)]
    pub value:    Option<serde_json::Value>,
    /// Cross-field comparison: compare field vs context[field2].
    #[serde(default)]
    pub field2:   Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    // Comparison
    Eq, Neq, Gt, Gte, Lt, Lte,
    // String
    Contains, StartsWith, EndsWith, Matches, In, NotIn,
    // Numeric
    Between, Outside, DivisibleBy,
    // Null / empty
    IsNull, IsNotNull, IsEmpty, IsNotEmpty,
    // Date / time
    Before, After, WithinDays, IsWeekday, IsWeekend,
    // List
    ContainsAny, ContainsAll, LengthEq, LengthGt, LengthLt,
    // Type check
    IsType,
    // Cross-field (legacy explicit names)
    FieldGtField, FieldEqField,
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Set       { field: String, value: ActionValue },
    Increment { field: String, #[serde(default)] value: Option<serde_json::Value> },
    Append    { field: String, value: ActionValue },
    Tag       { value: String },
    Trigger   { event: String },
    CallRule  { rule_id: String },
    Return    { #[serde(default)] value: Option<ActionValue> },
    Log       { level: LogLevel, message: String },
}

/// An action value is either a template expression (starts with `{{`) or a
/// plain JSON literal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionValue {
    Template(String),        // "{{ expr }}"
    Literal(serde_json::Value),
}

impl ActionValue {
    pub fn is_template(&self) -> bool {
        match self {
            ActionValue::Template(s) => s.trim_start().starts_with("{{"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel { Debug, Info, Warn }

// ---------------------------------------------------------------------------
// Ruleset
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruleset {
    pub name:        String,
    pub rule_ids:    Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Evaluation request / response  (used by both server and library)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    #[default]
    FirstMatch,
    AllMatch,
    Scored,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationRequest {
    #[serde(default)]
    pub rule_id:    Option<String>,
    #[serde(default)]
    pub ruleset:    Option<String>,
    #[serde(default)]
    pub strategy:   Strategy,
    #[serde(default)]
    pub dry_run:    bool,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub context:    serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResponse {
    pub matched:        bool,
    pub matched_rules:  Vec<String>,
    pub tags:           Vec<String>,
    pub output_context: serde_json::Value,
    pub duration_us:    u64,
    pub trace:          crate::trace::EvaluationTrace,
}
