"""
asyncio wrapper for :class:`axiom_rules.AxiomEngine`.

All blocking Rust calls are dispatched to the default executor
(thread pool) so they do not block the event loop.
"""

from __future__ import annotations

import asyncio
from concurrent.futures import ThreadPoolExecutor
from functools import partial
from typing import Any, Dict, Optional

from . import AxiomEngine as _SyncEngine

# Shared executor — Axiom evaluations are CPU-bound but short (µs-scale),
# so the default ThreadPoolExecutor sizing is fine.
_executor = ThreadPoolExecutor(thread_name_prefix="axiom-rules")


def _run_sync(fn, *args):
    """Run a synchronous callable in the thread-pool executor."""
    loop = asyncio.get_event_loop()
    return loop.run_in_executor(_executor, partial(fn, *args))


class AsyncAxiomEngine:
    """
    Asyncio-friendly wrapper around :class:`~axiom_rules.AxiomEngine`.

    Example::

        engine = AsyncAxiomEngine()
        await engine.load_rule_yaml(yaml_str)
        result = await engine.evaluate({"context": {"score": 750}})
    """

    def __init__(self) -> None:
        self._engine = _SyncEngine()

    # ── Rule loading ──────────────────────────────────────────────────────

    async def load_rule_yaml(self, yaml: str) -> None:
        """Load a rule from an ARS YAML string."""
        await _run_sync(self._engine.load_rule_yaml, yaml)

    async def load_rule_json(self, json: str) -> None:
        """Load a rule from an ARS JSON string."""
        await _run_sync(self._engine.load_rule_json, json)

    async def load_rule_file(self, path: str) -> None:
        """Load a rule from a YAML or JSON file (extension auto-detected)."""
        await _run_sync(self._engine.load_rule_file, path)

    async def load_bundle(self, path: str) -> None:
        """Load a bundle file containing ``rules:`` and optional ``rulesets:``."""
        await _run_sync(self._engine.load_bundle, path)

    # ── Evaluation ────────────────────────────────────────────────────────

    async def evaluate(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """
        Evaluate a request dict against all loaded rules.

        Returns a dict with ``matched``, ``matched_rules``, ``tags``,
        ``output_context``, ``duration_us``, ``trace``.
        """
        return await _run_sync(self._engine.evaluate, request)

    # ── Validation ────────────────────────────────────────────────────────

    def validate_rule(self, source: str) -> Optional[str]:
        """Validate ARS YAML/JSON. Returns ``None`` if valid, error string if not."""
        return self._engine.validate_rule(source)
