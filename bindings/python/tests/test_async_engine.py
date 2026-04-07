"""
pytest-asyncio tests for axiom_rules.aio.AsyncAxiomEngine.
"""

import textwrap
import pytest
import pytest_asyncio

from axiom_rules.aio import AsyncAxiomEngine

pytestmark = pytest.mark.asyncio

RULE_YAML = textwrap.dedent("""\
    ars_version: 1
    id: credit-check
    name: Credit Check
    version: 1
    priority: 5
    enabled: true
    conditions:
      all:
        - field: credit_score
          op: gte
          value: 700
    actions:
      - type: tag
        value: good-credit
      - type: set
        field: credit_ok
        value: true
""")


@pytest_asyncio.fixture
async def engine():
    e = AsyncAxiomEngine()
    await e.load_rule_yaml(RULE_YAML)
    return e


async def test_async_load_and_evaluate(engine):
    result = await engine.evaluate({"context": {"credit_score": 750}})
    assert result["matched"] is True
    assert "good-credit" in result["tags"]


async def test_async_no_match(engine):
    result = await engine.evaluate({"context": {"credit_score": 500}})
    assert result["matched"] is False


async def test_async_output_context(engine):
    result = await engine.evaluate({"context": {"credit_score": 800}})
    assert result["output_context"].get("credit_ok") is True


async def test_async_validate_rule_sync(engine):
    # validate_rule on AsyncAxiomEngine is synchronous (no I/O needed)
    assert engine.validate_rule(RULE_YAML) is None


async def test_concurrent_evaluations(engine):
    import asyncio
    requests = [{"context": {"credit_score": 700 + i * 10}} for i in range(20)]
    results = await asyncio.gather(*[engine.evaluate(r) for r in requests])
    assert all(r["matched"] for r in results)
