from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
VERIFIER = ROOT / "scripts/verify_evidence_bundle.py"
REQUIRED_ARTIFACTS = (
    "contract.json",
    "daemon-final.json",
    "receipt-request.json",
    "receipt-tampered.cose",
    "receipt.cose",
    "receipt.pub",
    "source-digest.txt",
    "verification.json",
)


class EvidenceBundleTests(unittest.TestCase):
    def test_desktop_qa_receipt_is_covered_by_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary)
            for name in REQUIRED_ARTIFACTS:
                (bundle / name).write_bytes(name.encode("utf-8"))
            (bundle / "DESKTOP_QA.json").write_text("{}\n", encoding="utf-8")

            result = subprocess.run(
                ["/usr/bin/python3", str(VERIFIER), str(bundle), "--write"],
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            manifest = json.loads((bundle / "MANIFEST.json").read_text(encoding="utf-8"))
            paths = {artifact["path"] for artifact in manifest["artifacts"]}
            self.assertIn("DESKTOP_QA.json", paths)

    def test_unlisted_artifact_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary)
            for name in REQUIRED_ARTIFACTS:
                (bundle / name).write_bytes(name.encode("utf-8"))
            generated = subprocess.run(
                ["/usr/bin/python3", str(VERIFIER), str(bundle), "--write"],
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            self.assertEqual(generated.returncode, 0, generated.stdout + generated.stderr)
            (bundle / "unlisted.bin").write_bytes(b"uncovered evidence")

            verified = subprocess.run(
                ["/usr/bin/python3", str(VERIFIER), str(bundle)],
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            self.assertNotEqual(verified.returncode, 0)
            self.assertIn("FAIL_EVIDENCE_BUNDLE unexpected_artifacts", verified.stdout)


if __name__ == "__main__":
    unittest.main()
