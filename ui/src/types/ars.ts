// TypeScript types mirroring the ARS schema and server response shapes.

// ---------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------

export type Operator =
  | 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte'
  | 'contains' | 'starts_with' | 'ends_with' | 'matches' | 'in' | 'not_in'
  | 'between' | 'outside' | 'divisible_by'
  | 'is_null' | 'is_not_null' | 'is_empty' | 'is_not_empty'
  | 'before' | 'after' | 'within_days' | 'is_weekday' | 'is_weekend'
  | 'contains_any' | 'contains_all' | 'length_eq' | 'length_gt' | 'length_lt'
  | 'is_type'
  | 'field_gt_field' | 'field_eq_field'

export interface LeafCondition {
  field: string
  op: Operator
  value?: unknown
  field2?: string
}

export interface AllGroup  { all:  ConditionNode[] }
export interface AnyGroup  { any:  ConditionNode[] }
export interface NoneGroup { none: ConditionNode[] }
export interface NotGroup  { not:  ConditionNode }   // single child only

export type ConditionGroup = AllGroup | AnyGroup | NoneGroup | NotGroup
export type ConditionNode  = LeafCondition | ConditionGroup

export function isLeaf(node: ConditionNode): node is LeafCondition {
  return 'field' in node
}
export function isAllGroup(node: ConditionNode): node is AllGroup {
  return 'all' in node
}
export function isAnyGroup(node: ConditionNode): node is AnyGroup {
  return 'any' in node
}
export function isNoneGroup(node: ConditionNode): node is NoneGroup {
  return 'none' in node
}
export function isNotGroup(node: ConditionNode): node is NotGroup {
  return 'not' in node
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export type ActionType = 'set' | 'increment' | 'append' | 'tag' | 'trigger' | 'call_rule' | 'return' | 'log'
export type LogLevel = 'debug' | 'info' | 'warn'

export interface SetAction      { type: 'set';       field: string; value: unknown }
export interface IncrementAction{ type: 'increment'; field: string; value?: number }
export interface AppendAction   { type: 'append';    field: string; value: unknown }
export interface TagAction      { type: 'tag';       value: string }
export interface TriggerAction  { type: 'trigger';   event: string }
export interface CallRuleAction { type: 'call_rule'; rule_id: string }
export interface ReturnAction   { type: 'return';    value?: unknown }
export interface LogAction      { type: 'log';       level: LogLevel; message: string }

export type Action =
  | SetAction | IncrementAction | AppendAction | TagAction
  | TriggerAction | CallRuleAction | ReturnAction | LogAction

// ---------------------------------------------------------------------------
// Rule
// ---------------------------------------------------------------------------

export interface Rule {
  ars_version: 1
  id: string
  name: string
  description?: string
  version: number
  priority: number
  enabled: boolean
  tags: string[]
  extends?: string
  conditions: ConditionGroup
  actions: Action[]
  metadata?: Record<string, unknown>
}

// ---------------------------------------------------------------------------
// Ruleset
// ---------------------------------------------------------------------------

export interface Ruleset {
  name: string
  description?: string
  rule_ids: string[]
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

export type Strategy = 'first_match' | 'all_match' | 'scored'

export interface EvaluationRequest {
  context: Record<string, unknown>
  rule_id?: string
  ruleset?: string
  strategy?: Strategy
  dry_run?: boolean
  timeout_ms?: number
}

export interface ConditionTrace {
  field?: string
  operator?: string
  value?: unknown
  actual_value?: unknown
  passed: boolean
  duration_us: number
}

export interface RuleTrace {
  rule_id: string
  matched: boolean
  conditions: ConditionTrace[]
  short_circuited: boolean
  actions_executed: string[]
  duration_us: number
}

export interface EvaluationTrace {
  rules_evaluated: number
  rules_matched: number
  strategy: Strategy
  total_duration_us: number
  timed_out: boolean
  rules: RuleTrace[]
}

export interface EvaluationResponse {
  matched: boolean
  matched_rules: string[]
  tags: string[]
  output_context: Record<string, unknown>
  duration_us: number
  trace: EvaluationTrace
}

// ---------------------------------------------------------------------------
// API key
// ---------------------------------------------------------------------------

export interface ApiKey {
  id: string
  role: 'admin' | 'editor' | 'viewer'
  description?: string
  created_at?: string
  created_by?: string
}
