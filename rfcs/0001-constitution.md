# RFC-0001: NEMESIS Desktop Constitution

**Status:** Accepted for the local desktop build  
**Authority:** Architect mission contract dated 2026-08-20  
**Scope:** macOS-first, local, desktop-only

## Identity

- **Name:** NEMESIS
- **Descriptor:** The Provable Agent Operating System
- **Tagline:** Autonomy under control.
- **Doctrine:** Intelligence proposes. NEMESIS governs. Evidence decides.

The product components are NEMESIS Desktop, NEMESIS Core, NEMESIS Kernel, NEMESIS Runtime, NEMESIS Grid, NEMESIS Forge, NEMESIS Vault, NEMESIS Evidence, NEMESIS Replay, NEMESIS Protocol, and NEMESIS SDK. The command namespace is `nemesis`.

## Constitutional invariants

1. Probabilistic workers are untrusted. Repository text, model output, plugins, MCP providers, tools, and frontends do not acquire authority by being useful or persuasive.
2. Workers may propose completion; only the kernel may commit completion after every mandatory predicate has acceptable, current, final-source-bound evidence and no unresolved blocker.
3. The model never owns the machine. It petitions NEMESIS through normalized typed actions.
4. Every consequential action requires a live capability covering its exact mission, subject, operation, resource, scope, budget, and expiry.
5. Capabilities may be attenuated. They may not be silently expanded or reused across missions.
6. Human approvals bind one exact normalized action digest, mission, expiry, and consumption state. One-shot approval replay fails.
7. Authoritative state is a durable hash-chained mission ledger, not a conversation, UI read model, worker transcript, or database cache.
8. A committed event has exactly one increasing sequence number and one predecessor. Terminal missions do not silently resume.
9. Verifier failure, crash, timeout, malformed output, or missing output is never pass.
10. Evidence states its verifier, consequence class, source binding, scope, freshness, and limitations. Unsupported residual claims remain visible.
11. Source mutation invalidates dependent evidence before completion evaluation.
12. Failure to establish the requested sandbox or broker boundary refuses execution. There is no weaker implicit fallback.
13. Secret material is brokered narrowly and is not inherited by workers. Receipt signing material is never a worker capability.
14. Replay reconstructs recorded state, inputs, outputs, and deterministic decisions. It does not promise identical future tokens from nondeterministic models.
15. Deferred mobile, web, remote, cross-platform packaging, and public-release work is never represented as implemented.

## Consequence classes

| Class | Meaning | Minimum accepted assurance |
|---|---|---|
| Informational | Observation or summary with no authority change | Direct source observation with provenance |
| Advisory | Recommendation that remains subject to judgment | Evidence, source binding, and explicit uncertainty |
| Decision-boundary | Mission completion, release readiness, or comparable control decision | Deterministic verification plus required independence |
| Safety-critical | Authorization, cryptography, sandbox, secret, or kernel-boundary change | Strong deterministic verification, explicit human authorization where configured, and elevated independent review |

The class is a kernel enumeration. A worker cannot lower it or choose the evidence policy that judges its own claim.

## Evidence vocabulary

- `VERIFIED`: a named authorized verifier established the stated bounded property against the recorded source and inputs.
- `BELIEVED`: available observations support the claim, but the accepted verifier or complete reproduction is absent.
- `UNKNOWN`: evidence does not establish the claim or conflicts remain.

A signature authenticates bytes and signer identity; it does not make every real-world statement in those bytes universally true.

## Trusted computing base

The initial trusted-computing-base budget is at most 2,500 nonblank, noncomment source lines across SPARK kernel packages. This is a configured review budget, not a proof claim. Any increase requires a new accepted RFC naming the invariant that cannot remain outside the kernel.

The local alpha TCB contains:

- bounded SPARK mission, transition, capability, budget, approval, evidence, completion, sequence, and recovery-decision packages;
- validated Ada conversion from protocol values into bounded kernel records;
- the durable event commit boundary;
- canonical receipt construction and the narrow local signer/verifier boundary;
- cryptographic and operating-system primitives explicitly listed as assumptions in proof and receipt reports.

Outside the TCB: Desktop renderer, Tauri UI bridge, workers, model providers, plugins, MCP servers, schedulers, context ranking, derived read models, repository content, build scripts, and human-readable summaries.

## Formal-verification claim boundary

SPARK and GNATprove results are reported per package, property, proof mode, tool version, assumptions, and remaining checks. NEMESIS does not claim universal soundness, whole-product formal verification, correctness of external libraries, or absence of supply-chain risk.

## Local desktop scope

This constitution governs the local macOS desktop product: Desktop, local Core daemon, Kernel, Runtime, isolated worker lanes, CLI recovery surface, evidence/replay, protocols, and independent receipt verifier. Public release and permanent name registration require fresh architect authorization plus evidence-based collision reconnaissance and appropriate human/legal review. No legal clearance is claimed here.

## Change control

A change to authorization, evidence acceptance, completion predicates, proof obligations, sandbox strength, receipt validation, or the meaning of `PASS` is a trust-surface change. It requires:

1. a dedicated RFC amendment;
2. hostile regression coverage written before implementation;
3. exact old/new semantics;
4. evidence of why the old boundary is insufficient;
5. architect approval before weakening any established protection.

Ordinary implementation may make a gate stricter without redefining a previously accepted pass, provided compatibility and migration effects are recorded.
