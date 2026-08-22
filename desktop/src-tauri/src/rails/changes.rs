//! Changes rail — lane diffs against canonical source. Derived, read-only.
//!
//! Every mission lane under `<home>/lanes/<mission_id>` is a Git worktree.
//! This view reports, per lane: branch, base revision, dirty files, and a
//! diff stat — computed live with `/usr/bin/git`, never cached. A lane that
//! cannot be interrogated is surfaced as CORRUPT with its error; it is never
//! silently dropped.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use super::RailError;

fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("/usr/bin/git")
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("git spawn failed: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn lane_view(home: &Path, mission_id: &str, lane: &Path) -> Value {
    let mut view = json!({
        "missionId": mission_id,
        "lanePath": lane.display().to_string(),
        "status": "OK",
    });
    match run_git(lane, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(branch) => view["branch"] = Value::String(branch),
        Err(error) => {
            view["status"] = Value::String("CORRUPT".to_owned());
            view["error"] = Value::String(error);
            return view;
        }
    }
    if let Ok(head) = run_git(lane, &["rev-parse", "HEAD"]) {
        view["head"] = Value::String(head);
    }
    match run_git(lane, &["status", "--porcelain=v1", "--untracked-files=all"]) {
        Ok(porcelain) => {
            let files: Vec<Value> = porcelain
                .lines()
                .take(64)
                .map(|line| {
                    let (code, path) = line.split_at(line.len().min(3));
                    json!({"code": code.trim(), "path": path.trim()})
                })
                .collect();
            view["dirtyCount"] = Value::from(porcelain.lines().count());
            view["dirtyFiles"] = Value::Array(files);
        }
        Err(error) => {
            view["status"] = Value::String("CORRUPT".to_owned());
            view["error"] = Value::String(error);
            return view;
        }
    }
    if let Ok(stat) = run_git(lane, &["diff", "--shortstat"]) {
        view["diffShortstat"] = Value::String(stat);
    }
    let contract = home.join("missions").join(mission_id).join("contract.json");
    if let Ok(bytes) = fs::read(&contract) {
        if let Ok(parsed) = serde_json::from_slice::<Value>(&bytes) {
            view["workspace"] = parsed.get("workspace").cloned().unwrap_or(Value::Null);
            view["baseRevision"] = parsed.get("baseRevision").cloned().unwrap_or(Value::Null);
            view["relativePath"] = parsed
                .get("action")
                .and_then(|action| action.get("relativePath"))
                .cloned()
                .unwrap_or(Value::Null);
        }
    }
    view
}

/// All lanes, newest mission id last. Fail-closed: an unreadable lanes
/// directory refuses; a broken individual lane is surfaced as CORRUPT.
pub fn snapshot(home: &Path) -> Result<Value, RailError> {
    let lanes_dir = home.join("lanes");
    let mut lanes = Vec::new();
    let entries = match fs::read_dir(&lanes_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"lanes": [], "laneCount": 0}));
        }
        Err(error) => return Err(RailError::storage(error.to_string())),
    };
    for entry in entries {
        let entry = entry.map_err(|error| RailError::storage(error.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        lanes.push(lane_view(home, &name, &path));
    }
    lanes.sort_by(|left, right| {
        left.get("missionId")
            .and_then(Value::as_str)
            .cmp(&right.get("missionId").and_then(Value::as_str))
    });
    Ok(json!({"laneCount": lanes.len(), "lanes": lanes}))
}
