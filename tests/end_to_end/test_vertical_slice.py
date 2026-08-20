from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / "scripts/run_vertical_slice.sh"
VERIFIER = ROOT / "runtime/target/debug/nemesis-verify"


class VerticalSliceTests(unittest.TestCase):
    def test_restart_source_binding_receipt_and_tamper_contract(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "evidence"
            run = subprocess.run(
                [str(RUNNER), "--output", str(output)],
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=120,
            )
            self.assertEqual(0, run.returncode, f"{run.stdout}\n{run.stderr}")
            self.assertIn("PASS_NEMESIS_BACKEND_VERTICAL_SLICE", run.stdout)
            receipt = output / "receipt.cose"
            public_key = output / "receipt.pub"
            tampered = output / "receipt-tampered.cose"
            self.assertTrue(receipt.is_file())
            self.assertTrue(public_key.is_file())
            self.assertTrue(tampered.is_file())

            valid = subprocess.run(
                [str(VERIFIER), "--receipt", str(receipt), "--public-key", str(public_key)],
                stdout=subprocess.PIPE,
                text=True,
            )
            self.assertEqual(0, valid.returncode, valid.stdout)
            invalid = subprocess.run(
                [str(VERIFIER), "--receipt", str(tampered), "--public-key", str(public_key)],
                stdout=subprocess.PIPE,
                text=True,
            )
            self.assertNotEqual(0, invalid.returncode)
            self.assertIn('"verdict":"REJECTED"', invalid.stdout)


if __name__ == "__main__":
    unittest.main()
