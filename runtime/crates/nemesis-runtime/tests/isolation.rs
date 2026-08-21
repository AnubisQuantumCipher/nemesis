use std::ffi::OsStr;
use std::fs;
use std::net::TcpListener;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Command;

use nemesis_runtime::{GitWorktreeManager, SandboxProfile, SandboxedCommand};
use tempfile::TempDir;

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git executes");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim()
        .to_owned()
}

fn fixture_repo() -> TempDir {
    let temp = tempfile::tempdir().expect("temporary repository");
    git(temp.path(), &["init", "-b", "main"]);
    fs::write(temp.path().join("value.txt"), "before\n").unwrap();
    git(temp.path(), &["add", "value.txt"]);
    git(
        temp.path(),
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
    temp
}

#[test]
fn creates_an_isolated_git_worktree_without_moving_main() {
    let repo = fixture_repo();
    let head_before = git(repo.path(), &["rev-parse", "main"]);
    let lane = repo.path().join("lanes/worker-1");

    let info = GitWorktreeManager::create(repo.path(), &lane, "nemesis/lane-worker-1")
        .expect("isolated lane");

    assert_eq!(info.root, lane.canonicalize().unwrap());
    assert_eq!(info.base_revision, head_before);
    assert_eq!(git(repo.path(), &["rev-parse", "main"]), head_before);
    assert!(lane.join(".git").is_file());
    fs::write(lane.join("value.txt"), "lane-only\n").unwrap();
    assert_eq!(
        fs::read_to_string(repo.path().join("value.txt")).unwrap(),
        "before\n"
    );
}

#[test]
fn generated_profile_is_default_deny_and_lane_scoped() {
    let temp = tempfile::tempdir().unwrap();
    let lane = temp.path().join("lane");
    fs::create_dir(&lane).unwrap();
    let profile =
        SandboxProfile::workspace_safe(&lane, Path::new("/usr/bin/true")).expect("profile");
    let source = profile.source();

    assert!(source.contains("(deny default)"));
    assert!(source.contains("(deny network*)"));
    let lane_rule = format!("(subpath \"{}\")", lane.canonicalize().unwrap().display());
    let parent_rule = format!("(subpath \"{}\")", temp.path().display());
    assert!(source.contains(&lane_rule));
    assert!(!source.contains(&parent_rule));
}

#[test]
fn sandbox_runs_inside_lane_with_a_clean_environment() {
    let temp = tempfile::tempdir().unwrap();
    let lane = temp.path().join("lane");
    fs::create_dir(&lane).unwrap();

    let output = SandboxedCommand::new(Path::new("/usr/bin/env"), &lane)
        .run_capture(&[])
        .expect("sandbox command starts");
    assert!(
        output.status.success(),
        "status={:?} signal={:?} stdout={} stderr={}",
        output.status.code(),
        output.status.signal(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let environment = String::from_utf8(output.stdout).unwrap();
    assert!(environment.contains("PATH=/usr/bin:/bin"));
    for forbidden in ["HOME=", "SSH_AUTH_SOCK=", "GITHUB_TOKEN=", "AWS_"] {
        assert!(
            !environment.contains(forbidden),
            "leaked {forbidden}: {environment}"
        );
    }
}

#[test]
fn sandbox_refuses_read_and_write_outside_lane() {
    let temp = tempfile::tempdir().unwrap();
    let lane = temp.path().join("lane");
    fs::create_dir(&lane).unwrap();
    let secret = temp.path().join("secret.txt");
    let sibling = temp.path().join("sibling.txt");
    fs::write(&secret, "not granted\n").unwrap();

    let read = SandboxedCommand::new(Path::new("/bin/cat"), &lane)
        .run_capture(&[secret.as_os_str()])
        .expect("sandbox command starts");
    assert!(!read.status.success());
    assert!(!String::from_utf8_lossy(&read.stdout).contains("not granted"));

    let write = SandboxedCommand::new(Path::new("/usr/bin/touch"), &lane)
        .run_capture(&[sibling.as_os_str()])
        .expect("sandbox command starts");
    assert!(!write.status.success());
    assert!(!sibling.exists());
}

#[test]
fn sandbox_refuses_a_reachable_loopback_connection() {
    let baseline = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    baseline.set_nonblocking(true).unwrap();
    let baseline_port = baseline.local_addr().unwrap().port().to_string();
    let baseline_output = Command::new("/usr/bin/nc")
        .args(["-z", "127.0.0.1", &baseline_port])
        .output()
        .unwrap();
    assert!(baseline_output.status.success());
    assert!(
        baseline.accept().is_ok(),
        "baseline listener was not reached"
    );

    let denied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    denied.set_nonblocking(true).unwrap();
    let denied_port = denied.local_addr().unwrap().port().to_string();
    let temp = tempfile::tempdir().unwrap();
    let lane = temp.path().join("lane");
    fs::create_dir(&lane).unwrap();
    let output = SandboxedCommand::new(Path::new("/usr/bin/nc"), &lane)
        .run_capture(&[
            OsStr::new("-z"),
            OsStr::new("127.0.0.1"),
            OsStr::new(&denied_port),
        ])
        .unwrap();
    assert!(!output.status.success());
    assert!(
        denied.accept().is_err(),
        "sandboxed worker reached a denied loopback listener"
    );
}
