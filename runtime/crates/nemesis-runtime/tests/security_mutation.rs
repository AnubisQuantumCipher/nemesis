use std::panic::{AssertUnwindSafe, catch_unwind};

use nemesis_protocol::{ProtocolError, parse_worker_message};
use nemesis_runtime::{AdaReplay, WorkerOutputAccumulator};
use sha2::{Digest, Sha256};

const VALID_WORKER: &[u8] = br#"{"jsonrpc":"2.0","id":1,"protocol":"nemesis.worker/v1","mission_id":"mis_0000000000000000000000","worker_id":"wrk_0000000000000000000000","method":"heartbeat","params":{}}"#;

fn record(sequence: u64, previous: [u8; 32]) -> (Vec<u8>, [u8; 32]) {
    let core = format!(
        "NEMESIS_LEDGER_V1|{sequence:020}|00|01|{}|{}|{}",
        "11".repeat(32),
        "22".repeat(32),
        hex::encode(previous)
    );
    let digest: [u8; 32] = Sha256::digest(core.as_bytes()).into();
    (
        format!("{core}|{}\n", hex::encode(digest)).into_bytes(),
        digest,
    )
}

#[test]
fn worker_protocol_mutations_never_panic_or_gain_authority() {
    for index in 0..VALID_WORKER.len() {
        let mut mutated = VALID_WORKER.to_vec();
        mutated[index] ^= 0x80;
        let result = catch_unwind(AssertUnwindSafe(|| parse_worker_message(&mutated)));
        assert!(result.is_ok(), "parser panicked at mutation {index}");
    }
    for forbidden in [
        "complete",
        "commit_mission_state",
        "issue_capability",
        "accept_evidence",
        "rewrite_policy",
        "expand_budget",
        "use_secret",
    ] {
        let message = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":1,\"protocol\":\"nemesis.worker/v1\",\"mission_id\":\"mis_0000000000000000000000\",\"worker_id\":\"wrk_0000000000000000000000\",\"method\":\"{forbidden}\",\"params\":{{}}}}"
        );
        assert!(matches!(
            parse_worker_message(message.as_bytes()),
            Err(ProtocolError::ForbiddenMethod(_))
        ));
    }
}

#[test]
fn encoded_path_and_schema_attacks_refuse() {
    for path in [
        "..%2fescape",
        "src/%2e%2e/escape",
        "src\\..\\escape",
        "src/../escape",
        "/tmp/escape",
        "src/\u{0}escape",
        ".git/config",
        "src//escape",
        "src/\nescape",
    ] {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "protocol": "nemesis.worker/v1",
            "mission_id": "mis_0000000000000000000000",
            "worker_id": "wrk_0000000000000000000000",
            "method": "propose_action",
            "params": {
                "action_kind": "filesystem.modify",
                "relative_path": path,
                "content_digest": "aa".repeat(32),
                "purpose": "hostile corpus"
            }
        });
        assert!(
            parse_worker_message(&serde_json::to_vec(&message).unwrap()).is_err(),
            "accepted hostile path {path:?}"
        );
    }
}

#[test]
fn every_ledger_byte_mutation_rejects() {
    let (first, head) = record(1, [0; 32]);
    let (second, _) = record(2, head);
    let ledger = [first, second].concat();
    AdaReplay::parse(&ledger).unwrap();
    for index in 0..ledger.len() {
        let mut mutated = ledger.clone();
        mutated[index] ^= 1;
        assert!(
            AdaReplay::parse(&mutated).is_err(),
            "accepted ledger byte {index}"
        );
    }
}

#[test]
fn output_flood_fails_without_retaining_over_limit_bytes() {
    let mut output = WorkerOutputAccumulator::new(1024);
    output.push(&vec![0x41; 1024]).unwrap();
    assert!(output.push(&[0x42]).is_err());
    assert_eq!(output.as_bytes().len(), 1024);
}
