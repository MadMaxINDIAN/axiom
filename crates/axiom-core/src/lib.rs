pub mod error;
pub mod schema;
pub mod parser;
pub mod registry;
pub mod resolver;
pub mod evaluator;
pub mod operators;
pub mod actions;
pub mod expression;
pub mod trace;
pub mod strategy;
pub mod call_rule_guard;
pub mod timeout;

// Re-export the most commonly used types at the crate root.
pub use schema::{
    Action, ActionValue, ConditionGroup, ConditionNode, EvaluationRequest,
    EvaluationResponse, LeafCondition, Operator, Rule, Ruleset, Strategy,
};
pub use registry::{Registry, RuleFilter};
pub use error::{EvaluationError, ParseError};
pub use parser::{parse_rule_json_str, parse_rule_yaml_str, parse_bundle_yaml};
pub use trace::EvaluationTrace;
