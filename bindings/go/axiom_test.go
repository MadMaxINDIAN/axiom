package axiom_test

import (
	"strings"
	"testing"

	axiom "github.com/axiom-rules/axiom/bindings/go"
)

// ---------------------------------------------------------------------------
// Minimal valid ARS YAML used across tests
// ---------------------------------------------------------------------------

const creditRule = `
ars_version: "1.0"
id: credit_check
name: Credit Check
conditions:
  all:
    - field: credit_score
      op: gte
      value: 700
actions:
  - type: set_value
    key: approved
    value: true
`

const invalidRule = `
ars_version: "1.0"
# missing required 'id' field
name: Bad Rule
conditions:
  all: []
actions: []
`

// ---------------------------------------------------------------------------
// Engine lifecycle
// ---------------------------------------------------------------------------

func TestNew(t *testing.T) {
	e, err := axiom.New()
	if err != nil {
		t.Fatalf("New() error: %v", err)
	}
	e.Close()
	// second Close must be a no-op (not panic)
	e.Close()
}

// ---------------------------------------------------------------------------
// Rule loading
// ---------------------------------------------------------------------------

func TestLoadRuleYAML_valid(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	if err := e.LoadRuleYAML(creditRule); err != nil {
		t.Fatalf("LoadRuleYAML() unexpected error: %v", err)
	}
}

func TestLoadRuleYAML_invalid(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	err := e.LoadRuleYAML(invalidRule)
	if err == nil {
		t.Fatal("LoadRuleYAML() expected error for invalid rule, got nil")
	}
}

func TestLoadRuleJSON_valid(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	jsonRule := `{
		"ars_version": "1.0",
		"id": "json_rule",
		"name": "JSON Rule",
		"conditions": {"all": [{"field": "score", "op": "gt", "value": 0}]},
		"actions": [{"type": "approve"}]
	}`

	if err := e.LoadRuleJSON(jsonRule); err != nil {
		t.Fatalf("LoadRuleJSON() unexpected error: %v", err)
	}
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

func TestEvaluate_matched(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	if err := e.LoadRuleYAML(creditRule); err != nil {
		t.Fatalf("setup: %v", err)
	}

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context: map[string]any{"credit_score": 750},
	})
	if err != nil {
		t.Fatalf("Evaluate() error: %v", err)
	}
	if !resp.Matched {
		t.Error("expected Matched=true for credit_score=750")
	}
	if len(resp.MatchedRules) == 0 {
		t.Error("expected at least one matched rule")
	}
}

func TestEvaluate_notMatched(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	if err := e.LoadRuleYAML(creditRule); err != nil {
		t.Fatalf("setup: %v", err)
	}

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context: map[string]any{"credit_score": 600},
	})
	if err != nil {
		t.Fatalf("Evaluate() error: %v", err)
	}
	if resp.Matched {
		t.Error("expected Matched=false for credit_score=600")
	}
}

func TestEvaluate_specificRule(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	if err := e.LoadRuleYAML(creditRule); err != nil {
		t.Fatalf("setup: %v", err)
	}

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context: map[string]any{"credit_score": 750},
		RuleID:  "credit_check",
	})
	if err != nil {
		t.Fatalf("Evaluate() error: %v", err)
	}
	if !resp.Matched {
		t.Error("expected match when targeting rule directly")
	}
}

func TestEvaluate_emptyEngine(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context: map[string]any{"x": 1},
	})
	if err != nil {
		t.Fatalf("Evaluate() on empty engine error: %v", err)
	}
	if resp.Matched {
		t.Error("empty engine should never match")
	}
}

// ---------------------------------------------------------------------------
// Validation (free function)
// ---------------------------------------------------------------------------

func TestValidateRule_valid(t *testing.T) {
	if err := axiom.ValidateRule(creditRule); err != nil {
		t.Errorf("ValidateRule() unexpected error: %v", err)
	}
}

func TestValidateRule_invalid(t *testing.T) {
	err := axiom.ValidateRule(invalidRule)
	if err == nil {
		t.Error("ValidateRule() expected error for invalid rule, got nil")
	}
}

func TestValidateRule_garbage(t *testing.T) {
	err := axiom.ValidateRule("not yaml at all }{}{")
	if err == nil {
		t.Error("ValidateRule() expected error for garbage input")
	}
	if !strings.Contains(err.Error(), "") {
		// Any non-nil error is acceptable
	}
}

// ---------------------------------------------------------------------------
// Multiple rules / strategy
// ---------------------------------------------------------------------------

const ruleA = `
ars_version: "1.0"
id: rule_a
name: Rule A
conditions:
  all:
    - field: x
      op: gt
      value: 0
actions:
  - type: tag
    tag: positive
`

const ruleB = `
ars_version: "1.0"
id: rule_b
name: Rule B
conditions:
  all:
    - field: x
      op: gt
      value: 100
actions:
  - type: tag
    tag: large
`

func TestEvaluate_allMatch(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	for _, r := range []string{ruleA, ruleB} {
		if err := e.LoadRuleYAML(r); err != nil {
			t.Fatalf("load: %v", err)
		}
	}

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context:  map[string]any{"x": 150},
		Strategy: "all_match",
	})
	if err != nil {
		t.Fatalf("Evaluate() error: %v", err)
	}
	if len(resp.MatchedRules) < 2 {
		t.Errorf("expected both rules to match for x=150, got %v", resp.MatchedRules)
	}
}

func TestEvaluate_firstMatch(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	for _, r := range []string{ruleA, ruleB} {
		if err := e.LoadRuleYAML(r); err != nil {
			t.Fatalf("load: %v", err)
		}
	}

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context:  map[string]any{"x": 150},
		Strategy: "first_match",
	})
	if err != nil {
		t.Fatalf("Evaluate() error: %v", err)
	}
	if len(resp.MatchedRules) != 1 {
		t.Errorf("first_match should return exactly 1 rule, got %v", resp.MatchedRules)
	}
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

func TestEvaluate_tracePopulated(t *testing.T) {
	e, _ := axiom.New()
	defer e.Close()

	if err := e.LoadRuleYAML(creditRule); err != nil {
		t.Fatalf("setup: %v", err)
	}

	resp, err := e.Evaluate(axiom.EvaluationRequest{
		Context: map[string]any{"credit_score": 750},
	})
	if err != nil {
		t.Fatalf("Evaluate() error: %v", err)
	}
	if resp.Trace.RulesEvaluated == 0 {
		t.Error("expected trace.rules_evaluated > 0")
	}
}
