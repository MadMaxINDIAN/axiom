# Axiom — Architecture Design Document

> **Version:** 1.3 &nbsp;·&nbsp; **Date:** April 2026 &nbsp;·&nbsp; **License:** Apache 2.0
> **Type:** Open-source, community-driven, no commercial intent
> **CNCF target:** Long-term donation to the Cloud Native Computing Foundation
> **Revision note:** v1.3 incorporates four implementation-detail observations from the v1.2 review. Changes annotated `[R3-N]`. Full finding log in §17.

---

## Table of Contents

| § | Section |
|---|---------|
| [01](#01-executive-summary) | Executive Summary |
| [02](#02-system-context--consumption-modes) | System Context & Consumption Modes |
| [03](#03-high-level-architecture) | High-Level Architecture |
| [04](#04-axiom-rule-schema-ars) | Axiom Rule Schema (ARS) |
| [05](#05-core-evaluation-engine) | Core Evaluation Engine |
| [06](#06-rest-server) | REST Server |
| [07](#07-storage-layer) | Storage Layer |
| [08](#08-language-bindings) | Language Bindings |
| [09](#09-visual-rule-builder) | Visual Rule Builder |
| [10](#10-cli) | CLI |
| [11](#11-observability--auditability) | Observability & Auditability |
| [12](#12-deployment-architecture) | Deployment Architecture |
| [13](#13-build-phases--requirement-mapping) | Build Phases & Requirement Mapping |
| [14](#14-cross-cutting-concerns) | Cross-Cutting Concerns |
| [15](#15-open-design-decisions) | Open Design Decisions |
| [16](#16-non-functional-requirements) | Non-Functional Requirements |
| [17](#17-architecture-review-findings-log) | Architecture Review Findings Log |

---

## 01 Executive Summary

Axiom is a modern, language-agnostic, open-source rules engine that externalises business conditional logic from application code into a versioned, human-readable data format. It targets the large gap between hardcoded if-else logic and heavyweight commercial solutions (IBM ODM, FICO Blaze) or DSL-heavy tools (Drools, OPA).

This document defines the complete system architecture: component boundaries, data models, evaluation algorithms, API contracts, storage strategies, deployment topology, and cross-cutting concerns. It is the primary technical reference for contributors and integrators.

### Design Targets

| Goal | Metric / Target |
|------|----------------|
| Rule evaluation throughput | 10,000 simple rules < 100 ms; 5,000 req/s on a 2-core server |
| Single-rule latency | 50-condition rule < 5 ms p99 |
| Library footprint | < 5 MB added to binary (Java / Node) |
| Zero runtime dependencies | Core engine ships as self-contained binary / shared lib |
| Safety | No arbitrary code execution; sandboxed expression evaluator |
| Portability | Identical rule format across server, Java, Node, Python, Go libraries |
| Auditability | Every evaluation returns a full structured trace |

---

## 02 System Context & Consumption Modes

Axiom integrates with host systems in three distinct consumption modes. All three share exactly the same Axiom Rule Schema (ARS) and the same evaluation semantics — rules authored for one mode run unchanged in all others.

### Mode 1 — Standalone REST Server

A self-hosted HTTP service written in Rust (Axum). Accepts rule definitions and evaluation requests over JSON/HTTP. Suitable for polyglot environments, service meshes, and teams that prefer network-level isolation of business logic.

### Mode 2 — Embedded Library

Native libraries for Java (JNI), Node.js (NAPI-RS), Python (PyO3), and Go (cgo). The evaluation engine executes in-process — zero network hop, zero external dependency. The Rust core is compiled once and wrapped per language.

### Mode 3 — Visual Rule Builder

A React/Vite/Tailwind web application that reads and writes ARS. Designed for product managers, compliance officers, and operations teams who need to define and manage rules without writing YAML or code.

### Mode Comparison

| Dimension | Mode 1 (Server) | Mode 2 (Library) | Mode 3 (UI) |
|-----------|----------------|-----------------|-------------|
| Language requirement | Any (HTTP client) | Java / Node / Python / Go | Browser |
| Latency profile | Network + eval | In-process eval only | Interactive |
| Deployment complexity | Docker / Helm | Library dependency | Static SPA |
| Rule storage | Server-side (SQLite/PG) | Caller-provided | Connected server |
| Audience | Platform / SRE teams | App developers | Business stakeholders |

---

## 03 High-Level Architecture

The system is structured as a layered monorepo. The Rust core crate is the single source of truth for all evaluation logic. All other components — server, language bindings, CLI, UI — are thin integration layers around it.

> **Design principle:** `axiom-core` has zero dependencies on any network, filesystem, or I/O library. All I/O is the responsibility of the layer above it. This ensures the core can be compiled for any target platform and embedded in any runtime.

### 3.1 Component Map

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                            axiom  (monorepo)                                 │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐   │
│  │                      axiom-core  (Rust crate)                         │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌──────────────────┐  ┌──────────────────────────┐  │   │
│  │  │ ARS Parser  │  │    Evaluator     │  │   Expression Engine      │  │   │
│  │  │ (serde_yaml │  │ (condition tree  │  │   (safe template         │  │   │
│  │  │ + serde_json│  │  walker + action │  │    sandbox, no I/O)      │  │   │
│  │  │ + schema    │  │  execution)      │  │                          │  │   │
│  │  │ validation) │  │                  │  │                          │  │   │
│  │  └─────────────┘  └──────────────────┘  └──────────────────────────┘  │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌──────────────────┐  ┌──────────────────────────┐  │   │
│  │  │Rule Registry│  │  Trace Builder   │  │  Conflict Detector [P2]  │  │   │
│  │  │(in-memory   │  │ (structured eval │  │  (heuristic analysis)    │  │   │
│  │  │ index by    │  │  audit log)      │  │                          │  │   │
│  │  │ id/tag/pri) │  │                  │  │                          │  │   │
│  │  └─────────────┘  └──────────────────┘  └──────────────────────────┘  │   │
│  │                                                                       │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐  │   │
│  │  │  call_rule_guard  (depth limit 4 Phase 1, cycle + missing-rule  │  │   │
│  │  │                    detection at load time)                       │  │   │
│  │  └─────────────────────────────────────────────────────────────────┘  │   │
│  └───────────────────────────────────────────────────────────────────────┘   │
│         │                  │                    │                 │           │
│  ┌──────┴──────┐  ┌────────┴───────┐  ┌────────┴──────┐  ┌──────┴────────┐  │
│  │axiom-server │  │  axiom-java    │  │  axiom-node   │  │  axiom-cli    │  │
│  │ (Axum HTTP  │  │  (JNI wrapper) │  │ (NAPI-RS +    │  │  (clap,       │  │
│  │  REST API)  │  │                │  │  TypeScript)  │  │   validate /  │  │
│  └──────┬──────┘  └────────────────┘  └───────────────┘  │   test /      │  │
│         │                                                  │   evaluate /  │  │
│  ┌──────┴──────┐  ┌────────────────┐  ┌───────────────┐  │   keygen)     │  │
│  │  Storage    │  │ axiom-python   │  │   axiom-ui    │  └───────────────┘  │
│  │  Layer      │  │ (PyO3/maturin) │  │ (React+Vite+  │                      │
│  │ (SQLite or  │  │                │  │  Tailwind SPA)│                      │
│  │  PostgreSQL)│  └────────────────┘  └───────────────┘                      │
│  └─────────────┘                                                             │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Repository Layout

| Path | Description |
|------|-------------|
| `axiom/` | Monorepo root; workspace `Cargo.toml` |
| `crates/axiom-core/` | Rust evaluation engine crate (no_std-compatible core logic) |
| `crates/axiom-server/` | Axum HTTP server; depends on axiom-core |
| `crates/axiom-cli/` | CLI binary (clap); depends on axiom-core |
| `bindings/java/` | JNI glue code + thin Java wrapper (Maven/Gradle) |
| `bindings/node/` | NAPI-RS bindings + TypeScript declaration files (npm) |
| `bindings/python/` | PyO3 / maturin bindings (PyPI native wheels) |
| `bindings/go/` | cgo bindings (pkg.go.dev) — Phase 4 |
| `ui/` | React + Vite + Tailwind visual rule builder SPA |
| `modules/axiom-finance/` | ARS bundle: loan eligibility, credit scoring, fraud detection |
| `modules/axiom-ecommerce/` | ARS bundle: pricing, discounts, promotions, shipping |
| `modules/axiom-access/` | ARS bundle: feature flags, entitlements, plan-based access |
| `modules/axiom-compliance/` | ARS bundle: KYC/AML, data residency, regulatory rules |
| `modules/axiom-ops/` | ARS bundle: alerting thresholds, incident routing, escalation |
| `docs/` | Docusaurus documentation site |
| `deploy/` | Docker images, Helm chart, docker-compose examples |
| `schema/` | ARS JSON Schema files (machine-readable open standard spec) |

---

## 04 Axiom Rule Schema (ARS)

ARS is the open, language-neutral wire format for all rule definitions. It is the contract between rule authors, the evaluation engine, and all language bindings. ARS is versioned independently of the engine; a single major ARS version must be supported by all libraries and the server simultaneously.

### 4.1 Rule Document Structure

```yaml
ars_version: 1                              # [AR-6] required — ARS schema this rule targets
id: loan-eligibility-check
name: Loan Eligibility Check
description: Determines if an applicant qualifies for a standard loan
version: 2
priority: 10
enabled: true
tags: [lending, standard]

conditions:
  all:
    - field: applicant.credit_score
      operator: gte
      value: 650
    - field: applicant.annual_income
      operator: gte
      value: 30000
    - field: applicant.existing_debt_ratio
      operator: lte
      value: 0.4

actions:
  - type: set
    field: result.eligible
    value: true
  - type: set
    field: result.max_loan_amount
    value: "{{ applicant.annual_income * 3 }}"
  - type: tag
    value: standard-loan-approved
```

### 4.2 Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ars_version` | `integer` | **Yes** | ARS schema version this rule conforms to. Must be `1` for the current schema. Required from day one to enable future schema migrations without ambiguity. **[AR-6]** |
| `id` | `string` | Yes | Globally unique slug (e.g. `loan-eligibility-check`) |
| `name` | `string` | Yes | Human-readable display name |
| `description` | `string` | No | Free-text explanation for rule authors |
| `version` | `integer` | Yes | Monotonically increasing; old versions retained in store |
| `priority` | `integer` | Yes | Evaluation order within ruleset; higher = earlier. Default: 0 |
| `enabled` | `boolean` | Yes | When false, rule is skipped in all evaluations |
| `tags` | `string[]` | No | Arbitrary labels for filtering and grouping |
| `extends` | `string` | No | [P2] ID of a base rule to inherit conditions from |
| `conditions` | `ConditionGroup` | Yes | Root condition group (`all` / `any` / `none` / `not`) |
| `actions` | `Action[]` | Yes | Ordered list of actions executed when conditions pass |
| `metadata` | `object` | No | Arbitrary key-value pairs for tooling (author, ticket, etc.) |

### 4.3 Condition Group Model

Every condition group has exactly one logical operator key whose value is an array of condition nodes. Each node is either a **leaf condition** (a field comparison) or a **nested group** (enabling arbitrary depth).

| Node type | Schema | Semantics |
|-----------|--------|-----------|
| Leaf condition | `{ field, operator, value? }` | Compare `context[field]` using operator |
| Cross-field leaf | `{ field, operator, field2 }` | Compare `context[field]` vs `context[field2]` |
| `all` group | `{ all: ConditionNode[] }` | AND — all children must be true |
| `any` group | `{ any: ConditionNode[] }` | OR — at least one child must be true |
| `none` group | `{ none: ConditionNode[] }` | NOR — no child may be true |
| `not` group | `{ not: ConditionNode }` | Accepts **a single child node only** (a leaf or a nested group). Negates that child's result. For multi-condition NOR use `none`. **[AR-1]** |

> **[AR-1] Migration:** Previous versions allowed `not: ConditionNode[]` (NAND). Migrate multi-child `not` arrays to `none` (if all must be false) or wrap children in an `all` group nested inside `not` (if the conjunction should be negated). The parser rejects array-valued `not` with a schema validation error.

### 4.4 Field Path Resolution

Fields use dot-notation paths into the context JSON object. Array indexing is supported. Missing fields resolve to `null`.

| Example path | Resolves to |
|-------------|-------------|
| `applicant.credit_score` | `{ applicant: { credit_score: 720 } }` → `720` |
| `order.items[0].price` | First item's price in the items array |
| `user.roles` | Entire roles array (used with list operators) |
| `metadata.flags.is_vip` | Deeply nested boolean |

### 4.5 Operator Taxonomy

| Category | Operators | Notes |
|----------|-----------|-------|
| Comparison | `eq` `neq` `gt` `gte` `lt` `lte` | Works for numbers, strings (lexicographic), and ISO 8601 dates |
| String | `contains` `starts_with` `ends_with` `matches` `in` `not_in` | `matches` uses PCRE2 regex with 1 ms per-pattern timeout |
| Numeric | `between` `outside` `divisible_by` | `between` is inclusive on both bounds |
| Null / Empty | `is_null` `is_not_null` `is_empty` `is_not_empty` | `is_empty` matches null, empty string, empty array, empty object |
| Date / Time | `before` `after` `within_days` `is_weekday` `is_weekend` | All dates parsed as ISO 8601. `within_days`: \|now − field\| < N days |
| List | `contains_any` `contains_all` `length_eq` `length_gt` `length_lt` | Operates on array-typed fields |
| Type check | `is_type` | Value one of: `string` `number` `boolean` `array` `object` `null` |
| Cross-field | `field_gt_field` `field_eq_field` | Both fields resolved from context before comparison |

### 4.6 Action Types

| Action | Required fields | Description |
|--------|----------------|-------------|
| `set` | `field, value` | Write value to output context at field path. Value may be a `{{ }}` template expression. |
| `increment` | `field, value?` | Add value (default 1) to a numeric field in output context. |
| `append` | `field, value` | Push value to an array field. Creates array if absent. |
| `tag` | `value` | Add a string tag to `result.tags[]`. |
| `trigger` | `event` | **Server mode:** enqueues an outbound webhook to the URL registered for `event` (HMAC-SHA256 signed). Default retry: **3 attempts, exponential backoff — 1 s, 4 s, 16 s**. Dead-letter written to `AXIOM_DEAD_LETTER_PATH` (default: `/data/dead-letter/`). All retry and dead-letter settings are overridable per-event in config. Dead-letter path is intentionally on disk rather than in the database, so the trigger path remains functional during storage outages. **Library mode:** invokes `engine.onTrigger(event, fn)`; no-op if no callback is registered. **[AR-9, R2-3, R3-2]** |
| `call_rule` | `rule_id` | Synchronously evaluate another named rule. Max call depth: **4** (Phase 1), raised to **8** in Phase 2 after benchmark validation. Cycles and missing-rule references are rejected at ruleset load time (see §5.1). **[AR-2, R2-1, R3-4]** |
| `return` | `value?` | Halt further ruleset evaluation and return optional value. |
| `log` | `level, message` | Emit a structured entry in the evaluation trace (level: `debug`/`info`/`warn`). |

> **Expression Sandbox:** Template expressions `{{ field * multiplier }}` are evaluated in a sandboxed arithmetic/string evaluator. No function calls, no identifiers, no loops. Supported operators: `+ - * / % == != < > && || !` and string concatenation. Recursion depth is capped at 16 AST nodes. All arithmetic uses checked operations — overflow returns `EvaluationError`, not a panic.

---

## 05 Core Evaluation Engine

The evaluation engine is implemented entirely in Rust as a standalone crate with no_std-compatible core logic. It has no async runtime dependency and exposes both synchronous and async evaluation surfaces (async via tokio in the server, sync in library bindings).

### 5.1 Internal Module Structure

| Module | Responsibility |
|--------|---------------|
| `axiom_core::schema` | Rust structs for ARS (`Rule`, `ConditionGroup`, `ConditionNode`, `Action`). Derives serde + JSON Schema. |
| `axiom_core::parser` | Parses YAML or JSON bytes into schema structs. Validates against ARS JSON Schema. Returns rich `ParseError` with field path and message. |
| `axiom_core::registry` | In-memory store keyed by `(id, version)`. Supports enable/disable, priority ordering, tag indexing, and ruleset grouping. |
| `axiom_core::resolver` | Resolves dot-notation field paths against a `serde_json::Value` context. Returns `ResolvedValue` enum (Number, Str, Bool, Array, Object, Null). |
| `axiom_core::evaluator` | Core evaluation logic: walks condition tree, calls resolver, dispatches to operator handlers. Returns `ConditionResult { matched, trace }`. |
| `axiom_core::operators` | One pure function per operator. Regex operators maintain a thread-local compiled regex cache with TTL eviction. |
| `axiom_core::actions` | Executes action list against a mutable output context. Calls expression engine for template values. Returns `ActionTrace`. |
| `axiom_core::expression` | Sandboxed expression evaluator. Tokenises → parses AST → evaluates. No external calls; recursion depth limit enforced at parse time. |
| `axiom_core::trace` | Assembles `EvaluationTrace` from condition and action results. Computes timing (start/end per rule and per condition). |
| `axiom_core::strategy` | Implements three strategies. **Scored algorithm [AR-3]:** score = leaf conditions passed / total leaf conditions (groups treated as virtual leaves — see §5.4). Ties broken by `priority DESC` then `id ASC`. Rules with score 0 are excluded from Scored results. |
| `axiom_core::call_rule_guard` | **[AR-2, R3-4]** Enforces maximum `call_rule` chain depth (default **4** in Phase 1, **8** in Phase 2 — runtime-configurable, not a schema value). At ruleset load time, performs three checks: (1) topological sort over the `call_rule` dependency graph — any cycle produces `CyclicRuleDependencyError`; (2) all rule IDs referenced in `call_rule` actions must exist in the registry — any missing reference produces `UnresolvedRuleReferenceError`; (3) depth limit validation. All three failures are load-time errors, consistent with the fail-fast philosophy throughout the engine. At evaluation time, only depth is checked (the other two are already guaranteed by the load-time checks). |
| `axiom_core::conflict` | [P2] Analyses a ruleset for rules whose conditions overlap and whose actions set the same field to contradictory values. |
| `axiom_core::timeout` | Wraps evaluation in a wall-clock budget. Tracks elapsed time and returns partial trace if budget exceeded (panic-safe). |

### 5.2 Evaluation Algorithm

1. Load ruleset from registry: filter `enabled = true`, sort by `priority DESC`.
2. For each rule (in priority order):
   - Evaluate condition tree recursively (see §5.3 for short-circuit behaviour).
   - If matched: execute action list, record `ActionTrace`.
   - Apply evaluation strategy: `FirstMatch` returns immediately; `AllMatch` continues; `Scored` accumulates.
3. Check timeout budget after each rule; return partial trace if exhausted.
4. Assemble final `EvaluationResult`: `{ matched_rules, tags, output_context, trace, strategy, duration_us }`.

> **Performance:** For `AllMatch` evaluations over large rulesets (> 1,000 rules), the registry pre-indexes rules by tag. Tag-filtered evaluations skip the full sort and binary-search the index directly. This keeps throughput above 10K rules/100ms even for complex rulesets.

### 5.3 Condition Tree Short-Circuit Behaviour

| Group type | Short-circuit behaviour | Trace completeness |
|------------|------------------------|--------------------|
| `all` | Stops at first false child | Partial trace up to failing condition |
| `any` | Stops at first true child | Partial trace up to matching condition |
| `none` | Stops at first true child (returns false) | Partial trace |
| `not` | Evaluates its single child, then inverts | Full trace (single child always fully evaluated) **[AR-1]** |
| dry-run mode (all types) | Never short-circuits | Full trace always |

In **dry-run mode** (`EV-11`), short-circuit evaluation is disabled so that the trace reveals every condition's result regardless of overall match outcome. This is the default mode in the VS Code extension and the UI live test panel.

### 5.4 Scored Strategy — Group Node Scoring `[R2-2]`

The Scored algorithm flattens nested groups to their constituent leaf conditions, then counts how many passed. Group nodes require a deliberate scoring rule because a group's resolved result may differ from its children's raw values (most notably for `not`, but the rule must generalise).

**Rule: each group node is treated as a single virtual leaf whose value is the group's final resolved result.**

Examples:

- `not: { field: x, op: eq, value: 5 }` evaluates to `true` (because `x != 5`) → counts as **1 virtual leaf passed**. The inner leaf's raw `false` is not the unit of scoring.
- `any: [A, B, C]` where only A is true → the `any` group resolves to `true` → counts as **1 virtual leaf passed**, regardless of B and C.
- `all: [A, B]` where A is false → the `all` group resolves to `false` → counts as **0 virtual leaves passed**.

This rule generalises cleanly: for any nesting depth, the score of a rule is the number of top-level condition nodes (leaf or group) that resolved to `true`, divided by the total number of top-level condition nodes. Deep nesting never inflates the denominator.

### 5.5 Expression Engine Security Model

- **No function calls** — the grammar has no function-call production rule.
- **No identifiers** — all field references are resolved before expression evaluation; the expression only sees concrete values.
- **No loops or recursion** — the AST is strictly a tree of binary/unary operators over literals.
- **Recursion depth limit** — AST depth > 16 nodes returns a `ParseError` at parse time.
- **Checked arithmetic** — all numeric operations use checked arithmetic; overflow returns `EvaluationError`, not panic.
- **Regex timeout** — PCRE2 match operations are bounded by a 1 ms per-pattern timeout enforced by the regex engine.

> **Security:** Rule definitions are _data_, not code. The evaluation engine treats every field in an ARS document as untrusted input and validates it against the ARS JSON Schema before any evaluation is attempted. No `eval()`, no dynamic dispatch, no reflection.

---

## 06 REST Server

The server is an Axum (Rust) HTTP application. It wraps `axiom-core` with persistence, authentication, horizontal-scaling support, and observability instrumentation.

### 6.1 API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Liveness probe — 200 if process alive. |
| `GET` | `/ready` | Readiness probe — 200 when storage reachable. |
| `GET` | `/metrics` | Prometheus text format: eval throughput, latency p50/p95/p99, match rate. |
| `GET` | `/v1/rules` | List all rules. Supports `?tag=`, `?enabled=`, `?ruleset=` query filters. |
| `POST` | `/v1/rules` | Create a new rule (ARS JSON/YAML body). Returns created rule with assigned version. |
| `GET` | `/v1/rules/{id}` | Get the active version of a rule by ID. |
| `GET` | `/v1/rules/{id}/versions` | List all retained versions of a rule. |
| `PUT` | `/v1/rules/{id}` | Update a rule. Increments version; previous version retained. |
| `PATCH` | `/v1/rules/{id}` | Partial update. Supports `{ enabled: false }` to disable without full PUT. |
| `DELETE` | `/v1/rules/{id}` | Soft-delete (disables all versions; data retained for audit). |
| `GET` | `/v1/rulesets` | List all named rulesets. |
| `POST` | `/v1/rulesets` | Create a ruleset (name + list of rule IDs). |
| `GET` | `/v1/rulesets/{name}` | Get a ruleset definition. |
| `PUT` | `/v1/rulesets/{name}` | Update ruleset (full replace of rule ID list). |
| `POST` | `/v1/evaluate` | Evaluate context against a named rule or ruleset. |
| `POST` | `/v1/evaluate/batch` | Submit array of contexts. **Max 1,000 contexts** (HTTP 400 if exceeded). Concurrency bounded by worker pool (default: 2× CPU cores). Each batch counts as N evaluations against the per-key rate limit. **[AR-10]** |
| `GET` | `/v1/keys` | List API keys (admin role only). Returns `id`, `role`, `description`, `created_at` — never the raw key value. |
| `POST` | `/v1/keys` | Create an API key. Body: `{ role: "editor", description: "CI deploy key" }`. Returns key value **once** — not stored in recoverable form. Admin role only. **[R2-new]** |
| `DELETE` | `/v1/keys/{id}` | Revoke an API key (sets `revoked_at`). Admin role only. |
| `POST` | `/v1/import` | Import a full YAML/JSON bundle (rules + rulesets). |
| `GET` | `/v1/export` | Export all rules and rulesets as a single YAML bundle. |

### 6.2 API Key Management `[R2-new]`

API keys are the sole authentication mechanism in Phase 1 and Phase 3. Each key has a `role` field (`admin` / `editor` / `viewer`) enforced server-side on every request.

**Phase 1 — Config file only.** Keys are defined in `axiom.yaml`. The server reads them at startup.

```yaml
# axiom.yaml
keys:
  - id: ci-deploy
    role: editor
    hash: "sha256:abcd1234..."   # SHA-256 of the actual key value — see §6.2.1 for generation
    description: "CI/CD deploy pipeline"
  - id: admin-key
    role: admin
    hash: "sha256:efgh5678..."
    description: "Local admin access"
```

**Phase 2 — REST API for key management.** The `/v1/keys` endpoints go live. Keys created via API are persisted to the `api_keys` table. Config-file keys continue to work alongside API-managed keys.

**Invariants (both phases):**
- Key values are SHA-256 hashed before storage. The plaintext value is shown exactly once on creation and never again.
- An admin-role key can create, list, and revoke any key except itself.
- Revoking the last admin key is rejected with HTTP 409.
- Config-file keys cannot be revoked via the API (they are re-read at startup). To retire a config-file key, remove it from the config and restart.

#### 6.2.1 Generating Key Hashes `[R3-3]`

Operators generating config-file keys must hash the key value before storing it in `axiom.yaml`. Use `axiom keygen` (see §10.1) to generate a key and its hash together in one step — this is the recommended approach. For environments where the CLI is not available, the hash can be produced manually:

```bash
# Generate a random key and hash it
KEY=$(openssl rand -hex 32)
HASH=$(echo -n "$KEY" | sha256sum | awk '{print $1}')

echo "key value (set as X-Axiom-Key header): $KEY"
echo "hash (put in axiom.yaml):  sha256:$HASH"
```

The `sha256:` prefix is required in `axiom.yaml` to make the hash algorithm explicit and allow future algorithm migration.

### 6.3 Evaluate Request / Response

```json
// POST /v1/evaluate — request
{
  "rule_id":    "loan-eligibility-check",
  "ruleset":    "lending-rules",
  "strategy":   "all_match",
  "dry_run":    false,
  "timeout_ms": 50,
  "context": {
    "applicant": {
      "credit_score": 720,
      "annual_income": 60000,
      "existing_debt_ratio": 0.2
    }
  }
}

// Response
{
  "matched": true,
  "matched_rules": ["loan-eligibility-check"],
  "tags": ["standard-loan-approved"],
  "output_context": {
    "result": { "eligible": true, "max_loan_amount": 180000 }
  },
  "duration_us": 312,
  "trace": {
    "rules_evaluated": 1,
    "rules": [{
      "rule_id": "loan-eligibility-check",
      "matched": true,
      "conditions": [
        { "field": "applicant.credit_score",       "op": "gte", "value": 650,   "actual": 720,   "passed": true },
        { "field": "applicant.annual_income",       "op": "gte", "value": 30000, "actual": 60000, "passed": true },
        { "field": "applicant.existing_debt_ratio", "op": "lte", "value": 0.4,   "actual": 0.2,   "passed": true }
      ],
      "actions_executed": ["set result.eligible", "set result.max_loan_amount", "tag standard-loan-approved"],
      "duration_us": 298
    }]
  }
}
```

### 6.4 Authentication & Security

- **API key** (`X-Axiom-Key` header): SHA-256 hashed keys stored in config or database. Each API key carries a `role` field (`admin` / `editor` / `viewer`) enforced server-side on every request. **[AR-7]**
- **Mutual TLS** [P2]: Client certificates validated against a configured CA bundle. For zero-trust inter-service communication.
- **Rate limiting**: Token bucket per API key. Default: 1,000 req/s. Returns HTTP 429 with `Retry-After` header when exceeded. Batch requests count as N evaluations against the limit.

### 6.5 Horizontal Scaling

The server is stateless with respect to rule evaluation. Rules are loaded from shared PostgreSQL at startup and cached in-memory.

**Polling is the reliability mechanism.** The server polls for rule changes every 10 seconds (configurable) regardless of anything else. `LISTEN/NOTIFY` is a best-effort latency accelerator only: it reduces propagation delay to near-zero when healthy, but missed notifications (connection drops, PgBouncer in transaction mode, network partitions) are harmless because the next poll cycle catches all changes. Each poll compares a server-side `updated_at` watermark against the cache and reloads only changed rules. No sticky sessions required. **[AR-4]**

---

## 07 Storage Layer

The storage layer is abstracted behind a `RuleStore` trait. Two concrete implementations ship: **SQLite** (default, zero-config) and **PostgreSQL** (recommended for HA production deployments).

### 7.1 Database Schema

```sql
-- rules table
-- PostgreSQL: use JSONB for tags to enable GIN index on tag-filtered queries [AR minor]
CREATE TABLE rules (
  id          TEXT        NOT NULL,
  version     INTEGER     NOT NULL,
  ars_version INTEGER     NOT NULL DEFAULT 1,     -- [AR-6] ARS schema version
  enabled     BOOLEAN     NOT NULL DEFAULT true,
  priority    INTEGER     NOT NULL DEFAULT 0,
  tags        TEXT        NOT NULL DEFAULT '[]',  -- TEXT in SQLite; JSONB in PostgreSQL
  definition  TEXT        NOT NULL,               -- full ARS JSON
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_by  TEXT,
  PRIMARY KEY (id, version)
);
-- PostgreSQL only: GIN index for performant tag-filtered lookups (§5.2)
-- CREATE INDEX rules_tags_gin ON rules USING GIN (tags::jsonb);

-- api_keys table [R2-new]
CREATE TABLE api_keys (
  id          TEXT        PRIMARY KEY,            -- user-visible key ID (slug)
  role        TEXT        NOT NULL,               -- admin | editor | viewer
  hash        TEXT        NOT NULL UNIQUE,        -- SHA-256(key_value) [R3-1]
  description TEXT,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_by  TEXT,                               -- ID of key that created this one
  revoked_at  TIMESTAMPTZ                         -- null = active
);
-- Index required for O(1) key lookup on every authenticated request [R3-1]
CREATE UNIQUE INDEX api_keys_hash_idx ON api_keys (hash);

-- rulesets table
CREATE TABLE rulesets (
  name        TEXT PRIMARY KEY,
  rule_ids    TEXT NOT NULL DEFAULT '[]',
  description TEXT,
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- evaluation_history (optional, disabled by default)
-- p95 row size ~4 KB; define a retention policy before enabling.
-- Partition by evaluated_at for large deployments [AR minor]
CREATE TABLE evaluation_history (
  id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  ruleset      TEXT,
  rule_id      TEXT,
  matched      BOOLEAN     NOT NULL,
  context      TEXT        NOT NULL,
  trace        TEXT        NOT NULL,
  duration_us  BIGINT      NOT NULL,
  evaluated_at TIMESTAMPTZ NOT NULL DEFAULT now()
) PARTITION BY RANGE (evaluated_at);              -- PostgreSQL only
```

### 7.2 RuleStore Trait

```rust
pub trait RuleStore: Send + Sync {
    async fn get_rule(&self, id: &str)             -> Result<Rule, StoreError>;
    async fn list_rules(&self, filter: RuleFilter) -> Result<Vec<Rule>, StoreError>;
    async fn upsert_rule(&self, rule: Rule)         -> Result<Rule, StoreError>;
    async fn disable_rule(&self, id: &str)          -> Result<(), StoreError>;
    async fn get_ruleset(&self, name: &str)         -> Result<Ruleset, StoreError>;
    async fn upsert_ruleset(&self, rs: Ruleset)     -> Result<Ruleset, StoreError>;
    async fn list_versions(&self, id: &str)         -> Result<Vec<u32>, StoreError>;
}
```

### 7.3 Storage Backend Failover Behaviour `[AR-8]`

If the storage backend becomes unreachable after startup:

| Condition | Behaviour |
|-----------|-----------|
| Storage unreachable | Continue serving evaluations from in-memory rule cache |
| `/ready` probe | HTTP 503 — load balancers stop routing new traffic to this instance |
| `/health` probe | HTTP 200 — process is alive; do not restart |
| Rule write requests (POST/PUT/PATCH) | HTTP 503 with `storage_unavailable` error |
| Key management requests | HTTP 503 — reads from config-file keys still work |
| `trigger` dead-letter writes | Written to disk (`AXIOM_DEAD_LETTER_PATH`), independent of storage health |
| Logging | `WARN` every 30 s with storage error details |
| Recovery | Automatic on reconnect; triggers full cache reload |

---

## 08 Language Bindings

All language bindings compile the same `axiom-core` Rust crate into a platform-specific shared library or native addon. The binding layer is intentionally thin: type marshalling only. Evaluation logic never lives in the binding layer.

### 8.1 Java Binding (JNI)

Maven/Gradle dependency bundles the native library for supported platforms as classifier JARs.

| Class / Interface | Purpose |
|-------------------|---------|
| `AxiomEngine` | Main entry point. Loads rules, exposes `evaluate()` and `evaluateRuleset()`. Thread-safe; share a single instance. |
| `Rule` | Immutable value object representing a parsed ARS rule. Built via `Rule.builder()` fluent API. |
| `EvaluationContext` | Wraps the JSON context. Accepts `Map<String,Object>`, `JsonNode`, or raw JSON string. |
| `EvaluationResult` | Holds `matched`, `matchedRules`, `tags`, `outputContext`, `trace`, `durationMicros`. |
| `EvaluationTrace` | Full structured trace. Serialises to JSON via Jackson. |
| `AxiomException` | Wraps all engine errors with structured message and optional field path. |

```java
// Java fluent builder API
Rule rule = Rule.builder()
  .id("premium-discount")
  .condition(Condition.all(
    Condition.field("user.plan").eq("premium"),
    Condition.field("order.total").gte(100)
  ))
  .action(Action.set("discount.percentage", 15))
  .build();

EvaluationResult result = engine.evaluate(rule, context);
```

### 8.2 Node.js / TypeScript Binding (NAPI-RS)

Published to npm as `@axiom-rules/core`. TypeScript declaration files are generated at build time from Rust struct definitions, ensuring type parity with the Java binding.

| Export | Purpose |
|--------|---------|
| `AxiomEngine` | Synchronous and async evaluation. `loadRule()`, `loadRuleset()`, `evaluate()`, `evaluateAsync()`. |
| `Rule` | TypeScript interface matching ARS schema. Importable for programmatic rule construction. |
| `EvaluationResult` | Typed result with full trace. Field names match Java binding for portability. |
| `validateRule(yaml)` | Parse and validate ARS YAML/JSON. Returns `ValidationResult` with typed errors array. |

### 8.3 Python Binding (PyO3 / maturin)

Published to PyPI as `axiom-rules`. Supports Python 3.9+. Provides both synchronous and `asyncio`-compatible evaluation via a thin Python async wrapper around the synchronous native call.

### 8.4 Binary Distribution Matrix `[AR-5, R2-4]`

Phase 1 ships two pre-built release binaries. macOS Intel is CI-verified (no release binary; `cargo build` instructions documented in `CONTRIBUTING.md`). Remaining targets added in Phase 2+ based on community demand and contributor capacity.

| Phase | Platform | Release binary | Notes |
|-------|----------|---------------|-------|
| **1** | linux-x86\_64 | ✅ | Cross-compiled via `cross-rs` |
| **1** | macOS Apple Silicon | ✅ | Native build on macOS ARM runners |
| **1** | macOS Intel | ❌ | CI-verified; contributor build docs in `CONTRIBUTING.md` **[R2-4]** |
| 2+ | linux-aarch64 (ARM) | ✅ | QEMU-tested in CI |
| 2+ | Windows x86\_64 | ✅ | |

---

## 09 Visual Rule Builder

The Visual Rule Builder is a single-page React application served as static assets. It connects to any running Axiom server via a configurable base URL. All data flows through the Axiom REST API — no backend of its own.

### 9.1 Module / Route Structure

| Route | Description |
|-------|-------------|
| `/rules` | Rule list with search, tag filter, status filter. Card and table display modes. |
| `/rules/new` | Rule editor: condition builder + action builder. Reads/writes ARS via server API. |
| `/rules/:id` | Rule detail: current version, history diff, test panel, enable/disable toggle. |
| `/rulesets` | Ruleset management: create, edit membership, evaluate a full ruleset in the test panel. |
| `/tables` | Decision table view: spreadsheet-style, each row = one rule, columns = conditions + actions. |
| `/flow` | React Flow node-based diagram showing rule chains via `call_rule` and `trigger` actions. [P1] |
| `/settings` | Server connection URL, API key, theme toggle, RBAC role display. |

### 9.2 Condition Builder Component

The condition builder is a recursive React component that mirrors the ARS condition tree. Each group node renders as a block with an operator selector (`all` / `any` / `none` / `not`) and a list of children. The `not` operator is rendered as a **single-child wrapper** — the UI enforces this constraint visually (no "Add condition" button inside a `not` group once a child exists), ensuring the UI cannot produce ARS documents that the parser would reject. **[AR-1]**

> **UX Principle:** The condition builder never shows raw YAML/JSON to the user unless they explicitly click "View source". The UI is the rule — the YAML is the implementation detail.

### 9.3 Live Test Panel

The live test panel accepts a JSON context payload and calls `POST /v1/evaluate` in dry-run mode on every keystroke (debounced 300 ms). The response trace is rendered as a collapsible tree: each rule shows pass/fail, each condition shows the resolved field value and the comparison result. This gives immediate feedback without mutating any output context.

### 9.4 Role-Based Access Control `[AR-7]`

Roles are enforced server-side by the API key's `role` field (§6.4). The UI reads the role from the server on connect and hides controls the current key does not have permission to use. There is no UI-side auth layer — the server is the authority.

| Role | Permissions |
|------|-------------|
| `admin` | Full access: create, edit, delete rules and rulesets; manage API keys via `/v1/keys`; change server config. |
| `editor` | Create and edit rules; create rulesets; use test panel. Cannot delete or manage keys. |
| `viewer` | Read-only: view rules, rulesets, evaluation history. Can use test panel. Cannot write anything. |

---

## 10 CLI

The CLI is a single statically-linked Rust binary distributed via GitHub Releases, Homebrew (macOS/Linux), Chocolatey (Windows), and as a Docker image.

### 10.1 Command Reference

| Command | Description | Exit code |
|---------|-------------|-----------|
| `axiom validate <path>` | Parse and schema-validate all ARS files at path. Prints errors with `file:line` references. | `1` on any error |
| `axiom test <path>` | Run all `*.test.yaml` files. Prints pass/fail per case. Outputs JUnit XML to `--output` if set. | `1` on any failure |
| `axiom evaluate --rule <id> --context <json>` | Evaluate a local rule file against a JSON context string. Add `--fail-on-no-match` to exit `1` when no match. **[AR minor]** | `0` / `1` with flag |
| `axiom evaluate --server <url> --rule <id> --context <json>` | Evaluate via a remote Axiom server. | `1` on server error |
| `axiom import <bundle.yaml> --server <url>` | Import a rule bundle into a remote server. | `1` on failure |
| `axiom export --server <url> --output <file>` | Export all rules from a remote server to a local bundle file. | `1` on failure |
| `axiom serve --rules <path>` | Start a local Axiom server loading rules from path. For local development. | `1` on startup error |
| `axiom keygen --role <role> [--description <text>]` | Generate a cryptographically random API key and print both the plaintext value (for use as `X-Axiom-Key`) and the `sha256:...` hash (for use in `axiom.yaml` or `POST /v1/keys`). The plaintext is printed once and not stored. **[R3-3]** | `0` always |

### 10.2 CI/CD Integration Pattern

1. On pull request: run `axiom validate rules/` then `axiom test rules/` in CI. Fail the PR if either exits `1`.
2. On merge to main: run `axiom import bundle.yaml --server $AXIOM_URL` to push validated rules to production.
3. The server reloads updated rules within at most one polling interval (default 10 s) without restart.

### 10.3 Rule Test File Format

```yaml
# loan-eligibility.test.yaml
rule: loan-eligibility-check
tests:
  - name: "Approved — high credit score, sufficient income"
    context:
      applicant:
        credit_score: 720
        annual_income: 60000
        existing_debt_ratio: 0.2
    expect:
      matched: true
      actions:
        result.eligible: true
        result.max_loan_amount: 180000

  - name: "Rejected — credit score below threshold"
    context:
      applicant:
        credit_score: 580
        annual_income: 60000
        existing_debt_ratio: 0.2
    expect:
      matched: false

  - name: "Rejected — high debt ratio despite good credit"
    context:
      applicant:
        credit_score: 720
        annual_income: 60000
        existing_debt_ratio: 0.55
    expect:
      matched: false
```

---

## 11 Observability & Auditability

Every evaluation — server, library, or CLI — returns an `EvaluationTrace` object. The trace is always serialisable to JSON and always has the same structure regardless of consumption mode.

### 11.1 Evaluation Trace Schema

| Field | Type | Description |
|-------|------|-------------|
| `rules_evaluated` | `integer` | Total number of rules checked (including non-matching). |
| `rules_matched` | `integer` | Number of rules that matched. |
| `strategy` | `string` | Evaluation strategy used: `first_match` \| `all_match` \| `scored`. |
| `total_duration_us` | `integer` | Wall-clock time for full evaluation in microseconds. |
| `timed_out` | `boolean` | True if evaluation was aborted due to timeout budget exhaustion. |
| `rules[].rule_id` | `string` | ID of the rule evaluated. |
| `rules[].matched` | `boolean` | Whether the rule matched. |
| `rules[].conditions[]` | `object[]` | Per-condition: `field`, `operator`, `value`, `actual_value`, `passed`, `duration_us`. |
| `rules[].short_circuited` | `boolean` | True if condition evaluation stopped early. |
| `rules[].actions_executed` | `string[]` | Actions executed (type + field + resolved value). |
| `rules[].duration_us` | `integer` | Evaluation time for this rule alone. |

### 11.2 Prometheus Metrics

| Metric name | Type | Description |
|-------------|------|-------------|
| `axiom_evaluations_total` | Counter | Total evaluation requests. Labels: `strategy`, `ruleset`. |
| `axiom_evaluation_duration_seconds` | Histogram | Latency buckets: 1 ms, 5 ms, 10 ms, 25 ms, 50 ms, 100 ms, 250 ms, 500 ms. |
| `axiom_rules_matched_total` | Counter | Total rule matches. Label: `rule_id`. |
| `axiom_rules_loaded` | Gauge | Current number of enabled rules in registry. |
| `axiom_evaluation_timeouts_total` | Counter | Evaluations aborted by timeout budget. |
| `axiom_store_query_duration_seconds` | Histogram | Storage layer query latency. |

### 11.3 Structured Logging

All server log output is structured JSON (tracing crate + JSON subscriber). Every log line includes: `timestamp`, `level`, `component`, `trace_id` (if request is traced), and `rule_id` or `ruleset` where applicable. Compatible with ELK, Loki, Datadog, and CloudWatch.

---

## 12 Deployment Architecture

### 12.1 Single-Instance (Development / Small Teams)

```yaml
# docker-compose.yml — single instance
services:
  axiom:
    image: ghcr.io/axiom-rules/axiom:latest
    ports: ['8080:8080']
    volumes:
      - ./data:/data              # SQLite database
      - ./dead-letter:/data/dead-letter   # trigger dead-letter path [R3-2]
    environment:
      AXIOM_STORAGE_BACKEND:    sqlite
      AXIOM_STORAGE_PATH:       /data/axiom.db
      AXIOM_API_KEY:            ${AXIOM_API_KEY}
      AXIOM_DEAD_LETTER_PATH:   /data/dead-letter   # default, shown explicitly
```

### 12.2 High-Availability (Production)

| Component | Replicas | Notes |
|-----------|----------|-------|
| `axiom-server` | 2–N | Stateless. Scale based on evaluation request volume. All share same PostgreSQL. |
| PostgreSQL | 1 primary + 1 replica | Rules are write-infrequent. Standard HA setup (Patroni or RDS Multi-AZ) is sufficient. |
| `axiom-ui` | 1–2 | Static SPA; served from nginx or a CDN. Connects to axiom-server via internal URL. |
| Load balancer | 1 | Any L7 LB (nginx, Traefik, AWS ALB). Sticky sessions not required. |

### 12.3 Kubernetes / Helm

Key configurable Helm chart values (`axiom/deploy/helm/`):

| Value | Description |
|-------|-------------|
| `replicaCount` | Number of axiom-server pods |
| `storage.backend` | `sqlite` or `postgres` |
| `storage.postgresUrl` | External PostgreSQL connection string |
| `deadLetter.path` | Mount path for trigger dead-letter files (default: `/data/dead-letter`) |
| `ingress.enabled` | Configure ingress with TLS termination |
| `metrics.serviceMonitor` | Enable Prometheus ServiceMonitor for kube-prometheus-stack |
| `ui.enabled` | Deploy axiom-ui as a separate Deployment + Service |

### 12.4 Storage Backend Failover

See §7.3 for the authoritative storage failover behaviour table. In summary: the server serves evaluations from its in-memory cache when storage is unreachable, fails the `/ready` probe (stopping new traffic), and recovers automatically on reconnect. Trigger dead-letter writes go to disk and are unaffected by storage outages. **[AR-8, R2-5]**

---

## 13 Build Phases & Requirement Mapping

### Phase 1 — Core Engine + REST Server _(Months 1–5)_

**Deliverables:** ARS schema (including required `ars_version` field) · Rust evaluation engine with `call_rule_guard` (depth 4, cycle detection, missing-rule detection at load time) · REST server · Role-scoped API key auth (config-file based) · `axiom keygen` CLI command · Docker image · Docker Compose example (with dead-letter volume mount) · Java library · Node.js/TypeScript library · linux-x86\_64 and macOS ARM release binaries · macOS Intel CI-verified build · Contributor build docs · Basic documentation site · OpenAPI 3.0 spec.

| ID | Requirement | Priority |
|----|-------------|----------|
| RM-01 | Define rules in YAML or JSON format conforming to ARS | **P0** |
| RM-02 | Load rules from local filesystem, directory watch, or REST API | **P0** |
| RM-03 | Version rules — each rule has a version number, old versions retained | **P0** |
| RM-04 | Enable and disable rules without deletion | **P0** |
| RM-05 | Assign priority to rules — higher priority rules evaluated first | **P0** |
| RM-09 | Validate rule schema on load, returning detailed error messages | **P0** |
| EV-01 | Evaluate context object against a named rule | **P0** |
| EV-02 | Evaluate context object against an entire ruleset | **P0** |
| EV-03 | Support three evaluation strategies: first-match, all-match, scored | **P0** |
| EV-04 | Produce a full evaluation trace | **P0** |
| SV-01 | REST API for rule management (CRUD operations) | **P0** |
| SV-02 | REST API for rule evaluation: `POST /evaluate` | **P0** |
| SV-04 | Role-scoped API key authentication for all endpoints (config-file keys, Phase 1) | **P0** |
| SV-06 | Provide OpenAPI 3.0 specification for all REST endpoints | **P0** |
| SV-09 | Configurable via environment variables and YAML config file | **P0** |
| SV-10 | Docker-first distribution — official Docker image | **P0** |
| LB-01 | Java library (Maven/Gradle) — core evaluation engine | **P0** |
| LB-02 | Node.js/TypeScript library (npm) — TypeScript types included | **P0** |
| LB-05 | All libraries share identical rule format (ARS) | **P0** |
| LB-06 | Libraries load rules from file path, string, URL, or object | **P0** |
| LB-08 | Libraries produce identical evaluation traces to the server | **P0** |
| OB-01 | Every evaluation returns a structured trace object | **P0** |
| OB-02 | Trace includes rules, conditions, pass/fail, actions, timing | **P0** |
| DX-07 | Docker Compose example with server + UI + PostgreSQL | **P0** |
| DX-08 | Comprehensive documentation site | **P0** |

### Phase 2 — Developer Tooling + Testing _(Months 6–9)_

**Deliverables:** CLI (full command set) · Rule testing framework · Python library · Hot-reload · Batch evaluation (max 1,000) · Dry-run · Timeout enforcement · Expression template engine · `trigger` webhook implementation (default 3× exponential backoff, disk dead-letter) · REST API key management endpoints · `call_rule` depth raised to 8 · Helm chart · Prometheus metrics · Structured JSON logging · Expand binary matrix to all 5 platforms · `axiom-finance` and `axiom-ecommerce` module bundles.

| ID | Requirement | Priority |
|----|-------------|----------|
| RM-06 | Group rules into named rulesets | P1 |
| RM-07 | Tag rules for filtering | P1 |
| RM-08 | Import/export rulesets as YAML/JSON bundles | P1 |
| RM-10 | Hot-reload rules from filesystem without restart | P1 |
| EV-05 | Expression evaluation in action values using `{{ }}` template syntax | P1 |
| EV-06 | Chained rule evaluation via `call_rule` action | P1 |
| EV-08 | Evaluate rules against streaming events (stateless) | P1 |
| EV-10 | Batch evaluation — submit array of contexts (max 1,000) | P1 |
| EV-11 | Dry-run mode — evaluate without applying actions | P1 |
| EV-12 | Evaluation timeout enforcement with partial trace return | P1 |
| SV-07 | Horizontal scaling — stateless design, rules from shared storage | P1 |
| SV-08 | Health check endpoints + Prometheus metrics | P1 |
| SV-11 | Rule storage backends: filesystem, PostgreSQL, SQLite | P1 |
| LB-03 | Python library (PyPI) — type hints included | P1 |
| LB-07 | Async/sync evaluation modes in libraries | P1 |
| LB-10 | Fluent builder API for programmatic rule construction | P1 |
| OB-03 | Structured JSON evaluation logs | P1 |
| OB-04 | Prometheus metrics exposure | P1 |
| DX-01 | CLI tool (`axiom`) for local validation, testing, evaluation | P1 |
| DX-02 | `axiom validate` — CI/CD integration, exit 1 on error | P1 |
| DX-03 | `axiom test` — run YAML test files | P1 |
| DX-04 | `axiom evaluate` — one-line CLI evaluation with `--fail-on-no-match` flag | P1 |
| DX-06 | Helm chart for Kubernetes deployment | P1 |
| TS-01 | Rule test files defined in YAML alongside rule files | P1 |
| TS-02 | `axiom test` runs all test files, reports pass/fail | P1 |
| TS-03 | JUnit XML output for CI/CD integration | P1 |

### Phase 3 — Visual Rule Builder _(Months 10–14)_

**Deliverables:** Condition builder UI (single-child `not` enforcement) · Decision table view · Live test panel · Rule history and diff view · RBAC (via server-enforced API key roles from Phase 1) · React Flow dependency diagram · Import/export bundles · `axiom-access` and `axiom-compliance` module bundles.

| ID | Requirement | Priority |
|----|-------------|----------|
| VB-01 | Web-based UI for creating and editing rules without YAML | **P0** |
| VB-02 | Condition builder: dropdown selectors, composable AND/OR/NOT (not = single child) | **P0** |
| VB-03 | Decision table view — spreadsheet-style interface | **P0** |
| VB-05 | Live test panel — paste context JSON, see evaluation in real time | **P0** |
| VB-08 | Import/export rules as YAML/JSON bundles from the UI | **P0** |
| VB-11 | UI is self-contained — connects to any Axiom server via URL | **P0** |
| VB-04 | Rule flow diagram — node-based editor showing rule chains | P1 |
| VB-06 | Rule search, filter by tag, status, ruleset | P1 |
| VB-07 | Rule change history — who changed what, when, diff view | P1 |
| VB-09 | Role-based access: admin, editor, viewer (server-enforced via API key roles) | P1 |
| VB-10 | Deployment controls — draft to active promotion with approval | P2 |
| VB-12 | Dark/light theme | P2 |

### Phase 4 — Advanced Engine Features _(Months 15–20)_

| ID | Requirement | Priority |
|----|-------------|----------|
| EV-07 | Rule conflict detection — warn on contradictory actions | P2 |
| EV-09 | Stateful context — accumulate state across events (CEP mode) | P2 |
| RM-11 | Rule inheritance — extend base rule, override conditions | P2 |
| SV-03 | GraphQL API as alternative to REST | P2 |
| SV-05 | Mutual TLS for secure inter-service communication | P2 |
| SV-12 | Webhook endpoint for rule change notifications | P2 |
| LB-04 | Go library (pkg.go.dev) | P2 |
| LB-09 | Zero-allocation fast path for simple condition evaluation | P2 |
| OB-05 | Evaluation history store — retain last N evaluations per ruleset | P2 |
| OB-06 | Rule usage analytics — match frequency per rule | P2 |
| TS-04 | Test coverage report — which conditions are covered by tests | P2 |
| TS-05 | Snapshot testing — save trace as snapshot, alert on changes | P2 |
| DX-05 | VS Code extension: ARS syntax highlighting, validation, autocomplete | P2 |
| DX-09 | Interactive browser playground — try Axiom without installation | P2 |

---

## 14 Cross-Cutting Concerns

### 14.1 ARS Versioning & Backward Compatibility

ARS is versioned independently of the engine. **Every rule document must include an `ars_version` field** (§4.2) — required from ARS 1.0, not added later. **[AR-6]** The schema version is also declared in the `Content-Type` header for API responses (`application/vnd.axiom.rule+json;version=1`). The evaluation engine always supports at least two major ARS versions simultaneously. Breaking changes require a public RFC, a migration guide, and a minimum 6-month deprecation period.

### 14.2 Safety & Sandboxing

Safety is a non-negotiable design constraint — not a feature. Three layers enforce it:

1. **Grammar-level:** The expression language grammar has no production rules for function calls, identifiers, or I/O. Invalid programs are rejected at parse time.
2. **AST-level:** Recursion depth > 16 is rejected before evaluation begins.
3. **Runtime-level:** Regex evaluation is bounded by per-pattern timeout; arithmetic uses checked operations; the entire evaluation is bounded by a configurable wall-clock budget.

### 14.3 Testing Philosophy

The target for `axiom-core` is **> 90% line coverage** and **> 80% branch coverage** enforced in CI. The test suite is structured as:

- **Unit tests:** One test module per operator, per action type, per condition group type.
- **Property-based tests** (proptest crate): Random rule + context combinations verifying evaluation consistency and no-panic guarantees.
- **Integration tests:** Full evaluation scenarios from YAML rule fixtures against JSON contexts, asserting on trace content.
- **Benchmark tests** (criterion crate): Regression benchmarks for 10K rule evaluation and single-rule latency run on every PR. < 5% degradation fails the PR.

### 14.4 Contribution Workflow

1. Open an issue or RFC for features that affect ARS or public API contracts.
2. Fork the monorepo and work on a feature branch.
3. All PRs must pass: `cargo test`, `cargo clippy`, `cargo fmt --check`, `axiom validate` and `axiom test` on the example rule bundles, and the benchmark regression check.
4. Two maintainer approvals required for merges to `main`.
5. **Semver:** patch for bug fixes; minor for new operators/actions (ARS-compatible); major for ARS breaking changes.

---

## 15 Open Design Decisions

Items resolved by a previous architecture review are marked **RESOLVED**. Remaining items require community RFC before implementation.

| Topic | Status / Recommendation |
|-------|------------------------|
| UI auth model | **RESOLVED [AR-7]:** API keys carry a `role` field enforced server-side from Phase 1. UI reads role on connect; no separate UI auth layer. OAuth2/OIDC remains P2 in Phase 4. |
| `trigger` action contract | **RESOLVED [AR-9, R2-3, R3-2]:** Server = webhook + HMAC-SHA256 + 3× exponential backoff (1 s / 4 s / 16 s) + disk dead-letter at `AXIOM_DEAD_LETTER_PATH` (default `/data/dead-letter/`). Library = `engine.onTrigger(event, fn)` callback. |
| `call_rule` depth default | **RESOLVED [R2-1]:** Phase 1 default = **4**. Raised to **8** in Phase 2 after p99 latency benchmarks. Runtime-configurable; not a schema value. |
| API key management model | **RESOLVED [R2-new]:** Phase 1 = config-file keys in `axiom.yaml` with `axiom keygen` for hash generation. Phase 2 = REST API (`/v1/keys`). Invariants documented in §6.2. |
| `call_rule` missing-rule behaviour | **RESOLVED [R3-4]:** Missing `call_rule` targets are load-time validation errors (`UnresolvedRuleReferenceError`), consistent with the fail-fast philosophy. Runtime never encounters an unresolved reference. |
| CEP state storage backend [EV-09] | Start with in-memory per-server for Phase 4; RFC for Redis-backed state before GA. |
| Rule conflict detection algorithm [EV-07] | Start with heuristic (same field + overlapping conditions); flag for manual review, not hard block. |
| Expression language extension | No function support for Phase 1–2. RFC required. High risk of DSL scope creep. |
| ARS 2.0 scope | Open RFC after Phase 2 ships. Candidates: duration operators, geo operators, ML score threshold. |

---

## 16 Non-Functional Requirements

| ID | Requirement |
|----|-------------|
| NF-01 | Evaluate 10,000 simple rules against a context in under 100 ms on a standard server. |
| NF-02 | Evaluate a single complex rule with 50 nested conditions in under 5 ms. |
| NF-03 | Server handles 5,000 evaluation requests/second on a single 2-core instance. |
| NF-04 | Embedded library adds less than 5 MB to application binary size (Java/Node). |
| NF-05 | Zero mandatory external runtime dependencies for the core evaluation engine. |
| NF-06 | Rules must be safe to evaluate — no arbitrary code execution possible from rule definitions. |
| NF-07 | Expression evaluation must be sandboxed — no filesystem, network, or system access. |
| NF-08 | Full test coverage (> 90%) on the evaluation engine core. |
| NF-09 | Server runs on Linux, macOS, and Windows. |
| NF-10 | All APIs are backwards-compatible within a major version. |

---

## 17 Architecture Review Findings Log

The authoritative decision log for all architecture review findings across all revisions. Future contributors can use this to understand why the architecture looks the way it does.

### Review Round 1 (v1.0 → v1.1)

| ID | Finding | Severity | Decision | Section(s) |
|----|---------|----------|----------|------------|
| AR-1 | `not` group accepted an array (NAND semantics), indistinguishable from `none` (NOR) in natural language. | High | `not` accepts **exactly one child node**. Parser rejects array-valued `not`. Migration path documented. | §4.3, §5.3, §9.2 |
| AR-2 | No `call_rule` depth limit or cycle detection. Mutual recursion would recurse until stack overflow or timeout. | High | Maximum depth enforced at evaluation time. Topological sort + missing-rule check at load time. New `call_rule_guard` module. Default: 4 (Phase 1), 8 (Phase 2). | §4.6, §5.1, §3.1 |
| AR-3 | Scored strategy underspecified: nested group counting, tie-break order, score-0 rule visibility all undefined. | Medium | Concrete algorithm: group nodes scored as virtual leaves (resolved result). Ties by `priority DESC` then `id ASC`. Score-0 rules excluded. | §5.1, §5.4 |
| AR-4 | `LISTEN/NOTIFY` framed as propagation mechanism. Missed notifications cause silently stale rules. | Medium | Polling is the reliability mechanism. `LISTEN/NOTIFY` is best-effort only. `updated_at` watermark per poll. | §6.5 |
| AR-5 | 15 binary artifacts per release unsustainable for a volunteer-maintained project. | Medium | Phase 1: 2 release binaries (linux-x86\_64, macOS ARM). macOS Intel: CI-verified, no binary. Phase 2+: full matrix. | §8.4 |
| AR-6 | No `ars_version` field. Engine cannot determine which schema a stored rule conforms to when ARS 2.0 ships. | High | `ars_version: 1` is a required field from ARS 1.0. Added to schema table, YAML example, SQL DDL, §14.1. | §4.1, §4.2, §7.1, §14.1 |
| AR-7 | RBAC roles defined but API keys had no `role` concept. Phase 3 UI would have no server enforcement. | High | Each API key carries a `role` field enforced server-side from Phase 1. UI reads role on connect. | §6.4, §9.4, §13 |
| AR-8 | No stated behaviour when storage backend becomes unreachable after startup. | Medium | Explicit failover table: serve from cache; `/ready` → 503; `/health` → 200; writes → 503; auto-recover. | §7.3 |
| AR-9 | `trigger` action vague: no webhook config model, retry policy, dead-letter, or library-mode contract. | Medium | Concrete contract: server = webhook + HMAC-SHA256 + retry + dead-letter. Library = callback. Defaults in Round 3. | §4.6 |
| AR-10 | Batch endpoint had no size limit. 100,000-context request could starve other evaluations. | Medium | Max 1,000 contexts per batch (HTTP 400 if exceeded). Worker pool bounded. N-count against rate limit. | §6.1 |
| AR-minor | `tags` as TEXT forgoes PostgreSQL JSONB indexing for a performance-critical path. | Low | PostgreSQL uses JSONB with GIN index. SQLite uses TEXT. DDL updated. | §7.1 |
| AR-minor | `evaluation_history` row sizes undocumented; no partitioning guidance. | Low | p95 row size noted (~4 KB). `PARTITION BY RANGE` added to DDL. | §7.1 |
| AR-minor | `axiom evaluate` exited `0` even when no match — breaks scripting use cases. | Low | Added `--fail-on-no-match` flag. | §10.1 |

### Review Round 2 (v1.1 → v1.2)

| ID | Finding | Severity | Decision | Section(s) |
|----|---------|----------|----------|------------|
| R2-1 | `call_rule` depth default of 8 is generous for Phase 1; could exhaust timeout budgets before benchmarks exist. | Low | Phase 1 default: **4**. Raised to **8** in Phase 2 after p99 benchmarks on realistic deep chains. | §4.6, §5.1, §3.1 |
| R2-2 | Scored strategy "flatten to leaves" was ambiguous for `not` groups (inner leaf `false`, group resolves `true`). | Medium | Group nodes are virtual leaves scored by their resolved result, not their children's raw values. New §5.4 added. | §5.1, §5.4 |
| R2-3 | `trigger` retry policy documented as "configurable" with no default. Zero-config experience undefined. | Low | Default: 3 attempts, 1 s / 4 s / 16 s exponential backoff. Disk dead-letter (not database). Per-event overridable. | §4.6 |
| R2-4 | Phase 1 matrix omits macOS Intel — a common contributor platform. No binary creates friction. | Low | macOS Intel added as CI-verified build target (no release binary). `cargo build` docs in `CONTRIBUTING.md`. | §8.4 |
| R2-5 | §12.4 duplicated §7.3 failover table verbatim — maintenance risk of drift. | Low | §12.4 reduced to one-paragraph summary with cross-reference to §7.3. | §12.4 |
| R2-new | API key creation/management surface undefined. Phase 3 UI team would have to reverse-engineer it. | Medium | Full model in §6.2: Phase 1 = config-file; Phase 2 = REST API. Invariants, `api_keys` table, endpoints all documented. | §6.1, §6.2, §7.1 |

### Review Round 3 (v1.2 → v1.3)

| ID | Finding | Severity | Decision | Section(s) |
|----|---------|----------|----------|------------|
| R3-1 | `api_keys` table had no index on `hash`. Every authenticated request requires a full table scan once REST key management goes live in Phase 2. | Low | Added `CREATE UNIQUE INDEX api_keys_hash_idx ON api_keys (hash)`. Also adds `UNIQUE` constraint on the column itself. | §7.1 |
| R3-2 | `trigger` dead-letter path documented as "configurable" with no stated default. Docker deployments have no obvious mount point. | Low | Default path: `/data/dead-letter/` (same volume as SQLite database). Added explicit mount to Docker Compose example and `deadLetter.path` to Helm values table. | §4.6, §12.1, §12.3 |
| R3-3 | `axiom.yaml` key format uses a SHA-256 hash but doesn't explain how operators generate it. Early adopters must figure it out themselves. | Low | Added `axiom keygen` CLI command that generates key + hash in one step (§10.1). Added §6.2.1 with manual fallback using `openssl`/`sha256sum` for environments without the CLI. `sha256:` prefix required in config to make algorithm explicit and allow future migration. | §6.2.1, §10.1 |
| R3-4 | `call_rule_guard` module documented cycle detection but not what happens when a referenced rule doesn't exist. Load vs. runtime failure was unstated. | Low | Explicitly documented as a **load-time validation error** (`UnresolvedRuleReferenceError`), consistent with fail-fast philosophy. All three `call_rule_guard` checks (cycles, missing rules, depth) are load-time. §5.1 and §15 updated. Component diagram updated. | §5.1, §3.1, §15 |

### Pre-Phase 1 Blockers — All Resolved

| Finding | Resolution |
|---------|-----------|
| AR-1 `not` semantics | ✅ Single child, parser enforced |
| AR-2 `call_rule` cycles + missing-rule detection | ✅ Topo sort + existence check at load time; depth 4 Phase 1 |
| AR-6 `ars_version` required field | ✅ In schema, YAML, SQL, §14.1 |
| AR-7 API key roles | ✅ Role field from Phase 1, server-enforced |
| R2-new API key management model | ✅ §6.2, phased config-file → REST API, `axiom keygen` |
| R3-4 Missing `call_rule` target behaviour | ✅ Load-time `UnresolvedRuleReferenceError`, documented in §5.1 and §15 |

---

*Axiom Architecture Design Document · v1.3 · April 2026 · Apache 2.0 · Community-driven*
