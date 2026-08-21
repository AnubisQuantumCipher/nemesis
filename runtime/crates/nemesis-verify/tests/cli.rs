use std::fs;
use std::process::Command;

use ed25519_dalek::SigningKey;
use nemesis_receipt::{ClaimReceipt, EpistemicStatus, ReceiptPayload, sign1};

fn fixture() -> (Vec<u8>, SigningKey) {
    let signing = SigningKey::from_bytes(&[21; 32]);
    let payload = ReceiptPayload {
        mission_id: "mis_0000000000000000000000".to_owned(),
        event_sequence: 9,
        terminal_state: "COMPLETE".to_owned(),
        source_digest: [0x55; 32],
        ledger_head: [0x66; 32],
        claims: vec![ClaimReceipt {
            id: "tests".to_owned(),
            mandatory: true,
            status: EpistemicStatus::Verified,
            evidence_digest: [0x77; 32],
        }],
        residuals: vec!["No public-release claim.".to_owned()],
    };
    (sign1(&payload, &signing).unwrap(), signing)
}

#[test]
fn standalone_cli_accepts_valid_and_rejects_tampered_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let receipt_path = temp.path().join("receipt.cose");
    let public_key_path = temp.path().join("receipt.pub");
    let (receipt, signing) = fixture();
    fs::write(&receipt_path, &receipt).unwrap();
    fs::write(
        &public_key_path,
        hex::encode(signing.verifying_key().as_bytes()),
    )
    .unwrap();

    let valid = Command::new(env!("CARGO_BIN_EXE_nemesis-verify"))
        .args(["--receipt"])
        .arg(&receipt_path)
        .args(["--public-key"])
        .arg(&public_key_path)
        .output()
        .unwrap();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let output: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(output["verdict"], "VERIFIED");
    assert_eq!(output["mission_id"], "mis_0000000000000000000000");
    assert_eq!(output["source_digest"], "55".repeat(32));

    let mut tampered = receipt;
    let middle = tampered.len() / 2;
    tampered[middle] ^= 1;
    fs::write(&receipt_path, tampered).unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_nemesis-verify"))
        .args(["--receipt"])
        .arg(&receipt_path)
        .args(["--public-key"])
        .arg(&public_key_path)
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let output: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(output["verdict"], "REJECTED");
}
