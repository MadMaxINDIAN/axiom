/**
 * @axiom-rules/core — TypeScript declarations
 * Generated from axiom-core Rust crate via NAPI-RS.
 */

// ---------------------------------------------------------------------------
// ARS schema types
// ---------------------------------------------------------------------------

export type Operator =
  | 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte'
  | 'contains' | 'starts_with' | 'ends_with' | 'matches' | 'in' | 'not_in'
  | 'between' | 'outside' | 'divisible_by'
  | 'is_null' | 'is_not_null' | 'is_empty' | 'is_not_empty'
  | 'before' | 'after' | 'within_days' | 'is_weekday' | 'is_weekend'
  | 'contains_any' | 'contains_all' | 'length_eq' | 'length_gt' | 'length_lt'
  | 'is_type'
  | 'field_gt_field' | 'field_eq_field';

export interface LeafCondition {
  field:    string;
  operator: Operator;
  value?:   unknown;
  field2?:  string;
}

export type ConditionNode = ConditionGroup | LeafCondition;

export type ConditionGroup =
  | { all:  ConditionNode[] }
  | { any:  ConditionNode[] }
  | { none: ConditionNode[] }
  | { not:  ConditionNode   };

export type ActionType = 'set' | 'increment' | 'append' | 'tag' | 'trigger'
                       | 'call_rule' | 'return' | 'log';

export interface Action {
  type:     ActionType;
  field?:   string;
  value?:   unknown;
  event?:   string;
  rule_id?: string;
  level?:   'debug' | 'info' | 'warn';
  message?: string;
}

/** A fully typed ARS rule document. */
export interface Rule {
  ars_version: 1;
  id:          string;
  name:        string;
  description?: string;
  version:     number;
  priority?:   number;
  enabled?:    boolean;
  tags?:       string[];
  extends?:    string;
  conditions:  ConditionGroup;
  actions:     Action[];
  metadata?:   Record<string, unknown>;
}

export interface Ruleset {
  name:        string;
  rule_ids:    string[];
  description?: string;
}

// ---------------------------------------------------------------------------
// Evaluation types
// ---------------------------------------------------------------------------

export type Strategy = 'first_match' | 'all_match' | 'scored';

export interface EvaluationRequest {
  rule_id?:    string;
  ruleset?:    string;
  strategy?:   Strategy;
  dry_run?:    boolean;
  timeout_ms?: number;
  context:     Record<string, unknown>;
}

export interface ConditionTrace {
  field:        string;
  operator:     string;
  value:        unknown;
  actual_value: unknown;
  passed:       boolean;
  duration_us:  number;
}

export interface RuleTrace {
  rule_id:          string;
  matched:          boolean;
  conditions:       ConditionTrace[];
  short_circuited:  boolean;
  actions_executed: string[];
  duration_us:      number;
}

export interface EvaluationTrace {
  rules_evaluated:   number;
  rules_matched:     number;
  strategy:          Strategy;
  total_duration_us: number;
  timed_out:         boolean;
  rules:             RuleTrace[];
}

export interface EvaluationResponse {
  matched:        boolean;
  matched_rules:  string[];
  tags:           string[];
  output_context: Record<string, unknown>;
  duration_us:    number;
  trace:          EvaluationTrace;
}

/** Returned by validateRule — null means valid. */
export type ValidationResult = { valid: true } | { valid: false; error: string };

// ---------------------------------------------------------------------------
// AxiomEngine class
// ---------------------------------------------------------------------------

export declare class AxiomEngine {
  constructor();

  /** Load a rule from an ARS YAML string. */
  loadRuleYaml(yaml: string): void;

  /** Load a rule from an ARS JSON string. */
  loadRuleJson(json: string): void;

  /** Load a rule from a file path (.yaml / .yml / .json). */
  loadRuleFile(path: string): void;

  /** Load a bundle YAML file (rules + rulesets). */
  loadBundle(path: string): void;

  /**
   * Evaluate a context against loaded rules.
   * Accepts an EvaluationRequest and returns an EvaluationResponse.
   */
  evaluate(request: EvaluationRequest): EvaluationResponse;

  /**
   * Validate an ARS source string without loading it.
   * Returns null if valid, or an error message string.
   */
  validateRule(source: string, isJson?: boolean): string | null;
}

// ---------------------------------------------------------------------------
// Standalone helpers
// ---------------------------------------------------------------------------

/**
 * Validate an ARS YAML or JSON string.
 * Returns `{ valid: true }` or `{ valid: false, error: string }`.
 */
export declare function validateRule(yamlOrJson: string): ValidationResult;
