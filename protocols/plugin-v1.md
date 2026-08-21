# NEMESIS Plugin, Skill, and MCP Boundary v1

## Skills

Skill promotion is a strict state machine:

`Proposed → Staged → StaticallyChecked → TestedInSandbox → Evaluated → Approved → Trusted`

States cannot be skipped. `Trusted` additionally requires a named approver and nonzero approval signature. Rejection is possible before trust; a trusted skill cannot silently rewrite itself.

## Signed plugin manifests

A canonical manifest binds plugin identity/version, `nemesis.plugin/v1`, kind, module SHA-256, requested capabilities, and evidence types. Ed25519 verification and a trusted public key are required before installation. Requested network hosts, filesystem roots, secrets, and events must each be subsets of the explicit grant.

Removing a plugin removes executable registration but leaves previously recorded mission data readable.

## WASI backend host

The local backend host uses Wasmtime with WASI Preview 1 linking, no inherited environment, stdio, filesystem preopens, or network grants. This bounded host currently accepts only an empty capability set. Module size, linear memory, instance/table count, and execution fuel are bounded. Infinite loops and oversized memory refuse.

## UI plugins

UI methods pass through an allowlisted message broker. The supplied CSP begins with `default-src 'none'`, allows no network connection, and exposes no shell/filesystem/native host object.

## MCP gateway

Tool definitions and schema digests are registered before a mission then frozen. Invocation requires an exact schema digest and capability superset, applies a call limit, and removes inherited environment values except bounded locale/timezone keys. Active schema mutation, unknown tools, ungranted egress, and rate exhaustion refuse.

```sh
./scripts/test_extensions.sh
```

These controls establish the exercised local extension boundary. They do not claim third-party plugin correctness or universal WebAssembly sandbox soundness.
