use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const CONTRACT_SHA256: &str =
    "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemStatus {
    ready: bool,
    core: String,
    kernel: String,
    runtime: String,
    contract_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimResult {
    id: String,
    status: String,
    evidence_digest: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MissionRunResult {
    status: String,
    mission_id: String,
    sequence: u64,
    source_digest: String,
    ledger_head: String,
    artifact_directory: String,
    claims: Vec<ClaimResult>,
    tamper_verdict: String,
}

fn project_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|error| error.to_string())
}

fn lowercase_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[tauri::command]
fn system_status() -> Result<SystemStatus, String> {
    let root = project_root()?;
    let contract = root
        .join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md");
    let bytes = fs::read(contract).map_err(|error| error.to_string())?;
    let contract_sha256 = hex::encode(Sha256::digest(bytes));
    let core_ready = root.join("build/bin/nemesis_core_daemon").is_file();
    let runtime_ready = [
        "nemesis-deterministic-worker",
        "nemesis-lane-create",
        "nemesis-signer",
        "nemesis-verify",
        "nemesis-worker-runner",
    ]
    .iter()
    .all(|binary| root.join("runtime/target/debug").join(binary).is_file());
    let contract_ready = contract_sha256 == CONTRACT_SHA256;
    Ok(SystemStatus {
        ready: core_ready && runtime_ready && contract_ready,
        core: if core_ready { "READY" } else { "MISSING" }.to_owned(),
        kernel: if core_ready {
            "AVAILABLE"
        } else {
            "UNAVAILABLE"
        }
        .to_owned(),
        runtime: if runtime_ready { "READY" } else { "MISSING" }.to_owned(),
        contract_sha256,
    })
}

fn run_result_from_values(
    verification: &Value,
    tamper_verdict: &str,
    artifact_directory: &Path,
) -> Result<MissionRunResult, String> {
    if verification.get("verdict").and_then(Value::as_str) != Some("VERIFIED")
        || tamper_verdict != "REJECTED"
    {
        return Err("backend evidence was not independently verified".to_owned());
    }
    let mission_id = verification
        .get("mission_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "verification omitted mission_id".to_owned())?;
    if mission_id.len() != 26 || !mission_id.starts_with("mis_") {
        return Err("verification returned an invalid mission identifier".to_owned());
    }
    let sequence = verification
        .get("event_sequence")
        .and_then(Value::as_u64)
        .ok_or_else(|| "verification omitted event_sequence".to_owned())?;
    let source_digest = verification
        .get("source_digest")
        .and_then(Value::as_str)
        .ok_or_else(|| "verification omitted source_digest".to_owned())?;
    let ledger_head = verification
        .get("ledger_head")
        .and_then(Value::as_str)
        .ok_or_else(|| "verification omitted ledger_head".to_owned())?;
    if !lowercase_digest(source_digest) || !lowercase_digest(ledger_head) {
        return Err("verification returned malformed digests".to_owned());
    }
    let claim_values = verification
        .get("claims")
        .and_then(Value::as_array)
        .ok_or_else(|| "verification omitted claims".to_owned())?;
    let mut claims = Vec::with_capacity(claim_values.len());
    for claim in claim_values {
        let id = claim
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "claim omitted id".to_owned())?;
        let status = claim
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| "claim omitted status".to_owned())?;
        let evidence_digest = claim
            .get("evidence_digest")
            .and_then(Value::as_str)
            .ok_or_else(|| "claim omitted evidence digest".to_owned())?;
        if status != "VERIFIED" || !lowercase_digest(evidence_digest) {
            return Err("mandatory claim was not VERIFIED".to_owned());
        }
        claims.push(ClaimResult {
            id: id.to_owned(),
            status: status.to_owned(),
            evidence_digest: evidence_digest.to_owned(),
        });
    }
    if claims.is_empty() {
        return Err("verification returned no completion claims".to_owned());
    }
    Ok(MissionRunResult {
        status: "VERIFIED".to_owned(),
        mission_id: mission_id.to_owned(),
        sequence,
        source_digest: source_digest.to_owned(),
        ledger_head: ledger_head.to_owned(),
        artifact_directory: artifact_directory.display().to_string(),
        claims,
        tamper_verdict: tamper_verdict.to_owned(),
    })
}

fn run_witnessed_mission_blocking() -> Result<MissionRunResult, String> {
    let root = project_root()?;
    let output_directory = root.join("receipts/desktop-latest");
    let gate = Command::new(root.join("scripts/run_vertical_slice.sh"))
        .args(["--output"])
        .arg(&output_directory)
        .current_dir(&root)
        .output()
        .map_err(|error| error.to_string())?;
    let gate_stdout = String::from_utf8_lossy(&gate.stdout);
    if !gate.status.success() || !gate_stdout.contains("PASS_NEMESIS_BACKEND_VERTICAL_SLICE") {
        return Err(format!(
            "backend gate failed: {}",
            String::from_utf8_lossy(&gate.stderr)
        ));
    }
    let manifest = Command::new("/usr/bin/python3")
        .arg(root.join("scripts/verify_evidence_bundle.py"))
        .arg(&output_directory)
        .arg("--write")
        .current_dir(&root)
        .output()
        .map_err(|error| error.to_string())?;
    if !manifest.status.success() {
        return Err("evidence manifest generation failed".to_owned());
    }
    let verification: Value = serde_json::from_slice(
        &fs::read(output_directory.join("verification.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let tampered = Command::new(root.join("runtime/target/debug/nemesis-verify"))
        .args(["--receipt"])
        .arg(output_directory.join("receipt-tampered.cose"))
        .args(["--public-key"])
        .arg(output_directory.join("receipt.pub"))
        .output()
        .map_err(|error| error.to_string())?;
    if tampered.status.success() {
        return Err("tampered receipt was accepted".to_owned());
    }
    let tamper_json: Value =
        serde_json::from_slice(&tampered.stdout).map_err(|error| error.to_string())?;
    let tamper_verdict = tamper_json
        .get("verdict")
        .and_then(Value::as_str)
        .ok_or_else(|| "tamper verifier omitted verdict".to_owned())?;
    run_result_from_values(&verification, tamper_verdict, &output_directory)
}

#[tauri::command]
async fn run_witnessed_mission() -> Result<MissionRunResult, String> {
    tauri::async_runtime::spawn_blocking(run_witnessed_mission_blocking)
        .await
        .map_err(|error| error.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            system_status,
            run_witnessed_mission
        ])
        .run(tauri::generate_context!())
        .expect("NEMESIS Desktop runtime failed");
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::run_result_from_values;

    #[test]
    fn maps_verified_backend_evidence_without_inventing_status() {
        let verification = json!({
            "verdict": "VERIFIED",
            "mission_id": "mis_0000000000000000000000",
            "event_sequence": 12,
            "source_digest": "11".repeat(32),
            "ledger_head": "22".repeat(32),
            "claims": [
                {"id": "build", "status": "VERIFIED", "evidence_digest": "33".repeat(32)},
                {"id": "tests", "status": "VERIFIED", "evidence_digest": "44".repeat(32)}
            ]
        });
        let result = run_result_from_values(
            &verification,
            "REJECTED",
            Path::new("/tmp/evidence"),
        )
        .unwrap();
        assert_eq!(result.status, "VERIFIED");
        assert_eq!(result.claims.len(), 2);
        assert_eq!(result.tamper_verdict, "REJECTED");
    }
}
