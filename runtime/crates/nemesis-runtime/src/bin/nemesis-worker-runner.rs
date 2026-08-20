use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use nemesis_runtime::SandboxedCommand;

fn run() -> Result<ExitCode, String> {
    let mut arguments = env::args_os().skip(1);
    let executable = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing exact worker executable".to_owned())?;
    let lane = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing isolated lane".to_owned())?;
    let worker_arguments: Vec<_> = arguments.collect();
    let worker_argument_refs: Vec<&OsStr> = worker_arguments
        .iter()
        .map(|value| value.as_os_str())
        .collect();

    let output = SandboxedCommand::new(&executable, &lane)
        .run_capture(&worker_argument_refs)
        .map_err(|error| error.to_string())?;
    io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    match output.status.code() {
        Some(code) if (0..=255).contains(&code) => Ok(ExitCode::from(code as u8)),
        Some(_) | None => Ok(ExitCode::from(125)),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("NEMESIS refused worker launch: {error}");
            ExitCode::from(126)
        }
    }
}
