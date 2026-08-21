use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use nemesis_protocol::{ProtocolError, WorkerEnvelope, parse_worker_message};
use thiserror::Error;

use crate::{RuntimeError, SandboxedCommand, WorkerOutputAccumulator};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    GenericSubprocess,
    Codex,
    ClaudeCode,
    OpenAiApi,
    AnthropicApi,
    LocalModel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkerRole {
    Planner,
    Builder,
    Reviewer,
    RedTeam,
    Verifier,
    Integrator,
    Recovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsequenceCeiling {
    Informational,
    Advisory,
    DecisionBoundary,
    SafetyCritical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorityFingerprint(pub [u8; 32]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterAvailability {
    Available { version: String },
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AdapterSource {
    Static(AdapterAvailability),
    Executable(PathBuf),
    ApiCapabilities { network: bool, secret: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterProfile {
    pub provider: ProviderKind,
    pub roles: Vec<WorkerRole>,
    pub consequence_ceiling: ConsequenceCeiling,
    source: AdapterSource,
}

impl AdapterProfile {
    pub fn subprocess(
        provider: ProviderKind,
        executable: PathBuf,
        roles: Vec<WorkerRole>,
        consequence_ceiling: ConsequenceCeiling,
    ) -> Self {
        Self {
            provider,
            roles,
            consequence_ceiling,
            source: AdapterSource::Executable(executable),
        }
    }

    pub fn available(
        provider: ProviderKind,
        roles: Vec<WorkerRole>,
        consequence_ceiling: ConsequenceCeiling,
        version: &str,
    ) -> Self {
        Self {
            provider,
            roles,
            consequence_ceiling,
            source: AdapterSource::Static(AdapterAvailability::Available {
                version: version.to_owned(),
            }),
        }
    }

    pub fn unavailable(
        provider: ProviderKind,
        roles: Vec<WorkerRole>,
        consequence_ceiling: ConsequenceCeiling,
        reason: &str,
    ) -> Self {
        Self {
            provider,
            roles,
            consequence_ceiling,
            source: AdapterSource::Static(AdapterAvailability::Unavailable {
                reason: reason.to_owned(),
            }),
        }
    }

    pub fn api(
        provider: ProviderKind,
        roles: Vec<WorkerRole>,
        consequence_ceiling: ConsequenceCeiling,
        network_capability: bool,
        secret_capability: bool,
    ) -> Self {
        Self {
            provider,
            roles,
            consequence_ceiling,
            source: AdapterSource::ApiCapabilities {
                network: network_capability,
                secret: secret_capability,
            },
        }
    }

    pub fn availability(&self) -> AdapterAvailability {
        match &self.source {
            AdapterSource::Static(availability) => availability.clone(),
            AdapterSource::Executable(path) => match path.metadata() {
                Ok(metadata)
                    if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 =>
                {
                    AdapterAvailability::Available {
                        version: format!("executable:{}", path.display()),
                    }
                }
                Ok(_) => AdapterAvailability::Unavailable {
                    reason: "configured path is not executable".to_owned(),
                },
                Err(error) => AdapterAvailability::Unavailable {
                    reason: format!("configured executable unavailable: {error}"),
                },
            },
            AdapterSource::ApiCapabilities { network, secret } => {
                if *network && *secret {
                    AdapterAvailability::Available {
                        version: "configured-scoped-api".to_owned(),
                    }
                } else {
                    AdapterAvailability::Unavailable {
                        reason: "API adapter requires explicit network and secret capabilities"
                            .to_owned(),
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedAdapter {
    pub provider: ProviderKind,
    pub role: WorkerRole,
    pub consequence: ConsequenceCeiling,
    pub authority: AuthorityFingerprint,
}

pub fn choose_fallback(
    candidates: &[AdapterProfile],
    role: WorkerRole,
    consequence: ConsequenceCeiling,
    authority: AuthorityFingerprint,
) -> Option<SelectedAdapter> {
    candidates.iter().find_map(|profile| {
        let compatible = profile.roles.contains(&role)
            && profile.consequence_ceiling >= consequence
            && matches!(
                profile.availability(),
                AdapterAvailability::Available { .. }
            );
        compatible.then_some(SelectedAdapter {
            provider: profile.provider,
            role,
            consequence,
            authority,
        })
    })
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("runtime refused adapter execution: {0}")]
    Runtime(#[from] RuntimeError),
    #[error("worker process failed: {0}")]
    Process(String),
    #[error("worker emitted invalid protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("worker emitted no protocol records")]
    EmptyOutput,
}

#[derive(Clone, Debug)]
pub struct GenericSubprocessAdapter {
    executable: PathBuf,
}

impl GenericSubprocessAdapter {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }

    pub fn run(
        &self,
        lane: &Path,
        arguments: &[OsString],
        maximum_output_bytes: usize,
    ) -> Result<Vec<WorkerEnvelope>, AdapterError> {
        let argument_refs: Vec<_> = arguments.iter().map(OsString::as_os_str).collect();
        let output = SandboxedCommand::new(&self.executable, lane).run_capture(&argument_refs)?;
        let mut bounded = WorkerOutputAccumulator::new(maximum_output_bytes);
        bounded.push(&output.stdout)?;
        bounded.push(&output.stderr)?;
        if !output.status.success() {
            return Err(AdapterError::Process(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        let mut messages = Vec::new();
        for line in output.stdout.split(|byte| *byte == b'\n') {
            if !line.is_empty() {
                messages.push(parse_worker_message(line)?);
            }
        }
        if messages.is_empty() {
            return Err(AdapterError::EmptyOutput);
        }
        Ok(messages)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UsageRecord {
    pub cost_microunits: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_tokens: u64,
}

pub type UsageBudget = UsageRecord;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageLedger {
    budget: UsageBudget,
    used: UsageRecord,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("worker usage exceeds the mission allocation")]
pub struct UsageExceeded;

impl UsageLedger {
    pub fn new(budget: UsageBudget) -> Self {
        Self {
            budget,
            used: UsageRecord::default(),
        }
    }

    pub fn record(&mut self, record: UsageRecord) -> Result<(), UsageExceeded> {
        let next = UsageRecord {
            cost_microunits: self
                .used
                .cost_microunits
                .checked_add(record.cost_microunits)
                .ok_or(UsageExceeded)?,
            input_tokens: self
                .used
                .input_tokens
                .checked_add(record.input_tokens)
                .ok_or(UsageExceeded)?,
            output_tokens: self
                .used
                .output_tokens
                .checked_add(record.output_tokens)
                .ok_or(UsageExceeded)?,
            context_tokens: self
                .used
                .context_tokens
                .checked_add(record.context_tokens)
                .ok_or(UsageExceeded)?,
        };
        if next.cost_microunits > self.budget.cost_microunits
            || next.input_tokens > self.budget.input_tokens
            || next.output_tokens > self.budget.output_tokens
            || next.context_tokens > self.budget.context_tokens
        {
            return Err(UsageExceeded);
        }
        self.used = next;
        Ok(())
    }

    pub fn used(&self) -> UsageRecord {
        self.used
    }
}
