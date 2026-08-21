#!/usr/bin/env python3
"""Verify the architect-authorized NEMESIS GitHub release mission bytes."""

from __future__ import annotations

import hashlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONTRACT = (
    ROOT
    / "docs/mission/NEMESIS_FULL_AUTONOMOUS_GITHUB_RELEASE_MISSION_2026-08-20.md"
)
EXPECTED_SHA256 = "8f62d762c30849ec6841b0fbde87004ced41365b456c98c79f6f1bf53c347ccc"
EXPECTED_BYTES = 12_735
EXPECTED_LINES = 137


def main() -> int:
    try:
        data = CONTRACT.read_bytes()
    except OSError as error:
        print(f"FAIL_RELEASE_CONTRACT unreadable={error}")
        return 1
    digest = hashlib.sha256(data).hexdigest()
    lines = len(data.splitlines())
    if digest != EXPECTED_SHA256 or len(data) != EXPECTED_BYTES or lines != EXPECTED_LINES:
        print(
            "FAIL_RELEASE_CONTRACT "
            f"sha256={digest} bytes={len(data)} lines={lines}"
        )
        return 1
    print(
        "PASS_RELEASE_CONTRACT "
        f"sha256={digest} bytes={len(data)} lines={lines}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
