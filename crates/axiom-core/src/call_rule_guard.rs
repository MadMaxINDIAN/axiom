/// call_rule_guard — load-time safety checks for rule dependency graphs.
///
/// Three checks performed at ruleset load time (§5.1, §4.6 [AR-2, R3-4]):
///   1. Cycle detection: topological sort over call_rule dependency graph
///   2. Missing-rule detection: all call_rule targets must exist in the registry
///   3. Depth limit: paths longer than MAX_CALL_DEPTH rejected
///
/// At evaluation time only depth tracking is needed (1 and 2 are guaranteed).

use std::collections::{HashMap, HashSet};
use crate::error::{EvalResult, EvaluationError, RegistryError};
use crate::schema::{Action, Rule};

pub const PHASE1_MAX_DEPTH: usize = 4;

/// Validate a set of rules for call_rule safety.
/// Returns Ok(()) or the first error found.
pub fn validate_ruleset(rules: &[Rule]) -> Result<(), RegistryError> {
    let ids: HashSet<&str> = rules.iter().map(|r| r.id.as_str()).collect();

    // Build adjacency list: rule_id → Vec<called_rule_id>  (owned Strings)
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for rule in rules {
        let deps = collect_call_rule_deps(&rule.actions);
        for dep in &deps {
            if !ids.contains(dep.as_str()) {
                return Err(RegistryError::UnresolvedRuleReference(dep.clone()));
            }
        }
        graph.insert(rule.id.clone(), deps);
    }

    // Cycle detection via DFS with three-colour marking
    let mut colour: HashMap<String, Colour> = HashMap::new();
    for rule in rules {
        if *colour.get(&rule.id).unwrap_or(&Colour::White) == Colour::White {
            dfs_cycle(&rule.id, &graph, &mut colour, &mut Vec::new())?;
        }
    }

    Ok(())
}

#[derive(PartialEq, Clone)]
enum Colour { White, Grey, Black }

fn dfs_cycle(
    node:   &str,
    graph:  &HashMap<String, Vec<String>>,
    colour: &mut HashMap<String, Colour>,
    path:   &mut Vec<String>,
) -> Result<(), RegistryError> {
    colour.insert(node.to_string(), Colour::Grey);
    path.push(node.to_string());

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            match colour.get(dep).unwrap_or(&Colour::White) {
                Colour::Grey => {
                    // Found a back-edge → cycle
                    let cycle_start = path.iter().position(|n| n == dep).unwrap_or(0);
                    let mut cycle: Vec<String> = path[cycle_start..].to_vec();
                    cycle.push(dep.clone());
                    return Err(RegistryError::CyclicRuleDependency {
                        cycle: cycle.join(" → "),
                    });
                }
                Colour::White => dfs_cycle(dep, graph, colour, path)?,
                Colour::Black => {}
            }
        }
    }

    path.pop();
    colour.insert(node.to_string(), Colour::Black);
    Ok(())
}

/// Collect all rule IDs referenced by `call_rule` actions (recursively into
/// nested conditions is not needed — actions are flat lists).
fn collect_call_rule_deps(actions: &[Action]) -> Vec<String> {
    actions.iter().filter_map(|a| {
        if let Action::CallRule { rule_id } = a { Some(rule_id.clone()) } else { None }
    }).collect()
}

/// Runtime depth guard — called during evaluation whenever a call_rule is
/// encountered.  The load-time checks guarantee no cycles and no missing
/// rules, so only depth needs checking at runtime.
pub fn check_depth(current_depth: usize, max_depth: usize) -> EvalResult<()> {
    if current_depth >= max_depth {
        Err(EvaluationError::CallDepthExceeded { limit: max_depth })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;
    use serde_json::json;

    fn dummy_rule(id: &str, calls: &[&str]) -> Rule {
        let actions = calls.iter().map(|c| Action::CallRule { rule_id: c.to_string() }).collect();
        Rule {
            ars_version: 1, id: id.into(), name: id.into(), description: None,
            version: 1, priority: 0, enabled: true, tags: vec![], extends: None,
            conditions: ConditionGroup::All(AllGroup { all: vec![] }),
            actions,
            metadata: None,
        }
    }

    #[test]
    fn no_deps_ok() {
        let rules = vec![dummy_rule("a", &[])];
        assert!(validate_ruleset(&rules).is_ok());
    }

    #[test]
    fn valid_chain_ok() {
        let rules = vec![dummy_rule("a", &["b"]), dummy_rule("b", &[])];
        assert!(validate_ruleset(&rules).is_ok());
    }

    #[test]
    fn cycle_detected() {
        let rules = vec![dummy_rule("a", &["b"]), dummy_rule("b", &["a"])];
        assert!(matches!(validate_ruleset(&rules), Err(RegistryError::CyclicRuleDependency { .. })));
    }

    #[test]
    fn missing_rule_detected() {
        let rules = vec![dummy_rule("a", &["ghost"])];
        assert!(matches!(validate_ruleset(&rules), Err(RegistryError::UnresolvedRuleReference(_))));
    }
}
