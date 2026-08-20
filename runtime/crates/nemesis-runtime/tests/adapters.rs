use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use nemesis_protocol::WorkerMessage;
use nemesis_runtime::{
    AdapterAvailability, AdapterProfile, AuthorityFingerprint, ConsequenceCeiling,
    GenericSubprocessAdapter, ProviderKind, UsageBudget, UsageLedger, UsageRecord, WorkerRole,
    choose_fallback,
};

const MESSAGE: &str = r#"{"jsonrpc":"2.0","id":1,"protocol":"nemesis.worker/v1","mission_id":"mis_0000000000000000000000","worker_id":"wrk_0000000000000000000000","method":"heartbeat","params":{}}\n"#;

#[test]
fn generic_subprocess_adapter_runs_protocol_worker_in_workspace_safe() {
    let temp = tempfile::tempdir().unwrap();
    let lane = temp.path().join("lane");
    fs::create_dir(&lane).unwrap();
    let adapter = GenericSubprocessAdapter::new(PathBuf::from("/usr/bin/printf"));
    let messages = adapter
        .run(&lane, &[OsString::from(MESSAGE)], 65_536)
        .expect("bounded protocol output");
    assert_eq!(messages.len(), 1);
    assert!(matches!(messages[0].message, WorkerMessage::Heartbeat));
}

#[test]
fn missing_provider_is_typed_unavailable_not_a_fallback_success() {
    let profile = AdapterProfile::subprocess(
        ProviderKind::Codex,
        PathBuf::from("/definitely/missing/codex"),
        vec![WorkerRole::Builder],
        ConsequenceCeiling::DecisionBoundary,
    );
    assert!(matches!(
        profile.availability(),
        AdapterAvailability::Unavailable { .. }
    ));
}

#[test]
fn fallback_preserves_authority_and_consequence_ceiling() {
    let authority = AuthorityFingerprint([0x42; 32]);
    let required_role = WorkerRole::Reviewer;
    let candidates = vec![
        AdapterProfile::unavailable(
            ProviderKind::ClaudeCode,
            vec![required_role],
            ConsequenceCeiling::SafetyCritical,
            "provider binary missing",
        ),
        AdapterProfile::available(
            ProviderKind::LocalModel,
            vec![required_role],
            ConsequenceCeiling::DecisionBoundary,
            "local-test",
        ),
    ];
    let selected = choose_fallback(
        &candidates,
        required_role,
        ConsequenceCeiling::DecisionBoundary,
        authority,
    )
    .expect("compatible local fallback");
    assert_eq!(selected.provider, ProviderKind::LocalModel);
    assert_eq!(selected.authority, authority);
    assert_eq!(selected.consequence, ConsequenceCeiling::DecisionBoundary);
}

#[test]
fn usage_accounting_refuses_over_budget_without_mutation() {
    let mut ledger = UsageLedger::new(UsageBudget {
        cost_microunits: 1_000,
        input_tokens: 2_000,
        output_tokens: 1_000,
        context_tokens: 4_000,
    });
    ledger
        .record(UsageRecord {
            cost_microunits: 100,
            input_tokens: 200,
            output_tokens: 50,
            context_tokens: 500,
        })
        .unwrap();
    let before = ledger.clone();
    assert!(
        ledger
            .record(UsageRecord {
                cost_microunits: 901,
                input_tokens: 1,
                output_tokens: 1,
                context_tokens: 1,
            })
            .is_err()
    );
    assert_eq!(ledger, before);
}

#[test]
fn api_profiles_require_explicit_network_and_secret_capabilities() {
    for provider in [ProviderKind::OpenAiApi, ProviderKind::AnthropicApi] {
        let profile = AdapterProfile::api(
            provider,
            vec![WorkerRole::Builder],
            ConsequenceCeiling::Advisory,
            false,
            false,
        );
        assert!(matches!(
            profile.availability(),
            AdapterAvailability::Unavailable { .. }
        ));
    }
}
