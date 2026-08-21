#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml -p nemesis-runtime --test evidence_replay
cargo test --manifest-path runtime/Cargo.toml -p nemesis-receipt
npm --prefix desktop test
cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib
python3 scripts/verify_evidence_bundle.py receipts/phase-11
runtime/target/debug/nemesis-replay --ledger receipts/phase-11/events.ledger >/dev/null
runtime/target/debug/nemesis-verify \
  --receipt receipts/phase-11/receipt.cose \
  --public-key receipts/phase-11/receipt.pub >/dev/null
if runtime/target/debug/nemesis-verify \
  --receipt receipts/phase-11/receipt-tampered.cose \
  --public-key receipts/phase-11/receipt.pub >/dev/null; then
  printf '%s\n' 'FAIL_PHASE11_TAMPER_ACCEPTED' >&2
  exit 1
fi
printf '%s\n' 'PASS_PHASE11_EVIDENCE_REPLAY'
