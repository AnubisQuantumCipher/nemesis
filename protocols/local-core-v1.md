# NEMESIS Local Core Protocol v1

Core listens only on `<NEMESIS_HOME>/core.sock`, a Unix-domain stream socket changed to owner read/write permissions after bind. Startup removes a stale socket path with `unlink`; it never binds TCP or a non-loopback interface.

Requests and responses are one LF-terminated JSON object, each bounded to 65,536 bytes. The Ada parser accepts a deliberately small flat JSON subset: unique ASCII keys, string/natural/boolean values, no escape sequences, no nested values, no duplicate or extra command fields, and schema `nemesis.local/v1`. Arbitrary JSON does not cross the SPARK boundary.

Implemented commands for the witnessed slice are:

- `ping`;
- `create` with mission, worker, contract, scope, and source digests;
- `authorize` bound to the exact contract digest;
- `create_grant` issuing the mission's single immutable parent capability
  grant while the mission is in `PLANNING`; every authority field except the
  identifier is derived from the authorized mission context, and re-issuance
  refuses (`grant_already_exists`);
- `create_approval` issuing one persisted, expiring, one-shot approval bound
  to an exact action digest while the mission is in `PLANNING`; re-issuance
  refuses (`approval_already_exists`), so a consumed approval can never be
  re-armed;
- `run`;
- `inspect`;
- `authorize_action` bound to mission, subject, resource, operation, scope,
  byte budget, and action digest;
- `action_completed` with new source and action identities;
- `accept_evidence` with claim, current source, and evidence identity;
- `propose_completion`;
- `receipt_payload` after Kernel-accepted completion.

Core owns event sequence and state. `authorize_action` no longer synthesizes
authority: it loads the persisted parent grant (TS-002), derives the
per-action child through the SPARK-proved `Derive_Child_Grant` whose
postcondition guarantees `Is_Attenuation (Parent, Child)`, loads the
persisted approval for the exact action digest, consumes it exactly once
through the SPARK-proved `Consume_Approval` (TS-001), persists the consumed
record durably before the `Action_Authorized` event commits, and only then
calls the SPARK capability authority on the attenuated child. Refusals from
the persisted-authority path carry a typed `decision` (`REFUSED_CAPABILITY`
or `REFUSED_APPROVAL`) plus a `reason` (`no_parent_grant`, `grant_corrupt`,
`parent_grant_inactive`, `not_attenuated`, `no_approval`, `approval_corrupt`,
`approval_replayed`, `approval_revoked`, `approval_expired`, mission/action
mismatch, or `authority_io_failure`). The final capability check on the
attenuated child can additionally refuse with `decision` alone (no `reason`
key): `REFUSED_CAPABILITY` for a mismatched subject/scope, or
`REFUSED_BUDGET` when `estimated_bytes` exceeds the child budget — reachable
only by a direct socket client, since the desktop compiler bounds
`replacement_bytes` to the same budget. Consumption precedes that capability
check, so a request that fails a later budget or scope check has already
burned its one-shot approval — the failure direction is always closed.
Worker-supplied
event sequence, timestamp, state, capability, evidence verdict, or
`COMPLETE` fields are not protocol inputs. Non-transition action/evidence
events use the proved `Commit_Event` kernel operation, which preserves state
and increments sequence exactly once.

## Recovery

Each mission stores:

- a content-addressed contract object and reference;
- fixed canonical hash-chained ledger;
- atomically renamed checkpoint binding state, sequence, chain head, and
  current source;
- content-addressed evidence objects and references;
- one fixed-width parent-grant record (`parent.grant`) and one fixed-width
  approval record per approved action digest (`approval-<digest>.apr`), both
  written tmp-then-rename with fail-closed `F_FULLFSYNC` on the record file;
  the containing-directory flush is best-effort and is ordered before the
  subsequent ledger commit's own full-device flush. A crash between approval
  consumption and the authorization event leaves the approval consumed
  (fail closed), never replayable.

Load verifies object hashes, contract format, first ledger payload binding, complete ledger chain, checkpoint state/sequence/head, and evidence-object integrity before calling the proved recovery constructor. A checkpoint AHEAD of the authoritative ledger fails closed as corrupt; a missing or lagging checkpoint is rebuilt from the hash-chain-validated ledger tail (the SQL-001 recovery), because the append-only ledger — not the derived checkpoint — is the sole source of truth. Core never guesses past a broken ledger chain.

The witnessed test terminates and restarts Core after an acknowledged action authorization, verifies exact sequence/state recovery, proves stale-source evidence cannot complete, completes only after fresh evidence, restarts again, and checks terminal state persistence.

## Boundaries

`python3 -m unittest tests.integration.test_daemon_api -v` exercises the local API and recovery behavior. `scripts/run_vertical_slice.sh --output <dir>` adds isolated worker execution, deterministic final-source verification, Keychain signing, standalone receipt verification, and one-byte tamper rejection.

This protocol is local alpha scope. Remote transport, device enrollment, mobile approval, web hosting, and public service operation are `DEFERRED`.
