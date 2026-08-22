#!/usr/bin/env python3
"""Verify the source-bound NEMESIS Desktop Phase 16 hardening receipt."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_RECEIPT = ROOT / "receipts/phase-16/PHASE16.json"
CONTRACT_SHA256 = "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779"
CONTRACT_BYTES = 77_321
CONTRACT_LINES = 2_188
RELEASE_CONTRACT_SHA256 = (
    "8f62d762c30849ec6841b0fbde87004ced41365b456c98c79f6f1bf53c347ccc"
)
RELEASE_CONTRACT_BYTES = 12_735
RELEASE_CONTRACT_LINES = 137
RELEASE_EPOCH = "v0.2.0-trust-surface"
EXPECTED_SENTINELS = {
    "PASS_PHASE16_VZ_SECURITY_HARDENING",
    "PASS_PHASE16_VZ_SECURITY_HARDENING_ORCHESTRATOR",
    "PASS_PHASE16_SECURITY_HARDENING",
}
EXPECTED_FINDINGS = {"H16-01", "H16-02", "H16-03", "H16-04"}
EXPECTED_ARTIFACTS = {
    "docs/verification/PHASE16_SECURITY_REVIEW.md",
    "runtime/Cargo.lock",
    "runtime/Cargo.toml",
    "runtime/crates/nemesis-protocol/src/lib.rs",
    "runtime/crates/nemesis-runtime/src/context.rs",
    "runtime/crates/nemesis-runtime/src/git_workflows.rs",
    "runtime/crates/nemesis-runtime/tests/context.rs",
    "runtime/crates/nemesis-runtime/tests/git_workflows.rs",
    "runtime/crates/nemesis-runtime/tests/isolation.rs",
    "runtime/crates/nemesis-runtime/tests/security_mutation.rs",
    "runtime/crates/nemesis-signer/src/lib.rs",
    "scripts/run_security_hardening.sh",
    "scripts/security_hardening_guest.sh",
    "scripts/test_security_hardening.sh",
    "scripts/test_phase16_receipt.py",
    "scripts/verify_phase16_receipt.py",
}
COVERED_PATHS = [
    "runtime",
    "kernel",
    "daemon",
    "desktop",
    "protocols",
    "config",
    "alire.toml",
    "nemesis.gpr",
    "scripts/build_ada.sh",
    "scripts/prove_kernel.sh",
    "scripts/run_security_hardening.sh",
    "scripts/security_hardening_guest.sh",
    "scripts/test_security_hardening.sh",
    "scripts/test_phase16_receipt.py",
    "scripts/verify_contract.py",
    "scripts/verify_phase16_receipt.py",
    "docs/verification/PHASE16_SECURITY_REVIEW.md",
]


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
    print(f"FAIL_PHASE16_RECEIPT {reason}")
    return 1


def main() -> int:
    receipt = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_RECEIPT
    try:
        data = json.loads(receipt.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"unreadable={error}")

    if data.get("schema") != "nemesis.phase16-hardening/v1":
        return fail("schema")
    if data.get("phase") != 16 or data.get("status") != "VERIFIED":
        return fail("phase_status")

    contract = data.get("architect_contract", {})
    contract_path = ROOT / contract.get("path", "")
    try:
        contract_bytes = contract_path.read_bytes()
    except OSError as error:
        return fail(f"contract_unreadable={error}")
    if (
        hashlib.sha256(contract_bytes).hexdigest() != CONTRACT_SHA256
        or len(contract_bytes) != CONTRACT_BYTES
        or len(contract_bytes.splitlines()) != CONTRACT_LINES
        or contract.get("sha256") != CONTRACT_SHA256
        or contract.get("bytes") != CONTRACT_BYTES
        or contract.get("lines") != CONTRACT_LINES
    ):
        return fail("contract_identity")
    epoch = data.get("epoch")
    if epoch is not None:
        if epoch != RELEASE_EPOCH:
            return fail("release_epoch")
        release_contract = data.get("release_contract", {})
        release_contract_path = ROOT / release_contract.get("path", "")
        try:
            release_contract_bytes = release_contract_path.read_bytes()
        except OSError as error:
            return fail(f"release_contract_unreadable={error}")
        if (
            hashlib.sha256(release_contract_bytes).hexdigest()
            != RELEASE_CONTRACT_SHA256
            or len(release_contract_bytes) != RELEASE_CONTRACT_BYTES
            or len(release_contract_bytes.splitlines()) != RELEASE_CONTRACT_LINES
            or release_contract.get("sha256") != RELEASE_CONTRACT_SHA256
            or release_contract.get("bytes") != RELEASE_CONTRACT_BYTES
            or release_contract.get("lines") != RELEASE_CONTRACT_LINES
        ):
            return fail("release_contract_identity")

    repository = data.get("repository", {})
    subject = repository.get("subject_commit", "")
    if git("cat-file", "-e", f"{subject}^{{commit}}", check=False).returncode != 0:
        return fail("missing_subject_commit")
    if git("merge-base", "--is-ancestor", subject, "HEAD", check=False).returncode != 0:
        return fail("subject_not_ancestor")
    subject_tree = git("rev-parse", f"{subject}^{{tree}}").stdout.strip()
    if repository.get("subject_tree") != subject_tree:
        return fail("subject_tree")
    changed = set(
        filter(
            None,
            git("diff", "--name-only", f"{subject}..HEAD", "--", *COVERED_PATHS)
            .stdout.splitlines(),
        )
    )
    if changed:
        return fail(f"covered_source_changed={sorted(changed)}")

    artifacts = data.get("artifact_sha256", {})
    if set(artifacts) != EXPECTED_ARTIFACTS:
        return fail("artifact_set")

    for relative, expected in artifacts.items():
        if relative.startswith("/") or ".." in Path(relative).parts:
            return fail("artifact_path")
        try:
            actual = hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()
        except OSError as error:
            return fail(f"artifact_unreadable={error}")
        if actual != expected:
            return fail(f"artifact_digest={relative}")

    gate = data.get("gate", {})
    if gate.get("command") != "./scripts/test_security_hardening.sh":
        return fail("gate_command")
    if gate.get("status") != "PASS" or set(gate.get("sentinels", [])) != EXPECTED_SENTINELS:
        return fail("gate_result")
    guest = gate.get("guest", {})
    if guest.get("base_image") != "anubis-xcode" or guest.get("deleted_after_run") is not True:
        return fail("guest_cleanup")
    if epoch is not None:
        log = gate.get("log", {})
        relative_log = log.get("file", "")
        if (
            not relative_log
            or relative_log.startswith("/")
            or ".." in Path(relative_log).parts
        ):
            return fail("gate_log_path")
        try:
            log_bytes = (ROOT / relative_log).read_bytes()
            log_text = log_bytes.decode("utf-8")
        except (OSError, UnicodeDecodeError) as error:
            return fail(f"gate_log_unreadable={error}")
        if (
            hashlib.sha256(log_bytes).hexdigest() != log.get("sha256")
            or len(log_bytes) != log.get("bytes")
            or len(log_text.splitlines()) != log.get("line_count")
        ):
            return fail("gate_log_identity")
        log_lines = log_text.splitlines()
        if any(
            not any(line == sentinel or line.startswith(f"{sentinel} ") for line in log_lines)
            for sentinel in EXPECTED_SENTINELS
        ):
            return fail("gate_log_sentinel")
        if any(
            line.startswith(("FAIL_", "FATAL_", "REFUSED_"))
            for line in log_lines
        ):
            return fail("gate_log_failure_marker")

    dependencies = data.get("dependency_review", {})
    if dependencies.get("cargo_audit") != "PASS" or dependencies.get("npm_audit") != "PASS":
        return fail("dependency_review")
    if data.get("proof_review", {}).get("bounded_kernel") != "PASS":
        return fail("proof_review")
    if {finding.get("id") for finding in data.get("accepted_findings", [])} != EXPECTED_FINDINGS:
        return fail("accepted_findings")
    residuals = data.get("residuals", [])
    if not any(
        residual.get("id") == "independent_external_security_review"
        and residual.get("status") == "NEEDS-HUMAN"
        for residual in residuals
    ):
        return fail("external_review_residual")
    dirty_covered = git(
        "status",
        "--porcelain",
        "--untracked-files=all",
        "--",
        *COVERED_PATHS,
    ).stdout
    if dirty_covered:
        return fail("dirty_covered_source")

    print(f"PASS_PHASE16_RECEIPT subject={subject} verdict=VERIFIED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
