//! Worker petition — a declared Agents-rail subprocess worker petitions the
//! SPARK/Ada kernel for a proposed action and receives a typed decision.
//!
//! This is the loop the architect named: NEMESIS already has the authority
//! kernel, but nothing turned a *declared* local subprocess agent (`codex-cli`
//! / `claude-code-cli`) into a petitioner of that kernel. Here it does, without
//! moving one byte of authority out of Ada/SPARK:
//!
//! 1. The petition is bound to a `nemesis.rail-agent/v1` row. API kinds
//!    (`openai-api` / `anthropic-api`) are permanently `UNAVAILABLE_NETWORK_DENIED`
//!    and never launch; a revoked agent or a missing executable never petitions.
//! 2. The capability SUBJECT is derived from the exact agent id, so the mission
//!    grant is issued to *this* agent; a petition presenting any other subject
//!    refuses `REFUSED_CAPABILITY`.
//! 3. The declared provider's exact executable is verified present + executable
//!    and bound (path, realpath, size) into the petition, and a NEMESIS protocol
//!    worker — a real, network-denied, Workspace-Safe subprocess — emits the
//!    `nemesis.worker/v1` `propose_action` on the agent's behalf, the adapter
//!    translation `adapters-v1.md` mandates. The raw provider is not executed
//!    in-loop: offline it cannot produce a proposal, and `codex --version`
//!    blocks under the strict non-interactive profile (both reproduced). Provider
//!    output is a proposal, never authority, and network is permanently denied.
//! 4. NEMESIS recomputes the normalized `action_digest` from the worker's OWN
//!    proposal and asks the kernel: read-only `capability_check`, then the
//!    one-shot `authorize_action`. The kernel decides. The verdict is exactly
//!    one of AUTHORIZED / REQUIRES_APPROVAL / REFUSED_* — never a silent execute.
//!
//! The worker has no way to widen its authority: the grant is derived
//! server-side (TS-002), the worker protocol forbids `complete`, and this module
//! never writes the lane or advances the mission past the kernel's decision.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

use crate::mission_runner::{
    CoreProcess, MissionCancellation, MissionExecutables, MissionRunError, ensure_private_dir,
    run_exact, sha256,
};
use crate::production::normalized_action_digest;
use crate::rails::agents;

/// A declared agent resolved to a launchable, network-free subprocess worker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAgent {
    pub agent_id: String,
    pub provider_kind: String,
    pub executable_path: String,
    pub worker_id: String,
}

/// The action a worker proposes: an exact repository-relative path plus the
/// lowercase SHA-256 of the replacement bytes. `estimated_bytes` is the reviewed
/// replacement size the kernel's byte budget is checked against — supplied by
/// the mission context, exactly as the sealed `authorize_action` path does.
#[derive(Clone, Debug)]
pub struct ProposedAction {
    pub relative_path: String,
    pub content_digest: String,
    pub estimated_bytes: u64,
}

/// Static, launch-free binding of the declared provider's on-disk identity into
/// the petition: the exact executable NEMESIS would run as this agent's isolated
/// subprocess — declared path, canonical realpath, executable bit, and size —
/// WITHOUT executing it. Launching the raw provider in-loop is not dependable
/// here: offline it cannot produce a `nemesis.worker/v1` proposal, and
/// `codex --version` blocks under the strict non-interactive Workspace-Safe
/// profile (both reproduced 2026-08-22). The proposal is emitted by the NEMESIS
/// protocol worker per `adapters-v1.md`; provider output is never authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBinding {
    pub executable_path: String,
    pub resolved_path: String,
    pub executable: bool,
    pub size_bytes: u64,
}

/// The kernel's typed decision on a worker petition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PetitionVerdict {
    /// In-grant AND a one-shot approval was consumed: the caller may execute.
    Authorized,
    /// In-grant but no matching one-shot approval — a human must approve first.
    RequiresApproval { reason: String },
    /// Out of grant, or the one-shot approval could not be consumed. Never executes.
    Refused {
        decision: String,
        reason: Option<String>,
    },
    /// The agent may not petition at all (network-denied / missing / revoked).
    Unavailable { status: String },
}

/// The full result of one petition: the bound agent identity, the declared
/// provider binding, the normalized action digest the kernel adjudicated, and
/// the typed verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetitionOutcome {
    pub agent_id: String,
    pub provider_kind: String,
    pub worker_id: String,
    pub action_digest: String,
    pub estimated_bytes: u64,
    pub binding: Option<ProviderBinding>,
    pub verdict: PetitionVerdict,
}

/// Derive the capability SUBJECT for an agent. The kernel grant's Subject is set
/// from this at `create`, so authority is bound to the exact declared agent: a
/// petition presenting any other worker id refuses `REFUSED_CAPABILITY`.
pub fn worker_id_for_agent(agent_id: &str) -> String {
    let digest = sha256(agent_id.as_bytes());
    format!("wrk_{}", &digest[..22])
}

/// Resolve an agent row into a launchable worker, or the exact reason it may not
/// petition. Reuses the rail's own `availability()` so there is a single source
/// of truth for network denial and executable presence — if `availability()`
/// ever reported an API kind as READY without network, this would surface it.
pub fn resolve_agent(entity: &Value) -> Result<ResolvedAgent, String> {
    if entity.get("status").and_then(Value::as_str) != Some("active") {
        return Err("AGENT_REVOKED".to_owned());
    }
    match agents::availability(entity) {
        "READY" => {}
        other => return Err(other.to_owned()),
    }
    let agent_id = entity
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "AGENT_MALFORMED".to_owned())?;
    let provider_kind = entity
        .get("providerKind")
        .and_then(Value::as_str)
        .ok_or_else(|| "AGENT_MALFORMED".to_owned())?;
    let executable_path = entity
        .get("executablePath")
        .and_then(Value::as_str)
        .ok_or_else(|| "AGENT_MALFORMED".to_owned())?;
    Ok(ResolvedAgent {
        worker_id: worker_id_for_agent(agent_id),
        agent_id: agent_id.to_owned(),
        provider_kind: provider_kind.to_owned(),
        executable_path: executable_path.to_owned(),
    })
}

/// Classify the kernel's read-only `capability_check` and (when capability is
/// granted) the one-shot `authorize_action` into exactly one typed verdict.
/// Authority stays in Ada; this only names the kernel's own decisions.
pub fn classify(capability: &Value, authorize: Option<&Value>) -> PetitionVerdict {
    let decision = capability
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("");
    if decision != "AUTHORIZED" {
        // Out of grant: `capability_check` already refused and no approval was
        // ever loaded or spent.
        return PetitionVerdict::Refused {
            decision: decision.to_owned(),
            reason: capability
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_owned),
        };
    }
    // Capability is in-grant; the one-shot `authorize_action` is authoritative.
    let Some(authorize) = authorize else {
        return PetitionVerdict::Refused {
            decision: "REFUSED_NO_AUTHORIZATION".to_owned(),
            reason: Some("authorize_action was not attempted".to_owned()),
        };
    };
    let final_decision = authorize
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("");
    let reason = authorize
        .get("reason")
        .and_then(Value::as_str)
        .map(str::to_owned);
    match final_decision {
        "AUTHORIZED" => PetitionVerdict::Authorized,
        "REFUSED_APPROVAL" if reason.as_deref() == Some("no_approval") => {
            PetitionVerdict::RequiresApproval {
                reason: "no_approval".to_owned(),
            }
        }
        other => PetitionVerdict::Refused {
            decision: other.to_owned(),
            reason,
        },
    }
}

/// Bind the declared provider's on-disk identity into the petition WITHOUT
/// launching it. `resolve_agent` already proved the executable is present and
/// executable (the rail's own `availability()` gate); this records the canonical
/// realpath and size so the exact provider binary is bound to the petition
/// evidence. The raw provider is deliberately not executed in-loop — see
/// `ProviderBinding`.
fn bind_provider(provider_exe: &str) -> ProviderBinding {
    let path = Path::new(provider_exe);
    let metadata = fs::metadata(path).ok();
    let executable = metadata
        .as_ref()
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false);
    let size_bytes = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
    let resolved_path = path
        .canonicalize()
        .ok()
        .map(|resolved| resolved.to_string_lossy().into_owned())
        .unwrap_or_default();
    ProviderBinding {
        executable_path: provider_exe.to_owned(),
        resolved_path,
        executable,
        size_bytes,
    }
}

/// Run the NEMESIS protocol worker under the sandbox and return the exact
/// `(relative_path, content_digest)` it proposed. The worker is bound to the
/// agent's derived id and confined to the lane; it can only propose.
fn emit_petition(
    executables: &MissionExecutables,
    lane: &Path,
    mission_id: &str,
    worker_id: &str,
    proposal: &ProposedAction,
    timeout: Duration,
    output_limit: usize,
    cancellation: &MissionCancellation,
) -> Result<(String, String), MissionRunError> {
    let output = run_exact(
        &executables.worker_runner,
        &[
            executables.worker.as_os_str(),
            lane.as_os_str(),
            OsStr::new("plan"),
            OsStr::new("--mission-id"),
            OsStr::new(mission_id),
            OsStr::new("--worker-id"),
            OsStr::new(worker_id),
            OsStr::new("--path"),
            OsStr::new(&proposal.relative_path),
            OsStr::new("--content-digest"),
            OsStr::new(&proposal.content_digest),
        ],
        None,
        timeout,
        output_limit,
        cancellation,
        "sandboxed worker petition",
    )?;
    let messages: Vec<Value> = output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice(line)
                .map_err(|error| MissionRunError::Refused(error.to_string()))
        })
        .collect::<Result<_, _>>()?;
    if messages.len() != 2
        || messages[0].get("method").and_then(Value::as_str) != Some("heartbeat")
        || messages[1].get("method").and_then(Value::as_str) != Some("propose_action")
    {
        return Err(MissionRunError::Refused(
            "worker did not emit heartbeat + propose_action".to_owned(),
        ));
    }
    let params = messages[1]
        .get("params")
        .ok_or_else(|| MissionRunError::Refused("propose_action missing params".to_owned()))?;
    let relative_path = params
        .get("relative_path")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            MissionRunError::Refused("propose_action missing relative_path".to_owned())
        })?;
    let content_digest = params
        .get("content_digest")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            MissionRunError::Refused("propose_action missing content_digest".to_owned())
        })?;
    // The worker's proposal is the petition. Bind the digest to exactly what the
    // worker emitted; a worker that echoes a different action than the staged
    // one is caught here before the kernel is ever asked.
    if relative_path != proposal.relative_path || content_digest != proposal.content_digest {
        return Err(MissionRunError::Refused(
            "worker proposal did not match the staged action".to_owned(),
        ));
    }
    Ok((relative_path.to_owned(), content_digest.to_owned()))
}

/// Run one worker petition against a mission that is already RUNNING with a
/// parent grant whose Subject is `worker_id_for_agent(agent.id)`. Binds the
/// declared provider's on-disk identity and runs a NEMESIS protocol worker under
/// the Workspace-Safe sandbox, recomputes the normalized action digest from the
/// worker's OWN proposal, and returns the kernel's typed verdict. Never writes
/// the lane and never advances the mission past the kernel's decision.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_agent_petition(
    core: &CoreProcess,
    executables: &MissionExecutables,
    lane: &Path,
    agent: &Value,
    mission_id: &str,
    scope_digest: &str,
    proposal: &ProposedAction,
    timeout: Duration,
    output_limit: usize,
    cancellation: &MissionCancellation,
) -> Result<PetitionOutcome, MissionRunError> {
    let resolved = match resolve_agent(agent) {
        Ok(resolved) => resolved,
        Err(status) => {
            // The agent may not petition. No subprocess is launched, the kernel
            // is never asked, and no authority is spent.
            return Ok(PetitionOutcome {
                agent_id: agent
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                provider_kind: agent
                    .get("providerKind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                worker_id: String::new(),
                action_digest: String::new(),
                estimated_bytes: proposal.estimated_bytes,
                binding: None,
                verdict: PetitionVerdict::Unavailable { status },
            });
        }
    };

    let binding = bind_provider(&resolved.executable_path);

    let (relative_path, content_digest) = emit_petition(
        executables,
        lane,
        mission_id,
        &resolved.worker_id,
        proposal,
        timeout,
        output_limit,
        cancellation,
    )?;

    // The worker chose the path + content; NEMESIS never invents the digest.
    let action_digest = normalized_action_digest(&relative_path, &content_digest);

    let capability = core.request(json!({
        "schema": "nemesis.local/v1",
        "command": "capability_check",
        "mission_id": mission_id,
        "worker_id": resolved.worker_id,
        "scope_digest": scope_digest,
        "action_digest": action_digest,
        "estimated_bytes": proposal.estimated_bytes,
    }))?;
    let authorize = if capability.get("decision").and_then(Value::as_str) == Some("AUTHORIZED") {
        Some(core.request(json!({
            "schema": "nemesis.local/v1",
            "command": "authorize_action",
            "mission_id": mission_id,
            "worker_id": resolved.worker_id,
            "scope_digest": scope_digest,
            "action_digest": action_digest,
            "estimated_bytes": proposal.estimated_bytes,
        }))?)
    } else {
        None
    };
    let verdict = classify(&capability, authorize.as_ref());

    Ok(PetitionOutcome {
        agent_id: resolved.agent_id,
        provider_kind: resolved.provider_kind,
        worker_id: resolved.worker_id,
        action_digest,
        estimated_bytes: proposal.estimated_bytes,
        binding: Some(binding),
        verdict,
    })
}

/// Derive a unique petition mission id from the agent plus a monotonic nonce, so
/// repeated petitions never collide on the one-shot `create`.
fn petition_mission_id(agent_id: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let digest = sha256(format!("{agent_id}:{nonce}").as_bytes());
    format!("mis_{}", &digest[..22])
}

/// Set up a fresh, isolated petition mission from a declared agent row and run
/// it end to end: a private lane, a short-lived Core daemon, mission `create`
/// (Subject bound to the agent) -> `authorize` -> `create_grant` ->
/// (`create_approval` iff `reviewed`) -> `run`, then `run_agent_petition`. The
/// kernel decides; nothing here executes the proposed action. This is the
/// app-facing path that lets a genuinely REGISTERED agent row petition.
pub(crate) fn petition_agent_action(
    home: &Path,
    executables: &MissionExecutables,
    agent: &Value,
    proposal: &ProposedAction,
    reviewed: bool,
    cancellation: &MissionCancellation,
) -> Result<PetitionOutcome, MissionRunError> {
    // Resolve first: an unavailable agent must never create a mission or a lane.
    let resolved = match resolve_agent(agent) {
        Ok(resolved) => resolved,
        Err(status) => {
            return Ok(PetitionOutcome {
                agent_id: agent
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                provider_kind: agent
                    .get("providerKind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                worker_id: String::new(),
                action_digest: String::new(),
                estimated_bytes: proposal.estimated_bytes,
                binding: None,
                verdict: PetitionVerdict::Unavailable { status },
            });
        }
    };

    let mission_id = petition_mission_id(&resolved.agent_id);
    let grant_id = format!("cap_{}", &mission_id[4..]);
    let approval_id = format!("apr_{}", &mission_id[4..]);

    // Private, isolated lane (the sandbox scope). A petition never writes it.
    let lane_parent = home.join("petitions");
    ensure_private_dir(&lane_parent)?;
    let lane = lane_parent.join(&mission_id[4..]);
    ensure_private_dir(&lane)?;
    let lane = lane
        .canonicalize()
        .map_err(|error| MissionRunError::Refused(format!("petition lane unavailable: {error}")))?;
    let scope_digest = sha256(lane.to_string_lossy().as_bytes());

    // Short-pathed Core home so the Unix socket stays within the sockaddr limit.
    let core_root = home.join(".core");
    ensure_private_dir(&core_root)?;
    let core_home = core_root.join(&mission_id[4..16]);
    ensure_private_dir(&core_home)?;

    let contract_digest = sha256(format!("nemesis.petition/{mission_id}").as_bytes());
    let source_digest = sha256(format!("nemesis.petition-source/{mission_id}").as_bytes());
    let action_digest = normalized_action_digest(&proposal.relative_path, &proposal.content_digest);

    let mut core = CoreProcess::new(&executables.daemon, &core_home);
    core.start()?;

    let ok = |response: Value| -> Result<(), MissionRunError> {
        if response.get("status").and_then(Value::as_str) != Some("OK") {
            return Err(MissionRunError::Refused(format!(
                "Core refused petition setup: {response}"
            )));
        }
        Ok(())
    };
    ok(core.request(json!({
        "schema": "nemesis.local/v1", "command": "create",
        "mission_id": mission_id, "worker_id": resolved.worker_id,
        "contract_digest": contract_digest, "scope_digest": scope_digest,
        "source_digest": source_digest,
    }))?)?;
    ok(core.request(json!({
        "schema": "nemesis.local/v1", "command": "authorize",
        "mission_id": mission_id, "contract_digest": contract_digest,
    }))?)?;
    ok(core.request(json!({
        "schema": "nemesis.local/v1", "command": "create_grant",
        "mission_id": mission_id, "grant_id": grant_id,
    }))?)?;
    if reviewed {
        ok(core.request(json!({
            "schema": "nemesis.local/v1", "command": "create_approval",
            "mission_id": mission_id, "approval_id": approval_id,
            "action_digest": action_digest,
        }))?)?;
    }
    ok(core.request(json!({
        "schema": "nemesis.local/v1", "command": "run", "mission_id": mission_id,
    }))?)?;

    run_agent_petition(
        &core,
        executables,
        &lane,
        agent,
        &mission_id,
        &scope_digest,
        proposal,
        Duration::from_secs(30),
        1024 * 1024,
        cancellation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::production::compile_local_contract;
    use serde_json::json;

    fn agent(kind: &str, executable: Option<&str>, status: &str) -> Value {
        let mut entity = json!({
            "schema": "nemesis.rail-agent/v1",
            "id": "petition-agent",
            "name": "Petition Agent",
            "providerKind": kind,
            "executablePath": executable,
            "roles": ["builder"],
            "consequenceCeiling": "decision-boundary",
            "budget": {"costMicrounits": 1, "inputTokens": 1, "outputTokens": 1},
            "status": status,
            "revision": 1,
        });
        if executable.is_none() {
            entity["executablePath"] = Value::Null;
        }
        entity
    }

    #[test]
    fn resolve_agent_denies_api_kinds_without_network() {
        // Tripwire: if an API provider ever became READY without network, this
        // flips and fails. openai-api / anthropic-api must never petition.
        for kind in ["openai-api", "anthropic-api"] {
            let entity = agent(kind, None, "active");
            assert_eq!(
                resolve_agent(&entity).unwrap_err(),
                "UNAVAILABLE_NETWORK_DENIED",
                "{kind} must be permanently network-denied"
            );
        }
    }

    #[test]
    fn resolve_agent_requires_a_present_executable() {
        let missing = agent("codex-cli", Some("/nemesis/definitely/not/here"), "active");
        assert_eq!(resolve_agent(&missing).unwrap_err(), "MISSING_EXECUTABLE");

        // `/usr/bin/true` exists on macOS and is a subprocess kind → READY.
        let present = agent("generic-subprocess", Some("/usr/bin/true"), "active");
        let resolved = resolve_agent(&present).expect("present executable resolves");
        assert_eq!(resolved.provider_kind, "generic-subprocess");
        assert_eq!(resolved.executable_path, "/usr/bin/true");
        assert_eq!(resolved.worker_id, worker_id_for_agent("petition-agent"));
    }

    #[test]
    fn resolve_agent_refuses_revoked_rows() {
        let revoked = agent("codex-cli", Some("/usr/bin/true"), "revoked");
        assert_eq!(resolve_agent(&revoked).unwrap_err(), "AGENT_REVOKED");
    }

    #[test]
    fn worker_id_binds_to_agent_identity() {
        let a = worker_id_for_agent("agent-a");
        let b = worker_id_for_agent("agent-b");
        assert_ne!(a, b, "distinct agents get distinct capability subjects");
        assert_eq!(a, worker_id_for_agent("agent-a"), "derivation is stable");
        assert_eq!(a.len(), 26);
        assert!(a.starts_with("wrk_"));
        assert!(
            a[4..]
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()),
            "worker id is a valid wrk_ subject: {a}"
        );
    }

    #[test]
    fn petition_digest_matches_the_mission_pipeline() {
        // The digest a petition recomputes from a worker proposal MUST equal the
        // digest the reviewed mission pipeline binds for the same action.
        let contract = json!({
            "schema": "nemesis.desktop-mission/v1",
            "missionId": "mis_0000000000000000000000",
            "goal": "petition digest parity",
            "workspace": "/tmp/nemesis-petition-workspace",
            "baseRevision": "0123456789abcdef0123456789abcdef01234567",
            "action": {
                "kind": "replace_utf8",
                "relativePath": "src/value.txt",
                "expectedSha256":
                    "1111111111111111111111111111111111111111111111111111111111111111",
                "replacement": "after\n",
            },
            "authority": {"network": false, "push": false, "publish": false, "secrets": false},
            "budgets": {"maxWriteBytes": 4096, "maxRuntimeSeconds": 60, "maxOutputBytes": 1024},
            "completion": ["git_diff_check", "content_match"],
        });
        let compiled =
            compile_local_contract(serde_json::to_string(&contract).unwrap().as_bytes()).unwrap();
        assert_eq!(
            compiled.action_digest,
            normalized_action_digest(&compiled.relative_path, &compiled.content_digest),
        );
    }

    #[test]
    fn classify_maps_every_kernel_decision_to_a_typed_verdict() {
        let refused_cap = json!({"decision": "REFUSED_CAPABILITY"});
        assert_eq!(
            classify(&refused_cap, None),
            PetitionVerdict::Refused {
                decision: "REFUSED_CAPABILITY".to_owned(),
                reason: None
            }
        );

        let refused_budget = json!({"decision": "REFUSED_BUDGET"});
        assert_eq!(
            classify(&refused_budget, None),
            PetitionVerdict::Refused {
                decision: "REFUSED_BUDGET".to_owned(),
                reason: None
            }
        );

        let capable = json!({"decision": "AUTHORIZED"});
        let authorized = json!({"decision": "AUTHORIZED"});
        assert_eq!(
            classify(&capable, Some(&authorized)),
            PetitionVerdict::Authorized
        );

        let no_approval = json!({"decision": "REFUSED_APPROVAL", "reason": "no_approval"});
        assert_eq!(
            classify(&capable, Some(&no_approval)),
            PetitionVerdict::RequiresApproval {
                reason: "no_approval".to_owned()
            }
        );

        let replayed = json!({"decision": "REFUSED_APPROVAL", "reason": "approval_replayed"});
        assert_eq!(
            classify(&capable, Some(&replayed)),
            PetitionVerdict::Refused {
                decision: "REFUSED_APPROVAL".to_owned(),
                reason: Some("approval_replayed".to_owned())
            }
        );

        // Capability granted but the one-shot was never attempted is never a
        // silent success.
        assert!(matches!(
            classify(&capable, None),
            PetitionVerdict::Refused { .. }
        ));
    }

    // ---- Crown jewel: a real declared subprocess agent petitions the kernel ----

    use crate::mission_runner::CoreProcess;
    use std::fs;
    use std::path::PathBuf;

    const CONTRACT: &str = "abababababababababababababababababababababababababababababababab";
    const SOURCE: &str = "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";

    fn executables() -> MissionExecutables {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        MissionExecutables::development(&root)
    }

    fn short_dir(tag: &str) -> PathBuf {
        let dir = PathBuf::from(format!("/tmp/nemp-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Prefer the real Codex CLI when present (that is the declared subprocess
    /// worker the mission targets); otherwise fall back to a portable POSIX
    /// binary so the loop still runs everywhere. The verdict is the kernel's
    /// either way — the provider only has to launch under the sandbox.
    fn demo_provider() -> (&'static str, String) {
        let codex = "/Users/sicarii/.local/bin/codex";
        if Path::new(codex).is_file() {
            ("codex-cli", codex.to_owned())
        } else {
            ("generic-subprocess", "/usr/bin/true".to_owned())
        }
    }

    fn built_agent(id: &str, kind: &str, executable: &str) -> Value {
        json!({
            "schema": "nemesis.rail-agent/v1",
            "id": id,
            "name": "Petition Demo",
            "providerKind": kind,
            "executablePath": executable,
            "roles": ["builder"],
            "consequenceCeiling": "decision-boundary",
            "budget": {"costMicrounits": 1, "inputTokens": 1, "outputTokens": 1},
            "status": "active",
            "revision": 1,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn drive(
        core: &CoreProcess,
        execs: &MissionExecutables,
        lane: &Path,
        scope_digest: &str,
        suffix: &str,
        setup_agent_id: &str,
        petition_agent: &Value,
        proposal: &ProposedAction,
        issue_approval: bool,
    ) -> PetitionOutcome {
        let mission_id = format!("mis_{:0>22}", suffix);
        let grant_id = format!("cap_{}", &mission_id[4..]);
        let approval_id = format!("apr_{}", &mission_id[4..]);
        let setup_worker = worker_id_for_agent(setup_agent_id);
        let cancellation = MissionCancellation::default();

        let ok = |value: Value| {
            assert_eq!(
                value.get("status").and_then(Value::as_str),
                Some("OK"),
                "core refused: {value}"
            );
        };
        ok(core
            .request(json!({
                "schema": "nemesis.local/v1", "command": "create",
                "mission_id": mission_id, "worker_id": setup_worker,
                "contract_digest": CONTRACT, "scope_digest": scope_digest,
                "source_digest": SOURCE,
            }))
            .unwrap());
        ok(core
            .request(json!({
                "schema": "nemesis.local/v1", "command": "authorize",
                "mission_id": mission_id, "contract_digest": CONTRACT,
            }))
            .unwrap());
        ok(core
            .request(json!({
                "schema": "nemesis.local/v1", "command": "create_grant",
                "mission_id": mission_id, "grant_id": grant_id,
            }))
            .unwrap());
        if issue_approval {
            let action_digest =
                normalized_action_digest(&proposal.relative_path, &proposal.content_digest);
            ok(core
                .request(json!({
                    "schema": "nemesis.local/v1", "command": "create_approval",
                    "mission_id": mission_id, "approval_id": approval_id,
                    "action_digest": action_digest,
                }))
                .unwrap());
        }
        ok(core
            .request(json!({
                "schema": "nemesis.local/v1", "command": "run", "mission_id": mission_id,
            }))
            .unwrap());

        run_agent_petition(
            core,
            execs,
            lane,
            petition_agent,
            &mission_id,
            scope_digest,
            proposal,
            Duration::from_secs(30),
            1024 * 1024,
            &cancellation,
        )
        .unwrap()
    }

    #[test]
    #[ignore = "requires built Ada daemon + Rust runtime worker binaries"]
    fn declared_subprocess_agent_petitions_the_kernel_and_the_kernel_decides() {
        let execs = executables();
        assert!(
            execs.is_ready(),
            "build the Ada daemon and runtime binaries first"
        );

        let (provider_kind, provider_exe) = demo_provider();
        let agent_id = "petition-demo-agent";
        let other_id = "petition-demo-other";
        let good_agent = built_agent(agent_id, provider_kind, &provider_exe);

        let home = short_dir("home");
        let lane = short_dir("lane").canonicalize().unwrap();
        let scope_digest = sha256(lane.to_string_lossy().as_bytes());

        let mut core = CoreProcess::new(&execs.daemon, &home);
        core.start().expect("core daemon starts");

        let in_grant = ProposedAction {
            relative_path: "src/value.txt".to_owned(),
            content_digest: sha256(b"after\n"),
            estimated_bytes: 6,
        };

        // 1. In-grant + approved -> AUTHORIZED. A real NEMESIS worker subprocess
        //    emitted the proposal; the declared provider binary is bound.
        let authorized = drive(
            &core,
            &execs,
            &lane,
            &scope_digest,
            "a1",
            agent_id,
            &good_agent,
            &in_grant,
            true,
        );
        assert_eq!(authorized.verdict, PetitionVerdict::Authorized);
        let binding = authorized.binding.expect("declared provider is bound");
        assert!(
            binding.executable,
            "declared provider is a present executable"
        );
        assert!(!binding.resolved_path.is_empty(), "provider realpath bound");
        assert_eq!(authorized.worker_id, worker_id_for_agent(agent_id));

        // 2. In-grant + NO approval -> REQUIRES_APPROVAL (never a silent execute).
        let requires = drive(
            &core,
            &execs,
            &lane,
            &scope_digest,
            "a2",
            agent_id,
            &good_agent,
            &in_grant,
            false,
        );
        assert_eq!(
            requires.verdict,
            PetitionVerdict::RequiresApproval {
                reason: "no_approval".to_owned()
            }
        );

        // 3. Over-budget proposal -> the kernel refuses REFUSED_BUDGET (tripwire:
        //    widening the 4096-byte grant flips this).
        let over_budget = ProposedAction {
            relative_path: "src/value.txt".to_owned(),
            content_digest: sha256(b"after\n"),
            estimated_bytes: 4_097,
        };
        let refused_budget = drive(
            &core,
            &execs,
            &lane,
            &scope_digest,
            "a3",
            agent_id,
            &good_agent,
            &over_budget,
            true,
        );
        assert_eq!(
            refused_budget.verdict,
            PetitionVerdict::Refused {
                decision: "REFUSED_BUDGET".to_owned(),
                reason: None
            }
        );

        // 4. Wrong subject: a DIFFERENT agent petitions a grant issued to the
        //    first -> REFUSED_CAPABILITY (proves the agent-subject binding).
        let other_agent = built_agent(other_id, provider_kind, &provider_exe);
        let refused_subject = drive(
            &core,
            &execs,
            &lane,
            &scope_digest,
            "a4",
            agent_id,
            &other_agent,
            &in_grant,
            true,
        );
        assert_eq!(
            refused_subject.verdict,
            PetitionVerdict::Refused {
                decision: "REFUSED_CAPABILITY".to_owned(),
                reason: None
            }
        );

        // 5. An API-kind agent never launches and never petitions.
        let api_agent = json!({
            "schema": "nemesis.rail-agent/v1", "id": "petition-api",
            "name": "API", "providerKind": "anthropic-api", "executablePath": Value::Null,
            "roles": ["reviewer"], "consequenceCeiling": "advisory",
            "budget": {"costMicrounits": 1, "inputTokens": 1, "outputTokens": 1},
            "status": "active", "revision": 1,
        });
        let unavailable = run_agent_petition(
            &core,
            &execs,
            &lane,
            &api_agent,
            "mis_0000000000000000000api",
            &scope_digest,
            &in_grant,
            Duration::from_secs(30),
            1024 * 1024,
            &MissionCancellation::default(),
        )
        .unwrap();
        assert_eq!(
            unavailable.verdict,
            PetitionVerdict::Unavailable {
                status: "UNAVAILABLE_NETWORK_DENIED".to_owned()
            }
        );
        assert!(
            unavailable.binding.is_none(),
            "network-denied agent must never launch or bind a subprocess"
        );

        drop(core);
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&lane);
    }
}
