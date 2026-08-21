#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/verify_contract.py
./scripts/verify_phase0.sh
./scripts/test_authority.sh
./scripts/test_storage.sh
./scripts/test_worker_boundary.sh
./scripts/test_adapters.sh
./scripts/test_scheduler.sh
./scripts/test_context.sh
./scripts/test_evidence_replay.sh
./scripts/test_extensions.sh
./scripts/test_git_workflows.sh
./scripts/test_automation.sh
./scripts/run_vertical_slice.sh --output receipts/desktop-latest
python3 scripts/verify_evidence_bundle.py receipts/desktop-latest --write
python3 scripts/verify_evidence_bundle.py receipts/desktop-latest
./scripts/verify_desktop.sh
./scripts/test_security_hardening.sh
python3 scripts/verify_phase16_receipt.py
python3 scripts/test_phase16_receipt.py
printf '%s\n' 'PASS_NEMESIS_DESKTOP_COMPLETE'
