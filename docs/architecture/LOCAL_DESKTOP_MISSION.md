# Bounded Local Desktop Mission

NEMESIS Desktop accepts `nemesis.desktop-mission/v1` contracts for one narrow operation: replace the bytes of one existing UTF-8 file inside an isolated Git worktree, then run two deterministic completion checks. The Desktop can create the contract from the Missions screen or compile an existing JSON file by absolute path.

The machine schema is [`protocols/schemas/nemesis.desktop-mission.v1.schema.json`](../../protocols/schemas/nemesis.desktop-mission.v1.schema.json). The Rust parser in `desktop/src-tauri/src/production.rs` is authoritative when the schema and implementation disagree; it rejects duplicate and unknown fields rather than normalizing them away.

## Desktop flow

1. Open **Missions**.
2. Enter a goal, absolute clean Git workspace, existing relative file path, and replacement UTF-8 text.
3. Select **Create exact contract**. NEMESIS reads the current `HEAD` and target bytes, computes their identities, and writes the draft under the private local home.
4. Inspect the normalized contract digest, normalized action digest, workspace, target, write/runtime/output budgets, and denied authority.
5. Select **Review authority**. The review shows both SHA-256 digests and the normalized contract bytes.
6. Select **Authorize and run**. The native bridge re-reads and recompiles the file; a changed byte, dirty/stale workspace, symlink, traversal, executable substitution, widened authority, or digest mismatch refuses before mutation.
7. NEMESIS creates a Git worktree lane, commits authorization through Core and Kernel, restarts Core at the declared recovery boundary, writes only the lane file, runs `git diff --check` plus a sandboxed content verifier, accepts current-source evidence, signs a local receipt, rejects a one-byte receipt mutation, and writes an authoritative replay.
8. **Evidence** shows the current claims, source digest, ledger head, local receipt directory, and isolated lane. **Replay** reconstructs recorded authoritative state; it does not claim identical future model tokens.

## Contract example

```json
{
  "schema": "nemesis.desktop-mission/v1",
  "missionId": "mis_0123456789abcdefghij12",
  "goal": "Replace one bounded UTF-8 file in an isolated lane.",
  "workspace": "/absolute/path/to/clean/repository",
  "baseRevision": "0000000000000000000000000000000000000000",
  "action": {
    "kind": "replace_utf8",
    "relativePath": "value.txt",
    "expectedSha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "replacement": "after\n"
  },
  "authority": {
    "network": false,
    "push": false,
    "publish": false,
    "secrets": false
  },
  "budgets": {
    "maxWriteBytes": 4096,
    "maxRuntimeSeconds": 300,
    "maxOutputBytes": 1048576
  },
  "completion": ["git_diff_check", "content_match"]
}
```

The Desktop drafter replaces the example identities with observed current values. Hand-authored contracts must provide exact current identities.

## Refusal and recovery

- The canonical repository must be clean and its `HEAD` must equal `baseRevision`.
- The target must be an existing regular non-symlink file under the canonical repository and its bytes must equal `expectedSha256`.
- Replacement and process output are bounded by the contract; command time is bounded and cancellation kills the current bounded child process.
- Child processes receive a fixed clean environment and absolute executable paths. Release resources must be regular executable files, not symlinks.
- Failure never becomes completion. Desktop renders a typed refusal code, exact cause, and recovery instruction. Failed mission logs remain under the local home.

## Current authority boundary

The flow binds explicit contract and action digests and uses the existing capability and completion decisions. It does not claim that `Nemesis.Kernel.Approvals` is durably integrated into the daemon: that accept-condition change is a recorded trust-surface blocker and is not silently landed by the production-readiness mission.

## Non-claims

This contract does not grant network, secret, push, publish, arbitrary command, arbitrary file-creation, merge, or canonical-repository mutation authority. It does not provide model-token replay, automatic updates, Developer ID signing, notarization, or Gatekeeper acceptance.
