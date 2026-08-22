//! Workspaces rail — canonical repositories and their isolated lanes.
//!
//! A workspace entity declares one canonical Git repository missions may
//! target. Registration is a governed mutation; repository health (exists,
//! is a Git work tree, current HEAD) is derived at read time and never stored
//! as truth.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use super::{
    MAX_NAME_BYTES, MAX_NOTE_BYTES, RailError, RailMutationRequest, bounded_text, entity_revision,
    entity_status, optional_text,
};

pub const SCHEMA: &str = "nemesis.rail-workspace/v1";
const MAX_PATH_BYTES: usize = 1024;

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
                        "workspace {} already exists with status {status}",
                        request.id
                    )));
                }
            }
            let name = bounded_text(&request.payload, "name", MAX_NAME_BYTES)?;
            let canonical_path = bounded_text(&request.payload, "canonicalPath", MAX_PATH_BYTES)?;
            if !Path::new(&canonical_path).is_absolute() || canonical_path.as_bytes().contains(&0) {
                return Err(RailError::refused(
                    "canonicalPath must be a bounded absolute path",
                ));
            }
            let state_root = crate::state_repo::state_root(home);
            if Path::new(&canonical_path) == state_root {
                return Err(RailError::refused(
                    "the governed state repository cannot be registered as a workspace",
                ));
            }
            let entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "name": name,
                "canonicalPath": canonical_path,
                "status": "active",
                "revision": revision,
            });
            Ok((
                entity,
                format!("register workspace '{name}' at {canonical_path}"),
            ))
        }
        "revoke" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("workspace {} does not exist", request.id))
            })?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("workspace is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke workspace — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "workspaces rail accepts register|revoke, not {other}"
        ))),
    }
}

/// Derived repository health for one workspace entity. Read-only.
pub fn health(entity: &Value) -> Value {
    let path = entity
        .get("canonicalPath")
        .and_then(Value::as_str)
        .unwrap_or("");
    let root = Path::new(path);
    if !root.is_dir() {
        return json!({"reachable": false, "git": false, "head": Value::Null, "clean": Value::Null});
    }
    let git = |args: &[&str]| -> Option<String> {
        let output = Command::new("/usr/bin/git")
            .args(args)
            .current_dir(root)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .output()
            .ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        } else {
            None
        }
    };
    match git(&["rev-parse", "HEAD"]) {
        Some(head) => {
            let clean = git(&["status", "--porcelain=v1", "--untracked-files=all"])
                .map(|status| status.is_empty());
            json!({
                "reachable": true,
                "git": true,
                "head": head,
                "clean": clean,
            })
        }
        None => json!({"reachable": true, "git": false, "head": Value::Null, "clean": Value::Null}),
    }
}
