use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::path::Path;

use surrealdb::Surreal;
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::types::SurrealValue;

#[path = "support/runtime_consumption.rs"]
mod runtime_consumption_support;

use runtime_consumption_support::PersistentRuntimeFixture;

#[derive(Debug, Deserialize, PartialEq, Serialize, SurrealValue)]
struct PersistentStorageProbe {
    value: String,
}

async fn open_persistent_storage_probe(root: &Path) -> Surreal<Db> {
    let db = Surreal::new::<SurrealKv>(root.to_path_buf())
        .await
        .expect("SurrealKV probe database should open");
    db.use_ns("vida")
        .use_db("primary")
        .await
        .expect("SurrealKV probe namespace should bind");
    db
}

fn enable_parallel_scheduler(fixture: &PersistentRuntimeFixture, max_parallel_agents: u32) {
    fixture.write_project_config(format!(
        "project_id: test\nagent_system:\n  max_parallel_agents: {max_parallel_agents}\n"
    ));
}

fn run_success(fixture: &PersistentRuntimeFixture, state_dir: Option<&Path>, args: &[&str]) {
    fixture.output_success_with_state_dir(args, state_dir);
}

fn run_json_success(
    fixture: &PersistentRuntimeFixture,
    state_dir: Option<&Path>,
    args: &[&str],
) -> Value {
    fixture.json_success_with_state_dir(args, state_dir)
}

fn create_epic_and_task(
    fixture: &PersistentRuntimeFixture,
    epic_id: &str,
    task_id: &str,
    task_title: &str,
) {
    run_json_success(
        fixture,
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
        fixture,
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
    fixture: &PersistentRuntimeFixture,
    epic_id: &str,
    task_id: &str,
    title: &str,
    owned_path: &str,
    conflict_domain: &str,
) {
    run_json_success(
        fixture,
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

fn has_parent_dependency(task: &Value, parent_id: &str) -> bool {
    task["dependencies"]
        .as_array()
        .expect("task dependencies should render as array")
        .iter()
        .any(|dependency| {
            dependency["edge_type"] == "parent-child" && dependency["depends_on_id"] == parent_id
        })
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
    let fixture_a = PersistentRuntimeFixture::project_shell("state-dir-a");
    let fixture_b = PersistentRuntimeFixture::project_shell("state-dir-b");

    create_epic_and_task(&fixture_a, "a-epic", "a-task", "A isolated task");
    create_epic_and_task(&fixture_b, "b-epic", "b-task", "B isolated task");

    let state_a = fixture_a.state_dir();
    let state_b = fixture_b.state_dir();
    assert!(state_a.exists(), "root a default state should exist");
    assert!(state_b.exists(), "root b default state should exist");

    let root_a_default = run_json_success(&fixture_a, None, &["task", "list", "--json"]);
    assert_task_visible_only(&root_a_default, "a-task", "b-task");
    let root_b_default = run_json_success(&fixture_b, None, &["task", "list", "--json"]);
    assert_task_visible_only(&root_b_default, "b-task", "a-task");

    let state_a_arg = state_a.to_str().expect("state a path should be utf8");
    let state_b_arg = state_b.to_str().expect("state b path should be utf8");
    let root_b_explicit_a = run_json_success(
        &fixture_b,
        None,
        &["task", "list", "--state-dir", state_a_arg, "--json"],
    );
    assert_task_visible_only(&root_b_explicit_a, "a-task", "b-task");
    let root_a_explicit_b = run_json_success(
        &fixture_a,
        None,
        &["task", "list", "--state-dir", state_b_arg, "--json"],
    );
    assert_task_visible_only(&root_a_explicit_b, "b-task", "a-task");

    let root_b_env_a = run_json_success(&fixture_b, Some(state_a), &["task", "list", "--json"]);
    assert_task_visible_only(&root_b_env_a, "a-task", "b-task");
    let root_a_env_b = run_json_success(&fixture_a, Some(state_b), &["task", "list", "--json"]);
    assert_task_visible_only(&root_a_env_b, "b-task", "a-task");
}

#[test]
fn help_and_version_do_not_require_or_create_state() {
    let fixture = PersistentRuntimeFixture::project_shell("no-state-help");
    let state_dir = fixture.state_dir();

    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["task", "--help"],
        vec!["taskflow", "--help"],
        vec!["recovery", "--help"],
    ] {
        run_success(&fixture, None, &args);
        assert!(
            !state_dir.exists(),
            "{} should not create state at {}",
            args.join(" "),
            state_dir.display()
        );
    }
}

#[test]
fn taskflow_proxy_and_root_task_surfaces_share_default_state_root() {
    let fixture_a = PersistentRuntimeFixture::project_shell("proxy-root-a");
    let fixture_b = PersistentRuntimeFixture::project_shell("proxy-root-b");

    create_epic_and_task(&fixture_a, "proxy-a-epic", "proxy-a-task", "Proxy A task");
    create_epic_and_task(&fixture_b, "proxy-b-epic", "proxy-b-task", "Proxy B task");

    let ready = run_json_success(&fixture_a, None, &["task", "ready", "--json"]);
    assert_task_visible_only(&ready, "proxy-a-task", "proxy-b-task");

    let graph = run_json_success(&fixture_a, None, &["taskflow", "graph-summary", "--json"]);
    assert_eq!(graph["surface"], "vida taskflow graph-summary");
    assert_eq!(
        graph["primary_ready_task"]["task"]["id"], "proxy-a-task",
        "taskflow proxy should read the same default state root as root task surfaces: {graph:#}"
    );
    assert_ne!(graph["primary_ready_task"]["task"]["id"], "proxy-b-task");
}

#[test]
fn state_store_availability_preserves_project_bound_task_state() {
    let fixture = PersistentRuntimeFixture::project_bound("state-store-available");

    create_epic_and_task(
        &fixture,
        "available-epic",
        "available-task",
        "Available persisted task",
    );

    let explicit_show = run_json_success(
        &fixture,
        Some(fixture.state_dir()),
        &["task", "show", "available-task", "--json"],
    );
    assert_eq!(explicit_show["task"]["id"], "available-task");
    assert!(has_parent_dependency(
        &explicit_show["task"],
        "available-epic"
    ));

    let default_show = run_json_success(
        &fixture,
        None,
        &["task", "show", "available-task", "--json"],
    );
    assert_eq!(default_show["task"]["id"], "available-task");
}

#[test]
fn team_worktree_parallel_scheduler_e2e_isolates_disjoint_work_and_blocks_overlap() {
    let fixture_a = PersistentRuntimeFixture::project_shell("team-worktree-a");
    let fixture_b = PersistentRuntimeFixture::project_shell("team-worktree-b");
    enable_parallel_scheduler(&fixture_a, 3);
    enable_parallel_scheduler(&fixture_b, 3);

    run_json_success(
        &fixture_a,
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
        &fixture_a,
        "team-a",
        "a-primary",
        "A primary",
        "src/a.rs",
        "domain-a",
    );
    create_parallel_task(
        &fixture_a,
        "team-a",
        "a-disjoint",
        "A disjoint",
        "docs/a.md",
        "domain-b",
    );
    create_parallel_task(
        &fixture_a,
        "team-a",
        "a-overlap",
        "A overlap",
        "src/a.rs",
        "domain-c",
    );

    run_json_success(
        &fixture_b,
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
        &fixture_b,
        "team-b",
        "b-primary",
        "B primary",
        "src/b.rs",
        "domain-d",
    );
    create_parallel_task(
        &fixture_b,
        "team-b",
        "b-disjoint",
        "B disjoint",
        "docs/b.md",
        "domain-e",
    );

    let graph_a = run_json_success(&fixture_a, None, &["taskflow", "graph-summary", "--json"]);
    assert_eq!(graph_a["surface"], "vida taskflow graph-summary");
    assert_eq!(graph_a["status"], "pass");
    let graph_a_text = graph_a.to_string();
    assert!(graph_a_text.contains("a-primary"));
    assert!(!graph_a_text.contains("b-primary"));

    let dispatch_a = run_json_success(
        &fixture_a,
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
        &fixture_b,
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
}

#[test]
fn surrealkv_backup_restore_persists_rows_after_reopen() {
    let fixture = PersistentRuntimeFixture::state_only("surrealkv-backup-restore");
    let state_dir = fixture.state_dir().to_path_buf();
    let backup_path = state_dir
        .parent()
        .expect("state directory should have a parent")
        .join("surrealkv-backup.surql");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

    runtime.block_on(async {
        let db = open_persistent_storage_probe(&state_dir).await;
        let _: Option<PersistentStorageProbe> = db
            .upsert(("state_backup_probe", "baseline"))
            .content(PersistentStorageProbe {
                value: "baseline".to_string(),
            })
            .await
            .expect("baseline row should persist");

        db.export(backup_path.clone())
            .await
            .expect("SurrealKV backup should export");
        assert!(
            std::fs::metadata(&backup_path)
                .expect("backup file should exist")
                .len()
                > 0,
            "SurrealKV backup should contain serialized state"
        );

        db.query("REMOVE TABLE state_backup_probe")
            .await
            .expect("backup restore fixture should remove the live table");
        db.import(backup_path.clone())
            .await
            .expect("SurrealKV backup should restore");
        drop(db);

        let reopened = open_persistent_storage_probe(&state_dir).await;
        let restored: Option<PersistentStorageProbe> = reopened
            .select(("state_backup_probe", "baseline"))
            .await
            .expect("restored row should be readable after reopen");
        assert_eq!(
            restored,
            Some(PersistentStorageProbe {
                value: "baseline".to_string(),
            })
        );
        drop(reopened);
    });
    runtime.shutdown_timeout(std::time::Duration::from_millis(250));
    let _ = std::fs::remove_file(backup_path);
}

#[test]
fn surrealkv_wal_commit_and_rollback_recover_after_reopen() {
    let fixture = PersistentRuntimeFixture::state_only("surrealkv-wal-recovery-rollback");
    let state_dir = fixture.state_dir().to_path_buf();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

    runtime.block_on(async {
        let db = open_persistent_storage_probe(&state_dir).await;
        let _: Option<PersistentStorageProbe> = db
            .upsert(("state_wal_probe", "durable"))
            .content(PersistentStorageProbe {
                value: "before-rollback".to_string(),
            })
            .await
            .expect("durable WAL row should persist");

        let transaction = db.begin().await.expect("commit transaction should begin");
        transaction
            .upsert(("state_wal_probe", "committed"))
            .content(PersistentStorageProbe {
                value: "committed".to_string(),
            })
            .await
            .expect("transactional row should write before commit");
        let db = transaction
            .commit()
            .await
            .expect("transactional row should commit");

        let transaction = db.begin().await.expect("rollback transaction should begin");
        transaction
            .upsert(("state_wal_probe", "rolled-back"))
            .content(PersistentStorageProbe {
                value: "must-not-survive".to_string(),
            })
            .await
            .expect("rollback row should write before cancel");
        let db = transaction
            .cancel()
            .await
            .expect("rollback transaction should cancel");
        drop(db);

        let reopened = open_persistent_storage_probe(&state_dir).await;
        let durable: Option<PersistentStorageProbe> = reopened
            .select(("state_wal_probe", "durable"))
            .await
            .expect("durable row should recover after reopen");
        let committed: Option<PersistentStorageProbe> = reopened
            .select(("state_wal_probe", "committed"))
            .await
            .expect("committed row should recover after reopen");
        let rolled_back: Option<PersistentStorageProbe> = reopened
            .select(("state_wal_probe", "rolled-back"))
            .await
            .expect("rolled-back row lookup should succeed after reopen");
        assert_eq!(
            durable,
            Some(PersistentStorageProbe {
                value: "before-rollback".to_string(),
            })
        );
        assert_eq!(
            committed,
            Some(PersistentStorageProbe {
                value: "committed".to_string(),
            })
        );
        assert_eq!(rolled_back, None);

        let transaction = reopened
            .begin()
            .await
            .expect("rollback update transaction should begin");
        transaction
            .upsert(("state_wal_probe", "durable"))
            .content(PersistentStorageProbe {
                value: "must-not-replace".to_string(),
            })
            .await
            .expect("rollback update should write before cancel");
        let reopened = transaction
            .cancel()
            .await
            .expect("rollback update transaction should cancel");
        drop(reopened);

        let reopened = open_persistent_storage_probe(&state_dir).await;
        let durable_after_rollback: Option<PersistentStorageProbe> = reopened
            .select(("state_wal_probe", "durable"))
            .await
            .expect("durable row should remain readable after rollback reopen");
        assert_eq!(
            durable_after_rollback,
            Some(PersistentStorageProbe {
                value: "before-rollback".to_string(),
            })
        );
        drop(reopened);
    });
    runtime.shutdown_timeout(std::time::Duration::from_millis(250));
}
