#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml -p nemesis-runtime --test git_workflows
printf '%s\n' 'PASS_PHASE13_LOCAL_GIT_WORKFLOWS'
