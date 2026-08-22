# NEMESIS Formal Assurance Report — Bounded SPARK Kernel Proof

Generated from `receipts/production-readiness-20260821/FORMAL_KERNEL.json` observed at 2026-08-22T04:08:43.220989Z.
Gate: `./scripts/prove_kernel.sh` → `PASS_KERNEL_PROOF_MANIFEST obligations=78 units=8`; independent verifier `scripts/verify_kernel_proof.py`; scope `config/formal-kernel-scope.json`.

## What is proved

GNATprove (SPARK, provers Alt-Ergo 2.4.0 / CVC5 1.1.2 via Why3, Alire alr 2.1.0) discharges 78 verification conditions across 8 units with 0 unproved checks, 0 warnings, 0 recorded assumptions, and no prover timeouts. Evidence for absence of run-time errors and the stated postconditions covers exactly these units and public operations:

- `nemesis-kernel-approvals` (kernel/src/nemesis-kernel-approvals.ads, kernel/src/nemesis-kernel-approvals.adb): `Consume_Approval`
- `nemesis-kernel-budgets` (kernel/src/nemesis-kernel-budgets.ads, kernel/src/nemesis-kernel-budgets.adb): `Can_Allocate`, `Consume`
- `nemesis-kernel-capabilities` (kernel/src/nemesis-kernel-capabilities.ads, kernel/src/nemesis-kernel-capabilities.adb): `Authorize`, `Is_Attenuation`
- `nemesis-kernel-completion` (kernel/src/nemesis-kernel-completion.ads, kernel/src/nemesis-kernel-completion.adb): `Evaluate`
- `nemesis-kernel-evidence` (kernel/src/nemesis-kernel-evidence.ads, kernel/src/nemesis-kernel-evidence.adb): `Acceptable`
- `nemesis-kernel-missions` (kernel/src/nemesis-kernel-missions.ads, kernel/src/nemesis-kernel-missions.adb): `Create`, `Restore`, `State_Of`, `Sequence_Of`, `Apply`, `Commit_Event`
- `nemesis-kernel-transitions` (kernel/src/nemesis-kernel-transitions.ads, kernel/src/nemesis-kernel-transitions.adb): `Allowed`
- `nemesis-kernel-types` (kernel/src/nemesis-kernel-types.ads, kernel/src/nemesis-kernel-types.adb): `Encode_State`, `Decode_State`, `Is_Terminal`, `Next`

Cross-unit authority invariants carried by these contracts:

- Authorization cannot widen: `Capabilities.Authorize` refuses requests outside the granted capability, and `Capabilities.Is_Attenuation` requires the child's mission/resource/scope to match, expiry and byte budget to be bounded by the parent, and every child operation to be present in the parent's operation set.
- Approvals are exact and one-shot at the kernel boundary: `Approvals.Consume_Approval`'s proved postcondition marks an accepted approval `Approval_Consumed` only on exact mission and action-digest match before expiry, and leaves the record unchanged on every refusal (`Approval_Replayed` covers double consumption).
- Budgets are monotone: `Budgets.Can_Allocate`/`Budgets.Consume` postconditions forbid negative allocation and overflow.
- Invalid states cannot transition or complete: `Transitions.Allowed`, `Missions.Apply`/`Commit_Event` preconditions, and `Completion.Evaluate` postconditions.
- Evidence cannot self-accept: `Evidence.Acceptable`'s proved postcondition.

Semantic calibration: `scripts/verify_proof_aba.py` executes an exact-byte A→B→A tamper gate on a disposable copy (B widens `Capabilities.Authorize` to return `Authorized`); run A proves green, run B fails for the intended GNATprove postcondition reason, restoration is byte-exact, and rerun A2 proves green (`PASS_KERNEL_PROOF_ABA A=0 B=nonzero A2=0 bytes_restored=true`).

## What is not proved

The following authority-bearing boundaries are `SPARK_Mode => Off` and carry no proof evidence; they are covered only by tests:

- `nemesis-core-checkpoints`: SPARK_Mode Off; durable checkpoint parsing, fsync, and atomic rename boundary (daemon/src/nemesis-core-checkpoints.ads, daemon/src/nemesis-core-checkpoints.adb)
- `nemesis-core-index`: SPARK_Mode Off; derived SQLite migration and index boundary (daemon/src/nemesis-core-index.ads, daemon/src/nemesis-core-index.adb)
- `nemesis-core-json`: SPARK_Mode Off; bounded local protocol parser (daemon/src/nemesis-core-json.ads, daemon/src/nemesis-core-json.adb)
- `nemesis-core-ledger`: SPARK_Mode Off; authoritative event framing, hashing, append, and recovery boundary (daemon/src/nemesis-core-ledger.ads, daemon/src/nemesis-core-ledger.adb)
- `nemesis-core-mission_store`: SPARK_Mode Off; primary authority-persistence and evidence-freshness boundary (daemon/src/nemesis-core-mission_store.ads, daemon/src/nemesis-core-mission_store.adb)
- `nemesis-core-objects`: SPARK_Mode Off; content-addressed evidence object boundary (daemon/src/nemesis-core-objects.ads, daemon/src/nemesis-core-objects.adb)
- `nemesis-core-paths`: SPARK_Mode Off; canonical path and symlink authorization boundary (daemon/src/nemesis-core-paths.ads, daemon/src/nemesis-core-paths.adb)
- `nemesis-core-root`: SPARK_Mode Off root package (daemon/src/nemesis-core.ads)
- `nemesis_core_daemon`: SPARK_Mode Off; socket dispatch and Kernel composition boundary; Approvals is not called (daemon/src/nemesis_core_daemon.adb)

Recorded trust-surface blockers preventing further proof integration without exact architect sign-off:

- `TS-001`: Requiring persisted exact one-shot approvals in authorize_action changes the current authorization accept condition.
- `TS-002`: Replacing the synthesized capability grant with persisted parent and attenuated child grants changes capability issuance and replay semantics.

## Non-extension of conclusions

SPARK conclusions apply only to the proved units and named obligations above. They do not extend to the Rust runtime or Tauri bridge, WebKit, SQLite, Git, the macOS Keychain, the sandbox, the operating system, compilers or provers, or any dependency. The production formal lane remains fail-closed at `BLOCKED_TRUST_SURFACE` until the recorded blockers receive exact sign-off; a green bounded proof of the eight kernel units is not a production-readiness claim.
