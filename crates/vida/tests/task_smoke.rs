use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("/tmp/vida-task-state-{}-{}", std::process::id(), nanos)
}

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
    let output = vida()
        .args(args)
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("vida command should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    thread::sleep(Duration::from_millis(100));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn normalize_json_fixture(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    serde_json::to_string_pretty(&parsed).expect("json output should pretty render")
}

fn donor_ready_semantic(value: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(value).expect("json output should parse");
    let rows = parsed.as_array().expect("ready output should be an array");
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

#[test]
fn task_command_round_trip_succeeds_via_binary_surface() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert!(
        import_stdout.contains("\"status\": \"ok\"") || import_stdout.contains("\"status\":\"ok\"")
    );

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
        blocked_stdout.contains("\"id\": \"vida-b\"")
            || blocked_stdout.contains("\"id\":\"vida-b\"")
    );
    assert!(
        blocked_stdout.contains("\"depends_on_id\": \"vida-a\"")
            || blocked_stdout.contains("\"depends_on_id\":\"vida-a\"")
    );

    let tree_stdout = run_and_assert_success(&["task", "tree", "vida-b", "--json"], &state_dir);
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
    assert_eq!(deps_after_remove_stdout.trim(), "[]");

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn validate_graph_broken_edge_matches_golden_fixture() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        &jsonl_path,
        concat!(
            "{\"id\":\"vida-broken\",\"title\":\"Broken task\",\"description\":\"broken\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-broken\",\"depends_on_id\":\"vida-missing\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("write broken task jsonl");

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert!(
        import_stdout.contains("\"status\": \"ok\"") || import_stdout.contains("\"status\":\"ok\"")
    );

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
    assert!(
        import_stdout.contains("\"status\": \"ok\"") || import_stdout.contains("\"status\":\"ok\"")
    );

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
fn task_show_fails_closed_for_missing_task_id() {
    let state_dir = unique_state_dir();
    let jsonl_path = format!("{state_dir}/issues.jsonl");
    fs::create_dir_all(&state_dir).expect("create state dir");
    sample_jsonl(&jsonl_path);

    let import_stdout =
        run_and_assert_success(&["task", "import-jsonl", &jsonl_path, "--json"], &state_dir);
    assert!(
        import_stdout.contains("\"status\": \"ok\"") || import_stdout.contains("\"status\":\"ok\"")
    );

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
    assert!(
        import_stdout.contains("\"status\": \"ok\"") || import_stdout.contains("\"status\":\"ok\"")
    );

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

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output should parse");
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
    assert_eq!(parsed["status"], parsed["shared_fields"]["status"]);
    assert_eq!(
        parsed["blocker_codes"],
        parsed["shared_fields"]["blocker_codes"]
    );
    assert_eq!(
        parsed["next_actions"],
        parsed["shared_fields"]["next_actions"]
    );
    assert_eq!(
        parsed["artifact_refs"],
        parsed["shared_fields"]["artifact_refs"]
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
    let load_error = parsed["host_agents"]["internal_dispatch_alias_load_error"]
        .as_str()
        .expect("internal_dispatch_alias_load_error should be present");
    assert!(load_error.contains("missing/dispatch-aliases.yaml"));

    let _ = fs::remove_dir_all(project_root);
}

#[test]
fn consume_bundle_check_exposes_shared_operator_contract_fields() {
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

    let stdout = run_and_assert_success(
        &["taskflow", "consume", "bundle", "check", "--json"],
        &state_dir,
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume bundle check json should parse");

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
    assert_eq!(parsed["operator_contracts"]["schema_version"], "release-1-v1");
    assert_eq!(parsed["operator_contracts"]["status"], "pass");
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
    assert!(parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array")
        .is_empty());
    assert!(parsed["next_actions"]
        .as_array()
        .expect("next_actions should be an array")
        .is_empty());

    let _ = fs::remove_dir_all(&state_dir);
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
fn consume_bundle_check_blocked_path_matches_blocker_codes_contract() {
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
        "cache_registry_contract_missing_triggered_domain_binding",
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

    let _ = fs::remove_dir_all(&state_dir);
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
        assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
            .as_array()
            .expect("downstream blockers should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pending_execution_preparation_evidence")));
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
    let lifecycle_stage = latest_status["lifecycle_stage"].as_str().unwrap_or_default();
    let combined = format!(
        "{} {} {}",
        handoff_state.to_ascii_lowercase(),
        policy_gate.to_ascii_lowercase(),
        lifecycle_stage.to_ascii_lowercase()
    );
    let approval_or_delegation_wait =
        combined.contains("approval") || combined.contains("delegat");
    let evidence_ready = parsed["payload"]["run_graph_bootstrap"]["approval_delegation_evidence_ready"]
        .as_bool()
        == Some(true)
        || latest_status["approval_delegation_evidence_ready"].as_bool() == Some(true)
        || parsed["payload"]["run_graph_bootstrap"]["evidence"]["approval_delegation"]["status"]
            .as_str()
            == Some("ready")
        || parsed["payload"]["run_graph_bootstrap"]["evidence"]["approval_delegation"]["ready"]
            .as_bool()
            == Some(true);

    if approval_or_delegation_wait && !evidence_ready {
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
        assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_blockers"]
            .as_array()
            .expect("downstream blockers should be an array")
            .iter()
            .any(|value| value.as_str() == Some("pending_approval_delegation_evidence")));
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
    assert_eq!(parsed["ok"], false);
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
fn consume_continue_fails_closed_on_lane_governance_status_evidence_conflict() {
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
    assert!(
        final_output.status.success(),
        "{}",
        String::from_utf8_lossy(&final_output.stderr)
    );
    let _final_parsed: serde_json::Value =
        serde_json::from_slice(&final_output.stdout).expect("consume final json should parse");

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
            packet["downstream_lane_status"] =
                serde_json::Value::String("lane_open".to_string());
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

    let _ = fs::remove_dir_all(&state_dir);
}
