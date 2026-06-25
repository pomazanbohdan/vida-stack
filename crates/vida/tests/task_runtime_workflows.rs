use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use vida_test_support as support;

fn vida() -> Command {
    support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_state_dir(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}/vida-{label}-{}-{nanos}-{counter}",
        std::env::temp_dir().display(),
        std::process::id()
    )
}

fn run_json(state_dir: &str, args: &[&str]) -> Value {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .unwrap_or_else(|error| panic!("{} should run: {error}", args.join(" ")));
    parse_json(args, &output)
}

fn run_json_success(state_dir: &str, args: &[&str]) -> Value {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .unwrap_or_else(|error| panic!("{} should run: {error}", args.join(" ")));
    assert!(
        output.status.success(),
        "{} should succeed: stdout={}; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_json(args, &output)
}

fn run_failure(state_dir: &str, args: &[&str]) -> Output {
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .unwrap_or_else(|error| panic!("{} should run: {error}", args.join(" ")));
    assert!(
        !output.status.success(),
        "{} should fail: stdout={}; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn parse_json(args: &[&str], output: &Output) -> Value {
    assert!(
        !output.stdout.is_empty(),
        "{} should emit JSON on stdout; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{} stdout should parse as JSON: {error}\nstdout={}\nstderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn boot_state(state_dir: &str) {
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "boot should succeed: {}",
        String::from_utf8_lossy(&boot.stderr)
    );
}

#[test]
fn task_runtime_workflows_treat_step_as_execution_only_and_subtask_as_work_item() {
    let state_dir = unique_state_dir("step-subtask-workflow");
    boot_state(&state_dir);

    run_json_success(
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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
    assert_eq!(todo_alias["task"]["issue_type"], "todo");
    assert_eq!(
        todo_alias["task"]["work_item_kind"]["canonical_issue_type"],
        "step"
    );

    let invalid_subtask = run_failure(
        &state_dir,
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

    let ready = run_json_success(&state_dir, &["task", "ready", "--json"]);
    let ready_ids = ready["tasks"]
        .as_array()
        .expect("ready tasks should be array")
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect::<Vec<_>>();
    assert!(ready_ids.contains(&"workflow-subtask"));
    assert!(!ready_ids.contains(&"workflow-step"));
    assert!(!ready_ids.contains(&"workflow-todo-alias"));

    let progress = run_json_success(&state_dir, &["task", "progress", "workflow-task", "--json"]);
    assert_eq!(progress["progress"]["descendant_count"], 1);
    assert_eq!(progress["progress"]["open_count"], 1);
    assert_eq!(progress["progress"]["closed_count"], 0);
    assert_eq!(
        progress["progress"]["status_counts"]["open"],
        serde_json::json!(1)
    );

    let status = run_json_success(&state_dir, &["status", "--json"]);
    assert_eq!(status["taskflow_counts"]["total_count"], 3);
    assert_eq!(status["taskflow_counts"]["open_count"], 3);
    assert_eq!(status["taskflow_counts"]["ready_count"], 2);

    let graph = run_json_success(&state_dir, &["task", "validate-graph", "--json"]);
    assert_eq!(graph["status"], "pass");
}

#[test]
fn task_runtime_workflows_cover_isolated_task_lifecycle_and_reimport() {
    let state_dir = unique_state_dir("runtime-workflow");
    boot_state(&state_dir);

    run_json_success(
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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

    let ready = run_json_success(&state_dir, &["task", "ready", "--json"]);
    let ready_ids = ready["tasks"]
        .as_array()
        .expect("ready tasks should be array")
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect::<Vec<_>>();
    assert!(ready_ids.contains(&"workflow-task"));

    let scheduler = run_json(&state_dir, &["taskflow", "scheduler", "dispatch", "--json"]);
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
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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
        &state_dir,
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
        &state_dir,
        &["taskflow", "run-graph", "status", "workflow-task", "--json"],
    );
    assert_eq!(run_graph["run_id"], "workflow-task");

    let recovery = run_json(
        &state_dir,
        &["taskflow", "recovery", "status", "workflow-task", "--json"],
    );
    assert_eq!(recovery["run_id"], "workflow-task");
    assert!(recovery["status"].is_string());

    run_json_success(
        &state_dir,
        &[
            "task",
            "close",
            "workflow-task",
            "--reason",
            "workflow proof completed: create, update, ready, scheduler, attempt, verify, run-graph, recovery, close, export, and import paths are covered.",
            "--json",
        ],
    );

    let export_path = std::path::Path::new(&state_dir).join("workflow-export.jsonl");
    run_json_success(
        &state_dir,
        &[
            "task",
            "export-jsonl",
            export_path.to_str().expect("export path should be utf8"),
            "--json",
        ],
    );
    assert!(export_path.exists());

    let import_state_dir = unique_state_dir("runtime-workflow-import");
    boot_state(&import_state_dir);
    run_json_success(
        &import_state_dir,
        &[
            "task",
            "import-jsonl",
            export_path.to_str().expect("export path should be utf8"),
            "--json",
        ],
    );
    let imported = run_json_success(
        &import_state_dir,
        &["task", "show", "workflow-task", "--json"],
    );
    assert_eq!(imported["task"]["status"], "closed");
    assert_eq!(
        imported["task"]["planner_metadata"]["owned_paths"][0],
        "crates/vida/tests/task_runtime_workflows.rs"
    );
}
