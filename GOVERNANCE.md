# NEMESIS Local Build Governance

## Authority

The architect-authorized mission contract archived under `docs/mission/` is the governing requirements source. Its desktop amendment supersedes conflicting inherited identity, platform, release, and sequence statements. Implementation notes, plans, worker output, UI text, and receipts may clarify execution but may not override it.

## Roles

- **Architect:** approves product identity, mission scope, trust-surface changes, public-release actions, and any weakening of an established control.
- **Kernel:** evaluates bounded authority and completion rules. It is software, not a human governance role, and cannot broaden its own policy.
- **Builder:** implements against the contract and cannot accept its own elevated evidence.
- **Reviewer/verifier:** independently evaluates the bounded property assigned to it. Its authority is limited by verifier registry and consequence class.
- **Recorder:** renders authoritative state and evidence without inventing status.
- **Human operator:** performs consequential external communication and any future public action after explicit authorization.

## Decision records

Architecture or trust decisions use numbered RFCs. An accepted RFC records scope, invariants, alternatives, security effects, migration, tests, proof impact, and residual risk. Code changes do not silently redefine an RFC.

The following require an RFC and architect acceptance:

- adding a package to the trusted computing base or exceeding its configured source budget;
- changing mission states or terminal-state behavior;
- widening capability, approval, budget, sandbox, secret, or network semantics;
- changing canonical event, evidence, context, or receipt formats;
- changing evidence acceptance or completion predicates;
- replacing or weakening a deterministic or proof gate;
- enabling remote/public transport, public release, or permanent ecosystem registration.

## Checkpoints and receipts

Development occurs on a local isolated branch with visible non-final checkpoint commits. No push or publish occurs. Each phase exit records:

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

This local repository uses Apache-2.0 code licensing and a DCO-style contribution intent for future work. No public contribution process, issue tracker, maintainer roster, release channel, or repository policy is active in this mission. Future public governance is `DEFERRED` and requires fresh architect authorization.
