use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use nemesis_desktop::{
    MissionCancellation, MissionDraftRequest, MissionExecutables, compile_local_contract,
    draft_local_mission, initialize_local_home, load_last_mission, preflight_local_mission,
    run_local_mission,
};
use sha2::{Digest, Sha256};

fn test_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from("/tmp").join(format!("nemesis-{name}-{}-{nonce}", std::process::id()))
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .current_dir(repo)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn create_repository(root: &Path) -> (PathBuf, String, String) {
    let repo = root.join("repository");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    fs::write(repo.join("value.txt"), "before\n").unwrap();
    git(&repo, &["add", "value.txt"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=NEMESIS Production Test",
            "-c",
            "user.email=nemesis-production@invalid",
            "commit",
            "-m",
            "fixture",
        ],
    );
    let revision = git(&repo, &["rev-parse", "HEAD"]);
    let expected = hex::encode(Sha256::digest(b"before\n"));
    (repo, revision, expected)
}

fn contract(repo: &Path, revision: &str, expected: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema": "nemesis.desktop-mission/v1",
        "missionId": "mis_0123456789abcdefghij12",
        "goal": "Replace one bounded UTF-8 file in an isolated lane.",
        "workspace": repo,
        "baseRevision": revision,
        "action": {
            "kind": "replace_utf8",
            "relativePath": "value.txt",
            "expectedSha256": expected,
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
    }))
    .unwrap()
}

#[test]
fn drafts_an_exact_source_bound_contract_inside_the_private_local_home() {
    let root = test_directory("draft");
    let (repo, revision, expected) = create_repository(&root);
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    let cancellation = MissionCancellation::default();

    let drafted = draft_local_mission(
        &home,
        &MissionDraftRequest {
            goal: "Replace one bounded UTF-8 file in an isolated lane.".to_owned(),
            workspace: repo.display().to_string(),
            relative_path: "value.txt".to_owned(),
            replacement: "after\n".to_owned(),
        },
        &cancellation,
    )
    .unwrap();

    assert!(Path::new(&drafted.path).starts_with(home.join("drafts")));
    assert_eq!(drafted.compiled.base_revision, revision);
    assert_eq!(drafted.compiled.expected_sha256, expected);
    assert_eq!(
        compile_local_contract(&fs::read(&drafted.path).unwrap()).unwrap(),
        drafted.compiled
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preflight_binds_clean_workspace_head_and_expected_target_bytes() {
    let root = test_directory("preflight");
    let (repo, revision, expected) = create_repository(&root);
    let compiled = compile_local_contract(&contract(&repo, &revision, &expected)).unwrap();
    let cancellation = MissionCancellation::default();

    preflight_local_mission(&compiled, &cancellation).unwrap();
    fs::write(repo.join("untracked.txt"), "unreviewed\n").unwrap();
    let error = preflight_local_mission(&compiled, &cancellation).unwrap_err();

    assert!(error.to_string().contains("workspace must be clean"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn executable_symlink_substitution_is_refused_before_lane_creation() {
    let root = test_directory("executable-symlink");
    let (repo, revision, expected) = create_repository(&root);
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    let compiled = compile_local_contract(&contract(&repo, &revision, &expected)).unwrap();
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let names = [
        "daemon",
        "lane-create",
        "worker-runner",
        "worker",
        "signer",
        "verifier-target",
        "replay",
    ];
    for name in names {
        let path = bin.join(name);
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let verifier = bin.join("verifier");
    std::os::unix::fs::symlink(bin.join("verifier-target"), &verifier).unwrap();
    let executables = MissionExecutables {
        daemon: bin.join("daemon"),
        lane_create: bin.join("lane-create"),
        worker_runner: bin.join("worker-runner"),
        worker: bin.join("worker"),
        signer: bin.join("signer"),
        verifier,
        replay: bin.join("replay"),
    };
    assert!(!executables.is_ready());
    let cancellation = MissionCancellation::default();

    let error = run_local_mission(
        &compiled,
        &compiled.contract_digest,
        &compiled.action_digest,
        &home,
        &executables,
        &cancellation,
        &mut |_| {},
    )
    .unwrap_err();

    assert!(error.to_string().contains("must not be a symlink"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn child_process_environment_is_sanitized_before_lane_creation() {
    let root = test_directory("clean-environment");
    let (repo, revision, expected) = create_repository(&root);
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    let compiled = compile_local_contract(&contract(&repo, &revision, &expected)).unwrap();
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let capture = root.join("captured-env.txt");
    let lane_create = bin.join("lane-create");
    fs::write(
        &lane_create,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"${{NEMESIS_SECRET_SHOULD_NOT_LEAK-unset}}\" > '{}'\nmkdir -p \"$2\"\nprintf '%s\\n' '{{\"status\":\"CREATED\",\"base_revision\":\"{}\"}}'\n",
            capture.display(),
            revision,
        ),
    )
    .unwrap();
    fs::set_permissions(&lane_create, fs::Permissions::from_mode(0o755)).unwrap();
    for name in [
        "daemon",
        "worker-runner",
        "worker",
        "signer",
        "verifier",
        "replay",
    ] {
        let path = bin.join(name);
        fs::write(&path, "#!/bin/sh\nexit 1\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let executables = MissionExecutables {
        daemon: bin.join("daemon"),
        lane_create,
        worker_runner: bin.join("worker-runner"),
        worker: bin.join("worker"),
        signer: bin.join("signer"),
        verifier: bin.join("verifier"),
        replay: bin.join("replay"),
    };
    let cancellation = MissionCancellation::default();
    unsafe { std::env::set_var("NEMESIS_SECRET_SHOULD_NOT_LEAK", "secret") };

    let _ = run_local_mission(
        &compiled,
        &compiled.contract_digest,
        &compiled.action_digest,
        &home,
        &executables,
        &cancellation,
        &mut |_| {},
    );
    unsafe { std::env::remove_var("NEMESIS_SECRET_SHOULD_NOT_LEAK") };

    assert_eq!(fs::read_to_string(capture).unwrap(), "unset\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reviewed_digest_mismatch_refuses_before_creating_a_lane() {
    let root = test_directory("review-mismatch");
    let (repo, revision, expected) = create_repository(&root);
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    let compiled = compile_local_contract(&contract(&repo, &revision, &expected)).unwrap();
    let executables = MissionExecutables::development(Path::new("/unreachable"));
    let cancellation = MissionCancellation::default();

    let error = run_local_mission(
        &compiled,
        &"0".repeat(64),
        &compiled.action_digest,
        &home,
        &executables,
        &cancellation,
        &mut |_| {},
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("reviewed contract digest mismatch")
    );
    assert!(!home.join("lanes").join(&compiled.mission_id).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pre_cancelled_mission_refuses_before_creating_a_lane() {
    let root = test_directory("cancelled");
    let (repo, revision, expected) = create_repository(&root);
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    let compiled = compile_local_contract(&contract(&repo, &revision, &expected)).unwrap();
    let executables = MissionExecutables::development(Path::new("/unreachable"));
    let cancellation = MissionCancellation::default();
    cancellation.cancel();

    let error = run_local_mission(
        &compiled,
        &compiled.contract_digest,
        &compiled.action_digest,
        &home,
        &executables,
        &cancellation,
        &mut |_| {},
    )
    .unwrap_err();

    assert!(error.to_string().contains("mission cancelled"));
    assert!(!home.join("lanes").join(&compiled.mission_id).exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires built Ada and Rust runtime executables plus macOS Keychain"]
fn executes_reviewed_contract_and_persists_current_receipt_and_replay() {
    let root = test_directory("mission-run");
    let (repo, revision, expected) = create_repository(&root);
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    let compiled = compile_local_contract(&contract(&repo, &revision, &expected)).unwrap();
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let executables = MissionExecutables::development(&repository_root);
    let mut phases = Vec::new();
    let cancellation = MissionCancellation::default();

    let result = run_local_mission(
        &compiled,
        &compiled.contract_digest,
        &compiled.action_digest,
        &home,
        &executables,
        &cancellation,
        &mut |progress| phases.push(progress.phase.clone()),
    )
    .unwrap();

    assert_eq!(result.status, "VERIFIED");
    assert_eq!(result.tamper_verdict, "REJECTED");
    assert_eq!(result.claims.len(), 2);
    assert_eq!(
        fs::read_to_string(repo.join("value.txt")).unwrap(),
        "before\n"
    );
    assert_eq!(
        fs::read_to_string(
            home.join("lanes")
                .join(&compiled.mission_id)
                .join("value.txt")
        )
        .unwrap(),
        "after\n"
    );
    assert!(phases.contains(&"REPLAY_VERIFIED".to_owned()));
    assert_eq!(load_last_mission(&home).unwrap().unwrap(), result);
    fs::remove_dir_all(root).unwrap();
}
