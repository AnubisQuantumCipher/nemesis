//! Integrations rail — providers expose tools; NEMESIS retains authority.
//!
//! Two integration kinds:
//! - `wasm-plugin`: a NEMESIS-owned plugin. Reviewable WebAssembly text is
//!   content-addressed at `state/content/<sha256>.wat`; the governed entity
//!   binds the module digest, exports, and an all-empty capability set. The
//!   TS-001 one-shot approval on the registering mission is the trust anchor
//!   that binds those exact manifest bytes — execution re-verifies the module
//!   digest and runs inside the bounded `WasiPluginHost` (no network, no
//!   filesystem, no secrets, no events, metered fuel).
//! - `external-provider`: a declared external tool provider. Tool schemas are
//!   digest-frozen at registration (MCP-gateway discipline); capabilities are
//!   deny-by-default and unrepresentable beyond empty in v1.
//!
//! Lifecycle: register → enable → disable → enable …; revoke is terminal.

use std::fs;
use std::path::Path;

use nemesis_runtime::{PluginCapabilities, WasiPluginHost};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    ChainedReceipt, MAX_NAME_BYTES, MAX_NOTE_BYTES, RailError, RailMutationRequest, append_receipt,
    bounded_text, current_entity, entity_revision, entity_status, lowercase_hex, one_of,
    optional_text,
};
use crate::production::atomic_write;
use crate::state_repo::state_root;

pub const SCHEMA: &str = "nemesis.rail-integration/v1";
pub const RUN_SCHEMA: &str = "nemesis.plugin-run/v1";
pub const RUN_CHAIN: &str = "plugin-runs";
pub const KINDS: &[&str] = &["wasm-plugin", "external-provider"];
pub const PROVIDER_KINDS: &[&str] = &[
    "codex-cli",
    "claude-code-cli",
    "local-model",
    "openai-api",
    "anthropic-api",
];
const MAX_MODULE_BYTES: usize = 1024 * 1024;
const MAX_TOOLS: usize = 16;
const PLUGIN_FUEL: u64 = 1_000_000;
const PLUGIN_MEMORY_BYTES: usize = 16 * 1024 * 1024;

pub fn module_path(home: &Path, digest: &str) -> std::path::PathBuf {
    state_root(home)
        .join("content")
        .join(format!("{digest}.wat"))
}

fn store_module(home: &Path, module_text: &str) -> Result<(String, u64), RailError> {
    if module_text.is_empty() || module_text.len() > MAX_MODULE_BYTES {
        return Err(RailError::refused(format!(
            "module text must contain 1..{MAX_MODULE_BYTES} bytes"
        )));
    }
    let digest = hex::encode(Sha256::digest(module_text.as_bytes()));
    let path = module_path(home, &digest);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| RailError::storage(error.to_string()))?;
    }
    match fs::read(&path) {
        Ok(existing) => {
            if existing != module_text.as_bytes() {
                return Err(RailError::corrupt(format!(
                    "content store digest collision at {digest}"
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            atomic_write(&path, module_text.as_bytes())
                .map_err(|error| RailError::storage(error.to_string()))?;
        }
        Err(error) => return Err(RailError::storage(error.to_string())),
    }
    Ok((digest, module_text.len() as u64))
}

fn validate_tools(payload: &Value) -> Result<Vec<Value>, RailError> {
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| RailError::refused("payload field tools must be an array"))?;
    if tools.is_empty() || tools.len() > MAX_TOOLS {
        return Err(RailError::refused(format!(
            "tools must declare 1..{MAX_TOOLS} entries"
        )));
    }
    let mut sealed = Vec::new();
    let mut names = Vec::new();
    for tool in tools {
        let name = bounded_text(tool, "name", MAX_NAME_BYTES)?;
        if names.contains(&name) {
            return Err(RailError::refused(format!("duplicate tool name {name}")));
        }
        let schema_digest = bounded_text(tool, "schemaDigest", 64)?;
        if !lowercase_hex(&schema_digest, 64) {
            return Err(RailError::refused(
                "tool schemaDigest must be lowercase 64-hex",
            ));
        }
        names.push(name.clone());
        sealed.push(json!({"name": name, "schemaDigest": schema_digest}));
    }
    Ok(sealed)
}

/// Deny-by-default is structural: the only accepted capability object is the
/// empty one. A request naming any host, root, secret, or event refuses.
fn validate_capabilities(payload: &Value) -> Result<Value, RailError> {
    let capabilities = payload.get("capabilities").cloned().unwrap_or(json!({
        "networkHosts": [], "filesystemRoots": [], "secrets": [], "events": []
    }));
    for key in ["networkHosts", "filesystemRoots", "secrets", "events"] {
        let list = capabilities
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| RailError::refused(format!("capabilities.{key} must be an array")))?;
        if !list.is_empty() {
            return Err(RailError::refused(format!(
                "capability {key} is deny-by-default; grants are unrepresentable in v1"
            )));
        }
    }
    Ok(capabilities)
}

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
                        "integration {} already exists with status {status}",
                        request.id
                    )));
                }
            }
            let name = bounded_text(&request.payload, "name", MAX_NAME_BYTES)?;
            let kind = one_of(&request.payload, "kind", KINDS)?;
            let tools = validate_tools(&request.payload)?;
            let capabilities = validate_capabilities(&request.payload)?;
            let mut entity = json!({
                "schema": SCHEMA,
                "id": request.id,
                "name": name,
                "kind": kind,
                "tools": tools,
                "capabilities": capabilities,
                "status": "proposed",
                "revision": revision,
            });
            match kind.as_str() {
                "wasm-plugin" => {
                    let module_text =
                        super::raw_text(&request.payload, "moduleWat", MAX_MODULE_BYTES)?;
                    let export = bounded_text(&request.payload, "export", MAX_NAME_BYTES)?;
                    let (digest, bytes) = store_module(home, &module_text)?;
                    entity["moduleDigest"] = Value::String(digest.clone());
                    entity["moduleBytes"] = Value::from(bytes);
                    entity["export"] = Value::String(export);
                    Ok((
                        entity,
                        format!("register wasm plugin '{name}' module {digest}"),
                    ))
                }
                "external-provider" => {
                    let provider_kind = one_of(&request.payload, "providerKind", PROVIDER_KINDS)?;
                    entity["providerKind"] = Value::String(provider_kind.clone());
                    Ok((
                        entity,
                        format!("register external provider '{name}' ({provider_kind})"),
                    ))
                }
                other => Err(RailError::refused(format!(
                    "unknown integration kind {other}"
                ))),
            }
        }
        "enable" | "disable" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("integration {} does not exist", request.id))
            })?;
            let status = entity_status(Some(current))
                .ok_or_else(|| RailError::corrupt("integration entity has no status"))?;
            let target = if request.verb == "enable" {
                "enabled"
            } else {
                "disabled"
            };
            let legal = matches!(
                (status.as_str(), target),
                ("proposed", "enabled") | ("disabled", "enabled") | ("enabled", "disabled")
            );
            if !legal {
                return Err(RailError::refused(format!(
                    "integration status {status} cannot become {target}"
                )));
            }
            let mut entity = current.clone();
            entity["status"] = Value::String(target.to_owned());
            entity["revision"] = Value::from(revision);
            Ok((entity, format!("{target} integration")))
        }
        "revoke" => {
            let current = current.ok_or_else(|| {
                RailError::refused(format!("integration {} does not exist", request.id))
            })?;
            if entity_status(Some(current)).as_deref() == Some("revoked") {
                return Err(RailError::refused("integration is already revoked"));
            }
            let reason = optional_text(&request.payload, "reason", MAX_NOTE_BYTES)?
                .unwrap_or_else(|| "operator revocation".to_owned());
            let mut entity = current.clone();
            entity["status"] = Value::String("revoked".to_owned());
            entity["revision"] = Value::from(revision);
            entity["revocationReason"] = Value::String(reason.clone());
            Ok((entity, format!("revoke integration — {reason}")))
        }
        other => Err(RailError::refused(format!(
            "integrations rail accepts register|enable|disable|revoke, not {other}"
        ))),
    }
}

/// Execute one ENABLED wasm plugin inside the bounded WASI host. Read-only
/// compute: empty capability set, metered fuel, bounded memory. The module is
/// re-verified against the kernel-bound digest before every run, and the
/// outcome is appended to a hash-chained receipt stream.
pub fn execute_plugin(home: &Path, integration_id: &str) -> Result<ChainedReceipt, RailError> {
    crate::state_repo::validate_entity_id(integration_id)?;
    let entity = current_entity(home, "integrations", integration_id)?.ok_or_else(|| {
        RailError::refused(format!("integration {integration_id} does not exist"))
    })?;
    if entity.get("kind").and_then(Value::as_str) != Some("wasm-plugin") {
        return Err(RailError::refused("only wasm-plugin integrations execute"));
    }
    if entity.get("status").and_then(Value::as_str) != Some("enabled") {
        return Err(RailError::refused(format!(
            "integration {integration_id} is not enabled"
        )));
    }
    let digest = entity
        .get("moduleDigest")
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::corrupt("wasm plugin has no moduleDigest"))?;
    let export = entity
        .get("export")
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::corrupt("wasm plugin has no export"))?;
    let module_bytes = fs::read(module_path(home, digest))
        .map_err(|error| RailError::corrupt(format!("plugin module unavailable: {error}")))?;
    if hex::encode(Sha256::digest(&module_bytes)) != digest {
        return Err(RailError::corrupt(
            "plugin module does not match the kernel-bound digest",
        ));
    }
    let host = WasiPluginHost::new(
        PluginCapabilities::default(),
        PLUGIN_FUEL,
        PLUGIN_MEMORY_BYTES,
    )
    .map_err(|error| RailError::storage(format!("plugin host: {error:?}")))?;
    let (verdict, detail) = match host.execute_i32(&module_bytes, export) {
        Ok(value) => (
            "EXECUTED",
            json!({
                "export": export,
                "result": value,
                "moduleDigest": digest,
                "fuelBound": PLUGIN_FUEL,
                "memoryBoundBytes": PLUGIN_MEMORY_BYTES,
                "capabilities": "EMPTY_DENY_ALL",
            }),
        ),
        Err(error) => (
            "REFUSED",
            json!({
                "export": export,
                "moduleDigest": digest,
                "error": format!("{error:?}"),
            }),
        ),
    };
    append_receipt(home, RUN_CHAIN, RUN_SCHEMA, integration_id, verdict, detail)
}
