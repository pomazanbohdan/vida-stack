use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "/tmp/vida-task-state-{}-{}-{}",
        std::process::id(),
        nanos,
        counter
    )
}

fn project_bound_state_dir() -> (String, String) {
    let project_root = unique_state_dir();
    let state_dir = format!("{project_root}/.vida/data/state");
    fs::create_dir_all(&state_dir).expect("create project-bound state dir");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("write AGENTS.md");
    fs::write(
        format!("{project_root}/vida.config.yaml"),
        "project:\n  id: test\n",
    )
    .expect("write vida.config.yaml");
    for relative in [".vida/config", ".vida/db", ".vida/project"] {
        fs::create_dir_all(format!("{project_root}/{relative}"))
            .expect("runtime project marker dir should exist");
    }
    (project_root, state_dir)
}

static PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn sample_jsonl(path: &str) {
    fs::write(
        path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"third\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
        ),
    )
    .expect("write task jsonl");
}

fn run_and_assert_success(args: &[&str], state_dir: &str) -> String {
    let output = run_with_state_lock_retry(|| {
        let mut command = vida();
        command.args(args).env("VIDA_STATE_DIR", state_dir);
        command
    });
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn run_command_capture(args: &[&str], state_dir: &str) -> std::process::Output {
    run_with_state_lock_retry(|| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vida"));
        command.args(args).env("VIDA_STATE_DIR", state_dir);
        command
    })
}

fn run_command_json(args: &[&str], state_dir: &str) -> serde_json::Value {
    let output = run_with_state_lock_retry(|| {
        let mut command = vida();
        command.args(args).env("VIDA_STATE_DIR", state_dir);
        command
    });
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json output should parse")
}

fn extract_plain_surface_line(output: &str, label: &str) -> String {
    let prefix = format!("{label}: ");
    output
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_string))
        .unwrap_or_else(|| panic!("{label} line missing from plain output"))
}

fn require_json_string(value: &serde_json::Value, label: &str) -> String {
    value
        .as_str()
        .map(|text| text.to_string())
        .unwrap_or_else(|| panic!("{} missing or not a string", label))
}

fn normalize_json_fixture(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    serde_json::to_string_pretty(&parsed).expect("json output should pretty render")
}

const STATE_LOCK_RETRY_LIMIT: usize = 600;

fn retry_backoff_delay(attempt: usize) -> Duration {
    Duration::from_millis(match attempt {
        0..=4 => 10,
        5..=9 => 25,
        10..=19 => 50,
        _ => 100,
    })
}

fn is_state_lock_error(output: &std::process::Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains("LOCK is already locked")
        || stderr.contains("timed out while waiting for authoritative datastore lock")
}

fn run_with_state_lock_retry<F>(mut builder: F) -> std::process::Output
where
    F: FnMut() -> Command,
{
    let mut last = None;
    for attempt in 0..STATE_LOCK_RETRY_LIMIT {
        match builder().output() {
            Ok(output) if output.status.success() || !is_state_lock_error(&output) => {
                return output
            }
            Ok(output) => {
                last = Some(output);
            }
            Err(error) if error.raw_os_error() == Some(26) => {
                // transient busy signal, fall through to retry
            }
            Err(error) => panic!("command should run: {error}"),
        }
        thread::sleep(retry_backoff_delay(attempt));
    }
    last.expect("state lock retry should capture at least one output")
}

fn assert_json_status_pass(output: &str) {
    let parsed: serde_json::Value = serde_json::from_str(output).expect("json output should parse");
    assert_eq!(parsed["status"], "pass");
}

fn donor_ready_semantic(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let rows = parsed
        .get("tasks")
        .and_then(serde_json::Value::as_array)
        .or_else(|| parsed.as_array())
        .expect("ready output should expose task rows");
    let normalized = rows
        .iter()
        .map(|row| {
            let dependencies = row["dependencies"]
                .as_array()
                .expect("dependencies should be an array");
            serde_json::json!({
                "id": row["id"].as_str().expect("id").to_string(),
                "status": row["status"].as_str().expect("status").to_string(),
                "dependency_targets": dependencies.iter().map(|dep| dep["depends_on_id"].as_str().expect("depends_on_id")).collect::<Vec<_>>(),
                "dependency_edge_types": dependencies.iter().map(|dep| dep.get("edge_type").or_else(|| dep.get("type")).and_then(|value| value.as_str()).expect("edge type")).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&normalized).expect("semantic ready output should render")
}

fn donor_show_semantic(value: &str) -> String {
    let row: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let dependencies = row["dependencies"]
        .as_array()
        .expect("dependencies should be an array");
    let normalized = serde_json::json!({
        "id": row["id"].as_str().expect("id").to_string(),
        "title": row["title"].as_str().expect("title").to_string(),
        "status": row["status"].as_str().expect("status").to_string(),
        "priority": row["priority"].as_i64().expect("priority"),
        "issue_type": row["issue_type"].as_str().expect("issue_type").to_string(),
        "dependency_targets": dependencies.iter().map(|dep| dep["depends_on_id"].as_str().expect("depends_on_id")).collect::<Vec<_>>(),
        "dependency_edge_types": dependencies.iter().map(|dep| dep.get("edge_type").or_else(|| dep.get("type")).and_then(|value| value.as_str()).expect("edge type")).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&normalized).expect("semantic show output should render")
}

fn donor_list_semantic(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let rows = parsed.as_array().expect("list output should be an array");
    let normalized = rows
        .iter()
        .map(|row| {
            let dependencies = row["dependencies"]
                .as_array()
                .expect("dependencies should be an array");
            serde_json::json!({
                "id": row["id"].as_str().expect("id").to_string(),
                "status": row["status"].as_str().expect("status").to_string(),
                "priority": row["priority"].as_i64().expect("priority"),
                "issue_type": row["issue_type"].as_str().expect("issue_type").to_string(),
                "dependency_targets": dependencies.iter().map(|dep| dep["depends_on_id"].as_str().expect("depends_on_id")).collect::<Vec<_>>(),
                "dependency_edge_types": dependencies.iter().map(|dep| dep.get("edge_type").or_else(|| dep.get("type")).and_then(|value| value.as_str()).expect("edge type")).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&normalized).expect("semantic list output should render")
}

fn require_string_array(value: &serde_json::Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{label} should be an array"))
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .unwrap_or_else(|| panic!("{label} entries should be strings"))
                .to_string()
        })
        .collect()
}

fn assert_release1_contract_mirror(surface: &serde_json::Value, mirror_key: &str, label: &str) {
    assert_eq!(
        surface["status"], surface[mirror_key]["status"],
        "{} top-level status should mirror {}.status",
        label, mirror_key
    );
    assert_eq!(
        surface["blocker_codes"], surface[mirror_key]["blocker_codes"],
        "{} blocker_codes should mirror {}.blocker_codes",
        label, mirror_key
    );
    assert_eq!(
        surface["next_actions"], surface[mirror_key]["next_actions"],
        "{} next_actions should mirror {}.next_actions",
        label, mirror_key
    );
    assert_eq!(
        surface["artifact_refs"], surface[mirror_key]["artifact_refs"],
        "{} artifact_refs should mirror {}.artifact_refs",
        label, mirror_key
    );
}

fn assert_shared_fields_consistency(surface: &serde_json::Value, label: &str) {
    assert_release1_contract_mirror(surface, "shared_fields", label);
}

fn assert_operator_contracts_consistency(surface: &serde_json::Value, label: &str) {
    assert_release1_contract_mirror(surface, "operator_contracts", label);
}

#[test]
fn task_command_round_trip_succeeds_via_binary_surface() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let list_stdout = run_and_assert_success(&["task", "list", "--json"], &state_dir);
    assert!(
        list_stdout.contains("\"id\": \"vida-b\"") || list_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        list_stdout.contains("\"id\": \"vida-a\"") || list_stdout.contains("\"id\":\"vida-a\"")
    );

    let ready_stdout = run_and_assert_success(&["task", "ready", "--json"], &state_dir);
    assert!(
        ready_stdout.contains("\"id\": \"vida-a\"") || ready_stdout.contains("\"id\":\"vida-a\"")
    );
    assert!(
        !ready_stdout.contains("\"id\": \"vida-b\"") && !ready_stdout.contains("\"id\":\"vida-b\"")
    );

    let scoped_ready_stdout = run_and_assert_success(
        &["task", "ready", "--scope", "vida-root", "--json"],
        &state_dir,
    );
    assert!(
        scoped_ready_stdout.contains("\"id\": \"vida-a\"")
            || scoped_ready_stdout.contains("\"id\":\"vida-a\"")
    );
    assert!(
        !scoped_ready_stdout.contains("\"id\": \"vida-b\"")
            && !scoped_ready_stdout.contains("\"id\":\"vida-b\"")
    );

    let deps_stdout = run_and_assert_success(&["task", "deps", "vida-b", "--json"], &state_dir);
    assert!(
        deps_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || deps_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );
    assert!(
        deps_stdout.contains("\"dependency_status\": \"open\"")
            || deps_stdout.contains("\"dependency_status\":\"open\"")
    );

    let reverse_stdout =
        run_and_assert_success(&["task", "reverse-deps", "vida-a", "--json"], &state_dir);
    assert!(
        reverse_stdout.contains("\"issue_id\": \"vida-b\"")
            || reverse_stdout.contains("\"issue_id\":\"vida-b\"")
    );
    assert!(
        reverse_stdout.contains("\"edge_type\": \"blocks\"")
            || reverse_stdout.contains("\"edge_type\":\"blocks\"")
    );

    let blocked_stdout = run_and_assert_success(&["task", "blocked", "--json"], &state_dir);
    assert!(
        blocked_stdout.contains("\"surface\": \"vida task blocked\"")
            || blocked_stdout.contains("\"surface\":\"vida task blocked\"")
    );
    assert!(
        blocked_stdout.contains("\"blocked_count\": 1")
            || blocked_stdout.contains("\"blocked_count\":1")
    );
    assert!(
        blocked_stdout.contains("\"id\": \"vida-b\"")
            || blocked_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        blocked_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || blocked_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );

    let tree_stdout = run_and_assert_success(&["task", "tree", "vida-b", "--json"], &state_dir);
    assert!(
        tree_stdout.contains("\"surface\": \"vida task tree\"")
            || tree_stdout.contains("\"surface\":\"vida task tree\"")
    );
    assert!(
        tree_stdout.contains("\"root_task_id\": \"vida-b\"")
            || tree_stdout.contains("\"root_task_id\":\"vida-b\"")
    );
    assert!(
        tree_stdout.contains("\"id\": \"vida-b\"") || tree_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        tree_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || tree_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );
    assert!(
        tree_stdout.contains("\"edge_type\": \"blocks\"")
            || tree_stdout.contains("\"edge_type\":\"blocks\"")
    );

    let validate_stdout = run_and_assert_success(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(validate_stdout.trim(), "[]");

    let critical_path_stdout =
        run_and_assert_success(&["task", "critical-path", "--json"], &state_dir);
    let critical_path_expected =
        include_str!("../../../tests/golden/taskflow/critical_path.json").trim_end();
    assert_eq!(
        normalize_json_fixture(&critical_path_stdout),
        normalize_json_fixture(critical_path_expected)
    );

    let dep_add_stdout = run_and_assert_success(
        &[
            "task",
            "dep",
            "add",
            "vida-c",
            "vida-root",
            "parent-child",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        dep_add_stdout.contains("\"issue_id\": \"vida-c\"")
            || dep_add_stdout.contains("\"issue_id\":\"vida-c\"")
    );
    assert!(
        dep_add_stdout.contains("\"depends_on_id\": \"vida-root\"")
            || dep_add_stdout.contains("\"depends_on_id\":\"vida-root\"")
    );

    let deps_after_add_stdout =
        run_and_assert_success(&["task", "deps", "vida-c", "--json"], &state_dir);
    assert!(
        deps_after_add_stdout.contains("\"depends_on_id\": \"vida-root\"")
            || deps_after_add_stdout.contains("\"depends_on_id\":\"vida-root\"")
    );

    let dep_remove_stdout = run_and_assert_success(
        &[
            "task",
            "dep",
            "remove",
            "vida-c",
            "vida-root",
            "parent-child",
            "--json",
        ],
        &state_dir,
    );
    assert!(
        dep_remove_stdout.contains("\"issue_id\": \"vida-c\"")
            || dep_remove_stdout.contains("\"issue_id\":\"vida-c\"")
    );

    let deps_after_remove_stdout =
        run_and_assert_success(&["task", "deps", "vida-c", "--json"], &state_dir);
    assert!(
        deps_after_remove_stdout.contains("\"surface\": \"vida task deps\"")
            || deps_after_remove_stdout.contains("\"surface\":\"vida task deps\"")
    );
    assert!(
        deps_after_remove_stdout.contains("\"task_id\": \"vida-c\"")
            || deps_after_remove_stdout.contains("\"task_id\":\"vida-c\"")
    );
    assert!(
        deps_after_remove_stdout.contains("\"dependency_count\": 0")
            || deps_after_remove_stdout.contains("\"dependency_count\":0")
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_create_update_close_round_trip_supports_planning_graph_views() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let root = run_command_json(
        &[
            "task",
            "create",
            "vida-root",
            "Root epic",
            "--type",
            "epic",
            "--status",
            "open",
            "--priority",
            "1",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(root["status"], "open");
    assert_eq!(root["issue_type"], "epic");

    let task_a = run_command_json(
        &[
            "task",
            "create",
            "vida-a",
            "Task A",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "2",
            "--parent-id",
            "vida-root",
            "--description",
            "first",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_a["status"], "open");
    assert_eq!(task_a["title"], "Task A");

    let task_b = run_command_json(
        &[
            "task",
            "create",
            "vida-b",
            "Task B",
            "--type",
            "task",
            "--status",
            "open",
            "--priority",
            "1",
            "--parent-id",
            "vida-root",
            "--description",
            "second",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(task_b["status"], "open");
    assert_eq!(task_b["title"], "Task B");

    let dep = run_command_json(
        &["task", "dep", "add", "vida-b", "vida-a", "blocks", "--json"],
        &state_dir,
    );
    assert_eq!(dep["issue_id"], "vida-b");
    assert_eq!(dep["depends_on_id"], "vida-a");
    assert_eq!(dep["edge_type"], "blocks");

    let updated = run_command_json(
        &[
            "task",
            "update",
            "vida-b",
            "--status",
            "in_progress",
            "--notes",
            "planning round trip proof",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(updated["status"], "in_progress");
    assert_eq!(updated["notes"], "planning round trip proof");

    let deps = run_command_json(&["task", "deps", "vida-b", "--json"], &state_dir);
    assert_eq!(deps["task_id"], "vida-b");
    assert_eq!(deps["dependency_count"], 2);
    let dependency_targets = deps["dependencies"]
        .as_array()
        .expect("dependencies should be an array")
        .iter()
        .map(|dependency| {
            dependency["depends_on_id"]
                .as_str()
                .expect("depends_on_id")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert!(dependency_targets.contains(&"vida-root".to_string()));
    assert!(dependency_targets.contains(&"vida-a".to_string()));

    let reverse = run_and_assert_success(&["task", "reverse-deps", "vida-a", "--json"], &state_dir);
    assert!(
        reverse.contains("\"issue_id\": \"vida-b\"") || reverse.contains("\"issue_id\":\"vida-b\"")
    );

    let blocked = run_and_assert_success(&["task", "blocked", "--json"], &state_dir);
    assert!(blocked.contains("\"blocked_count\": 1") || blocked.contains("\"blocked_count\":1"));
    assert!(blocked.contains("\"id\": \"vida-b\"") || blocked.contains("\"id\":\"vida-b\""));

    let critical_path = run_command_json(&["task", "critical-path", "--json"], &state_dir);
    assert_eq!(critical_path["length"], 2);
    assert_eq!(critical_path["root_task_id"], "vida-a");
    assert_eq!(critical_path["terminal_task_id"], "vida-b");

    let validate = run_and_assert_success(&["task", "validate-graph", "--json"], &state_dir);
    assert_eq!(validate.trim(), "[]");

    let closed = run_command_json(
        &[
            "task",
            "close",
            "vida-b",
            "--reason",
            "planning proof complete",
            "--json",
        ],
        &state_dir,
    );
    assert_eq!(closed["status"], "pass");
    assert_eq!(closed["task"]["status"], "closed");
    assert_eq!(closed["task"]["close_reason"], "planning proof complete");

    let shown = run_command_json(&["task", "show", "vida-b", "--json"], &state_dir);
    assert_eq!(shown["status"], "closed");
    assert_eq!(shown["close_reason"], "planning proof complete");
    assert_eq!(shown["notes"], "planning round trip proof");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn validate_graph_broken_edge_matches_golden_fixture() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        &jsonl_path,
        "{\"id\":\"vida-broken\",\"title\":\"Broken task\",\"description\":\"broken\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-broken\",\"depends_on_id\":\"vida-missing\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
    )
    .expect("write broken task jsonl");

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args(["task", "validate-graph", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("validate-graph should run");
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected =
        include_str!("../../../tests/golden/taskflow/validate_graph_missing_dependency.json")
            .trim_end();
    assert_eq!(
        normalize_json_fixture(&stdout),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn dep_add_fails_closed_when_second_parent_child_edge_is_added() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        &jsonl_path,
        concat!(
            "{\"id\":\"vida-root-a\",\"title\":\"Root A\",\"description\":\"a\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-root-b\",\"title\":\"Root B\",\"description\":\"b\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-child\",\"title\":\"Child\",\"description\":\"child\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-child\",\"depends_on_id\":\"vida-root-a\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("write parent-child jsonl");

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args([
            "task",
            "dep",
            "add",
            "vida-child",
            "vida-root-b",
            "parent-child",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("dep add should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("dependency mutation would create invalid graph"));
    assert!(stderr.contains("multiple_parent_edges"));

    let deps_stdout = run_and_assert_success(&["task", "deps", "vida-child", "--json"], &state_dir);
    assert!(
        deps_stdout.contains("\"depends_on_id\": \"vida-root-a\"")
            || deps_stdout.contains("\"depends_on_id\":\"vida-root-a\"")
    );
    assert!(
        !deps_stdout.contains("\"depends_on_id\": \"vida-root-b\"")
            && !deps_stdout.contains("\"depends_on_id\":\"vida-root-b\"")
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn run_graph_update_fails_closed_when_memory_correction_lacks_sealed_context() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let init = vida()
        .args(["taskflow", "run-graph", "init", "vida-memory-gov", "writer"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-memory-gov",
            "writer",
            "writer",
            "in_progress",
            "writer",
            "{\"policy_gate\":\"memory_correction_required\",\"context_state\":\"open\"}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("memory governance evidence shaping required"));
    assert!(stderr.contains("context_state must be `sealed`"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn run_graph_update_canonicalizes_conflicting_resume_meta() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let _ = run_and_assert_success(&["boot"], &state_dir);

    let _ = run_and_assert_success(
        &["taskflow", "run-graph", "init", "vida-memory-gov", "writer"],
        &state_dir,
    );

    let meta = "{\"resume_target\":\"dispatch.coach\",\"next_node\":\"writer\",\"handoff_state\":\"awaiting_writer\"}";
    let _ = run_and_assert_success(
        &[
            "taskflow",
            "run-graph",
            "update",
            "vida-memory-gov",
            "writer",
            "coach",
            "ready",
            "writer",
            meta,
        ],
        &state_dir,
    );

    let runtime = Runtime::new().expect("create tokio runtime");
    let (resume_target, next_node, handoff_state) = runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(&state_dir)
            .await
            .expect("open surreal store");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("use namespace/database");

        let mut resume_query = db
            .query("SELECT resume_target FROM resumability_capsule WHERE run_id = $run")
            .bind(("run", "vida-memory-gov"))
            .await
            .expect("query resumability");
        let resume_rows: Vec<Value> = resume_query.take(0).expect("take resume rows");
        let resume_row = resume_rows
            .first()
            .cloned()
            .expect("resumability capsule should exist");

        let mut execution_query = db
            .query("SELECT next_node FROM execution_plan_state WHERE run_id = $run")
            .bind(("run", "vida-memory-gov"))
            .await
            .expect("query execution plan");
        let execution_rows: Vec<Value> = execution_query.take(0).expect("take execution rows");
        let execution_row = execution_rows
            .first()
            .cloned()
            .expect("execution plan should exist");

        let mut governance_query = db
            .query("SELECT handoff_state FROM governance_state WHERE run_id = $run")
            .bind(("run", "vida-memory-gov"))
            .await
            .expect("query governance");
        let governance_rows: Vec<Value> = governance_query.take(0).expect("take governance rows");
        let governance_row = governance_rows
            .first()
            .cloned()
            .expect("governance state should exist");

        (
            resume_row["resume_target"].as_str().map(String::from),
            execution_row["next_node"].as_str().map(String::from),
            governance_row["handoff_state"].as_str().map(String::from),
        )
    });

    assert_eq!(resume_target.as_deref(), Some("dispatch.coach"));
    assert_eq!(next_node.as_deref(), Some("coach"));
    assert_eq!(handoff_state.as_deref(), Some("awaiting_coach"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_show_fails_closed_for_missing_task_id() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args(["task", "show", "vida-missing", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task show should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to show task: task is missing: vida-missing"));

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn task_list_json_ignores_render_color_emoji_styling() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert_json_status_pass(&import_stdout);

    let output = vida()
        .args(["task", "list", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "color_emoji")
        .output()
        .expect("task list should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(!stdout.contains('\u{1b}'));
    assert!(!stdout.contains("📘"));
    assert!(!stdout.contains("🔹"));

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("json output should parse");
    assert!(parsed.is_array(), "task list output should be json array");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn agent_feedback_fails_closed_for_unsupported_outcome() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "feedback-invalid-outcome",
            "--project-name",
            "Feedback Invalid Outcome",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let output = vida()
        .args([
            "agent-feedback",
            "--agent-id",
            "junior",
            "--score",
            "90",
            "--outcome",
            "deferred",
            "--task-class",
            "implementation",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("agent-feedback should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported feedback outcome `deferred`"));
    assert!(stderr.contains("Allowed values: success, failure, neutral."));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn donor_ready_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(&temp_root).expect("temp dir should be created");
    sample_jsonl(&jsonl_path);

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_ready = run_and_assert_success(&["task", "ready", "--json"], &state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_ready_semantic.json").trim_end();
    assert_eq!(
        donor_ready_semantic(&rust_ready),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn donor_show_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(&temp_root).expect("temp dir should be created");
    sample_jsonl(&jsonl_path);

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_show = run_and_assert_success(&["task", "show", "vida-b", "--json"], &state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_show_semantic.json").trim_end();
    assert_eq!(
        donor_show_semantic(&rust_show),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn donor_list_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(&temp_root).expect("temp dir should be created");
    fs::write(
        &jsonl_path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"alpha\"],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"in_progress\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[\"beta\"],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"third\",\"status\":\"closed\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"closed_at\":\"2026-03-09T00:00:00Z\",\"close_reason\":\"done\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
        ),
    )
    .expect("write task jsonl");

    let state_dir = format!("{temp_root}/state");
    let _import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    let rust_list = run_and_assert_success(&["task", "list", "--json"], &state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_list_semantic.json").trim_end();
    assert_eq!(
        donor_list_semantic(&rust_list),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn status_json_reports_dispatch_alias_registry_load_error_when_registry_missing() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let boot = vida()
        .arg("boot")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    fs::write(
        format!("{project_root}/vida.config.yaml"),
        r#"project:
  id: status-missing-registry
agent_extensions:
  enabled: true
  registries:
    dispatch_aliases: missing/dispatch-aliases.yaml
  enabled_framework_roles:
    - orchestrator
  validation:
    require_registry_files: true
agent_system:
  mode: native
  state_owner: orchestrator_only
"#,
    )
    .expect("config should be written");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "status-missing-registry",
            "--project-name",
            "Status Missing Registry",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("project activator should run");
    assert!(!activator.status.success());

    let status = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status should render json");
    assert_shared_fields_consistency(&parsed, "status surface");
    assert_operator_contracts_consistency(&parsed, "status surface");
    let load_error = parsed["host_agents"]["internal_dispatch_alias_load_error"]
        .as_str()
        .expect("internal_dispatch_alias_load_error should be present");
    assert!(load_error.contains("missing/dispatch-aliases.yaml"));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn status_json_reports_non_codex_host_agents_summary() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(init.status.success());

    let config_path = format!("{project_root}/vida.config.yaml");
    let mut config: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&config_path).expect("config exists"))
            .expect("config yaml should parse");
    let root = config
        .as_mapping_mut()
        .expect("config root should be a mapping");
    let host_env = root
        .get_mut(serde_yaml::Value::String("host_environment".to_string()))
        .and_then(serde_yaml::Value::as_mapping_mut)
        .expect("host_environment should exist");
    host_env.insert(
        serde_yaml::Value::String("cli_system".to_string()),
        serde_yaml::Value::String("qwen".to_string()),
    );
    fs::write(
        &config_path,
        serde_yaml::to_string(&config).expect("patched yaml should render"),
    )
    .expect("patch config");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "status-qwen",
            "--host-cli-system",
            "qwen",
            "--language",
            "english",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let boot = vida()
        .arg("boot")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let status = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status should render json");
    let host_agents = &parsed["host_agents"];
    assert_eq!(host_agents["host_cli_system"], "qwen");
    assert_eq!(host_agents["runtime_surface"], ".qwen");
    assert_eq!(host_agents["root_session_write_guard"]["status"], "missing");
    assert_eq!(parsed["root_session_write_guard"]["status"], "missing");
    let runtime_root = host_agents["runtime_root"]
        .as_str()
        .expect("runtime_root present");
    assert!(runtime_root.contains(".qwen"));
    let system_entry = &host_agents["system_entry"];
    assert!(system_entry.is_object());
    assert_eq!(
        system_entry["template_root"]
            .as_str()
            .expect("template_root"),
        ".qwen"
    );
    assert_eq!(
        system_entry["runtime_root"].as_str().expect("runtime_root"),
        ".qwen"
    );
    assert_eq!(
        system_entry["materialization_mode"]
            .as_str()
            .expect("materialization_mode"),
        "copy_tree_only"
    );
    assert_eq!(system_entry["enabled"].as_bool(), Some(true));
    assert_eq!(
        system_entry["carriers"]["qwen-primary"]["tier"]
            .as_str()
            .expect("carrier tier"),
        "qwen"
    );
    assert_eq!(
        system_entry["carriers"]["qwen-primary"]["rate"].as_i64(),
        Some(4)
    );
    let agents = host_agents["agents"]
        .as_object()
        .expect("agents summary should render");
    let qwen = agents
        .get("qwen-primary")
        .expect("qwen carrier summary should render");
    assert_eq!(qwen["tier"].as_str().expect("tier"), "qwen");
    assert_eq!(qwen["rate"].as_i64(), Some(4));
    assert_eq!(
        qwen["default_runtime_role"]
            .as_str()
            .expect("default runtime role"),
        "worker"
    );
    assert_eq!(qwen["feedback_count"].as_u64(), Some(0));
    assert!(qwen["effective_score"].is_null());
    assert!(qwen["lifecycle_state"].is_null());
    assert_eq!(
        host_agents["selection_policy"]["rule"],
        "capability_first_then_score_guard_then_cheapest_tier"
    );
    assert_eq!(host_agents["external_cli_preflight"]["status"], "pass");
    assert_eq!(
        host_agents["external_cli_preflight"]["requires_external_cli"],
        true
    );
    assert_eq!(
        host_agents["external_cli_preflight"]["external_cli_subagents_present"],
        false
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn status_json_restores_root_session_guard_after_consume_continue_snapshot() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output();
    let boot = boot.expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let dispatch_packets_dir = format!("{runtime_consumption_dir}/dispatch-packets");
    fs::create_dir_all(&dispatch_packets_dir).expect("dispatch packet dir should exist");

    let dispatch_packet_path = format!("{dispatch_packets_dir}/resume-packet.json");
    fs::write(
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
    fs::write(
        format!("{runtime_consumption_dir}/final-2026-03-19T00-00-00Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume continue",
            "source_dispatch_packet_path": dispatch_packet_path
        })
        .to_string(),
    )
    .expect("final snapshot should write");

    let status = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        status["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        status["host_agents"]["root_session_write_guard"]["status"],
        "blocked_by_default"
    );

    fs::remove_dir_all(state_dir).expect("state dir should be removed");
}

#[test]
fn status_json_prefers_latest_final_snapshot_guard_when_latest_snapshot_is_bundle_check() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output();
    let boot = boot.expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let runtime_consumption_dir = format!("{state_dir}/runtime-consumption");
    let dispatch_packets_dir = format!("{runtime_consumption_dir}/dispatch-packets");
    fs::create_dir_all(&dispatch_packets_dir).expect("dispatch packet dir should exist");

    let dispatch_packet_path = format!("{dispatch_packets_dir}/guard-packet.json");
    fs::write(
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
    fs::write(
        format!("{runtime_consumption_dir}/final-2026-03-19T00-00-01Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume continue",
            "source_dispatch_packet_path": dispatch_packet_path
        })
        .to_string(),
    )
    .expect("final snapshot should write");

    thread::sleep(Duration::from_millis(15));
    fs::write(
        format!("{runtime_consumption_dir}/bundle-check-2026-03-19T00-00-02Z.json"),
        serde_json::json!({
            "surface": "vida taskflow consume bundle check",
            "check": { "ok": true }
        })
        .to_string(),
    )
    .expect("bundle-check snapshot should write");

    let status = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        status["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        status["host_agents"]["root_session_write_guard"]["status"],
        "blocked_by_default"
    );

    fs::remove_dir_all(state_dir).expect("state dir should be removed");
}

#[test]
fn status_json_blocks_external_cli_when_sandbox_active_and_network_unreachable() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");
    let state_dir = format!("{project_root}/.vida/data/state");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("init should run");
    assert!(init.status.success());

    let config_path = format!("{project_root}/vida.config.yaml");
    let mut config: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&config_path).expect("config exists"))
            .expect("config yaml should parse");
    let root = config
        .as_mapping_mut()
        .expect("config root should be a mapping");
    let host_env = root
        .get_mut(serde_yaml::Value::String("host_environment".to_string()))
        .and_then(serde_yaml::Value::as_mapping_mut)
        .expect("host_environment should exist");
    host_env.insert(
        serde_yaml::Value::String("cli_system".to_string()),
        serde_yaml::Value::String("qwen".to_string()),
    );
    fs::write(
        &config_path,
        serde_yaml::to_string(&config).expect("patched yaml should render"),
    )
    .expect("patch config");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "status-qwen-offline",
            "--host-cli-system",
            "qwen",
            "--language",
            "english",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let status = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("CODEX_SANDBOX_MODE", "workspace-write")
        .env("VIDA_NETWORK_PROBE_OVERRIDE", "unreachable")
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status should render json");
    let preflight = &parsed["host_agents"]["external_cli_preflight"];
    assert_eq!(preflight["status"], "blocked");
    assert_eq!(
        preflight["blocker_code"],
        "external_cli_network_access_unavailable_under_sandbox"
    );
    assert!(preflight["next_actions"]
        .as_array()
        .expect("next actions should be array")
        .iter()
        .any(|row| row
            .as_str()
            .unwrap_or_default()
            .contains("Allow network access")));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn consume_bundle_check_exposes_shared_operator_contract_fields() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = run_command_capture(
        &["taskflow", "consume", "bundle", "check", "--json"],
        &state_dir,
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume bundle check json should parse");

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
    assert_eq!(
        parsed["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
    );
    assert!(
        matches!(
            parsed["operator_contracts"]["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "operator_contracts.status must stay within release-1 canonical enum"
    );
    assert_eq!(
        parsed["artifact_refs"]["root_artifact_id"],
        parsed["check"]["root_artifact_id"]
    );
    assert_eq!(
        parsed["artifact_refs"]["bundle_artifact_name"],
        "taskflow_runtime_bundle"
    );
    assert_eq!(
        parsed["artifact_refs"]["surface"],
        "vida taskflow consume bundle check"
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn consume_bundle_check_contract_id_stays_within_release1_canonical_enum() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let stdout = String::from_utf8_lossy(&status.stdout).into_owned();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("status json should parse");

    assert!(
        matches!(
            parsed["operator_contracts"]["contract_id"].as_str(),
            Some("release-1-operator-contracts") | Some("release-1-shared-fields")
        ),
        "operator_contracts.contract_id must stay within release-1 canonical enum"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_task_import_export_statuses_are_canonical() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    let import_output = vida()
        .args(["taskflow", "task", "import-jsonl", &import_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task import should run");
    assert!(import_output.status.success());
    let import_json: serde_json::Value =
        serde_json::from_slice(&import_output.stdout).expect("import json should parse");
    assert_eq!(import_json["status"], "pass");

    let export_path = format!("{state_dir}/exported.jsonl");
    let export_output = vida()
        .args(["taskflow", "task", "export-jsonl", &export_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task export should run");
    assert!(export_output.status.success());
    let export_json: serde_json::Value =
        serde_json::from_slice(&export_output.stdout).expect("export json should parse");
    assert_eq!(export_json["status"], "pass");

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn taskflow_task_import_ignores_legacy_helper_status_override_env() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    let output = vida()
        .args(["taskflow", "task", "import-jsonl", &import_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASK_BRIDGE_STATUS_OVERRIDE", "bananas")
        .output()
        .expect("task import should run");
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("import json should parse");
    assert_eq!(parsed["status"], "pass");

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn taskflow_task_update_ignores_legacy_helper_status_override_env() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    sample_jsonl(&import_path);

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let output = vida()
        .args([
            "taskflow", "task", "update", "vida-a", "--status", "pass", "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASK_BRIDGE_STATUS_OVERRIDE", "bananas")
        .output()
        .expect("task update should run");
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("update json should parse");
    assert_eq!(parsed["status"], "pass");

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_update_accepts_notes_file_for_shell_safe_progress_recording() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    let notes_path = format!("{state_dir}/notes.txt");
    sample_jsonl(&import_path);
    fs::write(
        &notes_path,
        "line 1\nline 2 with `backticks` and $(shell-like text)\n",
    )
    .expect("notes file should write");

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let parsed = run_command_json(
        &[
            "task",
            "update",
            "vida-a",
            "--status",
            "in_progress",
            "--notes-file",
            &notes_path,
            "--json",
        ],
        &state_dir,
    );

    assert_eq!(parsed["status"], "in_progress");
    assert_eq!(
        parsed["notes"],
        "line 1\nline 2 with `backticks` and $(shell-like text)\n"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn task_update_rejects_notes_and_notes_file_together() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");
    let import_path = format!("{state_dir}/tasks.jsonl");
    let notes_path = format!("{state_dir}/notes.txt");
    sample_jsonl(&import_path);
    fs::write(&notes_path, "safe note\n").expect("notes file should write");

    run_and_assert_success(
        &["task", "import-jsonl", &import_path, "--json"],
        &state_dir,
    );

    let output = run_command_capture(
        &[
            "task",
            "update",
            "vida-a",
            "--notes",
            "inline",
            "--notes-file",
            &notes_path,
            "--json",
        ],
        &state_dir,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Use only one notes source: --notes <text> or --notes-file <path>"),
        "stderr was: {stderr}"
    );

    fs::remove_dir_all(&state_dir).expect("cleanup state dir");
}

#[test]
fn consume_bundle_check_blocked_path_matches_blocker_codes_contract() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let output = vida()
        .args(["taskflow", "consume", "bundle", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume bundle check should run");
    assert!(
        !output.status.success(),
        "blocked consume bundle check should fail closed"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("consume bundle check blocked json should parse");

    let required_operator_blocker_codes = [
        "missing_protocol_binding_receipt",
        "protocol_binding_not_runtime_ready",
    ];
    let required_check_blocker_codes = [
        "missing_protocol_binding_rows",
        "missing_protocol_binding_receipt",
        "protocol_binding_not_runtime_ready",
        "invalid_metadata_tuple_key:protocol_binding_revision",
    ];
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
    );
    assert_eq!(parsed["operator_contracts"]["status"], "blocked");
    assert!(
        matches!(
            parsed["operator_contracts"]["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "operator_contracts.status must stay within release-1 canonical enum"
    );
    assert_eq!(
        parsed["blocker_codes"],
        parsed["operator_contracts"]["blocker_codes"]
    );
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array");
    let check_blockers = parsed["check"]["blockers"]
        .as_array()
        .expect("check blockers should be an array");
    for code in required_operator_blocker_codes {
        assert!(
            blocker_codes
                .iter()
                .any(|value| value.as_str() == Some(code)),
            "missing required blocker code: {code}"
        );
    }
    for code in required_check_blocker_codes {
        assert!(
            check_blockers
                .iter()
                .any(|value| value.as_str() == Some(code)),
            "missing required check blocker code: {code}"
        );
    }

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn consume_final_blocks_when_execution_preparation_is_required_without_handoff_evidence() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "architecture refactor implementation patch",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume final json should parse");

    let execution_plan = &parsed["payload"]["role_selection"]["execution_plan"];
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let required = dispatch_contract["execution_preparation_required"].as_bool() == Some(true)
        || dispatch_contract["lane_catalog"]
            .get("execution_preparation")
            .is_some()
        || dispatch_contract["lane_sequence"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|value| value.as_str() == Some("execution_preparation"));

    if required {
        let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
            .as_array()
            .expect("closure blockers should be an array");
        assert!(
            closure_blockers
                .iter()
                .any(|value| value.as_str() == Some("pending_execution_preparation_evidence")),
            "required execution_preparation lane must block without evidence/handoff packet"
        );
        assert_eq!(parsed["operator_contracts"]["status"], "blocked");
        assert!(parsed["blocker_codes"]
            .as_array()
            .expect("blocker_codes should be an array")
            .iter()
            .any(|value| value.as_str() == Some("closure_admission_block")));
        assert_eq!(
            parsed["payload"]["dispatch_receipt"]["blocker_code"],
            "pending_execution_preparation_evidence"
        );
        assert!(
            parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
                .as_array()
                .expect("downstream blockers should be an array")
                .iter()
                .any(|value| value.as_str() == Some("pending_execution_preparation_evidence"))
        );
        assert!(
            !parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
                .as_array()
                .expect("downstream blockers should be an array")
                .iter()
                .any(|value| {
                    matches!(
                        value.as_str(),
                        Some("unsupported_boundary") | Some("retrieval_evidence")
                    )
                }),
            "execution_preparation gate should not leak unsupported_boundary/retrieval_evidence blockers"
        );
        assert!(
            !parsed["blocker_codes"]
                .as_array()
                .expect("blocker_codes should be an array")
                .iter()
                .any(|value| {
                    matches!(
                        value.as_str(),
                        Some("unsupported_boundary") | Some("retrieval_evidence")
                    )
                }),
            "execution_preparation gate should not leak unsupported_boundary/retrieval_evidence blocker_codes"
        );
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn consume_final_blocks_when_approval_or_delegation_wait_lacks_evidence() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "implementation review approval handoff",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume final json should parse");

    let latest_status = &parsed["payload"]["run_graph_bootstrap"]["latest_status"];
    let handoff_state = latest_status["handoff_state"].as_str().unwrap_or_default();
    let policy_gate = latest_status["policy_gate"].as_str().unwrap_or_default();
    let lifecycle_stage = latest_status["lifecycle_stage"]
        .as_str()
        .unwrap_or_default();
    let combined = format!(
        "{} {} {}",
        handoff_state.to_ascii_lowercase(),
        policy_gate.to_ascii_lowercase(),
        lifecycle_stage.to_ascii_lowercase()
    );
    let approval_or_delegation_wait = combined.contains("approval") || combined.contains("delegat");
    if approval_or_delegation_wait {
        assert_eq!(
            latest_status["status"], "awaiting_approval",
            "approval/delegation wait branch should surface the structured approval wait status"
        );
        assert_eq!(latest_status["lifecycle_stage"], "approval_wait");
        assert_eq!(latest_status["policy_gate"], "approval_required");
        assert_eq!(latest_status["handoff_state"], "awaiting_approval");
        assert_eq!(latest_status["resume_target"], "dispatch.approval");
        assert_eq!(latest_status["next_node"], "approval");

        let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
            .as_array()
            .expect("closure blockers should be an array");
        assert!(
            closure_blockers
                .iter()
                .any(|value| value.as_str() == Some("pending_approval_delegation_evidence")),
            "approval/delegation wait branch must fail closed without evidence"
        );
        assert_eq!(
            parsed["payload"]["dispatch_receipt"]["blocker_code"],
            "pending_approval_delegation_evidence"
        );
        assert!(
            parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
                .as_array()
                .expect("downstream blockers should be an array")
                .iter()
                .any(|value| value.as_str() == Some("pending_approval_delegation_evidence"))
        );
    }

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_fails_closed_on_retrieval_decision_gate_when_not_synced() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let check = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding check should run");
    assert!(!check.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&check.stdout).expect("protocol-binding check json should parse");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(parsed["decision_gate"]["policy_gate"], "retrieval_evidence");
    assert_eq!(parsed["decision_gate"]["ready"], false);

    let blocker_code = parsed["decision_gate"]["blocker_code"]
        .as_str()
        .expect("decision gate blocker code should be present");
    assert!(
        blocker_code == "missing_protocol_binding_receipt"
            || blocker_code == "protocol_binding_not_runtime_ready",
        "unexpected decision gate blocker code: {blocker_code}"
    );
    assert_shared_fields_consistency(&parsed, "protocol-binding check");
    assert_operator_contracts_consistency(&parsed, "protocol-binding check");
    let contract_blockers = parsed["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should be array");
    assert_eq!(
        contract_blockers[0].as_str().unwrap(),
        blocker_code,
        "operator_contracts.blocker_codes must mirror decision_gate blocker_code"
    );
    assert!(
        parsed["operator_contracts"]["next_actions"]
            .as_array()
            .expect("operator_contracts.next_actions should be array")
            .iter()
            .any(|action| {
                action
                    .as_str()
                    .unwrap()
                    .contains("protocol-binding check --json")
            }),
        "operator_contracts.next_actions must reference protocol-binding check guidance"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_lock_retry_preserves_blocker_codes() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER.store(0, Ordering::SeqCst);
    let output = run_with_state_lock_retry(|| {
        let attempt = PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            let mut simul = Command::new("sh");
            simul
                .arg("-c")
                .arg("printf 'LOCK is already locked\\n' >&2; exit 1");
            simul
        } else {
            let mut command = vida();
            command
                .args(["taskflow", "protocol-binding", "check", "--json"])
                .env("VIDA_STATE_DIR", &state_dir);
            command
        }
    });

    assert!(!output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("protocol-binding check json should parse");
    assert_eq!(parsed["status"], "blocked");
    let blocker_code = parsed["decision_gate"]["blocker_code"]
        .as_str()
        .expect("decision gate blocker code should be present");
    assert!(
        blocker_code == "missing_protocol_binding_receipt"
            || blocker_code == "protocol_binding_not_runtime_ready",
        "unexpected decision gate blocker code: {blocker_code}"
    );
    let contract_blockers = parsed["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should be array");
    assert_eq!(
        contract_blockers[0].as_str().unwrap(),
        blocker_code,
        "operator_contracts.blocker_codes must mirror decision_gate blocker_code"
    );
    let shared_blockers = parsed["shared_fields"]["blocker_codes"]
        .as_array()
        .expect("shared_fields.blocker_codes should be array");
    assert_eq!(
        shared_blockers[0].as_str().unwrap(),
        blocker_code,
        "shared_fields.blocker_codes must mirror decision_gate blocker_code"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_plain_json_parity() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let plain_output = run_command_capture(&["taskflow", "protocol-binding", "check"], &state_dir);
    assert!(!plain_output.status.success());
    let plain_stdout = String::from_utf8_lossy(&plain_output.stdout);
    let plain_status = extract_plain_surface_line(&plain_stdout, "status");
    let plain_blockers: Vec<String> =
        serde_json::from_str(&extract_plain_surface_line(&plain_stdout, "blocker_codes"))
            .expect("blocker_codes should render as json array for plain output");
    let plain_next_actions: Vec<String> =
        serde_json::from_str(&extract_plain_surface_line(&plain_stdout, "next_actions"))
            .expect("next_actions should render as json array for plain output");
    let plain_shared_fields: serde_json::Value =
        serde_json::from_str(&extract_plain_surface_line(&plain_stdout, "shared_fields"))
            .expect("shared_fields should render as json object for plain output");
    let plain_operator_contracts: serde_json::Value = serde_json::from_str(
        &extract_plain_surface_line(&plain_stdout, "operator_contracts"),
    )
    .expect("operator_contracts should render as json object for plain output");

    let json_output = run_command_capture(
        &["taskflow", "protocol-binding", "check", "--json"],
        &state_dir,
    );
    let parsed_json: serde_json::Value = serde_json::from_slice(&json_output.stdout)
        .expect("protocol-binding check json should parse");
    let json_status = parsed_json["status"]
        .as_str()
        .expect("status should be string");
    assert_eq!(plain_status, json_status);
    let json_blockers = parsed_json["blocker_codes"]
        .as_array()
        .expect("json blocker_codes should be array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("blocker code should be string")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(plain_blockers, json_blockers);
    let json_next_actions = parsed_json["next_actions"]
        .as_array()
        .expect("json next_actions should be array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("next action should be string")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(plain_next_actions, json_next_actions);
    assert_eq!(plain_shared_fields, parsed_json["shared_fields"]);
    assert_eq!(plain_operator_contracts, parsed_json["operator_contracts"]);

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn consume_final_fails_closed_on_retrieval_policy_gate_when_not_synced() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "architecture refactor implementation patch",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    assert!(
        !output.status.success(),
        "consume final must fail closed when protocol binding evidence is missing"
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("consume final json should parse");

    let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
        .as_array()
        .expect("closure blockers should be an array");
    assert!(
        closure_blockers
            .iter()
            .any(|value| value.as_str() == Some("missing_protocol_binding_receipt"))
            || closure_blockers
                .iter()
                .any(|value| value.as_str() == Some("protocol_binding_not_runtime_ready")),
        "consume final must keep retrieval gate protocol-binding blockers in closure admission"
    );
    assert_eq!(parsed["operator_contracts"]["status"], "blocked");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn cross_surface_protocol_binding_parity() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync_json = run_command_json(
        &["taskflow", "protocol-binding", "sync", "--json"],
        &state_dir,
    );
    assert!(
        sync_json["compiled_payload_import_evidence"]["trusted"]
            .as_bool()
            .unwrap_or(false),
        "protocol-binding sync must produce trusted compiled payload evidence"
    );
    let receipt_id = require_json_string(
        &sync_json["receipt"]["receipt_id"],
        "protocol-binding sync receipt id",
    );

    let pb_status_json = run_command_json(
        &["taskflow", "protocol-binding", "status", "--json"],
        &state_dir,
    );
    let pb_summary = &pb_status_json["summary"];
    let blocking_issues = pb_summary["blocking_issue_count"]
        .as_u64()
        .expect("protocol-binding summary should expose blocking_issue_count");
    assert_eq!(
        require_json_string(
            &pb_summary["latest_receipt_id"],
            "protocol-binding summary latest_receipt_id"
        ),
        receipt_id
    );

    let consume_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "cross surface parity",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume_output.stdout).expect("consume final json should parse");
    assert_eq!(consume_json["surface"], "vida taskflow consume final");
    let consume_run_id = require_json_string(
        &consume_json["payload"]["dispatch_receipt"]["run_id"],
        "consume dispatch run_id",
    );
    let consume_artifact_run_id = require_json_string(
        &consume_json["operator_contracts"]["artifact_refs"]
            ["latest_run_graph_dispatch_receipt_id"],
        "consume artifact refs latest run graph dispatch receipt id",
    );

    let status_json = run_command_json(&["status", "--json"], &state_dir);
    let doctor_json = run_command_json(&["doctor", "--json"], &state_dir);

    let status_proto_id = require_json_string(
        &status_json["protocol_binding"]["latest_receipt_id"],
        "status protocol_binding latest_receipt_id",
    );
    let doctor_proto_id = require_json_string(
        &doctor_json["protocol_binding"]["latest_receipt_id"],
        "doctor protocol_binding latest_receipt_id",
    );
    assert!(status_proto_id.starts_with("protocol-binding-"));
    assert!(doctor_proto_id.starts_with("protocol-binding-"));
    assert_eq!(
        status_json["protocol_binding"]["blocking_issue_count"]
            .as_u64()
            .expect("status should expose blocking_issue_count"),
        blocking_issues
    );
    assert_eq!(
        doctor_json["protocol_binding"]["blocking_issue_count"]
            .as_u64()
            .expect("doctor should expose blocking_issue_count"),
        blocking_issues
    );

    let status_artifact_refs = &status_json["artifact_refs"];
    let doctor_artifact_refs = &doctor_json["operator_contracts"]["artifact_refs"];
    let doctor_root_trace = &doctor_json["trace_evidence"]["root_trace"];
    assert_eq!(
        require_json_string(
            &status_artifact_refs["protocol_binding_latest_receipt_id"],
            "status artifact_refs protocol_binding_latest_receipt_id"
        ),
        status_proto_id
    );
    assert_eq!(
        require_json_string(
            &doctor_artifact_refs["protocol_binding_latest_receipt_id"],
            "doctor artifact_refs protocol_binding_latest_receipt_id"
        ),
        doctor_proto_id
    );
    assert_eq!(
        doctor_artifact_refs["retrieval_trust_signal"]["source"],
        "runtime_consumption_snapshot_index"
    );
    assert_eq!(
        doctor_artifact_refs["retrieval_trust_signal"]["citation"],
        status_artifact_refs["runtime_consumption_latest_snapshot_path"]
    );
    assert_eq!(
        doctor_artifact_refs["retrieval_trust_signal"]["acl"],
        status_artifact_refs["protocol_binding_latest_receipt_id"]
    );
    assert_eq!(
        doctor_root_trace["runtime_consumption_latest_snapshot_path"],
        doctor_artifact_refs["retrieval_trust_signal"]["citation"]
    );
    assert_eq!(doctor_json["trace_evidence"]["status"], "pass");
    assert_eq!(
        doctor_root_trace["latest_run_graph_dispatch_receipt_id"],
        status_artifact_refs["latest_run_graph_dispatch_receipt_id"]
    );
    assert_eq!(
        doctor_root_trace["protocol_binding_latest_receipt_id"],
        status_artifact_refs["protocol_binding_latest_receipt_id"]
    );
    assert_eq!(
        doctor_root_trace["runtime_consumption_latest_snapshot_path"],
        status_artifact_refs["runtime_consumption_latest_snapshot_path"]
    );

    let status_run_id = require_json_string(
        &status_json["latest_run_graph_dispatch_receipt"]["run_id"],
        "status latest run graph dispatch receipt run_id",
    );
    let doctor_run_id = require_json_string(
        &doctor_artifact_refs["latest_run_graph_dispatch_receipt_id"],
        "doctor artifact_refs latest run graph dispatch receipt id",
    );
    assert_eq!(status_run_id, doctor_run_id);
    assert_eq!(status_run_id, consume_run_id);
    assert_eq!(status_run_id, consume_artifact_run_id);

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn cross_surface_protocol_binding_blocker_parity() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    run_and_assert_success(&["boot"], &state_dir);

    let status_json = run_command_json(&["status", "--json"], &state_dir);
    let doctor_json = run_command_json(&["doctor", "--json"], &state_dir);

    let status_blocker_codes =
        require_string_array(&status_json["blocker_codes"], "status blocker_codes");
    let doctor_blocker_codes =
        require_string_array(&doctor_json["blocker_codes"], "doctor blocker_codes");
    assert!(
        status_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_operator_evidence"),
        "status should fail closed on missing retrieval-trust operator evidence"
    );
    assert!(
        status_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_source_operator_evidence"),
        "status should fail closed on missing retrieval-trust source evidence"
    );
    assert!(
        status_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_signal_operator_evidence"),
        "status should fail closed on missing retrieval-trust signal evidence"
    );
    assert!(
        doctor_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_operator_evidence"),
        "doctor should fail closed on missing retrieval-trust operator evidence"
    );
    assert!(
        doctor_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_source_operator_evidence"),
        "doctor should fail closed on missing retrieval-trust source evidence"
    );
    assert!(
        doctor_blocker_codes
            .iter()
            .any(|code| code == "missing_retrieval_trust_signal_operator_evidence"),
        "doctor should fail closed on missing retrieval-trust signal evidence"
    );

    let doctor_next_actions =
        require_string_array(&doctor_json["next_actions"], "doctor next_actions");
    assert!(
        doctor_next_actions
            .iter()
            .any(|action| action.contains("protocol-binding sync")),
        "doctor next actions should include protocol-binding sync guidance"
    );
    assert!(
        doctor_next_actions
            .iter()
            .any(|action| action.contains("consume bundle check")),
        "doctor next actions should include consume bundle check guidance"
    );
    assert_eq!(doctor_json["trace_evidence"]["status"], "blocked");

    assert_eq!(
        status_json["protocol_binding"]["blocking_issue_count"],
        doctor_json["protocol_binding"]["blocking_issue_count"]
    );
    assert_eq!(
        status_json["protocol_binding"]["latest_receipt_id"],
        doctor_json["protocol_binding"]["latest_receipt_id"]
    );

    let consume_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "cross surface parity block",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    assert!(
        !consume_output.status.success(),
        "consume final must fail closed on protocol-binding absence"
    );
    let consume_json: serde_json::Value =
        serde_json::from_slice(&consume_output.stdout).expect("consume final json should parse");
    assert!(consume_json["trace_id"].is_null());
    assert!(consume_json["workflow_class"].is_null());
    assert!(consume_json["risk_tier"].is_null());
    let consume_blockers = require_string_array(
        &consume_json["payload"]["closure_admission"]["blockers"],
        "consume closure blockers",
    );
    assert!(
        consume_blockers
            .iter()
            .any(|code| code == "missing_protocol_binding_receipt"),
        "consume final should keep protocol-binding blockers in closure admission"
    );
    assert!(
        consume_blockers
            .iter()
            .any(|code| code.contains("protocol_binding")),
        "consume closure blockers should preserve protocol-binding-family evidence"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_operator_contract_parity() {
    let state_dir = unique_state_dir();
    fs::create_dir_all(&state_dir).expect("create state dir");

    run_and_assert_success(&["boot"], &state_dir);

    let initial_status_json = run_command_json(&["status", "--json"], &state_dir);
    let initial_blocking_count = initial_status_json["protocol_binding"]["blocking_issue_count"]
        .as_u64()
        .expect("status protocol_binding blocking_issue_count should exist");
    let initial_operator_status = initial_status_json["operator_contracts"]["status"]
        .as_str()
        .expect("operator_contracts.status should exist before sync");
    assert_eq!(
        initial_status_json["status"], initial_status_json["operator_contracts"]["status"],
        "top-level status must mirror the operator contract status before sync"
    );
    assert!(
        matches!(initial_operator_status, "pass" | "blocked"),
        "operator_contracts.status must stay within the release-1 canonical status enum before sync"
    );
    if initial_blocking_count > 0 {
        assert_eq!(
            initial_operator_status, "blocked",
            "protocol-binding blockers must force the top-level operator contract into blocked status before sync"
        );
    }

    let initial_pb_status_json = run_command_json(
        &["taskflow", "protocol-binding", "status", "--json"],
        &state_dir,
    );
    let pb_summary = &initial_pb_status_json["summary"];
    let pb_blocking = pb_summary["blocking_issue_count"]
        .as_u64()
        .expect("protocol-binding status summary blocking_issue_count should exist");
    assert_eq!(
        pb_blocking, initial_blocking_count,
        "status surface and protocol-binding status must agree on blocking_issue_count"
    );

    let sync_json = run_command_json(
        &["taskflow", "protocol-binding", "sync", "--json"],
        &state_dir,
    );
    assert!(
        sync_json["compiled_payload_import_evidence"]["trusted"]
            .as_bool()
            .unwrap_or(false),
        "protocol-binding sync must produce trusted compiled payload evidence"
    );

    let post_sync_status_json = run_command_json(&["status", "--json"], &state_dir);
    assert_eq!(
        post_sync_status_json["protocol_binding"]["blocking_issue_count"]
            .as_u64()
            .expect("status protocol_binding blocking_issue_count should exist after sync"),
        0,
        "canonical protocol-binding parity requires zero blockers after sync"
    );
    let post_sync_operator_status = post_sync_status_json["operator_contracts"]["status"]
        .as_str()
        .expect("operator_contracts.status should exist after sync");
    assert_eq!(
        post_sync_status_json["status"], post_sync_status_json["operator_contracts"]["status"],
        "top-level status must mirror the operator contract once blockers clear"
    );
    assert!(
        matches!(post_sync_operator_status, "pass" | "blocked"),
        "operator_contracts.status must remain within the release-1 canonical status enum after sync"
    );
    let post_sync_blockers = post_sync_status_json["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should remain an array after sync");
    assert!(
        !post_sync_blockers
            .iter()
            .filter_map(|value| value.as_str())
            .any(|code| code == "protocol_binding_blocking_issues"),
        "top-level operator contracts must stop reporting protocol-binding blockers after sync clears them"
    );

    let post_sync_pb_status_json = run_command_json(
        &["taskflow", "protocol-binding", "status", "--json"],
        &state_dir,
    );
    assert_eq!(
        post_sync_pb_status_json["summary"]["blocking_issue_count"],
        post_sync_status_json["protocol_binding"]["blocking_issue_count"],
        "status surface and protocol-binding status must stay aligned on blocking_issue_count after sync"
    );
    assert_eq!(
        post_sync_pb_status_json["summary"]["latest_receipt_id"],
        post_sync_status_json["protocol_binding"]["latest_receipt_id"],
        "latest_receipt_id must remain canonical across surfaces"
    );
    assert_eq!(
        post_sync_status_json["artifact_refs"]["protocol_binding_latest_receipt_id"],
        post_sync_status_json["protocol_binding"]["latest_receipt_id"],
        "status artifact_refs should mirror the canonical protocol-binding latest receipt after sync"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn protocol_binding_check_statuses_are_canonical() {
    let state_dir = unique_state_dir();
    run_and_assert_success(&["boot"], &state_dir);

    let output = run_command_capture(
        &["taskflow", "protocol-binding", "check", "--json"],
        &state_dir,
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("protocol-binding check json should parse");
    let top_status = json["status"]
        .as_str()
        .expect("protocol-binding check status should be string");
    let shared_status = json["shared_fields"]["status"]
        .as_str()
        .expect("shared_fields.status should be string");
    let contract_status = json["operator_contracts"]["status"]
        .as_str()
        .expect("operator_contracts.status should be string");
    assert_eq!(top_status, shared_status);
    assert_eq!(shared_status, contract_status);
    assert!(
        matches!(top_status, "pass" | "blocked"),
        "protocol-binding check status must remain canonical"
    );
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn consume_continue_fails_closed_on_lane_governance_status_evidence_conflict() {
    let (project_root, state_dir) = project_bound_state_dir();

    run_and_assert_success(&["boot"], &state_dir);

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let final_output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "resume lane governance conflict",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    let final_parsed: serde_json::Value =
        serde_json::from_slice(&final_output.stdout).expect("consume final json should parse");
    assert_eq!(final_parsed["surface"], "vida taskflow consume final");
    assert!(
        matches!(
            final_parsed["status"].as_str(),
            Some("pass") | Some("blocked")
        ),
        "consume final status must remain within the canonical enum"
    );

    let runtime_consumption_root = format!("{state_dir}/runtime-consumption");
    for relative_dir in ["dispatch-packets", "downstream-dispatch-packets"] {
        let dir_path = format!("{runtime_consumption_root}/{relative_dir}");
        let Ok(entries) = fs::read_dir(&dir_path) else {
            continue;
        };
        for entry in entries {
            let entry = entry.expect("read runtime-consumption entry");
            let file_type = entry.file_type().expect("entry file type");
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let packet_body = fs::read_to_string(&path).expect("read runtime packet");
            let mut packet: serde_json::Value =
                serde_json::from_str(&packet_body).expect("parse runtime packet");
            packet["lane_status"] = serde_json::Value::String("lane_open".to_string());
            packet["supersedes_receipt_id"] =
                serde_json::Value::String("receipt-superseded-1".to_string());
            packet["downstream_lane_status"] = serde_json::Value::String("lane_open".to_string());
            packet["downstream_supersedes_receipt_id"] =
                serde_json::Value::String("receipt-superseded-1".to_string());
            fs::write(
                &path,
                serde_json::to_vec_pretty(&packet).expect("serialize runtime packet"),
            )
            .expect("write runtime packet");
        }
    }

    let continue_output = vida()
        .args(["taskflow", "consume", "continue", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume continue should run");
    assert!(
        !continue_output.status.success(),
        "consume continue must fail closed on lane governance conflict"
    );
    let stderr = String::from_utf8_lossy(&continue_output.stderr);
    assert!(
        stderr.contains("conflicts with derived lane_status")
            || stderr.contains("execution_preparation_gate_blocked"),
        "stderr should fail closed for lane governance conflict packet, got: {stderr}"
    );

    let _ = fs::remove_dir_all(&project_root);
}
