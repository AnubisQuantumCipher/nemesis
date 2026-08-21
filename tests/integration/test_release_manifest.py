from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path

from scripts.release_manifest import build_manifest


class ReleaseManifestTests(unittest.TestCase):
    def test_manifest_derives_asset_identity_from_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            asset = Path(temporary) / "NEMESIS-Desktop-v0.1.0-macos-arm64.zip"
            asset.write_bytes(b"release bytes")

            manifest = build_manifest(
                asset=asset,
                version="0.1.0",
                commit="1" * 40,
                tree="2" * 40,
                architecture="arm64",
            )

            self.assertEqual(manifest["schema"], "nemesis.release/v1")
            self.assertEqual(manifest["tag"], "v0.1.0")
            self.assertEqual(manifest["source"]["commit"], "1" * 40)
            self.assertEqual(manifest["source"]["tree"], "2" * 40)
            self.assertEqual(manifest["artifact"]["bytes"], len(b"release bytes"))
            self.assertEqual(
                manifest["artifact"]["sha256"],
                hashlib.sha256(b"release bytes").hexdigest(),
            )
            self.assertEqual(manifest["artifact"]["signing"], "ad-hoc")
            self.assertFalse(manifest["artifact"]["notarized"])


if __name__ == "__main__":
    unittest.main()
