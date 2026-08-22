use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::production::{CompiledMission, ProductionError, atomic_write, compile_local_contract};

const MAX_CORE_MESSAGE_BYTES: usize = 65_536;
const MAX_RESULT_BYTES: u64 = 4 * 1024 * 1024;
const GIT: &str = "/usr/bin/git";

#[derive(Debug)]
pub enum MissionRunError {
    Refused(String),
    Io(std::io::Error),
    Storage(ProductionError),
}

impl std::fmt::Display for MissionRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(message) => formatter.write_str(message),
            Self::Io(error) => write!(formatter, "mission I/O failed: {error}"),
            Self::Storage(error) => write!(formatter, "mission storage failed: {error}"),
        }
    }
}

impl std::error::Error for MissionRunError {}

impl From<std::io::Error> for MissionRunError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ProductionError> for MissionRunError {
    fn from(error: ProductionError) -> Self {
        Self::Storage(error)
    }
}

fn refused(message: impl Into<String>) -> MissionRunError {
    MissionRunError::Refused(message.into())
}

/// Create a directory (and parents) then enforce private 0700 permissions on it,
/// so the Core home chain self-enforces the documented socket-directory invariant
/// instead of relying on an inherited umask.
fn ensure_private_dir(path: &Path) -> Result<(), MissionRunError> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct MissionCancellation {
    cancelled: Arc<AtomicBool>,
}

impl MissionCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

fn require_not_cancelled(cancellation: &MissionCancellation) -> Result<(), MissionRunError> {
    if cancellation.is_cancelled() {
        Err(refused("mission cancelled"))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MissionExecutables {
    pub daemon: PathBuf,
    pub lane_create: PathBuf,
    pub worker_runner: PathBuf,
    pub worker: PathBuf,
    pub signer: PathBuf,
    pub verifier: PathBuf,
    pub replay: PathBuf,
}

impl MissionExecutables {
    pub fn development(repository_root: &Path) -> Self {
        let runtime = repository_root.join("runtime/target/debug");
        Self {
            daemon: repository_root.join("build/bin/nemesis_core_daemon"),
            lane_create: runtime.join("nemesis-lane-create"),
            worker_runner: runtime.join("nemesis-worker-runner"),
            worker: runtime.join("nemesis-deterministic-worker"),
            signer: runtime.join("nemesis-signer"),
            verifier: runtime.join("nemesis-verify"),
            replay: runtime.join("nemesis-replay"),
        }
    }

    pub fn bundled(resource_root: &Path) -> Self {
        let binaries = resource_root.join("bin");
        Self {
            daemon: binaries.join("nemesis_core_daemon"),
            lane_create: binaries.join("nemesis-lane-create"),
            worker_runner: binaries.join("nemesis-worker-runner"),
            worker: binaries.join("nemesis-deterministic-worker"),
            signer: binaries.join("nemesis-signer"),
            verifier: binaries.join("nemesis-verify"),
            replay: binaries.join("nemesis-replay"),
        }
    }

    fn validate(&self) -> Result<(), MissionRunError> {
        for (name, path) in [
            ("Core daemon", &self.daemon),
            ("lane creator", &self.lane_create),
            ("worker runner", &self.worker_runner),
            ("deterministic worker", &self.worker),
            ("receipt signer", &self.signer),
            ("receipt verifier", &self.verifier),
            ("replay verifier", &self.replay),
        ] {
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                refused(format!("{name} unavailable at {}: {error}", path.display()))
            })?;
            if metadata.file_type().is_symlink() {
                return Err(refused(format!("{name} must not be a symlink")));
            }
            if !metadata.is_file() {
                return Err(refused(format!("{name} is not a regular file")));
            }
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(refused(format!("{name} is not executable")));
            }
        }
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.validate().is_ok()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionProgress {
    pub phase: String,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MissionDraftRequest {
    pub goal: String,
    pub workspace: String,
    pub relative_path: String,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftedMission {
    pub path: String,
    pub compiled: CompiledMission,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MissionClaimResult {
    pub id: String,
    pub status: String,
    pub evidence_digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MissionExecutionResult {
    pub status: String,
    pub mission_id: String,
    pub sequence: u64,
    pub source_digest: String,
    pub ledger_head: String,
    pub artifact_directory: String,
    pub lane_path: String,
    pub claims: Vec<MissionClaimResult>,
    pub tamper_verdict: String,
    pub replay_path: String,
}

fn progress(callback: &mut dyn FnMut(&MissionProgress), phase: &str, detail: &str) {
    callback(&MissionProgress {
        phase: phase.to_owned(),
        detail: detail.to_owned(),
    });
}

struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn read_bounded(mut reader: impl Read, limit: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    reader
        .by_ref()
        .take(u64::try_from(limit).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "process output exceeded mission budget",
        ));
    }
    Ok(bytes)
}

fn sanitize_environment(command: &mut Command) {
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("TMPDIR", "/tmp");
}

fn run_bounded(
    command: &mut Command,
    timeout: Duration,
    cancellation: &MissionCancellation,
    output_limit: usize,
) -> Result<BoundedOutput, MissionRunError> {
    sanitize_environment(command);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| refused("child stdout was unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| refused("child stderr was unavailable"))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, output_limit));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, output_limit));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if cancellation.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(refused("mission cancelled"));
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(refused("mission command exceeded maxRuntimeSeconds"));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| refused("stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| refused("stderr reader panicked"))??;
    Ok(BoundedOutput {
        status,
        stdout,
        stderr,
    })
}

fn checked_output(output: BoundedOutput, name: &str) -> Result<Vec<u8>, MissionRunError> {
    if !output.status.success() {
        return Err(refused(format!(
            "{name} failed with status {:?}: stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}

fn run_exact(
    executable: &Path,
    arguments: &[&OsStr],
    cwd: Option<&Path>,
    timeout: Duration,
    output_limit: usize,
    cancellation: &MissionCancellation,
    name: &str,
) -> Result<Vec<u8>, MissionRunError> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(directory) = cwd {
        command.current_dir(directory);
    }
    checked_output(
        run_bounded(&mut command, timeout, cancellation, output_limit)?,
        name,
    )
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn source_digest(base_revision: &str, relative_path: &str, content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(base_revision.as_bytes());
    hasher.update([0]);
    hasher.update(relative_path.as_bytes());
    hasher.update([0]);
    hasher.update(content);
    hex::encode(hasher.finalize())
}

fn canonical_digest(value: &Value) -> Result<String, MissionRunError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| refused(error.to_string()))
}

fn git_text(
    repo: &Path,
    arguments: &[&OsStr],
    timeout: Duration,
    output_limit: usize,
    cancellation: &MissionCancellation,
) -> Result<String, MissionRunError> {
    let bytes = run_exact(
        Path::new(GIT),
        arguments,
        Some(repo),
        timeout,
        output_limit,
        cancellation,
        "Git command",
    )?;
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|error| refused(format!("Git returned non-UTF-8 output: {error}")))
}

fn inspect_workspace(
    requested_path: &str,
    relative_path: &str,
    max_write_bytes: u64,
    timeout: Duration,
    output_limit: usize,
    cancellation: &MissionCancellation,
) -> Result<(PathBuf, String, Vec<u8>), MissionRunError> {
    let requested = Path::new(requested_path);
    let workspace = requested
        .canonicalize()
        .map_err(|error| refused(format!("workspace unavailable: {error}")))?;
    if fs::symlink_metadata(requested)?.file_type().is_symlink() {
        return Err(refused("workspace must not be a symlink"));
    }
    let top = git_text(
        &workspace,
        &[OsStr::new("rev-parse"), OsStr::new("--show-toplevel")],
        timeout,
        output_limit,
        cancellation,
    )?;
    let canonical_top = Path::new(&top)
        .canonicalize()
        .map_err(|error| refused(format!("Git top-level unavailable: {error}")))?;
    if canonical_top != workspace {
        return Err(refused("workspace must be the canonical Git top-level"));
    }
    let revision = git_text(
        &workspace,
        &[OsStr::new("rev-parse"), OsStr::new("HEAD")],
        timeout,
        output_limit,
        cancellation,
    )?;
    let status = git_text(
        &workspace,
        &[
            OsStr::new("status"),
            OsStr::new("--porcelain=v1"),
            OsStr::new("--untracked-files=all"),
        ],
        timeout,
        output_limit,
        cancellation,
    )?;
    if !status.is_empty() {
        return Err(refused("workspace must be clean before lane creation"));
    }
    let target = workspace.join(relative_path);
    let metadata = fs::symlink_metadata(&target)
        .map_err(|error| refused(format!("target file unavailable: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(refused(
            "target must be an existing regular non-symlink file",
        ));
    }
    if metadata.len() > max_write_bytes {
        return Err(refused("target exceeds maxWriteBytes"));
    }
    let canonical_target = target
        .canonicalize()
        .map_err(|error| refused(format!("target canonicalization failed: {error}")))?;
    if !canonical_target.starts_with(&workspace) {
        return Err(refused("target escapes the canonical workspace"));
    }
    Ok((workspace, revision, fs::read(canonical_target)?))
}

fn validate_workspace(
    mission: &CompiledMission,
    timeout: Duration,
    output_limit: usize,
    cancellation: &MissionCancellation,
) -> Result<(PathBuf, Vec<u8>), MissionRunError> {
    let (workspace, revision, content) = inspect_workspace(
        &mission.workspace,
        &mission.relative_path,
        mission.max_write_bytes,
        timeout,
        output_limit,
        cancellation,
    )?;
    if revision != mission.base_revision {
        return Err(refused("workspace HEAD does not match baseRevision"));
    }
    if sha256(&content) != mission.expected_sha256 {
        return Err(refused("target bytes do not match expectedSha256"));
    }
    Ok((workspace, content))
}

pub fn preflight_local_mission(
    mission: &CompiledMission,
    cancellation: &MissionCancellation,
) -> Result<(), MissionRunError> {
    require_not_cancelled(cancellation)?;
    let timeout = Duration::from_secs(mission.max_runtime_seconds);
    let output_limit = usize::try_from(mission.max_output_bytes)
        .map_err(|_| refused("maxOutputBytes is unsupported on this host"))?;
    validate_workspace(mission, timeout, output_limit, cancellation).map(|_| ())
}

pub fn draft_local_mission(
    home: &Path,
    request: &MissionDraftRequest,
    cancellation: &MissionCancellation,
) -> Result<DraftedMission, MissionRunError> {
    require_not_cancelled(cancellation)?;
    let timeout = Duration::from_secs(300);
    let output_limit = 1024 * 1024;
    let (workspace, revision, content) = inspect_workspace(
        &request.workspace,
        &request.relative_path,
        4_096,
        timeout,
        output_limit,
        cancellation,
    )?;
    let observed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| refused(format!("system clock is before UNIX epoch: {error}")))?
        .as_nanos();
    let identity_seed = serde_json::to_vec(&json!({
        "goal":request.goal,
        "workspace":workspace,
        "baseRevision":revision,
        "relativePath":request.relative_path,
        "replacement":request.replacement,
        "observedAt":observed_at
    }))
    .map_err(|error| refused(error.to_string()))?;
    let mission_identity = sha256(&identity_seed);
    let mission_id = format!("mis_{}", &mission_identity[..22]);
    let contract = json!({
        "schema":"nemesis.desktop-mission/v1",
        "missionId":mission_id,
        "goal":request.goal,
        "workspace":workspace,
        "baseRevision":revision,
        "action":{
            "kind":"replace_utf8",
            "relativePath":request.relative_path,
            "expectedSha256":sha256(&content),
            "replacement":request.replacement
        },
        "authority":{
            "network":false,
            "push":false,
            "publish":false,
            "secrets":false
        },
        "budgets":{
            "maxWriteBytes":4096,
            "maxRuntimeSeconds":300,
            "maxOutputBytes":1048576
        },
        "completion":["git_diff_check","content_match"]
    });
    let mut contract_bytes =
        serde_json::to_vec_pretty(&contract).map_err(|error| refused(error.to_string()))?;
    contract_bytes.push(b'\n');
    let compiled = compile_local_contract(&contract_bytes)?;
    let path = home.join("drafts").join(format!("{mission_id}.json"));
    atomic_write(&path, &contract_bytes)?;
    Ok(DraftedMission {
        path: path.display().to_string(),
        compiled,
    })
}
fn write_lane_file(
    lane: &Path,
    relative_path: &str,
    replacement: &[u8],
) -> Result<(), MissionRunError> {
    let lane = lane
        .canonicalize()
        .map_err(|error| refused(format!("lane canonicalization failed: {error}")))?;
    let target = lane.join(relative_path);
    let metadata = fs::symlink_metadata(&target)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(refused("lane target became a symlink or special file"));
    }
    let canonical_target = target.canonicalize()?;
    if !canonical_target.starts_with(&lane) {
        return Err(refused("lane target escaped after creation"));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&target)?;
    file.write_all(replacement)?;
    file.sync_all()?;
    Ok(())
}

struct CoreProcess {
    executable: PathBuf,
    home: PathBuf,
    child: Option<Child>,
}

impl CoreProcess {
    fn new(executable: &Path, home: &Path) -> Self {
        Self {
            executable: executable.to_path_buf(),
            home: home.to_path_buf(),
            child: None,
        }
    }

    fn socket(&self) -> PathBuf {
        self.home.join("core.sock")
    }

    fn start(&mut self) -> Result<(), MissionRunError> {
        if self.socket().as_os_str().as_bytes().len() > 103 {
            return Err(refused(
                "Core Unix socket path exceeds the macOS sockaddr_un limit",
            ));
        }
        ensure_private_dir(&self.home)?;
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.home.join("daemon.stdout.log"))?;
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.home.join("daemon.stderr.log"))?;
        let mut command = Command::new(&self.executable);
        command
            .arg("--home")
            .arg(&self.home)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        sanitize_environment(&mut command);
        self.child = Some(command.spawn()?);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(status) = self
                .child
                .as_mut()
                .ok_or_else(|| refused("Core child disappeared"))?
                .try_wait()?
            {
                return Err(refused(format!("Core exited before readiness: {status}")));
            }
            if self.socket().exists()
                && self
                    .request(json!({"schema":"nemesis.local/v1","command":"ping"}))
                    .ok()
                    .and_then(|value| {
                        value
                            .get("status")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .as_deref()
                    == Some("OK")
            {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.stop();
        Err(refused("Core Unix socket did not become ready"))
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }

    fn restart(&mut self) -> Result<(), MissionRunError> {
        self.stop();
        self.start()
    }

    fn request(&self, value: Value) -> Result<Value, MissionRunError> {
        let mut bytes = serde_json::to_vec(&value).map_err(|error| refused(error.to_string()))?;
        bytes.push(b'\n');
        if bytes.len() > MAX_CORE_MESSAGE_BYTES {
            return Err(refused("Core request exceeded protocol limit"));
        }
        let mut stream = UnixStream::connect(self.socket())?;
        stream.write_all(&bytes)?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut response = Vec::new();
        stream
            .take(u64::try_from(MAX_CORE_MESSAGE_BYTES).unwrap() + 1)
            .read_to_end(&mut response)?;
        if response.len() > MAX_CORE_MESSAGE_BYTES || !response.ends_with(b"\n") {
            return Err(refused("Core response was oversized or missing LF"));
        }
        serde_json::from_slice(&response).map_err(|error| refused(error.to_string()))
    }
}

impl Drop for CoreProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

fn expect_core(response: Value) -> Result<Value, MissionRunError> {
    if response.get("status").and_then(Value::as_str) != Some("OK") {
        return Err(refused(format!("Core refused request: {response}")));
    }
    Ok(response)
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, MissionRunError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| refused(format!("Core response omitted {key}")))
}

fn required_u64(value: &Value, key: &str) -> Result<u64, MissionRunError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| refused(format!("Core response omitted {key}")))
}

fn validate_result(result: &MissionExecutionResult) -> Result<(), MissionRunError> {
    if result.status != "VERIFIED"
        || result.tamper_verdict != "REJECTED"
        || result.mission_id.len() != 26
        || !result.mission_id.starts_with("mis_")
        || result.claims.is_empty()
        || ![&result.source_digest, &result.ledger_head]
            .iter()
            .all(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            })
        || result.claims.iter().any(|claim| {
            claim.status != "VERIFIED"
                || claim.evidence_digest.len() != 64
                || !claim
                    .evidence_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
    {
        return Err(refused("persisted mission result failed validation"));
    }
    Ok(())
}

pub fn run_local_mission(
    mission: &CompiledMission,
    reviewed_contract_digest: &str,
    reviewed_action_digest: &str,
    home: &Path,
    executables: &MissionExecutables,
    cancellation: &MissionCancellation,
    callback: &mut dyn FnMut(&MissionProgress),
) -> Result<MissionExecutionResult, MissionRunError> {
    if reviewed_contract_digest != mission.contract_digest {
        return Err(refused("reviewed contract digest mismatch"));
    }
    if reviewed_action_digest != mission.action_digest {
        return Err(refused("reviewed action digest mismatch"));
    }
    require_not_cancelled(cancellation)?;
    executables.validate()?;
    let timeout = Duration::from_secs(mission.max_runtime_seconds);
    let output_limit = usize::try_from(mission.max_output_bytes)
        .map_err(|_| refused("maxOutputBytes is unsupported on this host"))?;
    let (workspace, original_content) =
        validate_workspace(mission, timeout, output_limit, cancellation)?;
    progress(
        callback,
        "CONTRACT_VALIDATED",
        "Exact contract and workspace identities match.",
    );

    let mission_root = home.join("missions").join(&mission.mission_id);
    if mission_root.exists() {
        return Err(refused("missionId already exists in the local home"));
    }
    fs::create_dir_all(&mission_root)?;
    let mut contract_bytes = mission.normalized_contract.as_bytes().to_vec();
    contract_bytes.push(b'\n');
    atomic_write(&mission_root.join("contract.json"), &contract_bytes)?;
    let lane = home.join("lanes").join(&mission.mission_id);
    let branch = format!("nemesis/desktop-{}", &mission.mission_id[4..]);
    let lane_output = run_exact(
        &executables.lane_create,
        &[workspace.as_os_str(), lane.as_os_str(), OsStr::new(&branch)],
        None,
        timeout,
        output_limit,
        cancellation,
        "lane creation",
    )?;
    let lane_result: Value =
        serde_json::from_slice(&lane_output).map_err(|error| refused(error.to_string()))?;
    if lane_result.get("status").and_then(Value::as_str) != Some("CREATED")
        || lane_result.get("base_revision").and_then(Value::as_str)
            != Some(mission.base_revision.as_str())
    {
        return Err(refused("lane creator returned contradictory identity"));
    }
    let lane = lane.canonicalize()?;
    progress(
        callback,
        "LANE_CREATED",
        "Isolated Git worktree created from the reviewed base revision.",
    );

    let initial_source = source_digest(
        &mission.base_revision,
        &mission.relative_path,
        &original_content,
    );
    let scope_digest = sha256(lane.to_string_lossy().as_bytes());
    let worker_id = format!("wrk_{}", &mission.mission_id[4..]);
    let core_home = home.join(".core").join(&mission.mission_id[4..16]);
    ensure_private_dir(&home.join(".core"))?;
    ensure_private_dir(&core_home)?;
    let core_owner_path = core_home.join("mission-id.txt");
    if core_owner_path.exists() {
        if fs::read_to_string(&core_owner_path)?.trim() != mission.mission_id {
            return Err(refused("Core home prefix collides with another mission"));
        }
    } else {
        atomic_write(
            &core_owner_path,
            format!("{}\n", mission.mission_id).as_bytes(),
        )?;
    }
    let mut core = CoreProcess::new(&executables.daemon, &core_home);
    core.start()?;
    expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"create",
        "mission_id":mission.mission_id,
        "worker_id":worker_id,
        "contract_digest":mission.contract_digest,
        "scope_digest":scope_digest,
        "source_digest":initial_source
    }))?)?;
    expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"authorize",
        "mission_id":mission.mission_id,
        "contract_digest":mission.contract_digest
    }))?)?;
    expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"create_grant",
        "mission_id":mission.mission_id,
        "grant_id":format!("cap_{}", &mission.mission_id[4..])
    }))?)?;
    expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"create_approval",
        "mission_id":mission.mission_id,
        "approval_id":format!("apr_{}", &mission.mission_id[4..]),
        "action_digest":mission.action_digest
    }))?)?;
    progress(
        callback,
        "APPROVAL_ISSUED",
        "Durable parent grant and one-shot action approval were persisted before execution.",
    );
    expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"run",
        "mission_id":mission.mission_id
    }))?)?;
    progress(
        callback,
        "AUTHORIZED",
        "Core committed exact contract authorization before execution.",
    );

    let worker_output = run_exact(
        &executables.worker_runner,
        &[
            executables.worker.as_os_str(),
            lane.as_os_str(),
            OsStr::new("plan"),
            OsStr::new("--mission-id"),
            OsStr::new(&mission.mission_id),
            OsStr::new("--worker-id"),
            OsStr::new(&worker_id),
            OsStr::new("--path"),
            OsStr::new(&mission.relative_path),
            OsStr::new("--content-digest"),
            OsStr::new(&mission.content_digest),
        ],
        None,
        timeout,
        output_limit,
        cancellation,
        "sandboxed worker proposal",
    )?;
    let proposals: Vec<Value> = worker_output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).map_err(|error| refused(error.to_string())))
        .collect::<Result<_, _>>()?;
    if proposals.len() != 2
        || proposals[0].get("method").and_then(Value::as_str) != Some("heartbeat")
        || proposals[1].get("method").and_then(Value::as_str) != Some("propose_action")
        || proposals[1]
            .get("params")
            .and_then(|params| params.get("relative_path"))
            .and_then(Value::as_str)
            != Some(mission.relative_path.as_str())
        || proposals[1]
            .get("params")
            .and_then(|params| params.get("content_digest"))
            .and_then(Value::as_str)
            != Some(mission.content_digest.as_str())
    {
        return Err(refused(
            "worker proposal did not match the reviewed normalized action",
        ));
    }
    let authorized = expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"authorize_action",
        "mission_id":mission.mission_id,
        "worker_id":worker_id,
        "scope_digest":scope_digest,
        "action_digest":mission.action_digest,
        "estimated_bytes":mission.replacement_bytes
    }))?)?;
    if authorized.get("decision").and_then(Value::as_str) != Some("AUTHORIZED") {
        return Err(refused("Kernel did not authorize the exact action"));
    }
    let sequence_before_restart = required_u64(&authorized, "sequence")?;
    core.restart()?;
    let recovered = expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"inspect",
        "mission_id":mission.mission_id
    }))?)?;
    if required_u64(&recovered, "sequence")? != sequence_before_restart
        || required_string(&recovered, "state")? != "RUNNING"
    {
        return Err(refused(
            "Core recovery did not reproduce acknowledged state",
        ));
    }
    progress(
        callback,
        "RECOVERED",
        "Core restart recovered the exact committed mission state.",
    );

    write_lane_file(
        &lane,
        &mission.relative_path,
        mission.replacement.as_bytes(),
    )?;
    let final_source = source_digest(
        &mission.base_revision,
        &mission.relative_path,
        mission.replacement.as_bytes(),
    );
    expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"action_completed",
        "mission_id":mission.mission_id,
        "source_digest":final_source,
        "action_digest":mission.action_digest
    }))?)?;
    progress(
        callback,
        "ACTION_COMMITTED",
        "Reviewed replacement was written only inside the isolated lane.",
    );

    let diff_output = run_exact(
        Path::new(GIT),
        &[OsStr::new("diff"), OsStr::new("--check")],
        Some(&lane),
        timeout,
        output_limit,
        cancellation,
        "git diff verifier",
    )?;
    let content_output = run_exact(
        &executables.worker_runner,
        &[
            OsStr::new("/bin/cat"),
            lane.as_os_str(),
            OsStr::new(&mission.relative_path),
        ],
        None,
        timeout,
        output_limit,
        cancellation,
        "content verifier",
    )?;
    if content_output != mission.replacement.as_bytes() {
        return Err(refused(
            "content verifier did not observe reviewed replacement bytes",
        ));
    }
    let build_evidence = canonical_digest(&json!({
        "verifier":"/usr/bin/git diff --check",
        "source_digest":final_source,
        "exit_status":0,
        "stdout_digest":sha256(&diff_output),
        "stderr_digest":sha256(&[])
    }))?;
    let test_evidence = canonical_digest(&json!({
        "verifier":"/bin/cat under Workspace Safe",
        "source_digest":final_source,
        "exit_status":0,
        "stdout_digest":sha256(&content_output),
        "stderr_digest":sha256(&[])
    }))?;
    for (claim, evidence) in [("build", &build_evidence), ("tests", &test_evidence)] {
        expect_core(core.request(json!({
            "schema":"nemesis.local/v1",
            "command":"accept_evidence",
            "mission_id":mission.mission_id,
            "claim_id":claim,
            "source_digest":final_source,
            "evidence_digest":evidence
        }))?)?;
    }
    let completed = expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"propose_completion",
        "mission_id":mission.mission_id
    }))?)?;
    if required_string(&completed, "state")? != "COMPLETE" {
        return Err(refused("Kernel did not commit completion"));
    }
    let payload = expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"receipt_payload",
        "mission_id":mission.mission_id
    }))?)?;
    if required_string(&payload, "source_digest")? != final_source
        || payload.get("claims_verified").and_then(Value::as_bool) != Some(true)
    {
        return Err(refused(
            "receipt payload was not bound to final source and claims",
        ));
    }
    progress(
        callback,
        "EVIDENCE_ACCEPTED",
        "Deterministic evidence is current and Kernel committed completion.",
    );

    let evidence_root = mission_root.join("evidence");
    fs::create_dir_all(&evidence_root)?;
    let claims = vec![
        MissionClaimResult {
            id: "build".to_owned(),
            status: "VERIFIED".to_owned(),
            evidence_digest: build_evidence,
        },
        MissionClaimResult {
            id: "tests".to_owned(),
            status: "VERIFIED".to_owned(),
            evidence_digest: test_evidence,
        },
    ];
    let receipt_request = json!({
        "mission_id":mission.mission_id,
        "event_sequence":required_u64(&payload,"event_sequence")?,
        "terminal_state":required_string(&payload,"terminal_state")?,
        "source_digest":final_source,
        "ledger_head":required_string(&payload,"ledger_head")?,
        "claims":claims.iter().map(|claim| json!({
            "id":claim.id,
            "mandatory":true,
            "status":claim.status,
            "evidence_digest":claim.evidence_digest
        })).collect::<Vec<_>>(),
        "residuals":[
            "This receipt covers one bounded local file replacement in an isolated lane.",
            "Model-token replay, public signing, notarization, and third-party correctness are not claimed."
        ]
    });
    let mut request_bytes =
        serde_json::to_vec(&receipt_request).map_err(|error| refused(error.to_string()))?;
    request_bytes.push(b'\n');
    let request_path = evidence_root.join("receipt-request.json");
    let receipt_path = evidence_root.join("receipt.cose");
    let public_key_path = evidence_root.join("receipt.pub");
    atomic_write(&request_path, &request_bytes)?;
    let signer_output = run_exact(
        &executables.signer,
        &[
            OsStr::new("--request"),
            request_path.as_os_str(),
            OsStr::new("--receipt"),
            receipt_path.as_os_str(),
            OsStr::new("--public-key"),
            public_key_path.as_os_str(),
        ],
        None,
        timeout,
        output_limit,
        cancellation,
        "receipt signer",
    )?;
    let signer_result: Value =
        serde_json::from_slice(&signer_output).map_err(|error| refused(error.to_string()))?;
    if signer_result
        .get("private_key_exported")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(refused("signer did not attest private-key non-export"));
    }
    let verification_output = run_exact(
        &executables.verifier,
        &[
            OsStr::new("--receipt"),
            receipt_path.as_os_str(),
            OsStr::new("--public-key"),
            public_key_path.as_os_str(),
        ],
        None,
        timeout,
        output_limit,
        cancellation,
        "receipt verifier",
    )?;
    let verification: Value =
        serde_json::from_slice(&verification_output).map_err(|error| refused(error.to_string()))?;
    if verification.get("verdict").and_then(Value::as_str) != Some("VERIFIED")
        || verification.get("source_digest").and_then(Value::as_str) != Some(final_source.as_str())
    {
        return Err(refused("standalone verifier rejected the final receipt"));
    }
    let mut tampered = fs::read(&receipt_path)?;
    if tampered.is_empty() {
        return Err(refused("receipt is empty"));
    }
    let midpoint = tampered.len() / 2;
    tampered[midpoint] ^= 1;
    let tampered_path = evidence_root.join("receipt-tampered.cose");
    atomic_write(&tampered_path, &tampered)?;
    let mut tamper_command = Command::new(&executables.verifier);
    tamper_command
        .arg("--receipt")
        .arg(&tampered_path)
        .arg("--public-key")
        .arg(&public_key_path);
    let tamper_output = run_bounded(&mut tamper_command, timeout, cancellation, output_limit)?;
    if tamper_output.status.success()
        || serde_json::from_slice::<Value>(&tamper_output.stdout)
            .ok()
            .and_then(|value| {
                value
                    .get("verdict")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .as_deref()
            != Some("REJECTED")
    {
        return Err(refused("one-byte receipt mutation was not rejected"));
    }
    progress(
        callback,
        "RECEIPT_VERIFIED",
        "Signed receipt verified and one-byte mutation rejected.",
    );

    core.restart()?;
    let final_state = expect_core(core.request(json!({
        "schema":"nemesis.local/v1",
        "command":"inspect",
        "mission_id":mission.mission_id
    }))?)?;
    if required_string(&final_state, "state")? != "COMPLETE"
        || required_u64(&final_state, "sequence")? != required_u64(&completed, "sequence")?
    {
        return Err(refused("completed mission did not survive Core restart"));
    }
    let ledger_source = core_home
        .join("missions")
        .join(&mission.mission_id)
        .join("events.ledger");
    let ledger_path = evidence_root.join("events.ledger");
    fs::copy(&ledger_source, &ledger_path)?;
    let replay_output = run_exact(
        &executables.replay,
        &[OsStr::new("--ledger"), ledger_path.as_os_str()],
        None,
        timeout,
        output_limit,
        cancellation,
        "replay verifier",
    )?;
    let replay: Value =
        serde_json::from_slice(&replay_output).map_err(|error| refused(error.to_string()))?;
    if replay.get("verdict").and_then(Value::as_str) != Some("VERIFIED")
        || replay
            .get("exact_state_reconstruction")
            .and_then(Value::as_bool)
            != Some(true)
        || replay
            .get("exact_model_reexecution")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err(refused(
            "authoritative replay assurance was not established",
        ));
    }
    let replay_path = evidence_root.join("replay.json");
    let mut replay_bytes = replay_output;
    if !replay_bytes.ends_with(b"\n") {
        replay_bytes.push(b'\n');
    }
    atomic_write(&replay_path, &replay_bytes)?;
    progress(
        callback,
        "REPLAY_VERIFIED",
        "Authoritative state replay verified; model tokens remain comparative.",
    );

    if git_text(
        &workspace,
        &[OsStr::new("rev-parse"), OsStr::new("HEAD")],
        timeout,
        output_limit,
        cancellation,
    )? != mission.base_revision
        || fs::read(workspace.join(&mission.relative_path))? != original_content
    {
        return Err(refused(
            "canonical workspace changed during isolated mission",
        ));
    }

    let result = MissionExecutionResult {
        status: "VERIFIED".to_owned(),
        mission_id: mission.mission_id.clone(),
        sequence: required_u64(&completed, "sequence")?,
        source_digest: final_source,
        ledger_head: required_string(&payload, "ledger_head")?.to_owned(),
        artifact_directory: evidence_root.display().to_string(),
        lane_path: lane.display().to_string(),
        claims,
        tamper_verdict: "REJECTED".to_owned(),
        replay_path: replay_path.display().to_string(),
    };
    validate_result(&result)?;
    let mut result_bytes =
        serde_json::to_vec_pretty(&result).map_err(|error| refused(error.to_string()))?;
    result_bytes.push(b'\n');
    let result_path = mission_root.join("result.json");
    atomic_write(&result_path, &result_bytes)?;
    let pointer = json!({
        "schema":"nemesis.last-mission/v1",
        "missionId":mission.mission_id,
        "resultPath":format!("missions/{}/result.json",mission.mission_id)
    });
    let mut pointer_bytes =
        serde_json::to_vec_pretty(&pointer).map_err(|error| refused(error.to_string()))?;
    pointer_bytes.push(b'\n');
    atomic_write(&home.join("last-mission.json"), &pointer_bytes)?;
    Ok(result)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LastMissionPointer {
    schema: String,
    mission_id: String,
    result_path: String,
}

pub fn load_last_mission(home: &Path) -> Result<Option<MissionExecutionResult>, MissionRunError> {
    let pointer_path = home.join("last-mission.json");
    if !pointer_path.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(&pointer_path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 4096 {
        return Err(refused(
            "last-mission pointer is not a bounded regular file",
        ));
    }
    let pointer: LastMissionPointer = serde_json::from_slice(&fs::read(&pointer_path)?)
        .map_err(|error| refused(error.to_string()))?;
    if pointer.schema != "nemesis.last-mission/v1"
        || pointer.mission_id.len() != 26
        || !pointer.mission_id.starts_with("mis_")
    {
        return Err(refused("last-mission pointer is invalid"));
    }
    let relative = Path::new(&pointer.result_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(refused("last-mission result path is unsafe"));
    }
    let result_path = home.join(relative);
    if fs::symlink_metadata(&result_path)?.file_type().is_symlink() {
        return Err(refused("last-mission result must not be a symlink"));
    }
    let metadata = fs::metadata(&result_path)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_RESULT_BYTES {
        return Err(refused(
            "last-mission result is empty, oversized, or not regular",
        ));
    }
    let result: MissionExecutionResult = serde_json::from_slice(&fs::read(result_path)?)
        .map_err(|error| refused(error.to_string()))?;
    if result.mission_id != pointer.mission_id {
        return Err(refused("last-mission pointer and result disagree"));
    }
    validate_result(&result)?;
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::ensure_private_dir;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    // ARCH-001 regression: the Core home chain self-enforces private 0700
    // directories (including intermediates), not an inherited umask.
    #[test]
    fn core_home_chain_is_private_0700() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("nemesis-arch001-{}-{nonce}", std::process::id()));
        let core_parent = base.join(".core");
        let core_home = core_parent.join("abcdef123456");

        ensure_private_dir(&core_parent).unwrap();
        ensure_private_dir(&core_home).unwrap();
        // Idempotent second call must not fail or loosen permissions.
        ensure_private_dir(&core_home).unwrap();

        for directory in [&core_parent, &core_home] {
            let metadata = fs::symlink_metadata(directory).unwrap();
            assert!(
                metadata.is_dir(),
                "{} must be a directory",
                directory.display()
            );
            assert_eq!(
                metadata.permissions().mode() & 0o777,
                0o700,
                "{} must be private 0700",
                directory.display()
            );
        }

        let _ = fs::remove_dir_all(&base);
    }
}
