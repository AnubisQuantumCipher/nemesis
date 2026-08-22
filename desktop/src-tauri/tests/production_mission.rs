use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nemesis_desktop::{
    TextScale, compile_local_contract, initialize_local_home, load_settings, save_settings,
};

fn valid_contract() -> String {
    serde_json::json!({
        "schema": "nemesis.desktop-mission/v1",
        "missionId": "mis_0123456789abcdefghij12",
        "goal": "Replace one bounded UTF-8 file in an isolated lane.",
        "workspace": "/tmp/nemesis-workspace",
        "baseRevision": "a".repeat(40),
        "action": {
            "kind": "replace_utf8",
            "relativePath": "value.txt",
            "expectedSha256": "b".repeat(64),
            "replacement": "after\n"
        },
        "authority": {
            "network": false,
            "push": false,
            "publish": false,
            "secrets": false
        },
        "budgets": {
            "maxWriteBytes": 4096,
            "maxRuntimeSeconds": 300,
            "maxOutputBytes": 1048576
        },
        "completion": ["git_diff_check", "content_match"]
    })
    .to_string()
}

fn test_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("nemesis-{name}-{}-{nonce}", std::process::id()))
}

#[test]
fn compiles_equivalent_json_to_one_normalized_digest_and_exact_action() {
    let compact = valid_contract();
    let pretty: serde_json::Value = serde_json::from_str(&compact).unwrap();
    let pretty = serde_json::to_string_pretty(&pretty).unwrap();

    let first = compile_local_contract(compact.as_bytes()).unwrap();
    let second = compile_local_contract(pretty.as_bytes()).unwrap();

    assert_eq!(first.contract_digest, second.contract_digest);
    assert_eq!(first.action_digest, second.action_digest);
    assert_eq!(first.replacement_bytes, 6);
    assert_eq!(first.relative_path, "value.txt");
    assert_eq!(first.capabilities, vec!["filesystem.modify:exact"]);
    assert!(!first.normalized_contract.contains('\n'));
}

#[test]
fn rejects_duplicate_unknown_unsafe_or_authority_widening_contract_fields() {
    let duplicate = valid_contract().replacen("\"goal\":", "\"goal\":\"duplicate\",\"goal\":", 1);
    assert!(
        compile_local_contract(duplicate.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("duplicate field")
    );

    let mut unknown: serde_json::Value = serde_json::from_str(&valid_contract()).unwrap();
    unknown["ambientShell"] = serde_json::Value::Bool(true);
    assert!(
        compile_local_contract(unknown.to_string().as_bytes())
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );

    let mut traversal: serde_json::Value = serde_json::from_str(&valid_contract()).unwrap();
    traversal["action"]["relativePath"] = serde_json::json!("../escape");
    assert!(
        compile_local_contract(traversal.to_string().as_bytes())
            .unwrap_err()
            .to_string()
            .contains("relative path")
    );

    let mut network: serde_json::Value = serde_json::from_str(&valid_contract()).unwrap();
    network["authority"]["network"] = serde_json::Value::Bool(true);
    assert!(
        compile_local_contract(network.to_string().as_bytes())
            .unwrap_err()
            .to_string()
            .contains("authority must remain denied")
    );
}

#[test]
fn initializes_private_versioned_local_home_idempotently() {
    let root = test_directory("home");
    let _ = fs::remove_dir_all(&root);

    let first = initialize_local_home(&root).unwrap();
    let second = initialize_local_home(&root).unwrap();

    assert!(first.first_launch);
    assert!(!second.first_launch);
    assert_eq!(first.schema_version, 1);
    assert!(root.join("missions").is_dir());
    assert!(root.join("lanes").is_dir());
    assert!(root.join("receipts").is_dir());
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn settings_are_safe_persisted_and_reversible() {
    let root = test_directory("settings");
    let _ = fs::remove_dir_all(&root);
    initialize_local_home(&root).unwrap();

    let defaults = load_settings(&root).unwrap();
    assert_eq!(defaults.text_scale, TextScale::Standard);
    assert!(defaults.reduce_motion);

    let changed = nemesis_desktop::DesktopSettings {
        text_scale: TextScale::Large,
        reduce_motion: false,
    };
    save_settings(&root, &changed).unwrap();
    assert_eq!(load_settings(&root).unwrap(), changed);
    save_settings(&root, &defaults).unwrap();
    assert_eq!(load_settings(&root).unwrap(), defaults);
    fs::remove_dir_all(root).unwrap();
}
