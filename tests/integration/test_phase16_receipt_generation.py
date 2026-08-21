from __future__ import annotations

import hashlib
import subprocess
import unittest
from pathlib import Path

from scripts.generate_phase16_receipt import gate_metadata

ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "scripts/generate_phase16_receipt.py"


class Phase16ReceiptGenerationTests(unittest.TestCase):
    def test_gate_metadata_requires_every_terminal_sentinel(self) -> None:
        log = b"\n".join(
            [
                b"PASS_PHASE16_VZ_SECURITY_HARDENING",
                b"PASS_PHASE16_VZ_SECURITY_HARDENING_ORCHESTRATOR base=anubis-xcode",
                b"PASS_PHASE16_SECURITY_HARDENING",
                b"",
            ]
        )

        metadata = gate_metadata(log, observed_at="2026-08-20T23:00:00Z")

        self.assertEqual(metadata["status"], "PASS")
        self.assertEqual(metadata["log"]["bytes"], len(log))
        self.assertEqual(metadata["log"]["sha256"], hashlib.sha256(log).hexdigest())
        self.assertEqual(len(metadata["sentinels"]), 3)

        with self.assertRaisesRegex(ValueError, "missing hardening sentinel"):
            gate_metadata(
                b"PASS_PHASE16_SECURITY_HARDENING\n",
                observed_at="2026-08-20T23:00:00Z",
            )

    def test_generator_runs_as_a_script(self) -> None:
        result = subprocess.run(
            ["/usr/bin/python3", str(GENERATOR), "--help"],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
