package io.axiom;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Parameters for a single evaluation call.
 * Build via the nested {@link Builder} for a fluent API.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public class EvaluationRequest {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    @JsonProperty("rule_id")    private String ruleId;
    @JsonProperty("ruleset")    private String ruleset;
    @JsonProperty("strategy")   private String strategy = "first_match";
    @JsonProperty("dry_run")    private boolean dryRun;
    @JsonProperty("timeout_ms") private Long timeoutMs;
    @JsonProperty("context")    private Object context;

    private EvaluationRequest() {}

    String toJson() {
        try {
            return MAPPER.writeValueAsString(this);
        } catch (Exception e) {
            throw new AxiomException("Failed to serialise EvaluationRequest: " + e.getMessage(), e);
        }
    }

    // ── Builder ───────────────────────────────────────────────────────────

    public static Builder builder() { return new Builder(); }

    public static final class Builder {
        private final EvaluationRequest req = new EvaluationRequest();

        /** Target a specific rule by ID. */
        public Builder ruleId(String id) { req.ruleId = id; return this; }

        /** Target a named ruleset. */
        public Builder ruleset(String name) { req.ruleset = name; return this; }

        /** Evaluation strategy: first_match (default), all_match, scored. */
        public Builder strategy(String strategy) { req.strategy = strategy; return this; }

        /** Enable dry-run mode (disables short-circuit, returns full trace). */
        public Builder dryRun(boolean dryRun) { req.dryRun = dryRun; return this; }

        /** Wall-clock budget for this evaluation in milliseconds. */
        public Builder timeoutMs(long ms) { req.timeoutMs = ms; return this; }

        /** Context as an {@link EvaluationContext}. */
        public Builder context(EvaluationContext ctx) {
            try {
                req.context = MAPPER.readTree(ctx.toJson());
            } catch (Exception e) {
                throw new AxiomException("Invalid context JSON: " + e.getMessage(), e);
            }
            return this;
        }

        /** Context as a raw JSON string. */
        public Builder contextJson(String json) {
            try {
                req.context = MAPPER.readTree(json);
            } catch (Exception e) {
                throw new AxiomException("Invalid context JSON: " + e.getMessage(), e);
            }
            return this;
        }

        public EvaluationRequest build() {
            if (req.context == null) {
                throw new IllegalStateException("context is required");
            }
            return req;
        }
    }
}
