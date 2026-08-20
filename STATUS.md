# NEMESIS Desktop Local Build Status

## Verdict

`PARTIAL` — dependency-ordered Phases 0–7 are checkpointed on `build/desktop-local`; Phases 8–14 and 16 remain open. Mobile, web, remote, public beta, release-candidate, and public-release work remains `DEFERRED` under the desktop amendment.

Authoritative machine-readable state: `receipts/CHECKPOINT.json`.

## Resume point

- Repository: `/Users/sicarii/Desktop/Projects/nemesis`
- Implementation worktree: `/Users/sicarii/Desktop/Projects/nemesis/.worktrees/desktop-local`
- Implementation commit: `74b39ec6b242b3b495a5cea345836bdbdcb193df`
- Next dependency: Phase 8, typed desktop-required worker adapters.
- Push/publish: prohibited; no remote configured or used.

## Re-run receipts

```sh
python3 scripts/verify_checkpoint.py
./scripts/verify_phase0.sh
./scripts/test_authority.sh
./scripts/test_storage.sh
./scripts/test_worker_boundary.sh
python3 -m unittest tests.integration.test_daemon_api tests.end_to_end.test_vertical_slice -v
./scripts/verify_desktop.sh
```

The observed Phase 7 gate was `./scripts/verify_desktop.sh` → `PASS_NEMESIS_DESKTOP_VERTICAL_SLICE`. It includes React workflow tests, a production frontend build, a Tauri debug build, native process launch, the preserved app-operated evidence manifest, valid receipt verification, and tampered-receipt rejection.

## Implemented through Phase 7

- Architect contract archive and byte-integrity gate.
- Constitution, threat model, trust boundaries, security/evidence policy, governance, and desktop acceptance contract.
- Ada/SPARK mission transitions, event sequence, capabilities, budgets, approvals, evidence acceptance, and completion court.
- Hash-chained ledger, checkpoints, content-addressed objects, SQLite derived index, corruption/truncation recovery.
- Strict worker protocol, isolated Git lanes, clean environment, macOS Workspace Safe sandbox, cancellation, output bounds.
- Ada Core Unix-socket daemon with durable restart and stale-evidence refusal.
- Canonical CBOR COSE_Sign1 receipts, Keychain-backed Ed25519 signer, standalone verifier, one-byte mutation corpus.
- Tauri 2 + React NEMESIS Desktop mission cockpit, authority review, completion court, native app build/run entrypoint, browser visual QA, and app-operated witnessed mission.

## Open and non-claims

Open phases are enumerated exactly in `receipts/CHECKPOINT.json`. This checkpoint does not claim full worker-provider coverage, multi-worker scheduling, complete context/knowledge, complete replay/forking, plugin/skill trust promotion, full Git/automation workflows, security hardening, public readiness, legal clearance, or completion of any deferred surface.
