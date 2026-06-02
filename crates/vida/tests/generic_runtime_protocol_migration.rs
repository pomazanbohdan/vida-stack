use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn run_json(project_root: &Path, state_dir: &Path, args: &[&str]) -> Value {
    let output = vida()
        .args(args)
        .current_dir(project_root)
        .env("VIDA_STATE_DIR", state_dir)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_CONFIG")
        .output()
        .expect("vida command should run");
    assert!(
        output.status.success(),
        "command {args:?} should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("vida json should parse")
}

fn task_row_by_id<'a>(payload: &'a Value, task_id: &str) -> &'a Value {
    payload["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == task_id)
        .unwrap_or_else(|| panic!("task `{task_id}` should be present in {payload:#}"))
}

fn jsonl_row_by_id<'a>(rows: &'a [Value], task_id: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["id"] == task_id)
        .unwrap_or_else(|| panic!("jsonl row `{task_id}` should be present in {rows:#?}"))
}

fn has_parent_dependency(task: &Value, parent_id: &str) -> bool {
    task["dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|dependency| {
            dependency["edge_type"] == "parent-child" && dependency["depends_on_id"] == parent_id
        })
}

#[test]
fn generic_runtime_protocol_migration_fixture_project_without_vida_stack_docs_can_schedule() {
    let project_root = vida_test_support::temp_dir("vida-generic-runtime-fixture-project");
    let state_dir = project_root.join(".vida").join("data").join("state");
    fs::create_dir_all(&state_dir).expect("state dir should create");
    assert!(
        !project_root.join("AGENTS.sidecar.md").exists(),
        "fixture project must not rely on vida-stack docs"
    );

    let root = run_json(
        &project_root,
        &state_dir,
        &[
            "task",
            "create",
            "fixture-root",
            "Fixture root",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "1",
            "--json",
        ],
    );
    assert_eq!(root["status"], "pass");
    assert_eq!(
        root["task"]["work_item_kind"]["canonical_issue_type"],
        "epic"
    );

    let task = run_json(
        &project_root,
        &state_dir,
        &[
            "task",
            "create",
            "fixture-task",
            "Fixture task",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "1",
            "--parent-id",
            "fixture-root",
            "--description",
            "Generic runtime fixture without project-local docs",
            "--json",
        ],
    );
    assert_eq!(task["status"], "pass");
    assert!(has_parent_dependency(&task["task"], "fixture-root"));
    assert_eq!(
        task["task"]["work_item_kind"]["default_flow_binding"],
        "default_delivery"
    );
    assert!(
        task["task"]["source_repo"]
            .as_str()
            .expect("source_repo should render")
            .contains("vida-generic-runtime-fixture-project"),
        "source repo should be the fixture project, not the vida-stack checkout"
    );

    let ready = run_json(&project_root, &state_dir, &["task", "ready", "--json"]);
    let ready_task = task_row_by_id(&ready, "fixture-task");
    assert_eq!(
        ready_task["work_item_kind"]["default_flow_binding"],
        "default_delivery"
    );
    let list = run_json(
        &project_root,
        &state_dir,
        &["task", "list", "--all", "--json"],
    );
    let listed_task = task_row_by_id(&list, "fixture-task");
    assert_eq!(listed_task["parent_id"], "fixture-root");

    fs::remove_dir_all(project_root).expect("fixture project should clean up");
}

#[test]
fn work_item_taxonomy_existing_records_and_provider_mapping_round_trip_cross_project() {
    let project_root = vida_test_support::temp_dir("vida-generic-runtime-provider-fixture");
    let state_dir = project_root.join(".vida").join("data").join("state");
    fs::create_dir_all(&state_dir).expect("state dir should create");
    let import_path = project_root.join("provider-tasks.jsonl");
    let child = serde_json::json!({
        "id": "provider-child",
        "title": "Provider child",
        "description": "Cross-project provider story",
        "status": "open",
        "priority": 4,
        "issue_type": "",
        "created_at": "2026-06-02T00:00:00Z",
        "created_by": "migration-fixture",
        "updated_at": "2026-06-02T00:00:00Z",
        "source_repo": project_root.to_string_lossy(),
        "compaction_level": 0,
        "original_size": 0,
        "labels": [],
        "provider_mapping": {
            "provider": "jira",
            "external_id": "GEN-12",
            "external_parent_id": "GEN-1",
            "provider_issue_type": "story",
            "provider_status": "todo",
            "provider_priority": "p2"
        },
        "dependencies": []
    });
    let parent = serde_json::json!({
        "id": "provider-parent",
        "title": "Provider parent",
        "description": "Cross-project provider epic",
        "status": "open",
        "priority": 1,
        "issue_type": "",
        "created_at": "2026-06-02T00:00:00Z",
        "created_by": "migration-fixture",
        "updated_at": "2026-06-02T00:00:00Z",
        "source_repo": project_root.to_string_lossy(),
        "compaction_level": 0,
        "original_size": 0,
        "labels": [],
        "provider_mapping": {
            "provider": "jira",
            "external_id": "GEN-1",
            "provider_issue_type": "epic",
            "provider_status": "open"
        },
        "dependencies": []
    });
    fs::write(
        &import_path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&child).expect("child should serialize"),
            serde_json::to_string(&parent).expect("parent should serialize")
        ),
    )
    .expect("provider jsonl should write");

    let import = run_json(
        &project_root,
        &state_dir,
        &[
            "taskflow",
            "task",
            "import-jsonl",
            import_path.to_str().expect("import path should be utf8"),
            "--json",
        ],
    );
    assert_eq!(import["status"], "pass");

    let shown = run_json(
        &project_root,
        &state_dir,
        &["task", "show", "provider-child", "--json"],
    );
    assert_eq!(shown["task"]["issue_type"], "task");
    assert_eq!(
        shown["task"]["work_item_kind"]["canonical_issue_type"],
        "task"
    );
    assert_eq!(shown["task"]["status"], "open");
    assert_eq!(shown["task"]["priority"], 2);
    assert!(has_parent_dependency(&shown["task"], "provider-parent"));

    let ready = run_json(&project_root, &state_dir, &["task", "ready", "--json"]);
    let ready_child = task_row_by_id(&ready, "provider-child");
    assert_eq!(
        ready_child["work_item_kind"]["source_tiers"][0],
        "operator_request"
    );

    let export_path = project_root.join("exported-provider-tasks.jsonl");
    let export = run_json(
        &project_root,
        &state_dir,
        &[
            "taskflow",
            "task",
            "export-jsonl",
            export_path.to_str().expect("export path should be utf8"),
            "--json",
        ],
    );
    assert_eq!(export["status"], "pass");
    let exported = fs::read_to_string(&export_path).expect("exported jsonl should read");
    let rows = exported
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("exported row should parse"))
        .collect::<Vec<_>>();
    let exported_child = jsonl_row_by_id(&rows, "provider-child");
    assert_eq!(exported_child["provider_mapping"]["provider"], "jira");
    assert_eq!(exported_child["provider_mapping"]["external_id"], "GEN-12");
    assert_eq!(
        exported_child["provider_mapping"]["external_parent_id"],
        "GEN-1"
    );

    fs::remove_dir_all(project_root).expect("fixture project should clean up");
}
