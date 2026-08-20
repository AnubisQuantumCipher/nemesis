#!/usr/bin/env python3
"""Verify the restart-safe NEMESIS partial checkpoint."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHECKPOINT = ROOT / "receipts/CHECKPOINT.json"
EXPECTED_OPEN = [8, 9, 10, 11, 12, 13, 14, 16, "seal"]
EXPECTED_DEFERRED = [15, 17, 18, 19]
ALLOWED_CHECKPOINT_PATHS = {
    "STATUS.md",
    "receipts/CHECKPOINT.json",
    "scripts/verify_checkpoint.py",
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


def main() -> int:
    data = json.loads(CHECKPOINT.read_text(encoding="utf-8"))
    contract_path = ROOT / data["architect_contract"]["path"]
    contract = contract_path.read_bytes()
    if hashlib.sha256(contract).hexdigest() != data["architect_contract"]["sha256"]:
        print("FAIL_CHECKPOINT contract digest")
        return 1
    if len(contract) != data["architect_contract"]["bytes"]:
        print("FAIL_CHECKPOINT contract bytes")
        return 1
    if len(contract.splitlines()) != data["architect_contract"]["lines"]:
        print("FAIL_CHECKPOINT contract lines")
        return 1

    implementation = data["repository"]["implementation_commit"]
    if git("cat-file", "-e", f"{implementation}^{{commit}}", check=False).returncode != 0:
        print("FAIL_CHECKPOINT missing implementation commit")
        return 1
    if git("merge-base", "--is-ancestor", implementation, "HEAD", check=False).returncode != 0:
        print("FAIL_CHECKPOINT implementation is not an ancestor")
        return 1
    changed = set(
        filter(None, git("diff", "--name-only", f"{implementation}..HEAD").stdout.splitlines())
    )
    if not changed.issubset(ALLOWED_CHECKPOINT_PATHS):
        print(f"FAIL_CHECKPOINT unexpected post-implementation paths: {sorted(changed)}")
        return 1
    if git("status", "--porcelain").stdout:
        print("FAIL_CHECKPOINT dirty worktree")
        return 1
    if git("remote").stdout.strip():
        print("FAIL_CHECKPOINT unexpected Git remote")
        return 1

    for phase in data["completed_phases"]:
        commit = phase["commit"]
        if git("cat-file", "-e", f"{commit}^{{commit}}", check=False).returncode != 0:
            print(f"FAIL_CHECKPOINT missing phase commit {commit}")
            return 1
    if [entry["phase"] for entry in data["open_phases"]] != EXPECTED_OPEN:
        print("FAIL_CHECKPOINT open phase list")
        return 1
    if [entry["phase"] for entry in data["deferred_phases"]] != EXPECTED_DEFERRED:
        print("FAIL_CHECKPOINT deferred phase list")
        return 1
    if any(entry["status"] != "DEFERRED" for entry in data["deferred_phases"]):
        print("FAIL_CHECKPOINT deferred status")
        return 1

    for bundle in ("receipts/phase-6", "receipts/phase-7-desktop"):
        result = subprocess.run(
            ["/usr/bin/python3", "scripts/verify_evidence_bundle.py", bundle],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode != 0 or "PASS_EVIDENCE_BUNDLE" not in result.stdout:
            print(f"FAIL_CHECKPOINT evidence bundle {bundle}")
            return 1
    visual = json.loads(
        (ROOT / "receipts/phase-7-desktop/DESKTOP_QA.json").read_text(encoding="utf-8")
    )
    if visual["native"]["app_operated_mission"] is not True:
        print("FAIL_CHECKPOINT native mission evidence")
        return 1

    print("PASS_CHECKPOINT completed=0-7 next=8 verdict=PARTIAL")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
