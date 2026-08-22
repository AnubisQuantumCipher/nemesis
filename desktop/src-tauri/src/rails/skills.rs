//! Skills rail — proposed procedures staged before trust promotion.
//!
//! A skill is a reviewable local procedure text. The body is content-addressed
//! at `state/content/<sha256>.skill` (immutable plumbing); the governed entity
//! binds the body digest and carries the trust state machine, which mirrors
//! `nemesis-runtime`'s `SkillStatus` ladder: no state may be skipped, promotion
//! to `approved` requires a named approver and a nonzero signature, and
//! `revoked` is terminal.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    MAX_NAME_BYTES, RailError, RailMutationRequest, bounded_text, entity_revision, entity_status,
    lowercase_hex, optional_text,
};
use crate::production::atomic_write;
use crate::state_repo::state_root;

pub const SCHEMA: &str = "nemesis.rail-skill/v1";
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Trust ladder in promotion order. Single-step advancement only.
pub const LADDER: &[&str] = &[
    "proposed",
    "staged",
    "statically-checked",
    "tested-in-sandbox",
    "evaluated",
    "approved",
    "trusted",
];

pub fn content_path(home: &Path, digest: &str) -> std::path::PathBuf {
    state_root(home)
        .join("content")
        .join(format!("{digest}.skill"))
}

/// Content-address a skill body as immutable plumbing. Committing happens with
/// the placeholder commit; a digest collision with different bytes refuses.
pub fn store_body(home: &Path, body: &str) -> Result<(String, u64), RailError> {
    if body.is_empty() || body.len() > MAX_BODY_BYTES {
        return Err(RailError::refused(format!(
            "skill body must contain 1..{MAX_BODY_BYTES} bytes"
        )));
    }
    let digest = hex::encode(Sha256::digest(body.as_bytes()));
    let path = content_path(home, &digest);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| RailError::storage(error.to_string()))?;
    }
    match fs::read(&path) {
        Ok(existing) => {
            if existing != body.as_bytes() {
                return Err(RailError::corrupt(format!(
                    "content store digest collision at {digest}"
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            atomic_write(&path, body.as_bytes())
                .map_err(|error| RailError::storage(error.to_string()))?;
        }
        Err(error) => return Err(RailError::storage(error.to_string())),
    }
    Ok((digest, body.len() as u64))
}

/// Read and verify a stored body against its bound digest.
pub fn load_body(home: &Path, digest: &str) -> Result<String, RailError> {
    if !lowercase_hex(digest, 64) {
        return Err(RailError::refused("body digest must be lowercase 64-hex"));
    }
    let bytes = fs::read(content_path(home, digest))
        .map_err(|error| RailError::corrupt(format!("skill body unavailable: {error}")))?;
    if hex::encode(Sha256::digest(&bytes)) != digest {
        return Err(RailError::corrupt("skill body does not match bound digest"));
    }
    String::from_utf8(bytes).map_err(|_| RailError::corrupt("skill body is not UTF-8"))
}

pub fn plan(
    home: &Path,
    request: &RailMutationRequest,
    current: Option<&Value>,
) -> Result<(Value, String), RailError> {
    let revision = entity_revision(current) + 1;
    match request.verb.as_str() {
        "propose" => {
            if let Some(status) = entity_status(current) {
                if status != "revoked" {
                    return Err(RailError::refused(format!(
                        "skill {} already exists with status {status}; revoke before re-proposing",
                        request.id
                    )));
                }
            }
            let name = bounded_text(&request.payload, "name", MAX_NAME_BYTES)?;
            let body = super::raw_text(&request.payload, "body", MAX_BODY_BYTES)?;
            let (digest, bytes) = store_body(home, &body)?;
            let entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "name": name,
                "bodyDigest": digest,
                "bodyBytes": bytes,
                "status": "proposed",
                "approvedBy": Value::Null,
                "approvalSignature": Value::Null,
                "revision": revision,
            });
            Ok((entity, format!("propose skill '{name}' body {digest}")))
        }
        "advance" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("skill {} does not exist", request.id))
            })?;
            let status = entity_status(Some(current))
                .ok_or_else(|| RailError::corrupt("skill entity has no status"))?;
            if status == "revoked" {
                return Err(RailError::refused("revoked skills cannot advance"));
            }
            let target = bounded_text(&request.payload, "toStatus", MAX_NAME_BYTES)?;
            let position = LADDER
                .iter()
                .position(|step| *step == status)
                .ok_or_else(|| RailError::corrupt(format!("unknown skill status {status}")))?;
            let expected = LADDER.get(position + 1).copied().ok_or_else(|| {
                RailError::refused("skill is already at the terminal trust state")
            })?;
            if target != expected {
                return Err(RailError::refused(format!(
                    "trust states cannot be skipped: {status} advances only to {expected}"
                )));
            }
            let mut entity = current.clone();
            if target == "approved" {
                let approver = bounded_text(&request.payload, "approvedBy", MAX_NAME_BYTES)?;
                let signature = bounded_text(&request.payload, "approvalSignature", 128)?;
                if !lowercase_hex(&signature, 128) || signature.bytes().all(|byte| byte == b'0') {
                    return Err(RailError::refused(
                        "approval requires a nonzero lowercase 128-hex signature",
                    ));
                }
                entity["approvedBy"] = Value::String(approver);
                entity["approvalSignature"] = Value::String(signature);
            }
            entity["status"] = Value::String(target.clone());
            entity["revision"] = Value::from(revision);
            Ok((entity, format!("advance skill to {target}")))
        }
        "revoke" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("skill {} does not exist", request.id))
            })?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("skill is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", super::MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke skill — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "skills rail accepts propose|advance|revoke, not {other}"
        ))),
    }
}

/// Read-time integrity view: every non-revoked skill's body must exist and
/// match its bound digest; a broken binding is surfaced, never hidden.
pub fn body_integrity(home: &Path, entity: &Value) -> Value {
    let digest = entity
        .get("bodyDigest")
        .and_then(Value::as_str)
        .unwrap_or("");
    match load_body(home, digest) {
        Ok(_) => json!({"bodyPresent": true, "bodyVerified": true}),
        Err(error) => json!({
            "bodyPresent": false,
            "bodyVerified": false,
            "integrityError": error.to_string(),
        }),
    }
}
