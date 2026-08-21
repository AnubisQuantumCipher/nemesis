# NEMESIS Evidence Policy

## Status words

- `VERIFIED`: an authorized named verifier directly established the stated bounded property using recorded inputs and current source identity.
- `BELIEVED`: observations support the claim, but required reproduction, independence, source binding, or verifier authority is incomplete.
- `UNKNOWN`: evidence is absent, stale, conflicting, malformed, out of scope, or rejected.

Every status includes a scope statement. `VERIFIED` never means universally true. Any residual claim outside the verifier's scope remains `BELIEVED` or `UNKNOWN` and stays visible.

## Accepted evidence record

An evidence record contains:

- schema and evidence version;
- evidence identity and mission identity;
- claim identity and consequence class;
- verifier identity, version, authorization class, and executable digest;
- exact source revision and dependent object digests;
- exact test/proof/manifest identity;
- normalized command and arguments digest when applicable;
- bounded environment identity;
- start and finish observations assigned by Core, not a worker;
- process-owned exit status;
- stdout, stderr, and artifact digests;
- freshness and conflict state;
- prior event hash and committed sequence;
- verdict, limitations, and residual claims.

## Acceptance rules

Kernel accepts evidence only when:

1. schema, version, lengths, enumerations, and identifiers are valid;
2. the verifier is registered for the evidence type and consequence class;
3. the evidence scope covers the claim without extrapolation;
4. the source and all declared dependencies equal the current authoritative identities;
5. the action was authorized and its capability was active for the exact operation;
6. the verifier result is explicit pass rather than absence of failure;
7. no timeout, crash, malformed output, unsupported result, or conflict remains;
8. independence policy is satisfied;
9. the record is durably committed in sequence.

A worker's sentence, generated summary, screenshot without source binding, cached earlier log, or builder self-review is not sufficient evidence for an elevated completion claim.

## Freshness and invalidation

A source or dependency mutation changes its content identity. Evidence listing that identity becomes `STALE` before completion evaluation. It cannot be made current by editing metadata. The verifier must run again or a trusted dependency analysis must establish and record irrelevance. The initial desktop slice uses conservative invalidation: any mission-lane source change invalidates its test and build evidence.

## Consequence policy

| Consequence | Minimum evidence |
|---|---|
| Informational | Direct observation with provenance |
| Advisory | Bound evidence plus uncertainty and residuals |
| Decision-boundary | Deterministic check and required independent review |
| Safety-critical | Strong deterministic verification, configured human authorization, elevated independent review, and explicit assumption record |

Formal verification is an evidence strength, not a brand label. JACKAL and Anubis receipts are accepted only for typed lanes, exact versions, and qualified properties. Refusal and bounded results remain distinct and are never flattened into pass.

## Completion policy

Workers may emit `PROPOSE_COMPLETION`. They cannot emit authoritative `COMPLETE`. Kernel evaluates every mandatory claim for existence, acceptance, freshness, final-source binding, qualified verifier, required independence, and absence of unresolved blockers. Only Kernel may commit `VERIFYING -> COMPLETE`.

## Receipt policy

A signed receipt binds authoritative events, completion claims, accepted evidence, final source, verifier identities, residuals, and previous hashes. Signature verification establishes authenticity and integrity of those bytes under the named public key. Independent verification also checks canonical encoding and completion predicates; it does not call a model or start Core.

One-byte changes to the protected header, payload, action digest, evidence digest, source binding, event chain, or signature must reject. Unknown algorithms, keys, schemas, or critical headers reject.
