# NEMESIS Security Policy

## Supported versions

| Version | Security updates |
|---|---|
| `0.1.x` | Supported while it is the current alpha line |
| Pre-release local checkpoints | Evidence only; no update commitment |

This is a one-maintainer alpha. There is no bounty program and no guaranteed response SLA.

## Report a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/AnubisQuantumCipher/nemesis/security/advisories/new). Do **not** open a public issue for a suspected vulnerability.

Include:

- affected version and exact commit;
- affected trust boundary and bounded impact;
- minimal reproduction steps or hostile fixture;
- relevant mission, event, action, evidence, receipt, or log hashes;
- whether exploitation requires local access, a granted capability, user interaction, or a specific sandbox/toolchain;
- suggested coordination constraints, if any.

Do not include live credentials, private signing material, unrelated user data, or logs that have not been reviewed for secrets. If the private-reporting form is unavailable, open a non-sensitive issue stating only that the private channel is unavailable; do not disclose the vulnerability there.

The maintainer will reproduce the report, determine affected scope, coordinate a fix and disclosure when warranted, and preserve reporter credit if requested. No fixed embargo duration is asserted; public disclosure is coordinated against the specific report and remediation state.

## Non-negotiable controls

- Treat model, worker, repository, plugin, MCP, provider, tool, renderer, and imported evidence content as untrusted.
- Route authoritative mutation through bounded Core validation and Kernel authorization.
- Never let a worker issue a capability, accept elevated evidence, expand a budget, rewrite policy, or mark a mission complete.
- Bind one-shot approvals to the exact normalized action digest, mission, expiry, and consumed state.
- Default to refusal on parser, policy, capability, budget, verifier, signer, sandbox, or recovery failure.
- Verify evidence against the current source revision. Later dependent source changes invalidate it.
- Keep long-lived secrets and signing material out of worker environments and release artifacts.
- Preserve failed gates and hostile fixtures. Do not weaken a test, verifier, proof obligation, or pass definition to obtain green output.
- Run crash-capable, fuzz, exploit, and offensive stress work inside the approved Apple Virtualization.framework guest, not primarily on the host.

## Maintainer handling

1. Stop the affected mission or revoke the narrow capability if continuing could cause harm.
2. Preserve the exact source revision, event sequence, action digest, logs, receipt, and minimal reproducer.
3. Keep the claim `UNKNOWN` until reproduced. Once reproduced, record the exact bounded effect without severity inflation.
4. Add a failing hostile regression before changing production code.
5. Fix the source boundary, not the symptom. A trust-surface semantic change requires an accepted RFC/architect decision.
6. Re-run the narrow reproducer and every affected gate against final source.
7. Record residuals, untested environments, and external assumptions.
8. Publish an advisory only when the affected version, remediation, and disclosure state are grounded.

## Claim boundary

A gate claims only the property it directly exercises. A passing SPARK run applies to the named packages and obligations. A valid signature authenticates the signed bytes. A sandbox test demonstrates the exercised profile on the recorded host/tool version.

None of those establishes universal correctness, perfect sandboxing, supply-chain safety, legal clearance, correctness of external libraries, or safety of an arbitrary mission. Independent external security review remains `[NEEDS-HUMAN]`.

## Dependency and release review

Dependencies are pinned by lockfiles where supported. Kernel, policy, protocol, receipt, signer, secret, sandbox, and completion changes require hostile regression coverage and source-bound review. Generated manifests and receipts are derived from bytes rather than edited to match a desired result.

Every shipped `0.x` macOS artifact is ad-hoc signed and not notarized. Per the supported source-install distribution contract (`docs/release/SOURCE_INSTALL.md`), Developer ID signing, notarization, Gatekeeper approval, App Store distribution, mobile/web/remote-public products, package registration, and legal clearance are not claimed; trust in release bytes is established by hash verification against the release manifest.
