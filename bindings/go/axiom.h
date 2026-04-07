/* axiom.h — C header for the Axiom Go cgo binding.
 *
 * Build the Rust library first:
 *   cargo build --release -p axiom-go-sys
 *
 * Then link with:
 *   #cgo LDFLAGS: -L${SRCDIR}/lib -laxiom_go -ldl -lpthread -lm
 */

#ifndef AXIOM_H
#define AXIOM_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque engine handle. */
typedef struct AxiomHandle AxiomHandle;

/* ── Lifecycle ─────────────────────────────────────────────────────────── */

/** Create a new, empty engine. Must be freed with axiom_destroy(). */
AxiomHandle *axiom_create(void);

/** Destroy an engine handle. */
void axiom_destroy(AxiomHandle *handle);

/** Free a string returned by any axiom_* function. */
void axiom_free_string(char *ptr);

/* ── Rule loading ──────────────────────────────────────────────────────── */

/**
 * Load a rule from a YAML string.
 * Returns NULL on success, or a heap-allocated error string on failure.
 * The caller must free the returned string with axiom_free_string().
 */
char *axiom_load_rule_yaml(AxiomHandle *handle, const char *yaml);

/**
 * Load a rule from a JSON string.
 * Returns NULL on success, or a heap-allocated error string on failure.
 */
char *axiom_load_rule_json(AxiomHandle *handle, const char *json);

/**
 * Load a rule from a file path (YAML or JSON, auto-detected by extension).
 * Returns NULL on success, or a heap-allocated error string on failure.
 */
char *axiom_load_rule_file(AxiomHandle *handle, const char *path);

/**
 * Load a bundle file (YAML with `rules:` and optional `rulesets:` keys).
 * Returns NULL on success, or a heap-allocated error string on failure.
 */
char *axiom_load_bundle(AxiomHandle *handle, const char *path);

/* ── Evaluation ────────────────────────────────────────────────────────── */

/**
 * Evaluate a JSON-encoded EvaluationRequest.
 * Returns a heap-allocated JSON string containing the EvaluationResponse,
 * or a JSON error object {"error":"..."} on failure.
 * The caller must free the returned string with axiom_free_string().
 */
char *axiom_evaluate(AxiomHandle *handle, const char *request_json);

/* ── Validation ────────────────────────────────────────────────────────── */

/**
 * Validate ARS YAML or JSON without loading it.
 * Returns NULL if the source is valid.
 * Returns a heap-allocated error message string if invalid.
 * The caller must free non-NULL results with axiom_free_string().
 */
char *axiom_validate_rule(const char *source);

#ifdef __cplusplus
}
#endif

#endif /* AXIOM_H */
