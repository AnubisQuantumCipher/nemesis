#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
python3 -m unittest tests.phase0.test_phase0 -v
printf '%s\n' 'PASS_PHASE0'
