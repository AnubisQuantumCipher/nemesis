use std::collections::BTreeSet;

use nemesis_runtime::{
    AdaReplay, CausalGraph, EvidenceClass, EvidenceRecord, EvidenceStore, EvidenceVerdict,
    GraphNodeKind, ReplayError, VerifierRegistry, fork_replay,
};
use sha2::{Digest, Sha256};

fn ledger_record(
    sequence: u64,
    kind: u8,
    state: u8,
    source: [u8; 32],
    payload: [u8; 32],
    previous: [u8; 32],
) -> (Vec<u8>, [u8; 32]) {
    let core = format!(
        "NEMESIS_LEDGER_V1|{sequence:020}|{kind:02}|{state:02}|{}|{}|{}",
        hex::encode(source),
        hex::encode(payload),
        hex::encode(previous)
    );
    assert_eq!(core.len(), 239);
    let digest: [u8; 32] = Sha256::digest(core.as_bytes()).into();
    let mut record = format!("{core}|{}\n", hex::encode(digest)).into_bytes();
    assert_eq!(record.len(), 305);
    (std::mem::take(&mut record), digest)
}

#[test]
fn replay_reconstructs_hash_chain_and_rejects_mutation() {
    let (first, first_hash) = ledger_record(1, 0, 1, [1; 32], [2; 32], [0; 32]);
    let (second, second_hash) = ledger_record(2, 3, 4, [3; 32], [4; 32], first_hash);
    let mut ledger = [first, second].concat();
    let replay = AdaReplay::parse(&ledger).unwrap();
    assert_eq!(replay.events.len(), 2);
    assert_eq!(replay.final_state_code, 4);
    assert_eq!(replay.head, second_hash);
    assert!(replay.exact_state_reconstruction);
    assert!(!replay.exact_model_reexecution);

    ledger[100] ^= 1;
    assert!(matches!(
        AdaReplay::parse(&ledger),
        Err(ReplayError::Corrupt(_))
    ));
}

#[test]
fn old_source_and_unauthorized_verifier_cannot_complete_claims() {
    let mut registry = VerifierRegistry::new();
    registry
        .register("release-tests", EvidenceClass::DecisionBoundary)
        .unwrap();
    let mut store = EvidenceStore::new();
    let current = [0x44; 32];
    let stale = EvidenceRecord {
        id: 1,
        claim: "tests".to_owned(),
        source_digest: [0x33; 32],
        dependency_digests: BTreeSet::new(),
        verifier_id: "release-tests".to_owned(),
        class: EvidenceClass::DecisionBoundary,
        deterministic: true,
        independent: true,
        verdict: EvidenceVerdict::Pending,
    };
    assert!(store.accept(stale, current, &registry).is_err());

    let unauthorized = EvidenceRecord {
        id: 2,
        claim: "tests".to_owned(),
        source_digest: current,
        dependency_digests: BTreeSet::new(),
        verifier_id: "unknown".to_owned(),
        class: EvidenceClass::DecisionBoundary,
        deterministic: true,
        independent: true,
        verdict: EvidenceVerdict::Pending,
    };
    assert!(store.accept(unauthorized, current, &registry).is_err());
    assert!(!store.can_complete(&["tests".to_owned()], current));
}

#[test]
fn changed_dependency_invalidates_accepted_evidence() {
    let mut registry = VerifierRegistry::new();
    registry
        .register("compiler", EvidenceClass::DecisionBoundary)
        .unwrap();
    let source = [0x55; 32];
    let dependency = [0x66; 32];
    let mut store = EvidenceStore::new();
    let evidence = EvidenceRecord {
        id: 7,
        claim: "build".to_owned(),
        source_digest: source,
        dependency_digests: BTreeSet::from([dependency]),
        verifier_id: "compiler".to_owned(),
        class: EvidenceClass::DecisionBoundary,
        deterministic: true,
        independent: true,
        verdict: EvidenceVerdict::Pending,
    };
    store.accept(evidence, source, &registry).unwrap();
    assert!(store.can_complete(&["build".to_owned()], source));
    store.invalidate(source, &BTreeSet::from([dependency]));
    assert!(!store.can_complete(&["build".to_owned()], source));
    assert_eq!(store.get(7).unwrap().verdict, EvidenceVerdict::Stale);
}

#[test]
fn causal_graph_returns_recorded_chain_not_retrospective_story() {
    let mut graph = CausalGraph::new();
    graph.add(1, GraphNodeKind::Requirement, &[]).unwrap();
    graph.add(2, GraphNodeKind::Observation, &[1]).unwrap();
    graph.add(3, GraphNodeKind::Action, &[2]).unwrap();
    graph.add(4, GraphNodeKind::Evidence, &[3]).unwrap();
    assert_eq!(graph.path_to(4).unwrap(), vec![1, 2, 3, 4]);
    assert!(graph.add(5, GraphNodeKind::Action, &[99]).is_err());
}

#[test]
fn mission_fork_preserves_prefix_and_marks_model_reexecution_comparative() {
    let (first, first_hash) = ledger_record(1, 0, 1, [1; 32], [2; 32], [0; 32]);
    let (second, _) = ledger_record(2, 3, 4, [3; 32], [4; 32], first_hash);
    let replay = AdaReplay::parse(&[first, second].concat()).unwrap();
    let fork = fork_replay(&replay, 1, "mis_1111111111111111111111").unwrap();
    assert_eq!(fork.parent_sequence, 1);
    assert_eq!(fork.events.len(), 1);
    assert!(fork.model_reexecution_is_comparative);
    assert_eq!(replay.events.len(), 2);
}
