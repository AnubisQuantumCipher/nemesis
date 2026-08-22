//! Security rail — capabilities, policies, approvals, secrets, audit.
//!
//! Read-only aggregation of the authority truth the daemon persisted:
//! fixed-width parent grants and one-shot approvals from every mission core
//! home, the adoption receipt chain, and the product's standing postures.
//! Records that fail their fixed-width shape are surfaced as CORRUPT rows —
//! a broken authority artifact must be visible.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::RailError;
use crate::state_repo;

const GRANT_TAG: &str = "NEMESIS_GRANT_V1";
const APPROVAL_TAG: &str = "NEMESIS_APPROVAL_V1";

fn parse_grant(line: &str) -> Result<Value, String> {
    let fields: Vec<&str> = line.trim_end_matches('\n').split('|').collect();
    if fields.len() != 10 || fields[0] != GRANT_TAG {
        return Err(format!("grant record has {} fields", fields.len()));
    }
    let status = match fields[9] {
        "A" => "ACTIVE",
        "R" => "REVOKED",
        other => return Err(format!("unknown grant status {other}")),
    };
    Ok(json!({
        "grantId": fields[1].trim(),
        "missionId": fields[2].trim(),
        "subject": fields[3].trim(),
        "resource": fields[4].trim(),
        "operations": fields[5].trim(),
        "scopeDigest": fields[6].trim(),
        "expiresSequence": fields[7].trim().parse::<u64>().unwrap_or(0),
        "maximumBytes": fields[8].trim().parse::<u64>().unwrap_or(0),
        "status": status,
    }))
}

fn parse_approval(line: &str) -> Result<Value, String> {
    let fields: Vec<&str> = line.trim_end_matches('\n').split('|').collect();
    if fields.len() != 6 || fields[0] != APPROVAL_TAG {
        return Err(format!("approval record has {} fields", fields.len()));
    }
    let status = match fields[5] {
        "A" => "ARMED",
        "C" => "CONSUMED",
        "R" => "REVOKED",
        other => return Err(format!("unknown approval status {other}")),
    };
    Ok(json!({
        "approvalId": fields[1].trim(),
        "missionId": fields[2].trim(),
        "actionDigest": fields[3].trim(),
        "expiresSequence": fields[4].trim().parse::<u64>().unwrap_or(0),
        "status": status,
    }))
}

fn collect_authority(home: &Path) -> Result<(Vec<Value>, Vec<Value>), RailError> {
    let mut grants = Vec::new();
    let mut approvals = Vec::new();
    let core_root = home.join(".core");
    let core_entries = match fs::read_dir(&core_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((grants, approvals));
        }
        Err(error) => return Err(RailError::storage(error.to_string())),
    };
    for core_entry in core_entries.flatten() {
        let missions_dir = core_entry.path().join("missions");
        let Ok(mission_entries) = fs::read_dir(&missions_dir) else {
            continue;
        };
        for mission_entry in mission_entries.flatten() {
            let mission_dir = mission_entry.path();
            if !mission_dir.is_dir() {
                continue;
            }
            let grant_path = mission_dir.join("parent.grant");
            if grant_path.is_file() {
                match fs::read_to_string(&grant_path) {
                    Ok(text) => match parse_grant(&text) {
                        Ok(record) => grants.push(record),
                        Err(error) => grants.push(json!({
                            "status": "CORRUPT",
                            "path": grant_path.display().to_string(),
                            "error": error,
                        })),
                    },
                    Err(error) => grants.push(json!({
                        "status": "CORRUPT",
                        "path": grant_path.display().to_string(),
                        "error": error.to_string(),
                    })),
                }
            }
            let Ok(files) = fs::read_dir(&mission_dir) else {
                continue;
            };
            for file in files.flatten() {
                let name = file.file_name().to_string_lossy().into_owned();
                if !name.starts_with("approval-") || !name.ends_with(".apr") {
                    continue;
                }
                match fs::read_to_string(file.path()) {
                    Ok(text) => match parse_approval(&text) {
                        Ok(record) => approvals.push(record),
                        Err(error) => approvals.push(json!({
                            "status": "CORRUPT",
                            "path": file.path().display().to_string(),
                            "error": error,
                        })),
                    },
                    Err(error) => approvals.push(json!({
                        "status": "CORRUPT",
                        "path": file.path().display().to_string(),
                        "error": error.to_string(),
                    })),
                }
            }
        }
    }
    Ok((grants, approvals))
}

/// Full security snapshot. The adoption chain is re-verified on every call;
/// a broken chain fails the whole snapshot — that is the point.
pub fn snapshot(home: &Path) -> Result<Value, RailError> {
    let adoption_chain = state_repo::load_adoption_chain(home)?;
    let (grants, approvals) = collect_authority(home)?;
    let armed = approvals
        .iter()
        .filter(|record| record.get("status").and_then(Value::as_str) == Some("ARMED"))
        .count();
    let consumed = approvals
        .iter()
        .filter(|record| record.get("status").and_then(Value::as_str) == Some("CONSUMED"))
        .count();
    let corrupt = grants
        .iter()
        .chain(approvals.iter())
        .filter(|record| record.get("status").and_then(Value::as_str) == Some("CORRUPT"))
        .count();
    Ok(json!({
        "policy": {
            "network": "DENIED_BY_CONTRACT",
            "push": "DENIED_BY_CONTRACT",
            "publish": "DENIED_BY_CONTRACT",
            "secrets": "DENIED_BY_CONTRACT",
            "updates": "DISABLED_NO_AUTHENTICATED_UPDATER",
            "approvals": "ONE_SHOT_EXACT_ACTION_DIGEST",
            "capabilities": "PARENT_GRANT_ATTENUATED_CHILD",
        },
        "secretsPosture": {
            "stored": "NONE",
            "keychainReceiptSeed": "NEVER_TOUCHED_BY_DESKTOP",
        },
        "grants": grants,
        "approvals": approvals,
        "approvalTally": {"armed": armed, "consumed": consumed, "corrupt": corrupt},
        "adoptionChain": {
            "length": adoption_chain.len(),
            "head": adoption_chain
                .last()
                .map(|record| record.entry_hash.clone())
                .unwrap_or_else(|| state_repo::GENESIS_HASH.to_owned()),
            "verified": true,
            "records": adoption_chain,
        },
    }))
}
