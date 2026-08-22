//! Rail domain layer.
//!
//! Each governed rail owns: an entity schema, a verb table, and a validator
//! that turns `(current entity, verb, payload)` into the exact replacement
//! bytes a mission will write. The validator REFUSES anything the rail's
//! state machine forbids; the mission pipeline then carries the bytes through
//! compile → AuthorityReview → TS-001/TS-002 → receipt → replay; adoption
//! copies the kernel-authorized bytes into canonical state.
//!
//! Nothing in this module executes a mutation. It only proposes and refuses.

pub mod agents;
pub mod automations;
pub mod changes;
pub mod integrations;
pub mod knowledge;
pub mod security;
pub mod skills;
pub mod tests_rail;
pub mod workspaces;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::production::atomic_write;
use crate::state_repo::{self, StateRepoError};

pub const MAX_NOTE_BYTES: usize = 256;
pub const MAX_NAME_BYTES: usize = 64;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RailMutationRequest {
    pub rail: String,
    pub verb: String,
    pub id: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RailMutationDraft {
    pub rail: String,
    pub verb: String,
    pub id: String,
    pub goal: String,
    pub relative_path: String,
    pub replacement: String,
}

#[derive(Debug)]
pub enum RailError {
    Refused(String),
    Corrupt(String),
    Storage(String),
}

impl RailError {
    pub fn refused(message: impl Into<String>) -> Self {
        Self::Refused(message.into())
    }
    pub fn corrupt(message: impl Into<String>) -> Self {
        Self::Corrupt(message.into())
    }
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage(message.into())
    }
}

impl From<StateRepoError> for RailError {
    fn from(error: StateRepoError) -> Self {
        match error {
            StateRepoError::Refused(message) => Self::Refused(message),
            StateRepoError::Corrupt(message) => Self::Corrupt(message),
            StateRepoError::Storage(message) => Self::Storage(message),
        }
    }
}

impl std::fmt::Display for RailError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(message) => write!(formatter, "{message}"),
            Self::Corrupt(message) => write!(formatter, "{message}"),
            Self::Storage(message) => write!(formatter, "{message}"),
        }
    }
}

/// Bounded, trimmed, non-empty UTF-8 field.
pub fn bounded_text(value: &Value, key: &str, max: usize) -> Result<String, RailError> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::refused(format!("payload field {key} must be a string")))?
        .trim()
        .to_owned();
    if text.is_empty() || text.len() > max {
        return Err(RailError::refused(format!(
            "payload field {key} must contain 1..{max} bytes"
        )));
    }
    Ok(text)
}

pub fn optional_text(value: &Value, key: &str, max: usize) -> Result<Option<String>, RailError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => bounded_text(value, key, max).map(Some),
    }
}

/// Bounded, non-empty UTF-8 field preserved byte-for-byte (no trimming).
/// Use for content-addressed payloads where every byte is load-bearing.
pub fn raw_text(value: &Value, key: &str, max: usize) -> Result<String, RailError> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| RailError::refused(format!("payload field {key} must be a string")))?
        .to_owned();
    if text.is_empty() || text.len() > max {
        return Err(RailError::refused(format!(
            "payload field {key} must contain 1..{max} bytes"
        )));
    }
    Ok(text)
}

pub fn required_u64(value: &Value, key: &str) -> Result<u64, RailError> {
    value.get(key).and_then(Value::as_u64).ok_or_else(|| {
        RailError::refused(format!("payload field {key} must be an unsigned integer"))
    })
}

pub fn required_bool(value: &Value, key: &str) -> Result<bool, RailError> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| RailError::refused(format!("payload field {key} must be a boolean")))
}

pub fn lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn required_hex(value: &Value, key: &str, length: usize) -> Result<String, RailError> {
    let text = bounded_text(value, key, length)?;
    if !lowercase_hex(&text, length) {
        return Err(RailError::refused(format!(
            "payload field {key} must be lowercase {length}-hex"
        )));
    }
    Ok(text)
}

pub fn one_of(value: &Value, key: &str, allowed: &[&str]) -> Result<String, RailError> {
    let text = bounded_text(value, key, MAX_NAME_BYTES)?;
    if allowed.contains(&text.as_str()) {
        Ok(text)
    } else {
        Err(RailError::refused(format!(
            "payload field {key} must be one of {allowed:?}"
        )))
    }
}

/// Current parsed entity, if any. Empty placeholder files read as absent.
pub fn current_entity(home: &Path, rail: &str, id: &str) -> Result<Option<Value>, RailError> {
    match state_repo::entity_current(home, rail, id)? {
        None => Ok(None),
        Some((_, bytes)) if bytes.is_empty() => Ok(None),
        Some((_, bytes)) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| RailError::corrupt(format!("entity {rail}/{id} malformed: {error}"))),
    }
}

pub fn entity_revision(current: Option<&Value>) -> u64 {
    current
        .and_then(|value| value.get("revision"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

pub fn entity_status(current: Option<&Value>) -> Option<String> {
    current
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Serialize an entity candidate to the exact replacement bytes a mission
/// will write, enforcing the governed entity size bound.
pub fn seal_entity(entity: &Value) -> Result<String, RailError> {
    let mut text = serde_json::to_string_pretty(entity)
        .map_err(|error| RailError::storage(error.to_string()))?;
    text.push('\n');
    if text.len() as u64 > state_repo::MAX_ENTITY_BYTES {
        return Err(RailError::refused(format!(
            "entity exceeds the governed {} byte bound",
            state_repo::MAX_ENTITY_BYTES
        )));
    }
    Ok(text)
}

/// Validate one rail mutation and produce the exact mission draft inputs.
pub fn plan_mutation(
    home: &Path,
    request: &RailMutationRequest,
) -> Result<RailMutationDraft, RailError> {
    if !state_repo::rail_is_governed(&request.rail) {
        return Err(RailError::refused(format!(
            "rail {} accepts no governed mutations",
            request.rail
        )));
    }
    state_repo::validate_entity_id(&request.id)?;
    let current = current_entity(home, &request.rail, &request.id)?;
    let (entity, summary) = match request.rail.as_str() {
        "workspaces" => workspaces::plan(home, request, current.as_ref())?,
        "agents" => agents::plan(request, current.as_ref())?,
        "tests" => tests_rail::plan(home, request, current.as_ref())?,
        "knowledge" => knowledge::plan(home, request, current.as_ref())?,
        "skills" => skills::plan(home, request, current.as_ref())?,
        "automations" => automations::plan(home, request, current.as_ref())?,
        "integrations" => integrations::plan(home, request, current.as_ref())?,
        other => return Err(RailError::refused(format!("unknown governed rail {other}"))),
    };
    let replacement = seal_entity(&entity)?;
    let relative_path = state_repo::entity_relative_path(&request.rail, &request.id)?;
    let goal = format!(
        "RAIL {} {} {} — {}",
        request.rail.to_uppercase(),
        request.verb.to_uppercase(),
        request.id,
        summary
    );
    if goal.len() > 1024 {
        return Err(RailError::refused("mutation goal exceeds 1024 bytes"));
    }
    Ok(RailMutationDraft {
        rail: request.rail.clone(),
        verb: request.verb.clone(),
        id: request.id.clone(),
        goal,
        relative_path,
        replacement,
    })
}

/// Hash-chained JSONL receipt stream under `<home>/receipts/<name>.jsonl`.
/// Same chain discipline as the adoption chain: sequence, previous, entry hash.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChainedReceipt {
    pub schema: String,
    pub sequence: u64,
    pub subject: String,
    pub verdict: String,
    pub detail: Value,
    pub recorded_unix_seconds: u64,
    pub previous: String,
    pub entry_hash: String,
}

fn receipt_hash(record: &ChainedReceipt) -> Result<String, RailError> {
    let mut unsealed = record.clone();
    unsealed.entry_hash = String::new();
    let bytes =
        serde_json::to_vec(&unsealed).map_err(|error| RailError::storage(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub fn load_receipt_chain(
    home: &Path,
    name: &str,
    schema: &str,
) -> Result<Vec<ChainedReceipt>, RailError> {
    let path = home.join("receipts").join(format!("{name}.jsonl"));
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(RailError::storage(error.to_string())),
    };
    let mut records = Vec::new();
    let mut previous = state_repo::GENESIS_HASH.to_owned();
    for (index, line) in text.lines().enumerate() {
        let record: ChainedReceipt = serde_json::from_str(line).map_err(|error| {
            RailError::corrupt(format!("receipt {name}:{index} malformed: {error}"))
        })?;
        if record.schema != schema
            || record.sequence != (index as u64) + 1
            || record.previous != previous
            || receipt_hash(&record)? != record.entry_hash
        {
            return Err(RailError::corrupt(format!(
                "receipt chain {name} breaks at record {index}"
            )));
        }
        previous = record.entry_hash.clone();
        records.push(record);
    }
    Ok(records)
}

pub fn append_receipt(
    home: &Path,
    name: &str,
    schema: &str,
    subject: &str,
    verdict: &str,
    detail: Value,
) -> Result<ChainedReceipt, RailError> {
    let chain = load_receipt_chain(home, name, schema)?;
    let previous = chain
        .last()
        .map(|record| record.entry_hash.clone())
        .unwrap_or_else(|| state_repo::GENESIS_HASH.to_owned());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| RailError::storage(error.to_string()))?
        .as_secs();
    let mut record = ChainedReceipt {
        schema: schema.to_owned(),
        sequence: chain.len() as u64 + 1,
        subject: subject.to_owned(),
        verdict: verdict.to_owned(),
        detail,
        recorded_unix_seconds: now,
        previous,
        entry_hash: String::new(),
    };
    record.entry_hash = receipt_hash(&record)?;
    let mut line =
        serde_json::to_vec(&record).map_err(|error| RailError::storage(error.to_string()))?;
    line.push(b'\n');
    let path = home.join("receipts").join(format!("{name}.jsonl"));
    let mut existing = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(RailError::storage(error.to_string())),
    };
    existing.extend_from_slice(&line);
    atomic_write(&path, &existing).map_err(|error| RailError::storage(error.to_string()))?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NONCE: AtomicU64 = AtomicU64::new(0);

    fn scratch_home(name: &str) -> std::path::PathBuf {
        let nonce = NONCE.fetch_add(1, Ordering::SeqCst);
        let home = std::env::temp_dir().join(format!(
            "nemesis-rails-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(home.join("receipts")).unwrap();
        state_repo::initialize_state_repo(&home).unwrap();
        home
    }

    fn request(
        rail: &str,
        verb: &str,
        id: &str,
        payload: serde_json::Value,
    ) -> RailMutationRequest {
        RailMutationRequest {
            rail: rail.to_owned(),
            verb: verb.to_owned(),
            id: id.to_owned(),
            payload,
        }
    }

    /// Adopt a planned mutation directly (test-only shortcut past the mission
    /// pipeline): writes the sealed entity bytes into canonical state.
    fn apply(home: &std::path::Path, mutation: &RailMutationRequest) {
        let plan = plan_mutation(home, mutation).unwrap();
        let path = state_repo::state_root(home).join(&plan.relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, plan.replacement).unwrap();
    }

    #[test]
    fn skills_ladder_cannot_be_skipped() {
        let home = scratch_home("skills");
        let propose = request(
            "skills",
            "propose",
            "alpha",
            json!({"name": "Alpha", "body": "# procedure"}),
        );
        apply(&home, &propose);
        // Skipping straight to trusted refuses.
        let skip = request("skills", "advance", "alpha", json!({"toStatus": "trusted"}));
        let error = plan_mutation(&home, &skip).unwrap_err();
        assert!(error.to_string().contains("cannot be skipped"));
        // The single legal next step succeeds.
        let step = request("skills", "advance", "alpha", json!({"toStatus": "staged"}));
        let (entity, _) = skills::plan(
            &home,
            &step,
            current_entity(&home, "skills", "alpha").unwrap().as_ref(),
        )
        .unwrap();
        assert_eq!(entity["status"], "staged");
        // Approval without a nonzero signature refuses.
        for pre in [
            "staged",
            "statically-checked",
            "tested-in-sandbox",
            "evaluated",
        ] {
            apply(
                &home,
                &request("skills", "advance", "alpha", json!({"toStatus": pre})),
            );
        }
        let unsigned = request(
            "skills",
            "advance",
            "alpha",
            json!({"toStatus": "approved", "approvedBy": "op", "approvalSignature": "0".repeat(128)}),
        );
        assert!(plan_mutation(&home, &unsigned).is_err());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn skill_bodies_are_content_addressed_and_verified() {
        let home = scratch_home("body");
        let propose = request(
            "skills",
            "propose",
            "beta",
            json!({"name": "Beta", "body": "exact body"}),
        );
        apply(&home, &propose);
        let entity = current_entity(&home, "skills", "beta").unwrap().unwrap();
        let digest = entity["bodyDigest"].as_str().unwrap();
        assert_eq!(skills::load_body(&home, digest).unwrap(), "exact body");
        // Tamper the stored body: integrity must fail closed.
        fs::write(skills::content_path(&home, digest), "tampered").unwrap();
        assert!(skills::load_body(&home, digest).is_err());
        let integrity = skills::body_integrity(&home, &entity);
        assert_eq!(integrity["bodyVerified"], false);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn integrations_capabilities_are_deny_by_default() {
        let home = scratch_home("caps");
        let grasping = request(
            "integrations",
            "register",
            "grabby",
            json!({
                "name": "Grabby",
                "kind": "wasm-plugin",
                "moduleWat": "(module)",
                "export": "run",
                "tools": [{"name": "echo", "schemaDigest": "a".repeat(64)}],
                "capabilities": {"networkHosts": ["api.example.com"], "filesystemRoots": [], "secrets": [], "events": []},
            }),
        );
        let error = plan_mutation(&home, &grasping).unwrap_err();
        assert!(error.to_string().contains("deny-by-default"));
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn wasm_plugin_executes_bounded_and_refuses_tamper() {
        let home = scratch_home("wasm");
        let wat = "(module (func (export \"attest\") (result i32) i32.const 88))";
        let register = request(
            "integrations",
            "register",
            "attestor",
            json!({
                "name": "Attestor",
                "kind": "wasm-plugin",
                "moduleWat": wat,
                "export": "attest",
                "tools": [{"name": "attest", "schemaDigest": "b".repeat(64)}],
            }),
        );
        apply(&home, &register);
        apply(
            &home,
            &request("integrations", "enable", "attestor", json!({})),
        );
        let receipt = integrations::execute_plugin(&home, "attestor").unwrap();
        assert_eq!(receipt.verdict, "EXECUTED");
        assert_eq!(receipt.detail["result"], 88);
        // Tamper the stored module: execution must refuse before running.
        let entity = current_entity(&home, "integrations", "attestor")
            .unwrap()
            .unwrap();
        let digest = entity["moduleDigest"].as_str().unwrap();
        fs::write(integrations::module_path(&home, digest), "(module)").unwrap();
        assert!(integrations::execute_plugin(&home, "attestor").is_err());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn integration_lifecycle_is_ordered() {
        let home = scratch_home("lifecycle");
        let register = request(
            "integrations",
            "register",
            "prov",
            json!({
                "name": "Provider",
                "kind": "external-provider",
                "providerKind": "codex-cli",
                "tools": [{"name": "shell", "schemaDigest": "c".repeat(64)}],
            }),
        );
        apply(&home, &register);
        // disable before enable refuses; enable then disable succeeds.
        assert!(
            plan_mutation(
                &home,
                &request("integrations", "disable", "prov", json!({}))
            )
            .is_err()
        );
        apply(&home, &request("integrations", "enable", "prov", json!({})));
        assert!(
            plan_mutation(
                &home,
                &request("integrations", "disable", "prov", json!({}))
            )
            .is_ok()
        );
        // Executing a non-wasm integration refuses.
        assert!(integrations::execute_plugin(&home, "prov").is_err());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn knowledge_transitions_and_contradictions() {
        let home = scratch_home("knowledge");
        let assert_entry = |id: &str, value: &str| {
            request(
                "knowledge",
                "assert",
                id,
                json!({
                    "subject": "daemon", "predicate": "socket-mode", "value": value,
                    "scope": "global", "sourceDigest": "d".repeat(64),
                    "observedSequence": 4, "untrustedExternal": false,
                }),
            )
        };
        apply(&home, &assert_entry("fact-a", "0600"));
        apply(
            &home,
            &request(
                "knowledge",
                "transition",
                "fact-a",
                json!({"toStatus": "observed"}),
            ),
        );
        // verified cannot revert to observed.
        apply(
            &home,
            &request(
                "knowledge",
                "transition",
                "fact-a",
                json!({"toStatus": "verified"}),
            ),
        );
        let backwards = request(
            "knowledge",
            "transition",
            "fact-a",
            json!({"toStatus": "observed"}),
        );
        assert!(plan_mutation(&home, &backwards).is_err());
        // Conflicting active fact surfaces as a contradiction pair.
        apply(&home, &assert_entry("fact-b", "0644"));
        apply(
            &home,
            &request(
                "knowledge",
                "transition",
                "fact-b",
                json!({"toStatus": "observed"}),
            ),
        );
        let entities = state_repo::read_rail_entities(&home, "knowledge").unwrap();
        let findings = knowledge::contradictions(&entities);
        assert_eq!(findings.len(), 1);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn automation_dispatch_stops_at_unapproved_authority() {
        let home = scratch_home("dispatch");
        // A real user workspace with one committed file.
        let workspace = home.join("user-repo");
        fs::create_dir_all(&workspace).unwrap();
        let git = |args: &[&str]| {
            let output = std::process::Command::new("/usr/bin/git")
                .args(args)
                .current_dir(&workspace)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", &workspace)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .unwrap();
            assert!(output.status.success(), "git {args:?} failed");
        };
        git(&["init", "--quiet", "--initial-branch=main"]);
        git(&["config", "user.name", "t"]);
        git(&["config", "user.email", "t@local"]);
        fs::write(workspace.join("value.txt"), "before").unwrap();
        git(&["add", "--all"]);
        git(&["commit", "--quiet", "--no-verify", "-m", "seed"]);
        apply(
            &home,
            &request(
                "workspaces",
                "register",
                "user-repo",
                json!({"name": "User repo", "canonicalPath": workspace.display().to_string()}),
            ),
        );
        apply(
            &home,
            &request(
                "automations",
                "enqueue",
                "nightly",
                json!({
                    "name": "Nightly draft",
                    "operation": "draft-mission",
                    "trigger": {"kind": "manual", "key": "run-1"},
                    "missionTemplate": {
                        "goal": "Replace the value",
                        "workspaceId": "user-repo",
                        "relativePath": "value.txt",
                        "replacement": "after",
                    },
                }),
            ),
        );
        let receipt = automations::dispatch(&home, "nightly").unwrap();
        assert_eq!(
            receipt.verdict, "AUTHORIZED_DRAFT_ONLY",
            "{:?}",
            receipt.detail
        );
        // The draft exists; the mission was never compiled, approved, or run.
        let draft_path = receipt.detail["draftPath"].as_str().unwrap();
        assert!(std::path::Path::new(draft_path).is_file());
        assert!(!home.join("missions").exists());
        // Same trigger key refuses a second dispatch.
        let replay = automations::dispatch(&home, "nightly").unwrap();
        assert_eq!(replay.verdict, "DENIED_DUPLICATE_TRIGGER");
        // Operations beyond draft-mission are unrepresentable.
        let forbidden = request(
            "automations",
            "enqueue",
            "exfil",
            json!({
                "name": "Exfil",
                "operation": "external-delivery",
                "trigger": {"kind": "manual", "key": "x"},
                "missionTemplate": {
                    "goal": "g", "workspaceId": "user-repo",
                    "relativePath": "value.txt", "replacement": "y",
                },
            }),
        );
        assert!(plan_mutation(&home, &forbidden).is_err());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn entities_are_size_bounded() {
        let home = scratch_home("bound");
        let oversized = request(
            "knowledge",
            "assert",
            "big",
            json!({
                "subject": "s", "predicate": "p", "value": "v".repeat(1024),
                "scope": "global", "sourceDigest": "e".repeat(64),
                "observedSequence": 1, "untrustedExternal": false,
            }),
        );
        // 1024-byte value fits the field bound but the sealed entity must
        // stay within the governed 4096-byte replacement budget.
        match plan_mutation(&home, &oversized) {
            Ok(plan) => assert!(plan.replacement.len() as u64 <= state_repo::MAX_ENTITY_BYTES),
            Err(error) => assert!(error.to_string().contains("byte bound")),
        }
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn receipt_chain_rejects_tamper() {
        let home = scratch_home("chain");
        append_receipt(
            &home,
            "probe",
            "nemesis.test-run/v1",
            "s",
            "PASS",
            json!({}),
        )
        .unwrap();
        append_receipt(
            &home,
            "probe",
            "nemesis.test-run/v1",
            "s",
            "FAIL",
            json!({}),
        )
        .unwrap();
        assert_eq!(
            load_receipt_chain(&home, "probe", "nemesis.test-run/v1")
                .unwrap()
                .len(),
            2
        );
        let path = home.join("receipts/probe.jsonl");
        let mut bytes = fs::read(&path).unwrap();
        let target = bytes.len() / 3;
        bytes[target] ^= 0x01;
        fs::write(&path, &bytes).unwrap();
        assert!(load_receipt_chain(&home, "probe", "nemesis.test-run/v1").is_err());
        fs::remove_dir_all(&home).unwrap();
    }
}
