# NEMESIS

[![CI](https://github.com/AnubisQuantumCipher/nemesis/actions/workflows/ci.yml/badge.svg)](https://github.com/AnubisQuantumCipher/nemesis/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**The Provable Agent Operating System**  
**Autonomy under control.**

> Intelligence proposes. NEMESIS governs. Evidence decides.

NEMESIS is a local-first system for durable autonomous missions. Untrusted workers propose actions; a bounded Ada/SPARK authority layer decides whether those actions are permitted; deterministic verifiers decide whether completion claims are supported. The macOS desktop app exposes mission authority, evidence state, replay, and the witnessed local vertical slice.

This repository is a **source-visible macOS alpha**. It does not claim that models are correct, that every component is formally verified, or that a signed receipt proves facts outside its named verifiers.

## Bounded release scope

The `v0.1.0` release line contains:

- NEMESIS Desktop: Tauri 2, React, and TypeScript;
- NEMESIS Core: local Ada daemon and durable mission state;
- NEMESIS Kernel: selected SPARK authority packages and bounded proof gates;
- NEMESIS Runtime: Rust worker protocol, isolated local lanes, adapters, scheduling, context, extensions, local Git controls, and automation denial boundaries;
- signed local mission receipts and a standalone verifier;
- a self-contained, ad-hoc-signed macOS arm64 `.app` archive.

Not shipped: iPhone/iPad, web, remote-public daemon, cloud service, Windows/Linux packages, App Store distribution, Developer ID signing, or notarization. The inherited multi-platform `1.0` blueprint remains future architecture, not the status of this release.

## Architecture and trust flow

```text
Human authorization
        ↓
Desktop / CLI client (untrusted presentation)
        ↓ validated local protocol
Core daemon (durable owner)
        ↓ bounded records
SPARK Kernel (authority and completion)
        ↓ authorized action
Rust Runtime / isolated worker lane
        ↓ artifacts and observations
Deterministic verifiers
        ↓ accepted, source-bound evidence
Completion court → local signed receipt
```

Authority flows down only after validation. Evidence flows up only after independent checking. Workers cannot issue capabilities, accept their own elevated evidence, mutate authoritative mission state, or mark a mission complete.

Detailed boundaries:

- [Threat model](THREAT_MODEL.md)
- [Trust boundaries](TRUST_BOUNDARIES.md)
- [Evidence policy](docs/verification/EVIDENCE_POLICY.md)
- [Desktop dependency graph](docs/architecture/DESKTOP_DEPENDENCY_GRAPH.md)
- [Storage and recovery](docs/architecture/STORAGE.md)
- [Architecture constitution](rfcs/0001-constitution.md)

## Prerequisites

### Prebuilt app

- Apple Silicon Mac (`arm64`)
- macOS 14 or later
- Local `/usr/bin/git`, `/usr/bin/python3`, `/bin/cat`, and `/usr/bin/sandbox-exec` commands, as exercised by the bundled witnessed-mission fixture

The app may request access to a local Keychain item when producing its local Ed25519 receipt. It does not require a NEMESIS cloud account.

### Source build

- Node.js 22 and npm
- Rust 1.94 or later; Wasmtime `47.0.3` declares Rust 1.94 as its minimum
- Alire 2.1.0 with GNAT Native 14.2.1, GPRbuild 24.0.1, and the GNATprove toolchain used by the bounded proof gate
- `cargo-audit` 0.22.1 for the dependency audit
- `cargo-about` 0.9.2 for release license policy and third-party notices

The exact observed Ada/SPARK baseline and its retained macOS deployment-target warnings are documented in [the Ada toolchain guide](docs/developer-guide/ADA_TOOLCHAIN.md). Other toolchain combinations are not claimed compatible until exercised.

## Build and run from source

```sh
npm --prefix desktop ci
./scripts/build_ada.sh
cargo build --manifest-path runtime/Cargo.toml --workspace
./script/build_and_run.sh --verify
```

`./script/build_and_run.sh` builds the renderer and native bridge, creates a local debug `.app`, launches it, and requires the `nemesis-desktop` process to appear.

Run the witnessed backend slice directly:

```sh
./scripts/run_vertical_slice.sh --output receipts/desktop-latest
python3 scripts/verify_evidence_bundle.py receipts/desktop-latest --write
python3 scripts/verify_evidence_bundle.py receipts/desktop-latest
```

`receipts/desktop-latest/` is ignored generated output. The fixture creates a disposable repository and NEMESIS home under a temporary directory; it does not mutate the repository's default branch.

## Verification

### Portable and hosted gates

```sh
python3 scripts/verify_contract.py
python3 scripts/verify_release_contract.py
python3 -m unittest discover -s tests -p 'test_*.py'
cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml --workspace
npm --prefix desktop test
npm --prefix desktop run build
cargo test --manifest-path desktop/src-tauri/Cargo.toml
cargo audit --file runtime/Cargo.lock
npm --prefix desktop audit
```

GitHub Actions runs the contract/evidence regressions, Rust workspace, renderer/native desktop tests, builds, and dependency audits on supported GitHub-hosted macOS runners. It does **not** label the host-specific Tart/VZ or GNATprove boundary green through a placeholder.

### Full local desktop gate

```sh
./scripts/verify_complete.sh
```

This gate includes Ada executables, selected GNATprove obligations, renderer/native checks, the witnessed mission, valid and tampered receipt verification, dependency audits, and the disposable Apple Virtualization.framework hardening lane. The VZ step requires the local `anubis-xcode` Tart base and `~/.ssh/tart_anubis`; that external machine fixture is not distributed in this repository.

A pass establishes only the properties named by the sub-gates. The SPARK proof applies to the eight explicitly selected kernel units, not the Rust runtime, Tauri/WebKit, SQLite, Git, Keychain, sandbox engine, operating system, compilers, or third-party libraries.

## Evidence

| Evidence | Purpose |
|---|---|
| [`receipts/phase-6/`](receipts/phase-6/) | witnessed backend mission, local signature, independent verification, tamper rejection |
| [`receipts/phase-7-desktop/`](receipts/phase-7-desktop/) | desktop QA plus exact bundle manifest |
| [`receipts/phase-11/`](receipts/phase-11/) | evidence graph and exact authoritative replay |
| [`receipts/phase-16/PHASE16.json`](receipts/phase-16/PHASE16.json) | historical bounded hardening receipt and accepted findings |
| [`receipts/CHECKPOINT.json`](receipts/CHECKPOINT.json) | sealed pre-release `PARTIAL` checkpoint |
| [`receipts/acceptance-20260820/`](receipts/acceptance-20260820/) | preserved pre-release full-gate log and capture metadata |

Historical machine receipts intentionally retain observed local tool paths because changing them would invalidate their hashes. They are scoped evidence artifacts, not portable configuration; the release binary leak gate rejects private build paths and credential signatures from distributable executables.

## Build a release archive

From a clean committed macOS arm64 worktree:

```sh
./scripts/package_release.sh 0.1.0
```

The packager builds the release runtime, audits accepted dependency licenses, generates and embeds third-party notices, removes private Mach-O RPATHs, strips local symbols, rejects private path/credential signatures, applies an ad-hoc signature, creates the app ZIP, and derives `release-manifest.json` plus `SHA256SUMS` from final bytes. Output is written under ignored `release/v0.1.0/`.

## Install a GitHub release

Download these assets from [GitHub Releases](https://github.com/AnubisQuantumCipher/nemesis/releases):

- `NEMESIS-Desktop-v0.1.0-macos-arm64.zip`
- `release-manifest.json`
- `SHA256SUMS`

Place them in one directory, then verify before extracting:

```sh
shasum -a 256 -c SHA256SUMS
ditto -x -k NEMESIS-Desktop-v0.1.0-macos-arm64.zip .
codesign --verify --deep --strict --verbose=2 "NEMESIS Desktop.app"
```

The app is **ad-hoc signed and not notarized** because an active Apple Developer membership is unavailable. Gatekeeper rejection is expected for downloaded bytes. Prefer building from reviewed source. If you deliberately trust the verified archive, use macOS **Privacy & Security → Open Anyway**; do not interpret that override as Developer ID or notarization evidence.

## Security reporting

Do not file suspected vulnerabilities in a public issue. Use [GitHub private vulnerability reporting](https://github.com/AnubisQuantumCipher/nemesis/security/advisories/new). Include the affected commit/version, minimal reproducer, bounded impact, and any receipt or log hashes. See [SECURITY.md](SECURITY.md) for supported versions, response expectations, and non-claims. There is no bounty program.

## Limitations and residual risk

- Independent external security review remains `[NEEDS-HUMAN]`.
- The local VZ hardening result is host- and image-specific.
- GNAT 14.2.1 emits retained deployment-target override warnings on the observed macOS 26 host.
- `sandbox-exec`, Keychain, SQLite, Git, WebKit/Tauri, compiler/prover correctness, cryptographic dependencies, and macOS remain trusted assumptions outside the bounded proofs.
- Model-token replay is not claimed; replay reconstructs recorded authoritative state and decisions.
- Name collision reconnaissance found existing NEMESIS software/package/domain uses. The project makes no uniqueness or legal-clearance claim; see [the bounded reconnaissance](docs/release/NAME_COLLISION_RECONNAISSANCE.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md). Changes to authority, policy, capabilities, approvals, budgets, evidence acceptance, receipt formats, signing, sandboxing, or completion predicates require hostile regression coverage and the RFC/architect review described in [GOVERNANCE.md](GOVERNANCE.md). Security reports use the private channel above.

## License

Code and repository documentation are licensed under [Apache License 2.0](LICENSE), unless a file states otherwise.
