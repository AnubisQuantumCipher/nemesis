# NEMESIS Durable Storage Boundary

## Canonical versus derived state

The append-only ledger, checkpoint, and content-addressed object bytes are canonical. SQLite is a rebuildable derived index. A database row never overrides a valid ledger record, and a missing/corrupt index does not authorize a mission transition.

## Ledger v1

Each record is one fixed-length ASCII frame followed by LF:

```text
NEMESIS_LEDGER_V1|<sequence:20>|<kind:2>|<state:2>|<source-sha256:64>|<payload-sha256:64>|<previous-event-sha256:64>|<event-sha256:64>\n
```

The event digest is SHA-256 over the canonical bytes before the final separator/digest. Recovery validates schema, frame length, delimiters, decimal encoding, enumerations, lowercase hexadecimal fields, exact sequence, previous hash, event hash, and LF. A complete valid prefix plus an incomplete tail is reported as `Truncated_Tail` with the prefix state; non-tail mutation, reordering, duplication, or an invalid full frame is `Corrupt`. Core must preserve and resolve a truncated tail before appending. It must not run through `Corrupt` or `Unsupported_Version`.

Append writes one frame, verifies a full write, calls host `fsync`, and checks close before returning `Committed`. Tests simulate mutations, reordered records, duplicate records, a partial first frame, and a partial later frame. This establishes process-crash recovery behavior exercised on this host. It does not claim power-loss durability beyond the host filesystem and `fsync` semantics; directory-entry durability remains an explicit operating-system assumption.

## Checkpoints

A checkpoint is an atomically renamed fixed canonical record binding sequence, state, ledger head, source digest, and its own SHA-256. The temporary file is fully written, `fsync` succeeds, and close succeeds before rename. Checkpoints accelerate recovery; they do not replace ledger verification.

## Object store

Objects live at:

```text
<nemesis-home>/objects/sha256/<first-two-hex>/<remaining-sixty-two-hex>
```

`Put` derives the path from data bytes, writes and syncs a temporary file, then renames it. Existing objects are rehashed; a byte mismatch is `Collision_Detected`, not overwrite. `Get` rehashes bytes before returning `Loaded_Valid`.

## SQLite derived index

The local index uses the system SQLite 3 C API through a narrow Ada binding and links `-lsqlite3`. It enables WAL, `synchronous=FULL`, and foreign keys. Schema version 1 stores mission summaries and event identities; `PRAGMA user_version` is the migration authority. A newer unknown version is refused.

GNATCOLL SQLite 25.0.0 was evaluated first. Its transitive XMLAda/C build failed against the installed macOS 26 SDK and GNAT 14.2.1 fixed headers with missing C standard types in `stdio.h`. The failure occurred before NEMESIS code linked. The dependency was removed rather than patched or weakened. The direct binding keeps the authority surface smaller and avoids carrying XML/ORM dependencies into the daemon. SQLite and the system C ABI remain explicit unproved assumptions.

## Gates

```sh
./scripts/test_storage.sh
```

The gate runs the kernel, ledger hostile-recovery, checkpoint/object integrity, and SQLite migration/index executables. Build warnings produced by the current GNAT/macOS deployment-target mismatch are retained and documented; application-source warnings remain build-breaking.
