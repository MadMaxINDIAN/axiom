# Contributing to Axiom

Thank you for your interest in contributing! This document explains how to get
started, the development workflow, and the standards we hold contributions to.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Ways to Contribute](#ways-to-contribute)
3. [Development Setup](#development-setup)
4. [Making Changes](#making-changes)
5. [Pull Request Process](#pull-request-process)
6. [Coding Standards](#coding-standards)
7. [Testing](#testing)
8. [Documentation](#documentation)
9. [Release Process](#release-process)

---

## Code of Conduct

By participating you agree to abide by our
[Code of Conduct](CODE_OF_CONDUCT.md). Please report unacceptable behaviour
to `conduct@axiom-rules.io`.

---

## Ways to Contribute

- **Bug reports** — Open a [GitHub issue](https://github.com/axiom-rules/axiom/issues/new?template=bug_report.md).
- **Feature requests** — Open a [GitHub issue](https://github.com/axiom-rules/axiom/issues/new?template=feature_request.md).
- **Documentation** — Fix typos, improve examples, add tutorials.
- **Code** — Fix bugs, implement features, add tests.
- **Module bundles** — Contribute domain-specific rule bundles under `modules/`.
- **Language bindings** — Add support for a new language under `bindings/`.

---

## Development Setup

### Prerequisites

| Tool     | Version   |
|----------|-----------|
| Rust     | ≥ 1.78 (stable) |
| Go       | ≥ 1.22 |
| Node.js  | ≥ 20 LTS |
| Python   | ≥ 3.9 |
| Docker   | ≥ 24 (for integration tests) |

### Clone and build

```bash
git clone https://github.com/axiom-rules/axiom.git
cd axiom

# Build all Rust crates
cargo build

# Run the server locally (SQLite mode)
cargo run -p axiom-server

# Build the CLI
cargo build -p axiom-cli
```

### Running the test suite

```bash
# Rust unit + integration tests
cargo test

# Go binding tests (requires the built .so/.dll)
cd bindings/go && go test ./...

# Python binding tests (requires maturin dev install)
cd bindings/python && maturin develop && python -m pytest

# Node.js binding tests
cd bindings/node && npm test

# UI tests
cd ui && npm test
```

---

## Making Changes

1. **Fork** the repository and create a branch from `main`:

   ```bash
   git checkout -b feat/my-feature
   ```

2. **Keep branches focused** — one logical change per PR. If you are fixing a bug
   and cleaning up unrelated code, split into two PRs.

3. **Commit messages** follow [Conventional Commits](https://www.conventionalcommits.org/):

   ```
   <type>(<scope>): <short description>

   [optional body]

   [optional footer]
   ```

   Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`, `ci`.
   Scopes: `core`, `server`, `cli`, `go`, `python`, `node`, `java`, `ui`, `helm`, `docs`.

   Example:
   ```
   feat(core): add 'between' operator for numeric range checks
   ```

4. **Keep the diff small** — reviewers can only give quality feedback on focused
   changes. Prefer multiple small PRs to one large one.

---

## Pull Request Process

1. Ensure all tests pass: `cargo test && cargo clippy -- -D warnings`.
2. Update documentation if you change public APIs or CLI flags.
3. Add a changelog entry in `CHANGELOG.md` under `[Unreleased]`.
4. Open the PR against `main`. Fill in the PR template.
5. At least **one Maintainer approval** is required to merge.
6. Address review comments promptly. PRs inactive for 30 days may be closed.

### Draft PRs

Open a Draft PR early to share work-in-progress and get early feedback. Switch
to "Ready for Review" when you believe it is complete.

---

## Coding Standards

### Rust

- Follow `rustfmt` defaults (`cargo fmt`).
- Pass `cargo clippy -- -D warnings` with no warnings.
- Prefer `thiserror` for library error types; `anyhow` for binaries.
- All public items must have `///` doc comments.
- Unsafe code must include a `// SAFETY:` comment explaining the invariants.

### Go

- Run `gofmt` and `go vet`.
- Follow the [Effective Go](https://go.dev/doc/effective_go) style guide.

### TypeScript / JavaScript

- ESLint + Prettier configuration provided in `ui/.eslintrc.cjs`.
- No `any` types without a `// eslint-disable` comment and explanation.

### YAML (ARS rules)

- Use 2-space indentation.
- Include `description:` on all rules and rulesets.
- Tag all rules with at least one tag.
- Run `axiom validate <file>` before committing.

---

## Testing

### Unit tests

Every new function should have a corresponding unit test in the same file or a
sibling `_test` file.

### Integration tests

Server-level tests live in `crates/axiom-server/tests/`. They spin up a real
SQLite-backed server. Use `#[tokio::test]` and the `reqwest` HTTP client.

### Test fixtures

ARS YAML fixtures live in `tests/fixtures/`. New operators or schema changes
require corresponding fixture files.

---

## Documentation

- API reference is generated from Rust doc comments via `cargo doc`.
- User-facing docs live in `docs/` (Markdown, rendered with MkDocs).
- Run `mkdocs serve` to preview locally.
- All public HTTP endpoints must be documented in `docs/api-reference.md`.

---

## Release Process

Releases are managed by Maintainers:

1. Update version in `Cargo.toml` workspace `[package]` section.
2. Update `CHANGELOG.md` — move `[Unreleased]` to the new version with the date.
3. Open a release PR; merge after review.
4. Tag: `git tag v0.x.y && git push origin v0.x.y`.
5. CI publishes crates to crates.io, npm, PyPI, and GitHub Releases automatically.

---

## Questions?

Open a [GitHub Discussion](https://github.com/axiom-rules/axiom/discussions) or
join `#axiom` on the CNCF Slack (`cloud-native.slack.com`).
