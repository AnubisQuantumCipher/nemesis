#!/usr/bin/env python3
"""Derive NEMESIS release metadata and checksums from final artifact bytes."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


DESKTOP_CONTRACT_SHA256 = (
    "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779"
)
RELEASE_CONTRACT_SHA256 = (
    "8f62d762c30849ec6841b0fbde87004ced41365b456c98c79f6f1bf53c347ccc"
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_manifest(
    *,
    asset: Path,
    version: str,
    commit: str,
    tree: str,
    architecture: str,
) -> dict[str, Any]:
    data = asset.read_bytes()
    return {
        "schema": "nemesis.release/v1",
        "version": version,
        "tag": f"v{version}",
        "source": {
            "commit": commit,
            "tree": tree,
        },
        "contracts": {
            "desktop_sha256": DESKTOP_CONTRACT_SHA256,
            "release_sha256": RELEASE_CONTRACT_SHA256,
        },
        "artifact": {
            "name": asset.name,
            "bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
            "media_type": "application/zip",
            "platform": "macOS",
            "architecture": architecture,
            "minimum_macos": "14.0",
            "signing": "ad-hoc",
            "notarized": False,
        },
        "nonclaims": [
            "The artifact is not Developer ID signed or notarized.",
            "Gatekeeper approval and App Store distribution are not claimed.",
            "Windows, Linux, mobile, web, and remote-public surfaces are not shipped.",
            "SPARK evidence is bounded to the named kernel units and assumptions.",
        ],
    }


def write_outputs(manifest: dict[str, Any], asset: Path, output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "release-manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    checksums = output_dir / "SHA256SUMS"
    checksums.write_text(
        f"{sha256(asset)}  {asset.name}\n"
        f"{sha256(manifest_path)}  {manifest_path.name}\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--asset", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--tree", required=True)
    parser.add_argument("--architecture", required=True)
    parser.add_argument("--output-dir", required=True, type=Path)
    arguments = parser.parse_args()
    asset = arguments.asset.resolve()
    manifest = build_manifest(
        asset=asset,
        version=arguments.version,
        commit=arguments.commit,
        tree=arguments.tree,
        architecture=arguments.architecture,
    )
    write_outputs(manifest, asset, arguments.output_dir.resolve())
    print(
        "PASS_RELEASE_MANIFEST "
        f"asset={asset.name} bytes={manifest['artifact']['bytes']} "
        f"sha256={manifest['artifact']['sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
