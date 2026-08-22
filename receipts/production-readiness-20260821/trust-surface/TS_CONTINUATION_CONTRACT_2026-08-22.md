# NEMESIS — Continue TS-001/TS-002 and finish every phase

Date: 2026-08-22
Architect: AnubisQuantumCipher
Acting builder: fresh Oh My Pi session, `anthropic/claude-fable-5`, thinking `max`
Mode: one objective, autonomous, evidence-first, restart-safe
This file is the authorized contract. The locator message is not the contract.

## Why this session exists

The previous Fable 5 Max session `01a02976-fa2d-7000-a764-da46012bd7d2` died at
`2026-08-22T12:47:46Z` (08:47:46 EDT) with:

- official tracker **0/24** complete
- recorded phase **Orient 0/2** of an 8-phase / 24-item map
- three scouts dispatched (DesktopScout, GateScout, ReceiptScout)
- TS-001 / TS-002 implementation paths identified
- **no source edits**, no gates re-run, no merge, no release
- worktree left clean at `4f0f387`

Do **not** resume that journal. Start a **fresh** session. Use the dead
session only as reconnaissance, never as completion evidence.

## Starting identity — probe before any edit

- Worktree: `/Users/sicarii/Worktrees/nemesis-desktop-production-recovered`
- Branch: `production/desktop-v1-autonomous-recovery`
- Required HEAD: `4f0f387923545d994fc1f3e93ee3208f0f04ff77`
- Remote: `https://github.com/AnubisQuantumCipher/nemesis.git`
- Status must be clean and remote-equal before the first edit.
- Prior production roster sealed `BLOCKED_PRODUCTION_PUBLIC` (173 PASS / 9 BLOCKED / 0 PENDING). That seal is **superseded** by this contract. Preserve it as a scar; do not silently rewrite historical receipts.

If identity differs, stop `NEMESIS_TS_CONTINUATION_BLOCKED_BASE_DRIFT`.

## Operator authority (verbatim, still in force)

1. Developer ID, Apple membership, notarization, and Gatekeeper acceptance are **not** requirements. Supported distribution is open-source/source install, local build, and hash-verifiable unsigned artifacts. Redesign I-08/I-09/I-10/I-11/I-13/M-07 around that contract. Signing is an optional future non-claim, never a blocker. Never call an ad-hoc signature Developer ID.
2. There is no human execution lane. You are authorized to implement **TS-001** and **TS-002**, run independent autonomous reviews, merge through the protected PR if repository rules permit, and create/read back the successor release/checkpoint. Preserve safety, auditability, exact hashes, rollback, and branch protection. Never invent approval. If the host rejects an operation, preserve the exact technical rejection and take another permitted path.
3. Only **G-10** (native macOS accessibility-tree / window-restoration walkthrough) may remain `DEFERRED` / `ENVIRONMENT_BLOCKED` because `IOConsoleLocked=Yes`. Do not call that PASS. Everything else must reach PASS. Zero PENDING. Zero other BLOCKED at terminal.
4. Do not weaken security, evidence, authority, replay, or refusal semantics to make gates green.

## One objective

Complete **all eight phases / 24 tracker items** of the dead session, then seal a truthful terminal verdict.

### Official tracker (restore exactly, then execute)

1. **Orient** (2)
   - Map desktop UI, runtime, scripts, CI surfaces
   - Probe GitHub auth, PR 4, branch protection, releases
2. **TrustSurface** (8)
   - Implement authority store persistence (grants, approvals)
   - Wire TS-001 approval consumption into `authorize_action`
   - Wire TS-002 parent grant attenuation into `authorize_action`
   - Add `create_approval` and grant protocol commands
   - SPARK-prove `Derive_Child_Grant` attenuation
   - Hostile and crash-recovery authority tests
   - Desktop UI approval workflow integration
   - A→B→A tamper gate on the authority path
3. **Distribution** (3)
   - Redesign I-08..I-13 and M-07 around source-install contract
   - Prove clean build / install / launch / upgrade / uninstall
   - Hash-bound archive plus verification docs and assets
4. **Competitive** (3)
   - Verify competitor official sources
   - Build evidence-backed competitive matrix
   - Implement the highest-value missing capabilities
5. **Gates** (3)
   - Re-run functional / security / formal / reliability gates
   - Re-run perf / accessibility / privacy / package / replay / hostile gates
   - Extend `TRACKER.json` with new phases; preserve scars
6. **Review** (1)
   - Re-run independent autonomous reviewers on final bytes
7. **Release** (3)
   - Merge protected PR if permitted
   - Build successor release from the exact merge commit
   - Verify published bytes / hashes / readback
8. **Final** (1)
   - Reconcile `FINAL.json` and `FINAL_REPORT.md` to the real terminal state

## Trust-surface implementation notes (from dead-session recon + unmerged proposal)

Read first:

- `receipts/production-readiness-20260821/trust-surface/TS-PROPOSAL.md`
- `daemon/src/nemesis_core_daemon.adb` `authorize_action`
- `kernel/src/nemesis-kernel-approvals.ads`
- `kernel/src/nemesis-kernel-capabilities.ads`
- `config/formal-kernel-scope.json`
- `docs/architecture/FORMAL_ASSURANCE.md`

**TS-001:** daemon must consume a persisted, exact, unexpired, one-shot kernel approval inside `authorize_action`. Replay must refuse. Persist the consumed record crash-safely.

**TS-002:** replace the synthesized per-request grant with a persisted parent grant plus an attenuated child that satisfies `Is_Attenuation`. Fail closed on missing parent, failed attenuation, expiry, or revocation.

Do not apply a silent accept-condition change without tests, hostile cases, crash recovery, and an A→B→A tamper gate. This contract **is** the architect authorization to implement both.

## Competitive bar (verify live sources yourself)

NEMESIS is intended to be the final-boss competitor, not a generic dashboard. Inspect actual NEMESIS source/runtime/UI before claiming parity. Matrix values are only `SHIPPED`, `PARTIAL`, `ABSENT`, `OUT-OF-SCOPE`. Then implement the highest-value missing capabilities. Differentiated core: evidence-first authority, deterministic receipts/replay, safe refusal, secure local-first operation. Do not stop at a matrix.

Sources to re-verify:

- Hermes desktop/docs
- DeepSeek Harness
- RUBRIC
- Codex app
- Claude Code

## Non-interference

- Do not write the sibling worktree `/Users/sicarii/Worktrees/nemesis-desktop-production` or any other live agent checkout.
- Do not disable host guards (`com.sicarii.omp-host-guard`, `com.anubis.host-resource-guard`).
- Do not use Apple Developer credentials or claim notarization/Gatekeeper-accept.
- Do not modify immutable predecessor `v0.1.0`.
- Keep reviewer concurrency inside existing OMP host-guard limits.

## Terminal verdicts

Stop only with one of:

- `SEALED_LOCAL_PRODUCTION` — every required gate PASS except G-10 explicitly `DEFERRED`/`ENVIRONMENT_BLOCKED`; TS-001 and TS-002 implemented, tested, reviewed; source-install distribution proved; tracker/final receipts reconciled; successor release/readback done or exact host rejection recorded.
- `BLOCKED_TRUST_SURFACE` — only if a *new* accept-condition change beyond TS-001/TS-002 is required and cannot be completed safely. Preserve the unmerged proposal.
- `BLOCKED` — a reproduced technical blocker with no remaining authorized path. Name the exact command, output, and residual.

Do not stop at Orient, a plan, an intermediate commit, or a green subset.

## First actions after identity probes

1. Hash this file and print SHA-256 / `wc` counts.
2. Initialize the 8-phase / 24-item tracker above.
3. Ingest scout findings from journal `01a02976` as reconnaissance only.
4. Begin TrustSurface implementation. Do not linger in Orient.
