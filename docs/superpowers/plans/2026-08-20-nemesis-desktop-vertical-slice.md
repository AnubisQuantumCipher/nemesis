# NEMESIS Desktop Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first restart-safe NEMESIS Desktop mission from local authorization through an isolated untrusted worker, final-source-bound deterministic evidence, kernel-owned completion, signed receipt, independent verification, and one-byte tamper rejection.

**Architecture:** A small Ada/SPARK kernel owns authority and an Ada 2022 Core daemon owns durable mission state. A Tauri 2 Rust shell is an untrusted local IPC client around a React/TypeScript cockpit. A separate Rust signer/verifier boundary provides canonical CBOR COSE_Sign1 receipts using a macOS Keychain-backed Ed25519 seed; workers never receive signing material.

**Tech Stack:** Ada 2022, SPARK/GNATprove, Alire/GPRbuild, GNATCOLL JSON/SQLite, Rust 2024, Tauri 2, React 19, TypeScript, Vite, Vitest, canonical CBOR, Ed25519, macOS Keychain, Unix-domain sockets, Git worktrees, macOS sandbox profiles.

---

## File structure

- `rfcs/0001-constitution.md`: immutable local product and authority constitution.
- `THREAT_MODEL.md`, `TRUST_BOUNDARIES.md`, `SECURITY.md`, `GOVERNANCE.md`: Phase 0 trust policy.
- `docs/verification/DESKTOP_VERTICAL_SLICE_ACCEPTANCE.md`: executable end-to-end contract.
- `scripts/verify_phase0.sh`: structural Phase 0 gate.
- `alire.toml`, `nemesis.gpr`, `config/gnat.adc`: Ada/SPARK build contract.
- `kernel/src/nemesis-kernel-*.ads|adb`: bounded authority packages only.
- `kernel/tests/nemesis_kernel_tests.adb`: observable transition, capability, evidence, and completion tests.
- `daemon/src/nemesis-core-*.ads|adb`, `daemon/src/nemesis_core_daemon.adb`: validated IPC, durable store, recovery, worker broker.
- `protocols/schemas/*.json`, `protocols/openapi/nemesis-local.yaml`: versioned untrusted-boundary schemas.
- `runtime/Cargo.toml`, `runtime/crates/*`: sandbox/worktree broker, canonical receipt signer, CLI, and independent verifier.
- `workers/deterministic/worker.py`: untrusted JSON-RPC fixture.
- `desktop/package.json`, `desktop/src/*`, `desktop/src-tauri/*`: app-only mission flow.
- `tests/end-to-end/vertical_slice.py`: daemon restart, source binding, verifier, and tamper gate.
- `receipts/phase-*.json`: source-bound phase evidence.

### Task 1: Seal Phase 0 governance

**Files:**
- Create: `rfcs/0001-constitution.md`
- Create: `THREAT_MODEL.md`
- Create: `TRUST_BOUNDARIES.md`
- Create: `SECURITY.md`
- Create: `GOVERNANCE.md`
- Create: `docs/verification/EVIDENCE_POLICY.md`
- Create: `docs/verification/DESKTOP_VERTICAL_SLICE_ACCEPTANCE.md`
- Create: `scripts/verify_phase0.sh`
- Test: `tests/phase0/test_phase0.py`

- [ ] **Step 1: Write the failing Phase 0 contract test**

The test loads every required document, rejects inherited `SESHAT` identifiers outside the archived contract, and asserts the locked identity, doctrine, desktop-only scope, TCB budget, consequence classes, `VERIFIED/BELIEVED/UNKNOWN`, worker distrust, kernel-owned completion, fail-closed sandboxing, and explicit deferred list.

```python
required = {
    "rfcs/0001-constitution.md",
    "THREAT_MODEL.md",
    "TRUST_BOUNDARIES.md",
    "SECURITY.md",
    "GOVERNANCE.md",
    "docs/verification/EVIDENCE_POLICY.md",
    "docs/verification/DESKTOP_VERTICAL_SLICE_ACCEPTANCE.md",
}
for path in required:
    assert Path(path).is_file(), path
assert "Intelligence proposes. NEMESIS governs. Evidence decides." in constitution
assert {"VERIFIED", "BELIEVED", "UNKNOWN"} <= evidence_words
```

- [ ] **Step 2: Run the test and observe missing-document failure**

Run: `python3 -m unittest tests.phase0.test_phase0 -v`
Expected: nonzero, naming the first absent governance file.

- [ ] **Step 3: Write the governance documents and shell gate**

The constitution declares bounded claims and a source-line budget for the SPARK TCB; expansion requires an RFC. The threat model includes malicious models, repository prompt injection, workers, plugins, MCP, dependencies, malformed tools, forged evidence, replayed approvals, path/symlink attacks, secret exfiltration, daemon crash boundaries, compromised frontend, and unavailable signer. The acceptance document defines exact setup, action, restart, final-source hash, receipt, verifier, and byte-mutation assertions.

- [ ] **Step 4: Run the Phase 0 gates**

Run: `python3 -m unittest tests.phase0.test_phase0 -v && ./scripts/verify_phase0.sh`
Expected: both exit zero and print `PASS_PHASE0`.

- [ ] **Step 5: Commit the Phase 0 checkpoint**

Run: `git add LICENSE rfcs THREAT_MODEL.md TRUST_BOUNDARIES.md SECURITY.md GOVERNANCE.md docs scripts tests && git commit -m "checkpoint(phase-0): seal desktop constitution"`

### Task 2: Establish Ada/SPARK proof baseline

**Files:**
- Create: `alire.toml`
- Create: `nemesis.gpr`
- Create: `config/gnat.adc`
- Create: `kernel/src/nemesis.ads`
- Create: `kernel/src/nemesis-kernel.ads`
- Create: `kernel/src/nemesis-kernel-types.ads`
- Create: `kernel/src/nemesis-kernel-types.adb`
- Create: `kernel/tests/nemesis_kernel_tests.adb`
- Create: `scripts/prove_kernel.sh`

- [ ] **Step 1: Write a failing state-type test**

The harness requires bounded `Mission_Id`, all mission states, all refusal classes, terminal-state classification, and checked sequence increment.

```ada
pragma Assert (Nemesis.Kernel.Types.Is_Terminal (Complete));
pragma Assert (not Nemesis.Kernel.Types.Is_Terminal (Running));
pragma Assert (Nemesis.Kernel.Types.Next (0) = 1);
```

- [ ] **Step 2: Run the missing-package build**

Run: `alr build`
Expected: nonzero because kernel packages are not yet present.

- [ ] **Step 3: Add the minimal bounded SPARK package and project settings**

Use `pragma SPARK_Mode (On)`, explicit ranges, no unconstrained user input, `-gnat2022 -gnata -gnatwae -gnatf`, separate object/executable directories, and no proof suppression.

- [ ] **Step 4: Build, execute, and prove the selected package**

Run: `alr build && ./build/bin/nemesis_kernel_tests && ./scripts/prove_kernel.sh`
Expected: test prints `PASS_KERNEL_TYPES`; proof script exits zero and stores tool output under `receipts/raw/phase-1/`.

- [ ] **Step 5: Commit the proof baseline**

Run: `git add alire.toml nemesis.gpr config kernel scripts receipts && git commit -m "checkpoint(phase-1): establish SPARK baseline"`

### Task 3: Implement the mission state kernel with TDD

**Files:**
- Create: `kernel/src/nemesis-kernel-missions.ads`
- Create: `kernel/src/nemesis-kernel-missions.adb`
- Create: `kernel/src/nemesis-kernel-transitions.ads`
- Create: `kernel/src/nemesis-kernel-transitions.adb`
- Modify: `kernel/tests/nemesis_kernel_tests.adb`

- [ ] **Step 1: Add exhaustive failing transition-pair tests**

Iterate every source/destination pair. Assert only the documented table is accepted; terminal states never leave terminal; a commit increments sequence exactly once; refusal leaves mission and sequence unchanged.

- [ ] **Step 2: Run the narrow kernel test**

Run: `alr build && ./build/bin/nemesis_kernel_tests`
Expected: nonzero at the first missing transition implementation.

- [ ] **Step 3: Implement bounded mission records and the table-driven transition authority**

```ada
type Mission_Record is private;
procedure Apply
  (Mission  : in out Mission_Record;
   Target   : Mission_State;
   Decision : out Transition_Decision)
with Post =>
  (if Decision = Accepted then Sequence (Mission) = Sequence (Mission)'Old + 1
   else Sequence (Mission) = Sequence (Mission)'Old);
```

- [ ] **Step 4: Build, test, and prove transition properties**

Run: `alr build && ./build/bin/nemesis_kernel_tests && ./scripts/prove_kernel.sh`
Expected: `PASS_MISSION_TRANSITIONS` and no unproved selected checks.

- [ ] **Step 5: Commit Phase 2**

Run: `git add kernel receipts && git commit -m "checkpoint(phase-2): enforce mission transitions"`

### Task 4: Add durable ledger and restart recovery

**Files:**
- Create: `daemon/src/nemesis-core-ledger.ads`
- Create: `daemon/src/nemesis-core-ledger.adb`
- Create: `daemon/src/nemesis-core-recovery.ads`
- Create: `daemon/src/nemesis-core-recovery.adb`
- Create: `daemon/tests/nemesis_ledger_tests.adb`
- Create: `tests/crash-recovery/test_write_boundaries.py`

- [ ] **Step 1: Write failing ledger-chain and corrupt-tail tests**

Fixtures cover empty ledger, valid chain, reordered event, changed byte, truncated length prefix, duplicate sequence, and valid prefix plus corrupt suffix. Acknowledgement is asserted only after the durable rename/fsync boundary.

- [ ] **Step 2: Run the missing-ledger test**

Run: `alr build && ./build/bin/nemesis_ledger_tests`
Expected: nonzero because the ledger package is absent.

- [ ] **Step 3: Implement length-delimited canonical events, SHA-256 hash chaining, atomic checkpoints, and recovery**

The canonical record binds schema, mission, sequence, prior hash, event kind, payload digest, and source digest. Recovery returns exactly one of `Recovered`, `Empty`, `Corrupt`, or `Unsupported_Version`; it never guesses through corruption.

- [ ] **Step 4: Exercise every declared write boundary**

Run: `python3 -m unittest tests.crash-recovery.test_write_boundaries -v`
Expected: every injected stop recovers the old or new committed prefix, never a hybrid, and prints `PASS_LEDGER_RECOVERY`.

- [ ] **Step 5: Commit Phase 3**

Run: `git add daemon tests receipts && git commit -m "checkpoint(phase-3): add durable mission ledger"`

### Task 5: Implement capabilities, approvals, evidence, and completion

**Files:**
- Create: `kernel/src/nemesis-kernel-capabilities.ads`
- Create: `kernel/src/nemesis-kernel-capabilities.adb`
- Create: `kernel/src/nemesis-kernel-approvals.ads`
- Create: `kernel/src/nemesis-kernel-approvals.adb`
- Create: `kernel/src/nemesis-kernel-evidence.ads`
- Create: `kernel/src/nemesis-kernel-evidence.adb`
- Create: `kernel/src/nemesis-kernel-completion.ads`
- Create: `kernel/src/nemesis-kernel-completion.adb`
- Modify: `kernel/tests/nemesis_kernel_tests.adb`

- [ ] **Step 1: Add failing hostile-control tests**

Cover path traversal, symlink escape flag, operation widening, cross-mission reuse, expired/revoked grant, child escalation, budget underflow, altered action digest, one-shot replay, stale/wrong-source evidence, unauthorized verifier, missing claim, unresolved blocker, and worker completion attempt.

- [ ] **Step 2: Run the hostile tests and retain failure output**

Run: `alr build && ./build/bin/nemesis_kernel_tests`
Expected: nonzero on the first missing hostile control.

- [ ] **Step 3: Implement total decision functions with refusal-first defaults**

Every public decision returns an enumeration such as `Authorized`, `Refused_Capability`, `Refused_Policy`, `Refused_Budget`, `Requires_Approval`, `Requires_Stronger_Sandbox`, or `Requires_Independent_Review`. Unknown values cannot map to authorization.

- [ ] **Step 4: Run tests and selected proofs**

Run: `alr build && ./build/bin/nemesis_kernel_tests && ./scripts/prove_kernel.sh`
Expected: `PASS_HOSTILE_CONTROLS`, `PASS_COMPLETION_COURT`, and selected proof success.

- [ ] **Step 5: Commit Phase 4**

Run: `git add kernel receipts && git commit -m "checkpoint(phase-4): enforce capability and completion authority"`

### Task 6: Define protocols and isolate the untrusted worker

**Files:**
- Create: `protocols/schemas/mission-v1.schema.json`
- Create: `protocols/schemas/worker-v1.schema.json`
- Create: `protocols/schemas/evidence-v1.schema.json`
- Create: `protocols/schemas/receipt-v1.schema.json`
- Create: `protocols/openapi/nemesis-local.yaml`
- Create: `workers/deterministic/worker.py`
- Create: `runtime/crates/nemesis-runtime/src/sandbox.rs`
- Create: `runtime/crates/nemesis-runtime/src/worktree.rs`
- Test: `runtime/crates/nemesis-runtime/tests/hostile_worker.rs`

- [ ] **Step 1: Write failing schema and malicious-worker tests**

Reject unknown protocol versions, extra authoritative fields, worker timestamps/sequences, oversized messages, direct `COMPLETE`, absolute/outside paths, network requests, secret reads, another lane, approval replay, and event forgery.

- [ ] **Step 2: Run the protocol/runtime tests**

Run: `cargo test --manifest-path runtime/Cargo.toml -p nemesis-runtime`
Expected: nonzero because runtime and schema validation are absent.

- [ ] **Step 3: Implement strict JSON-RPC validation, isolated Git worktree creation, and fail-closed macOS sandbox startup**

The deterministic worker proposes one bounded file write and one deterministic test. Runtime refuses if the sandbox binary/profile cannot be established and strips inherited environment to a fixed allowlist.

- [ ] **Step 4: Run hostile worker tests**

Run: `cargo test --manifest-path runtime/Cargo.toml -p nemesis-runtime`
Expected: `hostile_worker` cases pass; no test executes an external network request.

- [ ] **Step 5: Commit Phase 5**

Run: `git add protocols workers runtime receipts && git commit -m "checkpoint(phase-5): isolate untrusted worker protocol"`

### Task 7: Build local receipts and the independent verifier

**Files:**
- Create: `runtime/crates/nemesis-receipt/src/lib.rs`
- Create: `runtime/crates/nemesis-signer/src/main.rs`
- Create: `runtime/crates/nemesis-verify/src/main.rs`
- Test: `runtime/crates/nemesis-receipt/tests/vectors.rs`
- Create: `specs/test-vectors/receipt-valid.cbor`
- Create: `specs/test-vectors/receipt-public-key.bin`

- [ ] **Step 1: Write failing canonical/signature/tamper tests**

Construct a fixed payload and assert deterministic canonical CBOR, COSE protected header `alg=EdDSA`, valid Ed25519 signature, and rejection after one-byte changes to each protected field or signature.

- [ ] **Step 2: Run the receipt tests**

Run: `cargo test --manifest-path runtime/Cargo.toml -p nemesis-receipt`
Expected: nonzero because canonical receipt code is absent.

- [ ] **Step 3: Implement canonical CBOR, COSE_Sign1, Keychain seed storage, and offline verification**

The signer creates or loads a generic-password Keychain item scoped to the local NEMESIS service, signs only validated payload bytes received over stdin, writes no private material, and returns an envelope. Test vectors use a fixed test-only seed clearly excluded from production identity code.

- [ ] **Step 4: Run verifier and tamper vectors**

Run: `cargo test --manifest-path runtime/Cargo.toml -p nemesis-receipt -p nemesis-verify`
Expected: valid vector accepts; every one-byte tamper rejects.

- [ ] **Step 5: Commit receipt boundary**

Run: `git add runtime specs receipts && git commit -m "checkpoint(phase-6a): add independent receipt verification"`

### Task 8: Implement the Ada Core daemon and CLI recovery API

**Files:**
- Create: `daemon/src/nemesis-core-protocol.ads`
- Create: `daemon/src/nemesis-core-protocol.adb`
- Create: `daemon/src/nemesis-core-worker_broker.ads`
- Create: `daemon/src/nemesis-core-worker_broker.adb`
- Create: `daemon/src/nemesis_core_daemon.adb`
- Create: `runtime/crates/nemesis-cli/src/main.rs`
- Test: `tests/integration/test_daemon_api.py`

- [ ] **Step 1: Write failing API lifecycle tests**

Exercise `mission create`, compiled contract inspection, exact authorization digest, run, inspect, agents, evidence, replay, verify, typed malformed request refusal, and daemon restart from the same NEMESIS home.

- [ ] **Step 2: Run the integration test**

Run: `python3 -m unittest tests.integration.test_daemon_api -v`
Expected: nonzero because the local socket is absent.

- [ ] **Step 3: Implement bounded local socket API and daemon orchestration**

Core validates JSON to bounded records, calls Kernel for every authoritative mutation, commits the ledger before acknowledging, creates a worktree lane, brokers the deterministic worker actions, runs the deterministic verifier, invalidates stale evidence, evaluates completion, and invokes the signer only after Kernel accepts completion.

- [ ] **Step 4: Run lifecycle, restart, and CLI commands**

Run: `python3 -m unittest tests.integration.test_daemon_api -v`
Expected: `PASS_DAEMON_API`, including identical mission sequence and state after restart.

- [ ] **Step 5: Commit daemon slice**

Run: `git add daemon runtime tests receipts && git commit -m "checkpoint(phase-6b): complete durable daemon mission"`

### Task 9: Build the NEMESIS Desktop cockpit

**Files:**
- Create: `desktop/package.json`
- Create: `desktop/src-tauri/Cargo.toml`
- Create: `desktop/src-tauri/tauri.conf.json`
- Create: `desktop/src-tauri/capabilities/default.json`
- Create: `desktop/src-tauri/src/main.rs`
- Create: `desktop/src/app/*`
- Create: `desktop/src/features/onboarding/*`
- Create: `desktop/src/features/missions/*`
- Create: `desktop/src/features/evidence/*`
- Create: `desktop/src/features/replay/*`
- Test: `desktop/src/**/*.test.tsx`

- [ ] **Step 1: Write failing app-state and accessibility tests**

Test onboarding-to-mission flow, contract digest review, authorization, typed refusal, daemon recovery banner, completion court, status labels that do not rely on color, keyboard focus, reduced motion, receipt verification, and visible tamper rejection.

- [ ] **Step 2: Run the desktop tests**

Run: `npm test --prefix desktop`
Expected: nonzero because app modules are absent.

- [ ] **Step 3: Implement the mission cockpit and narrow Tauri IPC client**

Use a stable three-column cockpit: mission rail, dense central activity/evidence surface, selected-object inspector, and bottom telemetry strip. Near-black/graphite surfaces use red only for state cues. Tauri commands start or connect to Core, send typed local requests, subscribe to events, restart Core for the recovery demonstration, and invoke the independent verifier. No broad shell or filesystem plugin is exposed to the renderer.

- [ ] **Step 4: Run component tests and build the actual app**

Run: `npm test --prefix desktop && npm run build --prefix desktop && cargo tauri build --debug --no-bundle --manifest-path desktop/src-tauri/Cargo.toml`
Expected: tests pass and a local debug executable is produced without signing/notarization claims.

- [ ] **Step 5: Commit Desktop alpha**

Run: `git add desktop receipts && git commit -m "checkpoint(phase-7): add NEMESIS Desktop cockpit"`

### Task 10: Prove the complete restart and tamper flow

**Files:**
- Create: `tests/end-to-end/vertical_slice.py`
- Create: `scripts/run_vertical_slice.sh`
- Create: `scripts/verify_receipts.py`
- Create: `receipts/phase-6-vertical-slice.json`
- Create: `receipts/phase-7-desktop.json`

- [ ] **Step 1: Write the failing end-to-end acceptance driver**

The driver creates a disposable fixture repository, launches Core and Desktop against a temporary NEMESIS home, creates and authorizes a mission, runs the untrusted worker, interrupts and restarts Core at a declared boundary, resumes, verifies final-source evidence, obtains the receipt, invokes `nemesis-verify`, flips one byte, and requires rejection.

- [ ] **Step 2: Run the driver before final wiring**

Run: `./scripts/run_vertical_slice.sh`
Expected: nonzero with the first unmet acceptance predicate.

- [ ] **Step 3: Complete only missing production wiring exposed by the driver**

No test, verifier, proof obligation, sandbox rule, capability, or completion predicate may be weakened. Fix the production boundary that caused each observed failure.

- [ ] **Step 4: Run the complete gate and launch the actual desktop surface**

Run: `./scripts/run_vertical_slice.sh`
Expected: `PASS_NEMESIS_DESKTOP_VERTICAL_SLICE`, valid receipt accepted, tampered receipt rejected, final source digest equals the receipt binding, and recovered sequence equals the durable ledger.

- [ ] **Step 5: Commit the witnessed slice**

Run: `git add tests scripts receipts && git commit -m "checkpoint(vertical-slice): seal restart and tamper gates"`

### Task 11: Review and record the checkpoint

**Files:**
- Create: `receipts/CHECKPOINT.json`
- Create: `STATUS.md`

- [ ] **Step 1: Run focused code review against the mission contract**

Inspect every trusted-boundary callsite, direct state mutation path, inherited identifier, sandbox fallback, evidence acceptance, completion transition, key exposure, and renderer capability. Fix blockers in source and add regression coverage.

- [ ] **Step 2: Re-run source-bound verification after the last change**

Run: `./scripts/verify_phase0.sh && ./scripts/prove_kernel.sh && cargo test --manifest-path runtime/Cargo.toml --workspace && npm test --prefix desktop && ./scripts/run_vertical_slice.sh`
Expected: every command exits zero against the same final source revision.

- [ ] **Step 3: Generate the restart-safe checkpoint**

`receipts/CHECKPOINT.json` records exact completed/open phases, source revision, gate commands and output digests, tool versions, deferred scope, residual risks, and the next dependency. `STATUS.md` renders that authoritative JSON without adding completion claims.

- [ ] **Step 4: Verify the checkpoint before stopping or proceeding**

Run: `python3 scripts/verify_receipts.py receipts/CHECKPOINT.json`
Expected: `PASS_CHECKPOINT`, with no receipt bound to an older revision.

- [ ] **Step 5: Commit the non-final checkpoint**

Run: `git add receipts/CHECKPOINT.json STATUS.md && git commit -m "checkpoint: record verified desktop vertical slice"`

## Plan self-review

- **Spec coverage:** Tasks 1–10 cover required first actions and inherited Phases 0–7 within the desktop amendment. Later desktop phases remain ordered in `docs/architecture/DESKTOP_DEPENDENCY_GRAPH.md` and receive separate plans only after this vertical slice passes.
- **Scope:** Mobile, web, Windows/Linux, public-release, signing/notarization, registration, and public collision work are explicitly deferred rather than silently counted.
- **Trust boundary:** Workers, renderer, model output, protocol input, libraries, and external providers remain untrusted; no task delegates completion to them.
- **No placeholders:** Every task names files, a failing gate, minimal production responsibility, a passing gate, and a checkpoint commit. Exact implementation may change only when an observed tool/API mismatch is captured as evidence and the trust properties remain unchanged.
