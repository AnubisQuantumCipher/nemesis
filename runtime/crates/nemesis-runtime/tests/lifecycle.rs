use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use nemesis_runtime::{SandboxedCommand, WorkerOutputAccumulator};

#[test]
fn worker_process_can_be_cancelled_without_state_authority() {
    let temp = tempfile::tempdir().unwrap();
    let lane = temp.path().join("lane");
    fs::create_dir(&lane).unwrap();
    let mut child = SandboxedCommand::new(Path::new("/bin/sleep"), &lane)
        .spawn(&[OsStr::new("30")])
        .expect("sandboxed worker starts");
    child.cancel().expect("worker cancellation");
    let output = child.wait_with_output().expect("worker exit");
    assert!(!output.status.success());
}

#[test]
fn output_accumulator_fails_closed_at_its_exact_limit() {
    let mut output = WorkerOutputAccumulator::new(8);
    output.push(b"1234").unwrap();
    output.push(b"5678").unwrap();
    assert_eq!(output.as_bytes(), b"12345678");
    assert!(output.push(b"9").is_err());
    assert_eq!(output.as_bytes(), b"12345678");
}
