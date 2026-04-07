package io.axiom;

import java.io.File;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

/**
 * Main entry point for the Axiom rules engine.
 *
 * <p>Thread-safe: share a single instance across threads. The underlying
 * Rust registry uses a {@code RwLock} — concurrent evaluations are fully
 * parallel; rule loading serialises briefly.</p>
 *
 * <pre>{@code
 * AxiomEngine engine = new AxiomEngine();
 * engine.loadRuleYaml(ruleYaml);
 *
 * EvaluationResult result = engine.evaluate(
 *     EvaluationRequest.builder()
 *         .ruleId("loan-eligibility-check")
 *         .context(EvaluationContext.fromMap(Map.of(
 *             "applicant", Map.of("credit_score", 720, "annual_income", 60000)
 *         )))
 *         .build()
 * );
 * }</pre>
 */
public class AxiomEngine implements AutoCloseable {

    // ── Native library loading ────────────────────────────────────────────

    static {
        loadNativeLibrary();
    }

    private static void loadNativeLibrary() {
        // 1. Try java.library.path first (set by user or Maven/Gradle)
        try {
            System.loadLibrary("axiom_java");
            return;
        } catch (UnsatisfiedLinkError ignored) {}

        // 2. Fall back to extracting from JAR resources
        String libName = nativeLibraryName();
        try (InputStream in = AxiomEngine.class.getResourceAsStream("/native/" + libName)) {
            if (in == null) {
                throw new UnsatisfiedLinkError(
                    "Native library '" + libName + "' not found in JAR. " +
                    "Set -Djava.library.path or add the correct native classifier JAR."
                );
            }
            Path temp = Files.createTempFile("axiom-java-", libName);
            temp.toFile().deleteOnExit();
            Files.copy(in, temp, StandardCopyOption.REPLACE_EXISTING);
            System.load(temp.toAbsolutePath().toString());
        } catch (java.io.IOException e) {
            throw new UnsatisfiedLinkError("Failed to extract native library: " + e.getMessage());
        }
    }

    private static String nativeLibraryName() {
        String os   = System.getProperty("os.name", "").toLowerCase();
        String arch = System.getProperty("os.arch", "").toLowerCase();
        String prefix = os.contains("win") ? "" : "lib";
        String suffix = os.contains("win") ? ".dll" : os.contains("mac") ? ".dylib" : ".so";
        return prefix + "axiom_java" + suffix;
    }

    // ── Native method declarations ─────────────────────────────────────────

    private native long   nativeCreate();
    private native void   nativeDestroy(long handle);
    private native void   nativeLoadRuleYaml(long handle, String yaml);
    private native void   nativeLoadRuleJson(long handle, String json);
    private native void   nativeLoadRuleFile(long handle, String path);
    private native void   nativeLoadBundle(long handle, String path);
    private native String nativeEvaluate(long handle, String requestJson);
    private static native String nativeValidate(String source, boolean isJson);

    // ── Instance state ────────────────────────────────────────────────────

    private final long handle;
    private volatile boolean closed = false;

    /** Create a new, empty engine instance. */
    public AxiomEngine() {
        this.handle = nativeCreate();
    }

    // ── Rule loading ──────────────────────────────────────────────────────

    /**
     * Load a rule from an ARS YAML string.
     * @throws AxiomException if the rule fails schema validation
     */
    public AxiomEngine loadRuleYaml(String yaml) {
        checkOpen();
        nativeLoadRuleYaml(handle, yaml);
        return this;
    }

    /**
     * Load a rule from an ARS JSON string.
     * @throws AxiomException if the rule fails schema validation
     */
    public AxiomEngine loadRuleJson(String json) {
        checkOpen();
        nativeLoadRuleJson(handle, json);
        return this;
    }

    /**
     * Load a rule from a file path. Detects YAML/JSON by extension.
     * @throws AxiomException if the file cannot be read or fails validation
     */
    public AxiomEngine loadRuleFile(File file) {
        checkOpen();
        nativeLoadRuleFile(handle, file.getAbsolutePath());
        return this;
    }

    /** @see #loadRuleFile(File) */
    public AxiomEngine loadRuleFile(Path path) {
        return loadRuleFile(path.toFile());
    }

    /**
     * Load a bundle YAML file containing multiple rules and/or rulesets.
     * @throws AxiomException if parsing or validation fails
     */
    public AxiomEngine loadBundle(File file) {
        checkOpen();
        nativeLoadBundle(handle, file.getAbsolutePath());
        return this;
    }

    /** @see #loadBundle(File) */
    public AxiomEngine loadBundle(Path path) {
        return loadBundle(path.toFile());
    }

    // ── Evaluation ────────────────────────────────────────────────────────

    /**
     * Evaluate an {@link EvaluationRequest} and return the result.
     * Thread-safe — multiple threads may call this concurrently.
     *
     * @throws AxiomException if the rule/ruleset is not found or evaluation fails
     */
    public EvaluationResult evaluate(EvaluationRequest request) {
        checkOpen();
        String responseJson = nativeEvaluate(handle, request.toJson());
        return EvaluationResult.fromJson(responseJson);
    }

    /**
     * Shorthand: evaluate context against a single rule by ID using
     * first-match strategy.
     */
    public EvaluationResult evaluateRule(String ruleId, EvaluationContext context) {
        return evaluate(
            EvaluationRequest.builder()
                .ruleId(ruleId)
                .context(context)
                .build()
        );
    }

    /**
     * Shorthand: evaluate context against a named ruleset using all-match strategy.
     */
    public EvaluationResult evaluateRuleset(String rulesetName, EvaluationContext context) {
        return evaluate(
            EvaluationRequest.builder()
                .ruleset(rulesetName)
                .strategy("all_match")
                .context(context)
                .build()
        );
    }

    // ── Validation ────────────────────────────────────────────────────────

    /**
     * Validate an ARS YAML or JSON string without loading it into the registry.
     * @return {@code null} if valid; otherwise an error message string
     */
    public static String validateRule(String source) {
        return nativeValidate(source, false);
    }

    /**
     * Validate an ARS JSON string.
     * @return {@code null} if valid; otherwise an error message string
     */
    public static String validateRuleJson(String json) {
        return nativeValidate(json, true);
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────

    /** Release the native registry. Safe to call multiple times. */
    @Override
    public synchronized void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    private void checkOpen() {
        if (closed) throw new IllegalStateException("AxiomEngine has been closed");
    }
}
