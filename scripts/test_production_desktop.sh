#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

npm --prefix desktop ci
npm --prefix desktop test
npm --prefix desktop run build
cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check
cargo test --manifest-path desktop/src-tauri/Cargo.toml
cargo test \
  --manifest-path desktop/src-tauri/Cargo.toml \
  --test mission_execution \
  executes_reviewed_contract_and_persists_current_receipt_and_replay \
  -- --ignored --exact
python3 -m unittest tests.integration.test_release_bundle -v
printf '%s\n' 'PASS_PRODUCTION_DESKTOP_AUTOMATED'
