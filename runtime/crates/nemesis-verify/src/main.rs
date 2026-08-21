use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use ed25519_dalek::VerifyingKey;
use nemesis_receipt::{EpistemicStatus, verify_sign1};
use serde_json::json;

const MAX_RECEIPT_BYTES: u64 = 1024 * 1024;

fn paths() -> Result<(PathBuf, PathBuf), String> {
    let mut arguments = env::args_os().skip(1);
    let mut receipt = None;
    let mut public_key = None;
    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {}", flag.to_string_lossy()))?;
        match flag.to_str() {
            Some("--receipt") if receipt.is_none() => receipt = Some(PathBuf::from(value)),
            Some("--public-key") if public_key.is_none() => public_key = Some(PathBuf::from(value)),
            _ => {
                return Err(format!(
                    "unknown or duplicate option: {}",
                    flag.to_string_lossy()
                ));
            }
        }
    }
    Ok((
        receipt.ok_or_else(|| "missing --receipt".to_owned())?,
        public_key.ok_or_else(|| "missing --public-key".to_owned())?,
    ))
}

fn verify() -> Result<serde_json::Value, String> {
    let (receipt_path, public_key_path) = paths()?;
    let metadata = fs::metadata(&receipt_path).map_err(|error| error.to_string())?;
    if metadata.len() == 0 || metadata.len() > MAX_RECEIPT_BYTES {
        return Err("receipt is empty or oversized".to_owned());
    }
    let receipt = fs::read(receipt_path).map_err(|error| error.to_string())?;
    let public_key_hex = fs::read_to_string(public_key_path).map_err(|error| error.to_string())?;
    let public_key_bytes = hex::decode(public_key_hex.trim()).map_err(|error| error.to_string())?;
    let public_key_array: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| "public key must contain 32 bytes".to_owned())?;
    let public_key =
        VerifyingKey::from_bytes(&public_key_array).map_err(|error| error.to_string())?;
    let verified = verify_sign1(&receipt, &public_key).map_err(|error| error.to_string())?;
    let claims: Vec<_> = verified
        .payload
        .claims
        .iter()
        .map(|claim| {
            json!({
                "id": claim.id,
                "mandatory": claim.mandatory,
                "status": match claim.status {
                    EpistemicStatus::Verified => "VERIFIED",
                    EpistemicStatus::Believed => "BELIEVED",
                    EpistemicStatus::Unknown => "UNKNOWN",
                },
                "evidence_digest": hex::encode(claim.evidence_digest),
            })
        })
        .collect();
    Ok(json!({
        "verdict": "VERIFIED",
        "schema": "nemesis.receipt/v1",
        "mission_id": verified.payload.mission_id,
        "event_sequence": verified.payload.event_sequence,
        "terminal_state": verified.payload.terminal_state,
        "source_digest": hex::encode(verified.payload.source_digest),
        "ledger_head": hex::encode(verified.payload.ledger_head),
        "claims": claims,
        "residuals": verified.payload.residuals,
    }))
}

fn main() -> ExitCode {
    match verify() {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string(&value).expect("JSON value serializes")
            );
            ExitCode::SUCCESS
        }
        Err(reason) => {
            println!(
                "{}",
                serde_json::to_string(&json!({"verdict": "REJECTED", "reason": reason}))
                    .expect("JSON value serializes")
            );
            ExitCode::from(1)
        }
    }
}
