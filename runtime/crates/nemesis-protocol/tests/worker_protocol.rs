use nemesis_protocol::{MAX_MESSAGE_BYTES, ProtocolError, WorkerMessage, parse_worker_message};
use serde_json::json;

const MISSION: &str = "mis_0000000000000000000000";
const WORKER: &str = "wrk_0000000000000000000000";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn envelope(method: &str, params: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 7,
        "protocol": "nemesis.worker/v1",
        "mission_id": MISSION,
        "worker_id": WORKER,
        "method": method,
        "params": params
    }))
    .expect("fixture JSON")
}

#[test]
fn accepts_a_bounded_relative_action_proposal() {
    let bytes = envelope(
        "propose_action",
        json!({
            "action_kind": "filesystem.modify",
            "relative_path": "src/value.txt",
            "content_digest": DIGEST,
            "purpose": "Apply the authorized deterministic fixture change."
        }),
    );

    let parsed = parse_worker_message(&bytes).expect("valid proposal");
    assert_eq!(parsed.mission_id.as_str(), MISSION);
    assert_eq!(parsed.worker_id.as_str(), WORKER);
    assert!(matches!(parsed.message, WorkerMessage::ProposeAction(_)));
}

#[test]
fn permits_only_a_completion_proposal() {
    let bytes = envelope(
        "propose_completion",
        json!({"claim_ids": ["build", "tests"]}),
    );
    let parsed = parse_worker_message(&bytes).expect("proposal is protocol-valid");
    assert!(matches!(
        parsed.message,
        WorkerMessage::ProposeCompletion(_)
    ));

    let forbidden = envelope("complete", json!({}));
    assert!(matches!(
        parse_worker_message(&forbidden),
        Err(ProtocolError::ForbiddenMethod(method)) if method == "complete"
    ));
}

#[test]
fn rejects_worker_supplied_authoritative_fields() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&envelope("heartbeat", json!({}))).unwrap();
    value["event_sequence"] = json!(91);
    assert!(matches!(
        parse_worker_message(&serde_json::to_vec(&value).unwrap()),
        Err(ProtocolError::InvalidEnvelope(_))
    ));

    let mut params = json!({});
    params["authoritative_timestamp"] = json!("2026-08-20T00:00:00Z");
    assert!(matches!(
        parse_worker_message(&envelope("heartbeat", params)),
        Err(ProtocolError::InvalidParams(_))
    ));
}

#[test]
fn rejects_unknown_protocol_and_identifiers() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&envelope("heartbeat", json!({}))).unwrap();
    value["protocol"] = json!("nemesis.worker/v2");
    assert!(matches!(
        parse_worker_message(&serde_json::to_vec(&value).unwrap()),
        Err(ProtocolError::UnsupportedVersion(version)) if version == "nemesis.worker/v2"
    ));

    value["protocol"] = json!("nemesis.worker/v1");
    value["mission_id"] = json!("wrong");
    assert!(matches!(
        parse_worker_message(&serde_json::to_vec(&value).unwrap()),
        Err(ProtocolError::InvalidIdentifier(_))
    ));
}

#[test]
fn rejects_absolute_and_traversing_action_paths() {
    for path in ["/tmp/escape", "../escape", "src/../../escape", "."] {
        let bytes = envelope(
            "propose_action",
            json!({
                "action_kind": "filesystem.modify",
                "relative_path": path,
                "content_digest": DIGEST,
                "purpose": "malicious fixture"
            }),
        );
        assert!(matches!(
            parse_worker_message(&bytes),
            Err(ProtocolError::InvalidAction(_))
        ));
    }
}

#[test]
fn rejects_unknown_params_and_oversized_messages() {
    let bytes = envelope("heartbeat", json!({"surprise": true}));
    assert!(matches!(
        parse_worker_message(&bytes),
        Err(ProtocolError::InvalidParams(_))
    ));

    let oversized = vec![b'x'; MAX_MESSAGE_BYTES + 1];
    assert!(matches!(
        parse_worker_message(&oversized),
        Err(ProtocolError::MessageTooLarge { .. })
    ));
}
