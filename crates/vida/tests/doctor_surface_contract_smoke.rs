use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
        "project:\n  id: test\n",
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

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    "unsupported_architecture_reserved_workflow_boundary";
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str =
    "clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str =
    "run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.";

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
                "run `vida taskflow consume bundle check --json` to record retrieval-trust operator evidence.",
            )
    });
    let has_retrieval_trust_signal_next_action = next_actions.iter().any(|action| {
            action.as_str()
            == Some(
                "run `vida taskflow protocol-binding sync --json` and `vida taskflow consume bundle check --json` to materialize retrieval-trust citation/freshness/acl signal.",
            )
    });
    let has_retrieval_trust_source_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("missing_retrieval_trust_source_operator_evidence"));
    let has_retrieval_trust_source_next_action = next_actions.iter().any(|action| {
            action.as_str()
            == Some(
                "run `vida taskflow consume bundle check --json` so runtime consumption snapshots publish retrieval-trust source evidence.",
            )
    });
    let has_recovery_readiness_blocker = blocker_codes
        .iter()
        .any(|code| code.as_str() == Some("recovery_readiness_blocked"));
    let has_recovery_readiness_next_action = next_actions.iter().any(|action| {
            action.as_str()
            == Some(
                "inspect `vida taskflow recovery latest --json`, then run `vida taskflow consume continue --json` after `recovery_ready=true` is proven for resume/rollback handoff.",
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
