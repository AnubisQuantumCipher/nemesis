//! Governed rail-state end-to-end proof.
//!
//! The ignored test drives REAL missions through the full authority path —
//! compile → reviewed digests → Ada daemon create_grant / create_approval →
//! SPARK-proved attenuation and one-shot consumption → lane write → receipt →
//! tamper probe → replay — and then adopts the kernel-authorized bytes into
//! canonical rail state. It installs the shipped built-in skill pack and the
//! shipped built-in wasm plugin, so the ecosystem artifacts themselves are
//! exercised, not simulated.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nemesis_desktop::rails::{self, RailMutationRequest};
use nemesis_desktop::state_repo;
use nemesis_desktop::{
    CompiledMission, MissionCancellation, MissionDraftRequest, MissionExecutables,
    compile_local_contract, draft_local_mission, initialize_local_home, run_local_mission,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn test_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from("/tmp").join(format!("nemesis-{name}-{}-{nonce}", std::process::id()))
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn builtin_skill_pack_is_hash_bound_to_its_body() {
    let root = repository_root().join("library/skills/mission-hygiene");
    let manifest = manifest(&root.join("manifest.json"));
    assert_eq!(manifest["schema"], "nemesis.skill-pack/v1");
    let body = fs::read(root.join(manifest["bodyPath"].as_str().unwrap())).unwrap();
    assert_eq!(
        manifest["bodySha256"].as_str().unwrap(),
        hex::encode(Sha256::digest(&body)),
        "library skill body does not match its manifest hash"
    );
    assert_eq!(manifest["bodyBytes"].as_u64().unwrap(), body.len() as u64);
}

#[test]
fn builtin_plugin_is_hash_bound_and_deny_by_default() {
    let root = repository_root().join("library/plugins/rail-attest");
    let manifest = manifest(&root.join("manifest.json"));
    assert_eq!(manifest["schema"], "nemesis.plugin-pack/v1");
    let module = fs::read(root.join(manifest["modulePath"].as_str().unwrap())).unwrap();
    assert_eq!(
        manifest["moduleSha256"].as_str().unwrap(),
        hex::encode(Sha256::digest(&module)),
        "library plugin module does not match its manifest hash"
    );
    for capability in ["networkHosts", "filesystemRoots", "secrets", "events"] {
        assert_eq!(
            manifest["capabilities"][capability]
                .as_array()
                .unwrap()
                .len(),
            0,
            "built-in plugin must request no capabilities"
        );
    }
    // Tool schemas are digest-frozen: recompute each digest from the shipped
    // schema text.
    for tool in manifest["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        let schema_text = manifest["toolSchemas"][name].as_str().unwrap();
        assert_eq!(
            tool["schemaDigest"].as_str().unwrap(),
            hex::encode(Sha256::digest(schema_text.as_bytes())),
        );
    }
}

/// Draft, execute, and adopt one governed rail mutation through the complete
/// authority path. Returns the adopted mission's compiled contract.
fn govern(
    home: &Path,
    executables: &MissionExecutables,
    request: &RailMutationRequest,
) -> CompiledMission {
    let plan = rails::plan_mutation(home, request).unwrap();
    state_repo::ensure_entity_file(home, &request.rail, &request.id).unwrap();
    let status = state_repo::require_clean(home).unwrap();
    let drafted = draft_local_mission(
        home,
        &MissionDraftRequest {
            goal: plan.goal,
            workspace: status.root,
            relative_path: plan.relative_path,
            replacement: plan.replacement,
        },
        &MissionCancellation::default(),
    )
    .unwrap();
    let compiled = compile_local_contract(&fs::read(&drafted.path).unwrap()).unwrap();
    assert_eq!(compiled.contract_digest, drafted.compiled.contract_digest);
    let result = run_local_mission(
        &compiled,
        &compiled.contract_digest,
        &compiled.action_digest,
        home,
        executables,
        &MissionCancellation::default(),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(result.status, "VERIFIED");
    assert_eq!(result.tamper_verdict, "REJECTED");
    let record = state_repo::adopt_mission(home, &compiled.mission_id).unwrap();
    assert_eq!(record.rail, request.rail);
    assert_eq!(record.content_digest, compiled.content_digest);
    compiled
}

#[test]
#[ignore = "requires built Ada and Rust runtime executables plus macOS Keychain"]
fn governs_builtin_skill_and_plugin_through_the_full_authority_path() {
    let root = test_directory("rail-governance");
    let home = root.join("home");
    initialize_local_home(&home).unwrap();
    state_repo::initialize_state_repo(&home).unwrap();
    let executables = MissionExecutables::development(&repository_root());

    // 1. Install the shipped built-in skill pack through the kernel.
    let skill_root = repository_root().join("library/skills/mission-hygiene");
    let skill_manifest = manifest(&skill_root.join("manifest.json"));
    let body = fs::read_to_string(skill_root.join("skill.md")).unwrap();
    govern(
        &home,
        &executables,
        &RailMutationRequest {
            rail: "skills".to_owned(),
            verb: "propose".to_owned(),
            id: "mission-hygiene".to_owned(),
            payload: json!({"name": skill_manifest["name"], "body": body}),
        },
    );
    let skills = state_repo::read_rail_entities(&home, "skills").unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].value["status"], "proposed");
    assert_eq!(
        skills[0].value["bodyDigest"].as_str().unwrap(),
        skill_manifest["bodySha256"].as_str().unwrap(),
        "governed record must bind exactly the shipped, hash-bound body"
    );
    assert_eq!(
        rails::skills::load_body(&home, skills[0].value["bodyDigest"].as_str().unwrap()).unwrap(),
        body
    );

    // 2. Install the shipped built-in wasm plugin through the kernel.
    let plugin_root = repository_root().join("library/plugins/rail-attest");
    let plugin_manifest = manifest(&plugin_root.join("manifest.json"));
    let module_wat = fs::read_to_string(plugin_root.join("module.wat")).unwrap();
    govern(
        &home,
        &executables,
        &RailMutationRequest {
            rail: "integrations".to_owned(),
            verb: "register".to_owned(),
            id: "rail-attest".to_owned(),
            payload: json!({
                "name": plugin_manifest["name"],
                "kind": "wasm-plugin",
                "moduleWat": module_wat,
                "export": plugin_manifest["export"],
                "tools": plugin_manifest["tools"],
            }),
        },
    );

    // 3. Enable it through a second one-shot-reviewed mission.
    govern(
        &home,
        &executables,
        &RailMutationRequest {
            rail: "integrations".to_owned(),
            verb: "enable".to_owned(),
            id: "rail-attest".to_owned(),
            payload: json!({}),
        },
    );

    // 4. Execute inside the bounded WASI host; it attests the 14-rail surface.
    let receipt = rails::integrations::execute_plugin(&home, "rail-attest").unwrap();
    assert_eq!(receipt.verdict, "EXECUTED");
    assert_eq!(receipt.detail["result"], 14);
    assert_eq!(
        receipt.detail["moduleDigest"].as_str().unwrap(),
        plugin_manifest["moduleSha256"].as_str().unwrap()
    );

    // 5. The adoption chain carries all three governed mutations, and the
    //    security surface reports every one-shot approval as consumed.
    let chain = state_repo::load_adoption_chain(&home).unwrap();
    assert_eq!(chain.len(), 3);
    let security = rails::security::snapshot(&home).unwrap();
    assert_eq!(security["approvalTally"]["corrupt"], 0);
    assert!(security["approvalTally"]["consumed"].as_u64().unwrap() >= 3);
    assert_eq!(security["adoptionChain"]["length"], 3);

    fs::remove_dir_all(root).unwrap();
}
