//! Automations rail — local triggers that stop at unapproved authority.
//!
//! An automation entity binds one mission template to one local trigger. The
//! only operation the schema can express is `draft-mission`: dispatch composes
//! a mission draft for human review and STOPS. Network, secrets, approval,
//! push, publish, and external delivery are unrepresentable, not merely
//! denied. Dispatch is idempotent per trigger key, rate-bounded per minute,
//! and every decision — including refusals — lands in a hash-chained receipt.

use std::path::Path;

use serde_json::{Value, json};

use super::{
    ChainedReceipt, MAX_NAME_BYTES, MAX_NOTE_BYTES, RailError, RailMutationRequest, append_receipt,
    bounded_text, current_entity, entity_revision, entity_status, load_receipt_chain, one_of,
    optional_text, required_u64,
};
use crate::mission_runner::{MissionCancellation, MissionDraftRequest, draft_local_mission};

pub const SCHEMA: &str = "nemesis.rail-automation/v1";
pub const RUN_SCHEMA: &str = "nemesis.automation-run/v1";
pub const RUN_CHAIN: &str = "automation-runs";
pub const TRIGGER_KINDS: &[&str] = &["local-schedule", "manual"];
/// The complete operation vocabulary. Everything else is unrepresentable.
pub const OPERATIONS: &[&str] = &["draft-mission"];
const MAX_DISPATCHES_PER_MINUTE: usize = 6;
const MAX_REPLACEMENT_BYTES: usize = 4096;

pub fn plan(
    home: &Path,
    request: &RailMutationRequest,
    current: Option<&Value>,
) -> Result<(Value, String), RailError> {
    let revision = entity_revision(current) + 1;
    match request.verb.as_str() {
        "enqueue" => {
            if let Some(status) = entity_status(current) {
                if status != "revoked" {
                    return Err(RailError::refused(format!(
                        "automation {} already exists with status {status}",
                        request.id
                    )));
                }
            }
            let name = bounded_text(&request.payload, "name", MAX_NAME_BYTES)?;
            let operation = one_of(&request.payload, "operation", OPERATIONS)?;
            let trigger = request
                .payload
                .get("trigger")
                .ok_or_else(|| RailError::refused("payload field trigger is required"))?;
            let trigger_kind = one_of(trigger, "kind", TRIGGER_KINDS)?;
            let trigger_key = bounded_text(trigger, "key", MAX_NAME_BYTES)?;
            let due = if trigger_kind == "local-schedule" {
                required_u64(trigger, "dueUnixSeconds")?
            } else {
                0
            };
            let template = request
                .payload
                .get("missionTemplate")
                .ok_or_else(|| RailError::refused("payload field missionTemplate is required"))?;
            let goal = bounded_text(template, "goal", 1024)?;
            let workspace_id = bounded_text(template, "workspaceId", 48)?;
            crate::state_repo::validate_entity_id(&workspace_id)?;
            let workspace =
                current_entity(home, "workspaces", &workspace_id)?.ok_or_else(|| {
                    RailError::refused(format!("workspace {workspace_id} is not registered"))
                })?;
            if workspace.get("status").and_then(Value::as_str) != Some("active") {
                return Err(RailError::refused(format!(
                    "workspace {workspace_id} is not active"
                )));
            }
            let relative_path = bounded_text(template, "relativePath", 512)?;
            nemesis_protocol::validate_repository_relative_path(&relative_path)
                .map_err(|error| RailError::refused(format!("relative path refused: {error}")))?;
            let replacement = bounded_text(template, "replacement", MAX_REPLACEMENT_BYTES)?;
            let entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "name": name,
                "operation": operation,
                "trigger": {
                    "kind": trigger_kind,
                    "key": trigger_key,
                    "dueUnixSeconds": due,
                },
                "missionTemplate": {
                    "goal": goal,
                    "workspaceId": workspace_id,
                    "relativePath": relative_path,
                    "replacement": replacement,
                },
                "status": "queued",
                "revision": revision,
            });
            Ok((
                entity,
                format!("enqueue automation '{name}' ({trigger_kind})"),
            ))
        }
        "revoke" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("automation {} does not exist", request.id))
            })?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("automation is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke automation — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "automations rail accepts enqueue|revoke, not {other}"
        ))),
    }
}

fn now_unix() -> Result<u64, RailError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| RailError::storage(error.to_string()))
}

/// Dispatch one automation. Terminal outcomes (all receipted):
/// - `AUTHORIZED_DRAFT_ONLY` — a mission draft now exists for human review;
/// - `DENIED_*` — typed refusal; nothing was drafted.
/// Dispatch NEVER compiles, reviews, approves, or runs the mission.
pub fn dispatch(home: &Path, automation_id: &str) -> Result<ChainedReceipt, RailError> {
    crate::state_repo::validate_entity_id(automation_id)?;
    let record_denial = |verdict: &str, detail: Value| -> Result<ChainedReceipt, RailError> {
        append_receipt(home, RUN_CHAIN, RUN_SCHEMA, automation_id, verdict, detail)
    };
    let Some(entity) = current_entity(home, "automations", automation_id)? else {
        return record_denial("DENIED_UNKNOWN_AUTOMATION", json!({}));
    };
    if entity.get("status").and_then(Value::as_str) != Some("queued") {
        return record_denial("DENIED_NOT_QUEUED", json!({"status": entity.get("status")}));
    }
    let trigger = entity.get("trigger").cloned().unwrap_or_default();
    let trigger_key = trigger
        .get("key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let due = trigger
        .get("dueUnixSeconds")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let now = now_unix()?;
    if trigger.get("kind").and_then(Value::as_str) == Some("local-schedule") && now < due {
        return record_denial(
            "DENIED_NOT_DUE",
            json!({"dueUnixSeconds": due, "nowUnixSeconds": now}),
        );
    }
    let chain = load_receipt_chain(home, RUN_CHAIN, RUN_SCHEMA)?;
    let already_dispatched = chain.iter().any(|record| {
        record.subject == automation_id
            && record.verdict == "AUTHORIZED_DRAFT_ONLY"
            && record.detail.get("triggerKey").and_then(Value::as_str) == Some(&trigger_key)
    });
    if already_dispatched {
        return record_denial(
            "DENIED_DUPLICATE_TRIGGER",
            json!({"triggerKey": trigger_key}),
        );
    }
    let recent = chain
        .iter()
        .filter(|record| {
            record.verdict == "AUTHORIZED_DRAFT_ONLY" && record.recorded_unix_seconds + 60 > now
        })
        .count();
    if recent >= MAX_DISPATCHES_PER_MINUTE {
        return record_denial(
            "DENIED_RATE_LIMITED",
            json!({"recentDispatches": recent, "boundPerMinute": MAX_DISPATCHES_PER_MINUTE}),
        );
    }
    let template = entity.get("missionTemplate").cloned().unwrap_or_default();
    let workspace_id = template
        .get("workspaceId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let Some(workspace) = current_entity(home, "workspaces", &workspace_id)? else {
        return record_denial(
            "DENIED_WORKSPACE_UNKNOWN",
            json!({"workspaceId": workspace_id}),
        );
    };
    if workspace.get("status").and_then(Value::as_str) != Some("active") {
        return record_denial(
            "DENIED_WORKSPACE_INACTIVE",
            json!({"workspaceId": workspace_id}),
        );
    }
    let workspace_path = workspace
        .get("canonicalPath")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let state_root = crate::state_repo::state_root(home);
    if Path::new(&workspace_path) == state_root {
        return record_denial(
            "DENIED_AUTHORITY_BOUNDARY",
            json!({"reason": "automations may not draft against the governed state repository"}),
        );
    }
    let draft_request = MissionDraftRequest {
        goal: template
            .get("goal")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        workspace: workspace_path,
        relative_path: template
            .get("relativePath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        replacement: template
            .get("replacement")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    };
    match draft_local_mission(home, &draft_request, &MissionCancellation::default()) {
        Ok(drafted) => append_receipt(
            home,
            RUN_CHAIN,
            RUN_SCHEMA,
            automation_id,
            "AUTHORIZED_DRAFT_ONLY",
            json!({
                "triggerKey": trigger_key,
                "draftPath": drafted.path,
                "missionId": drafted.compiled.mission_id,
                "contractDigest": drafted.compiled.contract_digest,
                "actionDigest": drafted.compiled.action_digest,
                "boundary": "draft only; compile, review, approval, and run remain human acts",
            }),
        ),
        Err(error) => record_denial(
            "DENIED_DRAFT_REFUSED",
            json!({"triggerKey": trigger_key, "error": error.to_string()}),
        ),
    }
}
