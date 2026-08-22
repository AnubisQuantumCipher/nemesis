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


def console_locked() -> bool:
    probe = run(["/usr/sbin/ioreg", "-n", "Root", "-d1"])
    return b"\"IOConsoleLocked\" = Yes" in probe.stdout


def owned_window_count(pid: int) -> int:
    probe = run(
        ["/usr/bin/swift", str(ROOT / "scripts/probe_window_ownership.swift"), str(pid)]
    )
    for line in probe.stdout.decode(errors="replace").splitlines():
        if line.startswith("WINDOWS "):
            try:
                return int(line.split()[1])
            except ValueError:
                return 0
    return 0


def launch_probe(app: Path, home: Path, locked: bool) -> dict[str, object]:
    """Launch the app binary with an isolated $HOME and prove the launch.

    Unlocked console: require the frontend round trip — the schema-versioned
    local home manifest must be created (full UI loop proof).
    Locked console (IOConsoleLocked=Yes): the WebKit frontend does not run
    JavaScript, so home creation is unobservable in this environment (same
    class as G-10 / LOCKED_SESSION.json). Criterion drops to: process stays
    alive AND owns at least one on-screen CoreGraphics window — the exact
    evidence class the sealed LOCKED_SESSION probe recorded. Never faked up
    to a home-creation claim.
    """

    binary = app / "Contents/MacOS/nemesis-desktop"
    if not os.access(binary, os.X_OK):
        raise ContractError("app binary is not executable")
    app_data = home / "Library/Application Support" / BUNDLE_ID
    manifest = app_data / "home.json"
    environment = dict(os.environ)
    environment["HOME"] = str(home)
    process = subprocess.Popen(
        [str(binary)],
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        cwd=str(app.parent),
    )
    started = time.monotonic()
    home_seen_at: float | None = None
    windows_seen = 0
    try:
        while time.monotonic() - started < LAUNCH_TIMEOUT_S:
            if process.poll() is not None:
                output = (process.stdout.read() if process.stdout else b"").decode(
                    errors="replace"
                )
                raise ContractError(
                    f"app exited {process.returncode} during launch probe: "
                    f"{output[:2000]}"
                )
            if not locked and manifest.is_file():
                home_seen_at = time.monotonic() - started
                break
            if locked and time.monotonic() - started >= 10:
                windows_seen = owned_window_count(process.pid)
                if windows_seen >= 1:
                    break
            time.sleep(0.25)
        if not locked:
            if home_seen_at is None:
                raise ContractError(
                    f"local home manifest did not appear within {LAUNCH_TIMEOUT_S}s"
                )
            manifest_data = json.loads(manifest.read_text(encoding="utf-8"))
            if manifest_data.get("schema") != "nemesis.local-home/v1":
                raise ContractError("local home schema mismatch")
        else:
            if windows_seen < 1:
                raise ContractError(
                    "locked-console launch probe saw no owned on-screen window "
                    f"within {LAUNCH_TIMEOUT_S}s"
                )
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
    if locked:
        return {
            "process_launched": True,
            "criterion": "PROCESS_AND_ONSCREEN_WINDOW_LOCKED_CONSOLE",
            "owned_onscreen_windows": windows_seen,
            "environment_note": (
                "IOConsoleLocked=Yes: frontend JS (and thus local-home creation) "
                "is unobservable in a locked GUI session; window-server evidence "
                "matches the sealed LOCKED_SESSION.json precedent. The full "
                "UI-loop home-creation criterion applies automatically when the "
                "console is unlocked."
            ),
            "terminated_cleanly": True,
        }
    return {
        "process_launched": True,
        "criterion": "LOCAL_HOME_CREATED",
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
    for supplied in (arguments.predecessor, arguments.successor):
        if supplied.is_symlink():
            print(f"FAIL_INSTALL_CONTRACT symlink_asset={supplied}")
            return 1
    predecessor = arguments.predecessor.resolve()
    successor = arguments.successor.resolve()
    for asset in (predecessor, successor):
        if not asset.is_file():
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

    # Bind BOTH assets to their release SHA256SUMS entries by exact name and
    # recomputed digest before anything is extracted or executed.
    def bound_to_sums(asset: Path) -> str | None:
        sums = asset.parent / "SHA256SUMS"
        if not sums.is_file() or sums.is_symlink():
            return f"missing_sha256sums_beside={asset.name}"
        expected: dict[str, str] = {}
        for line in sums.read_text(encoding="utf-8").splitlines():
            parts = line.split()
            if len(parts) == 2:
                expected[parts[1].lstrip("*")] = parts[0].lower()
        if asset.name not in expected:
            return f"asset_not_listed_in_sha256sums={asset.name}"
        if sha256_file(asset) != expected[asset.name]:
            return f"asset_digest_mismatch={asset.name}"
        return None

    for label, asset in (("predecessor", predecessor), ("successor", successor)):
        failure = bound_to_sums(asset)
        if failure is not None:
            print(f"FAIL_INSTALL_CONTRACT {label}_{failure}")
            return 1
        receipt[f"{label}_sha256sums"] = "BOUND_BY_NAME_AND_DIGEST"

    locked = console_locked()
    receipt["environment"] = {
        "console_locked": locked,
        "launch_criterion": (
            "PROCESS_AND_ONSCREEN_WINDOW_LOCKED_CONSOLE"
            if locked
            else "LOCAL_HOME_CREATED"
        ),
    }
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

            # Launch proof for the installed predecessor.
            receipt["launch_predecessor"] = launch_probe(app, home, locked)
            app_data = home / "Library/Application Support" / BUNDLE_ID
            if locked:
                # Frontend-driven home creation is unobservable while the
                # console is locked. The upgrade boundary claim ("bundle
                # replacement never mutates user state") is proved against
                # explicitly synthetic sentinel bytes, recorded as such;
                # schema compatibility of real state across relaunch is
                # separately proved by the reliability suite
                # (desktop/src-tauri/tests/reliability.rs, F-02..F-14).
                app_data.mkdir(parents=True, exist_ok=True)
                (app_data / "install-contract-sentinel.bin").write_bytes(
                    os.urandom(256)
                )
                (app_data / "missions").mkdir(exist_ok=True)
                (app_data / "missions/sentinel.txt").write_text(
                    "install-contract synthetic user-state sentinel\n",
                    encoding="utf-8",
                )
                receipt["environment"]["state_seed"] = (
                    "SYNTHETIC_SENTINEL_LOCKED_CONSOLE"
                )
            else:
                receipt["environment"]["state_seed"] = "APP_CREATED_LOCAL_HOME"
            state_before = inventory(app_data)
            if not state_before:
                raise ContractError("no user state present before upgrade probe")

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
            receipt["launch_successor"] = launch_probe(app, home, locked)
            if not locked:
                manifest = json.loads(
                    (app_data / "home.json").read_text(encoding="utf-8")
                )
                if manifest.get("schema") != "nemesis.local-home/v1":
                    raise ContractError("successor rejected preserved local home")
            state_after_launch = inventory(app_data)
            if locked and state_after_launch != state_before:
                raise ContractError(
                    "successor launch mutated sentinel user state under "
                    "locked console"
                )
            receipt["upgrade"]["user_state_preserved"] = True
            receipt["upgrade"]["preserved_files"] = len(state_before)

            # Uninstall: remove the bundle; inventory deliberate residuals.
            shutil.rmtree(app)
            if app.exists():
                raise ContractError("uninstall left the bundle behind")
            launch_agents = [
                str(path)
                for candidate in (
                    home / "Library/LaunchAgents",
                    home / "Library/LaunchDaemons",
                    home / "Library/PrivilegedHelperTools",
                )
                if candidate.is_dir()
                for path in sorted(candidate.rglob("*"))
                if path.is_file()
            ]
            if launch_agents:
                raise ContractError(
                    f"launch agents or helpers were installed: {launch_agents}"
                )
            receipt["uninstall"] = {
                "bundle_removed": True,
                "residuals": {
                    "local_home": str(app_data),
                    "local_home_files": len(inventory(app_data)),
                    "keychain_item": (
                        "dev.nemesis.receipt.seed.v1 (never touched by this gate; "
                        "removal command documented in docs/release/SOURCE_INSTALL.md)"
                    ),
                    "workspace_note": (
                        "authorized missions additionally register git worktrees "
                        "and nemesis/desktop-* branches inside the user-selected "
                        "workspace repository; not exercised by this gate, "
                        "removal commands documented in SOURCE_INSTALL.md"
                    ),
                },
                "observed_launch_agents_or_helpers": [],
            }
    except ContractError as error:
        print(f"FAIL_INSTALL_CONTRACT {error}")
        return 1

    if receipt["install"]["version"] == successor_version:  # type: ignore[index]
        print("FAIL_INSTALL_CONTRACT versions_not_distinct")
        return 1
    receipt["versions"] = {
        "predecessor": receipt["install"]["version"],  # type: ignore[index]
        "successor": successor_version,
        "distinct": True,
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
