use nemesis_receipt::EpistemicStatus;
use nemesis_signer::parse_request;
use serde_json::json;

#[test]
fn converts_strict_json_request_to_bounded_receipt_payload() {
    let request = json!({
        "mission_id": "mis_0000000000000000000000",
        "event_sequence": 12,
        "terminal_state": "COMPLETE",
        "source_digest": "11".repeat(32),
        "ledger_head": "22".repeat(32),
        "claims": [{
            "id": "tests",
            "mandatory": true,
            "status": "VERIFIED",
            "evidence_digest": "33".repeat(32)
        }],
        "residuals": ["Public release remains DEFERRED."]
    });
    let payload = parse_request(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert_eq!(payload.event_sequence, 12);
    assert_eq!(payload.source_digest, [0x11; 32]);
    assert_eq!(payload.claims[0].status, EpistemicStatus::Verified);
}

#[test]
fn rejects_unknown_fields_and_nonhex_digests() {
    let mut request = json!({
        "mission_id": "mis_0000000000000000000000",
        "event_sequence": 12,
        "terminal_state": "COMPLETE",
        "source_digest": "11".repeat(32),
        "ledger_head": "22".repeat(32),
        "claims": [{
            "id": "tests",
            "mandatory": true,
            "status": "VERIFIED",
            "evidence_digest": "33".repeat(32)
        }],
        "residuals": []
    });
    request["worker_supplied_sequence"] = json!(91);
    assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_err());
    request
        .as_object_mut()
        .unwrap()
        .remove("worker_supplied_sequence");
    request["source_digest"] = json!("not-hex");
    assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_err());
}
