#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/build_ada.sh
./build/bin/nemesis_kernel_tests
./build/bin/nemesis_authority_tests
./build/bin/nemesis_authority_store_tests
./build/bin/nemesis_path_tests
python3 -m unittest tests.phase0.test_tcb_budget tests.phase0.test_kernel_proof_manifest -v
python3 scripts/check_tcb_budget.py
./scripts/prove_kernel.sh
python3 scripts/verify_proof_aba.py
python3 scripts/verify_authority_aba.py
printf '%s\n' 'PASS_PHASE4_CAPABILITY_POLICY'
