"""
axiom-rules — Python binding for the Axiom rules engine.

Synchronous API::

    from axiom_rules import AxiomEngine

    engine = AxiomEngine()
    engine.load_rule_yaml(open("loan.yaml").read())
    result = engine.evaluate({"context": {"annual_income": 60_000}})

Async API::

    from axiom_rules.aio import AsyncAxiomEngine

    engine = AsyncAxiomEngine()
    await engine.load_rule_yaml(open("loan.yaml").read())
    result = await engine.evaluate({"context": {"annual_income": 60_000}})
"""

from ._axiom_rules import AxiomEngine, validate_rule

__all__ = ["AxiomEngine", "validate_rule"]
