use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "support/runtime_consumption.rs"]
mod runtime_consumption;

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
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

fn project_bound_state_dir() -> (String, String) {
    let project_root = unique_state_dir();
    let state_dir = format!("{project_root}/.vida/data/state");
    std::fs::create_dir_all(&state_dir).expect("create project-bound state dir");
    std::fs::write(format!("{project_root}/AGENTS.md"), "project").expect("write AGENTS.md");
    std::fs::write(
        format!("{project_root}/vida.config.yaml"),
        concat!(
            "project:\n",
            "  id: test\n",
            "agent_extensions:\n",
            "  role_selection:\n",
            "    mode: auto\n",
            "    fallback_role: orchestrator\n",
            "agent_system:\n",
            "  mode: native\n",
            "  state_owner: orchestrator_only\n",
        ),
    )
    .expect("write vida.config.yaml");
    for relative in [".vida/config", ".vida/db", ".vida/project"] {
        std::fs::create_dir_all(format!("{project_root}/{relative}"))
            .expect("runtime project marker dir should exist");
    }
    (project_root, state_dir)
}

fn is_canonical_operator_status(value: &str) -> bool {
    matches!(value, "pass" | "blocked")
}

fn assert_not_json_output(surface: &str, stdout: &str) {
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout).is_err(),
        "{surface} default human output must not be JSON: {stdout}"
    );
}

fn assert_no_raw_terminal_controls(surface: &str, stdout: &str) {
    assert!(
        !stdout.chars().any(|character| {
            character.is_control() && !matches!(character, '\n' | '\r' | '\t')
        }),
        "{surface} default human output must not contain raw terminal controls: {stdout:?}"
    );
}

fn assert_success(output: &std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &std::process::Output, context: &str) {
    assert!(
        !output.status.success(),
        "{context} should fail closed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    "unsupported_architecture_reserved_workflow_boundary";
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str =
    "clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "run `vida taskflow consume continue` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.";

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
    let snapshot = final_snapshot_with_shared_fields(snapshot);
    std::fs::write(
        format!("{runtime_consumption_dir}/{file_name}"),
        snapshot.to_string(),
    )
    .expect("final runtime-consumption snapshot should be written");
}

fn final_snapshot_with_shared_fields(mut snapshot: serde_json::Value) -> serde_json::Value {
    let Some(object) = snapshot.as_object_mut() else {
        return snapshot;
    };
    if object.contains_key("shared_fields") {
        return snapshot;
    }
    object.insert(
        "shared_fields".to_string(),
        serde_json::json!({
            "status": object
                .get("status")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("blocked")),
            "blocker_codes": object
                .get("blocker_codes")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
            "next_actions": object
                .get("next_actions")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        }),
    );
    snapshot
}

fn case10_closure_admission_record() -> serde_json::Value {
    serde_json::json!({
        "status": "pass",
        "admitted": true,
        "closure_decision": "closed",
        "decision_owner": "release-owner",
        "decision_at": "2026-05-19T00:00:00Z",
        "evidence_bundle_refs": ["evidence-bundle-case10"],
        "open_risk_acceptance_ids": ["risk-acceptance-case10"],
        "blockers": [],
        "proof_surfaces": ["vida taskflow consume final"],
        "evidence_table": [
            {
                "evidence_class": "closure_decision_record",
                "status": "pass",
                "evidence_refs": ["closure-record-case10"]
            },
            {
                "evidence_class": "runtime_consumption_final_snapshot",
                "status": "pass",
                "evidence_refs": ["final-snapshot-case10"]
            },
            {
                "evidence_class": "docflow_readiness_and_proof_receipts",
                "status": "pass",
                "evidence_refs": ["docflow-readiness-case10", "docflow-proof-case10"]
            },
            {
                "evidence_class": "lane_execution_and_handoff_receipts",
                "status": "pass",
                "evidence_refs": ["lane-execution-case10", "handoff-case10"]
            },
            {
                "evidence_class": "replay_checkpoint_lineage_artifacts",
                "status": "pass",
                "evidence_refs": ["checkpoint-case10", "replay-case10"]
            },
            {
                "evidence_class": "risk_acceptance_artifacts",
                "status": "pass",
                "evidence_refs": ["risk-acceptance-case10"]
            },
            {
                "evidence_class": "evidence_bundle_linkage",
                "status": "pass",
                "evidence_refs": ["evidence-bundle-case10"]
            }
        ]
    })
}

fn runtime_closure_admission_record() -> serde_json::Value {
    let evidence_table = runtime_closure_admission_evidence_table();
    serde_json::json!({
        "status": "pass",
        "admitted": true,
        "blockers": [],
        "proof_surfaces": [
            "vida taskflow consume bundle check",
            "vida docflow readiness-check --profile active-canon",
            "vida docflow proofcheck --profile active-canon"
        ],
        "evidence_table": evidence_table
    })
}

fn runtime_closure_admission_artifact() -> serde_json::Value {
    serde_json::json!({
        "artifact_type": "closure_admission_record",
        "owner_surface": "taskflow_consume_final",
        "workflow_class": "delegated_development_packet",
        "status": "pass",
        "release_scope": "CASE-10 closure admission evidence table completed",
        "supported_workflow_classes": ["delegated_development_packet"],
        "closure_decision": "admit",
        "decision_at": "2026-05-19T00:00:00Z",
        "decision_owner": "taskflow",
        "evidence_bundle_refs": [
            "vida taskflow consume bundle check",
            "vida docflow readiness-check --profile active-canon",
            "vida docflow proofcheck --profile active-canon"
        ],
        "blocked_by": [],
        "evidence_table": runtime_closure_admission_evidence_table()
    })
}

fn runtime_closure_admission_evidence_table() -> serde_json::Value {
    serde_json::json!([
        {
            "requirement": "taskflow_bundle_check",
            "status": "pass",
            "evidence_refs": ["vida taskflow consume bundle check"],
            "blockers": []
        },
        {
            "requirement": "docflow_readiness",
            "status": "pass",
            "evidence_refs": [
                "vida docflow readiness-check --profile active-canon",
                "vida docflow proofcheck --profile active-canon"
            ],
            "blockers": []
        },
        {
            "requirement": "approved_design_packet",
            "status": "pass",
            "evidence_refs": ["design_first_not_required"],
            "blockers": []
        },
        {
            "requirement": "spec_work_pool_dev_handoff",
            "status": "pass",
            "evidence_refs": ["tracked_flow_entry"],
            "blockers": []
        },
        {
            "requirement": "execution_preparation",
            "status": "pass",
            "evidence_refs": ["implementer"],
            "blockers": []
        }
    ])
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
            "{\"next_node\":\"coach\",\"selected_backend\":\"runtime_selected_tier\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"architecture_reserved\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
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
        step["next_action"]
            .as_str()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION)
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
                "run `vida taskflow consume bundle check` to record retrieval-trust operator evidence.",
            )
    });
    let has_retrieval_trust_signal_next_action = next_actions.iter().any(|action| {
            action.as_str()
            == Some(
                "run `vida taskflow protocol-binding sync` and `vida taskflow consume bundle check` to materialize retrieval-trust citation/freshness/acl signal.",
            )
    });
    let has_retrieval_trust_source_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("missing_retrieval_trust_source_operator_evidence"));
    let has_retrieval_trust_source_next_action = next_actions.iter().any(|action| {
            action.as_str()
            == Some(
                "run `vida taskflow consume bundle check` so runtime consumption snapshots publish retrieval-trust source evidence.",
            )
    });
    let has_recovery_readiness_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("recovery_readiness_blocked"));
    let has_recovery_readiness_next_action = next_actions.iter().any(|action| {
            action.as_str()
            == Some(
                "inspect `vida taskflow recovery latest`, then run `vida taskflow consume continue` after `recovery_ready=true` is proven for resume/rollback handoff.",
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
fn taskflow_consume_continue_fails_closed_when_developer_handoff_packet_is_pending() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    write_final_snapshot(
        &state_dir,
        "final-developer-handoff-pending.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "blocked",
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": {},
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {},
            },
            "closure_admission": {
                "status": "blocked",
                "blockers": ["pending_developer_handoff_packet"],
            },
            "payload": {
                "closure_admission": {
                    "status": "blocked",
                    "blockers": ["pending_developer_handoff_packet"],
                }
            },
            "dispatch_receipt": {
                "blocker_code": null
            }
        }),
    );

    let continue_cmd = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow consume continue should run");
    assert!(
        !continue_cmd.status.success(),
        "continue should fail-closed when developer handoff packet contract is still pending"
    );
    let stderr = String::from_utf8_lossy(&continue_cmd.stderr);
    assert!(
        stderr.contains("execution_preparation_gate_blocked"),
        "stderr should mention execution-preparation gate blocker, got: {stderr}"
    );
    assert!(
        stderr.contains("pending_execution_preparation_evidence"),
        "stderr should preserve canonical execution-preparation gate wording, got: {stderr}"
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
                    "regenerate consume-final evidence so canonical risk/register, closure/readiness, and release-1 operator-contract fields are complete.",
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

#[test]
fn status_and_doctor_default_human_output_is_compact_toon_with_explicit_json_parity() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let status_plain = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status default should run");
    assert!(
        status_plain.status.success(),
        "status default should succeed: stderr={}",
        String::from_utf8_lossy(&status_plain.stderr)
    );
    let status_stdout = String::from_utf8_lossy(&status_plain.stdout);
    assert!(
        status_stdout.starts_with("vida status\n"),
        "status default should start with TOON section title: {status_stdout}"
    );
    assert!(
        status_stdout.contains("  state_dir:"),
        "status default should expose compact TOON field names: {status_stdout}"
    );
    assert!(
        status_stdout.contains("  runtime_consumption:"),
        "status default should expose compact runtime evidence: {status_stdout}"
    );
    assert!(
        !status_stdout.contains("--json"),
        "status default human output should not suggest explicit JSON commands: {status_stdout}"
    );
    assert_not_json_output("vida status", &status_stdout);
    assert_no_raw_terminal_controls("vida status", &status_stdout);

    let status_json = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status json should run");
    assert!(
        status_json.status.success(),
        "status json should succeed: stderr={}",
        String::from_utf8_lossy(&status_json.stderr)
    );
    let status_payload: serde_json::Value =
        serde_json::from_slice(&status_json.stdout).expect("status json should parse");
    assert_eq!(status_payload["surface"], "vida status");
    assert!(status_payload.get("operator_contracts").is_some());
    assert!(status_payload.get("blocker_codes").is_some());
    assert!(status_payload.get("next_actions").is_some());
    assert!(status_payload.get("artifact_refs").is_some());

    let doctor_plain = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor default should run");
    assert!(
        doctor_plain.status.success(),
        "doctor default should succeed: stderr={}",
        String::from_utf8_lossy(&doctor_plain.stderr)
    );
    let doctor_stdout = String::from_utf8_lossy(&doctor_plain.stdout);
    assert!(
        doctor_stdout.starts_with("vida doctor\n"),
        "doctor default should start with TOON section title: {doctor_stdout}"
    );
    assert!(
        doctor_stdout.contains("  storage_metadata:"),
        "doctor default should expose compact TOON field names: {doctor_stdout}"
    );
    assert!(
        doctor_stdout.contains("  runtime_consumption:"),
        "doctor default should expose compact runtime evidence: {doctor_stdout}"
    );
    assert_not_json_output("vida doctor", &doctor_stdout);
    assert_no_raw_terminal_controls("vida doctor", &doctor_stdout);

    let doctor_json = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor json should run");
    assert!(
        doctor_json.status.success(),
        "doctor json should succeed: stderr={}",
        String::from_utf8_lossy(&doctor_json.stderr)
    );
    let doctor_payload: serde_json::Value =
        serde_json::from_slice(&doctor_json.stdout).expect("doctor json should parse");
    assert_eq!(doctor_payload["surface"], "vida doctor");
    assert!(doctor_payload.get("operator_contracts").is_some());
    assert!(doctor_payload.get("blocker_codes").is_some());
    assert!(doctor_payload.get("next_actions").is_some());
    assert!(doctor_payload.get("artifact_refs").is_some());
}

#[test]
fn status_and_orchestrator_init_support_compact_field_selection_output() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let status_json = vida()
        .args([
            "status",
            "--json",
            "--view",
            "compact",
            "--fields",
            "status,blocker_codes,next_actions",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status compact json fields should run");
    assert!(
        status_json.status.success(),
        "status compact json fields should succeed: stderr={}",
        String::from_utf8_lossy(&status_json.stderr)
    );
    let status_payload: serde_json::Value =
        serde_json::from_slice(&status_json.stdout).expect("status compact json should parse");
    let status_object = status_payload
        .as_object()
        .expect("status selected payload should be an object");
    assert_eq!(status_object.len(), 3);
    assert!(status_object.get("status").is_some());
    assert!(status_object.get("blocker_codes").is_some());
    assert!(status_object.get("next_actions").is_some());
    assert!(status_object.get("runtime_consumption").is_none());

    let status_plain = vida()
        .args([
            "status",
            "--view",
            "compact",
            "--fields",
            "status,blocker_codes,next_actions",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status compact fields default output should run");
    assert!(
        status_plain.status.success(),
        "status compact fields default output should succeed: stderr={}",
        String::from_utf8_lossy(&status_plain.stderr)
    );
    let status_stdout = String::from_utf8_lossy(&status_plain.stdout);
    assert!(status_stdout.starts_with("vida status\n"));
    assert!(status_stdout.contains("status:"));
    assert!(status_stdout.contains("blocker_codes"));
    assert!(status_stdout.contains("next_actions"));
    assert!(!status_stdout.contains("runtime_consumption"));
    assert!(!status_stdout.contains("--json"));
    assert_not_json_output("vida status --fields", &status_stdout);
    assert_no_raw_terminal_controls("vida status --fields", &status_stdout);

    let (_, project_state_dir) = project_bound_state_dir();
    let boot_project = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &project_state_dir)
        .output()
        .expect("project-bound boot should run");
    assert!(
        boot_project.status.success(),
        "project-bound boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot_project.stderr)
    );

    let orchestrator_json = vida()
        .args([
            "orchestrator-init",
            "--json",
            "--view",
            "compact",
            "--fields",
            "status,active_bounded_unit,next_actions",
        ])
        .env("VIDA_STATE_DIR", &project_state_dir)
        .output()
        .expect("orchestrator-init compact json fields should run");
    assert!(
        orchestrator_json.status.success(),
        "orchestrator-init compact json fields should succeed: stderr={}",
        String::from_utf8_lossy(&orchestrator_json.stderr)
    );
    let orchestrator_payload: serde_json::Value = serde_json::from_slice(&orchestrator_json.stdout)
        .expect("orchestrator-init compact json should parse");
    let orchestrator_object = orchestrator_payload
        .as_object()
        .expect("orchestrator selected payload should be an object");
    assert_eq!(orchestrator_object.len(), 3);
    assert!(orchestrator_object.get("status").is_some());
    assert!(orchestrator_object.get("active_bounded_unit").is_some());
    assert!(orchestrator_object.get("next_actions").is_some());
    assert!(orchestrator_object.get("runtime_bundle_summary").is_none());

    let orchestrator_plain = vida()
        .args([
            "orchestrator-init",
            "--view",
            "compact",
            "--fields",
            "status,active_bounded_unit,next_actions",
        ])
        .env("VIDA_STATE_DIR", &project_state_dir)
        .output()
        .expect("orchestrator-init compact fields default output should run");
    assert!(
        orchestrator_plain.status.success(),
        "orchestrator-init compact fields default output should succeed: stderr={}",
        String::from_utf8_lossy(&orchestrator_plain.stderr)
    );
    let orchestrator_stdout = String::from_utf8_lossy(&orchestrator_plain.stdout);
    assert!(orchestrator_stdout.starts_with("vida orchestrator-init\n"));
    assert!(orchestrator_stdout.contains("status:"));
    assert!(orchestrator_stdout.contains("active_bounded_unit"));
    assert!(orchestrator_stdout.contains("next_actions"));
    assert!(!orchestrator_stdout.contains("runtime_bundle_summary"));
    assert!(!orchestrator_stdout.contains("--json"));
    assert_not_json_output("vida orchestrator-init --fields", &orchestrator_stdout);
    assert_no_raw_terminal_controls("vida orchestrator-init --fields", &orchestrator_stdout);
}

#[test]
fn status_and_doctor_help_describe_default_toon_and_explicit_json() {
    for (args, surface) in [
        (&["status", "--help"][..], "vida status"),
        (&["doctor", "--help"][..], "vida doctor"),
    ] {
        let output = vida()
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("{surface} help should execute: {error}"));
        assert!(
            output.status.success(),
            "{surface} help should succeed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("compact TOON"),
            "{surface} help should document default compact TOON output: {stdout}"
        );
        assert!(
            stdout.contains("--json"),
            "{surface} help should document explicit JSON output: {stdout}"
        );
        assert!(
            stdout.contains("machine-readable JSON"),
            "{surface} help should describe JSON as machine-readable and explicit: {stdout}"
        );
    }
}

#[test]
fn status_and_orchestrator_init_help_describe_view_fields_and_json_options() {
    for (args, surface) in [
        (&["status", "--help"][..], "vida status"),
        (
            &["orchestrator-init", "--help"][..],
            "vida orchestrator-init",
        ),
    ] {
        let output = vida()
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("{surface} help should execute: {error}"));
        assert!(
            output.status.success(),
            "{surface} help should succeed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("--view"),
            "{surface} help should document compact/full view selection: {stdout}"
        );
        assert!(
            stdout.contains("--fields"),
            "{surface} help should document field selection: {stdout}"
        );
        assert!(
            stdout.contains("--json"),
            "{surface} help should document explicit JSON output: {stdout}"
        );
    }
}

#[test]
fn agent_host_bridge_outputs_default_toon_json_and_help_contracts() {
    let request_dir = unique_state_dir();
    std::fs::create_dir_all(&request_dir).expect("host bridge request dir should exist");
    let request_path = format!("{request_dir}/host-bridge-request.json");
    std::fs::write(
        &request_path,
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        })
        .to_string(),
    )
    .expect("host bridge request should be written");

    let default_output = vida()
        .args(["agent", "host-bridge", "--request", &request_path])
        .output()
        .expect("agent host-bridge default output should run");
    assert!(
        !default_output.status.success(),
        "agent host-bridge default output should fail closed for untrusted request path"
    );
    let stdout = String::from_utf8_lossy(&default_output.stdout);
    assert_not_json_output("vida agent host-bridge", &stdout);
    assert_no_raw_terminal_controls("vida agent host-bridge", &stdout);
    assert!(
        stdout.starts_with("vida agent host-bridge\n"),
        "agent host-bridge default output should be compact TOON: {stdout}"
    );
    assert!(stdout.contains("status: blocked"));
    assert!(stdout.contains("blocker_codes[1]:"));
    assert!(stdout.contains("host_bridge_request_untrusted_path"));
    assert!(
        !stdout.contains("completion:"),
        "blocked default output must not advertise completion: {stdout}"
    );
    assert!(
        !stdout.contains("--json"),
        "default completion guidance should not force JSON: {stdout}"
    );

    let json_output = vida()
        .args(["agent", "host-bridge", "--request", &request_path, "--json"])
        .output()
        .expect("agent host-bridge json output should run");
    assert!(
        !json_output.status.success(),
        "agent host-bridge json output should fail closed for untrusted request path"
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("host-bridge json should parse");
    assert_eq!(payload["surface"], "vida agent host-bridge");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["host_bridge_request_untrusted_path"])
    );
    assert_eq!(payload["shared_fields"]["status"], payload["status"]);
    assert_eq!(
        payload["shared_fields"]["blocker_codes"],
        payload["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        payload["shared_fields"]["next_actions"],
        payload["operator_contracts"]["next_actions"]
    );
    assert_eq!(
        payload["shared_fields"]["artifact_refs"],
        payload["operator_contracts"]["artifact_refs"]
    );
    assert_eq!(
        payload["operator_contracts"]["contract_id"],
        "host-agent-bridge-adapter-v1"
    );

    let help = vida()
        .args(["agent", "host-bridge", "--help"])
        .output()
        .expect("agent host-bridge help should run");
    assert!(
        help.status.success(),
        "agent host-bridge help should succeed: stderr={}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("--json"));
    assert!(help_stdout.contains("--state-dir"));
    assert!(
        help_stdout.contains("default compact TOON"),
        "agent host-bridge help should document default compact TOON: {help_stdout}"
    );
}

#[test]
fn agent_init_downstream_packet_preview_synthesizes_request_text_from_stale_coach_packet() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");

    let packet_dir = format!("{state_dir}/runtime-consumption/downstream-dispatch-packets");
    std::fs::create_dir_all(&packet_dir).expect("downstream packet dir should exist");
    let packet_path = format!("{packet_dir}/stale-empty-request-coach.json");
    std::fs::write(
        &packet_path,
        serde_json::json!({
            "packet_kind": "runtime_downstream_dispatch_packet",
            "run_id": "run-stale-empty-request-coach",
            "dispatch_target": "coach",
            "downstream_dispatch_target": "coach",
            "activation_runtime_role": "coach",
            "packet_template_kind": "coach_review_packet",
            "prompt": "# VIDA downstream dispatch packet\n\nRequest: ",
            "coach_review_packet": {
                "review_goal": "Validate implementer handoff evidence before coach approval.",
                "review_subject": "feature dev task",
                "blocking_question": "Does the implementer delivery include receipt-backed execution evidence?",
                "proof_target": "receipt-backed implementation evidence",
                "expected_output": "Return blocker if implementation evidence is missing.",
                "review_focus": [
                    "implementation_artifacts",
                    "source_dispatch_status",
                    "receipt_backed"
                ],
                "read_only_paths": [
                    "crates/vida/src/runtime_dispatch_state.rs"
                ]
            },
            "role_selection_full": {
                "ok": true,
                "activation_source": "packet",
                "selection_mode": "fixed",
                "fallback_role": "orchestrator",
                "request": "coach review",
                "selected_role": "coach",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": [],
                "compiled_bundle": null,
                "execution_plan": {
                    "backend_admissibility_matrix": [
                        {
                            "backend_id": "vibe_cli",
                            "backend_class": "external",
                            "lane_admissibility": {
                                "coach": true
                            }
                        }
                    ],
                    "development_flow": {
                        "coach": {
                            "executor_backend": "vibe_cli"
                        }
                    }
                },
                "reason": "test"
            }
        })
        .to_string(),
    )
    .expect("stale downstream packet should be written");

    let output = vida()
        .args(["agent-init", "--downstream-packet", &packet_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("agent-init downstream packet preview should run");
    assert_success(&output, "agent-init downstream packet preview");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("agent-init preview json should parse");
    assert_eq!(payload["selection"]["mode"], "downstream_packet");
    assert_eq!(payload["selection"]["selected_role"], "coach");
    let request_text = payload["selection"]["request_text"]
        .as_str()
        .expect("selection request text should be present");
    assert!(
        request_text.contains("Validate implementer handoff evidence"),
        "agent-init preview should synthesize request text from structured coach packet: {request_text}"
    );
    assert!(
        request_text.contains("receipt-backed implementation evidence"),
        "agent-init preview request should carry proof target: {request_text}"
    );
}

#[test]
fn agent_host_bridge_trusted_missing_receipt_fails_closed_within_latency_budget() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let bridge_dir = format!("{state_dir}/runtime-consumption/host-tool-bridge");
    std::fs::create_dir_all(&packet_dir).expect("dispatch packet dir should exist");
    std::fs::create_dir_all(&bridge_dir).expect("host bridge dir should exist");
    let packet_path = format!("{packet_dir}/run-host-bridge.json");
    let request_path = format!("{bridge_dir}/request.json");
    let result_path = format!("{bridge_dir}/result.json");
    let receipt_path = format!("{bridge_dir}/receipt.json");
    std::fs::write(&packet_path, "{}").expect("dispatch packet should be written");
    std::fs::write(
        &request_path,
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-host-bridge",
            "run_id": "run-host-bridge",
            "dispatch_target": "implementation",
            "packet_path": packet_path,
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": receipt_path
        })
        .to_string(),
    )
    .expect("host bridge request should be written");

    let started = Instant::now();
    let output = vida()
        .args([
            "agent",
            "host-bridge",
            "--request",
            &request_path,
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .output()
        .expect("host bridge json should run");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "trusted missing-receipt host bridge should fail closed inside operator latency budget; elapsed={elapsed:?}"
    );
    assert!(
        !output.status.success(),
        "missing dispatch receipt should fail closed"
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge json should parse");
    assert_eq!(payload["surface"], "vida agent host-bridge");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["host_bridge_dispatch_receipt_missing"])
    );
    assert_eq!(
        payload["shared_fields"]["blocker_codes"],
        payload["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        payload["shared_fields"]["artifact_refs"],
        payload["operator_contracts"]["artifact_refs"]
    );
}

#[derive(Debug)]
struct HostBridgeLaneFixture {
    state_dir: String,
    run_id: String,
    request_path: String,
    result_path: String,
    bridge_receipt_path: String,
}

fn persist_host_bridge_lane_receipt_with_helper(
    state_dir: &str,
    run_id: &str,
    dispatch_packet_path: &str,
    downstream_packet_path: &str,
    activation_result_path: &str,
) {
    let helper = std::env::current_exe().expect("current test binary should resolve");
    let output = Command::new(helper)
        .args([
            "--ignored",
            "--exact",
            "runtime_receipt_helper_process",
            "--nocapture",
        ])
        .env(runtime_consumption::RECEIPT_HELPER_STATE_DIR_ENV, state_dir)
        .env(runtime_consumption::RECEIPT_HELPER_RUN_ID_ENV, run_id)
        .env(
            runtime_consumption::RECEIPT_HELPER_DISPATCH_TARGET_ENV,
            "implementer",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DISPATCH_PACKET_PATH_ENV,
            dispatch_packet_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_TARGET_ENV,
            "coach",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_PACKET_PATH_ENV,
            downstream_packet_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_READY_ENV,
            "false",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_STATUS_ENV,
            "blocked",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_BLOCKERS_ENV,
            "pending_host_bridge_completion",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_RESULT_PATH_ENV,
            activation_result_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DISPATCH_STATUS_ENV,
            "bridge_request_pending",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_LANE_STATUS_ENV,
            "lane_open",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_BLOCKER_CODE_ENV,
            "host_tool_bridge_adapter_required",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_TASK_CLASS_ENV,
            "implementation",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_LIFECYCLE_STAGE_ENV,
            "implementer_blocked",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_HANDOFF_STATE_ENV,
            "none",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_RESUME_TARGET_ENV,
            "dispatch.implementer",
        )
        .output()
        .expect("runtime receipt helper process should run");
    assert_success(&output, "runtime receipt helper process");
}

fn create_host_bridge_lane_fixture(test_name: &str, changed_file: &str) -> HostBridgeLaneFixture {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");

    let run_id = format!("run-{test_name}");
    let parent_id = format!("{run_id}-epic");
    let create_parent = vida()
        .args([
            "task",
            "create",
            &parent_id,
            "Host bridge public proof epic",
            "--type",
            "epic",
            "--status",
            "open",
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .output()
        .expect("parent task create should run");
    assert_success(&create_parent, "parent task create");

    let create_task = vida()
        .args([
            "task",
            "create",
            &run_id,
            "Host bridge public proof task",
            "--type",
            "task",
            "--status",
            "open",
            "--parent-id",
            &parent_id,
            "--owned-path",
            "crates/vida/src/lib.rs",
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .output()
        .expect("task create should run");
    assert_success(&create_task, "task create");

    let artifact_dir = format!("{state_dir}/attempt-artifacts");
    std::fs::create_dir_all(&artifact_dir).expect("artifact dir should exist");
    let artifact_path = format!("{artifact_dir}/{test_name}-attempt.json");
    std::fs::write(
        &artifact_path,
        serde_json::json!({
            "artifact_kind": "patch_proposal",
            "task_id": run_id,
            "stage_id": "implementation",
            "changed_files": [changed_file]
        })
        .to_string(),
    )
    .expect("attempt artifact should be written");

    let record_attempt = vida()
        .args([
            "task",
            "attempt",
            "record",
            &run_id,
            "--stage-id",
            "implementation",
            "--backend",
            "internal_subagents",
            "--model-profile",
            "middle",
            "--isolation",
            "patch_proposal",
            "--status",
            "accepted",
            "--artifact-ref",
            &artifact_path,
            "--consolidation-receipt",
            &format!("{test_name}-consolidation-receipt"),
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .output()
        .expect("attempt record should run");
    assert_success(&record_attempt, "attempt record");

    let packet_dir = format!("{state_dir}/runtime-consumption/downstream-dispatch-packets");
    let bridge_dir = format!("{state_dir}/runtime-consumption/host-tool-bridge");
    let activation_dir = format!("{state_dir}/runtime-consumption/dispatch-results");
    std::fs::create_dir_all(&packet_dir).expect("packet dir should exist");
    std::fs::create_dir_all(&bridge_dir).expect("bridge dir should exist");
    std::fs::create_dir_all(&activation_dir).expect("activation dir should exist");
    let packet_path = format!("{packet_dir}/{test_name}.json");
    let request_path = format!("{bridge_dir}/{test_name}-request.json");
    let result_path = format!("{bridge_dir}/{test_name}-result.json");
    let bridge_receipt_path = format!("{bridge_dir}/{test_name}-receipt.json");
    let activation_result_path = format!("{activation_dir}/{test_name}-activation.json");

    std::fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": run_id,
            "dispatch_target": "implementer",
            "activation_runtime_role": "worker",
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["crates/vida/src/lib.rs"],
            "read_only_paths": ["crates/vida/src"],
            "delivery_task_packet": {
                "goal": "Complete host bridge lane evidence.",
                "scope_in": ["dispatch_target:implementer"],
                "handoff_task_class": "implementation",
                "handoff_runtime_role": "worker",
                "owned_paths": ["crates/vida/src/lib.rs"],
                "read_only_paths": ["crates/vida/src"],
                "definition_of_done": ["host bridge completion is receipt-backed"],
                "verification_command": "cargo test -p vida host_bridge_public_cli",
                "proof_target": "host bridge completion receipt",
                "stop_rules": ["stop if bridge evidence is missing"],
                "blocking_question": "none"
            },
            "downstream_dispatch_target": "coach",
            "downstream_dispatch_active_target": "implementer",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["pending_implementation_evidence"],
            "downstream_dispatch_status": "blocked",
            "downstream_lane_status": "lane_blocked"
        })
        .to_string(),
    )
    .expect("packet should be written");

    std::fs::write(
        &request_path,
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": test_name,
            "run_id": run_id,
            "task_id": run_id,
            "dispatch_target": "implementer",
            "packet_path": packet_path,
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "implementation_isolation": {
                "schema_version": "implementation-isolation-v1",
                "artifact_contract": "stage_attempt_implementation_artifact_v1",
                "owned_paths": ["crates/vida/src/lib.rs"]
            },
            "implementation_artifacts": [],
            "result_path": result_path,
            "receipt_path": bridge_receipt_path
        })
        .to_string(),
    )
    .expect("request should be written");

    std::fs::write(
        &activation_result_path,
        serde_json::json!({
            "artifact_kind": "runtime_dispatch_result",
            "status": "blocked",
            "execution_state": "bridge_request_pending",
            "host_tool_bridge_request": {
                "request_path": request_path,
                "result_path": result_path,
                "receipt_path": bridge_receipt_path
            }
        })
        .to_string(),
    )
    .expect("activation result should be written");

    persist_host_bridge_lane_receipt_with_helper(
        &state_dir,
        &run_id,
        &packet_path,
        &packet_path,
        &activation_result_path,
    );

    HostBridgeLaneFixture {
        state_dir,
        run_id,
        request_path,
        result_path,
        bridge_receipt_path,
    }
}

#[test]
#[ignore = "helper process for public host bridge fixture setup"]
fn runtime_receipt_helper_process() {
    if std::env::var(runtime_consumption::RECEIPT_HELPER_STATE_DIR_ENV).is_ok() {
        runtime_consumption::persist_ready_downstream_receipt_from_env();
    }
}

#[test]
fn host_bridge_public_cli_completes_with_taskflow_attempt_artifacts_without_parent_db_lock() {
    let fixture =
        create_host_bridge_lane_fixture("host-bridge-public-pass", "crates/vida/src/lib.rs");

    let output = vida()
        .args([
            "lane",
            "complete",
            &fixture.run_id,
            "--receipt-id",
            "host-bridge-public-pass-receipt",
            "--host-bridge-request",
            &fixture.request_path,
            "--host-agent-id",
            "agent-public-proof",
            "--host-bridge-summary",
            "internal agent completed",
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("lane complete public cli should run");
    assert_success(&output, "lane complete public cli pass");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("lane complete json should parse");
    assert_eq!(payload["surface"], "vida lane");
    assert_eq!(payload["status"], "pass");
    assert_eq!(payload["dispatch_status"], "executed");
    assert_eq!(payload["lane_status"], "lane_completed");
    assert_eq!(payload["blocker_codes"], serde_json::json!([]));
    let bridge_result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.result_path).expect("bridge result should exist"),
    )
    .expect("bridge result should be json");
    assert_eq!(bridge_result["execution_state"], "executed");
    assert_eq!(bridge_result["scope_validation"]["status"], "pass");
    assert!(
        std::path::Path::new(&fixture.bridge_receipt_path).exists(),
        "bridge receipt should be materialized"
    );
}

#[test]
fn host_bridge_public_cli_blocks_out_of_scope_taskflow_attempt_artifacts_without_parent_db_lock() {
    let fixture = create_host_bridge_lane_fixture(
        "host-bridge-public-block",
        "crates/vida/src/root_command_router.rs",
    );

    let output = vida()
        .args([
            "lane",
            "complete",
            &fixture.run_id,
            "--receipt-id",
            "host-bridge-public-block-receipt",
            "--host-bridge-request",
            &fixture.request_path,
            "--host-agent-id",
            "agent-public-proof",
            "--host-bridge-summary",
            "internal agent completed",
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("lane complete public cli should run");
    assert_failure(&output, "lane complete public cli blocked");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("lane complete blocked json should parse");
    assert_eq!(payload["surface"], "vida lane");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(payload["dispatch_status"], "blocked");
    assert!(payload["blocker_codes"]
        .as_array()
        .expect("blocker codes should be an array")
        .iter()
        .any(|code| code == "implementation_attempt_scope_guard_violation"));
    let bridge_result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.result_path).expect("bridge result should exist"),
    )
    .expect("bridge result should be json");
    assert_eq!(bridge_result["execution_state"], "blocked");
    assert_eq!(bridge_result["scope_validation"]["status"], "blocked");
}

#[test]
fn taskflow_next_outputs_default_toon_json_and_help_contracts() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let default_output = vida()
        .args(["taskflow", "next"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow next default output should run");
    assert!(
        !default_output.status.success(),
        "taskflow next should fail closed when no ready task exists"
    );
    let stdout = String::from_utf8_lossy(&default_output.stdout);
    assert_not_json_output("vida taskflow next", &stdout);
    assert_no_raw_terminal_controls("vida taskflow next", &stdout);
    assert!(
        stdout.starts_with("vida taskflow next\n"),
        "taskflow next default output should be compact TOON: {stdout}"
    );
    assert!(stdout.contains("status: blocked"));
    assert!(stdout.contains("blocker_codes[1]:"));
    assert!(stdout.contains("no_ready_tasks"));
    assert!(stdout.contains("recommended_command: vida task ready"));
    assert!(stdout.contains("recommended_surface: vida task ready"));
    assert!(
        !stdout.contains("--json"),
        "taskflow next default human output should not suggest explicit JSON commands: {stdout}"
    );

    let json_output = vida()
        .args(["taskflow", "next", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow next json output should run");
    assert!(
        !json_output.status.success(),
        "taskflow next json should fail closed when no ready task exists"
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("taskflow next json should parse");
    assert_eq!(payload["surface"], "vida taskflow next");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["no_ready_tasks"])
    );
    assert_eq!(payload["recommended_surface"], "vida task ready");
    assert_eq!(payload["shared_fields"]["status"], payload["status"]);
    assert_eq!(
        payload["shared_fields"]["blocker_codes"],
        payload["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        payload["shared_fields"]["next_actions"],
        payload["operator_contracts"]["next_actions"]
    );
    assert_eq!(
        payload["shared_fields"]["artifact_refs"],
        payload["operator_contracts"]["artifact_refs"]
    );
    assert_eq!(
        payload["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );

    let help = vida()
        .args(["taskflow", "next", "--help"])
        .output()
        .expect("taskflow next help should run");
    assert!(
        help.status.success(),
        "taskflow next help should succeed: stderr={}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("vida taskflow next"));
    assert!(help_stdout.contains("--json"));
    assert!(
        help_stdout.contains("compact TOON"),
        "taskflow next help should document default compact TOON: {help_stdout}"
    );
    assert!(
        help_stdout.contains("machine-readable JSON"),
        "taskflow next help should document explicit machine-readable JSON: {help_stdout}"
    );
}

#[test]
fn taskflow_closeout_outputs_default_toon_json_compact_and_help_contracts() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let default_output = vida()
        .args(["taskflow", "closeout"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow closeout default output should run");
    assert!(
        default_output.status.success(),
        "taskflow closeout default should succeed: stderr={}",
        String::from_utf8_lossy(&default_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&default_output.stdout);
    assert_not_json_output("vida taskflow closeout", &stdout);
    assert_no_raw_terminal_controls("vida taskflow closeout", &stdout);
    assert!(
        stdout.starts_with("vida taskflow closeout\n"),
        "taskflow closeout default output should be compact TOON: {stdout}"
    );
    for field in [
        "ready_count",
        "open_count",
        "active_agents_count",
        "active_lanes_count",
        "active_bounded_unit",
        "continuation_required_now",
        "stale_run_graph_present",
        "root_local_write_allowed",
        "all_epics_closed",
        "next_action",
    ] {
        assert!(
            stdout.contains(field),
            "taskflow closeout default output should include {field}: {stdout}"
        );
    }
    assert!(
        !stdout.contains("--json"),
        "taskflow closeout default human output should not suggest explicit JSON commands: {stdout}"
    );

    let fields_output = vida()
        .args([
            "taskflow",
            "closeout",
            "--view",
            "compact",
            "--fields",
            "status,next_action,open_count",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow closeout fields output should run");
    assert!(
        fields_output.status.success(),
        "taskflow closeout fields output should succeed: stderr={}",
        String::from_utf8_lossy(&fields_output.stderr)
    );
    let fields_stdout = String::from_utf8_lossy(&fields_output.stdout);
    assert_not_json_output("vida taskflow closeout --fields", &fields_stdout);
    assert!(fields_stdout.contains("status:"));
    assert!(fields_stdout.contains("next_action:"));
    assert!(fields_stdout.contains("open_count:"));
    assert!(
        !fields_stdout.contains("ready_count:"),
        "--fields should omit unrequested fields: {fields_stdout}"
    );

    let json_output = vida()
        .args(["taskflow", "closeout", "--json", "--compact"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow closeout json compact output should run");
    assert!(
        json_output.status.success(),
        "taskflow closeout json compact should succeed: stderr={}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("taskflow closeout json should parse");
    assert_eq!(payload["surface"], "vida taskflow closeout");
    assert_eq!(payload["status"], "pass");
    assert_eq!(payload["view"], "compact");
    for field in [
        "ready_count",
        "open_count",
        "active_agents_count",
        "active_lanes_count",
        "active_bounded_unit",
        "continuation_required_now",
        "stale_run_graph_present",
        "root_local_write_allowed",
        "all_epics_closed",
        "next_action",
    ] {
        assert!(
            payload.get(field).is_some(),
            "taskflow closeout json should include {field}: {payload}"
        );
    }
    assert!(
        matches!(
            payload["next_action"].as_str(),
            Some("none" | "close_epic" | "reconcile" | "recover_lane" | "run_gate")
        ),
        "next_action must be a compact enum: {payload}"
    );

    let fields_json_output = vida()
        .args([
            "taskflow",
            "closeout",
            "--json",
            "--fields",
            "status,next_action,open_count",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow closeout fields json output should run");
    assert!(
        fields_json_output.status.success(),
        "taskflow closeout fields json should succeed: stderr={}",
        String::from_utf8_lossy(&fields_json_output.stderr)
    );
    let fields_payload: serde_json::Value = serde_json::from_slice(&fields_json_output.stdout)
        .expect("taskflow closeout fields json should parse");
    let fields_object = fields_payload
        .as_object()
        .expect("fields payload should be an object");
    assert_eq!(fields_object.len(), 3);
    for expected in ["status", "next_action", "open_count"] {
        assert!(
            fields_object.contains_key(expected),
            "fields payload should contain {expected}: {fields_payload}"
        );
    }
    assert!(!fields_object.contains_key("ready_count"));

    let help = vida()
        .args(["taskflow", "closeout", "--help"])
        .output()
        .expect("taskflow closeout help should run");
    assert!(
        help.status.success(),
        "taskflow closeout help should succeed: stderr={}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("vida taskflow closeout"));
    assert!(help_stdout.contains("--compact"));
    assert!(help_stdout.contains("--view"));
    assert!(help_stdout.contains("--fields"));
    assert!(help_stdout.contains("--json"));
    assert!(help_stdout.contains("compact TOON"));
    assert!(help_stdout.contains("machine-readable JSON"));
    assert!(help_stdout.contains("next_action enum"));
    for field in [
        "ready_count",
        "open_count",
        "active_agents_count",
        "active_lanes_count",
        "active_bounded_unit",
        "continuation_required_now",
        "stale_run_graph_present",
        "root_local_write_allowed",
        "all_epics_closed",
        "next_action",
    ] {
        assert!(
            help_stdout.contains(field),
            "taskflow closeout help should document {field}: {help_stdout}"
        );
    }
    for variant in [
        "none",
        "close_epic",
        "reconcile",
        "recover_lane",
        "run_gate",
    ] {
        assert!(
            help_stdout.contains(variant),
            "taskflow closeout help should document next_action variant {variant}: {help_stdout}"
        );
    }
}

fn boot_session_triage_state() -> String {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );
    state_dir
}

fn create_session_triage_task(
    state_dir: &str,
    task_id: &str,
    title: &str,
    issue_type: &str,
    status: &str,
    priority: &str,
    parent_id: Option<&str>,
) {
    let mut args = vec![
        "task",
        "create",
        task_id,
        title,
        "--type",
        issue_type,
        "--status",
        status,
        "--priority",
        priority,
    ];
    if let Some(parent_id) = parent_id {
        args.extend(["--parent-id", parent_id]);
    }
    args.push("--json");

    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("session triage task create should run");
    assert!(
        output.status.success(),
        "session triage task create should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn session_triage_outputs_default_toon_json_and_help_contracts() {
    let state_dir = boot_session_triage_state();
    create_session_triage_task(
        &state_dir,
        "session-triage-epic",
        "Session triage epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "session-triage-active-task",
        "Session triage active task",
        "task",
        "in_progress",
        "1",
        Some("session-triage-epic"),
    );

    let default_output = vida()
        .args(["session", "triage"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("session triage default output should run");
    assert!(
        default_output.status.success(),
        "session triage default output should pass: stderr={}",
        String::from_utf8_lossy(&default_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&default_output.stdout);
    assert_not_json_output("vida session triage", &stdout);
    assert_no_raw_terminal_controls("vida session triage", &stdout);
    assert!(
        stdout.starts_with("vida session triage\n"),
        "session triage default output should be compact TOON: {stdout}"
    );
    assert!(stdout.contains("status: pass"));
    assert!(stdout.contains("active_bounded_unit:"));
    assert!(stdout.contains("session-triage-active-task"));
    assert!(stdout.contains("current_epic:"));
    assert!(stdout.contains("session-triage-epic"));
    assert!(stdout.contains("graph_validation:"));
    assert!(stdout.contains("valid: true"));
    assert!(
        !stdout.contains("--json"),
        "session triage default human output should not suggest explicit JSON commands: {stdout}"
    );

    let json_output = vida()
        .args([
            "session",
            "triage",
            "--task",
            "session-triage-active-task",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("session triage json output should run");
    assert!(
        json_output.status.success(),
        "session triage json output should pass: stderr={}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("session triage json should parse");
    assert_eq!(payload["surface"], "vida session triage");
    assert_eq!(payload["status"], "pass");
    assert_eq!(
        payload["active_bounded_unit"]["id"],
        "session-triage-active-task"
    );
    assert_eq!(payload["current_epic"]["id"], "session-triage-epic");
    assert_eq!(payload["graph_validation"]["valid"], true);
    assert_eq!(payload["graph_validation"]["issue_count"], 0);
    assert_eq!(
        payload["vida_owned_evidence"]["state_store_shared_inputs"],
        true
    );
    assert_eq!(
        payload["external_evidence"]["github"],
        "not_read_by_default"
    );
    assert_eq!(payload["shared_fields"]["status"], payload["status"]);
    assert_eq!(
        payload["shared_fields"]["blocker_codes"],
        payload["operator_contracts"]["blocker_codes"]
    );
    assert_eq!(
        payload["shared_fields"]["artifact_refs"],
        payload["operator_contracts"]["artifact_refs"]
    );

    let help = vida()
        .args(["session", "triage", "--help"])
        .output()
        .expect("session triage help should run");
    assert!(
        help.status.success(),
        "session triage help should succeed: stderr={}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("vida session triage"));
    assert!(help_stdout.contains("--json"));
    assert!(
        help_stdout.contains("compact TOON"),
        "session triage help should document default compact TOON: {help_stdout}"
    );
    assert!(
        help_stdout.contains("machine-readable"),
        "session triage help should document explicit machine-readable JSON: {help_stdout}"
    );
}

#[test]
fn session_triage_fails_closed_for_missing_explicit_task() {
    let state_dir = boot_session_triage_state();
    let output = vida()
        .args([
            "session",
            "triage",
            "--task",
            "missing-session-task",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("session triage missing task json output should run");
    assert!(
        !output.status.success(),
        "session triage should fail closed for missing explicit task"
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("session triage json should parse");
    assert_eq!(payload["surface"], "vida session triage");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["next_action_target_missing"])
    );
    assert_eq!(payload["target_task"], serde_json::Value::Null);
    assert_eq!(payload["graph_validation"]["valid"], true);
    assert_eq!(
        payload["artifact_refs"]["target_task_id"],
        "missing-session-task"
    );
}

#[test]
fn session_triage_fails_closed_for_multiple_active_tasks_without_explicit_binding() {
    let state_dir = boot_session_triage_state();
    create_session_triage_task(
        &state_dir,
        "session-triage-multiple-active-epic",
        "Session triage multiple active epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "session-triage-active-a",
        "Session triage active A",
        "task",
        "in_progress",
        "1",
        Some("session-triage-multiple-active-epic"),
    );
    create_session_triage_task(
        &state_dir,
        "session-triage-active-b",
        "Session triage active B",
        "task",
        "in_progress",
        "2",
        Some("session-triage-multiple-active-epic"),
    );

    let blocked = vida()
        .args(["session", "triage", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("session triage multiple active json output should run");
    assert!(
        !blocked.status.success(),
        "session triage should fail closed for multiple active tasks without explicit binding"
    );
    let blocked_payload: serde_json::Value =
        serde_json::from_slice(&blocked.stdout).expect("session triage json should parse");
    assert_eq!(blocked_payload["surface"], "vida session triage");
    assert_eq!(blocked_payload["status"], "blocked");
    assert_eq!(
        blocked_payload["blocker_codes"],
        serde_json::json!(["foreign_claim_conflict_blocked"])
    );
    assert_eq!(
        blocked_payload["active_bounded_unit"],
        serde_json::Value::Null
    );
    assert_eq!(
        blocked_payload["sequential_vs_parallel_posture"],
        "ambiguous_until_explicit_binding"
    );
    assert_eq!(
        blocked_payload["active_bounded_unit_candidates"]
            .as_array()
            .expect("active candidates should be array")
            .len(),
        2
    );

    let explicit = vida()
        .args([
            "session",
            "triage",
            "--task",
            "session-triage-active-b",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("session triage explicit active json output should run");
    assert!(
        explicit.status.success(),
        "explicit session triage should pass for selected active task: stderr={}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    let explicit_payload: serde_json::Value =
        serde_json::from_slice(&explicit.stdout).expect("session triage json should parse");
    assert_eq!(explicit_payload["status"], "pass");
    assert_eq!(
        explicit_payload["active_bounded_unit"]["id"],
        "session-triage-active-b"
    );
}

#[test]
fn doctor_json_prefers_latest_final_snapshot_guard_when_latest_snapshot_is_bundle_check() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let dispatch_packets_dir = format!("{runtime_consumption_dir}/dispatch-packets");
    std::fs::create_dir_all(&dispatch_packets_dir).expect("dispatch packet dir should exist");

    let dispatch_packet_path = format!("{dispatch_packets_dir}/guard-packet.json");
    std::fs::write(
        &dispatch_packet_path,
        serde_json::json!({
            "root_session_write_guard": {
                "status": "blocked_by_default",
                "root_session_role": "orchestrator",
                "local_write_requires_exception_path": true,
                "required_exception_evidence": "exception_path_receipt_id",
                "pre_write_checkpoint_required": true
            }
        })
        .to_string(),
    )
    .expect("dispatch packet should write");

    let final_snapshot_path = format!("{runtime_consumption_dir}/final-2026-03-19T00-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-03-19T00-00-01Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": final_snapshot_path,
                }
            },
            "payload": {
                "closure_admission": case10_closure_admission_record()
            },
            "source_dispatch_packet_path": dispatch_packet_path,
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": final_snapshot_path,
            }
        }),
    );

    std::thread::sleep(std::time::Duration::from_millis(15));
    std::fs::write(
        format!("{runtime_consumption_dir}/bundle-check-2026-03-19T00-00-02Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume bundle check",
            "check": { "ok": true }
        })
        .to_string(),
    )
    .expect("bundle-check snapshot should write");

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");

    assert_eq!(
        doctor_json["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(final_snapshot_path)
    );
}

#[test]
fn doctor_json_ignores_newer_incomplete_final_when_admissible_final_exists() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    sync_protocol_binding(&state_dir);

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let admitted_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-05-19T00-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-19T00-00-01Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": admitted_snapshot_path,
                }
            },
            "payload": {
                "closure_admission": case10_closure_admission_record()
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": admitted_snapshot_path,
            }
        }),
    );

    std::thread::sleep(std::time::Duration::from_millis(15));
    let stale_snapshot_path = format!("{runtime_consumption_dir}/final-2026-05-18T00-00-02Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-18T00-00-02Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": stale_snapshot_path,
                }
            },
            "payload": {
                "closure_admission": {
                    "status": "blocked",
                    "admitted": false,
                    "blockers": ["missing release evidence"]
                }
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": stale_snapshot_path,
            }
        }),
    );

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    let blocker_codes = doctor_json["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be array");

    assert!(
        !blocker_codes.iter().any(|code| {
            code.as_str() == Some("incomplete_release_admission_operator_evidence")
        }),
        "doctor json must not let a newer incomplete final snapshot override admissible closure evidence"
    );
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(admitted_snapshot_path)
    );
}

#[test]
fn doctor_json_accepts_latest_terminal_continue_closure_release_admission() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    sync_protocol_binding(&state_dir);

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let older_snapshot_path = format!("{runtime_consumption_dir}/final-2026-05-18T00-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-18T00-00-01Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": older_snapshot_path,
                }
            },
            "payload": {
                "closure_admission": case10_closure_admission_record()
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": older_snapshot_path,
            }
        }),
    );

    std::thread::sleep(std::time::Duration::from_millis(15));
    let terminal_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-05-19T00-00-02Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-19T00-00-02Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume continue",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": terminal_snapshot_path,
                }
            },
            "payload": {
                "dispatch_receipt": {
                    "dispatch_status": "executed",
                    "lane_status": "lane_completed"
                },
                "release_admission": case10_closure_admission_record()
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": terminal_snapshot_path,
            }
        }),
    );

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    let blocker_codes = doctor_json["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be array");

    assert!(
        !blocker_codes.iter().any(|code| {
            code.as_str() == Some("incomplete_release_admission_operator_evidence")
        }),
        "doctor json must accept the latest terminal consume-continue closure receipt"
    );
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(terminal_snapshot_path)
    );
}

#[test]
fn doctor_json_rejects_final_snapshot_without_case10_evidence_families() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    sync_protocol_binding(&state_dir);

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let snapshot_path = format!("{runtime_consumption_dir}/final-2026-05-19T00-00-03Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-19T00-00-03Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": snapshot_path,
                }
            },
            "payload": {
                "closure_admission": {
                    "status": "pass",
                    "admitted": true,
                    "closure_decision": "closed",
                    "decision_owner": "release-owner",
                    "decision_at": "2026-05-19T00:00:00Z",
                    "evidence_bundle_refs": ["evidence-bundle-case10"],
                    "blockers": [],
                    "evidence_table": [
                        {
                            "evidence_class": "runtime_consumption_final_snapshot",
                            "status": "pass",
                            "evidence_refs": ["final-snapshot-case10"]
                        }
                    ]
                }
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": snapshot_path,
            }
        }),
    );

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    let blocker_codes = doctor_json["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be array");

    assert!(
        blocker_codes.iter().any(|code| {
            code.as_str() == Some("incomplete_release_admission_operator_evidence")
        }),
        "doctor json must reject a final snapshot that lacks CASE-10 evidence families"
    );
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(snapshot_path)
    );
}

#[test]
fn bundle_check_retrieval_trust_evidence_clears_status_and_doctor_retrieval_blockers() {
    let (_project_root, state_dir) = project_bound_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let protocol_sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        protocol_sync.status.success(),
        "protocol-binding sync should succeed: {}",
        String::from_utf8_lossy(&protocol_sync.stderr)
    );
    let protocol_sync_json: serde_json::Value =
        serde_json::from_slice(&protocol_sync.stdout).expect("protocol-binding sync json");
    let protocol_binding_receipt_id = protocol_sync_json["receipt"]["receipt_id"]
        .as_str()
        .expect("protocol-binding receipt id should be present")
        .to_string();

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let snapshot_path = format!("{runtime_consumption_dir}/final-2026-05-19T00-00-04Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-19T00-00-04Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": snapshot_path,
                }
            },
            "payload": {
                "closure_admission": {
                    "status": "pass",
                    "admitted": true,
                    "closure_decision": "closed",
                    "decision_owner": "release-owner",
                    "decision_at": "2026-05-19T00:00:00Z",
                    "evidence_bundle_refs": ["evidence-bundle-case10"],
                    "blockers": [],
                    "evidence_table": [
                        {
                            "evidence_class": "runtime_consumption_final_snapshot",
                            "status": "pass",
                            "evidence_refs": ["final-snapshot-case10"]
                        }
                    ]
                }
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": snapshot_path,
            }
        }),
    );

    std::thread::sleep(std::time::Duration::from_millis(15));
    std::fs::write(
        format!("{runtime_consumption_dir}/bundle-check-2026-05-19T00-00-05Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume bundle check",
            "check": { "ok": true },
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "root_artifact_id": "framework-agent-definition",
                    "bundle_artifact_name": "taskflow_runtime_bundle",
                    "surface": "vida taskflow consume bundle check"
                }
            },
            "bundle": {
                "cache_delivery_contract": {
                    "retrieval_trust_evidence": {
                        "source": "runtime_consumption_snapshot_index",
                        "source_registry_ref": "runtime_consumption_snapshot_registry:latest_recorded_final_snapshot",
                        "citation": snapshot_path,
                        "freshness": "final",
                        "freshness_posture": "latest_recorded_final_snapshot",
                        "acl": protocol_binding_receipt_id,
                        "acl_context": format!("protocol_binding_receipt:{protocol_binding_receipt_id}"),
                        "acl_propagation": "protocol_binding_receipt_runtime_gate"
                    }
                }
            }
        })
        .to_string(),
    )
    .expect("bundle-check snapshot should be written");

    for surface_args in [
        vec!["status", "--json"],
        vec!["status", "--summary", "--json"],
        vec!["doctor", "--json"],
    ] {
        let output = vida()
            .args(surface_args)
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("operator surface should run");
        assert!(output.status.success());
        let surface_json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("operator json should parse");
        let blocker_codes = surface_json["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be array");

        assert!(
            blocker_codes.iter().any(|code| {
                code.as_str() == Some("incomplete_release_admission_operator_evidence")
            }),
            "one incomplete final snapshot alone must keep release admission blocked"
        );
        for retrieval_blocker in [
            "missing_retrieval_trust_operator_evidence",
            "missing_retrieval_trust_signal_operator_evidence",
            "missing_retrieval_trust_source_operator_evidence",
        ] {
            assert!(
                !blocker_codes
                    .iter()
                    .any(|code| code.as_str() == Some(retrieval_blocker)),
                "{retrieval_blocker} should be cleared by latest passing bundle-check retrieval trust evidence"
            );
        }
    }
}

#[test]
fn status_and_doctor_accept_runtime_closure_admission_after_bundle_check() {
    let (_project_root, state_dir) = project_bound_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let protocol_sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        protocol_sync.status.success(),
        "protocol-binding sync should succeed: {}",
        String::from_utf8_lossy(&protocol_sync.stderr)
    );
    let protocol_sync_json: serde_json::Value =
        serde_json::from_slice(&protocol_sync.stdout).expect("protocol-binding sync json");
    let protocol_binding_receipt_id = protocol_sync_json["receipt"]["receipt_id"]
        .as_str()
        .expect("protocol-binding receipt id should be present")
        .to_string();

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let final_snapshot_path = format!("{runtime_consumption_dir}/final-2026-05-19T00-00-06Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-05-19T00-00-06Z.json",
        serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": final_snapshot_path,
                    "protocol_binding_latest_receipt_id": protocol_binding_receipt_id,
                }
            },
            "payload": {
                "closure_admission": runtime_closure_admission_record(),
                "closure_admission_artifact": runtime_closure_admission_artifact()
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": final_snapshot_path,
            }
        }),
    );

    std::thread::sleep(std::time::Duration::from_millis(15));
    std::fs::write(
        format!("{runtime_consumption_dir}/bundle-check-2026-05-19T00-00-07Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume bundle check",
            "check": { "ok": true },
            "blocker_codes": [],
            "next_actions": [],
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "root_artifact_id": "framework-agent-definition",
                    "bundle_artifact_name": "taskflow_runtime_bundle",
                    "surface": "vida taskflow consume bundle check"
                }
            },
            "bundle": {
                "cache_delivery_contract": {
                    "retrieval_trust_evidence": {
                        "source": "runtime_consumption_snapshot_index",
                        "source_registry_ref": "runtime_consumption_snapshot_registry:latest_final_release_admission",
                        "citation": final_snapshot_path,
                        "freshness": "final",
                        "freshness_posture": "latest_final_release_admission_snapshot",
                        "acl": protocol_binding_receipt_id,
                        "acl_context": format!("protocol_binding_receipt:{protocol_binding_receipt_id}"),
                        "acl_propagation": "protocol_binding_receipt_runtime_gate"
                    }
                }
            }
        })
        .to_string(),
    )
    .expect("bundle-check snapshot should be written");

    let projection_dir = format!("{state_dir}/operator-projections");
    std::fs::create_dir_all(&projection_dir).expect("operator projection dir should exist");
    std::fs::write(
        format!("{projection_dir}/status-summary-v2-latest.json"),
        serde_json::json!({
            "surface": "vida status",
            "view": "summary",
            "status": "blocked",
            "blocker_codes": [
                "incomplete_release_admission_operator_evidence",
                "missing_retrieval_trust_operator_evidence"
            ],
            "next_actions": ["stale cached summary"],
            "shared_fields": {
                "status": "blocked",
                "blocker_codes": [
                    "incomplete_release_admission_operator_evidence",
                    "missing_retrieval_trust_operator_evidence"
                ],
                "next_actions": ["stale cached summary"]
            },
            "operator_contracts": {
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "blocked",
                "blocker_codes": [
                    "incomplete_release_admission_operator_evidence",
                    "missing_retrieval_trust_operator_evidence"
                ],
                "next_actions": ["stale cached summary"],
                "artifact_refs": {
                    "runtime_consumption_latest_snapshot_path": format!("{runtime_consumption_dir}/bundle-check-2026-05-19T00-00-07Z.json")
                }
            }
        })
        .to_string(),
    )
    .expect("stale status summary projection should be written");

    for surface_args in [
        vec!["status", "--json"],
        vec!["status", "--summary", "--json"],
        vec!["doctor", "--json"],
    ] {
        let output = vida()
            .args(surface_args)
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("operator surface should run");
        assert!(output.status.success());
        let surface_json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("operator json should parse");
        let blocker_codes = surface_json["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be array");

        assert!(
            !blocker_codes.iter().any(|code| {
                code.as_str() == Some("incomplete_release_admission_operator_evidence")
            }),
            "latest passing runtime consume-final closure admission must satisfy release admission after bundle-check"
        );
    }
}

#[test]
fn status_and_doctor_quarantine_missing_task_orphan_run_graph() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let ready_task_id = "taskflow-defect-case-10-runtime-probe-closure-status-blocker";
    let ready_parent_id = "taskflow-defect-case-10-runtime-probe-parent";
    let ready_parent = vida()
        .args([
            "task",
            "create",
            ready_parent_id,
            "CASE-10 runtime probe parent",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "0",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("ready parent task should be created");
    assert!(
        ready_parent.status.success(),
        "ready parent task creation should succeed: {}",
        String::from_utf8_lossy(&ready_parent.stderr)
    );
    let ready = vida()
        .args([
            "task",
            "create",
            ready_task_id,
            "CASE-10 runtime-probe closure status blocker",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "0",
            "--parent-id",
            ready_parent_id,
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("ready task should be created");
    assert!(
        ready.status.success(),
        "ready task creation should succeed: {}",
        String::from_utf8_lossy(&ready.stderr)
    );

    let orphan_run_id = "runtime-probe-closure";
    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            orphan_run_id,
            "implementation",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("orphan run graph should init");
    assert!(init.status.success());
    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            orphan_run_id,
            "analysis",
            "analysis",
            "blocked",
            "analysis",
            "{\"policy_gate\":\"validation_report_required\",\"context_state\":\"sealed\",\"resume_target\":\"none\",\"recovery_ready\":false}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("orphan run graph should update");
    assert!(update.status.success());

    let missing = vida()
        .args(["task", "show", orphan_run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("missing task probe should run");
    assert!(!missing.status.success());

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "status stdout={} stderr={}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_eq!(
        status_json["continuation_binding"]["stale_missing_task_run_graph_status"]["run_id"],
        orphan_run_id
    );
    assert_ne!(
        status_json["continuation_binding"]["ambiguity_reason"],
        "latest_run_graph_status_blocked"
    );
    assert!(!status_json["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("status blocker codes")
        .iter()
        .any(|code| code == "continuation_binding_ambiguous"));

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    let doctor_blockers = doctor_json["blocker_codes"]
        .as_array()
        .expect("doctor blocker codes");
    assert!(!doctor_blockers
        .iter()
        .any(|code| code == "recovery_readiness_blocked"));
}

#[test]
fn projection_surfaces_fail_closed_for_ready_missing_task_run_host_bridge() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "zzzz-run-host-bridge";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );
    sync_protocol_binding(&state_dir);

    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            run_id,
            "host_bridge",
            "planning",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-host-bridge run graph should init");
    assert!(
        init.status.success(),
        "run graph init should succeed: stderr={}",
        String::from_utf8_lossy(&init.stderr)
    );

    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            run_id,
            "host_bridge",
            "host_bridge",
            "ready",
            "planning",
            "{\"next_node\":\"implementer\",\"selected_backend\":\"internal_subagents\",\"lane_id\":\"host_bridge_lane\",\"lifecycle_stage\":\"implementation_dispatch_ready\",\"policy_gate\":\"host_tool_bridge_adapter_required\",\"handoff_state\":\"handoff_pending\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.implementer\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-host-bridge run graph should update");
    assert!(
        update.status.success(),
        "run graph update should succeed: stderr={}",
        String::from_utf8_lossy(&update.stderr)
    );

    let missing_task = vida()
        .args(["task", "show", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task show should run");
    assert!(!missing_task.status.success());

    let orchestrator = vida()
        .args(["orchestrator-init", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("orchestrator-init should run");
    assert!(
        orchestrator.status.success(),
        "orchestrator-init should succeed: stderr={}",
        String::from_utf8_lossy(&orchestrator.stderr)
    );
    let orchestrator_json: serde_json::Value =
        serde_json::from_slice(&orchestrator.stdout).expect("orchestrator-init json should parse");
    assert_ne!(
        orchestrator_json["continuation_binding"]["active_bounded_unit"]["task_id"],
        run_id
    );

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "status should succeed: stderr={}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_ne!(
        status_json["continuation_binding"]["active_bounded_unit"]["task_id"],
        run_id
    );

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(
        run_graph.status.success(),
        "run-graph status should succeed: stderr={}",
        String::from_utf8_lossy(&run_graph.stderr)
    );
    let run_graph_json: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run-graph json should parse");
    assert_eq!(
        run_graph_json["projection_truth"]["stale_state_suspected"],
        true
    );
    assert_eq!(
        run_graph_json["projection_truth"]["next_lawful_operator_action"],
        format!(
            "vida lane retire {run_id} --receipt-id {run_id} --reason \"missing TaskFlow task stale run\""
        )
    );

    let recovery = vida()
        .args(["taskflow", "recovery", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery latest should run");
    assert!(
        !recovery.status.success(),
        "recovery latest should fail closed for stale missing-task host-bridge runs"
    );
    let recovery_json: serde_json::Value =
        serde_json::from_slice(&recovery.stdout).expect("recovery json should parse");
    assert_eq!(recovery_json["status"], "blocked");
    assert!(
        recovery_json["blocker_codes"]
            .as_array()
            .expect("recovery blocker codes")
            .iter()
            .any(|code| code == "stale_missing_task_run_graph"),
        "recovery status should expose missing-task blocker: {recovery_json}"
    );

    let consume = vida()
        .args([
            "taskflow", "consume", "continue", "--run-id", run_id, "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume continue should run");
    assert!(
        !consume.status.success(),
        "consume continue should fail closed for missing task authority"
    );
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume.stdout).expect("consume json should parse");
    assert_eq!(consume_json["status"], "blocked");
    assert_eq!(
        consume_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );

    let ready_parent_id = "analyst-ready-after-stale-pass-parent";
    create_session_triage_task(
        &state_dir,
        ready_parent_id,
        "Analyst ready after stale pass parent",
        "epic",
        "open",
        "1",
        None,
    );
    let ready_task_id = "analyst-ready-after-stale-pass-run";
    create_session_triage_task(
        &state_dir,
        ready_task_id,
        "Analyst ready after stale pass run",
        "task",
        "in_progress",
        "1",
        Some(ready_parent_id),
    );
    let dispatch = vida()
        .args(["agent", "dispatch-next", "--dev-team", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("dispatch preview should run");
    assert!(
        !dispatch.status.success(),
        "dispatch should fail closed while latest run references missing task authority: {}",
        String::from_utf8_lossy(&dispatch.stdout)
    );
    let dispatch_json: serde_json::Value =
        serde_json::from_slice(&dispatch.stdout).expect("dispatch-next json should parse");
    assert_eq!(dispatch_json["status"], "blocked");
    assert_eq!(dispatch_json["lanes_selected"], 0);
    assert_eq!(
        dispatch_json["parallelization_planner"]["materializes_packets"],
        false
    );
    assert_eq!(dispatch_json["flow_projection"]["status"], "blocked");
    assert!(dispatch_json["blocker_codes"].as_array().is_some());
    assert!(
        !dispatch_json.to_string().contains("open_delegated_cycle"),
        "dev-team dispatch must not misclassify missing-task stale runs as open delegated cycles: {dispatch_json}"
    );
    assert!(
        !dispatch_json
            .to_string()
            .contains("dispatch_packet_contract_invalid"),
        "dev-team dispatch must fail before stale packet repair/validation noise: {dispatch_json}"
    );
    let _ = std::fs::remove_dir_all(project_root);
}

#[test]
fn projection_surfaces_fail_closed_for_pass_missing_task_run_host_bridge() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "zzzz-run-host-bridge-pass";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: stderr={}",
        String::from_utf8_lossy(&boot.stderr)
    );
    sync_protocol_binding(&state_dir);

    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            run_id,
            "host_bridge",
            "planning",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("pass missing-task run graph should init");
    assert!(
        init.status.success(),
        "run graph init should succeed: stderr={}",
        String::from_utf8_lossy(&init.stderr)
    );

    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            run_id,
            "host_bridge",
            "host_bridge",
            "pass",
            "planning",
            "{\"next_node\":\"implementer\",\"selected_backend\":\"internal_subagents\",\"lane_id\":\"host_bridge_lane\",\"lifecycle_stage\":\"implementation_dispatch_ready\",\"policy_gate\":\"host_tool_bridge_adapter_required\",\"handoff_state\":\"handoff_pending\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.implementer\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("pass missing-task run graph should update");
    assert!(
        update.status.success(),
        "run graph update should succeed: stderr={}",
        String::from_utf8_lossy(&update.stderr)
    );

    let missing_task = vida()
        .args(["task", "show", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task show should run");
    assert!(!missing_task.status.success());

    let orchestrator = vida()
        .args(["orchestrator-init", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("orchestrator-init should run");
    assert!(
        orchestrator.status.success(),
        "orchestrator-init should succeed: stderr={}",
        String::from_utf8_lossy(&orchestrator.stderr)
    );
    let orchestrator_json: serde_json::Value =
        serde_json::from_slice(&orchestrator.stdout).expect("orchestrator-init json should parse");
    assert_ne!(
        orchestrator_json["continuation_binding"]["active_bounded_unit"]["task_id"],
        run_id
    );

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "status should succeed: stderr={}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_ne!(
        status_json["continuation_binding"]["active_bounded_unit"]["task_id"],
        run_id
    );

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(
        run_graph.status.success(),
        "run-graph status should succeed: stderr={}",
        String::from_utf8_lossy(&run_graph.stderr)
    );
    let run_graph_json: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run-graph json should parse");
    assert_eq!(
        run_graph_json["projection_truth"]["stale_state_suspected"],
        true
    );
    assert_eq!(
        run_graph_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );
    assert_eq!(
        run_graph_json["projection_truth"]["next_lawful_operator_action"],
        format!(
            "vida lane retire {run_id} --receipt-id {run_id} --reason \"missing TaskFlow task stale run\""
        )
    );

    let recovery = vida()
        .args(["taskflow", "recovery", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery status should run");
    assert!(
        !recovery.status.success(),
        "recovery status should fail closed for stale missing-task host-bridge runs"
    );
    let recovery_json: serde_json::Value =
        serde_json::from_slice(&recovery.stdout).expect("recovery json should parse");
    assert_eq!(recovery_json["status"], "blocked");
    assert_eq!(
        recovery_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );

    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    std::fs::create_dir_all(&packet_dir).expect("dispatch packet dir should exist");
    std::fs::write(
        format!("{packet_dir}/zzzz-malformed-latest-packet.json"),
        serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "packet_template_kind": "delivery_task_packet",
            "run_id": "zzzz-malformed-latest-packet",
            "dispatch_target": "implementer",
            "delivery_task_packet": {
                "task_id": "zzzz-malformed-latest-packet",
                "request_text": "Implement only crates/vida/src/taskflow_consume_resume.rs"
            }
        })
        .to_string(),
    )
    .expect("malformed dispatch packet should be written");

    let default_consume = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("default consume continue should run");
    assert!(
        !default_consume.status.success(),
        "default consume continue should fail closed for latest missing task authority"
    );
    let default_consume_json: serde_json::Value =
        serde_json::from_slice(&default_consume.stdout).expect("default consume json should parse");
    assert_eq!(default_consume_json["status"], "blocked");
    assert_eq!(
        default_consume_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );
    assert_ne!(
        default_consume_json["blocker_codes"],
        serde_json::json!(["dispatch_packet_contract_invalid"])
    );

    let consume = vida()
        .args([
            "taskflow", "consume", "continue", "--run-id", run_id, "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume continue should run");
    assert!(
        !consume.status.success(),
        "consume continue should fail closed for pass missing task authority"
    );
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume.stdout).expect("consume json should parse");
    assert_eq!(consume_json["status"], "blocked");
    assert_eq!(
        consume_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );
    let retire = vida()
        .args([
            "lane",
            "retire",
            run_id,
            "--receipt-id",
            run_id,
            "--reason",
            "missing TaskFlow task stale run",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("lane retire should run");
    assert!(
        retire.status.success(),
        "lane retire should clean missing-task stale run without an existing receipt: stderr={}",
        String::from_utf8_lossy(&retire.stderr)
    );
    let default_after_retire = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("default consume continue after retire should run");
    let default_after_retire_json: serde_json::Value =
        serde_json::from_slice(&default_after_retire.stdout)
            .expect("default consume after retire json should parse");
    assert_ne!(
        default_after_retire_json["blocker_codes"],
        serde_json::json!(["dispatch_packet_contract_invalid"]),
        "retired stale run must not let unrelated malformed packet files block default consume"
    );
    let _ = std::fs::remove_dir_all(project_root);
}
