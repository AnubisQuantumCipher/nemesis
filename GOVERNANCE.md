# NEMESIS Project Governance

## Authority

The architect-authorized contracts archived under `docs/mission/` are the governing requirements sources. The desktop amendment supersedes conflicting inherited identity and platform statements; the 2026-08-20 GitHub release contract supersedes the former prohibition on creating and publishing `AnubisQuantumCipher/nemesis`. Implementation notes, plans, worker output, UI text, and receipts may clarify execution but may not override those contracts.

## Roles

- **Architect:** approves product identity, mission scope, trust-surface changes, public-release actions, and any weakening of an established control.
- **Kernel:** evaluates bounded authority and completion rules. It is software, not a human governance role, and cannot broaden its own policy.
- **Builder:** implements against the contract and cannot accept its own elevated evidence.
- **Reviewer/verifier:** independently evaluates the bounded property assigned to it. Its authority is limited by verifier registry and consequence class.
- **Recorder:** renders authoritative state and evidence without inventing status.
- **Human operator:** ratifies consequential communication to external parties. Explicit actions on the operator's own repository, tags, releases, and metadata may be automated only when the current architect contract authorizes them.

## Decision records

Architecture or trust decisions use numbered RFCs. An accepted RFC records scope, invariants, alternatives, security effects, migration, tests, proof impact, and residual risk. Code changes do not silently redefine an RFC.

The following require an RFC and architect acceptance:

- adding a package to the trusted computing base or exceeding its configured source budget;
- changing mission states or terminal-state behavior;
- widening capability, approval, budget, sandbox, secret, or network semantics;
- changing canonical event, evidence, context, or receipt formats;
- changing evidence acceptance or completion predicates;
- replacing or weakening a deterministic or proof gate;
- enabling remote/public product transport, broadening distribution beyond the authorized macOS alpha, or registering a permanent package/domain identity.

## Checkpoints and receipts

Development uses scoped branches or isolated worktrees with visible evidence-bearing commits. Public integration uses pull requests and required hosted checks where the repository plan supports them. No contributor may force-push published history, bypass a red/pending check, or publish an artifact without exact authorization. Each phase exit records:

- architect contract digest;
- source revision or dirty-source digest when a pre-commit gate is intentionally run;
- exact command and tool identity;
- exit status and output digests;
- produced artifacts;
- properties established and properties not established;
- open residuals, deferred work, and next dependency.

A phase receipt is append-only after commit. A correction creates a new receipt that supersedes the old one; it does not rewrite historical evidence.

## Evidence review

Builders do not accept their own elevated claims. Safety-critical changes require deterministic verification, explicit human authorization where configured, and independent review. Model review may support informational or advisory claims but is not sole evidence for a decision-boundary or safety-critical completion predicate.

## Pass-definition changes

A change to what `PASS` means is a finding, not ordinary cleanup. It must preserve the prior fixture, explain the discrepancy, show old and proposed semantics, identify downstream claims, and obtain architect acceptance before any weakening. A stricter interpretation may land with migration evidence when it does not invalidate architect intent.

## Contribution and publication posture

NEMESIS is published under Apache-2.0 with DCO sign-off intent. Contributions use issues and pull requests under [CONTRIBUTING.md](CONTRIBUTING.md); suspected vulnerabilities use the private channel in [SECURITY.md](SECURITY.md).

The current maintainer is `@AnubisQuantumCipher`. One-person maintenance means the repository cannot honestly require two human reviewers today. Critical authority and security changes still require deterministic hostile coverage, explicit architect acceptance, and an independent review artifact before merge. Review requirements may tighten when additional qualified maintainers exist.

The active public channel is the source-visible macOS `v0.1.x` alpha on GitHub. Developer ID signing, notarization, App Store distribution, package registries, domains, mobile/web/remote-public products, and broader platform releases remain outside this authorization.
