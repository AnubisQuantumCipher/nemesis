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
(Anthropic, `code.claude.com/docs`, incl. macOS desktop app), Bridgemind
(BridgeMind One, `bridgemind.ai`, `docs.bridgemind.ai`,
`github.com/bridge-mind`, macOS v0.1.1). "RUBRIC" resolves to two official
products — rubric-app.com (runtime-governance SDK + hosted control plane) and
getrubric.app (dashboard scaffold for other agents) — **neither is a
local-first desktop agent OS; both are excluded from the capability rows
below as non-competitors** (name-collision noted in
`docs/release/NAME_COLLISION_RECONNAISSANCE.md` territory).

## Matrix

| Capability | NEMESIS | Hermes | DeepSeek Harness | OpenAI Codex | Claude Code | Bridgemind |
|---|---|---|---|---|---|---|
| Native macOS desktop app | SHIPPED (`desktop/src-tauri`, Tauri 2; `scripts/package_release.sh` arm64 `.app`) | PARTIAL (TUI + messaging gateway; "Hermes Desktop" is a branded link, no documented macOS `.app` install) | ABSENT (Web UI at `127.0.0.1:3080` + TUI; browser-based) | SHIPPED (inside ChatGPT macOS app) | SHIPPED (dedicated desktop app, universal build) | SHIPPED (signed & notarized universal macOS 26 app, `BridgeMindOne-universal.dmg`) |
| Local-first (no cloud account; offline core) | SHIPPED (all seven backends local binaries; network permanently denied by contract: `production.rs` refuses `authority.network=true`; `test_operations.sh` greps out network APIs) | SHIPPED (MIT; local models "completely free to run") | PARTIAL (DeepSeek API key in quickstart; provider swappable to any OpenAI-compatible endpoint) | ABSENT (ChatGPT account forced; cloud models) | ABSENT (subscription required; Anthropic-hosted models) | ABSENT (browser sign-in + entitlement gate before the workspace mounts; hosted Together Whisper dictation; delegates to hosted-account engines) |
| Explicit pre-execution authority review | SHIPPED (`AuthorityReview.tsx` full-surface review of exact contract + action digests before `run_mission`; every governed rail mutation rides the same surface — `protocols/desktop-rails-v1.md`) | SHIPPED (dangerous-pattern approval prompt: once/session/always/deny; `smart` aux-LLM triage) | PARTIAL (Web UI "asks before operations" under active policy; semantics underspecified in public docs) | SHIPPED (approval policies × sandbox modes, granular categories) | SHIPPED (allow/ask/deny rules, per-tool + per-argument) | PARTIAL (approval cards pause sensitive writes / credit-consuming actions; read-only runs uninterrupted; no published policy semantics) |
| One-shot approvals bound to exact action bytes | SHIPPED (TS-001: persisted approval bound to the exact action digest, consumed once by SPARK `Consume_Approval`, durable before the event commits; replay refuses across crash/restart — `daemon/src/nemesis_core_daemon.adb`, `tests/integration/test_daemon_api.py::test_authority_is_persisted_one_shot_and_fail_closed`) | PARTIAL (`once` is per-execution but pattern/glob scoped; `always` persists a glob rule) | ABSENT (not documented) | PARTIAL ("approve once or for that type this session" — category scope, not action bytes) | PARTIAL ("don't ask again" persists a rule file; file-edit approvals last the session) | PARTIAL ("Allow once" vs "Always allow" per destination — action-scoped, no byte-exact digest binding documented) |
| Capability attenuation with mechanical proof | SHIPPED (TS-002: persisted parent grant → per-action child via SPARK-proved `Derive_Child_Grant`; postcondition `Is_Attenuation (Parent, Child)`; 81 obligations green — `kernel/src/nemesis-kernel-capabilities.ads`, `receipts/production-readiness-20260821/FORMAL_KERNEL.json`) | ABSENT | ABSENT | ABSENT | ABSENT | ABSENT (no attenuation claim in official docs) |
| Formal verification of the authority kernel | SHIPPED (8 SPARK units, 81 obligations, 0 unproved, A→B→A proof tamper gate — `scripts/prove_kernel.sh`, `scripts/verify_proof_aba.py`) | ABSENT (no official proof claim) | ABSENT (no official proof claim) | ABSENT (no official proof claim) | ABSENT (docs: "Permission rules are enforced by Claude Code, not by the model"; no proof claim) | ABSENT (no proof claim in official docs) |
| Deterministic, hash-chained receipts + replay | SHIPPED (305-byte hash-chained ledger; COSE_Sign1 Ed25519 receipts; standalone `nemesis-verify`; tamper fixtures; `nemesis-replay` exact-state reconstruction; hash-chained rail receipt streams — adoptions, test-runs, automation-runs, plugin-runs) | ABSENT (SQLite session db; no signing/chain claim) | PARTIAL (append-only session log with resume/fork/replay; no cryptographic chain) | PARTIAL (opt-in OTel events + enterprise compliance logs; no chain) | ABSENT (user-authored hooks only) | ABSENT (no hash-chain, signature, or replay claim in surveyed docs) |
| Fail-closed refusal semantics | SHIPPED (typed `REFUSED_*` decisions + reasons on the wire; consume-before-authorize burns approvals toward refusal; strict parsers refuse ambiguity; rail reads refuse on any corrupt entity — `BLOCKED_RAIL_STATE`) | SHIPPED (deny-on-timeout; hardline blocklist below `--yolo`) | ABSENT from public docs (no timeout/parse-failure commitment either way — recorded as the finding) | SHIPPED (auto-review parse/timeout failures fail closed) | PARTIAL (critical-path `rm` protection in every mode; but `PreToolUse` hook exit-0 is fail-open) | ABSENT from public docs (connection-verify failure keeps prior state; no tool-execution fail-closed commitment either way — recorded as the finding) |
| OS sandbox for executed actions | SHIPPED (Apple `sandbox-exec` Workspace Safe profile: deny-default, deny-network, single canonicalized executable — `runtime/crates/nemesis-runtime/src/lib.rs`; wasm plugins in bounded WASI: empty capabilities, metered fuel) | PARTIAL (opt-in containers; dangerous-command checks skipped inside them) | PARTIAL (sandboxes are swappable plugins; VM/container recommended, not enforced) | SHIPPED (Seatbelt / bwrap+seccomp / Windows sandbox / microVMs) | PARTIAL (OS-level sandbox for Bash tool only) | PARTIAL (Chat mode sandboxed, "nothing is mounted unless you mount it"; Code mode runs engines as real shells with no described OS-sandbox layer) |
| Crash recovery with durable state proof | SHIPPED (ledger-first commit order; checkpoint rebuild from ledger tail (SQL-001); kill-9 authority durability proven in `test_daemon_api.py`; reliability suite F-02..F-14) | PARTIAL (SQLite persistence; no replay guarantee) | PARTIAL (hot-reload rollback + replay-first event stream; no crash-ordering claim) | PARTIAL (git-branch workflow recommended) | PARTIAL (session resume; no cryptographic guarantee) | PARTIAL (memory/chats persist; schedules resume on reopen with no exact-time guarantee; no cryptographic claim) |
| First-party skill/plugin system (formats, lifecycle, governance) | SHIPPED (NEMESIS-owned formats `nemesis.rail-skill/v1` + `nemesis.rail-integration/v1`: content-addressed hashed bodies/modules, single-step trust ladder, structurally deny-by-default capabilities; install/enable/disable/revoke are one-shot-reviewed governed mutations with hash-chained receipts; built-in pack + plugin exercised end-to-end through the Ada daemon — `protocols/desktop-rails-v1.md`, `library/`, `desktop/src-tauri/tests/rail_governance.rs`) | SHIPPED (`~/.hermes/skills/` agentskills.io standard + `~/.hermes/plugins/` middleware + MCP) | SHIPPED (everything-is-a-plugin on the Cordis kernel; hot-reload with rollback) | SHIPPED (universal plugin directory: skills, connectors, MCP, hooks, browser extensions, scheduled templates) | SHIPPED (plugins bundle skills/agents/hooks/MCP/LSP/monitors; `SKILL.md` layers) | SHIPPED (starter skills library + curated plugin catalog with app-managed credentials) |
| Ecosystem catalog breadth (marketplaces, third-party inventory) | PARTIAL (deliberately small audited first-party library: one skill pack + one wasm plugin; no marketplace; every installable is hash-bound and kernel-reviewed) | SHIPPED (40+ tools, 7 terminal backends, Skills Hub) | SHIPPED (`dsh-plugin` GitHub topic discovery) | SHIPPED (shared ChatGPT↔Codex plugin directory with partner OAuth) | SHIPPED (`claude-plugins-official` + community + private/enterprise marketplaces) | SHIPPED (curated catalog: GitHub, Linear, Stripe, Cloudflare, Gmail, Supabase, and more) |
| Auto-review / reviewer-agent ergonomics | OUT-OF-SCOPE (doctrine: "Intelligence proposes. NEMESIS governs." — approval authority is never delegated to a model; the automations rail can express only `draft-mission`; approval is unrepresentable) | PARTIAL (`smart` mode: aux-LLM triage — auto-approve low-risk, auto-deny dangerous, escalate uncertain; no published policy) | ABSENT | SHIPPED (auto-approval reviewer subagent with published open-source policy) | SHIPPED (classifier-driven `auto` mode) | PARTIAL ("Automatic" composer mode + drafts-awaiting-approval pattern; no published reviewer policy) |
| Windows / Linux / mobile / web surfaces | OUT-OF-SCOPE (constitution pins these DEFERRED; `tests/phase0/test_phase0.py` enforces) | SHIPPED (cross-platform CLI + native Windows installer) | SHIPPED (cross-platform Node; Web UI) | SHIPPED (Windows sandbox; web; mobile tasks) | SHIPPED (Windows/Linux installers; web; iOS/Android) | PARTIAL (macOS shipped; docs record Windows in-development and Linux not supported despite homepage download links; no mobile/web) |

## Position

No verified competitor combines: (a) formal verification of the authority
kernel, (b) hash-chained deterministic receipts with standalone verification,
(c) one-shot approvals bound to exact serialized action bytes, and (d)
local-first native macOS desktop operation. NEMESIS ships all four — and, as
of the boss-harness epoch, a fourteen-rail product surface whose every
governed mutation travels that same proved authority path, plus a first-party
skill/plugin system whose install, promotion, and revocation are themselves
one-shot-reviewed kernel decisions. Bridgemind adds a native-macOS,
skills-and-plugins competitor, but it is cloud-account-bound and delegates
execution to hosted Claude Code / Codex engines, so the differentiated
quadrant is unchanged. The rows where competitors lead (catalog breadth,
reviewer-agent delegation, non-macOS platforms) are deliberately bounded or
pinned OUT-OF-SCOPE by the constitution and Phase-0 doctrine tests.

## Highest-value missing capability — implemented, not just tabled

At the sealed v0.2.0 baseline, the largest in-scope gap between NEMESIS's
claims and its product surface was the ecosystem row: a bounded WASI plugin
host and signed manifests existed in the runtime crate, but there was no
first-party skill/plugin SYSTEM — no formats an operator could install, no
lifecycle, no receipts, no cockpit surface. That capability was implemented
this epoch (2026-08-22, boss-harness completion):

- NEMESIS-owned skill format: governed record + content-addressed hashed
  body, single-step trust ladder, named-approver + nonzero-signature gate,
  terminal revocation (`desktop/src-tauri/src/rails/skills.rs`);
- NEMESIS-owned plugin format: reviewable WebAssembly text, digest-bound
  module, digest-frozen tool schemas, structurally deny-by-default
  capabilities — grants beyond empty are unrepresentable
  (`desktop/src-tauri/src/rails/integrations.rs`);
- install / list / enable / disable / revoke as one-shot-reviewed governed
  mutations, adopted into a private Git state repository and recorded in a
  hash-chained receipt stream (`desktop/src-tauri/src/state_repo.rs`);
- one built-in skill pack and one built-in wasm plugin, hash-bound in
  `library/` and exercised end-to-end through the real Ada daemon by
  `rail_governance::governs_builtin_skill_and_plugin_through_the_full_authority_path`.

Catalog breadth remains deliberately PARTIAL: a small audited library beats
an unaudited marketplace under this product's doctrine. Remaining
competitor-led rows are OUT-OF-SCOPE by doctrine (auto-review delegation,
non-macOS platforms); no in-scope differentiated-core capability remains
ABSENT.

## Source register (competitors, accessed 2026-08-22)

- Hermes: github.com/NousResearch/hermes-agent (README, releases);
  hermes-agent.nousresearch.com/docs/user-guide/security;
  /docs/user-guide/features/skills; /docs/developer-guide/plugins;
  /docs/reference/faq
- DeepSeek Harness: github.com/deepseek-ai/deepseek-harness;
  deepseek.com/harness/en/ (product, privacy, data-processing);
  deepseek-harness.github.io/deepseek-harness/en/guide/quickstart;
  github.com/topics/dsh-plugin
- OpenAI Codex: learn.chatgpt.com/docs/agent-approvals-security;
  learn.chatgpt.com/docs/sandboxing; learn.chatgpt.com/docs/plugins;
  openai.com/index/running-codex-safely/;
  github.com/openai/codex (codex-rs/core/src/guardian/policy.md)
- Claude Code: code.claude.com/docs/en/{overview,desktop-quickstart,
  permissions,permission-modes,hooks,plugins,skills,discover-plugins};
  docs.anthropic.com/en/docs/claude-code/permissions;
  github.com/anthropics/claude-plugins-official
- Bridgemind: bridgemind.ai (homepage, downloads);
  docs.bridgemind.ai/docs/{macos,windows,agent-mode,code-mode,
  skills-and-plugins,routines}; github.com/bridge-mind
- RUBRIC disambiguation: docs.rubric-app.com (architecture, claude-code
  adapter); getrubric.app
