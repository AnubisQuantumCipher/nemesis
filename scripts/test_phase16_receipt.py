#!/usr/bin/env python3
"""Exercise valid and tampered Phase 16 receipt verification."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RECEIPT = ROOT / "receipts/phase-16/PHASE16.json"
VERIFIER = ROOT / "scripts/verify_phase16_receipt.py"
TAMPERED_ARTIFACT = "docs/verification/PHASE16_SECURITY_REVIEW.md"


def verify(path: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["/usr/bin/python3", str(VERIFIER), str(path)],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def main() -> int:
    valid = verify(RECEIPT)
    if valid.returncode != 0 or "PASS_PHASE16_RECEIPT" not in valid.stdout:
        print(f"FAIL_PHASE16_RECEIPT_VALID\n{valid.stdout}{valid.stderr}")
        return 1

    data = json.loads(RECEIPT.read_text(encoding="utf-8"))
    data["artifact_sha256"][TAMPERED_ARTIFACT] = "00" * 32
    with tempfile.TemporaryDirectory(prefix=".phase16-receipt-", dir=ROOT / "receipts") as temp:
        tampered = Path(temp) / "PHASE16.json"
        tampered.write_text(
            json.dumps(data, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        rejected = verify(tampered)
    if rejected.returncode == 0 or "FAIL_PHASE16_RECEIPT artifact_digest" not in rejected.stdout:
        print(f"FAIL_PHASE16_RECEIPT_TAMPER_ACCEPTED\n{rejected.stdout}{rejected.stderr}")
        return 1

    print(valid.stdout.strip())
    print("PASS_PHASE16_RECEIPT_TAMPER_REJECTION")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
