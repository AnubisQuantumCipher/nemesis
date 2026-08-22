use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use nemesis_protocol::validate_repository_relative_path;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CONTRACT_BYTES: usize = 64 * 1024;
const MAX_GOAL_BYTES: usize = 1024;
const MAX_WORKSPACE_BYTES: usize = 4096;
const MAX_WRITE_BYTES: u64 = 4096;
const MAX_RUNTIME_SECONDS: u64 = 900;
const MAX_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const HOME_SCHEMA_VERSION: u64 = 1;

#[derive(Debug)]
pub enum ProductionError {
    InvalidContract(String),
    InvalidHome(String),
    Io(std::io::Error),
}

impl fmt::Display for ProductionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract(message) | Self::InvalidHome(message) => {
                formatter.write_str(message)
            }
            Self::Io(error) => write!(formatter, "local storage error: {error}"),
        }
    }
}

impl std::error::Error for ProductionError {}

impl From<std::io::Error> for ProductionError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LocalMissionContract {
    schema: String,
    mission_id: String,
    goal: String,
    workspace: String,
    base_revision: String,
    action: FileAction,
    authority: DeniedAuthority,
    budgets: MissionBudgets,
    completion: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FileAction {
    kind: String,
    relative_path: String,
    expected_sha256: String,
    replacement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeniedAuthority {
    network: bool,
    push: bool,
    publish: bool,
    secrets: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MissionBudgets {
    max_write_bytes: u64,
    max_runtime_seconds: u64,
    max_output_bytes: u64,
}

#[derive(Serialize)]
struct NormalizedAction<'a> {
    kind: &'static str,
    relative_path: &'a str,
    content_digest: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledMission {
    pub mission_id: String,
    pub goal: String,
    pub workspace: String,
    pub base_revision: String,
    pub relative_path: String,
    pub expected_sha256: String,
    pub content_digest: String,
    pub replacement_bytes: u64,
    #[serde(skip_serializing)]
    pub(crate) replacement: String,
    pub contract_digest: String,
    pub action_digest: String,
    pub normalized_contract: String,
    pub capabilities: Vec<String>,
    pub max_write_bytes: u64,
    pub max_runtime_seconds: u64,
    pub max_output_bytes: u64,
}

fn lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn contract_error(message: impl Into<String>) -> ProductionError {
    ProductionError::InvalidContract(message.into())
}

pub fn compile_local_contract(bytes: &[u8]) -> Result<CompiledMission, ProductionError> {
    if bytes.is_empty() || bytes.len() > MAX_CONTRACT_BYTES {
        return Err(contract_error("contract is empty or exceeds 64 KiB"));
    }
    let contract: LocalMissionContract =
        serde_json::from_slice(bytes).map_err(|error| contract_error(error.to_string()))?;
    if contract.schema != "nemesis.desktop-mission/v1" {
        return Err(contract_error("unsupported contract schema"));
    }
    if contract.mission_id.len() != 26
        || !contract.mission_id.starts_with("mis_")
        || !contract.mission_id[4..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(contract_error(
            "missionId must be mis_ plus 22 ASCII alphanumerics",
        ));
    }
    let goal = contract.goal.trim();
    if goal.is_empty() || goal.len() > MAX_GOAL_BYTES {
        return Err(contract_error("goal must contain 1..1024 UTF-8 bytes"));
    }
    if contract.workspace.len() > MAX_WORKSPACE_BYTES
        || !Path::new(&contract.workspace).is_absolute()
        || contract.workspace.as_bytes().contains(&0)
    {
        return Err(contract_error("workspace must be a bounded absolute path"));
    }
    if !lowercase_hex(&contract.base_revision, 40) {
        return Err(contract_error(
            "baseRevision must be lowercase 40-hex Git identity",
        ));
    }
    if contract.action.kind != "replace_utf8" {
        return Err(contract_error("only replace_utf8 actions are supported"));
    }
    validate_repository_relative_path(&contract.action.relative_path)
        .map_err(|error| contract_error(format!("relative path refused: {error}")))?;
    if !lowercase_hex(&contract.action.expected_sha256, 64) {
        return Err(contract_error(
            "expectedSha256 must be lowercase SHA-256 hex",
        ));
    }
    let replacement_bytes = u64::try_from(contract.action.replacement.len())
        .map_err(|_| contract_error("replacement length is unsupported"))?;
    if contract.budgets.max_write_bytes == 0
        || contract.budgets.max_write_bytes > MAX_WRITE_BYTES
        || replacement_bytes > contract.budgets.max_write_bytes
    {
        return Err(contract_error(
            "replacement exceeds the bounded maxWriteBytes budget",
        ));
    }
    if contract.budgets.max_runtime_seconds == 0
        || contract.budgets.max_runtime_seconds > MAX_RUNTIME_SECONDS
    {
        return Err(contract_error("maxRuntimeSeconds must be in 1..900"));
    }
    if contract.budgets.max_output_bytes == 0
        || contract.budgets.max_output_bytes > MAX_OUTPUT_BYTES
    {
        return Err(contract_error("maxOutputBytes must be in 1..16777216"));
    }
    if contract.authority.network
        || contract.authority.push
        || contract.authority.publish
        || contract.authority.secrets
    {
        return Err(contract_error(
            "network, push, publish, and secrets authority must remain denied",
        ));
    }
    if contract.completion != ["git_diff_check", "content_match"] {
        return Err(contract_error(
            "completion must be exactly git_diff_check and content_match",
        ));
    }

    let normalized_contract =
        serde_json::to_string(&contract).map_err(|error| contract_error(error.to_string()))?;
    let contract_digest = hex::encode(Sha256::digest(normalized_contract.as_bytes()));
    let content_digest = hex::encode(Sha256::digest(contract.action.replacement.as_bytes()));
    let normalized_action = serde_json::to_vec(&NormalizedAction {
        kind: "filesystem.modify",
        relative_path: &contract.action.relative_path,
        content_digest: &content_digest,
    })
    .map_err(|error| contract_error(error.to_string()))?;
    let action_digest = hex::encode(Sha256::digest(normalized_action));

    Ok(CompiledMission {
        mission_id: contract.mission_id,
        goal: contract.goal,
        workspace: contract.workspace,
        base_revision: contract.base_revision,
        relative_path: contract.action.relative_path,
        expected_sha256: contract.action.expected_sha256,
        content_digest,
        replacement_bytes,
        replacement: contract.action.replacement,
        contract_digest,
        action_digest,
        normalized_contract,
        capabilities: vec!["filesystem.modify:exact".to_owned()],
        max_write_bytes: contract.budgets.max_write_bytes,
        max_runtime_seconds: contract.budgets.max_runtime_seconds,
        max_output_bytes: contract.budgets.max_output_bytes,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalHomeStatus {
    pub first_launch: bool,
    pub schema_version: u64,
    pub root: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct HomeManifest {
    schema: String,
    schema_version: u64,
}

fn ensure_private_directory(path: &Path) -> Result<(), ProductionError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ProductionError::InvalidHome(format!(
                "local home path is not a private directory: {}",
                path.display()
            )));
        }
    } else {
        fs::create_dir_all(path)?;
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ProductionError> {
    let parent = path
        .parent()
        .ok_or_else(|| ProductionError::InvalidHome("storage path has no parent".to_owned()))?;
    ensure_private_directory(parent)?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ProductionError::InvalidHome("invalid storage filename".to_owned()))?,
        std::process::id()
    ));
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

/// Remove crash-orphaned atomic-write temporaries (`.<name>.tmp-<pid>`) from one
/// directory. Bounded scan; regular files only; symlinks and directories are skipped.
fn sweep_stale_temporaries(directory: &Path) -> Result<(), ProductionError> {
    for entry in fs::read_dir(directory)?.take(4096) {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with('.') {
            continue;
        }
        let Some((_, suffix)) = name.rsplit_once(".tmp-") else {
            continue;
        };
        if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        if entry.file_type()?.is_file() {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

pub fn initialize_local_home(root: &Path) -> Result<LocalHomeStatus, ProductionError> {
    ensure_private_directory(root)?;
    sweep_stale_temporaries(root)?;
    for directory in [
        "drafts", "missions", "lanes", "receipts", "logs", "support", "tmp",
    ] {
        let path = root.join(directory);
        ensure_private_directory(&path)?;
        sweep_stale_temporaries(&path)?;
    }
    let manifest_path = root.join("home.json");
    let first_launch = !manifest_path.exists();
    if first_launch {
        let manifest = HomeManifest {
            schema: "nemesis.local-home/v1".to_owned(),
            schema_version: HOME_SCHEMA_VERSION,
        };
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| ProductionError::InvalidHome(error.to_string()))?;
        let mut terminated = bytes;
        terminated.push(b'\n');
        atomic_write(&manifest_path, &terminated)?;
    } else {
        let metadata = fs::symlink_metadata(&manifest_path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 4096 {
            return Err(ProductionError::InvalidHome(
                "local home manifest is not a bounded regular file".to_owned(),
            ));
        }
        let manifest: HomeManifest = serde_json::from_slice(&fs::read(&manifest_path)?)
            .map_err(|error| ProductionError::InvalidHome(error.to_string()))?;
        if manifest.schema != "nemesis.local-home/v1"
            || manifest.schema_version != HOME_SCHEMA_VERSION
        {
            return Err(ProductionError::InvalidHome(
                "unsupported local home schema".to_owned(),
            ));
        }
    }
    Ok(LocalHomeStatus {
        first_launch,
        schema_version: HOME_SCHEMA_VERSION,
        root: root.to_path_buf(),
    })
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextScale {
    Standard,
    Large,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DesktopSettings {
    pub text_scale: TextScale,
    pub reduce_motion: bool,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            text_scale: TextScale::Standard,
            reduce_motion: true,
        }
    }
}

pub fn load_settings(root: &Path) -> Result<DesktopSettings, ProductionError> {
    let path = root.join("settings.json");
    if !path.exists() {
        return Ok(DesktopSettings::default());
    }
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 4096 {
        return Err(ProductionError::InvalidHome(
            "settings are not a bounded regular file".to_owned(),
        ));
    }
    serde_json::from_slice(&fs::read(path)?)
        .map_err(|error| ProductionError::InvalidHome(error.to_string()))
}

pub fn save_settings(root: &Path, settings: &DesktopSettings) -> Result<(), ProductionError> {
    let mut bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| ProductionError::InvalidHome(error.to_string()))?;
    bytes.push(b'\n');
    atomic_write(&root.join("settings.json"), &bytes)
}
