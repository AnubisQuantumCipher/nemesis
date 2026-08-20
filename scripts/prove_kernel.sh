#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

alr exec -- env -u LIBRARY_PATH gnatprove \
  -P "$ROOT/nemesis.gpr" \
  -u nemesis-kernel-types.adb \
     nemesis-kernel-transitions.adb \
     nemesis-kernel-missions.adb \
  --mode=all \
  --level=2 \
  --checks-as-errors=on \
  --warnings=error \
  --report=statistics

printf '%s\n' 'PASS_KERNEL_PROOF_BASELINE'
