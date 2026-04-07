use crate::error::{ParseError, ParseResult};
use crate::schema::{ARS_VERSION, Rule, Ruleset};

/// Parse a single ARS rule from YAML bytes.
pub fn parse_rule_yaml(bytes: &[u8]) -> ParseResult<Rule> {
    let rule: Rule = serde_yaml::from_slice(bytes)?;
    validate_rule(&rule)?;
    Ok(rule)
}

/// Parse a single ARS rule from JSON bytes.
pub fn parse_rule_json(bytes: &[u8]) -> ParseResult<Rule> {
    let rule: Rule = serde_json::from_slice(bytes)?;
    validate_rule(&rule)?;
    Ok(rule)
}

/// Parse a single ARS rule from a YAML string.
pub fn parse_rule_yaml_str(s: &str) -> ParseResult<Rule> {
    parse_rule_yaml(s.as_bytes())
}

/// Parse a single ARS rule from a JSON string.
pub fn parse_rule_json_str(s: &str) -> ParseResult<Rule> {
    parse_rule_json(s.as_bytes())
}

/// Parse a bundle YAML file that may contain a list of rules and rulesets.
///
/// Bundle format:
/// ```yaml
/// rules:
///   - ars_version: 1
///     id: ...
/// rulesets:
///   - name: ...
///     rule_ids: [...]
/// ```
pub fn parse_bundle_yaml(bytes: &[u8]) -> ParseResult<(Vec<Rule>, Vec<Ruleset>)> {
    let doc: serde_yaml::Value = serde_yaml::from_slice(bytes)?;
    let mut rules: Vec<Rule> = Vec::new();
    let mut rulesets: Vec<Ruleset> = Vec::new();

    if let Some(r_list) = doc.get("rules").and_then(|v| v.as_sequence()) {
        for entry in r_list {
            let json_val = serde_json::to_value(entry)?;
            let rule: Rule = serde_json::from_value(json_val)?;
            validate_rule(&rule)?;
            rules.push(rule);
        }
    }

    if let Some(rs_list) = doc.get("rulesets").and_then(|v| v.as_sequence()) {
        for entry in rs_list {
            let json_val = serde_json::to_value(entry)?;
            let rs: Ruleset = serde_json::from_value(json_val)?;
            rulesets.push(rs);
        }
    }

    Ok((rules, rulesets))
}

/// Parse a bundle JSON file that may contain a list of rules and rulesets.
pub fn parse_bundle_json(bytes: &[u8]) -> ParseResult<(Vec<Rule>, Vec<Ruleset>)> {
    let doc: serde_json::Value = serde_json::from_slice(bytes)?;
    let mut rules: Vec<Rule> = Vec::new();
    let mut rulesets: Vec<Ruleset> = Vec::new();

    if let Some(r_list) = doc.get("rules").and_then(|v| v.as_array()) {
        for entry in r_list {
            let rule: Rule = serde_json::from_value(entry.clone())?;
            validate_rule(&rule)?;
            rules.push(rule);
        }
    }

    if let Some(rs_list) = doc.get("rulesets").and_then(|v| v.as_array()) {
        for entry in rs_list {
            let rs: Ruleset = serde_json::from_value(entry.clone())?;
            rulesets.push(rs);
        }
    }

    Ok((rules, rulesets))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_rule(rule: &Rule) -> ParseResult<()> {
    // ARS version check
    if rule.ars_version != ARS_VERSION {
        return Err(ParseError::ArsVersion {
            got:      rule.ars_version,
            expected: ARS_VERSION,
        });
    }

    // ID must not be empty
    if rule.id.trim().is_empty() {
        return Err(ParseError::Schema {
            field:   "id".into(),
            message: "rule id must not be empty".into(),
        });
    }

    // Name must not be empty
    if rule.name.trim().is_empty() {
        return Err(ParseError::Schema {
            field:   "name".into(),
            message: "rule name must not be empty".into(),
        });
    }

    // At least one action
    if rule.actions.is_empty() {
        return Err(ParseError::Schema {
            field:   "actions".into(),
            message: "rule must have at least one action".into(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML: &str = r#"
ars_version: 1
id: test-rule
name: Test Rule
version: 1
priority: 10
enabled: true
conditions:
  all:
    - field: score
      operator: gte
      value: 100
actions:
  - type: tag
    value: passed
"#;

    #[test]
    fn parses_valid_yaml() {
        let rule = parse_rule_yaml_str(VALID_YAML).unwrap();
        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.priority, 10);
    }

    #[test]
    fn rejects_wrong_ars_version() {
        let bad = VALID_YAML.replace("ars_version: 1", "ars_version: 99");
        assert!(parse_rule_yaml_str(&bad).is_err());
    }

    #[test]
    fn rejects_empty_id() {
        let bad = VALID_YAML.replace("id: test-rule", "id: \"\"");
        assert!(parse_rule_yaml_str(&bad).is_err());
    }
}
