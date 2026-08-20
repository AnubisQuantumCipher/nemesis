# NEMESIS Desktop Design

**Authority:** Architect mission contract archived at `docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md`, SHA-256 `ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779`.

**Scope:** Local-first macOS desktop product only. The contract already supplies the architect-approved product and visual design; this document records implementation boundaries and resolves desktop-specific seams without changing that contract.

## Product contract

NEMESIS is **The Provable Agent Operating System**. Its tagline is **Autonomy under control.** Its doctrine is **Intelligence proposes. NEMESIS governs. Evidence decides.** Worker output is untrusted. Only deterministic, source-bound, authorized evidence may satisfy mandatory completion predicates. The UI distinguishes `VERIFIED`, `BELIEVED`, and `UNKNOWN`, preserves residuals, and never turns worker prose into authoritative state.

## Architecture

### Trusted authority

A small Ada/SPARK kernel owns bounded mission records, legal transitions, event sequence rules, capability checks, budgets, approval consumption, evidence acceptance, completion predicates, and recovery invariants. Public kernel inputs are bounded Ada records and enumerations; the kernel never interprets arbitrary JSON. SPARK proof claims are package- and property-specific. Ordinary Ada, Rust, JavaScript, libraries, the operating system, and cryptographic implementations remain documented assumptions rather than being mislabeled formally verified.

### NEMESIS Core daemon

An Ada 2022 daemon owns the durable mission lifecycle. Its untrusted JSON boundary validates version, message size, identifiers, and field shapes before constructing kernel records. It exposes only a local Unix-domain socket, refuses non-loopback/network operation in this mission, appends canonical hash-chained events, maintains derived read state, and recovers from the last durable event/checkpoint after restart. A local process supervisor starts the daemon when the desktop app or CLI needs it; desktop closure does not define mission completion.

### Runtime and isolated lanes

Workers use versioned JSON-RPC over stdio. The first worker is deterministic and intentionally untrusted. It may observe, propose an action, return an artifact, emit a claim, report a blocker, or propose completion. It cannot issue capabilities, commit mission events, accept evidence, or mark completion. Every implementation lane is a Git worktree. macOS sandbox establishment is checked before execution and has no weaker fallback. The execution broker performs normalized actions only after kernel authorization and records exact executable, argument, environment, output, artifact, capability, source, and previous-event digests.

### Receipts and independent verification

The daemon constructs a canonical mission-receipt payload from authoritative state. A narrow local signer uses an Ed25519 key whose seed is stored in the macOS Keychain and emits a canonical CBOR/COSE_Sign1 envelope. The private key is never placed in worker or inherited mission environments. The standalone `nemesis-verify` binary receives only the receipt and public verification material; it starts no daemon and calls no model. It verifies canonical encoding, protected headers, signature, hash chain, final-source binding, evidence freshness, required completion predicates, and absence of unresolved blockers. One-byte mutations of payload, action, evidence, source binding, or signature fail.

### CLI recovery surface

The secondary `nemesis` CLI supports `mission create`, `mission run`, `mission inspect`, `agents`, `evidence`, `replay`, and `verify`. It uses the same daemon protocol and kernel-owned state as the desktop UI. It exists for recovery, automation, and independent checks; it is not the product front door.

### Desktop application

Tauri 2 hosts a React and TypeScript UI. Rust commands are a narrow local IPC client and process supervisor, not an authority engine. The desktop includes onboarding, Home, Missions, Workspaces, Agents, Changes, Tests, Evidence, Knowledge, Skills, Automations, Integrations, Security, Replay, and Settings navigation. The Phase 6 alpha must provide an app-only path through workspace selection, mission composition, compiled-contract review, exact authorization, execution, restart recovery, completion court, signed receipt, independent verification, and tamper demonstration.

The visual system uses near-black and graphite surfaces, hard geometric typography, restrained red state cues, dense telemetry, sharp short motion, and minimal decoration. Red denotes attention/refusal/failure rather than generic branding. Evidence status also uses text and shape, never color alone. Keyboard navigation, reduced motion, focus visibility, readable contrast, and screen-reader labels are mandatory.

## Data flow

1. The user composes a goal and selects a local workspace in NEMESIS Desktop.
2. Core compiles it into a versioned mission contract with explicit invariants, authority, budgets, and required evidence.
3. The user authorizes the exact contract digest.
4. Kernel commits the authorization transition and issues attenuated, expiring capabilities.
5. Runtime creates an isolated Git worktree and a bounded context capsule.
6. An untrusted worker proposes normalized actions over JSON-RPC.
7. Core validates the message; Kernel authorizes or refuses; Runtime executes only authorized actions in the established sandbox.
8. Deterministic verifiers bind results to the current source revision and submit typed evidence.
9. A worker may propose completion. Kernel evaluates every mandatory claim and blocker, then alone may transition `VERIFYING` to `COMPLETE`.
10. Core emits a signed local receipt. The standalone verifier checks it without Core, Desktop, or a model.

## Error and recovery rules

- Invalid, oversized, unknown-version, malformed, out-of-sequence, or unauthorized input is refused with a typed error and recorded when safe.
- A verifier crash, timeout, malformed response, or nonzero status never becomes pass.
- Failure to establish a requested sandbox is `REQUIRES_STRONGER_SANDBOX` or `BLOCKED_WITH_EVIDENCE`; it never falls back.
- Event acknowledgement occurs only after the append and durability boundary succeeds.
- On restart, a complete valid prefix recovers; truncated or corrupt suffixes are refused and preserved for diagnosis. Ambiguous state does not run.
- Source changes invalidate dependent evidence before completion evaluation.
- Missing external providers remain typed unavailable states. Provider fallback cannot expand authority.
- Required human decisions stay pending; an LLM assertion cannot replace them.

## Verification strategy

TDD protects each observable contract. Ada unit/property tests enumerate transition pairs, capability attenuation, budgets, approvals, evidence freshness, and completion. Recovery tests interrupt every declared durable write boundary and compare recovered authoritative state. Protocol tests mutate messages and enforce limits. Sandbox tests use malicious fixtures in a bounded Apple Virtualization guest when crash/fuzz/exploit behavior is involved. Desktop verification launches the actual application, drives the complete vertical slice, closes/reopens the UI, restarts Core, verifies the receipt independently, mutates one byte, and observes rejection.

Each phase writes a machine-readable receipt containing the contract digest, source revision, command, tool identity, exit status, stdout/stderr digests, produced artifacts, and verdict. A receipt says `VERIFIED` only for properties its cited gate establishes; other claims are `BELIEVED` or `UNKNOWN` with residuals.

## Deferred surfaces

Mobile, web, remote mobile approvals, public cloud, public release infrastructure, public signing/notarization, App Store/TestFlight, Windows/Linux packaging, and public-name collision/legal clearance are explicitly `DEFERRED`. No file or UI state may present them as built. Local implementation must retain versioned protocol and replaceable UI seams without implementing those surfaces.
