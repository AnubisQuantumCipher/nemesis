//! Governed rail-state repository.
//!
//! Rail state lives as small JSON entity files inside a private Git repository
//! at `<home>/state`. Every mutation of a governed entity travels the existing
//! mission pipeline — compile → AuthorityReview → TS-001 one-shot approval →
//! TS-002 attenuated child grant → SPARK-authorized lane write → receipt →
//! replay — and only then is the kernel-authorized byte string *adopted* into
//! the canonical state file. Adoption is mechanical: it verifies the mission
//! terminal result and copies exactly the bytes whose digest the kernel
//! authorized. The desktop never decides content; it executes decisions.
//!
//! Fail-closed rules:
//! - unreadable / oversized / non-JSON entity files refuse the whole read;
//! - adoption refuses on any digest, workspace, or verdict mismatch;
//! - the adoption receipt chain is hash-linked and verified before append.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::production::{GovernedWriteLock, atomic_write, compile_local_contract};

pub const STATE_DIR: &str = "state";
pub const ADOPTION_CHAIN: &str = "receipts/adoptions.jsonl";
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub const MAX_ENTITY_BYTES: u64 = 4096;
const MAX_CHAIN_BYTES: u64 = 16 * 1024 * 1024;
const REGISTRY_SCHEMA: &str = "nemesis.rail-registry/v1";
const ADOPTION_SCHEMA: &str = "nemesis.rail-adoption/v1";

/// Governed rails: each owns a directory of entity files inside the state
/// repository. `changes` and `security` are derived read-only views and hold
/// no governed entities.
pub const GOVERNED_RAILS: &[&str] = &[
    "workspaces",
    "agents",
    "tests",
    "knowledge",
    "skills",
    "automations",
    "integrations",
];

#[derive(Debug)]
pub enum StateRepoError {
    Storage(String),
    Corrupt(String),
    Refused(String),
}

impl StateRepoError {
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage(message.into())
    }
    pub fn corrupt(message: impl Into<String>) -> Self {
        Self::Corrupt(message.into())
    }
    pub fn refused(message: impl Into<String>) -> Self {
        Self::Refused(message.into())
    }
}

impl std::fmt::Display for StateRepoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "state storage failure: {message}"),
            Self::Corrupt(message) => write!(formatter, "state corruption: {message}"),
            Self::Refused(message) => write!(formatter, "state refusal: {message}"),
        }
    }
}

pub fn state_root(home: &Path) -> PathBuf {
    home.join(STATE_DIR)
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, StateRepoError> {
    let output = Command::new("/usr/bin/git")
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("HOME", root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .map_err(|error| StateRepoError::storage(format!("git spawn failed: {error}")))?;
    if !output.status.success() {
        return Err(StateRepoError::storage(format!(
            "git {} failed: {}",
            args.first().copied().unwrap_or("?"),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn validate_entity_id(id: &str) -> Result<(), StateRepoError> {
    let bytes = id.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 48
        && bytes[0].is_ascii_lowercase() | bytes[0].is_ascii_digit()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(StateRepoError::refused(
            "entity id must be 1..48 of [a-z0-9-] starting alphanumeric",
        ))
    }
}

pub fn rail_is_governed(rail: &str) -> bool {
    GOVERNED_RAILS.contains(&rail)
}

pub fn entity_relative_path(rail: &str, id: &str) -> Result<String, StateRepoError> {
    if !rail_is_governed(rail) {
        return Err(StateRepoError::refused(format!(
            "rail {rail} holds no governed entities"
        )));
    }
    validate_entity_id(id)?;
    Ok(format!("{rail}/{id}.json"))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateRepoStatus {
    pub root: String,
    pub head: String,
    pub clean: bool,
}

/// Create the governed state repository if absent; verify identity if present.
/// Idempotent. Registry seeds give every governed rail an anchor file so the
/// repository is never empty and rails are individually versioned.
pub fn initialize_state_repo(home: &Path) -> Result<StateRepoStatus, StateRepoError> {
    let root = state_root(home);
    fs::create_dir_all(&root)
        .map_err(|error| StateRepoError::storage(format!("create state root: {error}")))?;
    if !root.join(".git").is_dir() {
        run_git(&root, &["init", "--quiet", "--initial-branch=main"])?;
        run_git(&root, &["config", "user.name", "NEMESIS Local"])?;
        run_git(&root, &["config", "user.email", "nemesis@local"])?;
        run_git(&root, &["config", "commit.gpgsign", "false"])?;
    }
    for rail in GOVERNED_RAILS {
        let rail_dir = root.join(rail);
        fs::create_dir_all(&rail_dir)
            .map_err(|error| StateRepoError::storage(format!("create rail dir: {error}")))?;
        let registry = rail_dir.join("registry.json");
        if !registry.exists() {
            let seed = serde_json::json!({
                "schema": REGISTRY_SCHEMA,
                "rail": rail,
                "version": 1,
            });
            let bytes = serde_json::to_vec_pretty(&seed)
                .map_err(|error| StateRepoError::storage(error.to_string()))?;
            atomic_write(&registry, &bytes)
                .map_err(|error| StateRepoError::storage(error.to_string()))?;
        }
    }
    let status = run_git(
        &root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    if !status.is_empty() {
        run_git(&root, &["add", "--all"])?;
        run_git(
            &root,
            &[
                "commit",
                "--quiet",
                "--no-verify",
                "-m",
                "plumbing: seed governed rail registries",
            ],
        )?;
    }
    let head = run_git(&root, &["rev-parse", "HEAD"])?;
    let clean = run_git(
        &root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?
    .is_empty();
    Ok(StateRepoStatus {
        root: root.display().to_string(),
        head,
        clean,
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RailEntity {
    pub id: String,
    pub relative_path: String,
    pub sha256: String,
    pub bytes: u64,
    pub value: Value,
}

/// Read every governed entity of one rail. Any unreadable, oversized, empty,
/// or non-JSON entity file refuses the whole read — a corrupt store must be
/// visible, never silently filtered.
pub fn read_rail_entities(home: &Path, rail: &str) -> Result<Vec<RailEntity>, StateRepoError> {
    if !rail_is_governed(rail) {
        return Err(StateRepoError::refused(format!(
            "rail {rail} holds no governed entities"
        )));
    }
    let rail_dir = state_root(home).join(rail);
    let mut entities = Vec::new();
    let entries = fs::read_dir(&rail_dir)
        .map_err(|error| StateRepoError::storage(format!("read rail dir {rail}: {error}")))?;
    for entry in entries {
        let entry = entry.map_err(|error| StateRepoError::storage(error.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "registry.json" || !name.ends_with(".json") {
            continue;
        }
        let id = name.trim_end_matches(".json").to_owned();
        validate_entity_id(&id)
            .map_err(|_| StateRepoError::corrupt(format!("foreign file in {rail}: {name}")))?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| StateRepoError::storage(error.to_string()))?;
        if !metadata.is_file() {
            return Err(StateRepoError::corrupt(format!(
                "{rail}/{name} is not a regular file"
            )));
        }
        if metadata.len() > MAX_ENTITY_BYTES {
            return Err(StateRepoError::corrupt(format!(
                "{rail}/{name} exceeds {MAX_ENTITY_BYTES} bytes"
            )));
        }
        let bytes =
            fs::read(entry.path()).map_err(|error| StateRepoError::storage(error.to_string()))?;
        if bytes.is_empty() {
            // Placeholder awaiting its first authorized adoption; not active.
            continue;
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
            StateRepoError::corrupt(format!("{rail}/{name} is not valid JSON: {error}"))
        })?;
        entities.push(RailEntity {
            id,
            relative_path: format!("{rail}/{name}"),
            sha256: hex::encode(Sha256::digest(&bytes)),
            bytes: metadata.len(),
            value,
        });
    }
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(entities)
}

/// Current canonical bytes of one entity file, if the file exists.
pub fn entity_current(
    home: &Path,
    rail: &str,
    id: &str,
) -> Result<Option<(String, Vec<u8>)>, StateRepoError> {
    let relative = entity_relative_path(rail, id)?;
    let path = state_root(home).join(&relative);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {
            if metadata.len() > MAX_ENTITY_BYTES {
                return Err(StateRepoError::corrupt(format!(
                    "{relative} exceeds {MAX_ENTITY_BYTES} bytes"
                )));
            }
            let bytes =
                fs::read(&path).map_err(|error| StateRepoError::storage(error.to_string()))?;
            Ok(Some((hex::encode(Sha256::digest(&bytes)), bytes)))
        }
        Ok(_) => Err(StateRepoError::corrupt(format!(
            "{relative} is not a regular file"
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StateRepoError::storage(error.to_string())),
    }
}

/// Ensure the entity file exists so a `replace_utf8` mission can target it.
/// A brand-new file is created empty and committed as plumbing: an empty
/// entity carries no semantics — readers treat it as absent — so no authority
/// decision happens here. Returns the relative path and the current SHA-256.
pub fn ensure_entity_file(
    home: &Path,
    rail: &str,
    id: &str,
) -> Result<(String, String), StateRepoError> {
    let relative = entity_relative_path(rail, id)?;
    let root = state_root(home);
    if let Some((sha, _)) = entity_current(home, rail, id)? {
        return Ok((relative, sha));
    }
    let path = root.join(&relative);
    atomic_write(&path, b"").map_err(|error| StateRepoError::storage(error.to_string()))?;
    run_git(&root, &["add", "--", &relative])?;
    run_git(
        &root,
        &[
            "commit",
            "--quiet",
            "--no-verify",
            "-m",
            &format!("plumbing: placeholder {relative}"),
        ],
    )?;
    let sha = hex::encode(Sha256::digest(b""));
    Ok((relative, sha))
}

/// The state repository must be clean and identified before drafting.
pub fn require_clean(home: &Path) -> Result<StateRepoStatus, StateRepoError> {
    let status = initialize_state_repo(home)?;
    if !status.clean {
        return Err(StateRepoError::refused(
            "state repository is dirty; refuse to draft over unadopted changes",
        ));
    }
    Ok(status)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AdoptionRecord {
    pub schema: String,
    pub sequence: u64,
    pub mission_id: String,
    pub rail: String,
    pub relative_path: String,
    pub content_digest: String,
    pub contract_digest: String,
    pub action_digest: String,
    pub state_commit: String,
    pub previous: String,
    pub entry_hash: String,
}

fn record_hash(record: &AdoptionRecord) -> Result<String, StateRepoError> {
    let mut unsealed = record.clone();
    unsealed.entry_hash = String::new();
    let bytes = serde_json::to_vec(&unsealed)
        .map_err(|error| StateRepoError::storage(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn chain_path(home: &Path) -> PathBuf {
    home.join(ADOPTION_CHAIN)
}

/// Load and fully verify the adoption chain. Any framing, schema, sequence,
/// linkage, or hash defect refuses the whole chain.
pub fn load_adoption_chain(home: &Path) -> Result<Vec<AdoptionRecord>, StateRepoError> {
    let path = chain_path(home);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(StateRepoError::storage(error.to_string())),
    };
    if !metadata.is_file() {
        return Err(StateRepoError::corrupt("adoption chain is not a file"));
    }
    if metadata.len() > MAX_CHAIN_BYTES {
        return Err(StateRepoError::corrupt("adoption chain exceeds 16 MiB"));
    }
    let text =
        fs::read_to_string(&path).map_err(|error| StateRepoError::storage(error.to_string()))?;
    let mut records = Vec::new();
    let mut previous = GENESIS_HASH.to_owned();
    for (index, line) in text.lines().enumerate() {
        let record: AdoptionRecord = serde_json::from_str(line).map_err(|error| {
            StateRepoError::corrupt(format!("adoption record {index} malformed: {error}"))
        })?;
        if record.schema != ADOPTION_SCHEMA {
            return Err(StateRepoError::corrupt(format!(
                "adoption record {index} has wrong schema"
            )));
        }
        if record.sequence != (index as u64) + 1 {
            return Err(StateRepoError::corrupt(format!(
                "adoption record {index} sequence mismatch"
            )));
        }
        if record.previous != previous {
            return Err(StateRepoError::corrupt(format!(
                "adoption record {index} breaks the hash chain"
            )));
        }
        if record_hash(&record)? != record.entry_hash {
            return Err(StateRepoError::corrupt(format!(
                "adoption record {index} hash mismatch"
            )));
        }
        previous = record.entry_hash.clone();
        records.push(record);
    }
    Ok(records)
}

/// Adopt one COMPLETE, receipt-verified mission into canonical rail state.
///
/// Verifies, in order:
/// 1. persisted mission result exists, `status=VERIFIED`, `tamperVerdict=REJECTED`;
/// 2. persisted contract recompiles cleanly and targets the state repository;
/// 3. the lane holds exactly the kernel-authorized bytes (`contentDigest`);
/// 4. the canonical file still holds the reviewed base bytes (`expectedSha256`);
/// then atomically writes the authorized bytes, commits them, and appends a
/// hash-chained adoption record. Idempotent per mission.
pub fn adopt_mission(home: &Path, mission_id: &str) -> Result<AdoptionRecord, StateRepoError> {
    if mission_id.len() != 26
        || !mission_id.starts_with("mis_")
        || !mission_id[4..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(StateRepoError::refused("mission id shape refused"));
    }
    // Serialize the whole load -> verify -> commit -> append critical section:
    // held until this function returns, so no concurrent adoption (in this
    // process or another app instance sharing the home) can produce a git
    // commit whose adoption record a racing writer then clobbers.
    let _lock = GovernedWriteLock::acquire(home)
        .map_err(|error| StateRepoError::storage(error.to_string()))?;

    let chain = load_adoption_chain(home)?;
    if let Some(existing) = chain.iter().find(|record| record.mission_id == mission_id) {
        return Ok(existing.clone());
    }

    let mission_dir = home.join("missions").join(mission_id);
    let result_bytes = fs::read(mission_dir.join("result.json"))
        .map_err(|error| StateRepoError::refused(format!("mission result unavailable: {error}")))?;
    let result: Value = serde_json::from_slice(&result_bytes)
        .map_err(|error| StateRepoError::corrupt(format!("mission result malformed: {error}")))?;
    let text = |key: &str| -> Result<String, StateRepoError> {
        result
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| StateRepoError::corrupt(format!("mission result missing {key}")))
    };
    if text("status")? != "VERIFIED" || text("tamperVerdict")? != "REJECTED" {
        return Err(StateRepoError::refused(
            "mission result is not a verified terminal state",
        ));
    }
    if text("missionId")? != mission_id {
        return Err(StateRepoError::corrupt("mission result identity mismatch"));
    }
    let lane_path = PathBuf::from(text("lanePath")?);

    let contract_bytes = fs::read(mission_dir.join("contract.json")).map_err(|error| {
        StateRepoError::refused(format!("mission contract unavailable: {error}"))
    })?;
    let compiled = compile_local_contract(&contract_bytes)
        .map_err(|error| StateRepoError::corrupt(format!("persisted contract refused: {error}")))?;

    let root = state_root(home);
    let canonical_root = root
        .canonicalize()
        .map_err(|error| StateRepoError::storage(error.to_string()))?;
    let workspace = PathBuf::from(&compiled.workspace)
        .canonicalize()
        .map_err(|error| StateRepoError::storage(error.to_string()))?;
    if workspace != canonical_root {
        return Err(StateRepoError::refused(
            "mission workspace is not the governed state repository",
        ));
    }
    let rail = compiled
        .relative_path
        .split('/')
        .next()
        .unwrap_or_default()
        .to_owned();
    if !rail_is_governed(&rail) {
        return Err(StateRepoError::refused(
            "mission target is outside every governed rail",
        ));
    }

    let lane_file = lane_path.join(&compiled.relative_path);
    let lane_bytes = fs::read(&lane_file)
        .map_err(|error| StateRepoError::refused(format!("lane artifact unavailable: {error}")))?;
    let lane_digest = hex::encode(Sha256::digest(&lane_bytes));
    if lane_digest != compiled.content_digest {
        return Err(StateRepoError::refused(
            "lane bytes do not match the kernel-authorized content digest",
        ));
    }

    let canonical_file = root.join(&compiled.relative_path);
    let current = fs::read(&canonical_file)
        .map_err(|error| StateRepoError::storage(format!("canonical read: {error}")))?;
    let current_digest = hex::encode(Sha256::digest(&current));
    if current_digest != compiled.expected_sha256 {
        return Err(StateRepoError::refused(
            "canonical state advanced since review; adoption is stale",
        ));
    }

    atomic_write(&canonical_file, &lane_bytes)
        .map_err(|error| StateRepoError::storage(error.to_string()))?;
    run_git(&root, &["add", "--", &compiled.relative_path])?;
    run_git(
        &root,
        &[
            "commit",
            "--quiet",
            "--no-verify",
            "-m",
            &format!(
                "adopt {}: mission {} content {}",
                compiled.relative_path, mission_id, compiled.content_digest
            ),
        ],
    )?;
    let state_commit = run_git(&root, &["rev-parse", "HEAD"])?;

    let previous = chain
        .last()
        .map(|record| record.entry_hash.clone())
        .unwrap_or_else(|| GENESIS_HASH.to_owned());
    let mut record = AdoptionRecord {
        schema: ADOPTION_SCHEMA.to_owned(),
        sequence: chain.len() as u64 + 1,
        mission_id: mission_id.to_owned(),
        rail,
        relative_path: compiled.relative_path.clone(),
        content_digest: compiled.content_digest.clone(),
        contract_digest: compiled.contract_digest.clone(),
        action_digest: compiled.action_digest.clone(),
        state_commit,
        previous,
        entry_hash: String::new(),
    };
    record.entry_hash = record_hash(&record)?;
    let mut line =
        serde_json::to_vec(&record).map_err(|error| StateRepoError::storage(error.to_string()))?;
    line.push(b'\n');
    let path = chain_path(home);
    let mut existing = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(StateRepoError::storage(error.to_string())),
    };
    existing.extend_from_slice(&line);
    atomic_write(&path, &existing).map_err(|error| StateRepoError::storage(error.to_string()))?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    static NONCE: AtomicU64 = AtomicU64::new(0);

    fn scratch_home(name: &str) -> PathBuf {
        let nonce = NONCE.fetch_add(1, Ordering::SeqCst);
        let home = std::env::temp_dir().join(format!(
            "nemesis-state-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(home.join("receipts")).unwrap();
        home
    }

    #[test]
    fn initialize_is_idempotent_and_clean() {
        let home = scratch_home("init");
        let first = initialize_state_repo(&home).unwrap();
        assert!(first.clean);
        let second = initialize_state_repo(&home).unwrap();
        assert_eq!(first.head, second.head);
        assert!(second.clean);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn entity_ids_are_bounded_and_shape_checked() {
        assert!(validate_entity_id("valid-id-9").is_ok());
        for refused in [
            "",
            "UPPER",
            "has_underscore",
            "-leading",
            "a b",
            &"x".repeat(49),
        ] {
            assert!(validate_entity_id(refused).is_err(), "accepted {refused:?}");
        }
        assert!(entity_relative_path("changes", "x").is_err());
        assert!(entity_relative_path("skills", "ok-id").is_ok());
    }

    #[test]
    fn placeholders_read_as_absent_and_corrupt_entities_refuse() {
        let home = scratch_home("read");
        initialize_state_repo(&home).unwrap();
        ensure_entity_file(&home, "skills", "pending").unwrap();
        assert!(read_rail_entities(&home, "skills").unwrap().is_empty());
        fs::write(state_root(&home).join("skills/broken.json"), b"{not json").unwrap();
        let error = read_rail_entities(&home, "skills").unwrap_err();
        assert!(matches!(error, StateRepoError::Corrupt(_)));
        fs::remove_dir_all(&home).unwrap();
    }

    fn seed_completed_mission(
        home: &std::path::Path,
        mission_id: &str,
        replacement: &str,
    ) -> (String, String) {
        seed_completed_mission_entity(home, mission_id, "subject", replacement)
    }

    fn seed_completed_mission_entity(
        home: &std::path::Path,
        mission_id: &str,
        entity: &str,
        replacement: &str,
    ) -> (String, String) {
        initialize_state_repo(home).unwrap();
        let (relative, expected_sha) = ensure_entity_file(home, "skills", entity).unwrap();
        let root = state_root(home);
        let base_revision = run_git(&root, &["rev-parse", "HEAD"]).unwrap();
        let contract = serde_json::json!({
            "schema": "nemesis.desktop-mission/v1",
            "missionId": mission_id,
            "goal": format!("RAIL SKILLS PROPOSE {entity} — test"),
            "workspace": root.display().to_string(),
            "baseRevision": base_revision,
            "action": {
                "kind": "replace_utf8",
                "relativePath": relative,
                "expectedSha256": expected_sha,
                "replacement": replacement,
            },
            "authority": {"network": false, "push": false, "publish": false, "secrets": false},
            "budgets": {"maxWriteBytes": 4096, "maxRuntimeSeconds": 900, "maxOutputBytes": 1048576},
            "completion": ["git_diff_check", "content_match"],
        });
        let mission_dir = home.join("missions").join(mission_id);
        fs::create_dir_all(&mission_dir).unwrap();
        fs::write(
            mission_dir.join("contract.json"),
            serde_json::to_vec(&contract).unwrap(),
        )
        .unwrap();
        let lane = home.join("lanes").join(mission_id);
        fs::create_dir_all(lane.join("skills")).unwrap();
        fs::write(lane.join(&relative), replacement).unwrap();
        let result = serde_json::json!({
            "status": "VERIFIED",
            "missionId": mission_id,
            "sequence": 9,
            "sourceDigest": hex::encode(Sha256::digest(replacement.as_bytes())),
            "ledgerHead": "ab".repeat(32),
            "artifactDirectory": mission_dir.join("evidence").display().to_string(),
            "lanePath": lane.display().to_string(),
            "claims": [],
            "tamperVerdict": "REJECTED",
            "replayPath": mission_dir.join("evidence/replay.json").display().to_string(),
        });
        fs::write(
            mission_dir.join("result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        (relative, expected_sha)
    }

    #[test]
    fn adoption_applies_exactly_authorized_bytes_and_chains() {
        let home = scratch_home("adopt");
        let mission_id = "mis_adoptadoptadoptadopt00";
        let replacement = "{\n  \"schema\": \"nemesis.rail-skill/v1\",\n  \"id\": \"subject\"\n}\n";
        let (relative, _) = seed_completed_mission(&home, mission_id, replacement);
        let record = adopt_mission(&home, mission_id).unwrap();
        assert_eq!(record.sequence, 1);
        assert_eq!(record.rail, "skills");
        let canonical = fs::read_to_string(state_root(&home).join(&relative)).unwrap();
        assert_eq!(canonical, replacement);
        // Idempotent: a second adoption returns the same chained record.
        let again = adopt_mission(&home, mission_id).unwrap();
        assert_eq!(again.entry_hash, record.entry_hash);
        assert_eq!(load_adoption_chain(&home).unwrap().len(), 1);
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn adoption_refuses_tampered_lane_bytes() {
        let home = scratch_home("tamper");
        let mission_id = "mis_tampertampertamper0000";
        let (relative, _) = seed_completed_mission(&home, mission_id, "{\"a\":1}\n");
        fs::write(
            home.join("lanes").join(mission_id).join(&relative),
            "{\"a\":2}\n",
        )
        .unwrap();
        let error = adopt_mission(&home, mission_id).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("kernel-authorized content digest")
        );
        assert!(load_adoption_chain(&home).unwrap().is_empty());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn adoption_refuses_when_canonical_state_advanced() {
        let home = scratch_home("stale");
        let mission_id = "mis_stalestale0stale000000";
        let (relative, _) = seed_completed_mission(&home, mission_id, "{\"a\":1}\n");
        fs::write(state_root(&home).join(&relative), "interleaved").unwrap();
        let error = adopt_mission(&home, mission_id).unwrap_err();
        assert!(error.to_string().contains("adoption is stale"));
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn adoption_chain_rejects_single_byte_tamper() {
        let home = scratch_home("chain");
        let mission_id = "mis_chainchain0chain000000";
        let replacement = "{\"a\":1}\n";
        seed_completed_mission(&home, mission_id, replacement);
        adopt_mission(&home, mission_id).unwrap();
        let path = home.join(ADOPTION_CHAIN);
        let mut bytes = fs::read(&path).unwrap();
        let target = bytes.len() / 2;
        bytes[target] ^= 0x01;
        fs::write(&path, &bytes).unwrap();
        assert!(load_adoption_chain(&home).is_err());
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn concurrent_adoptions_serialize_and_lose_no_chain_record() {
        // Regression for the boss code-review finding: without a lock spanning
        // load -> git commit -> append, N concurrent adoptions produce N git
        // commits but fewer chain records (a racing atomic_write clobbers the
        // loser), or fail outright on git index.lock contention. The
        // governed-write lock must turn N adoptions into exactly N linked
        // records and N adoption commits — never a commit without a receipt.
        const N: usize = 8;
        let home = scratch_home("concurrent-adopt");
        initialize_state_repo(&home).unwrap();
        let mut mission_ids = Vec::with_capacity(N);
        for index in 0..N {
            let mission_id = format!("mis_concurrent{index:012}");
            seed_completed_mission_entity(
                &home,
                &mission_id,
                &format!("subject-{index}"),
                "{\"a\":1}\n",
            );
            mission_ids.push(mission_id);
        }

        let home = Arc::new(home);
        let barrier = Arc::new(Barrier::new(N));
        let handles: Vec<_> = mission_ids
            .into_iter()
            .map(|mission_id| {
                let home = Arc::clone(&home);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    adopt_mission(&home, &mission_id)
                })
            })
            .collect();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        for result in &results {
            result
                .as_ref()
                .expect("every concurrent adoption must succeed under the lock");
        }

        // Exactly N records; load_adoption_chain verifies sequence and hash
        // linkage, so a survivor with a stale `previous` would also be caught.
        let chain = load_adoption_chain(&home).unwrap();
        assert_eq!(chain.len(), N, "a racing adoption lost its chain record");
        let mut sequences: Vec<u64> = chain.iter().map(|record| record.sequence).collect();
        sequences.sort_unstable();
        assert_eq!(sequences, (1..=N as u64).collect::<Vec<_>>());

        // One adoption commit per record: no governed commit lacks a receipt.
        let log = run_git(&state_root(&home), &["log", "--pretty=%s"]).unwrap();
        let adopt_commits = log
            .lines()
            .filter(|line| line.starts_with("adopt skills/"))
            .count();
        assert_eq!(adopt_commits, N, "git commits and chain records disagree");

        fs::remove_dir_all(&*home).unwrap();
    }
}
