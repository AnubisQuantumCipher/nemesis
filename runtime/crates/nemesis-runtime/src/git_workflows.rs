use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GitAuthority {
    pub create_branch: bool,
    pub commit: bool,
    pub push: bool,
    pub merge: bool,
    pub delete_branch: bool,
    pub delete_worktree: bool,
    pub delete_remote: bool,
}

impl GitAuthority {
    pub fn local_commit_only() -> Self {
        Self {
            create_branch: true,
            commit: true,
            push: false,
            merge: false,
            delete_branch: false,
            delete_worktree: false,
            delete_remote: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitOperation {
    Observe,
    CreateBranch(String),
    Commit,
    Push,
    Merge,
    DeleteBranch(String),
    DeleteWorktree(String),
    DeleteRemote(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitDecision {
    Authorized,
    Refused,
}

#[derive(Clone, Debug)]
pub struct GitPolicy {
    default_branch: String,
    authority: GitAuthority,
    owned_worktrees: BTreeSet<String>,
}

impl GitPolicy {
    pub fn new(default_branch: &str, authority: GitAuthority) -> Self {
        Self {
            default_branch: default_branch.to_owned(),
            authority,
            owned_worktrees: BTreeSet::new(),
        }
    }

    pub fn with_owned_worktree(mut self, path: &Path) -> Self {
        self.owned_worktrees.insert(path.display().to_string());
        self
    }

    pub fn decide(&self, operation: &GitOperation) -> GitDecision {
        let authorized = match operation {
            GitOperation::Observe => true,
            GitOperation::CreateBranch(branch) => {
                self.authority.create_branch && valid_lane_branch(branch)
            }
            GitOperation::Commit => self.authority.commit,
            GitOperation::Push => self.authority.push,
            GitOperation::Merge => self.authority.merge,
            GitOperation::DeleteBranch(branch) => {
                self.authority.delete_branch
                    && branch != &self.default_branch
                    && valid_lane_branch(branch)
            }
            GitOperation::DeleteWorktree(path) => {
                self.authority.delete_worktree && self.owned_worktrees.contains(path)
            }
            GitOperation::DeleteRemote(_) => self.authority.delete_remote,
        };
        if authorized {
            GitDecision::Authorized
        } else {
            GitDecision::Refused
        }
    }
}

fn valid_lane_branch(branch: &str) -> bool {
    branch.starts_with("nemesis/")
        && branch.len() <= 128
        && !branch.contains("..")
        && !branch.ends_with('/')
        && branch
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
}

#[derive(Debug, Error)]
pub enum GitWorkflowError {
    #[error("Git operation is not authorized")]
    Refused,
    #[error("repository, worktree, branch, or path is invalid")]
    InvalidBoundary,
    #[error("Git command failed: {0}")]
    Command(String),
    #[error("default branch changed during lane operation")]
    DefaultBranchChanged,
    #[error("staged paths differ from the exact request")]
    StagedPathMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitReceipt {
    pub branch: String,
    pub base_revision: String,
    pub commit: String,
    pub default_head_before: String,
    pub default_head_after: String,
    pub paths: Vec<String>,
    pub diff_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiCheckReceipt {
    pub check_name: String,
    pub source_digest: [u8; 32],
    pub conclusion: String,
    pub evidence_digest: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct GitWorkflow {
    repository: PathBuf,
    default_branch: String,
}

impl GitWorkflow {
    pub fn new(repository: &Path, default_branch: &str) -> Result<Self, GitWorkflowError> {
        let repository = repository
            .canonicalize()
            .map_err(|_| GitWorkflowError::InvalidBoundary)?;
        if !repository.join(".git").exists() || default_branch.is_empty() {
            return Err(GitWorkflowError::InvalidBoundary);
        }
        let workflow = Self {
            repository,
            default_branch: default_branch.to_owned(),
        };
        workflow.git(
            &workflow.repository,
            &["rev-parse", "--verify", default_branch],
        )?;
        Ok(workflow)
    }

    pub fn validate_commit_paths(&self, paths: &[String]) -> Result<Vec<String>, GitWorkflowError> {
        if paths.is_empty() || paths.len() > 1_024 {
            return Err(GitWorkflowError::InvalidBoundary);
        }
        let mut normalized = BTreeSet::new();
        for path in paths {
            let candidate = Path::new(path);
            if path.is_empty()
                || candidate.is_absolute()
                || candidate
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
                || candidate
                    .components()
                    .next()
                    .is_some_and(|component| component.as_os_str() == ".git")
                || !normalized.insert(path.clone())
            {
                return Err(GitWorkflowError::InvalidBoundary);
            }
        }
        Ok(normalized.into_iter().collect())
    }

    pub fn commit_lane(
        &self,
        lane: &Path,
        expected_branch: &str,
        paths: &[String],
        message: &str,
        authority: &GitAuthority,
    ) -> Result<CommitReceipt, GitWorkflowError> {
        if !authority.commit || !valid_lane_branch(expected_branch) || message.trim().is_empty() {
            return Err(GitWorkflowError::Refused);
        }
        let lane = lane
            .canonicalize()
            .map_err(|_| GitWorkflowError::InvalidBoundary)?;
        let common_dir_output = self.git(&lane, &["rev-parse", "--git-common-dir"])?;
        let common_dir = {
            let path = PathBuf::from(String::from_utf8_lossy(&common_dir_output.stdout).trim());
            if path.is_absolute() {
                path
            } else {
                lane.join(path)
            }
        }
        .canonicalize()
        .map_err(|_| GitWorkflowError::InvalidBoundary)?;
        let repository_git = self
            .repository
            .join(".git")
            .canonicalize()
            .map_err(|_| GitWorkflowError::InvalidBoundary)?;
        if common_dir != repository_git {
            return Err(GitWorkflowError::InvalidBoundary);
        }
        let branch = self.git_text(&lane, &["branch", "--show-current"])?;
        if branch != expected_branch {
            return Err(GitWorkflowError::InvalidBoundary);
        }
        let exact_paths = self.validate_commit_paths(paths)?;
        let default_head_before =
            self.git_text(&self.repository, &["rev-parse", &self.default_branch])?;
        let base_revision = self.git_text(&lane, &["rev-parse", "HEAD"])?;

        let mut add = Command::new("/usr/bin/git");
        add.current_dir(&lane).args(["add", "--"]);
        add.args(&exact_paths);
        Self::checked(
            add.output()
                .map_err(|error| GitWorkflowError::Command(error.to_string()))?,
        )?;
        self.git(&lane, &["diff", "--cached", "--check"])?;
        let staged_output = self.git(&lane, &["diff", "--cached", "--name-only", "-z"])?;
        let staged: BTreeSet<_> = staged_output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect();
        if staged != exact_paths.iter().cloned().collect() {
            return Err(GitWorkflowError::StagedPathMismatch);
        }
        let diff = self.git(&lane, &["diff", "--cached", "--binary"])?;
        let diff_digest: [u8; 32] = Sha256::digest(&diff.stdout).into();
        let commit = Command::new("/usr/bin/git")
            .current_dir(&lane)
            .args([
                "-c",
                "user.name=NEMESIS Local",
                "-c",
                "user.email=nemesis-local@invalid",
                "commit",
                "-m",
                message,
            ])
            .output()
            .map_err(|error| GitWorkflowError::Command(error.to_string()))?;
        Self::checked(commit)?;
        let commit = self.git_text(&lane, &["rev-parse", "HEAD"])?;
        let default_head_after =
            self.git_text(&self.repository, &["rev-parse", &self.default_branch])?;
        if default_head_before != default_head_after {
            return Err(GitWorkflowError::DefaultBranchChanged);
        }
        Ok(CommitReceipt {
            branch,
            base_revision,
            commit,
            default_head_before,
            default_head_after,
            paths: exact_paths,
            diff_digest,
        })
    }

    pub fn accept_ci(receipt: &CiCheckReceipt, current_source: [u8; 32]) -> bool {
        valid_ci_name(&receipt.check_name)
            && receipt.source_digest == current_source
            && receipt.conclusion == "success"
            && receipt.evidence_digest != [0; 32]
    }

    fn git(&self, directory: &Path, arguments: &[&str]) -> Result<Output, GitWorkflowError> {
        let output = Command::new("/usr/bin/git")
            .current_dir(directory)
            .args(arguments)
            .output()
            .map_err(|error| GitWorkflowError::Command(error.to_string()))?;
        Self::checked(output)
    }

    fn git_text(&self, directory: &Path, arguments: &[&str]) -> Result<String, GitWorkflowError> {
        let output = self.git(directory, arguments)?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn checked(output: Output) -> Result<Output, GitWorkflowError> {
        if output.status.success() {
            Ok(output)
        } else {
            Err(GitWorkflowError::Command(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }
}

fn valid_ci_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}
