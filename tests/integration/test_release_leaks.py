from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.check_release_leaks import find_leaks


class ReleaseLeakTests(unittest.TestCase):
    def test_private_build_path_and_token_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "artifact.bin"
            artifact.write_bytes(
                b"prefix /Users/private-builder/project "
                b"github_pat_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 suffix"
            )

            leaks = find_leaks(artifact.read_bytes())

            self.assertEqual(
                {leak["kind"] for leak in leaks},
                {"private_user_path", "github_token"},
            )


if __name__ == "__main__":
    unittest.main()
