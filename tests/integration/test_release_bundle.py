from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RELEASE_CONFIG = ROOT / "desktop/src-tauri/tauri.release.conf.json"
EXPECTED_RESOURCES = {
    "../../build/bin/nemesis_core_daemon": "bin/nemesis_core_daemon",
    "../../runtime/target/release/nemesis-deterministic-worker": "bin/nemesis-deterministic-worker",
    "../../runtime/target/release/nemesis-lane-create": "bin/nemesis-lane-create",
    "../../runtime/target/release/nemesis-signer": "bin/nemesis-signer",
    "../../runtime/target/release/nemesis-verify": "bin/nemesis-verify",
    "../../runtime/target/release/nemesis-worker-runner": "bin/nemesis-worker-runner",
    "../../scripts/run_vertical_slice.py": "scripts/run_vertical_slice.py",
    "../../scripts/run_vertical_slice.sh": "scripts/run_vertical_slice.sh",
    "../../scripts/verify_contract.py": "scripts/verify_contract.py",
    "../../scripts/verify_evidence_bundle.py": "scripts/verify_evidence_bundle.py",
    "../../docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md": "docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md",
    "../../docs/mission/NEMESIS_FULL_AUTONOMOUS_GITHUB_RELEASE_MISSION_2026-08-20.md": "docs/mission/NEMESIS_FULL_AUTONOMOUS_GITHUB_RELEASE_MISSION_2026-08-20.md",
    "../../receipts/phase-11/replay.json": "receipts/phase-11/replay.json",
    "../../LICENSE": "LICENSE",
}


class ReleaseBundleTests(unittest.TestCase):
    def test_release_config_bundles_complete_standalone_runtime(self) -> None:
        config = json.loads(RELEASE_CONFIG.read_text(encoding="utf-8"))

        self.assertEqual(config["identifier"], "com.anubisquantumcipher.nemesis")
        self.assertTrue(config["bundle"]["active"])
        self.assertEqual(config["bundle"]["targets"], ["app"])
        self.assertEqual(config["bundle"]["resources"], EXPECTED_RESOURCES)


if __name__ == "__main__":
    unittest.main()
