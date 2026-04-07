use std::collections::HashMap;

use crate::call_rule_guard;
use crate::error::{EvalResult, EvaluationError, RegistryError};
use crate::evaluator::{self, EvalConfig, merge_output};
use crate::schema::{EvaluationRequest, EvaluationResponse, Rule, Ruleset, Strategy};

// ---------------------------------------------------------------------------
// Rule Registry
// ---------------------------------------------------------------------------

/// In-memory rule registry.
///
/// Keyed by `(id, version)`. The active version per id is the highest version.
#[derive(Debug, Default)]
pub struct Registry {
    rules:    HashMap<(String, u32), Rule>,
    active:   HashMap<String, u32>,
    rulesets: HashMap<String, Ruleset>,
    /// Phase 2: runtime-configurable call_rule depth (default 8).
    pub max_call_depth: usize,
}

impl Registry {
    pub fn new() -> Self {
        Registry { max_call_depth: 8, ..Default::default() }
    }

    // ── Rule management ────────────────────────────────────────────────────

    pub fn upsert_rule(&mut self, rule: Rule) -> Result<(), RegistryError> {
        let id      = rule.id.clone();
        let version = rule.version;
        let current = self.active.get(&id).copied().unwrap_or(0);
        if version >= current { self.active.insert(id.clone(), version); }
        self.rules.insert((id, version), rule);
        call_rule_guard::validate_ruleset(&self.active_rules_all())?;
        Ok(())
    }

    pub fn load_rules(&mut self, rules: Vec<Rule>) -> Result<(), RegistryError> {
        for rule in rules {
            let id      = rule.id.clone();
            let version = rule.version;
            let current = self.active.get(&id).copied().unwrap_or(0);
            if version >= current { self.active.insert(id.clone(), version); }
            self.rules.insert((id, version), rule);
        }
        call_rule_guard::validate_ruleset(&self.active_rules_all())?;
        Ok(())
    }

    pub fn disable_rule(&mut self, id: &str) {
        if let Some(version) = self.active.get(id).copied() {
            if let Some(rule) = self.rules.get_mut(&(id.to_string(), version)) {
                rule.enabled = false;
            }
        }
    }

    pub fn get_rule(&self, id: &str) -> Option<&Rule> {
        let version = self.active.get(id)?;
        self.rules.get(&(id.to_string(), *version))
    }

    pub fn list_rules(&self, filter: &RuleFilter) -> Vec<&Rule> {
        let mut rules: Vec<&Rule> = self.active.iter()
            .filter_map(|(id, ver)| self.rules.get(&(id.clone(), *ver)))
            .filter(|r| filter.matches(r))
            .collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
        rules
    }

    pub fn list_versions(&self, id: &str) -> Vec<u32> {
        let mut v: Vec<u32> = self.rules.keys()
            .filter(|(rid, _)| rid == id)
            .map(|(_, ver)| *ver)
            .collect();
        v.sort_unstable();
        v
    }

    // ── Ruleset management ─────────────────────────────────────────────────

    pub fn upsert_ruleset(&mut self, rs: Ruleset) { self.rulesets.insert(rs.name.clone(), rs); }
    pub fn get_ruleset(&self, name: &str) -> Option<&Ruleset> { self.rulesets.get(name) }
    pub fn list_rulesets(&self) -> Vec<&Ruleset> {
        let mut v: Vec<&Ruleset> = self.rulesets.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    // ── Evaluation ─────────────────────────────────────────────────────────

    /// Evaluate a request.  Returns the response plus any triggered event names
    /// for the caller to dispatch as webhooks.
    pub fn evaluate_full(
        &self,
        req: &EvaluationRequest,
    ) -> EvalResult<(EvaluationResponse, Vec<String>)> {
        let rules = self.rules_for_request(req)?;

        // Provide a closure for call_rule dispatch
        let lookup = |id: &str| self.get_rule(id).cloned();

        let cfg = EvalConfig {
            strategy:       &req.strategy,
            dry_run:        req.dry_run,
            timeout_ms:     req.timeout_ms,
            max_call_depth: self.max_call_depth,
            rule_lookup:    &lookup,
        };

        let (trace, output_context, matched_rules, tags, triggered) =
            evaluator::evaluate(&rules, &req.context, &cfg)?;

        let resp = EvaluationResponse {
            matched: !matched_rules.is_empty(),
            matched_rules,
            tags,
            output_context,
            duration_us: trace.total_duration_us,
            trace,
        };
        Ok((resp, triggered))
    }

    /// Convenience wrapper — discards triggered events (suitable for library/CLI use).
    pub fn evaluate(&self, req: &EvaluationRequest) -> EvalResult<EvaluationResponse> {
        self.evaluate_full(req).map(|(resp, _)| resp)
    }

    // ── Internal ───────────────────────────────────────────────────────────

    fn active_rules_all(&self) -> Vec<Rule> {
        self.active.iter()
            .filter_map(|(id, ver)| self.rules.get(&(id.clone(), *ver)).cloned())
            .collect()
    }

    fn rules_for_request(&self, req: &EvaluationRequest) -> EvalResult<Vec<Rule>> {
        if let Some(rule_id) = &req.rule_id {
            let rule = self.get_rule(rule_id)
                .ok_or_else(|| EvaluationError::Registry(RegistryError::NotFound(rule_id.clone())))?;
            if !rule.enabled { return Ok(vec![]); }
            return Ok(vec![rule.clone()]);
        }

        if let Some(name) = &req.ruleset {
            let rs = self.get_ruleset(name)
                .ok_or_else(|| EvaluationError::Registry(RegistryError::RulesetNotFound(name.clone())))?;
            let mut rules: Vec<Rule> = rs.rule_ids.iter()
                .filter_map(|id| self.get_rule(id))
                .filter(|r| r.enabled)
                .cloned()
                .collect();
            rules.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
            return Ok(rules);
        }

        // No target: evaluate all enabled rules
        Ok(self.list_rules(&RuleFilter::default()).into_iter().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// RuleFilter
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct RuleFilter {
    pub tag:     Option<String>,
    pub enabled: Option<bool>,
}

impl RuleFilter {
    fn matches(&self, rule: &Rule) -> bool {
        if let Some(ref tag) = self.tag {
            if !rule.tags.contains(tag) { return false; }
        }
        if let Some(enabled) = self.enabled {
            if rule.enabled != enabled { return false; }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;
    use serde_json::json;

    fn simple_rule(id: &str, priority: i32) -> Rule {
        Rule {
            ars_version: 1, id: id.into(), name: id.into(), description: None,
            version: 1, priority, enabled: true, tags: vec![], extends: None,
            conditions: ConditionGroup::All(AllGroup { all: vec![] }),
            actions: vec![Action::Tag { value: id.into() }],
            metadata: None,
        }
    }

    #[test]
    fn upsert_and_get() {
        let mut reg = Registry::new();
        reg.upsert_rule(simple_rule("a", 10)).unwrap();
        assert!(reg.get_rule("a").is_some());
    }

    #[test]
    fn evaluate_all_match() {
        let mut reg = Registry::new();
        reg.upsert_rule(simple_rule("a", 10)).unwrap();
        reg.upsert_rule(simple_rule("b", 5)).unwrap();
        let req = EvaluationRequest { strategy: Strategy::AllMatch, context: json!({}), ..Default::default() };
        let resp = reg.evaluate(&req).unwrap();
        assert_eq!(resp.matched_rules.len(), 2);
    }

    #[test]
    fn dry_run_returns_full_trace() {
        let mut reg = Registry::new();
        reg.upsert_rule(simple_rule("a", 0)).unwrap();
        let req = EvaluationRequest { dry_run: true, context: json!({}), ..Default::default() };
        let resp = reg.evaluate(&req).unwrap();
        assert_eq!(resp.trace.rules_evaluated, 1);
    }
}
