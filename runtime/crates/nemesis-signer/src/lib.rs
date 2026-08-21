use std::io::Write;
use std::process::{Command, Stdio};

use ed25519_dalek::SigningKey;
use nemesis_receipt::{ClaimReceipt, EpistemicStatus, ReceiptPayload};
use serde::Deserialize;
use thiserror::Error;
use zeroize::Zeroize;

const KEYCHAIN_SERVICE: &str = "dev.nemesis.receipt.seed.v1";
const KEYCHAIN_ACCOUNT: &str = "local-default";
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum SignerError {
    #[error("sign request is empty or oversized")]
    RequestSize,
    #[error("invalid sign request: {0}")]
    InvalidRequest(String),
    #[error("Keychain operation failed: {0}")]
    Keychain(String),
    #[error("random key generation failed: {0}")]
    Random(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum RequestStatus {
    Verified,
    Believed,
    Unknown,
}

impl From<RequestStatus> for EpistemicStatus {
    fn from(value: RequestStatus) -> Self {
        match value {
            RequestStatus::Verified => Self::Verified,
            RequestStatus::Believed => Self::Believed,
            RequestStatus::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestClaim {
    id: String,
    mandatory: bool,
    status: RequestStatus,
    evidence_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestPayload {
    mission_id: String,
    event_sequence: u64,
    terminal_state: String,
    source_digest: String,
    ledger_head: String,
    claims: Vec<RequestClaim>,
    residuals: Vec<String>,
}

fn digest(value: &str, name: &str) -> Result<[u8; 32], SignerError> {
    let bytes = hex::decode(value)
        .map_err(|error| SignerError::InvalidRequest(format!("{name}: {error}")))?;
    bytes
        .try_into()
        .map_err(|_| SignerError::InvalidRequest(format!("{name} must contain exactly 32 bytes")))
}

pub fn parse_request(bytes: &[u8]) -> Result<ReceiptPayload, SignerError> {
    if bytes.is_empty() || bytes.len() > MAX_REQUEST_BYTES {
        return Err(SignerError::RequestSize);
    }
    let request: RequestPayload = serde_json::from_slice(bytes)
        .map_err(|error| SignerError::InvalidRequest(error.to_string()))?;
    let claims = request
        .claims
        .into_iter()
        .map(|claim| {
            Ok(ClaimReceipt {
                id: claim.id,
                mandatory: claim.mandatory,
                status: claim.status.into(),
                evidence_digest: digest(&claim.evidence_digest, "evidence_digest")?,
            })
        })
        .collect::<Result<Vec<_>, SignerError>>()?;
    Ok(ReceiptPayload {
        mission_id: request.mission_id,
        event_sequence: request.event_sequence,
        terminal_state: request.terminal_state,
        source_digest: digest(&request.source_digest, "source_digest")?,
        ledger_head: digest(&request.ledger_head, "ledger_head")?,
        claims,
        residuals: request.residuals,
    })
}

fn signing_key_from_password(mut password: Vec<u8>) -> Result<SigningKey, SignerError> {
    if password.len() != 32 {
        password.fill(0);
        return Err(SignerError::Keychain(
            "stored seed has an invalid length".to_owned(),
        ));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&password);
    password.fill(0);
    let key = SigningKey::from_bytes(&seed);
    seed.fill(0);
    Ok(key)
}

fn add_arguments() -> Vec<&'static str> {
    vec![
        "add-generic-password",
        "-a",
        KEYCHAIN_ACCOUNT,
        "-s",
        KEYCHAIN_SERVICE,
        "-T",
        "/usr/bin/security",
        "-w",
    ]
}

fn prompt_input(secret: &str) -> String {
    format!("{secret}\n{secret}\n")
}

fn find_keychain_password() -> Result<Option<Vec<u8>>, SignerError> {
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            KEYCHAIN_SERVICE,
            "-w",
        ])
        .output()
        .map_err(|error| SignerError::Keychain(error.to_string()))?;
    if output.status.success() {
        let mut encoded = output.stdout;
        let password = std::str::from_utf8(&encoded)
            .map_err(|error| SignerError::Keychain(error.to_string()))
            .and_then(|value| {
                hex::decode(value.trim()).map_err(|error| SignerError::Keychain(error.to_string()))
            });
        encoded.zeroize();
        return password.map(Some);
    }
    if output.status.code() == Some(44) {
        return Ok(None);
    }
    Err(SignerError::Keychain(
        String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    ))
}

fn create_keychain_password() -> Result<(), SignerError> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|error| SignerError::Random(error.to_string()))?;
    let mut encoded = hex::encode(seed);
    seed.fill(0);
    let mut input = prompt_input(&encoded).into_bytes();
    let mut child = Command::new("/usr/bin/security")
        .args(add_arguments())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| SignerError::Keychain(error.to_string()))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| SignerError::Keychain("security stdin unavailable".to_owned()))?;
        stdin
            .write_all(&input)
            .and_then(|_| stdin.flush())
            .map_err(|error| SignerError::Keychain(error.to_string()))?;
    }
    input.fill(0);
    encoded.zeroize();
    let output = child
        .wait_with_output()
        .map_err(|error| SignerError::Keychain(error.to_string()))?;
    if output.status.success() || output.status.code() == Some(45) {
        Ok(())
    } else {
        Err(SignerError::Keychain(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

pub fn load_or_create_keychain_signing_key() -> Result<SigningKey, SignerError> {
    if let Some(password) = find_keychain_password()? {
        return signing_key_from_password(password);
    }
    create_keychain_password()?;
    find_keychain_password()?
        .ok_or_else(|| SignerError::Keychain("created seed could not be retrieved".to_owned()))
        .and_then(signing_key_from_password)
}

#[cfg(test)]
mod tests {
    use super::{add_arguments, prompt_input};

    #[test]
    fn keychain_creation_trusts_only_the_stable_security_binary() {
        let arguments = add_arguments();
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["-T", "/usr/bin/security"])
        );
        assert_eq!(arguments.last(), Some(&"-w"));
        assert!(!arguments.iter().any(|argument| argument.contains("secret")));
    }

    #[test]
    fn prompted_secret_is_supplied_twice_over_stdin() {
        assert_eq!(prompt_input("secret"), "secret\nsecret\n");
    }
}
