#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo audit --file runtime/Cargo.lock
cargo audit --file desktop/src-tauri/Cargo.lock
(
  cd desktop
  npm audit
)
./scripts/build_ada.sh
./build/bin/nemesis_kernel_tests
./build/bin/nemesis_authority_tests
./build/bin/nemesis_path_tests
./build/bin/nemesis_ledger_tests
./build/bin/nemesis_storage_tests
./build/bin/nemesis_index_tests
./scripts/prove_kernel.sh
./scripts/run_security_hardening.sh
printf '%s\n' 'PASS_PHASE16_SECURITY_HARDENING'
