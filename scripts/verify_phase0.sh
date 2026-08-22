#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
python3 scripts/verify_production_contract.py
python3 -m unittest tests.phase0.test_phase0 tests.phase0.test_production_contract tests.phase0.test_kernel_proof_manifest -v
printf '%s\n' 'PASS_PHASE0'
