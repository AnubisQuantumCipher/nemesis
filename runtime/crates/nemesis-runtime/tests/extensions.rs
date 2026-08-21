use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::SigningKey;
use nemesis_runtime::{
    ExtensionError, McpGateway, McpToolDefinition, PluginCapabilities, PluginKind, PluginRegistry,
    SkillRecord, SkillStatus, UiPluginBroker, UnsignedPluginManifest, WasiPluginHost,
};

#[test]
fn skill_promotion_cannot_skip_required_states_or_trust_without_signature() {
    let mut skill = SkillRecord::new("safe-build", [1; 32], [2; 32]);
    assert!(skill.transition(SkillStatus::Trusted).is_err());
    for status in [
        SkillStatus::Staged,
        SkillStatus::StaticallyChecked,
        SkillStatus::TestedInSandbox,
        SkillStatus::Evaluated,
        SkillStatus::Approved,
    ] {
        skill.transition(status).unwrap();
    }
    assert!(skill.transition(SkillStatus::Trusted).is_err());
    skill.bind_approval("architect", [3; 64]).unwrap();
    skill.transition(SkillStatus::Trusted).unwrap();
}

#[test]
fn signed_plugin_cannot_exceed_granted_capabilities() {
    let signing = SigningKey::from_bytes(&[8; 32]);
    let requested = PluginCapabilities {
        network_hosts: BTreeSet::from(["api.example.invalid".to_owned()]),
        filesystem_roots: BTreeSet::new(),
        secrets: BTreeSet::new(),
        events: BTreeSet::from(["verification.completed".to_owned()]),
    };
    let manifest = UnsignedPluginManifest {
        id: "example-verifier".to_owned(),
        version: "1.0.0".to_owned(),
        protocol: "nemesis.plugin/v1".to_owned(),
        kind: PluginKind::WasiBackend,
        module_digest: [4; 32],
        capabilities: requested.clone(),
        evidence_types: BTreeSet::from(["example.receipt".to_owned()]),
    }
    .sign(&signing)
    .unwrap();
    let mut registry = PluginRegistry::new();
    assert!(
        registry
            .install(
                manifest.clone(),
                &signing.verifying_key(),
                &PluginCapabilities::default()
            )
            .is_err()
    );
    registry
        .install(manifest, &signing.verifying_key(), &requested)
        .unwrap();
    registry.record_data("example-verifier", [9; 32]).unwrap();
    registry.remove("example-verifier").unwrap();
    assert_eq!(registry.data_for("example-verifier"), &[[9; 32]]);
}

#[test]
fn wasi_host_runs_bounded_module_without_ambient_capabilities() {
    let wat = br#"(module
      (func (export "run") (result i32)
        i32.const 7))"#;
    let host = WasiPluginHost::new(PluginCapabilities::default(), 100_000, 1 << 20).unwrap();
    assert_eq!(host.execute_i32(wat, "run").unwrap(), 7);
}

#[test]
fn wasi_host_enforces_fuel_and_memory_limits() {
    let infinite = br#"(module
      (func (export "run") (result i32)
        (loop $forever (br $forever))
        i32.const 0))"#;
    let fuel_limited = WasiPluginHost::new(PluginCapabilities::default(), 1_000, 1 << 20).unwrap();
    assert!(fuel_limited.execute_i32(infinite, "run").is_err());

    let oversized_memory = br#"(module
      (memory 2)
      (func (export "run") (result i32) i32.const 0))"#;
    let memory_limited =
        WasiPluginHost::new(PluginCapabilities::default(), 10_000, 65_536).unwrap();
    assert!(memory_limited.execute_i32(oversized_memory, "run").is_err());
}

#[test]
fn ui_plugin_broker_rejects_arbitrary_host_operations() {
    let broker = UiPluginBroker::new(BTreeSet::from([
        "evidence.read".to_owned(),
        "selection.observe".to_owned(),
    ]));
    assert!(broker.authorize("evidence.read").is_ok());
    assert!(matches!(
        broker.authorize("shell.exec"),
        Err(ExtensionError::CapabilityDenied)
    ));
    assert!(
        broker
            .content_security_policy()
            .contains("default-src 'none'")
    );
}

#[test]
fn mcp_gateway_freezes_schema_strips_secrets_and_enforces_rate() {
    let required = BTreeSet::from(["network:api.example.invalid".to_owned()]);
    let mut gateway = McpGateway::new(1);
    gateway
        .register(McpToolDefinition {
            name: "checks.read".to_owned(),
            schema_digest: [5; 32],
            required_capabilities: required.clone(),
        })
        .unwrap();
    gateway.freeze();
    assert!(
        gateway
            .register(McpToolDefinition {
                name: "mutated".to_owned(),
                schema_digest: [6; 32],
                required_capabilities: BTreeSet::new(),
            })
            .is_err()
    );
    let inherited = BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
        ("GITHUB_TOKEN".to_owned(), "secret".to_owned()),
    ]);
    let invocation = gateway
        .invoke("checks.read", [5; 32], &required, &inherited)
        .unwrap();
    assert_eq!(
        invocation.environment,
        BTreeMap::from([("LANG".to_owned(), "C".to_owned())])
    );
    assert!(
        gateway
            .invoke("checks.read", [5; 32], &required, &inherited)
            .is_err()
    );
    assert!(
        gateway
            .invoke("checks.read", [7; 32], &required, &inherited)
            .is_err()
    );
}
