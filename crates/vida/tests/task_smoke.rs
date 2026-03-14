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

fn repo_root() -> String {
    env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida")
        .to_string()
}

fn donor_taskflow_runtime_name() -> String {
    ["taskflow", "v0"].join("-")
}

fn donor_taskflow_launcher() -> String {
    format!("{}/{}/src/vida", repo_root(), donor_taskflow_runtime_name())
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
fn donor_ready_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(format!("{temp_root}/.beads")).expect("beads dir should be created");
    sample_jsonl(&jsonl_path);

    let donor_import = Command::new(donor_taskflow_launcher())
        .args(["task", "import-jsonl", &jsonl_path, "--json"])
        .env("VIDA_ROOT", &temp_root)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            "/home/unnamed/project/vida-stack/.venv/bin/python3",
        )
        .output()
        .expect("donor import should run");
    assert!(donor_import.status.success());

    let donor_ready = Command::new(donor_taskflow_launcher())
        .args(["task", "ready", "--json"])
        .env("VIDA_ROOT", &temp_root)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            "/home/unnamed/project/vida-stack/.venv/bin/python3",
        )
        .output()
        .expect("donor ready should run");
    assert!(donor_ready.status.success());
    let donor_stdout = String::from_utf8_lossy(&donor_ready.stdout);

    let rust_state_dir = format!("{temp_root}/rust-state");
    let _import_stdout = run_and_assert_success(
        &["task", "import-jsonl", &jsonl_path, "--json"],
        &rust_state_dir,
    );
    let rust_ready = run_and_assert_success(&["task", "ready", "--json"], &rust_state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_ready_semantic.json").trim_end();
    assert_eq!(
        donor_ready_semantic(&donor_stdout),
        normalize_json_fixture(expected)
    );
    let rust_semantic = donor_ready_semantic(&rust_ready);
    assert!(rust_semantic.contains("\"id\": \"vida-a\""));
    assert!(rust_semantic.contains("\"id\": \"vida-c\""));

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn donor_show_output_matches_semantic_parity_fixture() {
    let temp_root = unique_state_dir();
    let jsonl_path = format!("{temp_root}/issues.jsonl");
    fs::create_dir_all(format!("{temp_root}/.beads")).expect("beads dir should be created");
    sample_jsonl(&jsonl_path);

    let donor_import = Command::new(donor_taskflow_launcher())
        .args(["task", "import-jsonl", &jsonl_path, "--json"])
        .env("VIDA_ROOT", &temp_root)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            "/home/unnamed/project/vida-stack/.venv/bin/python3",
        )
        .output()
        .expect("donor import should run");
    assert!(donor_import.status.success());

    let donor_show = Command::new(donor_taskflow_launcher())
        .args(["task", "show", "vida-b", "--json"])
        .env("VIDA_ROOT", &temp_root)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            "/home/unnamed/project/vida-stack/.venv/bin/python3",
        )
        .output()
        .expect("donor show should run");
    assert!(donor_show.status.success());
    let donor_stdout = String::from_utf8_lossy(&donor_show.stdout);

    let rust_state_dir = format!("{temp_root}/rust-state");
    let _import_stdout = run_and_assert_success(
        &["task", "import-jsonl", &jsonl_path, "--json"],
        &rust_state_dir,
    );
    let rust_show = run_and_assert_success(&["task", "show", "vida-b", "--json"], &rust_state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_show_semantic.json").trim_end();
    assert_eq!(
        donor_show_semantic(&donor_stdout),
        normalize_json_fixture(expected)
    );
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
    fs::create_dir_all(format!("{temp_root}/.beads")).expect("beads dir should be created");
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

    let donor_import = Command::new(donor_taskflow_launcher())
        .args(["task", "import-jsonl", &jsonl_path, "--json"])
        .env("VIDA_ROOT", &temp_root)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            "/home/unnamed/project/vida-stack/.venv/bin/python3",
        )
        .output()
        .expect("donor import should run");
    assert!(donor_import.status.success());

    let donor_list = Command::new(donor_taskflow_launcher())
        .args(["task", "list", "--json"])
        .env("VIDA_ROOT", &temp_root)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            "/home/unnamed/project/vida-stack/.venv/bin/python3",
        )
        .output()
        .expect("donor list should run");
    assert!(donor_list.status.success());
    let donor_stdout = String::from_utf8_lossy(&donor_list.stdout);

    let rust_state_dir = format!("{temp_root}/rust-state");
    let _import_stdout = run_and_assert_success(
        &["task", "import-jsonl", &jsonl_path, "--json"],
        &rust_state_dir,
    );
    let rust_list = run_and_assert_success(&["task", "list", "--json"], &rust_state_dir);

    let expected =
        include_str!("../../../tests/golden/taskflow/donor_list_semantic.json").trim_end();
    assert_eq!(
        donor_list_semantic(&donor_stdout),
        normalize_json_fixture(expected)
    );
    assert_eq!(
        donor_list_semantic(&rust_list),
        normalize_json_fixture(expected)
    );

    let _ = fs::remove_dir_all(&temp_root);
}
