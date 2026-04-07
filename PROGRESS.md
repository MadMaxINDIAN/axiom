# Axiom — Build Progress

> Architecture reference: `docs/axiom-architecture-v1.3.md`
> Last updated: 2026-04-07

---

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Complete |
| 🚧 | In progress |
| ⬜ | Not started |
| [P2] | Phase 2 item |
| [P3] | Phase 3 item |
| [P4] | Phase 4 item |

---

## Phase 1 — Core Engine + REST Server _(Months 1–5)_ ✅

### Rust Core (`crates/axiom-core`)

| Component | Status | Notes |
|-----------|--------|-------|
| ARS schema structs (`Rule`, `ConditionGroup`, `Action`, …) | ✅ | `src/schema.rs`; `op` accepted as alias for `operator` |
| YAML / JSON parser + schema validation | ✅ | `src/parser.rs`; bundle parser supports `rules:` + `rulesets:` |
| Bundle import/export parser (`parse_bundle_yaml/json`) | ✅ | `src/parser.rs` |
| In-memory rule registry | ✅ | `src/registry.rs`; runtime-configurable `max_call_depth` |
| Dot-notation + array-index resolver | ✅ | `src/resolver.rs` |
| Condition tree evaluator (short-circuit, dry-run) | ✅ | `src/evaluator.rs` |
| All 30+ ARS operators | ✅ | `src/operators.rs` |
| Action executor (set, increment, append, tag, log, return, trigger, call_rule) | ✅ | `src/evaluator.rs` |
| Sandboxed `{{ expr }}` template engine (AST depth limit 16) | ✅ | `src/expression.rs` |
| `call_rule_guard` (load-time cycle detect, missing-rule, runtime depth limit 8) | ✅ | `src/call_rule_guard.rs` |
| Evaluation trace (`EvaluationTrace`, `RuleTrace`, `ConditionTrace`) | ✅ | `src/trace.rs` |
| Three evaluation strategies (FirstMatch, AllMatch, Scored §5.4) | ✅ | `src/evaluator.rs` |
| Timeout budget per evaluation | ✅ | `src/timeout.rs` |
| `EvalConfig` (strategy, dry_run, timeout_ms, max_call_depth, rule_lookup) | ✅ | `src/evaluator.rs` |
| Unit tests | ✅ | Inline `#[cfg(test)]` across all modules |

### REST Server (`crates/axiom-server`)

| Endpoint / Feature | Status | Notes |
|-------------------|--------|-------|
| `GET /health` | ✅ | |
| `GET /ready` | ✅ | 503 when storage unreachable |
| `GET /metrics` | ✅ | Prometheus text format |
| `GET /v1/rules` | ✅ | tag / enabled filters |
| `POST /v1/rules` | ✅ | |
| `GET /v1/rules/:id` | ✅ | |
| `GET /v1/rules/:id/versions` | ✅ | |
| `PUT /v1/rules/:id` | ✅ | auto-increments version |
| `PATCH /v1/rules/:id` | ✅ | partial update |
| `DELETE /v1/rules/:id` | ✅ | soft-delete (disable) |
| `GET /v1/rulesets` | ✅ | |
| `POST /v1/rulesets` | ✅ | |
| `GET /v1/rulesets/:name` | ✅ | |
| `PUT /v1/rulesets/:name` | ✅ | |
| `POST /v1/evaluate` | ✅ | rate-limited, metrics, webhook dispatch |
| `POST /v1/evaluate/batch` | ✅ | max 1,000; rate-limited by N |
| `GET /v1/keys` | ✅ | admin only |
| `POST /v1/keys` | ✅ | [P2] creates DB key, returns plaintext once |
| `DELETE /v1/keys/:id` | ✅ | [P2] revoke; guards last admin |
| `POST /v1/import` | ✅ | |
| `GET /v1/export` | ✅ | |
| SQLite storage backend | ✅ | `src/storage/sqlite.rs` |
| PostgreSQL storage backend | ✅ | `src/storage/postgres.rs`; JSONB + GIN index |
| API key auth (`X-Axiom-Key`, SHA-256, roles: viewer / editor / admin) | ✅ | `src/auth.rs` |
| Config-file + DB keys | ✅ | `src/config.rs` + `src/routes/keys.rs` |
| Background rule-poll loop (default 10 s) | ✅ | `src/main.rs` |
| Filesystem hot-reload (`rules_dir` + `notify` crate) | ✅ | [P2] `src/watch.rs`; 200 ms debounce |
| Storage failover (serve from cache on outage) | ✅ | poll loop + ready probe |
| Token-bucket rate limiting (per API key) | ✅ | [P2] `src/rate_limit.rs`; 1,000 req/s default |
| Trigger webhooks (HMAC-SHA256, 3× exp backoff, dead-letter) | ✅ | [P2] `src/webhook.rs` |
| Prometheus metrics (counters, histograms, gauges) | ✅ | [P2] `src/metrics.rs` |
| Structured JSON logging | ✅ | [P2] `tracing-subscriber` JSON format |
| Mutual TLS | ⬜ | [P3] |

### CLI (`crates/axiom-cli`)

| Command | Status | Notes |
|---------|--------|-------|
| `axiom validate <path>` | ✅ | detects single rules and bundles automatically |
| `axiom test <path>` | ✅ | JUnit XML via `--output` |
| `axiom evaluate --rule <path> --context <json>` | ✅ | local (no server) |
| `axiom evaluate --server … --rule-id … --context …` | ✅ | remote |
| `axiom import <bundle> --server …` | ✅ | |
| `axiom export --server …` | ✅ | |
| `axiom serve --rules <path>` | ✅ | delegates to server binary |
| `axiom keygen --role …` | ✅ | prints key + `sha256:` hash |

### Language Bindings

| Binding | Status | Notes |
|---------|--------|-------|
| Node.js / TypeScript (NAPI-RS) | ✅ | `bindings/node/` — Rust glue, `index.js`, `index.d.ts`, 7 tests |
| Java (JNI) | ✅ | `bindings/java/` — Rust JNI glue, Java wrappers, Maven pom, 9 JUnit 5 tests |
| Python (PyO3 / maturin) | ✅ | [P2] `bindings/python/` — PyO3 0.23, asyncio wrapper, 18 pytest tests |
| Go (cgo) | ⬜ | [P4] |

### Infrastructure

| Item | Status | Notes |
|------|--------|-------|
| Cargo workspace | ✅ | `Cargo.toml` |
| ARS JSON Schema (`schema/ars-v1.json`) | ✅ | JSON Schema 2020-12 |
| Dockerfile (multi-stage) | ✅ | |
| `docker-compose.yml` (single-instance, SQLite) | ✅ | |
| `docker-compose.ha.yml` (2 replicas + PostgreSQL + nginx) | ✅ | [P2] `deploy/docker-compose.ha.yml` + `deploy/nginx.conf` |
| Helm chart (`deploy/helm/axiom/`) | ✅ | [P2] Deployment, Service, Ingress, ConfigMap, PVC, HPA, ServiceMonitor |
| GitHub Actions CI | ⬜ | |
| OpenAPI 3.0 spec | ⬜ | |
| Docusaurus docs site | ⬜ | [P3] |

---

## Phase 2 — Developer Tooling + Testing _(Months 6–9)_ ✅

| Item | Status | Notes |
|------|--------|-------|
| Python binding (PyO3 / maturin) | ✅ | `bindings/python/` |
| Hot-reload / filesystem watch | ✅ | `notify` crate, `src/watch.rs`, `rules_dir` config |
| Batch evaluation (up to 1,000) | ✅ | Sequential under single read lock; µs-scale evals |
| Dry-run mode (full trace, no side-effects) | ✅ | `EvalConfig.dry_run` |
| Timeout enforcement (per-eval budget) | ✅ | `src/timeout.rs` |
| `trigger` webhook (3× exp backoff, dead-letter) | ✅ | `src/webhook.rs` |
| REST API key management (`POST/DELETE /v1/keys`) | ✅ | `src/routes/keys.rs` |
| `call_rule` depth raised to 8 | ✅ | `Registry::max_call_depth = 8` (was 4 in P1) |
| Rate limiting (token bucket per key) | ✅ | `src/rate_limit.rs` |
| Prometheus metrics (eval throughput, latency histogram) | ✅ | `src/metrics.rs` |
| Structured JSON logging (tracing crate) | ✅ | `tracing-subscriber` JSON format |
| Helm chart | ✅ | `deploy/helm/axiom/` |
| `axiom-finance` module bundle | ✅ | `modules/axiom-finance/bundle.yaml` — 12 rules, 2 rulesets |
| `axiom-ecommerce` module bundle | ✅ | `modules/axiom-ecommerce/bundle.yaml` — 13 rules, 5 rulesets |
| linux-aarch64 + Windows release binaries | ⬜ | |
| Rule conflict detector (§5.1) | ⬜ | |
| `extends` base-rule inheritance | ⬜ | |

---

## Phase 3 — Visual Rule Builder _(Months 10–12)_ ⬜

| Item | Status | Notes |
|------|--------|-------|
| React + Vite + Tailwind SPA (`ui/`) | ⬜ | |
| `/rules` list view (search, tag filter, card/table) | ⬜ | |
| `/rules/new` condition + action builder | ⬜ | |
| `/rules/:id` detail, history diff, test panel | ⬜ | |
| `/rulesets` management | ⬜ | |
| `/tables` decision table view | ⬜ | |
| `/flow` call_rule + trigger dependency diagram | ⬜ | |
| `/settings` connection, API key, theme | ⬜ | |
| Live test panel (dry-run on keystroke, 300 ms debounce) | ⬜ | |
| Role-based UI control (viewer / editor / admin) | ⬜ | |
| Mutual TLS | ⬜ | |
| GitHub Actions CI (Rust + Node + Java + Python) | ⬜ | |
| OpenAPI 3.0 spec | ⬜ | |
| Docusaurus docs site | ⬜ | |

---

## Phase 4 — Community + Ecosystem _(Months 13+)_ ⬜

| Item | Status | Notes |
|------|--------|-------|
| Go binding (cgo) | ⬜ | |
| `axiom-access` module bundle | ⬜ | |
| `axiom-compliance` module bundle | ⬜ | |
| `axiom-ops` module bundle | ⬜ | |
| VS Code extension | ⬜ | |
| CNCF donation prep | ⬜ | |

---

## Requirement Coverage

| ID | Requirement | Phase | Status |
|----|-------------|-------|--------|
| RM-01 | YAML / JSON rule format (ARS) | P1 | ✅ |
| RM-02 | Load rules from filesystem / REST API | P1 | ✅ |
| RM-03 | Version rules, retain old versions | P1 | ✅ |
| RM-04 | Enable / disable without deletion | P1 | ✅ |
| RM-05 | Priority-ordered evaluation | P1 | ✅ |
| RM-09 | Schema validation with detailed errors | P1 | ✅ |
| EV-01 | Evaluate context against named rule | P1 | ✅ |
| EV-02 | Evaluate context against ruleset | P1 | ✅ |
| EV-03 | Three evaluation strategies | P1 | ✅ |
| EV-04 | Full evaluation trace | P1 | ✅ |
| EV-05 | Dry-run mode | P2 | ✅ |
| EV-06 | Per-evaluation timeout budget | P2 | ✅ |
| EV-07 | Batch evaluation (up to 1,000) | P2 | ✅ |
| SV-01 | REST CRUD for rules | P1 | ✅ |
| SV-02 | `POST /v1/evaluate` | P1 | ✅ |
| SV-03 | `POST /v1/evaluate/batch` | P2 | ✅ |
| SV-04 | Role-scoped API key auth | P1 | ✅ |
| SV-05 | REST API key management | P2 | ✅ |
| SV-06 | OpenAPI 3.0 spec | P3 | ⬜ |
| SV-07 | Rate limiting (token bucket per key) | P2 | ✅ |
| SV-08 | Trigger webhooks (HMAC, retry, dead-letter) | P2 | ✅ |
| SV-09 | Env var + YAML config | P1 | ✅ |
| SV-10 | Docker image | P1 | ✅ |
| SV-11 | HA deployment (multi-replica + PG) | P2 | ✅ |
| LB-01 | Java library | P1 | ✅ |
| LB-02 | Node.js / TypeScript library | P1 | ✅ |
| LB-03 | Python library | P2 | ✅ |
| LB-04 | Go library | P4 | ⬜ |
| LB-05 | Identical ARS format across all libraries | P1 | ✅ |
| LB-06 | Load rules from file / string / object | P1 | ✅ |
| LB-08 | Identical traces to server | P1 | ✅ |
| OB-01 | Structured evaluation trace | P1 | ✅ |
| OB-02 | Trace includes rules, conditions, timing | P1 | ✅ |
| OB-03 | Prometheus metrics | P2 | ✅ |
| OB-04 | Structured JSON logging | P2 | ✅ |
| DX-01 | Helm chart | P2 | ✅ |
| DX-02 | Finance module bundle | P2 | ✅ |
| DX-03 | E-commerce module bundle | P2 | ✅ |
| DX-07 | Docker Compose example | P1 | ✅ |
| DX-08 | Documentation site | P3 | ⬜ |
