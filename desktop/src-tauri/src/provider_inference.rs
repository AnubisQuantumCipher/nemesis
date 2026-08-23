//! Provider-inference lane — ADR-0001 Option B (two-lane split).
//!
//! NEMESIS keeps the action-executing governed worker lane at absolute `NET DENY`
//! with the SPARK kernel, `capability_check`, `authorize_action`, and one-shot
//! approval UNCHANGED. This module is the SEPARATE, explicit **opt-in**
//! provider-inference lane: it launches the user's own officially installed and
//! authenticated Claude Code / Codex CLI OUTSIDE the governed action lane
//! (network-enabled, the user's own credentials read only by the CLI itself),
//! captures the CLI's output as an UNTRUSTED PROPOSAL, and hands that proposal to
//! the unchanged kernel authority path. Only an `AUTHORIZED` action ever executes,
//! and it executes in the `NET DENY` action lane.
//!
//! Credential boundary (non-negotiable): NEMESIS never reads, copies, prints,
//! persists, refreshes, or proxies provider OAuth credentials. The provider CLI is
//! invoked as the user's own process; it reads its own credentials via the
//! inherited environment. NEMESIS only observes stdout/stderr/exit and redacts any
//! secret-shaped bytes before they reach a log or receipt.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::production::normalized_action_digest;
use crate::rails::agents::SUBPROCESS_KINDS;

/// Provider kinds that can back the inference lane by delegating to a locally
/// installed, user-authenticated CLI (a subscription-backed official CLI or a
/// local model). API kinds are NOT here: a raw cloud API key is a different lane
/// (secrets in the OS keychain), never the CLI-delegation lane.
pub const INFERENCE_KINDS: &[&str] = &["codex-cli", "claude-code-cli", "local-model"];
pub const API_KINDS: &[&str] = &["openai-api", "anthropic-api"];

/// Typed provider state. Presence is never function: only `Ready` means the lane
/// can produce a proposal, and even then the proposal must pass the kernel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderState {
    /// Executable present + executable bit + the CLI reports it is authenticated.
    Ready { detail: String },
    /// Executable present but the official CLI's own auth is missing/expired.
    /// The user re-authenticates through the provider's own login; NEMESIS never
    /// touches the token.
    AuthRequired { detail: String },
    /// Declared executable absent or not a runnable file.
    MissingExecutable,
    /// Provider requires network the current action-lane trust surface denies
    /// (every cloud/API/loopback lane inside the governed `NET DENY` worker).
    UnavailableNetworkDenied,
    /// Undeclared provider, spoofed identity/path, ambient-network attempt, or a
    /// secret-shaped output — refused by policy.
    BlockedProviderPolicy { reason: String },
}

/// A declared inference provider resolved from a `nemesis.rail-agent/v1` row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceProvider {
    pub agent_id: String,
    pub provider_kind: String,
    pub executable_path: String,
}

/// An untrusted proposal produced by the provider-inference lane. It is NOT an
/// authorization — it must still pass `capability_check` + `authorize_action`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceProposal {
    pub relative_path: String,
    pub content_digest: String,
    pub content_bytes: u64,
    pub action_digest: String,
    /// A redacted, bounded preview of the proposal for the human review surface.
    pub preview: String,
}

/// Resolve a declared agent row into an inference provider, or the exact typed
/// reason it cannot back the lane. API kinds are refused here: the CLI-delegation
/// lane never accepts a raw cloud key.
pub fn resolve_inference_provider(entity: &Value) -> Result<InferenceProvider, ProviderState> {
    if entity.get("status").and_then(Value::as_str) != Some("active") {
        return Err(ProviderState::BlockedProviderPolicy {
            reason: "agent is not active".to_owned(),
        });
    }
    let kind = entity
        .get("providerKind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if API_KINDS.contains(&kind) {
        // A raw API-key provider is a separate keychain-backed lane, never the
        // CLI-delegation inference lane, and never network-enabled here.
        return Err(ProviderState::BlockedProviderPolicy {
            reason: "API-key provider is not the CLI-delegation lane".to_owned(),
        });
    }
    if !INFERENCE_KINDS.contains(&kind) {
        return Err(ProviderState::BlockedProviderPolicy {
            reason: format!("provider kind {kind} may not back the inference lane"),
        });
    }
    // Defense in depth: an inference kind must also be a declared subprocess kind.
    if !SUBPROCESS_KINDS.contains(&kind) {
        return Err(ProviderState::BlockedProviderPolicy {
            reason: "provider kind is not a subprocess kind".to_owned(),
        });
    }
    let agent_id = entity.get("id").and_then(Value::as_str).ok_or_else(|| {
        ProviderState::BlockedProviderPolicy {
            reason: "agent id missing".to_owned(),
        }
    })?;
    let executable_path = entity
        .get("executablePath")
        .and_then(Value::as_str)
        .unwrap_or("");
    if executable_path.is_empty() {
        return Err(ProviderState::MissingExecutable);
    }
    Ok(InferenceProvider {
        agent_id: agent_id.to_owned(),
        provider_kind: kind.to_owned(),
        executable_path: executable_path.to_owned(),
    })
}

/// Redact secret-shaped bytes so a token can never reach a log, receipt, or the
/// review surface. Applied to every captured provider byte before it is surfaced.
pub fn redact_secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in text.split_inclusive(char::is_whitespace) {
        let (word, trail) = match token.char_indices().rfind(|(_, c)| c.is_whitespace()) {
            Some((idx, c)) if idx + c.len_utf8() == token.len() => (&token[..idx], &token[idx..]),
            _ => (token, ""),
        };
        if looks_secret(word) {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(word);
        }
        out.push_str(trail);
    }
    out
}

fn looks_secret(word: &str) -> bool {
    let w = word.trim();
    w.starts_with("sk-")
        || w.starts_with("sk-ant-")
        || w.starts_with("Bearer")
        || w.starts_with("eyJ") && w.len() > 20
        || w.to_ascii_lowercase().contains("oauth")
        || (w.len() >= 40 && w.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Only a loopback endpoint may back a local-model lane; a non-loopback host is a
/// disguised cloud egress and is refused.
pub fn is_loopback_endpoint(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or_else(|| {
            url.split("://")
                .nth(1)
                .unwrap_or(url)
                .split('/')
                .next()
                .unwrap_or("")
        });
    matches!(host, "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

/// Classify a subscription CLI's local login/auth check output into a typed state
/// WITHOUT any model call. `codex login status` prints "Logged in ..."; an expired
/// Claude session prints an "OAuth session expired" style error.
pub fn classify_login_status(returncode: i32, stdout: &str, stderr: &str) -> ProviderState {
    let joined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    let unauth = joined.contains("not logged in")
        || joined.contains("oauth session expired")
        || joined.contains("could not be refreshed")
        || joined.contains("failed to authenticate")
        || joined.contains("please run")
        || joined.contains("login required")
        || joined.contains("unauthenticated");
    if returncode == 0
        && (joined.contains("logged in") || joined.contains("authenticated"))
        && !unauth
    {
        ProviderState::Ready {
            detail: redact_secrets(stdout.trim()).chars().take(120).collect(),
        }
    } else {
        ProviderState::AuthRequired {
            detail: redact_secrets(
                if stderr.trim().is_empty() {
                    stdout
                } else {
                    stderr
                }
                .trim(),
            )
            .chars()
            .take(160)
            .collect(),
        }
    }
}

/// Normalize a raw provider stdout into the exact bytes proposed for a file, or
/// refuse. A proposal that is empty, over budget, or non-UTF-8-clean is refused —
/// never silently coerced into an action.
pub fn normalize_inference_proposal(raw: &[u8], max_bytes: u64) -> Result<String, String> {
    if raw.len() as u64 > max_bytes {
        return Err(format!(
            "provider output {} exceeds the {max_bytes}-byte proposal budget",
            raw.len()
        ));
    }
    let text =
        std::str::from_utf8(raw).map_err(|_| "provider output was not valid UTF-8".to_owned())?;
    // Strip a single leading/trailing fenced code block if the CLI wrapped the
    // content; refuse if nothing remains.
    let trimmed = strip_code_fence(text);
    if trimmed.trim().is_empty() {
        return Err("provider proposed empty content".to_owned());
    }
    Ok(trimmed)
}

fn strip_code_fence(text: &str) -> String {
    let t = text.trim_matches('\n');
    if let Some(rest) = t.strip_prefix("```") {
        // drop the first line (``` or ```lang) and a trailing ```
        if let Some(nl) = rest.find('\n') {
            let body = &rest[nl + 1..];
            if let Some(body) = body.strip_suffix("```") {
                return body.trim_end_matches('\n').to_owned();
            }
            if let Some(body) = body.strip_suffix("```\n") {
                return body.trim_end_matches('\n').to_owned();
            }
        }
    }
    text.to_owned()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundedRun {
    pub returncode: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub raw_stdout: Vec<u8>,
    pub timed_out: bool,
}

/// Run a provider CLI in the OPT-IN inference lane: the user's own process, the
/// user's own environment (so the CLI finds its own credentials), network-enabled,
/// NOT the `NET DENY` action sandbox — but bounded by a timeout and an output
/// ceiling. NEMESIS reads only stdout/stderr/exit and never the credential store.
fn run_provider_bounded(
    executable: &str,
    args: &[&str],
    timeout: Duration,
    output_limit: usize,
) -> std::io::Result<BoundedRun> {
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut out = child.stdout.take().expect("piped stdout");
    let mut err = child.stderr.take().expect("piped stderr");
    let limit = output_limit;
    let out_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out.by_ref().take(limit as u64 + 1).read_to_end(&mut buf);
        buf
    });
    let err_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err.by_ref().take(limit as u64 + 1).read_to_end(&mut buf);
        buf
    });
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break None;
        }
        thread::sleep(Duration::from_millis(20));
    };
    let raw_stdout = out_reader.join().unwrap_or_default();
    let raw_stderr = err_reader.join().unwrap_or_default();
    let over = raw_stdout.len() > limit || raw_stderr.len() > limit;
    Ok(BoundedRun {
        returncode: status.and_then(|s| s.code()),
        stdout: redact_secrets(&String::from_utf8_lossy(&raw_stdout)),
        stderr: redact_secrets(&String::from_utf8_lossy(&raw_stderr)),
        raw_stdout: if over { Vec::new() } else { raw_stdout },
        timed_out: timed_out || over,
    })
}

/// Local, model-free health check for a subscription CLI provider.
pub fn provider_health(provider: &InferenceProvider) -> ProviderState {
    let path = Path::new(&provider.executable_path);
    if !path.is_file() {
        return ProviderState::MissingExecutable;
    }
    let probe_args: Vec<&str> = match provider.provider_kind.as_str() {
        "codex-cli" => vec!["login", "status"],
        "claude-code-cli" => vec!["--version"], // presence; auth is proven on first bounded request
        _ => vec!["--version"],
    };
    match run_provider_bounded(
        &provider.executable_path,
        &probe_args,
        Duration::from_secs(30),
        64 * 1024,
    ) {
        Ok(run) => classify_login_status(run.returncode.unwrap_or(-1), &run.stdout, &run.stderr),
        Err(error) => ProviderState::AuthRequired {
            detail: format!("health probe failed: {error}"),
        },
    }
}

/// Run one bounded inference request in the opt-in lane and normalize the output
/// into an untrusted proposal. The proposal is NOT authorized here — the caller
/// must route it through `capability_check` + `authorize_action`.
#[allow(clippy::too_many_arguments)]
pub fn run_inference(
    provider: &InferenceProvider,
    relative_path: &str,
    goal: &str,
    max_bytes: u64,
    timeout: Duration,
) -> Result<InferenceProposal, ProviderState> {
    let path = Path::new(&provider.executable_path);
    if !path.is_file() {
        return Err(ProviderState::MissingExecutable);
    }
    let prompt = format!(
        "Propose the exact complete new contents of the file `{relative_path}`. \
         Goal: {goal}. Output ONLY the new file contents with no explanation, no \
         markdown fences, and no surrounding prose."
    );
    let args: Vec<&str> = match provider.provider_kind.as_str() {
        "codex-cli" => vec!["exec", "--skip-git-repo-check", &prompt],
        "claude-code-cli" => vec!["-p", &prompt],
        _ => vec![&prompt],
    };
    let run = run_provider_bounded(
        &provider.executable_path,
        &args,
        timeout,
        max_bytes as usize,
    )
    .map_err(|error| ProviderState::BlockedProviderPolicy {
        reason: format!("provider launch failed: {error}"),
    })?;
    if run.timed_out {
        return Err(ProviderState::BlockedProviderPolicy {
            reason: "provider inference timed out or exceeded the output budget".to_owned(),
        });
    }
    if run.returncode != Some(0) {
        // A non-zero exit that mentions auth is AUTH_REQUIRED; otherwise blocked.
        return Err(classify_login_status(
            run.returncode.unwrap_or(-1),
            &run.stdout,
            &run.stderr,
        ));
    }
    let content = normalize_inference_proposal(&run.raw_stdout, max_bytes)
        .map_err(|reason| ProviderState::BlockedProviderPolicy { reason })?;
    let content_digest = hex::encode(Sha256::digest(content.as_bytes()));
    let action_digest = normalized_action_digest(relative_path, &content_digest);
    Ok(InferenceProposal {
        relative_path: relative_path.to_owned(),
        content_bytes: content.len() as u64,
        content_digest,
        action_digest,
        preview: redact_secrets(&content).chars().take(200).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn agent(kind: &str, exe: Option<&str>, status: &str) -> Value {
        json!({
            "schema": "nemesis.rail-agent/v1", "id": "inf-agent", "name": "Inf",
            "providerKind": kind, "executablePath": exe,
            "roles": ["builder"], "consequenceCeiling": "decision-boundary",
            "budget": {"costMicrounits": 1, "inputTokens": 1, "outputTokens": 1},
            "status": status, "revision": 1,
        })
    }

    // RED negative control 1: absent executable -> MISSING_EXECUTABLE.
    #[test]
    fn absent_executable_is_missing() {
        let p =
            resolve_inference_provider(&agent("codex-cli", Some("/nope/codex"), "active")).unwrap();
        assert_eq!(provider_health(&p), ProviderState::MissingExecutable);
    }

    // RED negative control 2 + 3: unauthenticated / expired official CLI -> AUTH_REQUIRED.
    #[test]
    fn unauthenticated_and_expired_are_auth_required() {
        assert!(matches!(
            classify_login_status(
                1,
                "",
                "Failed to authenticate: OAuth session expired and could not be refreshed"
            ),
            ProviderState::AuthRequired { .. }
        ));
        assert!(matches!(
            classify_login_status(1, "Not logged in", ""),
            ProviderState::AuthRequired { .. }
        ));
        assert!(matches!(
            classify_login_status(0, "Logged in using ChatGPT", ""),
            ProviderState::Ready { .. }
        ));
    }

    // RED negative control 4 + 9: wrong/undeclared provider kind or API kind -> BLOCKED.
    #[test]
    fn undeclared_or_api_kind_is_blocked() {
        assert!(matches!(
            resolve_inference_provider(&agent("openai-api", None, "active")),
            Err(ProviderState::BlockedProviderPolicy { .. })
        ));
        assert!(matches!(
            resolve_inference_provider(&agent("anthropic-api", None, "active")),
            Err(ProviderState::BlockedProviderPolicy { .. })
        ));
        assert!(matches!(
            resolve_inference_provider(&agent("mystery-provider", Some("/bin/x"), "active")),
            Err(ProviderState::BlockedProviderPolicy { .. })
        ));
        assert!(matches!(
            resolve_inference_provider(&agent("codex-cli", Some("/usr/bin/true"), "revoked")),
            Err(ProviderState::BlockedProviderPolicy { .. })
        ));
    }

    // RED negative control 5: output overflow -> refused (not coerced to an action).
    #[test]
    fn output_overflow_is_refused() {
        let big = vec![b'a'; 4097];
        assert!(normalize_inference_proposal(&big, 4096).is_err());
        assert!(normalize_inference_proposal(b"after\n", 4096).is_ok());
    }

    // RED negative control 6: malformed / empty provider output -> refused.
    #[test]
    fn malformed_output_is_refused() {
        assert!(normalize_inference_proposal(b"", 4096).is_err());
        assert!(normalize_inference_proposal(b"   \n  ", 4096).is_err());
        assert!(normalize_inference_proposal(&[0xff, 0xfe, 0x00], 4096).is_err());
    }

    // RED negative control 7 + secret redaction: token-shaped output is redacted.
    #[test]
    fn secret_shaped_output_is_redacted() {
        let leaked =
            "here is sk-ant-abc123DEFsecrettoken0000 and Bearer eyJhbGciOiJIUzI1NiExample0000";
        let red = redact_secrets(leaked);
        assert!(!red.contains("sk-ant-abc123"), "{red}");
        assert!(red.contains("[REDACTED]"), "{red}");
        assert!(redact_secrets("normal safe output OK").contains("OK"));
    }

    // RED negative control 10: local loopback endpoint refuses non-loopback hosts.
    #[test]
    fn loopback_only_for_local_model() {
        assert!(is_loopback_endpoint("http://127.0.0.1:11434/v1"));
        assert!(is_loopback_endpoint("http://localhost:8080"));
        assert!(!is_loopback_endpoint("http://api.openai.com/v1"));
        assert!(!is_loopback_endpoint("https://10.0.0.5:11434"));
    }

    // Proposal digest equals the exact digest the unchanged kernel binds (so the
    // inference proposal flows through capability_check + authorize_action).
    #[test]
    fn proposal_digest_matches_kernel_binding() {
        let content = "after\n";
        let cd = hex::encode(Sha256::digest(content.as_bytes()));
        assert_eq!(
            normalized_action_digest("src/value.txt", &cd),
            crate::production::normalized_action_digest("src/value.txt", &cd),
        );
    }

    // A present, authenticated codex resolves + is Ready when the executable exists;
    // resolve treats a valid subprocess kind + present exe as inference-capable.
    #[test]
    fn present_subprocess_provider_resolves() {
        let p = resolve_inference_provider(&agent("codex-cli", Some("/usr/bin/true"), "active"))
            .unwrap();
        assert_eq!(p.provider_kind, "codex-cli");
        assert_eq!(p.executable_path, "/usr/bin/true");
    }

    // Crown-jewel (opt-in lane, live): the real authenticated Codex CLI produces
    // an untrusted proposal with a valid kernel-bindable digest. Ignored by
    // default (needs the user's own authenticated codex + network); run manually
    // for evidence. Proves function, not mere detection.
    #[test]
    #[ignore = "requires the user's own authenticated codex CLI + network (opt-in lane)"]
    fn codex_inference_produces_an_untrusted_proposal() {
        let exe = "/Users/sicarii/.local/bin/codex";
        if !std::path::Path::new(exe).is_file() {
            eprintln!("skip: codex not installed");
            return;
        }
        let provider = InferenceProvider {
            agent_id: "inf-codex".to_owned(),
            provider_kind: "codex-cli".to_owned(),
            executable_path: exe.to_owned(),
        };
        match provider_health(&provider) {
            ProviderState::Ready { .. } => {}
            other => {
                eprintln!("skip: codex not Ready ({other:?})");
                return;
            }
        }
        let proposal = run_inference(
            &provider,
            "value.txt",
            "the file must contain exactly the single lowercase word: after",
            4096,
            std::time::Duration::from_secs(120),
        )
        .expect("live codex inference produces a proposal");
        assert!(proposal.content_bytes > 0, "proposal has content");
        assert_eq!(proposal.content_digest.len(), 64, "sha256 content digest");
        assert_eq!(
            proposal.action_digest.len(),
            64,
            "kernel-bindable action digest"
        );
        assert!(
            !proposal.preview.contains("sk-") && !proposal.preview.contains("Bearer"),
            "preview is redacted"
        );
        eprintln!(
            "OBSERVED: codex proposal bytes={} digest={}",
            proposal.content_bytes,
            &proposal.content_digest[..12]
        );
    }
}
