# Axiom — Build Progress

> Architecture reference: `docs/axiom-architecture-v1.3.md`

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

## Phase 1 — Core Engine + REST Server _(Months 1–5)_

### Rust Core (`crates/axiom-core`)

| Component | Status | Notes |
|-----------|--------|-------|
| ARS schema structs (`Rule`, `ConditionGroup`, `Action`, …) | ✅ | `src/schema.rs` |
| YAML / JSON parser + schema validation | ✅ | `src/parser.rs` |
| Bundle import parser | ✅ | `parse_bundle_yaml` in `src/parser.rs` |
| In-memory rule registry | ✅ | `src/registry.rs` |
| Dot-notation + array-index resolver | ✅ | `src/resolver.rs` |
| Condition tree evaluator (short-circuit, dry-run) | ✅ | `src/evaluator.rs` |
| All 30+ ARS operators | ✅ | `src/operators.rs` |
| Action executor (set, increment, append, tag, log, return) | ✅ | `src/evaluator.rs` |
| Sandboxed `{{ expr }}` template engine (depth limit 16) | ✅ | `src/expression.rs` |
| `call_rule_guard` (cycle detect, missing-rule, depth limit 4) | ✅ | `src/call_rule_guard.rs` |
| Evaluation trace (`EvaluationTrace`, `RuleTrace`, `ConditionTrace`) | ✅ | `src/trace.rs` |
| Three evaluation strategies (FirstMatch, AllMatch, Scored §5.4) | ✅ | `src/evaluator.rs` |
| Timeout budget | ✅ | `src/timeout.rs` |
| Unit tests (24 passing) | ✅ | inline `#[cfg(test)]` |

### REST Server (`crates/axiom-server`)

| Endpoint | Status | Notes |
|----------|--------|-------|
| `GET /health` | ✅ | |
| `GET /ready` | ✅ | 503 when storage unreachable |
| `GET /metrics` | ✅ | Prometheus text format |
| `GET /v1/rules` | ✅ | tag / enabled filters |
| `POST /v1/rules` | ✅ | |
| `GET /v1/rules/:id` | ✅ | |
| `GET /v1/rules/:id/versions` | ✅ | |
| `PUT /v1/rules/:id` | ✅ | auto-increments version |
| `PATCH /v1/rules/:id` | ✅ | partial update |
| `DELETE /v1/rules/:id` | ✅ | soft-delete |
| `GET /v1/rulesets` | ✅ | |
| `POST /v1/rulesets` | ✅ | |
| `GET /v1/rulesets/:name` | ✅ | |
| `PUT /v1/rulesets/:name` | ✅ | |
| `POST /v1/evaluate` | ✅ | |
| `POST /v1/evaluate/batch` | ✅ | max 1,000 |
| `GET /v1/keys` | ✅ | admin only; read-only in Phase 1 |
| `POST /v1/keys` | ✅ | [P2] REST key management |
| `DELETE /v1/keys/:id` | ✅ | [P2] |
| `POST /v1/import` | ✅ | |
| `GET /v1/export` | ✅ | |
| SQLite storage backend | ✅ | `src/storage/sqlite.rs` |
| PostgreSQL storage backend | ✅ | `src/storage/postgres.rs` |
| API key auth (`X-Axiom-Key`, SHA-256, roles) | ✅ | `src/auth.rs` |
| Config-file keys (`axiom.yaml` / `AXIOM_API_KEY` env) | ✅ | `src/config.rs` |
| Background rule-poll loop (default 10 s) | ✅ | `src/main.rs` |
| Storage failover (serve from cache on outage) | ✅ | poll loop + ready probe |
| Rate limiting | ✅ | [P2] token bucket per key — `src/rate_limit.rs` |
| Mutual TLS | ⬜ | [P2] |

### CLI (`crates/axiom-cli`)

| Command | Status | Notes |
|---------|--------|-------|
| `axiom validate <path>` | ✅ | |
| `axiom test <path>` | ✅ | JUnit XML via `--output` |
| `axiom evaluate --rule <path> --context <json>` | ✅ | local |
| `axiom evaluate --server … --rule-id … --context …` | ✅ | remote |
| `axiom import <bundle> --server …` | ✅ | |
| `axiom export --server …` | ✅ | |
| `axiom serve --rules <path>` | ✅ | stub; delegates to server binary |
| `axiom keygen --role …` | ✅ | prints key + `sha256:` hash |

### Language Bindings

| Binding | Status | Notes |
|---------|--------|-------|
| Node.js / TypeScript (NAPI-RS) | ✅ | `bindings/node/` — Rust glue, `index.js`, `index.d.ts`, tests |
| Java (JNI) | ✅ | `bindings/java/` — Rust JNI glue, Java wrappers, Maven pom, tests |
| Python (PyO3 / maturin) | ✅ | `bindings/python/` — Rust PyO3 glue, asyncio wrapper, pytest tests |
| Go (cgo) | ⬜ | [P4] |

### Infrastructure

| Item | Status | Notes |
|------|--------|-------|
| Cargo workspace | ✅ | `Cargo.toml` |
| ARS JSON Schema (`schema/ars-v1.json`) | ✅ | JSON Schema 2020-12 |
| Dockerfile (multi-stage) | ✅ | |
| `docker-compose.yml` (single-instance) | ✅ | |
| `docker-compose.ha.yml` (HA + PG) | ✅ | `deploy/docker-compose.ha.yml` + `deploy/nginx.conf` |
| Helm chart (`deploy/helm/`) | ✅ | [P2] |
| GitHub Actions CI | ⬜ | |
| OpenAPI 3.0 spec | ⬜ | |
| Docusaurus docs site (`docs/`) | ⬜ | |

---

## Phase 2 — Developer Tooling + Testing _(Months 6–9)_

| Item | Status | Notes |
|------|--------|-------|
| Python binding (PyO3 / maturin) | ✅ | `bindings/python/` |
| Hot-reload / filesystem watch | ✅ | `notify` crate, `src/watch.rs`, `rules_dir` config |
| Batch evaluation worker pool (2× CPU) | ✅ | Sequential under single read lock (µs-scale evals) |
| Dry-run mode (disable short-circuit) | ✅ | `EvalConfig.dry_run` |
| Timeout enforcement (per-eval budget) | ✅ | `src/timeout.rs` |
| `trigger` webhook (3× exp backoff, dead-letter) | ✅ | `src/webhook.rs` |
| REST API key management (`POST/DELETE /v1/keys`) | ✅ | `src/routes/keys.rs` |
| `call_rule` depth raised to 8 | ✅ | `Registry::max_call_depth = 8` |
| Rate limiting (token bucket per key) | ✅ | `src/rate_limit.rs` |
| Prometheus metrics (eval throughput, latency histogram) | ✅ | `src/metrics.rs` |
| Structured JSON logging (tracing crate) | ✅ | `tracing-subscriber` JSON format |
| Helm chart | ✅ | `deploy/helm/axiom/` — Deployment, Service, Ingress, ConfigMap, PVC, HPA, ServiceMonitor |
| `axiom-finance` module bundle | ✅ | `modules/axiom-finance/bundle.yaml` — 12 rules, 2 rulesets |
| `axiom-ecommerce` module bundle | ✅ | `modules/axiom-ecommerce/bundle.yaml` — 13 rules, 5 rulesets |
| linux-aarch64 + Windows release binaries | ⬜ | |
| Rule conflict detector (§5.1 [P2]) | ⬜ | |
| `extends` base-rule inheritance | ⬜ | |

---

## Phase 3 — Visual Rule Builder _(Months 10–12)_

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
| Role-based control hiding (viewer/editor/admin) | ⬜ | |
| REST API key management endpoints | ⬜ | |
| Mutual TLS | ⬜ | |

---

## Phase 4 — Community + Ecosystem _(Months 13+)_

| Item | Status | Notes |
|------|--------|-------|
| Go binding (cgo) | ⬜ | |
| `axiom-access` module bundle | ⬜ | |
| `axiom-compliance` module bundle | ⬜ | |
| `axiom-ops` module bundle | ⬜ | |
| VS Code extension | ⬜ | |
| CNCF donation prep | ⬜ | |

---

## Requirement Coverage (Phase 1 P0s)

| ID | Requirement | Status |
|----|-------------|--------|
| RM-01 | YAML / JSON rule format (ARS) | ✅ |
| RM-02 | Load rules from filesystem / REST API | ✅ |
| RM-03 | Version rules, retain old versions | ✅ |
| RM-04 | Enable / disable without deletion | ✅ |
| RM-05 | Priority-ordered evaluation | ✅ |
| RM-09 | Schema validation with detailed errors | ✅ |
| EV-01 | Evaluate context against named rule | ✅ |
| EV-02 | Evaluate context against ruleset | ✅ |
| EV-03 | Three evaluation strategies | ✅ |
| EV-04 | Full evaluation trace | ✅ |
| SV-01 | REST CRUD for rules | ✅ |
| SV-02 | `POST /v1/evaluate` | ✅ |
| SV-04 | Role-scoped API key auth (Phase 1 config-file) | ✅ |
| SV-06 | OpenAPI 3.0 spec | ⬜ |
| SV-09 | Env var + YAML config | ✅ |
| SV-10 | Docker image | ✅ |
| LB-01 | Java library | ✅ |
| LB-02 | Node.js / TypeScript library | ✅ |
| LB-05 | Identical ARS format across all libraries | ✅ |
| LB-06 | Load rules from file / string / URL / object | ✅ |
| LB-08 | Identical traces to server | ✅ |
| OB-01 | Structured evaluation trace | ✅ |
| OB-02 | Trace includes rules, conditions, timing | ✅ |
| DX-07 | Docker Compose example | ✅ |
| DX-08 | Documentation site | ⬜ |
