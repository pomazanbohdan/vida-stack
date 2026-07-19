use std::ffi::OsStr;
use std::io;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use taskflow_host_bridge::effective_host_bridge_request_with_registry;

#[path = "support/runtime_consumption.rs"]
mod runtime_consumption;

struct VidaCommand {
    command: Command,
}

impl VidaCommand {
    fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg(arg);
        self
    }

    fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.command.env(key, value);
        self
    }

    fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.command.env_remove(key);
        self
    }

    fn current_dir(&mut self, dir: impl AsRef<std::path::Path>) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    fn output(&mut self) -> io::Result<Output> {
        let mut last = None;
        for attempt in 0..20 {
            let output = self.command.output()?;
            if !is_state_store_read_lock_contention(&output) {
                return Ok(output);
            }
            last = Some(output);
            thread::sleep(Duration::from_millis((50 * (attempt + 1)).min(500)));
        }
        Ok(last.expect("retry loop should capture lock-contention output"))
    }
}

fn is_state_store_read_lock_contention(output: &Output) -> bool {
    !output.status.success()
        && (String::from_utf8_lossy(&output.stdout).contains("state_store_read_lock_contention")
            || String::from_utf8_lossy(&output.stderr).contains("state_store_read_lock_contention"))
}

fn vida() -> VidaCommand {
    VidaCommand {
        command: vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida")),
    }
}

fn unique_state_dir() -> String {
    static STATE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = STATE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "/tmp/vida-doctor-contract-state-{}-{nanos}-{counter}",
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
            "operator_surfaces:\n",
            "  taskflow:\n",
            "    graph_summary:\n",
            "      cache_policy:\n",
            "        mode: cache_first_with_authoritative_fallback\n",
            "        read_cache_before_authoritative_open: true\n",
            "        freshness_contract: projection_contract_version_and_state_marker\n",
            "        stale_projection_behavior: reject_and_recompute\n",
            "        refresh_flag_supported: false\n",
            "        authoritative_fallback: true\n",
            "        help_summary: \"Cache policy is cache-first with authoritative fallback; stale or mismatched projections are rejected and recomputed.\"\n",
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

fn run_git(project_root: &str, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {args:?} should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_BLOCKER: &str =
    "unsupported_architecture_reserved_workflow_boundary";
const UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION: &str = "clear unsupported/architecture-reserved workflow boundary state in run-graph policy/context before operator handoff.";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_BLOCKER: &str =
    "missing_run_graph_dispatch_receipt_operator_evidence";
const MISSING_RUN_GRAPH_DISPATCH_RECEIPT_OPERATOR_EVIDENCE_NEXT_ACTION: &str = "run `vida taskflow consume continue` to materialize or refresh run-graph dispatch receipt evidence before operator handoff.";

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

fn final_snapshot_for_run(
    snapshot_path: &str,
    run_id: &str,
    closure_admission: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida taskflow consume final",
        "source_run_id": run_id,
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
                "latest_run_graph_dispatch_receipt_id": run_id,
            }
        },
        "payload": {
            "closure_admission": closure_admission,
        },
        "artifact_refs": {
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": run_id,
        }
    })
}

fn seed_run_graph(state_dir: &str, run_id: &str) {
    let seed = vida()
        .args([
            "taskflow",
            "run-graph",
            "seed",
            run_id,
            "doctor current-projection test run",
            "--json",
        ])
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("run-graph seed should run");
    assert!(
        seed.status.success(),
        "run-graph seed should succeed: {}",
        String::from_utf8_lossy(&seed.stderr)
    );
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

    vida_test_support::assert_release1_operator_shape("vida doctor", &parsed);
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
        let action = action.as_str().unwrap_or_default();
        action.contains("recovery_ready=true") || action.contains("no validated run_id")
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
fn doctor_and_status_public_surface_matrix_preserves_default_and_json_contracts() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");

    for (surface, args) in [
        ("vida doctor", vec!["doctor", "--json"]),
        ("vida status", vec!["status", "--json"]),
    ] {
        let output = vida()
            .args(args)
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("json surface should run");
        assert_success(&output, surface);
        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("json surface output should parse");
        vida_test_support::assert_release1_operator_shape(surface, &payload);
    }

    for (surface, args) in [
        ("vida doctor", vec!["doctor"]),
        ("vida status", vec!["status"]),
    ] {
        let output = vida()
            .args(args)
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("default surface should run");
        assert_success(&output, surface);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_not_json_output(surface, &stdout);
        assert_no_raw_terminal_controls(surface, &stdout);
        assert!(
            stdout.contains("status"),
            "{surface} default output should expose operator status: {stdout}"
        );
    }
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
            action.as_str() == Some(UNSUPPORTED_ARCHITECTURE_RESERVED_WORKFLOW_BOUNDARY_NEXT_ACTION)
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

    let default_cases = [
        vida_test_support::CliOutputContractCase {
            surface: "vida status",
            args: &["status"],
            required_stdout: &["state dir:", "runtime consumption:"],
            forbidden_stdout: &["--json"],
        },
        vida_test_support::CliOutputContractCase {
            surface: "vida doctor",
            args: &["doctor"],
            required_stdout: &["storage metadata:", "runtime consumption:"],
            forbidden_stdout: &[],
        },
    ];
    vida_test_support::assert_cli_default_output_matrix(default_cases, |args| {
        vida()
            .args(args)
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("default command should run")
    });

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
    vida_test_support::assert_release1_operator_status_is_canonical("vida status", &status_payload);
    assert!(status_payload.get("operator_contracts").is_some());
    assert!(status_payload.get("blocker_codes").is_some());
    assert!(status_payload.get("next_actions").is_some());
    assert!(status_payload.get("artifact_refs").is_some());

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
    vida_test_support::assert_release1_operator_status_is_canonical("vida doctor", &doctor_payload);
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
    let cases = [
        vida_test_support::CliOutputContractCase {
            surface: "vida status",
            args: &["status", "--help"],
            required_stdout: &["--view", "--fields", "--json"],
            forbidden_stdout: &[],
        },
        vida_test_support::CliOutputContractCase {
            surface: "vida orchestrator-init",
            args: &["orchestrator-init", "--help"],
            required_stdout: &[
                "--view",
                "--fields",
                "--json",
                "active_step",
                "active_parent_task",
                "active_epic",
            ],
            forbidden_stdout: &[],
        },
    ];
    vida_test_support::assert_cli_help_output_matrix(cases, |args| {
        vida().args(args).output().expect("help command should run")
    });
}

#[test]
fn active_step_attribution_help_surfaces_are_discoverable() {
    let cases = [
        vida_test_support::CliOutputContractCase {
            surface: "vida doctor",
            args: &["doctor", "--help"],
            required_stdout: &[
                "active-task-attribution",
                "vida doctor active-task-attribution --json",
                "Field/view/detail selection",
                "status, blocker_codes, active_step, parent_task",
            ],
            forbidden_stdout: &[],
        },
        vida_test_support::CliOutputContractCase {
            surface: "vida task steps",
            args: &["task", "steps", "--help"],
            required_stdout: &[
                "active_step",
                "active_parent_task",
                "active_epic",
                "orchestrator-init",
            ],
            forbidden_stdout: &[],
        },
        vida_test_support::CliOutputContractCase {
            surface: "vida doctor active-task-attribution",
            args: &["doctor", "active-task-attribution", "--help"],
            required_stdout: &[
                "active_step",
                "active_parent_task",
                "active_epic",
                "orchestrator-init",
            ],
            forbidden_stdout: &[],
        },
    ];
    vida_test_support::assert_cli_help_output_matrix(cases, |args| {
        vida().args(args).output().expect("help command should run")
    });
}

#[test]
fn active_task_attribution_json_passes_without_contradiction() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_git(&project_root, &["init"]);
    run_git(
        &project_root,
        &["config", "user.email", "vida@example.invalid"],
    );
    run_git(&project_root, &["config", "user.name", "VIDA Test"]);
    std::fs::write(format!("{project_root}/.gitignore"), ".vida/\n").expect("write gitignore");
    std::fs::create_dir_all(format!("{project_root}/crates/vida/src"))
        .expect("create source fixture dir");
    std::fs::write(
        format!("{project_root}/crates/vida/src/doctor_surface.rs"),
        "old\n",
    )
    .expect("write owned fixture");
    run_git(&project_root, &["add", "."]);
    run_git(&project_root, &["commit", "-m", "baseline"]);
    std::fs::write(
        format!("{project_root}/crates/vida/src/doctor_surface.rs"),
        "new\n",
    )
    .expect("modify owned fixture");
    create_session_triage_task(
        &state_dir,
        "active-task-attribution-epic",
        "Active task attribution epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "active-task-attribution-parent",
        "Active task attribution parent",
        "task",
        "in_progress",
        "1",
        Some("active-task-attribution-epic"),
    );
    let step = vida()
        .args([
            "task",
            "create",
            "active-task-attribution-step",
            "Active task attribution step",
            "--type",
            "step",
            "--status",
            "in_progress",
            "--parent-id",
            "active-task-attribution-parent",
            "--owned-path",
            "crates/vida/src/doctor_surface.rs",
            "--owned-path",
            "crates/vida/src/cli.rs",
            "--owned-path",
            "crates/vida/src/task_surface.rs",
            "--owned-path",
            "crates/vida/tests/doctor_surface_contract_smoke.rs",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("active attribution step create should run");
    assert_success(&step, "active attribution step create");

    let output = vida()
        .args(["doctor", "active-task-attribution", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("active attribution doctor should run");
    assert_success(&output, "active attribution doctor pass");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("active attribution json should parse");
    assert_eq!(payload["status"], "pass");
    assert_eq!(payload["blocker_codes"], serde_json::json!([]));
    assert_eq!(
        payload["active_step"]["task_id"],
        "active-task-attribution-step"
    );
    assert_eq!(
        payload["parent_task"]["task_id"],
        "active-task-attribution-parent"
    );
    assert!(payload.get("orchestrator_projection").is_some());
    assert_eq!(payload["dirty_summary"]["status"], "pass");
    assert!(payload["next_actions"].is_array());
}

#[test]
fn active_task_attribution_json_blocks_dirty_owner_contradiction() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_git(&project_root, &["init"]);
    run_git(
        &project_root,
        &["config", "user.email", "vida@example.invalid"],
    );
    run_git(&project_root, &["config", "user.name", "VIDA Test"]);
    std::fs::write(format!("{project_root}/.gitignore"), ".vida/\n").expect("write gitignore");
    std::fs::create_dir_all(format!("{project_root}/crates/vida/src"))
        .expect("create source fixture dir");
    std::fs::write(
        format!("{project_root}/crates/vida/src/doctor_surface.rs"),
        "old\n",
    )
    .expect("write owned fixture");
    std::fs::write(format!("{project_root}/README.md"), "old\n").expect("write readme fixture");
    run_git(&project_root, &["add", "."]);
    run_git(&project_root, &["commit", "-m", "baseline"]);
    std::fs::write(
        format!("{project_root}/crates/vida/src/doctor_surface.rs"),
        "new\n",
    )
    .expect("modify owned fixture");
    std::fs::write(format!("{project_root}/README.md"), "new\n").expect("modify unowned fixture");
    create_session_triage_task(
        &state_dir,
        "active-task-attribution-dirty-epic",
        "Active task attribution dirty epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "active-task-attribution-dirty-parent",
        "Active task attribution dirty parent",
        "task",
        "in_progress",
        "1",
        Some("active-task-attribution-dirty-epic"),
    );
    let step = vida()
        .args([
            "task",
            "create",
            "active-task-attribution-dirty-step",
            "Active task attribution dirty step",
            "--type",
            "step",
            "--status",
            "in_progress",
            "--parent-id",
            "active-task-attribution-dirty-parent",
            "--owned-path",
            "crates/vida/src/doctor_surface.rs",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("active attribution dirty step create should run");
    assert_success(&step, "active attribution dirty step create");

    let output = vida()
        .args(["doctor", "active-task-attribution", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("active attribution dirty doctor should run");
    assert_failure(&output, "active attribution dirty doctor blocked");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("active attribution json should parse");
    assert_eq!(payload["status"], "blocked");
    assert!(payload["blocker_codes"]
        .as_array()
        .expect("blocker codes should be array")
        .iter()
        .any(|code| code == "dirty_ownership_ambiguous"));
    assert_eq!(
        payload["dirty_summary"]["unmatched_files"],
        serde_json::json!(["README.md"])
    );
    assert_eq!(
        payload["active_step"]["task_id"],
        "active-task-attribution-dirty-step"
    );
}

#[test]
fn active_task_attribution_json_ignores_tampered_task_snapshot() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_git(&project_root, &["init"]);
    run_git(
        &project_root,
        &["config", "user.email", "vida@example.invalid"],
    );
    run_git(&project_root, &["config", "user.name", "VIDA Test"]);
    std::fs::write(format!("{project_root}/.gitignore"), ".vida/\n").expect("write gitignore");
    std::fs::write(format!("{project_root}/README.md"), "old\n").expect("write readme fixture");
    run_git(&project_root, &["add", "."]);
    run_git(&project_root, &["commit", "-m", "baseline"]);
    std::fs::write(format!("{project_root}/README.md"), "new\n").expect("modify unowned fixture");
    create_session_triage_task(
        &state_dir,
        "active-task-attribution-tampered-epic",
        "Active task attribution tampered epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "active-task-attribution-tampered-parent",
        "Active task attribution tampered parent",
        "task",
        "in_progress",
        "1",
        Some("active-task-attribution-tampered-epic"),
    );
    let step = vida()
        .args([
            "task",
            "create",
            "active-task-attribution-tampered-step",
            "Active task attribution tampered step",
            "--type",
            "step",
            "--status",
            "in_progress",
            "--parent-id",
            "active-task-attribution-tampered-parent",
            "--owned-path",
            "crates/vida/src/doctor_surface.rs",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("active attribution tampered step create should run");
    assert_success(&step, "active attribution tampered step create");

    let snapshot_path = format!("{project_root}/.vida/exports/tasks.snapshot.jsonl");
    let snapshot = std::fs::read_to_string(&snapshot_path).expect("task snapshot should exist");
    let tampered = snapshot
        .lines()
        .map(|line| {
            let mut row: serde_json::Value =
                serde_json::from_str(line).expect("snapshot row should parse");
            if row["id"] == "active-task-attribution-tampered-parent"
                || row["id"] == "active-task-attribution-tampered-step"
            {
                row["planner_metadata"]["owned_paths"] = serde_json::json!(["README.md", ".vida"]);
            }
            row.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&snapshot_path, format!("{tampered}\n")).expect("tamper task snapshot");

    let output = vida()
        .args(["doctor", "active-task-attribution", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("active attribution tampered doctor should run");
    assert_failure(
        &output,
        "active attribution doctor should ignore tampered snapshot and block",
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("active attribution json should parse");
    assert_eq!(payload["status"], "blocked");
    assert!(payload["blocker_codes"]
        .as_array()
        .expect("blocker codes should be array")
        .iter()
        .any(|code| code == "dirty_ownership_ambiguous"));
    assert_eq!(
        payload["dirty_summary"]["unmatched_files"],
        serde_json::json!(["README.md"])
    );
}

#[test]
fn task_steps_outputs_default_toon_json_and_filters() {
    let state_dir = unique_state_dir();
    create_session_triage_task(
        &state_dir,
        "task-steps-epic",
        "Task steps epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "task-steps-parent-a",
        "Task steps parent A",
        "task",
        "in_progress",
        "1",
        Some("task-steps-epic"),
    );
    create_session_triage_task(
        &state_dir,
        "task-steps-parent-b",
        "Task steps parent B",
        "task",
        "open",
        "2",
        Some("task-steps-epic"),
    );

    let step_a = vida()
        .args([
            "task",
            "create",
            "task-steps-step-a",
            "Task steps step A",
            "--type",
            "step",
            "--status",
            "in_progress",
            "--parent-id",
            "task-steps-parent-a",
            "--owned-path",
            "crates/vida/src/task_surface.rs",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task step a create should run");
    assert_success(&step_a, "task step a create");

    let step_b = vida()
        .args([
            "task",
            "create",
            "task-steps-step-b",
            "Task steps step B",
            "--type",
            "step",
            "--status",
            "closed",
            "--parent-id",
            "task-steps-parent-b",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task step b create should run");
    assert_success(&step_b, "task step b create");

    let default_output = vida()
        .args(["task", "steps", "--since", "3h", "--with-parent"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task steps default output should run");
    assert_success(&default_output, "task steps default");
    let default_stdout = String::from_utf8_lossy(&default_output.stdout);
    assert_not_json_output("vida task steps", &default_stdout);
    assert!(default_stdout.contains(
        "task_steps[2]{id,status,parent_id,parent_title,created,closed,close_reason,owned_paths}:"
    ));
    assert!(default_stdout.contains("task-steps-step-a"));
    assert!(default_stdout.contains("Task steps parent A"));

    let json_output = vida()
        .args([
            "task",
            "steps",
            "--since",
            "3h",
            "--with-parent",
            "--parent-id",
            "task-steps-parent-a",
            "--status",
            "in_progress",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task steps json output should run");
    assert_success(&json_output, "task steps json");
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("task steps json should parse");
    assert_eq!(payload["surface"], "vida task steps");
    assert_eq!(payload["status"], "success");
    assert_eq!(payload["count"], 1);
    let row = &payload["steps"][0];
    assert_eq!(row["id"], "task-steps-step-a");
    assert_eq!(row["status"], "in_progress");
    assert_eq!(row["parent_id"], "task-steps-parent-a");
    assert_eq!(row["parent_title"], "Task steps parent A");
    assert!(row["created"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(row["closed"].is_null());
    assert!(row["close_reason"].is_null());
    assert_eq!(
        row["owned_paths"],
        serde_json::json!(["crates/vida/src/task_surface.rs"])
    );
}

#[test]
fn task_steps_rejects_oversized_since_filter() {
    let state_dir = unique_state_dir();

    let output = vida()
        .args(["task", "steps", "--since", "9223372036854775808s", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task steps oversized since output should run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "oversized --since should be rejected without panic: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("oversized task steps json should parse");
    assert_eq!(payload["surface"], "vida task steps");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["invalid_since_filter"])
    );
    assert!(payload["error"]
        .as_str()
        .is_some_and(|error| error.contains("too large")));
}

#[test]
fn owned_status_from_dirty_with_active_step_maps_taskflow_owners() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_git(&project_root, &["init"]);
    run_git(
        &project_root,
        &["config", "user.email", "vida@example.invalid"],
    );
    run_git(&project_root, &["config", "user.name", "VIDA Test"]);
    std::fs::write(format!("{project_root}/.gitignore"), ".vida/\n").expect("write gitignore");
    std::fs::create_dir_all(format!("{project_root}/crates/vida/src"))
        .expect("create source fixture dir");
    std::fs::write(
        format!("{project_root}/crates/vida/src/task_surface.rs"),
        "old\n",
    )
    .expect("write owned fixture");
    std::fs::write(format!("{project_root}/README.md"), "old\n").expect("write readme fixture");
    run_git(
        &project_root,
        &[
            "add",
            ".gitignore",
            "AGENTS.md",
            "vida.config.yaml",
            "crates",
            "README.md",
        ],
    );
    run_git(&project_root, &["commit", "-m", "baseline"]);
    std::fs::write(
        format!("{project_root}/crates/vida/src/task_surface.rs"),
        "new\n",
    )
    .expect("modify owned fixture");
    std::fs::write(format!("{project_root}/README.md"), "new\n").expect("modify unowned fixture");

    create_session_triage_task(
        &state_dir,
        "owned-status-epic",
        "Owned status epic",
        "epic",
        "open",
        "0",
        None,
    );
    let parent = vida()
        .args([
            "task",
            "create",
            "owned-status-parent",
            "Owned status parent",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            "owned-status-epic",
            "--owned-path",
            "crates/vida/src/task_surface.rs",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("owned-status parent create should run");
    assert_success(&parent, "owned-status parent create");
    let step = vida()
        .args([
            "task",
            "create",
            "owned-status-step",
            "Owned status step",
            "--type",
            "step",
            "--status",
            "in_progress",
            "--parent-id",
            "owned-status-parent",
            "--owned-path",
            "crates/vida/src/task_surface.rs",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("owned-status step create should run");
    assert_success(&step, "owned-status step create");

    let output = vida()
        .args([
            "task",
            "owned-status",
            "--from-dirty",
            "--with-active-step",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("owned-status dirty attribution should run");
    assert_failure(
        &output,
        "owned-status dirty attribution with unmatched file",
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("owned-status json should parse");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(payload["task_id"], "owned-status-parent");
    assert_eq!(payload["active_step"]["task_id"], "owned-status-step");
    assert_eq!(
        payload["active_parent_task"]["task_id"],
        "owned-status-parent"
    );
    assert_eq!(payload["active_epic"]["task_id"], "owned-status-epic");
    assert_eq!(
        payload["owned_paths"],
        serde_json::json!(["crates/vida/src/task_surface.rs"])
    );
    assert_eq!(
        payload["matched_files"],
        serde_json::json!(["crates/vida/src/task_surface.rs"])
    );
    assert_eq!(payload["unmatched_files"], serde_json::json!(["README.md"]));
    assert_eq!(payload["unowned_paths"], serde_json::json!(["README.md"]));
    assert_eq!(payload["confidence"], "mixed");
    assert!(payload["next_actions"]
        .as_array()
        .expect("next_actions should be array")
        .iter()
        .any(|action| action
            .as_str()
            .is_some_and(|text| text.contains("unrelated dirty files"))));

    let file_override_output = vida()
        .args([
            "task",
            "owned-status",
            "owned-status-parent",
            "--from-dirty",
            "--json",
            "--file",
            "classify-dirty",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("owned-status file override named classify-dirty should run");
    assert_failure(
        &file_override_output,
        "owned-status file override named classify-dirty should not route to classifier",
    );
    let file_override_payload: serde_json::Value =
        serde_json::from_slice(&file_override_output.stdout)
            .expect("owned-status file override json should parse");
    assert_eq!(file_override_payload["status"], "blocked");
    assert_eq!(file_override_payload["task_id"], "owned-status-parent");
    assert!(
        file_override_payload.get("groups").is_none(),
        "owned-status output must not be classify-dirty receipt"
    );

    let task_id_output = vida()
        .args([
            "task",
            "owned-status",
            "classify-dirty",
            "--from-dirty",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("owned-status task id named classify-dirty should run");
    assert_failure(
        &task_id_output,
        "owned-status task id named classify-dirty should not route to classifier",
    );
    let task_id_payload: serde_json::Value = serde_json::from_slice(&task_id_output.stdout)
        .expect("owned-status task id json should parse");
    assert_eq!(task_id_payload["status"], "blocked");
    assert_eq!(task_id_payload["task_id"], "classify-dirty");
    assert!(
        task_id_payload.get("groups").is_none(),
        "owned-status task-id output must not be classify-dirty receipt"
    );
}

#[test]
fn classify_dirty_groups_owned_paths_and_reports_unclassified() {
    let (project_root, state_dir) = project_bound_state_dir();
    run_git(&project_root, &["init"]);
    run_git(
        &project_root,
        &["config", "user.email", "vida@example.invalid"],
    );
    run_git(&project_root, &["config", "user.name", "VIDA Test"]);
    std::fs::write(format!("{project_root}/.gitignore"), ".vida/\n").expect("write gitignore");
    std::fs::create_dir_all(format!("{project_root}/crates/vida/src"))
        .expect("create source fixture dir");
    std::fs::write(
        format!("{project_root}/crates/vida/src/task_surface.rs"),
        "old\n",
    )
    .expect("write owned fixture");
    std::fs::write(format!("{project_root}/README.md"), "old\n").expect("write readme fixture");
    run_git(
        &project_root,
        &[
            "add",
            ".gitignore",
            "AGENTS.md",
            "vida.config.yaml",
            "crates",
            "README.md",
        ],
    );
    run_git(&project_root, &["commit", "-m", "baseline"]);
    std::fs::write(
        format!("{project_root}/crates/vida/src/task_surface.rs"),
        "new\n",
    )
    .expect("modify owned fixture");
    std::fs::write(format!("{project_root}/README.md"), "new\n")
        .expect("modify unclassified fixture");

    create_session_triage_task(
        &state_dir,
        "classify-dirty-epic",
        "Classify dirty epic",
        "epic",
        "open",
        "0",
        None,
    );
    let task = vida()
        .args([
            "task",
            "create",
            "classify-dirty-task",
            "Classify dirty task",
            "--type",
            "task",
            "--status",
            "in_progress",
            "--priority",
            "1",
            "--parent-id",
            "classify-dirty-epic",
            "--owned-path",
            "crates/vida/src/task_surface.rs",
            "--proof-target",
            "cargo test classify_dirty",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("classify dirty task create should run");
    assert_success(&task, "classify dirty task create");

    let output = vida()
        .args(["task", "classify-dirty", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&project_root)
        .output()
        .expect("classify-dirty json should run");
    assert_failure(&output, "classify-dirty json with unclassified files");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("classify-dirty json should parse");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(payload["groups"][0]["task_id"], "classify-dirty-task");
    assert_eq!(payload["groups"][0]["epic_id"], "classify-dirty-epic");
    assert_eq!(
        payload["groups"][0]["files"],
        serde_json::json!(["crates/vida/src/task_surface.rs"])
    );
    assert_eq!(payload["groups"][0]["confidence"], "high");
    assert!(payload["groups"][0]["reasons"]
        .as_array()
        .expect("reasons should be array")
        .iter()
        .any(|reason| reason
            .as_str()
            .is_some_and(|text| text.contains("proof targets"))));
    assert_eq!(payload["unclassified"], serde_json::json!(["README.md"]));
    assert!(payload["next_actions"]
        .as_array()
        .expect("next_actions should be array")
        .iter()
        .any(|action| action
            .as_str()
            .is_some_and(|text| text.contains("unclassified files"))));
}

#[test]
fn taskflow_route_topic_help_documents_run_id_for_route_surfaces() {
    let route_help = vida()
        .args(["taskflow", "help", "route"])
        .output()
        .expect("taskflow route topic help should run");
    assert!(
        route_help.status.success(),
        "taskflow route topic help should succeed: stderr={}",
        String::from_utf8_lossy(&route_help.stderr)
    );
    let route_stdout = String::from_utf8_lossy(&route_help.stdout);
    for expected in [
        "vida taskflow route explain [--json]",
        "vida taskflow route explain [--run-id <run-id>] [--json]",
        "vida route explain [--run-id <run-id>] [--json]",
        "vida taskflow validate-routing [--run-id <run-id>] [--json]",
    ] {
        assert!(
            route_stdout.contains(expected),
            "route topic help should document {expected}: {route_stdout}"
        );
    }

    let validate_help = vida()
        .args(["taskflow", "help", "validate-routing"])
        .output()
        .expect("taskflow validate-routing topic help should run");
    assert!(
        validate_help.status.success(),
        "taskflow validate-routing topic help should succeed: stderr={}",
        String::from_utf8_lossy(&validate_help.stderr)
    );
    let validate_stdout = String::from_utf8_lossy(&validate_help.stdout);
    for expected in [
        "vida taskflow validate-routing [--run-id <run-id>] [--json]",
        "vida taskflow route explain [--run-id <run-id>] [--json]",
        "vida taskflow config-actuation census [--run-id <run-id>] [--json]",
    ] {
        assert!(
            validate_stdout.contains(expected),
            "validate-routing topic help should document {expected}: {validate_stdout}"
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
                "definition_of_done": [
                    "coach preview includes synthesized request text"
                ],
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
fn agent_init_execute_dispatch_autotester_packet_materializes_worker_test_scope_request() {
    let state_dir = unique_state_dir();
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .current_dir(&workspace_root)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");

    let packet_dir = format!("{state_dir}/runtime-consumption/downstream-dispatch-packets");
    std::fs::create_dir_all(&packet_dir).expect("downstream packet dir should exist");
    let packet_path = format!("{packet_dir}/autotester-worker-scope.json");
    let owned_paths = serde_json::json!(["lib/src/features/activity/record_detail_view.dart"]);
    let compiled_bundle = serde_json::json!({
        "agent_system": {
            "routing": {
                "default": {
                    "executor_backend": "internal_subagents"
                }
            }
        },
        "dev_team_readiness": {
            "roles": [
                {"role_id": "designer", "runtime_role": "designer", "task_classes": ["design"]},
                {"role_id": "autotester", "runtime_role": "worker", "task_classes": ["implementation_medium"]},
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "flows": [
                {
                    "flow_id": "configured_autotester_flow",
                    "enabled": true,
                    "ordered_steps": [
                        {"role_id": "designer"},
                        {"role_id": "autotester"},
                        {"role_id": "developer"}
                    ]
                }
            ]
        },
        "carrier_runtime": {
            "model_selection": {
                "enabled": true,
                "candidate_scope": "unified_carrier_model_profiles",
                "default_strategy": "balanced_cost_quality"
            },
            "roles": [
                {
                    "role_id": "middle",
                    "tier": "middle",
                    "rate": 4,
                    "normalized_cost_units": 4,
                    "default_runtime_role": "worker",
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation_medium"],
                    "reasoning_band": "medium",
                    "default_model_profile": "codex_gpt55_medium_write",
                    "model_profiles": {
                        "codex_gpt55_medium_write": {
                            "profile_id": "codex_gpt55_medium_write",
                            "model_ref": "gpt-5.5",
                            "provider": "openai",
                            "reasoning_effort": "medium",
                            "plan_mode_reasoning_effort": "high",
                            "sandbox_mode": "workspace-write",
                            "normalized_cost_units": 4,
                            "speed_tier": "fast",
                            "quality_tier": "medium",
                            "write_scope": "workspace-write",
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation_medium"],
                            "readiness": { "required": true, "ready": true }
                        }
                    }
                }
            ]
        }
    });
    let role_selection_full = serde_json::json!({
        "ok": true,
        "activation_source": "packet",
        "selection_mode": "fixed",
        "fallback_role": "orchestrator",
        "request": "Use meeting-specific event fields when scheduling Meeting activities",
        "selected_role": "worker",
        "conversational_mode": null,
        "single_task_only": false,
        "tracked_flow_entry": "dev-pack",
        "allow_freeform_chat": false,
        "confidence": "high",
        "matched_terms": ["dev_team_flow_id:configured_autotester_flow"],
        "compiled_bundle": compiled_bundle,
        "execution_plan": {
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "activation_runtime_role": "worker",
                "runtime_role": "worker",
                "task_class": "implementation_medium",
                "selected_backend_id": "internal_subagents"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true,
                        "autotester": true
                    }
                }
            ]
        },
        "reason": "test"
    });
    let delivery_task_packet = serde_json::json!({
        "task_id": "activity-meeting-event-form-fields",
        "backlog_id": "activity-meeting-event-form-fields",
        "goal": "author autotester coverage",
        "scope_in": ["autotester coverage"],
        "owned_paths": owned_paths,
        "read_only_paths": ["lib/src/features/activity/record_detail_view.dart"],
        "definition_of_done": ["host bridge request carries autotester lane contract"],
        "verification_command": "vida agent-init --downstream-packet autotester-worker-scope.json --execute-dispatch --json",
        "proof_target": "autotester host bridge request",
        "stop_rules": ["stop if request contract is wrong"],
        "blocking_question": "Does autotester request use worker implementation scope?",
        "handoff_runtime_role": "worker",
        "handoff_task_class": "implementation_medium",
        "implementation_isolation": {
            "canonical_worktree_writes_allowed": false,
            "owned_paths": owned_paths
        }
    });
    let packet_body = serde_json::json!({
        "packet_kind": "runtime_downstream_dispatch_packet",
        "run_id": "activity-meeting-event-form-fields",
        "task_id": "activity-meeting-event-form-fields",
        "dispatch_target": "autotester",
        "downstream_dispatch_target": "autotester",
        "downstream_dispatch_ready": true,
        "downstream_dispatch_blockers": [],
        "activation_agent_type": "middle",
        "activation_runtime_role": "worker",
        "runtime_role": "worker",
        "task_class": "implementation_medium",
        "handoff_runtime_role": "worker",
        "handoff_task_class": "implementation_medium",
        "selected_backend": "internal_subagents",
        "packet_template_kind": "delivery_task_packet",
        "prompt": "autotester proof",
        "owned_paths": owned_paths,
        "read_only_paths": ["lib/src/features/activity/record_detail_view.dart"],
        "implementation_isolation": {
            "canonical_worktree_writes_allowed": false,
            "owned_paths": owned_paths
        },
        "role_selection_full": role_selection_full,
        "delivery_task_packet": delivery_task_packet
    });
    std::fs::write(&packet_path, packet_body.to_string())
        .expect("autotester downstream packet should be written");

    let output = vida()
        .args([
            "agent-init",
            "--downstream-packet",
            &packet_path,
            "--execute-dispatch",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_AGENT_INIT_EXECUTE_DISPATCH_WORKER", "1")
        .current_dir(&workspace_root)
        .output()
        .expect("agent-init execute-dispatch should run");
    assert_success(&output, "agent-init execute-dispatch");

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("agent-init execute json should parse");
    assert_eq!(payload["surface"], "vida agent-init");
    assert!(
        matches!(payload["status"].as_str(), Some("pass" | "blocked")),
        "agent-init should reach dispatch materialization or an execution-evidence blocker: {payload}"
    );
    assert_eq!(payload["dispatch_target"], "autotester");
    match payload["status"].as_str() {
        Some("pass") => {
            assert_eq!(
                payload["execution_evidence"]["receipt_backed"], true,
                "pass execute-dispatch result must carry receipt-backed execution evidence: {payload}"
            );
        }
        Some("blocked") => {
            assert_eq!(
                payload["execution_state"], "bridge_request_pending",
                "blocked internal_subagents execute-dispatch should materialize a bridge request instead of an activation view: {payload}"
            );
            assert_eq!(
                payload["blocker_code"], "host_tool_bridge_adapter_required",
                "blocked execute-dispatch should expose the host bridge blocker: {payload}"
            );
            assert!(
                payload["host_tool_bridge_request"]["request_path"]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty()),
                "blocked execute-dispatch should expose the bridge request path: {payload}"
            );
            assert!(
                payload["next_actions"]
                    .as_array()
                    .is_some_and(|actions| !actions.is_empty()),
                "blocked execute-dispatch should expose actionable next actions: {payload}"
            );
        }
        other => panic!("unexpected execute-dispatch status {other:?}: {payload}"),
    }

    let request_dir = format!("{state_dir}/host-tool-bridge/requests");
    let request_path = std::fs::read_dir(&request_dir)
        .expect("host bridge request dir should exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .expect("host bridge request json should be materialized");
    let request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&request_path).expect("host bridge request should be readable"),
    )
    .expect("host bridge request json should parse");
    assert_eq!(request["dispatch_target"], "autotester");
    assert_eq!(request["runtime_role"], "worker");
    assert_eq!(request["task_class"], "implementation_medium");
    assert_eq!(
        request["implementation_isolation"]["canonical_worktree_writes_allowed"],
        false
    );
    let request_owned_paths = request["owned_paths"]
        .as_array()
        .expect("host bridge request should carry owned paths");
    assert!(
        request_owned_paths
            .iter()
            .any(|path| path == "test" || path == "tests"),
        "autotester request should include test write scope: {request_owned_paths:?}"
    );
    let isolation_owned_paths = request["implementation_isolation"]["owned_paths"]
        .as_array()
        .expect("implementation isolation should carry owned paths");
    assert!(
        isolation_owned_paths
            .iter()
            .any(|path| path == "test" || path == "tests"),
        "autotester isolation should include test write scope: {isolation_owned_paths:?}"
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
        elapsed < Duration::from_secs(5),
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

#[test]
fn agent_host_bridge_json_retains_sanitized_lock_open_diagnostic() {
    let state_dir = unique_state_dir();
    let request_dir = format!("{state_dir}/host-tool-bridge/requests");
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    std::fs::create_dir_all(&request_dir).expect("host bridge request dir should exist");
    std::fs::create_dir_all(&packet_dir).expect("dispatch packet dir should exist");
    let request_path = format!("{request_dir}/request.json");
    let packet_path = format!("{packet_dir}/packet.json");
    let result_path = format!("{state_dir}/host-tool-bridge/results/result.json");
    let receipt_path = format!("{state_dir}/host-tool-bridge/receipts/receipt.json");
    std::fs::create_dir_all(
        std::path::Path::new(&result_path)
            .parent()
            .expect("result parent should exist"),
    )
    .expect("result dir should exist");
    std::fs::create_dir_all(
        std::path::Path::new(&receipt_path)
            .parent()
            .expect("receipt parent should exist"),
    )
    .expect("receipt dir should exist");
    std::fs::write(&packet_path, "{}").expect("dispatch packet should exist");
    let request_base = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-lock-diagnostic",
            "run_id": "run-lock-diagnostic",
            "task_id": "task-lock-diagnostic",
            "attempt_id": "attempt-lock-diagnostic",
            "packet_id": "packet-lock-diagnostic",
            "dispatch_target": "implementer",
            "packet_path": packet_path,
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "request_path": request_path,
            "result_path": result_path,
            "receipt_path": receipt_path
        });
    let registry = serde_json::json!({
        "adapter_kind": "codex_host_tools",
        "adapter_capability_id": "codex.multi_agent_v1",
        "invocation_mode": "parent_host_tool_api",
        "dispatch_transport": "host_tool_bridge",
        "receipt_mode": "host_bridge_receipt",
        "operations": {
            "spawn": "multi_agent_v1.spawn_agent",
            "wait": "multi_agent_v1.wait_agent",
            "dispose": "multi_agent_v1.close_agent"
        },
        "dispose_policy": "configured"
    });
    let mut request = effective_host_bridge_request_with_registry(&request_base, &registry)
        .expect("configured registry should materialize current adapter contract");
    let snapshot = request["adapter_operations"].clone();
    request["adapter_contract_snapshot"] = snapshot.clone();
    request["adapter_contract_hash"] = serde_json::Value::String(
        blake3::hash(&serde_json::to_vec(&snapshot).expect("snapshot should serialize"))
            .to_hex()
            .to_string(),
    );
    std::fs::write(&request_path, request.to_string())
    .expect("host bridge request should exist");

    let lock_path = std::path::Path::new(&state_dir).join("LOCK");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("state lock should open");
    std::fs::write(&lock_path, "999999").expect("state lock marker should be written");
    lock_file
        .lock_exclusive()
        .expect("test should hold the state lock");

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
        .expect("host bridge JSON should run");
    assert!(!output.status.success(), "held state lock must fail closed");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge JSON should parse");
    assert_eq!(payload["status"], "blocked");
    assert!(
        payload["blocker_codes"]
            .as_array()
            .expect("blocker codes")
            .iter()
            .any(|code| code == "authoritative_state_store_locked"),
        "unexpected host bridge lock payload: {payload}"
    );
    assert_eq!(payload["state_access"]["error_kind"], "lock_contention");
    assert_eq!(payload["state_access"]["retryable"], true);
    assert_eq!(
        payload["state_access"]["blocker_code"],
        "authoritative_state_store_locked"
    );
    assert_eq!(payload["state_access"]["open_stage"], "datastore_open");
    assert_eq!(payload["state_access"]["lock_evidence"], "datastore");
    assert!(payload.get("error").is_none());
    assert!(payload["state_access"].to_string().find(&state_dir).is_none());
    assert!(payload["state_access"].to_string().find("999999").is_none());
    assert!(payload["state_access"].to_string().find("LOCK").is_none());

    let _ = lock_file.unlock();
    let _ = std::fs::remove_dir_all(&state_dir);
}

#[derive(Debug)]
struct HostBridgeLaneFixture {
    state_dir: String,
    run_id: String,
    request_path: String,
    packet_path: String,
    result_path: String,
    bridge_receipt_path: String,
}

fn persist_host_bridge_lane_receipt_with_helper(
    state_dir: &str,
    run_id: &str,
    dispatch_packet_path: &str,
    downstream_packet_path: &str,
    activation_result_path: &str,
    dispatch_target: &str,
) {
    let (downstream_target, task_class) = match dispatch_target {
        "coach" => ("tester", "coach"),
        "tester" => ("reviewer", "verification"),
        "reviewer" => ("release_closure", "review"),
        _ => ("developer", "implementation"),
    };
    let lifecycle_stage = format!("{dispatch_target}_blocked");
    let resume_target = format!("dispatch.{dispatch_target}");
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
            dispatch_target,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DISPATCH_PACKET_PATH_ENV,
            dispatch_packet_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_TARGET_ENV,
            downstream_target,
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
            task_class,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_LIFECYCLE_STAGE_ENV,
            &lifecycle_stage,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_HANDOFF_STATE_ENV,
            "none",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_RESUME_TARGET_ENV,
            &resume_target,
        )
        .output()
        .expect("runtime receipt helper process should run");
    assert_success(&output, "runtime receipt helper process");
}

fn persist_host_bridge_lane_receipt_with_target(
    state_dir: &str,
    run_id: &str,
    dispatch_target: &str,
    downstream_target: &str,
    dispatch_status: &str,
    lane_status: &str,
    blocker_code: &str,
    lifecycle_stage: &str,
) {
    persist_host_bridge_lane_receipt_with_target_and_active_node(
        state_dir,
        run_id,
        dispatch_target,
        dispatch_target,
        downstream_target,
        dispatch_status,
        lane_status,
        blocker_code,
        lifecycle_stage,
    );
}

#[allow(clippy::too_many_arguments)]
fn persist_host_bridge_lane_receipt_with_target_and_active_node(
    state_dir: &str,
    run_id: &str,
    dispatch_target: &str,
    active_node: &str,
    downstream_target: &str,
    dispatch_status: &str,
    lane_status: &str,
    blocker_code: &str,
    lifecycle_stage: &str,
) {
    persist_host_bridge_lane_receipt_with_target_and_active_node_and_downstream_state(
        state_dir,
        run_id,
        dispatch_target,
        active_node,
        downstream_target,
        dispatch_status,
        lane_status,
        blocker_code,
        lifecycle_stage,
        "false",
        "blocked",
    );
}

#[allow(clippy::too_many_arguments)]
fn persist_host_bridge_lane_receipt_with_target_and_active_node_and_downstream_state(
    state_dir: &str,
    run_id: &str,
    dispatch_target: &str,
    active_node: &str,
    downstream_target: &str,
    dispatch_status: &str,
    lane_status: &str,
    blocker_code: &str,
    lifecycle_stage: &str,
    downstream_ready: &str,
    downstream_status: &str,
) {
    let project_root = format!("{state_dir}/../../..");
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let downstream_packet_dir =
        format!("{state_dir}/runtime-consumption/downstream-dispatch-packets");
    let result_dir = format!("{state_dir}/runtime-consumption/dispatch-results");
    std::fs::create_dir_all(&packet_dir).expect("dispatch packet dir should exist");
    std::fs::create_dir_all(&downstream_packet_dir)
        .expect("downstream dispatch packet dir should exist");
    std::fs::create_dir_all(&result_dir).expect("dispatch result dir should exist");
    let dispatch_packet_path = format!("{packet_dir}/{run_id}-{dispatch_target}.json");
    let downstream_packet_path =
        format!("{downstream_packet_dir}/{run_id}-{downstream_target}.json");
    let result_path = format!("{result_dir}/{run_id}-{dispatch_target}.json");
    std::fs::write(
        &dispatch_packet_path,
        serde_json::json!({
            "run_id": run_id,
            "dispatch_target": dispatch_target,
            "packet_template_kind": "verifier_proof_packet",
            "proof_goal": "Complete host bridge proof.",
            "verification_command": "cargo test -p vida host_bridge_public_cli",
            "proof_target": "host bridge completion receipt",
            "read_only_paths": ["crates/vida/src"],
            "blocking_question": "none",
            "role_selection_full": {
                "execution_plan": {
                    "development_flow": {
                        "dispatch_contract": {
                            "execution_lane_sequence": [
                                "developer",
                                "developer_rework",
                                "coach",
                                "tester",
                                "reviewer",
                                "release_closure"
                            ],
                            "lane_catalog": {
                                "developer": {
                                    "dispatch_target": "developer",
                                    "task_class": "implementation",
                                    "stage": "execution",
                                    "runtime_role": "worker"
                                },
                                "developer_rework": {
                                    "dispatch_target": "developer_rework",
                                    "task_class": "implementation",
                                    "stage": "execution",
                                    "runtime_role": "worker"
                                },
                                "coach": {
                                    "dispatch_target": "coach",
                                    "task_class": "coach",
                                    "stage": "quality_gate",
                                    "runtime_role": "coach"
                                },
                                "tester": {
                                    "dispatch_target": "tester",
                                    "task_class": "verification",
                                    "stage": "verification",
                                    "runtime_role": "verifier"
                                },
                                "reviewer": {
                                    "dispatch_target": "reviewer",
                                    "task_class": "review",
                                    "stage": "review",
                                    "runtime_role": "verifier"
                                },
                                "release_closure": {
                                    "dispatch_target": "release_closure",
                                    "task_class": "release_readiness",
                                    "stage": "release_readiness",
                                    "runtime_role": "release"
                                }
                            }
                        }
                    }
                }
            },
            "verifier_proof_packet": {
                "proof_goal": "Complete host bridge proof.",
                "verification_command": "cargo test -p vida host_bridge_public_cli",
                "proof_target": "host bridge completion receipt",
                "read_only_paths": ["crates/vida/src"],
                "blocking_question": "none"
            }
        })
        .to_string(),
    )
    .expect("dispatch packet should write");
    std::fs::write(
        &downstream_packet_path,
        serde_json::json!({
            "run_id": run_id,
            "dispatch_target": downstream_target,
            "packet_template_kind": "verifier_proof_packet",
            "proof_goal": "Continue downstream host bridge proof.",
            "verification_command": "cargo test -p vida host_bridge_public_cli",
            "proof_target": "host bridge downstream packet",
            "read_only_paths": ["crates/vida/src"],
            "blocking_question": "none",
            "verifier_proof_packet": {
                "proof_goal": "Continue downstream host bridge proof.",
                "verification_command": "cargo test -p vida host_bridge_public_cli",
                "proof_target": "host bridge downstream packet",
                "read_only_paths": ["crates/vida/src"],
                "blocking_question": "none"
            }
        })
        .to_string(),
    )
    .expect("downstream packet should write");
    std::fs::write(&result_path, "{}").expect("dispatch result should write");

    let helper = std::env::current_exe().expect("current test binary should resolve");
    let output = Command::new(helper)
        .current_dir(project_root)
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
            dispatch_target,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_ACTIVE_NODE_ENV,
            active_node,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DISPATCH_PACKET_PATH_ENV,
            &dispatch_packet_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_TARGET_ENV,
            downstream_target,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_PACKET_PATH_ENV,
            &downstream_packet_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_READY_ENV,
            downstream_ready,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_STATUS_ENV,
            downstream_status,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DOWNSTREAM_BLOCKERS_ENV,
            blocker_code,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_RESULT_PATH_ENV,
            &result_path,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_DISPATCH_STATUS_ENV,
            dispatch_status,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_LANE_STATUS_ENV,
            lane_status,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_BLOCKER_CODE_ENV,
            blocker_code,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_TASK_CLASS_ENV,
            "implementation",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_LIFECYCLE_STAGE_ENV,
            lifecycle_stage,
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_HANDOFF_STATE_ENV,
            "none",
        )
        .env(
            runtime_consumption::RECEIPT_HELPER_RESUME_TARGET_ENV,
            format!("dispatch.{dispatch_target}"),
        )
        .output()
        .expect("runtime receipt helper process should run");
    assert_success(&output, "runtime receipt helper process");
}

fn delete_run_graph_row_with_helper(state_dir: &str, table: &str, run_id: &str) {
    let helper = std::env::current_exe().expect("current test binary should resolve");
    let output = Command::new(helper)
        .args([
            "--ignored",
            "--exact",
            "runtime_delete_run_graph_row_helper_process",
            "--nocapture",
        ])
        .env(
            runtime_consumption::RUN_GRAPH_DELETE_STATE_DIR_ENV,
            state_dir,
        )
        .env(runtime_consumption::RUN_GRAPH_DELETE_TABLE_ENV, table)
        .env(runtime_consumption::RUN_GRAPH_DELETE_RUN_ID_ENV, run_id)
        .output()
        .expect("runtime delete helper process should run");
    assert_success(&output, "runtime delete helper process");
}

fn create_host_bridge_lane_fixture(test_name: &str, changed_file: &str) -> HostBridgeLaneFixture {
    create_host_bridge_lane_fixture_for_target(test_name, changed_file, "implementer")
}

fn create_host_bridge_lane_fixture_for_target(
    test_name: &str,
    changed_file: &str,
    dispatch_target: &str,
) -> HostBridgeLaneFixture {
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
    let (task_class, runtime_role, downstream_target) = match dispatch_target {
        "coach" => ("coach", "coach", "tester"),
        "tester" => ("verification", "verifier", "reviewer"),
        "reviewer" => ("review", "verifier", "release_closure"),
        _ => ("implementation", "worker", "developer"),
    };
    let role_selection_full = serde_json::json!({
        "ok": true,
        "activation_source": "test_fixture",
        "selection_mode": "test",
        "fallback_role": "orchestrator",
        "request": "test",
        "selected_role": runtime_role,
        "conversational_mode": null,
        "single_task_only": true,
        "tracked_flow_entry": null,
        "allow_freeform_chat": false,
        "confidence": "high",
        "matched_terms": [],
        "compiled_bundle": {},
        "reason": "test fixture",
        "execution_plan": {
            "development_flow": {
                "dispatch_contract": {
                    "execution_lane_sequence": [
                        "implementer",
                        "developer",
                        "developer_rework",
                        "coach",
                        "tester",
                        "reviewer",
                        "release_closure"
                    ],
                    "lane_catalog": {
                        "implementer": {
                            "dispatch_target": "implementer",
                            "task_class": "implementation",
                            "stage": "execution",
                            "runtime_role": "worker"
                        },
                        "developer": {
                            "dispatch_target": "developer",
                            "task_class": "implementation",
                            "stage": "execution",
                            "runtime_role": "worker"
                        },
                        "developer_rework": {
                            "dispatch_target": "developer_rework",
                            "task_class": "implementation",
                            "stage": "execution",
                            "runtime_role": "worker"
                        },
                        "coach": {
                            "dispatch_target": "coach",
                            "task_class": "coach",
                            "stage": "quality_gate",
                            "runtime_role": "coach"
                        },
                        "tester": {
                            "dispatch_target": "tester",
                            "task_class": "verification",
                            "stage": "verification",
                            "runtime_role": "verifier"
                        },
                        "reviewer": {
                            "dispatch_target": "reviewer",
                            "task_class": "review",
                            "stage": "review",
                            "runtime_role": "verifier"
                        },
                        "release_closure": {
                            "dispatch_target": "release_closure",
                            "task_class": "release_readiness",
                            "stage": "release_readiness",
                            "runtime_role": "release"
                        }
                    }
                }
            }
        }
    });

    std::fs::write(
        &packet_path,
        serde_json::json!({
            "run_id": run_id,
            "dispatch_target": dispatch_target,
            "activation_runtime_role": runtime_role,
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": ["crates/vida/src/lib.rs"],
            "read_only_paths": ["crates/vida/src"],
            "delivery_task_packet": {
                "goal": "Complete host bridge lane evidence.",
                "scope_in": [format!("dispatch_target:{dispatch_target}")],
                "handoff_task_class": task_class,
                "handoff_runtime_role": runtime_role,
                "owned_paths": ["crates/vida/src/lib.rs"],
                "read_only_paths": ["crates/vida/src"],
                "definition_of_done": ["host bridge completion is receipt-backed"],
                "verification_command": "cargo test -p vida host_bridge_public_cli",
                "proof_target": "host bridge completion receipt",
                "stop_rules": ["stop if bridge evidence is missing"],
                "blocking_question": "none"
            },
            "downstream_dispatch_target": downstream_target,
            "downstream_dispatch_active_target": "implementer",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["pending_implementation_evidence"],
            "downstream_dispatch_status": "blocked",
            "downstream_lane_status": "lane_blocked",
            "run_graph_bootstrap": {},
            "role_selection_full": role_selection_full
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
            "dispatch_target": dispatch_target,
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
        dispatch_target,
    );

    HostBridgeLaneFixture {
        state_dir,
        run_id,
        request_path,
        packet_path,
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
#[ignore = "helper process for public persisted-state row deletion"]
fn runtime_delete_run_graph_row_helper_process() {
    if std::env::var(runtime_consumption::RUN_GRAPH_DELETE_STATE_DIR_ENV).is_ok() {
        runtime_consumption::delete_run_graph_row_from_env();
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
fn host_bridge_public_cli_receipt_contract_failure_maps_to_mismatch_without_state_access() {
    let fixture = create_host_bridge_lane_fixture(
        "host-bridge-public-invalid-receipt",
        "crates/vida/src/lib.rs",
    );
    let mut request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.request_path)
            .expect("invalid receipt request should be readable"),
    )
    .expect("invalid receipt request should parse");
    let request_object = request
        .as_object_mut()
        .expect("invalid receipt request should be an object");
    request_object.insert(
        "attempt_id".to_string(),
        serde_json::json!("host-bridge-public-invalid-receipt-attempt"),
    );
    request_object.insert(
        "packet_id".to_string(),
        serde_json::json!("host-bridge-public-invalid-receipt-packet"),
    );
    request_object.insert("receipt_mode".to_string(), serde_json::json!("host_bridge_receipt"));
    request_object.insert("adapter_kind".to_string(), serde_json::json!("codex_host_tools"));
    request_object.insert(
        "adapter_capability_id".to_string(),
        serde_json::json!("codex.multi_agent_v1"),
    );
    request_object.insert(
        "invocation_mode".to_string(),
        serde_json::json!("parent_host_tool_api"),
    );
    let registry = serde_json::json!({
        "adapter_kind": "codex_host_tools",
        "adapter_capability_id": "codex.multi_agent_v1",
        "invocation_mode": "parent_host_tool_api",
        "dispatch_transport": "host_tool_bridge",
        "receipt_mode": "host_bridge_receipt",
        "operations": {
            "spawn": "multi_agent_v1.spawn_agent",
            "wait": "multi_agent_v1.wait_agent",
            "dispose": "multi_agent_v1.close_agent"
        },
        "dispose_policy": "configured"
    });
    request = effective_host_bridge_request_with_registry(&request, &registry)
        .expect("invalid receipt request should materialize strict adapter fields");
    let adapter_contract_snapshot = request
        .get("adapter_operations")
        .cloned()
        .expect("strict request should carry adapter operations");
    let adapter_contract_hash = blake3::hash(
        &serde_json::to_vec(&adapter_contract_snapshot)
            .expect("adapter contract snapshot should serialize"),
    )
    .to_hex()
    .to_string();
    let request_object = request
        .as_object_mut()
        .expect("strict invalid receipt request should be an object");
    request_object.insert(
        "adapter_contract_snapshot".to_string(),
        adapter_contract_snapshot,
    );
    request_object.insert(
        "adapter_contract_hash".to_string(),
        serde_json::json!(adapter_contract_hash),
    );
    request_object.insert(
        "adapter_contract_source".to_string(),
        serde_json::json!("configured_registry"),
    );
    std::fs::write(
        &fixture.request_path,
        serde_json::to_vec_pretty(&request).expect("strict invalid receipt request should serialize"),
    )
    .expect("strict invalid receipt request should be written");
    persist_host_bridge_lane_receipt_with_target_and_active_node_and_downstream_state(
        &fixture.state_dir,
        &fixture.run_id,
        "implementer",
        "implementer",
        "developer",
        "",
        "lane_open",
        "host_tool_bridge_adapter_required",
        "implementer_blocked",
        "false",
        "blocked",
    );

    let output = vida()
        .args([
            "agent",
            "host-bridge",
            "--request",
            &fixture.request_path,
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("host bridge invalid receipt JSON should run");
    assert_failure(&output, "host bridge invalid receipt contract");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("host bridge invalid receipt JSON should parse");
    assert_eq!(payload["surface"], "vida agent host-bridge");
    assert_eq!(payload["status"], "blocked");
    assert!(
        payload["blocker_codes"]
            .as_array()
            .expect("blocker codes should render")
            .iter()
            .any(|code| code == "host_bridge_dispatch_receipt_mismatch"),
        "invalid receipt contract should map to typed mismatch blocker: {payload}"
    );
    assert!(payload.get("state_access").is_none());
    assert!(payload.get("error").is_none());
    let payload_text = payload.to_string();
    assert!(!payload_text.contains("receipt contract invalid"));
}

#[test]
fn host_bridge_public_cli_summary_prose_does_not_create_false_rework_blocker() {
    let fixture = create_host_bridge_lane_fixture(
        "host-bridge-verification-summary",
        "crates/vida/src/lib.rs",
    );

    let output = vida()
        .args([
            "lane",
            "complete",
            &fixture.run_id,
            "--receipt-id",
            "host-bridge-verification-summary-receipt",
            "--host-bridge-request",
            &fixture.request_path,
            "--host-agent-id",
            "agent-verification-proof",
            "--host-bridge-summary",
            "verifier proof passed focused host-bridge tests and confirmed pending receipt was the only closure blocker",
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("verification lane complete should run");
    assert_success(
        &output,
        "lane complete should pass with positive blocker prose",
    );
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
    .expect("bridge result should parse");
    assert_eq!(bridge_result["status"], "pass");
    assert_eq!(bridge_result["execution_state"], "executed");
    assert_eq!(
        bridge_result["execution_evidence"]["completion_verdict"],
        "pass"
    );
    assert_eq!(bridge_result["blocker_codes"], serde_json::json!([]));
}

#[test]
fn host_bridge_public_cli_quality_gate_matrix_routes_pass_and_blocked_decisions() {
    let cases = [
        (
            "coach",
            "coach decision=approve; implementation accepted",
            true,
            None,
            None,
            "tester",
        ),
        (
            "coach",
            "coach decision=blocked; scheduledAt missing for non-all-day meeting",
            false,
            Some("coach_rework_required"),
            Some("developer"),
            "developer_rework",
        ),
        (
            "tester",
            "tester decision=approve; focused proof passed",
            true,
            None,
            None,
            "reviewer",
        ),
        (
            "tester",
            "tester decision=blocked; focused proof failed",
            false,
            Some("verification_rework_required"),
            Some("developer"),
            "developer_rework",
        ),
        (
            "reviewer",
            "reviewer decision=approve; proof review accepted",
            true,
            None,
            None,
            "release_closure",
        ),
        (
            "reviewer",
            "reviewer decision=blocked; proof review needs tester rework",
            false,
            Some("review_rework_required"),
            Some("tester"),
            "tester",
        ),
    ];

    for (target, summary, should_pass, blocker_code, rework_target, allowed_next_node) in cases {
        let fixture = create_host_bridge_lane_fixture_for_target(
            &format!(
                "host-bridge-{target}-{}-summary",
                if should_pass { "pass" } else { "blocked" }
            ),
            "crates/vida/src/lib.rs",
            target,
        );
        let receipt_id = format!(
            "host-bridge-{target}-{}-summary-receipt",
            if should_pass { "pass" } else { "blocked" }
        );
        let mut command = vida();
        command.args([
            "lane",
            "complete",
            &fixture.run_id,
            "--receipt-id",
            &receipt_id,
            "--host-bridge-request",
            &fixture.request_path,
            "--host-agent-id",
            "agent-quality-gate-proof",
            "--host-bridge-summary",
            summary,
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ]);
        if should_pass {
            command.args([
                "--decision",
                "approve",
                "--verdict",
                "pass",
                "--allowed-next-node",
                allowed_next_node,
            ]);
        } else {
            command.args([
                "--decision",
                "rework_required",
                "--verdict",
                "rework_required",
                "--blocker-code",
                blocker_code.expect("blocked case should name blocker code"),
                "--rework-target",
                rework_target.expect("blocked case should name rework target"),
            ]);
        }
        let output = command
            .output()
            .expect("quality-gate lane complete should run");
        if should_pass {
            assert_success(&output, &format!("lane complete should pass for {target}"));
        } else {
            assert_failure(&output, &format!("lane complete should block for {target}"));
        }
        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
                panic!(
                    "lane complete json should parse for {target}: {error}; stdout={}; stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
            });
        assert_eq!(payload["surface"], "vida lane", "{target}");
        assert_eq!(
            payload["status"],
            if should_pass { "pass" } else { "blocked" },
            "{target}"
        );

        let bridge_result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&fixture.result_path).expect("bridge result should exist"),
        )
        .expect("bridge result should parse");
        assert_eq!(
            bridge_result["status"],
            if should_pass { "pass" } else { "blocked" },
            "{target}"
        );
        assert_eq!(
            bridge_result["execution_state"],
            if should_pass { "executed" } else { "blocked" },
            "{target}"
        );
        assert_eq!(
            bridge_result["decision"],
            if should_pass {
                "approve"
            } else {
                "rework_required"
            },
            "{target}"
        );
        assert_eq!(
            bridge_result["verdict"],
            if should_pass {
                "pass"
            } else {
                "rework_required"
            },
            "{target}"
        );
        assert_eq!(
            bridge_result["execution_evidence"]["receipt_backed"], true,
            "receipt-backed execution must not imply pass verdict for {target}: {bridge_result}"
        );
        assert_eq!(
            bridge_result["execution_evidence"]["completion_verdict"],
            if should_pass {
                "pass"
            } else {
                "rework_required"
            },
            "{target}"
        );

        if should_pass {
            assert_eq!(
                bridge_result["blocker_codes"],
                serde_json::json!([]),
                "{target}"
            );
            assert_eq!(
                bridge_result["rework_target"],
                serde_json::Value::Null,
                "{target}"
            );
            assert_eq!(
                bridge_result["allowed_next_node"], allowed_next_node,
                "{target}"
            );
        } else {
            let blocker_code = blocker_code.expect("blocked case should name blocker code");
            let rework_target = rework_target.expect("blocked case should name rework target");
            assert!(
                !payload["blocker_codes"]
                    .as_array()
                    .expect("blocker codes should be an array")
                    .is_empty(),
                "lane payload should expose a blocked envelope for {target}: {payload}"
            );
            let completion_result_path = payload["artifact_refs"]
                ["downstream_dispatch_result_path"]
                .as_str()
                .expect("completion result path should be present");
            let completion_result: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(completion_result_path)
                    .expect("completion result should exist"),
            )
            .expect("completion result should parse");
            assert_eq!(completion_result["status"], "blocked", "{target}");
            assert_eq!(completion_result["execution_state"], "blocked", "{target}");
            assert_eq!(completion_result["decision"], "rework_required", "{target}");
            assert_eq!(completion_result["verdict"], "rework_required", "{target}");
            assert_eq!(
                completion_result["completion_verdict"], "rework_required",
                "{target}"
            );
            assert_eq!(
                completion_result["rework_target"], rework_target,
                "{target}"
            );
            assert_eq!(
                completion_result["allowed_next_node"], allowed_next_node,
                "{target}"
            );
            assert_eq!(
                completion_result["blocker_code"], blocker_code,
                "completion result should preserve blocker for {target}: {completion_result}"
            );
            assert_eq!(completion_result["closure_ready"], false, "{target}");
        }
    }
}

#[test]
fn coach_blocked_flow_does_not_dispatch_verification() {
    let fixture = create_host_bridge_lane_fixture_for_target(
        "host-bridge-coach-blocked-no-verification",
        "crates/vida/src/lib.rs",
        "coach",
    );

    let output = vida()
        .args([
            "lane",
            "complete",
            &fixture.run_id,
            "--receipt-id",
            "host-bridge-coach-blocked-no-verification-receipt",
            "--host-bridge-request",
            &fixture.request_path,
            "--host-agent-id",
            "agent-coach-blocked-regression",
            "--host-bridge-summary",
            "coach decision=blocked; implementation must return to developer rework",
            "--decision",
            "rework_required",
            "--verdict",
            "rework_required",
            "--blocker-code",
            "coach_rework_required",
            "--rework-target",
            "developer",
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("blocked coach lane complete should run");
    assert_failure(&output, "blocked coach lane complete");

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("lane complete json should parse");
    assert_eq!(payload["status"], "blocked");
    assert!(
        !payload["blocker_codes"]
            .as_array()
            .expect("lane blocker codes should be an array")
            .is_empty(),
        "lane payload should expose a blocked envelope: {payload}"
    );
    assert_ne!(payload["downstream_dispatch_target"], "tester");
    assert_ne!(payload["downstream_dispatch_target"], "verification");
    assert_ne!(payload["downstream_dispatch_target"], "verifier");
    let downstream_packet_path = payload["artifact_refs"]["downstream_dispatch_packet_path"]
        .as_str()
        .expect("blocked lane payload should retain the current coach packet path");
    let downstream_packet: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(downstream_packet_path)
            .expect("downstream packet reference should be readable"),
    )
    .expect("downstream packet reference should parse");
    assert_eq!(downstream_packet["dispatch_target"], "coach");
    assert_ne!(downstream_packet["dispatch_target"], "tester");
    assert_ne!(downstream_packet["activation_runtime_role"], "verifier");

    let completion_result_path = payload["artifact_refs"]["downstream_dispatch_result_path"]
        .as_str()
        .expect("blocked completion result path should be present");
    let completion_result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(completion_result_path)
            .expect("blocked completion result should exist"),
    )
    .expect("blocked completion result should parse");
    assert_eq!(completion_result["status"], "blocked");
    assert_eq!(completion_result["decision"], "rework_required");
    assert_eq!(completion_result["verdict"], "rework_required");
    assert_eq!(completion_result["blocker_code"], "coach_rework_required");
    assert_eq!(
        completion_result["blocker_codes"],
        serde_json::json!(["coach_rework_required"])
    );
    assert_eq!(completion_result["rework_target"], "developer");
    assert_eq!(completion_result["allowed_next_node"], "developer_rework");
    assert_ne!(completion_result["allowed_next_node"], "verification");
    assert_ne!(completion_result["allowed_next_node"], "tester");
}

#[test]
fn host_bridge_public_cli_retries_retryable_blocked_request_after_attempt_artifacts() {
    let fixture =
        create_host_bridge_lane_fixture("host-bridge-public-retry", "crates/vida/src/lib.rs");
    let mut request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.request_path).expect("request should exist"),
    )
    .expect("request should parse");
    request["status"] = serde_json::json!("retryable_blocked");
    request["adapter_kind"] = serde_json::json!("codex_host_tools");
    request["adapter_capability_id"] = serde_json::json!("codex.multi_agent_v1");
    request["request_path"] = serde_json::json!(fixture.request_path.clone());
    std::fs::write(
        &fixture.request_path,
        serde_json::to_string_pretty(&request).expect("request should serialize"),
    )
    .expect("request should write");
    std::fs::write(
        &fixture.result_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "status": "blocked",
            "request_id": "host-bridge-public-retry",
            "run_id": &fixture.run_id,
            "task_id": &fixture.run_id,
            "dispatch_target": "implementer",
            "backend_id": "internal_subagents",
            "source_dispatch_packet_path": &fixture.packet_path,
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_code": "implementation_artifacts_missing",
            "blocker_codes": ["implementation_artifacts_missing"],
            "rework_target": "developer",
            "allowed_next_node": "developer"
        })
        .to_string(),
    )
    .expect("blocked bridge result should write");
    std::fs::write(
        &fixture.bridge_receipt_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_receipt",
            "status": "blocked",
            "request_id": "host-bridge-public-retry",
            "run_id": &fixture.run_id,
            "task_id": &fixture.run_id,
            "dispatch_target": "implementer",
            "backend_id": "internal_subagents",
            "dispatch_packet_path": &fixture.packet_path,
            "blocker_code": "implementation_artifacts_missing",
            "blocker_codes": ["implementation_artifacts_missing"]
        })
        .to_string(),
    )
    .expect("blocked bridge receipt should write");

    let lane_show = vida()
        .args(["lane", "show", &fixture.run_id, "--json"])
        .env("VIDA_STATE_DIR", &fixture.state_dir)
        .output()
        .expect("lane show retry guidance should run");
    assert_failure(&lane_show, "lane show retry guidance");
    let lane_payload: serde_json::Value =
        serde_json::from_slice(&lane_show.stdout).expect("lane show json should parse");
    assert_eq!(
        lane_payload["recommended_surface"], "vida agent host-bridge",
        "lane payload should recommend active implementer request: {lane_payload}"
    );
    assert!(lane_payload["recommended_command"]
        .as_str()
        .expect("recommended command")
        .starts_with("vida agent host-bridge --request "));
    assert!(
        !lane_payload["recommended_command"]
            .as_str()
            .expect("recommended command")
            .contains("exception-takeover"),
        "retryable host bridge blockers must not recommend exception takeover first"
    );
    std::fs::remove_file(&fixture.bridge_receipt_path)
        .expect("retry completion should replace the stale host bridge receipt path");
    let corrected_result_path = std::path::PathBuf::from(&fixture.result_path);
    std::fs::write(
        &corrected_result_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "status": "pass",
            "execution_state": "executed",
            "request_id": "host-bridge-public-retry",
            "run_id": &fixture.run_id,
            "task_id": &fixture.run_id,
            "dispatch_target": "implementer",
            "backend_id": "internal_subagents",
            "source_dispatch_packet_path": &fixture.packet_path,
            "decision": "pass",
            "verdict": "pass",
            "blocker_codes": [],
            "execution_evidence": {
                "receipt_backed": true
            }
        })
        .to_string(),
    )
    .expect("corrected bridge result should write");
    let corrected_result_path = corrected_result_path.display().to_string();

    let output = vida()
        .args([
            "agent",
            "host-bridge",
            "--request",
            &fixture.request_path,
            "--retry-completion",
            "--host-agent-id",
            "agent-public-retry-proof",
            "--submit-result",
            &corrected_result_path,
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("agent host-bridge retry should run");
    assert_success(&output, "agent host-bridge retry after retryable blocker");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("agent host-bridge json should parse");
    assert_eq!(payload["surface"], "vida lane");
    assert_eq!(payload["status"], "pass");
    assert!(!payload["blocker_codes"]
        .as_array()
        .expect("blocker codes should render")
        .iter()
        .any(|code| code.as_str() == Some("host_bridge_request_not_pending")));
}

#[test]
fn host_bridge_public_cli_retries_blocked_request_with_lawful_rework_contract() {
    let fixture = create_host_bridge_lane_fixture_for_target(
        "host-bridge-blocked-contract-retry",
        "crates/vida/src/lib.rs",
        "coach",
    );
    let mut request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.request_path).expect("request should exist"),
    )
    .expect("request should parse");
    request["status"] = serde_json::json!("blocked");
    request["request_status"] = serde_json::json!("blocked");
    request["blocked_result_contract"] = serde_json::json!({
        "status": "blocked",
        "decision": "rework_required",
        "verdict": "rework_required",
        "allowed_next_node": "developer_rework",
        "rework_target": "developer",
        "blocker_codes": ["coach_rework_required"]
    });
    std::fs::write(
        &fixture.request_path,
        serde_json::to_string_pretty(&request).expect("request should serialize"),
    )
    .expect("request should write");

    std::fs::write(
        &fixture.result_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "status": "blocked",
            "execution_state": "executed",
            "request_id": "host-bridge-blocked-contract-retry",
            "run_id": &fixture.run_id,
            "task_id": &fixture.run_id,
            "dispatch_target": "coach",
            "backend_id": "internal_subagents",
            "source_dispatch_packet_path": &fixture.packet_path,
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_codes": ["coach_rework_required"],
            "rework_target": "developer",
            "allowed_next_node": "developer_rework",
            "execution_evidence": {
                "receipt_backed": true
            }
        })
        .to_string(),
    )
    .expect("rework bridge result should write");

    let output = vida()
        .args([
            "agent",
            "host-bridge",
            "--request",
            &fixture.request_path,
            "--retry-completion",
            "--host-agent-id",
            "agent-blocked-contract-retry-proof",
            "--submit-result",
            &fixture.result_path,
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("agent host-bridge contract retry should run");
    assert_success(
        &output,
        "agent host-bridge retry after blocked result contract",
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("agent host-bridge json should parse");
    assert_eq!(payload["surface"], "vida lane");
    assert!(!payload["blocker_codes"]
        .as_array()
        .expect("blocker codes should render")
        .iter()
        .any(|code| code.as_str() == Some("host_bridge_request_not_pending")));
    let result_path = payload["artifact_refs"]["host_bridge_result_path"]
        .as_str()
        .expect("host bridge result path should be present");
    let result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(result_path).expect("host bridge result should be readable"),
    )
    .expect("host bridge result should parse");
    assert_eq!(result["allowed_next_node"], "developer_rework");
    assert_eq!(result["decision"], "rework_required");
    assert_eq!(result["verdict"], "rework_required");
    assert_eq!(
        payload["artifact_refs"]["downstream_dispatch_target"],
        "developer_rework"
    );
    assert_eq!(
        payload["artifact_refs"]["downstream_dispatch_status"],
        "packet_ready"
    );
    assert_eq!(payload["artifact_refs"]["downstream_dispatch_ready"], true);
    assert!(payload["artifact_refs"]["downstream_dispatch_blockers"]
        .as_array()
        .expect("downstream blockers should render")
        .is_empty());
    let downstream_packet_path = payload["artifact_refs"]["downstream_dispatch_packet_path"]
        .as_str()
        .expect("rework retry should write downstream dispatch packet");
    let downstream_packet: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(downstream_packet_path)
            .expect("downstream dispatch packet should be readable"),
    )
    .expect("downstream dispatch packet should parse");
    assert_eq!(downstream_packet["dispatch_target"], "developer_rework");
    let status_output = vida()
        .args([
            "taskflow",
            "run-graph",
            "status",
            &fixture.run_id,
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("run graph status should run");
    assert_success(&status_output, "run graph status after rework retry");
    let run_graph_status: serde_json::Value =
        serde_json::from_slice(&status_output.stdout).expect("run graph status should parse");
    assert_eq!(run_graph_status["status"], "pass");
    assert_eq!(run_graph_status["blocker_codes"], serde_json::json!([]));
    assert_eq!(
        run_graph_status["run_graph_status"]["active_node"],
        "coach"
    );
    assert_eq!(
        run_graph_status["run_graph_status"]["next_node"],
        "developer_rework"
    );
    assert_eq!(
        run_graph_status["run_graph_status"]["lifecycle_stage"],
        "developer_rework_dispatch_ready"
    );
    assert_eq!(run_graph_status["run_graph_status"]["status"], "ready");
    assert_eq!(
        run_graph_status["run_graph_status"]["resume_target"],
        "dispatch.developer_rework"
    );
    assert_eq!(
        run_graph_status["delegation_gate"]["delegated_cycle_open"],
        false
    );
}

#[test]
fn run_graph_status_reconciles_half_applied_rework_downstream_packet() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "run-half-applied-coach-rework";
    create_session_triage_task(
        &state_dir,
        run_id,
        "Half-applied coach rework",
        "epic",
        "in_progress",
        "1",
        None,
    );
    persist_host_bridge_lane_receipt_with_target_and_active_node_and_downstream_state(
        &state_dir,
        run_id,
        "coach",
        "coach",
        "developer",
        "executed",
        "lane_completed",
        "",
        "coach_blocked",
        "true",
        "packet_ready",
    );
    let result_path =
        format!("{state_dir}/runtime-consumption/dispatch-results/{run_id}-coach.json");
    std::fs::write(
        &result_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "status": "blocked",
            "execution_state": "blocked",
            "request_id": run_id,
            "run_id": run_id,
            "task_id": run_id,
            "dispatch_target": "coach",
            "backend_id": "internal_subagents",
            "source_dispatch_packet_path": format!(
                "{state_dir}/runtime-consumption/downstream-dispatch-packets/{run_id}-coach.json"
            ),
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_codes": ["meeting_attendee_selector_contract_incomplete"],
            "rework_target": "developer",
            "allowed_next_node": "developer_rework",
            "execution_evidence": {
                "receipt_backed": true
            }
        })
        .to_string(),
    )
    .expect("rework result should write");

    let status_output = vida()
        .args([
            "taskflow",
            "run-graph",
            "status",
            run_id,
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .current_dir(&project_root)
        .output()
        .expect("run graph status should run");
    assert_success(&status_output, "half-applied rework run graph status");
    let payload: serde_json::Value =
        serde_json::from_slice(&status_output.stdout).expect("run graph status should parse");
    assert_eq!(payload["status"], "pass");
    assert_eq!(payload["blocker_codes"], serde_json::json!([]));
    assert_eq!(
        payload["run_graph_status"]["active_node"],
        "developer_rework"
    );
    assert_eq!(
        payload["run_graph_status"]["lifecycle_stage"],
        "developer_rework_dispatch_ready"
    );
    assert_eq!(
        payload["run_graph_status"]["resume_target"],
        "dispatch.developer_rework"
    );
    assert_eq!(payload["delegation_gate"]["delegated_cycle_open"], false);
}

#[test]
fn run_graph_status_reconciles_half_applied_verifier_rework_downstream_packet() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "run-half-applied-verifier-rework";
    create_session_triage_task(
        &state_dir,
        run_id,
        "Half-applied verifier rework",
        "epic",
        "in_progress",
        "1",
        None,
    );
    persist_host_bridge_lane_receipt_with_target_and_active_node_and_downstream_state(
        &state_dir,
        run_id,
        "tester",
        "tester",
        "developer",
        "executed",
        "lane_completed",
        "",
        "tester_blocked",
        "true",
        "packet_ready",
    );
    let result_path =
        format!("{state_dir}/runtime-consumption/dispatch-results/{run_id}-tester.json");
    std::fs::write(
        &result_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "status": "blocked",
            "execution_state": "blocked",
            "request_id": run_id,
            "run_id": run_id,
            "dispatch_target": "tester",
            "decision": "rework",
            "verdict": "fail",
            "blocker_codes": [
                "meeting_attendee_selector_contract_incomplete",
                "meeting_current_user_attendee_default_missing",
                "meeting_attendee_add_remove_widget_proof_missing"
            ],
            "rework_target": "developer",
            "allowed_next_node": "developer_rework",
            "execution_evidence": {
                "receipt_backed": true
            }
        })
        .to_string(),
    )
    .expect("verifier rework result should write");

    let status_output = vida()
        .args([
            "taskflow",
            "run-graph",
            "status",
            run_id,
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .current_dir(&project_root)
        .output()
        .expect("run graph status should run");
    assert_success(
        &status_output,
        "half-applied verifier rework run graph status",
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&status_output.stdout).expect("run graph status should parse");
    assert_eq!(payload["status"], "pass");
    assert_eq!(payload["blocker_codes"], serde_json::json!([]));
    assert_eq!(
        payload["run_graph_status"]["active_node"],
        "developer_rework"
    );
    assert_eq!(
        payload["run_graph_status"]["lifecycle_stage"],
        "developer_rework_dispatch_ready"
    );
    assert_eq!(
        payload["run_graph_status"]["resume_target"],
        "dispatch.developer_rework"
    );
    assert_eq!(payload["delegation_gate"]["delegated_cycle_open"], false);
}

#[test]
fn host_bridge_public_cli_uses_submitted_rework_result_as_retry_evidence() {
    let fixture = create_host_bridge_lane_fixture_for_target(
        "host-bridge-submitted-rework-retry",
        "crates/vida/src/lib.rs",
        "coach",
    );
    let mut request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.request_path).expect("request should exist"),
    )
    .expect("request should parse");
    request["status"] = serde_json::json!("blocked");
    request["request_status"] = serde_json::json!("blocked");
    request
        .as_object_mut()
        .expect("request object")
        .remove("blocked_result_contract");
    std::fs::write(
        &fixture.request_path,
        serde_json::to_string_pretty(&request).expect("request should serialize"),
    )
    .expect("request should write");

    std::fs::write(
        &fixture.result_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "status": "blocked",
            "execution_state": "executed",
            "request_id": "host-bridge-submitted-rework-retry",
            "run_id": &fixture.run_id,
            "task_id": &fixture.run_id,
            "dispatch_target": "coach",
            "backend_id": "internal_subagents",
            "source_dispatch_packet_path": &fixture.packet_path,
            "decision": "rework_required",
            "verdict": "rework_required",
            "blocker_codes": ["host_agent_execution_failed"],
            "rework_target": "developer",
            "allowed_next_node": "developer_rework",
            "execution_evidence": {
                "receipt_backed": true
            }
        })
        .to_string(),
    )
    .expect("rework bridge result should write");
    std::fs::write(
        &fixture.bridge_receipt_path,
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_receipt",
            "status": "blocked",
            "completion_receipt_id": "stale-public-retry-receipt",
            "request_id": "host-bridge-submitted-rework-retry",
            "run_id": &fixture.run_id,
            "task_id": &fixture.run_id,
            "dispatch_target": "coach",
            "backend_id": "internal_subagents",
            "dispatch_packet_path": &fixture.packet_path
        })
        .to_string(),
    )
    .expect("stale canonical receipt should write");

    let output = vida()
        .args([
            "agent",
            "host-bridge",
            "--request",
            &fixture.request_path,
            "--retry-completion",
            "--host-agent-id",
            "agent-submitted-rework-retry-proof",
            "--submit-result",
            &fixture.result_path,
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("agent host-bridge submitted retry should run");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("agent host-bridge json should parse");
    assert_eq!(payload["surface"], "vida lane", "payload: {payload:#}");
    assert!(!payload["blocker_codes"]
        .as_array()
        .expect("blocker codes should render")
        .iter()
        .any(|code| code.as_str() == Some("host_bridge_request_not_pending")));
    assert!(payload["artifact_refs"]["host_bridge_result_path"].is_string());
    let receipt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.bridge_receipt_path)
            .expect("host bridge receipt should be readable"),
    )
    .expect("host bridge receipt should parse");
    assert_ne!(
        receipt["completion_receipt_id"],
        "stale-public-retry-receipt"
    );
}

#[test]
fn host_bridge_public_cli_fails_closed_when_receipt_target_differs_from_request_target() {
    let fixture =
        create_host_bridge_lane_fixture("host-bridge-stale-receipt", "crates/vida/src/lib.rs");
    let mut request: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fixture.request_path).expect("request should exist"),
    )
    .expect("request should parse");
    request["adapter_kind"] = serde_json::json!("codex_host_tools");
    request["adapter_capability_id"] = serde_json::json!("codex.multi_agent_v1");
    request["request_path"] = serde_json::json!(fixture.request_path.clone());
    std::fs::write(
        &fixture.request_path,
        serde_json::to_string_pretty(&request).expect("request should serialize"),
    )
    .expect("request should write");
    persist_host_bridge_lane_receipt_with_target_and_active_node(
        &fixture.state_dir,
        &fixture.run_id,
        "coach",
        "implementer",
        "tester",
        "bridge_request_pending",
        "lane_open",
        "host_tool_bridge_adapter_required",
        "implementer_blocked",
    );
    let stale_result_path = format!(
        "{}/runtime-consumption/dispatch-results/{}-coach.json",
        fixture.state_dir, fixture.run_id
    );
    let mut stale_result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&stale_result_path).expect("stale result should exist"),
    )
    .expect("stale result should parse");
    stale_result["host_tool_bridge_request"] = serde_json::json!({
        "request_path": fixture.request_path.clone(),
        "result_path": fixture.result_path.clone(),
        "receipt_path": fixture.bridge_receipt_path.clone()
    });
    std::fs::write(
        &stale_result_path,
        serde_json::to_string_pretty(&stale_result).expect("stale result should serialize"),
    )
    .expect("stale result should retain host bridge request metadata");
    std::fs::write(
        format!(
            "{}/runtime-consumption/dispatch-packets/{}-coach.json",
            fixture.state_dir, fixture.run_id
        ),
        serde_json::json!({
            "run_id": fixture.run_id,
            "dispatch_target": "coach",
            "source_dispatch_target": "implementer",
            "source_dispatch_status": "bridge_request_pending",
            "source_blocker_code": "host_tool_bridge_adapter_required",
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": ["pending_implementation_evidence"]
        })
        .to_string(),
    )
    .expect("stale coach packet should preserve blocked implementer source");

    let lane_show = vida()
        .args(["lane", "show", &fixture.run_id, "--json"])
        .env("VIDA_STATE_DIR", &fixture.state_dir)
        .output()
        .expect("lane show should run");
    assert_failure(&lane_show, "lane show stale receipt guidance");
    let lane_payload: serde_json::Value =
        serde_json::from_slice(&lane_show.stdout).expect("lane show json should parse");
    assert_eq!(
        lane_payload["recommended_surface"],
        "vida agent host-bridge"
    );
    assert!(lane_payload["recommended_command"]
        .as_str()
        .expect("recommended command")
        .contains(&fixture.request_path));

    let output = vida()
        .args([
            "agent",
            "host-bridge",
            "--request",
            &fixture.request_path,
            "--complete",
            "--host-agent-id",
            "agent-stale-receipt-proof",
            "--summary",
            "completion after recovery reconciled active implementer request",
            "--state-dir",
            &fixture.state_dir,
            "--json",
        ])
        .output()
        .expect("agent host-bridge stale receipt completion should run");
    assert_failure(
        &output,
        "agent host-bridge should fail closed when stale receipt target differs from request target",
    );
    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined_output.contains("host_bridge_dispatch_receipt_mismatch"),
        "stale receipt mismatch should be explicit: output={combined_output}"
    );
}

#[test]
fn lane_show_recommends_host_bridge_completion_before_exception_takeover() {
    let fixture =
        create_host_bridge_lane_fixture("host-bridge-lane-guidance", "crates/vida/src/lib.rs");

    let output = vida()
        .args(["lane", "show", &fixture.run_id, "--json"])
        .env("VIDA_STATE_DIR", &fixture.state_dir)
        .output()
        .expect("lane show should run");
    assert_failure(&output, "lane show blocked host bridge");
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("lane show json should parse");

    assert_eq!(payload["surface"], "vida lane");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["recommended_surface"], "vida agent host-bridge",
        "payload={payload}"
    );
    assert!(payload["recommended_command"]
        .as_str()
        .expect("recommended command should exist")
        .starts_with("vida agent host-bridge --request "));
    assert!(payload["recommended_command"]
        .as_str()
        .expect("recommended command should exist")
        .contains(&fixture.request_path));
    assert!(
        !payload["recommended_command"]
            .as_str()
            .expect("recommended command should exist")
            .contains("exception-takeover"),
        "host bridge pending lanes must not recommend exception takeover as primary action"
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
    assert!(help_stdout.contains("--refresh"));
    assert!(help_stdout.contains("--no-cache"));
    assert!(
        help_stdout.contains("compact TOON"),
        "taskflow next help should document default compact TOON: {help_stdout}"
    );
    assert!(
        help_stdout.contains("machine-readable JSON"),
        "taskflow next help should document explicit machine-readable JSON: {help_stdout}"
    );
    assert!(
        help_stdout.contains("authoritative recompute"),
        "taskflow next help should document cache refresh behavior: {help_stdout}"
    );
}

#[test]
fn taskflow_team_continue_outputs_help_default_json_and_state_dir_contracts() {
    let (project_root, state_dir) = project_bound_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");

    let team_help = vida()
        .args(["taskflow", "team", "--help"])
        .env_remove("VIDA_STATE_DIR")
        .output()
        .expect("taskflow team help should run");
    assert_success(&team_help, "taskflow team help");
    let help_stdout = String::from_utf8_lossy(&team_help.stdout);
    assert!(help_stdout.contains("VIDA TaskFlow help: team"));
    assert!(help_stdout.contains("vida taskflow team continue <task-id>"));
    assert!(help_stdout.contains("Default human output uses compact TOON/plain"));
    assert!(help_stdout.contains("--json emits the machine-readable operator contract"));
    assert!(help_stdout.contains("--state-dir <path>"));

    let continue_help = vida()
        .args(["taskflow", "team", "continue", "--help"])
        .env_remove("VIDA_STATE_DIR")
        .output()
        .expect("taskflow team continue help should run");
    assert_success(&continue_help, "taskflow team continue help");
    assert!(String::from_utf8_lossy(&continue_help.stdout).contains("VIDA TaskFlow help: team"));

    let default_output = vida()
        .args([
            "taskflow",
            "team",
            "continue",
            "activity-meeting-like-fixture",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow team continue default should run");
    assert_failure(
        &default_output,
        "taskflow team continue default should fail closed without receipt",
    );
    let default_stdout = String::from_utf8_lossy(&default_output.stdout);
    assert_not_json_output("vida taskflow team continue", &default_stdout);
    assert_no_raw_terminal_controls("vida taskflow team continue", &default_stdout);
    assert!(default_stdout.contains("missing_run_graph_dispatch_receipt"));
    assert!(default_stdout.contains("artifact_refs:"));

    let json_output = vida()
        .args([
            "taskflow",
            "team",
            "continue",
            "activity-meeting-like-fixture",
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .env_remove("VIDA_STATE_DIR")
        .output()
        .expect("taskflow team continue json should run");
    assert_failure(
        &json_output,
        "taskflow team continue json should fail closed without receipt",
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("team continue json should parse");
    assert_eq!(payload["surface"], "vida taskflow consume continue");
    assert_eq!(payload["status"], "blocked");
    assert_eq!(
        payload["blocker_codes"],
        serde_json::json!(["missing_run_graph_dispatch_receipt"])
    );
    assert_eq!(
        payload["artifact_refs"]["run_id"],
        "activity-meeting-like-fixture"
    );
    assert_eq!(
        payload["operator_contracts"]["artifact_refs"]["surface"],
        "vida taskflow consume continue"
    );
    assert!(payload["next_actions"].as_array().is_some_and(|actions| {
        actions.iter().any(|action| {
            action
                .as_str()
                .is_some_and(|text| text.contains("vida taskflow consume continue"))
        })
    }));

    let diagnose_output = vida()
        .args([
            "taskflow",
            "team",
            "diagnose",
            "activity-meeting-like-fixture",
            "--state-dir",
            &state_dir,
            "--json",
        ])
        .env_remove("VIDA_STATE_DIR")
        .output()
        .expect("taskflow team diagnose json should run");
    assert_failure(
        &diagnose_output,
        "taskflow team diagnose json should fail closed without receipt",
    );
    let diagnose_payload: serde_json::Value =
        serde_json::from_slice(&diagnose_output.stdout).expect("team diagnose json should parse");
    let diagnose_text =
        serde_json::to_string(&diagnose_payload).expect("team diagnose json should render");
    assert_eq!(
        diagnose_payload["next_command"],
        "vida taskflow team status activity-meeting-like-fixture"
    );
    for forbidden in [
        "vida lane",
        "vida taskflow run-graph",
        "vida agent-init",
        "vida agent host-bridge",
    ] {
        assert!(
            !diagnose_text.contains(forbidden),
            "team diagnose should not recommend manual bridge glue `{forbidden}`: {diagnose_text}"
        );
    }

    let _ = std::fs::remove_dir_all(project_root);
}

#[test]
fn taskflow_graph_summary_documents_and_outputs_cache_policy() {
    let (_project_root, state_dir) = project_bound_state_dir();
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

    let json_output = vida()
        .args(["taskflow", "graph-summary", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("graph-summary json output should run");
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("graph-summary json should parse");
    assert_eq!(payload["surface"], "vida taskflow graph-summary");
    assert_eq!(
        payload["cache_policy"]["mode"],
        "cache_first_with_authoritative_fallback"
    );
    assert_eq!(
        payload["cache_policy"]["read_cache_before_authoritative_open"],
        true
    );
    assert_eq!(
        payload["cache_policy"]["stale_projection_behavior"],
        "reject_and_recompute"
    );
    assert_eq!(
        payload["cache_policy"]["source"],
        "vida.config.yaml:operator_surfaces.taskflow.graph_summary.cache_policy"
    );

    let help = vida()
        .args(["taskflow", "graph-summary", "--help"])
        .output()
        .expect("graph-summary help should run");
    assert!(
        help.status.success(),
        "graph-summary help should succeed: stderr={}",
        String::from_utf8_lossy(&help.stderr)
    );
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("cache-first"));
    assert!(help_stdout.contains("authoritative fallback"));
    assert!(help_stdout.contains("cache_policy"));
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
fn status_and_doctor_materialize_clean_state_root_guard_baseline() {
    let state_dir = unique_state_dir();

    let reset = vida()
        .args(vec![
            "state".to_string(),
            "reset".to_string(),
            "--archive".to_string(),
            "--reinit".to_string(),
            "--state-dir".to_string(),
            state_dir.clone(),
            "--json".to_string(),
        ])
        .output()
        .expect("state reset should run");
    assert!(
        reset.status.success(),
        "state reset should succeed: stderr={}",
        String::from_utf8_lossy(&reset.stderr)
    );

    let status = vida()
        .args(vec![
            "status".to_string(),
            "--state-dir".to_string(),
            state_dir.clone(),
            "--json".to_string(),
        ])
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "status should succeed: stderr={}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");

    let doctor = vida()
        .args(vec![
            "doctor".to_string(),
            "--state-dir".to_string(),
            state_dir.clone(),
            "--json".to_string(),
        ])
        .output()
        .expect("doctor should run");
    assert!(
        doctor.status.success(),
        "doctor should return blocked JSON instead of process failure: stderr={}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");

    for payload in [&status_json, &doctor_json] {
        assert_eq!(
            payload["root_session_write_guard"]["status"],
            "blocked_by_default"
        );
        assert_eq!(
            payload["root_session_write_guard"]["root_local_write_allowed"],
            false
        );
        assert_eq!(
            payload["root_session_write_guard"]["idle_state_baseline"],
            true
        );
    }
    assert_eq!(
        status_json["root_session_write_guard"]["status"],
        doctor_json["root_session_write_guard"]["status"]
    );
    assert!(
        !doctor_json["blocker_codes"]
            .as_array()
            .expect("doctor blocker codes should be an array")
            .iter()
            .any(|code| code.as_str() == Some("missing_root_session_write_guard")),
        "doctor should not report missing root-session write guard: {doctor_json}"
    );

    let _ = std::fs::remove_dir_all(state_dir);
}

#[test]
fn status_and_doctor_ignore_forged_final_snapshot_dispatch_receipt_without_persisted_receipt() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let dispatch_packet_path = format!("{runtime_consumption_dir}/dispatch-packets/forged.json");
    let dispatch_result_path = format!("{runtime_consumption_dir}/dispatch-results/forged.json");
    let final_snapshot_path = format!("{runtime_consumption_dir}/final-2026-06-11T00-00-04Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-06-11T00-00-04Z.json",
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
                "dispatch_receipt": {
                    "run_id": "forged-final-run",
                    "dispatch_target": "developer",
                    "dispatch_status": "blocked",
                    "lane_status": "lane_exception_takeover",
                    "exception_path_receipt_id": "forged-exception-receipt",
                    "supersedes_receipt_id": "forged-supersede-receipt",
                    "dispatch_kind": "agent_init",
                    "dispatch_packet_path": dispatch_packet_path,
                    "dispatch_result_path": dispatch_result_path,
                    "recorded_at": "2026-06-11T00:00:00Z"
                },
                "closure_admission": case10_closure_admission_record()
            },
            "artifact_refs": {
                "runtime_consumption_latest_snapshot_path": final_snapshot_path,
            }
        }),
    );

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(status.status.success());
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert!(status_json["latest_run_graph_dispatch_receipt"].is_null());
    assert_eq!(
        status_json["artifact_refs"]["latest_run_graph_dispatch_receipt_id"],
        serde_json::Value::Null
    );

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");
    assert_eq!(
        doctor_json["artifact_refs"]["latest_run_graph_dispatch_receipt_id"],
        serde_json::Value::Null
    );

    let _ = std::fs::remove_dir_all(state_dir);
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
fn doctor_json_uses_current_run_final_snapshot_over_newer_retired_and_malformed_artifacts() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);
    seed_run_graph(&state_dir, "doctor-current-run");

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let current_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-07-10T00-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T00-00-01Z.json",
        final_snapshot_for_run(
            &current_snapshot_path,
            "doctor-current-run",
            case10_closure_admission_record(),
        ),
    );

    std::thread::sleep(std::time::Duration::from_millis(15));
    let retired_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-07-10T00-00-02Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T00-00-02Z.json",
        final_snapshot_for_run(
            &retired_snapshot_path,
            "retired-unrelated-run",
            case10_closure_admission_record(),
        ),
    );
    std::thread::sleep(std::time::Duration::from_millis(15));
    std::fs::write(
        format!("{runtime_consumption_dir}/final-2026-07-10T00-00-03Z.json"),
        "{ malformed final snapshot",
    )
    .expect("malformed final snapshot should write");

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json should parse");

    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(current_snapshot_path)
    );
    assert_eq!(
        doctor_json["artifact_refs"]["retrieval_trust_signal"]["citation"],
        serde_json::json!(current_snapshot_path)
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]["effective_run_id"],
        "doctor-current-run"
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]
            ["runtime_consumption_selected_final_snapshot_path"],
        serde_json::json!(current_snapshot_path)
    );
    for blocker in [
        "incomplete_release_admission_operator_evidence",
        "missing_retrieval_trust_operator_evidence",
    ] {
        assert!(
            !doctor_json["blocker_codes"]
                .as_array()
                .expect("doctor blockers should be an array")
                .iter()
                .any(|code| code.as_str() == Some(blocker)),
            "{blocker} must not be inherited from newer retired or malformed artifacts: {doctor_json}"
        );
    }
}

#[test]
fn doctor_json_fails_closed_when_current_run_selected_final_is_partial() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);
    seed_run_graph(&state_dir, "doctor-partial-run");

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let older_valid_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-07-10T01-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T01-00-01Z.json",
        final_snapshot_for_run(
            &older_valid_snapshot_path,
            "doctor-partial-run",
            case10_closure_admission_record(),
        ),
    );
    std::thread::sleep(std::time::Duration::from_millis(15));
    let partial_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-07-10T01-00-02Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T01-00-02Z.json",
        final_snapshot_for_run(
            &partial_snapshot_path,
            "doctor-partial-run",
            serde_json::json!({
                "status": "blocked",
                "admitted": false,
                "blockers": ["missing current-run closure evidence"],
            }),
        ),
    );
    std::thread::sleep(std::time::Duration::from_millis(15));
    let unrelated_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-07-10T01-00-03Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T01-00-03Z.json",
        final_snapshot_for_run(
            &unrelated_snapshot_path,
            "unrelated-newer-run",
            case10_closure_admission_record(),
        ),
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
        .expect("doctor blockers should be an array");

    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(partial_snapshot_path)
    );
    for blocker in [
        "incomplete_release_admission_operator_evidence",
        "missing_retrieval_trust_operator_evidence",
    ] {
        assert!(
            blocker_codes
                .iter()
                .any(|code| code.as_str() == Some(blocker)),
            "{blocker} must remain blocked for the selected partial run snapshot: {doctor_json}"
        );
    }
}

#[test]
fn doctor_json_fails_closed_without_matching_effective_run_final_despite_valid_global_evidence() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);
    seed_run_graph(&state_dir, "doctor-missing-final-run");

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let global_snapshot_path = format!("{runtime_consumption_dir}/final-2026-07-10T02-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T02-00-01Z.json",
        final_snapshot_for_run(
            &global_snapshot_path,
            "newer-global-run",
            case10_closure_admission_record(),
        ),
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
        .expect("doctor blockers should be an array");
    let trace_blocker_codes = doctor_json["trace_evidence"]["blocker_codes"]
        .as_array()
        .expect("trace blockers should be an array");

    for blocker in [
        "incomplete_release_admission_operator_evidence",
        "missing_retrieval_trust_operator_evidence",
        "trace_missing",
    ] {
        assert!(
            blocker_codes
                .iter()
                .any(|code| code.as_str() == Some(blocker)),
            "{blocker} must remain blocked without same-run final evidence: {doctor_json}"
        );
    }
    assert!(trace_blocker_codes
        .iter()
        .any(|code| code == "trace_missing"));
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]["effective_run_id"],
        "doctor-missing-final-run"
    );
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::Value::Null
    );
    assert_eq!(
        doctor_json["artifact_refs"]["retrieval_trust_signal"],
        serde_json::Value::Null
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]["runtime_consumption_latest_snapshot_path"],
        serde_json::Value::Null
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]
            ["runtime_consumption_selected_final_snapshot_path"],
        serde_json::Value::Null
    );
}

#[test]
fn doctor_json_fails_closed_on_malformed_effective_run_final_despite_valid_global_evidence() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);
    seed_run_graph(&state_dir, "doctor-malformed-final-run");

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let global_snapshot_path = format!("{runtime_consumption_dir}/final-2026-07-10T03-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T03-00-01Z.json",
        final_snapshot_for_run(
            &global_snapshot_path,
            "valid-global-run",
            case10_closure_admission_record(),
        ),
    );
    std::thread::sleep(std::time::Duration::from_millis(15));
    let malformed_snapshot_path =
        format!("{runtime_consumption_dir}/final-2026-07-10T03-00-02Z.json");
    std::fs::write(&malformed_snapshot_path, "{ malformed same-run final")
        .expect("malformed final snapshot should write");

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
        .expect("doctor blockers should be an array");
    let trace_blocker_codes = doctor_json["trace_evidence"]["blocker_codes"]
        .as_array()
        .expect("trace blockers should be an array");

    for blocker in [
        "incomplete_release_admission_operator_evidence",
        "missing_retrieval_trust_operator_evidence",
        "trace_incomplete",
    ] {
        assert!(
            blocker_codes
                .iter()
                .any(|code| code.as_str() == Some(blocker)),
            "{blocker} must remain blocked for malformed effective-run evidence: {doctor_json}"
        );
    }
    assert!(trace_blocker_codes
        .iter()
        .any(|code| code == "trace_incomplete"));
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(malformed_snapshot_path)
    );
    assert_eq!(
        doctor_json["artifact_refs"]["retrieval_trust_signal"],
        serde_json::Value::Null
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(malformed_snapshot_path)
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]
            ["runtime_consumption_selected_final_snapshot_path"],
        serde_json::json!(malformed_snapshot_path)
    );
}

#[test]
fn doctor_json_keeps_global_final_fallback_without_effective_run() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let global_snapshot_path = format!("{runtime_consumption_dir}/final-2026-07-10T04-00-01Z.json");
    write_final_snapshot(
        &state_dir,
        "final-2026-07-10T04-00-01Z.json",
        final_snapshot_for_run(
            &global_snapshot_path,
            "global-fallback-run",
            case10_closure_admission_record(),
        ),
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
        .expect("doctor blockers should be an array");
    let trace_blocker_codes = doctor_json["trace_evidence"]["blocker_codes"]
        .as_array()
        .expect("trace blockers should be an array");

    for blocker in [
        "incomplete_release_admission_operator_evidence",
        "missing_retrieval_trust_operator_evidence",
        "missing_retrieval_trust_signal_operator_evidence",
        "missing_retrieval_trust_source_operator_evidence",
    ] {
        assert!(
            !blocker_codes
                .iter()
                .any(|code| code.as_str() == Some(blocker)),
            "{blocker} must stay clear for no-run global fallback: {doctor_json}"
        );
    }
    assert!(!trace_blocker_codes
        .iter()
        .any(|code| code == "trace_incomplete"));
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]["effective_run_id"],
        serde_json::Value::Null
    );
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(global_snapshot_path)
    );
    assert_eq!(
        doctor_json["artifact_refs"]["retrieval_trust_signal"]["citation"],
        serde_json::json!(global_snapshot_path)
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]["runtime_consumption_latest_snapshot_path"],
        serde_json::json!(global_snapshot_path)
    );
    assert_eq!(
        doctor_json["trace_evidence"]["root_trace"]
            ["runtime_consumption_selected_final_snapshot_path"],
        serde_json::Value::Null
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

        let _release_admission_blocked = blocker_codes
            .iter()
            .any(|code| code.as_str() == Some("incomplete_release_admission_operator_evidence"));
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
fn consume_bundle_check_without_final_snapshot_fails_closed_on_retrieval_trust() {
    let (_project_root, state_dir) = project_bound_state_dir();

    let reset = vida()
        .args(["state", "reset", "--archive", "--reinit", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("state reset should run");
    assert!(
        reset.status.success(),
        "state reset should succeed: {}",
        String::from_utf8_lossy(&reset.stderr)
    );

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run after state reset");
    assert!(
        boot.status.success(),
        "boot should succeed after state reset: stdout={} stderr={}",
        String::from_utf8_lossy(&boot.stdout),
        String::from_utf8_lossy(&boot.stderr)
    );

    sync_protocol_binding(&state_dir);

    let bundle_check = vida()
        .args(["taskflow", "consume", "bundle", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("bundle check should run");
    assert!(
        !bundle_check.stdout.is_empty(),
        "bundle check should emit JSON even when the minimal fixture has blockers: stderr={}",
        String::from_utf8_lossy(&bundle_check.stderr)
    );
    let bundle_check_json: serde_json::Value =
        serde_json::from_slice(&bundle_check.stdout).expect("bundle check json should parse");
    let blocker_codes = bundle_check_json["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array");
    for expected in [
        "missing_retrieval_trust_evidence_acl",
        "missing_retrieval_trust_evidence_acl_context",
        "missing_retrieval_trust_evidence_acl_propagation",
        "missing_retrieval_trust_evidence_citation",
        "missing_retrieval_trust_evidence_freshness",
        "missing_retrieval_trust_evidence_freshness_posture",
        "missing_retrieval_trust_evidence_source",
        "missing_retrieval_trust_evidence_source_registry_ref",
    ] {
        assert!(
            blocker_codes
                .iter()
                .any(|code| code.as_str() == Some(expected)),
            "bundle check must fail closed with `{expected}` when no final snapshot provides retrieval trust: {blocker_codes:?}"
        );
    }

    let snapshot_path = bundle_check_json["snapshot_path"]
        .as_str()
        .expect("bundle check should write a snapshot");
    let snapshot_body =
        std::fs::read_to_string(snapshot_path).expect("bundle check snapshot should be readable");
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_body).expect("bundle check snapshot should parse");
    assert_eq!(
        snapshot["bundle"]["cache_delivery_contract"]["retrieval_trust_evidence"],
        serde_json::json!({})
    );
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
            "vida lane retire {run_id} --receipt-id {run_id} --reason \"missing TaskFlow task stale run\" --json"
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
    assert!(
        dispatch_json["parallelization_planner"]["materializes_packets"].is_null()
            || dispatch_json["parallelization_planner"]["materializes_packets"] == false
    );
    assert!(
        dispatch_json["flow_projection"]["status"].is_null()
            || dispatch_json["flow_projection"]["status"] == "blocked"
    );
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
fn projection_surfaces_fail_closed_for_closed_task_downstream_handoff_after_exception_takeover() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "zzzz-closed-task-downstream-handoff";
    let parent_id = "closed-task-downstream-handoff-parent";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");
    sync_protocol_binding(&state_dir);
    create_session_triage_task(
        &state_dir,
        parent_id,
        "Closed task downstream handoff parent",
        "epic",
        "closed",
        "1",
        None,
    );
    create_session_triage_task(
        &state_dir,
        run_id,
        "Closed task downstream handoff",
        "task",
        "closed",
        "1",
        Some(parent_id),
    );

    persist_host_bridge_lane_receipt_with_target_and_active_node_and_downstream_state(
        &state_dir,
        run_id,
        "architect",
        "architect",
        "internal_subagents",
        "executed",
        "lane_completed",
        "",
        "architect_complete",
        "true",
        "packet_ready",
    );

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert_success(
        &run_graph,
        "run-graph status should return structured blocked status for closed-task stale run",
    );
    let run_graph_json: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run-graph json should parse");
    assert_eq!(run_graph_json["status"], "blocked");
    assert_eq!(
        run_graph_json["projection_truth"]["stale_state_suspected"],
        true
    );
    let run_graph_text =
        serde_json::to_string(&run_graph_json).expect("run-graph json should render");
    assert!(
        run_graph_text.contains("vida task reconcile-closed-runs"),
        "run-graph status should expose the canonical closed-run repair action: {run_graph_text}"
    );
    assert_eq!(
        run_graph_json["blocker_codes"],
        serde_json::json!(["closed_task_active_run_projection_mismatch"])
    );
    assert_eq!(
        run_graph_json["projection_truth"]["stale_state_suspected"],
        true
    );
    assert_eq!(
        run_graph_json["projection_truth"]["next_lawful_operator_action"],
        "vida task reconcile-closed-runs --limit 25"
    );
    assert!(
        !run_graph_json["next_actions"]
            .to_string()
            .contains("consume continue"),
        "closed task run-graph status must not recommend downstream consume: {run_graph_json}"
    );
    assert!(
        !run_graph_json["next_actions"]
            .to_string()
            .contains("--execute-dispatch"),
        "closed task run-graph status must not recommend carrier dispatch: {run_graph_json}"
    );

    let _ = std::fs::remove_dir_all(project_root);
}

#[test]
fn dev_team_materialization_blocks_missing_owned_paths_before_packet_write() {
    let state_dir = boot_session_triage_state();
    sync_protocol_binding(&state_dir);
    create_session_triage_task(
        &state_dir,
        "missing-owned-paths-epic",
        "Missing owned paths epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "missing-owned-paths-task",
        "Missing owned paths task",
        "task",
        "in_progress",
        "0",
        Some("missing-owned-paths-epic"),
    );

    let dispatch = vida()
        .args([
            "agent",
            "dispatch-next",
            "--dev-team",
            "--materialize-packets",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("dispatch-next json should run");
    assert!(
        !dispatch.status.success(),
        "dispatch-next should fail closed before packet write"
    );
    let dispatch_json: serde_json::Value =
        serde_json::from_slice(&dispatch.stdout).expect("dispatch-next json should parse");
    assert_eq!(dispatch_json["status"], "blocked");
    assert!(
        dispatch_json["blocker_codes"]
            .as_array()
            .expect("blocker codes should be an array")
            .iter()
            .any(|code| code == "dispatch_packet_contract_invalid"),
        "dispatch-next should expose dispatch_packet_contract_invalid: {dispatch_json}"
    );
    assert_eq!(dispatch_json["packet_materialization"]["status"], "blocked");
    assert_eq!(
        dispatch_json["packet_materialization"]["materializes_packets"],
        false
    );
    assert_eq!(
        dispatch_json["packet_materialization"]["artifacts"],
        serde_json::json!([])
    );
    let errors = dispatch_json["packet_materialization"]["errors"]
        .as_array()
        .expect("packet materialization errors should be an array");
    assert!(
        errors.iter().any(|error| {
            error["task_id"] == "missing-owned-paths-task"
                && error["missing_fields"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|field| field == "owned_paths")
        }),
        "missing owned_paths should be reported before packet artifact write: {dispatch_json}"
    );
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let written_packets = std::fs::read_dir(&packet_dir)
        .ok()
        .into_iter()
        .flatten()
        .count();
    assert_eq!(
        written_packets, 0,
        "no dispatch packet should be written when required owned_paths are missing"
    );
    let _ = std::fs::remove_dir_all(state_dir);
}

#[test]
fn dev_team_dispatch_next_json_preview_does_not_materialize_packets_without_flag() {
    let state_dir = boot_session_triage_state();
    sync_protocol_binding(&state_dir);
    create_session_triage_task(
        &state_dir,
        "preview-only-epic",
        "Preview only epic",
        "epic",
        "open",
        "0",
        None,
    );
    create_session_triage_task(
        &state_dir,
        "preview-only-task",
        "Preview only task",
        "task",
        "in_progress",
        "0",
        Some("preview-only-epic"),
    );

    let dispatch = vida()
        .args(["agent", "dispatch-next", "--dev-team", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("dispatch-next preview json should run");
    assert!(
        dispatch.status.success(),
        "dispatch-next preview should succeed: stderr={} stdout={}",
        String::from_utf8_lossy(&dispatch.stderr),
        String::from_utf8_lossy(&dispatch.stdout)
    );
    let dispatch_stdout = String::from_utf8_lossy(&dispatch.stdout);
    let dispatch_json: serde_json::Value =
        serde_json::from_slice(&dispatch.stdout).expect("dispatch-next json should parse");
    assert_eq!(dispatch_json["status"], "pass");
    assert_eq!(dispatch_json["mode"], "preview-dev-team");
    assert_eq!(
        dispatch_json["packet_materialization"]["status"],
        "not_requested"
    );
    assert_eq!(dispatch_json["packet_materialization"]["requested"], false);
    assert_eq!(
        dispatch_json["packet_materialization"]["materializes_packets"],
        false
    );
    assert_eq!(
        dispatch_json["packet_materialization"]["artifacts"],
        serde_json::json!([])
    );
    assert!(
        !dispatch_stdout.contains("dispatch_packet_path"),
        "preview-only dispatch must not reference dispatch packet paths: {dispatch_stdout}"
    );
    let packet_dir = format!("{state_dir}/runtime-consumption/dispatch-packets");
    let written_packets = std::fs::read_dir(&packet_dir)
        .ok()
        .into_iter()
        .flatten()
        .count();
    assert_eq!(
        written_packets, 0,
        "preview-only dispatch must not write dispatch packet artifacts"
    );
    let _ = std::fs::remove_dir_all(state_dir);
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
            "vida lane retire {run_id} --receipt-id {run_id} --reason \"missing TaskFlow task stale run\" --json"
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

    let default_toon_consume = vida()
        .args(["taskflow", "consume", "continue"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("default consume continue should run");
    assert!(
        !default_toon_consume.status.success(),
        "default consume continue should fail closed for latest missing task authority"
    );
    let default_toon_stdout = String::from_utf8_lossy(&default_toon_consume.stdout);
    assert_not_json_output("vida taskflow consume continue", &default_toon_stdout);
    assert_no_raw_terminal_controls("vida taskflow consume continue", &default_toon_stdout);
    assert!(
        default_toon_stdout.contains("stale_missing_task_run_graph"),
        "default consume continue should expose the shared stale-run blocker: {default_toon_stdout}"
    );
    assert!(
        default_toon_stdout.contains("vida lane retire"),
        "default consume continue should expose the shared repair action: {default_toon_stdout}"
    );
    assert!(
        !default_toon_stdout.contains("--json"),
        "default next action should not bias operators toward --json: {default_toon_stdout}"
    );

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

#[test]
fn projection_surfaces_fail_closed_for_receipt_backed_missing_execution_row_host_bridge() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "zzzz-run-host-bridge-missing-execution-row";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");
    sync_protocol_binding(&state_dir);
    persist_host_bridge_lane_receipt_with_target(
        &state_dir,
        run_id,
        "coach",
        "tester",
        "bridge_request_pending",
        "lane_open",
        "host_tool_bridge_adapter_required",
        "coach_blocked",
    );
    delete_run_graph_row_with_helper(&state_dir, "execution_plan_state", run_id);

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(
        run_graph.status.success(),
        "run-graph status should return a structured fail-closed envelope instead of generic MissingTask: stderr={}",
        String::from_utf8_lossy(&run_graph.stderr)
    );
    let run_graph_json: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run-graph json should parse");
    assert_eq!(run_graph_json["status"], "blocked");
    assert_eq!(
        run_graph_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );
    assert_eq!(run_graph_json["run_graph_status"]["active_node"], "coach");
    assert_eq!(
        run_graph_json["run_graph_status"]["checkpoint_kind"],
        "missing_execution_plan_state"
    );
    assert_eq!(
        run_graph_json["projection_truth"]["stale_state_suspected"],
        true
    );

    let recovery = vida()
        .args(["taskflow", "recovery", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery status should run");
    assert_failure(
        &recovery,
        "recovery status should fail closed for missing execution row",
    );
    let recovery_json: serde_json::Value =
        serde_json::from_slice(&recovery.stdout).expect("recovery json should parse");
    assert_eq!(recovery_json["status"], "blocked");
    assert_eq!(
        recovery_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );

    let orchestrator = vida()
        .args(["orchestrator-init", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("orchestrator-init should run");
    assert_success(
        &orchestrator,
        "orchestrator-init should return structured blocked status for missing execution row",
    );
    let orchestrator_json: serde_json::Value =
        serde_json::from_slice(&orchestrator.stdout).expect("orchestrator json should parse");
    assert!(
        matches!(
            orchestrator_json["status"].as_str(),
            Some("blocked") | Some("pending")
        ),
        "orchestrator-init should not pass while the latest receipt-backed run has no execution row: {orchestrator_json}"
    );
    assert_eq!(
        orchestrator_json["continuation_binding"]["continuation_allowed"],
        false
    );
    assert_eq!(
        orchestrator_json["sequential_vs_parallel_posture"],
        "unknown_until_run_graph_blocker_resolved"
    );

    let consume = vida()
        .args([
            "taskflow", "consume", "continue", "--run-id", run_id, "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume continue should run");
    assert_failure(
        &consume,
        "consume continue should fail closed for missing execution row",
    );
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume.stdout).expect("consume json should parse");
    assert_eq!(consume_json["status"], "blocked");
    assert_eq!(
        consume_json["blocker_codes"],
        serde_json::json!(["stale_missing_task_run_graph"])
    );
    assert!(
        !String::from_utf8_lossy(&consume.stderr).contains("Failed to read run-graph status"),
        "consume continue must not leak generic run-graph MissingTask stderr"
    );

    let _ = std::fs::remove_dir_all(project_root);
}

#[test]
fn open_active_blocked_receipt_mismatch_does_not_recommend_lane_retire() {
    let (project_root, state_dir) = project_bound_state_dir();
    let parent_id = "zzzz-open-active-blocked-mismatch-parent";
    let run_id = "zzzz-open-active-blocked-mismatch";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");
    sync_protocol_binding(&state_dir);
    create_session_triage_task(
        &state_dir,
        parent_id,
        "Open active blocked mismatch parent",
        "epic",
        "open",
        "1",
        None,
    );
    create_session_triage_task(
        &state_dir,
        run_id,
        "Open active blocked mismatch task",
        "task",
        "in_progress",
        "1",
        Some(parent_id),
    );
    persist_host_bridge_lane_receipt_with_target_and_active_node(
        &state_dir,
        run_id,
        "autotester",
        "designer",
        "developer",
        "blocked",
        "autotester_lane",
        "host_bridge_completion_result_blocked",
        "designer_blocked",
    );

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(
        !run_graph.stdout.is_empty(),
        "run-graph status should emit structured json for open active mismatch: stderr={}",
        String::from_utf8_lossy(&run_graph.stderr)
    );
    let run_graph_json: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run-graph json should parse");
    assert_eq!(
        run_graph_json["projection_truth"]["stale_state_suspected"],
        false
    );
    assert_eq!(run_graph_json["run_graph_status"]["status"], "blocked");
    assert_eq!(
        run_graph_json["run_graph_status"]["active_node"],
        "autotester"
    );
    assert_eq!(
        run_graph_json["run_graph_status"]["lifecycle_stage"],
        "autotester_blocked"
    );
    let run_graph_text =
        serde_json::to_string(&run_graph_json).expect("run-graph json should render");
    assert!(
        !run_graph_text.contains("vida lane retire"),
        "open active blocked mismatch must not recommend lane retire: {run_graph_text}"
    );
    assert!(
        !run_graph_text.contains("exception-takeover"),
        "open active blocked mismatch must not recommend exception takeover as the recovery path: {run_graph_text}"
    );

    let recovery = vida()
        .args(["taskflow", "recovery", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery status should run");
    assert!(
        !recovery.stdout.is_empty(),
        "recovery status should emit structured json for open active mismatch: stderr={}",
        String::from_utf8_lossy(&recovery.stderr)
    );
    let recovery_json: serde_json::Value =
        serde_json::from_slice(&recovery.stdout).expect("recovery json should parse");
    assert_eq!(recovery_json["status"], "blocked");
    let recovery_text = serde_json::to_string(&recovery_json).expect("recovery json should render");
    assert!(
        !recovery_text.contains("exception-takeover"),
        "open active blocked mismatch recovery must not recommend exception takeover: {recovery_text}"
    );
    let _ = std::fs::remove_dir_all(project_root);
}

#[test]
fn recovery_latest_open_task_stale_receipt_does_not_recommend_unretirable_lane() {
    let (project_root, state_dir) = project_bound_state_dir();
    let parent_id = "zzzz-recovery-open-stale-parent";
    let run_id = "zzzz-recovery-open-stale-receipt";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");
    sync_protocol_binding(&state_dir);
    create_session_triage_task(
        &state_dir,
        parent_id,
        "Recovery open stale parent",
        "epic",
        "open",
        "1",
        None,
    );
    create_session_triage_task(
        &state_dir,
        run_id,
        "Recovery open stale task",
        "task",
        "in_progress",
        "1",
        Some(parent_id),
    );
    persist_host_bridge_lane_receipt_with_target_and_active_node(
        &state_dir,
        run_id,
        "autotester",
        "designer",
        "developer",
        "blocked",
        "autotester_lane",
        "host_bridge_completion_result_blocked",
        "designer_blocked",
    );

    let recovery_latest = vida()
        .args(["taskflow", "recovery", "latest", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery latest should run");
    assert_failure(
        &recovery_latest,
        "recovery latest should fail closed for open-task stale receipt",
    );
    let recovery_json: serde_json::Value =
        serde_json::from_slice(&recovery_latest.stdout).expect("recovery latest json should parse");
    let recovery_text = serde_json::to_string(&recovery_json).expect("recovery json should render");
    assert!(
        !recovery_text.contains("vida lane retire"),
        "recovery latest must not recommend unretirable lane retire for open task: {recovery_text}"
    );

    let retire = vida()
        .args([
            "lane",
            "retire",
            run_id,
            "--receipt-id",
            run_id,
            "--reason",
            "stale blocked dispatch receipt",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("lane retire should run");
    assert_failure(&retire, "lane retire should reject open task stale receipt");
    let retire_json: serde_json::Value =
        serde_json::from_slice(&retire.stdout).expect("lane retire json should parse");
    assert_eq!(retire_json["status"], "blocked");
    assert_eq!(
        retire_json["blocker_codes"],
        serde_json::json!(["lane_retire_task_not_closed"])
    );

    let _ = std::fs::remove_dir_all(project_root);
}

#[test]
fn run_graph_status_recovers_stale_blocked_receipt_without_exception_takeover() {
    let (project_root, state_dir) = project_bound_state_dir();
    let run_id = "zzzz-run-stale-blocked-autotester-receipt";
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert_success(&boot, "boot");
    sync_protocol_binding(&state_dir);
    persist_host_bridge_lane_receipt_with_target_and_active_node(
        &state_dir,
        run_id,
        "autotester",
        "designer",
        "developer",
        "blocked",
        "autotester_lane",
        "host_bridge_completion_result_blocked",
        "designer_blocked",
    );

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(
        !run_graph.stdout.is_empty(),
        "run-graph status should emit structured json even when blocked: stderr={}",
        String::from_utf8_lossy(&run_graph.stderr)
    );
    let run_graph_json: serde_json::Value =
        serde_json::from_slice(&run_graph.stdout).expect("run-graph json should parse");
    assert_eq!(run_graph_json["status"], "blocked");

    let recovery = vida()
        .args(["taskflow", "recovery", "status", run_id, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery status should run");
    assert_failure(
        &recovery,
        "recovery status should fail closed for stale blocked receipt",
    );
    let recovery_json: serde_json::Value =
        serde_json::from_slice(&recovery.stdout).expect("recovery json should parse");
    assert_eq!(recovery_json["status"], "blocked");
    let recovery_text = serde_json::to_string(&recovery_json).expect("recovery json should render");
    assert!(
        !recovery_text.contains("exception-takeover"),
        "stale blocked receipt recovery must not recommend exception takeover as the only lawful path: {recovery_text}"
    );
    assert!(
        recovery_text.contains("vida lane retire") || recovery_text.contains("packet repair"),
        "stale blocked receipt recovery should expose a lawful stale-state repair action: {recovery_text}"
    );
    assert_eq!(
        recovery_json["projection_truth"]["stale_state_suspected"],
        true
    );

    let orchestrator = vida()
        .args(["orchestrator-init", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("orchestrator-init should run");
    assert!(
        !orchestrator.stdout.is_empty(),
        "orchestrator-init should emit structured json even when blocked: stderr={}",
        String::from_utf8_lossy(&orchestrator.stderr)
    );
    let orchestrator_json: serde_json::Value =
        serde_json::from_slice(&orchestrator.stdout).expect("orchestrator json should parse");
    let orchestrator_text =
        serde_json::to_string(&orchestrator_json).expect("orchestrator json should render");
    assert!(
        !orchestrator_text.contains("exception-takeover"),
        "orchestrator-init must not leave exception takeover as the only continuation for stale blocked receipt: {orchestrator_text}"
    );
    assert!(
        orchestrator_text.contains("vida task reconcile-closed-runs")
            || orchestrator_text.contains("vida lane retire"),
        "orchestrator-init should surface a lawful non-exception recovery action: {orchestrator_text}"
    );
    let _ = std::fs::remove_dir_all(project_root);
}
