# NEMESIS Formal Assurance Report — Bounded SPARK Kernel Proof

Generated from `receipts/production-readiness-20260821/FORMAL_KERNEL.json` (regenerated 2026-08-22 after the architect-authorized TS-001/TS-002 trust-surface integration).
Gate: `./scripts/prove_kernel.sh` → `PASS_KERNEL_PROOF_MANIFEST obligations=81 units=8`; independent verifier `scripts/verify_kernel_proof.py`; scope `config/formal-kernel-scope.json`.

## What is proved

GNATprove (SPARK, provers Alt-Ergo 2.4.0 / CVC5 1.1.2 via Why3, Alire alr 2.1.0) discharges 81 verification conditions across 8 units with 0 unproved checks, 0 warnings, 0 recorded assumptions, and no prover timeouts. Evidence for absence of run-time errors and the stated postconditions covers exactly these units and public operations:

- `nemesis-kernel-approvals` (kernel/src/nemesis-kernel-approvals.ads, kernel/src/nemesis-kernel-approvals.adb): `Consume_Approval`
- `nemesis-kernel-budgets` (kernel/src/nemesis-kernel-budgets.ads, kernel/src/nemesis-kernel-budgets.adb): `Can_Allocate`, `Consume`
- `nemesis-kernel-capabilities` (kernel/src/nemesis-kernel-capabilities.ads, kernel/src/nemesis-kernel-capabilities.adb): `Authorize`, `Is_Attenuation`, `Derive_Child_Grant`
- `nemesis-kernel-completion` (kernel/src/nemesis-kernel-completion.ads, kernel/src/nemesis-kernel-completion.adb): `Evaluate`
- `nemesis-kernel-evidence` (kernel/src/nemesis-kernel-evidence.ads, kernel/src/nemesis-kernel-evidence.adb): `Acceptable`
- `nemesis-kernel-missions` (kernel/src/nemesis-kernel-missions.ads, kernel/src/nemesis-kernel-missions.adb): `Create`, `Restore`, `State_Of`, `Sequence_Of`, `Apply`, `Commit_Event`
- `nemesis-kernel-transitions` (kernel/src/nemesis-kernel-transitions.ads, kernel/src/nemesis-kernel-transitions.adb): `Allowed`
- `nemesis-kernel-types` (kernel/src/nemesis-kernel-types.ads, kernel/src/nemesis-kernel-types.adb): `Encode_State`, `Decode_State`, `Is_Terminal`, `Next`

Cross-unit authority invariants carried by these contracts:

- Authorization cannot widen: `Capabilities.Authorize` refuses requests outside the granted capability, and `Capabilities.Is_Attenuation` is proved as an exact characterization (if and only if): it holds precisely when both grants are active, the child's mission/resource/scope match the parent, expiry and byte budget are bounded by the parent, and every child operation is present in the parent's operation set.
- Derived authority is attenuation by construction: `Capabilities.Derive_Child_Grant`'s proved postcondition guarantees `Is_Attenuation (Parent, Result)` together with exact field bindings (parent mission/subject/resource/scope inherited, single requested operation, caller-bounded expiry and byte budget). The daemon's `authorize_action` uses this operation for every per-action child grant (TS-002).
- Approvals are exact and one-shot at the kernel boundary: `Approvals.Consume_Approval`'s proved postcondition marks an accepted approval `Approval_Consumed` only on exact mission and action-digest match before expiry, and leaves the record unchanged on every refusal (`Approval_Replayed` covers double consumption). The daemon's `authorize_action` consumes a persisted approval through this operation before every authorization event (TS-001).
- Budgets are monotone: `Budgets.Consume`'s proved postcondition forbids negative allocation, and together with the non-negative `Budget_Unit` subtype (`0 .. 2**63 - 1`) and the `Can_Allocate` guard it prevents overflow or any increase.
- Invalid states cannot transition or complete: `Transitions.Allowed`'s proved postcondition (a terminal source admits no transition), the terminal-state guards in the bodies of `Missions.Apply`/`Commit_Event` together with their proved postconditions, and `Completion.Evaluate`'s proved postcondition.
- Evidence cannot self-accept: `Evidence.Acceptable`'s proved postcondition.

Semantic calibration (two independent A→B→A tamper gates):

- `scripts/verify_proof_aba.py` executes an exact-byte A→B→A gate on a disposable copy (B widens `Capabilities.Authorize` to return `Authorized`); run A proves green, run B fails for the intended GNATprove postcondition reason, restoration is byte-exact, and rerun A2 proves green (`PASS_KERNEL_PROOF_ABA`).
- `scripts/verify_authority_aba.py` executes the same discipline on the daemon authority path (B widens `authorize_action` to also accept `Approval_Replayed`); the hostile daemon API test rejects run B because a replayed approval authorizes, restoration is byte-exact, and rerun A2 is green (`PASS_AUTHORITY_ABA`).

## What is not proved

The following authority-bearing boundaries are `SPARK_Mode => Off` and carry no proof evidence; they are covered only by tests:

- `nemesis-core-authority_store`: SPARK_Mode Off; durable parent-grant and one-shot approval persistence boundary for TS-001/TS-002 (fixed-width strict parse, F_FULLFSYNC, atomic rename) (daemon/src/nemesis-core-authority_store.ads, daemon/src/nemesis-core-authority_store.adb)
- `nemesis-core-checkpoints`: SPARK_Mode Off; durable checkpoint parsing, fsync, and atomic rename boundary (daemon/src/nemesis-core-checkpoints.ads, daemon/src/nemesis-core-checkpoints.adb)
- `nemesis-core-index`: SPARK_Mode Off; derived SQLite migration and index boundary (daemon/src/nemesis-core-index.ads, daemon/src/nemesis-core-index.adb)
- `nemesis-core-json`: SPARK_Mode Off; bounded local protocol parser (daemon/src/nemesis-core-json.ads, daemon/src/nemesis-core-json.adb)
- `nemesis-core-ledger`: SPARK_Mode Off; authoritative event framing, hashing, append, and recovery boundary (daemon/src/nemesis-core-ledger.ads, daemon/src/nemesis-core-ledger.adb)
- `nemesis-core-mission_store`: SPARK_Mode Off; primary authority-persistence and evidence-freshness boundary (daemon/src/nemesis-core-mission_store.ads, daemon/src/nemesis-core-mission_store.adb)
- `nemesis-core-objects`: SPARK_Mode Off; content-addressed evidence object boundary (daemon/src/nemesis-core-objects.ads, daemon/src/nemesis-core-objects.adb)
- `nemesis-core-paths`: SPARK_Mode Off; canonical path and symlink authorization boundary (daemon/src/nemesis-core-paths.ads, daemon/src/nemesis-core-paths.adb)
- `nemesis-core-root`: SPARK_Mode Off root package (daemon/src/nemesis-core.ads)
- `nemesis_core_daemon`: SPARK_Mode Off; socket dispatch and Kernel composition boundary; authorize_action consumes persisted one-shot approvals via Approvals.Consume_Approval and derives attenuated child grants from the persisted parent via Capabilities.Derive_Child_Grant (daemon/src/nemesis_core_daemon.adb)

## Trust-surface resolutions

The two blockers recorded by the no-human production mission received exact architect sign-off and were implemented on 2026-08-22. The authorizing contract is archived at `receipts/production-readiness-20260821/trust-surface/TS_CONTINUATION_CONTRACT_2026-08-22.md` (sha256 `920900f3a7d37a9a3e1d51541997070789a0e1e7bddecf31808076c67a00d3f9`); the original unmerged proposal is preserved as `receipts/production-readiness-20260821/trust-surface/TS-PROPOSAL.md`.

- `TS-001` (resolved): `authorize_action` now requires a persisted, exact, unexpired approval consumed exactly once by the SPARK kernel, made durable before the authorization event commits. Replay refuses, including across daemon kill/restart. Enforced by `tests/integration/test_daemon_api.py`, `daemon/tests/nemesis_authority_store_tests.adb`, and the `scripts/verify_authority_aba.py` tamper gate.
- `TS-002` (resolved): the synthesized per-request grant is gone; a single immutable persisted parent grant per mission (issued via `create_grant`, all authority fields derived server-side from the authorized mission context) is attenuated per action through the proved `Derive_Child_Grant`. Missing, corrupt, or inactive parents refuse.

`config/formal-kernel-scope.json` records both resolutions with the authorizing contract identity; `scripts/verify_kernel_proof.py` refuses silent trust-surface erasure (empty blockers without documented, hash-anchored resolutions fail the gate).

## Non-extension of conclusions

SPARK conclusions apply only to the proved units and named obligations above. They do not extend to the Rust runtime or Tauri bridge, WebKit, SQLite, Git, the macOS Keychain, the sandbox, the operating system, compilers or provers, or any dependency. The formal lane reports `PASS_BOUNDED`: a green bounded proof of the eight kernel units, now composed with the daemon authority path that actually calls the proved operations, is still not a whole-system correctness claim; unproved `SPARK_Mode => Off` boundaries remain covered by tests and hostile gates only.
