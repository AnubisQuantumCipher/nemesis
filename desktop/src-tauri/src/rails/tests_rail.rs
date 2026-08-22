//! Tests rail — deterministic checks bound to source.
//!
//! A test entity binds one deterministic check to one registered workspace
//! and an exact source object. Registration is governed; execution is
//! read-only, runs only the two deterministic verifier kinds the mission
//! pipeline itself uses, and appends a hash-chained run receipt that binds
//! the observed source digest — a run against changed source is visibly
//! STALE, never silently green.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    ChainedReceipt, MAX_NAME_BYTES, MAX_NOTE_BYTES, RailError, RailMutationRequest, append_receipt,
    bounded_text, current_entity, entity_revision, entity_status, one_of, optional_text,
};

pub const SCHEMA: &str = "nemesis.rail-test/v1";
pub const RUN_SCHEMA: &str = "nemesis.test-run/v1";
pub const RUN_CHAIN: &str = "test-runs";
pub const KINDS: &[&str] = &["git-diff-check", "content-match"];

pub fn plan(
    home: &Path,
    request: &RailMutationRequest,
    current: Option<&Value>,
) -> Result<(Value, String), RailError> {
    let revision = entity_revision(current) + 1;
    match request.verb.as_str() {
        "register" => {
            if let Some(status) = entity_status(current) {
                if status != "revoked" {
                    return Err(RailError::refused(format!(
                        "test {} already exists with status {status}",
                        request.id
                    )));
                }
            }
            let name = bounded_text(&request.payload, "name", MAX_NAME_BYTES)?;
            let kind = one_of(&request.payload, "kind", KINDS)?;
            let workspace_id = bounded_text(&request.payload, "workspaceId", 48)?;
            let workspace =
                current_entity(home, "workspaces", &workspace_id)?.ok_or_else(|| {
                    RailError::refused(format!("workspace {workspace_id} is not registered"))
                })?;
            if workspace.get("status").and_then(Value::as_str) != Some("active") {
                return Err(RailError::refused(format!(
                    "workspace {workspace_id} is not active"
                )));
            }
            let relative_path = bounded_text(&request.payload, "relativePath", 512)?;
            nemesis_protocol::validate_repository_relative_path(&relative_path)
                .map_err(|error| RailError::refused(format!("relative path refused: {error}")))?;
            let expected = optional_text(&request.payload, "expectedSha256", 64)?;
            if kind == "content-match" {
                let digest = expected.as_deref().ok_or_else(|| {
                    RailError::refused("content-match tests require expectedSha256")
                })?;
                if !super::lowercase_hex(digest, 64) {
                    return Err(RailError::refused(
                        "expectedSha256 must be lowercase 64-hex",
                    ));
                }
            } else if expected.is_some() {
                return Err(RailError::refused(
                    "git-diff-check tests accept no expectedSha256",
                ));
            }
            let entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "name": name,
                "kind": kind,
                "workspaceId": workspace_id,
                "relativePath": relative_path,
                "expectedSha256": expected,
                "status": "active",
                "revision": revision,
            });
            Ok((entity, format!("register {kind} test '{name}'")))
        }
        "revoke" => {
            let current = current
                .ok_or_else(|| RailError::refused(format!("test {} does not exist", request.id)))?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("test is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke test — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "tests rail accepts register|revoke, not {other}"
        ))),
    }
}

/// Execute one registered, active test. Read-only: never mutates the
/// workspace; records a hash-chained run receipt binding the observed source.
pub fn run(home: &Path, test_id: &str) -> Result<ChainedReceipt, RailError> {
    let entity = current_entity(home, "tests", test_id)?
        .ok_or_else(|| RailError::refused(format!("test {test_id} does not exist")))?;
    if entity.get("status").and_then(Value::as_str) != Some("active") {
        return Err(RailError::refused(format!("test {test_id} is not active")));
    }
    let kind = entity
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::corrupt("test entity has no kind"))?;
    let workspace_id = entity
        .get("workspaceId")
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::corrupt("test entity has no workspaceId"))?;
    let relative_path = entity
        .get("relativePath")
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::corrupt("test entity has no relativePath"))?;
    let workspace = current_entity(home, "workspaces", workspace_id)?
        .ok_or_else(|| RailError::refused(format!("workspace {workspace_id} is gone")))?;
    if workspace.get("status").and_then(Value::as_str) != Some("active") {
        return Err(RailError::refused(format!(
            "workspace {workspace_id} is not active"
        )));
    }
    let root = workspace
        .get("canonicalPath")
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::corrupt("workspace entity has no canonicalPath"))?;
    let target = Path::new(root).join(relative_path);
    let source_bytes = std::fs::read(&target)
        .map_err(|error| RailError::refused(format!("test source unavailable: {error}")))?;
    let source_digest = hex::encode(Sha256::digest(&source_bytes));

    let (verdict, observed) = match kind {
        "git-diff-check" => {
            let output = Command::new("/usr/bin/git")
                .args(["diff", "--check", "--", relative_path])
                .current_dir(root)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("LANG", "C")
                .env("LC_ALL", "C")
                .output()
                .map_err(|error| RailError::storage(format!("git spawn failed: {error}")))?;
            let verdict = if output.status.success() {
                "PASS"
            } else {
                "FAIL"
            };
            (
                verdict,
                json!({
                    "exitSuccess": output.status.success(),
                    "output": String::from_utf8_lossy(&output.stdout).trim(),
                }),
            )
        }
        "content-match" => {
            let expected = entity
                .get("expectedSha256")
                .and_then(Value::as_str)
                .ok_or_else(|| RailError::corrupt("content-match test has no expectedSha256"))?;
            let verdict = if source_digest == expected {
                "PASS"
            } else {
                "FAIL"
            };
            (verdict, json!({"expectedSha256": expected}))
        }
        other => return Err(RailError::corrupt(format!("unknown test kind {other}"))),
    };

    append_receipt(
        home,
        RUN_CHAIN,
        RUN_SCHEMA,
        test_id,
        verdict,
        json!({
            "kind": kind,
            "workspaceId": workspace_id,
            "relativePath": relative_path,
            "sourceDigest": source_digest,
            "observation": observed,
        }),
    )
}

/// Latest run per test with staleness derived against current source bytes.
pub fn latest_runs(home: &Path) -> Result<Vec<Value>, RailError> {
    let chain = super::load_receipt_chain(home, RUN_CHAIN, RUN_SCHEMA)?;
    let mut latest: Vec<Value> = Vec::new();
    for record in chain.iter().rev() {
        if latest
            .iter()
            .any(|entry| entry.get("testId").and_then(Value::as_str) == Some(&record.subject))
        {
            continue;
        }
        let recorded_source = record
            .detail
            .get("sourceDigest")
            .and_then(Value::as_str)
            .unwrap_or("");
        let stale = match (
            record.detail.get("workspaceId").and_then(Value::as_str),
            record.detail.get("relativePath").and_then(Value::as_str),
        ) {
            (Some(workspace_id), Some(relative_path)) => {
                match current_entity(home, "workspaces", workspace_id)? {
                    Some(workspace) => {
                        let root = workspace
                            .get("canonicalPath")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        match std::fs::read(Path::new(root).join(relative_path)) {
                            Ok(bytes) => hex::encode(Sha256::digest(&bytes)) != recorded_source,
                            Err(_) => true,
                        }
                    }
                    None => true,
                }
            }
            _ => true,
        };
        latest.push(json!({
            "testId": record.subject,
            "verdict": record.verdict,
            "sequence": record.sequence,
            "recordedUnixSeconds": record.recorded_unix_seconds,
            "sourceDigest": recorded_source,
            "stale": stale,
        }));
    }
    latest.reverse();
    Ok(latest)
}
