use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use surrealdb::Surreal;
use surrealdb::engine::local::{Db, SurrealKv};
use taskflow_host_bridge::HostBridgeAdapterOperations;

#[path = "support/runtime_consumption.rs"]
mod runtime_consumption_support;

use runtime_consumption_support::PersistentRuntimeFixture;

const ZOMBIE_D_TAMPER_STATE_DIR_ENV: &str = "VIDA_ZOMBIE_D_TAMPER_STATE_DIR";
const ZOMBIE_D_TAMPER_KIND_ENV: &str = "VIDA_ZOMBIE_D_TAMPER_KIND";
const ZOMBIE_D_TAMPER_HELPER_TEST: &str = "zombie_d_apply_launcher_snapshot_tamper_from_env";

#[derive(Debug, Clone, Copy)]
enum ZombieDTamperKind {
    AuthorityId,
    ContentHash,
    DefaultFlowAlias,
}

impl ZombieDTamperKind {
    fn as_env(self) -> &'static str {
        match self {
            Self::AuthorityId => "authority_id",
            Self::ContentHash => "content_hash",
            Self::DefaultFlowAlias => "default_flow_alias",
        }
    }

    fn tampered_marker(self) -> &'static str {
        match self {
            Self::AuthorityId => "team-flow-authority:tampered",
            Self::ContentHash => "tampered-content-hash",
            Self::DefaultFlowAlias => "tampered-flow-alias",
        }
    }

    fn from_env(value: &str) -> Self {
        match value {
            "authority_id" => Self::AuthorityId,
            "content_hash" => Self::ContentHash,
            "default_flow_alias" => Self::DefaultFlowAlias,
            other => panic!("unknown ZOMBIE-D tamper kind: {other}"),
        }
    }

    fn apply(self, row: &mut serde_json::Value) {
        match self {
            Self::AuthorityId => {
                row["compiled_bundle"]["team_flow_authority"]["authority_id"] =
                    serde_json::json!("team-flow-authority:tampered");
            }
            Self::ContentHash => {
                row["compiled_bundle"]["team_flow_authority"]["content_blake3"] =
                    serde_json::json!("tampered-content-hash");
            }
            Self::DefaultFlowAlias => {
                row["compiled_bundle"]["team_flow_authority"]["selected_config"]["authority_selection"]
                    ["default_flow_id"] = serde_json::json!("tampered-flow-alias");
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TamperedAuthorityOutcome {
    Blocked,
    Canonicalized,
}

impl TamperedAuthorityOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Canonicalized => "canonicalized",
        }
    }
}

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn canonical_host_bridge_config_path() -> PathBuf {
    let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vida.config.yaml");
    std::fs::canonicalize(&source_path).unwrap_or_else(|error| {
        panic!(
            "canonical VIDA config source should exist at {}: {error}",
            source_path.display()
        )
    })
}

fn canonical_host_bridge_config() -> serde_json::Value {
    let yaml: serde_yaml::Value = serde_yaml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vida.config.yaml"
    )))
    .expect("canonical VIDA config should parse");
    serde_json::to_value(yaml).expect("canonical VIDA config should convert to json")
}

#[test]
fn host_bridge_fixture_config_source_is_canonical_worktree_root_path() {
    let source_path = canonical_host_bridge_config_path();
    assert!(source_path.is_file(), "canonical config should be a file");
    let worktree_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("vida crate should have a worktree root ancestor")
        .join("vida.config.yaml");
    assert_eq!(
        source_path,
        std::fs::canonicalize(&worktree_root).expect("worktree-root config should exist")
    );
}

fn canonical_host_bridge_adapter_contract() -> HostBridgeAdapterOperations {
    let config = canonical_host_bridge_config();
    let registry = config
        .pointer("/host_environment/systems/codex/host_tool_bridge")
        .expect("canonical host bridge registry should exist");
    HostBridgeAdapterOperations::from_registry_value(registry)
        .expect("canonical host bridge registry should resolve")
}

fn canonical_host_bridge_backend_id() -> String {
    canonical_host_bridge_config()
        .pointer("/party_chat/single_agent/backend")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .expect("canonical host bridge backend should exist")
}

fn canonical_host_bridge_execution_boundary() -> String {
    canonical_host_bridge_config()
        .pointer("/host_environment/systems/codex/execution_boundary")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .expect("canonical host bridge execution boundary should exist")
}

fn canonical_default_delivery_dispatch_contract() -> serde_json::Value {
    let lanes = [
        (
            "analyst",
            "development_specification",
            "specification",
            "business_analyst",
            "test_author",
            true,
            false,
        ),
        (
            "test_author",
            "development_test_author",
            "test_authoring",
            "worker",
            "coach_test_gate",
            true,
            false,
        ),
        (
            "coach_test_gate",
            "development_coach",
            "coach",
            "coach",
            "developer",
            true,
            false,
        ),
        (
            "developer",
            "development_implementer",
            "implementation",
            "worker",
            "coach_implementation_gate",
            true,
            false,
        ),
        (
            "coach_implementation_gate",
            "development_coach",
            "coach",
            "coach",
            "duplication_reviewer",
            true,
            false,
        ),
        (
            "duplication_reviewer",
            "development_verifier",
            "review",
            "verifier",
            "tester",
            true,
            false,
        ),
        (
            "tester",
            "development_verifier",
            "verification",
            "verifier",
            "prover",
            true,
            false,
        ),
        (
            "prover",
            "development_verifier",
            "quality_gate",
            "prover",
            "release_closure",
            true,
            false,
        ),
        (
            "release_closure",
            "development_verifier",
            "release_readiness",
            "prover",
            "",
            true,
            true,
        ),
    ];
    let resolved_lanes = lanes.iter().map(|(node_id, dispatch_alias, task_class, runtime_role, next_node, included, terminal)| {
        serde_json::json!({
            "node_id": node_id,
            "lane_id": node_id,
            "role_id": node_id,
            "dispatch_target": dispatch_alias,
            "dispatch_alias": dispatch_alias,
            "task_class": task_class,
            "runtime_role": runtime_role,
            "packet_template_kind": if *task_class == "coach" { "coach_review_packet" } else if *task_class == "verification" || *task_class == "release_readiness" { "verifier_proof_packet" } else { "delivery_task_packet" },
            "closure_class": "proof",
            "stage": "execution",
            "inclusion_rule": if *included { "always" } else { "when_review_triggered" },
            "included": included,
            "required": true,
            "next_node": if next_node.is_empty() { serde_json::Value::Null } else { serde_json::json!(next_node) },
            "completion_blocker": "pending_verification_evidence",
            "evidence_requirements": [],
            "proof_gates": {"required_outputs": []},
            "command_ref": "agent-init-worker",
            "rework": {"targets": if *task_class == "release_readiness" { serde_json::json!([]) } else { serde_json::json!(["developer"]) }},
            "terminal": terminal
        })
    }).collect::<Vec<_>>();
    let mut lane_catalog = serde_json::Map::new();
    let mut dispatch_target_index = serde_json::Map::new();
    let mut runtime_role_index = serde_json::Map::new();
    for lane in &resolved_lanes {
        let node_id = lane["node_id"].as_str().expect("canonical node id");
        lane_catalog.insert(node_id.to_owned(), lane.clone());
        for (field_name, index) in [
            ("dispatch_target", &mut dispatch_target_index),
            ("runtime_role", &mut runtime_role_index),
        ] {
            for value in [lane[field_name].as_str(), Some(node_id)]
                .into_iter()
                .flatten()
            {
                index
                    .entry(value.to_owned())
                    .or_insert_with(|| serde_json::Value::Array(Vec::new()))
                    .as_array_mut()
                    .expect("canonical index array")
                    .push(serde_json::json!(node_id));
            }
        }
    }
    serde_json::json!({
        "status": "ready",
        "blocker_codes": [],
        "fallback_role": "orchestrator",
            "selected_flow_set": "default_delivery",
        "team_flow_authority_id": "team-flow-authority:e06013100fb8761bf279e5687fdc9e71f3be5681c5b0306debe714f9a10222b6",
        "team_flow_config_hash": "ba6d19ea2c63f72585ccd12099f7aa12772c6ece1de79d7259718a04da8ca4da",
        "team_flow_registry_hash": "a7a7e8efa425dfcfa95e404c3cc56c0f4c3bd4a2d3dbe12454a28e840bbdd4aa",
        "team_flow_authority_selected_node_id": "coach_implementation_gate",
        "selected_node_id": "coach_implementation_gate",
        "execution_preparation_required": false,
        "root_session_must_remain_orchestrator": true,
        "resolved_lanes": resolved_lanes,
        "lane_sequence": ["analyst", "test_author", "coach_test_gate", "developer", "coach_implementation_gate", "duplication_reviewer", "tester", "prover", "release_closure"],
        "execution_lane_sequence": ["analyst", "test_author", "coach_test_gate", "developer", "coach_implementation_gate", "duplication_reviewer", "tester", "prover", "release_closure"],
        "lane_catalog": lane_catalog,
        "dispatch_target_index": dispatch_target_index,
        "runtime_role_index": runtime_role_index
    })
}

fn canonical_persisted_team_flow_authority(label: &str) -> serde_json::Value {
    let fixture = PersistentRuntimeFixture::project_bound_with_canonical_sources(
        label,
        &canonical_project_root(),
    );
    fixture.boot();
    let payload = fixture.json_success(&["orchestrator-init", "--full", "--json"]);
    payload
        .pointer("/dev_team_readiness/team_flow_authority")
        .cloned()
        .expect("orchestrator-init should expose persisted TeamFlow authority")
}

fn canonical_host_bridge_carrier_id(runtime_role: &str, task_class: &str) -> String {
    let config = canonical_host_bridge_config();
    let agents = config
        .pointer("/host_environment/codex/agents")
        .and_then(serde_json::Value::as_object)
        .expect("canonical host bridge carrier catalog should exist");
    agents
        .iter()
        .find(|(_, entry)| {
            entry
                .get("runtime_roles")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .any(|role| role.as_str() == Some(runtime_role))
                && entry
                    .get("task_classes")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .any(|class| class.as_str() == Some(task_class))
        })
        .map(|(carrier_id, _)| carrier_id.clone())
        .or_else(|| {
            agents
                .iter()
                .find(|(_, entry)| {
                    entry
                        .get("default_runtime_role")
                        .and_then(serde_json::Value::as_str)
                        == Some(runtime_role)
                })
                .map(|(carrier_id, _)| carrier_id.clone())
        })
        .expect("canonical host bridge carrier should support the fixture lane")
}

fn strict_host_bridge_request(
    mut request: serde_json::Value,
    packet_id: &str,
    attempt_id: &str,
    packet_path: &Path,
    request_path: &Path,
    result_path: &Path,
    receipt_path: &Path,
) -> serde_json::Value {
    let object = request
        .as_object_mut()
        .expect("host bridge request fixture should be an object");
    let run_id = object
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .expect("host bridge request should supply run_id")
        .to_owned();
    let runtime_role = object
        .get("runtime_role")
        .and_then(serde_json::Value::as_str)
        .expect("host bridge request should supply runtime_role")
        .to_owned();
    let task_class = object
        .get("task_class")
        .and_then(serde_json::Value::as_str)
        .expect("host bridge request should supply task_class")
        .to_owned();
    let contract = canonical_host_bridge_adapter_contract();
    let adapter_contract_snapshot = contract.to_value();
    let adapter_contract_hash = blake3::hash(
        &serde_json::to_vec(&adapter_contract_snapshot)
            .expect("adapter contract snapshot should serialize"),
    )
    .to_hex()
    .to_string();
    object.insert("schema_version".to_owned(), serde_json::json!(1));
    object.insert("status".to_owned(), serde_json::json!("pending"));
    object
        .entry("task_id".to_owned())
        .or_insert_with(|| serde_json::json!(run_id));
    object.insert("attempt_id".to_owned(), serde_json::json!(attempt_id));
    object.insert("packet_id".to_owned(), serde_json::json!(packet_id));
    object
        .entry("packet_path".to_owned())
        .or_insert_with(|| serde_json::json!(packet_path.display().to_string()));
    object.insert(
        "backend_id".to_owned(),
        serde_json::json!(canonical_host_bridge_backend_id()),
    );
    object.insert(
        "carrier_id".to_owned(),
        serde_json::json!(canonical_host_bridge_carrier_id(&runtime_role, &task_class)),
    );
    object.insert(
        "execution_boundary".to_owned(),
        serde_json::json!(canonical_host_bridge_execution_boundary()),
    );
    object.insert(
        "dispatch_transport".to_owned(),
        serde_json::json!(contract.dispatch_transport),
    );
    object.insert(
        "receipt_mode".to_owned(),
        serde_json::json!(contract.receipt_mode),
    );
    object.insert(
        "adapter_kind".to_owned(),
        serde_json::json!(contract.adapter_kind),
    );
    object.insert(
        "adapter_capability_id".to_owned(),
        serde_json::json!(contract.adapter_capability_id),
    );
    object.insert(
        "invocation_mode".to_owned(),
        serde_json::json!(contract.invocation_mode),
    );
    object.insert(
        "adapter_operations".to_owned(),
        adapter_contract_snapshot.clone(),
    );
    object.insert(
        "adapter_contract_snapshot".to_owned(),
        adapter_contract_snapshot,
    );
    object.insert(
        "adapter_contract_hash".to_owned(),
        serde_json::json!(adapter_contract_hash),
    );
    object.insert(
        "adapter_contract_source".to_owned(),
        serde_json::json!(canonical_host_bridge_config_path().display().to_string()),
    );
    object
        .entry("request_path".to_owned())
        .or_insert_with(|| serde_json::json!(request_path.display().to_string()));
    object
        .entry("result_path".to_owned())
        .or_insert_with(|| serde_json::json!(result_path.display().to_string()));
    object
        .entry("receipt_path".to_owned())
        .or_insert_with(|| serde_json::json!(receipt_path.display().to_string()));
    request
}

fn project_host_bridge_result_identity(
    mut result: serde_json::Value,
    request: &serde_json::Value,
) -> serde_json::Value {
    const IDENTITY_FIELDS: &[&str] = &[
        "request_id",
        "run_id",
        "task_id",
        "attempt_id",
        "packet_id",
        "packet_path",
        "result_path",
        "receipt_path",
        "dispatch_target",
        "backend_id",
        "carrier_id",
        "adapter_kind",
        "adapter_capability_id",
        "invocation_mode",
        "dispatch_transport",
        "receipt_mode",
        "adapter_contract_source",
        "adapter_contract_snapshot",
        "adapter_contract_hash",
        "adapter_operations",
        "request_path",
    ];
    let object = result
        .as_object_mut()
        .expect("host bridge result fixture should be an object");
    for field in IDENTITY_FIELDS {
        if let Some(value) = request.get(*field) {
            object.insert((*field).to_owned(), value.clone());
        }
    }
    if let Some(packet_path) = request.get("packet_path") {
        object.insert(
            "source_dispatch_packet_path".to_owned(),
            packet_path.clone(),
        );
    }
    result
}

fn unique_lane_state_root(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn run_vida_json_with_state(
    args: &[&str],
    state_root: &std::path::Path,
) -> (serde_json::Value, bool) {
    let mut command = vida();
    command.args(args).env("VIDA_STATE_DIR", state_root);
    if let Some(project_root) = state_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    {
        command.current_dir(project_root);
    }
    let output = command.output().expect("vida command should launch");
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "json output should parse for args {args:?}: {error}\nstatus: {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (payload, output.status.success())
}

struct HostBridgeFixture {
    root: PathBuf,
    state_root: PathBuf,
    request_path: PathBuf,
    result_path: PathBuf,
    blocked_result_path: PathBuf,
    invalid_result_path: PathBuf,
    stale_result_path: PathBuf,
    missing_result_path: PathBuf,
}

fn create_host_bridge_fixture(prefix: &str) -> HostBridgeFixture {
    let root = unique_lane_state_root(prefix);
    let state_root = root.join(".vida/data/state");
    let bridge_dir = state_root.join("runtime-consumption/host-tool-bridge");
    let packet_dir = state_root.join("runtime-consumption/dispatch-packets");
    std::fs::create_dir_all(&bridge_dir).expect("create host bridge dir");
    std::fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = packet_dir.join("analyst.json");
    let request_path = bridge_dir.join("request.json");
    let result_path = bridge_dir.join("designer-pass.json");
    let blocked_result_path = bridge_dir.join("designer-blocked.json");
    let invalid_result_path = bridge_dir.join("invalid-next-lane.json");
    let stale_result_path = bridge_dir.join("stale-result.json");
    let missing_result_path = bridge_dir.join("missing-result.json");
    let request = strict_host_bridge_request(
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-zombie-d-analyst",
            "run_id": "run-zombie-d-analyst",
            "task_id": "run-zombie-d-analyst",
            "dispatch_target": "analyst",
            "allowed_next_node": "designer",
            "packet_path": packet_path,
            "runtime_role": "business_analyst",
            "task_class": "specification",
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": bridge_dir.join("receipt.json"),
            "required_result_fields": [
                "decision",
                "verdict",
                "blocker_codes",
                "rework_target",
                "allowed_next_node"
            ]
        }),
        "packet-zombie-d-analyst",
        "attempt-zombie-d-analyst",
        &packet_path,
        &request_path,
        &result_path,
        &bridge_dir.join("receipt.json"),
    );
    std::fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": request["run_id"],
            "task_id": request["task_id"],
            "attempt_id": request["attempt_id"],
            "packet_id": request["packet_id"],
            "backend_id": request["backend_id"],
            "dispatch_target": request["dispatch_target"]
        })
        .to_string(),
    )
    .expect("write packet");
    std::fs::write(&request_path, request.to_string()).expect("write request");
    HostBridgeFixture {
        root,
        state_root,
        request_path,
        result_path,
        blocked_result_path,
        invalid_result_path,
        stale_result_path,
        missing_result_path,
    }
}

fn run_host_bridge_json(args: &[&str], state_root: &Path) -> (serde_json::Value, bool) {
    run_vida_json_with_state(args, state_root)
}

fn assert_blocker(payload: &serde_json::Value, expected: &str) {
    assert_eq!(payload["status"].as_str(), Some("blocked"));
    assert!(
        payload["blocker_codes"]
            .as_array()
            .expect("blocker_codes should render")
            .iter()
            .any(|code| code.as_str() == Some(expected)),
        "expected blocker {expected}: {payload}"
    );
}

struct HostBridgeReworkFixture {
    root: PathBuf,
    state_root: PathBuf,
    request_path: PathBuf,
    result_path: PathBuf,
    pass_result_path: PathBuf,
}

struct HostBridgeTerminalClosureFixture {
    root: PathBuf,
    state_root: PathBuf,
    request_path: PathBuf,
    result_path: PathBuf,
}

fn create_host_bridge_terminal_closure_fixture(prefix: &str) -> HostBridgeTerminalClosureFixture {
    let root = unique_lane_state_root(prefix);
    let state_root = root.join(".vida/data/state");
    let bridge_dir = state_root.join("runtime-consumption/host-tool-bridge");
    let packet_dir = state_root.join("runtime-consumption/dispatch-packets");
    std::fs::create_dir_all(&bridge_dir).expect("create host bridge dir");
    std::fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = packet_dir.join("gamma-review.json");
    let request_path = bridge_dir.join("gamma-review-request.json");
    let result_path = bridge_dir.join("gamma-review-terminal-closure.json");
    let request = strict_host_bridge_request(
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-gamma-review",
            "run_id": "run-gamma-review",
            "task_id": "run-gamma-review",
            "dispatch_target": "gamma_review",
            "allowed_next_node": "reviewer",
            "runtime_role": "verifier",
            "task_class": "review",
            "packet_path": packet_path,
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": bridge_dir.join("receipt.json"),
            "required_result_fields": [
                "decision",
                "verdict",
                "blocker_codes",
                "allowed_next_node"
            ]
        }),
        "packet-gamma-review",
        "attempt-gamma-review",
        &packet_path,
        &request_path,
        &result_path,
        &bridge_dir.join("receipt.json"),
    );
    std::fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": request["run_id"],
            "task_id": request["task_id"],
            "attempt_id": request["attempt_id"],
            "packet_id": request["packet_id"],
            "backend_id": request["backend_id"],
            "dispatch_target": "gamma_review",
            "role_selection_full": {
                "conversational_mode": null,
                "single_task_only": true,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": ["dev_team_flow_id:default_delivery"],
                "reason": "test fixture",
                "execution_plan": {
                    "development_flow": {
                        "dispatch_contract": {
                            "lane_sequence": [
                                "alpha_build",
                                "beta_verify",
                                "gamma_review"
                            ],
                            "execution_lane_sequence": [
                                "alpha_build",
                                "beta_verify",
                                "gamma_review",
                                "terminal_closure"
                            ],
                            "lane_catalog": {
                                "alpha_build": {
                                    "dispatch_target": "alpha_build",
                                    "task_class": "implementation"
                                },
                                "beta_verify": {
                                    "dispatch_target": "beta_verify",
                                    "task_class": "verification"
                                },
                                "gamma_review": {
                                    "dispatch_target": "gamma_review",
                                    "task_class": "review"
                                }
                            }
                        }
                    }
                }
            }
        })
        .to_string(),
    )
    .expect("write packet");
    std::fs::write(&request_path, request.to_string()).expect("write request");
    std::fs::write(
        &result_path,
        project_host_bridge_result_identity(
            serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "pass",
                "execution_state": "completed",
                "decision": "approve",
                "verdict": "pass",
                "completion_verdict": "pass",
                "blocker_codes": [],
                "rework_target": null,
                "allowed_next_node": "terminal_closure",
                "execution_evidence": {"receipt_backed": true},
                "source_dispatch_packet_path": packet_path
            }),
            &request,
        )
        .to_string(),
    )
    .expect("write terminal closure result");
    HostBridgeTerminalClosureFixture {
        root,
        state_root,
        request_path,
        result_path,
    }
}

fn create_host_bridge_rework_fixture(prefix: &str) -> HostBridgeReworkFixture {
    let root = unique_lane_state_root(prefix);
    let state_root = root.join(".vida/data/state");
    let bridge_dir = state_root.join("runtime-consumption/host-tool-bridge");
    let packet_dir = state_root.join("runtime-consumption/dispatch-packets");
    std::fs::create_dir_all(&bridge_dir).expect("create host bridge dir");
    std::fs::create_dir_all(&packet_dir).expect("create packet dir");
    let packet_path = packet_dir.join("coach.json");
    let request_path = bridge_dir.join("coach-request.json");
    let result_path = bridge_dir.join("coach-rework.json");
    let pass_result_path = bridge_dir.join("coach-pass-invalid-rework.json");
    let request = strict_host_bridge_request(
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-coach-rework",
            "run_id": "run-coach-rework",
            "task_id": "run-coach-rework",
            "dispatch_target": "coach_implementation_gate",
            "allowed_next_node": "tester",
            "runtime_role": "coach",
            "task_class": "coach",
            "packet_path": packet_path,
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": bridge_dir.join("receipt.json"),
            "required_result_fields": [
                "decision",
                "verdict",
                "blocker_codes",
                "rework_target",
                "allowed_next_node"
            ]
        }),
        "packet-coach-rework",
        "attempt-coach-rework",
        &packet_path,
        &request_path,
        &result_path,
        &bridge_dir.join("receipt.json"),
    );
    let persisted_authority =
        canonical_persisted_team_flow_authority(&format!("{prefix}-authority"));
    std::fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": request["run_id"],
            "task_id": request["task_id"],
            "attempt_id": request["attempt_id"],
            "packet_id": request["packet_id"],
            "backend_id": request["backend_id"],
            "dispatch_target": request["dispatch_target"],
            "role_selection_full": {
                "ok": true,
                "activation_source": "test_fixture",
                "selection_mode": "test",
                "request": "coach rework",
                "fallback_role": "orchestrator",
                "selected_role": "coach",
                "conversational_mode": null,
                "single_task_only": true,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": ["dev_team_flow_id:default_delivery"],
                "tracked_flow_entry": "default_delivery",
                "compiled_bundle": {"team_flow_authority": persisted_authority},
                "reason": "test fixture",
                "execution_plan": {
                    "team_flow_authority_selected_flow_id": "default_delivery",
                    "team_flow_authority_selected_node_id": "coach_implementation_gate",
                    "development_flow": {"dispatch_contract": canonical_default_delivery_dispatch_contract()}
                }
            }
        })
        .to_string(),
    )
    .expect("write packet");
    std::fs::write(&request_path, request.to_string()).expect("write request");
    let rework_result = project_host_bridge_result_identity(
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "schema_version": 1,
            "status": "blocked",
            "execution_state": "blocked",
            "decision": "rework_required",
            "verdict": "rework_required",
            "completion_verdict": "rework_required",
            "blocker_codes": ["coach_rework_required"],
            "rework_target": "developer",
            "allowed_next_node": "developer",
            "execution_evidence": {"receipt_backed": true},
            "source_dispatch_packet_path": packet_path
        }),
        &request,
    );
    std::fs::write(&result_path, rework_result.to_string()).expect("write rework result");
    let mut pass_result = rework_result;
    pass_result["status"] = serde_json::json!("pass");
    pass_result["execution_state"] = serde_json::json!("executed");
    pass_result["decision"] = serde_json::json!("pass");
    pass_result["verdict"] = serde_json::json!("pass");
    pass_result["completion_verdict"] = serde_json::json!("pass");
    pass_result["blocker_codes"] = serde_json::json!(Vec::<String>::new());
    std::fs::write(&pass_result_path, pass_result.to_string()).expect("write pass result");
    HostBridgeReworkFixture {
        root,
        state_root,
        request_path,
        result_path,
        pass_result_path,
    }
}

#[test]
fn host_bridge_missing_request_json_parse_error_is_machine_readable() {
    let output = vida()
        .arg("agent")
        .arg("host-bridge")
        .arg("--complete")
        .arg("--decision")
        .arg("pass")
        .arg("--verdict")
        .arg("pass")
        .arg("--allowed-next-node")
        .arg("designer")
        .arg("--json")
        .output()
        .expect("vida agent host-bridge should launch");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "json parse errors should not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse error stdout should be JSON");
    assert_eq!(payload["surface"].as_str(), Some("vida agent host-bridge"));
    assert!(
        payload["blocker_codes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|code| code.as_str() == Some("cli_parse_error"))
    );
    assert!(
        payload["error"]
            .as_str()
            .is_some_and(|error| { error.contains("--request <REQUEST>") })
    );
}

#[test]
fn lane_exception_takeover_json_parse_error_is_machine_readable() {
    let output = vida()
        .arg("lane")
        .arg("exception-takeover")
        .arg("ldr-032")
        .arg("--json")
        .output()
        .expect("vida lane exception-takeover should launch");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "lane json parse errors should not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("lane parse error stdout should be JSON");
    assert_eq!(
        payload["surface"].as_str(),
        Some("vida lane exception-takeover")
    );
    assert!(
        payload["blocker_codes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|code| code.as_str() == Some("lane_parse_error"))
    );
    assert!(
        payload["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("Invalid or incomplete arguments"))
    );
}

#[test]
fn host_bridge_completion_command_resolves_packet_next_target() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "vida-lane-completion-e2e-{}-{nanos}",
        std::process::id()
    ));
    let state_root = root.join(".vida/data/state");
    std::fs::create_dir_all(&root).expect("create test project root");
    let init = vida()
        .arg("init")
        .current_dir(&root)
        .env("VIDA_STATE_DIR", &state_root)
        .output()
        .expect("init should launch");
    assert!(
        init.status.success(),
        "init should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let boot = vida()
        .arg("boot")
        .current_dir(&root)
        .env("VIDA_STATE_DIR", &state_root)
        .output()
        .expect("boot should launch");
    assert!(
        boot.status.success(),
        "boot should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&boot.stdout),
        String::from_utf8_lossy(&boot.stderr)
    );
    let packet_path = state_root.join("runtime-consumption/downstream-dispatch-packets/run.json");
    let request_path = state_root.join("host-tool-bridge/requests/request.json");
    let result_path = state_root.join("host-tool-bridge/results/result.json");
    let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
    std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
        .expect("create packet parent");
    std::fs::create_dir_all(request_path.parent().expect("request parent"))
        .expect("create request parent");
    let request = strict_host_bridge_request(
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-analyst",
            "run_id": "run-analyst",
            "task_id": "run-analyst",
            "dispatch_target": "analyst",
            "allowed_next_node": "closure",
            "packet_path": packet_path,
            "runtime_role": "business_analyst",
            "task_class": "specification",
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": receipt_path
        }),
        "packet-analyst",
        "attempt-analyst",
        &packet_path,
        &request_path,
        &result_path,
        &receipt_path,
    );
    std::fs::write(
        &packet_path,
        serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "run_id": request["run_id"],
            "task_id": request["task_id"],
            "attempt_id": request["attempt_id"],
            "packet_id": request["packet_id"],
            "backend_id": request["backend_id"],
            "dispatch_target": request["dispatch_target"],
            "downstream_dispatch_active_target": "analyst",
            "downstream_dispatch_target": "designer"
        })
        .to_string(),
    )
    .expect("write packet");
    std::fs::write(&request_path, request.to_string()).expect("write request");

    let output = vida()
        .arg("agent")
        .arg("host-bridge")
        .arg("--request")
        .arg(&request_path)
        .arg("--state-dir")
        .arg(&state_root)
        .arg("--json")
        .output()
        .expect("vida agent host-bridge should launch");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge output should be JSON");
    assert!(
        payload["blocker_codes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|code| code.as_str() == Some("host_bridge_dispatch_receipt_missing")),
        "fixture should remain blocked only by missing DB receipt: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let command = payload["host_bridge"]["completion_command"]
        .as_str()
        .expect("completion command should render");
    assert!(
        !command.contains("--allowed-next-node"),
        "completion command must not trust packet/request next-node routing: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn host_bridge_completion_command_does_not_read_packet_outside_state_root() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "vida-lane-completion-boundary-e2e-{}-{nanos}",
        std::process::id()
    ));
    let state_root = root.join(".vida/data/state");
    let outside_packet_path = root.join("outside-packet.json");
    let request_path = state_root.join("host-tool-bridge/requests/request.json");
    let result_path = state_root.join("host-tool-bridge/results/result.json");
    let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
    std::fs::create_dir_all(request_path.parent().expect("request parent"))
        .expect("create request parent");
    let request = strict_host_bridge_request(
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-analyst",
            "run_id": "run-analyst",
            "task_id": "run-analyst",
            "dispatch_target": "analyst",
            "packet_path": outside_packet_path,
            "runtime_role": "business_analyst",
            "task_class": "specification",
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": receipt_path
        }),
        "packet-analyst-outside-root",
        "attempt-analyst-outside-root",
        &outside_packet_path,
        &request_path,
        &result_path,
        &receipt_path,
    );
    std::fs::write(
        &outside_packet_path,
        serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "run_id": request["run_id"],
            "task_id": request["task_id"],
            "attempt_id": request["attempt_id"],
            "packet_id": request["packet_id"],
            "backend_id": request["backend_id"],
            "dispatch_target": request["dispatch_target"],
            "downstream_dispatch_active_target": "analyst",
            "downstream_dispatch_target": "leaked-target"
        })
        .to_string(),
    )
    .expect("write outside packet");
    std::fs::write(&request_path, request.to_string()).expect("write request");

    let output = vida()
        .arg("agent")
        .arg("host-bridge")
        .arg("--request")
        .arg(&request_path)
        .arg("--state-dir")
        .arg(&state_root)
        .arg("--json")
        .output()
        .expect("vida agent host-bridge should launch");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge output should be JSON");
    let command = payload["host_bridge"]["completion_command"]
        .as_str()
        .expect("completion command should render");
    assert!(
        !command.contains("leaked-target"),
        "completion command must not disclose out-of-state-root packet fields: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!command.contains("--allowed-next-node"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn host_bridge_zombie_d_result_validation_matrix_covers_boundary_rows() {
    let fixture = create_host_bridge_fixture("vida-host-bridge-zombie-d-matrix");
    let request = fixture.request_path.to_string_lossy().to_string();
    let result = fixture.result_path.to_string_lossy().to_string();
    let blocked_result = fixture.blocked_result_path.to_string_lossy().to_string();
    let invalid_result = fixture.invalid_result_path.to_string_lossy().to_string();
    let stale_result = fixture.stale_result_path.to_string_lossy().to_string();
    let missing_result = fixture.missing_result_path.to_string_lossy().to_string();

    let (scaffold, scaffold_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--scaffold-result",
            &result,
            "--host-agent-id",
            "host-agent-zombie-d",
            "--receipt-id",
            "receipt-zombie-d",
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(
        scaffold_success,
        "valid analyst->designer scaffold: {scaffold}"
    );
    assert_eq!(scaffold["mode"], "result_scaffold");
    assert_eq!(scaffold["result"]["allowed_next_node"], "designer");
    assert_eq!(
        scaffold["result"]["identity_binding"]["request_id"],
        "req-zombie-d-analyst"
    );

    let (valid, valid_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(valid_success, "valid analyst->designer result: {valid}");
    assert_eq!(valid["validation"]["accepted_completion"], true);
    assert_eq!(valid["validation"]["final_state"], "Passed");

    let mut invalid_next: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture.result_path).expect("read result"))
            .expect("parse scaffolded result");
    invalid_next["allowed_next_node"] = serde_json::json!("invalid-next-lane");
    std::fs::write(&fixture.invalid_result_path, invalid_next.to_string())
        .expect("write invalid next result");
    let (invalid, invalid_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &invalid_result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(!invalid_success, "invalid next lane should fail closed");
    assert_blocker(&invalid, "invalid_allowed_next_node_for_execution_plan");

    let (missing, missing_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &missing_result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(!missing_success, "missing result should fail closed");
    assert_blocker(&missing, "host_bridge_result_unreadable");

    let mut stale: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture.result_path).expect("read result"))
            .expect("parse scaffolded result");
    stale["request_id"] = serde_json::json!("stale-request");
    stale["identity_binding"]["request_id"] = serde_json::json!("stale-request");
    std::fs::write(&fixture.stale_result_path, stale.to_string()).expect("write stale result");
    let (stale_payload, stale_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &stale_result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(!stale_success, "stale result should fail closed");
    assert_blocker(&stale_payload, "host_bridge_result_request_id_mismatch");

    let (blocked, blocked_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--scaffold-result",
            &blocked_result,
            "--blocker-code",
            "designer_review_required",
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(
        blocked_success,
        "blocked result scaffold stays valid: {blocked}"
    );
    assert_eq!(blocked["result"]["verdict"], "blocked");
    assert_eq!(
        blocked["validation"]["validation"]["accepted_completion"],
        false
    );
    assert_eq!(
        blocked["validation"]["validation"]["authority_blocker_codes"],
        serde_json::json!(["designer_review_required"])
    );

    let _ = std::fs::remove_dir_all(&fixture.root);
}

#[test]
fn host_bridge_validate_result_accepts_coach_rework_backedge_to_developer_rework() {
    let fixture = create_host_bridge_rework_fixture("vida-host-bridge-coach-rework-backedge");
    let request = fixture.request_path.to_string_lossy().to_string();
    let result = fixture.result_path.to_string_lossy().to_string();
    let pass_result = fixture.pass_result_path.to_string_lossy().to_string();

    let (rework, rework_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(
        !rework_success,
        "unreceipted coach rework backedge must fail closed: {rework}"
    );
    assert_eq!(rework["status"], "blocked");
    assert_eq!(rework["validation"]["final_state"], "Blocked");
    assert_blocker(&rework, "team_flow_rework_receipt_required");
    assert!(
        !rework["blocker_codes"]
            .as_array()
            .expect("blocker codes should render")
            .iter()
            .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
        "{rework}"
    );

    let (pass, pass_success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &pass_result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(
        !pass_success,
        "pass result must not use rework backedge: {pass}"
    );
    assert_blocker(&pass, "invalid_allowed_next_node_for_execution_plan");

    let _ = std::fs::remove_dir_all(&fixture.root);
}

#[test]
fn host_bridge_validate_result_accepts_terminal_closure_after_final_pass() {
    let fixture =
        create_host_bridge_terminal_closure_fixture("vida-host-bridge-terminal-closure-final-pass");
    let request = fixture.request_path.to_string_lossy().to_string();
    let result = fixture.result_path.to_string_lossy().to_string();

    let (payload, success) = run_host_bridge_json(
        &[
            "agent",
            "host-bridge",
            "--request",
            &request,
            "--validate-result",
            &result,
            "--json",
        ],
        &fixture.state_root,
    );
    assert!(
        success,
        "terminal closure pass result should validate without invalid next blocker: {payload}"
    );
    assert_eq!(payload["status"], "pass");
    assert_eq!(payload["validation"]["final_state"], "Passed");
    assert!(
        !payload["blocker_codes"]
            .as_array()
            .expect("blocker codes should render")
            .iter()
            .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
        "{payload}"
    );

    let _ = std::fs::remove_dir_all(&fixture.root);
}

#[test]
fn lane_public_surface_matrix_fails_closed_with_json_contracts() {
    let root = unique_lane_state_root("vida-lane-surface-matrix");
    let state_root = root.join(".vida/data/state");
    std::fs::create_dir_all(&root).expect("lane matrix root should exist");
    let init = vida()
        .arg("init")
        .current_dir(&root)
        .env("VIDA_STATE_DIR", &state_root)
        .output()
        .expect("init should launch");
    assert!(
        init.status.success(),
        "init should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let boot = vida()
        .arg("boot")
        .current_dir(&root)
        .env("VIDA_STATE_DIR", &state_root)
        .output()
        .expect("boot should launch");
    assert!(
        boot.status.success(),
        "boot should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&boot.stdout),
        String::from_utf8_lossy(&boot.stderr)
    );

    for (label, args, expected_surface, expected_blocker) in [
        (
            "run_lane root",
            vec!["lane", "--json"],
            "vida lane",
            "unsupported_blocker_code",
        ),
        (
            "run_lane show missing run",
            vec!["lane", "show", "matrix-missing-run", "--json"],
            "vida lane show",
            "missing_lane_receipt",
        ),
        (
            "run_lane takeover-ready missing run",
            vec!["lane", "takeover-ready", "matrix-missing-run", "--json"],
            "vida lane takeover-ready",
            "missing_lane_receipt",
        ),
        (
            "run_lane complete missing run with result file",
            vec![
                "lane",
                "complete",
                "matrix-missing-run",
                "--receipt-id",
                "receipt-1",
                "--host-bridge-result-file",
                "missing.json",
                "--json",
            ],
            "vida lane complete",
            "missing_lane_receipt",
        ),
    ] {
        let (payload, success) = run_vida_json_with_state(&args, &state_root);
        assert!(!success, "{label} should fail closed: {payload}");
        assert_eq!(
            payload["surface"].as_str(),
            Some(expected_surface),
            "{label}"
        );
        assert_eq!(payload["status"].as_str(), Some("blocked"), "{label}");
        assert!(
            payload["blocker_codes"]
                .as_array()
                .expect("blocker_codes should be an array")
                .iter()
                .any(|code| code.as_str() == Some(expected_blocker)),
            "{label} should expose {expected_blocker}: {payload}"
        );
    }

    let _ = std::fs::remove_dir_all(root);
}

fn team_flow_authority_signature(payload: &serde_json::Value) -> serde_json::Value {
    let readiness = payload
        .get("dev_team_readiness")
        .expect("orchestrator init should expose dev_team_readiness");
    let authority = readiness
        .get("team_flow_authority")
        .filter(|value| value.is_object())
        .expect("dev_team_readiness should expose persisted TeamFlow authority");
    let selected_flow = readiness
        .get("default_flow_id")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            authority
                .pointer("/selected_config/authority_selection/default_flow_id")
                .and_then(serde_json::Value::as_str)
        })
        .filter(|value| !value.is_empty())
        .expect("TeamFlow authority should expose selected flow");
    let selected_flow_lanes = authority
        .pointer("/resolved_all_flow_payload/flows")
        .and_then(serde_json::Value::as_array)
        .and_then(|flows| {
            flows.iter().find(|flow| {
                flow.get("flow_id").and_then(serde_json::Value::as_str) == Some(selected_flow)
            })
        })
        .and_then(|flow| flow.get("lanes"));
    let sequence_source = selected_flow_lanes
        .filter(|lanes| !sequence_node_ids(lanes).is_empty())
        .or_else(|| readiness.get("sequence"));
    let sequence = sequence_source
        .map(sequence_node_ids)
        .filter(|sequence| !sequence.is_empty())
        .expect("selected TeamFlow authority should expose a non-empty sequence");
    let execution_sequence = sequence_source
        .map(execution_sequence_node_ids)
        .filter(|sequence| !sequence.is_empty())
        .expect("selected TeamFlow authority should expose a non-empty execution sequence");
    let excluded_sequence = sequence_source
        .map(excluded_sequence_node_ids)
        .unwrap_or_default();
    serde_json::json!({
        "authority_id": authority["authority_id"],
        "content_blake3": authority["content_blake3"],
        "resolved_all_flow_payload_blake3": authority["resolved_all_flow_payload_blake3"],
        "config_id": authority.pointer("/selected_config/config_id"),
        "profile": authority.pointer("/selected_config/profile"),
        "registry_hash": authority.pointer("/selected_config/registry_hash"),
        "selected_flow": selected_flow,
        "sequence": sequence,
        "execution_sequence": execution_sequence,
        "excluded_sequence": excluded_sequence
    })
}

fn sequence_node_ids(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|entries| entries.iter().filter_map(sequence_node_id).collect())
        .unwrap_or_default()
}

fn execution_sequence_node_ids(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry_included(entry))
                .filter_map(sequence_node_id)
                .collect()
        })
        .unwrap_or_default()
}

fn excluded_sequence_node_ids(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| !entry_included(entry))
                .filter_map(sequence_node_id)
                .collect()
        })
        .unwrap_or_default()
}

fn sequence_node_id(entry: &serde_json::Value) -> Option<String> {
    entry
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| {
            entry
                .get("node_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            entry
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn entry_included(entry: &serde_json::Value) -> bool {
    entry
        .get("included")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

fn dispatch_contract(value: &serde_json::Value) -> Option<&serde_json::Value> {
    if value
        .get("team_flow_authority_id")
        .and_then(serde_json::Value::as_str)
        .is_some()
        && value.get("lane_sequence").is_some()
    {
        return Some(value);
    }
    match value {
        serde_json::Value::Object(map) => map.values().find_map(dispatch_contract),
        serde_json::Value::Array(values) => values.iter().find_map(dispatch_contract),
        _ => None,
    }
}

fn dispatch_authority_signature(packet: &serde_json::Value) -> serde_json::Value {
    let contract = dispatch_contract(packet)
        .expect("dispatch packet should carry a TeamFlow execution contract");
    serde_json::json!({
        "authority_id": contract["team_flow_authority_id"],
        "selected_flow": contract["selected_flow_set"],
        "lane_sequence": sequence_node_ids(&contract["lane_sequence"]),
        "execution_lane_sequence": sequence_node_ids(&contract["execution_lane_sequence"])
    })
}

fn first_string_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(serde_json::Value::as_str) {
                    if !found.is_empty() {
                        return Some(found.to_owned());
                    }
                }
            }
            map.values()
                .find_map(|entry| first_string_field(entry, keys))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|entry| first_string_field(entry, keys)),
        _ => None,
    }
}

fn collect_blocker_codes(value: &serde_json::Value, output: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, entry) in map {
                if key == "blocker_codes" || key == "blockers" {
                    match entry {
                        serde_json::Value::Array(values) => {
                            for value in values {
                                if let Some(code) = value.as_str() {
                                    output.push(code.to_owned());
                                } else if let Some(code) =
                                    value.get("code").and_then(serde_json::Value::as_str)
                                {
                                    output.push(code.to_owned());
                                }
                            }
                        }
                        serde_json::Value::String(code) => output.push(code.clone()),
                        _ => {}
                    }
                }
                collect_blocker_codes(entry, output);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_blocker_codes(value, output);
            }
        }
        _ => {}
    }
}

fn assert_replay_status_matches(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    context: &str,
) {
    for field in [
        "run_id",
        "task_id",
        "task_class",
        "active_node",
        "next_node",
        "status",
        "route_task_class",
        "selected_backend",
        "lane_id",
        "lifecycle_stage",
        "policy_gate",
        "handoff_state",
        "context_state",
        "checkpoint_kind",
        "resume_target",
        "recovery_ready",
    ] {
        assert_eq!(
            actual["run_graph_status"][field], expected["run_graph_status"][field],
            "{context} should preserve run-graph field {field}"
        );
    }
}

fn canonical_project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("vida crate should have a canonical worktree root")
        .to_path_buf()
}

#[test]
#[ignore = "invoked as a short-lived ZOMBIE-D tamper subprocess"]
fn zombie_d_apply_launcher_snapshot_tamper_from_env() {
    let Ok(state_dir) = std::env::var(ZOMBIE_D_TAMPER_STATE_DIR_ENV) else {
        return;
    };
    let tamper_kind = std::env::var(ZOMBIE_D_TAMPER_KIND_ENV)
        .map(|value| ZombieDTamperKind::from_env(&value))
        .expect("ZOMBIE-D tamper kind should be set with tamper state dir");
    let state_dir = PathBuf::from(state_dir);
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(state_dir)
            .await
            .expect("launcher snapshot state db should open for tamper probe");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("launcher snapshot state namespace should open for tamper probe");
        let mut row: serde_json::Value = db
            .select(("launcher_activation_snapshot", "launcher_live"))
            .await
            .expect("launcher activation snapshot should be readable for tamper probe")
            .expect("launcher activation snapshot should exist for tamper probe");
        if let Some(object) = row.as_object_mut() {
            object.remove("id");
        }
        tamper_kind.apply(&mut row);
        let _: Option<serde_json::Value> = db
            .update(("launcher_activation_snapshot", "launcher_live"))
            .content(row)
            .await
            .expect("tampered launcher activation snapshot should persist");
        drop(db);
    });
    runtime.shutdown_timeout(std::time::Duration::from_millis(250));
}

fn tampered_authority_probe(
    label: &str,
    tamper_kind: ZombieDTamperKind,
    expected_code_fragment: &str,
) -> TamperedAuthorityOutcome {
    let fixture = PersistentRuntimeFixture::project_bound_with_canonical_sources(
        label,
        &canonical_project_root(),
    );
    fixture.boot();
    let baseline = fixture.json_success(&["orchestrator-init", "--full", "--json"]);
    let baseline_signature = team_flow_authority_signature(&baseline);
    let helper_output = Command::new(
        std::env::current_exe().expect("current integration test executable should resolve"),
    )
    .args([
        "--exact",
        ZOMBIE_D_TAMPER_HELPER_TEST,
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ])
    .env(ZOMBIE_D_TAMPER_STATE_DIR_ENV, fixture.state_dir())
    .env(ZOMBIE_D_TAMPER_KIND_ENV, tamper_kind.as_env())
    .output()
    .expect("ZOMBIE-D tamper helper subprocess should launch");
    assert!(
        helper_output.status.success(),
        "ZOMBIE-D tamper helper should exit cleanly: stdout={} stderr={}",
        String::from_utf8_lossy(&helper_output.stdout),
        String::from_utf8_lossy(&helper_output.stderr)
    );
    let (payload, success) = fixture.json_allow_failure(&["orchestrator-init", "--full", "--json"]);
    if success {
        let after_signature = team_flow_authority_signature(&payload);
        assert_eq!(
            after_signature, baseline_signature,
            "successful tamper recovery must restore the canonical TeamFlow authority"
        );
        let payload_text = payload.to_string();
        assert!(
            !payload_text.contains(tamper_kind.tampered_marker()),
            "successful tamper recovery must not expose the tampered marker: {payload}"
        );
        let reread = fixture.json_success(&["orchestrator-init", "--full", "--json"]);
        assert_eq!(
            team_flow_authority_signature(&reread),
            baseline_signature,
            "canonical TeamFlow authority must remain restored on a second public read"
        );
        assert!(
            !reread.to_string().contains(tamper_kind.tampered_marker()),
            "second public read must not expose the tampered marker: {reread}"
        );
        return TamperedAuthorityOutcome::Canonicalized;
    }
    let mut blocker_codes = Vec::new();
    collect_blocker_codes(&payload, &mut blocker_codes);
    let nested_blocked = payload
        .pointer("/dev_team_readiness/status")
        .and_then(serde_json::Value::as_str)
        == Some("blocked")
        || payload
            .pointer("/dev_team_readiness/team_flow_authority_status")
            .and_then(serde_json::Value::as_str)
            == Some("blocked");
    assert!(
        nested_blocked,
        "tampered TeamFlow authority must fail closed: {payload}"
    );
    assert!(
        blocker_codes
            .iter()
            .any(|code| code.contains(expected_code_fragment)),
        "tampered TeamFlow authority should expose {expected_code_fragment}: {payload}"
    );
    TamperedAuthorityOutcome::Blocked
}

#[test]
fn zombie_d_team_flow_authority_survives_restart_replay_and_tamper_matrix() {
    let fixture = PersistentRuntimeFixture::project_bound_with_canonical_sources(
        "zombie-d-team-flow-authority-replay",
        &canonical_project_root(),
    );
    fixture.boot();
    let (baseline, baseline_success) =
        fixture.json_allow_failure(&["orchestrator-init", "--full", "--json"]);
    assert!(
        baseline_success,
        "production blocker: durable TeamFlow authority is not reachable through public fixture: {baseline}"
    );
    let authority_signature = team_flow_authority_signature(&baseline);
    let run_id = format!(
        "zombie-d-team-flow-replay-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    );
    fixture.create_authority_bound_run_graph_task(&run_id);
    let seed = fixture.json_success(&[
        "taskflow",
        "run-graph",
        "seed",
        &run_id,
        "continue development",
        "--json",
    ]);
    assert_eq!(
        seed.pointer("/payload/status/run_id")
            .and_then(serde_json::Value::as_str),
        Some(run_id.as_str())
    );
    let advance = fixture.json_success(&["taskflow", "run-graph", "advance", &run_id, "--json"]);
    assert!(
        advance
            .pointer("/payload/status/next_node")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "run graph should select a canonical next node: {advance}"
    );
    let first_status =
        fixture.json_success(&["taskflow", "run-graph", "status", &run_id, "--json"]);
    assert_eq!(
        first_status["run_graph_status"]["next_node"],
        advance["payload"]["status"]["next_node"]
    );
    let dispatch_init =
        fixture.json_allow_failure(&["taskflow", "run-graph", "dispatch-init", &run_id, "--json"]);
    assert!(
        dispatch_init.1,
        "production blocker: persisted TeamFlow dispatch/receipt state is not reachable: {}",
        dispatch_init.0
    );
    let post_dispatch_status =
        fixture.json_success(&["taskflow", "run-graph", "status", &run_id, "--json"]);
    let packet_path = dispatch_init
        .0
        .get("dispatch_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .expect("dispatch-init should expose a persisted dispatch packet path");
    let packet_path = if packet_path.is_absolute() {
        packet_path
    } else {
        fixture.state_dir().join(packet_path)
    };
    let packet: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&packet_path)
            .expect("persisted dispatch packet should be readable"),
    )
    .expect("persisted dispatch packet should be valid json");
    let packet_signature = dispatch_authority_signature(&packet);
    assert_eq!(
        packet_signature["authority_id"],
        authority_signature["authority_id"]
    );
    assert_eq!(
        packet_signature["selected_flow"],
        authority_signature["selected_flow"]
    );
    assert_eq!(
        packet_signature["lane_sequence"],
        authority_signature["execution_sequence"]
    );
    assert_eq!(
        packet_signature["execution_lane_sequence"],
        authority_signature["execution_sequence"]
    );
    let full_sequence = authority_signature["sequence"]
        .as_array()
        .expect("authority signature should expose full sequence");
    let execution_sequence = authority_signature["execution_sequence"]
        .as_array()
        .expect("authority signature should expose execution sequence");
    assert!(!execution_sequence.is_empty());
    let packet_execution_sequence = packet_signature["execution_lane_sequence"]
        .as_array()
        .expect("packet signature should expose execution sequence");
    let mut full_cursor = 0;
    for execution_node in execution_sequence {
        let relative_position = full_sequence[full_cursor..]
            .iter()
            .position(|node| node == execution_node)
            .expect("execution sequence should preserve full sequence order");
        full_cursor += relative_position + 1;
    }
    for excluded_node in authority_signature["excluded_sequence"]
        .as_array()
        .expect("authority signature should expose excluded sequence")
    {
        assert!(
            !packet_execution_sequence
                .iter()
                .any(|node| node == excluded_node),
            "excluded TeamFlow node must not be an execution target: {excluded_node}"
        );
    }
    let carrier_relation = first_string_field(&packet, &["carrier_relation"]);
    let backend_relation = first_string_field(&packet, &["executor_backend_relation"]);
    assert_eq!(
        carrier_relation.is_some(),
        backend_relation.is_some(),
        "carrier/backend authority relations must be jointly observable when exposed"
    );
    let carrier_backend_status = if carrier_relation.is_some() {
        "pass"
    } else {
        "na"
    };

    let replayed_authority = fixture.json_success(&["orchestrator-init", "--full", "--json"]);
    assert_eq!(
        team_flow_authority_signature(&replayed_authority),
        authority_signature,
        "restart must preserve TeamFlow authority identity, hashes, selected flow, and sequence"
    );
    let replayed_status =
        fixture.json_success(&["taskflow", "run-graph", "status", &run_id, "--json"]);
    assert_replay_status_matches(
        &post_dispatch_status,
        &replayed_status,
        "replayed run graph",
    );
    let replayed_dispatch =
        fixture.json_allow_failure(&["taskflow", "run-graph", "dispatch-init", &run_id, "--json"]);
    assert!(
        replayed_dispatch.1 || replayed_dispatch.0["status"] == "blocked",
        "same run/receipt replay must return a canonical verdict: {}",
        replayed_dispatch.0
    );
    if replayed_dispatch.1 {
        let replay_packet_path = replayed_dispatch
            .0
            .get("dispatch_packet_path")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
            .expect("replayed dispatch should expose packet path");
        let replay_packet_path = if replay_packet_path.is_absolute() {
            replay_packet_path
        } else {
            fixture.state_dir().join(replay_packet_path)
        };
        let replay_packet: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(replay_packet_path)
                .expect("replayed packet should be readable"),
        )
        .expect("replayed packet should be valid json");
        assert_eq!(
            dispatch_authority_signature(&replay_packet),
            packet_signature
        );
    }
    let packet_path_arg = packet_path.to_string_lossy().to_string();
    let stale_receipt = fixture.json_allow_failure(&[
        "lane",
        "complete",
        &run_id,
        "--receipt-id",
        "zombie-d-stale-receipt",
        "--host-bridge-result-file",
        &packet_path_arg,
        "--json",
    ]);
    assert!(!stale_receipt.1 && stale_receipt.0["status"] == "blocked");
    let stale_status =
        fixture.json_success(&["taskflow", "run-graph", "status", &run_id, "--json"]);
    assert_replay_status_matches(
        &post_dispatch_status,
        &stale_status,
        "stale-receipt run graph",
    );

    let tamper_outcomes = [
        tampered_authority_probe(
            "zombie-d-team-flow-authority-identity-tamper",
            ZombieDTamperKind::AuthorityId,
            "team_flow_authority",
        ),
        tampered_authority_probe(
            "zombie-d-team-flow-authority-hash-tamper",
            ZombieDTamperKind::ContentHash,
            "team_flow_authority",
        ),
        tampered_authority_probe(
            "zombie-d-team-flow-authority-alias-tamper",
            ZombieDTamperKind::DefaultFlowAlias,
            "team_flow_authority",
        ),
    ];
    let tamper_outcome_labels = tamper_outcomes
        .iter()
        .map(|outcome| outcome.as_str())
        .collect::<Vec<_>>();

    let matrix = serde_json::json!({
        "Z": {"status":"pass", "evidence_refs":["restart/replay", "stale receipt"]},
        "O": {"status":"pass", "evidence_refs":["run-graph state"]},
        "M": {"status":"pass", "evidence_refs":["authority signature + packet contract"]},
        "B": {"status":carrier_backend_status, "evidence_refs":["carrier_relation + executor_backend_relation"], "reason":"fixture field availability"},
        "I": {"status":"pass", "evidence_refs":["json CLI surfaces"]},
        "E": {"status":"pass", "evidence_refs":["identity/hash/alias tamper blockers or canonical recovery"], "outcomes":tamper_outcome_labels},
        "S": {"status":"pass", "evidence_refs":["persisted snapshot replay"]},
        "R": {"status":"pass", "evidence_refs":["same run/receipt replay"]},
        "P": {"status":"pass", "evidence_refs":["launcher_activation_snapshot"]},
        "C": {"status":"pass", "evidence_refs":["orchestrator-init ↔ dispatch packet ↔ run graph"]}
    });
    for category in ["Z", "O", "M", "B", "I", "E", "S", "R", "P", "C"] {
        assert!(
            matrix[category]["status"].is_string(),
            "ZOMBIE-D category {category} must be recorded"
        );
    }
    assert!(matches!(
        matrix["B"]["status"].as_str(),
        Some("pass") | Some("na")
    ));
}
