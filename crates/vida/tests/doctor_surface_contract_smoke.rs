use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!(
        "/tmp/vida-doctor-contract-state-{}-{nanos}",
        std::process::id()
    )
}

fn is_canonical_operator_status(value: &str) -> bool {
    matches!(value, "pass" | "blocked")
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    "unsupported_architecture_reserved_workflow_boundary";
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str =
    "Clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.";

fn sync_protocol_binding(state_dir: &str) {
    let output = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        output.status.success(),
        "protocol-binding sync should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_final_snapshot(state_dir: &str, file_name: &str, snapshot: serde_json::Value) {
    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    std::fs::create_dir_all(&runtime_consumption_dir)
        .expect("runtime-consumption directory should be created");
    std::fs::write(
        format!("{runtime_consumption_dir}/{file_name}"),
        snapshot.to_string(),
    )
    .expect("final runtime-consumption snapshot should be written");
}

fn init_run_graph_with_architecture_reserved_gate(state_dir: &str) {
    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            "vida-a",
            "writer",
            "analysis",
        ])
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("taskflow run-graph init should run");
    assert!(init.status.success());

    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-a",
            "writer",
            "writer",
            "ready",
            "analysis",
            "{\"next_node\":\"coach\",\"selected_backend\":\"codex\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"architecture_reserved\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());
}

fn assert_fixture_has_doctor_run_graph_negative_control_step() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/golden/taskflow/critical_path.json"
    ))
    .expect("critical-path fixture should parse");
    let steps = fixture["release_1_contract_steps"]
        .as_array()
        .expect("release_1_contract_steps should be array");
    let step = steps
        .iter()
        .find(|entry| entry["id"] == "doctor_run_graph_negative_control")
        .expect("doctor run-graph negative-control step should exist");
    assert_eq!(step["mode"], "fail_closed");
    assert_eq!(
        step["blocker_code"],
        MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER
    );
    assert_eq!(
        step["next_action"],
        MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION
    );
}

#[test]
fn doctor_json_emits_operator_contract_fields() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());

    let stdout = String::from_utf8_lossy(&doctor.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor json should parse");

    assert_eq!(parsed["surface"], "vida doctor");
    assert!(parsed["status"].is_string());
    assert!(parsed["status"] == "pass" || parsed["status"] == "blocked");
    assert!(parsed["blocker_codes"].is_array());
    assert!(parsed["next_actions"].is_array());
    assert!(parsed["artifact_refs"].is_object());
    assert_eq!(
        parsed["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
    );
    assert_eq!(parsed["status"], parsed["operator_contracts"]["status"]);
    assert_eq!(
        parsed["blocker_codes"],
        parsed["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        parsed["next_actions"],
        parsed["operator_contracts"]["next_actions"]
    );
    assert_eq!(
        parsed["artifact_refs"],
        parsed["operator_contracts"]["artifact_refs"]
    );
    assert!(parsed["shared_fields"].is_object());
    assert_eq!(parsed["status"], parsed["shared_fields"]["status"]);
    assert_eq!(
        parsed["blocker_codes"],
        parsed["shared_fields"]["blocker_codes"]
    );
    assert_eq!(
        parsed["next_actions"],
        parsed["shared_fields"]["next_actions"]
    );
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be array");
    let next_actions = parsed["next_actions"]
        .as_array()
        .expect("next_actions should be array");
    let has_retrieval_trust_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("missing_retrieval_trust_operator_evidence"));
    let has_retrieval_trust_signal_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("missing_retrieval_trust_signal_operator_evidence"));
    let has_retrieval_trust_next_action = next_actions.iter().any(|action| {
        action.as_str()
            == Some(
                "Run `vida taskflow consume bundle check --json` to record retrieval-trust operator evidence.",
            )
    });
    let has_retrieval_trust_signal_next_action = next_actions.iter().any(|action| {
        action.as_str()
            == Some(
                "Run `vida taskflow protocol-binding sync --json` and `vida taskflow consume bundle check --json` to materialize retrieval-trust citation/freshness/ACL signal.",
            )
    });
    let has_retrieval_trust_source_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("missing_retrieval_trust_source_operator_evidence"));
    let has_retrieval_trust_source_next_action = next_actions.iter().any(|action| {
        action.as_str()
            == Some(
                "Run `vida taskflow consume bundle check --json` so runtime consumption snapshots publish retrieval-trust source evidence.",
            )
    });
    let has_recovery_readiness_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("recovery_readiness_blocked"));
    let has_recovery_readiness_next_action = next_actions.iter().any(|action| {
        action.as_str()
            == Some(
                "Run `vida taskflow run-graph recover --json` and confirm `recovery_ready=true` before resume/rollback handoff.",
            )
    });
    let has_unsupported_architecture_reserved_boundary_blocker = blocker_codes.iter().any(|code| {
        code.as_str() == Some(UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER)
    });
    let has_unsupported_architecture_reserved_boundary_next_action =
        next_actions.iter().any(|action| {
            action.as_str() == Some(UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION)
        });
    let has_missing_dispatch_receipt_blocker = blocker_codes.iter().any(|code| {
        code.as_str() == Some(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER)
    });
    let has_missing_dispatch_receipt_next_action = next_actions.iter().any(|action| {
        action.as_str() == Some(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION)
    });
    assert_eq!(
        has_retrieval_trust_blocker, has_retrieval_trust_next_action,
        "retrieval-trust blocker and next_action must stay in parity"
    );
    assert_eq!(
        has_retrieval_trust_signal_blocker, has_retrieval_trust_signal_next_action,
        "retrieval-trust signal blocker and next_action must stay in parity"
    );
    assert_eq!(
        has_retrieval_trust_source_blocker, has_retrieval_trust_source_next_action,
        "retrieval-trust source blocker and next_action must stay in parity"
    );
    assert_eq!(
        has_recovery_readiness_blocker, has_recovery_readiness_next_action,
        "recovery readiness blocker and next_action must stay in parity"
    );
    assert_eq!(
        has_unsupported_architecture_reserved_boundary_blocker,
        has_unsupported_architecture_reserved_boundary_next_action,
        "unsupported/architecture-reserved workflow boundary blocker and next_action must stay in parity"
    );
    assert_eq!(
        has_missing_dispatch_receipt_blocker, has_missing_dispatch_receipt_next_action,
        "missing dispatch receipt blocker and next_action must stay in parity"
    );
    assert!(
        !has_unsupported_architecture_reserved_boundary_blocker,
        "negative-control: unsupported/architecture-reserved workflow boundary blocker must stay absent without run-graph gate evidence"
    );
    assert!(
        !has_missing_dispatch_receipt_blocker,
        "negative-control: missing dispatch receipt blocker must stay absent without run-graph gate evidence"
    );

    let artifact_refs = parsed["artifact_refs"]
        .as_object()
        .expect("artifact_refs should be object");
    assert!(artifact_refs.contains_key("runtime_consumption_latest_snapshot_path"));
    assert!(artifact_refs.contains_key("latest_run_graph_dispatch_receipt_id"));
    assert!(artifact_refs.contains_key("protocol_binding_latest_receipt_id"));
    assert!(artifact_refs.contains_key("retrieval_trust_signal"));
    assert!(artifact_refs.contains_key("latest_task_reconciliation_receipt_id"));
    assert!(artifact_refs.contains_key("effective_instruction_bundle_receipt_id"));
}

#[test]
fn taskflow_consume_continue_fails_closed_without_execution_preparation_contract() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let continue_cmd = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow consume continue should run");
    assert!(
        !continue_cmd.status.success(),
        "continue should fail-closed without execution-preparation contract/evidence"
    );
    let stderr = String::from_utf8_lossy(&continue_cmd.stderr);
    assert!(
        stderr.contains("execution_preparation_gate_blocked"),
        "stderr should mention execution-preparation gate blocker, got: {stderr}"
    );
}

#[test]
fn taskflow_consume_continue_fails_closed_when_operator_contract_status_is_blocked() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    write_final_snapshot(
        &state_dir,
        "final-operator-contract-blocked.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "blocked",
                "blocker_codes": ["pending_execution_preparation_evidence"],
                "next_actions": [],
                "artifact_refs": {},
            },
            "payload": {
                "closure_admission": {
                    "status": "admit",
                    "blockers": [],
                }
            },
            "dispatch_receipt": {}
        }),
    );

    let continue_cmd = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow consume continue should run");
    assert!(
        !continue_cmd.status.success(),
        "continue should fail-closed when release-1 operator contract status is not admitted"
    );
    let stderr = String::from_utf8_lossy(&continue_cmd.stderr);
    assert!(
        stderr.contains("execution_preparation_gate_blocked"),
        "stderr should mention operator-contract status gate blocker, got: {stderr}"
    );
}

#[test]
fn doctor_and_protocol_binding_share_canonical_status() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let pb = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding check should run");
    assert!(!pb.status.success());
    let pb_json: serde_json::Value =
        serde_json::from_slice(&pb.stdout).expect("protocol-binding json should parse");
    let pb_status = pb_json["status"]
        .as_str()
        .expect("protocol-binding status should be string");
    assert!(pb_status == "pass" || pb_status == "blocked");

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    let doctor_protocol_binding = &doctor_json["protocol_binding"];
    assert_eq!(doctor_json["operator_contracts"]["status"], pb_status);
    assert_eq!(
        doctor_json["operator_contracts"]["status"],
        pb_json["operator_contracts"]["status"]
    );
    let doctor_blockers = doctor_json["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("doctor blocker codes should be array");
    let pb_blockers = pb_json["blocker_codes"]
        .as_array()
        .expect("protocol-binding blocker codes should be array");
    if pb_status == "blocked" {
        assert!(
            !doctor_blockers.is_empty(),
            "doctor blocked status should include blocker evidence"
        );
        assert!(
            !pb_blockers.is_empty(),
            "protocol-binding blocked status should include blocker evidence"
        );
    }
    assert!(
        doctor_protocol_binding["blocking_issue_count"].is_number(),
        "doctor protocol_binding rollup should still be present"
    );
}

#[test]
fn doctor_json_blocks_on_unsupported_architecture_reserved_boundary_contract() {
    assert_fixture_has_doctor_run_graph_negative_control_step();

    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    sync_protocol_binding(&state_dir);
    init_run_graph_with_architecture_reserved_gate(&state_dir);

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());

    let stdout = String::from_utf8_lossy(&doctor.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor json should parse");
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be array");
    let next_actions = parsed["next_actions"]
        .as_array()
        .expect("next_actions should be array");
    assert!(
        blocker_codes.iter().any(|code| {
            code.as_str() == Some(UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER)
        }),
        "doctor must fail-closed with unsupported architecture-reserved workflow boundary blocker"
    );
    assert!(
        next_actions.iter().any(|action| {
            action.as_str()
                == Some(UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION)
        }),
        "doctor must publish remediation action for unsupported architecture-reserved workflow boundary blocker"
    );
    assert!(
        blocker_codes.iter().any(|code| {
            code.as_str() == Some(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER)
        }),
        "doctor must fail-closed when run-graph gate exists without dispatch receipt evidence"
    );
    assert!(
        next_actions.iter().any(|action| {
            action.as_str()
                == Some(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION)
        }),
        "doctor must publish remediation action for missing run-graph dispatch receipt evidence"
    );
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(parsed["status"], parsed["operator_contracts"]["status"]);
    assert_eq!(
        parsed["blocker_codes"],
        parsed["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        parsed["next_actions"],
        parsed["operator_contracts"]["next_actions"]
    );
    assert_eq!(parsed["status"], parsed["shared_fields"]["status"]);
    assert_eq!(
        parsed["blocker_codes"],
        parsed["shared_fields"]["blocker_codes"]
    );
    assert_eq!(
        parsed["next_actions"],
        parsed["shared_fields"]["next_actions"]
    );
}

#[test]
fn doctor_json_blocks_when_final_snapshot_top_level_operator_contract_parity_is_broken() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    sync_protocol_binding(&state_dir);

    let incompatible_snapshot_path =
        format!("{state_dir}/runtime-consumption/final-incomplete.json");
    std::fs::create_dir_all(format!("{state_dir}/runtime-consumption"))
        .expect("runtime-consumption directory should be created");
    std::fs::write(
        &incompatible_snapshot_path,
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": incompatible_snapshot_path,
            },
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "blocked",
                "blocker_codes": ["parity_mismatch"],
                "next_actions": ["normalize top-level operator contract mirrors"],
                "artifact_refs": {
                    "retrieval_trust_signal": {
                        "source": "runtime_consumption_snapshot_index",
                        "citation": "runtime-consumption/final-incomplete.json",
                        "freshness": "final",
                        "acl": "protocol-binding-receipt-id",
                    }
                }
            },
            "payload": {
                "docflow_activation": {
                    "evidence": {
                        "registry": {"ok": true},
                        "check": {"ok": true},
                        "readiness": {"verdict": "ready"},
                    }
                },
                "closure_admission": {
                    "status": "admit",
                    "blockers": [],
                }
            }
        })
        .to_string(),
    )
    .expect("incompatible final snapshot should be written");

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());

    let stdout = String::from_utf8_lossy(&doctor.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor json should parse");
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be array");
    let next_actions = parsed["next_actions"]
        .as_array()
        .expect("next_actions should be array");
    assert!(
        blocker_codes.iter().any(|code| {
            code.as_str() == Some("incomplete_release_admission_operator_evidence")
        }),
        "doctor must fail-closed when final snapshot top-level/operator-contract parity is broken"
    );
    assert!(
        next_actions.iter().any(|action| {
            action.as_str()
                == Some(
                    "Regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete.",
                )
        }),
        "doctor must publish remediation action when final snapshot release-admission evidence is incomplete"
    );
    let shared_blocker_codes = parsed["shared_fields"]["blocker_codes"]
        .as_array()
        .expect("shared_fields blocker_codes should be array");
    assert!(
        shared_blocker_codes.iter().any(|code| {
            code.as_str() == Some("incomplete_release_admission_operator_evidence")
        }),
        "shared_fields mirror must surface the same parity blocker"
    );
}

#[test]
fn canonical_operator_contract_status_is_shared_across_surfaces() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let pb = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding check should run");
    assert!(!pb.status.success());
    let pb_json: serde_json::Value =
        serde_json::from_slice(&pb.stdout).expect("protocol-binding check json should parse");
    let pb_operator_status = pb_json["operator_contracts"]["status"]
        .as_str()
        .expect("protocol-binding operator_contracts.status should exist");
    assert!(is_canonical_operator_status(pb_operator_status));

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    let doctor_operator_status = doctor_json["operator_contracts"]["status"]
        .as_str()
        .expect("doctor operator_contracts.status should exist");
    assert!(is_canonical_operator_status(doctor_operator_status));

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(status.status.success());
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    let status_operator_status = status_json["operator_contracts"]["status"]
        .as_str()
        .expect("status operator_contracts.status should exist");
    assert!(is_canonical_operator_status(status_operator_status));
}
