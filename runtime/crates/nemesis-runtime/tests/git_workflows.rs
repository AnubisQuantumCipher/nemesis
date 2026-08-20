use std::fs;
use std::path::Path;
use std::process::Command;

use nemesis_runtime::{
    CiCheckReceipt, GitAuthority, GitDecision, GitOperation, GitPolicy, GitWorkflow,
};

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .current_dir(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn fixture() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "-b", "main"]);
    fs::write(repo.path().join("value.txt"), "before\n").unwrap();
    git(repo.path(), &["add", "value.txt"]);
    git(
        repo.path(),
        &[
            "-c",
            "user.name=NEMESIS Test",
            "-c",
            "user.email=nemesis-test@invalid",
            "commit",
            "-m",
            "fixture",
        ],
    );
    repo
}

#[test]
fn push_merge_and_unrelated_deletion_refuse_without_exact_authority() {
    let policy = GitPolicy::new("main", GitAuthority::local_commit_only());
    assert_eq!(policy.decide(&GitOperation::Push), GitDecision::Refused);
    assert_eq!(policy.decide(&GitOperation::Merge), GitDecision::Refused);
    assert_eq!(
        policy.decide(&GitOperation::DeleteBranch("user/work".to_owned())),
        GitDecision::Refused
    );
    assert_eq!(
        policy.decide(&GitOperation::DeleteWorktree(
            "/tmp/user-worktree".to_owned()
        )),
        GitDecision::Refused
    );
}

#[test]
fn exact_lane_commit_leaves_default_branch_unchanged() {
    let repo = fixture();
    let main_before = git(repo.path(), &["rev-parse", "main"]);
    let lane = repo.path().join("lane");
    git(
        repo.path(),
        &[
            "worktree",
            "add",
            "-b",
            "nemesis/lane-1",
            lane.to_str().unwrap(),
            "main",
        ],
    );
    fs::write(lane.join("value.txt"), "after\n").unwrap();
    fs::write(lane.join("unrelated.txt"), "do not stage\n").unwrap();
    let workflow = GitWorkflow::new(repo.path(), "main").unwrap();
    let receipt = workflow
        .commit_lane(
            &lane,
            "nemesis/lane-1",
            &["value.txt".to_owned()],
            "fixture change",
            &GitAuthority::local_commit_only(),
        )
        .unwrap();
    assert_eq!(git(repo.path(), &["rev-parse", "main"]), main_before);
    assert_eq!(receipt.default_head_before, receipt.default_head_after);
    assert_ne!(receipt.commit, main_before);
    assert_eq!(receipt.paths, vec!["value.txt"]);
    assert!(lane.join("unrelated.txt").is_file());
    assert!(git(&lane, &["status", "--short"]).contains("?? unrelated.txt"));
}

#[test]
fn path_traversal_and_git_metadata_staging_refuse() {
    let repo = fixture();
    let workflow = GitWorkflow::new(repo.path(), "main").unwrap();
    for path in ["../escape", ".git/config", "/tmp/absolute"] {
        assert!(workflow.validate_commit_paths(&[path.to_owned()]).is_err());
    }
}

#[test]
fn ci_observation_is_accepted_only_for_current_source() {
    let current = [0x42; 32];
    let passing = CiCheckReceipt {
        check_name: "release-tests".to_owned(),
        source_digest: current,
        conclusion: "success".to_owned(),
        evidence_digest: [0x55; 32],
    };
    assert!(GitWorkflow::accept_ci(&passing, current));
    assert!(!GitWorkflow::accept_ci(&passing, [0x43; 32]));
    let failed = CiCheckReceipt {
        conclusion: "failure".to_owned(),
        ..passing
    };
    assert!(!GitWorkflow::accept_ci(&failed, current));
}
