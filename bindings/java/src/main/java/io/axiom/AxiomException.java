package io.axiom;

/**
 * Thrown by {@link AxiomEngine} when the native engine returns an error.
 * Wraps structured error messages from the Rust core with a message and
 * an optional field path indicating where validation failed.
 */
public class AxiomException extends RuntimeException {

    private final String fieldPath;

    public AxiomException(String message) {
        super(message);
        this.fieldPath = null;
    }

    public AxiomException(String message, String fieldPath) {
        super(message);
        this.fieldPath = fieldPath;
    }

    public AxiomException(String message, Throwable cause) {
        super(message, cause);
        this.fieldPath = null;
    }

    /**
     * Field path where a schema validation error occurred, or {@code null}
     * if the error is not associated with a specific field.
     */
    public String getFieldPath() {
        return fieldPath;
    }

    @Override
    public String toString() {
        if (fieldPath != null) {
            return "AxiomException at '" + fieldPath + "': " + getMessage();
        }
        return "AxiomException: " + getMessage();
    }
}
