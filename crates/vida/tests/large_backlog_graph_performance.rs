use serde_json::Value;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const BACKLOG_DIRECT_CHILDREN: usize = 120;
const GRAPH_READ_BUDGET: Duration = Duration::from_secs(20);
const STATE_LOCK_RETRY_LIMIT: usize = 5;

struct TimedJson {
    value: Value,
    duration: Duration,
}

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn is_state_lock_error(output: &Output) -> bool {
    String::from_utf8_lossy(&output.stderr).contains(vida_test_support::STATE_LOCK_ERROR_MESSAGE)
}

fn run_output(args: &[&str], state_dir: &str) -> Output {
    vida_test_support::command_output_with_retry(
        || {
            let mut command = vida();
            command.args(args).env("VIDA_STATE_DIR", state_dir);
            command
        },
        STATE_LOCK_RETRY_LIMIT,
        is_state_lock_error,
    )
}

fn run_json(args: &[&str], state_dir: &str) -> TimedJson {
    let started_at = Instant::now();
    let output = run_output(args, state_dir);
    let duration = started_at.elapsed();
    assert!(
        output.status.success(),
        "args={args:?}\nduration={duration:?}\nstatus={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "args={args:?}\nduration={duration:?}\njson parse error={error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    TimedJson { value, duration }
}

fn assert_graph_read_budget(surface: &str, duration: Duration) {
    assert!(
        duration <= GRAPH_READ_BUDGET,
        "{surface} exceeded large-backlog graph read budget: {duration:?}"
    );
}

fn ids_from_array(value: &Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{label} should be an array: {value}"))
        .iter()
        .filter_map(|item| item["id"].as_str().map(str::to_string))
        .collect()
}

fn blocked_task_ids(value: &Value) -> Vec<String> {
    value["tasks"]
        .as_array()
        .unwrap_or_else(|| panic!("blocked tasks should be an array: {value}"))
        .iter()
        .filter_map(|item| item["task"]["id"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn large_backlog_graph_progress_and_scheduler_surfaces_stay_deterministic() {
    let state_dir = vida_test_support::temp_dir("vida-large-backlog-state");
    let state_dir_string = state_dir.display().to_string();
    let jsonl_path = state_dir.join("large-backlog.jsonl");
    let fixture =
        vida_test_support::write_large_backlog_jsonl(&jsonl_path, BACKLOG_DIRECT_CHILDREN)
            .expect("large backlog jsonl fixture should write");
    let jsonl_path_string = jsonl_path.display().to_string();

    let import = run_json(
        &["task", "import-jsonl", &jsonl_path_string, "--json"],
        &state_dir_string,
    );
    assert_eq!(import.value["status"], "pass");

    let progress = run_json(
        &[
            "task",
            "progress",
            &fixture.root_id,
            "--basis",
            "direct-children",
            "--json",
        ],
        &state_dir_string,
    );
    assert_graph_read_budget("vida task progress", progress.duration);
    assert_eq!(progress.value["status"], "pass");
    assert_eq!(
        progress.value["progress"]["direct_child_count"],
        fixture.direct_child_count
    );
    assert_eq!(
        progress.value["progress"]["descendant_count"],
        fixture.direct_child_count
    );
    assert_eq!(
        progress.value["progress"]["open_count"],
        fixture.open_child_count
    );
    assert_eq!(
        progress.value["progress"]["closed_count"],
        fixture.closed_child_count
    );
    assert_eq!(
        progress.value["progress"]["in_progress_count"],
        fixture.in_progress_child_count
    );

    let tree = run_json(
        &["task", "tree", &fixture.root_id, "--json"],
        &state_dir_string,
    );
    assert_graph_read_budget("vida task tree", tree.duration);
    assert_eq!(tree.value["status"], "pass");
    assert_eq!(tree.value["root_task_id"], fixture.root_id);
    assert_eq!(tree.value["child_count"], fixture.direct_child_count);
    assert_eq!(
        tree.value["children"]
            .as_array()
            .expect("tree children should be an array")
            .len(),
        fixture.direct_child_count
    );

    let ready = run_json(
        &["task", "ready", "--scope", &fixture.root_id, "--json"],
        &state_dir_string,
    );
    assert_graph_read_budget("vida task ready", ready.duration);
    assert_eq!(ready.value["status"], "pass");
    assert_eq!(ready.value["ready_count"], fixture.open_child_count);
    let ready_ids = ids_from_array(&ready.value["tasks"], "ready tasks");
    assert!(ready_ids.contains(&fixture.primary_ready_id));

    let blocked = run_json(&["task", "blocked", "--json"], &state_dir_string);
    assert_graph_read_budget("vida task blocked", blocked.duration);
    assert_eq!(blocked.value["status"], "pass");
    assert_eq!(
        blocked.value["blocked_count"],
        fixture.blocked_open_child_count
    );
    assert!(blocked_task_ids(&blocked.value).contains(&fixture.blocked_task_id));

    let graph_summary = run_json(&["taskflow", "graph-summary", "--json"], &state_dir_string);
    assert_graph_read_budget("vida taskflow graph-summary", graph_summary.duration);
    assert_eq!(
        graph_summary.value["surface"],
        "vida taskflow graph-summary"
    );
    assert_eq!(graph_summary.value["ready_count"], fixture.open_child_count);
    assert_eq!(
        graph_summary.value["blocked_count"],
        fixture.blocked_open_child_count
    );

    let scheduler = run_json(
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--current-task-id",
            &fixture.primary_ready_id,
            "--limit",
            "3",
            "--state-dir",
            &state_dir_string,
            "--json",
        ],
        &state_dir_string,
    );
    assert_graph_read_budget("vida taskflow scheduler dispatch", scheduler.duration);
    assert_eq!(
        scheduler.value["surface"],
        "vida taskflow scheduler dispatch"
    );
    assert_eq!(
        scheduler.value["ready_count"],
        graph_summary.value["ready_count"]
    );
    assert_eq!(
        scheduler.value["blocked_count"],
        graph_summary.value["blocked_count"]
    );
    assert_eq!(
        scheduler.value["selected_primary_task"]["id"],
        fixture.primary_ready_id
    );
    assert_eq!(
        scheduler.value["scheduling"]["current_task_id"],
        fixture.primary_ready_id
    );

    let status = run_json(
        &["status", "--fields", "taskflow_counts", "--json"],
        &state_dir_string,
    );
    assert_graph_read_budget("vida status --fields taskflow_counts", status.duration);
    assert_eq!(
        status.value["taskflow_counts"]["total_count"],
        fixture.total_task_count
    );
    assert_eq!(
        status.value["taskflow_counts"]["open_count"],
        fixture.open_child_count + 1
    );
    assert_eq!(
        status.value["taskflow_counts"]["closed_count"],
        fixture.closed_child_count
    );
    assert_eq!(
        status.value["taskflow_counts"]["in_progress_count"],
        fixture.in_progress_child_count
    );

    eprintln!(
        "large backlog diagnostics: import={:?} progress={:?} tree={:?} ready={:?} blocked={:?} graph_summary={:?} scheduler={:?} status={:?}",
        import.duration,
        progress.duration,
        tree.duration,
        ready.duration,
        blocked.duration,
        graph_summary.duration,
        scheduler.duration,
        status.duration
    );

    let _ = std::fs::remove_dir_all(state_dir);
}

#[test]
fn simulated_lock_pressure_retry_is_deterministic() {
    let mut attempts = 0usize;
    let output = vida_test_support::retry_with_backoff(
        || {
            attempts += 1;
            if attempts < 4 {
                vida_test_support::simulated_state_lock_output()
            } else {
                vida_test_support::simulated_success_output("ready\n")
            }
        },
        STATE_LOCK_RETRY_LIMIT,
        is_state_lock_error,
    );

    assert!(output.status.success());
    assert_eq!(attempts, 4);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ready\n");
}
