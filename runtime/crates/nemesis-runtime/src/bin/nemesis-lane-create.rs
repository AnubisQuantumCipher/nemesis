use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use nemesis_runtime::GitWorktreeManager;

fn run() -> Result<serde_json::Value, String> {
    let mut arguments = env::args_os().skip(1);
    let repo = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing canonical repository".to_owned())?;
    let lane = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing lane path".to_owned())?;
    let branch = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| "missing UTF-8 lane branch".to_owned())?;
    if arguments.next().is_some() {
        return Err("unexpected lane creator arguments".to_owned());
    }
    let info =
        GitWorktreeManager::create(&repo, &lane, &branch).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "status": "CREATED",
        "root": info.root,
        "branch": info.branch,
        "base_revision": info.base_revision
    }))
}

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string(&value).expect("JSON value serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("NEMESIS refused lane creation: {error}");
            ExitCode::from(1)
        }
    }
}
