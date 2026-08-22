#!/usr/bin/env python3
"""Run a byte-restoring A→B→A GNATprove calibration in a disposable copy."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "receipts/production-readiness-20260821/proof-aba"
RECEIPT = ROOT / "receipts/production-readiness-20260821/PROOF_ABA.json"
MUTATION_PATH = Path("kernel/src/nemesis-kernel-capabilities.adb")
ORIGINAL = "         return Refused_Capability;"
MUTATED = "         return Authorized;"
COPY_DIRS = ("config", "kernel", "daemon", "scripts")
COPY_FILES = ("alire.toml", "nemesis.gpr")
INVENTORY_ROOTS = (
    "config/formal-kernel-scope.json",
    "kernel/src",
    "daemon/src",
    "scripts/prove_kernel.sh",
    "scripts/verify_kernel_proof.py",
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def hash_inventory(root: Path) -> dict[str, str]:
    files: list[Path] = []
    for relative in INVENTORY_ROOTS:
        path = root / relative
        if path.is_dir():
            files.extend(candidate for candidate in path.rglob("*") if candidate.is_file())
        else:
            files.append(path)
    return {
        str(path.relative_to(root)): sha256(path.read_bytes())
        for path in sorted(set(files))
    }


def git(*arguments: str) -> str:
    return subprocess.run(
        ["/usr/bin/git", *arguments],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout.strip()


def run_proof(root: Path) -> subprocess.CompletedProcess[bytes]:
    environment = {
        "HOME": str(Path.home()),
        "LANG": "C",
        "LC_ALL": "C",
        "NEMESIS_PROOF_CHECK_ONLY": "1",
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "TMPDIR": "/tmp",
    }
    return subprocess.run(
        ["/bin/bash", "scripts/prove_kernel.sh"],
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=300,
    )


def log_metadata(path: Path, exit_status: int) -> dict[str, object]:
    data = path.read_bytes()
    return {
        "path": str(path.relative_to(ROOT)),
        "sha256": sha256(data),
        "bytes": len(data),
        "lines": data.count(b"\n"),
        "exit_status": exit_status,
    }


def main() -> int:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    for stale in (RECEIPT, OUTPUT / "A.log", OUTPUT / "B.log", OUTPUT / "A2.log"):
        if stale.exists():
            stale.unlink()
    source_before = hash_inventory(ROOT)
    with tempfile.TemporaryDirectory(prefix="nemesis-proof-aba-") as temporary:
        copy = Path(temporary) / "repo"
        copy.mkdir()
        for directory in COPY_DIRS:
            shutil.copytree(ROOT / directory, copy / directory)
        for filename in COPY_FILES:
            shutil.copy2(ROOT / filename, copy / filename)
        copied_before = hash_inventory(copy)

        a = run_proof(copy)
        (OUTPUT / "A.log").write_bytes(a.stdout)
        if a.returncode != 0 or b"PASS_KERNEL_PROOF_BASELINE" not in a.stdout:
            print("FAIL_KERNEL_PROOF_ABA initial_A")
            return 1

        target = copy / MUTATION_PATH
        original_bytes = target.read_bytes()
        text = original_bytes.decode("utf-8")
        if text.count(ORIGINAL) != 1:
            print("FAIL_KERNEL_PROOF_ABA mutation_anchor")
            return 1
        target.write_text(text.replace(ORIGINAL, MUTATED, 1), encoding="utf-8")
        if target.read_bytes() == original_bytes:
            print("FAIL_KERNEL_PROOF_ABA mutation_noop")
            return 1

        b = run_proof(copy)
        (OUTPUT / "B.log").write_bytes(b.stdout)
        lower_b = b.stdout.lower()
        if b.returncode == 0 or not (
            b"postcondition" in lower_b and (b"might fail" in lower_b or b"medium:" in lower_b)
        ):
            print("FAIL_KERNEL_PROOF_ABA violation_not_rejected_for_intended_reason")
            return 1

        target.write_bytes(original_bytes)
        if target.read_bytes() != original_bytes or hash_inventory(copy) != copied_before:
            print("FAIL_KERNEL_PROOF_ABA byte_restoration")
            return 1

        a2 = run_proof(copy)
        (OUTPUT / "A2.log").write_bytes(a2.stdout)
        if a2.returncode != 0 or b"PASS_KERNEL_PROOF_BASELINE" not in a2.stdout:
            print("FAIL_KERNEL_PROOF_ABA final_A")
            return 1

    if hash_inventory(ROOT) != source_before:
        print("FAIL_KERNEL_PROOF_ABA subject_tree_changed")
        return 1
    receipt = {
        "schema": "nemesis.kernel-proof-aba/v1",
        "status": "PASS",
        "subject": {
            "commit": git("rev-parse", "HEAD"),
            "tree": git("rev-parse", "HEAD^{tree}"),
            "source_inventory_sha256": sha256(
                json.dumps(source_before, sort_keys=True, separators=(",", ":")).encode()
            ),
        },
        "mutation": {
            "path": str(MUTATION_PATH),
            "old": ORIGINAL.strip(),
            "new": MUTATED.strip(),
            "expected_reason": "GNATprove postcondition failure",
            "landed_in_subject": False,
        },
        "runs": {
            "A": log_metadata(OUTPUT / "A.log", 0),
            "B": log_metadata(OUTPUT / "B.log", b.returncode),
            "A2": log_metadata(OUTPUT / "A2.log", 0),
        },
        "restoration": {
            "copy_inventory_equal": True,
            "subject_inventory_equal": True,
            "temporary_copy_removed": True,
        },
        "observed_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        "terminal_marker": "PASS_KERNEL_PROOF_ABA",
    }
    temporary = RECEIPT.with_name(f".{RECEIPT.name}.tmp-{os.getpid()}")
    temporary.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(RECEIPT)
    print("PASS_KERNEL_PROOF_ABA A=0 B=nonzero A2=0 bytes_restored=true")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
