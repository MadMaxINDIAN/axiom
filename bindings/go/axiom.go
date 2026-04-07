// Package axiom provides Go bindings for the Axiom rules engine.
//
// Build requirements:
//
//  1. Build the Rust FFI library:
//     cargo build --release -p axiom-go-sys
//
//  2. Copy (or symlink) the output to bindings/go/lib/:
//     mkdir -p bindings/go/lib
//     cp target/release/libaxiom_go.so bindings/go/lib/   # Linux
//     cp target/release/libaxiom_go.dylib bindings/go/lib/ # macOS
//     cp target/release/axiom_go.dll bindings/go/lib/       # Windows
//
//  3. Build or test:
//     cd bindings/go && go test ./...
//
// Example:
//
//	engine, err := axiom.New()
//	if err != nil { log.Fatal(err) }
//	defer engine.Close()
//
//	if err := engine.LoadRuleYAML(yamlStr); err != nil { log.Fatal(err) }
//
//	result, err := engine.Evaluate(axiom.EvaluationRequest{
//	    Context: map[string]any{"credit_score": 720},
//	})
package axiom

/*
#cgo CFLAGS: -I${SRCDIR}
#cgo linux   LDFLAGS: -L${SRCDIR}/lib -laxiom_go -ldl -lpthread -lm
#cgo darwin  LDFLAGS: -L${SRCDIR}/lib -laxiom_go -ldl -lpthread
#cgo windows LDFLAGS: -L${SRCDIR}/lib -laxiom_go -lws2_32 -lbcrypt -lntdll -luserenv
#include "axiom.h"
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"errors"
	"fmt"
	"runtime"
	"unsafe"
)

// ---------------------------------------------------------------------------
// ARS types
// ---------------------------------------------------------------------------

// EvaluationRequest mirrors the Rust EvaluationRequest schema.
type EvaluationRequest struct {
	Context   map[string]any `json:"context"`
	RuleID    string         `json:"rule_id,omitempty"`
	Ruleset   string         `json:"ruleset,omitempty"`
	Strategy  string         `json:"strategy,omitempty"` // "first_match" | "all_match" | "scored"
	DryRun    bool           `json:"dry_run,omitempty"`
	TimeoutMs *int           `json:"timeout_ms,omitempty"`
}

// ConditionTrace holds per-condition trace data.
type ConditionTrace struct {
	Field       string `json:"field,omitempty"`
	Operator    string `json:"operator,omitempty"`
	Value       any    `json:"value,omitempty"`
	ActualValue any    `json:"actual_value,omitempty"`
	Passed      bool   `json:"passed"`
	DurationUs  int64  `json:"duration_us"`
}

// RuleTrace holds trace data for a single rule evaluation.
type RuleTrace struct {
	RuleID          string           `json:"rule_id"`
	Matched         bool             `json:"matched"`
	Conditions      []ConditionTrace `json:"conditions"`
	ShortCircuited  bool             `json:"short_circuited"`
	ActionsExecuted []string         `json:"actions_executed"`
	DurationUs      int64            `json:"duration_us"`
}

// EvaluationTrace is the top-level trace returned by the engine.
type EvaluationTrace struct {
	RulesEvaluated int64       `json:"rules_evaluated"`
	RulesMatched   int64       `json:"rules_matched"`
	Strategy       string      `json:"strategy"`
	TotalDurationUs int64      `json:"total_duration_us"`
	TimedOut       bool        `json:"timed_out"`
	Rules          []RuleTrace `json:"rules"`
}

// EvaluationResponse is the result of a single evaluation.
type EvaluationResponse struct {
	Matched       bool           `json:"matched"`
	MatchedRules  []string       `json:"matched_rules"`
	Tags          []string       `json:"tags"`
	OutputContext map[string]any `json:"output_context"`
	DurationUs    int64          `json:"duration_us"`
	Trace         EvaluationTrace `json:"trace"`
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

// Engine is a thread-safe Axiom rules engine instance.
// Create with [New]; close with [Engine.Close].
type Engine struct {
	handle *C.AxiomHandle
}

// New creates a new, empty Engine.
func New() (*Engine, error) {
	h := C.axiom_create()
	if h == nil {
		return nil, errors.New("axiom: failed to create engine")
	}
	e := &Engine{handle: h}
	runtime.SetFinalizer(e, (*Engine).Close)
	return e, nil
}

// Close releases the engine and all associated memory.
// It is safe to call Close multiple times.
func (e *Engine) Close() {
	if e.handle != nil {
		C.axiom_destroy(e.handle)
		e.handle = nil
	}
}

// ---------------------------------------------------------------------------
// Rule loading
// ---------------------------------------------------------------------------

// LoadRuleYAML loads a rule from an ARS YAML string.
func (e *Engine) LoadRuleYAML(yaml string) error {
	cs := C.CString(yaml)
	defer C.free(unsafe.Pointer(cs))
	return freeAndCheck(C.axiom_load_rule_yaml(e.handle, cs))
}

// LoadRuleJSON loads a rule from an ARS JSON string.
func (e *Engine) LoadRuleJSON(jsonStr string) error {
	cs := C.CString(jsonStr)
	defer C.free(unsafe.Pointer(cs))
	return freeAndCheck(C.axiom_load_rule_json(e.handle, cs))
}

// LoadRuleFile loads a rule from a YAML or JSON file (extension auto-detected).
func (e *Engine) LoadRuleFile(path string) error {
	cs := C.CString(path)
	defer C.free(unsafe.Pointer(cs))
	return freeAndCheck(C.axiom_load_rule_file(e.handle, cs))
}

// LoadBundle loads a bundle file containing rules and optional rulesets.
func (e *Engine) LoadBundle(path string) error {
	cs := C.CString(path)
	defer C.free(unsafe.Pointer(cs))
	return freeAndCheck(C.axiom_load_bundle(e.handle, cs))
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

// Evaluate evaluates req against all loaded rules.
func (e *Engine) Evaluate(req EvaluationRequest) (*EvaluationResponse, error) {
	reqJSON, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("axiom: marshal request: %w", err)
	}

	cs := C.CString(string(reqJSON))
	defer C.free(unsafe.Pointer(cs))

	raw := C.axiom_evaluate(e.handle, cs)
	if raw == nil {
		return nil, errors.New("axiom: null response from engine")
	}
	defer C.axiom_free_string(raw)

	respStr := C.GoString(raw)

	// Check for error envelope
	var errEnv struct{ Error string `json:"error"` }
	if json.Unmarshal([]byte(respStr), &errEnv) == nil && errEnv.Error != "" {
		return nil, fmt.Errorf("axiom: %s", errEnv.Error)
	}

	var resp EvaluationResponse
	if err := json.Unmarshal([]byte(respStr), &resp); err != nil {
		return nil, fmt.Errorf("axiom: unmarshal response: %w", err)
	}
	return &resp, nil
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

// ValidateRule validates ARS YAML or JSON without loading it.
// Returns nil if valid, or an error describing the problem.
func ValidateRule(source string) error {
	cs := C.CString(source)
	defer C.free(unsafe.Pointer(cs))
	raw := C.axiom_validate_rule(cs)
	if raw == nil {
		return nil
	}
	msg := C.GoString(raw)
	C.axiom_free_string(raw)
	return errors.New(msg)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// freeAndCheck converts a nullable C string return value to a Go error.
// NULL means success; non-NULL is an error message that is freed here.
func freeAndCheck(raw *C.char) error {
	if raw == nil {
		return nil
	}
	msg := C.GoString(raw)
	C.axiom_free_string(raw)

	// Strip JSON error envelope if present
	var env struct{ Error string `json:"error"` }
	if json.Unmarshal([]byte(msg), &env) == nil && env.Error != "" {
		return fmt.Errorf("axiom: %s", env.Error)
	}
	if msg == "null" {
		return nil
	}
	return fmt.Errorf("axiom: %s", msg)
}
