#!/usr/bin/env python3
"""Phase J operator diagnostics (J-07/J-08).

Reports secret-safe Core/Kernel/Runtime identity, binary inventory, daemon health,
receipt-verifier and sandbox availability, and the network/update posture of the
installed product. Emits no secrets. Works in-tree (build/bin + runtime/target)
or against an installed app via --resources <App.app/Contents/Resources>.

Exit 0 and marker PASS_OPERATOR_DIAGNOSTICS when the Core daemon answers a ping
and the required backend executables are present; non-zero otherwise.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BACKENDS = [
    "nemesis_core_daemon",
    "nemesis-lane-create",
    "nemesis-worker-runner",
    "nemesis-deterministic-worker",
    "nemesis-signer",
    "nemesis-verify",
    "nemesis-replay",
]


def find_binary(name: str, resources: Path | None) -> Path | None:
    candidates = []
    if resources is not None:
        candidates.append(resources / "bin" / name)
    if name == "nemesis_core_daemon":
        candidates.append(ROOT / "build/bin" / name)
    else:
        candidates.append(ROOT / "runtime/target/debug" / name)
        candidates.append(ROOT / "runtime/target/release" / name)
        candidates.append(ROOT / "build/bin" / name)
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


def app_version() -> str:
    try:
        conf = json.loads((ROOT / "desktop/src-tauri/tauri.conf.json").read_text())
        return str(conf.get("version", "unknown"))
    except OSError:
        return "unknown"


def daemon_ping(daemon: Path) -> dict:
    with tempfile.TemporaryDirectory() as temporary:
        home = Path(temporary) / "home"
        home.mkdir()
        sock = home / "core.sock"
        process = subprocess.Popen(
            [str(daemon), "--home", str(home)],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        try:
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline and not sock.exists():
                time.sleep(0.005)
            if not sock.exists():
                return {"status": "FAIL", "reason": "socket not created"}
            request = b'{"command":"ping","schema":"nemesis.local/v1"}\n'
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                client.settimeout(5)
                client.connect(str(sock))
                client.sendall(request)
                response = bytearray()
                while not response.endswith(b"\n"):
                    chunk = client.recv(4096)
                    if not chunk:
                        break
                    response.extend(chunk)
            parsed = json.loads(response)
            return {"status": parsed.get("status", "UNKNOWN")}
        finally:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


def main() -> int:
    resources = None
    if "--resources" in sys.argv:
        resources = Path(sys.argv[sys.argv.index("--resources") + 1])

    inventory = {}
    for name in BACKENDS:
        path = find_binary(name, resources)
        inventory[name] = {
            "present": path is not None,
            "bytes": path.stat().st_size if path else None,
            "executable": bool(path and os.access(path, os.X_OK)),
        }

    daemon_path = find_binary("nemesis_core_daemon", resources)
    health = daemon_ping(daemon_path) if daemon_path else {"status": "UNAVAILABLE"}

    contract_rel = "docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"
    contract_sha = None
    contract_path = (resources / contract_rel) if resources else (ROOT / contract_rel)
    if contract_path.is_file():
        import hashlib
        contract_sha = hashlib.sha256(contract_path.read_bytes()).hexdigest()

    report = {
        "schema": "nemesis.operator-diagnostics/v1",
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "app_version": app_version(),
        "contract_sha256": contract_sha,
        "core_daemon_health": health,
        "backend_inventory": inventory,
        "receipt_verifier_present": inventory["nemesis-verify"]["present"],
        "replay_verifier_present": inventory["nemesis-replay"]["present"],
        "sandbox": "WORKSPACE_SAFE_AVAILABLE" if inventory["nemesis-worker-runner"]["present"] else "UNAVAILABLE",
        "network": "DENIED_BY_CONTRACT",
        "updates": "DISABLED_NO_AUTHENTICATED_UPDATER",
        "secrets_included": False,
    }
    print(json.dumps(report, indent=1, sort_keys=True))

    required_present = all(inventory[name]["present"] for name in BACKENDS)
    if health.get("status") == "OK" and required_present:
        print("PASS_OPERATOR_DIAGNOSTICS")
        return 0
    print(f"INCOMPLETE_OPERATOR_DIAGNOSTICS daemon={health.get('status')} all_backends={required_present}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
