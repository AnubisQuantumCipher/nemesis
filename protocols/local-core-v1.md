# NEMESIS Local Core Protocol v1

Core listens only on `<NEMESIS_HOME>/core.sock`, a Unix-domain stream socket changed to owner read/write permissions after bind. Startup removes a stale socket path with `unlink`; it never binds TCP or a non-loopback interface.

Requests and responses are one LF-terminated JSON object, each bounded to 65,536 bytes. The Ada parser accepts a deliberately small flat JSON subset: unique ASCII keys, string/natural/boolean values, no escape sequences, no nested values, no duplicate or extra command fields, and schema `nemesis.local/v1`. Arbitrary JSON does not cross the SPARK boundary.

Implemented commands for the witnessed slice are:

- `ping`;
- `create` with mission, worker, contract, scope, and source digests;
- `authorize` bound to the exact contract digest;
- `run`;
- `inspect`;
- `authorize_action` bound to mission, subject, resource, operation, scope, byte budget, and action digest;
- `action_completed` with new source and action identities;
- `accept_evidence` with claim, current source, and evidence identity;
- `propose_completion`;
- `receipt_payload` after Kernel-accepted completion.

Core owns event sequence and state. `authorize_action` constructs bounded Ada records and calls the SPARK capability authority. Worker-supplied event sequence, timestamp, state, capability, evidence verdict, or `COMPLETE` fields are not protocol inputs. Non-transition action/evidence events use the proved `Commit_Event` kernel operation, which preserves state and increments sequence exactly once.

## Recovery

Each mission stores:

- a content-addressed contract object and reference;
- fixed canonical hash-chained ledger;
- atomically renamed checkpoint binding state, sequence, chain head, and current source;
- content-addressed evidence objects and references.

Load verifies object hashes, contract format, first ledger payload binding, complete ledger chain, checkpoint state/sequence/head, and evidence-object integrity before calling the proved recovery constructor. Ledger/checkpoint disagreement returns corrupt state; Core does not guess or run.

The witnessed test terminates and restarts Core after an acknowledged action authorization, verifies exact sequence/state recovery, proves stale-source evidence cannot complete, completes only after fresh evidence, restarts again, and checks terminal state persistence.

## Boundaries

`python3 -m unittest tests.integration.test_daemon_api -v` exercises the local API and recovery behavior. `scripts/run_vertical_slice.sh --output <dir>` adds isolated worker execution, deterministic final-source verification, Keychain signing, standalone receipt verification, and one-byte tamper rejection.

This protocol is local alpha scope. Remote transport, device enrollment, mobile approval, web hosting, and public service operation are `DEFERRED`.
