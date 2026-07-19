use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use taskflow_host_bridge::HostBridgeAdapterOperations;

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
                    entry.get("default_runtime_role").and_then(serde_json::Value::as_str)
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
    object.insert("receipt_mode".to_owned(), serde_json::json!(contract.receipt_mode));
    object.insert("adapter_kind".to_owned(), serde_json::json!(contract.adapter_kind));
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
    std::fs::write(
        &request_path,
        request.to_string(),
    )
    .expect("write request");
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
    std::fs::write(
        &request_path,
        request.to_string(),
    )
    .expect("write request");
    std::fs::write(
        &result_path,
        project_host_bridge_result_identity(serde_json::json!({
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
        }), &request)
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
                "execution_plan": {
                    "development_flow": {
                        "dispatch_contract": {
                            "execution_lane_sequence": [
                                "developer",
                                "coach_implementation_gate",
                                "tester"
                            ],
                            "lane_catalog": {
                                "developer": {
                                    "dispatch_target": "developer",
                                    "task_class": "implementation"
                                },
                                "coach_implementation_gate": {
                                    "dispatch_target": "coach_implementation_gate",
                                    "task_class": "coach"
                                },
                                "tester": {
                                    "dispatch_target": "tester",
                                    "task_class": "verification"
                                },
                                "developer_rework": {
                                    "dispatch_target": "developer_rework",
                                    "task_class": "implementation"
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
    std::fs::write(
        &request_path,
        request.to_string(),
    )
    .expect("write request");
    let rework_result = project_host_bridge_result_identity(serde_json::json!({
        "artifact_kind": "host_tool_bridge_result",
        "schema_version": 1,
        "status": "blocked",
        "execution_state": "blocked",
        "decision": "rework_required",
        "verdict": "rework_required",
        "completion_verdict": "rework_required",
        "blocker_codes": ["coach_rework_required"],
        "rework_target": "developer",
        "allowed_next_node": "developer_rework",
        "execution_evidence": {"receipt_backed": true},
        "source_dispatch_packet_path": packet_path
    }), &request);
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
    assert!(payload["blocker_codes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|code| code.as_str() == Some("cli_parse_error")));
    assert!(payload["error"]
        .as_str()
        .is_some_and(|error| { error.contains("--request <REQUEST>") }));
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
    assert!(payload["blocker_codes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|code| code.as_str() == Some("lane_parse_error")));
    assert!(payload["reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("Invalid or incomplete arguments")));
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
    std::fs::write(
        &request_path,
        request.to_string(),
    )
    .expect("write request");

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
    std::fs::write(
        &request_path,
        request.to_string(),
    )
    .expect("write request");

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
        rework_success,
        "coach rework backedge should validate without invalid next blocker: {rework}"
    );
    assert_eq!(rework["status"], "pass");
    assert_eq!(rework["validation"]["final_state"], "Blocked");
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
