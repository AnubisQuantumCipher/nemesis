# Desktop rails protocol — `nemesis.rails/v1`

The fourteen-rail cockpit is backed by a **governed state repository**: a
private Git repository at `<NEMESIS_HOME>/state` whose entity files may change
through exactly one path — the mission pipeline the SPARK kernel already
governs. The UI proposes; the kernel decides; the desktop adopts.

## Governed rails and entities

Seven rails own governed entities; `changes` and `security` are derived
read-only views.

| Rail | Entity schema | Verbs |
|---|---|---|
| workspaces | `nemesis.rail-workspace/v1` | register, revoke |
| agents | `nemesis.rail-agent/v1` | register, revoke |
| tests | `nemesis.rail-test/v1` | register, revoke |
| knowledge | `nemesis.rail-knowledge/v1` | assert, transition, revoke |
| skills | `nemesis.rail-skill/v1` | propose, advance, revoke |
| automations | `nemesis.rail-automation/v1` | enqueue, revoke |
| integrations | `nemesis.rail-integration/v1` | register, enable, disable, revoke |

Entity files live at `state/<rail>/<id>.json`; ids are 1..48 of `[a-z0-9-]`
starting alphanumeric; every entity carries `schema`, `id`, `status`, and a
monotonic `revision`, and is bounded to **4096 bytes** — the mission write
budget. Rail reads fail closed: one unreadable, oversized, or non-JSON entity
refuses the whole rail read with a typed `BLOCKED_RAIL_STATE`.

## The mutation path (TS-001/TS-002 preserved)

1. The rail panel proposes `{rail, verb, id, payload}`.
2. The rail validator refuses anything its state machine forbids (skipped
   trust states, illegal knowledge transitions, non-empty capability
   requests, unknown verbs) and produces the exact replacement bytes.
3. A brand-new entity file is created empty and committed as plumbing — an
   empty file has no semantics and readers treat it as absent.
4. The mutation becomes a normal `nemesis.desktop-mission/v1` contract on the
   state repository and travels the EXISTING pipeline unchanged:
   compile → AuthorityReview (exact contract and action digests) →
   `create_grant` → `create_approval` → SPARK-proved `Derive_Child_Grant`
   attenuation → SPARK-proved one-shot `Consume_Approval` → authorized lane
   write → deterministic verifiers → signed receipt → one-byte tamper probe →
   independent replay.
5. **Adoption** is mechanical execution of the already-made decision: it
   verifies the persisted result (`status=VERIFIED`, `tamperVerdict=REJECTED`),
   recompiles the persisted contract, checks the lane bytes against the
   kernel-authorized `contentDigest`, checks the canonical file still holds
   the reviewed `expectedSha256` (stale adoptions refuse), then writes the
   exact authorized bytes, commits them, and appends a hash-chained record to
   `receipts/adoptions.jsonl`. Adoption is idempotent per mission.

The desktop never composes authority: no grant, no approval, and no byte of
governed state originates outside the kernel-reviewed path.

## Skill format (`nemesis.rail-skill/v1` + `nemesis.skill-pack/v1`)

A skill is a reviewable local procedure. The body is content-addressed at
`state/content/<sha256>.skill` (immutable; digest collisions with differing
bytes refuse) and the governed record binds `bodyDigest`. The trust ladder is
single-step: proposed → staged → statically-checked → tested-in-sandbox →
evaluated → approved → trusted; `approved` requires a named approver and a
nonzero 128-hex signature; `revoked` is terminal. Reads verify the body
against its digest and surface a broken binding as `BODY UNVERIFIED`.

Shippable skill packs (`library/skills/<id>/`) carry `manifest.json`
(`nemesis.skill-pack/v1`) binding `bodySha256`/`bodyBytes` to the reviewable
body file. Installation is a `propose` mutation whose payload is the body
verbatim — the governed record's digest MUST equal the pack manifest's.

## Plugin format (`nemesis.rail-integration/v1` + `nemesis.plugin-pack/v1`)

Integrations expose tools; NEMESIS retains authority.

- `wasm-plugin`: reviewable WebAssembly text content-addressed at
  `state/content/<sha256>.wat`; the governed record binds `moduleDigest` and
  a single typed `export`. Execution re-verifies the module digest and runs
  inside `nemesis-runtime`'s `WasiPluginHost`: empty capability set, no
  inherited stdio/env/filesystem/network, metered fuel, bounded memory.
  Every run appends to the hash-chained `receipts/plugin-runs.jsonl`.
- `external-provider`: a declared provider (`codex-cli`, `claude-code-cli`,
  `local-model`, `openai-api`, `anthropic-api`) whose tool schemas are
  digest-frozen at registration.

**Deny-by-default is structural**: the only representable capability object
is the empty one; a request naming any network host, filesystem root, secret,
or event refuses at validation. There is no silent network and no secret
view. The kernel-approved digest binding of the registering mission is the
manifest trust anchor — the same binding `SignedPluginManifest` provides in
the runtime crate, with the SPARK-consumed one-shot approval as the root of
trust instead of a local signing key.

Lifecycle: register → enable → disable → enable …; revoke terminal. Each step
is a one-shot-reviewed governed mutation; install/enable/disable/revoke all
land in the adoption chain.

## Automations boundary

The automation schema can express exactly one operation: `draft-mission`.
Network, secrets, approval, push, publish, and external delivery are
unrepresentable. Dispatch is idempotent per trigger key, rate-bounded per
minute, refuses templates that target the governed state repository, and
records every decision — including refusals — in the hash-chained
`receipts/automation-runs.jsonl`. A dispatch's terminal success is
`AUTHORIZED_DRAFT_ONLY`: a draft exists for human review, and compile,
review, approval, and run remain human acts.

## Receipts

All receipt streams (`adoptions`, `test-runs`, `automation-runs`,
`plugin-runs`) share one chain discipline: strict sequence, `previous` hash
linkage from a 64-zero genesis, and a SHA-256 entry hash over the canonical
record. A single flipped byte anywhere refuses the whole chain.

## Gates

- `cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib` — rail
  validators, state machines, chain tamper rejection, WASI execution bounds.
- `cargo test --manifest-path desktop/src-tauri/Cargo.toml --test
  rail_governance` — library pack hash-binding (always) and, with built
  executables, `governs_builtin_skill_and_plugin_through_the_full_authority_path
  -- --ignored --exact`: three real missions through the Ada daemon install,
  enable, and execute the shipped built-ins end-to-end.
- `npm --prefix desktop test` — all fourteen destinations, panel behavior,
  typed refusal surfacing, mutation request shapes.
