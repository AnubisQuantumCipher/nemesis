use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use thiserror::Error;
mod adapters;
mod automation;
mod context;
mod evidence;
mod extensions;
mod git_workflows;
mod scheduler;

pub use adapters::{
    AdapterAvailability, AdapterError, AdapterProfile, AuthorityFingerprint, ConsequenceCeiling,
    GenericSubprocessAdapter, ProviderKind, SelectedAdapter, UsageBudget, UsageExceeded,
    UsageLedger, UsageRecord, WorkerRole, choose_fallback,
};
pub use automation::{
    AutomationCheckpoint, AutomationDecision, AutomationError, AutomationOperation,
    AutomationPolicy, AutomationQueue, AutomationRequest, AutomationStatus, DispatchResult,
    TriggerKind,
};
pub use context::{
    ContextCapsule, ContextCompiler, ContextError, ContextRequest, IndexedFile, KnowledgeBase,
    KnowledgeEntry, KnowledgeStatus, SignedContextCapsule, index_repository,
    verify_context_capsule,
};
pub use evidence::{
    AdaReplay, CausalGraph, EvidenceClass, EvidenceError, EvidenceRecord, EvidenceStore,
    EvidenceVerdict, GraphError, GraphNodeKind, LedgerEvent, MissionFork, ReplayError,
    VerifierRegistry, fork_replay,
};
pub use extensions::{
    ExtensionError, McpGateway, McpInvocation, McpToolDefinition, PluginCapabilities, PluginKind,
    PluginRegistry, SignedPluginManifest, SkillRecord, SkillStatus, UiPluginBroker,
    UnsignedPluginManifest, WasiPluginHost,
};
pub use git_workflows::{
    CiCheckReceipt, CommitReceipt, GitAuthority, GitDecision, GitOperation, GitPolicy, GitWorkflow,
    GitWorkflowError,
};
pub use scheduler::{
    Assignment, LaneId, PatchCandidate, PatchConflict, SchedulePlan, SchedulerDecision,
    SchedulerError, SchedulerLimits, TaskId, TaskSpec, WorkerCandidate, WorkerId,
    detect_patch_conflicts, evidence_is_independent, schedule,
};

const GIT: &str = "/usr/bin/git";
const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("invalid repository or lane path: {0}")]
    InvalidPath(String),
    #[error("invalid lane branch: {0}")]
    InvalidBranch(String),
    #[error("lane already exists: {0}")]
    LaneExists(String),
    #[error("git command failed: {0}")]
    GitFailed(String),
    #[error("sandbox profile cannot represent path: {0}")]
    InvalidSandboxPath(String),
    #[error("sandbox command failed to start: {0}")]
    SandboxStart(#[source] std::io::Error),
    #[error("worker output would exceed limit {limit} bytes (attempted {attempted})")]
    OutputLimitExceeded { attempted: usize, limit: usize },
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneInfo {
    pub root: PathBuf,
    pub branch: String,
    pub base_revision: String,
}

pub struct GitWorktreeManager;

impl GitWorktreeManager {
    pub fn create(repo: &Path, lane: &Path, branch: &str) -> Result<LaneInfo, RuntimeError> {
        let repo = repo
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(repo.display().to_string()))?;
        if !repo.join(".git").exists() {
            return Err(RuntimeError::InvalidPath(repo.display().to_string()));
        }
        if !valid_branch(branch) {
            return Err(RuntimeError::InvalidBranch(branch.to_owned()));
        }
        if lane.exists() {
            return Err(RuntimeError::LaneExists(lane.display().to_string()));
        }
        let parent = lane
            .parent()
            .ok_or_else(|| RuntimeError::InvalidPath(lane.display().to_string()))?;
        fs::create_dir_all(parent)?;
        let parent = parent
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(parent.display().to_string()))?;
        let file_name = lane
            .file_name()
            .ok_or_else(|| RuntimeError::InvalidPath(lane.display().to_string()))?;
        let lane = parent.join(file_name);

        let base_revision = git_output(&repo, ["rev-parse", "HEAD"])?;
        let output = Command::new(GIT)
            .current_dir(&repo)
            .args(["worktree", "add", "-b", branch])
            .arg(&lane)
            .arg(&base_revision)
            .output()?;
        if !output.status.success() {
            return Err(RuntimeError::GitFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        let root = lane
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(lane.display().to_string()))?;
        Ok(LaneInfo {
            root,
            branch: branch.to_owned(),
            base_revision,
        })
    }
}

fn valid_branch(branch: &str) -> bool {
    branch.starts_with("nemesis/")
        && branch.len() <= 128
        && !branch.contains("..")
        && !branch.ends_with('/')
        && branch
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
}

fn git_output<'a>(
    repo: &Path,
    args: impl IntoIterator<Item = &'a str>,
) -> Result<String, RuntimeError> {
    let output = Command::new(GIT).current_dir(repo).args(args).output()?;
    if !output.status.success() {
        return Err(RuntimeError::GitFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| RuntimeError::GitFailed(error.to_string()))
}

#[derive(Clone, Debug)]
pub struct SandboxProfile {
    source: String,
}

impl SandboxProfile {
    pub fn workspace_safe(lane: &Path, executable: &Path) -> Result<Self, RuntimeError> {
        let lane = lane
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(lane.display().to_string()))?;
        if !lane.is_dir() {
            return Err(RuntimeError::InvalidPath(lane.display().to_string()));
        }
        let executable = executable
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(executable.display().to_string()))?;
        if !executable.is_file() {
            return Err(RuntimeError::InvalidPath(executable.display().to_string()));
        }
        let lane = sandbox_quote(&lane)?;
        let executable = sandbox_quote(&executable)?;
        let source = format!(
            "(version 1)\n\
             (deny default)\n\
             (deny network*)\n\
             (allow sysctl-read)\n\
             (allow file-read-data (literal \"/\"))\n\
             (allow signal (target self))\n\
             (allow process-fork)\n\
             (allow process-exec (literal \"{executable}\"))\n\
             (allow file-read*\n\
               (subpath \"/System\")\n\
               (subpath \"/usr/lib\")\n\
               (subpath \"/usr/share\")\n\
               (subpath \"/private/var/db/timezone\")\n\
               (literal \"/dev/null\")\n\
               (literal \"{executable}\")\n\
               (subpath \"{lane}\"))\n\
             (allow file-write*\n\
               (literal \"/dev/null\")\n\
               (subpath \"{lane}\"))\n"
        );
        Ok(Self { source })
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

fn sandbox_quote(path: &Path) -> Result<String, RuntimeError> {
    let value = path
        .to_str()
        .ok_or_else(|| RuntimeError::InvalidSandboxPath(path.display().to_string()))?;
    if value.contains(['\0', '\n', '\r']) {
        return Err(RuntimeError::InvalidSandboxPath(value.to_owned()));
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[derive(Clone, Debug)]
pub struct SandboxedCommand {
    executable: PathBuf,
    lane: PathBuf,
}

impl SandboxedCommand {
    pub fn new(executable: &Path, lane: &Path) -> Self {
        Self {
            executable: executable.to_path_buf(),
            lane: lane.to_path_buf(),
        }
    }

    fn command(&self, args: &[&OsStr]) -> Result<Command, RuntimeError> {
        if !Path::new(SANDBOX_EXEC).is_file() {
            return Err(RuntimeError::InvalidPath(SANDBOX_EXEC.to_owned()));
        }
        let lane = self
            .lane
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(self.lane.display().to_string()))?;
        let executable = self
            .executable
            .canonicalize()
            .map_err(|_| RuntimeError::InvalidPath(self.executable.display().to_string()))?;
        let profile = SandboxProfile::workspace_safe(&lane, &executable)?;
        let mut command = Command::new(SANDBOX_EXEC);
        command
            .arg("-p")
            .arg(profile.source())
            .arg(&executable)
            .args(args.iter().map(OsString::from))
            .current_dir(&lane)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C");
        Ok(command)
    }

    pub fn run_capture(&self, args: &[&OsStr]) -> Result<Output, RuntimeError> {
        self.command(args)?
            .output()
            .map_err(RuntimeError::SandboxStart)
    }

    pub fn spawn(&self, args: &[&OsStr]) -> Result<SandboxedChild, RuntimeError> {
        let child = self
            .command(args)?
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(RuntimeError::SandboxStart)?;
        Ok(SandboxedChild { child })
    }
}

pub struct SandboxedChild {
    child: Child,
}

impl SandboxedChild {
    pub fn cancel(&mut self) -> Result<(), RuntimeError> {
        self.child.kill().map_err(RuntimeError::SandboxStart)
    }

    pub fn wait_with_output(self) -> Result<Output, RuntimeError> {
        self.child
            .wait_with_output()
            .map_err(RuntimeError::SandboxStart)
    }
}

#[derive(Clone, Debug)]
pub struct WorkerOutputAccumulator {
    limit: usize,
    bytes: Vec<u8>,
}

impl WorkerOutputAccumulator {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            bytes: Vec::new(),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<(), RuntimeError> {
        let attempted = self.bytes.len().saturating_add(chunk.len());
        if attempted > self.limit {
            return Err(RuntimeError::OutputLimitExceeded {
                attempted,
                limit: self.limit,
            });
        }
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
