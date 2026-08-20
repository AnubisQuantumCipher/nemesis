use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const LEDGER_RECORD_BYTES: usize = 305;
const LEDGER_CORE_BYTES: usize = 239;
const LEDGER_LINE_BYTES: usize = 304;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub sequence: u64,
    pub kind_code: u8,
    pub state_code: u8,
    pub source_digest: [u8; 32],
    pub payload_digest: [u8; 32],
    pub previous_hash: [u8; 32],
    pub event_hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdaReplay {
    pub events: Vec<LedgerEvent>,
    pub final_state_code: u8,
    pub head: [u8; 32],
    pub exact_state_reconstruction: bool,
    pub exact_model_reexecution: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReplayError {
    #[error("ledger is empty")]
    Empty,
    #[error("ledger has a truncated record")]
    Truncated,
    #[error("ledger is corrupt: {0}")]
    Corrupt(String),
    #[error("mission fork request is invalid")]
    InvalidFork,
}

fn parse_digest(value: &[u8]) -> Result<[u8; 32], ReplayError> {
    if value.len() != 64
        || !value
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(ReplayError::Corrupt("invalid digest encoding".to_owned()));
    }
    let decoded = hex::decode(value)
        .map_err(|error| ReplayError::Corrupt(format!("invalid digest: {error}")))?;
    decoded
        .try_into()
        .map_err(|_| ReplayError::Corrupt("digest width changed".to_owned()))
}

fn parse_decimal<T>(value: &[u8], field: &str) -> Result<T, ReplayError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let text = std::str::from_utf8(value)
        .map_err(|error| ReplayError::Corrupt(format!("{field}: {error}")))?;
    text.parse::<T>()
        .map_err(|error| ReplayError::Corrupt(format!("{field}: {error}")))
}

impl AdaReplay {
    pub fn parse(bytes: &[u8]) -> Result<Self, ReplayError> {
        if bytes.is_empty() {
            return Err(ReplayError::Empty);
        }
        if bytes.len() % LEDGER_RECORD_BYTES != 0 {
            return Err(ReplayError::Truncated);
        }
        let mut events = Vec::with_capacity(bytes.len() / LEDGER_RECORD_BYTES);
        let mut expected_sequence = 1u64;
        let mut expected_previous = [0u8; 32];
        for record in bytes.chunks_exact(LEDGER_RECORD_BYTES) {
            if &record[..17] != b"NEMESIS_LEDGER_V1"
                || record[17] != b'|'
                || record[38] != b'|'
                || record[41] != b'|'
                || record[44] != b'|'
                || record[109] != b'|'
                || record[174] != b'|'
                || record[239] != b'|'
                || record[LEDGER_RECORD_BYTES - 1] != b'\n'
            {
                return Err(ReplayError::Corrupt(
                    "schema, delimiter, or terminator mismatch".to_owned(),
                ));
            }
            let sequence = parse_decimal::<u64>(&record[18..38], "sequence")?;
            let kind_code = parse_decimal::<u8>(&record[39..41], "kind")?;
            let state_code = parse_decimal::<u8>(&record[42..44], "state")?;
            if sequence != expected_sequence || kind_code > 9 || state_code > 10 {
                return Err(ReplayError::Corrupt(
                    "sequence or enumeration mismatch".to_owned(),
                ));
            }
            let source_digest = parse_digest(&record[45..109])?;
            let payload_digest = parse_digest(&record[110..174])?;
            let previous_hash = parse_digest(&record[175..239])?;
            let event_hash = parse_digest(&record[240..LEDGER_LINE_BYTES])?;
            if previous_hash != expected_previous {
                return Err(ReplayError::Corrupt("previous hash mismatch".to_owned()));
            }
            let calculated: [u8; 32] = Sha256::digest(&record[..LEDGER_CORE_BYTES]).into();
            if event_hash != calculated {
                return Err(ReplayError::Corrupt("event hash mismatch".to_owned()));
            }
            events.push(LedgerEvent {
                sequence,
                kind_code,
                state_code,
                source_digest,
                payload_digest,
                previous_hash,
                event_hash,
            });
            expected_sequence = expected_sequence
                .checked_add(1)
                .ok_or_else(|| ReplayError::Corrupt("sequence exhausted".to_owned()))?;
            expected_previous = event_hash;
        }
        let last = events.last().ok_or(ReplayError::Empty)?;
        Ok(Self {
            final_state_code: last.state_code,
            head: last.event_hash,
            events,
            exact_state_reconstruction: true,
            exact_model_reexecution: false,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceClass {
    Informational,
    Advisory,
    DecisionBoundary,
    SafetyCritical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceVerdict {
    Pending,
    Accepted,
    Stale,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: u64,
    pub claim: String,
    pub source_digest: [u8; 32],
    pub dependency_digests: BTreeSet<[u8; 32]>,
    pub verifier_id: String,
    pub class: EvidenceClass,
    pub deterministic: bool,
    pub independent: bool,
    pub verdict: EvidenceVerdict,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerifierRegistry {
    verifiers: BTreeMap<String, EvidenceClass>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvidenceError {
    #[error("evidence or verifier record is invalid")]
    Invalid,
    #[error("evidence or verifier identifier already exists")]
    Duplicate,
    #[error("verifier is not authorized for this consequence")]
    UnauthorizedVerifier,
    #[error("evidence is stale for current source")]
    StaleSource,
    #[error("elevated evidence lacks deterministic independent verification")]
    WeakEvidence,
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

impl VerifierRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        verifier_id: &str,
        maximum_class: EvidenceClass,
    ) -> Result<(), EvidenceError> {
        if !valid_label(verifier_id) {
            return Err(EvidenceError::Invalid);
        }
        if self
            .verifiers
            .insert(verifier_id.to_owned(), maximum_class)
            .is_some()
        {
            return Err(EvidenceError::Duplicate);
        }
        Ok(())
    }

    fn authorizes(&self, verifier_id: &str, class: EvidenceClass) -> bool {
        self.verifiers
            .get(verifier_id)
            .is_some_and(|maximum| *maximum >= class)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvidenceStore {
    records: BTreeMap<u64, EvidenceRecord>,
}

impl EvidenceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept(
        &mut self,
        mut evidence: EvidenceRecord,
        current_source: [u8; 32],
        registry: &VerifierRegistry,
    ) -> Result<(), EvidenceError> {
        if evidence.id == 0
            || !valid_label(&evidence.claim)
            || !valid_label(&evidence.verifier_id)
            || evidence.verdict != EvidenceVerdict::Pending
        {
            return Err(EvidenceError::Invalid);
        }
        if self.records.contains_key(&evidence.id) {
            return Err(EvidenceError::Duplicate);
        }
        if evidence.source_digest != current_source {
            return Err(EvidenceError::StaleSource);
        }
        if !registry.authorizes(&evidence.verifier_id, evidence.class) {
            return Err(EvidenceError::UnauthorizedVerifier);
        }
        if evidence.class >= EvidenceClass::DecisionBoundary
            && (!evidence.deterministic || !evidence.independent)
        {
            return Err(EvidenceError::WeakEvidence);
        }
        evidence.verdict = EvidenceVerdict::Accepted;
        self.records.insert(evidence.id, evidence);
        Ok(())
    }

    pub fn invalidate(
        &mut self,
        current_source: [u8; 32],
        changed_dependencies: &BTreeSet<[u8; 32]>,
    ) {
        for evidence in self.records.values_mut() {
            if evidence.verdict == EvidenceVerdict::Accepted
                && (evidence.source_digest != current_source
                    || !evidence
                        .dependency_digests
                        .is_disjoint(changed_dependencies))
            {
                evidence.verdict = EvidenceVerdict::Stale;
            }
        }
    }

    pub fn can_complete(&self, required_claims: &[String], current_source: [u8; 32]) -> bool {
        required_claims.iter().all(|claim| {
            self.records.values().any(|evidence| {
                evidence.claim == *claim
                    && evidence.source_digest == current_source
                    && evidence.verdict == EvidenceVerdict::Accepted
            })
        })
    }

    pub fn get(&self, id: u64) -> Option<&EvidenceRecord> {
        self.records.get(&id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphNodeKind {
    Requirement,
    Observation,
    Decision,
    Capability,
    Action,
    Artifact,
    Evidence,
    Completion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GraphNode {
    kind: GraphNodeKind,
    causes: Vec<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CausalGraph {
    nodes: BTreeMap<u64, GraphNode>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("causal node identifier is invalid or duplicated")]
    InvalidNode,
    #[error("causal parent does not exist")]
    MissingCause,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, id: u64, kind: GraphNodeKind, causes: &[u64]) -> Result<(), GraphError> {
        if id == 0 || self.nodes.contains_key(&id) {
            return Err(GraphError::InvalidNode);
        }
        let mut normalized = causes.to_vec();
        normalized.sort_unstable();
        normalized.dedup();
        if normalized
            .iter()
            .any(|cause| !self.nodes.contains_key(cause))
        {
            return Err(GraphError::MissingCause);
        }
        self.nodes.insert(
            id,
            GraphNode {
                kind,
                causes: normalized,
            },
        );
        Ok(())
    }

    pub fn path_to(&self, target: u64) -> Result<Vec<u64>, GraphError> {
        if !self.nodes.contains_key(&target) {
            return Err(GraphError::MissingCause);
        }
        fn visit(graph: &CausalGraph, id: u64, visited: &mut BTreeSet<u64>, path: &mut Vec<u64>) {
            if !visited.insert(id) {
                return;
            }
            if let Some(node) = graph.nodes.get(&id) {
                for cause in &node.causes {
                    visit(graph, *cause, visited, path);
                }
            }
            path.push(id);
        }
        let mut visited = BTreeSet::new();
        let mut path = Vec::new();
        visit(self, target, &mut visited, &mut path);
        Ok(path)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionFork {
    pub mission_id: String,
    pub parent_sequence: u64,
    pub parent_head: [u8; 32],
    pub events: Vec<LedgerEvent>,
    pub model_reexecution_is_comparative: bool,
}

pub fn fork_replay(
    replay: &AdaReplay,
    parent_sequence: u64,
    mission_id: &str,
) -> Result<MissionFork, ReplayError> {
    if mission_id.len() != 26 || !mission_id.starts_with("mis_") || parent_sequence == 0 {
        return Err(ReplayError::InvalidFork);
    }
    let events: Vec<_> = replay
        .events
        .iter()
        .take_while(|event| event.sequence <= parent_sequence)
        .cloned()
        .collect();
    let parent = events.last().ok_or(ReplayError::InvalidFork)?;
    if parent.sequence != parent_sequence {
        return Err(ReplayError::InvalidFork);
    }
    Ok(MissionFork {
        mission_id: mission_id.to_owned(),
        parent_sequence,
        parent_head: parent.event_hash,
        events,
        model_reexecution_is_comparative: true,
    })
}
