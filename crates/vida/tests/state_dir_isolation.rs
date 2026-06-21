use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vida_test_support as support;

fn vida() -> Command {
    support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_project_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    PathBuf::from("/tmp").join(format!(
        "vida-{label}-{}-{nanos}-{counter}",
        std::process::id()
    ))
}

fn default_state_dir(project_root: &Path) -> PathBuf {
    project_root.join(".vida").join("data").join("state")
}

fn create_project_root(project_root: &Path) {
    std::fs::create_dir_all(project_root.join(".vida").join("config"))
        .expect("project config dir should exist");
    std::fs::create_dir_all(project_root.join(".vida").join("db"))
        .expect("project db dir should exist");
    std::fs::create_dir_all(project_root.join(".vida").join("project"))
        .expect("project marker dir should exist");
    std::fs::write(project_root.join("AGENTS.md"), "# Test agents\n")
        .expect("AGENTS.md should be written");
    std::fs::write(project_root.join("vida.config.yaml"), "project_id: test\n")
        .expect("vida.config.yaml should be written");
}

fn enable_parallel_scheduler(project_root: &Path, max_parallel_agents: u32) {
    std::fs::write(
        project_root.join("vida.config.yaml"),
        format!("project_id: test\nagent_system:\n  max_parallel_agents: {max_parallel_agents}\n"),
    )
    .expect("vida.config.yaml should enable parallel scheduler");
}

fn run_with_retry<F>(mut build: F) -> Output
where
    F: FnMut() -> Command,
{
    let mut last_output = None;
    for attempt in 0..6 {
        let output = build()
            .output()
            .unwrap_or_else(|error| panic!("vida command should run: {error}"));
        if !is_state_lock_error(&output) {
            return output;
        }
        last_output = Some(output);
        thread::sleep(Duration::from_millis(150 * (attempt + 1)));
    }
    last_output.expect("state lock retry should record output")
}

fn is_state_lock_error(output: &Output) -> bool {
    if output.status.success() {
        return false;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    stderr.contains("timed out while waiting for authoritative datastore lock")
        || stdout.contains("timed out while waiting for authoritative datastore lock")
        || stderr.contains("authoritative_state_required_for_mutation")
        || stdout.contains("authoritative_state_required_for_mutation")
}

fn run_success(project_root: &Path, state_dir: Option<&Path>, args: &[&str]) -> Output {
    let output = run_with_retry(|| {
        let mut command = vida();
        command
            .args(args)
            .current_dir(project_root)
            .env_remove("VIDA_STATE_DIR");
        if let Some(state_dir) = state_dir {
            command.env("VIDA_STATE_DIR", state_dir);
        }
        command
    });
    assert!(
        output.status.success(),
        "{} should succeed: stdout={}; stderr={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_json_success(project_root: &Path, state_dir: Option<&Path>, args: &[&str]) -> Value {
    let output = run_success(project_root, state_dir, args);
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

fn create_epic_and_task(project_root: &Path, epic_id: &str, task_id: &str, task_title: &str) {
    run_json_success(
        project_root,
        None,
        &[
            "task",
            "create",
            epic_id,
            &format!("{task_title} Epic"),
            "--type",
            "epic",
            "--priority",
            "99",
            "--json",
        ],
    );
    run_json_success(
        project_root,
        None,
        &[
            "task",
            "create",
            task_id,
            task_title,
            "--parent-id",
            epic_id,
            "--priority",
            "0",
            "--json",
        ],
    );
}

fn create_parallel_task(
    project_root: &Path,
    epic_id: &str,
    task_id: &str,
    title: &str,
    owned_path: &str,
    conflict_domain: &str,
) {
    run_json_success(
        project_root,
        None,
        &[
            "task",
            "create",
            task_id,
            title,
            "--parent-id",
            epic_id,
            "--priority",
            "1",
            "--execution-mode",
            "parallel_safe",
            "--order-bucket",
            "team-wave",
            "--parallel-group",
            "team-worktree",
            "--conflict-domain",
            conflict_domain,
            "--owned-path",
            owned_path,
            "--json",
        ],
    );
}

fn task_ids(value: &Value) -> Vec<String> {
    value["tasks"]
        .as_array()
        .expect("tasks should render as array")
        .iter()
        .filter_map(|task| task["id"].as_str())
        .map(ToString::to_string)
        .collect()
}

fn json_string_array(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} should render as array: {value:#}"))
        .iter()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn rejected_reasons(value: &Value, task_id: &str) -> Vec<String> {
    value["rejected_candidates"]
        .as_array()
        .unwrap_or_else(|| panic!("rejected_candidates should render as array: {value:#}"))
        .iter()
        .find(|candidate| candidate["task_id"] == task_id)
        .unwrap_or_else(|| panic!("{task_id} should be rejected: {value:#}"))["reasons"]
        .as_array()
        .expect("rejected candidate reasons should render")
        .iter()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn assert_task_visible_only(value: &Value, present: &str, absent: &str) {
    let ids = task_ids(value);
    assert!(
        ids.iter().any(|id| id == present),
        "{present} should be visible in {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| id == absent),
        "{absent} should not leak into {ids:?}"
    );
}

#[test]
fn state_dir_env_explicit_and_default_roots_do_not_leak_tasks() {
    let root_a = unique_project_root("state-dir-a");
    let root_b = unique_project_root("state-dir-b");
    create_project_root(&root_a);
    create_project_root(&root_b);

    create_epic_and_task(&root_a, "a-epic", "a-task", "A isolated task");
    create_epic_and_task(&root_b, "b-epic", "b-task", "B isolated task");

    let state_a = default_state_dir(&root_a);
    let state_b = default_state_dir(&root_b);
    assert!(state_a.exists(), "root a default state should exist");
    assert!(state_b.exists(), "root b default state should exist");

    let root_a_default = run_json_success(&root_a, None, &["task", "list", "--json"]);
    assert_task_visible_only(&root_a_default, "a-task", "b-task");
    let root_b_default = run_json_success(&root_b, None, &["task", "list", "--json"]);
    assert_task_visible_only(&root_b_default, "b-task", "a-task");

    let state_a_arg = state_a.to_str().expect("state a path should be utf8");
    let state_b_arg = state_b.to_str().expect("state b path should be utf8");
    let root_b_explicit_a = run_json_success(
        &root_b,
        None,
        &["task", "list", "--state-dir", state_a_arg, "--json"],
    );
    assert_task_visible_only(&root_b_explicit_a, "a-task", "b-task");
    let root_a_explicit_b = run_json_success(
        &root_a,
        None,
        &["task", "list", "--state-dir", state_b_arg, "--json"],
    );
    assert_task_visible_only(&root_a_explicit_b, "b-task", "a-task");

    let root_b_env_a = run_json_success(&root_b, Some(&state_a), &["task", "list", "--json"]);
    assert_task_visible_only(&root_b_env_a, "a-task", "b-task");
    let root_a_env_b = run_json_success(&root_a, Some(&state_b), &["task", "list", "--json"]);
    assert_task_visible_only(&root_a_env_b, "b-task", "a-task");

    let _ = std::fs::remove_dir_all(&root_a);
    let _ = std::fs::remove_dir_all(&root_b);
}

#[test]
fn help_and_version_do_not_require_or_create_state() {
    let root = unique_project_root("no-state-help");
    create_project_root(&root);
    let state_dir = default_state_dir(&root);

    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["task", "--help"],
        vec!["taskflow", "--help"],
        vec!["recovery", "--help"],
    ] {
        run_success(&root, None, &args);
        assert!(
            !state_dir.exists(),
            "{} should not create state at {}",
            args.join(" "),
            state_dir.display()
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn taskflow_proxy_and_root_task_surfaces_share_default_state_root() {
    let root_a = unique_project_root("proxy-root-a");
    let root_b = unique_project_root("proxy-root-b");
    create_project_root(&root_a);
    create_project_root(&root_b);

    create_epic_and_task(&root_a, "proxy-a-epic", "proxy-a-task", "Proxy A task");
    create_epic_and_task(&root_b, "proxy-b-epic", "proxy-b-task", "Proxy B task");

    let ready = run_json_success(&root_a, None, &["task", "ready", "--json"]);
    assert_task_visible_only(&ready, "proxy-a-task", "proxy-b-task");

    let graph = run_json_success(&root_a, None, &["taskflow", "graph-summary", "--json"]);
    assert_eq!(graph["surface"], "vida taskflow graph-summary");
    assert_eq!(
        graph["primary_ready_task"]["task"]["id"], "proxy-a-task",
        "taskflow proxy should read the same default state root as root task surfaces: {graph:#}"
    );
    assert_ne!(graph["primary_ready_task"]["task"]["id"], "proxy-b-task");

    let _ = std::fs::remove_dir_all(&root_a);
    let _ = std::fs::remove_dir_all(&root_b);
}

#[test]
fn team_worktree_parallel_scheduler_e2e_isolates_disjoint_work_and_blocks_overlap() {
    let root_a = unique_project_root("team-worktree-a");
    let root_b = unique_project_root("team-worktree-b");
    create_project_root(&root_a);
    create_project_root(&root_b);
    enable_parallel_scheduler(&root_a, 3);
    enable_parallel_scheduler(&root_b, 3);

    run_json_success(
        &root_a,
        None,
        &[
            "task",
            "create",
            "team-a",
            "Team A",
            "--type",
            "epic",
            "--priority",
            "99",
            "--json",
        ],
    );
    create_parallel_task(
        &root_a,
        "team-a",
        "a-primary",
        "A primary",
        "src/a.rs",
        "domain-a",
    );
    create_parallel_task(
        &root_a,
        "team-a",
        "a-disjoint",
        "A disjoint",
        "docs/a.md",
        "domain-b",
    );
    create_parallel_task(
        &root_a,
        "team-a",
        "a-overlap",
        "A overlap",
        "src/a.rs",
        "domain-c",
    );

    run_json_success(
        &root_b,
        None,
        &[
            "task",
            "create",
            "team-b",
            "Team B",
            "--type",
            "epic",
            "--priority",
            "99",
            "--json",
        ],
    );
    create_parallel_task(
        &root_b,
        "team-b",
        "b-primary",
        "B primary",
        "src/b.rs",
        "domain-d",
    );
    create_parallel_task(
        &root_b,
        "team-b",
        "b-disjoint",
        "B disjoint",
        "docs/b.md",
        "domain-e",
    );

    let graph_a = run_json_success(&root_a, None, &["taskflow", "graph-summary", "--json"]);
    assert_eq!(graph_a["surface"], "vida taskflow graph-summary");
    assert_eq!(graph_a["status"], "pass");
    let graph_a_text = graph_a.to_string();
    assert!(graph_a_text.contains("a-primary"));
    assert!(!graph_a_text.contains("b-primary"));

    let dispatch_a = run_json_success(
        &root_a,
        None,
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--scope",
            "team-a",
            "--current-task-id",
            "a-primary",
            "--limit",
            "3",
            "--dry-run",
            "--json",
        ],
    );
    assert_eq!(dispatch_a["surface"], "vida taskflow scheduler dispatch");
    assert_eq!(dispatch_a["status"], "pass");
    let selected_a = json_string_array(&dispatch_a, "selected_task_ids");
    assert_eq!(selected_a, vec!["a-primary", "a-disjoint"]);
    assert!(!selected_a.iter().any(|task_id| task_id == "a-overlap"));
    assert!(!selected_a.iter().any(|task_id| task_id == "b-primary"));
    let overlap_reasons = rejected_reasons(&dispatch_a, "a-overlap");
    assert!(
        overlap_reasons
            .iter()
            .any(|reason| reason.starts_with("owned_path_already_selected:")),
        "overlap should fail closed on owned path collision: {overlap_reasons:?}"
    );

    let dispatch_b = run_json_success(
        &root_b,
        None,
        &[
            "taskflow",
            "scheduler",
            "dispatch",
            "--scope",
            "team-b",
            "--current-task-id",
            "b-primary",
            "--limit",
            "3",
            "--dry-run",
            "--json",
        ],
    );
    let selected_b = json_string_array(&dispatch_b, "selected_task_ids");
    assert_eq!(selected_b, vec!["b-primary", "b-disjoint"]);
    assert!(!selected_b.iter().any(|task_id| task_id == "a-primary"));

    let _ = std::fs::remove_dir_all(&root_a);
    let _ = std::fs::remove_dir_all(&root_b);
}
