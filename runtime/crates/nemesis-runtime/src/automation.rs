use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AutomationOperation {
    Observe,
    RunWorker,
    ModifyWorkspace,
    Network,
    Secret,
    Approval,
    Push,
    Publish,
    ExternalDelivery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerKind {
    LocalSchedule,
    LocalFileEvent,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationRequest {
    pub id: u64,
    pub mission_template_digest: [u8; 32],
    pub operation: AutomationOperation,
    pub trigger: TriggerKind,
    pub trigger_key: String,
    pub due_unix_seconds: u64,
    pub requires_approval: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutomationDecision {
    Authorized,
    DeniedAuthorityBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomationPolicy {
    allowed_unattended: BTreeSet<AutomationOperation>,
}

impl AutomationPolicy {
    pub fn safe_unattended(allowed_unattended: BTreeSet<AutomationOperation>) -> Self {
        Self { allowed_unattended }
    }

    pub fn evaluate(&self, request: &AutomationRequest) -> AutomationDecision {
        let permanently_denied = matches!(
            request.operation,
            AutomationOperation::Network
                | AutomationOperation::Secret
                | AutomationOperation::Approval
                | AutomationOperation::Push
                | AutomationOperation::Publish
                | AutomationOperation::ExternalDelivery
        );
        if request.requires_approval
            || permanently_denied
            || !self.allowed_unattended.contains(&request.operation)
        {
            AutomationDecision::DeniedAuthorityBoundary
        } else {
            AutomationDecision::Authorized
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutomationStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Denied,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QueueEntry {
    request: AutomationRequest,
    status: AutomationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchResult {
    pub request_id: u64,
    pub decision: AutomationDecision,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AutomationError {
    #[error("automation queue configuration or request is invalid")]
    Invalid,
    #[error("automation request or trigger is duplicated")]
    Duplicate,
    #[error("automation queue is full")]
    QueueFull,
    #[error("automation dispatch rate is exhausted")]
    RateLimited,
    #[error("automation request was not found or is in the wrong state")]
    NotFound,
    #[error("automation checkpoint signature failed")]
    Signature,
    #[error("automation checkpoint serialization failed")]
    Serialization,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QueueSnapshot {
    capacity: usize,
    maximum_per_minute: usize,
    entries: BTreeMap<u64, QueueEntry>,
    order: Vec<u64>,
    trigger_keys: BTreeSet<String>,
    dispatch_times: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomationCheckpoint {
    pub encoded: Vec<u8>,
    pub digest: [u8; 32],
    pub signature: [u8; 64],
    pub public_key: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomationQueue {
    capacity: usize,
    maximum_per_minute: usize,
    entries: BTreeMap<u64, QueueEntry>,
    order: Vec<u64>,
    trigger_keys: BTreeSet<String>,
    dispatch_times: Vec<u64>,
}

fn valid_trigger(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

impl AutomationQueue {
    pub fn new(capacity: usize, maximum_per_minute: usize) -> Result<Self, AutomationError> {
        if capacity == 0 || maximum_per_minute == 0 {
            return Err(AutomationError::Invalid);
        }
        Ok(Self {
            capacity,
            maximum_per_minute,
            entries: BTreeMap::new(),
            order: Vec::new(),
            trigger_keys: BTreeSet::new(),
            dispatch_times: Vec::new(),
        })
    }

    pub fn enqueue(&mut self, request: AutomationRequest) -> Result<(), AutomationError> {
        if request.id == 0
            || !valid_trigger(&request.trigger_key)
            || request.mission_template_digest == [0; 32]
        {
            return Err(AutomationError::Invalid);
        }
        if self.entries.contains_key(&request.id)
            || self.trigger_keys.contains(&request.trigger_key)
        {
            return Err(AutomationError::Duplicate);
        }
        if self.entries.len() >= self.capacity {
            return Err(AutomationError::QueueFull);
        }
        self.trigger_keys.insert(request.trigger_key.clone());
        self.order.push(request.id);
        self.entries.insert(
            request.id,
            QueueEntry {
                request,
                status: AutomationStatus::Queued,
            },
        );
        Ok(())
    }

    pub fn dispatch_next(
        &mut self,
        now_unix_seconds: u64,
        policy: &AutomationPolicy,
    ) -> Result<DispatchResult, AutomationError> {
        let cutoff = now_unix_seconds.saturating_sub(60);
        self.dispatch_times.retain(|time| *time >= cutoff);
        let request_id = self
            .order
            .iter()
            .copied()
            .find(|id| {
                self.entries.get(id).is_some_and(|entry| {
                    entry.status == AutomationStatus::Queued
                        && entry.request.due_unix_seconds <= now_unix_seconds
                })
            })
            .ok_or(AutomationError::NotFound)?;
        let entry = self
            .entries
            .get_mut(&request_id)
            .ok_or(AutomationError::NotFound)?;
        let decision = policy.evaluate(&entry.request);
        if decision == AutomationDecision::DeniedAuthorityBoundary {
            entry.status = AutomationStatus::Denied;
            return Ok(DispatchResult {
                request_id,
                decision,
            });
        }
        if self.dispatch_times.len() >= self.maximum_per_minute {
            return Err(AutomationError::RateLimited);
        }
        entry.status = AutomationStatus::Running;
        self.dispatch_times.push(now_unix_seconds);
        Ok(DispatchResult {
            request_id,
            decision,
        })
    }

    pub fn complete(&mut self, request_id: u64, success: bool) -> Result<(), AutomationError> {
        let entry = self
            .entries
            .get_mut(&request_id)
            .ok_or(AutomationError::NotFound)?;
        if entry.status != AutomationStatus::Running {
            return Err(AutomationError::NotFound);
        }
        entry.status = if success {
            AutomationStatus::Completed
        } else {
            AutomationStatus::Failed
        };
        Ok(())
    }

    pub fn status(&self, request_id: u64) -> Option<AutomationStatus> {
        self.entries.get(&request_id).map(|entry| entry.status)
    }

    fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            capacity: self.capacity,
            maximum_per_minute: self.maximum_per_minute,
            entries: self.entries.clone(),
            order: self.order.clone(),
            trigger_keys: self.trigger_keys.clone(),
            dispatch_times: self.dispatch_times.clone(),
        }
    }

    pub fn checkpoint(
        &self,
        signing_key: &SigningKey,
    ) -> Result<AutomationCheckpoint, AutomationError> {
        let encoded =
            serde_json::to_vec(&self.snapshot()).map_err(|_| AutomationError::Serialization)?;
        let digest: [u8; 32] = Sha256::digest(&encoded).into();
        Ok(AutomationCheckpoint {
            signature: signing_key.sign(&encoded).to_bytes(),
            public_key: signing_key.verifying_key().to_bytes(),
            encoded,
            digest,
        })
    }

    pub fn recover(
        checkpoint: &AutomationCheckpoint,
        verifying_key: &VerifyingKey,
    ) -> Result<Self, AutomationError> {
        let digest: [u8; 32] = Sha256::digest(&checkpoint.encoded).into();
        if digest != checkpoint.digest || checkpoint.public_key != verifying_key.to_bytes() {
            return Err(AutomationError::Signature);
        }
        verifying_key
            .verify_strict(
                &checkpoint.encoded,
                &Signature::from_bytes(&checkpoint.signature),
            )
            .map_err(|_| AutomationError::Signature)?;
        let snapshot: QueueSnapshot = serde_json::from_slice(&checkpoint.encoded)
            .map_err(|_| AutomationError::Serialization)?;
        if snapshot.capacity == 0
            || snapshot.maximum_per_minute == 0
            || snapshot.entries.len() > snapshot.capacity
            || snapshot.order.len() != snapshot.entries.len()
            || snapshot.trigger_keys.len() != snapshot.entries.len()
            || snapshot
                .order
                .iter()
                .any(|id| !snapshot.entries.contains_key(id))
        {
            return Err(AutomationError::Invalid);
        }
        let queue = Self {
            capacity: snapshot.capacity,
            maximum_per_minute: snapshot.maximum_per_minute,
            entries: snapshot.entries,
            order: snapshot.order,
            trigger_keys: snapshot.trigger_keys,
            dispatch_times: snapshot.dispatch_times,
        };
        if queue
            .entries
            .values()
            .any(|entry| !queue.trigger_keys.contains(&entry.request.trigger_key))
        {
            return Err(AutomationError::Invalid);
        }
        Ok(queue)
    }
}
