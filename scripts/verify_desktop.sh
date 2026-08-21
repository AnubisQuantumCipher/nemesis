#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./script/build_and_run.sh --verify
python3 scripts/verify_evidence_bundle.py receipts/phase-7-desktop
runtime/target/debug/nemesis-verify \
  --receipt receipts/phase-7-desktop/receipt.cose \
  --public-key receipts/phase-7-desktop/receipt.pub >/dev/null
if runtime/target/debug/nemesis-verify \
  --receipt receipts/phase-7-desktop/receipt-tampered.cose \
  --public-key receipts/phase-7-desktop/receipt.pub >/dev/null; then
  printf '%s\n' 'FAIL_DESKTOP_TAMPER_ACCEPTED' >&2
  exit 1
fi
pkill -x nemesis-desktop >/dev/null 2>&1 || true
printf '%s\n' 'PASS_NEMESIS_DESKTOP_VERTICAL_SLICE'
