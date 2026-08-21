#!/usr/bin/env python3
"""Verify the restart-safe NEMESIS Desktop PARTIAL checkpoint."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHECKPOINT = ROOT / "receipts/CHECKPOINT.json"
STATUS = ROOT / "STATUS.md"
EXPECTED_PHASES = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16]
EXPECTED_DEFERRED = [15, 17, 18, 19]
EXPECTED_IMPLEMENTATION = "9bd9fd4a04bfbb6a5ab4f45799ee388d41e84296"
EXPECTED_COMPLETE_SCRIPT_SHA256 = "d9b647214dc86ebbf33cbed1e0d61b28c1e07ecda097a131858faeb301700ff7"
EXPECTED_LOG_SHA256 = "766a440b133eb34db560bd6772cf225a380f73db8439574123aa101bb5392a51"
EXPECTED_LOG_BYTES = 81_785
EXPECTED_LOG_LINES = 1_538
ALLOWED_CHECKPOINT_PATHS = {
    "STATUS.md",
    "receipts/CHECKPOINT.json",
    "receipts/acceptance-20260820/CAPTURE.json",
    "receipts/acceptance-20260820/verify-complete.log",
    "scripts/verify_checkpoint.py",
    "scripts/verify_complete.sh",
}


def git(*arguments: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["/usr/bin/git", *arguments],
        cwd=ROOT,
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def fail(reason: str) -> int:
    print(f"FAIL_CHECKPOINT {reason}")
    return 1


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    try:
        checkpoint = json.loads(CHECKPOINT.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"unreadable={error}")
    if checkpoint.get("schema") != "nemesis.checkpoint/v2":
        return fail("schema")
    if checkpoint.get("verdict") != "PARTIAL":
        return fail("verdict")
    if checkpoint.get("terminal_product_verdict") != "NOT_RUN_ARCHITECT_ROUTED":
        return fail("terminal_verdict_boundary")

    contract = checkpoint.get("architect_contract", {})
    contract_path = ROOT / contract.get("path", "")
    try:
        contract_bytes = contract_path.read_bytes()
    except OSError as error:
        return fail(f"contract_unreadable={error}")
    if (
        hashlib.sha256(contract_bytes).hexdigest()
        != "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779"
        or len(contract_bytes) != 77_321
        or len(contract_bytes.splitlines()) != 2_188
        or contract.get("sha256")
        != "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779"
        or contract.get("bytes") != 77_321
        or contract.get("lines") != 2_188
    ):
        return fail("contract_identity")

    implementation = checkpoint.get("repository", {}).get("implementation_commit", "")
    if implementation != EXPECTED_IMPLEMENTATION:
        return fail("implementation_commit")
    if git("cat-file", "-e", f"{implementation}^{{commit}}", check=False).returncode != 0:
        return fail("missing_implementation_commit")
    if git("merge-base", "--is-ancestor", implementation, "HEAD", check=False).returncode != 0:
        return fail("implementation_not_ancestor")
    changed = set(
        filter(None, git("diff", "--name-only", f"{implementation}..HEAD").stdout.splitlines())
    )
    if not changed.issubset(ALLOWED_CHECKPOINT_PATHS):
        return fail(f"unexpected_post_implementation_paths={sorted(changed)}")
    dirty_lines = git("status", "--porcelain", "--untracked-files=all").stdout.splitlines()
    dirty_paths = {line[3:] for line in dirty_lines if len(line) >= 4}
    if not dirty_paths.issubset(ALLOWED_CHECKPOINT_PATHS):
        return fail(f"unexpected_dirty_paths={sorted(dirty_paths)}")
    if git("remote").stdout.strip():
        return fail("unexpected_remote")

    phases = checkpoint.get("completed_phases", [])
    if [entry.get("phase") for entry in phases] != EXPECTED_PHASES:
        return fail("completed_phase_list")
    for entry in phases:
        commit = entry.get("commit", "")
        if git("cat-file", "-e", f"{commit}^{{commit}}", check=False).returncode != 0:
            return fail(f"missing_phase_commit={commit}")
        if git("merge-base", "--is-ancestor", commit, implementation, check=False).returncode != 0:
            return fail(f"phase_not_in_implementation={entry.get('phase')}")
    if [entry.get("phase") for entry in checkpoint.get("deferred_phases", [])] != EXPECTED_DEFERRED:
        return fail("deferred_phase_list")
    if any(entry.get("status") != "DEFERRED" for entry in checkpoint.get("deferred_phases", [])):
        return fail("deferred_phase_status")
    open_phases = checkpoint.get("open_phases", [])
    if open_phases != [
        {
            "name": "Terminal product verdict",
            "phase": "terminal-verdict",
            "status": "ROUTED_TO_FRESH_SESSION",
        }
    ]:
        return fail("open_phase_boundary")

    for bundle in ("receipts/phase-6", "receipts/phase-7-desktop", "receipts/phase-11"):
        result = subprocess.run(
            ["/usr/bin/python3", "scripts/verify_evidence_bundle.py", bundle],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode != 0 or "PASS_EVIDENCE_BUNDLE" not in result.stdout:
            return fail(f"evidence_bundle={bundle}")

    phase16 = subprocess.run(
        ["/usr/bin/python3", "scripts/test_phase16_receipt.py"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if phase16.returncode != 0 or "PASS_PHASE16_RECEIPT_TAMPER_REJECTION" not in phase16.stdout:
        return fail("phase16_receipt")

    acceptance = checkpoint.get("acceptance_battery", {})
    capture_path = ROOT / acceptance.get("capture", {}).get("metadata", "")
    log_path = ROOT / acceptance.get("capture", {}).get("file", "")
    try:
        capture = json.loads(capture_path.read_text(encoding="utf-8"))
        log_bytes = log_path.read_bytes()
        log_text = log_bytes.decode("utf-8")
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return fail(f"acceptance_capture_unreadable={error}")
    if (
        hashlib.sha256(log_bytes).hexdigest() != EXPECTED_LOG_SHA256
        or len(log_bytes) != EXPECTED_LOG_BYTES
        or len(log_text.splitlines()) != EXPECTED_LOG_LINES
        or capture.get("sha256") != EXPECTED_LOG_SHA256
        or capture.get("bytes") != EXPECTED_LOG_BYTES
        or capture.get("line_count") != EXPECTED_LOG_LINES
        or capture.get("process_exit") != 0
        or capture.get("first_line")
        != "PASS_CONTRACT_INTEGRITY sha256=ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779 bytes=77321 lines=2188"
        or capture.get("last_line") != "PASS_NEMESIS_DESKTOP_COMPLETE"
        or log_text.splitlines()[-1] != "PASS_NEMESIS_DESKTOP_COMPLETE"
    ):
        return fail("acceptance_capture")
    if (
        acceptance.get("status") != "PASS"
        or acceptance.get("sentinel") != "PASS_NEMESIS_DESKTOP_COMPLETE"
        or acceptance.get("corrected_script_sha256") != EXPECTED_COMPLETE_SCRIPT_SHA256
        or sha256(ROOT / "scripts/verify_complete.sh") != EXPECTED_COMPLETE_SCRIPT_SHA256
    ):
        return fail("acceptance_gate")

    try:
        status_text = STATUS.read_text(encoding="utf-8")
    except OSError as error:
        return fail(f"status_unreadable={error}")
    for required in (
        "`PARTIAL`",
        "terminal product verdict to a fresh OMP session",
        EXPECTED_LOG_SHA256,
        "PASS_NEMESIS_DESKTOP_COMPLETE",
    ):
        if required not in status_text:
            return fail(f"status_missing={required}")

    print(
        "PASS_CHECKPOINT completed=0-14,16 acceptance=PASS "
        "next=terminal-verdict verdict=PARTIAL"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
