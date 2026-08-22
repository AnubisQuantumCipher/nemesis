#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RECEIPT="$ROOT/receipts/production-readiness-20260821/FORMAL_KERNEL.json"
rm -f "$RECEIPT"
rm -rf "$ROOT/build/proof" "$ROOT/build/obj/debug/gnatprove"

UNIT_NAMES=()
for SOURCE in "$ROOT"/kernel/src/nemesis-kernel-*.adb; do
  if [[ ! -f "$SOURCE" ]]; then
    printf '%s\n' 'REFUSED_KERNEL_PROOF_EMPTY_SOURCE_ROSTER' >&2
    exit 2
  fi
  UNIT_NAMES+=("${SOURCE##*/}")
done
if [[ "${#UNIT_NAMES[@]}" -eq 0 ]]; then
  printf '%s\n' 'REFUSED_KERNEL_PROOF_EMPTY_SOURCE_ROSTER' >&2
  exit 2
fi

alr exec -- env -u LIBRARY_PATH gnatprove \
  -P "$ROOT/nemesis.gpr" \
  -u "${UNIT_NAMES[@]}" \
  --mode=all \
  --level=2 \
  --checks-as-errors=on \
  --warnings=error \
  --report=statistics

if [[ "${NEMESIS_PROOF_CHECK_ONLY:-0}" == "1" ]]; then
  python3 scripts/verify_kernel_proof.py
else
  python3 scripts/verify_kernel_proof.py --write "$RECEIPT"
fi
printf '%s\n' 'PASS_KERNEL_PROOF_BASELINE'
