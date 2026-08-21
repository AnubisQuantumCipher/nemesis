from __future__ import annotations

import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
VERIFIER = ROOT / "scripts/verify_phase16_receipt.py"
RECEIPT = ROOT / "receipts/phase-16-release/PHASE16.json"


class Phase16PublicVerificationTests(unittest.TestCase):
    def test_receipt_verifies_when_clone_has_a_git_remote(self) -> None:
        result = subprocess.run(
            ["/usr/bin/python3", str(VERIFIER), str(RECEIPT)],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("PASS_PHASE16_RECEIPT", result.stdout)


if __name__ == "__main__":
    unittest.main()
