use std::process::{Command, Output};

use serde_json::Value;

#[path = "support/runtime_consumption.rs"]
mod runtime_consumption_support;

use runtime_consumption_support::PersistentRuntimeFixture;

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} should succeed: stdout={}; stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_json(fixture: &PersistentRuntimeFixture, args: &[&str]) -> Value {
    let (json, _) = fixture.json_allow_failure(args);
    json
}

fn run_json_success(fixture: &PersistentRuntimeFixture, args: &[&str]) -> Value {
    fixture.json_success(args)
}

fn zombie_d_matrix_note(doubts: Value) -> String {
    let matrix = serde_json::json!({
        "schema_version": 1,
        "categories": {
            "Z": {"status": "pass", "evidence_refs": ["z"]},
            "O": {"status": "pass", "evidence_refs": ["o"]},
            "M": {"status": "na", "reason": "single fixture contract"},
            "B": {"status": "pass", "evidence_refs": ["b"]},
            "I": {"status": "pass", "evidence_refs": ["i"]},
            "E": {"status": "pass", "evidence_refs": ["e"]},
            "S": {"status": "pass", "evidence_refs": ["s"]}
        },
        "doubts": doubts
    });
    format!(
        "task_proof_evidence:\n  proof_target: zombie_d_matrix\n  result: pass\n  evidence: {matrix}"
    )
}

fn zombie_d_rpc_matrix_note() -> String {
    let matrix = serde_json::json!({
        "schema_version": 1,
        "metadata": {
            "schema_version": 1,
            "applicable_categories": ["R", "P", "C"]
        },
        "categories": {
            "Z": {"status": "pass", "evidence_refs": ["z"]},
            "O": {"status": "pass", "evidence_refs": ["o"]},
            "M": {"status": "na", "reason": "single fixture contract"},
            "B": {"status": "pass", "evidence_refs": ["b"]},
            "I": {"status": "pass", "evidence_refs": ["i"]},
            "E": {"status": "pass", "evidence_refs": ["e"]},
            "S": {"status": "pass", "evidence_refs": ["s"]},
            "R": {"status": "pass", "evidence_refs": ["replay-test"]},
            "P": {"status": "pass", "evidence_refs": ["persistence-test"]},
            "C": {"status": "pass", "evidence_refs": ["cross-surface-test"]}
        },
        "doubts": []
    });
    format!(
        "task_proof_evidence:\n  proof_target: zombie_d_matrix\n  result: pass\n  evidence: {matrix}"
    )
}

fn assert_zombie_d_operator_shape(value: &Value, label: &str) {
    vida_test_support::assert_release1_operator_shape(label, value);
    assert!(matches!(
        value["status"].as_str(),
        Some("pass") | Some("blocked")
    ));
    if value["status"] == "blocked" {
        assert!(
            value["blocker_codes"]
                .as_array()
                .is_some_and(|codes| !codes.is_empty()),
            "{label} blocked verdict must expose blocker_codes: {value}"
        );
        assert!(
            value["next_actions"]
                .as_array()
                .is_some_and(|actions| !actions.is_empty()),
            "{label} blocked verdict must expose next_actions: {value}"
        );
    }
}

fn assert_zombie_d_host_bridge_shape(value: &Value) {
    assert_eq!(value["surface"], "vida agent host-bridge");
    assert!(matches!(
        value["status"].as_str(),
        Some("pass") | Some("blocked")
    ));
    assert!(value["blocker_codes"].is_array());
    assert!(value["next_actions"].is_array());
    assert!(value["artifact_refs"].is_object());
    for field in ["status", "blocker_codes", "next_actions", "artifact_refs"] {
        assert_eq!(value["shared_fields"][field], value[field]);
        assert_eq!(value["operator_contracts"][field], value[field]);
    }
    assert!(value["host_bridge"]["required_result_fields"]
        .as_array()
        .is_some_and(|fields| fields.iter().any(|field| field == "allowed_next_node")));
    if value["status"] == "blocked" {
        assert!(value["blocker_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()));
        assert!(value["next_actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()));
    }
}

fn run_failure(fixture: &PersistentRuntimeFixture, args: &[&str]) -> Output {
    let output = fixture.capture(args);
    assert!(
        !output.status.success(),
        "{} should fail: stdout={}; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn json_string_vec(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{pointer} should be an array: {value:#}"))
        .iter()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn delete_run_graph_row_with_helper(fixture: &PersistentRuntimeFixture, table: &str, run_id: &str) {
    let helper = std::env::current_exe().expect("current test binary should resolve");
    let output = Command::new(helper)
        .args([
            "--ignored",
            "--exact",
            "runtime_delete_run_graph_row_helper_process",
            "--nocapture",
        ])
        .env(
            runtime_consumption_support::RUN_GRAPH_DELETE_STATE_DIR_ENV,
            fixture.state_dir_string(),
        )
        .env(
            runtime_consumption_support::RUN_GRAPH_DELETE_TABLE_ENV,
            table,
        )
        .env(
            runtime_consumption_support::RUN_GRAPH_DELETE_RUN_ID_ENV,
            run_id,
        )
        .output()
        .expect("runtime delete helper process should run");
    assert_success(&output, "runtime delete helper process");
}

#[test]
fn task_runtime_workflows_assumption_doubt_test_stale_missing_task_identity_variant_fails_closed() {
    let fixture = PersistentRuntimeFixture::state_only("stale-missing-task-identity");
    fixture.boot();

    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "stale-parent",
            "Stale Parent",
            "--type",
            "epic",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "stale-run-task",
            "Stale Run Task",
            "--type",
            "task",
            "--parent-id",
            "stale-parent",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            "stale-run-task",
            "--json",
        ],
    );
    delete_run_graph_row_with_helper(&fixture, "task", "stale-run-task");

    let run_graph = run_json(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "status",
            "stale-run-task",
            "--json",
        ],
    );
    let run_graph_blockers = json_string_vec(&run_graph, "/blocker_codes");
    assert!(
        run_graph_blockers
            .iter()
            .any(|code| code == "stale_missing_task_run_graph"),
        "missing TaskFlow task should fail closed as stale run graph: {run_graph:#}"
    );

    let recovery = run_json(
        &fixture,
        &["taskflow", "recovery", "status", "stale-run-task", "--json"],
    );
    let recovery_blockers = json_string_vec(&recovery, "/blocker_codes");
    assert!(
        recovery_blockers
            .iter()
            .any(|code| code == "stale_missing_task_run_graph"),
        "recovery should preserve stale missing task blocker: {recovery:#}"
    );
}

#[test]
fn closed_task_stale_host_bridge_run_projection_is_not_active_recovery() {
    let fixture = PersistentRuntimeFixture::state_only("closed-task-stale-host-bridge-run");
    fixture.boot();

    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "closed-host-bridge-parent",
            "Closed Host Bridge Parent",
            "--type",
            "epic",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "closed-host-bridge-run",
            "Closed Host Bridge Run",
            "--type",
            "runtime_defect",
            "--status",
            "in_progress",
            "--parent-id",
            "closed-host-bridge-parent",
            "--json",
        ],
    );
    let seed = run_json_success(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "seed",
            "closed-host-bridge-run",
            "continue development",
            "--json",
        ],
    );
    assert_eq!(
        seed["payload"]["status"]["run_id"],
        "closed-host-bridge-run"
    );
    let advance = run_json_success(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "advance",
            "closed-host-bridge-run",
            "--json",
        ],
    );
    assert_eq!(
        advance["payload"]["status"]["run_id"],
        "closed-host-bridge-run"
    );
    let recovery = run_json(
        &fixture,
        &[
            "taskflow",
            "recovery",
            "status",
            "closed-host-bridge-run",
            "--json",
        ],
    );
    assert_eq!(recovery["status"], "blocked");
    assert!(json_string_vec(&recovery, "/blocker_codes")
        .iter()
        .any(|code| code == "open_delegated_cycle"));
    assert_eq!(
        recovery["recovery"]["delegation_gate"]["delegated_cycle_open"],
        true
    );
    assert_eq!(
        recovery["recovery"]["delegation_gate"]["delegated_cycle_state"],
        "handoff_pending"
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "close",
            "closed-host-bridge-run",
            "--reason",
            "closed before stale host bridge projection recovery",
            "--json",
        ],
    );

    let run_graph = run_json(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "status",
            "closed-host-bridge-run",
            "--json",
        ],
    );
    assert_eq!(run_graph["status"], "pass");
    assert!(
        run_graph["blocker_codes"]
            .as_array()
            .is_some_and(|codes| codes.is_empty()),
        "closed task run-graph projection must not expose active blockers: {run_graph:#}"
    );
    assert_eq!(run_graph["run_graph_status"]["active_node"], "closure");
    assert_eq!(
        run_graph["run_graph_status"]["lifecycle_stage"],
        "closure_complete"
    );
    assert_eq!(
        run_graph["run_graph_status"]["policy_gate"],
        "closed_run_archived"
    );
    assert_eq!(run_graph["run_graph_status"]["context_state"], "sealed");
    assert_eq!(run_graph["run_graph_status"]["recovery_ready"], false);
    assert_eq!(run_graph["delegation_gate"]["delegated_cycle_open"], false);
    assert_eq!(
        run_graph["delegation_gate"]["delegated_cycle_state"],
        "clear"
    );

    let recovery = run_json(
        &fixture,
        &[
            "taskflow",
            "recovery",
            "status",
            "closed-host-bridge-run",
            "--json",
        ],
    );
    let recovery_blockers = json_string_vec(&recovery, "/blocker_codes");
    assert!(
        recovery_blockers
            .iter()
            .any(|code| code == "closed_task_active_run_projection_mismatch"),
        "raw closed task recovery must remain fail-closed: {recovery:#}"
    );
    assert!(
        recovery["next_actions"]
            .to_string()
            .contains("vida task reconcile-closed-runs --limit 25"),
        "raw closed task recovery should name the stable reconcile command: {recovery:#}"
    );
}

#[test]
#[ignore = "helper process for task_runtime_workflows persisted-state row deletion"]
fn runtime_delete_run_graph_row_helper_process() {
    if std::env::var(runtime_consumption_support::RUN_GRAPH_DELETE_STATE_DIR_ENV).is_ok() {
        runtime_consumption_support::delete_run_graph_row_from_env();
    }
}

#[test]
fn task_create_duplicate_id_with_invalid_parent_kind_sentinel_uses_generic_error() {
    let fixture = PersistentRuntimeFixture::state_only("duplicate-invalid-parent-kind-sentinel");
    fixture.boot();

    let duplicate_sentinel_id = "duplicate_invalid_parent_child_kind";
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            duplicate_sentinel_id,
            "Duplicate Sentinel",
            "--type",
            "epic",
            "--json",
        ],
    );

    let duplicate_sentinel = run_failure(
        &fixture,
        &[
            "task",
            "create",
            duplicate_sentinel_id,
            "Duplicate Sentinel",
            "--type",
            "epic",
            "--json",
        ],
    );
    let duplicate_sentinel_text = format!(
        "{}{}",
        String::from_utf8_lossy(&duplicate_sentinel.stdout),
        String::from_utf8_lossy(&duplicate_sentinel.stderr)
    );
    assert!(duplicate_sentinel_text.contains("task already exists"));
    assert!(!duplicate_sentinel_text.contains("dependency_graph_issues"));
    assert!(!duplicate_sentinel_text.contains("graph_issue"));
}

#[test]
fn task_runtime_workflows_treat_step_as_execution_only_and_subtask_as_work_item() {
    let fixture = PersistentRuntimeFixture::state_only("step-subtask-workflow");
    fixture.boot();

    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-epic",
            "Workflow Epic",
            "--type",
            "epic",
            "--execution-mode",
            "container_only",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-task",
            "Workflow Task",
            "--type",
            "task",
            "--parent-id",
            "workflow-epic",
            "--json",
        ],
    );
    let subtask = run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-subtask",
            "Workflow Subtask",
            "--type",
            "subtask",
            "--parent-id",
            "workflow-task",
            "--json",
        ],
    );
    assert_eq!(
        subtask["task"]["work_item_kind"]["canonical_issue_type"],
        "subtask"
    );
    assert_eq!(subtask["task"]["work_item_kind"]["flow_bindable"], true);

    let step = run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-step",
            "Workflow Step",
            "--type",
            "step",
            "--parent-id",
            "workflow-subtask",
            "--json",
        ],
    );
    assert_eq!(
        step["task"]["work_item_kind"]["canonical_issue_type"],
        "step"
    );
    assert_eq!(step["task"]["work_item_kind"]["flow_bindable"], false);

    let todo_alias = run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-todo-alias",
            "Workflow Todo Alias",
            "--type",
            "todo",
            "--status",
            "closed",
            "--parent-id",
            "workflow-task",
            "--json",
        ],
    );
    assert_eq!(todo_alias["task"]["issue_type"], "step");
    assert_eq!(
        todo_alias["task"]["work_item_kind"]["canonical_issue_type"],
        "step"
    );

    let invalid_subtask = run_failure(
        &fixture,
        &[
            "task",
            "create",
            "invalid-subtask",
            "Invalid Subtask",
            "--type",
            "subtask",
            "--parent-id",
            "workflow-epic",
            "--json",
        ],
    );
    let invalid_text = format!(
        "{}{}",
        String::from_utf8_lossy(&invalid_subtask.stdout),
        String::from_utf8_lossy(&invalid_subtask.stderr)
    );
    assert!(invalid_text.contains("invalid_parent_child_kind"));
    let invalid_step_json = run_failure(
        &fixture,
        &[
            "task",
            "create",
            "invalid-step-json",
            "Invalid Step",
            "--type",
            "step",
            "--parent-id",
            "workflow-epic",
            "--json",
        ],
    );
    assert!(
        invalid_step_json.stderr.is_empty(),
        "json failure should not emit stderr: {}",
        String::from_utf8_lossy(&invalid_step_json.stderr)
    );
    let invalid_step_payload: Value =
        serde_json::from_slice(&invalid_step_json.stdout).expect("invalid step json payload");
    assert_eq!(invalid_step_payload["status"], "blocked");
    assert_eq!(
        invalid_step_payload["blocker_codes"],
        serde_json::json!(["dependency_graph_issues"])
    );
    assert_eq!(
        invalid_step_payload["graph_issue"]["issue_type"],
        "invalid_parent_child_kind"
    );
    assert!(invalid_step_payload["graph_issue"]["detail"]
        .as_str()
        .expect("graph issue detail")
        .contains("got `epic`"));
    assert!(invalid_step_payload["next_actions"][0]
        .as_str()
        .expect("next action")
        .contains("steps require a task or subtask parent"));
    let invalid_step_default = run_failure(
        &fixture,
        &[
            "task",
            "create",
            "invalid-step-default",
            "Invalid Step",
            "--type",
            "step",
            "--parent-id",
            "workflow-epic",
        ],
    );
    let invalid_step_default_text = format!(
        "{}{}",
        String::from_utf8_lossy(&invalid_step_default.stdout),
        String::from_utf8_lossy(&invalid_step_default.stderr)
    );
    assert!(invalid_step_default_text.contains("invalid_parent_child_kind"));
    assert!(invalid_step_default_text.contains("got `epic`"));
    assert!(invalid_step_default_text.contains("step work item"));

    let ready = run_json_success(&fixture, &["task", "ready", "--json"]);
    let ready_ids = ready["tasks"]
        .as_array()
        .expect("ready tasks should be array")
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect::<Vec<_>>();
    assert!(ready_ids.contains(&"workflow-subtask"));
    assert!(!ready_ids.contains(&"workflow-step"));
    assert!(!ready_ids.contains(&"workflow-todo-alias"));

    let progress = run_json_success(&fixture, &["task", "progress", "workflow-task", "--json"]);
    assert_eq!(progress["progress"]["descendant_count"], 1);
    assert_eq!(progress["progress"]["open_count"], 1);
    assert_eq!(progress["progress"]["closed_count"], 0);
    assert_eq!(
        progress["progress"]["status_counts"]["open"],
        serde_json::json!(1)
    );

    let status = run_json_success(&fixture, &["status", "--json"]);
    assert_eq!(status["taskflow_counts"]["total_count"], 3);
    assert_eq!(status["taskflow_counts"]["open_count"], 3);
    assert_eq!(status["taskflow_counts"]["ready_count"], 2);

    let import_path = fixture.state_dir().join("taxonomy-alias-import.jsonl");
    let imported_todo = serde_json::json!({
        "id": "workflow-import-todo-alias",
        "title": "Workflow Imported Todo Alias",
        "status": "closed",
        "issue_type": "todo",
        "created_at": "1",
        "created_by": "test",
        "updated_at": "1",
        "source_repo": ".",
        "dependencies": [{
            "issue_id": "workflow-import-todo-alias",
            "depends_on_id": "workflow-task",
            "edge_type": "parent-child",
            "created_at": "1",
            "created_by": "test",
            "metadata": "{}",
            "thread_id": ""
        }]
    });
    let imported_subtask = serde_json::json!({
        "id": "workflow-import-subtask-alias",
        "title": "Workflow Imported Subtask Alias",
        "status": "closed",
        "issue_type": "sub_task",
        "created_at": "1",
        "created_by": "test",
        "updated_at": "1",
        "source_repo": ".",
        "dependencies": [{
            "issue_id": "workflow-import-subtask-alias",
            "depends_on_id": "workflow-task",
            "edge_type": "parent-child",
            "created_at": "1",
            "created_by": "test",
            "metadata": "{}",
            "thread_id": ""
        }]
    });
    std::fs::write(
        &import_path,
        format!("{}\n{}\n", imported_todo, imported_subtask),
    )
    .expect("write taxonomy alias import");
    run_json_success(
        &fixture,
        &[
            "task",
            "import-jsonl",
            import_path.to_str().expect("import path should be utf8"),
            "--json",
        ],
    );
    let imported_todo_show = run_json_success(
        &fixture,
        &["task", "show", "workflow-import-todo-alias", "--json"],
    );
    assert_eq!(imported_todo_show["task"]["issue_type"], "step");
    let imported_subtask_show = run_json_success(
        &fixture,
        &["task", "show", "workflow-import-subtask-alias", "--json"],
    );
    assert_eq!(imported_subtask_show["task"]["issue_type"], "subtask");

    let progress_after_import =
        run_json_success(&fixture, &["task", "progress", "workflow-task", "--json"]);
    assert_eq!(progress_after_import["progress"]["descendant_count"], 2);
    assert_eq!(progress_after_import["progress"]["open_count"], 1);
    assert_eq!(progress_after_import["progress"]["closed_count"], 1);

    let graph = run_json_success(&fixture, &["task", "validate-graph", "--json"]);
    assert_eq!(graph["status"], "pass");
}

#[test]
fn task_runtime_workflows_preserve_literal_metadata_values() {
    let fixture = PersistentRuntimeFixture::state_only("literal-metadata-workflow");
    fixture.boot();

    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "literal-epic",
            "Literal Metadata Epic",
            "--type",
            "epic",
            "--execution-mode",
            "container_only",
            "--json",
        ],
    );
    let created = run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "literal-task",
            "Literal Metadata Task",
            "--parent-id",
            "literal-epic",
            "--owned-path",
            "src/a,src/b",
            "--owned-path-literal",
            "docs/path,with,comma.md",
            "--acceptance-target",
            "alpha,beta",
            "--acceptance-target-literal",
            "One acceptance target, with commas, preserved",
            "--proof-target",
            "proof-a,proof-b",
            "--proof-target-literal",
            "Manual proof, with comma, preserved",
            "--json",
        ],
    );
    assert_eq!(
        created["task"]["planner_metadata"]["owned_paths"],
        serde_json::json!(["docs/path,with,comma.md", "src/a", "src/b"])
    );
    assert_eq!(
        created["task"]["planner_metadata"]["acceptance_targets"],
        serde_json::json!([
            "One acceptance target, with commas, preserved",
            "alpha",
            "beta"
        ])
    );
    assert_eq!(
        created["task"]["planner_metadata"]["proof_targets"],
        serde_json::json!(["Manual proof, with comma, preserved", "proof-a", "proof-b"])
    );

    let updated = run_json_success(
        &fixture,
        &[
            "task",
            "update",
            "literal-task",
            "--owned-path-literal",
            "docs/updated,path.md",
            "--acceptance-target-literal",
            "Updated acceptance, still one value",
            "--proof-target-literal",
            "Updated proof, still one value",
            "--json",
        ],
    );
    assert_eq!(
        updated["task"]["planner_metadata"]["owned_paths"],
        serde_json::json!(["docs/updated,path.md"])
    );
    assert_eq!(
        updated["task"]["planner_metadata"]["acceptance_targets"],
        serde_json::json!(["Updated acceptance, still one value"])
    );
    assert_eq!(
        updated["task"]["planner_metadata"]["proof_targets"],
        serde_json::json!(["Updated proof, still one value"])
    );

    let clear_conflict = run_failure(
        &fixture,
        &[
            "task",
            "update",
            "literal-task",
            "--proof-target-literal",
            "conflicting proof, still one value",
            "--clear-proof-targets",
            "--json",
        ],
    );
    let clear_conflict_text = format!(
        "{}{}",
        String::from_utf8_lossy(&clear_conflict.stdout),
        String::from_utf8_lossy(&clear_conflict.stderr)
    );
    assert!(clear_conflict_text.contains("--proof-target-literal"));

    let import_path = fixture.state_dir().join("literal-metadata-import.json");
    let import_payload = serde_json::json!({
        "tasks": [{
            "id": "literal-import-task",
            "title": "Literal Import Task",
            "type": "task",
            "parent_id": "literal-epic",
            "planner_metadata": {
                "owned_paths": ["docs/import,path.md"],
                "acceptance_targets": ["Imported acceptance, still one value"],
                "proof_targets": ["Imported proof, still one value"]
            }
        }]
    });
    std::fs::write(
        &import_path,
        serde_json::to_string_pretty(&import_payload).expect("serialize import payload"),
    )
    .expect("write literal metadata import");
    run_json_success(
        &fixture,
        &[
            "task",
            "import",
            "--file",
            import_path.to_str().expect("import path should be utf8"),
            "--json",
        ],
    );
    let imported = run_json_success(&fixture, &["task", "show", "literal-import-task", "--json"]);
    assert_eq!(
        imported["task"]["planner_metadata"]["owned_paths"],
        serde_json::json!(["docs/import,path.md"])
    );
    assert_eq!(
        imported["task"]["planner_metadata"]["acceptance_targets"],
        serde_json::json!(["Imported acceptance, still one value"])
    );
    assert_eq!(
        imported["task"]["planner_metadata"]["proof_targets"],
        serde_json::json!(["Imported proof, still one value"])
    );
}

#[test]
fn task_runtime_workflows_cover_isolated_task_lifecycle_and_reimport() {
    let fixture = PersistentRuntimeFixture::state_only("runtime-workflow");
    fixture.boot();

    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-epic",
            "Workflow Harness Epic",
            "--type",
            "epic",
            "--execution-mode",
            "container_only",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "workflow-task",
            "Workflow Harness Task",
            "--parent-id",
            "workflow-epic",
            "--execution-mode",
            "sequential",
            "--order-bucket",
            "workflow",
            "--conflict-domain",
            "workflow-e2e",
            "--owned-path",
            "crates/vida/tests/task_runtime_workflows.rs",
            "--acceptance-target",
            "isolated workflow lifecycle is covered",
            "--proof-target",
            "workflow proof",
            "--json",
        ],
    );
    let update = run_json_success(
        &fixture,
        &[
            "task",
            "update",
            "workflow-task",
            "--priority",
            "1",
            "--notes",
            "runtime workflow harness updated this task before scheduling",
            "--json",
        ],
    );
    assert_eq!(update["task"]["priority"], 1);

    let ready = run_json_success(&fixture, &["task", "ready", "--json"]);
    let ready_ids = ready["tasks"]
        .as_array()
        .expect("ready tasks should be array")
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect::<Vec<_>>();
    assert!(ready_ids.contains(&"workflow-task"));

    let scheduler = run_json(&fixture, &["taskflow", "scheduler", "dispatch", "--json"]);
    assert_eq!(scheduler["surface"], "vida taskflow scheduler dispatch");
    assert!(
        scheduler["selected_task_ids"]
            .as_array()
            .expect("selected_task_ids should be array")
            .iter()
            .any(|id| id.as_str() == Some("workflow-task")),
        "scheduler should select workflow-task: {scheduler:#}"
    );

    let attempt = run_json_success(
        &fixture,
        &[
            "task",
            "attempt",
            "record",
            "workflow-task",
            "--attempt-id",
            "workflow-attempt-1",
            "--stage-id",
            "implementation",
            "--backend",
            "local-harness",
            "--model-profile",
            "test",
            "--isolation",
            "readonly",
            "--status",
            "produced",
            "--artifact-ref",
            "workflow-artifact.json",
            "--json",
        ],
    );
    assert_eq!(attempt["status"], "pass");
    assert_eq!(attempt["attempt"]["attempt_id"], "workflow-attempt-1");

    let attempt_status = run_json_success(
        &fixture,
        &[
            "task",
            "attempt",
            "status",
            "workflow-task",
            "--stage-id",
            "implementation",
            "--json",
        ],
    );
    assert_eq!(attempt_status["status"], "pass");

    let verify = run_json_success(
        &fixture,
        &[
            "task",
            "verify",
            "workflow-task",
            "--source-fixed",
            "--tests-green",
            "--evidence",
            "workflow proof",
            "--json",
        ],
    );
    assert_eq!(verify["source_fixed"], true);
    assert_eq!(verify["tests_green"], true);

    run_json_success(
        &fixture,
        &[
            "task",
            "proof",
            "attach-evidence",
            "workflow-task",
            "--proof-target",
            "workflow proof",
            "--result",
            "pass",
            "--evidence",
            "workflow proof",
            "--json",
        ],
    );

    let dispatch_init = run_json_success(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            "workflow-task",
            "--json",
        ],
    );
    assert_eq!(dispatch_init["run_id"], "workflow-task");

    let run_graph = run_json_success(
        &fixture,
        &["taskflow", "run-graph", "status", "workflow-task", "--json"],
    );
    assert_eq!(run_graph["run_id"], "workflow-task");
    assert_eq!(run_graph["task_identity"]["run_id"], "workflow-task");
    assert_eq!(
        run_graph["task_identity"]["feature_epic_id"],
        "workflow-epic"
    );
    assert_eq!(run_graph["task_identity"]["dev_task_id"], "workflow-task");

    let recovery = run_json(
        &fixture,
        &["taskflow", "recovery", "status", "workflow-task", "--json"],
    );
    assert_eq!(recovery["run_id"], "workflow-task");
    assert_eq!(recovery["task_identity"]["run_id"], "workflow-task");
    assert_eq!(
        recovery["task_identity"]["feature_epic_id"],
        "workflow-epic"
    );
    assert_eq!(recovery["task_identity"]["dev_task_id"], "workflow-task");
    assert!(recovery["status"].is_string());

    run_json_success(
        &fixture,
        &[
            "task",
            "close",
            "workflow-task",
            "--reason",
            "workflow proof completed: create, update, ready, scheduler, attempt, verify, run-graph, recovery, close, export, and import paths are covered.",
            "--json",
        ],
    );

    let export_path = fixture.state_dir().join("workflow-export.jsonl");
    run_json_success(
        &fixture,
        &[
            "task",
            "export-jsonl",
            export_path.to_str().expect("export path should be utf8"),
            "--json",
        ],
    );
    assert!(export_path.exists());

    let import_fixture = PersistentRuntimeFixture::state_only("runtime-workflow-import");
    import_fixture.boot();
    run_json_success(
        &import_fixture,
        &[
            "task",
            "import-jsonl",
            export_path.to_str().expect("export path should be utf8"),
            "--json",
        ],
    );
    let imported = run_json_success(
        &import_fixture,
        &["task", "show", "workflow-task", "--json"],
    );
    assert_eq!(imported["task"]["status"], "closed");
    assert_eq!(
        imported["task"]["planner_metadata"]["owned_paths"][0],
        "crates/vida/tests/task_runtime_workflows.rs"
    );
}

#[test]
fn task_close_uses_latest_valid_persisted_zombie_d_matrix() {
    let fixture = PersistentRuntimeFixture::state_only("latest-valid-zombie-d-close");
    fixture.boot();

    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "latest-valid-zombie-d-epic",
            "Latest valid ZOMBIE-D epic",
            "--type",
            "epic",
            "--execution-mode",
            "container_only",
            "--json",
        ],
    );
    let older = zombie_d_matrix_note(serde_json::json!([{"id": "resolved-doubt"}]));
    let latest = zombie_d_matrix_note(serde_json::json!([]));
    let notes = format!("{older}\n\n{latest}");
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "latest-valid-zombie-d-task",
            "Latest valid ZOMBIE-D task",
            "--parent-id",
            "latest-valid-zombie-d-epic",
            "--labels",
            "zombie-d",
            "--owned-path",
            "crates/vida/src/zombie_d_gate.rs",
            "--notes",
            &notes,
            "--execution-mode",
            "sequential",
            "--json",
        ],
    );
    let close = run_json_success(
        &fixture,
        &[
            "task",
            "close",
            "latest-valid-zombie-d-task",
            "--reason",
            "latest valid ZOMBIE-D matrix supersedes resolved historical doubt",
            "--json",
        ],
    );
    assert_eq!(close["status"], "pass");
    let show = run_json_success(
        &fixture,
        &["task", "show", "latest-valid-zombie-d-task", "--json"],
    );
    assert_eq!(show["task"]["status"], "closed");
    assert!(show["task"]["notes"]
        .as_str()
        .is_some_and(|notes| notes.contains("resolved-doubt")
            && notes.matches("task_proof_evidence:").count() == 2));
}

#[test]
fn task_close_accepts_replay_persistence_consistency_matrix_and_taskflow_metadata() {
    let fixture = PersistentRuntimeFixture::state_only("zombie-d-rpc-matrix-close");
    fixture.boot();
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "zombie-d-rpc-epic",
            "ZOMBIE-D RPC matrix epic",
            "--type",
            "epic",
            "--execution-mode",
            "container_only",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "zombie-d-rpc-task",
            "Replay persistence cross-surface contract",
            "--parent-id",
            "zombie-d-rpc-epic",
            "--labels",
            "zombie-d,replay,persistence,cross-surface",
            "--owned-path",
            "crates/vida/src/zombie_d_gate.rs",
            "--acceptance-target",
            "Replay,Persistence,Cross-surface consistency",
            "--proof-target",
            "cargo test -p vida zombie_d -- --test-threads=1",
            "--notes",
            &zombie_d_rpc_matrix_note(),
            "--execution-mode",
            "sequential",
            "--json",
        ],
    );
    run_json_success(
        &fixture,
        &[
            "task",
            "proof",
            "attach-evidence",
            "zombie-d-rpc-task",
            "--proof-target",
            "cargo test -p vida zombie_d -- --test-threads=1",
            "--result",
            "pass",
            "--artifact-ref",
            "artifacts/zombie-d-rpc-focused-proof.txt",
            "--evidence",
            "R/P/C focused contract test and legacy migration proof passed",
            "--json",
        ],
    );
    let close = run_json_success(
        &fixture,
        &[
            "task",
            "close",
            "zombie-d-rpc-task",
            "--reason",
            "R P C matrix proof passed",
            "--json",
        ],
    );
    assert_eq!(close["status"], "pass");
    let show = run_json_success(&fixture, &["task", "show", "zombie-d-rpc-task", "--json"]);
    assert_eq!(show["task"]["status"], "closed");
    assert_eq!(
        show["task"]["planner_metadata"]["acceptance_targets"],
        serde_json::json!(["Replay", "Persistence", "Cross-surface consistency"])
    );
    assert_eq!(
        show["task"]["planner_metadata"]["proof_targets"][0],
        "cargo test -p vida zombie_d -- --test-threads=1"
    );
}


#[test]
fn task_proof_projection_cli_matrix_is_deterministic_for_duplicate_stale_and_latest_evidence() {
    let fixture = PersistentRuntimeFixture::state_only("proof-projection-close-gate-matrix");
    fixture.boot();
    run_json_success(
        &fixture,
        &[
            "task",
            "create",
            "proof-projection-epic",
            "Proof projection epic",
            "--type",
            "epic",
            "--execution-mode",
            "container_only",
            "--json",
        ],
    );


    let cargo_target = "cargo test -p vida proof_projection";
    let diagnostics_target = "vida diagnostics post-commit --json";
    let notes = format!(
        "task_proof_evidence:\n  proof_target: {cargo_target}\n  result: pass\n  evidence: older pass\n\n         task_proof_evidence:\n  proof_target: {cargo_target}\n  result: fail\n  evidence: newer fail\n\n         task_proof_evidence:\n  proof_target: {diagnostics_target}\n  result: pass\n  evidence: diagnostics pass\n\n{}",
        zombie_d_matrix_note(serde_json::json!([]))
    );
    let import_path = fixture.state_dir().join("proof-projection-matrix.jsonl");
    let import_record = serde_json::json!({
        "id": "proof-projection-task",
        "display_id": null,
        "title": "Proof projection task",
        "description": "",
        "status": "in_progress",
        "priority": 2,
        "issue_type": "task",
        "created_at": "2026-07-28T00:00:00Z",
        "created_by": "test",
        "updated_at": "2026-07-28T00:00:00Z",
        "closed_at": null,
        "close_reason": null,
        "source_repo": ".",
        "compaction_level": 0,
        "original_size": 0,
        "notes": notes,
        "labels": ["zombie-d"],
        "planner_metadata": {
            "owned_paths": ["crates/vida/src/zombie_d_gate.rs"],
            "proof_targets": [
                cargo_target,
                cargo_target,
                "vida diagnostics --json",
                diagnostics_target,
                "zombie_d_matrix"
            ]
        },
        "dependencies": [{
            "issue_id": "proof-projection-task",
            "depends_on_id": "proof-projection-epic",
            "edge_type": "parent-child",
            "created_at": "1",
            "created_by": "test",
            "metadata": "{}",
            "thread_id": ""
        }]
    });
    std::fs::write(
        &import_path,
        format!(
            "{}\n",
            serde_json::to_string(&import_record).expect("serialize proof projection import")
        ),
    )
    .expect("write proof projection import");
    run_json_success(
        &fixture,
        &[
            "task",
            "import-jsonl",
            import_path.to_str().expect("proof projection import path should be utf8"),
            "--json",
        ],
    );

    let status = run_json_success(
        &fixture,
        &["task", "proof", "status", "proof-projection-task", "--json"],
    );
    assert_eq!(status["configured_proof_target_count"], 3);
    assert_eq!(status["stored_proof_target_count"], 4);
    assert_eq!(status["satisfied_count"], 2);
    assert_eq!(status["missing_count"], 1);
    assert_eq!(status["missing_targets"], serde_json::json!([cargo_target]));
    assert_eq!(
        status["duplicate_proof_targets"],
        serde_json::json!([diagnostics_target])
    );
    assert_eq!(
        status["stale_proof_targets"],
        serde_json::json!(["vida diagnostics --json"])
    );
    assert_eq!(status["proof_targets"].as_array().map(Vec::len), Some(3));

    let blocked_close = run_json(
        &fixture,
        &[
            "task",
            "close",
            "proof-projection-task",
            "--reason",
            "proof projection matrix is incomplete",
            "--json",
        ],
    );
    assert_eq!(blocked_close["status"], "blocked");
    assert!(blocked_close["blocker_codes"]
        .as_array()
        .is_some_and(|codes| codes.iter().any(|code| code == "missing_structured_proof_evidence")));

    run_json_success(
        &fixture,
        &[
            "task",
            "proof",
            "attach-evidence",
            "proof-projection-task",
            "--proof-target",
            cargo_target,
            "--result",
            "pass",
            "--artifact-ref",
            "artifacts/proof-projection.txt",
            "--evidence",
            "latest cargo proof passed",
            "--json",
        ],
    );
    let passed_close = run_json_success(
        &fixture,
        &[
            "task",
            "close",
            "proof-projection-task",
            "--reason",
            "proof projection matrix passed",
            "--json",
        ],
    );
    assert_eq!(passed_close["status"], "pass");
}

#[test]
fn team_flow_transition_zombie_d_public_matrix() {
    let fixture = PersistentRuntimeFixture::state_only("team-flow-transition-zombie-d");
    fixture.boot();

    let (routing, routing_success) =
        fixture.json_allow_failure(&["taskflow", "validate-routing", "--json"]);
    assert!(routing_success || routing["status"] == "blocked");
    assert!(matches!(
        routing["status"].as_str(),
        Some("pass") | Some("blocked")
    ));
    assert_zombie_d_operator_shape(&routing, "vida taskflow validate-routing");
    if routing["status"] == "pass" {
        assert!(routing["route_count"].as_u64().unwrap_or_default() > 0);
    } else {
        assert!(routing["blocker_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()));
    }

    let (route, route_success) = fixture.json_allow_failure(&[
        "taskflow",
        "route",
        "explain",
        "--dispatch-target",
        "analyst",
        "--json",
    ]);
    assert!(route_success || route["status"] == "blocked");
    if route["status"] == "pass" {
        assert_eq!(route["route"]["dispatch_target"], "analyst");
        assert!(route["route"]["route_present"].as_bool().unwrap_or(false));
    } else {
        assert!(route["blocker_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()));
    }

    let (route_by_role, route_by_role_success) = fixture.json_allow_failure(&[
        "taskflow",
        "route",
        "explain",
        "--runtime-role",
        "business_analyst",
        "--json",
    ]);
    assert_eq!(route["status"], route_by_role["status"]);
    assert_eq!(route_success, route_by_role_success);
    assert_eq!(route["blocker_codes"], route_by_role["blocker_codes"]);
    assert_eq!(route["next_actions"], route_by_role["next_actions"]);
    assert_eq!(route["artifact_refs"], route_by_role["artifact_refs"]);
    if route["status"] == "pass" {
        assert_eq!(
            route["route"]["dispatch_target"],
            route_by_role["route"]["dispatch_target"]
        );
        assert_eq!(
            route["route"]["allowed_next_node"],
            route_by_role["route"]["allowed_next_node"]
        );
    }
    assert_zombie_d_operator_shape(&route, "vida taskflow route explain");
    assert_zombie_d_operator_shape(&route_by_role, "vida taskflow route explain");

    let compact = fixture.capture(&[
        "taskflow",
        "route",
        "explain",
        "--dispatch-target",
        "analyst",
    ]);
    assert!(!compact.stdout.is_empty());
    let compact_stdout = String::from_utf8_lossy(&compact.stdout);
    assert!(!compact_stdout.contains("--json"));
    assert!(compact_stdout.contains("route") || compact_stdout.contains("blocked"));

    let help = fixture.capture(&["taskflow", "route", "explain", "--help"]);
    let help_text = format!(
        "{}{}",
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr)
    );
    assert!(help_text.contains("--json"));
    assert!(help_text.contains("--dispatch-target"));
    assert!(help_text.contains("--runtime-role"));

    let run_id = "team-flow-transition-zombie-d-runtime";
    fixture.create_run_graph_backing_task(run_id);
    let seed = run_json_success(
        &fixture,
        &[
            "taskflow",
            "run-graph",
            "seed",
            run_id,
            "continue development",
            "--json",
        ],
    );
    assert_eq!(seed["payload"]["status"]["run_id"], run_id);
    let advance = run_json_success(
        &fixture,
        &["taskflow", "run-graph", "advance", run_id, "--json"],
    );
    let next_node = advance["payload"]["status"]["next_node"]
        .as_str()
        .expect("advance should expose canonical next_node");
    assert!(!next_node.is_empty());
    assert!(!next_node.contains('-'));

    let persisted = run_json_success(
        &fixture,
        &["taskflow", "run-graph", "status", run_id, "--json"],
    );
    assert_zombie_d_operator_shape(&persisted, "vida taskflow run-graph status");
    assert_eq!(persisted["run_graph_status"]["run_id"], run_id);
    assert_eq!(persisted["run_graph_status"]["next_node"], next_node);

    let bridge_fixture =
        PersistentRuntimeFixture::state_only("team-flow-transition-zombie-d-host-bridge");
    bridge_fixture.boot();
    let bridge_state_dir = bridge_fixture.state_dir_string();
    let bridge_run_id = "team-flow-transition-zombie-d-host-bridge-run";
    bridge_fixture.create_run_graph_backing_task(bridge_run_id);
    let dispatch_init = run_json_success(
        &bridge_fixture,
        &[
            "taskflow",
            "run-graph",
            "dispatch-init",
            bridge_run_id,
            "--json",
        ],
    );
    let dispatch_packet_path = dispatch_init["dispatch_packet_path"]
        .as_str()
        .expect("host bridge parity dispatch should expose packet path");
    let (agent_init, agent_init_success) = bridge_fixture.json_allow_failure(&[
        "agent-init",
        "--dispatch-packet",
        dispatch_packet_path,
        "--execute-dispatch",
        "--json",
    ]);
    assert!(
        agent_init_success || agent_init["status"] == "blocked",
        "host bridge dispatch must return a canonical verdict: {agent_init}"
    );
    let request_path = agent_init["host_tool_bridge_request"]["request_path"]
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            agent_init["artifact_refs"]["host_bridge_request_path"]
                .as_str()
                .map(str::to_string)
        })
        .or_else(|| {
            std::fs::read_dir(
                std::path::Path::new(&bridge_state_dir).join("host-tool-bridge/requests"),
            )
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .map(|path| path.display().to_string())
        })
        .expect("host bridge parity must materialize a request path");
    let (direct_bridge, direct_bridge_success) = bridge_fixture.json_allow_failure(&[
        "agent",
        "host-bridge",
        "--request",
        &request_path,
        "--state-dir",
        &bridge_state_dir,
        "--json",
    ]);
    assert!(
        direct_bridge_success || direct_bridge["status"] == "blocked",
        "direct host bridge must fail closed with a canonical verdict: {direct_bridge}"
    );
    assert_zombie_d_host_bridge_shape(&direct_bridge);
    let (bridge_persisted, bridge_persisted_success) = bridge_fixture.json_allow_failure(&[
        "taskflow",
        "run-graph",
        "status",
        bridge_run_id,
        "--json",
    ]);
    assert!(
        bridge_persisted_success || bridge_persisted["status"] == "blocked",
        "persisted bridge status must be canonical: {bridge_persisted}"
    );
    assert_zombie_d_operator_shape(&bridge_persisted, "vida taskflow run-graph status");
    assert_eq!(
        direct_bridge["artifact_refs"]["request_path"],
        bridge_persisted["artifact_refs"]["host_bridge_request_path"],
        "direct host bridge and persisted run graph must identify the same request artifact"
    );

    let missing_route = run_json(
        &fixture,
        &[
            "taskflow",
            "route",
            "explain",
            "--dispatch-target",
            "designer",
            "--json",
        ],
    );
    assert_eq!(missing_route["status"], "blocked");
    assert!(missing_route["blocker_codes"]
        .as_array()
        .is_some_and(|codes| codes.iter().any(|code| code == "route_missing")));
    assert_zombie_d_operator_shape(&missing_route, "vida taskflow route explain");
}
