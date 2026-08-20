#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/build_ada.sh
./build/bin/nemesis_kernel_tests
./build/bin/nemesis_authority_tests
./build/bin/nemesis_path_tests
python3 -m unittest tests.phase0.test_tcb_budget -v
python3 scripts/check_tcb_budget.py
./scripts/prove_kernel.sh
printf '%s\n' 'PASS_PHASE4_CAPABILITY_POLICY'
