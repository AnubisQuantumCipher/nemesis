use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::Manager;

const CONTRACT_SHA256: &str = "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779";

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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplayEventView {
    sequence: u64,
    kind_code: u8,
    state_code: u8,
    event_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplayView {
    verdict: String,
    final_state_code: u8,
    head: String,
    exact_state_reconstruction: bool,
    exact_model_reexecution: bool,
    events: Vec<ReplayEventView>,
}

fn replay_from_value(value: &Value) -> Result<ReplayView, String> {
    if value.get("verdict").and_then(Value::as_str) != Some("VERIFIED")
        || value
            .get("exact_state_reconstruction")
            .and_then(Value::as_bool)
            != Some(true)
        || value
            .get("exact_model_reexecution")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err("replay assurance fields are invalid".to_owned());
    }
    let final_state = value
        .get("final_state_code")
        .and_then(Value::as_u64)
        .and_then(|state| u8::try_from(state).ok())
        .filter(|state| *state <= 10)
        .ok_or_else(|| "replay final state is invalid".to_owned())?;
    let head = value
        .get("head")
        .and_then(Value::as_str)
        .filter(|digest| lowercase_digest(digest))
        .ok_or_else(|| "replay head is invalid".to_owned())?;
    let values = value
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "replay events are missing".to_owned())?;
    let mut events = Vec::with_capacity(values.len());
    for event in values {
        let sequence = event
            .get("sequence")
            .and_then(Value::as_u64)
            .ok_or_else(|| "replay event sequence is invalid".to_owned())?;
        let kind_code = event
            .get("kind_code")
            .and_then(Value::as_u64)
            .and_then(|kind| u8::try_from(kind).ok())
            .filter(|kind| *kind <= 9)
            .ok_or_else(|| "replay event kind is invalid".to_owned())?;
        let state_code = event
            .get("state_code")
            .and_then(Value::as_u64)
            .and_then(|state| u8::try_from(state).ok())
            .filter(|state| *state <= 10)
            .ok_or_else(|| "replay event state is invalid".to_owned())?;
        let event_hash = event
            .get("event_hash")
            .and_then(Value::as_str)
            .filter(|digest| lowercase_digest(digest))
            .ok_or_else(|| "replay event hash is invalid".to_owned())?;
        events.push(ReplayEventView {
            sequence,
            kind_code,
            state_code,
            event_hash: event_hash.to_owned(),
        });
    }
    if events.is_empty() {
        return Err("replay contains no events".to_owned());
    }
    Ok(ReplayView {
        verdict: "VERIFIED".to_owned(),
        final_state_code: final_state,
        head: head.to_owned(),
        exact_state_reconstruction: true,
        exact_model_reexecution: false,
        events,
    })
}

#[tauri::command]
fn load_replay(handle: tauri::AppHandle) -> Result<ReplayView, String> {
    let path = project_root(&handle)?.join("receipts/phase-11/replay.json");
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if metadata.len() == 0 || metadata.len() > 64 * 1024 * 1024 {
        return Err("replay artifact is empty or oversized".to_owned());
    }
    let value: Value = serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    replay_from_value(&value)
}

fn select_support_root(resource_root: &Path, source_root: &Path) -> Result<PathBuf, String> {
    let bundled_contract =
        resource_root.join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md");
    if bundled_contract.is_file() {
        let bytes = fs::read(&bundled_contract).map_err(|error| error.to_string())?;
        if hex::encode(Sha256::digest(bytes)) != CONTRACT_SHA256 {
            return Err("bundled architect contract digest is invalid".to_owned());
        }
        return resource_root
            .canonicalize()
            .map_err(|error| error.to_string());
    }
    source_root
        .canonicalize()
        .map_err(|error| error.to_string())
}

fn find_development_source_root(start: &Path) -> Result<PathBuf, String> {
    for ancestor in start.ancestors() {
        let contract =
            ancestor.join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md");
        if contract.is_file() {
            let bytes = fs::read(contract).map_err(|error| error.to_string())?;
            if hex::encode(Sha256::digest(bytes)) != CONTRACT_SHA256 {
                return Err("development architect contract digest is invalid".to_owned());
            }
            return ancestor.canonicalize().map_err(|error| error.to_string());
        }
    }
    Err("development source root is unavailable".to_owned())
}

fn project_root(handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource_root = handle
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    if resource_root
        .join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md")
        .is_file()
    {
        return select_support_root(&resource_root, &resource_root);
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let start = executable
        .parent()
        .ok_or_else(|| "desktop executable has no parent directory".to_owned())?;
    let source_root = find_development_source_root(start)?;
    select_support_root(&resource_root, &source_root)
}
fn runtime_binary_directory(root: &Path) -> PathBuf {
    let bundled = root.join("bin");
    if bundled.is_dir() {
        bundled
    } else {
        root.join("runtime/target/debug")
    }
}

fn lowercase_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[tauri::command]
fn system_status(handle: tauri::AppHandle) -> Result<SystemStatus, String> {
    let root = project_root(&handle)?;
    let contract = root.join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md");
    let bytes = fs::read(contract).map_err(|error| error.to_string())?;
    let contract_sha256 = hex::encode(Sha256::digest(bytes));
    let runtime_directory = runtime_binary_directory(&root);
    let core_ready = if runtime_directory == root.join("bin") {
        runtime_directory.join("nemesis_core_daemon").is_file()
    } else {
        root.join("build/bin/nemesis_core_daemon").is_file()
    };
    let runtime_ready = [
        "nemesis-deterministic-worker",
        "nemesis-lane-create",
        "nemesis-signer",
        "nemesis-verify",
        "nemesis-worker-runner",
    ]
    .iter()
    .all(|binary| runtime_directory.join(binary).is_file());
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

fn run_witnessed_mission_blocking(
    root: PathBuf,
    output_directory: PathBuf,
) -> Result<MissionRunResult, String> {
    fs::create_dir_all(&output_directory).map_err(|error| error.to_string())?;
    let runtime_directory = runtime_binary_directory(&root);
    let mut gate_command = Command::new(root.join("scripts/run_vertical_slice.sh"));
    gate_command
        .args(["--output"])
        .arg(&output_directory)
        .current_dir(&root);
    if runtime_directory == root.join("bin") {
        gate_command.env("NEMESIS_PREBUILT_BIN_DIR", &runtime_directory);
    }
    let gate = gate_command.output().map_err(|error| error.to_string())?;
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
        &fs::read(output_directory.join("verification.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let tampered = Command::new(runtime_directory.join("nemesis-verify"))
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
async fn run_witnessed_mission(handle: tauri::AppHandle) -> Result<MissionRunResult, String> {
    let root = project_root(&handle)?;
    let output_directory = handle
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("receipts/desktop-latest");
    tauri::async_runtime::spawn_blocking(move || {
        run_witnessed_mission_blocking(root, output_directory)
    })
    .await
    .map_err(|error| error.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            system_status,
            load_replay,
            run_witnessed_mission
        ])
        .run(tauri::generate_context!())
        .expect("NEMESIS Desktop runtime failed");
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::json;

    use super::{
        find_development_source_root, replay_from_value, run_result_from_values,
        runtime_binary_directory, select_support_root,
    };

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
        let result =
            run_result_from_values(&verification, "REJECTED", Path::new("/tmp/evidence")).unwrap();
        assert_eq!(result.status, "VERIFIED");
        assert_eq!(result.claims.len(), 2);
        assert_eq!(result.tamper_verdict, "REJECTED");
    }

    #[test]
    fn maps_only_exact_verified_replay() {
        let value = json!({
            "verdict": "VERIFIED",
            "final_state_code": 8,
            "head": "aa".repeat(32),
            "exact_state_reconstruction": true,
            "exact_model_reexecution": false,
            "events": [{
                "sequence": 1,
                "kind_code": 0,
                "state_code": 1,
                "event_hash": "bb".repeat(32)
            }]
        });
        let replay = replay_from_value(&value).unwrap();
        assert_eq!(replay.events.len(), 1);
        assert!(!replay.exact_model_reexecution);
    }
    #[test]
    fn selects_bundled_support_root_when_contract_matches() {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let resource_root =
            std::env::temp_dir().join(format!("nemesis-support-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&resource_root);
        let mission_dir = resource_root.join("docs/mission");
        fs::create_dir_all(&mission_dir).unwrap();
        fs::copy(
            source_root.join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
            mission_dir.join("NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
        )
        .unwrap();

        let selected = select_support_root(&resource_root, &source_root).unwrap();

        assert_eq!(selected, resource_root.canonicalize().unwrap());
        fs::remove_dir_all(resource_root).unwrap();
    }
    #[test]
    fn rejects_bundled_support_root_with_wrong_contract() {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let resource_root = std::env::temp_dir().join(format!(
            "nemesis-support-root-invalid-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&resource_root);
        let mission_dir = resource_root.join("docs/mission");
        fs::create_dir_all(&mission_dir).unwrap();
        fs::write(
            mission_dir.join("NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
            b"tampered contract",
        )
        .unwrap();

        let rejected = select_support_root(&resource_root, &source_root).unwrap_err();

        assert_eq!(rejected, "bundled architect contract digest is invalid");
        fs::remove_dir_all(resource_root).unwrap();
    }

    #[test]
    fn packaged_runtime_uses_resource_bin_directory() {
        let resource_root =
            std::env::temp_dir().join(format!("nemesis-runtime-bin-{}", std::process::id()));
        let _ = fs::remove_dir_all(&resource_root);
        fs::create_dir_all(resource_root.join("bin")).unwrap();

        let selected = runtime_binary_directory(&resource_root);

        assert_eq!(selected, resource_root.join("bin"));
        fs::remove_dir_all(resource_root).unwrap();
    }
    #[test]
    fn discovers_development_source_root_from_runtime_ancestors() {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let fixture_root =
            std::env::temp_dir().join(format!("nemesis-source-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&fixture_root);
        let mission_dir = fixture_root.join("docs/mission");
        let nested = fixture_root.join("desktop/src-tauri/target/debug");
        fs::create_dir_all(&mission_dir).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::copy(
            source_root.join("docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
            mission_dir.join("NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
        )
        .unwrap();

        let discovered = find_development_source_root(&nested).unwrap();

        assert_eq!(discovered, fixture_root.canonicalize().unwrap());
        fs::remove_dir_all(fixture_root).unwrap();
    }
    #[test]
    fn rejects_development_source_root_with_wrong_contract() {
        let fixture_root = std::env::temp_dir().join(format!(
            "nemesis-source-root-invalid-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&fixture_root);
        let mission_dir = fixture_root.join("docs/mission");
        let nested = fixture_root.join("desktop/src-tauri/target/debug");
        fs::create_dir_all(&mission_dir).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            mission_dir.join("NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
            b"tampered contract",
        )
        .unwrap();

        let rejected = find_development_source_root(&nested).unwrap_err();

        assert_eq!(rejected, "development architect contract digest is invalid");
        fs::remove_dir_all(fixture_root).unwrap();
    }
}
