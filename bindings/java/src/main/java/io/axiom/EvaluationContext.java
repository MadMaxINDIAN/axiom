package io.axiom;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.util.Map;

/**
 * Wraps the JSON context passed to {@link AxiomEngine#evaluate}.
 * Accepts a {@link Map}, a raw JSON string, or any Jackson-serialisable object.
 */
public class EvaluationContext {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final String json;

    private EvaluationContext(String json) {
        this.json = json;
    }

    /** Wrap a raw JSON string directly. */
    public static EvaluationContext fromJson(String json) {
        return new EvaluationContext(json);
    }

    /** Wrap a {@code Map<String, Object>} (field names → values). */
    public static EvaluationContext fromMap(Map<String, Object> map) {
        try {
            return new EvaluationContext(MAPPER.writeValueAsString(map));
        } catch (JsonProcessingException e) {
            throw new AxiomException("Failed to serialise context map: " + e.getMessage(), e);
        }
    }

    /** Wrap any Jackson-serialisable POJO. */
    public static EvaluationContext fromObject(Object obj) {
        try {
            return new EvaluationContext(MAPPER.writeValueAsString(obj));
        } catch (JsonProcessingException e) {
            throw new AxiomException("Failed to serialise context object: " + e.getMessage(), e);
        }
    }

    /** Returns the context as a JSON string (used by the native layer). */
    public String toJson() {
        return json;
    }

    @Override
    public String toString() {
        return json;
    }
}
