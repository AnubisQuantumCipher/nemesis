#!/usr/bin/env bash
set -euo pipefail

if [[ "${NEMESIS_VZ_GUEST:-}" != "1" ]]; then
  printf '%s\n' 'REFUSED_SECURITY_HARDENING_REQUIRES_VZ_GUEST' >&2
  exit 2
fi

export PATH="$HOME/.cargo/bin:$PATH"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml --workspace
printf '%s\n' 'PASS_PHASE16_VZ_SECURITY_HARDENING'
