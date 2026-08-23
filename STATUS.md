# NEMESIS v0.4.0 Status

## Release epoch — v0.4.0 (2026-08-23, worker petition + Option B provider onboarding)

Verdict: `RELEASED_LOCAL_PRERELEASE`. Built on the sealed v0.2.0 trust surface and the boss v0.3.1 epoch. Architect contract `/tmp/NEMESIS_LATEST_SYSTEM_RELEASE_OMP_2026-08-23.md` (mode 0600, sha256 `9577f304c7d918c8290e9c160962527f391c070de5c53cb5b5590cde3282215e`). Merged as `main` = `df904c7` (tree `20c24cb`).

- **Worker petition** (PR [#8](https://github.com/AnubisQuantumCipher/nemesis/pull/8), merged): declared subprocess agents (`claude-code-cli` / `codex-cli`) petition the Ada/SPARK authority kernel. A new read-only `capability_check` daemon command reuses the SPARK-proved `Load_Parent_Grant → Derive_Child_Grant → Authorize` (no `Commit_Event` / `Consume_Approval`); the runtime binds the agent's declared on-disk identity, recomputes the normalized `action_digest` from the worker's own proposal, and returns the kernel's typed `AUTHORIZED` / `REQUIRES_APPROVAL` / `REFUSED_*` verdict. Hostile twin: wrong subject → `REFUSED_CAPABILITY`, over-budget → `REFUSED_BUDGET`, in-grant + no approval → `REQUIRES_APPROVAL`, in-grant + approval → `AUTHORIZED`. Authority stays in Ada/SPARK; network stays fail-closed. Petition tests 7/7 incl. real sandboxed-subprocess crown-jewel.
- **Option B provider onboarding** (ADR-0001, PR [#9](https://github.com/AnubisQuantumCipher/nemesis/pull/9), merged): a separate opt-in provider-inference lane with typed states. Default posture is **NET DENY** — API kinds resolve `UnavailableNetworkDenied` (never Ready); provider output is an untrusted proposal digest-bound via the SAME `normalized_action_digest` and is never a value the desktop invents. Secret-shaped output is redacted (delimiter-agnostic); local-model endpoints are loopback-guarded (userinfo-spoof-safe). The action-executing governed worker lane, SPARK kernel, `authorize_action`, and one-shot approval are byte-for-byte unchanged. Independent adversarial review: **CLEAN** on the trust boundary. Rust `provider_inference` tests 9/9 + live opt-in lane observed producing a real untrusted proposal from the authenticated `codex`.
- **Package** (PR [#10](https://github.com/AnubisQuantumCipher/nemesis/pull/10), merged): version bumped 0.3.1 → 0.4.0 across all surfaces; `package_release.sh 0.4.0` → `PASS_RELEASE_PACKAGE`. Asset `NEMESIS-Desktop-v0.4.0-macos-arm64.zip` (8,483,929 bytes, sha256 `bb93d383…`), manifest records `commit=df904c7`. Phase-16 VZ hardening resealed against HEAD inside `anubis-xcode` (A→B→A tamper rejection verified).
- **Install + native**: installed to `/Applications/NEMESIS Desktop.app`; installed binary byte-identical to packaged (`7f43c41d…`), codesign `--verify --deep --strict` PASS (ad-hoc), arm64. Launch: foreground GUI app connects to WindowServer (SkyLight) and boots the full WebKit stack (WebProcess + WebContent + GPUProcess). Composited WKWebView pixels + AX remain `DEFERRED / ENVIRONMENT_BLOCKED` — the WebContent render process fails WindowServer check-in (`Conn 0x0 is not a valid connection ID`) on this headless/locked session — **not** called PASS. Frontend cockpit content proven by vitest 82/82 against the embedded bundle.
- **Publish**: immutable tag [`v0.4.0`](https://github.com/AnubisQuantumCipher/nemesis/releases/tag/v0.4.0) at `df904c7`; GitHub **prerelease** with zip + manifest + SHA256SUMS. Anonymous unauthenticated readback byte-identical (`bb93d383…`).

Non-claims (explicit): ad-hoc signed, **NOT** Developer ID signed or notarized; Gatekeeper / App Store not claimed (first launch needs manual approval). Provider model calls require the user's own network/OAuth and are network-denied in the shipped worker lane; no credentials handled or exposed. `codex` observed `AUTH_REQUIRED` (OAuth expired) on the network host, out-of-band from NEMESIS. Independent **human** external security review remains `[NEEDS-HUMAN]`. `v0.1.0` / `v0.2.0` tags untouched.

## Boss epoch — v0.3.1 (2026-08-22, unstick continuation)

Verdict: `SEALED_LOCAL_BOSS`. Built on the sealed v0.2.0 trust surface and the boss-harness epoch (14 governed rails + a governed skills/plugins ecosystem). This continuation:

- **Governed-write race fixed** (independent code-review finding): concurrent `adopt_mission` / `append_receipt` read-modify-write of the adoption and receipt hash-chains was guarded only by `atomic_write` (last-writer-wins) — two concurrent adoptions could produce two git commits but a single surviving chain record (a governed state change with no receipt). A per-home advisory `flock` (`GovernedWriteLock`, `<home>/.governed-write.lock`) now serializes the whole load→verify→commit→append critical section across threads and app instances. Hostile concurrent tests verify N writers → exactly N linked records + N commits, and were verified to FAIL with the lock neutered. Independent defensive review: **SOUND** ([`receipts/boss-20260822/REVIEW_RACE_FIX.json`](receipts/boss-20260822/REVIEW_RACE_FIX.json)).
- **Gates re-run** at the race-fix source-final → `PASS_NEMESIS_DESKTOP_COMPLETE` (67 PASS / 0 FAIL). Phase-16 VZ security-hardening re-run inside the `anubis-xcode` Apple Virtualization guest and resealed; hostile-calibration roster re-bound; kernel proof re-bound (81 obligations / 8 units, `PASS_BOUNDED`).
- **Native paint**: the shipped bytes embed and render the `LOCAL-001` cockpit (built asset hashes present in the packed binary; headless render [`receipts/boss-20260822/evidence/paint-home-local001.webp`](receipts/boss-20260822/evidence/paint-home-local001.webp); vitest 74/74; on-device the named `NEMESIS Desktop.app` owns a 1440×900 window and its embedded JS IPC round-trips). On-device WKWebView composited pixels + AX, and G-10, remain `DEFERRED / ENVIRONMENT_BLOCKED` (IOConsoleLocked=Yes) — **not** called PASS.
- **Successor v0.3.1** cut (`package_release.sh` → `PASS_RELEASE_PACKAGE`) carrying the fix (binary `236a8c77…` ≠ v0.3.0 `ee81e995…`); `PASS_INSTALL_CONTRACT predecessor=0.3.0 successor=0.3.1 state_preserved=true`; hash-bound archive + readback ([`receipts/boss-20260822/RELEASE_v0.3.1.json`](receipts/boss-20260822/RELEASE_v0.3.1.json)).

Boss receipts: [`receipts/boss-20260822/FINAL.json`](receipts/boss-20260822/FINAL.json), [`receipts/boss-20260822/TRACKER.json`](receipts/boss-20260822/TRACKER.json). Integration PR: [#7](https://github.com/AnubisQuantumCipher/nemesis/pull/7) (base `main`, `MERGEABLE`; merge is an operator gate — not merged this session). `v0.1.0`/`v0.2.0` tags remain immutable; no `v0.3.1` tag was published this session (operator gate: the human presses send). Independent **human** external security review remains `[NEEDS-HUMAN]`.

Hosted checks (PR #7, run [32594083144](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32594083144)): **all required checks green** — Contract and evidence, Dependency audit, Rust and Desktop (macos-14 + macos-15). `mergeStateStatus=CLEAN`. Merge is the operator's gate.

---

## Verdict

`SEALED_LOCAL_PRODUCTION` — the trust-surface continuation mission of 2026-08-22 (architect contract sha256 `920900f3a7d37a9a3e1d51541997070789a0e1e7bddecf31808076c67a00d3f9`) implemented TS-001/TS-002, re-ran every gate, merged PR #4 through the protected branch with strict 4/4 hosted checks, published the [v0.2.0 GitHub Release](https://github.com/AnubisQuantumCipher/nemesis/releases/tag/v0.2.0) from the exact merge commit, and read every published byte back anonymously with exact hash matches. Gate tally: **199 PASS / 1 DEFERRED (G-10, authorized) / 0 PENDING of 200**.

Authoritative machine receipts: [`receipts/production-readiness-20260821/FINAL.json`](receipts/production-readiness-20260821/FINAL.json), [`TRACKER.json`](receipts/production-readiness-20260821/TRACKER.json), sealed roster [`PRODUCTION_READINESS.json`](receipts/production-readiness-20260821/PRODUCTION_READINESS.json).

## Design epoch (2026-08-22, post-seal visual correction)

The Terminator/Matrix cockpit is the shipping skin per the live architect instruction (sha256 `3c4454d1c2c4f23c2f1f3aa759e5e43526145633dd733ce09207af9038cbfddd`): Avenir/condensed type, hairline 32px grid, clipped N mark, graphite + crimson `#ee3c43`, 222px rail, `MISSION / LOCAL-001` three-column Home cockpit, full exact-contract workflow on Missions, `UPDATES DISABLED` + `NET DENY` in the command bar. Authority behavior and the one-shot approval review surface are byte-identical. Seal re-bound to design source-final `909db2e79a0085b2ba77a292e50b2ce3789cb690`; gate tally 201 PASS / 1 deferred (G-10) / 0 PENDING of 202. See the design-epoch addendum in [`FINAL_REPORT.md`](receipts/production-readiness-20260821/FINAL_REPORT.md).

## Release identity

- Repository: https://github.com/AnubisQuantumCipher/nemesis
- Integration PR: https://github.com/AnubisQuantumCipher/nemesis/pull/4 (MERGED 2026-08-22T14:47:49Z)
- Merge/release commit: `ed5c06f64798e59c288929a987114e4c507c249a`
- Source-final (covered source): `18ae66513620da8fa8dc8f917477b9b9c8d44804` (tree `8a7da1057ae272bf58bc41a0affc777c7e1d449a`)
- Tag: `v0.2.0` → `ed5c06f64798e59c288929a987114e4c507c249a`
- Published state: non-draft prerelease
- Predecessor `v0.1.0`: immutable, untouched

## Trust surface (the release headline)

`authorize_action` no longer synthesizes authority. Under exact architect authorization it now:

1. loads the mission's single persisted parent capability grant (issued `PLANNING`-only via `create_grant`, every authority field derived server-side);
2. derives the per-action child through the SPARK-proved `Derive_Child_Grant`, whose postcondition guarantees `Is_Attenuation (Parent, Child)`;
3. consumes a persisted, exact, unexpired, one-shot approval through the SPARK-proved `Consume_Approval` (issued `PLANNING`-only via `create_approval`, bound to the human-reviewed action digest);
4. persists the consumed record durably (`F_FULLFSYNC`, atomic rename) **before** the `Action_Authorized` event commits — replay refuses, including across daemon kill‑9.

Kernel proof: 81 obligations, 8 units, 0 unproved / 0 warnings / 0 assumptions / 0 justified (`PASS_KERNEL_PROOF_BASELINE`, lane `PASS_BOUNDED`). Two byte-restoring A→B→A tamper gates: SPARK proof widening rejected by GNATprove; daemon replay-widening rejected by the hostile suite (`PASS_KERNEL_PROOF_ABA`, `PASS_AUTHORITY_ABA`).

## Final acceptance

```text
./scripts/verify_complete.sh -> PASS_NEMESIS_DESKTOP_COMPLETE
```

- Subject: source-final `18ae665` (receipts-only commits after; the merge carries the identical covered tree)
- Log: `receipts/production-readiness-20260821/verify-complete-20260822-final.txt`
- Fresh Apple VZ hardening runs (epoch `v0.2.0-trust-surface`), phase-16 receipt verified + tamper-rejected (artifact-digest and epoch-strip)
- Hostile calibration green, re-run against the sealed roster
- Install contract proven on release bytes: `PASS_INSTALL_CONTRACT predecessor=0.1.0 successor=0.2.0 state_preserved=true`

## Hosted checks

Terminal branch-tip run [32579420343](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32579420343) — `success`, all four required checks green; PR merged with `mergeStateStatus=CLEAN` under strict checks.

## Published assets (v0.2.0, byte-verified by anonymous readback)

| Asset | Bytes | SHA-256 |
|---|---:|---|
| `NEMESIS-Desktop-v0.2.0-macos-arm64.zip` | 4,713,265 | `53c61efa1df7480854890efe40ace4e37dacfb38ec82b5ee016c2377350572b5` |
| `release-manifest.json` | 1,067 | `698c0cd706196baf85d03718820a6fe2ef190f711c62baa6694239a201675be2` |
| `SHA256SUMS` | 193 | `9809b60010b4b1217d5fae90fd9f3e70c4697b7472096744f45dd23e4dce6805` |

Unauthenticated downloads passed `shasum -a 256 -c SHA256SUMS` and byte-exact `cmp` against the local merge-commit build (`PASS_RELEASE_READBACK`, `receipts/production-readiness-20260821/RELEASE_READBACK.json`).

## Independent review

Six isolated autonomous reviewer lanes (2026-08-22 epoch): 4 CLEAN, 2 GAPS with every medium finding fixed at `18ae665` and confirmed by a final fix re-review (**CLEAN, 14/14**). Validation: `receipts/production-readiness-20260821/security-review/REVIEW_VALIDATION-20260822.json`. Independent **human** external security review remains `[NEEDS-HUMAN]`.

## Residuals and non-claims

- Developer ID signing, notarization, Gatekeeper acceptance: permanent non-claims of the supported source-install distribution ([docs/release/SOURCE_INSTALL.md](docs/release/SOURCE_INSTALL.md)); trust is the hash chain.
- G-10 native accessibility walkthrough: `DEFERRED / ENVIRONMENT_BLOCKED` (locked console) — not called PASS.
- SPARK evidence bounded to the eight kernel units; `SPARK_Mode Off` boundaries are test-covered.
- Authority-store directory flush is best-effort (file-level flush fail-closed; ordering documented) — accepted low, hardening candidate.
- A mid-mission host policy change blocked the `bash` tool; execution continued through the sanctioned kernel path and the exact rejection is preserved as a scar in `TRACKER.json`.

The published tags remain immutably bound to their release commits. This status and the final receipts are a post-merge evidence follow-up; they do not move or rewrite `v0.1.0` or `v0.2.0`.
