use serde_json::Value;
use std::time::Instant;

use crate::error::{EvalResult, EvaluationError};
use crate::schema::{
    Action, ActionValue, ConditionGroup, ConditionNode, LeafCondition, Rule, Strategy,
};
use crate::trace::{ConditionTrace, EvaluationTrace, RuleTrace, RuleTraceBuilder};
use crate::{expression, operators, resolver};

// ---------------------------------------------------------------------------
// Evaluation configuration
// ---------------------------------------------------------------------------

/// All knobs for a single evaluation run.
pub struct EvalConfig<'a> {
    pub strategy:       &'a Strategy,
    pub dry_run:        bool,
    pub timeout_ms:     Option<u64>,
    /// Maximum call_rule chain depth.  Phase 1 = 4, Phase 2 = 8.
    pub max_call_depth: usize,
    /// Callback used to look up a rule by ID for call_rule dispatch.
    /// The evaluator is I/O-free; the caller provides the lookup.
    pub rule_lookup:    &'a dyn Fn(&str) -> Option<Rule>,
}

impl<'a> EvalConfig<'a> {
    pub fn simple(strategy: &'a Strategy, rule_lookup: &'a dyn Fn(&str) -> Option<Rule>) -> Self {
        EvalConfig {
            strategy,
            dry_run:        false,
            timeout_ms:     None,
            max_call_depth: 8,
            rule_lookup,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Evaluate `rules` (priority-sorted, enabled-filtered) against `context`.
///
/// Returns `(trace, output_context, matched_rule_ids, tags, triggered_events)`.
/// `triggered_events` is a list of event names from `trigger` actions — the
/// caller (server/library) is responsible for dispatching them.
pub fn evaluate(
    rules:  &[Rule],
    context: &Value,
    cfg:    &EvalConfig,
) -> EvalResult<(EvaluationTrace, Value, Vec<String>, Vec<String>, Vec<String>)> {
    let start     = Instant::now();
    let budget_us = cfg.timeout_ms.map(|ms| ms * 1_000);

    let mut trace = EvaluationTrace {
        strategy: cfg.strategy.clone(),
        ..Default::default()
    };
    let mut output_context = Value::Object(Default::default());
    let mut matched_rules: Vec<String> = Vec::new();
    let mut tags:          Vec<String> = Vec::new();
    let mut triggered:     Vec<String> = Vec::new();
    let mut scored: Vec<(f64, RuleTrace, Value, Vec<String>, Vec<String>)> = Vec::new();

    for rule in rules {
        // Timeout check (§5.2 step 3)
        if let Some(budget) = budget_us {
            if start.elapsed().as_micros() as u64 >= budget {
                trace.timed_out = true;
                break;
            }
        }

        let rule_start = start.elapsed().as_micros() as u64;
        let mut builder = RuleTraceBuilder::new(&rule.id, rule_start);

        let (matched, score) = eval_condition_group(
            &rule.conditions, context, cfg.dry_run, &mut builder, cfg.strategy,
        )?;

        trace.rules_evaluated += 1;

        // In dry-run: execute actions for trace even if unmatched (but skip side effects)
        if matched || cfg.dry_run {
            let mut local_out  = Value::Object(Default::default());
            let mut local_tags: Vec<String> = Vec::new();
            let mut local_trig: Vec<String> = Vec::new();

            execute_actions(
                &rule.actions, context,
                &mut local_out, &mut local_tags, &mut local_trig,
                &mut builder,
                0, cfg.max_call_depth,
                cfg.dry_run,
                cfg.rule_lookup,
            )?;

            let rule_end = start.elapsed().as_micros() as u64;
            let rule_trace = builder.finish(matched, rule_end);
            trace.rules_matched += if matched { 1 } else { 0 };

            match cfg.strategy {
                Strategy::FirstMatch if matched => {
                    matched_rules.push(rule.id.clone());
                    tags.extend(local_tags);
                    triggered.extend(local_trig);
                    merge_output(&mut output_context, local_out);
                    trace.rules.push(rule_trace);
                    break;
                }
                Strategy::AllMatch if matched => {
                    matched_rules.push(rule.id.clone());
                    tags.extend(local_tags);
                    triggered.extend(local_trig);
                    merge_output(&mut output_context, local_out);
                    trace.rules.push(rule_trace);
                }
                Strategy::Scored => {
                    scored.push((score, rule_trace, local_out, local_tags, local_trig));
                }
                _ => {
                    // dry-run non-matching rule: keep trace, discard output
                    trace.rules.push(rule_trace);
                }
            }
        } else {
            let rule_end = start.elapsed().as_micros() as u64;
            trace.rules.push(builder.finish(false, rule_end));
        }
    }

    // Resolve scored strategy (§5.4)
    if let Strategy::Scored = cfg.strategy {
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (score, rule_trace, local_out, local_tags, local_trig) in scored {
            if score > 0.0 {
                matched_rules.push(rule_trace.rule_id.clone());
                tags.extend(local_tags);
                triggered.extend(local_trig);
                merge_output(&mut output_context, local_out);
            }
            trace.rules.push(rule_trace);
        }
    }

    trace.total_duration_us = start.elapsed().as_micros() as u64;
    tags.dedup();
    triggered.dedup();

    Ok((trace, output_context, matched_rules, tags, triggered))
}

// ---------------------------------------------------------------------------
// Condition evaluation
// ---------------------------------------------------------------------------

fn eval_condition_group(
    group:    &ConditionGroup,
    context:  &Value,
    dry_run:  bool,
    builder:  &mut RuleTraceBuilder,
    strategy: &Strategy,
) -> EvalResult<(bool, f64)> {
    let nodes: &[ConditionNode];
    let is_all; let is_any; let is_none;

    match group {
        ConditionGroup::All(g)  => { nodes = &g.all;  is_all = true;  is_any = false; is_none = false; }
        ConditionGroup::Any(g)  => { nodes = &g.any;  is_all = false; is_any = true;  is_none = false; }
        ConditionGroup::None(g) => { nodes = &g.none; is_all = false; is_any = false; is_none = true;  }
        ConditionGroup::Not(g)  => {
            let (child_matched, _) = eval_node(&g.not, context, dry_run, builder, strategy)?;
            return Ok((!child_matched, if !child_matched { 1.0 } else { 0.0 }));
        }
    }

    let total = nodes.len();
    let mut passed = 0usize;

    for node in nodes {
        let (node_matched, _) = eval_node(node, context, dry_run, builder, strategy)?;
        if node_matched { passed += 1; }

        // Short-circuit (§5.3) — disabled in dry-run so every condition is traced
        if !dry_run {
            if is_all  && !node_matched { builder.short_circuited = true; break; }
            if (is_any || is_none) &&  node_matched { builder.short_circuited = true; break; }
        }
    }

    let matched = if is_all  { passed == total }
                  else if is_any  { passed > 0 }
                  else if is_none { passed == 0 }
                  else { unreachable!() };

    Ok((matched, if matched { 1.0 } else { 0.0 }))
}

fn eval_node(
    node:     &ConditionNode,
    context:  &Value,
    dry_run:  bool,
    builder:  &mut RuleTraceBuilder,
    strategy: &Strategy,
) -> EvalResult<(bool, f64)> {
    match node {
        ConditionNode::Group(g) => eval_condition_group(g, context, dry_run, builder, strategy),
        ConditionNode::Leaf(l)  => {
            let (matched, ct) = eval_leaf(l, context)?;
            builder.conditions.push(ct);
            Ok((matched, if matched { 1.0 } else { 0.0 }))
        }
    }
}

fn eval_leaf(leaf: &LeafCondition, context: &Value) -> EvalResult<(bool, ConditionTrace)> {
    let field_val  = resolver::resolve_owned(context, &leaf.field);
    let field2_val = leaf.field2.as_deref().map(|p| resolver::resolve_owned(context, p));

    let matched = operators::apply(
        &leaf.operator, &field_val, leaf.value.as_ref(), field2_val.as_ref(),
    );

    let ct = ConditionTrace {
        field:        leaf.field.clone(),
        operator:     format!("{:?}", leaf.operator).to_lowercase(),
        value:        leaf.value.clone().unwrap_or(Value::Null),
        actual_value: field_val,
        passed:       matched,
        duration_us:  0,
    };
    Ok((matched, ct))
}

// ---------------------------------------------------------------------------
// Action execution
// ---------------------------------------------------------------------------

/// Execute an action list.
///
/// - `dry_run = true` skips I/O side effects (`trigger`, actual `call_rule`
///   dispatch) while still recording what *would* have fired in the trace.
/// - `rule_lookup` is provided by the registry for `call_rule` dispatch.
/// - Collected trigger event names are appended to `triggered`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_actions(
    actions:     &[Action],
    context:     &Value,
    output:      &mut Value,
    tags:        &mut Vec<String>,
    triggered:   &mut Vec<String>,
    builder:     &mut RuleTraceBuilder,
    call_depth:  usize,
    max_depth:   usize,
    dry_run:     bool,
    rule_lookup: &dyn Fn(&str) -> Option<Rule>,
) -> EvalResult<Option<Value>> {
    for action in actions {
        match action {
            Action::Set { field, value } => {
                let resolved = resolve_action_value(value, context)?;
                set_path(output, field, resolved.clone());
                builder.actions_executed.push(format!("set {field} = {resolved}"));
            }

            Action::Increment { field, value } => {
                let delta = value.as_ref().and_then(|v| v.as_f64()).unwrap_or(1.0);
                let current = resolve_output_or_context(output, context, field)
                    .as_f64().unwrap_or(0.0);
                let new_val = Value::Number(
                    serde_json::Number::from_f64(current + delta)
                        .ok_or(EvaluationError::Overflow)?
                );
                set_path(output, field, new_val);
                builder.actions_executed.push(format!("increment {field} by {delta}"));
            }

            Action::Append { field, value } => {
                let resolved = resolve_action_value(value, context)?;
                match get_path_mut(output, field) {
                    Some(Value::Array(a)) => { a.push(resolved.clone()); }
                    _                    => set_path(output, field, Value::Array(vec![resolved.clone()])),
                }
                builder.actions_executed.push(format!("append {field}"));
            }

            Action::Tag { value } => {
                tags.push(value.clone());
                builder.actions_executed.push(format!("tag {value}"));
            }

            Action::Log { level, message } => {
                builder.actions_executed.push(format!("log[{level:?}] {message}"));
            }

            Action::Trigger { event } => {
                builder.actions_executed.push(format!("trigger {event}"));
                if !dry_run {
                    // Collect for caller to dispatch; axiom-core has no I/O (§3)
                    triggered.push(event.clone());
                }
            }

            Action::CallRule { rule_id } => {
                if call_depth >= max_depth {
                    return Err(EvaluationError::CallDepthExceeded { limit: max_depth });
                }
                builder.actions_executed.push(format!("call_rule {rule_id}"));

                if !dry_run {
                    // Dispatch: look up the rule and evaluate it synchronously
                    if let Some(callee) = rule_lookup(rule_id) {
                        let mut callee_builder = RuleTraceBuilder::new(rule_id, 0);
                        // Use AllMatch semantics for the callee (we want all its actions to run)
                        let no_lookup: &dyn Fn(&str) -> Option<Rule> = &|_| None;
                        execute_actions(
                            &callee.actions, context, output, tags, triggered,
                            &mut callee_builder,
                            call_depth + 1, max_depth,
                            dry_run,
                            rule_lookup,
                        )?;
                        // Merge callee trace entries into parent
                        builder.actions_executed.extend(
                            callee_builder.actions_executed.iter().map(|s| format!("  [{rule_id}] {s}"))
                        );
                    }
                }
            }

            Action::Return { value } => {
                let ret = match value {
                    Some(av) => Some(resolve_action_value(av, context)?),
                    None     => None,
                };
                builder.actions_executed.push("return".to_string());
                return Ok(ret);
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn resolve_action_value(av: &ActionValue, context: &Value) -> EvalResult<Value> {
    match av {
        ActionValue::Literal(v)  => Ok(v.clone()),
        ActionValue::Template(s) => {
            if expression::extract_template(s).is_some() {
                expression::eval_template(s, context)
            } else {
                Ok(Value::String(s.clone()))
            }
        }
    }
}

fn resolve_output_or_context(output: &Value, context: &Value, path: &str) -> Value {
    let from_out = resolver::resolve_owned(output, path);
    if !from_out.is_null() { from_out } else { resolver::resolve_owned(context, path) }
}

/// Write `value` at a dot-notation path into a JSON object, creating
/// intermediate objects as needed.
pub fn set_path(target: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = target;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = current {
                map.insert(part.to_string(), value);
                return;
            }
        } else if let Value::Object(map) = current {
            current = map.entry(part.to_string())
                .or_insert_with(|| Value::Object(Default::default()));
        } else {
            return;
        }
    }
}

fn get_path_mut<'a>(target: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = target;
    for part in &parts {
        match current {
            Value::Object(map) => current = map.get_mut(*part)?,
            _                  => return None,
        }
    }
    Some(current)
}

pub(crate) fn merge_output(target: &mut Value, patch: Value) {
    if let (Value::Object(t), Value::Object(p)) = (target, patch) {
        for (k, v) in p {
            match t.get_mut(&k) {
                Some(existing) => merge_output(existing, v),
                None           => { t.insert(k, v); }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AllGroup, ConditionGroup, ConditionNode, LeafCondition, Operator};
    use serde_json::json;

    fn make_rule(id: &str, priority: i32, conditions: ConditionGroup, actions: Vec<Action>) -> Rule {
        Rule {
            ars_version: 1, id: id.to_string(), name: id.to_string(), description: None,
            version: 1, priority, enabled: true, tags: vec![], extends: None,
            conditions, actions, metadata: None,
        }
    }

    fn no_lookup(_: &str) -> Option<Rule> { None }

    fn run(rules: &[Rule], context: &Value, strategy: Strategy) -> (Vec<String>, Vec<String>) {
        let cfg = EvalConfig { strategy: &strategy, dry_run: false, timeout_ms: None,
            max_call_depth: 8, rule_lookup: &no_lookup };
        let (_, _, matched, tags, _) = evaluate(rules, context, &cfg).unwrap();
        (matched, tags)
    }

    #[test]
    fn first_match() {
        let rule = make_rule("r1", 10,
            ConditionGroup::All(AllGroup { all: vec![
                ConditionNode::Leaf(LeafCondition { field: "x".into(), operator: Operator::Eq,
                    value: Some(json!(1)), field2: None })
            ]}),
            vec![Action::Tag { value: "hit".into() }],
        );
        let (matched, tags) = run(&[rule], &json!({ "x": 1 }), Strategy::FirstMatch);
        assert_eq!(matched, vec!["r1"]);
        assert_eq!(tags, vec!["hit"]);
    }

    #[test]
    fn no_match() {
        let rule = make_rule("r1", 0,
            ConditionGroup::All(AllGroup { all: vec![
                ConditionNode::Leaf(LeafCondition { field: "x".into(), operator: Operator::Eq,
                    value: Some(json!(99)), field2: None })
            ]}),
            vec![],
        );
        let (matched, _) = run(&[rule], &json!({ "x": 1 }), Strategy::FirstMatch);
        assert!(matched.is_empty());
    }

    #[test]
    fn dry_run_evaluates_all_conditions() {
        // Two conditions: first passes, second fails — in dry-run both should appear in trace
        let rule = make_rule("r1", 0,
            ConditionGroup::All(AllGroup { all: vec![
                ConditionNode::Leaf(LeafCondition { field: "a".into(), operator: Operator::Eq,
                    value: Some(json!(1)), field2: None }),
                ConditionNode::Leaf(LeafCondition { field: "b".into(), operator: Operator::Eq,
                    value: Some(json!(99)), field2: None }),
            ]}),
            vec![Action::Tag { value: "dry".into() }],
        );
        let cfg = EvalConfig { strategy: &Strategy::AllMatch, dry_run: true,
            timeout_ms: None, max_call_depth: 8, rule_lookup: &no_lookup };
        let (trace, _, _, _, _) = evaluate(&[rule], &json!({ "a": 1, "b": 1 }), &cfg).unwrap();
        // Both conditions should appear in trace despite short-circuit that would normally stop
        assert_eq!(trace.rules[0].conditions.len(), 2);
    }

    #[test]
    fn trigger_skipped_in_dry_run() {
        let rule = make_rule("r1", 0,
            ConditionGroup::All(AllGroup { all: vec![
                ConditionNode::Leaf(LeafCondition { field: "x".into(), operator: Operator::Eq,
                    value: Some(json!(1)), field2: None })
            ]}),
            vec![Action::Trigger { event: "payment-flagged".into() }],
        );
        let cfg = EvalConfig { strategy: &Strategy::FirstMatch, dry_run: true,
            timeout_ms: None, max_call_depth: 8, rule_lookup: &no_lookup };
        let (_, _, _, _, triggered) =
            evaluate(&[rule], &json!({ "x": 1 }), &cfg).unwrap();
        assert!(triggered.is_empty(), "trigger must not fire in dry-run");
    }

    #[test]
    fn trigger_fires_in_normal_mode() {
        let rule = make_rule("r1", 0,
            ConditionGroup::All(AllGroup { all: vec![
                ConditionNode::Leaf(LeafCondition { field: "x".into(), operator: Operator::Eq,
                    value: Some(json!(1)), field2: None })
            ]}),
            vec![Action::Trigger { event: "order-placed".into() }],
        );
        let cfg = EvalConfig { strategy: &Strategy::FirstMatch, dry_run: false,
            timeout_ms: None, max_call_depth: 8, rule_lookup: &no_lookup };
        let (_, _, _, _, triggered) =
            evaluate(&[rule], &json!({ "x": 1 }), &cfg).unwrap();
        assert_eq!(triggered, vec!["order-placed"]);
    }
}
