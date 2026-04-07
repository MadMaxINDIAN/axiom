use serde::{Deserialize, Serialize};
use crate::schema::Strategy;

// ---------------------------------------------------------------------------
// Evaluation trace — identical structure across all consumption modes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationTrace {
    pub rules_evaluated:  usize,
    pub rules_matched:    usize,
    pub strategy:         Strategy,
    pub total_duration_us: u64,
    pub timed_out:        bool,
    pub rules:            Vec<RuleTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTrace {
    pub rule_id:          String,
    pub matched:          bool,
    pub conditions:       Vec<ConditionTrace>,
    pub short_circuited:  bool,
    pub actions_executed: Vec<String>,
    pub duration_us:      u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionTrace {
    pub field:        String,
    pub operator:     String,
    pub value:        serde_json::Value,
    pub actual_value: serde_json::Value,
    pub passed:       bool,
    pub duration_us:  u64,
}

/// Convenience builder used inside the evaluator.
pub struct RuleTraceBuilder {
    pub rule_id:  String,
    pub start_us: u64,
    pub conditions: Vec<ConditionTrace>,
    pub actions_executed: Vec<String>,
    pub short_circuited: bool,
}

impl RuleTraceBuilder {
    pub fn new(rule_id: impl Into<String>, start_us: u64) -> Self {
        RuleTraceBuilder {
            rule_id: rule_id.into(),
            start_us,
            conditions: Vec::new(),
            actions_executed: Vec::new(),
            short_circuited: false,
        }
    }

    pub fn finish(self, matched: bool, end_us: u64) -> RuleTrace {
        RuleTrace {
            rule_id:          self.rule_id,
            matched,
            conditions:       self.conditions,
            short_circuited:  self.short_circuited,
            actions_executed: self.actions_executed,
            duration_us:      end_us.saturating_sub(self.start_us),
        }
    }
}
