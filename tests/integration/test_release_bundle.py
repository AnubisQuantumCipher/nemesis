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
    "../../runtime/target/release/nemesis-replay": "bin/nemesis-replay",
    "../../docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md": "docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md",
    "../../LICENSE": "LICENSE",
    "../../build/release/THIRD_PARTY_NOTICES.txt": "THIRD_PARTY_NOTICES.txt",
}


class ReleaseBundleTests(unittest.TestCase):
    def test_release_config_bundles_complete_standalone_runtime(self) -> None:
        config = json.loads(RELEASE_CONFIG.read_text(encoding="utf-8"))

        self.assertEqual(config["identifier"], "com.anubisquantumcipher.nemesis")
        self.assertTrue(config["bundle"]["active"])
        self.assertEqual(config["bundle"]["targets"], ["app"])
        self.assertEqual(config["bundle"]["resources"], EXPECTED_RESOURCES)

    def test_release_contract_with_private_paths_is_not_bundled(self) -> None:
        config = json.loads(RELEASE_CONFIG.read_text(encoding="utf-8"))

        self.assertNotIn(
            "../../docs/mission/NEMESIS_FULL_AUTONOMOUS_GITHUB_RELEASE_MISSION_2026-08-20.md",
            config["bundle"]["resources"],
        )


if __name__ == "__main__":
    unittest.main()
