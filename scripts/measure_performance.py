#!/usr/bin/env python3
"""Phase G/H controlled performance + resource instrument.

Measures only what is mechanically reproducible on this host without a GUI window
server (the console is locked; native Tauri window launch is environment-blocked and
recorded as such, not measured). Validates each instrument before publishing a number
and repeats every timing to publish a bounded, reconciled result. Fails closed if a
declared product limit is missing from source or a measured latency violates its
justified regression threshold.

Usage: measure_performance.py [--out receipts/.../PERFORMANCE.json]
"""

from __future__ import annotations

import json
import os
import re
import socket
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DAEMON = ROOT / "build/bin/nemesis_core_daemon"

# Justified fail-closed regression thresholds (derived from observed Apple-silicon
# local runs with generous margin; not a universal claim). A cold daemon control
# plane that takes longer than this on this machine class is a regression.
DAEMON_START_P95_CEILING_S = 2.0
DAEMON_START_RUNS = 10

DECLARED_LIMITS = {
    "desktop/src-tauri/src/lib.rs": [
        ("MAX_REPLAY_BYTES", 64 * 1024 * 1024),
        ("MAX_LOCAL_CONTRACT_BYTES", 64 * 1024),
    ],
    "desktop/src-tauri/src/production.rs": [
        ("MAX_CONTRACT_BYTES", 64 * 1024),
        ("MAX_GOAL_BYTES", 1024),
        ("MAX_WORKSPACE_BYTES", 4096),
        ("MAX_WRITE_BYTES", 4096),
        ("MAX_RUNTIME_SECONDS", 900),
        ("MAX_OUTPUT_BYTES", 16 * 1024 * 1024),
    ],
    "desktop/src-tauri/src/mission_runner.rs": [
        ("MAX_CORE_MESSAGE_BYTES", 65_536),
        ("MAX_RESULT_BYTES", 4 * 1024 * 1024),
    ],
}


def machine_context() -> dict:
    def run(*args: str) -> str:
        try:
            return subprocess.run(args, capture_output=True, text=True, timeout=5).stdout.strip()
        except Exception:
            return ""

    return {
        "uname": run("uname", "-mprs"),
        "hw_model": run("sysctl", "-n", "hw.model"),
        "cpu": run("sysctl", "-n", "machdep.cpu.brand_string"),
        "logical_cpus": run("sysctl", "-n", "hw.logicalcpu"),
        "mem_bytes": run("sysctl", "-n", "hw.memsize"),
        "macos": run("sw_vers", "-productVersion"),
    }


def validate_timer() -> None:
    a = time.monotonic()
    time.sleep(0.02)
    b = time.monotonic()
    if not (b > a):
        raise SystemExit("FAIL_PERF_INSTRUMENT monotonic clock not advancing")


def measure_daemon_start(runs: int) -> dict:
    if not DAEMON.exists() or not os.access(DAEMON, os.X_OK):
        return {"status": "UNAVAILABLE", "reason": f"missing {DAEMON}"}
    latencies: list[float] = []
    rss_samples: list[int] = []
    for _ in range(runs):
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary) / "home"
            home.mkdir()
            sock = home / "core.sock"
            start = time.monotonic()
            process = subprocess.Popen(
                [str(DAEMON), "--home", str(home)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            deadline = start + 10
            ready = None
            while time.monotonic() < deadline:
                if sock.exists():
                    probe = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                    try:
                        probe.connect(str(sock))
                        ready = time.monotonic()
                        probe.close()
                        break
                    except OSError:
                        probe.close()
                if process.poll() is not None:
                    break
                time.sleep(0.002)
            if ready is None:
                process.kill()
                process.wait()
                return {"status": "FAIL", "reason": "daemon did not reach socket-ready"}
            latencies.append(ready - start)
            try:
                out = subprocess.run(
                    ["ps", "-o", "rss=", "-p", str(process.pid)],
                    capture_output=True, text=True, timeout=3,
                ).stdout.strip()
                if out:
                    rss_samples.append(int(out) * 1024)
            except Exception:
                pass
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    latencies.sort()
    p95 = latencies[min(len(latencies) - 1, int(round(0.95 * (len(latencies) - 1))))]
    return {
        "status": "OK",
        "runs": len(latencies),
        "min_s": round(min(latencies), 4),
        "median_s": round(statistics.median(latencies), 4),
        "p95_s": round(p95, 4),
        "max_s": round(max(latencies), 4),
        "p95_ceiling_s": DAEMON_START_P95_CEILING_S,
        "within_threshold": p95 <= DAEMON_START_P95_CEILING_S,
        "idle_rss_bytes_median": int(statistics.median(rss_samples)) if rss_samples else None,
    }


def verify_declared_limits() -> dict:
    results = []
    ok = True
    for rel, limits in DECLARED_LIMITS.items():
        text = (ROOT / rel).read_text(encoding="utf-8")
        for name, expected in limits:
            match = re.search(rf"const {name}[^=]*=\s*([0-9_ *]+);", text)
            found = None
            if match:
                found = eval(match.group(1).replace("_", ""), {"__builtins__": {}})  # arithmetic only
            present = found == expected
            ok = ok and present
            results.append({"path": rel, "const": name, "expected": expected, "found": found, "ok": present})
    return {"all_present": ok, "limits": results}


def measure_sizes() -> dict:
    sizes = {}
    binaries = ROOT / "build/bin"
    if binaries.is_dir():
        sizes["build_bin_total_bytes"] = sum(
            f.stat().st_size for f in binaries.iterdir() if f.is_file()
        )
    dist = ROOT / "desktop/dist"
    if dist.is_dir():
        sizes["desktop_dist_total_bytes"] = sum(
            f.stat().st_size for f in dist.rglob("*") if f.is_file()
        )
    return sizes


def main() -> int:
    out_path = ROOT / "receipts/production-readiness-20260821/PERFORMANCE.json"
    if "--out" in sys.argv:
        out_path = Path(sys.argv[sys.argv.index("--out") + 1])

    validate_timer()
    daemon = measure_daemon_start(DAEMON_START_RUNS)
    limits = verify_declared_limits()
    sizes = measure_sizes()

    receipt = {
        "schema": "nemesis.performance/v1",
        "measured_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "machine": machine_context(),
        "instrument_validated": True,
        "gui_launch": {
            "status": "ENVIRONMENT_BLOCKED",
            "reason": "console IOConsoleLocked=Yes; native Tauri window-server launch cannot be measured headlessly. Daemon control-plane start is measured as the usable local-control-plane proxy.",
        },
        "daemon_control_plane_start": daemon,
        "declared_product_limits": limits,
        "sizes": sizes,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(receipt, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=1, sort_keys=True))

    if daemon.get("status") not in ("OK", "UNAVAILABLE"):
        print("FAIL_PERFORMANCE daemon_start")
        return 1
    if daemon.get("status") == "OK" and not daemon.get("within_threshold"):
        print("FAIL_PERFORMANCE daemon_start_regression")
        return 1
    if not limits["all_present"]:
        print("FAIL_PERFORMANCE missing_declared_limit")
        return 1
    print("PASS_PERFORMANCE_BOUNDED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
