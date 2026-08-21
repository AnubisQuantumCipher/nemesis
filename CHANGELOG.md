# Changelog

This file records user-visible NEMESIS changes. Verification status comes from the named gates and receipts, not from this summary.

## Unreleased

No user-visible changes are queued beyond the `v0.1.0` release line.

## 0.1.0 — 2026-08-20

Initial source-visible macOS alpha.

### Included

- Local Tauri/React desktop cockpit for mission, authority, evidence, completion, and replay views.
- Ada Core daemon with append-only ledger, checkpoints, content-addressed objects, SQLite-derived index, and deterministic recovery/refusal paths.
- Bounded SPARK Kernel packages for mission transitions, capabilities, budgets, approvals, evidence, and completion.
- Rust worker protocol, isolated local worktree lane, subprocess adapters, deterministic scheduler, provenance context capsules, evidence replay, bounded WASI/plugin/MCP surfaces, local Git controls, and unattended automation denial boundaries.
- Locally signed canonical mission receipts plus a standalone verifier and tamper fixtures.
- Source-bound phase receipts and a disposable Apple Virtualization.framework hardening gate.
- Ad-hoc-signed macOS arm64 app archive with byte-derived release manifest and checksums.
- GitHub-hosted macOS CI for portable contract, evidence, Rust, renderer/native desktop, and dependency gates.

### Distribution boundary

- The macOS app is ad-hoc signed, not Developer ID signed, and not notarized.
- Gatekeeper approval and App Store distribution are not claimed.
- Mobile, web, remote-public, Windows, Linux, and package-registry artifacts are not included.
- Independent external security review remains `[NEEDS-HUMAN]`.
- Formal verification is limited to the named SPARK units and documented assumptions.

Release notes: [docs/release/V0.1.0.md](docs/release/V0.1.0.md).
