use std::fs;

use ed25519_dalek::SigningKey;
use nemesis_runtime::{
    ContextCompiler, ContextError, ContextRequest, KnowledgeBase, KnowledgeEntry, KnowledgeStatus,
    WorkerRole, index_repository, verify_context_capsule,
};

fn entry(id: u64, value: &str) -> KnowledgeEntry {
    KnowledgeEntry {
        id,
        subject: "repository".to_owned(),
        predicate: "language".to_owned(),
        value: value.to_owned(),
        scope: "mission".to_owned(),
        source_digest: [id as u8; 32],
        observed_sequence: id,
        valid_from: id,
        valid_until: None,
        status: KnowledgeStatus::Verified,
        supersedes: None,
        contradicted_by: vec![],
        untrusted_external: false,
    }
}

fn request(sequence: u64, max_bytes: usize) -> ContextRequest {
    ContextRequest {
        mission_id: "mis_0000000000000000000000".to_owned(),
        source_digest: [0x44; 32],
        worker_role: WorkerRole::Builder,
        obligations: vec!["build".to_owned(), "tests".to_owned()],
        invariants: vec!["push=false".to_owned(), "tests_must_not_weaken".to_owned()],
        decisions: vec!["use isolated lane".to_owned()],
        failed_approaches: vec!["ambient shell refused".to_owned()],
        blockers: vec![],
        capability_summary: "filesystem.modify:lane-only".to_owned(),
        remaining_budget_units: 100,
        capsule_sequence: sequence,
        maximum_bytes: max_bytes,
    }
}

#[test]
fn superseded_revoked_and_expired_facts_are_not_active() {
    let mut knowledge = KnowledgeBase::new();
    knowledge.insert(entry(1, "Ada")).unwrap();
    knowledge
        .insert(KnowledgeEntry {
            id: 2,
            supersedes: Some(1),
            value: "Ada 2022".to_owned(),
            ..entry(2, "unused")
        })
        .unwrap();
    knowledge
        .insert(KnowledgeEntry {
            id: 3,
            status: KnowledgeStatus::Revoked,
            ..entry(3, "revoked")
        })
        .unwrap();
    knowledge
        .insert(KnowledgeEntry {
            id: 4,
            valid_until: Some(4),
            ..entry(4, "expired")
        })
        .unwrap();

    let current = knowledge.active_at(5);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].id, 2);
    assert_eq!(
        knowledge.get(1).unwrap().status,
        KnowledgeStatus::Superseded
    );
}

#[test]
fn context_replacement_retains_obligations_and_visible_contradictions() {
    let signing = SigningKey::from_bytes(&[5; 32]);
    let mut knowledge = KnowledgeBase::new();
    knowledge.insert(entry(1, "Ada")).unwrap();
    knowledge.insert(entry(2, "Rust")).unwrap();
    knowledge.record_contradiction(1, 2).unwrap();

    let first = ContextCompiler::compile(&knowledge, request(1, 8_192), &signing).unwrap();
    knowledge.revoke(2).unwrap();
    let second = ContextCompiler::compile(&knowledge, request(2, 8_192), &signing).unwrap();

    assert_eq!(first.capsule.obligations, second.capsule.obligations);
    assert_eq!(first.capsule.invariants, second.capsule.invariants);
    assert_eq!(first.capsule.contradictions, vec![(1, 2)]);
    assert!(second.capsule.knowledge.iter().all(|fact| fact.id != 2));
    assert!(verify_context_capsule(&second, &signing.verifying_key()).is_ok());
}

#[test]
fn context_budget_refuses_instead_of_omitting_mandatory_state() {
    let signing = SigningKey::from_bytes(&[6; 32]);
    let knowledge = KnowledgeBase::new();
    assert!(matches!(
        ContextCompiler::compile(&knowledge, request(1, 32), &signing),
        Err(ContextError::BudgetTooSmall)
    ));
}

#[test]
fn capsule_signature_rejects_one_byte_mutation() {
    let signing = SigningKey::from_bytes(&[7; 32]);
    let capsule =
        ContextCompiler::compile(&KnowledgeBase::new(), request(1, 8_192), &signing).unwrap();
    let mut tampered = capsule.clone();
    tampered.encoded[0] ^= 1;
    assert!(verify_context_capsule(&tampered, &signing.verifying_key()).is_err());
}

#[test]
fn repository_index_binds_relative_files_and_rejects_traversal() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("source.txt"), "alpha\n").unwrap();
    let indexed = index_repository(temp.path(), &["source.txt".to_owned()], 1_024).unwrap();
    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].relative_path, "source.txt");
    assert_eq!(indexed[0].byte_length, 6);
    assert!(index_repository(temp.path(), &["../escape".to_owned()], 1_024).is_err());
}
