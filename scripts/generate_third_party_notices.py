#!/usr/bin/env python3
"""Generate deterministic third-party notices for shipped Rust and npm code."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


LICENSE_PREFIXES = ("license", "copying", "notice")


def cargo_sections(inventories: list[dict[str, Any]]) -> tuple[set[str], dict[str, set[str]]]:
    packages: set[str] = set()
    texts: dict[str, set[str]] = {}
    for inventory in inventories:
        for entry in inventory.get("crates", []):
            package = entry.get("package", {})
            name = package.get("name")
            version = package.get("version")
            license_expression = package.get("license") or entry.get("license") or "UNKNOWN"
            if name and version:
                packages.add(f"{name} {version} — {license_expression}")
        for license_entry in inventory.get("licenses", []):
            text = license_entry.get("text", "").strip()
            if not text:
                raise ValueError("cargo license entry omitted text")
            users = texts.setdefault(text, set())
            for usage in license_entry.get("used_by", []):
                crate = usage.get("crate", {})
                name = crate.get("name")
                version = crate.get("version")
                if name and version:
                    users.add(f"{name} {version}")
    return packages, texts


def npm_sections(desktop: Path) -> tuple[set[str], dict[str, set[str]]]:
    lock = json.loads((desktop / "package-lock.json").read_text(encoding="utf-8"))
    packages: set[str] = set()
    texts: dict[str, set[str]] = {}
    for location, package in lock.get("packages", {}).items():
        if not location.startswith("node_modules/") or package.get("dev") is True:
            continue
        name = location.removeprefix("node_modules/")
        version = package.get("version")
        license_expression = package.get("license", "UNKNOWN")
        if not version:
            raise ValueError(f"npm package omitted version: {name}")
        package_id = f"{name} {version}"
        packages.add(f"{package_id} — {license_expression}")
        package_dir = desktop / location
        license_files = sorted(
            path
            for path in package_dir.iterdir()
            if path.is_file() and path.name.lower().startswith(LICENSE_PREFIXES)
        )
        if not license_files:
            raise ValueError(f"npm package omitted license file: {name}")
        for license_file in license_files:
            text = license_file.read_text(encoding="utf-8").strip()
            if not text:
                raise ValueError(f"npm package has empty license file: {name}")
            texts.setdefault(text, set()).add(package_id)
    return packages, texts


def generate_notices(cargo_json: list[Path], desktop: Path) -> str:
    inventories = [json.loads(path.read_text(encoding="utf-8")) for path in cargo_json]
    cargo_packages, cargo_texts = cargo_sections(inventories)
    npm_packages, npm_texts = npm_sections(desktop)
    combined_texts = dict(cargo_texts)
    for text, users in npm_texts.items():
        combined_texts.setdefault(text, set()).update(users)

    lines = [
        "NEMESIS THIRD-PARTY NOTICES",
        "===========================",
        "",
        "Generated from the locked Rust dependency graphs with cargo-about 0.9.2",
        "and from production npm packages present after `npm ci`.",
        "",
        "RUST PACKAGE INVENTORY",
        "----------------------",
        *sorted(cargo_packages, key=str.casefold),
        "",
        "SHIPPED NPM PACKAGE INVENTORY",
        "-----------------------------",
        *sorted(npm_packages, key=str.casefold),
        "",
        "LICENSE AND NOTICE TEXTS",
        "------------------------",
    ]
    for index, (text, users) in enumerate(
        sorted(combined_texts.items(), key=lambda item: (sorted(item[1]), item[0])),
        start=1,
    ):
        lines.extend(
            [
                "",
                f"[{index}] Used by: {', '.join(sorted(users, key=str.casefold))}",
                "",
                text,
            ]
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cargo-json", action="append", required=True, type=Path)
    parser.add_argument("--desktop", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    try:
        notices = generate_notices(arguments.cargo_json, arguments.desktop.resolve())
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        print(f"FAIL_THIRD_PARTY_NOTICES error={type(error).__name__}")
        return 1
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(notices, encoding="utf-8")
    print(f"PASS_THIRD_PARTY_NOTICES output={output} bytes={len(notices.encode())}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
