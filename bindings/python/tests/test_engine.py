"""
pytest test suite for axiom-rules Python binding.

Run after `maturin develop` has been executed::

    cd bindings/python
    maturin develop
    pytest tests/
"""

import json
import textwrap
import pytest

# Import after maturin develop installs the extension module.
from axiom_rules import AxiomEngine, validate_rule

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

LOAN_RULE_YAML = textwrap.dedent("""\
    ars_version: 1
    id: loan-eligibility
    name: Loan Eligibility
    version: 1
    priority: 10
    enabled: true
    conditions:
      all:
        - field: annual_income
          op: gte
          value: 50000
        - field: credit_score
          op: gte
          value: 650
    actions:
      - type: set
        field: approved
        value: true
      - type: tag
        value: approved
""")

FRAUD_RULE_JSON = json.dumps({
    "ars_version": 1,
    "id": "fraud-flag",
    "name": "Fraud Flag",
    "version": 1,
    "priority": 20,
    "enabled": True,
    "conditions": {
        "all": [
            {"field": "transaction_amount", "op": "gt", "value": 10000},
            {"field": "country", "op": "neq", "value": "US"},
        ]
    },
    "actions": [
        {"type": "tag", "value": "fraud"},
        {"type": "set", "field": "flagged", "value": True},
    ],
})


@pytest.fixture
def engine():
    return AxiomEngine()


@pytest.fixture
def loan_engine():
    e = AxiomEngine()
    e.load_rule_yaml(LOAN_RULE_YAML)
    return e


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

class TestLoadRuleYaml:
    def test_loads_valid_yaml(self, engine):
        engine.load_rule_yaml(LOAN_RULE_YAML)  # should not raise

    def test_raises_on_invalid_yaml(self, engine):
        with pytest.raises(ValueError, match="ars_version"):
            engine.load_rule_yaml("not: a: valid: rule")


class TestLoadRuleJson:
    def test_loads_valid_json(self, engine):
        engine.load_rule_json(FRAUD_RULE_JSON)  # should not raise

    def test_raises_on_invalid_json(self, engine):
        with pytest.raises(ValueError):
            engine.load_rule_json('{"id": "missing-version"}')


class TestEvaluate:
    def test_match(self, loan_engine):
        result = loan_engine.evaluate({
            "context": {"annual_income": 75000, "credit_score": 720},
            "strategy": "all_match",
        })
        assert result["matched"] is True
        assert "loan-eligibility" in result["matched_rules"]
        assert "approved" in result["tags"]
        assert result["output_context"]["approved"] is True

    def test_no_match(self, loan_engine):
        result = loan_engine.evaluate({
            "context": {"annual_income": 20000, "credit_score": 720},
        })
        assert result["matched"] is False
        assert result["matched_rules"] == []

    def test_output_context_set(self, loan_engine):
        result = loan_engine.evaluate({
            "context": {"annual_income": 80000, "credit_score": 700},
        })
        assert result["output_context"].get("approved") is True

    def test_duration_us_present(self, loan_engine):
        result = loan_engine.evaluate({"context": {}})
        assert "duration_us" in result
        assert isinstance(result["duration_us"], int)

    def test_trace_present(self, loan_engine):
        result = loan_engine.evaluate({"context": {"annual_income": 60000, "credit_score": 700}})
        trace = result["trace"]
        assert "rules_evaluated" in trace
        assert trace["rules_evaluated"] >= 1

    def test_dry_run_does_not_apply_actions(self, loan_engine):
        result = loan_engine.evaluate({
            "context": {"annual_income": 80000, "credit_score": 750},
            "dry_run": True,
            "strategy": "all_match",
        })
        # In dry_run, evaluation still reports matches, but side-effects are skipped.
        assert "trace" in result

    def test_two_rules_all_match(self, engine):
        engine.load_rule_yaml(LOAN_RULE_YAML)
        engine.load_rule_json(FRAUD_RULE_JSON)
        result = engine.evaluate({
            "context": {
                "annual_income": 60000,
                "credit_score": 700,
                "transaction_amount": 15000,
                "country": "RU",
            },
            "strategy": "all_match",
        })
        assert set(result["matched_rules"]) == {"loan-eligibility", "fraud-flag"}


class TestValidateRule:
    def test_valid_yaml_returns_none(self):
        assert validate_rule(LOAN_RULE_YAML) is None

    def test_invalid_returns_error_string(self):
        err = validate_rule("ars_version: 1\nid: nope")  # missing required fields
        assert err is not None
        assert isinstance(err, str)

    def test_instance_method_matches_module_function(self, engine):
        assert engine.validate_rule(LOAN_RULE_YAML) == validate_rule(LOAN_RULE_YAML)
