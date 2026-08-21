use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const WORKER_PROTOCOL_VERSION: &str = "nemesis.worker/v1";
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_PURPOSE_BYTES: usize = 1024;
const MAX_PATH_BYTES: usize = 4096;
const MAX_CLAIMS: usize = 32;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("worker message is {actual} bytes; limit is {limit}")]
    MessageTooLarge { actual: usize, limit: usize },
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("invalid worker envelope: {0}")]
    InvalidEnvelope(String),
    #[error("unsupported worker protocol: {0}")]
    UnsupportedVersion(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("forbidden worker method: {0}")]
    ForbiddenMethod(String),
    #[error("invalid method parameters: {0}")]
    InvalidParams(String),
    #[error("invalid action proposal: {0}")]
    InvalidAction(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoundedId(String);

impl BoundedId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: &str, prefix: &str) -> Result<Self, ProtocolError> {
        if value.len() != 26
            || !value.starts_with(prefix)
            || !value[prefix.len()..]
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        {
            return Err(ProtocolError::InvalidIdentifier(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkerEnvelope {
    pub id: u64,
    pub mission_id: BoundedId,
    pub worker_id: BoundedId,
    pub message: WorkerMessage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum WorkerMessage {
    Heartbeat,
    ProposeAction(ActionProposal),
    EmitClaim(ClaimProposal),
    ReturnArtifact(ArtifactProposal),
    ReportBlocker(BlockerReport),
    ProposeCompletion(CompletionProposal),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    #[serde(rename = "filesystem.modify")]
    FilesystemModify,
    #[serde(rename = "process.execute")]
    ProcessExecute,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionProposal {
    pub action_kind: ActionKind,
    pub relative_path: String,
    pub content_digest: String,
    pub purpose: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimProposal {
    pub claim_id: String,
    pub statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactProposal {
    pub relative_path: String,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockerReport {
    pub code: String,
    pub evidence_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionProposal {
    pub claim_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyParams {}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a str, ProtocolError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::InvalidEnvelope(format!("{field} must be a string")))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

pub fn validate_repository_relative_path(value: &str) -> Result<(), ProtocolError> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value
            .bytes()
            .any(|byte| byte == 0 || byte == b'%' || byte == b'\\' || byte.is_ascii_control())
        || Path::new(value).is_absolute()
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".." | ".git"))
    {
        return Err(ProtocolError::InvalidAction(
            "path must be a bounded, unencoded relative repository path".to_owned(),
        ));
    }
    Ok(())
}

fn deserialize_params<T: for<'de> Deserialize<'de>>(params: Value) -> Result<T, ProtocolError> {
    serde_json::from_value(params).map_err(|error| ProtocolError::InvalidParams(error.to_string()))
}

pub fn parse_worker_message(bytes: &[u8]) -> Result<WorkerEnvelope, ProtocolError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge {
            actual: bytes.len(),
            limit: MAX_MESSAGE_BYTES,
        });
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| ProtocolError::InvalidJson(error.to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| ProtocolError::InvalidEnvelope("top level must be an object".to_owned()))?;

    let expected: BTreeSet<&str> = [
        "jsonrpc",
        "id",
        "protocol",
        "mission_id",
        "worker_id",
        "method",
        "params",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if actual != expected {
        return Err(ProtocolError::InvalidEnvelope(
            "top-level fields must exactly match the v1 envelope".to_owned(),
        ));
    }
    if required_string(object, "jsonrpc")? != "2.0" {
        return Err(ProtocolError::InvalidEnvelope(
            "jsonrpc must equal 2.0".to_owned(),
        ));
    }
    let id = object.get("id").and_then(Value::as_u64).ok_or_else(|| {
        ProtocolError::InvalidEnvelope("id must be an unsigned integer".to_owned())
    })?;
    let protocol = required_string(object, "protocol")?;
    if protocol != WORKER_PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(protocol.to_owned()));
    }
    let mission_id = BoundedId::parse(required_string(object, "mission_id")?, "mis_")?;
    let worker_id = BoundedId::parse(required_string(object, "worker_id")?, "wrk_")?;
    let method = required_string(object, "method")?;
    let params = object
        .get("params")
        .cloned()
        .ok_or_else(|| ProtocolError::InvalidEnvelope("params is required".to_owned()))?;

    let message = match method {
        "heartbeat" => {
            let _: EmptyParams = deserialize_params(params)?;
            WorkerMessage::Heartbeat
        }
        "propose_action" => {
            let action: ActionProposal = deserialize_params(params)?;
            validate_repository_relative_path(&action.relative_path)?;
            if !valid_digest(&action.content_digest) {
                return Err(ProtocolError::InvalidAction(
                    "content_digest must be lowercase SHA-256 hex".to_owned(),
                ));
            }
            if action.purpose.trim().is_empty() || action.purpose.len() > MAX_PURPOSE_BYTES {
                return Err(ProtocolError::InvalidAction(
                    "purpose is empty or oversized".to_owned(),
                ));
            }
            WorkerMessage::ProposeAction(action)
        }
        "emit_claim" => {
            let claim: ClaimProposal = deserialize_params(params)?;
            if !valid_label(&claim.claim_id) || claim.statement.trim().is_empty() {
                return Err(ProtocolError::InvalidParams(
                    "claim id or statement is invalid".to_owned(),
                ));
            }
            WorkerMessage::EmitClaim(claim)
        }
        "return_artifact" => {
            let artifact: ArtifactProposal = deserialize_params(params)?;
            validate_repository_relative_path(&artifact.relative_path)?;
            if !valid_digest(&artifact.digest) {
                return Err(ProtocolError::InvalidParams(
                    "artifact digest must be lowercase SHA-256 hex".to_owned(),
                ));
            }
            WorkerMessage::ReturnArtifact(artifact)
        }
        "report_blocker" => {
            let blocker: BlockerReport = deserialize_params(params)?;
            if !valid_label(&blocker.code) || !valid_digest(&blocker.evidence_digest) {
                return Err(ProtocolError::InvalidParams(
                    "blocker code or evidence digest is invalid".to_owned(),
                ));
            }
            WorkerMessage::ReportBlocker(blocker)
        }
        "propose_completion" => {
            let completion: CompletionProposal = deserialize_params(params)?;
            if completion.claim_ids.is_empty()
                || completion.claim_ids.len() > MAX_CLAIMS
                || completion.claim_ids.iter().any(|claim| !valid_label(claim))
            {
                return Err(ProtocolError::InvalidParams(
                    "completion claim list is invalid".to_owned(),
                ));
            }
            WorkerMessage::ProposeCompletion(completion)
        }
        other => return Err(ProtocolError::ForbiddenMethod(other.to_owned())),
    };

    Ok(WorkerEnvelope {
        id,
        mission_id,
        worker_id,
        message,
    })
}
