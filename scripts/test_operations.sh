#!/usr/bin/env bash
# Phase J — privacy and operations gate.
# Verifies operator diagnostics, bounded/secret-scanned/no-transmit support
# bundle (including the secret-refusal control), and the telemetry/network
# absence posture. Fails closed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# J-07/J-08 operator diagnostics.
python3 scripts/operator_diagnostics.py >/dev/null

# J-05/J-06 support bundle: happy path + secret-refusal control + no auto-transmit.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
HOME_DIR="$WORK/home"
mkdir -p "$HOME_DIR/.core/abc"
printf '{"schema":"nemesis.local-home/v1","schemaVersion":1}\n' >"$HOME_DIR/home.json"
printf '{"textScale":"standard","reduceMotion":true}\n' >"$HOME_DIR/settings.json"
printf 'daemon started\nsocket ready\n' >"$HOME_DIR/.core/abc/daemon.stdout.log"

bundle_out="$(python3 scripts/support_bundle.py --home "$HOME_DIR" --out "$WORK/out")"
printf '%s\n' "$bundle_out" | grep -q 'transmission=disabled'
printf '%s\n' "$bundle_out" | grep -q 'SUPPORT_BUNDLE_READY'

# Secret-refusal control: a planted secret must refuse and leave no bundle.
printf 'aws_secret_access_key = AKIAEXAMPLE\n' >"$HOME_DIR/.core/abc/daemon.stderr.log"
if python3 scripts/support_bundle.py --home "$HOME_DIR" --out "$WORK/out2" >/dev/null 2>&1; then
  echo "FAIL_PRODUCTION_OPERATIONS support bundle did not refuse a planted secret" >&2
  exit 1
fi
[[ ! -d "$WORK/out2" ]] || { echo "FAIL_PRODUCTION_OPERATIONS refused bundle left artifacts" >&2; exit 1; }

# J-01/J-02 network-egress absence in the desktop application code (no egress =>
# no telemetry transmission). Matches only real egress mechanisms, not UI names.
if grep -RInE 'reqwest|hyper::|\bureq\b|XMLHttpRequest|navigator\.sendBeacon|window\.fetch|[^.]\bfetch\(' \
    desktop/src desktop/src-tauri/src 2>/dev/null; then
  echo "FAIL_PRODUCTION_OPERATIONS unexpected network egress reference in desktop app" >&2
  exit 1
fi

printf '%s\n' 'PASS_PRODUCTION_OPERATIONS'
