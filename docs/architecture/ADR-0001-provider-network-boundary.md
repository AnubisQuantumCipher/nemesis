# ADR-0001 — Ordinary-user provider onboarding vs the NET DENY trust surface

- Status: **PROPOSED — architect sign-off required** (`SIGNOFF_REQUIRED_PROVIDER_NETWORK_BOUNDARY`)
- Date: 2026-08-23
- Mission: NEMESIS latest-system release (`/tmp/NEMESIS_LATEST_SYSTEM_RELEASE_OMP_2026-08-23.md`)
- Baseline: `origin/main` = `9561dfa8b489c4db7c81c0145994c12ce80951bf` (PR #8 merged, tree `fcfd90bf…`, all required checks green)

## Context

Phase 2 of the release mission asks for a "safe ordinary-user provider/onboarding path" giving a public
user a truthful choice among: (1) Claude Code subscription worker, (2) Codex/ChatGPT subscription worker,
(3) API-key provider, (4) local model, (5) the offline NEMESIS protocol worker.

NEMESIS's shipping identity (binding, per the mission) includes `NET DENY`: the worker sandbox
`SandboxProfile::workspace_safe` emits `(deny network*)`, and the LOCAL-001 cockpit displays `NET DENY`
as a product promise. The contract's non-negotiable boundary (line 84): "Do not silently weaken NET DENY,
the SPARK kernel, `authorize_action`, `capability_check`, approval consumption, or verifier acceptance.
Detecting an installed CLI is not evidence that it can work. A successful `--version` is not a functioning
provider." Line 90: if enabling a real cloud CLI requires changing a trust-surface network rule, worker
sandbox policy, authorization predicate, or PASS meaning — stop, produce this artifact, and return
`SIGNOFF_REQUIRED_PROVIDER_NETWORK_BOUNDARY`.

## Empirical findings (observed firsthand on this host, 2026-08-23; secrets never printed)

Each installed CLI was probed with a real bounded invocation, not just `--version`:

| Provider | Present | Executable | Auth state (observed) | Functional on network host? | Inside NEMESIS worker (`NET DENY`) |
|---|---|---|---|---|---|
| `codex-cli` (`/Users/sicarii/.local/bin/codex`) | yes | yes | `codex login status` → "Logged in using ChatGPT" | **OBSERVED FUNCTIONAL** — `codex exec "Reply with exactly OK"` → `OK`, rc 0, 12,307 tokens | cannot reach OpenAI → `UNAVAILABLE_NETWORK_DENIED` |
| `claude-code-cli` (`/Users/sicarii/.local/bin/claude`) | yes | yes | `claude -p` → "Failed to authenticate: OAuth session expired" | `AUTH_REQUIRED` (would also need network once re-authed) | `AUTH_REQUIRED` + `UNAVAILABLE_NETWORK_DENIED` |

Supporting facts already in the merged baseline:
- `claude --version` returns cleanly but **EPERMs / blocks** under the strict one-exec `workspace_safe`
  profile (reproduced during the worker-petition mission); it is not sandbox-runnable as an in-lane worker.
- The proven **offline NEMESIS protocol worker** (`nemesis-deterministic-worker`) petitions the kernel and
  is authorized/refused correctly under full `NET DENY` — this is the one lane that functions today.

Conclusion of the probe: a subscription CLI **genuinely functions** (codex returned a real model response),
and the *only* thing preventing it from working inside NEMESIS's governed worker lane is `NET DENY`. This is
the "observed, not detected" distinction the contract demands (line 84).

## The boundary

Model inference for any cloud/subscription/API provider — and even a local OpenAI-compatible model over
`127.0.0.1` — requires network egress. NEMESIS's governed worker lane denies **all** network
(`(deny network*)`), and the shipping face promises `NET DENY`. Therefore **no** external provider lane
(1–4 above) can be made functional for an ordinary user without changing a trust surface. Only lane 5
(offline protocol worker) functions under the current promise.

## Provider adapter contract (typed states) — specification for post-sign-off implementation

The onboarding must classify every provider into exactly one typed state and **never label provider
presence as provider function** (contract line 121):

- `READY` — declared, executable present + `+x`, auth valid, and the selected lane's network policy permits
  its transport. Under current `NET DENY`, only the offline protocol worker and a network-free local CLI
  can reach `READY`.
- `AUTH_REQUIRED` — executable present but the official CLI's own auth is missing/expired (observed for
  `claude`). NEMESIS must **not** read, refresh, copy, print, persist, or proxy the OAuth token; the user
  re-authenticates through the provider's own `login` flow.
- `MISSING_EXECUTABLE` — declared path absent or not executable.
- `UNAVAILABLE_NETWORK_DENIED` — provider requires network the current trust surface denies (observed for
  `codex` inside the lane, and structurally for every cloud/API/loopback lane).
- `REFUSED_CAPABILITY` — the kernel (`capability_check`/`authorize_action`) refused the proposed action; an
  authority decision, never a provider decision.
- `BLOCKED_PROVIDER_POLICY` — an undeclared provider, a spoofed identity/path, an attempted ambient network,
  or a secret-shaped output was detected and refused.

The existing merged baseline already implements the `READY` / `MISSING_EXECUTABLE` /
`UNAVAILABLE_NETWORK_DENIED` foundation in `desktop/src-tauri/src/rails/agents.rs::availability` and
`desktop/src-tauri/src/petition.rs::resolve_agent`; `AUTH_REQUIRED`, `REFUSED_CAPABILITY` (surfaced from the
kernel), and `BLOCKED_PROVIDER_POLICY` are the additions this ADR proposes once the network decision is made.

## Required RED negative controls (specification; to be authored before any adapter code)

Absent executable; unauthenticated official CLI; revoked/expired auth; wrong executable identity/path
replacement; output overflow/timeout/cancellation; malformed provider output; secret-shaped output
redaction; attempted ambient network or undeclared provider; API key never appearing in
config/log/receipt; local loopback endpoint refusing non-loopback destinations; worker proposal still
requiring kernel `authorize_action` + exact one-shot approval.

## Options for the architect

**Option A — grant the governed worker lane network.** Add a network-egress allowance to
`workspace_safe` for provider lanes. **Rejected without sign-off**: it directly weakens `NET DENY` and the
worker-sandbox policy — the core provable-OS promise for the action executor. Do not do this.

**Option B — two-lane split (recommended for sign-off).** Keep the **action-executing** worker at absolute
`NET DENY` and unchanged authority (`capability_check`/`authorize_action`/one-shot approval). Run the
**provider-inference** step as the user's *own* already-authenticated official CLI in a *separate*
network-enabled process **outside** the governed lane, with credentials provider-owned (never read/copied
by NEMESIS). Its output is an untrusted proposal that still passes the kernel unchanged. This preserves the
authority trust surface but **changes the product's network posture**: NEMESIS would cause network egress
via the provider CLI, so the cockpit `NET DENY` promise must be re-expressed as, e.g., "authority lane:
NET DENY; provider inference: user-owned network, opt-in". That re-expression is a **product-identity /
shipping-face decision** (the mission forbids restyling/renaming LOCAL-001 without authorization) and thus
requires architect sign-off.

**Option C — keep `NET DENY` absolute.** Ship only the offline protocol worker as functional; every cloud/
API/local-network lane is honestly `UNAVAILABLE_NETWORK_DENIED` in onboarding. No trust-surface change, but
the "ordinary-user subscription provider" gap is *not* closed — it is documented, not delivered.

## Non-negotiables preserved by this mission (nothing weakened)

`NET DENY`, the SPARK kernel, `authorize_action`, `capability_check`, one-shot approval consumption, and
verifier acceptance are **unchanged**. No OAuth token was read, copied, printed, persisted, or proxied
(the probes surfaced only the CLIs' own status strings and a redacted `OK`). No verifier accept condition
or PASS meaning was altered. PR #8 (worker-petition + LOCAL-001 cockpit + Phase-16 reseal) is merged and
hosted-green as the clean baseline.

## Decision required

The architect must choose Option A (reject), Option B (recommended; a product-identity/network re-expression),
or Option C (status quo). Enabling any functional subscription/cloud/API/local-network provider is a
trust-surface change outside this mission's standing authorization. Until that decision is made, ordinary-user
subscription-provider readiness is **not** claimed.

Terminal verdict for this mission: `SIGNOFF_REQUIRED_PROVIDER_NETWORK_BOUNDARY`.
