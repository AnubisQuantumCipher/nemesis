#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
if [[ -n "${NEMESIS_PREBUILT_BIN_DIR:-}" ]]; then
  for binary in \
    nemesis_core_daemon \
    nemesis-deterministic-worker \
    nemesis-lane-create \
    nemesis-signer \
    nemesis-verify \
    nemesis-worker-runner
  do
    if [[ ! -x "$NEMESIS_PREBUILT_BIN_DIR/$binary" ]]; then
      printf '%s\n' "REFUSED_PREBUILT_BINARY_MISSING name=$binary" >&2
      exit 2
    fi
  done
else
  ./scripts/build_ada.sh
  cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
  cargo build --manifest-path runtime/Cargo.toml --workspace
fi
python3 scripts/run_vertical_slice.py "$@"
