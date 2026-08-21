# NEMESIS Agent Worker Protocol v1

## Transport

Workers communicate over bounded JSON-RPC-style records on stdio. One UTF-8 JSON object occupies one line and is at most 65,536 bytes. Core assigns transport framing, time observations, event sequence, authorization, and cancellation state. Unknown protocol versions, methods, top-level fields, parameters, identifiers, and oversized records refuse.

Every worker record contains exactly:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "protocol": "nemesis.worker/v1",
  "mission_id": "mis_0000000000000000000000",
  "worker_id": "wrk_0000000000000000000000",
  "method": "heartbeat",
  "params": {}
}
```

The machine-readable schema is `protocols/schemas/nemesis.worker.v1.schema.json`. The Rust parser in `runtime/crates/nemesis-protocol` additionally enforces semantic relative paths, bounded purposes, lowercase SHA-256 digests, and identifier shapes.

## Worker methods

Allowed methods are `heartbeat`, `propose_action`, `emit_claim`, `return_artifact`, `report_blocker`, and `propose_completion`.

There is no worker method for committing mission state, issuing capabilities, accepting evidence, assigning event sequence/timestamp, changing policy/budget, or marking completion. Any unknown method, including `complete`, is a typed protocol refusal.

A valid action proposal is not authorization. Core validates and normalizes it, Kernel evaluates the exact capability/policy/budget/approval state, and Runtime executes only an `AUTHORIZED` action. The worker's stdout and exit status are observations, not authoritative evidence until an authorized verifier accepts a source-bound record.

## Process and sandbox boundary

The first deterministic worker is a separate Rust process. It emits only protocol-valid proposals and never edits mission state or workspace files directly. Runtime creates a distinct Git worktree, clears the inherited environment, supplies only `PATH=/usr/bin:/bin` and `LANG=C`, and launches the exact worker executable through the requested macOS sandbox profile.

`Workspace Safe` is default-deny, denies network, permits execution of only the exact worker binary, permits reads of required system runtime paths and the lane, and permits writes only to the lane plus `/dev/null`. The profile grants read-data on the root directory itself because macOS 26 aborts a sandboxed platform binary when that directory operation is denied; it does not grant root subpaths. The behavioral gate first proves an unsandboxed loopback listener is reachable, then verifies the sandboxed process cannot reach a second listener. It also verifies outside-file read and sibling write denial.

Failure to locate `sandbox-exec`, canonicalize the lane/executable, parse the profile, start the process, or establish the requested profile is failure. Runtime has no weaker fallback.

## Gate

```sh
./scripts/test_worker_boundary.sh
```

The gate must print `PASS_PHASE5_WORKER_BOUNDARY`. This establishes the tested protocol and local sandbox behaviors on the recorded host/toolchain. It does not establish perfect sandboxing or absence of operating-system vulnerabilities.
