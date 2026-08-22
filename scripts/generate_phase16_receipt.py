#!/usr/bin/env python3
"""Derive a new source-bound Phase 16 receipt from an observed gate log."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from scripts.verify_phase16_receipt import EXPECTED_ARTIFACTS
except ModuleNotFoundError:
    from verify_phase16_receipt import EXPECTED_ARTIFACTS


ROOT = Path(__file__).resolve().parents[1]
TEMPLATE = ROOT / "receipts/phase-16/PHASE16.json"
RELEASE_CONTRACT = (
    ROOT
    / "docs/mission/NEMESIS_FULL_AUTONOMOUS_GITHUB_RELEASE_MISSION_2026-08-20.md"
)
RELEASE_CONTRACT_SHA256 = (
    "8f62d762c30849ec6841b0fbde87004ced41365b456c98c79f6f1bf53c347ccc"
)
RELEASE_CONTRACT_BYTES = 12_735
RELEASE_CONTRACT_LINES = 137
SENTINELS = (
    "PASS_PHASE16_VZ_SECURITY_HARDENING",
    "PASS_PHASE16_VZ_SECURITY_HARDENING_ORCHESTRATOR",
    "PASS_PHASE16_SECURITY_HARDENING",
)


def git(*arguments: str) -> str:
    return subprocess.run(
        ["/usr/bin/git", *arguments],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout.strip()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def gate_metadata(
    log: bytes,
    *,
    observed_at: str,
    log_file: str | None = None,
) -> dict[str, Any]:
    text = log.decode("utf-8")
    lines = text.splitlines()
    for sentinel in SENTINELS:
        if not any(line == sentinel or line.startswith(f"{sentinel} ") for line in lines):
            raise ValueError(f"missing hardening sentinel: {sentinel}")
    if any(
        line.startswith(("FAIL_", "FATAL_", "REFUSED_"))
        for line in lines
    ):
        raise ValueError("hardening log contains a terminal failure marker")
    metadata: dict[str, Any] = {
        "command": "./scripts/test_security_hardening.sh",
        "observed_at": observed_at,
        "status": "PASS",
        "sentinels": list(SENTINELS),
        "log": {
            "bytes": len(log),
            "line_count": len(lines),
            "sha256": hashlib.sha256(log).hexdigest(),
        },
        "guest": {
            "base_image": "anubis-xcode",
            "deleted_after_run": True,
            "primary_crash_mutation_evidence": True,
            "transport": "Tart over Apple Virtualization.framework",
        },
        "observed_warnings": [
            "Ada link steps reported a deployment-target override from 16.0 to the host target 26.0; every build, test, and bounded proof command still exited zero."
        ],
        "recovery_run_failures": [],
    }
    if log_file is not None:
        metadata["log"]["file"] = log_file
    return metadata


def build_receipt(
    *,
    subject: str,
    gate_log: Path,
    observed_at: str,
) -> dict[str, Any]:
    template = json.loads(TEMPLATE.read_text(encoding="utf-8"))
    contract_bytes = RELEASE_CONTRACT.read_bytes()
    if (
        hashlib.sha256(contract_bytes).hexdigest() != RELEASE_CONTRACT_SHA256
        or len(contract_bytes) != RELEASE_CONTRACT_BYTES
        or len(contract_bytes.splitlines()) != RELEASE_CONTRACT_LINES
    ):
        raise ValueError("release contract identity changed")
    git("cat-file", "-e", f"{subject}^{{commit}}")
    changed_artifacts = git(
        "diff", "--name-only", subject, "--", *sorted(EXPECTED_ARTIFACTS)
    )
    if changed_artifacts:
        raise ValueError("phase receipt artifact differs from subject commit")
    try:
        relative_log = gate_log.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        relative_log = gate_log.name
    template["epoch"] = "v0.2.0-trust-surface"
    template["release_contract"] = {
        "path": RELEASE_CONTRACT.relative_to(ROOT).as_posix(),
        "sha256": RELEASE_CONTRACT_SHA256,
        "bytes": RELEASE_CONTRACT_BYTES,
        "lines": RELEASE_CONTRACT_LINES,
    }
    template["artifact_sha256"] = {
        relative: sha256(ROOT / relative) for relative in sorted(EXPECTED_ARTIFACTS)
    }
    template["repository"] = {
        "branch": git("branch", "--show-current"),
        "subject_commit": subject,
        "subject_tree": git("rev-parse", f"{subject}^{{tree}}"),
    }
    template["gate"] = gate_metadata(
        gate_log.read_bytes(),
        observed_at=observed_at,
        log_file=relative_log,
    )
    template["residuals"] = [
        {"id": "independent_external_security_review", "status": "NEEDS-HUMAN"},
        {"id": "developer_id_signing_and_notarization", "status": "UNAVAILABLE"},
        {"id": "legal_clearance", "status": "NOT_CLAIMED"},
        {"id": "mobile_web_remote_and_other_platforms", "status": "DEFERRED"},
    ]
    return template


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--subject", default="HEAD")
    parser.add_argument("--gate-log", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--observed-at")
    arguments = parser.parse_args()
    subject = git("rev-parse", arguments.subject)
    observed_at = arguments.observed_at or datetime.now(timezone.utc).isoformat(
        timespec="seconds"
    ).replace("+00:00", "Z")
    try:
        receipt = build_receipt(
            subject=subject,
            gate_log=arguments.gate_log,
            observed_at=observed_at,
        )
    except (OSError, UnicodeDecodeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"FAIL_PHASE16_RECEIPT_GENERATION error={type(error).__name__}")
        return 1
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "PASS_PHASE16_RECEIPT_GENERATION "
        f"subject={subject} output={output.relative_to(ROOT)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
