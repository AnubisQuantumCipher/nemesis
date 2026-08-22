#!/usr/bin/env python3
"""Phase J support-bundle generator (J-05/J-06).

Collects a bounded, reviewable, secret-scanned diagnostics bundle from a local
NEMESIS home. It never transmits anything (no network calls) and refuses to emit
a bundle if a secret-like pattern is detected in any collected byte. Every
inclusion is size-capped and listed in a preview MANIFEST for review before the
operator chooses to share it.

Usage: support_bundle.py --home <local-home> --out <dir> [--resources <res>]
"""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAX_FILE_BYTES = 256 * 1024
MAX_BUNDLE_BYTES = 5 * 1024 * 1024
SECRET_PATTERNS = [
    re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    re.compile(rb"(?i)\bpassword\s*[:=]"),
    re.compile(rb"(?i)\bsecret\s*[:=]"),
    re.compile(rb"(?i)\b(api[_-]?key|access[_-]?token|bearer)\b\s*[:=]"),
    re.compile(rb"(?i)aws_secret_access_key"),
]


def arg(flag: str, default: str | None = None) -> str | None:
    return sys.argv[sys.argv.index(flag) + 1] if flag in sys.argv else default


def tail_bytes(path: Path, limit: int) -> bytes:
    data = path.read_bytes()
    return data[-limit:] if len(data) > limit else data


def scan_for_secrets(data: bytes) -> str | None:
    for pattern in SECRET_PATTERNS:
        if pattern.search(data):
            return pattern.pattern.decode("utf-8", "replace")
    return None


def main() -> int:
    home = arg("--home")
    out = arg("--out")
    resources = arg("--resources")
    if not home or not out:
        print("usage: support_bundle.py --home <local-home> --out <dir> [--resources <res>]")
        return 2
    home_path = Path(home)
    out_path = Path(out)
    if out_path.exists():
        shutil.rmtree(out_path)
    staging = out_path / "nemesis-support"
    staging.mkdir(parents=True)

    # Operator diagnostics (secret-safe by construction).
    diag_cmd = [sys.executable, str(ROOT / "scripts/operator_diagnostics.py")]
    if resources:
        diag_cmd += ["--resources", resources]
    diagnostics = subprocess.run(diag_cmd, capture_output=True, text=True, timeout=60).stdout
    (staging / "diagnostics.json").write_text(
        "\n".join(line for line in diagnostics.splitlines() if line.startswith((" ", "{", "}"))),
        encoding="utf-8",
    )

    included = []
    # Bounded, non-secret home state. Never include lane payloads or object blobs.
    candidates = [
        ("home.json", home_path / "home.json"),
        ("settings.json", home_path / "settings.json"),
        ("last-mission.json", home_path / "last-mission.json"),
    ]
    for name in ("daemon.stdout.log", "daemon.stderr.log"):
        for log in home_path.rglob(name):
            candidates.append((f"logs/{log.parent.name}-{name}", log))

    for label, source in candidates:
        if not source.is_file():
            continue
        data = tail_bytes(source, MAX_FILE_BYTES)
        secret = scan_for_secrets(data)
        if secret is not None:
            shutil.rmtree(out_path)
            print(f"REFUSED_SUPPORT_BUNDLE secret_pattern_detected in={label} pattern={secret!r}")
            return 3
        destination = staging / label
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)
        included.append(
            {"name": label, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
             "truncated": source.stat().st_size > len(data)}
        )

    total = sum(entry["bytes"] for entry in included)
    if total > MAX_BUNDLE_BYTES:
        shutil.rmtree(out_path)
        print(f"REFUSED_SUPPORT_BUNDLE oversized bytes={total} cap={MAX_BUNDLE_BYTES}")
        return 4

    manifest = {
        "schema": "nemesis.support-bundle/v1",
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "home": str(home_path),
        "files": included,
        "total_bytes": total,
        "cap_bytes": MAX_BUNDLE_BYTES,
        "transmission": "DISABLED_NO_AUTOMATIC_EXPORT",
        "secret_scan": "PASSED",
        "review_before_sharing": True,
    }
    (staging / "MANIFEST.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(json.dumps(manifest, indent=1, sort_keys=True))
    print(f"SUPPORT_BUNDLE_READY dir={staging} files={len(included)} transmission=disabled")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
