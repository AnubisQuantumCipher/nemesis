use std::collections::BTreeSet;

use ed25519_dalek::SigningKey;
use nemesis_runtime::{
    AutomationDecision, AutomationOperation, AutomationPolicy, AutomationQueue, AutomationRequest,
    AutomationStatus, TriggerKind,
};

fn request(id: u64, operation: AutomationOperation) -> AutomationRequest {
    AutomationRequest {
        id,
        mission_template_digest: [id as u8; 32],
        operation,
        trigger: TriggerKind::LocalSchedule,
        trigger_key: format!("daily-{id}"),
        due_unix_seconds: 1_000,
        requires_approval: false,
    }
}

#[test]
fn unattended_authority_boundary_denies_instead_of_waiting_or_proceeding() {
    let policy = AutomationPolicy::safe_unattended(BTreeSet::from([AutomationOperation::Observe]));
    assert_eq!(
        policy.evaluate(&request(1, AutomationOperation::ModifyWorkspace)),
        AutomationDecision::DeniedAuthorityBoundary
    );
    let mut approval = request(2, AutomationOperation::Observe);
    approval.requires_approval = true;
    assert_eq!(
        policy.evaluate(&approval),
        AutomationDecision::DeniedAuthorityBoundary
    );
    assert_eq!(
        policy.evaluate(&request(3, AutomationOperation::Push)),
        AutomationDecision::DeniedAuthorityBoundary
    );
}

#[test]
fn queue_is_idempotent_bounded_and_rate_limited() {
    let policy = AutomationPolicy::safe_unattended(BTreeSet::from([AutomationOperation::Observe]));
    let mut queue = AutomationQueue::new(2, 1).unwrap();
    queue
        .enqueue(request(1, AutomationOperation::Observe))
        .unwrap();
    assert!(
        queue
            .enqueue(request(9, AutomationOperation::Observe))
            .is_ok()
    );
    let duplicate = AutomationRequest {
        id: 3,
        trigger_key: "daily-1".to_owned(),
        ..request(3, AutomationOperation::Observe)
    };
    assert!(queue.enqueue(duplicate).is_err());
    assert!(
        queue
            .enqueue(request(4, AutomationOperation::Observe))
            .is_err()
    );

    let first = queue.dispatch_next(1_000, &policy).unwrap();
    assert_eq!(first.decision, AutomationDecision::Authorized);
    assert_eq!(queue.status(1), Some(AutomationStatus::Running));
    assert!(queue.dispatch_next(1_000, &policy).is_err());
    queue.complete(1, true).unwrap();
    assert_eq!(queue.status(1), Some(AutomationStatus::Completed));
    let second = queue.dispatch_next(1_061, &policy).unwrap();
    assert_eq!(second.request_id, 9);
}

#[test]
fn denied_request_is_terminal_not_waiting_approval() {
    let policy = AutomationPolicy::safe_unattended(BTreeSet::new());
    let mut queue = AutomationQueue::new(4, 4).unwrap();
    queue
        .enqueue(request(1, AutomationOperation::Network))
        .unwrap();
    let dispatch = queue.dispatch_next(1_000, &policy).unwrap();
    assert_eq!(
        dispatch.decision,
        AutomationDecision::DeniedAuthorityBoundary
    );
    assert_eq!(queue.status(1), Some(AutomationStatus::Denied));
}

#[test]
fn signed_queue_checkpoint_recovers_exactly_and_rejects_tamper() {
    let signing = SigningKey::from_bytes(&[17; 32]);
    let mut queue = AutomationQueue::new(4, 2).unwrap();
    queue
        .enqueue(request(1, AutomationOperation::Observe))
        .unwrap();
    let checkpoint = queue.checkpoint(&signing).unwrap();
    let recovered = AutomationQueue::recover(&checkpoint, &signing.verifying_key()).unwrap();
    assert_eq!(recovered, queue);

    let mut tampered = checkpoint.clone();
    tampered.encoded[0] ^= 1;
    assert!(AutomationQueue::recover(&tampered, &signing.verifying_key()).is_err());
}
