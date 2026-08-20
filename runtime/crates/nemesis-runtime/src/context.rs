use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::WorkerRole;

const MAX_TEXT_BYTES: usize = 4_096;
const MAX_COLLECTION_ITEMS: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeStatus {
    Proposed,
    Observed,
    Verified,
    Disputed,
    Superseded,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: u64,
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub scope: String,
    pub source_digest: [u8; 32],
    pub observed_sequence: u64,
    pub valid_from: u64,
    pub valid_until: Option<u64>,
    pub status: KnowledgeStatus,
    pub supersedes: Option<u64>,
    pub contradicted_by: Vec<u64>,
    pub untrusted_external: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnowledgeBase {
    entries: BTreeMap<u64, KnowledgeEntry>,
    contradictions: BTreeSet<(u64, u64)>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContextError {
    #[error("knowledge entry or context request is invalid")]
    InvalidInput,
    #[error("knowledge identifier already exists")]
    DuplicateKnowledge,
    #[error("referenced knowledge entry does not exist")]
    MissingKnowledge,
    #[error("mandatory context exceeds the configured byte budget")]
    BudgetTooSmall,
    #[error("context serialization failed: {0}")]
    Serialization(String),
    #[error("context signature verification failed")]
    Signature,
    #[error("repository index escaped its root or exceeded its budget")]
    RepositoryBoundary,
    #[error("repository I/O failed: {0}")]
    RepositoryIo(String),
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES && !value.as_bytes().contains(&0)
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, mut entry: KnowledgeEntry) -> Result<(), ContextError> {
        if entry.id == 0
            || !bounded_text(&entry.subject)
            || !bounded_text(&entry.predicate)
            || !bounded_text(&entry.value)
            || !bounded_text(&entry.scope)
            || entry.contradicted_by.len() > MAX_COLLECTION_ITEMS
            || self.entries.contains_key(&entry.id)
        {
            return if self.entries.contains_key(&entry.id) {
                Err(ContextError::DuplicateKnowledge)
            } else {
                Err(ContextError::InvalidInput)
            };
        }
        entry.contradicted_by.sort_unstable();
        entry.contradicted_by.dedup();
        if let Some(old_id) = entry.supersedes {
            let old = self
                .entries
                .get_mut(&old_id)
                .ok_or(ContextError::MissingKnowledge)?;
            old.status = KnowledgeStatus::Superseded;
        }
        self.entries.insert(entry.id, entry);
        Ok(())
    }

    pub fn get(&self, id: u64) -> Option<&KnowledgeEntry> {
        self.entries.get(&id)
    }

    pub fn revoke(&mut self, id: u64) -> Result<(), ContextError> {
        self.entries
            .get_mut(&id)
            .ok_or(ContextError::MissingKnowledge)?
            .status = KnowledgeStatus::Revoked;
        Ok(())
    }

    pub fn record_contradiction(&mut self, left: u64, right: u64) -> Result<(), ContextError> {
        if left == right || !self.entries.contains_key(&left) || !self.entries.contains_key(&right)
        {
            return Err(ContextError::MissingKnowledge);
        }
        let pair = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        self.contradictions.insert(pair);
        let left_entry = self
            .entries
            .get_mut(&left)
            .ok_or(ContextError::MissingKnowledge)?;
        if !left_entry.contradicted_by.contains(&right) {
            left_entry.contradicted_by.push(right);
            left_entry.contradicted_by.sort_unstable();
        }
        left_entry.status = KnowledgeStatus::Disputed;
        let right_entry = self
            .entries
            .get_mut(&right)
            .ok_or(ContextError::MissingKnowledge)?;
        if !right_entry.contradicted_by.contains(&left) {
            right_entry.contradicted_by.push(left);
            right_entry.contradicted_by.sort_unstable();
        }
        right_entry.status = KnowledgeStatus::Disputed;
        Ok(())
    }

    pub fn active_at(&self, sequence: u64) -> Vec<KnowledgeEntry> {
        self.entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.status,
                    KnowledgeStatus::Observed | KnowledgeStatus::Verified
                ) && entry.valid_from <= sequence
                    && entry.valid_until.is_none_or(|until| sequence <= until)
            })
            .cloned()
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextRequest {
    pub mission_id: String,
    pub source_digest: [u8; 32],
    pub worker_role: WorkerRole,
    pub obligations: Vec<String>,
    pub invariants: Vec<String>,
    pub decisions: Vec<String>,
    pub failed_approaches: Vec<String>,
    pub blockers: Vec<String>,
    pub capability_summary: String,
    pub remaining_budget_units: u64,
    pub capsule_sequence: u64,
    pub maximum_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCapsule {
    pub schema: String,
    pub mission_id: String,
    pub source_digest: [u8; 32],
    pub worker_role: String,
    pub obligations: Vec<String>,
    pub invariants: Vec<String>,
    pub decisions: Vec<String>,
    pub failed_approaches: Vec<String>,
    pub blockers: Vec<String>,
    pub contradictions: Vec<(u64, u64)>,
    pub knowledge: Vec<KnowledgeEntry>,
    pub capability_summary: String,
    pub remaining_budget_units: u64,
    pub capsule_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedContextCapsule {
    pub capsule: ContextCapsule,
    pub encoded: Vec<u8>,
    pub digest: [u8; 32],
    pub signature: [u8; 64],
    pub public_key: [u8; 32],
}

fn validate_list(values: &[String]) -> bool {
    values.len() <= MAX_COLLECTION_ITEMS && values.iter().all(|value| bounded_text(value))
}

fn encode_capsule(capsule: &ContextCapsule) -> Result<Vec<u8>, ContextError> {
    serde_json::to_vec(capsule).map_err(|error| ContextError::Serialization(error.to_string()))
}

pub struct ContextCompiler;

impl ContextCompiler {
    pub fn compile(
        knowledge: &KnowledgeBase,
        request: ContextRequest,
        signing_key: &SigningKey,
    ) -> Result<SignedContextCapsule, ContextError> {
        if request.mission_id.len() != 26
            || !request.mission_id.starts_with("mis_")
            || request.capsule_sequence == 0
            || request.maximum_bytes == 0
            || !validate_list(&request.obligations)
            || !validate_list(&request.invariants)
            || !validate_list(&request.decisions)
            || !validate_list(&request.failed_approaches)
            || !validate_list(&request.blockers)
            || !bounded_text(&request.capability_summary)
        {
            return Err(ContextError::InvalidInput);
        }
        let mut capsule = ContextCapsule {
            schema: "nemesis.context/v1".to_owned(),
            mission_id: request.mission_id,
            source_digest: request.source_digest,
            worker_role: format!("{:?}", request.worker_role),
            obligations: request.obligations,
            invariants: request.invariants,
            decisions: request.decisions,
            failed_approaches: request.failed_approaches,
            blockers: request.blockers,
            contradictions: knowledge.contradictions.iter().copied().collect(),
            knowledge: Vec::new(),
            capability_summary: request.capability_summary,
            remaining_budget_units: request.remaining_budget_units,
            capsule_sequence: request.capsule_sequence,
        };
        let mandatory = encode_capsule(&capsule)?;
        if mandatory.len() > request.maximum_bytes {
            return Err(ContextError::BudgetTooSmall);
        }
        for entry in knowledge.active_at(request.capsule_sequence) {
            capsule.knowledge.push(entry);
            let candidate = encode_capsule(&capsule)?;
            if candidate.len() > request.maximum_bytes {
                capsule.knowledge.pop();
            }
        }
        let encoded = encode_capsule(&capsule)?;
        let digest: [u8; 32] = Sha256::digest(&encoded).into();
        let signature = signing_key.sign(&encoded).to_bytes();
        Ok(SignedContextCapsule {
            capsule,
            encoded,
            digest,
            signature,
            public_key: signing_key.verifying_key().to_bytes(),
        })
    }
}

pub fn verify_context_capsule(
    signed: &SignedContextCapsule,
    verifying_key: &VerifyingKey,
) -> Result<(), ContextError> {
    let encoded = encode_capsule(&signed.capsule)?;
    let digest: [u8; 32] = Sha256::digest(&signed.encoded).into();
    if encoded != signed.encoded
        || digest != signed.digest
        || verifying_key.to_bytes() != signed.public_key
    {
        return Err(ContextError::Signature);
    }
    verifying_key
        .verify_strict(&signed.encoded, &Signature::from_bytes(&signed.signature))
        .map_err(|_| ContextError::Signature)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedFile {
    pub relative_path: String,
    pub digest: [u8; 32],
    pub byte_length: usize,
}

pub fn index_repository(
    root: &Path,
    relative_paths: &[String],
    maximum_bytes: usize,
) -> Result<Vec<IndexedFile>, ContextError> {
    let root = root
        .canonicalize()
        .map_err(|error| ContextError::RepositoryIo(error.to_string()))?;
    let mut requested = BTreeSet::new();
    for relative in relative_paths {
        let path = Path::new(relative);
        if relative.is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            || !requested.insert(relative.clone())
        {
            return Err(ContextError::RepositoryBoundary);
        }
    }
    let mut total = 0usize;
    let mut indexed = Vec::with_capacity(requested.len());
    for relative in requested {
        let candidate = root.join(&relative);
        let canonical = candidate
            .canonicalize()
            .map_err(|error| ContextError::RepositoryIo(error.to_string()))?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(ContextError::RepositoryBoundary);
        }
        let bytes =
            fs::read(&canonical).map_err(|error| ContextError::RepositoryIo(error.to_string()))?;
        total = total
            .checked_add(bytes.len())
            .ok_or(ContextError::RepositoryBoundary)?;
        if total > maximum_bytes {
            return Err(ContextError::RepositoryBoundary);
        }
        indexed.push(IndexedFile {
            relative_path: relative,
            digest: Sha256::digest(&bytes).into(),
            byte_length: bytes.len(),
        });
    }
    Ok(indexed)
}
