# NEMESIS Context and Knowledge v1

Knowledge entries are provenance-bound records with subject, predicate, value, scope, source digest, observation sequence, validity interval, status, supersession, contradiction links, and an untrusted-external label.

Statuses are `Proposed`, `Observed`, `Verified`, `Disputed`, `Superseded`, `Revoked`, and `Expired`. Only current `Observed` or `Verified` entries can enter an active capsule. Superseded, revoked, disputed, or expired facts remain inspectable in the knowledge base but are not presented as active truth.

A context request names the mission/source, worker role, every assigned obligation, immutable invariants, decisions, failed approaches, blockers, capability summary, remaining budget, capsule sequence, and byte ceiling. The compiler serializes mandatory state first. If mandatory state exceeds the ceiling, it refuses with `BudgetTooSmall`; it never silently omits an obligation or invariant. Current knowledge is then included deterministically by entry identifier while space remains.

The exact canonical JSON capsule is SHA-256 bound and Ed25519 signed. Verification re-encodes the structured capsule, compares exact bytes/digest/public key, and verifies the signature. A one-byte encoded mutation rejects.

Repository indexing accepts only unique relative normal-component paths, resolves each path under the canonical root, rejects traversal/symlink escape/non-files, enforces a total byte ceiling, and records path, byte length, and SHA-256.

```sh
./scripts/test_context.sh
```

The gate establishes context replacement obligation retention, contradiction visibility, stale/revoked exclusion, byte-budget refusal, signature tamper rejection, and repository path binding. It does not expose hidden model reasoning or promote retrieval candidates into authority.
