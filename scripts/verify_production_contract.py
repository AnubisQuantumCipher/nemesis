#!/usr/bin/env python3
"""Verify the binding 2026-08-21 NEMESIS production mission bytes."""

from __future__ import annotations

import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = (
    ROOT
    / "docs/mission/NEMESIS_DESKTOP_PRODUCTION_READINESS_AUTONOMOUS_MISSION_2026-08-21.md"
)
EXPECTED_SHA256 = "2485cd682cdedfeb0bfd13ce2aabd51194c72204495de0c12058631cb7c92c87"
EXPECTED_BYTES = 26_694
EXPECTED_LINES = 320
EXPECTED_WORDS = 3_417


def verify_contract(data: bytes) -> list[str]:
    observed = {
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
        "lines": data.count(b"\n"),
        "words": len(data.split()),
    }
    expected = {
        "sha256": EXPECTED_SHA256,
        "bytes": EXPECTED_BYTES,
        "lines": EXPECTED_LINES,
        "words": EXPECTED_WORDS,
    }
    return [
        f"{name}: expected={expected[name]} observed={observed[name]}"
        for name in expected
        if observed[name] != expected[name]
    ]


def main() -> int:
    try:
        data = CONTRACT.read_bytes()
    except OSError as error:
        print(f"FAIL_PRODUCTION_CONTRACT unreadable={error}")
        return 1
    errors = verify_contract(data)
    if errors:
        for error in errors:
            print(f"FAIL_PRODUCTION_CONTRACT {error}")
        return 1
    print(
        "PASS_PRODUCTION_CONTRACT "
        f"sha256={EXPECTED_SHA256} bytes={EXPECTED_BYTES} "
        f"lines={EXPECTED_LINES} words={EXPECTED_WORDS}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
