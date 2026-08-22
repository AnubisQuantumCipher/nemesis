# Changelog

This file records user-visible NEMESIS changes. Verification status comes from the named gates and receipts, not from this summary.


## 0.3.1 — 2026-08-22

Concurrency hardening for the governed hash-chained logs.

### Fixed

- Governed-write race (independent code-review finding): concurrent
  `adopt_mission` / `append_receipt` read-modify-write of the adoption and
  receipt chains was guarded only by `atomic_write` (last-writer-wins) with no
  lock. Two concurrent adoptions could produce two git commits but a single
  surviving chain record — a governed state change with no receipt — or fail on
  git `index.lock` contention. A per-home advisory lock
  (`GovernedWriteLock`, `flock(LOCK_EX)` on `<home>/.governed-write.lock`) now
  serializes the whole load→verify→commit→append critical section across
  threads and across app instances sharing a home. Hostile concurrent tests
  (`concurrent_adoptions_serialize_and_lose_no_chain_record`,
  `concurrent_receipt_appends_preserve_every_record`) verify N concurrent
  writers yield exactly N linked records and N adoption commits; verified to
  fail with the lock neutered. Independent defensive review: SOUND
  (`receipts/boss-20260822/REVIEW_RACE_FIX.json`).

## 0.3.0 — 2026-08-22

Boss-harness completion: the cockpit becomes a fourteen-rail product with a
governed skills/plugins ecosystem, and production builds can no longer paint
blank.

### Included

- NativePaint (proved and closed): a binary compiled without the Tauri CLI's
  `custom-protocol` feature embeds no frontend and loads a dead dev URL —
  demonstrated live (the webview issued `GET /` to a probe listener on
  `127.0.0.1:1420`). Production builds now refuse to compile when
  `desktop/dist/index.html` is missing (`FAIL_FRONTEND_DIST_MISSING`) or has
  no mount point (`FAIL_FRONTEND_DIST_HOLLOW`); `package_release.sh` asserts
  the packaged binary embeds the asset map
  (`FAIL_RELEASE_FRONTEND_NOT_EMBEDDED`).
- Nine new rails — Workspaces, Agents, Changes, Tests, Knowledge, Skills,
  Automations, Integrations, Security — joining Home, Missions, Evidence,
  Replay, Settings. Rail state lives in a private governed Git repository;
  every mutation travels the existing pipeline (compile → AuthorityReview →
  TS-001 one-shot approval → TS-002 attenuation → receipt → replay) and is
  then mechanically adopted with digest verification and a hash-chained
  adoption receipt (`protocols/desktop-rails-v1.md`).
- NEMESIS-owned skill format (content-addressed hashed bodies, single-step
  trust ladder, named-approver approval gate) and plugin format (reviewable
  WebAssembly text, digest-frozen tool schemas, structurally deny-by-default
  capabilities, bounded WASI execution with metered fuel). Install, enable,
  disable, and revoke are one-shot-reviewed governed mutations.
- Built-in library: `library/skills/mission-hygiene` and
  `library/plugins/rail-attest`, hash-bound by manifests and exercised
  end-to-end through the real Ada daemon by the new
  `rail_governance` gate.
- Automations stop at unapproved authority: the schema can express only
  `draft-mission`; dispatch is idempotent per trigger key, rate-bounded, and
  every decision (including refusals) lands in a hash-chained receipt.
- Security rail: read-only aggregation of persisted parent grants, one-shot
  approvals (armed/consumed tallies), deny postures, and the verified
  adoption chain. Corrupt authority records are surfaced, never filtered.
- Competitive matrix: six columns (Bridgemind added from official sources),
  ecosystem row split into system (SHIPPED) vs catalog breadth (PARTIAL,
  deliberate).
- Restored Terminator/Matrix cockpit design (first release carrying it) per
  the architect instruction of 2026-08-22 (sha256
  `3c4454d1c2c4f23c2f1f3aa759e5e43526145633dd733ce09207af9038cbfddd`):
  Avenir/Arial Narrow condensed type, hairline 32px grid, clipped N mark,
  dim graphite with crimson `#ee3c43`, 222px rail, `MISSION / LOCAL-001`
  three-column Home cockpit, `UPDATES DISABLED` and `NET DENY` in the
  command bar. Authority behavior and the one-shot approval review surface
  unchanged.

## 0.2.0 — 2026-08-22

Trust-surface integration release: the desktop authority review is now
enforced by the kernel, not merely displayed.

### Included

- TS-001 (architect-authorized): `authorize_action` requires a persisted,
  exact, unexpired, one-shot approval consumed by the SPARK-proved
  `Consume_Approval` and made durable before the authorization event commits.
  Replay refuses, including across daemon crash/restart.
- TS-002 (architect-authorized): the synthesized per-request capability grant
  is gone. One immutable persisted parent grant per mission is attenuated
  per action through the new SPARK-proved `Derive_Child_Grant`, whose
  postcondition guarantees `Is_Attenuation (Parent, Child)`.
- New local protocol commands `create_grant` and `create_approval` (issued
  only in `PLANNING`, driven by the explicit desktop authorization click);
  typed refusal reasons on the authority path.
- New durable authority store (`parent.grant`, `approval-<digest>.apr`) with
  fixed-width strict parsing, `F_FULLFSYNC`, and atomic rename.
- Kernel proof grew to 81 obligations across the same 8 units;
  `Is_Attenuation` is now proved as an exact (iff) characterization.
- Second A→B→A tamper gate on the daemon authority path
  (`scripts/verify_authority_aba.py`): a compiling replay-widening variant is
  rejected by the hostile daemon API test, then byte-identical restoration
  returns the gate to green.
- Authority review panel surfaces the one-shot approval and parent→child
  grant lineage; hostile and crash-recovery authority tests across Ada and
  Python suites.
- Supported-distribution contract formalized around source install and
  hash-verified unsigned artifacts (`docs/release/SOURCE_INSTALL.md`) with an
  executable install/upgrade/uninstall proof
  (`scripts/verify_install_contract.py`).

### Distribution boundary

- Unchanged from 0.1.0: ad-hoc signed, not Developer ID signed, not
  notarized; Gatekeeper acceptance not claimed. Per the architect contract of
  2026-08-22 these are permanent non-claims of the supported source-install
  distribution, never blockers.

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
