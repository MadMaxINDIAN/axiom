package io.axiom;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.util.List;

/**
 * Holds the result of a rule or ruleset evaluation.
 *
 * <p>Field names match the ARS JSON response schema and the Node.js binding
 * for portability across language targets.</p>
 */
public class EvaluationResult {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    @JsonProperty("matched")
    private boolean matched;

    @JsonProperty("matched_rules")
    private List<String> matchedRules;

    @JsonProperty("tags")
    private List<String> tags;

    @JsonProperty("output_context")
    private JsonNode outputContext;

    @JsonProperty("duration_us")
    private long durationMicros;

    @JsonProperty("trace")
    private EvaluationTrace trace;

    // ── Accessors ─────────────────────────────────────────────────────────

    /** Whether at least one rule matched. */
    public boolean isMatched() { return matched; }

    /** IDs of rules that matched, in evaluation order. */
    public List<String> getMatchedRules() { return matchedRules; }

    /** Tags collected from all matching rule `tag` actions. */
    public List<String> getTags() { return tags; }

    /**
     * The merged output context produced by all matching rules' `set` and
     * `increment` actions, as a Jackson {@link JsonNode}.
     */
    public JsonNode getOutputContext() { return outputContext; }

    /** Total evaluation wall-clock time in microseconds. */
    public long getDurationMicros() { return durationMicros; }

    /** Full structured evaluation trace. */
    public EvaluationTrace getTrace() { return trace; }

    // ── Factory ───────────────────────────────────────────────────────────

    /** Deserialise from the JSON string returned by the native layer. */
    static EvaluationResult fromJson(String json) {
        try {
            return MAPPER.readValue(json, EvaluationResult.class);
        } catch (Exception e) {
            throw new AxiomException("Failed to deserialise EvaluationResult: " + e.getMessage(), e);
        }
    }

    @Override
    public String toString() {
        try {
            return MAPPER.writerWithDefaultPrettyPrinter().writeValueAsString(this);
        } catch (Exception e) {
            return "EvaluationResult{matched=" + matched + "}";
        }
    }

    // ── Inner types ───────────────────────────────────────────────────────

    public static class EvaluationTrace {
        @JsonProperty("rules_evaluated")  public int rulesEvaluated;
        @JsonProperty("rules_matched")    public int rulesMatched;
        @JsonProperty("strategy")         public String strategy;
        @JsonProperty("total_duration_us")public long totalDurationUs;
        @JsonProperty("timed_out")        public boolean timedOut;
        @JsonProperty("rules")            public List<RuleTrace> rules;
    }

    public static class RuleTrace {
        @JsonProperty("rule_id")          public String ruleId;
        @JsonProperty("matched")          public boolean matched;
        @JsonProperty("conditions")       public List<ConditionTrace> conditions;
        @JsonProperty("short_circuited")  public boolean shortCircuited;
        @JsonProperty("actions_executed") public List<String> actionsExecuted;
        @JsonProperty("duration_us")      public long durationUs;
    }

    public static class ConditionTrace {
        @JsonProperty("field")        public String field;
        @JsonProperty("operator")     public String operator;
        @JsonProperty("value")        public JsonNode value;
        @JsonProperty("actual_value") public JsonNode actualValue;
        @JsonProperty("passed")       public boolean passed;
        @JsonProperty("duration_us")  public long durationUs;
    }
}
