#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
./scripts/build_ada.sh
cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo build --manifest-path runtime/Cargo.toml --workspace
python3 scripts/run_vertical_slice.py "$@"
