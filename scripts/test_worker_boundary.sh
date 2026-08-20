#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml --workspace
cargo build --manifest-path runtime/Cargo.toml --workspace
mkdir -p build/phase5-worker-lane
runtime/target/debug/nemesis-worker-runner \
  runtime/target/debug/nemesis-deterministic-worker \
  build/phase5-worker-lane \
  plan \
  --mission-id mis_0000000000000000000000 \
  --worker-id wrk_0000000000000000000000 \
  --path value.txt \
  --content-digest aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
printf '%s\n' 'PASS_PHASE5_WORKER_BOUNDARY'
