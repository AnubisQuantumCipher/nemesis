# NEMESIS Desktop — Evidence-Backed Competitive Matrix

Verified 2026-08-22 against OFFICIAL sources only (vendor docs, vendor GitHub
orgs, vendor product pages). Matrix values are restricted to `SHIPPED`,
`PARTIAL`, `ABSENT`, `OUT-OF-SCOPE`. Every NEMESIS cell cites repository
code, receipts, or gates; every competitor cell cites the official source
read on 2026-08-22. Absence of official documentation is recorded as the
finding, never guessed around.

Competitors verified: Hermes Agent (Nous Research, `github.com/NousResearch/
hermes-agent`, v0.2.x, MIT), DeepSeek Harness (`github.com/deepseek-ai/
deepseek-harness`, `dsh`, developer preview, MIT), OpenAI Codex (ChatGPT
desktop app / CLI / IDE / cloud, `learn.chatgpt.com/docs`), Claude Code
(Anthropic, `code.claude.com/docs`, incl. macOS desktop app). "RUBRIC"
resolves to two official products — rubric-app.com (runtime-governance SDK +
hosted control plane) and getrubric.app (dashboard scaffold for other
agents) — **neither is a local-first desktop agent OS; both are excluded
from the capability rows below as non-competitors** (name-collision noted in
`docs/release/NAME_COLLISION_RECONNAISSANCE.md` territory).

## Matrix

| Capability | NEMESIS | Hermes | DeepSeek Harness | OpenAI Codex | Claude Code |
|---|---|---|---|---|---|
| Native macOS desktop app | SHIPPED (`desktop/src-tauri`, Tauri 2; `scripts/package_release.sh` arm64 `.app`) | PARTIAL (TUI + messaging gateway; "Hermes Desktop" is a branded link, no documented macOS `.app` install) | ABSENT (Web UI at `127.0.0.1:3080` + TUI; browser-based) | SHIPPED (inside ChatGPT macOS app) | SHIPPED (dedicated desktop app, universal build) |
| Local-first (no cloud account; offline core) | SHIPPED (all seven backends local binaries; network permanently denied by contract: `production.rs` refuses `authority.network=true`; `test_operations.sh` greps out network APIs) | SHIPPED (MIT; local models "completely free to run") | SHIPPED ("locally-first"; local storage by default) | ABSENT (ChatGPT account forced; cloud models) | ABSENT (subscription required; Anthropic-hosted models) |
| Explicit pre-execution authority review | SHIPPED (`AuthorityReview.tsx` full-surface review of exact contract + action digests before `run_mission`) | SHIPPED (dangerous-pattern approval prompt: once/session/always/deny) | PARTIAL (Web UI "asks before operations" under active policy; semantics underspecified in public docs) | SHIPPED (approval policies × sandbox modes, granular categories) | SHIPPED (allow/ask/deny rules, per-tool + per-argument) |
| One-shot approvals bound to exact action bytes | SHIPPED (TS-001, 2026-08-22: persisted approval bound to the exact action digest, consumed once by SPARK `Consume_Approval`, durable before the event commits; replay refuses across crash/restart — `daemon/src/nemesis_core_daemon.adb`, `tests/integration/test_daemon_api.py::test_authority_is_persisted_one_shot_and_fail_closed`) | PARTIAL (`once` is per-execution but pattern/glob scoped; `always` persists a glob rule) | ABSENT (not documented) | PARTIAL ("approve once or for that type this session" — category scope, not action bytes) | PARTIAL ("don't ask again" persists a rule file; file-edit approvals last the session) |
| Capability attenuation with mechanical proof | SHIPPED (TS-002: persisted parent grant → per-action child via SPARK-proved `Derive_Child_Grant`; postcondition `Is_Attenuation (Parent, Child)`; 81 obligations green — `kernel/src/nemesis-kernel-capabilities.ads`, `receipts/production-readiness-20260821/FORMAL_KERNEL.json`) | ABSENT | ABSENT | ABSENT | ABSENT |
| Formal verification of the authority kernel | SHIPPED (8 SPARK units, 81 obligations, 0 unproved, A→B→A proof tamper gate — `scripts/prove_kernel.sh`, `scripts/verify_proof_aba.py`) | ABSENT (no official proof claim) | ABSENT (no official proof claim) | ABSENT (no official proof claim) | ABSENT (docs: "Permission rules are enforced by Claude Code, not by the model"; no proof claim) |
| Deterministic, hash-chained receipts + replay | SHIPPED (305-byte hash-chained ledger; COSE_Sign1 Ed25519 receipts; standalone `nemesis-verify`; tamper fixtures; `nemesis-replay` exact-state reconstruction) | ABSENT (SQLite session db; no signing/chain claim) | PARTIAL (append-only session log with resume/fork/replay; no cryptographic chain) | PARTIAL (opt-in OTel events + enterprise compliance logs; no chain) | ABSENT (user-authored hooks only) |
| Fail-closed refusal semantics | SHIPPED (typed `REFUSED_*` decisions + reasons on the wire; consume-before-authorize burns approvals toward refusal; strict parsers refuse ambiguity; kernel completion court) | SHIPPED (deny-on-timeout; hardline blocklist below `--yolo`) | ABSENT from public docs (recommendation-level only) | SHIPPED (auto-review parse/timeout failures fail closed) | PARTIAL (critical-path `rm` protection in every mode; but `PreToolUse` hook exit-0 is fail-open) |
| OS sandbox for executed actions | SHIPPED (Apple `sandbox-exec` Workspace Safe profile: deny-default, deny-network, single canonicalized executable — `runtime/crates/nemesis-runtime/src/lib.rs`) | PARTIAL (opt-in containers; dangerous-command checks skipped inside them) | PARTIAL (sandboxes are swappable plugins; VM/container recommended, not enforced) | SHIPPED (Seatbelt / bwrap+seccomp / Windows sandbox / microVMs) | PARTIAL (OS-level sandbox for Bash tool only) |
| Crash recovery with durable state proof | SHIPPED (ledger-first commit order; checkpoint rebuild from ledger tail (SQL-001); kill-9 authority durability proven in `test_daemon_api.py`; reliability suite F-02..F-14) | PARTIAL (SQLite persistence; no replay guarantee) | SHIPPED-adjacent (replay-first event stream; no crash-ordering claims) | PARTIAL (git-branch workflow recommended) | PARTIAL (session resume; no cryptographic guarantee) |
| Tool/plugin ecosystem breadth | PARTIAL (bounded WASI plugin host + signed manifests shipped; catalog deliberately small) | SHIPPED (40+ tools, 7 terminal backends, MCP) | SHIPPED (everything-is-a-plugin) | SHIPPED (MCP, skills, IDE surfaces) | SHIPPED (MCP, plugins, skills, hooks) |
| Auto-review / reviewer-agent ergonomics | OUT-OF-SCOPE (doctrine: "Intelligence proposes. NEMESIS governs." — approval authority is never delegated to a model; `automation.rs` permanently denies `Approval`) | ABSENT | ABSENT | SHIPPED (auto-approval reviewer subagent with published policy) | SHIPPED (classifier-driven `auto` mode) |
| Windows / Linux / mobile / web surfaces | OUT-OF-SCOPE (constitution pins these DEFERRED; `tests/phase0/test_phase0.py` enforces) | SHIPPED (cross-platform CLI) | SHIPPED (cross-platform) | SHIPPED (Windows sandbox; cloud) | SHIPPED (Windows/Linux beta) |

## Position

No verified competitor combines: (a) formal verification of the authority
kernel, (b) hash-chained deterministic receipts with standalone verification,
(c) one-shot approvals bound to exact serialized action bytes, and (d)
local-first native macOS desktop operation. NEMESIS ships all four. That
quadrant is the product category; the matrix rows where competitors lead
(ecosystem breadth, reviewer-agent ergonomics, non-macOS platforms) are
either deliberately bounded (small audited plugin catalog) or pinned
OUT-OF-SCOPE by the constitution and Phase-0 doctrine tests.

## Highest-value missing capability — implemented, not just tabled

At the 2026-08-22 baseline (`4f0f387`), the single largest gap between
NEMESIS's differentiated claims and its enforcement was that the daemon's
`authorize_action` synthesized a fresh grant per request and never consumed
an approval — the desktop review workflow was truthful but **displayed**
authority rather than kernel-enforced authority (recorded blockers TS-001 /
TS-002). That capability — the defining row of this matrix — was implemented
this session under exact architect authorization:

- persisted parent grant + SPARK-proved child attenuation (TS-002);
- persisted, exact, crash-durable one-shot approvals (TS-001);
- typed refusal reasons surfaced end-to-end (daemon JSON → mission runner →
  typed `CommandFailure` → cockpit);
- hostile, crash-recovery, and A→B→A tamper evidence
  (`PASS_AUTHORITY_STORE_DURABILITY`, `PASS_AUTHORITY_ABA`,
  `test_authority_is_persisted_one_shot_and_fail_closed`).

Remaining competitor-led rows are OUT-OF-SCOPE by doctrine (auto-review
delegation, non-macOS platforms) or deliberately bounded (plugin catalog
size); no in-scope differentiated-core capability remains ABSENT.

## Source register (competitors, accessed 2026-08-22)

- Hermes: github.com/NousResearch/hermes-agent (README, releases);
  hermes-agent.nousresearch.com/docs/user-guide/security; /docs/reference/faq
- DeepSeek Harness: github.com/deepseek-ai/deepseek-harness;
  deepseek.com/harness/en/ (product, privacy, data-processing);
  deepseek-harness.github.io/deepseek-harness/en/guide/quickstart
- OpenAI Codex: learn.chatgpt.com/docs/agent-approvals-security;
  learn.chatgpt.com/docs/sandboxing; openai.com/index/running-codex-safely/
- Claude Code: code.claude.com/docs/en/{overview,desktop-quickstart,
  permissions,permission-modes,hooks};
  docs.anthropic.com/en/docs/claude-code/permissions
- RUBRIC disambiguation: docs.rubric-app.com (architecture, claude-code
  adapter); getrubric.app
