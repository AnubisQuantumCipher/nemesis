#!/usr/bin/env python3
"""Verify the architect-authorized NEMESIS Desktop mission bytes."""

from __future__ import annotations

import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"
EXPECTED_SHA256 = "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779"
EXPECTED_BYTES = 77_321
EXPECTED_LINES = 2_188


def main() -> int:
    data = CONTRACT.read_bytes()
    actual_sha256 = hashlib.sha256(data).hexdigest()
    actual_bytes = len(data)
    actual_lines = len(data.splitlines())
    observed = (actual_sha256, actual_bytes, actual_lines)
    expected = (EXPECTED_SHA256, EXPECTED_BYTES, EXPECTED_LINES)
    if observed != expected:
        print(
            "FAIL_CONTRACT_INTEGRITY "
            f"sha256={actual_sha256} bytes={actual_bytes} lines={actual_lines}"
        )
        return 1
    print(
        "PASS_CONTRACT_INTEGRITY "
        f"sha256={actual_sha256} bytes={actual_bytes} lines={actual_lines}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
