# Phase 16 Security, Dependency, Threat, and Proof Review

**Scope:** NEMESIS Desktop local-only amendment through Phase 14, hardened under the Phase 16 contract in `docs/architecture/DESKTOP_DEPENDENCY_GRAPH.md`.

**Primary crash/mutation venue:** Apple Virtualization.framework guest cloned from local Tart image `anubis-xcode`. Host-local crash, fuzz, and exploit execution is not accepted as evidence.

## Accepted findings and fixes

### H16-01 — encoded traversal ambiguity reached the worker parser

The first guest run rejected ordinary `..` and absolute paths but accepted `..%2fescape`. The protocol now exposes one strict repository-path validator that rejects encoded separators, backslashes, control bytes, empty/dot components, and `.git` metadata (`runtime/crates/nemesis-protocol/src/lib.rs:147-163`). Worker parsing, context indexing, and local Git staging reuse that policy (`runtime/crates/nemesis-protocol/src/lib.rs:224-250`, `runtime/crates/nemesis-runtime/src/context.rs:311-317`, `runtime/crates/nemesis-runtime/src/git_workflows.rs:172-179`). The guest-only regression corpus covers those hostile forms (`runtime/crates/nemesis-runtime/tests/security_mutation.rs:50-82`).

### H16-02 — pinned Wasmtime release had active RustSec advisories

`cargo audit --file runtime/Cargo.lock` initially exited nonzero and reported 19 vulnerabilities against Wasmtime/wasmtime-wasi 38.0.4, including the AArch64 Cranelift sandbox-escape advisory RUSTSEC-2026-0096. The workspace now pins Wasmtime and wasmtime-wasi 47.0.3 (`runtime/Cargo.toml:30-31`). A second `cargo audit --file runtime/Cargo.lock` exited zero, and the extension tests plus full guest workspace battery passed against the upgraded lockfile.

### H16-03 — Keychain seed text was not zeroized on the read path

The signer already zeroized decoded seed buffers, but the hexadecimal stdout returned by `/usr/bin/security` was dropped without explicit erasure. The read path now decodes from a mutable byte vector and zeroizes that vector on both success and decode failure (`runtime/crates/nemesis-signer/src/lib.rs:134-155`). Signer tests passed after the change.

### H16-04 — isolation test depended on a host-specific Git path

The clean guest exposed a test-only `/opt/homebrew/bin/git` dependency. The isolation fixture now uses the same stable `/usr/bin/git` path as production (`runtime/crates/nemesis-runtime/tests/isolation.rs:11-16`). The clean guest subsequently passed the full workspace battery.

## Threat-boundary review

- Native worker execution canonicalizes the lane and executable, generates a default-deny macOS sandbox profile, denies network access, permits writes only below the lane, executes only the selected binary, and clears inherited environment state (`runtime/crates/nemesis-runtime/src/lib.rs:161-198`, `runtime/crates/nemesis-runtime/src/lib.rs:230-253`). Missing `sandbox-exec` returns an error; there is no weaker fallback (`runtime/crates/nemesis-runtime/src/lib.rs:230-233`).
- Repository paths share a bounded, relative, unencoded grammar; no subsystem silently normalizes a form rejected at the worker boundary (`runtime/crates/nemesis-protocol/src/lib.rs:147-163`). The mutation corpus confirms parser failures do not panic or acquire authoritative methods (`runtime/crates/nemesis-runtime/tests/security_mutation.rs:23-48`).
- The WASI plugin host requires an empty capability set, enables fuel accounting, limits memory, instances, and tables, and supplies no inherited files, environment, or network resources (`runtime/crates/nemesis-runtime/src/extensions.rs:267-330`).
- The desktop renderer receives Tauri core defaults only; application effects remain the three fixed typed commands in the native bridge (`desktop/src-tauri/capabilities/default.json:1-7`, `desktop/src-tauri/src/lib.rs:325-331`). No shell or filesystem plugin is granted.
- Production Rust sources in `runtime/crates/*/src`, `desktop/src-tauri/src`, and `daemon/src` contain no `unsafe` block as of this review. This is a source observation, not a transitive-dependency claim.

## Dependency review

Observed commands:

```text
cargo audit --file runtime/Cargo.lock
npm audit
```

After H16-02, both commands exited zero. The Rust audit used the locally updated RustSec advisory database; the npm audit reported no dependency vulnerabilities. These are point-in-time advisory checks, not proof that dependencies contain no vulnerabilities.

## Proof review

Observed host command:

```text
./scripts/build_ada.sh
./build/bin/nemesis_kernel_tests
./build/bin/nemesis_authority_tests
./build/bin/nemesis_path_tests
./build/bin/nemesis_ledger_tests
./build/bin/nemesis_storage_tests
./build/bin/nemesis_index_tests
./scripts/prove_kernel.sh
```

All commands exited zero. `scripts/prove_kernel.sh` runs GNATprove in `--mode=all`, level 2, with checks and warnings as errors over the eight named kernel units (`scripts/prove_kernel.sh:7-21`). The observed sentinel was `PASS_KERNEL_PROOF_BASELINE`.

This proof boundary covers the declared kernel units only. It does not mechanize the Rust runtime, Tauri renderer, dependency implementations, macOS sandbox engine, Keychain, Git, SQLite, or the full Ada daemon/storage stack.

## VZ hardening battery

`scripts/run_security_hardening.sh` creates a disposable Tart clone, boots it headlessly, syncs source without build artifacts, runs `scripts/security_hardening_guest.sh`, then stops and deletes the guest. The guest gate verifies contract bytes, formatting, and the complete Rust workspace test set. The accepted sentinel is `PASS_PHASE16_VZ_SECURITY_HARDENING_ORCHESTRATOR`; a missing tool, SSH key, VZ guest marker, or surviving guest clone is a failure.

## Residuals and deferred boundaries

- `[NEEDS-HUMAN]` Independent external security review has not been performed.
- macOS `sandbox-exec` behavior is exercised on the current host and VZ guest but is not a stable cross-platform API. Windows, Linux, remote workers, and mobile remain deferred.
- Advisory results can change after this checkpoint; rerun the dependency gates against the current advisory databases.
- Public distribution, signing/notarization, public-name collision research, legal review, beta, release-candidate, and public-release work remain `DEFERRED` under the desktop-only amendment.
