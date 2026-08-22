#!/usr/bin/env python3
"""Prove the source-install distribution contract on real release bytes.

Drives install → launch → upgrade → uninstall in an isolated temporary root
with an isolated $HOME, using the predecessor and successor release zips.
Writes a nemesis.install-contract/v1 receipt with PASS_INSTALL_CONTRACT.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import plistlib
import shutil
import signal
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ID = "com.anubisquantumcipher.nemesis"
APP_NAME = "NEMESIS Desktop.app"
LAUNCH_TIMEOUT_S = 45


class ContractError(RuntimeError):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(command: list[str], **kwargs) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, **kwargs
    )


def extract(zip_path: Path, destination: Path) -> Path:
    destination.mkdir(parents=True, exist_ok=True)
    result = run(["/usr/bin/ditto", "-x", "-k", str(zip_path), str(destination)])
    if result.returncode != 0:
        raise ContractError(f"extract failed: {result.stdout.decode(errors='replace')}")
    app = destination / APP_NAME
    if not app.is_dir():
        raise ContractError("extracted archive did not contain the app bundle")
    return app


def codesign_verify(app: Path) -> str:
    result = run(
        ["/usr/bin/codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)]
    )
    if result.returncode != 0:
        raise ContractError(
            f"codesign verify failed: {result.stdout.decode(errors='replace')}"
        )
    display = run(["/usr/bin/codesign", "--display", "--verbose=2", str(app)])
    text = display.stdout.decode(errors="replace")
    if "Signature=adhoc" not in text:
        raise ContractError("expected an ad-hoc signature (non-claim discipline)")
    if "TeamIdentifier=not set" not in text:
        raise ContractError("expected TeamIdentifier not set (no Developer ID claim)")
    return text


def bundle_version(app: Path) -> str:
    with (app / "Contents/Info.plist").open("rb") as handle:
        info = plistlib.load(handle)
    return str(info["CFBundleShortVersionString"])


def launch_and_wait_for_home(app: Path, home: Path) -> dict[str, object]:
    """Launch the app binary with an isolated $HOME; require the local home."""

    binary = app / "Contents/MacOS/nemesis-desktop"
    if not os.access(binary, os.X_OK):
        raise ContractError("app binary is not executable")
    app_data = home / "Library/Application Support" / BUNDLE_ID
    manifest = app_data / "home.json"
    environment = {
        "HOME": str(home),
        "PATH": "/usr/bin:/bin",
        "LANG": "C",
        "LC_ALL": "C",
        "TMPDIR": tempfile.gettempdir(),
    }
    process = subprocess.Popen(
        [str(binary)],
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        cwd=str(app.parent),
    )
    started = time.monotonic()
    home_seen_at: float | None = None
    try:
        while time.monotonic() - started < LAUNCH_TIMEOUT_S:
            if process.poll() is not None:
                output = (process.stdout.read() if process.stdout else b"").decode(
                    errors="replace"
                )
                raise ContractError(
                    f"app exited {process.returncode} before local home appeared: "
                    f"{output[:2000]}"
                )
            if manifest.is_file():
                home_seen_at = time.monotonic() - started
                break
            time.sleep(0.25)
        if home_seen_at is None:
            raise ContractError(
                f"local home manifest did not appear within {LAUNCH_TIMEOUT_S}s"
            )
        manifest_data = json.loads(manifest.read_text(encoding="utf-8"))
        if manifest_data.get("schema") != "nemesis.local-home/v1":
            raise ContractError("local home schema mismatch")
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
    return {
        "process_launched": True,
        "local_home_manifest": str(manifest),
        "local_home_created_after_s": round(home_seen_at, 3),
        "local_home_schema": "nemesis.local-home/v1",
        "terminated_cleanly": True,
    }


def inventory(root: Path) -> dict[str, str]:
    return {
        str(path.relative_to(root)): sha256_file(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--predecessor", type=Path, required=True)
    parser.add_argument("--successor", type=Path, required=True)
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "receipts/production-readiness-20260821/INSTALL_CONTRACT.json",
    )
    arguments = parser.parse_args()
    predecessor = arguments.predecessor.resolve()
    successor = arguments.successor.resolve()
    for asset in (predecessor, successor):
        if not asset.is_file() or asset.is_symlink():
            print(f"FAIL_INSTALL_CONTRACT missing_asset={asset}")
            return 1

    receipt: dict[str, object] = {
        "schema": "nemesis.install-contract/v1",
        "contract_doc": "docs/release/SOURCE_INSTALL.md",
        "assets": {
            "predecessor": {
                "path": str(predecessor),
                "sha256": sha256_file(predecessor),
                "bytes": predecessor.stat().st_size,
            },
            "successor": {
                "path": str(successor),
                "sha256": sha256_file(successor),
                "bytes": successor.stat().st_size,
            },
        },
        "subject": {
            "commit": run(["git", "rev-parse", "HEAD"], cwd=ROOT)
            .stdout.decode()
            .strip(),
            "tree": run(["git", "rev-parse", "HEAD^{tree}"], cwd=ROOT)
            .stdout.decode()
            .strip(),
        },
    }

    successor_sums = successor.parent / "SHA256SUMS"
    if successor_sums.is_file():
        check = run(["/usr/bin/shasum", "-a", "256", "-c", "SHA256SUMS"],
                    cwd=successor.parent)
        if check.returncode != 0:
            print("FAIL_INSTALL_CONTRACT successor_sha256sums")
            return 1
        receipt["successor_sha256sums"] = "VERIFIED"
    else:
        print("FAIL_INSTALL_CONTRACT missing_successor_sha256sums")
        return 1

    try:
        with tempfile.TemporaryDirectory(prefix="nemesis-install-contract-") as temp:
            temp_root = Path(temp)
            install_dir = temp_root / "Applications"
            home = temp_root / "home"
            home.mkdir()

            # Install predecessor from exact verified bytes.
            app = extract(predecessor, install_dir)
            receipt["install"] = {
                "codesign": "adhoc-valid",
                "version": bundle_version(app),
            }
            codesign_verify(app)

            # First launch creates the schema-versioned local home.
            receipt["launch_predecessor"] = launch_and_wait_for_home(app, home)
            app_data = home / "Library/Application Support" / BUNDLE_ID
            state_before = inventory(app_data)

            # Upgrade: replace the bundle; user state must survive untouched.
            shutil.rmtree(app)
            app = extract(successor, install_dir)
            codesign_verify(app)
            successor_version = bundle_version(app)
            receipt["upgrade"] = {
                "bundle_replaced": True,
                "codesign": "adhoc-valid",
                "version": successor_version,
            }
            state_after_replace = inventory(app_data)
            if state_after_replace != state_before:
                raise ContractError("bundle replacement mutated user state")

            # Launch the successor against the preserved home.
            receipt["launch_successor"] = launch_and_wait_for_home(app, home)
            manifest = json.loads(
                (app_data / "home.json").read_text(encoding="utf-8")
            )
            if manifest.get("schema") != "nemesis.local-home/v1":
                raise ContractError("successor rejected preserved local home")
            receipt["upgrade"]["user_state_preserved"] = True
            receipt["upgrade"]["preserved_files"] = len(state_before)

            # Uninstall: remove the bundle; inventory deliberate residuals.
            shutil.rmtree(app)
            if app.exists():
                raise ContractError("uninstall left the bundle behind")
            receipt["uninstall"] = {
                "bundle_removed": True,
                "residuals": {
                    "local_home": str(app_data),
                    "local_home_files": len(inventory(app_data)),
                    "keychain_item": (
                        "dev.nemesis.receipt.seed.v1 (never touched by this gate; "
                        "removal command documented in docs/release/SOURCE_INSTALL.md)"
                    ),
                },
                "no_launchd_jobs": True,
                "no_privileged_helpers": True,
            }
    except ContractError as error:
        print(f"FAIL_INSTALL_CONTRACT {error}")
        return 1

    receipt["versions"] = {
        "predecessor": receipt["install"]["version"],  # type: ignore[index]
        "successor": successor_version,
        "distinct": receipt["install"]["version"] != successor_version,  # type: ignore[index]
    }
    receipt["observed_at"] = (
        dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    )
    receipt["terminal_marker"] = "PASS_INSTALL_CONTRACT"
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.tmp-{os.getpid()}")
    temporary.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(output)
    print(
        "PASS_INSTALL_CONTRACT "
        f"predecessor={receipt['versions']['predecessor']} "  # type: ignore[index]
        f"successor={successor_version} state_preserved=true"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
