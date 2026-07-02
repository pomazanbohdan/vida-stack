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
