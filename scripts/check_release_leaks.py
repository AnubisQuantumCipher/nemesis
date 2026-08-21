#!/usr/bin/env python3
"""Reject private build paths and credential signatures in release binaries."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


PATTERNS = (
    ("private_user_path", re.compile(rb"/Users/[^/\x00\s]+")),
    ("private_temp_path", re.compile(rb"/private/tmp/[^\x00\s]+")),
    ("github_token", re.compile(rb"(?:gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,})")),
    ("private_key", re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
)


def find_leaks(data: bytes) -> list[dict[str, int | str]]:
    leaks: list[dict[str, int | str]] = []
    for kind, pattern in PATTERNS:
        for match in pattern.finditer(data):
            leaks.append({"kind": kind, "offset": match.start()})
    return leaks


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+", type=Path)
    arguments = parser.parse_args()
    checked = 0
    failed = False
    for path in arguments.files:
        data = path.read_bytes()
        checked += 1
        for leak in find_leaks(data):
            failed = True
            print(
                "FAIL_RELEASE_LEAK "
                f"path={path} kind={leak['kind']} offset={leak['offset']}"
            )
    if failed:
        return 1
    print(f"PASS_RELEASE_LEAK_SCAN files={checked}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
