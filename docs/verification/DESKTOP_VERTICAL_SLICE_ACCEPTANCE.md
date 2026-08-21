# NEMESIS Desktop Vertical-Slice Acceptance

## Objective

From the actual local NEMESIS Desktop app, a user selects a disposable local repository, creates and reviews a structured mission contract, authorizes its exact digest, runs one untrusted worker in an isolated lane, survives Core restart, accepts deterministic evidence bound to the final source, lets Kernel alone commit completion, receives a locally signed receipt, verifies it independently, changes one byte, and observes rejection.

## Test fixture

The gate creates all mutable data under a bounded temporary directory and a temporary NEMESIS home. The fixture repository has one committed text file and one deterministic test command. It has no remote. The worker starts with a clean fixed environment, no network capability, no secret capability, and write authority only inside its Git worktree.

The archived architect contract must first pass `python3 scripts/verify_contract.py`. The test records tool versions and the source revision it exercises.

## Required flow

1. Launch NEMESIS Core on an owner-scoped Unix-domain socket with no non-loopback listener.
2. Launch NEMESIS Desktop and complete local onboarding without a terminal-only prerequisite.
3. Select the fixture repository and compose a bounded mission to modify the fixture value and run its deterministic test.
4. Display the compiled `nemesis.mission/v1` contract, including goal, workspace, invariants, authority, budgets, routing, and mandatory completion claims.
5. Authorize the exact normalized contract digest. A changed contract requires a new authorization.
6. Core commits authorization before creating an execution lane.
7. Runtime creates a separate Git worktree and proves the requested macOS sandbox profile is established. No weaker fallback is allowed.
8. The untrusted deterministic worker receives a bounded context capsule and may only propose the required file mutation, deterministic test, claim, and completion.
9. Runtime/Core reject attempts by that worker to write authoritative events, provide sequence/timestamp, read secrets, use network, touch another lane, reuse approval, or emit authoritative `COMPLETE`.
10. Core normalizes the proposed file action, Kernel authorizes it against the exact capability, Runtime performs it inside the lane, and Core durably records the result.
11. Terminate Core at the declared restart point after an acknowledged committed event, then relaunch it against the same NEMESIS home.
12. Recovery returns exactly the prior mission identity, committed sequence, event hash, state, lane, capability consumption, and open obligations. Desktop reconnects and renders recovered state.
13. Run the deterministic test through the broker. Evidence records process-owned exit status, output digests, manifest identity, and current final source digest.
14. A worker proposes completion. Kernel enters `VERIFYING`, evaluates every mandatory claim, and alone commits `COMPLETE` if all accepted evidence is fresh and no blocker remains.
15. Core constructs the canonical receipt payload; the local Keychain-backed signer returns a COSE_Sign1 Ed25519 envelope.
16. The standalone `nemesis-verify` binary accepts the receipt without Desktop, Core, or any model. Its output names the final source digest, event-chain head, completion claims, evidence identities, signer identity, and bounded result.
17. Copy the receipt and flip one byte in each protected region across the mutation corpus. Every mutated receipt rejects with a typed reason and nonzero exit status.
18. Change the mission-lane source after verification. Prior test evidence becomes stale and cannot satisfy a new completion evaluation.

## Mandatory assertions

- The canonical repository and default branch are unchanged by the worker lane.
- No worker writes Core's ledger, checkpoint, database, identity, policy, or receipt directories.
- Every authoritative event sequence increments exactly once and links the prior hash.
- A verifier failure is preserved as failure.
- Evidence source digest equals the source digest in the accepted completion receipt.
- The recovered state is derived from durable records, not the Desktop cache or worker transcript.
- Desktop status distinguishes `VERIFIED`, `BELIEVED`, and `UNKNOWN` using text and non-color cues.
- Receipt mutation rejection is produced by the independent verifier rather than a source-text assertion.
- No push, publish, remote registration, public signing, notarization, Login Item, Full Disk Access, mobile, web, Windows, or Linux action occurs.

## Gate command and verdict

The final gate is:

```sh
./scripts/run_vertical_slice.sh
```

It exits zero only after printing:

```text
PASS_NEMESIS_DESKTOP_VERTICAL_SLICE
```

The success receipt includes exact sub-gate commands and output digests. Any missing prerequisite, sandbox failure, signer failure, unavailable tool, stale evidence, corrupted state, or unresolved blocker produces a nonzero exit and an evidenced typed result. It does not print the pass sentinel.

## Claims not established by this slice

This gate does not establish whole-product formal verification, perfect sandboxing, correctness of third-party dependencies, cross-platform behavior, public distribution readiness, legal clearance, identical replay of nondeterministic model tokens, or completion of deferred mobile/web/public-release scope.
