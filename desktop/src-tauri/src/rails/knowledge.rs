//! Knowledge rail — provenance-aware facts, contradictions, revocations.
//!
//! Entries mirror the context-v1 knowledge model: subject/predicate/value with
//! a bound source digest, an explicit status ladder, supersession links, and
//! an untrusted-external marker. Only `observed` and `verified` entries within
//! their validity window count as active truth; contradictions among active
//! entries are surfaced at read time, never resolved silently.

use std::path::Path;

use serde_json::{Value, json};

use super::{
    MAX_NOTE_BYTES, RailError, RailMutationRequest, bounded_text, entity_revision, entity_status,
    one_of, optional_text, required_bool, required_hex, required_u64,
};

pub const SCHEMA: &str = "nemesis.rail-knowledge/v1";
pub const SCOPES: &[&str] = &["mission", "workspace", "global"];
pub const STATUSES: &[&str] = &[
    "proposed",
    "observed",
    "verified",
    "disputed",
    "superseded",
    "revoked",
    "expired",
];
const MAX_SUBJECT_BYTES: usize = 128;
const MAX_VALUE_BYTES: usize = 1024;

fn legal_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("proposed", "observed")
            | ("observed", "verified")
            | ("proposed", "disputed")
            | ("observed", "disputed")
            | ("verified", "disputed")
            | ("proposed", "superseded")
            | ("observed", "superseded")
            | ("verified", "superseded")
            | ("disputed", "superseded")
            | ("proposed", "expired")
            | ("observed", "expired")
            | ("verified", "expired")
    )
}

pub fn plan(
    home: &Path,
    request: &RailMutationRequest,
    current: Option<&Value>,
) -> Result<(Value, String), RailError> {
    let revision = entity_revision(current) + 1;
    match request.verb.as_str() {
        "assert" => {
            if let Some(status) = entity_status(current) {
                if status != "revoked" {
                    return Err(RailError::refused(format!(
                        "knowledge entry {} already exists with status {status}",
                        request.id
                    )));
                }
            }
            let subject = bounded_text(&request.payload, "subject", MAX_SUBJECT_BYTES)?;
            let predicate = bounded_text(&request.payload, "predicate", MAX_SUBJECT_BYTES)?;
            let value = bounded_text(&request.payload, "value", MAX_VALUE_BYTES)?;
            let scope = one_of(&request.payload, "scope", SCOPES)?;
            let source_digest = required_hex(&request.payload, "sourceDigest", 64)?;
            let observed_sequence = required_u64(&request.payload, "observedSequence")?;
            let untrusted_external = required_bool(&request.payload, "untrustedExternal")?;
            let supersedes = optional_text(&request.payload, "supersedes", 48)?;
            if let Some(target) = supersedes.as_deref() {
                crate::state_repo::validate_entity_id(target)?;
                let target_entity =
                    super::current_entity(home, "knowledge", target)?.ok_or_else(|| {
                        RailError::refused(format!("superseded entry {target} does not exist"))
                    })?;
                let target_status = target_entity
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !legal_transition(target_status, "superseded") {
                    return Err(RailError::refused(format!(
                        "entry {target} in status {target_status} cannot be superseded"
                    )));
                }
            }
            let entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "subject": subject,
                "predicate": predicate,
                "value": value,
                "scope": scope,
                "sourceDigest": source_digest,
                "observedSequence": observed_sequence,
                "status": "proposed",
                "supersedes": supersedes,
                "contradictedBy": Value::Null,
                "untrustedExternal": untrusted_external,
                "revision": revision,
            });
            Ok((entity, format!("assert {subject} {predicate}")))
        }
        "transition" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("knowledge entry {} does not exist", request.id))
            })?;
            let status = entity_status(Some(current))
                .ok_or_else(|| RailError::corrupt("knowledge entry has no status"))?;
            let target = one_of(&request.payload, "toStatus", STATUSES)?;
            if !legal_transition(&status, &target) {
                return Err(RailError::refused(format!(
                    "knowledge status {status} cannot transition to {target}"
                )));
            }
            let mut entity = current.clone();
            if target == "disputed" {
                let contradicted_by = bounded_text(&request.payload, "contradictedBy", 48)?;
                crate::state_repo::validate_entity_id(&contradicted_by)?;
                entity["contradictedBy"] = Value::String(contradicted_by);
            }
            entity["status"] = Value::String(target.clone());
            entity["revision"] = Value::from(revision);
            Ok((entity, format!("transition to {target}")))
        }
        "revoke" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("knowledge entry {} does not exist", request.id))
            })?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("knowledge entry is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke knowledge entry — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "knowledge rail accepts assert|transition|revoke, not {other}"
        ))),
    }
}

fn is_active(entity: &Value) -> bool {
    matches!(
        entity.get("status").and_then(Value::as_str),
        Some("observed") | Some("verified")
    )
}

/// Active entries sharing subject+predicate with differing values are
/// contradictions. They are reported as pairs; presentation must show both.
pub fn contradictions(entities: &[crate::state_repo::RailEntity]) -> Vec<Value> {
    let mut findings = Vec::new();
    let active: Vec<_> = entities
        .iter()
        .filter(|entity| is_active(&entity.value))
        .collect();
    for (index, left) in active.iter().enumerate() {
        for right in active.iter().skip(index + 1) {
            let same_subject = left.value.get("subject") == right.value.get("subject")
                && left.value.get("predicate") == right.value.get("predicate");
            let differing = left.value.get("value") != right.value.get("value");
            if same_subject && differing {
                findings.push(json!({
                    "subject": left.value.get("subject"),
                    "predicate": left.value.get("predicate"),
                    "entries": [left.id, right.id],
                }));
            }
        }
    }
    findings
}
