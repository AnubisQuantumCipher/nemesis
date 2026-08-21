#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml -p nemesis-runtime --test adapters
cargo run --quiet --manifest-path runtime/Cargo.toml \
  -p nemesis-runtime --bin nemesis-adapter-health
printf '%s\n' 'PASS_PHASE8_WORKER_ADAPTERS'
