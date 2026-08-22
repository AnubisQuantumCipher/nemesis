#!/usr/bin/env bash
# Phase F — reliability, recovery, and data-integrity aggregate gate.
# Exercises desktop local-home/settings/last-mission recovery seams and the
# Ada daemon lifecycle/partial-read/concurrency seams. Every negative path must
# reach its intended semantic refusal; a syntax error, panic, or missing fixture
# is not valid negative evidence.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DAEMON="build/bin/nemesis_core_daemon"
if [[ ! -x "$DAEMON" ]]; then
  echo "FAIL_PRODUCTION_RELIABILITY missing_daemon=$DAEMON" >&2
  exit 1
fi

# Desktop recovery seams (Phase F: F-02, F-04, F-05, F-07, F-10, F-11, F-12, F-14).
cargo test --manifest-path desktop/src-tauri/Cargo.toml --test reliability

# Daemon durability seams (Phase F: F-03, F-06, F-13; corrupt/stale F-04).
python3 -m unittest \
  tests.integration.test_daemon_api \
  -v

printf '%s\n' 'PASS_PRODUCTION_RELIABILITY'
