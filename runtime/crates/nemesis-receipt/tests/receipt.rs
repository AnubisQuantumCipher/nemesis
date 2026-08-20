use ed25519_dalek::SigningKey;
use nemesis_receipt::{
    ClaimReceipt, EpistemicStatus, ReceiptError, ReceiptPayload, sign1, verify_sign1,
};

fn payload(status: EpistemicStatus) -> ReceiptPayload {
    ReceiptPayload {
        mission_id: "mis_0000000000000000000000".to_owned(),
        event_sequence: 12,
        terminal_state: "COMPLETE".to_owned(),
        source_digest: [0x11; 32],
        ledger_head: [0x22; 32],
        claims: vec![
            ClaimReceipt {
                id: "build".to_owned(),
                mandatory: true,
                status,
                evidence_digest: [0x33; 32],
            },
            ClaimReceipt {
                id: "tests".to_owned(),
                mandatory: true,
                status: EpistemicStatus::Verified,
                evidence_digest: [0x44; 32],
            },
        ],
        residuals: vec!["Public release remains DEFERRED.".to_owned()],
    }
}

#[test]
fn canonical_sign1_is_deterministic_and_independently_verifiable() {
    let signing = SigningKey::from_bytes(&[7; 32]);
    let first = sign1(&payload(EpistemicStatus::Verified), &signing).unwrap();
    let second = sign1(&payload(EpistemicStatus::Verified), &signing).unwrap();
    assert_eq!(first, second);

    let verified = verify_sign1(&first, &signing.verifying_key()).unwrap();
    assert_eq!(verified.payload, payload(EpistemicStatus::Verified));
    assert!(verified.is_bound_to(&[0x11; 32]));
    assert!(!verified.is_bound_to(&[0x12; 32]));
}

#[test]
fn every_single_byte_mutation_is_rejected() {
    let signing = SigningKey::from_bytes(&[9; 32]);
    let receipt = sign1(&payload(EpistemicStatus::Verified), &signing).unwrap();
    for index in 0..receipt.len() {
        let mut tampered = receipt.clone();
        tampered[index] ^= 1;
        assert!(
            verify_sign1(&tampered, &signing.verifying_key()).is_err(),
            "mutation at byte {index} was accepted"
        );
    }
}

#[test]
fn wrong_key_and_nonverified_mandatory_claim_are_rejected() {
    let signing = SigningKey::from_bytes(&[11; 32]);
    let wrong = SigningKey::from_bytes(&[12; 32]);
    let complete = sign1(&payload(EpistemicStatus::Verified), &signing).unwrap();
    assert!(matches!(
        verify_sign1(&complete, &wrong.verifying_key()),
        Err(ReceiptError::Signature)
    ));

    let believed = sign1(&payload(EpistemicStatus::Believed), &signing).unwrap();
    assert!(matches!(
        verify_sign1(&believed, &signing.verifying_key()),
        Err(ReceiptError::IncompleteClaims)
    ));
}
