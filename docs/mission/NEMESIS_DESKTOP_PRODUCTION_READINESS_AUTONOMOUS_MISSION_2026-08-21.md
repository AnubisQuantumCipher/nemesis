# NEMESIS Desktop — Fully Autonomous Production-Readiness Mission

**Date:** 2026-08-21  
**Architect:** AnubisQuantumCipher  
**Lead:** fresh Oh My Pi session using `openai-codex/gpt-5.6-sol`, Max reasoning  
**Mode:** one objective, deep autonomous execution, no human in the loop  
**Repository:** `https://github.com/AnubisQuantumCipher/nemesis`  
**Dedicated worktree:** `/Users/sicarii/Worktrees/nemesis-desktop-production`  
**Mission branch:** `production/desktop-v1-autonomous`  
**Baseline main commit:** `d18cb7cc5bd918a6e9dd67aa99088dab9503e39b`  
**Prior immutable release:** annotated `v0.1.0` targeting `4e10cfcfeb3adf20ced7a01309ac0b2fb028373b`  
**Prior final receipt:** `receipts/release-v0.1.0/FINAL.json`, SHA-256 `96b444a7c4e8c7f987a3edbf51d28f54cfca8f24e8082f161f045ea493f66f9c`

## Architect authorization and no-human rule

The architect directs this mission to complete **every automatable requirement for a real production-ready NEMESIS Desktop application** without returning for questions, approvals, selections, incremental prompts, button presses, or manual commands. The human is not an execution lane and is not in the loop.

Within this contract, act autonomously on the architect's own local NEMESIS worktree and `AnubisQuantumCipher/nemesis` GitHub repository. You may research current standards, inspect and modify desktop-specific code, tests, documentation, package and build scripts, CI workflows, SPARK contracts, security fixtures, receipts, and release metadata; run local and hosted gates; use isolated Apple Virtualization/Tart guests under the existing host guardrails; spawn bounded independent reviewer agents; create commits; push the mission branch; open and update pull requests; respond to review findings; merge ordinary non-trust-surface changes after all exact-head gates are terminal green; tag and publish a successor release only when its complete gate is met; and read back all hosted state.

Do not ask the architect anything. Resolve ordinary ambiguity from this contract, the live tree, current platform standards, and reproducible probes. Timebox competing approaches by testing the cheapest falsification. Continue across failures until the underlying cause is fixed or an external prerequisite is mechanically proven impossible.

This authorization does **not** permit fabricating evidence, weakening a verifier, deleting an assertion, lowering a threshold, bypassing branch policy, exposing secrets, spending money, making legal filings, changing Apple-account state, typing credentials, or claiming a capability that was not observed. Never ask the architect to perform such a step. Complete all other lanes first and record the unavailable prerequisite only at the terminal verdict.

## One objective

Advance the existing source-visible macOS arm64 alpha into the strongest honestly **production-ready NEMESIS Desktop product** possible, limited to the local macOS desktop application and its required local Core, Kernel, Runtime, evidence, replay, installation, update, recovery, security, and support surfaces.

A polished dashboard, a passing test suite, an ad-hoc ZIP, or a model's opinion is not production readiness. Production readiness requires the application to be functionally complete for its declared desktop product promise, independently reviewed, formally bounded where SPARK applies, installable and supportable, secure by default, resilient under failure, accessible, performant under controlled measurements, releasable from exact source, and sealed by local plus hosted evidence.

## Explicit scope exclusions

Keep these out of the implementation and release boundary:

- iPhone, iPad, Android, web application, browser extension;
- remote-public daemon, cloud-hosted control plane, SaaS, public multi-tenant service;
- Windows and Linux packages;
- App Store/TestFlight work;
- domain acquisition, trademark/name clearance, incorporation, contracts, legal filings, or payment;
- offensive activity against third-party systems;
- unrelated Anubis, JACKAL, ZirOS, DeskTidy, GBrain, Hermes, or other repositories.

Do not use excluded work to inflate completion. Preserve versioned seams only where the current desktop application already requires them.

## Non-interference and ownership

This mission owns only `/Users/sicarii/Worktrees/nemesis-desktop-production` and branch `production/desktop-v1-autonomous`.

Do not modify, reset, clean, stash, rebase, delete, close, or absorb:

- `/Users/sicarii/Desktop/Projects/nemesis` on `main`;
- `/Users/sicarii/Desktop/Projects/nemesis/.worktrees/desktop-local` and its completed OMP sessions;
- `/Users/sicarii/Worktrees/jackal-unified-completion-20260820`;
- any unrelated branch, PR, release, VM base image, or receipt.

The existing `v0.1.0` tag and published release are immutable evidence epochs. Never move, overwrite, replace, or delete them or their assets. A successor release must use a new version/tag and preserve the predecessor.

Before writing, re-probe worktree path, branch, HEAD, upstream, status, worktree roster, live writers, GitHub default branch, branch protection, releases, open PRs, and required checks. Stop writing on unexplained concurrent drift.

## Current observations to re-probe, not trust

At contract authoring time:

- `main` was clean and equal to `origin/main` at `d18cb7cc5bd918a6e9dd67aa99088dab9503e39b`.
- `v0.1.0` was a non-draft prerelease with three assets and byte-identical readback evidence.
- The release was ad-hoc signed and not notarized; Gatekeeper rejection was expected.
- Apple Developer membership was inactive; no Developer ID/notarization capability was available.
- `STATUS.md` named independent external security review `[NEEDS-HUMAN]` and limited SPARK evidence to eight named kernel units.
- `desktop/src/components/MissionCockpit.tsx` rendered non-mission navigation surfaces as placeholders.
- Base `tauri.conf.json` had `bundle.active: false`; release packaging used a separate release configuration.
- The kernel tree contained twenty Ada/SPARK source and test files across approvals, budgets, capabilities, completion, evidence, missions, transitions, types, and roots.

Measure every item again before relying on it.

## Operating doctrine

1. One lead writer owns the worktree.
2. Use a durable task tracker covering every gate in this contract; never collapse the work into a short cosmetic checklist.
3. Use RED → GREEN → REFACTOR for every defect and requirement that can be tested.
4. Preserve failed attempts and exact causes.
5. Commit and push bounded non-final checkpoints after coherent verified slices; label them non-final.
6. Re-run affected gates after every material fix and after every commit that changes tracked-file inventories.
7. Never infer completion from process presence, prose, a screenshot, one green job, or an earlier commit.
8. Keep verified, believed, unknown, and blocked separate.
9. Backlog adjacent discoveries with exact `path:symbol` pointers and continue the objective.
10. Continue autonomously through context pressure by writing restart-safe checkpoint receipts and opening a fresh OMP continuation on the same branch if necessary. Do not wait for the architect.

## Phase A — Admission, claim audit, and production definition

1. Freeze baseline identities and current release/receipt state.
2. Inventory every user-visible desktop claim in README, STATUS, release notes, UI copy, screenshots, package metadata, menus, and onboarding.
3. Build a claim-to-code-to-test-to-receipt matrix. Every production claim must have a real execution path and a runnable gate.
4. Identify every placeholder, dead control, disabled navigation route, source-tree dependency, dev-only path, unhandled error, or simulated status in the desktop product.
5. Define one machine-readable production-readiness roster with independent typed lanes, not one Boolean. At minimum: function, authority, evidence/replay, formal kernel, security review, dependency/supply chain, accessibility, performance, reliability/recovery, install/upgrade/uninstall, privacy, packaging/signing/notarization, hosted CI, release/readback, supportability.
6. Derive overall state fail-closed from required lane statuses. `SKIPPED`, `UNKNOWN`, `INDETERMINATE`, or `BLOCKED` may not become PASS.
7. Add an independent validator for the roster with duplicate-key and contradictory-state poisons.

## Phase B — Complete the actual Desktop product

Every visible navigation destination and control must either be fully functional for the declared desktop product or be removed from the production UI. No `surface-placeholder`, dead button, fake number, canned success, or documentation-only feature may remain behind a production claim.

Implement and verify the coherent desktop workflow end to end:

- first launch and local-home initialization;
- mission creation from an exact local contract;
- authority review with normalized action digest;
- explicit local authorization semantics without ambient authority;
- mission execution through the bundled Core/Kernel/Runtime path;
- live progress and durable state after relaunch;
- agents/workers, bounded capabilities, budgets, context and workspace visibility where claimed;
- evidence inspection, source binding, receipt details, and tamper rejection;
- replay from authoritative events with explicit distinction between state replay and model-token replay;
- policies and approvals where claimed;
- settings that are real, validated, persisted, reversible, and safe by default;
- useful empty, loading, degraded, refusal, crash, corruption, and recovery states;
- keyboard-only operation, menus, focus behavior, window restoration, and quit/relaunch semantics;
- no source checkout required by the installed application;
- no private absolute paths, credentials, debug symbols, development servers, or mutable external scripts in shipped bytes.

Use the smallest truthful release promise. Do not retain unfinished surfaces merely to look broad.

## Phase C — Full SPARK authority-kernel program

The prior eight-unit proof boundary is not sufficient for this mission's production claim.

1. Inventory every Ada/SPARK unit that can influence authorization, approvals, capabilities, budgets, policy, mission transitions, evidence acceptance, completion, receipt state, or durable authority.
2. Build a checked obligation manifest covering every authority-bearing package spec/body and every public operation. Include exact source hashes, toolchain/prover versions, proof level/options, generated obligation counts, discharged counts, timeouts, unproved checks, assumptions, and exclusions.
3. Strengthen implementation contracts and tests until every in-scope authority-bearing unit has current-source GNATprove evidence for absence of run-time errors and the strongest feasible functional contracts.
4. Cover cross-unit invariants: authorization cannot widen; approvals are exact, expiring, and one-shot; budgets are monotone; capabilities are attenuated; invalid state cannot transition; evidence cannot self-accept; completion requires all mandatory current-source claims; receipt construction cannot invent authority.
5. Add hostile unit and integration tests for boundary values, malformed conversions, replay, stale source binding, double consumption, denied transitions, and exception paths.
6. Require nonzero obligation and unit counts; reject empty proof runs, excluded in-scope files, timeouts, warnings promoted to silence, or stale `.ali`/proof artifacts.
7. Execute a semantic A→B→A gate on a safe disposable copy: A exact bytes and green proof; B compiling authority violation rejected by proof/gate for the intended reason; restore exact A hash; purge owned artifacts; green rerun.
8. Publish a source-bound formal-assurance report that names precisely what is and is not proved. Never extend SPARK conclusions to Rust, WebKit/Tauri, SQLite, Git, Keychain, sandbox, OS, compiler/prover, or dependencies.

## Phase D — Independent autonomous external security review

The lead builder may not approve its own security work.

1. Create at least two fresh isolated reviewer contexts after a candidate source hash is frozen. At least one must be a separate process/context from the lead; prefer a distinct model/provider when available. Record actual model/provider/process identity and limitations. Do not call same-context self-review external.
2. Give reviewers the exact source hash, threat model, trust boundaries, production roster, changed-file manifest, and gate artifacts—not the lead's private reasoning.
3. Reviewer one performs architecture and trust-boundary review. Reviewer two performs adversarial code and release review. Use additional specialist lanes for SPARK, Tauri/WebKit IPC, Rust unsafe/process/path handling, Keychain/signing, SQLite/event durability, package/supply chain, and accessibility where needed.
4. Require severity, CWE/category where applicable, `path:line`, exploit/precondition, impact, reproduction or static reasoning, and remediation.
5. No finding closes from lead prose. Reproduce, test, fix, rerun, and obtain a fresh final review against the final exact diff/source hash. A stale review is provenance, not approval.
6. Archive detached reviewer receipts outside the subject manifest to avoid recursive self-invalidation. Validate verdict schema and reviewer identity.
7. Do not claim a human penetration test or human external audit. The honest lane name is `independent_autonomous_external_review` unless an actual unrelated human organization performed it—which this no-human mission does not assume.

Required adversarial coverage includes:

- Tauri command exposure, CSP, WebView origin and asset protocols, navigation and injection;
- path traversal, symlink/TOCTOU, canonicalization, Unicode and encoded separator attacks;
- process execution, argv integrity, environment sanitization, executable substitution and PATH attacks;
- local protocol authentication/framing, malformed/duplicate JSON, partial reads and restart races;
- capability/approval replay, double consumption, confused deputy and authority widening;
- evidence substitution, stale PASS artifacts, receipt mutation, source mismatch, signer misuse;
- Keychain access control, zeroization, secret leakage, crash logs and environment inheritance;
- SQLite corruption, transaction boundaries, migration failure, concurrent access and fsync assumptions;
- malicious repository instructions, build scripts, plugins/MCP/tool output and prompt injection;
- sandbox profile enforcement and bypass attempts in the approved VZ guest;
- dependency advisories, lockfile integrity, action pinning, SBOM, licenses and provenance;
- release archive privacy, Mach-O dependencies/RPATHs, entitlements, signatures and update path;
- denial of service, unbounded files/arrays/events/logs, disk exhaustion and resource cleanup.

All crash-capable, exploit, fuzz, mutation, malicious-plugin/worker, and sandbox-escape work runs only inside the approved bounded Apple Virtualization/Tart guest under existing 8-vCPU/12-GiB ceilings, host reserve admission, watchdog, orphan cleanup, and per-run `caffeinate`. Never weaken host guards to make a test run.

## Phase E — Reliability, recovery, and data integrity

Build automated, receipt-bearing tests for:

- clean first install and first run;
- normal quit, force quit, crash, kill during commit, and host restart recovery;
- corrupted/truncated/duplicate event and database records;
- interrupted migration and rollback;
- concurrent client access and single-authority ownership;
- disk-full and read-only filesystem behavior;
- missing binaries/resources, permission denial, Keychain denial, sandbox denial, network denial;
- stale worktree/source, deleted workspace, renamed paths, symlink swaps;
- log growth, rotation, redaction, cache bounds, evidence retention and cleanup;
- long-running mission soak and repeated launch/quit cycles;
- upgrade from v0.1.0 data to the successor version, downgrade refusal where unsafe, rollback, and uninstall while preserving or explicitly deleting user data;
- no duplicate action, authority, event, receipt, or notification after restart.

Every expected failure must reach the intended semantic seam. Syntax errors, setup failures, OOMs, timeouts, watchdog kills, or unavailable fixtures are not valid negative evidence.

## Phase F — Accessibility and native usability

Treat accessibility as a release gate, not polish.

- Full keyboard navigation and visible focus across every production surface.
- VoiceOver names, roles, values, grouping, order and live-state announcements.
- WCAG-relevant contrast measurements for normal, muted, error, red authority, verified and disabled states.
- Scalable text and layout at supported macOS display scaling; no load-bearing seven/eight-pixel text that becomes unreadable.
- Reduced-motion behavior for pulse/animation.
- No color-only status distinctions; glyph/text equivalents remain.
- Window resizing at minimum bounds without clipping load-bearing controls.
- Clear destructive/authority actions, refusal reasons, recovery guidance and undo where applicable.
- Automated accessibility/DOM tests plus an observed native macOS accessibility-tree and keyboard walkthrough captured from the built app.

Preserve the black/graphite/crimson mission-cockpit identity while making the production surface readable and operable.

## Phase G — Controlled performance and resource measurements

Validate each instrument before publishing numbers. Record machine, OS, build, tool and workload. Do not generalize beyond them.

Measure at minimum:

- cold and warm launch to usable local-control-plane state;
- idle CPU, RSS and wakeups over a bounded interval;
- mission start latency and UI responsiveness under a controlled bounded mission;
- replay load for declared small/medium/maximum supported event sets;
- database/evidence growth and bounded log/cache behavior;
- sustained soak with repeated missions and launch/quit cycles;
- package size and startup resource availability.

Assert product limits in code and tests. A number is publishable only after ground-truth reconciliation and repeated controlled runs. Performance regression thresholds must be justified from the observed machine/workload and fail closed without pretending universality.

## Phase H — Install, update, rollback, uninstall and distribution

1. Produce a normal macOS installation artifact and document one authoritative install path.
2. Verify the exact installed `/Applications`-style app from a clean temporary user-data root, not the build tree.
3. Verify all bundled resources, executable architecture, minimum macOS, entitlements, Mach-O dependencies/RPATHs, privacy strings, license/notices, SBOM and manifest.
4. Implement or explicitly exclude automatic updates. If implemented, require signed metadata, rollback/freeze/replay protection, exact channel/version policy and negative controls. Never ship an unauthenticated updater.
5. Verify upgrade from v0.1.0, migration failure recovery, rollback, uninstall and data-retention choices.
6. Re-download every published asset anonymously and compare bytes and hashes.
7. Developer ID signing, hardened-runtime signing, notarization and Gatekeeper acceptance are required for `COMPLETE_PRODUCTION_PUBLIC`. Re-probe capability without printing identities or secrets. Do not prompt for credentials or account action. If the inactive membership makes these impossible, finish every other lane and return `BLOCKED_PRODUCTION_PUBLIC` with this exact external prerequisite as a residual. Never use `Open Anyway` as production-signing evidence.
8. Ad-hoc signing may support local validation but cannot satisfy public production distribution.

## Phase I — Privacy, supportability and operations

- Telemetry remains absent or opt-in, schema-documented, locally redacted and deletable.
- No prompts, repository content, secrets or mission evidence leave the Mac without explicit bounded authority.
- Add privacy, data locations, backup/restore, export/delete, diagnostics, log-redaction and vulnerability-reporting documentation matching reality.
- Add support bundle generation that is bounded, reviewable before export, secret-scanned, and disabled from automatic transmission.
- Add operator diagnostics for Core/Kernel/Runtime versions, resource identity, database health, receipt verifier and sandbox status without exposing secrets.
- Provide a production troubleshooting matrix whose commands actually run against the installed build.

## Trust-surface prohibition

Do not silently change what PASS means, a verifier accept condition, an authorization rule, key handling, signature semantics, evidence acceptance, completion policy, branch-protection requirement, or mandatory gate roster.

Fix the product to satisfy existing semantics and add orthogonal stronger gates. Never delete or relax an old gate. If a necessary repair would alter a trust surface, write a source-bound finding and exact proposed patch under the mission evidence directory, continue every independent lane, and leave that patch unmerged. Because this mission has no human-in-the-loop interaction, the terminal state is `BLOCKED_TRUST_SURFACE` rather than asking for approval.

## Mandatory hostile and tamper controls

For every new production gate, calibrate:

- zero-work and empty-corpus rejection;
- malformed, duplicate and contradictory result rejection;
- missing/stale/unlisted artifact rejection;
- source/commit/tree mismatch rejection;
- interrupted-run stale PASS cleanup;
- alternate invocation such as Python `-O` where relevant;
- changed tracked/untracked inventory after commit;
- symlink and path-substitution behavior;
- A→B→A semantic tamper with exact byte restoration and green rerun.

Capture originating command exit status with `pipefail`, timestamps, cwd, tool identity, subject hash, output size/hash and terminal marker. A log without retained exit status is supporting evidence only.

## GitHub and release workflow

- Create and push truthful non-final checkpoint commits after verified phases.
- Open one mission-owned production-readiness PR from `production/desktop-v1-autonomous`.
- Require all current-head checks and independent reviewer receipts to be terminal green.
- Address every load-bearing review finding and rerun reviews against the final hash.
- Do not merge a trust-surface change without exact architect sign-off; no broad autonomy statement substitutes for that boundary.
- Merge ordinary verified production work only through the protected PR; do not force-push or rewrite history.
- Build successor release assets only from the exact final merge commit.
- Choose the successor version from semantic scope. Never call it `1.0.0` unless every `COMPLETE_PRODUCTION_PUBLIC` gate—including Developer ID/notarization—passes. Otherwise publish no production-final release; a clearly labeled prerelease/RC is allowed only when its bounded gates pass and it does not imply production readiness.
- Never move or overwrite `v0.1.0` or replace its assets.
- Create a detached post-release evidence PR if final hosted/readback facts occur after tagging.

## Terminal verdicts

Exactly one terminal verdict is required.

### `COMPLETE_PRODUCTION_PUBLIC`

Use only when all phases are complete, every production roster lane is PASS, no production-visible placeholder remains, all authority-bearing SPARK units have current-source bounded proof evidence, independent autonomous external review is accepted against the final hash, clean install/upgrade/rollback/uninstall and accessibility/reliability/performance gates pass, Developer ID/hardened-runtime signing and notarization succeed, Gatekeeper accepts fresh downloaded bytes, protected exact-head CI is green, successor release assets are anonymously read back byte-identical, and the final receipt validates.

### `BLOCKED_PRODUCTION_PUBLIC`

Use only after **all automatable work is finished and shipped through the strongest safe bounded checkpoint**, when one or more external prerequisites remain mechanically impossible without a human/account/credential/trust-surface decision. Name each blocker, exact failed/refused command or API state, completed lanes, open lanes, and strongest honest product status. Do not stop early merely because signing, notarization, legal clearance or an external account is unavailable.

### `BLOCKED_TRUST_SURFACE`

Use only when every non-conflicting lane is complete but an exact proposed change to PASS/authorization/key/evidence/completion semantics requires architect sign-off. Preserve the proposal unmerged and name its hash/path.

Context checkpoints are not terminal verdicts. Continue automatically after them.

## Final receipt

Produce a machine-readable final receipt plus human report containing:

- baseline, final branch/commit/tree, upstream and dirty state;
- changed-file manifest and diff statistics;
- exact production-readiness roster with typed status/evidence;
- SPARK unit/obligation manifest and proof assumptions;
- independent reviewer identities, reviewed hashes, findings and dispositions;
- command table with CANONICAL / FOCUSED / VZ / HOSTED / BLOCKED classification, exit status, marker and artifact hash;
- hostile-control and A→B→A receipts;
- accessibility observations and artifacts;
- controlled performance raw data, instrument validation and bounded conclusions;
- install/upgrade/rollback/uninstall evidence;
- signing/notarization/Gatekeeper evidence or exact blocker;
- PR/check/release URLs and exact SHAs;
- anonymous asset readback identities;
- failures preserved as scars;
- residuals and explicit non-claims;
- proof that excluded platforms and unrelated worktrees stayed untouched;
- the exact terminal verdict.

The report must never use `production-ready`, `complete`, `verified`, or equivalent without the same row naming a reproducible gate and observed result.

## Immediate first actions

1. Verify this contract's SHA-256 and metrics from the locator message.
2. Re-probe baseline/worktree/branch/upstream/status and coexistence boundaries.
3. Read the current final receipt, STATUS, README, threat model, security policy, release notes, Tauri configs, desktop navigation/components, kernel source roster, proof scripts, CI and package scripts.
4. Initialize the full durable tracker from every phase and gate above.
5. Run the existing canonical baseline and instrument calibration before edits.
6. Begin the claim/placeholder/production-readiness audit and continue autonomously.

Do not answer with a plan. Execute until one terminal verdict is sealed.