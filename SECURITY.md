# NEMESIS Desktop Security Policy

## Current status

NEMESIS Desktop is a local, unpublished build. It has no public repository, release, package, installer, security intake address, or bounty program. Do not upload vulnerabilities, source, receipts, or logs to a public service. The architect must authorize any future disclosure channel and public policy.

## Non-negotiable controls

- Treat model, worker, repository, plugin, MCP, provider, tool, renderer, and imported evidence content as untrusted.
- Route every authoritative mutation through bounded Core validation and Kernel authorization.
- Never let a worker issue a capability, accept evidence, expand a budget, rewrite policy, or mark a mission complete.
- Bind one-shot approvals to exact normalized action digest, mission, expiry, and consumed state.
- Default to refusal on parser, policy, capability, budget, verifier, signer, sandbox, or recovery failure.
- Verify evidence against the current source revision. Later source changes invalidate dependent evidence.
- Keep long-lived secrets and signing material out of worker environments and artifacts.
- Preserve failed gates and hostile fixtures. Do not weaken tests, verifiers, proof obligations, or pass definitions to obtain green output.
- Run crash-capable, fuzz, exploit, or offensive stress work inside an approved Apple Virtualization guest rather than on the host.
- Do not push, publish, register, sign publicly, notarize, alter Login Items, request Full Disk Access, or access unrelated repositories during this mission.

## Local handling of a suspected vulnerability

1. Stop the affected mission or revoke the narrow capability if continuing could cause harm.
2. Preserve the exact source revision, event sequence, action digest, logs, receipt, and minimal reproducer under the local project.
3. Mark the claim `UNKNOWN` until reproduced. If reproduced, record the exact bounded effect and affected boundary; do not inflate severity.
4. Add a failing hostile regression test before changing production code.
5. Fix the source boundary, not the symptom. Keep policy and verifier semantics unchanged unless the architect approves a trust-surface RFC.
6. Re-run the narrow reproducer and every affected gate against the final revision.
7. Record residuals, untested environments, and external assumptions.
8. If disclosure to an external party becomes necessary, prepare a draft only. The human selects and uses the authorized intake channel.

## Supported local security claims

A gate may claim only the property it directly exercises. A passing SPARK run applies to named packages and obligations. A valid signature authenticates the signed bytes. A sandbox test demonstrates the exercised profile on the recorded host/tool version. None of these establishes universal correctness, perfect sandboxing, supply-chain safety, legal clearance, or correctness of external libraries.

## Dependency and change review

Dependencies are pinned by lockfiles where the ecosystem supports it. Kernel, policy, protocol schema, receipt, signer, secret, sandbox, and completion changes receive hostile regression coverage and source-bound review. Generated receipts are derived from current bytes; committed evidence is never edited by hand to match a desired outcome.

## Public-release gate

Before any future public release, the architect must separately authorize collision reconnaissance, public security intake, licensing/NOTICE review, dependency and provenance review, signing/notarization, installer testing, supported-platform claims, and publication. This local policy does not satisfy or bypass that gate.
