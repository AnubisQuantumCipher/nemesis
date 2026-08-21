from __future__ import annotations

import unittest
from pathlib import Path

from scripts.run_vertical_slice import executable_paths


class ReleaseRuntimeTests(unittest.TestCase):
    def test_prebuilt_layout_resolves_every_backend_executable(self) -> None:
        root = Path("/source")
        prebuilt = Path("/bundle/bin")

        paths = executable_paths(root, prebuilt)

        self.assertEqual(
            paths,
            {
                "daemon": prebuilt / "nemesis_core_daemon",
                "lane_create": prebuilt / "nemesis-lane-create",
                "worker_runner": prebuilt / "nemesis-worker-runner",
                "worker": prebuilt / "nemesis-deterministic-worker",
                "signer": prebuilt / "nemesis-signer",
                "verifier": prebuilt / "nemesis-verify",
            },
        )


if __name__ == "__main__":
    unittest.main()
