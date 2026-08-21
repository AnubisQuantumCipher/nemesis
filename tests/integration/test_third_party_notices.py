from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.generate_third_party_notices import generate_notices


class ThirdPartyNoticesTests(unittest.TestCase):
    def test_notices_include_shipped_cargo_and_npm_license_texts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cargo_inventory = root / "cargo.json"
            cargo_inventory.write_text(
                json.dumps(
                    {
                        "crates": [
                            {
                                "package": {
                                    "name": "crate-a",
                                    "version": "1.2.3",
                                    "license": "MIT",
                                }
                            }
                        ],
                        "licenses": [
                            {
                                "id": "MIT",
                                "text": "Cargo MIT license text",
                                "used_by": [
                                    {"crate": {"name": "crate-a", "version": "1.2.3"}}
                                ],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            desktop = root / "desktop"
            package = desktop / "node_modules/package-b"
            package.mkdir(parents=True)
            (package / "LICENSE").write_text("NPM MIT license text", encoding="utf-8")
            (desktop / "package-lock.json").write_text(
                json.dumps(
                    {
                        "packages": {
                            "": {"name": "app", "version": "0.1.0"},
                            "node_modules/package-b": {
                                "version": "4.5.6",
                                "license": "MIT",
                            },
                            "node_modules/dev-only": {
                                "version": "9.9.9",
                                "license": "MIT",
                                "dev": True,
                            },
                        }
                    }
                ),
                encoding="utf-8",
            )

            notices = generate_notices([cargo_inventory], desktop)

            self.assertIn("crate-a 1.2.3 — MIT", notices)
            self.assertIn("package-b 4.5.6 — MIT", notices)
            self.assertIn("Cargo MIT license text", notices)
            self.assertIn("NPM MIT license text", notices)
            self.assertNotIn("dev-only", notices)


if __name__ == "__main__":
    unittest.main()
