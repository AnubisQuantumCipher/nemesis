#!/usr/bin/env python3
"""Enforce the configured NEMESIS SPARK trusted-source budget."""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUDGET = 2_500


def count_ada_sloc(paths: Iterable[Path]) -> int:
    total = 0
    for path in paths:
        if not path.is_file():
            raise FileNotFoundError(path)
        for line in path.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped and not stripped.startswith("--"):
                total += 1
    return total


def trusted_sources() -> list[Path]:
    return sorted((ROOT / "kernel/src").glob("nemesis-kernel*.ad?"))


def main() -> int:
    sources = trusted_sources()
    if not sources:
        print("FAIL_TCB_BUDGET no trusted kernel sources found")
        return 1
    sloc = count_ada_sloc(sources)
    if sloc > BUDGET:
        print(f"FAIL_TCB_BUDGET sloc={sloc} budget={BUDGET} files={len(sources)}")
        return 1
    print(f"PASS_TCB_BUDGET sloc={sloc} budget={BUDGET} files={len(sources)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
