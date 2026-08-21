# NEMESIS Worker Adapter Contract v1

Adapters translate provider-specific processes or APIs into `nemesis.worker/v1`; they never acquire mission authority from provider identity.

Each profile records provider kind, supported roles, consequence ceiling, executable/API availability, and usage accounting. Fallback selection is deterministic in contract order and returns the original authority fingerprint unchanged. A candidate is eligible only when it is available, supports the assigned role, and has a sufficient consequence ceiling.

Implemented local profiles:

- generic sandboxed subprocess;
- Codex CLI health/profile;
- Claude Code CLI health/profile;
- local model CLI health/profile;
- OpenAI API and Anthropic API profiles that remain typed `UNAVAILABLE` until both scoped network and secret capabilities are present.

The generic subprocess adapter runs the exact executable through Workspace Safe, applies a combined stdout/stderr byte bound, requires successful process exit, and parses every nonempty stdout line through the strict worker protocol. Provider output remains a proposal even after successful parsing.

Usage is recorded in integer cost microunits and input/output/context token units. Checked addition and per-field limits refuse over-budget records without mutating the ledger.

```sh
./scripts/test_adapters.sh
runtime/target/debug/nemesis-adapter-health
```

These gates establish adapter/profile behavior on configured local executables. They do not call paid APIs, inherit provider credentials, or treat provider availability as evidence of mission completion.
