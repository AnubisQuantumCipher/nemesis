# NEMESIS Desktop Threat Model

**Version:** 2026-08-20 local alpha baseline  
**Scope:** macOS-first local desktop application, Core daemon, Kernel, Runtime, CLI, protocols, evidence, replay, and receipt verifier

## Security objective

Probabilistic or compromised components may propose work but cannot silently obtain authority, rewrite authoritative state, satisfy their own completion claims, widen policy, inherit secrets, or erase evidence. NEMESIS must fail closed and retain enough evidence to distinguish verified state, bounded belief, unknowns, and blockers.

## Protected assets

- Mission contracts, invariants, budgets, approvals, capabilities, and terminal state.
- Event order, sequence numbers, hash chain, checkpoints, and recovery decisions.
- Workspace and source revision identity, user files, Git references, and unrelated worktrees.
- Test, proof, review, artifact, and source-binding evidence.
- Receipt signing key, public verification identity, and canonical receipt bytes.
- Local secrets, provider tokens, SSH material, Keychain items, and environment values.
- Exact worker context, failed approaches, blockers, and causal replay.
- Availability of the local daemon without accepting ambiguous or corrupted state.

## Trust assumptions

The host operator and architect authorization are trusted for local policy decisions. The macOS kernel, filesystem durability semantics, Keychain/Security framework, compiler toolchains, cryptographic libraries, Git, SQLite, and hardware are dependencies and explicitly documented assumptions; they are not proven by NEMESIS. The SPARK proof result applies only to named packages and obligations.

All models, workers, repository contents, generated code, plugins, MCP servers, provider responses, tool output, UI renderer input, imported receipts, and network content are untrusted.

## Threat actors and required controls

| Threat | Attack | Required control | Failure state |
|---|---|---|---|
| Wrong or malicious model | Fabricates success, requests excess authority, or hides residual work | Workers can only propose; Kernel owns decisions; deterministic evidence required | Refusal or `BLOCKED_WITH_EVIDENCE` |
| Repository prompt injection | Source text tells a worker to export secrets, weaken gates, or ignore policy | Context labels repository content untrusted; capabilities and brokers remain authoritative | Refusal recorded with source provenance |
| Fabricated tool output | Worker emits fake passing logs or exit status | Runtime records process-owned status and output digests; worker assertions remain claims | Evidence rejected |
| Malformed tool or provider response | Oversized, invalid, duplicate, unknown-version, or schema-changing message | Strict versioned schema, message bounds, enumerated fields, no worker timestamps/sequences | Typed protocol refusal |
| Capability escalation | Read becomes write, child exceeds parent, resource or host changes | Exact normalized scope, attenuation check, mission/subject binding, expiry, revocation | `REFUSED_CAPABILITY` |
| Approval replay/substitution | One approval authorizes a modified action or another mission | One-shot normalized action digest, mission binding, expiry, consumed state | `REFUSED_APPROVAL` |
| Path traversal | `..`, alternate encodings, absolute paths, mount escape | Canonicalization before authority comparison, exact root, bounded path segments | `REFUSED_CAPABILITY` |
| Symlink or race escape | In-root path resolves outside after check | Symlink resolution, race-resistant open where available, revalidation, special-file rejection | Refusal; no fallback |
| Secret exfiltration | Worker reads inherited environment, Keychain, SSH, or `.env` | Fixed clean environment, secret broker, deny roots, scoped proxy operations, no raw signing key | Refusal and audit event |
| Ungranted network access | Worker contacts an arbitrary host or encodes a nested request | Default-deny sandbox and broker by host/port/protocol/method/path; no ambient network | `REFUSED_CAPABILITY` |
| Worker event forgery | Worker supplies event sequence, authoritative timestamp, capability, approval, or completion | Core discards/declines authoritative fields; Kernel/Core allocate sequence and time | Protocol refusal |
| Cross-lane mutation | Worker modifies canonical repository or another lane | Isolated Git worktree root and sandbox; broker opens only scoped paths | Refusal; lane quarantined |
| Gate weakening | Patch deletes tests, changes expected outcome, bypasses verifier, or changes `PASS` | Trust-surface diff review, preserved hostile tests, architect approval for semantic weakening | Mission blocked |
| Stale evidence | Earlier green result is cited after source change | Source/dependency binding and automatic invalidation before completion | Evidence stale |
| Unauthorized verifier | Builder accepts its own elevated claim or unknown verifier signs evidence | Versioned verifier registry and consequence-class policy | Evidence rejected |
| Receipt mutation | Payload, action, evidence, source, header, or signature byte changes | Canonical CBOR, protected COSE header, Ed25519 signature, full verifier checks | Verification rejected |
| Signing-key theft | Worker inherits or reads signing material | Keychain storage, narrow signer stdin, no private output, sandbox denies Keychain access | Signing unavailable or refused |
| Unavailable signer | Keychain prompt, item, framework, or helper unavailable | Mission state remains complete only if policy permits unsigned local status; signed-receipt predicate stays open | `BLOCKED_WITH_EVIDENCE` for signed receipt |
| Daemon crash | Process dies before, during, or after a state write | Length-delimited append, hash chain, durability boundary, atomic checkpoint, prefix recovery | Recover exact prefix or refuse corruption |
| Ledger truncation/reorder | Tail is cut, event changed, duplicate sequence, predecessor mismatch | Verify length, canonical digest, sequence, previous hash, schema version | Recovery refuses corrupted suffix |
| Duplicate delivery | Client retries an acknowledged or unknown request | Request idempotency key and committed-result lookup | Prior result or typed conflict |
| Compromised frontend | Renderer sends arbitrary IPC, displays different approval, or claims green | Narrow Tauri commands, digest-bound authorization, Core schema validation, status text plus digest | Refusal; UI is not authority |
| Malicious plugin or MCP | Attempts full daemon authority or runtime schema mutation | Separate process/webview, capabilities, CSP, fixed active schema, egress and secret filters | Plugin disabled/refused |
| Dependency compromise | Build or runtime dependency acts maliciously | Pin/review dependencies, isolate build/runtime, dependency receipts, small TCB assumptions | Build blocked or risk marked `UNKNOWN` |
| Resource exhaustion | Worker floods output, processes, disk, time, tokens, or cost | Bounded messages/streams, process/storage/time/cost budgets, cancellation and backpressure | `REFUSED_BUDGET` |
| Provider fallback drift | Fallback silently gets more authority or different evidence status | Authority is mission-owned, not provider-owned; role and consequence ceilings checked | Fallback refused |
| Context omission | New worker misses mission invariant, blocker, revoked fact, or failed approach | Signed bounded context capsule with mandatory obligations and provenance | Assignment refused |

## Sandbox profiles

- `Observe`: no mutation.
- `Workspace Safe`: writes only through a broker inside one isolated worktree.
- `Container`: restricted container boundary.
- `VM`: disposable Apple Virtualization guest.
- `Remote`: separate controlled host; deferred for this mission.
- `Privileged Local`: exceptional and exact human authorization; not part of unattended alpha flow.

Failure to establish the requested profile does not silently fall back to a weaker profile. Crash-capable, fuzz, exploit, or offensive stress evidence is gathered in an approved Apple Virtualization guest, never primarily on the host.

## Receipt and evidence limitations

A valid receipt proves that the included bytes verify under the named key and that the standalone verifier accepted the encoded bounded predicates. It does not prove model correctness, universal software safety, external-library correctness, real-world truth beyond verifier scope, legal clearance, or deterministic replay of future model tokens.

## Residual risks at Phase 0

Implementation has not yet established any runtime control. Until a phase gate is source-bound and accepted, its controls are design requirements with status `UNKNOWN`, not operational guarantees. External security review and formal proof audit are later desktop hardening gates and cannot be self-certified.
