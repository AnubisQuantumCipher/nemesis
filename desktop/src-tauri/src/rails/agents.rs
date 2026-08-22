//! Agents rail — untrusted workers with declared roles and ceilings.
//!
//! An agent entity declares one worker: its provider kind, the roles it may
//! fill, its consequence ceiling, and its usage budget. Agents never carry
//! authority — the kernel authorizes actions, agents only propose. API-backed
//! providers are always derived UNAVAILABLE because network authority is
//! denied by contract.

use std::path::Path;

use serde_json::{Value, json};

use super::{
    MAX_NAME_BYTES, MAX_NOTE_BYTES, RailError, RailMutationRequest, bounded_text, entity_revision,
    entity_status, one_of, optional_text, required_u64,
};

pub const SCHEMA: &str = "nemesis.rail-agent/v1";

pub const PROVIDER_KINDS: &[&str] = &[
    "generic-subprocess",
    "codex-cli",
    "claude-code-cli",
    "local-model",
    "openai-api",
    "anthropic-api",
];
pub const SUBPROCESS_KINDS: &[&str] = &[
    "generic-subprocess",
    "codex-cli",
    "claude-code-cli",
    "local-model",
];
pub const ROLES: &[&str] = &[
    "planner",
    "builder",
    "reviewer",
    "red-team",
    "verifier",
    "integrator",
    "recovery",
];
pub const CEILINGS: &[&str] = &[
    "informational",
    "advisory",
    "decision-boundary",
    "safety-critical",
];

pub fn plan(
    request: &RailMutationRequest,
    current: Option<&Value>,
) -> Result<(Value, String), RailError> {
    let revision = entity_revision(current) + 1;
    match request.verb.as_str() {
        "register" => {
            if let Some(status) = entity_status(current) {
                if status != "revoked" {
                    return Err(RailError::refused(format!(
                        "agent {} already exists with status {status}",
                        request.id
                    )));
                }
            }
            let name = bounded_text(&request.payload, "name", MAX_NAME_BYTES)?;
            let provider_kind = one_of(&request.payload, "providerKind", PROVIDER_KINDS)?;
            let executable_path = optional_text(&request.payload, "executablePath", 1024)?;
            if SUBPROCESS_KINDS.contains(&provider_kind.as_str()) {
                let path = executable_path.as_deref().ok_or_else(|| {
                    RailError::refused("subprocess providers require executablePath")
                })?;
                if !Path::new(path).is_absolute() {
                    return Err(RailError::refused("executablePath must be absolute"));
                }
            } else if executable_path.is_some() {
                return Err(RailError::refused("API providers accept no executablePath"));
            }
            let roles = request
                .payload
                .get("roles")
                .and_then(Value::as_array)
                .ok_or_else(|| RailError::refused("payload field roles must be an array"))?;
            if roles.is_empty() || roles.len() > ROLES.len() {
                return Err(RailError::refused("roles must name 1..7 worker roles"));
            }
            let mut role_names = Vec::new();
            for role in roles {
                let role = role
                    .as_str()
                    .ok_or_else(|| RailError::refused("roles must be strings"))?;
                if !ROLES.contains(&role) {
                    return Err(RailError::refused(format!("unknown role {role}")));
                }
                if role_names.contains(&role.to_owned()) {
                    return Err(RailError::refused(format!("duplicate role {role}")));
                }
                role_names.push(role.to_owned());
            }
            let ceiling = one_of(&request.payload, "consequenceCeiling", CEILINGS)?;
            let budget = request
                .payload
                .get("budget")
                .ok_or_else(|| RailError::refused("payload field budget is required"))?;
            let cost = required_u64(budget, "costMicrounits")?;
            let input_tokens = required_u64(budget, "inputTokens")?;
            let output_tokens = required_u64(budget, "outputTokens")?;
            if cost == 0 || input_tokens == 0 || output_tokens == 0 {
                return Err(RailError::refused("budget fields must be nonzero"));
            }
            let entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "name": name,
                "providerKind": provider_kind,
                "executablePath": executable_path,
                "roles": role_names,
                "consequenceCeiling": ceiling,
                "budget": {
                    "costMicrounits": cost,
                    "inputTokens": input_tokens,
                    "outputTokens": output_tokens,
                },
                "status": "active",
                "revision": revision,
            });
            Ok((
                entity,
                format!("register agent '{name}' ({provider_kind}, ceiling {ceiling})"),
            ))
        }
        "revoke" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("agent {} does not exist", request.id))
            })?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("agent is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke agent — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "agents rail accepts register|revoke, not {other}"
        ))),
    }
}

/// Derived availability. Network authority is denied by contract, so API
/// providers are permanently UNAVAILABLE_NETWORK_DENIED; subprocess providers
/// are READY only when their exact executable exists.
pub fn availability(entity: &Value) -> &'static str {
    let kind = entity
        .get("providerKind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !SUBPROCESS_KINDS.contains(&kind) {
        return "UNAVAILABLE_NETWORK_DENIED";
    }
    let executable = entity
        .get("executablePath")
        .and_then(Value::as_str)
        .unwrap_or("");
    let path = Path::new(executable);
    if path.is_file() {
        "READY"
    } else {
        "MISSING_EXECUTABLE"
    }
}
