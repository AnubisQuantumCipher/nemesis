# NEMESIS Evidence and Replay v1

Evidence records bind a claim to final source, declared dependency digests, verifier identity, consequence class, determinism, independence, and verdict. A verifier registry limits the maximum consequence each verifier may establish. Decision-boundary and safety-critical evidence must be deterministic and independent.

Accepted evidence becomes `Stale` when its source changes or a declared dependency intersects the changed-object set. Completion requires accepted current evidence for every required claim; unsupported claims remain unsupported.

The replay parser independently validates every fixed Ada ledger frame: schema, length, delimiters, LF, sequence, event/state codes, lowercase digests, previous hash, and SHA-256 event hash. It reconstructs exact recorded state and explicitly reports `exact_model_reexecution: false`.

The causal graph accepts only references to already committed nodes and returns recorded ancestor chains. Mission forks preserve an exact event prefix and parent head; any model rerun is labeled comparative.

`nemesis-replay --ledger <path>` starts no daemon and calls no model. The Phase 11 evidence bundle includes the original signed receipt, tamper fixture, final ledger, and independent replay JSON.

```sh
./scripts/test_evidence_replay.sh
```

A valid signature or replay establishes only the encoded and reconstructed bounded properties. It does not make nondeterministic model tokens reproducible or prove external real-world claims.
