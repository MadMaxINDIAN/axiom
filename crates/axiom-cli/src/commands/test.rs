use std::path::Path;
use serde::Deserialize;
use serde_json::Value;
use axiom_core::{parser, Registry, EvaluationRequest, Strategy};

/// Run all `*.test.yaml` files found under `path`.
/// Returns (pass_count, fail_count).
pub fn run(path: &Path, output_junit: Option<&Path>) -> anyhow::Result<(usize, usize)> {
    let test_files = walk_test_files(path);
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut junit_cases: Vec<JUnitCase> = Vec::new();

    for file in &test_files {
        let bytes = std::fs::read(file)?;
        let suite: TestSuite = serde_yaml::from_slice(&bytes)?;

        // Load the target rule
        let rule_file = resolve_rule_file(file, &suite.rule)?;
        let rule_bytes = std::fs::read(&rule_file)?;
        let rule = parser::parse_rule_yaml(&rule_bytes)?;

        let mut registry = Registry::new();
        registry.upsert_rule(rule).map_err(|e| anyhow::anyhow!("{e}"))?;

        for test in &suite.tests {
            let req = EvaluationRequest {
                rule_id:    Some(suite.rule.clone()),
                strategy:   Strategy::AllMatch,
                dry_run:    false,
                timeout_ms: Some(1_000),
                context:    test.context.clone(),
                ..Default::default()
            };

            match registry.evaluate(&req) {
                Ok(resp) => {
                    let ok = check_expectations(test, &resp);
                    if ok {
                        println!("  PASS  {}: {}", file.display(), test.name);
                        pass += 1;
                        junit_cases.push(JUnitCase { name: test.name.clone(), failure: None });
                    } else {
                        let msg = format!(
                            "expected matched={:?}, got matched={}, matched_rules={:?}",
                            test.expect.matched, resp.matched, resp.matched_rules
                        );
                        eprintln!("  FAIL  {}: {} — {msg}", file.display(), test.name);
                        fail += 1;
                        junit_cases.push(JUnitCase { name: test.name.clone(), failure: Some(msg) });
                    }
                }
                Err(e) => {
                    eprintln!("  ERROR {}: {} — {e}", file.display(), test.name);
                    fail += 1;
                    junit_cases.push(JUnitCase { name: test.name.clone(), failure: Some(e.to_string()) });
                }
            }
        }
    }

    println!("\nResults: {pass} passed, {fail} failed.");

    if let Some(out) = output_junit {
        write_junit(out, &junit_cases, pass, fail)?;
    }

    Ok((pass, fail))
}

fn check_expectations(
    test: &TestCase,
    resp: &axiom_core::schema::EvaluationResponse,
) -> bool {
    if let Some(expected_matched) = test.expect.matched {
        if resp.matched != expected_matched { return false; }
    }
    if let Some(ref expected_tags) = test.expect.tags {
        for tag in expected_tags {
            if !resp.tags.contains(tag) { return false; }
        }
    }
    if let Some(ref expected_actions) = test.expect.actions {
        for (path, expected_val) in expected_actions {
            let actual = axiom_core::resolver::resolve_owned(&resp.output_context, path);
            if actual != *expected_val { return false; }
        }
    }
    true
}

fn resolve_rule_file(test_file: &Path, rule_id: &str) -> anyhow::Result<std::path::PathBuf> {
    let dir = test_file.parent().unwrap_or_else(|| Path::new("."));
    // Try <rule_id>.yaml, <rule_id>.yml next to the test file
    for ext in &["yaml", "yml"] {
        let candidate = dir.join(format!("{rule_id}.{ext}"));
        if candidate.exists() { return Ok(candidate); }
    }
    anyhow::bail!("could not find rule file for '{}' near {}", rule_id, test_file.display())
}

fn walk_test_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if root.is_file() {
        files.push(root.to_path_buf());
        return files;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                files.extend(walk_test_files(&p));
            } else if p.file_name().and_then(|n| n.to_str())
                       .map(|n| n.ends_with(".test.yaml") || n.ends_with(".test.yml"))
                       .unwrap_or(false) {
                files.push(p);
            }
        }
    }
    files
}

// ── Test file schema ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TestSuite {
    rule:  String,
    tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
struct TestCase {
    name:    String,
    context: Value,
    expect:  Expectation,
}

#[derive(Debug, Deserialize)]
struct Expectation {
    #[serde(default)]
    matched: Option<bool>,
    #[serde(default)]
    tags:    Option<Vec<String>>,
    #[serde(default)]
    actions: Option<std::collections::HashMap<String, Value>>,
}

// ── JUnit output ──────────────────────────────────────────────────────────────

struct JUnitCase { name: String, failure: Option<String> }

fn write_junit(path: &Path, cases: &[JUnitCase], pass: usize, fail: usize) -> anyhow::Result<()> {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(&format!(
        r#"<testsuite name="axiom" tests="{}" failures="{}" errors="0">"#,
        pass + fail, fail
    ));
    for c in cases {
        xml.push_str(&format!(r#"  <testcase name="{}">"#, c.name));
        if let Some(ref msg) = c.failure {
            xml.push_str(&format!(r#"    <failure message="{msg}"/>"#));
        }
        xml.push_str("  </testcase>\n");
    }
    xml.push_str("</testsuite>\n");
    std::fs::write(path, xml)?;
    Ok(())
}
