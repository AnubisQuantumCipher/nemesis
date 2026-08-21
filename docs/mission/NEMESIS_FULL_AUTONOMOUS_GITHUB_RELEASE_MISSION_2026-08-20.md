# NEMESIS — Full Autonomous GitHub Completion and Release Mission

**Date:** 2026-08-20
**Architect:** AnubisQuantumCipher
**Builder:** fresh Oh My Pi session, `openai-codex/gpt-5.6-sol`, Max reasoning
**Mode:** one objective, deep autonomous execution, evidence-decided
**Repository root:** `/Users/sicarii/Desktop/Projects/nemesis`
**Implementation worktree:** `/Users/sicarii/Desktop/Projects/nemesis/.worktrees/desktop-local`
**Starting branch:** `build/desktop-local`
**Starting HEAD:** `4bab66cd617e430cfd8dec4e16de7ffd6433f0b2`
**Target GitHub owner/repository:** `AnubisQuantumCipher/nemesis`

## Architect authorization

The architect explicitly authorizes this mission to act autonomously without asking for incremental permission. Within this contract, use tools, edit code and documentation, run local and hosted gates, make commits, create the GitHub repository if it does not exist, configure the remote, push branches, open and merge pull requests when safe, create tags, publish GitHub Releases, upload release assets, and update repository metadata. The architect authorizes public GitHub publication of NEMESIS and the first evidence-backed release.

This is not permission to fabricate green state, weaken a verifier, suppress a failure, expose credentials, spend money, make legal filings, enroll in paid services, publish to the App Store, or claim signing/notarization that the inactive Apple membership cannot provide. Use existing authenticated GitHub access without printing or copying token values.

## One objective

Take the current clean, checkpointed NEMESIS desktop-local tree from `PARTIAL` to the strongest honestly releasable, fully documented GitHub state, and finish with exactly one terminal verdict: `COMPLETE`, `PARTIAL`, or `BLOCKED`.

`COMPLETE` requires both local product evidence and hosted GitHub/release evidence. Do not stop after planning, local commits, a pushed branch, a draft PR, or a release draft. Continue until every authorized dependency-ordered gate is terminal, or a concrete blocker is reproduced and recorded.

## Verified launch facts to re-probe

These facts were observed immediately before launch but are not authority until you reproduce them:

- `build/desktop-local` was clean at `4bab66cd617e430cfd8dec4e16de7ffd6433f0b2`.
- No Git remote was configured.
- `AnubisQuantumCipher/nemesis` did not exist on GitHub.
- `gh` was authenticated as `AnubisQuantumCipher` with repository/workflow scope.
- A separate JACKAL OMP process is active in `/Users/sicarii/Worktrees/jackal-unified-completion-20260820`; it and all unrelated trees are strict non-interference boundaries.

## Admission and source of truth

1. Read completely before editing:
   - `STATUS.md`
   - `receipts/CHECKPOINT.json`
   - `docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md`
   - `GOVERNANCE.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRUST_BOUNDARIES.md`
   - `docs/verification/PHASE16_SECURITY_REVIEW.md`
   - `receipts/phase-16/PHASE16.json`
   - `scripts/verify_checkpoint.py`
   - `scripts/verify_complete.sh`
2. Prove repository identity, branch, exact HEAD, worktree cleanliness, worktree map, remotes, GitHub identity, repository nonexistence/existence, and absence of another NEMESIS writer.
3. Run `python3 scripts/verify_checkpoint.py` before changing bytes and preserve its exact output.
4. Treat Git, source files, executable gates, receipts, GitHub API responses, hosted check logs, and downloaded release-asset hashes as authority. Historical prose and GBrain are continuity pointers only.
5. Maintain a task tracker covering local completion, documentation, hosted integration, release, and final independent replay.

## Dependency-ordered work

### A. Independent terminal verdict and gap audit

- Independently inspect the checkpoint and acceptance evidence for anti-vacuity, current-source binding, nonzero test execution, exact artifact coverage, negative controls, tamper rejection, and cleanup.
- Re-run the bounded checkpoint verifier. Re-run broader gates only when required by changed source or to validate the release candidate; do not manufacture stale evidence.
- Determine whether the desktop-only product obligations permit `COMPLETE`. If a real product blocker exists, reproduce it, fix the root cause, add a regression control, and rerun invalidated gates.
- Preserve explicit residuals: external security review, bounded SPARK coverage, macOS/toolchain assumptions, inactive Apple membership, and intentionally deferred mobile/web/public-App-Store scope.

### B. Product and repository completeness

- Inspect the whole tracked tree and ensure a new developer can understand, build, verify, run, and audit NEMESIS from repository documentation alone.
- Create a high-quality root `README.md` because none exists at launch. It must accurately cover identity, doctrine, bounded desktop scope, architecture, trust model, prerequisites, build/run commands, verification commands, evidence locations, security reporting, limitations/residuals, contributing/development workflow, license, and release/install guidance.
- Reconcile all status, architecture, security, verification, contribution, changelog, and release documentation with the actual tree. Fix stale claims rather than changing reality to match prose.
- Add or correct only product-required source, tests, packaging, CI, scripts, and metadata. No cosmetic churn or unrelated refactors.
- Ensure repository hygiene: `.gitignore`, no generated secrets, no local credentials, no private paths in public artifacts except where unavoidable in historical machine receipts and clearly scoped, no oversized accidental build directories, and deterministic/reproducible commands where feasible.
- Add an honest changelog/release-notes source and an appropriate contribution/security policy if absent.

### C. GitHub creation and hosted verification

The architect explicitly authorizes creation of the public repository `AnubisQuantumCipher/nemesis` if it still does not exist.

- Create/configure the GitHub repository with an accurate description, homepage only if real, relevant topics, Apache-2.0 licensing metadata, issues enabled, and appropriate default branch.
- Do not initialize remote bytes separately in a way that creates unrelated history. Preserve the local repository lineage.
- Add `origin` and push evidence-bearing checkpoints.
- Integrate `build/desktop-local` into `main` using the safest history-preserving path. A pull request is preferred when it provides hosted review/check evidence. Do not force-push or rewrite published history.
- Add GitHub Actions CI that runs meaningful, available, license-compatible gates on supported GitHub-hosted runners. Keep host-specific macOS/VZ/Keychain/SPARK limitations explicit; do not mark an unrun host-only gate green through a placeholder.
- Watch every mission-owned required check to terminal state. Failure, timeout, cancellation, action-required, and pending are not green. Diagnose actual logs and fix root causes.
- Merge only after mission-owned required checks are terminal success and the final diff is understood.
- Configure reasonable repository metadata and protections when supported without paid-plan dependency. Do not misrepresent unavailable protection as configured.

### D. Release engineering

- Choose the first honest semantic version from the actual maturity and compatibility state; do not call an alpha-quality artifact `1.0.0` merely to sound complete.
- Produce the strongest viable macOS Desktop release artifact from final source. Because Apple membership is inactive, do not claim Developer ID signing, notarization, Gatekeeper approval, or App Store distribution. Ad-hoc/unsigned distribution is allowed only with explicit installation warnings and verification instructions.
- Build release assets from the final merged commit, generate SHA-256 checksums and a machine-readable manifest, and verify downloaded bytes after publication.
- Create an annotated tag bound to the exact release commit.
- Publish a non-draft GitHub Release with accurate release notes, supported host/architecture, install/run steps, verification commands, known limitations, security posture, and links to evidence.
- Upload only appropriate distributable assets and checksum/manifest files. Do not upload caches, credentials, private logs, or unrelated artifacts.
- After publication, query the GitHub API and independently download/rehash release assets. Require tag, target commit, release URL, asset list, sizes, and hashes to match the local manifest.

### E. Final sealing and receipts

- Update `STATUS.md` and machine-readable checkpoint/release receipts to final reality. A status badge or maturity claim is valid only while its gate exists and passes.
- Run the full final-source local acceptance roster and all changed-surface tests. Preserve exact logs, exit codes, test counts, tool versions, hashes, and non-claims.
- Recheck Git worktree cleanliness, local/remote equality, default branch identity, tag identity, release state, hosted checks, and release-asset hashes.
- Create a final evidence receipt in the repository that names:
  - final commit and tree hash;
  - local gate commands and exact terminal markers;
  - hosted workflow run/check URLs and conclusions;
  - PR/merge URL if used;
  - repository and release URLs;
  - tag and release target commit;
  - release asset names, sizes, and SHA-256 values;
  - deferred scope and residual assumptions;
  - explicit non-claims.
- Commit and push the final receipt. If the receipt changes the tagged tree, either design the receipt so it binds the release commit without requiring retagging, or produce a clearly documented follow-up evidence commit; never move a published tag silently.

## Failure and trust-surface rules

- Read the actual failure, reproduce it, minimize it, and fix the underlying truth. Do not delete assertions, loosen thresholds, special-case a failing row, hide a true accept, or relabel a failure as deferred.
- Never silently change a verifier accept condition, soundness gate, authorization rule, signing rule, key-handling rule, attestation digest, or what `PASS` means. If completion truly requires such a trust-surface change, write the finding and proposed patch, preserve the failing evidence, and return `BLOCKED` pending architect sign-off.
- Do not overwrite, reset, clean, stash, terminate, or modify the active JACKAL worktree/process or any unrelated repository/session.
- No credential values may appear in output, logs, commits, issues, releases, or receipts.
- Do not purchase services or change billing. Do not make legal claims or filings. Naming/trademark research may be bounded and documented but is not legal clearance.
- Do not publish mobile, web, Windows/Linux packages, App Store artifacts, or notarization claims unless the existing architect contract actually requires them and the necessary authorized credentials exist; otherwise retain them as explicit deferred scope.

## Autonomy and continuity

Use full tool autonomy inside this contract. Do not stop to ask routine implementation, Git, GitHub, CI, documentation, packaging, or release questions that evidence can decide. When two approaches compete, run the cheaper falsification. Make small evidence-bearing checkpoint commits and push them when useful.

If context pressure threatens completion, first create and push a restart-safe checkpoint plus a self-contained continuation receipt naming exact remaining work, then continue in the strongest available fresh context rather than issuing a false completion verdict.

## Terminal verdicts

Return exactly one:

### `COMPLETE`
Only when the bounded desktop product is sealed by current local evidence, documentation matches reality, the public GitHub repository is live, final source is on the default branch, mission-owned hosted checks are terminal green, the release tag and non-draft GitHub Release exist, published assets independently match their manifest, the worktree is clean and remote-equal, and residuals/non-claims are explicit.

### `PARTIAL`
Use only when substantial authorized work is honestly complete but one or more required completion gates remain open and are not a hard blocker. Name each open gate precisely and leave a restart-safe pushed checkpoint.

### `BLOCKED`
Use when a reproduced blocker, unavailable required trust-surface sign-off, missing external capability, or failed mandatory gate prevents completion. Preserve the exact failure, strongest verified checkpoint, proposed next action, and all untouched boundaries.

The final response must cite reproducible artifacts and URLs for every perfective claim. No verdict is predetermined. Evidence decides.