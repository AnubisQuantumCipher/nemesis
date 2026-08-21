use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nemesis_receipt::sign1;
use nemesis_signer::{load_or_create_keychain_signing_key, parse_request};
use serde_json::json;

fn paths() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let mut arguments = env::args_os().skip(1);
    let mut request = None;
    let mut receipt = None;
    let mut public_key = None;
    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {}", flag.to_string_lossy()))?;
        match flag.to_str() {
            Some("--request") if request.is_none() => request = Some(PathBuf::from(value)),
            Some("--receipt") if receipt.is_none() => receipt = Some(PathBuf::from(value)),
            Some("--public-key") if public_key.is_none() => public_key = Some(PathBuf::from(value)),
            _ => {
                return Err(format!(
                    "unknown or duplicate option: {}",
                    flag.to_string_lossy()
                ));
            }
        }
    }
    Ok((
        request.ok_or_else(|| "missing --request".to_owned())?,
        receipt.ok_or_else(|| "missing --receipt".to_owned())?,
        public_key.ok_or_else(|| "missing --public-key".to_owned())?,
    ))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("output path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("output path has no filename: {}", path.display()))?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|error| error.to_string())?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| error.to_string())?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn sign() -> Result<serde_json::Value, String> {
    let (request_path, receipt_path, public_key_path) = paths()?;
    let request = fs::read(&request_path).map_err(|error| error.to_string())?;
    let payload = parse_request(&request).map_err(|error| error.to_string())?;
    let signing_key = load_or_create_keychain_signing_key().map_err(|error| error.to_string())?;
    let receipt = sign1(&payload, &signing_key).map_err(|error| error.to_string())?;
    let public_key = hex::encode(signing_key.verifying_key().as_bytes());
    atomic_write(&receipt_path, &receipt)?;
    atomic_write(&public_key_path, public_key.as_bytes())?;
    Ok(json!({
        "status": "SIGNED",
        "receipt": receipt_path,
        "public_key": public_key,
        "private_key_exported": false
    }))
}

fn main() -> ExitCode {
    match sign() {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string(&value).expect("JSON value serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("NEMESIS signer refused: {error}");
            ExitCode::from(1)
        }
    }
}
