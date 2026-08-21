# NEMESIS Desktop Trust Boundaries

## Authority flow

```text
Human reviews exact contract/action digest
              |
              v
NEMESIS Desktop / CLI (untrusted clients)
              |
       validated local protocol
              v
NEMESIS Core (Ada durable owner)
              |
       bounded typed records
              v
NEMESIS Kernel (small SPARK authority)
              |
     authorized normalized action
              v
NEMESIS Runtime and isolated worker lane
              |
       artifacts and observations
              v
Authorized deterministic verifiers
              |
       typed source-bound evidence
              v
Kernel completion court -> signed receipt
```

Authority flows downward only after authorization. Evidence flows upward only after independent validation. Workers never write authoritative mission state.

## Boundary inventory

| Boundary | Trusted input accepted | Rejected or untrusted input | Enforcement owner |
|---|---|---|---|
| Human to Desktop | Exact authorization over displayed normalized digest | Free-form chat as authority | Core/Kernel digest validation |
| Desktop/CLI to Core | Versioned bounded local command | Renderer state, caller timestamp/sequence, unknown fields | Ada protocol validator |
| Core to Kernel | Enumerations, bounded strings/arrays, normalized digests | Arbitrary JSON, paths, commands, model text | Ada conversion layer and SPARK preconditions |
| Kernel to Runtime | Exact authorized action and capability identifier | Ambient shell/tool access or broader resource | Runtime broker |
| Runtime to worker | Bounded context and attenuated lane capabilities | Signing key, unrelated environment, canonical repository write | Sandbox and clean environment |
| Worker to Runtime | Observe/propose/action/claim/artifact/blocker/completion proposal | Event commit, capability issue, evidence acceptance, policy rewrite, `COMPLETE` | Worker protocol schema |
| Runtime to verifier | Exact source, command, manifest, and artifact identities | Worker-supplied pass status | Verifier adapter |
| Verifier to Kernel | Registered evidence type/version/scope/source/result | Unsupported consequence or stale source | Evidence authority |
| Core to signer | Canonical kernel-approved receipt payload | Worker bytes, free-form executable request, secret-read request | Narrow signer protocol |
| Receipt to standalone verifier | Canonical CBOR/COSE envelope and public key identity | Noncanonical, unknown algorithm/version, stale or incomplete predicates | `nemesis-verify` |
| Plugin/MCP to Core | Fixed schema and explicit capability manifest | Runtime schema mutation, inherited secrets, arbitrary host operations | Plugin/MCP gateway |

## Trusted computing base

The bounded TCB is defined in RFC-0001. TCB membership does not mean all code is formally verified. Proof receipts name the packages, tool version, mode, obligations, assumptions, and unresolved checks. Every new TCB package requires an RFC.

## Local transport boundary

Core listens only on an owner-scoped Unix-domain socket under the configured NEMESIS home. It does not bind an unauthenticated non-loopback interface. Socket directory and file permissions must deny other local users. Message size, version, and request identity are validated before dispatch. Windows named pipes and remote authenticated transports are architectural seams only and are not implemented in this desktop mission.

## Filesystem boundary

A capability names one canonical workspace root and allowed operations. Runtime resolves paths, rejects traversal and special files, checks symlink resolution and mount policy, and uses race-resistant operations where the host supports them. Workers operate in isolated Git worktrees; the canonical repository and sibling lanes are not writable worker resources.

## Secrets and signing boundary

Core starts workers with a fixed environment allowlist. Long-lived credentials and receipt signing material stay in the macOS Keychain or a future secret manager. A worker receives at most a short-lived scoped proxy result or refusal. The local signer accepts canonical payload bytes and returns a signed envelope; it exposes no private-key operation beyond that receipt protocol.

## Frontend boundary

Desktop is replaceable and untrusted. Tauri exposes named commands only; it does not expose arbitrary shell, filesystem, Keychain, network, or native-object access to the renderer. Approval views include the exact normalized action and digest. Status uses text and shape in addition to color. Core remains authoritative if the UI closes, crashes, reconnects, or displays stale data.

## Public source and release boundary

The 2026-08-20 release contract authorizes the public GitHub source repository and a bounded `v0.1.0` macOS arm64 alpha. Repository publication does not grant the NEMESIS product, renderer, workers, plugins, or MCP servers any GitHub credential or push/merge/release authority. Mission-owned GitHub operations remain external operator actions governed by their exact contract.

The distributed app embeds only the local witnessed-mission backend, source contracts, replay fixture, and license. Release executables pass a private-path/credential-signature leak gate, Mach-O dependency/RPATH inspection, and an ad-hoc code-signature verification. The release is not Developer ID signed or notarized; Gatekeeper and App Store trust are not implied.

The following remain `DEFERRED`:

- iPhone application.
- iPad application and Apple Pencil flows.
- Hosted or self-hosted web application/dashboard.
- Public cloud, public webhook, and public messaging service.
- Remote mobile approvals, device enrollment, push notifications, and lost-device revocation.
- Windows application/installer.
- Linux application/installer.
- Developer ID signing and notarization.
- TestFlight and App Store distribution.
- Package-registry publication, domain acquisition, and permanent ecosystem registration.
- Legal-clearance review; bounded name-collision reconnaissance is recorded, but it is not legal advice or clearance.

These deferrals do not count as bounded macOS alpha completion predicates. They also must not appear in receipts or UI as implemented capabilities.
