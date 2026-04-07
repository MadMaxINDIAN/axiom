package io.axiom;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class AxiomEngineTest {

    private static final String LOAN_RULE_YAML = """
        ars_version: 1
        id: loan-eligibility-check
        name: Loan Eligibility Check
        version: 1
        priority: 10
        enabled: true
        tags: [lending]
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
        """;

    private AxiomEngine engine;

    @BeforeEach
    void setUp() {
        engine = new AxiomEngine();
        engine.loadRuleYaml(LOAN_RULE_YAML);
    }

    @Test
    void evaluateRule_match() {
        EvaluationContext ctx = EvaluationContext.fromMap(Map.of(
            "applicant", Map.of(
                "credit_score", 720,
                "annual_income", 60000,
                "existing_debt_ratio", 0.2
            )
        ));

        EvaluationResult result = engine.evaluateRule("loan-eligibility-check", ctx);

        assertTrue(result.isMatched());
        assertEquals(1, result.getMatchedRules().size());
        assertEquals("loan-eligibility-check", result.getMatchedRules().get(0));
        assertTrue(result.getTags().contains("standard-loan-approved"));
        assertTrue(result.getOutputContext().get("result").get("eligible").asBoolean());
        assertEquals(180000.0,
            result.getOutputContext().get("result").get("max_loan_amount").asDouble(), 0.01);
    }

    @Test
    void evaluateRule_noMatch_lowCreditScore() {
        EvaluationContext ctx = EvaluationContext.fromMap(Map.of(
            "applicant", Map.of(
                "credit_score", 580,
                "annual_income", 60000,
                "existing_debt_ratio", 0.2
            )
        ));

        EvaluationResult result = engine.evaluateRule("loan-eligibility-check", ctx);

        assertFalse(result.isMatched());
        assertTrue(result.getMatchedRules().isEmpty());
    }

    @Test
    void evaluateRule_noMatch_highDebtRatio() {
        EvaluationContext ctx = EvaluationContext.fromMap(Map.of(
            "applicant", Map.of(
                "credit_score", 720,
                "annual_income", 60000,
                "existing_debt_ratio", 0.55
            )
        ));

        EvaluationResult result = engine.evaluateRule("loan-eligibility-check", ctx);

        assertFalse(result.isMatched());
    }

    @Test
    void traceContainsConditionDetails() {
        EvaluationContext ctx = EvaluationContext.fromMap(Map.of(
            "applicant", Map.of(
                "credit_score", 720,
                "annual_income", 60000,
                "existing_debt_ratio", 0.2
            )
        ));

        EvaluationResult result = engine.evaluateRule("loan-eligibility-check", ctx);

        assertFalse(result.getTrace().rules.isEmpty());
        EvaluationResult.RuleTrace ruleTrace = result.getTrace().rules.get(0);
        assertEquals("loan-eligibility-check", ruleTrace.ruleId);
        assertTrue(ruleTrace.matched);
        assertFalse(ruleTrace.conditions.isEmpty());
        assertTrue(ruleTrace.conditions.get(0).passed);
    }

    @Test
    void validateRule_valid() {
        String error = AxiomEngine.validateRule(LOAN_RULE_YAML);
        assertNull(error, "Expected no validation error but got: " + error);
    }

    @Test
    void validateRule_wrongArsVersion() {
        String bad = LOAN_RULE_YAML.replace("ars_version: 1", "ars_version: 99");
        String error = AxiomEngine.validateRule(bad);
        assertNotNull(error);
        assertTrue(error.contains("99"));
    }

    @Test
    void loadRuleJson_works() {
        String ruleJson = """
            {
              "ars_version": 1,
              "id": "json-rule",
              "name": "JSON Rule",
              "version": 1,
              "enabled": true,
              "conditions": { "all": [{"field": "x", "operator": "eq", "value": 1}] },
              "actions": [{"type": "tag", "value": "hit"}]
            }
            """;

        try (AxiomEngine eng = new AxiomEngine()) {
            eng.loadRuleJson(ruleJson);
            EvaluationResult result = eng.evaluateRule(
                "json-rule",
                EvaluationContext.fromJson("{\"x\": 1}")
            );
            assertTrue(result.isMatched());
            assertTrue(result.getTags().contains("hit"));
        }
    }

    @Test
    void autoClose_doesNotThrow() {
        assertDoesNotThrow(() -> {
            try (AxiomEngine eng = new AxiomEngine()) {
                eng.loadRuleYaml(LOAN_RULE_YAML);
            }
        });
    }

    @Test
    void usageAfterClose_throws() {
        AxiomEngine eng = new AxiomEngine();
        eng.close();
        assertThrows(IllegalStateException.class,
            () -> eng.loadRuleYaml(LOAN_RULE_YAML));
    }

    @Test
    void fullRequestBuilder() {
        EvaluationRequest req = EvaluationRequest.builder()
            .ruleId("loan-eligibility-check")
            .strategy("first_match")
            .timeoutMs(100)
            .context(EvaluationContext.fromMap(Map.of(
                "applicant", Map.of(
                    "credit_score", 720,
                    "annual_income", 60000,
                    "existing_debt_ratio", 0.2
                )
            )))
            .build();

        EvaluationResult result = engine.evaluate(req);
        assertTrue(result.isMatched());
    }
}
