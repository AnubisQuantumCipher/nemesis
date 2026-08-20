#!/usr/bin/env python3
"""Derive or verify a deterministic NEMESIS vertical-slice evidence manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

CONTRACT_SHA256 = "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779"
ARTIFACTS = (
    "contract.json",
    "daemon-final.json",
    "receipt-request.json",
    "receipt-tampered.cose",
    "receipt.cose",
    "receipt.pub",
    "source-digest.txt",
    "verification.json",
)


def manifest(directory: Path) -> dict[str, object]:
    artifacts: list[dict[str, object]] = []
    for name in ARTIFACTS:
        path = directory / name
        data = path.read_bytes()
        artifacts.append(
            {
                "path": name,
                "bytes": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    return {
        "schema": "nemesis.evidence-bundle/v1",
        "architect_contract_sha256": CONTRACT_SHA256,
        "gate": "PASS_NEMESIS_BACKEND_VERTICAL_SLICE",
        "artifacts": artifacts,
        "nonclaims": [
            "NEMESIS Desktop UI is not established by this backend bundle.",
            "Mobile, web, cross-platform packaging, and public release remain DEFERRED.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    directory = arguments.directory.resolve()
    expected = manifest(directory)
    path = directory / "MANIFEST.json"
    encoded = json.dumps(expected, sort_keys=True, separators=(",", ":")) + "\n"
    if arguments.write:
        path.write_text(encoded, encoding="utf-8")
    if not path.is_file() or path.read_text(encoding="utf-8") != encoded:
        print("FAIL_EVIDENCE_BUNDLE")
        return 1
    print("PASS_EVIDENCE_BUNDLE")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
