use std::process::Command;

use nemesis_protocol::{WorkerMessage, parse_worker_message};

const MISSION: &str = "mis_0000000000000000000000";
const WORKER: &str = "wrk_0000000000000000000000";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn worker() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nemesis-deterministic-worker"))
}

#[test]
fn plan_emits_only_protocol_valid_proposals() {
    let output = worker()
        .args([
            "plan",
            "--mission-id",
            MISSION,
            "--worker-id",
            WORKER,
            "--path",
            "value.txt",
            "--content-digest",
            DIGEST,
        ])
        .output()
        .expect("worker starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let messages: Vec<_> = stdout
        .lines()
        .map(|line| parse_worker_message(line.as_bytes()).expect("valid worker message"))
        .collect();
    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0].message, WorkerMessage::Heartbeat));
    assert!(matches!(
        messages[1].message,
        WorkerMessage::ProposeAction(_)
    ));
    assert!(!stdout.contains("event_sequence"));
    assert!(!stdout.contains("authoritative_timestamp"));
    assert!(!stdout.contains("\"method\":\"complete\""));
}

#[test]
fn completion_mode_emits_a_proposal_not_a_state_transition() {
    let output = worker()
        .args(["complete", "--mission-id", MISSION, "--worker-id", WORKER])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed = parse_worker_message(&output.stdout).expect("valid completion proposal");
    assert!(matches!(
        parsed.message,
        WorkerMessage::ProposeCompletion(_)
    ));
}

#[test]
fn invalid_action_path_fails_without_emitting_a_proposal() {
    let output = worker()
        .args([
            "plan",
            "--mission-id",
            MISSION,
            "--worker-id",
            WORKER,
            "--path",
            "../escape",
            "--content-digest",
            DIGEST,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
