use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use nemesis_runtime::{
    AdapterAvailability, AdapterProfile, ConsequenceCeiling, ProviderKind, WorkerRole,
};
use serde_json::json;

fn executable(name: &str) -> PathBuf {
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .map(|directory| directory.join(name))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(format!("/unavailable/{name}")))
}

fn main() -> ExitCode {
    let profiles = [
        AdapterProfile::subprocess(
            ProviderKind::Codex,
            executable("codex"),
            vec![WorkerRole::Builder, WorkerRole::Recovery],
            ConsequenceCeiling::DecisionBoundary,
        ),
        AdapterProfile::subprocess(
            ProviderKind::ClaudeCode,
            executable("claude"),
            vec![WorkerRole::Reviewer, WorkerRole::RedTeam],
            ConsequenceCeiling::DecisionBoundary,
        ),
        AdapterProfile::subprocess(
            ProviderKind::LocalModel,
            executable("ollama"),
            vec![WorkerRole::Planner],
            ConsequenceCeiling::Advisory,
        ),
        AdapterProfile::api(
            ProviderKind::OpenAiApi,
            vec![WorkerRole::Builder],
            ConsequenceCeiling::Advisory,
            false,
            false,
        ),
        AdapterProfile::api(
            ProviderKind::AnthropicApi,
            vec![WorkerRole::Reviewer],
            ConsequenceCeiling::Advisory,
            false,
            false,
        ),
    ];
    let rows: Vec<_> = profiles
        .iter()
        .map(|profile| {
            let (status, detail) = match profile.availability() {
                AdapterAvailability::Available { version } => ("AVAILABLE", version),
                AdapterAvailability::Unavailable { reason } => ("UNAVAILABLE", reason),
            };
            json!({
                "provider": format!("{:?}", profile.provider),
                "status": status,
                "detail": detail,
                "consequence_ceiling": format!("{:?}", profile.consequence_ceiling),
                "roles": profile.roles.iter().map(|role| format!("{role:?}")).collect::<Vec<_>>()
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string(&json!({
            "schema": "nemesis.adapters/v1",
            "providers": rows,
            "authority_source": "mission contract",
            "fallback_expands_authority": false
        }))
        .expect("adapter health JSON serializes")
    );
    ExitCode::SUCCESS
}
