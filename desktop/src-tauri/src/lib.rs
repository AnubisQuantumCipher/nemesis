mod mission_runner;
mod production;

pub use mission_runner::{
    DraftedMission, MissionCancellation, MissionClaimResult, MissionDraftRequest,
    MissionExecutables, MissionExecutionResult, MissionProgress, MissionRunError,
    draft_local_mission, load_last_mission, preflight_local_mission, run_local_mission,
};
pub use production::{
    CompiledMission, DesktopSettings, LocalHomeStatus, ProductionError, TextScale,
    compile_local_contract, initialize_local_home, load_settings, save_settings,
};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::Manager;

const CONTRACT_SHA256: &str = "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779";
const CONTRACT_RELATIVE_PATH: &str =
    "docs/mission/NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md";
const MAX_REPLAY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LOCAL_CONTRACT_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandFailure {
    pub code: String,
    pub message: String,
    pub recovery: String,
}

impl CommandFailure {
    fn new(code: &str, message: impl Into<String>, recovery: &str) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            recovery: recovery.to_owned(),
        }
    }

    fn storage(error: impl std::fmt::Display) -> Self {
        Self::new(
            "BLOCKED_LOCAL_STORAGE",
            error.to_string(),
            "Inspect the local-home path and permissions, then retry. NEMESIS did not continue.",
        )
    }

    fn contract(error: impl std::fmt::Display) -> Self {
        Self::new(
            "REFUSED_CONTRACT",
            error.to_string(),
            "Correct the exact local contract; any byte change requires a new authority review.",
        )
    }

    fn mission(error: impl std::fmt::Display) -> Self {
        let message = error.to_string();
        let (code, recovery) = if message.contains("mission cancelled") {
            (
                "REFUSED_CANCELLED",
                "The cancellation was recorded. Inspect preserved mission evidence before retrying.",
            )
        } else if message.contains("reviewed contract digest mismatch")
            || message.contains("reviewed action digest mismatch")
        {
            (
                "REFUSED_REVIEW_MISMATCH",
                "Recompile and review the changed contract before authorizing it.",
            )
        } else if message.contains("workspace")
            || message.contains("target")
            || message.contains("baseRevision")
        {
            (
                "REFUSED_WORKSPACE",
                "Restore the reviewed clean workspace identity or compile a new contract.",
            )
        } else {
            (
                "BLOCKED_MISSION",
                "Inspect the preserved local mission logs and resolve the named semantic failure.",
            )
        };
        Self::new(code, message, recovery)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub ready: bool,
    pub core: String,
    pub kernel: String,
    pub runtime: String,
    pub contract_sha256: String,
    pub local_home: String,
    pub first_launch: bool,
    pub schema_version: u64,
    pub app_version: String,
    pub sandbox: String,
    pub network: String,
    pub updates: String,
    pub settings: DesktopSettings,
    pub last_mission: Option<MissionExecutionResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayEventView {
    pub sequence: u64,
    pub kind_code: u8,
    pub state_code: u8,
    pub event_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayView {
    pub verdict: String,
    pub final_state_code: u8,
    pub head: String,
    pub exact_state_reconstruction: bool,
    pub exact_model_reexecution: bool,
    pub events: Vec<ReplayEventView>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionRuntimeSnapshot {
    pub running: bool,
    pub phase: String,
    pub detail: String,
    pub last_result: Option<MissionExecutionResult>,
    pub error: Option<CommandFailure>,
}

impl Default for MissionRuntimeSnapshot {
    fn default() -> Self {
        Self {
            running: false,
            phase: "IDLE".to_owned(),
            detail: "No local mission is running.".to_owned(),
            last_result: None,
            error: None,
        }
    }
}

#[derive(Clone, Default)]
struct DesktopController {
    snapshot: Arc<Mutex<MissionRuntimeSnapshot>>,
    cancellation: MissionCancellation,
}

fn lock_snapshot(
    controller: &DesktopController,
) -> Result<std::sync::MutexGuard<'_, MissionRuntimeSnapshot>, CommandFailure> {
    controller.snapshot.lock().map_err(|_| {
        CommandFailure::new(
            "BLOCKED_STATE",
            "Desktop mission state lock is poisoned.",
            "Quit and relaunch NEMESIS Desktop; no new mission was authorized.",
        )
    })
}

fn lowercase_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn replay_from_value(value: &Value) -> Result<ReplayView, CommandFailure> {
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
        return Err(CommandFailure::new(
            "REFUSED_REPLAY",
            "Replay assurance fields are invalid.",
            "Re-run the receipt-bound replay verifier against the current mission ledger.",
        ));
    }
    let final_state = value
        .get("final_state_code")
        .and_then(Value::as_u64)
        .and_then(|state| u8::try_from(state).ok())
        .filter(|state| *state <= 10)
        .ok_or_else(|| {
            CommandFailure::new(
                "REFUSED_REPLAY",
                "Replay final state is invalid.",
                "Inspect the current ledger and replay-verifier output.",
            )
        })?;
    let head = value
        .get("head")
        .and_then(Value::as_str)
        .filter(|digest| lowercase_digest(digest))
        .ok_or_else(|| {
            CommandFailure::new(
                "REFUSED_REPLAY",
                "Replay chain head is invalid.",
                "Inspect the current ledger and replay-verifier output.",
            )
        })?;
    let values = value
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CommandFailure::new(
                "REFUSED_REPLAY",
                "Replay events are missing.",
                "Run a local mission before opening Replay.",
            )
        })?;
    if values.is_empty() {
        return Err(CommandFailure::new(
            "REFUSED_REPLAY",
            "Replay contains no events.",
            "Run a local mission before opening Replay.",
        ));
    }
    let mut events = Vec::with_capacity(values.len());
    for event in values {
        let sequence = event
            .get("sequence")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                CommandFailure::new(
                    "REFUSED_REPLAY",
                    "Replay event sequence is invalid.",
                    "Inspect the current ledger and replay-verifier output.",
                )
            })?;
        let kind_code = event
            .get("kind_code")
            .and_then(Value::as_u64)
            .and_then(|kind| u8::try_from(kind).ok())
            .filter(|kind| *kind <= 9)
            .ok_or_else(|| {
                CommandFailure::new(
                    "REFUSED_REPLAY",
                    "Replay event kind is invalid.",
                    "Inspect the current ledger and replay-verifier output.",
                )
            })?;
        let state_code = event
            .get("state_code")
            .and_then(Value::as_u64)
            .and_then(|state| u8::try_from(state).ok())
            .filter(|state| *state <= 10)
            .ok_or_else(|| {
                CommandFailure::new(
                    "REFUSED_REPLAY",
                    "Replay event state is invalid.",
                    "Inspect the current ledger and replay-verifier output.",
                )
            })?;
        let event_hash = event
            .get("event_hash")
            .and_then(Value::as_str)
            .filter(|digest| lowercase_digest(digest))
            .ok_or_else(|| {
                CommandFailure::new(
                    "REFUSED_REPLAY",
                    "Replay event hash is invalid.",
                    "Inspect the current ledger and replay-verifier output.",
                )
            })?;
        events.push(ReplayEventView {
            sequence,
            kind_code,
            state_code,
            event_hash: event_hash.to_owned(),
        });
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

fn select_support_root(resource_root: &Path, source_root: &Path) -> Result<PathBuf, String> {
    let bundled_contract = resource_root.join(CONTRACT_RELATIVE_PATH);
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
        let contract = ancestor.join(CONTRACT_RELATIVE_PATH);
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

fn project_root(handle: &tauri::AppHandle) -> Result<PathBuf, CommandFailure> {
    let resource_root = handle
        .path()
        .resource_dir()
        .map_err(CommandFailure::storage)?;
    if resource_root.join(CONTRACT_RELATIVE_PATH).is_file() {
        return select_support_root(&resource_root, &resource_root)
            .map_err(CommandFailure::storage);
    }
    let executable = std::env::current_exe().map_err(CommandFailure::storage)?;
    let start = executable.parent().ok_or_else(|| {
        CommandFailure::new(
            "BLOCKED_RESOURCES",
            "Desktop executable has no parent directory.",
            "Reinstall NEMESIS Desktop from the authoritative artifact.",
        )
    })?;
    let source_root = find_development_source_root(start).map_err(CommandFailure::storage)?;
    select_support_root(&resource_root, &source_root).map_err(CommandFailure::storage)
}

fn local_home(handle: &tauri::AppHandle) -> Result<PathBuf, CommandFailure> {
    handle
        .path()
        .app_data_dir()
        .map_err(CommandFailure::storage)
}

fn executables_for(root: &Path) -> MissionExecutables {
    if root.join("bin").is_dir() {
        MissionExecutables::bundled(root)
    } else {
        MissionExecutables::development(root)
    }
}

fn executable_ready(paths: &MissionExecutables) -> bool {
    paths.is_ready()
}

fn read_local_contract(path: &str) -> Result<CompiledMission, CommandFailure> {
    let requested = Path::new(path);
    if !requested.is_absolute() {
        return Err(CommandFailure::contract(
            "Local contract path must be absolute.",
        ));
    }
    let metadata = fs::symlink_metadata(requested).map_err(CommandFailure::contract)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_LOCAL_CONTRACT_BYTES
    {
        return Err(CommandFailure::contract(
            "Local contract must be a non-symlink regular file between 1 byte and 64 KiB.",
        ));
    }
    let bytes = fs::read(requested).map_err(CommandFailure::contract)?;
    compile_local_contract(&bytes).map_err(CommandFailure::contract)
}

#[tauri::command]
fn system_status(handle: tauri::AppHandle) -> Result<SystemStatus, CommandFailure> {
    let root = project_root(&handle)?;
    let contract_bytes =
        fs::read(root.join(CONTRACT_RELATIVE_PATH)).map_err(CommandFailure::storage)?;
    let contract_sha256 = hex::encode(Sha256::digest(contract_bytes));
    let executables = executables_for(&root);
    let runtime_ready = executable_ready(&executables);
    let core_ready = executables.daemon.is_file();
    let contract_ready = contract_sha256 == CONTRACT_SHA256;
    let home = local_home(&handle)?;
    let home_status = initialize_local_home(&home).map_err(CommandFailure::storage)?;
    let settings = load_settings(&home).map_err(CommandFailure::storage)?;
    let last_mission = load_last_mission(&home).map_err(CommandFailure::storage)?;
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
        local_home: home.display().to_string(),
        first_launch: home_status.first_launch,
        schema_version: home_status.schema_version,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        sandbox: if executables.worker_runner.is_file() {
            "WORKSPACE_SAFE_AVAILABLE"
        } else {
            "UNAVAILABLE"
        }
        .to_owned(),
        network: "DENIED_BY_CONTRACT".to_owned(),
        updates: "DISABLED_NO_AUTHENTICATED_UPDATER".to_owned(),
        settings,
        last_mission,
    })
}

#[tauri::command]
fn draft_mission(
    handle: tauri::AppHandle,
    request: MissionDraftRequest,
) -> Result<DraftedMission, CommandFailure> {
    let home = local_home(&handle)?;
    initialize_local_home(&home).map_err(CommandFailure::storage)?;
    draft_local_mission(&home, &request, &MissionCancellation::default())
        .map_err(CommandFailure::mission)
}

#[tauri::command]
fn compile_mission(path: String) -> Result<CompiledMission, CommandFailure> {
    let compiled = read_local_contract(&path)?;
    let cancellation = MissionCancellation::default();
    preflight_local_mission(&compiled, &cancellation).map_err(CommandFailure::mission)?;
    Ok(compiled)
}

#[tauri::command]
async fn run_mission(
    handle: tauri::AppHandle,
    state: tauri::State<'_, DesktopController>,
    path: String,
    reviewed_contract_digest: String,
    reviewed_action_digest: String,
) -> Result<MissionExecutionResult, CommandFailure> {
    let controller = state.inner().clone();
    {
        let mut snapshot = lock_snapshot(&controller)?;
        if snapshot.running {
            return Err(CommandFailure::new(
                "REFUSED_ALREADY_RUNNING",
                "A local mission is already running.",
                "Wait for completion or cancel the active mission before starting another.",
            ));
        }
        controller.cancellation.reset();
        snapshot.running = true;
        snapshot.phase = "STARTING".to_owned();
        snapshot.detail = "Re-reading the exact reviewed local contract.".to_owned();
        snapshot.error = None;
    }
    let root = project_root(&handle)?;
    let home = local_home(&handle)?;
    initialize_local_home(&home).map_err(CommandFailure::storage)?;
    let compiled = match read_local_contract(&path) {
        Ok(compiled) => compiled,
        Err(error) => {
            let mut snapshot = lock_snapshot(&controller)?;
            snapshot.running = false;
            snapshot.phase = "REFUSED".to_owned();
            snapshot.detail = error.message.clone();
            snapshot.error = Some(error.clone());
            return Err(error);
        }
    };
    let executables = executables_for(&root);
    let worker_controller = controller.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        run_local_mission(
            &compiled,
            &reviewed_contract_digest,
            &reviewed_action_digest,
            &home,
            &executables,
            &worker_controller.cancellation,
            &mut |progress| {
                if let Ok(mut snapshot) = worker_controller.snapshot.lock() {
                    snapshot.phase = progress.phase.clone();
                    snapshot.detail = progress.detail.clone();
                }
            },
        )
    })
    .await
    .map_err(|error| {
        CommandFailure::new(
            "BLOCKED_RUNTIME",
            error.to_string(),
            "Quit and relaunch NEMESIS Desktop, then inspect preserved mission evidence.",
        )
    })?;
    match result {
        Ok(result) => {
            let mut snapshot = lock_snapshot(&controller)?;
            snapshot.running = false;
            snapshot.phase = "COMPLETE".to_owned();
            snapshot.detail = "Receipt and authoritative replay are current.".to_owned();
            snapshot.last_result = Some(result.clone());
            snapshot.error = None;
            Ok(result)
        }
        Err(error) => {
            let failure = CommandFailure::mission(error);
            let mut snapshot = lock_snapshot(&controller)?;
            snapshot.running = false;
            snapshot.phase = "REFUSED".to_owned();
            snapshot.detail = failure.message.clone();
            snapshot.error = Some(failure.clone());
            Err(failure)
        }
    }
}

#[tauri::command]
fn mission_status(
    state: tauri::State<'_, DesktopController>,
) -> Result<MissionRuntimeSnapshot, CommandFailure> {
    Ok(lock_snapshot(state.inner())?.clone())
}

#[tauri::command]
fn cancel_mission(
    state: tauri::State<'_, DesktopController>,
) -> Result<MissionRuntimeSnapshot, CommandFailure> {
    let controller = state.inner();
    let mut snapshot = lock_snapshot(controller)?;
    if !snapshot.running {
        return Err(CommandFailure::new(
            "REFUSED_NOT_RUNNING",
            "No local mission is running.",
            "Compile and review a contract before starting a mission.",
        ));
    }
    controller.cancellation.cancel();
    snapshot.phase = "CANCELLING".to_owned();
    snapshot.detail = "Cancellation requested; the current bounded process will stop.".to_owned();
    Ok(snapshot.clone())
}

#[tauri::command]
fn load_replay(handle: tauri::AppHandle) -> Result<ReplayView, CommandFailure> {
    let home = local_home(&handle)?;
    let result = load_last_mission(&home)
        .map_err(CommandFailure::storage)?
        .ok_or_else(|| {
            CommandFailure::new(
                "REFUSED_NO_REPLAY",
                "No completed local mission is available for replay.",
                "Run and verify a local mission first.",
            )
        })?;
    let replay_path = Path::new(&result.replay_path);
    let canonical_home = home.canonicalize().map_err(CommandFailure::storage)?;
    let metadata = fs::symlink_metadata(replay_path).map_err(CommandFailure::storage)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_REPLAY_BYTES
    {
        return Err(CommandFailure::new(
            "REFUSED_REPLAY",
            "Current replay is empty, oversized, a symlink, or not a regular file.",
            "Inspect the mission evidence directory and rerun verification.",
        ));
    }
    let canonical_replay = replay_path
        .canonicalize()
        .map_err(CommandFailure::storage)?;
    if !canonical_replay.starts_with(&canonical_home) {
        return Err(CommandFailure::new(
            "REFUSED_REPLAY",
            "Current replay path escapes the local home.",
            "Inspect the last-mission pointer and local-home integrity.",
        ));
    }
    let value: Value =
        serde_json::from_slice(&fs::read(canonical_replay).map_err(CommandFailure::storage)?)
            .map_err(CommandFailure::storage)?;
    replay_from_value(&value)
}

#[tauri::command]
fn get_settings(handle: tauri::AppHandle) -> Result<DesktopSettings, CommandFailure> {
    let home = local_home(&handle)?;
    initialize_local_home(&home).map_err(CommandFailure::storage)?;
    load_settings(&home).map_err(CommandFailure::storage)
}

#[tauri::command]
fn update_settings(
    handle: tauri::AppHandle,
    settings: DesktopSettings,
) -> Result<DesktopSettings, CommandFailure> {
    let home = local_home(&handle)?;
    initialize_local_home(&home).map_err(CommandFailure::storage)?;
    save_settings(&home, &settings).map_err(CommandFailure::storage)?;
    load_settings(&home).map_err(CommandFailure::storage)
}

pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .menu(tauri::menu::Menu::default)
        .manage(DesktopController::default())
        .invoke_handler(tauri::generate_handler![
            system_status,
            draft_mission,
            compile_mission,
            run_mission,
            mission_status,
            cancel_mission,
            load_replay,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::json;

    use super::{
        CONTRACT_RELATIVE_PATH, find_development_source_root, replay_from_value,
        select_support_root,
    };

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
    fn rejects_empty_or_model_exact_replay() {
        let value = json!({
            "verdict": "VERIFIED",
            "final_state_code": 8,
            "head": "aa".repeat(32),
            "exact_state_reconstruction": true,
            "exact_model_reexecution": true,
            "events": []
        });
        assert_eq!(
            replay_from_value(&value).unwrap_err().code,
            "REFUSED_REPLAY"
        );
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
            source_root.join(CONTRACT_RELATIVE_PATH),
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
            source_root.join(CONTRACT_RELATIVE_PATH),
            mission_dir.join("NEMESIS_DESKTOP_MASTER_BUILD_MISSION_2026-08-20.md"),
        )
        .unwrap();

        let discovered = find_development_source_root(&nested).unwrap();

        assert_eq!(discovered, fixture_root.canonicalize().unwrap());
        fs::remove_dir_all(fixture_root).unwrap();
    }
}
