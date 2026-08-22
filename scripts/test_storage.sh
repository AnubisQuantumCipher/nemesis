#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

./scripts/build_ada.sh
./build/bin/nemesis_kernel_tests
./build/bin/nemesis_ledger_tests
./build/bin/nemesis_storage_tests
./build/bin/nemesis_authority_store_tests
./build/bin/nemesis_index_tests
printf '%s\n' 'PASS_PHASE3_DURABLE_STORAGE'
