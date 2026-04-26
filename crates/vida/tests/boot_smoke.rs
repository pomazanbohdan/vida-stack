use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;
use vida_test_support as support;

const SNAPSHOT_OVERWRITE_HELPER_STATE_DIR_ENV: &str =
    "VIDA_BOOT_SMOKE_SNAPSHOT_OVERWRITE_STATE_DIR";
const SNAPSHOT_OVERWRITE_HELPER_SOURCE_ENV: &str = "VIDA_BOOT_SMOKE_SNAPSHOT_OVERWRITE_SOURCE";
const SNAPSHOT_OVERWRITE_HELPER_CONFIG_PATH_ENV: &str =
    "VIDA_BOOT_SMOKE_SNAPSHOT_OVERWRITE_CONFIG_PATH";
const SNAPSHOT_OVERWRITE_HELPER_COMPILED_BUNDLE_ENV: &str =
    "VIDA_BOOT_SMOKE_SNAPSHOT_OVERWRITE_COMPILED_BUNDLE";

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestLauncherActivationSnapshot {
    source: String,
    source_config_path: String,
    source_config_digest: String,
    captured_at: String,
    compiled_bundle: serde_json::Value,
    pack_router_keywords: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestExecutionPlanStateRow {
    run_id: String,
    task_id: String,
    task_class: String,
    active_node: String,
    next_node: Option<String>,
    status: String,
    updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestRoutedRunStateRow {
    run_id: String,
    route_task_class: String,
    selected_backend: String,
    lane_id: String,
    lifecycle_stage: String,
    updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestGovernanceStateRow {
    run_id: String,
    policy_gate: String,
    handoff_state: String,
    context_state: String,
    updated_at: String,
}

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestResumabilityCapsuleRow {
    run_id: String,
    checkpoint_kind: String,
    resume_target: String,
    recovery_ready: bool,
    updated_at: String,
}

struct FileRestoreGuard {
    path: String,
    original_body: String,
}

impl FileRestoreGuard {
    fn new(path: String) -> Self {
        let original_body = fs::read_to_string(&path).expect("guarded file should read");
        Self {
            path,
            original_body,
        }
    }
}

impl Drop for FileRestoreGuard {
    fn drop(&mut self) {
        atomic_write_file(&self.path, &self.original_body);
    }
}

fn atomic_write_file(path: &str, body: &str) {
    let target = std::path::Path::new(path);
    let parent = target
        .parent()
        .expect("atomic write target should have parent");
    let tmp_path = parent.join(format!(
        ".{}.tmp-{}",
        target
            .file_name()
            .expect("atomic write target should have file name")
            .to_string_lossy(),
        std::process::id()
    ));
    fs::write(&tmp_path, body).expect("temp file should be written");
    fs::rename(&tmp_path, target).expect("temp file should atomically replace target");
}

fn vida() -> Command {
    support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn installed_vida() -> (String, Command) {
    let root = unique_state_dir();
    let install_root = format!("{root}/vida-install");
    let bin_dir = format!("{install_root}/bin");
    fs::create_dir_all(&bin_dir).expect("installed bin dir should exist");

    copy_executable(env!("CARGO_BIN_EXE_vida"), &format!("{bin_dir}/vida"));
    write_executable_script(
        &format!("{bin_dir}/{}", donor_taskflow_runtime_name()),
        "#!/bin/sh\nprintf 'taskflow placeholder\\n'\n",
    );

    let mut command = Command::new(format!("{bin_dir}/vida"));
    command.current_dir(&root);
    (root, command)
}

static UNIQUE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
static BOOT_PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "/tmp/vida-test-state-{}-{}-{}",
        std::process::id(),
        nanos,
        counter
    )
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

fn donor_docflow_runtime_name() -> String {
    ["codex", "v0"].join("-")
}

fn donor_docflow_script_name() -> String {
    ["codex", "py"].join(".")
}

fn write_executable_script(path: &str, body: &str) {
    fs::write(path, body).expect("script should be written");
    let mut perms = fs::metadata(path).expect("script metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script perms should be updated");
}

fn copy_executable(from: &str, to: &str) {
    fs::copy(from, to).expect("binary should be copied");
    let mut perms = fs::metadata(to)
        .expect("copied binary metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(to, perms).expect("copied binary perms should be updated");
}

fn task_rows_from_payload<'a>(
    value: &'a serde_json::Value,
    label: &str,
) -> &'a Vec<serde_json::Value> {
    value
        .as_array()
        .or_else(|| value.get("tasks").and_then(serde_json::Value::as_array))
        .unwrap_or_else(|| panic!("{label} payload should expose task rows"))
}

fn write_file(path: &str, body: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent).expect("parent dir should exist");
    }
    fs::write(path, body).expect("file should be written");
}

fn write_runtime_lane_completion_result_fixture(path: &str, run_id: &str, completed_target: &str) {
    write_file(
        path,
        &serde_json::json!({
            "artifact_kind": "runtime_lane_completion_result",
            "status": "pass",
            "execution_state": "executed",
            "run_id": run_id,
            "completed_target": completed_target,
            "completion_receipt_id": format!("{run_id}-{completed_target}-receipt"),
            "source_dispatch_packet_path": "test-fixture",
            "recorded_at": "2026-04-10T00:00:00Z"
        })
        .to_string(),
    );
}

fn seed_runtime_consumption_final_snapshot(state_dir: &str) -> String {
    let snapshot_path = format!("{state_dir}/runtime-consumption/final-test.json");
    write_file(
        &snapshot_path,
        &serde_json::json!({
            "payload": {
                "role_selection": {
                    "execution_plan": {
                        "root_session_write_guard": {
                            "status": "blocked_by_default",
                            "root_session_role": "orchestrator",
                            "local_write_requires_exception_path": true,
                            "lawful_write_surface": "vida agent-init",
                            "explicit_user_ordered_agent_mode_is_sticky": true,
                            "saturation_recovery_required_before_local_fallback": true,
                            "local_fallback_without_lane_recovery_forbidden": true,
                            "host_local_write_capability_is_not_authority": true,
                            "required_exception_evidence": "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard.",
                            "pre_write_checkpoint_required": true
                        },
                        "orchestration_contract": {
                            "root_session_write_guard": {
                                "status": "blocked_by_default",
                                "root_session_role": "orchestrator",
                                "local_write_requires_exception_path": true,
                                "lawful_write_surface": "vida agent-init",
                                "explicit_user_ordered_agent_mode_is_sticky": true,
                                "saturation_recovery_required_before_local_fallback": true,
                                "local_fallback_without_lane_recovery_forbidden": true,
                                "host_local_write_capability_is_not_authority": true,
                                "required_exception_evidence": "Run `vida taskflow recovery latest --json` and `vida taskflow consume continue --json` to confirm runtime artifacts expose the canonical root-session pre-write guard.",
                                "pre_write_checkpoint_required": true
                            }
                        }
                    }
                }
            }
        })
        .to_string(),
    );
    snapshot_path
}

fn scaffold_runtime_project_root(project_root: &str, agents_body: &str) {
    write_file(&format!("{project_root}/AGENTS.md"), agents_body);
    write_file(
        &format!("{project_root}/vida.config.yaml"),
        "project:\n  id: test\n",
    );
    for relative in [".vida/config", ".vida/db", ".vida/project"] {
        fs::create_dir_all(format!("{project_root}/{relative}"))
            .expect("runtime project marker dir should exist");
    }
}

fn bootstrap_project_runtime(project_id: &str, project_name: &str) -> (String, String) {
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
            project_id,
            "--project-name",
            project_name,
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    (project_root, state_dir)
}

fn project_bound_taskflow_consume_final_with_timeout(
    project_root: &str,
    state_dir: &str,
    request: &str,
) -> std::process::Output {
    run_command_with_state_lock_retry(|| {
        let mut command = Command::new("timeout");
        command.args(["-k", "5s", "20s"]);
        command
            .arg(env!("CARGO_BIN_EXE_vida"))
            .args(["taskflow", "consume", "final", request, "--json"])
            .current_dir(project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", state_dir);
        command
    })
}

fn overwrite_launcher_activation_snapshot(state_dir: &str, compiled_bundle: serde_json::Value) {
    overwrite_launcher_activation_snapshot_with_source(state_dir, "state_store", compiled_bundle);
}

fn overwrite_launcher_activation_snapshot_with_source(
    state_dir: &str,
    source: &str,
    compiled_bundle: serde_json::Value,
) {
    let config_path = format!("{}/vida.config.yaml", repo_root());
    overwrite_launcher_activation_snapshot_with_metadata(
        state_dir,
        source,
        &config_path,
        compiled_bundle,
    );
}

fn overwrite_launcher_activation_snapshot_with_metadata(
    state_dir: &str,
    source: &str,
    config_path: &str,
    compiled_bundle: serde_json::Value,
) {
    run_launcher_activation_snapshot_overwrite_helper(
        state_dir,
        source,
        config_path,
        &compiled_bundle,
    );
}

fn run_launcher_activation_snapshot_overwrite_helper(
    state_dir: &str,
    source: &str,
    config_path: &str,
    compiled_bundle: &serde_json::Value,
) {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .arg("boot_smoke_launcher_activation_snapshot_overwrite_helper")
        .arg("--exact")
        .env(SNAPSHOT_OVERWRITE_HELPER_STATE_DIR_ENV, state_dir)
        .env(SNAPSHOT_OVERWRITE_HELPER_SOURCE_ENV, source)
        .env(SNAPSHOT_OVERWRITE_HELPER_CONFIG_PATH_ENV, config_path)
        .env(
            SNAPSHOT_OVERWRITE_HELPER_COMPILED_BUNDLE_ENV,
            compiled_bundle.to_string(),
        )
        .output()
        .expect("snapshot overwrite helper should run");
    assert!(
        output.status.success(),
        "snapshot helper stdout: {}\nsnapshot helper stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    wait_for_state_unlock(state_dir);
}

fn overwrite_launcher_activation_snapshot_in_process(
    state_dir: &str,
    source: &str,
    config_path: &str,
    compiled_bundle: serde_json::Value,
) {
    let config_body = fs::read(&config_path).expect("config should be readable for digest");
    let config_digest = blake3::hash(&config_body).to_hex().to_string();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db = open_test_state_db_with_retry(state_dir).await;
        let _: Option<TestLauncherActivationSnapshot> = db
            .upsert(("launcher_activation_snapshot", "launcher_live"))
            .content(TestLauncherActivationSnapshot {
                source: source.to_string(),
                source_config_path: config_path.to_string(),
                source_config_digest: config_digest,
                captured_at: "2026-03-13T00:00:00Z".to_string(),
                compiled_bundle,
                pack_router_keywords: serde_json::json!({}),
            })
            .await
            .expect("launcher activation snapshot should update");
        drop(db);
    });
    runtime.shutdown_timeout(Duration::from_millis(250));
}

async fn open_test_state_db_with_retry(state_dir: &str) -> Surreal<Db> {
    let mut last_lock_error = None;
    for attempt in 0..MAX_BOOT_RETRY_ATTEMPTS {
        match Surreal::new::<SurrealKv>(PathBuf::from(state_dir)).await {
            Ok(db) => match db.use_ns("vida").use_db("primary").await {
                Ok(_) => return db,
                Err(error) if is_state_lock_error_text(&error.to_string()) => {
                    last_lock_error = Some(error.to_string());
                    drop(db);
                    tokio::time::sleep(retry_backoff_delay(attempt)).await;
                }
                Err(error) => panic!("state namespace should open: {error}"),
            },
            Err(error) if is_state_lock_error_text(&error.to_string()) => {
                last_lock_error = Some(error.to_string());
                tokio::time::sleep(retry_backoff_delay(attempt)).await;
            }
            Err(error) => panic!("state db should open: {error}"),
        }
    }

    panic!(
        "state db should open after lock retries: {}",
        last_lock_error.unwrap_or_else(|| "unknown lock error".to_string())
    );
}

fn upsert_run_graph_status_rows(
    state_dir: &str,
    run_id: &str,
    task_id: &str,
    task_class: &str,
    active_node: &str,
    next_node: Option<&str>,
    status: &str,
    route_task_class: &str,
    selected_backend: &str,
    lane_id: &str,
    lifecycle_stage: &str,
    policy_gate: &str,
    handoff_state: &str,
    context_state: &str,
    checkpoint_kind: &str,
    resume_target: &str,
    recovery_ready: bool,
) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(PathBuf::from(state_dir))
            .await
            .expect("state db should open");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("state namespace should open");
        let updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos().to_string())
            .unwrap_or_else(|_| "0".to_string());
        let _: Option<TestExecutionPlanStateRow> = db
            .upsert(("execution_plan_state", run_id))
            .content(TestExecutionPlanStateRow {
                run_id: run_id.to_string(),
                task_id: task_id.to_string(),
                task_class: task_class.to_string(),
                active_node: active_node.to_string(),
                next_node: next_node.map(str::to_string),
                status: status.to_string(),
                updated_at: updated_at.clone(),
            })
            .await
            .expect("execution plan should be seeded");
        let _: Option<TestRoutedRunStateRow> = db
            .upsert(("routed_run_state", run_id))
            .content(TestRoutedRunStateRow {
                run_id: run_id.to_string(),
                route_task_class: route_task_class.to_string(),
                selected_backend: selected_backend.to_string(),
                lane_id: lane_id.to_string(),
                lifecycle_stage: lifecycle_stage.to_string(),
                updated_at: updated_at.clone(),
            })
            .await
            .expect("routed run should be seeded");
        let _: Option<TestGovernanceStateRow> = db
            .upsert(("governance_state", run_id))
            .content(TestGovernanceStateRow {
                run_id: run_id.to_string(),
                policy_gate: policy_gate.to_string(),
                handoff_state: handoff_state.to_string(),
                context_state: context_state.to_string(),
                updated_at: updated_at.clone(),
            })
            .await
            .expect("governance state should be seeded");
        let _: Option<TestResumabilityCapsuleRow> = db
            .upsert(("resumability_capsule", run_id))
            .content(TestResumabilityCapsuleRow {
                run_id: run_id.to_string(),
                checkpoint_kind: checkpoint_kind.to_string(),
                resume_target: resume_target.to_string(),
                recovery_ready,
                updated_at,
            })
            .await
            .expect("resumability capsule should be seeded");
        drop(db);
    });
}

fn seed_run_graph_status(
    state_dir: &str,
    run_id: &str,
    policy_gate: &str,
    handoff_state: &str,
    context_state: &str,
) {
    upsert_run_graph_status_rows(
        state_dir,
        run_id,
        "vida-a",
        "writer",
        "writer",
        None,
        "ready",
        "analysis",
        "middle",
        "writer_lane",
        "active",
        policy_gate,
        handoff_state,
        context_state,
        "execution_cursor",
        "none",
        true,
    );
}

fn sync_protocol_binding(state_dir: &str) {
    let output = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["taskflow", "protocol-binding", "sync", "--json"])
            .env("VIDA_STATE_DIR", state_dir);
        command
    });
    assert!(
        output.status.success(),
        "protocol-binding sync should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn bounded_vida_output<F>(
    timeout_args: &[&str],
    expectation: &'static str,
    mut build: F,
) -> std::process::Output
where
    F: FnMut(&mut Command),
{
    let mut last = None;
    for _ in 0..3 {
        let mut command = Command::new("timeout");
        command.args(timeout_args);
        command.arg(env!("CARGO_BIN_EXE_vida"));
        build(&mut command);
        let output = command.output().expect(expectation);
        if is_retryable_temporary_failure(&output) {
            last = Some(output);
            continue;
        }
        last = Some(output);
        break;
    }
    last.expect(expectation)
}

fn bounded_vida_output_with_state_lock_retry<F>(
    timeout_args: &[&str],
    expectation: &'static str,
    mut build: F,
) -> std::process::Output
where
    F: FnMut(&mut Command),
{
    retry_with_backoff(
        &mut || {
            let mut command = Command::new("timeout");
            command.args(timeout_args);
            command.arg(env!("CARGO_BIN_EXE_vida"));
            build(&mut command);
            command.output().expect(expectation)
        },
        MAX_BOOT_RETRY_ATTEMPTS,
        |output, _| is_state_lock_error(output),
    )
}

fn run_protocol_binding_check_with_timeout(state_dir: &std::path::Path) -> std::process::Output {
    bounded_vida_output(
        &["-k", "5s", "20s"],
        "protocol-binding check should run",
        |command| {
            command
                .args(["taskflow", "protocol-binding", "check", "--json"])
                .env(
                    "VIDA_STATE_DIR",
                    state_dir
                        .to_str()
                        .expect("state dir should be utf-8 compatible"),
                );
        },
    )
}

fn create_minimal_release_archive(archive_path: &str) {
    let stage_root = format!("{}/release-stage", unique_state_dir());
    let package_root = format!("{stage_root}/vida-stack-v-test");
    let bin_dir = format!("{package_root}/bin");
    let codex_agents_dir = format!("{package_root}/.codex/agents");
    let template_dir = format!("{package_root}/install/assets");

    fs::create_dir_all(&bin_dir).expect("bin dir should exist");
    fs::create_dir_all(&codex_agents_dir).expect("codex agents dir should exist");
    fs::create_dir_all(&template_dir).expect("template dir should exist");

    write_executable_script(
        &format!("{bin_dir}/vida"),
        "#!/bin/sh\nprintf 'vida placeholder\\n'\n",
    );
    write_file(&format!("{package_root}/AGENTS.md"), "framework\n");
    write_file(&format!("{package_root}/AGENTS.sidecar.md"), "sidecar\n");
    write_file(
        &format!("{package_root}/vida.config.yaml"),
        "project:\n  id: packaged\n",
    );
    write_file(&format!("{package_root}/.codex/config.toml"), "[agents]\n");
    write_file(
        &format!("{codex_agents_dir}/junior.toml"),
        "vida_runtime_roles = \"worker\"\n",
    );
    write_file(
        &format!("{template_dir}/vida.config.yaml.template"),
        "project:\n  id: <PROJECT_ID>\n",
    );

    let parent = std::path::Path::new(archive_path)
        .parent()
        .expect("archive parent should exist");
    fs::create_dir_all(parent).expect("archive parent dir should exist");

    let output = Command::new("tar")
        .args(["-czf", archive_path, "-C", &stage_root, "vida-stack-v-test"])
        .output()
        .expect("tar should create release archive");
    assert!(
        output.status.success(),
        "tar should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_dir_all(&stage_root).expect("release stage dir should be removed");
}

const MAX_BOOT_RETRY_ATTEMPTS: usize = 60;

fn retry_with_backoff<F, P>(
    operation: &mut F,
    max_attempts: usize,
    mut should_retry: P,
) -> std::process::Output
where
    F: FnMut() -> std::process::Output,
    P: FnMut(&std::process::Output, usize) -> bool,
{
    let mut last = None;
    for attempt in 0..max_attempts {
        let output = operation();
        if !should_retry(&output, attempt) {
            return output;
        }
        last = Some(output);
        thread::sleep(retry_backoff_delay(attempt));
    }
    last.expect("retry helper should capture at least one output")
}

fn run_with_retry<F>(mut op: F) -> std::process::Output
where
    F: FnMut() -> std::process::Output,
{
    retry_with_backoff(&mut op, MAX_BOOT_RETRY_ATTEMPTS, |output, _| {
        !output.status.success()
    })
}

fn run_with_retry_until<F, P>(mut op: F, mut predicate: P) -> std::process::Output
where
    F: FnMut() -> std::process::Output,
    P: FnMut(&std::process::Output) -> bool,
{
    retry_with_backoff(&mut op, MAX_BOOT_RETRY_ATTEMPTS, |output, _| {
        !predicate(output)
    })
}

fn command_output_with_retry(command: &mut Command) -> std::process::Output {
    let mut last = None;
    for attempt in 0..60 {
        match command.output() {
            Ok(output) if output.status.success() || !is_retryable_temporary_failure(&output) => {
                return output;
            }
            Ok(output) => {
                last = Some(output);
                thread::sleep(retry_backoff_delay(attempt));
            }
            Err(error) if error.raw_os_error() == Some(26) => {
                thread::sleep(retry_backoff_delay(attempt));
            }
            Err(error) => panic!("command should run: {error}"),
        }
    }

    last.expect("command retry helper should capture at least one output")
}

fn is_retryable_temporary_failure(output: &std::process::Output) -> bool {
    output.status.code() == Some(124) || is_state_lock_error(output)
}

fn is_state_lock_error_text(text: &str) -> bool {
    text.contains(support::STATE_LOCK_ERROR_MESSAGE)
        || text.contains("timed out while waiting for authoritative datastore lock")
        || text.contains("Timed out opening authoritative state store")
}

fn is_state_lock_error(output: &std::process::Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    is_state_lock_error_text(&stderr)
}

fn retry_backoff_delay(attempt: usize) -> Duration {
    Duration::from_millis(match attempt {
        0..=4 => 10,
        5..=9 => 25,
        10..=19 => 50,
        _ => 100,
    })
}

fn memory_output_with_timeout_retry(state_dir: &str) -> std::process::Output {
    bounded_vida_output(&["-k", "2s", "8s"], "memory should run", |command| {
        command.arg("memory").env("VIDA_STATE_DIR", state_dir);
    })
}

fn project_bound_taskflow_consume_continue_with_timeout(
    project_root: &str,
    state_dir: &str,
    args: &[&str],
) -> std::process::Output {
    bounded_vida_output_with_state_lock_retry(
        &["-k", "5s", "20s"],
        "taskflow consume continue should run",
        |command| {
            command
                .args(["taskflow", "consume", "continue"])
                .args(args)
                .current_dir(project_root)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", state_dir);
        },
    )
}

fn project_bound_taskflow_consume_continue_once_with_timeout(
    project_root: &str,
    state_dir: &str,
    args: &[&str],
) -> std::process::Output {
    bounded_vida_output(
        &["-k", "5s", "20s"],
        "taskflow consume continue should run",
        |command| {
            command
                .args(["taskflow", "consume", "continue"])
                .args(args)
                .current_dir(project_root)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", state_dir);
        },
    )
}

fn mark_project_run_graph_closure_complete(project_root: &str, state_dir: &str, run_id: &str) {
    let output = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "run-graph",
                "update",
                run_id,
                "implementation",
                "closure",
                "completed",
                "implementation",
                r#"{"lifecycle_stage":"closure_complete","resume_target":null,"checkpoint_kind":"execution_cursor","context_state":"sealed","handoff_state":"none","policy_gate":"not_required","recovery_ready":false}"#,
            ])
            .current_dir(project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", state_dir);
        command
    });
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn taskflow_consume_bundle_check_with_timeout(state_dir: &str) -> std::process::Output {
    bounded_vida_output(
        &["-k", "5s", "20s"],
        "taskflow consume bundle check json should run",
        |command| {
            command
                .args(["taskflow", "consume", "bundle", "check", "--json"])
                .env("VIDA_STATE_DIR", state_dir);
        },
    )
}

fn status_with_timeout(project_root: &str, state_dir: &str, args: &[&str]) -> std::process::Output {
    bounded_vida_output(&["-k", "5s", "20s"], "status should run", |command| {
        command
            .args(args)
            .current_dir(project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", state_dir);
    })
}

fn doctor_with_timeout(state_dir: &str, args: &[&str]) -> std::process::Output {
    bounded_vida_output(&["-k", "5s", "20s"], "doctor should run", |command| {
        command.args(args).env("VIDA_STATE_DIR", state_dir);
    })
}

fn taskflow_run_graph_latest_with_timeout(state_dir: &str, json: bool) -> std::process::Output {
    taskflow_run_graph_with_timeout(state_dir, "latest", None, json)
}

fn taskflow_run_graph_status_with_timeout(
    state_dir: &str,
    run_id: &str,
    json: bool,
) -> std::process::Output {
    taskflow_run_graph_with_timeout(state_dir, "status", Some(run_id), json)
}

fn taskflow_run_graph_with_timeout(
    state_dir: &str,
    subcommand: &str,
    run_id: Option<&str>,
    json: bool,
) -> std::process::Output {
    bounded_vida_output(
        &["-k", "5s", "20s"],
        "taskflow run-graph should run",
        |command| {
            command.args(["taskflow", "run-graph", subcommand]);
            if let Some(run_id) = run_id {
                command.arg(run_id);
            }
            if json {
                command.arg("--json");
            }
            command.env("VIDA_STATE_DIR", state_dir);
        },
    )
}

fn taskflow_recovery_status_with_timeout(
    state_dir: &str,
    run_id: &str,
    json: bool,
) -> std::process::Output {
    taskflow_recovery_with_timeout(state_dir, "status", Some(run_id), json)
}

fn taskflow_recovery_latest_with_timeout(
    state_dir: &str,
    subcommand: &str,
    json: bool,
) -> std::process::Output {
    taskflow_recovery_with_timeout(state_dir, subcommand, None, json)
}

fn taskflow_recovery_with_timeout(
    state_dir: &str,
    subcommand: &str,
    run_id: Option<&str>,
    json: bool,
) -> std::process::Output {
    bounded_vida_output(
        &["-k", "5s", "20s"],
        "taskflow recovery should run",
        |command| {
            command.args(["taskflow", "recovery", subcommand]);
            if let Some(run_id) = run_id {
                command.arg(run_id);
            }
            if json {
                command.arg("--json");
            }
            command.env("VIDA_STATE_DIR", state_dir);
        },
    )
}

fn status_or_doctor_with_timeout(state_dir: &str, args: &[&str]) -> std::process::Output {
    bounded_vida_output(
        &["-k", "5s", "20s"],
        "status/doctor should run",
        |command| {
            command
                .args(args)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", state_dir);
        },
    )
}

fn run_with_state_lock_retry<F>(mut op: F) -> std::process::Output
where
    F: FnMut() -> std::process::Output,
{
    support::retry_with_backoff(&mut op, 600, |output| is_state_lock_error(output))
}

fn run_command_with_state_lock_retry<F>(mut build: F) -> std::process::Output
where
    F: FnMut() -> Command,
{
    run_with_state_lock_retry(|| build().output().expect("command should run"))
}

struct StateStoreLockGuard {
    _runtime: tokio::runtime::Runtime,
    _db: Surreal<Db>,
}

impl StateStoreLockGuard {
    fn acquire(state_dir: &str) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let db = runtime.block_on(async {
            let db: Surreal<Db> = Surreal::new::<SurrealKv>(PathBuf::from(state_dir))
                .await
                .expect("state db should open for lock hold");
            db.use_ns("vida")
                .use_db("primary")
                .await
                .expect("state namespace should open");
            db
        });
        Self {
            _runtime: runtime,
            _db: db,
        }
    }
}

fn boot_with_retry(state_dir: &str) -> std::process::Output {
    run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command.arg("boot");
        command.env("VIDA_STATE_DIR", state_dir);
        command
    })
}

fn wait_for_state_unlock(state_dir: &str) {
    let direct_lock_path = Path::new(state_dir).join("LOCK");
    let nested_lock_path = Path::new(state_dir)
        .join(".vida")
        .join("data")
        .join("state")
        .join("LOCK");
    let deadline = SystemTime::now() + Duration::from_secs(2);
    while (direct_lock_path.exists() || nested_lock_path.exists()) && SystemTime::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn boot_smoke_launcher_activation_snapshot_overwrite_helper() {
    let Ok(state_dir) = std::env::var(SNAPSHOT_OVERWRITE_HELPER_STATE_DIR_ENV) else {
        return;
    };
    let source = std::env::var(SNAPSHOT_OVERWRITE_HELPER_SOURCE_ENV)
        .expect("snapshot overwrite helper source should be set");
    let config_path = std::env::var(SNAPSHOT_OVERWRITE_HELPER_CONFIG_PATH_ENV)
        .expect("snapshot overwrite helper config path should be set");
    let compiled_bundle = serde_json::from_str(
        &std::env::var(SNAPSHOT_OVERWRITE_HELPER_COMPILED_BUNDLE_ENV)
            .expect("snapshot overwrite helper compiled bundle should be set"),
    )
    .expect("snapshot overwrite helper compiled bundle should parse");

    overwrite_launcher_activation_snapshot_in_process(
        &state_dir,
        &source,
        &config_path,
        compiled_bundle,
    );
    wait_for_state_unlock(&state_dir);
}

#[test]
fn boot_smoke_direct_snapshot_overwrite_releases_lock_before_next_cli() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    overwrite_launcher_activation_snapshot(
        &state_dir,
        serde_json::json!({
            "role_selection": {
                "fallback_role": "orchestrator",
                "mode": "auto"
            },
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only"
            }
        }),
    );

    let next_boot = bounded_vida_output_with_state_lock_retry(
        &["-k", "5s", "20s"],
        "boot after direct snapshot overwrite should run",
        |command| {
            command.arg("boot").env("VIDA_STATE_DIR", &state_dir);
        },
    );
    assert!(
        next_boot.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&next_boot.stdout),
        String::from_utf8_lossy(&next_boot.stderr)
    );
}

#[test]
fn root_help_succeeds() {
    let output = vida().arg("--help").output().expect("root help should run");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: vida [COMMAND]"));
    assert!(stdout.contains("boot"));
    assert!(stdout.contains("task"));
    assert!(stdout.contains(
        "task inspection, mutation, and graph routing over the authoritative state store"
    ));
    assert!(stdout.contains("memory"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("consume"));
    assert!(stdout.contains("lane"));
    assert!(stdout.contains("approval"));
    assert!(stdout.contains("recovery"));
    assert!(stdout.contains("thin root alias to the TaskFlow consume family"));
    assert!(stdout.contains("inspect or mutate canonical lane/takeover operator state"));
    assert!(stdout.contains(
        "approval           family-owned root operator surface for approval inspection over the run-graph approval law"
    ));
    assert!(stdout.contains("thin root alias to the TaskFlow recovery family"));
    assert!(stdout.contains("taskflow"));
    assert!(stdout.contains("docflow"));
}

#[test]
fn root_version_succeeds() {
    let output = vida()
        .arg("--version")
        .output()
        .expect("root version should run");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), format!("vida {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn protocol_view_accepts_multiple_targets_in_json_mode() {
    let output = vida()
        .args([
            "protocol",
            "view",
            "bootstrap/router",
            "agent-definitions/entry.worker-entry",
            "--json",
        ])
        .output()
        .expect("protocol view should run");
    if !output.status.success() {
        panic!(
            "consume final failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("protocol view json should parse");
    assert_eq!(parsed["surface"], "vida protocol view");
    assert_eq!(parsed["requested_names"][0], "bootstrap/router");
    assert_eq!(
        parsed["requested_names"][1],
        "agent-definitions/entry.worker-entry"
    );
    assert_eq!(parsed["targets"][0]["resolved_id"], "bootstrap/router");
    assert_eq!(
        parsed["targets"][1]["resolved_id"],
        "agent-definitions/entry.worker-entry"
    );
}

#[test]
fn protocol_view_accepts_multiple_targets_in_plain_mode() {
    let output = vida()
        .args([
            "protocol",
            "view",
            "bootstrap/router",
            "agent-definitions/entry.worker-entry",
        ])
        .output()
        .expect("protocol view should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("===== bootstrap/router ====="));
    assert!(stdout.contains("===== agent-definitions/entry.worker-entry ====="));
}

#[test]
fn boot_succeeds() {
    let output = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", unique_state_dir())
        .output()
        .expect("boot should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida boot scaffold ready"));
    assert!(stdout.contains("authoritative state store: kv-surrealkv"));
    assert!(stdout.contains("authoritative state spine: initialized (state-v1, 8 entity surfaces, mutation root vida task)"));
    assert!(stdout.contains("framework instruction bundle: seeded"));
    assert!(stdout.contains(
        "instruction source tree: vida/config/instructions/bundles/framework-source -> instruction_memory"
    ));
    assert!(stdout.contains(
        "instruction ingest: 3 imported, 0 unchanged, 0 updated from vida/config/instructions/bundles/framework-source"
    ));
    assert!(stdout.contains("boot compatibility: backward_compatible (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration preflight: backward_compatible / no_migration_required (normal_boot_allowed)"
    ));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
    assert!(stdout.contains("effective instruction bundle: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains(
        "effective instruction bundle receipt: effective-bundle-framework-agent-definition-"
    ));
    assert!(stdout.contains(
        "framework memory ingest: 1 imported, 0 unchanged, 0 updated from vida/config/instructions/bundles/framework-memory-source"
    ));
}

#[test]
fn boot_supports_color_render_mode() {
    let output = vida()
        .args(["boot", "--render", "color"])
        .env("VIDA_STATE_DIR", unique_state_dir())
        .output()
        .expect("boot should run with color render mode");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida boot scaffold ready\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34mauthoritative state store\u{1b}[0m"));
}

#[test]
fn boot_is_idempotent_for_unchanged_source_trees() {
    let state_dir = unique_state_dir();
    let first = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("first boot should run");
    assert!(first.status.success());

    let second = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("second boot should run");
    assert!(second.status.success());

    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains(
        "instruction ingest: 0 imported, 3 unchanged, 0 updated from vida/config/instructions/bundles/framework-source"
    ));
    assert!(stdout.contains("effective instruction bundle: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains(
        "effective instruction bundle receipt: effective-bundle-framework-agent-definition-"
    ));
    assert!(stdout.contains(
        "framework memory ingest: 0 imported, 1 unchanged, 0 updated from vida/config/instructions/bundles/framework-memory-source"
    ));
    assert!(stdout.contains("boot compatibility: backward_compatible (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration preflight: backward_compatible / no_migration_required (normal_boot_allowed)"
    ));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
}

#[test]
fn command_family_help_succeeds() {
    for command in ["boot", "task", "memory", "status", "doctor"] {
        let output = vida()
            .args([command, "--help"])
            .output()
            .expect("command help should run");

        assert!(output.status.success(), "{command} help should succeed");
    }
}

#[test]
fn taskflow_proxy_help_is_runtime_specific() {
    let output = vida()
        .args(["taskflow", "help"])
        .output()
        .expect("taskflow proxy help should run");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow runtime family"));
    assert!(stdout.contains(
        "`vida task` and `vida taskflow task` address the same authoritative backlog store."
    ));
    assert!(stdout.contains("vida task ready --json"));
    assert!(stdout.contains("vida task next --json"));
    assert!(stdout.contains(
        "vida taskflow help [task|parallelism|dependencies|queue|next|graph-summary|plan|replan|scheduler|status|consume|continuation|packet|dispatch|run-graph|recovery|doctor|protocol-binding|bootstrap-spec|query]"
    ));
    assert!(stdout.contains("vida taskflow status --summary --json"));
    assert!(stdout.contains("vida taskflow scheduler dispatch --json"));
    assert!(stdout.contains("vida taskflow query \"what should I run next?\""));
    assert!(stdout.contains(
        "A green test, successful build, or commentary update is not a stop boundary when a next lawful continuation item is already known."
    ));
    assert!(stdout.contains(
        "User-ordered execution takes priority over self-directed cleanup or adjacent development unless the user explicitly authorizes a broader scope."
    ));
}

#[test]
fn taskflow_proxy_help_supports_task_topic() {
    let output = vida()
        .args(["taskflow", "help", "task"])
        .output()
        .expect("taskflow task topic help should run");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: task"));
    assert!(stdout.contains("`vida task` is the root parity surface"));
    assert!(stdout.contains("vida task next [--scope <task-id>] [--state-dir <path>] [--json]"));
    assert!(stdout.contains("vida task ready --scope <task-id> --json"));
    assert!(stdout.contains("vida task next-display-id <parent-display-id> --json"));
    assert!(stdout.contains("vida task reparent-children <from-parent-id> <to-parent-id> --json"));
    assert!(stdout.contains(
        "vida task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description"
    ));
    assert!(stdout.contains("vida task ensure <task-id> <title> --parent-id <parent-id>"));
    assert!(stdout
        .contains("vida task update <task-id> --status in_progress --notes-file <path> --json"));
    assert!(stdout.contains("vida task import-jsonl .vida/exports/tasks.snapshot.jsonl --json"));
    assert!(stdout.contains("vida task export-jsonl .vida/exports/tasks.snapshot.jsonl --json"));
    assert!(stdout.contains("Parent-child edges preserve epic/task structure"));
}

#[test]
fn taskflow_task_help_alias_routes_to_canonical_task_help() {
    let output = vida()
        .args(["taskflow", "task", "help"])
        .output()
        .expect("taskflow task help alias should run");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: task"));
    assert!(stdout.contains("vida task next [--scope <task-id>] [--state-dir <path>] [--json]"));
    assert!(stdout.contains("vida task ready --scope <task-id> --json"));
    assert!(stdout.contains("vida task next-display-id <parent-display-id> --json"));
    assert!(stdout.contains("vida task reparent-children <from-parent-id> <to-parent-id> --json"));
    assert!(stdout.contains("vida task ensure <task-id> <title> --parent-id <parent-id>"));
    assert!(stdout.contains("vida task import-jsonl .vida/exports/tasks.snapshot.jsonl --json"));
    assert!(stdout
        .contains("vida task update <task-id> --status in_progress --notes-file <path> --json"));
}

#[test]
fn root_task_help_supports_next_topic() {
    let output = vida()
        .args(["task", "help", "next"])
        .output()
        .expect("root task next help should run");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: next"));
    assert!(stdout.contains("vida task next [--scope <task-id>] [--state-dir <path>] [--json]"));
}

#[test]
fn root_task_help_routes_backlog_subcommand_topics_to_canonical_task_help() {
    let output = vida()
        .args(["task", "help", "blocked"])
        .output()
        .expect("root task blocked help should run");

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: task"));
    assert!(stdout.contains("vida task blocked --json"));
    assert!(stdout.contains("vida task critical-path --json"));
}

#[test]
fn taskflow_next_reports_aggregate_next_step_surface() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(
        boot.status.success(),
        "boot stdout={}\nboot stderr={}",
        String::from_utf8_lossy(&boot.stdout),
        String::from_utf8_lossy(&boot.stderr)
    );

    let output = vida()
        .args(["taskflow", "next", "--state-dir"])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("taskflow next should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow next json should parse");
    assert_eq!(parsed["surface"], "vida taskflow next");
    assert!(parsed["status"].is_string());
    assert!(parsed["blocker_codes"].is_array());
    assert!(parsed["next_actions"].is_array());
    assert!(parsed["ready_count"].is_number());
    assert!(parsed.get("primary_ready_task").is_some());
    assert!(parsed.get("recovery").is_some());
}

#[test]
fn taskflow_next_accepts_scope_for_subtree_planning() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(
        boot.status.success(),
        "boot stdout={}\nboot stderr={}",
        String::from_utf8_lossy(&boot.stdout),
        String::from_utf8_lossy(&boot.stderr)
    );
    let create_scope = vida()
        .args([
            "task",
            "create",
            "r1-01-commands",
            "Commands",
            "--state-dir",
        ])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("scope task create should run");
    assert!(
        create_scope.status.success(),
        "{}",
        String::from_utf8_lossy(&create_scope.stderr)
    );

    let output = vida()
        .args([
            "taskflow",
            "next",
            "--scope",
            "r1-01-commands",
            "--state-dir",
        ])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("scoped taskflow next should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("scoped taskflow next json should parse");
    assert_eq!(parsed["surface"], "vida taskflow next");
    assert_eq!(parsed["scope_task_id"], "r1-01-commands");
    assert!(parsed["status"].is_string());
    assert!(parsed["ready_count"].is_number());
    assert!(parsed["recommended_command"].is_string());
}

#[test]
fn taskflow_next_accepts_explicit_state_dir_override() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "next", "--state-dir"])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("taskflow next with state-dir should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow next with state-dir json should parse");
    assert_eq!(parsed["surface"], "vida taskflow next");
    assert!(parsed["status"].is_string());
    assert!(parsed["ready_count"].is_number());
}

#[test]
fn task_root_next_alias_routes_to_taskflow_next_surface() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());
    let create_scope = vida()
        .args([
            "task",
            "create",
            "r1-01-commands",
            "Commands",
            "--state-dir",
        ])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("scope task create should run");
    assert!(
        create_scope.status.success(),
        "{}",
        String::from_utf8_lossy(&create_scope.stderr)
    );

    let output = vida()
        .args(["task", "next", "--scope", "r1-01-commands", "--state-dir"])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("root task next alias should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root task next json should parse");
    assert_eq!(parsed["surface"], "vida taskflow next");
    assert_eq!(parsed["scope_task_id"], "r1-01-commands");
    assert!(parsed["status"].is_string());
    assert!(parsed["recommended_command"].is_string());
}

#[test]
fn task_root_next_alias_accepts_explicit_state_dir_override() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["task", "next", "--state-dir"])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("root task next with state-dir should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root task next with state-dir json should parse");
    assert_eq!(parsed["surface"], "vida taskflow next");
    assert!(parsed["status"].is_string());
    assert!(parsed["recommended_command"].is_string());
}

#[test]
fn taskflow_graph_summary_reports_ready_blocked_and_critical_path() {
    let output = vida()
        .args(["taskflow", "graph-summary", "--json"])
        .output()
        .expect("taskflow graph-summary should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow graph-summary json should parse");
    assert_eq!(parsed["surface"], "vida taskflow graph-summary");
    assert!(parsed["status"].is_string());
    assert!(parsed["blocker_codes"].is_array());
    assert!(parsed["next_actions"].is_array());
    assert!(parsed["ready_count"].is_number());
    assert!(parsed["blocked_count"].is_number());
    assert!(parsed["critical_path_length"].is_number());
    assert!(parsed.get("primary_ready_task").is_some());
    assert!(parsed.get("primary_blocked_task").is_some());
    assert!(parsed["waves"].is_array());
    assert!(parsed.get("critical_path").is_some());
}

#[test]
fn taskflow_graph_explain_reports_projection_truth() {
    let output = vida()
        .args(["taskflow", "graph", "explain", "--json"])
        .output()
        .expect("taskflow graph explain should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow graph explain json should parse");
    assert_eq!(parsed["surface"], "vida taskflow graph explain");
    assert!(parsed["status"].is_string());
    assert!(parsed["blocker_codes"].is_array());
    assert!(parsed["ready_now"].is_boolean());
    assert!(parsed["ready_parallel_safe"].is_boolean());
    assert!(parsed["blocked_by"].is_array());
    assert!(parsed["parallel_blockers"].is_array());
    assert_eq!(
        parsed["truth_source"],
        "canonical_task_graph_scheduler_projection"
    );
}

#[test]
fn taskflow_scheduler_dispatch_reports_preview_plan() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "scheduler", "dispatch", "--state-dir"])
        .arg(&state_dir)
        .args(["--json"])
        .output()
        .expect("taskflow scheduler dispatch should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow scheduler dispatch json should parse");
    assert_eq!(parsed["surface"], "vida taskflow scheduler dispatch");
    assert!(parsed["status"].is_string());
    assert!(parsed["blocker_codes"].is_array());
    assert!(parsed["next_actions"].is_array());
    assert!(parsed["max_parallel_agents"].is_number());
    assert!(parsed["selected_parallel_tasks"].is_array());
    assert!(parsed["selected_task_ids"].is_array());
    assert!(parsed["rejected_candidates"].is_array());
    assert!(parsed.get("scheduling").is_some());
}

fn create_scheduler_smoke_task(
    state_dir: &str,
    task_id: &str,
    title: &str,
    priority: &str,
    execution_mode: &str,
    order_bucket: Option<&str>,
    parallel_group: Option<&str>,
    conflict_domain: Option<&str>,
) {
    let output = bounded_vida_output(
        &["-k", "5s", "20s"],
        "scheduler smoke task create should run",
        |command| {
            command.args([
                "task",
                "create",
                task_id,
                title,
                "--type",
                "task",
                "--status",
                "open",
                "--priority",
                priority,
                "--execution-mode",
                execution_mode,
                "--state-dir",
                state_dir,
                "--json",
            ]);
            if let Some(value) = order_bucket {
                command.args(["--order-bucket", value]);
            }
            if let Some(value) = parallel_group {
                command.args(["--parallel-group", value]);
            }
            if let Some(value) = conflict_domain {
                command.args(["--conflict-domain", value]);
            }
        },
    );
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed_scheduler_execute_smoke_tasks(state_dir: &str) {
    create_scheduler_smoke_task(
        state_dir,
        "sched-primary",
        "Scheduler primary",
        "1",
        "parallel_safe",
        Some("wave-a"),
        Some("docs"),
        Some("primary"),
    );
    create_scheduler_smoke_task(
        state_dir,
        "sched-parallel-a",
        "Scheduler parallel A",
        "2",
        "parallel_safe",
        Some("wave-a"),
        Some("docs"),
        Some("parallel-a"),
    );
    create_scheduler_smoke_task(
        state_dir,
        "sched-unsafe",
        "Scheduler unsafe",
        "3",
        "sequential",
        None,
        None,
        None,
    );
}

fn scheduler_execute_smoke_payload(requested_parallel_limit: u64) -> serde_json::Value {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    seed_scheduler_execute_smoke_tasks(&state_dir);

    let output = bounded_vida_output(
        &["-k", "5s", "20s"],
        "scheduler execute smoke should run",
        |command| {
            command.args([
                "taskflow",
                "scheduler",
                "dispatch",
                "--current-task-id",
                "sched-primary",
                "--state-dir",
                &state_dir,
                "--execute",
                "--limit",
                &requested_parallel_limit.to_string(),
                "--json",
            ]);
        },
    );
    assert!(
        !output.status.success(),
        "scheduler --execute should remain blocked until external execution is available, got success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).expect("scheduler execute json should parse")
}

#[test]
fn taskflow_scheduler_dispatch_execute_smoke_reports_projection_truth_and_parallel_cap() {
    let cap_one = scheduler_execute_smoke_payload(1);
    assert_eq!(cap_one["surface"], "vida taskflow scheduler dispatch");
    assert_eq!(cap_one["status"], "blocked");
    assert_eq!(cap_one["max_parallel_agents"], 1);
    assert_eq!(cap_one["execute_requested"], true);
    assert_eq!(cap_one["execute_supported"], true);
    if cap_one["execution_attempted"] == true {
        assert_eq!(
            cap_one["execution_status"],
            "scheduler_agent_init_activation_view_only"
        );
        assert_eq!(
            cap_one["blocker_codes"],
            serde_json::json!(["scheduler_agent_init_activation_view_only"])
        );
        assert_eq!(cap_one["dispatch_receipt"]["receipt_status"], "persisted");
        assert_eq!(
            cap_one["dispatch_receipt"]["execute_status"],
            "scheduler_agent_init_activation_view_only"
        );
        assert_eq!(
            cap_one["dispatch_receipt"]["preview_only_reason"],
            "scheduler_agent_init_activation_view_only"
        );
        assert_eq!(cap_one["dispatch_receipt"]["receipt_persisted"], true);
        assert!(cap_one["dispatch_receipt"]["receipt_id"].is_string());
        let cap_one_receipt_path = cap_one["dispatch_receipt"]["receipt_path"]
            .as_str()
            .expect("scheduler execute receipt path should render");
        assert!(Path::new(cap_one_receipt_path).exists());
    } else {
        assert_eq!(cap_one["execution_attempted"], false);
        assert_eq!(
            cap_one["execution_status"],
            "execution_preparation_gate_blocked"
        );
        assert!(cap_one["blocker_codes"]
            .as_array()
            .expect("blocker codes should render")
            .iter()
            .any(|code| code == "execution_preparation_gate_blocked"));
        assert_eq!(cap_one["dispatch_receipt"]["receipt_persisted"], false);
    }
    assert_eq!(
        cap_one["selected_task_ids"],
        serde_json::json!(["sched-primary"])
    );
    assert_eq!(cap_one["selected_parallel_tasks"], serde_json::json!([]));
    let cap_one_rejected = cap_one["rejected_candidates"]
        .as_array()
        .expect("rejected candidates should render");
    assert!(cap_one_rejected.iter().any(|candidate| {
        candidate["task"]["id"] == "sched-parallel-a"
            && candidate["reasons"]
                .as_array()
                .expect("candidate reasons should render")
                .iter()
                .any(|reason| reason == "max_parallel_agents_cap_reached")
    }));
    assert!(cap_one_rejected.iter().any(|candidate| {
        candidate["task"]["id"] == "sched-unsafe"
            && candidate["parallel_blockers"]
                .as_array()
                .expect("parallel blockers should render")
                .iter()
                .any(|reason| reason == "execution_mode_not_parallel_safe")
    }));

    let cap_two = scheduler_execute_smoke_payload(2);
    assert_eq!(cap_two["surface"], "vida taskflow scheduler dispatch");
    assert_eq!(cap_two["status"], "blocked");
    assert_eq!(cap_two["max_parallel_agents"], 2);
    assert_eq!(cap_two["execute_requested"], true);
    assert_eq!(cap_two["execute_supported"], true);
    assert_eq!(
        cap_two["selected_task_ids"],
        serde_json::json!(["sched-primary", "sched-parallel-a"])
    );
    assert_eq!(
        cap_two["selected_parallel_tasks"][0]["id"],
        "sched-parallel-a"
    );
    if cap_two["execution_attempted"] == true {
        assert_eq!(
            cap_two["execution_status"],
            "scheduler_agent_init_activation_view_only"
        );
        assert_eq!(
            cap_two["reservations"][0]["reservation_status"],
            "reservation_persisted"
        );
        assert_eq!(
            cap_two["reservations"][0]["execute_status"],
            "activation_view_only"
        );
        assert_eq!(
            cap_two["reservations"][0]["preview_only_reason"],
            "agent_init_activation_view_only"
        );
        assert_eq!(cap_two["reservations"][0]["reservation_persisted"], true);
        assert_eq!(cap_two["reservations"][0]["execute_supported"], true);
        assert_eq!(cap_two["reservations"][0]["execution_attempted"], true);
        assert_eq!(
            cap_two["dispatch_receipt"]["selected_task_ids"],
            serde_json::json!(["sched-primary", "sched-parallel-a"])
        );
        assert_eq!(cap_two["dispatch_receipt"]["execute_supported"], true);
        assert_eq!(cap_two["dispatch_receipt"]["execution_attempted"], true);
        assert!(cap_two["dispatch_receipt"]["receipt_id"].is_string());
        let cap_two_receipt_path = cap_two["dispatch_receipt"]["receipt_path"]
            .as_str()
            .expect("scheduler execute receipt path should render");
        assert!(Path::new(cap_two_receipt_path).exists());
    } else {
        assert_eq!(cap_two["execution_attempted"], false);
        assert_eq!(
            cap_two["execution_status"],
            "execution_preparation_gate_blocked"
        );
        assert!(cap_two["blocker_codes"]
            .as_array()
            .expect("blocker codes should render")
            .iter()
            .any(|code| code == "execution_preparation_gate_blocked"));
        assert_eq!(cap_two["dispatch_receipt"]["receipt_persisted"], false);
    }
    let cap_two_rejected = cap_two["rejected_candidates"]
        .as_array()
        .expect("rejected candidates should render");
    assert!(cap_two_rejected.iter().any(|candidate| {
        candidate["task"]["id"] == "sched-unsafe"
            && candidate["reasons"]
                .as_array()
                .expect("candidate reasons should render")
                .iter()
                .any(|reason| reason == "execution_mode_not_parallel_safe")
    }));
}

#[test]
fn taskflow_proxy_help_supports_graph_summary_topic() {
    let output = vida()
        .args(["taskflow", "help", "graph-summary"])
        .output()
        .expect("taskflow graph-summary topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: graph-summary"));
    assert!(stdout.contains("vida taskflow graph-summary [--json]"));
    assert!(stdout.contains("ready_count, blocked_count, critical_path_length"));
    assert!(stdout.contains("waves"));
    assert!(stdout.contains("vida task validate-graph"));
}

#[test]
fn taskflow_proxy_help_supports_graph_topic() {
    let output = vida()
        .args(["taskflow", "help", "graph"])
        .output()
        .expect("taskflow graph topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: graph"));
    assert!(stdout.contains("vida taskflow graph explain"));
    assert!(stdout.contains("ready_now"));
    assert!(stdout.contains("parallel_blockers"));
}

#[test]
fn taskflow_proxy_help_supports_scheduler_topic() {
    let output = vida()
        .args(["taskflow", "help", "scheduler"])
        .output()
        .expect("taskflow scheduler topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: scheduler"));
    assert!(stdout.contains("vida taskflow scheduler dispatch"));
    assert!(stdout.contains("max_parallel_agents"));
    assert!(stdout.contains("selected_primary_task"));
    assert!(stdout.contains("selected_parallel_tasks"));
}

#[test]
fn taskflow_proxy_help_supports_next_scope_contract() {
    let output = vida()
        .args(["taskflow", "help", "next"])
        .output()
        .expect("taskflow next topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida task next [--scope <task-id>] [--state-dir <path>] [--json]"));
    assert!(stdout.contains("scope_task_id"));
    assert!(stdout.contains("Unknown scoped task ids fail closed"));
}

#[test]
fn taskflow_proxy_help_supports_recovery_topic() {
    let output = vida()
        .args(["taskflow", "help", "recovery"])
        .output()
        .expect("taskflow recovery topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: recovery"));
    assert!(stdout.contains("vida taskflow recovery status <run-id> [--json]"));
    assert!(stdout.contains("vida taskflow recovery latest [--json]"));
    assert!(stdout.contains("vida taskflow recovery checkpoint <run-id> [--json]"));
    assert!(stdout.contains("vida taskflow recovery checkpoint-latest [--json]"));
    assert!(stdout.contains("vida taskflow recovery gate <run-id> [--json]"));
    assert!(stdout.contains("vida taskflow recovery gate-latest [--json]"));
    assert!(stdout.contains("resume_node, resume_status, checkpoint_kind"));
}

#[test]
fn taskflow_proxy_help_supports_doctor_topic() {
    let output = vida()
        .args(["taskflow", "help", "doctor"])
        .output()
        .expect("taskflow doctor topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: doctor"));
    assert!(stdout.contains("vida taskflow doctor [--json]"));
    assert!(stdout.contains("task store summary"));
    assert!(stdout.contains("dependency graph integrity"));
    assert!(stdout.contains("runtime-consumption evidence posture"));
    assert!(stdout.contains("latest recovery, checkpoint, gate, and dispatch receipt summaries"));
}

#[test]
fn taskflow_proxy_help_supports_protocol_binding_topic() {
    let output = vida()
        .args(["taskflow", "help", "protocol-binding"])
        .output()
        .expect("taskflow protocol-binding topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: protocol-binding"));
    assert!(stdout.contains("vida taskflow protocol-binding sync [--json]"));
    assert!(stdout.contains("vida taskflow protocol-binding status [--json]"));
    assert!(stdout.contains("vida taskflow protocol-binding check [--json]"));
}

#[test]
fn taskflow_proxy_help_supports_consume_topic() {
    let output = vida()
        .args(["taskflow", "help", "consume"])
        .output()
        .expect("taskflow consume topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: consume"));
    assert!(stdout.contains("vida taskflow consume bundle [--json]"));
    assert!(stdout.contains("vida taskflow consume bundle check [--json]"));
    assert!(stdout.contains("vida taskflow consume final \"<request>\" --json"));
    assert!(stdout.contains(
        "Bundle inspection, final intake, continuation, and bounded advance are launcher-owned and in-process"
    ));
}

#[test]
fn root_consume_bundle_check_alias_matches_taskflow_surface() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["consume", "bundle", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("root consume bundle check alias should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Authoritative state root is not project-bound"));
}

#[test]
fn root_recovery_latest_alias_matches_taskflow_surface() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["recovery", "latest", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("root recovery latest alias should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root recovery latest json should parse");
    assert_eq!(parsed["surface"], "vida taskflow recovery latest");
}

#[test]
fn root_lane_surface_fails_closed_with_canonical_json_envelope() {
    let output = vida()
        .args(["lane", "--json"])
        .output()
        .expect("root lane surface should run");

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root lane json should parse");
    assert_eq!(parsed["surface"], "vida lane");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(parsed["trace_id"], serde_json::Value::Null);
    assert_eq!(parsed["workflow_class"], serde_json::Value::Null);
    assert_eq!(parsed["risk_tier"], serde_json::Value::Null);
    assert_eq!(parsed["artifact_refs"], serde_json::json!([]));
    assert_eq!(
        parsed["blocker_codes"],
        serde_json::json!(["unsupported_blocker_code"])
    );
    assert!(parsed["next_actions"].is_array());
}

#[test]
fn root_approval_surface_emits_blocked_canonical_json_envelope() {
    let output = vida()
        .args(["approval", "--json"])
        .output()
        .expect("root approval surface should run");

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root approval json should parse");
    assert_eq!(parsed["surface"], "vida approval");
    assert_eq!(parsed["status"], "blocked");
    assert_eq!(parsed["trace_id"], serde_json::Value::Null);
    assert_eq!(parsed["workflow_class"], serde_json::Value::Null);
    assert_eq!(parsed["risk_tier"], serde_json::Value::Null);
    assert_eq!(parsed["artifact_refs"], serde_json::json!([]));
    assert_eq!(
        parsed["blocker_codes"],
        serde_json::json!(["unsupported_blocker_code"])
    );
    assert!(parsed["reason"]
        .as_str()
        .expect("approval reason should be a string")
        .contains("root surface blocks missing or invalid approval requests"));
    assert_eq!(
        parsed["next_actions"][0],
        "Use `vida approval show --latest --json` or `vida approval show <run-id> --json` once approval evidence exists."
    );
}

#[test]
fn root_approval_show_latest_smokes_waiting_for_approval() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());
    wait_for_state_unlock(&state_dir);
    let init = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "run-graph",
                "init",
                "run-approval-latest",
                "implementation",
                "implementation",
            ])
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        init.status.success(),
        "{}{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let update = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "run-graph",
                "update",
                "run-approval-latest",
                "implementation",
                "approval",
                "awaiting_approval",
                "implementation",
                "{\"next_node\":\"approval\",\"selected_backend\":\"internal_subagents\",\"lane_id\":\"dev_pack_direct\",\"lifecycle_stage\":\"approval_wait\",\"policy_gate\":\"approval_required\",\"handoff_state\":\"awaiting_approval\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"conversation_cursor\",\"resume_target\":\"dispatch.approval\",\"recovery_ready\":true}",
            ])
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        update.status.success(),
        "{}{}",
        String::from_utf8_lossy(&update.stdout),
        String::from_utf8_lossy(&update.stderr)
    );

    let output = bounded_vida_output_with_state_lock_retry(
        &["-k", "5s", "20s"],
        "root approval show latest should run",
        |command| {
            command
                .args(["approval", "show", "--latest", "--json"])
                .env("VIDA_STATE_DIR", &state_dir);
        },
    );

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root approval latest json should parse");
    assert_eq!(parsed["surface"], "vida approval");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["run_id"], "run-approval-latest");
    assert_eq!(parsed["task_id"], "run-approval-latest");
    assert_eq!(parsed["approval_status"], "waiting_for_approval");
    assert_eq!(parsed["gate_level"], "block");
    assert_eq!(parsed["expiry_state"], "not_tracked");
    assert!(!parsed["decision_reason"]
        .as_str()
        .expect("approval decision reason should render")
        .trim()
        .is_empty());
}

#[test]
fn root_approval_show_run_smokes_completed_run_as_approved() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());
    wait_for_state_unlock(&state_dir);
    let init = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "run-graph",
                "init",
                "run-approval-specific",
                "implementation",
                "implementation",
            ])
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        init.status.success(),
        "{}{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let update = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "run-graph",
                "update",
                "run-approval-specific",
                "implementation",
                "closure",
                "completed",
                "implementation",
                "{\"next_node\":null,\"selected_backend\":\"internal_subagents\",\"lane_id\":\"dev_pack_direct\",\"lifecycle_stage\":\"implementation_complete\",\"policy_gate\":\"not_required\",\"handoff_state\":\"none\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"conversation_cursor\",\"resume_target\":\"none\",\"recovery_ready\":true}",
            ])
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        update.status.success(),
        "{}{}",
        String::from_utf8_lossy(&update.stdout),
        String::from_utf8_lossy(&update.stderr)
    );

    let output = bounded_vida_output_with_state_lock_retry(
        &["-k", "5s", "20s"],
        "root approval show run should run",
        |command| {
            command
                .args(["approval", "show", "run-approval-specific", "--json"])
                .env("VIDA_STATE_DIR", &state_dir);
        },
    );

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("root approval run json should parse");
    assert_eq!(parsed["surface"], "vida approval");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["run_id"], "run-approval-specific");
    assert_eq!(parsed["approval_status"], "approved");
    assert_eq!(parsed["gate_level"], "observe");
    assert_eq!(parsed["expiry_state"], "not_applicable");
    assert!(!parsed["decision_reason"]
        .as_str()
        .expect("approval decision reason should render")
        .trim()
        .is_empty());
}

#[test]
fn taskflow_doctor_routes_in_process_without_taskflow_binary() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let delegated_taskflow_bin = format!("{state_dir}/delegated-taskflow-runtime");
    write_executable_script(
        &delegated_taskflow_bin,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    let output = vida()
        .args(["taskflow", "doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &delegated_taskflow_bin)
        .output()
        .expect("taskflow doctor should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow doctor json should parse");
    assert_eq!(parsed["surface"], "vida doctor");
    assert_eq!(parsed["task_store"]["total_count"], 0);
    assert_eq!(parsed["taskflow_snapshot_bridge"]["total_receipts"], 0);
    assert_eq!(
        parsed["launcher_runtime_paths"]["taskflow_surface"],
        "vida taskflow"
    );
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_protocol_binding_bridge_syncs_into_authoritative_state_store() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let precheck = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["taskflow", "protocol-binding", "check", "--json"])
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(!precheck.status.success());
    let precheck_stdout = String::from_utf8_lossy(&precheck.stdout);
    let precheck_json: serde_json::Value = serde_json::from_str(&precheck_stdout)
        .expect("protocol-binding precheck json should parse");
    assert_eq!(precheck_json["status"], "blocked");
    assert_eq!(precheck_json["summary"]["total_receipts"], 0);

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(sync.status.success());
    let sync_stdout = String::from_utf8_lossy(&sync.stdout);
    let sync_json: serde_json::Value =
        serde_json::from_str(&sync_stdout).expect("protocol-binding sync json should parse");
    assert_eq!(sync_json["surface"], "vida taskflow protocol-binding sync");
    assert_eq!(
        sync_json["receipt"]["scenario"],
        "v0.2.2-taskflow-wave1-primary"
    );
    assert_eq!(
        sync_json["receipt"]["primary_state_authority"],
        "taskflow_state_store"
    );
    assert_eq!(sync_json["receipt"]["total_bindings"], 5);
    assert_eq!(sync_json["receipt"]["blocking_issue_count"], 0);
    assert_eq!(sync_json["receipt"]["script_bound_count"], 0);
    assert_eq!(sync_json["receipt"]["fully_runtime_bound_count"], 5);
    assert_eq!(
        sync_json["compiled_payload_import_evidence"]["imported"],
        true
    );
    assert_eq!(
        sync_json["compiled_payload_import_evidence"]["trusted"],
        true
    );
    assert_eq!(
        sync_json["compiled_payload_import_evidence"]["effective_bundle_artifact_count"],
        3
    );
    let bindings = sync_json["bindings"]
        .as_array()
        .expect("protocol-binding sync rows should be an array");
    assert_eq!(bindings.len(), 5);
    assert!(bindings
        .iter()
        .all(|row| row["binding_status"] == "fully-runtime-bound"));
    assert!(bindings
        .iter()
        .all(|row| row["primary_state_authority"] == "taskflow_state_store"));

    let status = vida()
        .args(["taskflow", "protocol-binding", "status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding status should run");
    assert!(status.status.success());
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    let status_json: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("protocol-binding status json should parse");
    assert_eq!(status_json["summary"]["total_receipts"], 1);
    assert_eq!(status_json["summary"]["script_bound_count"], 0);
    assert_eq!(status_json["summary"]["fully_runtime_bound_count"], 5);
    assert_eq!(status_json["summary"]["unbound_count"], 0);
    assert_eq!(status_json["summary"]["blocking_issue_count"], 0);
    assert_eq!(
        status_json["compiled_payload_import_evidence"]["trusted"],
        true
    );

    let check = run_with_state_lock_retry(|| {
        vida()
            .args(["taskflow", "protocol-binding", "check", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("protocol-binding check should run")
    });
    assert!(
        !check.stdout.is_empty(),
        "status={:?}\nstdout:\n{}\nstderr:\n{}",
        check.status.code(),
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let check_stdout = String::from_utf8_lossy(&check.stdout);
    let check_json: serde_json::Value =
        serde_json::from_str(&check_stdout).expect("protocol-binding check json should parse");
    assert_eq!(check_json["status"], "pass");
    assert_eq!(
        check_json["compiled_payload_import_evidence"]["trusted"],
        true
    );

    let doctor = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor.status.success());
    let doctor_stdout = String::from_utf8_lossy(&doctor.stdout);
    let doctor_json: serde_json::Value =
        serde_json::from_str(&doctor_stdout).expect("doctor json should parse");
    assert_eq!(doctor_json["protocol_binding"]["total_receipts"], 1);
    assert_eq!(doctor_json["protocol_binding"]["script_bound_count"], 0);
    assert_eq!(
        doctor_json["protocol_binding"]["fully_runtime_bound_count"],
        5
    );
    assert_eq!(doctor_json["protocol_binding"]["blocking_issue_count"], 0);

    let status_root = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(status_root.status.success());
    let status_root_stdout = String::from_utf8_lossy(&status_root.stdout);
    let status_root_json: serde_json::Value =
        serde_json::from_str(&status_root_stdout).expect("status json should parse");
    assert_eq!(status_root_json["protocol_binding"]["total_receipts"], 1);
    assert_eq!(status_root_json["protocol_binding"]["active_bindings"], 5);
}

#[test]
fn taskflow_protocol_binding_check_fails_closed_without_compiled_payload_import_evidence() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    overwrite_launcher_activation_snapshot_with_source(
        &state_dir,
        "script_runtime",
        serde_json::json!({
            "role_selection": {
                "mode": "auto",
                "fallback_role": "orchestrator"
            },
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only"
            }
        }),
    );

    let check = run_protocol_binding_check_with_timeout(Path::new(&state_dir));
    assert!(!check.status.success());
    assert_ne!(
        check.status.code(),
        Some(124),
        "protocol-binding check timed out under lock contention: {}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn taskflow_status_alias_routes_to_root_status_surface() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "status", "--summary", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow status alias should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow status alias json should parse");
    assert_eq!(parsed["surface"], "vida status");
    assert_eq!(parsed["view"], "summary");
    assert!(parsed["protocol_binding"].is_object());
}

#[test]
fn task_blocked_supports_compact_json_summary_view() {
    let state_dir = unique_state_dir();
    let seed_path = format!("{state_dir}/blocked-summary-seed.jsonl");

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    fs::write(
        &seed_path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-blocker\",\"title\":\"Blocker task\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-blocked\",\"title\":\"Blocked task\",\"description\":\"blocked\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-blocked\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-blocked\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("blocked summary seed jsonl should be written");

    let import = vida()
        .args(["task", "import-jsonl", &seed_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task import should run");
    assert!(import.status.success());

    let output = vida()
        .args(["task", "blocked", "--summary", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task blocked summary json should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("task blocked summary json should parse");
    assert_eq!(parsed["surface"], "vida task blocked");
    assert_eq!(parsed["view"], "summary");
    assert_eq!(parsed["blocked_count"], 1);
    assert_eq!(parsed["tasks"][0]["id"], "vida-blocked");
    assert_eq!(parsed["tasks"][0]["blocker_count"], 1);
    assert_eq!(
        parsed["tasks"][0]["blockers"][0]["depends_on_id"],
        "vida-blocker"
    );
    assert!(parsed["tasks"][0]["description"].is_null());
}

#[test]
fn task_list_supports_compact_json_summary_view() {
    let state_dir = unique_state_dir();
    let seed_path = format!("{state_dir}/task-list-summary-seed.jsonl");

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    fs::write(
        &seed_path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-task\",\"title\":\"Tracked task\",\"description\":\"task\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
        ),
    )
    .expect("task list summary seed jsonl should be written");

    let import = vida()
        .args(["task", "import-jsonl", &seed_path, "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task import should run");
    assert!(import.status.success());

    let output = vida()
        .args(["task", "list", "--summary", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("task list summary json should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("task list summary json should parse");
    assert_eq!(parsed["surface"], "vida task list");
    assert_eq!(parsed["status"], "pass");
    assert_eq!(parsed["view"], "summary");
    assert_eq!(parsed["task_count"], 2);
    assert_eq!(parsed["shared_fields"]["status"], "pass");
    assert_eq!(parsed["operator_contracts"]["status"], "pass");
    assert_eq!(parsed["tasks"][0]["id"], "vida-root");
    assert!(parsed["tasks"][0]["description"].is_null());
}

#[test]
fn taskflow_protocol_binding_check_fails_closed_when_init_compiled_bundle_missing_agent_system_mode(
) {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    overwrite_launcher_activation_snapshot(
        &state_dir,
        serde_json::json!({
            "role_selection": {
                "mode": "auto",
                "fallback_role": "orchestrator"
            },
            "agent_system": {
                "state_owner": "orchestrator_only"
            }
        }),
    );

    let check = run_protocol_binding_check_with_timeout(Path::new(&state_dir));
    assert!(!check.status.success());
    assert_ne!(
        check.status.code(),
        Some(124),
        "protocol-binding check timed out under lock contention: {}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let stdout = String::from_utf8_lossy(&check.stdout);
    let check_stderr = String::from_utf8_lossy(&check.stderr);
    assert!(
        stdout.contains("invalid_compiled_bundle_agent_system_mode")
            || stdout.contains("protocol_binding_not_runtime_ready")
            || check_stderr.contains("LOCK is already locked")
            || check_stderr.contains("protocol-binding")
            || check_stderr.contains("Failed to")
            || check_stderr.contains("Invalid launcher activation snapshot")
            || check_stderr.contains("invalid launcher activation snapshot"),
        "expected fail-closed protocol-binding diagnostics\nstdout:\n{}\nstderr:\n{}",
        stdout,
        check_stderr
    );
}

#[test]
fn protocol_binding_check_lock_retry_preserves_boot_blocker_codes() {
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

    BOOT_PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER.store(0, Ordering::SeqCst);
    let check_output = run_with_state_lock_retry(|| {
        let attempt = BOOT_PROTOCOL_BINDING_LOCK_SIMULATION_COUNTER.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            support::simulated_state_lock_output()
        } else {
            let mut command = vida();
            command
                .args(["taskflow", "protocol-binding", "check", "--json"])
                .env("VIDA_STATE_DIR", &state_dir);
            command.output().expect("protocol-binding check should run")
        }
    });

    assert!(!check_output.status.success());
    let check_json: serde_json::Value = serde_json::from_slice(&check_output.stdout)
        .expect("protocol-binding check json should parse");
    let decision_blocker = check_json["decision_gate"]["blocker_code"]
        .as_str()
        .expect("decision gate blocker code should be present");
    let contract_blockers = check_json["operator_contracts"]["blocker_codes"]
        .as_array()
        .expect("operator_contracts.blocker_codes should be array");
    let shared_blockers = check_json["shared_fields"]["blocker_codes"]
        .as_array()
        .expect("shared_fields.blocker_codes should be array");
    assert_eq!(
        contract_blockers[0].as_str().unwrap(),
        decision_blocker,
        "operator_contracts.blocker_codes must mirror decision_gate blocker_code"
    );
    assert_eq!(
        shared_blockers[0].as_str().unwrap(),
        decision_blocker,
        "shared_fields.blocker_codes must mirror decision_gate blocker_code"
    );

    let doctor_output = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor_output.status.success());
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor_output.stdout).expect("doctor json should parse");
    let doctor_blockers = doctor_json["blocker_codes"]
        .as_array()
        .expect("doctor blocker codes should be array");
    assert!(
        doctor_blockers.iter().any(|code| {
            matches!(
                code.as_str(),
                Some("missing_retrieval_trust_operator_evidence")
                    | Some("missing_retrieval_trust_source_operator_evidence")
                    | Some("missing_retrieval_trust_signal_operator_evidence")
            )
        }),
        "doctor blocker codes should still report retrieval-trust evidence issues after lock retry"
    );
}

#[test]
fn taskflow_recovery_latest_reports_none_on_empty_booted_state() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let text_output = taskflow_recovery_latest_with_timeout(&state_dir, "latest", false);
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow recovery latest"));
    assert!(text_stdout.contains("recovery: none"));

    let json_output = taskflow_recovery_latest_with_timeout(&state_dir, "latest", true);
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("recovery latest json should parse");
    assert_eq!(parsed["surface"], "vida taskflow recovery latest");
    assert!(parsed["recovery"].is_null());
}

#[test]
fn taskflow_consume_bundle_renders_runtime_bundle_json() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = bounded_vida_output(
        &["15s"],
        "taskflow consume bundle json should run",
        |command| {
            command
                .args(["taskflow", "consume", "bundle", "--json"])
                .env("VIDA_STATE_DIR", &state_dir);
        },
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Authoritative state root is not project-bound"),
        "{stderr}"
    );
    assert!(
        stderr.contains("no DB-backed project root is available"),
        "{stderr}"
    );
}

#[test]
fn orchestrator_init_renders_compiled_startup_view_json() {
    let state_dir = unique_state_dir();

    let output = run_with_retry(|| {
        vida()
            .args(["orchestrator-init", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("orchestrator-init json should run")
    });
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Authoritative state root is not project-bound"),
        "{stderr}"
    );
}

#[test]
fn agent_init_renders_worker_startup_view_json_for_explicit_role() {
    let state_dir = unique_state_dir();

    let output = run_with_retry(|| {
        vida()
            .args(["agent-init", "--role", "worker", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("agent-init json should run")
    });
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Authoritative state root is not project-bound"),
        "{stderr}"
    );
}

#[test]
fn bootstrap_init_surfaces_report_installed_vs_source_launcher_parity() {
    let home_root = format!("{}/home", unique_state_dir());
    let local_vida = format!("{home_root}/.local/bin/vida");
    let cargo_vida = format!("{home_root}/.cargo/bin/vida");
    fs::create_dir_all(format!("{home_root}/.local/bin")).expect("local bin dir should exist");
    fs::create_dir_all(format!("{home_root}/.cargo/bin")).expect("cargo bin dir should exist");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &local_vida);
    copy_executable(env!("CARGO_BIN_EXE_vida"), &cargo_vida);

    for (project_id, project_name, args) in [
        (
            "launcher-parity-orchestrator-smoke",
            "Launcher Parity Orchestrator Smoke",
            vec!["orchestrator-init", "--json"],
        ),
        (
            "launcher-parity-agent-smoke",
            "Launcher Parity Agent Smoke",
            vec!["agent-init", "--role", "worker", "--json"],
        ),
    ] {
        let (project_root, state_dir) = bootstrap_project_runtime(project_id, project_name);
        let output = run_command_with_state_lock_retry(|| {
            let mut command = vida();
            command
                .args(&args)
                .current_dir(&project_root)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("HOME", &home_root)
                .env("VIDA_STATE_DIR", &state_dir);
            command
        });
        assert!(
            output.status.success(),
            "{} {:?}",
            String::from_utf8_lossy(&output.stderr),
            args
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("init surface json should parse");
        let launcher = &parsed["runtime_bundle_summary"]["launcher_runtime_paths"];
        assert_eq!(launcher["status"], "pass", "{args:?}");
        assert_eq!(launcher["next_actions"].as_array().map(Vec::len), Some(0));
        assert_eq!(launcher["divergent_installed_binaries"], false);
        assert_eq!(launcher["vida"], "vida");
        assert_eq!(launcher["taskflow_surface"], "vida taskflow");
        assert_eq!(launcher["project_root"], project_root);

        let active_fingerprint = launcher["active_executable_fingerprint"]
            .as_str()
            .expect("active executable fingerprint should render");
        assert!(!active_fingerprint.is_empty());
        let installed = launcher["installed_binaries"]
            .as_array()
            .expect("installed binary evidence should render");
        assert!(
            installed.len() >= 3,
            "HOME-installed copies and active source binary should be reported: {installed:?}"
        );
        let installed_paths = installed
            .iter()
            .map(|entry| {
                assert_eq!(entry["fingerprint"], active_fingerprint);
                entry["path"]
                    .as_str()
                    .expect("installed binary path should render")
                    .to_string()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(installed_paths.contains(&local_vida));
        assert!(installed_paths.contains(&cargo_vida));
        assert_eq!(
            installed
                .iter()
                .filter(|entry| entry["active"] == true)
                .count(),
            1,
            "exactly one active source-built launcher should be marked active"
        );
        fs::remove_dir_all(project_root).expect("temp root should be removed");
    }
}

#[test]
fn agent_init_dispatch_packet_reports_view_only_activation_semantics() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "agent-init-view-only-activation",
        "Agent Init View Only Activation",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");

    let output = vida()
        .args([
            "agent-init",
            "--dispatch-packet",
            dispatch_packet_path,
            "--json",
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("agent-init dispatch packet should run");
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("agent-init dispatch json should parse");
    assert_eq!(parsed["selection"]["mode"], "dispatch_packet");
    assert_eq!(parsed["activation_semantics"]["view_only"], true);
    assert_eq!(parsed["activation_semantics"]["executes_packet"], false);
    assert_eq!(
        parsed["activation_semantics"]["transfers_root_session_write_authority"],
        false
    );
    assert!(parsed["activation_semantics"]["next_lawful_action"]
        .as_str()
        .unwrap_or_default()
        .contains("receipt-backed evidence"));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn agent_init_parallel_role_views_do_not_surface_eagain_lock_failures() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "agent-init-parallel-read-lock",
        "Agent Init Parallel Read Lock",
    );
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = Vec::new();

    for _ in 0..4 {
        let barrier = Arc::clone(&barrier);
        let project_root = project_root.clone();
        let state_dir = state_dir.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            vida()
                .args(["agent-init", "--role", "worker", "--json"])
                .current_dir(&project_root)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("parallel agent-init json should run")
        }));
    }

    let outputs = handles
        .into_iter()
        .map(|handle| handle.join().expect("parallel agent-init should join"))
        .collect::<Vec<_>>();

    for (index, output) in outputs.iter().enumerate() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("os error 11"),
            "parallel agent-init invocation {index} surfaced os error 11: {stderr}"
        );
        assert!(
            !stderr.contains("Resource temporarily unavailable"),
            "parallel agent-init invocation {index} surfaced EAGAIN text: {stderr}"
        );
        assert!(
            !is_state_lock_error(output),
            "parallel agent-init invocation {index} surfaced a datastore lock error: {stderr}"
        );
        assert!(
            output.status.success(),
            "parallel agent-init invocation {index} failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            stderr
        );

        let parsed: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("parallel agent-init json should parse");
        assert_eq!(parsed["selected_role"], "worker");
        assert!(
            parsed["runtime_bundle_summary"].is_object(),
            "parallel agent-init invocation {index} should render runtime bundle summary"
        );
    }

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_bundle_check_reports_ready_runtime_bundle() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(sync.status.success());

    let output = run_with_retry_until(
        || {
            vida()
                .args(["taskflow", "consume", "bundle", "check", "--json"])
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("taskflow consume bundle check json should run")
        },
        |candidate| !candidate.stdout.is_empty(),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Authoritative state root is not project-bound")
                || stderr.contains("no DB-backed project root is available"),
            "unexpected stderr: {stderr}"
        );
        return;
    }
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume bundle check json should parse");
    assert_eq!(parsed["surface"], "vida taskflow consume bundle check");
    assert_eq!(parsed["check"]["ok"], true);
    assert_eq!(
        parsed["check"]["root_artifact_id"],
        "framework-agent-definition"
    );
    assert_eq!(
        parsed["check"]["boot_classification"],
        "backward_compatible"
    );
    assert_eq!(parsed["check"]["migration_state"], "no_migration_required");
    let check_activation_status = parsed["check"]["activation_status"]
        .as_str()
        .expect("check activation status should be string");
    assert!(matches!(
        check_activation_status,
        "pending" | "ready_enough_for_normal_work"
    ));
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["receipt_backed"],
        true
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["status"],
        "pass"
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["closure_inputs_ready"],
        true
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["docflow_status"],
        "pass"
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["blocker_codes"],
        serde_json::json!([])
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["docflow_blocker_codes"],
        serde_json::json!([])
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["has_readiness_surface"],
        true
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["has_proof_surface"],
        true
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["receipt_evidence"]["receipt_backed"],
        true
    );
    let seam_docflow_surfaces = parsed["seam_closure_admission_receipt_check"]
        ["docflow_proof_surfaces"]
        .as_array()
        .expect("seam docflow proof surfaces should be an array");
    assert!(seam_docflow_surfaces
        .iter()
        .filter_map(serde_json::Value::as_str)
        .any(|value| value.contains("readiness-check")));
    assert!(seam_docflow_surfaces
        .iter()
        .filter_map(serde_json::Value::as_str)
        .any(|value| value.contains("proofcheck")));
    assert!(
        parsed["seam_closure_admission_receipt_check"]["total_receipts"]
            .as_u64()
            .expect("seam closure total_receipts should be numeric")
            > 0
    );
    assert_eq!(parsed["db_first_activation_truth"]["ok"], true);
    assert_eq!(parsed["db_first_activation_truth"]["source"], "state_store");
    let blockers = parsed["check"]["blockers"]
        .as_array()
        .expect("blockers should be an array");
    assert!(blockers.is_empty());
    assert_eq!(parsed["effective_blockers"], parsed["check"]["blockers"]);
    let snapshot_path = parsed["snapshot_path"]
        .as_str()
        .expect("consume bundle check should report snapshot path");
    assert!(std::path::Path::new(snapshot_path).is_file());
}

#[test]
fn taskflow_consume_bundle_check_fails_closed_when_init_db_first_source_not_authoritative() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(sync.status.success());

    overwrite_launcher_activation_snapshot_with_source(
        &state_dir,
        "script_runtime",
        serde_json::json!({
            "role_selection": {
                "mode": "auto",
                "fallback_role": "orchestrator"
            },
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only"
            }
        }),
    );

    let output = taskflow_consume_bundle_check_with_timeout(&state_dir);
    assert!(!output.status.success());
    assert_ne!(
        output.status.code(),
        Some(124),
        "taskflow consume bundle check timed out under lock contention: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("authoritative launcher activation source must be `state_store`")
                || stderr.contains(
                    "opening authoritative state store timed out while waiting for authoritative datastore lock"
                )
                || stderr.contains("LOCK is already locked"),
            "{stderr}"
        );
    } else {
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("consume bundle check json should parse");
        assert_eq!(parsed["check"]["ok"], false);
        assert_eq!(parsed["db_first_activation_truth"]["ok"], false);
        assert_eq!(
            parsed["db_first_activation_truth"]["source"],
            "script_runtime"
        );
        let error = parsed["db_first_activation_truth"]["error"]
            .as_str()
            .expect("db-first activation truth error should be present");
        assert!(error.contains("authoritative launcher activation source must be `state_store`"));
        assert!(parsed["effective_blockers"].as_array().is_some_and(|rows| {
            rows.iter()
                .any(|value| value == "missing_launcher_activation_snapshot")
        }));
    }
}

#[test]
fn taskflow_consume_bundle_check_fails_fast_under_state_lock_contention() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(sync.status.success());

    let held_state_lock = StateStoreLockGuard::acquire(&state_dir);

    let output = taskflow_consume_bundle_check_with_timeout(&state_dir);
    assert!(
        !output.status.success(),
        "consume bundle check should fail while the state store lock is held"
    );
    assert_ne!(
        output.status.code(),
        Some(124),
        "consume bundle check timed out instead of failing fast: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LOCK is already locked")
            || stderr.contains("timed out while waiting for authoritative datastore lock"),
        "expected lock contention to be reported, got stderr={stderr}"
    );
    assert!(
        stderr.contains("another VIDA process still holds the authoritative datastore lock"),
        "expected explicit lock remediation hint, got stderr={stderr}"
    );

    drop(held_state_lock);
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn status_and_doctor_fail_closed_with_lock_remediation_hint() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let held_state_lock = StateStoreLockGuard::acquire(&state_dir);
    let expected_hint = "another VIDA process still holds the authoritative datastore lock";

    for args in [["status", "--json"], ["doctor", "--json"]] {
        let output = status_or_doctor_with_timeout(&state_dir, &args);
        assert!(
            !output.status.success(),
            "{} should fail while the state store lock is held",
            args[0]
        );
        assert_ne!(
            output.status.code(),
            Some(124),
            "{} timed out instead of failing fast: stdout={} stderr={}",
            args[0],
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Failed to open authoritative state store:")
                && stderr.contains(expected_hint),
            "expected {} lock remediation hint in stderr, got {stderr}",
            args[0]
        );
    }

    drop(held_state_lock);
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn status_and_doctor_text_surfaces_fail_closed_with_lock_remediation_hint() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let held_state_lock = StateStoreLockGuard::acquire(&state_dir);
    let expected_hint = "another VIDA process still holds the authoritative datastore lock";

    for args in [["status"], ["doctor"]] {
        let output = status_or_doctor_with_timeout(&state_dir, &args);
        assert!(
            !output.status.success(),
            "{} should fail while the state store lock is held",
            args[0]
        );
        assert_ne!(
            output.status.code(),
            Some(124),
            "{} timed out instead of failing fast: stdout={} stderr={}",
            args[0],
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Failed to open authoritative state store:")
                && stderr.contains(expected_hint),
            "expected {} text-surface lock remediation hint in stderr, got {stderr}",
            args[0]
        );
    }

    drop(held_state_lock);
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn parallel_read_only_task_surfaces_do_not_fail_on_state_lock_contention() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let root = vida()
        .args([
            "task",
            "ensure",
            "parallel-root",
            "Parallel Root",
            "--description",
            "root for parallel read-only task surfaces",
            "--status",
            "open",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("root task ensure should run");
    assert!(
        root.status.success(),
        "{}",
        String::from_utf8_lossy(&root.stderr)
    );

    let child = vida()
        .args([
            "task",
            "ensure",
            "parallel-child",
            "Parallel Child",
            "--parent-id",
            "parallel-root",
            "--description",
            "ready child for scoped next",
            "--status",
            "open",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("child task ensure should run");
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );

    let state_dir_for_children = state_dir.clone();
    let children_handle = std::thread::spawn(move || {
        vida()
            .args(["task", "children", "parallel-root", "--json"])
            .env("VIDA_STATE_DIR", &state_dir_for_children)
            .output()
            .expect("task children should run")
    });

    let state_dir_for_next = state_dir.clone();
    let next_handle = std::thread::spawn(move || {
        vida()
            .args(["task", "next", "--scope", "parallel-root", "--json"])
            .env("VIDA_STATE_DIR", &state_dir_for_next)
            .output()
            .expect("task next should run")
    });

    let children_output = children_handle.join().expect("children thread should join");
    let next_output = next_handle.join().expect("next thread should join");

    assert!(
        children_output.status.success(),
        "task children should not fail under concurrent read-only load: stdout={} stderr={}",
        String::from_utf8_lossy(&children_output.stdout),
        String::from_utf8_lossy(&children_output.stderr)
    );
    assert!(
        next_output.status.success(),
        "task next should not fail under concurrent read-only load: stdout={} stderr={}",
        String::from_utf8_lossy(&next_output.stdout),
        String::from_utf8_lossy(&next_output.stderr)
    );
    assert!(
        !is_state_lock_error(&children_output),
        "task children should not surface lock contention: {}",
        String::from_utf8_lossy(&children_output.stderr)
    );
    assert!(
        !is_state_lock_error(&next_output),
        "task next should not surface lock contention: {}",
        String::from_utf8_lossy(&next_output.stderr)
    );

    let children_json: serde_json::Value =
        serde_json::from_slice(&children_output.stdout).expect("task children json should parse");
    let next_json: serde_json::Value =
        serde_json::from_slice(&next_output.stdout).expect("task next json should parse");
    assert_eq!(children_json["child_count"], 1);
    assert_eq!(children_json["children"][0]["child_id"], "parallel-child");
    assert!(
        next_json["ready_count"]
            .as_u64()
            .expect("task next ready_count should be numeric")
            >= 1,
        "scoped next should remain machine-readable under concurrent read-only access: {next_json}"
    );

    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_consume_bundle_check_fails_closed_without_protocol_binding_receipt() {
    let project_root = unique_state_dir();
    let state_dir = format!("{project_root}/.vida/data/state");
    fs::create_dir_all(&state_dir).expect("state dir should exist");
    scaffold_runtime_project_root(&project_root, "project");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "consume", "bundle", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow consume bundle check json should run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Authoritative state root is not project-bound")
                || stderr.contains("no DB-backed project root is available"),
            "unexpected stderr: {stderr}"
        );
        return;
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume bundle check json should parse");
    assert_eq!(parsed["check"]["ok"], false);
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["receipt_backed"],
        false
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["status"],
        "pass"
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["closure_inputs_ready"],
        true
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["docflow_status"],
        "pass"
    );
    assert_eq!(
        parsed["seam_closure_admission_receipt_check"]["receipt_evidence"]["receipt_backed"],
        true
    );
    let blockers = parsed["check"]["blockers"]
        .as_array()
        .expect("blockers should be an array");
    assert!(blockers
        .iter()
        .any(|value| value == "missing_protocol_binding_receipt"));
    assert!(blockers
        .iter()
        .any(|value| value == "protocol_binding_not_runtime_ready"));
}

fn canonical_activation_status(status: &str, activation_pending: bool) -> &'static str {
    let normalized = status.trim().to_ascii_lowercase();
    if activation_pending || normalized == "pending" || normalized == "pending_activation" {
        "pending"
    } else {
        "ready_enough_for_normal_work"
    }
}

fn assert_project_activation_status_is_canonical(status_json: &serde_json::Value, label: &str) {
    let activation_pending = status_json["project_activation"]["activation_pending"]
        .as_bool()
        .unwrap_or(false);
    let status_surface_activation_status = status_json["project_activation"]["status"]
        .as_str()
        .unwrap_or("ready_enough_for_normal_work");
    assert_eq!(
        status_json["project_activation"]["status"],
        canonical_activation_status(status_surface_activation_status, activation_pending),
        "{} project activation status must stay canonical",
        label
    );
}

fn assert_canonical_activation_status_value(value: &serde_json::Value, label: &str) {
    let status = value
        .as_str()
        .unwrap_or_else(|| panic!("{label} field must be a string"));
    assert!(
        status == "pending" || status == "ready_enough_for_normal_work",
        "{label} must stay within canonical activation vocabulary, got `{status}`"
    );
}

#[test]
fn status_and_consume_bundle_check_handle_legacy_pending_activation() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let receipts_dir = std::path::Path::new(&state_dir).join(".vida/receipts");
    std::fs::create_dir_all(&receipts_dir).expect("receipts directory should exist");
    let receipt = serde_json::json!({
        "surface": "vida project-activator",
        "project_root": state_dir,
        "status": "pending_activation",
        "activation_pending": true,
        "project_shape": {},
        "triggers": {},
        "activation_algorithm": {},
        "normal_work_defaults": {},
        "interview": {
            "one_shot_example": "vida docflow check --profile active-canon",
            "required_inputs": []
        },
        "host_environment": {},
        "next_steps": []
    });
    std::fs::write(
        receipts_dir.join("project-activation.latest.json"),
        receipt.to_string(),
    )
    .expect("activation receipt should be written");

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_ROOT", &state_dir)
        .output()
        .expect("status should run");
    assert!(status.status.success());
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_project_activation_status_is_canonical(
        &status_json,
        "status and consume bundle check pending activation",
    );

    let check = vida()
        .args(["taskflow", "consume", "bundle", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_ROOT", &state_dir)
        .output()
        .expect("taskflow consume bundle check should run");
    assert!(!check.status.success());
    assert!(check.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&check.stderr);
    assert!(
        stderr.contains("Authoritative state root is not project-bound"),
        "{stderr}"
    );
}

#[test]
fn explicit_root_and_state_dirs_keep_activation_status_canonical_through_status_and_consume_check()
{
    let root_dir = unique_state_dir();
    let state_dir = unique_state_dir();
    fs::create_dir_all(&root_dir).expect("create root dir");
    fs::create_dir_all(&state_dir).expect("create state dir");

    let boot = vida()
        .arg("boot")
        .env("VIDA_ROOT", &root_dir)
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let status = vida()
        .args(["status", "--json"])
        .env("VIDA_ROOT", &root_dir)
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(status.status.success());
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_eq!(status_json["surface"], "vida status");
    let activation = &status_json["project_activation"];
    assert!(activation.is_object());
    let activation_pending = activation["activation_pending"]
        .as_bool()
        .expect("project activation pending flag should be boolean");
    let activation_status = activation["status"]
        .as_str()
        .expect("project activation status should be present");
    assert_eq!(
        activation["status"],
        canonical_activation_status(activation_status, activation_pending)
    );
    assert_canonical_activation_status_value(
        &activation["status"],
        "status.project_activation.status",
    );

    let check = vida()
        .args(["taskflow", "consume", "bundle", "check", "--json"])
        .env("VIDA_ROOT", &root_dir)
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume bundle check should run");
    assert!(!check.status.success());
    assert!(check.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&check.stderr);
    assert!(
        stderr.contains("Authoritative state root is not project-bound"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(&root_dir);
    let _ = fs::remove_dir_all(&state_dir);
}

#[test]
fn taskflow_consume_final_renders_direct_runtime_consumption_snapshot() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("consume-final-direct", "Consume Final Direct");

    let output = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final json should parse");
    assert_eq!(parsed["surface"], "vida taskflow consume final");
    assert_eq!(
        parsed["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
    );
    assert_eq!(parsed["status"], parsed["operator_contracts"]["status"]);
    assert_eq!(parsed["payload"]["closure_authority"], "taskflow");
    assert_eq!(parsed["payload"]["request_text"], "probe closure");
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_surface"],
        "vida agent-init"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_command"],
        "vida agent-init"
    );
    let dispatch_packet_path = parsed["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");
    assert!(std::path::Path::new(dispatch_packet_path).is_file());
    let dispatch_packet_body =
        std::fs::read_to_string(dispatch_packet_path).expect("dispatch packet should be readable");
    let dispatch_packet_json: serde_json::Value =
        serde_json::from_str(&dispatch_packet_body).expect("dispatch packet json should parse");
    assert_eq!(
        dispatch_packet_json["packet_template_kind"],
        "delivery_task_packet"
    );
    assert!(dispatch_packet_json["dispatch_command"]
        .as_str()
        .expect("dispatch packet command should be present")
        .starts_with("vida agent-init --dispatch-packet "));
    let snapshot_path = parsed["snapshot_path"]
        .as_str()
        .expect("consume final should report snapshot path");
    assert!(std::path::Path::new(snapshot_path).is_file());
    assert_eq!(
        parsed["failure_control_evidence"]["rollback"]["source_run_id"],
        parsed["payload"]["dispatch_receipt"]["run_id"]
    );
    assert_eq!(
        parsed["failure_control_evidence"]["rollback"]["source_dispatch_packet_path"],
        dispatch_packet_path
    );
    let snapshot_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(snapshot_path).expect("snapshot should be readable"),
    )
    .expect("snapshot json should parse");
    assert_eq!(
        snapshot_json["failure_control_evidence"]["rollback"]["source_run_id"],
        parsed["payload"]["dispatch_receipt"]["run_id"]
    );
    assert_eq!(
        snapshot_json["failure_control_evidence"]["rollback"]["source_dispatch_packet_path"],
        dispatch_packet_path
    );

    let status = status_or_doctor_with_timeout(&state_dir, &["status", "--json"]);
    assert!(status.status.success());
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    let status_json: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status json should parse");
    assert!(
        status_json["runtime_consumption"]["final_snapshots"]
            .as_u64()
            .expect("final snapshot count should be numeric")
            >= 1
    );
    assert_eq!(status_json["runtime_consumption"]["latest_kind"], "final");
    assert_eq!(
        status_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        snapshot_path
    );

    let doctor = status_or_doctor_with_timeout(&state_dir, &["doctor", "--json"]);
    assert!(doctor.status.success());
    let doctor_stdout = String::from_utf8_lossy(&doctor.stdout);
    let doctor_json: serde_json::Value =
        serde_json::from_str(&doctor_stdout).expect("doctor json should parse");
    assert!(
        doctor_json["runtime_consumption"]["final_snapshots"]
            .as_u64()
            .expect("final snapshot count should be numeric")
            >= 1
    );
    assert_eq!(doctor_json["runtime_consumption"]["latest_kind"], "final");
    assert_eq!(
        doctor_json["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        snapshot_path
    );
}

#[test]
fn taskflow_consume_final_executes_ready_downstream_closure_step() {
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
            "closure-test",
            "--project-name",
            "Closure Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let config_path = format!("{project_root}/vida.config.yaml");
    let config_body = fs::read_to_string(&config_path).expect("config should read");
    let config_body = config_body
        .replace("coach_required: yes", "coach_required: no")
        .replace(
            "independent_verification_required: yes",
            "independent_verification_required: no",
        );
    atomic_write_file(&config_path, &config_body);

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
    let sync = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );

    let output = vida()
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    assert!(
        !output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final json should parse");
    assert_eq!(parsed["payload"]["direct_consumption_ready"], false);
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_status"],
        "blocked"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_target"],
        "closure"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_ready"],
        true
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_status"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_executed_count"],
        0
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_last_target"],
        serde_json::Value::Null
    );
    assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_result_path"].is_null());
    assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_trace_path"].is_null());
    if let Some(downstream_packet_path) =
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_packet_path"].as_str()
    {
        assert!(std::path::Path::new(downstream_packet_path).is_file());
    }

    let status_output = vida()
        .args(["status", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(
        status_output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&status_output.stdout),
        String::from_utf8_lossy(&status_output.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status_output.stdout).expect("status json should parse");
    assert_eq!(
        status_json["latest_run_graph_dispatch_receipt"]["downstream_dispatch_status"],
        serde_json::Value::Null
    );
    assert!(
        status_json["latest_run_graph_dispatch_receipt"]["downstream_dispatch_result_path"]
            .is_null()
    );
    assert!(
        status_json["latest_run_graph_dispatch_receipt"]["downstream_dispatch_executed_count"]
            .as_u64()
            .unwrap_or(0)
            == 0
    );
    assert_eq!(
        status_json["latest_run_graph_dispatch_receipt"]["downstream_dispatch_last_target"],
        serde_json::Value::Null
    );
    assert_eq!(
        status_json["latest_run_graph_status"]["active_node"],
        "orchestrator"
    );
    assert_eq!(status_json["latest_run_graph_status"]["status"], "blocked");

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_resumes_from_persisted_final_snapshot() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("continue-from-final", "Continue From Final");
    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");
    let dispatch_packet_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dispatch_packet_path).expect("dispatch packet should read"),
    )
    .expect("dispatch packet should parse");
    let source_run_id = dispatch_packet_json["run_id"]
        .as_str()
        .expect("dispatch packet run_id should be present");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");
    let snapshot_path = initial_json["snapshot_path"]
        .as_str()
        .expect("snapshot path should be present");
    let mut downstream_packet_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(false);
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream dispatch packet should render"),
    );
    let mut snapshot_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(snapshot_path).expect("snapshot body should read"),
    )
    .expect("snapshot body should parse");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_target"] =
        serde_json::json!("corrupted-from-final-snapshot");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_status"] = serde_json::json!("blocked");
    snapshot_body["payload"]["role_selection"]["selected_role"] =
        serde_json::json!("corrupted-role");
    atomic_write_file(
        snapshot_path,
        &serde_json::to_string_pretty(&snapshot_body).expect("mutated snapshot should render"),
    );

    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &["--json"],
    );
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_json: serde_json::Value =
        serde_json::from_slice(&resumed.stdout).expect("consume continue json should parse");
    assert_eq!(resumed_json["surface"], "vida taskflow consume continue");
    assert_eq!(resumed_json["source_run_id"], source_run_id);
    assert_eq!(
        resumed_json["source_dispatch_packet_path"],
        dispatch_packet_path
    );
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_accepts_explicit_dispatch_packet_path() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("continue-explicit-packet", "Continue Explicit Packet");
    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");
    let dispatch_packet_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dispatch_packet_path).expect("dispatch packet should read"),
    )
    .expect("dispatch packet should parse");
    let run_id = dispatch_packet_json["run_id"]
        .as_str()
        .expect("dispatch packet run_id should be present");
    let initial_dispatch_target = initial_json["payload"]["dispatch_receipt"]["dispatch_target"]
        .as_str()
        .expect("initial dispatch target should be present")
        .to_string();
    let initial_dispatch_status = initial_json["payload"]["dispatch_receipt"]["dispatch_status"]
        .as_str()
        .expect("initial dispatch status should be present")
        .to_string();
    let snapshot_path = initial_json["snapshot_path"]
        .as_str()
        .expect("snapshot path should be present");
    let mut snapshot_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(snapshot_path).expect("snapshot body should read"),
    )
    .expect("snapshot body should parse");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_target"] =
        serde_json::json!("corrupted-from-final-snapshot");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_status"] = serde_json::json!("blocked");
    snapshot_body["payload"]["role_selection"]["selected_role"] =
        serde_json::json!("corrupted-role");
    atomic_write_file(
        snapshot_path,
        &serde_json::to_string_pretty(&snapshot_body).expect("mutated snapshot should render"),
    );

    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &["--dispatch-packet", dispatch_packet_path, "--json"],
    );
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_json: serde_json::Value =
        serde_json::from_slice(&resumed.stdout).expect("consume continue json should parse");
    assert_eq!(resumed_json["surface"], "vida taskflow consume continue");
    assert_eq!(resumed_json["source_run_id"], run_id);
    assert_eq!(
        resumed_json["source_dispatch_packet_path"],
        dispatch_packet_path
    );
    assert_eq!(
        resumed_json["dispatch_receipt"]["dispatch_target"],
        initial_dispatch_target
    );
    assert_eq!(
        resumed_json["dispatch_receipt"]["dispatch_status"],
        initial_dispatch_status
    );
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn agent_init_fails_closed_for_dispatch_packet_missing_template_required_fields() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("dispatch-packet-validation", "Dispatch Packet Validation");

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "fix dispatch packet validation",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .map(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                PathBuf::from(&project_root).join(path)
            }
        })
        .expect("dispatch packet path should be present");
    let mut dispatch_packet_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&dispatch_packet_path).expect("dispatch packet should read"),
    )
    .expect("dispatch packet should parse");
    dispatch_packet_json["delivery_task_packet"]["goal"] = serde_json::Value::Null;
    if let Some(parent) = dispatch_packet_path.parent() {
        fs::create_dir_all(parent).expect("dispatch packet parent should exist");
    }
    fs::write(
        &dispatch_packet_path,
        serde_json::to_string_pretty(&dispatch_packet_json)
            .expect("mutated dispatch packet should render"),
    )
    .expect("mutated dispatch packet should be written");

    let output = vida()
        .args([
            "agent-init",
            "--dispatch-packet",
            dispatch_packet_path
                .to_str()
                .expect("dispatch packet path should be utf-8"),
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("agent-init should run");
    assert!(!output.status.success(), "agent-init should fail closed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required packet fields"));
    assert!(stderr.contains("goal"));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_prefers_latest_final_snapshot_after_bundle_check() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-prefers-latest-final",
        "Continue Prefers Latest Final",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let source_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present")
        .to_string();
    let source_dispatch_packet_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&source_dispatch_packet_path).expect("dispatch packet should read"),
    )
    .expect("dispatch packet should parse");
    let expected_resume_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .unwrap_or(&source_dispatch_packet_path)
        .to_string();
    let source_run_id = source_dispatch_packet_json["run_id"]
        .as_str()
        .expect("dispatch packet run_id should be present")
        .to_string();
    let initial_snapshot_path = initial_json["snapshot_path"]
        .as_str()
        .expect("snapshot path should be present");
    mark_project_run_graph_closure_complete(&project_root, &state_dir, &source_run_id);
    let mut snapshot_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(initial_snapshot_path).expect("snapshot body should read"),
    )
    .expect("snapshot body should parse");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_target"] =
        serde_json::json!("corrupted-from-final-snapshot");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_status"] = serde_json::json!("blocked");
    snapshot_body["payload"]["role_selection"]["selected_role"] =
        serde_json::json!("corrupted-role");
    atomic_write_file(
        initial_snapshot_path,
        &serde_json::to_string_pretty(&snapshot_body).expect("mutated snapshot should render"),
    );

    let check = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["taskflow", "consume", "bundle", "check", "--json"])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        !check.stdout.is_empty(),
        "status={:?}\nstderr:\n{}",
        check.status.code(),
        String::from_utf8_lossy(&check.stderr)
    );
    let check_json: serde_json::Value =
        serde_json::from_slice(&check.stdout).expect("bundle check json should parse");
    assert_eq!(check_json["surface"], "vida taskflow consume bundle check");
    assert_eq!(check_json["operator_contracts"]["status"], "blocked");
    assert_eq!(
        check_json["blocker_codes"],
        serde_json::json!([
            "invalid_cache_key_input:startup_bundle_revision",
            "invalid_invalidation_tuple_key:startup_bundle_revision"
        ])
    );
    assert!(check_json["snapshot_path"].as_str().is_some());

    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &["--json"],
    );
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_json: serde_json::Value =
        serde_json::from_slice(&resumed.stdout).expect("consume continue json should parse");
    assert_eq!(resumed_json["surface"], "vida taskflow consume continue");
    assert_eq!(resumed_json["status"], "pass");
    let resumed_snapshot_path = resumed_json["snapshot_path"]
        .as_str()
        .expect("resume snapshot path should be present");
    assert_eq!(resumed_json["source_run_id"], source_run_id);
    let resumed_source_dispatch_packet_path = resumed_json["source_dispatch_packet_path"]
        .as_str()
        .expect("resume source dispatch packet path should be present");
    assert!(
        resumed_source_dispatch_packet_path == source_dispatch_packet_path
            || resumed_source_dispatch_packet_path == expected_resume_packet_path
            || resumed_source_dispatch_packet_path.contains("/downstream-dispatch-packets/"),
        "unexpected resume packet path: {resumed_source_dispatch_packet_path}"
    );

    let status = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["status", "--json"])
            .current_dir(&project_root)
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(status.status.success());
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_eq!(status_json["runtime_consumption"]["latest_kind"], "final");
    assert_ne!(resumed_snapshot_path, initial_snapshot_path);

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_accepts_explicit_run_id() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("continue-explicit-run-id", "Continue Explicit Run Id");

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let run_id = initial_json["payload"]["dispatch_receipt"]["run_id"]
        .as_str()
        .expect("run id should be present");

    let resumed = project_bound_taskflow_consume_continue_once_with_timeout(
        &project_root,
        &state_dir,
        &["--run-id", run_id, "--json"],
    );
    assert!(
        !resumed.status.success(),
        "consume continue should fail closed"
    );
    let stderr = String::from_utf8_lossy(&resumed.stderr);
    assert!(stderr.contains("Run-graph resume gate denied"));
    assert!(stderr.contains(run_id));
    assert!(stderr.contains("recovery_ready is false"));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_accepts_explicit_downstream_packet_path() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-explicit-downstream-packet",
        "Continue Explicit Downstream Packet",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "clarify the scope and write the specification before implementation",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream or dispatch packet path should be present");
    let downstream_dispatch_packet_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    let run_id = downstream_dispatch_packet_json["run_id"]
        .as_str()
        .expect("downstream dispatch packet run_id should be present");
    let mut downstream_packet_body: serde_json::Value = downstream_dispatch_packet_json.clone();
    let completion_result_path = format!("{project_root}/runtime-completion-result-1.json");
    write_runtime_lane_completion_result_fixture(&completion_result_path, run_id, "implementer");
    downstream_packet_body["downstream_dispatch_target"] = serde_json::json!("business_analyst");
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(true);
    downstream_packet_body["downstream_dispatch_blockers"] = serde_json::json!([]);
    downstream_packet_body["downstream_dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["downstream_dispatch_result_path"] =
        serde_json::json!(completion_result_path);
    downstream_packet_body["downstream_lane_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["lane_status"] = serde_json::json!("packet_ready");
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream dispatch packet should render"),
    );
    let snapshot_path = initial_json["snapshot_path"]
        .as_str()
        .expect("snapshot path should be present");
    mark_project_run_graph_closure_complete(&project_root, &state_dir, run_id);
    let mut snapshot_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(snapshot_path).expect("snapshot body should read"),
    )
    .expect("snapshot body should parse");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_target"] =
        serde_json::json!("corrupted-from-final-snapshot");
    snapshot_body["payload"]["dispatch_receipt"]["dispatch_status"] = serde_json::json!("blocked");
    snapshot_body["payload"]["role_selection"]["selected_role"] =
        serde_json::json!("corrupted-role");
    atomic_write_file(
        snapshot_path,
        &serde_json::to_string_pretty(&snapshot_body).expect("mutated snapshot should render"),
    );
    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &[
            "--downstream-packet",
            downstream_dispatch_packet_path,
            "--json",
        ],
    );
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_json: serde_json::Value =
        serde_json::from_slice(&resumed.stdout).expect("consume continue json should parse");
    assert_eq!(resumed_json["surface"], "vida taskflow consume continue");
    assert_eq!(resumed_json["source_run_id"], run_id);
    assert_eq!(
        resumed_json["source_dispatch_packet_path"],
        downstream_dispatch_packet_path
    );
    assert!(resumed_json["dispatch_receipt"]["dispatch_target"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(resumed_json["dispatch_receipt"]["dispatch_status"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_rejects_executed_completed_downstream_packet_when_recovery_is_not_ready(
) {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-executed-completed-semantics",
        "Continue Executed Completed Semantics",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");
    let mut downstream_packet_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(false);
    downstream_packet_body["downstream_dispatch_status"] = serde_json::json!("executed");
    downstream_packet_body["downstream_lane_status"] = serde_json::json!("lane_completed");
    downstream_packet_body["downstream_dispatch_blockers"] = serde_json::json!([]);
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream packet should render"),
    );

    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &[
            "--downstream-packet",
            downstream_dispatch_packet_path,
            "--json",
        ],
    );
    assert!(!resumed.status.success());
    let stderr = String::from_utf8_lossy(&resumed.stderr);
    assert!(stderr.contains("Run-graph resume gate denied"), "{stderr}");
    assert!(stderr.contains("recovery_ready is false"), "{stderr}");
}

#[test]
fn taskflow_consume_continue_rejects_packet_ready_downstream_packet_when_recovery_is_not_ready() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-packet-ready-semantics",
        "Continue Packet Ready Semantics",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe packet ready",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");
    let mut downstream_packet_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    let completion_result_path = format!("{project_root}/runtime-completion-result-2.json");
    write_runtime_lane_completion_result_fixture(
        &completion_result_path,
        initial_json["payload"]["dispatch_receipt"]["run_id"]
            .as_str()
            .unwrap_or("packet-ready-semantics"),
        "implementer",
    );
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(false);
    downstream_packet_body["downstream_dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["downstream_dispatch_result_path"] =
        serde_json::json!(completion_result_path);
    downstream_packet_body["downstream_lane_status"] = serde_json::json!("lane_running");
    downstream_packet_body["downstream_dispatch_blockers"] = serde_json::json!([]);
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream packet should render"),
    );

    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &[
            "--downstream-packet",
            downstream_dispatch_packet_path,
            "--json",
        ],
    );
    assert!(!resumed.status.success());
    let stderr = String::from_utf8_lossy(&resumed.stderr);
    assert!(stderr.contains("Run-graph resume gate denied"), "{stderr}");
    assert!(stderr.contains("recovery_ready is false"), "{stderr}");
}

#[test]
fn taskflow_consume_continue_rejects_mismatched_run_id_and_dispatch_packet() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-mismatched-run-id-dispatch",
        "Continue Mismatched Run Id Dispatch",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");

    let resumed = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "consume",
                "continue",
                "--run-id",
                "mismatched-run-id",
                "--dispatch-packet",
                dispatch_packet_path,
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(!resumed.status.success());
    let stderr = String::from_utf8_lossy(&resumed.stderr);
    assert!(
        stderr.contains("does not match persisted dispatch packet run_id"),
        "{stderr}"
    );
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_rejects_mismatched_run_id_and_downstream_packet() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-mismatched-run-id-downstream",
        "Continue Mismatched Run Id Downstream",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");

    let resumed = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "consume",
                "continue",
                "--run-id",
                "mismatched-run-id",
                "--downstream-packet",
                downstream_dispatch_packet_path,
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(!resumed.status.success());
    let stderr = String::from_utf8_lossy(&resumed.stderr);
    assert!(
        stderr.contains("does not match persisted downstream dispatch packet run_id"),
        "{stderr}"
    );
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_auto_picks_ready_downstream_packet() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "continue-auto-picks-ready-downstream",
        "Continue Auto Picks Ready Downstream",
    );

    let initial = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "probe closure",
    );
    assert!(
        !initial.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");
    let mut downstream_packet_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    let run_id = downstream_packet_body["run_id"]
        .as_str()
        .expect("downstream dispatch packet run id should be present");
    mark_project_run_graph_closure_complete(&project_root, &state_dir, run_id);
    let completion_result_path = format!("{project_root}/runtime-completion-result-3.json");
    write_runtime_lane_completion_result_fixture(&completion_result_path, run_id, "implementer");
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(true);
    downstream_packet_body["downstream_dispatch_blockers"] = serde_json::json!([]);
    downstream_packet_body["downstream_dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["downstream_dispatch_result_path"] =
        serde_json::json!(completion_result_path);
    downstream_packet_body["downstream_lane_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["lane_status"] = serde_json::json!("packet_ready");
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream packet should render"),
    );

    let resumed = project_bound_taskflow_consume_continue_with_timeout(
        &project_root,
        &state_dir,
        &["--json"],
    );
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_json: serde_json::Value =
        serde_json::from_slice(&resumed.stdout).expect("consume continue json should parse");
    assert_eq!(resumed_json["surface"], "vida taskflow consume continue");
    assert_eq!(
        resumed_json["source_dispatch_packet_path"],
        downstream_dispatch_packet_path
    );
    assert_eq!(
        resumed_json["dispatch_receipt"]["dispatch_target"],
        "closure"
    );
    assert!(resumed_json["dispatch_receipt"]["dispatch_status"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_continue_auto_executes_ready_downstream_taskflow_packet() {
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
            "downstream-auto-exec",
            "--project-name",
            "Downstream Auto Exec",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );
    let activator_json: serde_json::Value =
        serde_json::from_slice(&activator.stdout).expect("project activator should render json");
    assert_eq!(
        activator_json["activation_log"]["db_first_activation_truth"]["source"],
        "state_store"
    );
    assert!(
        activator_json["activation_log"]["db_first_activation_truth"]["source_config_digest"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(
        activator_json["activation_log"]["db_first_activation_truth"]["read_back_verified"],
        true
    );

    let boot = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .arg("boot")
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    sync_protocol_binding(&state_dir);

    let initial = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "consume",
                "final",
                "clarify the scope and write the specification before implementation",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(!initial.status.success());

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    assert_eq!(
        initial_json["payload"]["dispatch_receipt"]["dispatch_target"],
        "business_analyst"
    );
    assert_eq!(
        initial_json["payload"]["dispatch_receipt"]["dispatch_status"],
        "blocked"
    );
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");
    let mut downstream_packet_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    let completion_result_path = format!("{project_root}/runtime-completion-result-4.json");
    write_runtime_lane_completion_result_fixture(
        &completion_result_path,
        "continue-auto-executes-ready-downstream-taskflow-packet",
        "implementer",
    );
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(true);
    downstream_packet_body["downstream_dispatch_blockers"] = serde_json::json!([]);
    downstream_packet_body["downstream_dispatch_result_path"] =
        serde_json::json!(completion_result_path);
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream packet should render"),
    );

    let resumed = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args([
                "taskflow",
                "consume",
                "continue",
                "--dispatch-packet",
                downstream_dispatch_packet_path,
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );

    let resumed_json: serde_json::Value =
        serde_json::from_slice(&resumed.stdout).expect("consume continue json should parse");
    assert_eq!(
        resumed_json["source_dispatch_packet_path"],
        downstream_dispatch_packet_path
    );
    assert!(resumed_json["dispatch_receipt"]["dispatch_target"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(resumed_json["dispatch_receipt"]["dispatch_status"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(resumed_json["dispatch_receipt"]["dispatch_result_path"].is_null());
    assert_eq!(
        resumed_json["dispatch_receipt"]["downstream_dispatch_target"],
        serde_json::Value::Null
    );
    assert_eq!(
        resumed_json["dispatch_receipt"]["downstream_dispatch_ready"],
        false
    );
    assert!(
        resumed_json["dispatch_receipt"]["downstream_dispatch_executed_count"]
            .as_u64()
            .unwrap_or(0)
            == 0
    );
    assert_eq!(
        resumed_json["dispatch_receipt"]["downstream_dispatch_last_target"],
        serde_json::Value::Null
    );
    assert_eq!(
        resumed_json["dispatch_receipt"]["downstream_dispatch_status"],
        serde_json::Value::Null
    );
    assert!(resumed_json["dispatch_receipt"]["downstream_dispatch_result_path"].is_null());
    assert!(resumed_json["dispatch_receipt"]["downstream_dispatch_trace_path"].is_null());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_advance_auto_progresses_ready_chain() {
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
            "advance-auto-progress",
            "--project-name",
            "Advance Auto Progress",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let initial = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "clarify the scope and write the specification before implementation",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow consume final json should run");
    assert!(!initial.status.success());

    let initial_json: serde_json::Value =
        serde_json::from_slice(&initial.stdout).expect("initial consume final json should parse");
    let downstream_dispatch_packet_path = initial_json["payload"]["dispatch_receipt"]
        ["downstream_dispatch_packet_path"]
        .as_str()
        .or_else(|| initial_json["payload"]["dispatch_receipt"]["dispatch_packet_path"].as_str())
        .expect("downstream dispatch packet path should be present");
    let mut downstream_packet_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(downstream_dispatch_packet_path)
            .expect("downstream dispatch packet should read"),
    )
    .expect("downstream dispatch packet should parse");
    let completion_result_path = format!("{project_root}/runtime-completion-result-5.json");
    write_runtime_lane_completion_result_fixture(
        &completion_result_path,
        "consume-advance-ready-downstream-taskflow-packet",
        "implementer",
    );
    downstream_packet_body["downstream_dispatch_ready"] = serde_json::json!(true);
    downstream_packet_body["downstream_dispatch_blockers"] = serde_json::json!([]);
    downstream_packet_body["downstream_dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["downstream_dispatch_result_path"] =
        serde_json::json!(completion_result_path);
    downstream_packet_body["downstream_lane_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["dispatch_status"] = serde_json::json!("packet_ready");
    downstream_packet_body["lane_status"] = serde_json::json!("packet_ready");
    atomic_write_file(
        downstream_dispatch_packet_path,
        &serde_json::to_string_pretty(&downstream_packet_body)
            .expect("mutated downstream packet should render"),
    );

    let advanced = vida()
        .args(["taskflow", "consume", "advance", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow consume advance should run");
    assert!(
        advanced.status.success(),
        "{}{}",
        String::from_utf8_lossy(&advanced.stdout),
        String::from_utf8_lossy(&advanced.stderr)
    );

    let advanced_json: serde_json::Value =
        serde_json::from_slice(&advanced.stdout).expect("consume advance json should parse");
    assert_eq!(advanced_json["surface"], "vida taskflow consume advance");
    assert_eq!(
        advanced_json["dispatch_receipt"]["dispatch_target"],
        "business_analyst"
    );
    assert_eq!(
        advanced_json["dispatch_receipt"]["dispatch_status"],
        "blocked"
    );
    assert_eq!(
        advanced_json["dispatch_receipt"]["downstream_dispatch_status"],
        serde_json::Value::Null
    );
    assert_eq!(
        advanced_json["dispatch_receipt"]["downstream_dispatch_last_target"],
        serde_json::Value::Null
    );
    assert!(
        advanced_json["dispatch_receipt"]["downstream_dispatch_executed_count"]
            .as_u64()
            .unwrap_or(0)
            == 0
    );
    assert!(
        advanced_json["rounds_executed"]
            .as_u64()
            .expect("rounds executed should be numeric")
            >= 1
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn consume_final_uses_local_project_context_when_repo_context_is_missing() {
    let state_dir = unique_state_dir();
    let root = unique_state_dir();
    let project_root = format!("{root}/project");
    fs::create_dir_all(&project_root).expect("project root should exist");

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
            "temp",
            "--project-name",
            "Temp",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let output = vida()
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final should render json payload");
    assert_eq!(
        parsed["payload"]["runtime_bundle"]["config_path"],
        format!("{project_root}/vida.config.yaml")
    );
    assert_eq!(
        parsed["payload"]["runtime_bundle"]["vida_root"],
        project_root
    );
    assert_eq!(parsed["payload"]["direct_consumption_ready"], false);
    assert_eq!(parsed["payload"]["docflow_verdict"]["status"], "blocked");
    assert_eq!(parsed["payload"]["docflow_verdict"]["ready"], false);
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    assert_eq!(parsed["payload"]["closure_admission"]["admitted"], false);
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["readiness"]["verdict"],
        "blocked"
    );
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["proof"]["verdict"],
        "blocked"
    );
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["receipt_evidence"]["receipt_backed"],
        true
    );
    let readiness_artifact_path = parsed["payload"]["docflow_activation"]["evidence"]["readiness"]
        ["artifact_path"]
        .as_str()
        .expect("readiness artifact path should be a string");
    assert!(readiness_artifact_path.ends_with("vida/config/docflow-readiness.current.jsonl"));
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["receipt_evidence"]
            ["readiness_receipt_path"],
        readiness_artifact_path
    );
    let blockers = parsed["payload"]["docflow_verdict"]["blockers"]
        .as_array()
        .expect("blockers should be an array");
    assert!(blockers.contains(&serde_json::Value::String(
        "missing_proof_verdict".to_string()
    )));
    let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
        .as_array()
        .expect("closure blockers should be an array");
    assert!(closure_blockers.contains(&serde_json::Value::String(
        "missing_closure_proof".to_string()
    )));
}

#[test]
fn consume_final_fails_closed_without_lane_bundle_fallback_when_runtime_bundle_build_fails() {
    let state_dir = unique_state_dir();
    let root = unique_state_dir();
    let project_root = format!("{root}/project");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    scaffold_runtime_project_root(&project_root, "project");

    let output = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["taskflow", "consume", "final", "probe closure", "--json"])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final should render blocking json payload");
    assert_eq!(parsed["payload"]["direct_consumption_ready"], false);
    assert_eq!(parsed["payload"]["role_selection"]["ok"], false);
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        "unresolved"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["status"],
        "blocked"
    );
    let shared_blockers = parsed["shared_fields"]["blocker_codes"]
        .as_array()
        .expect("shared blocker codes should be an array");
    assert!(shared_blockers.contains(&serde_json::Value::String(
        "bundle_activation_not_ready".to_string()
    )));
    assert!(shared_blockers.contains(&serde_json::Value::String(
        "docflow_verdict_block".to_string()
    )));
}

#[test]
fn consume_final_keeps_authoritative_launcher_snapshot_when_config_digest_changes() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let initial = vida()
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("initial consume final should run");
    assert!(!initial.status.success());
    let config_path = format!("{}/vida.config.yaml", repo_root());
    let restore_guard = FileRestoreGuard::new(config_path.clone());
    let original_config = restore_guard.original_body.clone();
    let updated_parallel_agents = 9_u64;
    let updated_config = original_config
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("max_parallel_agents:") {
                format!("  max_parallel_agents: {updated_parallel_agents}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_ne!(
        updated_config, original_config,
        "test config mutation should change max_parallel_agents"
    );
    atomic_write_file(&config_path, &updated_config);

    let output = vida()
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    drop(restore_guard);

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final should render json payload");
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        serde_json::json!("unresolved")
    );
    assert_eq!(
        parsed["payload"]["runtime_bundle"]["activation_source"],
        serde_json::json!("state_store")
    );
    assert_eq!(
        parsed["payload"]["runtime_bundle"]["config_path"],
        serde_json::Value::String(format!("{}/crates/vida/vida.config.yaml", repo_root()))
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["bundle_check"]["activation_status"],
        serde_json::json!("pending")
    );
    assert_eq!(parsed["shared_fields"]["status"], "blocked");
    assert_eq!(
        parsed["shared_fields"]["status"],
        parsed["operator_contracts"]["status"]
    );
    let blocker_codes = parsed["shared_fields"]["blocker_codes"]
        .as_array()
        .expect("shared blocker codes should be an array");
    assert!(blocker_codes.contains(&serde_json::Value::String(
        "bundle_activation_not_ready".to_string()
    )));
    assert!(blocker_codes.contains(&serde_json::Value::String(
        "closure_admission_block".to_string()
    )));
    assert!(blocker_codes.contains(&serde_json::Value::String(
        "docflow_verdict_block".to_string()
    )));
}

#[test]
fn taskflow_consume_final_selects_scope_discussion_role_for_spec_queries() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("scope-discussion-routing", "Scope Discussion Routing");

    let output = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "clarify spec scope",
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final scope json should parse");
    assert_eq!(parsed["payload"]["request_text"], "clarify spec scope");
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        "auto"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["project_profiles"]
            .as_array()
            .expect("project profiles should be an array")
            .len(),
        16
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["project_flows"]
            .as_array()
            .expect("project flows should be an array")
            .len(),
        6
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["selected_role"],
        "business_analyst"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        "scope_discussion"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["tracked_flow_entry"],
        "spec-pack"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["status"],
        "design_first"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["active_cycle"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["single_task_only"],
        true
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["allow_freeform_chat"],
        true
    );
    assert_eq!(parsed["payload"]["role_selection"]["confidence"], "high");
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_keyword_match"
    );
    assert_eq!(
        parsed["payload"]["taskflow_handoff_plan"]["status"],
        "spec_first_handoff_required"
    );
    assert_eq!(
        parsed["payload"]["taskflow_handoff_plan"]["design_packet_activation_source"],
        "runtime_assignment"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_target"],
        "business_analyst"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_kind"],
        "agent_lane"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_status"],
        "blocked"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_surface"],
        "vida agent-init"
    );
    assert!(parsed["payload"]["dispatch_receipt"]["dispatch_command"]
        .as_str()
        .expect("dispatch command should be present")
        .contains("vida agent-init"));
    let dispatch_packet_path = parsed["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");
    assert!(std::path::Path::new(dispatch_packet_path).is_file());
    assert!(parsed["payload"]["dispatch_receipt"]["dispatch_result_path"].is_null());
    assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_target"].is_null());
    assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_command"].is_null());
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["downstream_dispatch_ready"],
        false
    );
    assert!(parsed["payload"]["dispatch_receipt"]["downstream_dispatch_packet_path"].is_null());
    assert!(parsed["payload"]["dispatch_receipt"]["activation_agent_type"].is_null());
    let matched_terms = parsed["payload"]["role_selection"]["matched_terms"]
        .as_array()
        .expect("matched terms should be an array");
    assert!(matched_terms.iter().any(|value| value == "clarify"));
    assert!(matched_terms.iter().any(|value| value == "spec"));
    assert!(matched_terms.iter().any(|value| value == "scope"));
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    assert_eq!(parsed["payload"]["closure_admission"]["admitted"], false);
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_routes_mixed_feature_delivery_requests_to_spec_first() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("mixed-feature-spec-first", "Mixed Feature Spec First");

    let output = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "create a single-page html file, research the game mechanics, create detailed specifications, develop an implementation plan, and write the full code",
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final mixed feature json should parse");
    assert_eq!(
        parsed["payload"]["role_selection"]["selected_role"],
        "business_analyst"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        "scope_discussion"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["tracked_flow_entry"],
        "spec-pack"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_feature_design_request"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["status"],
        "design_first"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["active_cycle"],
        serde_json::Value::Null
    );
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    assert_eq!(parsed["payload"]["closure_admission"]["admitted"], false);
    let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
        .as_array()
        .expect("closure blockers should be an array");
    assert!(closure_blockers
        .iter()
        .any(|row| row == "pending_developer_handoff_packet"));
    assert_eq!(parsed["payload"]["direct_consumption_ready"], false);
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["pre_execution_design_gate"]
            ["design_runtime"],
        "vida docflow"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["pre_execution_design_gate"]
            ["design_template"],
        "docs/product/spec/templates/feature-design-document.template.md"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["execution_plan"]["development_flow"]
            ["activation_status"],
        "blocked_pending_design_packet"
    );
    let todo_sequence = parsed["payload"]["role_selection"]["execution_plan"]
        ["pre_execution_design_gate"]["todo_sequence"]
        .as_array()
        .expect("todo sequence should be an array");
    assert_eq!(todo_sequence.len(), 4);
    assert!(todo_sequence.iter().any(
        |row| row == "create one epic and one spec task in vida taskflow before code execution"
    ));
    assert!(todo_sequence.iter().any(|row| row
        == "keep the design artifact canonical through vida docflow init/finalize-edit/check"));
    let taskflow_sequence = parsed["payload"]["role_selection"]["execution_plan"]
        ["pre_execution_design_gate"]["taskflow_sequence"]
        .as_array()
        .expect("taskflow sequence should be an array");
    assert_eq!(taskflow_sequence[0], "spec-pack");
    assert_eq!(taskflow_sequence[1], "work-pool-pack");
    assert_eq!(taskflow_sequence[2], "dev-pack");
    let structured_todo = parsed["payload"]["role_selection"]["execution_plan"]
        ["pre_execution_todo"]["items"]
        .as_array()
        .expect("structured todo should be an array");
    assert_eq!(structured_todo.len(), 6);
    assert_eq!(structured_todo[0]["id"], "taskflow_epic_open");
    assert_eq!(structured_todo[1]["id"], "taskflow_spec_task_open");
    assert_eq!(structured_todo[2]["id"], "design_doc_scope");
    assert_eq!(structured_todo[3]["id"], "design_doc_finalize");
    assert_eq!(structured_todo[4]["id"], "taskflow_spec_task_close");
    assert_eq!(structured_todo[5]["id"], "taskflow_packet_shape");
    assert_eq!(structured_todo[2]["runtime"], "vida docflow");
    assert_eq!(structured_todo[5]["runtime"], "vida taskflow");
    let orchestration_contract =
        &parsed["payload"]["role_selection"]["execution_plan"]["orchestration_contract"];
    assert_eq!(
        orchestration_contract["mode"],
        "delegated_orchestration_cycle"
    );
    assert_eq!(orchestration_contract["root_session_role"], "orchestrator");
    assert_eq!(
        orchestration_contract["initial_response"]["plan_required_before_substantive_execution"],
        true
    );
    assert_eq!(
        orchestration_contract["delegation_policy"]["normal_write_producing_work"],
        "delegated_by_default"
    );
    assert_eq!(
        orchestration_contract["delegation_policy"]
            ["canonical_project_delegated_execution_surface"],
        "vida agent-init"
    );
    assert_eq!(
        orchestration_contract["delegation_policy"]["host_subagent_apis_are_backend_details"],
        true
    );
    assert_eq!(
        orchestration_contract["delegation_policy"]
            ["local_implementation_without_exception_path_forbidden"],
        true
    );
    let active_cycle = orchestration_contract["active_cycle"]
        .as_array()
        .expect("active cycle should be an array");
    assert_eq!(active_cycle[0], "publish_initial_execution_plan");
    assert!(active_cycle
        .iter()
        .any(|step| step == "delegate_specification_or_research_lane"));
    assert!(active_cycle
        .iter()
        .any(|step| step == "delegate_implementer_lane"));
    let replanning_checkpoints = orchestration_contract["replanning"]["checkpoints"]
        .as_array()
        .expect("replanning checkpoints should be an array");
    assert!(replanning_checkpoints
        .iter()
        .any(|step| step == "after_design_gate"));
    assert!(replanning_checkpoints
        .iter()
        .any(|step| step == "after_implementation_evidence"));
    let tracked_bootstrap =
        &parsed["payload"]["role_selection"]["execution_plan"]["tracked_flow_bootstrap"];
    assert_eq!(tracked_bootstrap["required"], true);
    assert!(tracked_bootstrap["design_doc_path"]
        .as_str()
        .expect("design doc path should be a string")
        .ends_with("-design.md"));
    assert!(tracked_bootstrap["docflow"]["check_command"]
        .as_str()
        .expect("docflow check command should be a string")
        .starts_with("vida docflow check --root . docs/product/spec/"));
    assert!(tracked_bootstrap["epic"]["task_id"]
        .as_str()
        .expect("epic task id should be a string")
        .starts_with("feature-"));
    assert!(tracked_bootstrap["spec_task"]["task_id"]
        .as_str()
        .expect("spec task id should be a string")
        .ends_with("-spec"));
    assert!(tracked_bootstrap["epic"]["create_command"]
        .as_str()
        .expect("epic create command should be a string")
        .contains("vida task create feature-"));
    assert!(tracked_bootstrap["bootstrap_command"]
        .as_str()
        .expect("bootstrap command should be a string")
        .starts_with("vida taskflow bootstrap-spec "));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_plain_prefers_bootstrap_spec_over_manual_design_steps() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("plain-bootstrap-spec", "Plain Bootstrap Spec");

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "create a single-page html file, research the game mechanics, create detailed specifications, develop an implementation plan, and write the full code",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("plain consume final should run");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("execution mode: delegated_orchestration_cycle"));
    assert!(stdout.contains("first step: publish a concise execution plan"));
    assert!(stdout.contains("next tracked command: vida taskflow bootstrap-spec "));
    assert!(stdout.contains("delegated lanes: specification, implementer"));
    assert!(!stdout.contains("next epic command:"));
    assert!(!stdout.contains("next design command:"));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_bootstrap_spec_creates_epic_spec_task_and_design_doc() {
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
            "flappy-test",
            "--project-name",
            "Flappy Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let request = "Create a single-page HTML file containing a Flappy Bird game. Research the game mechanics, create detailed specifications, develop an implementation plan, and write the full code.";
    let output = vida()
        .args(["taskflow", "bootstrap-spec", request, "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("bootstrap-spec should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("bootstrap-spec should render json");
    assert_eq!(parsed["surface"], "vida taskflow bootstrap-spec");
    assert_eq!(parsed["admission"]["status"], "admitted");
    assert_eq!(parsed["admission"]["admitted"], true);
    assert!(parsed["admission"]["consumed_evidence"]
        .as_array()
        .expect("admission consumed_evidence should be an array")
        .iter()
        .any(|value| value == "tracked_flow_bootstrap.docflow.finalize_command"));
    assert!(parsed["admission"]["consumed_evidence"]
        .as_array()
        .expect("admission consumed_evidence should be an array")
        .iter()
        .any(|value| value == "tracked_flow_bootstrap.spec_task.task_id"));
    let design_doc_rel = parsed["design_doc"]["path"]
        .as_str()
        .expect("design doc path should exist");
    assert!(std::path::Path::new(&project_root)
        .join(design_doc_rel)
        .is_file());
    let spec_readme = fs::read_to_string(format!("{project_root}/docs/product/spec/README.md"))
        .expect("spec readme should exist");
    assert!(spec_readme.contains(design_doc_rel));
    let receipt_rel = parsed["receipt_path"]
        .as_str()
        .expect("receipt path should exist");
    assert!(std::path::Path::new(&project_root)
        .join(receipt_rel)
        .is_file());
    assert!(parsed["epic"]["task_id"]
        .as_str()
        .expect("epic task id should exist")
        .starts_with("feature-"));
    assert!(parsed["spec_task"]["task_id"]
        .as_str()
        .expect("spec task id should exist")
        .ends_with("-spec"));

    let check = vida()
        .args(["docflow", "check-file", "--path", design_doc_rel])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("docflow check-file should run");
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let check_stdout = String::from_utf8_lossy(&check.stdout);
    assert!(check_stdout.contains("verdict: ok"), "{check_stdout}");

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_bootstrap_spec_plain_reports_orchestrated_follow_up_commands() {
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
            "flappy-test",
            "--project-name",
            "Flappy Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let request = "Create a single-page HTML file containing a Flappy Bird game. Research the game mechanics, create detailed specifications, develop an implementation plan, and write the full code.";
    let output = vida()
        .args(["taskflow", "bootstrap-spec", request])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("plain bootstrap-spec should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("first step: publish a concise execution plan before mutating the design document or dispatching write-producing work"));
    assert!(stdout.contains("finalize design: vida docflow finalize-edit "));
    assert!(stdout.contains("check design: vida docflow check --root . "));
    assert!(stdout.contains("close spec task: vida task close "));
    assert!(stdout.contains("next work-pool command: vida task ensure "));
    assert!(stdout.contains("next dev command: vida task ensure "));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_bootstrap_spec_self_heals_missing_product_spec_readme() {
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
            "flappy-test",
            "--project-name",
            "Flappy Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    fs::remove_file(format!("{project_root}/docs/product/spec/README.md"))
        .expect("spec readme should be removable for self-heal proof");

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
    sync_protocol_binding(&state_dir);

    let request = "Create a single-page HTML file containing a Flappy Bird game. Research the game mechanics, create detailed specifications, develop an implementation plan, and write the full code.";
    let output = vida()
        .args(["taskflow", "bootstrap-spec", request, "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("bootstrap-spec should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let spec_readme = fs::read_to_string(format!("{project_root}/docs/product/spec/README.md"))
        .expect("spec readme should be recreated");
    assert!(spec_readme.contains("# Product Spec Guide"));
    assert!(spec_readme.contains("flappy-bird"));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn project_activator_fails_closed_when_authoritative_state_store_cannot_open() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");

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

    let config_path = format!("{project_root}/vida.config.yaml");
    let original_config =
        fs::read_to_string(&config_path).expect("config should read before activation attempt");

    let blocked_state_path = format!("{project_root}/blocked-state");
    write_file(&blocked_state_path, "not-a-directory");

    let output = vida()
        .args([
            "project-activator",
            "--project-id",
            "blocked-state-test",
            "--project-name",
            "Blocked State Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--state-dir",
            &blocked_state_path,
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed closed before mutation"));
    assert!(stderr.contains("authoritative state store"));

    let after_config =
        fs::read_to_string(&config_path).expect("config should still be readable after failure");
    assert_eq!(after_config, original_config);

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn project_activator_after_init_without_boot() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env_remove("VIDA_STATE_DIR")
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
            "test-project",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env_remove("VIDA_STATE_DIR")
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "stderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&activator.stderr),
        String::from_utf8_lossy(&activator.stdout)
    );

    let payload: serde_json::Value =
        serde_json::from_slice(&activator.stdout).expect("activator stdout should be json");
    assert_eq!(payload["surface"], "vida project-activator");
    assert_eq!(
        payload["activation_log"]["db_first_activation_truth"]["read_back_verified"],
        true
    );
    assert!(Path::new(&project_root).join(".vida/data/state").is_dir());

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn project_activator_rejects_foreign_env_state_dir_for_mutation() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");

    let init = vida()
        .arg("init")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env_remove("VIDA_STATE_DIR")
        .output()
        .expect("init should run");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let foreign_project_root = unique_state_dir();
    let foreign_state_dir = format!("{foreign_project_root}/.vida/data/state");
    fs::create_dir_all(&foreign_state_dir).expect("foreign state dir should exist");

    let activator = vida()
        .args([
            "project-activator",
            "--project-id",
            "test-project",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &foreign_state_dir)
        .output()
        .expect("project activator should run");
    assert!(!activator.status.success());

    let stderr = String::from_utf8_lossy(&activator.stderr);
    assert!(stderr.contains("failed closed before mutation"));
    assert!(
        stderr.contains("non-project state dir")
            || stderr.contains("default authoritative state dir")
    );
    assert!(!stderr.contains("DB-first activation truth"));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
    fs::remove_dir_all(foreign_project_root).expect("foreign temp root should be removed");
}

#[test]
fn status_json_exposes_host_agent_summary() {
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
            "status-test",
            "--project-name",
            "Status Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let output = status_with_timeout(&project_root, &state_dir, &["status", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status should render json");
    assert_eq!(parsed["host_agents"]["host_cli_system"], "codex");
    assert_eq!(parsed["host_agents"]["agents"]["junior"]["rate"], 1);
    assert_eq!(parsed["host_agents"]["internal_dispatch_alias_count"], 6);
    assert!(parsed["host_agents"]["named_lanes"].is_null());
    assert_eq!(parsed["host_agents"]["budget"]["total_estimated_units"], 0);

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn project_activator_materializes_codex_spark_for_read_only_codex_tiers() {
    let project_root = unique_state_dir();
    fs::create_dir_all(&project_root).expect("project root should exist");

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
            "codex-spark-materialization",
            "--project-name",
            "Codex Spark Materialization",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );
    let activator_payload: serde_json::Value =
        serde_json::from_slice(&activator.stdout).expect("project activator should render json");
    assert_eq!(activator_payload["surface"], "vida project-activator");
    assert_eq!(activator_payload["post_init_restart_required"], true);
    let activator_parsed = &activator_payload["view"];
    assert_eq!(activator_parsed["status"], "ready_enough_for_normal_work");
    assert_eq!(
        activator_parsed["host_environment"]["selected_cli_system"],
        "codex"
    );
    assert_eq!(
        activator_parsed["host_environment"]["template_materialized"],
        true
    );
    assert_eq!(
        activator_parsed["host_environment"]["materialization_required"],
        false
    );
    assert_eq!(
        activator_parsed["host_environment"]["runtime_template_root"],
        ".codex"
    );

    let config_path = std::path::Path::new(&project_root).join("vida.config.yaml");
    let config = fs::read_to_string(&config_path).expect("config should exist");
    let updated = config.replace(
        r#"      architect:
        tier: architect
        rate: 32
        reasoning_band: xhigh
        model: gpt-5.3-codex-spark
        model_reasoning_effort: high
        sandbox_mode: read-only
        default_runtime_role: solution_architect
        runtime_roles:
          - solution_architect
        task_classes:
          - architecture
          - execution_preparation
          - hard_escalation
          - meta_analysis
"#,
        r#"      architect:
        tier: architect
        rate: 32
        reasoning_band: xhigh
        default_model_profile: codex_spark_xhigh_arch
        model_profiles:
          codex_spark_high_arch:
            model: gpt-5.3-codex-spark
            reasoning_effort: high
            sandbox_mode: read-only
            runtime_roles:
              - solution_architect
            task_classes:
              - architecture
              - execution_preparation
              - hard_escalation
              - meta_analysis
          codex_spark_xhigh_arch:
            model: gpt-5.3-codex-spark
            reasoning_effort: xhigh
            sandbox_mode: read-only
            runtime_roles:
              - solution_architect
            task_classes:
              - architecture
              - execution_preparation
              - hard_escalation
              - meta_analysis
        default_runtime_role: solution_architect
        runtime_roles:
          - solution_architect
        task_classes:
          - architecture
          - execution_preparation
          - hard_escalation
          - meta_analysis
"#,
    );
    assert_ne!(
        updated, config,
        "expected architect legacy carrier block replacement"
    );
    fs::write(&config_path, updated).expect("config should update");

    let rerender = vida()
        .args([
            "project-activator",
            "--project-id",
            "codex-spark-materialization",
            "--project-name",
            "Codex Spark Materialization",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator rerender should run");
    assert!(
        rerender.status.success(),
        "{}",
        String::from_utf8_lossy(&rerender.stderr)
    );

    let junior = fs::read_to_string(format!("{project_root}/.codex/agents/junior.toml"))
        .expect("junior codex carrier should materialize");
    let middle = fs::read_to_string(format!("{project_root}/.codex/agents/middle.toml"))
        .expect("middle codex carrier should materialize");
    let senior = fs::read_to_string(format!("{project_root}/.codex/agents/senior.toml"))
        .expect("senior codex carrier should materialize");
    let architect = fs::read_to_string(format!("{project_root}/.codex/agents/architect.toml"))
        .expect("architect codex carrier should materialize");

    assert!(junior.contains("model = \"gpt-5.4\""));
    assert!(middle.contains("model = \"gpt-5.4\""));
    assert!(senior.contains("model = \"gpt-5.3-codex-spark\""));
    assert!(architect.contains("model = \"gpt-5.3-codex-spark\""));
    assert!(senior.contains("sandbox_mode = \"read-only\""));
    assert!(architect.contains("sandbox_mode = \"read-only\""));
    assert!(senior.contains("model_reasoning_effort = \"high\""));
    assert!(architect.contains("model_reasoning_effort = \"xhigh\""));

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_task_close_records_auto_feedback_and_budget() {
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
            "close-test",
            "--project-name",
            "Close Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let request = "Create a single-page HTML file containing a Flappy Bird game. Research the mechanics, create specifications, and develop an implementation plan.";
    let bootstrap = vida()
        .args(["taskflow", "bootstrap-spec", request, "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("bootstrap-spec should run");
    assert!(
        bootstrap.status.success(),
        "{}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );
    let bootstrap_json: serde_json::Value =
        serde_json::from_slice(&bootstrap.stdout).expect("bootstrap json should parse");
    let spec_task_id = bootstrap_json["spec_task"]["task_id"]
        .as_str()
        .expect("spec task id should exist")
        .to_string();

    let close = run_with_state_lock_retry(|| {
        vida()
            .args([
                "taskflow",
                "task",
                "close",
                &spec_task_id,
                "--reason",
                "design packet finalized and handed off into tracked work-pool shaping",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("task close should run")
    });
    assert!(
        close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&close.stdout),
        String::from_utf8_lossy(&close.stderr)
    );
    let close_json: serde_json::Value =
        serde_json::from_slice(&close.stdout).expect("task close json should parse");
    assert_eq!(close_json["host_agent_telemetry"]["status"], "recorded");
    assert_eq!(
        close_json["host_agent_telemetry"]["assignment"]["selected_tier"],
        "middle"
    );

    let scorecards =
        fs::read_to_string(format!("{project_root}/.vida/state/worker-scorecards.json"))
            .expect("scorecards should exist");
    let scorecards_json: serde_json::Value =
        serde_json::from_str(&scorecards).expect("scorecards json should parse");
    assert_eq!(
        scorecards_json["agents"]["middle"]["feedback"]
            .as_array()
            .expect("feedback should be an array")
            .len(),
        1
    );

    let observability = fs::read_to_string(format!(
        "{project_root}/.vida/state/host-agent-observability.json"
    ))
    .expect("observability ledger should exist");
    let observability_json: serde_json::Value =
        serde_json::from_str(&observability).expect("observability json should parse");
    assert_eq!(observability_json["budget"]["total_estimated_units"], 8);
    assert_eq!(
        observability_json["events"][0]["source"],
        "vida taskflow task close"
    );
    assert_eq!(
        observability_json["events"][0]["feedback_event"]["artifact_type"],
        "feedback_event"
    );
    assert_eq!(
        observability_json["events"][0]["evaluation_baseline"]["artifact_type"],
        "evaluation_run"
    );
    assert_eq!(
        observability_json["events"][0]["prompt_lifecycle_baseline"]["lifecycle_state"],
        "draft"
    );
    assert_eq!(
        observability_json["events"][0]["safety_baseline"]["safety_gate"],
        "observe"
    );
    let prompt_lifecycle =
        fs::read_to_string(format!("{project_root}/.vida/state/prompt-lifecycle.json"))
            .expect("prompt lifecycle store should exist");
    let prompt_lifecycle_json: serde_json::Value =
        serde_json::from_str(&prompt_lifecycle).expect("prompt lifecycle json should parse");
    let workflows = prompt_lifecycle_json["workflows"]
        .as_object()
        .expect("prompt lifecycle workflows should render");
    assert_eq!(workflows.len(), 1);
    assert_eq!(
        workflows
            .values()
            .next()
            .expect("one workflow baseline should exist")["lifecycle_state"],
        "draft"
    );

    let status = status_with_timeout(&project_root, &state_dir, &["status", "--json"]);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json should parse");
    assert_eq!(
        status_json["host_agents"]["latest_feedback_event"]["artifact_type"],
        "feedback_event"
    );
    assert_eq!(
        status_json["host_agents"]["latest_evaluation_baseline"]["artifact_type"],
        "evaluation_run"
    );
    assert_eq!(
        status_json["host_agents"]["latest_prompt_lifecycle_baseline"]["lifecycle_state"],
        "draft"
    );
    assert_eq!(
        status_json["host_agents"]["latest_safety_baseline"]["safety_gate"],
        "observe"
    );
    assert_eq!(
        status_json["host_agents"]["stores"]["prompt_lifecycle"],
        ".vida/state/prompt-lifecycle.json"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_task_close_skips_auto_feedback_while_awaiting_approval() {
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
            "approval-gate-test",
            "--project-name",
            "Approval Gate Test",
            "--language",
            "english",
            "--host-cli-system",
            "codex",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .output()
        .expect("project activator should run");
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
    sync_protocol_binding(&state_dir);

    let request = "Create a one-page app with a delivery plan and implementation spec.";
    let bootstrap = vida()
        .args(["taskflow", "bootstrap-spec", request, "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("bootstrap-spec should run");
    assert!(
        bootstrap.status.success(),
        "{}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );
    let bootstrap_json: serde_json::Value =
        serde_json::from_slice(&bootstrap.stdout).expect("bootstrap json should parse");
    let spec_task_id = bootstrap_json["spec_task"]["task_id"]
        .as_str()
        .expect("spec task id should exist")
        .to_string();

    let close = run_with_state_lock_retry(|| {
        vida()
            .args([
                "taskflow",
                "task",
                "close",
                &spec_task_id,
                "--reason",
                "awaiting_approval gate remains active; approval required before completion",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("task close should run")
    });
    assert!(
        close.status.success(),
        "{}{}",
        String::from_utf8_lossy(&close.stdout),
        String::from_utf8_lossy(&close.stderr)
    );
    let close_json: serde_json::Value =
        serde_json::from_slice(&close.stdout).expect("task close json should parse");
    assert_eq!(close_json["host_agent_telemetry"]["status"], "skipped");
    assert_eq!(
        close_json["host_agent_telemetry"]["reason"],
        "feedback_deferred_for_canonical_close_status"
    );
    assert_eq!(
        close_json["host_agent_telemetry"]["canonical_status"],
        "awaiting_approval"
    );
    assert_eq!(
        close_json["host_agent_telemetry"]["canonical_gate"],
        "approval_required"
    );

    let scorecards =
        fs::read_to_string(format!("{project_root}/.vida/state/worker-scorecards.json"))
            .expect("scorecards should exist");
    let scorecards_json: serde_json::Value =
        serde_json::from_str(&scorecards).expect("scorecards json should parse");
    assert_eq!(
        scorecards_json["agents"]["middle"]["feedback"]
            .as_array()
            .expect("feedback should be an array")
            .len(),
        0
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn agent_feedback_fails_closed_when_notes_exceed_bounded_ingestion_contract() {
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

    let activator = run_with_state_lock_retry(|| {
        vida()
            .args([
                "project-activator",
                "--project-id",
                "feedback-notes-bound-test",
                "--project-name",
                "Feedback Notes Bound Test",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .output()
            .expect("project activator should run")
    });
    assert!(
        activator.status.success(),
        "{}",
        String::from_utf8_lossy(&activator.stderr)
    );

    let boot = run_with_state_lock_retry(|| {
        vida()
            .arg("boot")
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("boot should run")
    });
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );

    let long_notes = "x".repeat(2049);
    let feedback = run_with_state_lock_retry(|| {
        vida()
            .args([
                "agent-feedback",
                "--agent-id",
                "middle",
                "--score",
                "82",
                "--notes",
                &long_notes,
                "--json",
            ])
            .current_dir(&project_root)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("agent-feedback should run")
    });
    assert!(
        !feedback.status.success(),
        "{}",
        String::from_utf8_lossy(&feedback.stdout)
    );
    let stderr = String::from_utf8_lossy(&feedback.stderr);
    assert!(
        stderr.contains("Feedback notes exceed bounded ingestion contract"),
        "{stderr}"
    );

    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_selects_pbi_discussion_role_for_backlog_queries() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("pbi-discussion-routing", "PBI Discussion Routing");

    let output = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "prioritize backlog work pool",
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final pbi json should parse");
    assert_eq!(
        parsed["payload"]["request_text"],
        "prioritize backlog work pool"
    );
    assert_eq!(parsed["payload"]["role_selection"]["selected_role"], "pm");
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        "pbi_discussion"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["tracked_flow_entry"],
        "work-pool-pack"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["single_task_only"],
        true
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["allow_freeform_chat"],
        true
    );
    assert_eq!(parsed["payload"]["role_selection"]["confidence"], "high");
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_keyword_match"
    );
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_target"],
        "pm"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_kind"],
        "agent_lane"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_status"],
        "blocked"
    );
    assert_eq!(
        parsed["payload"]["dispatch_receipt"]["dispatch_surface"],
        "vida agent-init"
    );
    let dispatch_packet_path = parsed["payload"]["dispatch_receipt"]["dispatch_packet_path"]
        .as_str()
        .expect("dispatch packet path should be present");
    assert!(std::path::Path::new(dispatch_packet_path).is_file());
    assert!(parsed["payload"]["dispatch_receipt"]["dispatch_result_path"].is_null());
    assert!(parsed["payload"]["dispatch_receipt"]["activation_agent_type"].is_null());
    let matched_terms = parsed["payload"]["role_selection"]["matched_terms"]
        .as_array()
        .expect("matched terms should be an array");
    assert!(matched_terms.iter().any(|value| value == "prioritize"));
    assert!(matched_terms.iter().any(|value| value == "backlog"));
    assert!(matched_terms.iter().any(|value| value == "work pool"));
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_does_not_match_short_substring_false_positive() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "short-substring-false-positive-probe",
        "Short Substring False Positive Probe",
    );

    let output =
        project_bound_taskflow_consume_final_with_timeout(&project_root, &state_dir, "trace cache");
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final false-positive probe should parse");
    assert_eq!(
        parsed["payload"]["role_selection"]["selected_role"],
        "orchestrator"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_no_keyword_match"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["matched_terms"]
            .as_array()
            .expect("matched terms should be an array")
            .len(),
        0
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        "auto"
    );
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_does_not_match_ac_inside_incidental_words() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("ac-false-positive-probe", "AC False Positive Probe");

    let output = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "trace cache invalidation",
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final ac false-positive probe should parse");
    assert_eq!(
        parsed["payload"]["role_selection"]["selected_role"],
        "orchestrator"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_no_keyword_match"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["matched_terms"],
        serde_json::json!([])
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        "auto"
    );
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_does_not_match_aspect_incidental_word() {
    let (project_root, state_dir) =
        bootstrap_project_runtime("aspect-false-positive-probe", "Aspect False Positive Probe");

    let output = project_bound_taskflow_consume_final_with_timeout(
        &project_root,
        &state_dir,
        "review one aspect of caching",
    );
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("consume final aspect false-positive probe should parse");
    assert_eq!(
        parsed["payload"]["role_selection"]["selected_role"],
        "orchestrator"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_no_keyword_match"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["matched_terms"],
        serde_json::json!([])
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        "auto"
    );
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "blocked");
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_recovery_checkpoint_latest_reports_none_on_empty_booted_state() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let text_output = taskflow_recovery_latest_with_timeout(&state_dir, "checkpoint-latest", false);
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow recovery checkpoint-latest"));
    assert!(text_stdout.contains("checkpoint: none"));

    let json_output = taskflow_recovery_latest_with_timeout(&state_dir, "checkpoint-latest", true);
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("checkpoint latest json should parse");
    assert_eq!(
        parsed["surface"],
        "vida taskflow recovery checkpoint-latest"
    );
    assert!(parsed["checkpoint"].is_null());
}

#[test]
fn taskflow_recovery_gate_latest_reports_none_on_empty_booted_state() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let text_output = taskflow_recovery_latest_with_timeout(&state_dir, "gate-latest", false);
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow recovery gate-latest"));
    assert!(text_stdout.contains("gate: none"));

    let json_output = taskflow_recovery_latest_with_timeout(&state_dir, "gate-latest", true);
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("gate latest json should parse");
    assert_eq!(parsed["surface"], "vida taskflow recovery gate-latest");
    assert!(parsed["gate"].is_null());
}

#[test]
fn taskflow_recovery_status_fails_closed_for_missing_run() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "recovery", "status", "missing-run", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow recovery status should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.trim().is_empty());
}

#[test]
fn taskflow_recovery_checkpoint_fails_closed_for_missing_run() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args([
            "taskflow",
            "recovery",
            "checkpoint",
            "missing-run",
            "--json",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow recovery checkpoint should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.trim().is_empty());
}

#[test]
fn taskflow_recovery_gate_fails_closed_for_missing_run() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "recovery", "gate", "missing-run", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow recovery gate should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.trim().is_empty());
}

#[test]
fn taskflow_run_graph_status_fails_closed_for_missing_run() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["taskflow", "run-graph", "status", "missing-run", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph status should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.trim().is_empty());
}

#[test]
fn taskflow_run_graph_latest_reports_none_on_empty_booted_state() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let text_output = taskflow_run_graph_latest_with_timeout(&state_dir, false);
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow run-graph latest"));
    assert!(text_stdout.contains("status: none"));

    let json_output = taskflow_run_graph_latest_with_timeout(&state_dir, true);
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&json_stdout).expect("run-graph latest json should parse");
    assert_eq!(parsed["surface"], "vida taskflow run-graph latest");
    assert!(parsed["status"].is_null());
}

#[test]
fn taskflow_run_graph_bridge_syncs_non_empty_latest_flow_surfaces() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            "vida-a",
            "writer",
            "analysis",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph init should run");
    assert!(init.status.success());

    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-a",
            "writer",
            "writer",
            "ready",
            "analysis",
            "{\"next_node\":\"coach\",\"selected_backend\":\"runtime_selected_tier\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"policy_gate_required\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());

    let run_graph_latest = taskflow_run_graph_latest_with_timeout(&state_dir, true);
    assert!(run_graph_latest.status.success());
    let run_graph_latest_stdout = String::from_utf8_lossy(&run_graph_latest.stdout);
    let run_graph_latest_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_latest_stdout).expect("run-graph latest should parse");
    assert_eq!(
        run_graph_latest_parsed["run_graph_status"]["run_id"],
        "vida-a"
    );
    assert_eq!(
        run_graph_latest_parsed["run_graph_status"]["active_node"],
        "writer"
    );
    assert_eq!(
        run_graph_latest_parsed["run_graph_status"]["next_node"],
        "writer"
    );
    assert_eq!(
        run_graph_latest_parsed["run_graph_status"]["policy_gate"],
        "policy_gate_required"
    );
    assert_eq!(
        run_graph_latest_parsed["run_graph_status"]["checkpoint_kind"],
        "execution_cursor"
    );

    let recovery_latest = taskflow_recovery_latest_with_timeout(&state_dir, "latest", true);
    assert!(recovery_latest.status.success());
    let recovery_latest_stdout = String::from_utf8_lossy(&recovery_latest.stdout);
    let recovery_latest_parsed: serde_json::Value =
        serde_json::from_str(&recovery_latest_stdout).expect("recovery latest should parse");
    assert_eq!(recovery_latest_parsed["recovery"]["run_id"], "vida-a");
    assert_eq!(recovery_latest_parsed["recovery"]["resume_node"], "writer");
    assert_eq!(recovery_latest_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_latest_parsed["recovery"]["resume_target"],
        "dispatch.writer_lane"
    );

    let checkpoint_latest =
        taskflow_recovery_latest_with_timeout(&state_dir, "checkpoint-latest", true);
    assert!(checkpoint_latest.status.success());
    let checkpoint_latest_stdout = String::from_utf8_lossy(&checkpoint_latest.stdout);
    let checkpoint_latest_parsed: serde_json::Value =
        serde_json::from_str(&checkpoint_latest_stdout).expect("checkpoint latest should parse");
    assert_eq!(checkpoint_latest_parsed["checkpoint"]["run_id"], "vida-a");
    assert_eq!(
        checkpoint_latest_parsed["checkpoint"]["checkpoint_kind"],
        "execution_cursor"
    );
    assert_eq!(
        checkpoint_latest_parsed["checkpoint"]["resume_target"],
        "dispatch.writer_lane"
    );
    assert_eq!(
        checkpoint_latest_parsed["checkpoint"]["recovery_ready"],
        true
    );

    let gate_latest = taskflow_recovery_latest_with_timeout(&state_dir, "gate-latest", true);
    assert!(gate_latest.status.success());
    let gate_latest_stdout = String::from_utf8_lossy(&gate_latest.stdout);
    let gate_latest_parsed: serde_json::Value =
        serde_json::from_str(&gate_latest_stdout).expect("gate latest should parse");
    assert_eq!(gate_latest_parsed["gate"]["run_id"], "vida-a");
    assert_eq!(
        gate_latest_parsed["gate"]["policy_gate"],
        "policy_gate_required"
    );
    assert_eq!(
        gate_latest_parsed["gate"]["handoff_state"],
        "awaiting_writer"
    );
    assert_eq!(gate_latest_parsed["gate"]["context_state"], "sealed");
    assert_eq!(
        gate_latest_parsed["gate"]["delegation_gate"]["delegated_cycle_open"],
        true
    );
    assert_eq!(
        gate_latest_parsed["gate"]["delegation_gate"]["local_exception_takeover_gate"],
        "blocked_open_delegated_cycle"
    );
    assert_eq!(
        gate_latest_parsed["gate"]["delegation_gate"]["continuation_signal"],
        "continue_routing_non_blocking"
    );

    let status_output = run_command_with_state_lock_retry(|| {
        let mut cmd = vida();
        cmd.args(["status", "--json"]);
        cmd.env("VIDA_STATE_DIR", &state_dir);
        cmd
    });
    assert!(status_output.status.success());
    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    let status_parsed: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status json should parse");
    assert_eq!(
        status_parsed["latest_run_graph_checkpoint"]["run_id"],
        "vida-a"
    );
    assert_eq!(status_parsed["latest_run_graph_gate"]["run_id"], "vida-a");

    let doctor_output = run_command_with_state_lock_retry(|| {
        let mut cmd = vida();
        cmd.args(["doctor", "--json"]);
        cmd.env("VIDA_STATE_DIR", &state_dir);
        cmd
    });
    assert!(doctor_output.status.success());
    let doctor_stdout = String::from_utf8_lossy(&doctor_output.stdout);
    let doctor_parsed: serde_json::Value =
        serde_json::from_str(&doctor_stdout).expect("doctor json should parse");
    assert_eq!(doctor_parsed["latest_run_graph_status"]["run_id"], "vida-a");
    assert_eq!(
        doctor_parsed["latest_run_graph_recovery"]["run_id"],
        "vida-a"
    );
    assert_eq!(
        doctor_parsed["latest_run_graph_checkpoint"]["checkpoint_kind"],
        "execution_cursor"
    );
    assert_eq!(
        doctor_parsed["latest_run_graph_gate"]["policy_gate"],
        "policy_gate_required"
    );
}

#[test]
fn status_and_doctor_text_surfaces_report_non_empty_latest_flow_state() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            "vida-a",
            "writer",
            "analysis",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph init should run");
    assert!(init.status.success());

    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-a",
            "writer",
            "writer",
            "ready",
            "analysis",
            "{\"next_node\":\"coach\",\"selected_backend\":\"runtime_selected_tier\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"policy_gate_required\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());

    let status_output = run_command_with_state_lock_retry(|| {
        let mut cmd = vida();
        cmd.arg("status");
        cmd.env("VIDA_STATE_DIR", &state_dir);
        cmd
    });
    assert!(status_output.status.success());
    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_stdout.contains("latest run graph status: run=vida-a"));
    assert!(status_stdout.contains("latest run graph recovery: run=vida-a"));
    assert!(status_stdout.contains("latest run graph checkpoint: run=vida-a"));
    assert!(status_stdout.contains("latest run graph gate: run=vida-a"));
    assert!(status_stdout.contains("checkpoint=execution_cursor"));
    assert!(status_stdout.contains("gate=policy_gate_required"));

    let doctor_output = run_command_with_state_lock_retry(|| {
        let mut cmd = vida();
        cmd.arg("doctor");
        cmd.env("VIDA_STATE_DIR", &state_dir);
        cmd
    });
    assert!(doctor_output.status.success());
    let doctor_stdout = String::from_utf8_lossy(&doctor_output.stdout);
    assert!(doctor_stdout.contains("latest run graph status: pass (run=vida-a"));
    assert!(doctor_stdout.contains("latest run graph recovery: pass (run=vida-a"));
    assert!(doctor_stdout.contains("latest run graph checkpoint: pass (run=vida-a"));
    assert!(doctor_stdout.contains("latest run graph gate: pass (run=vida-a"));
    assert!(doctor_stdout.contains("checkpoint=execution_cursor"));
    assert!(doctor_stdout.contains("gate=policy_gate_required"));
}

#[test]
fn taskflow_direct_run_surfaces_report_non_empty_bridged_flow_state() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let init = vida()
        .args([
            "taskflow",
            "run-graph",
            "init",
            "vida-a",
            "writer",
            "analysis",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph init should run");
    assert!(init.status.success());

    let update = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-a",
            "writer",
            "writer",
            "ready",
            "analysis",
            "{\"next_node\":\"coach\",\"selected_backend\":\"runtime_selected_tier\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"policy_gate_required\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());

    let run_graph_status = taskflow_run_graph_status_with_timeout(&state_dir, "vida-a", true);
    assert!(run_graph_status.status.success());
    let run_graph_status_stdout = String::from_utf8_lossy(&run_graph_status.stdout);
    let run_graph_status_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_status_stdout).expect("run-graph status should parse");
    assert_eq!(run_graph_status_parsed["run_id"], "vida-a");
    assert_eq!(run_graph_status_parsed["status"]["active_node"], "writer");
    assert_eq!(run_graph_status_parsed["status"]["next_node"], "writer");
    assert_eq!(
        run_graph_status_parsed["status"]["selected_backend"],
        "runtime_selected_tier"
    );

    let recovery_status = taskflow_recovery_status_with_timeout(&state_dir, "vida-a", true);
    assert!(recovery_status.status.success());
    let recovery_status_stdout = String::from_utf8_lossy(&recovery_status.stdout);
    let recovery_status_parsed: serde_json::Value =
        serde_json::from_str(&recovery_status_stdout).expect("recovery status should parse");
    assert_eq!(recovery_status_parsed["run_id"], "vida-a");
    assert_eq!(recovery_status_parsed["recovery"]["resume_node"], "writer");
    assert_eq!(recovery_status_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_status_parsed["recovery"]["policy_gate"],
        "policy_gate_required"
    );

    let checkpoint_status =
        taskflow_recovery_with_timeout(&state_dir, "checkpoint", Some("vida-a"), true);
    assert!(checkpoint_status.status.success());
    let checkpoint_status_stdout = String::from_utf8_lossy(&checkpoint_status.stdout);
    let checkpoint_status_parsed: serde_json::Value =
        serde_json::from_str(&checkpoint_status_stdout).expect("checkpoint status should parse");
    assert_eq!(checkpoint_status_parsed["run_id"], "vida-a");
    assert_eq!(
        checkpoint_status_parsed["checkpoint"]["checkpoint_kind"],
        "execution_cursor"
    );
    assert_eq!(
        checkpoint_status_parsed["checkpoint"]["resume_target"],
        "dispatch.writer_lane"
    );

    let gate_status = taskflow_recovery_with_timeout(&state_dir, "gate", Some("vida-a"), true);
    assert!(gate_status.status.success());
    let gate_status_stdout = String::from_utf8_lossy(&gate_status.stdout);
    let gate_status_parsed: serde_json::Value =
        serde_json::from_str(&gate_status_stdout).expect("gate status should parse");
    assert_eq!(gate_status_parsed["run_id"], "vida-a");
    assert_eq!(
        gate_status_parsed["gate"]["policy_gate"],
        "policy_gate_required"
    );
    assert_eq!(
        gate_status_parsed["gate"]["handoff_state"],
        "awaiting_writer"
    );
    assert_eq!(gate_status_parsed["gate"]["context_state"], "sealed");
}

#[test]
fn taskflow_run_graph_seed_builds_scope_discussion_state_from_configured_agent_system() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-scope",
                "clarify spec scope",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());
    let seed_stdout = String::from_utf8_lossy(&seed.stdout);
    let seed_parsed: serde_json::Value =
        serde_json::from_str(&seed_stdout).expect("run-graph seed json should parse");
    assert_eq!(seed_parsed["surface"], "vida taskflow run-graph seed");
    assert_eq!(seed_parsed["payload"]["status"]["run_id"], "vida-scope");
    assert_eq!(
        seed_parsed["payload"]["status"]["task_class"],
        "scope_discussion"
    );
    assert_eq!(seed_parsed["payload"]["status"]["active_node"], "planning");
    assert_eq!(
        seed_parsed["payload"]["status"]["next_node"],
        "business_analyst"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["route_task_class"],
        "spec-pack"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["selected_backend"],
        "unknown"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["lane_id"],
        "business_analyst_lane"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["handoff_state"],
        "awaiting_business_analyst"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["checkpoint_kind"],
        "conversation_cursor"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["resume_target"],
        "dispatch.business_analyst_lane"
    );
    assert_eq!(
        seed_parsed["payload"]["role_selection"]["conversational_mode"],
        "scope_discussion"
    );

    let recovery = taskflow_recovery_status_with_timeout(&state_dir, "vida-scope", true);
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(
        recovery_parsed["recovery"]["resume_node"],
        "business_analyst"
    );
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
}

#[test]
fn taskflow_run_graph_seed_builds_pbi_discussion_state_from_configured_agent_system() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-pbi",
                "prioritize backlog work pool",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());
    let seed_stdout = String::from_utf8_lossy(&seed.stdout);
    let seed_parsed: serde_json::Value =
        serde_json::from_str(&seed_stdout).expect("run-graph seed json should parse");
    assert_eq!(seed_parsed["surface"], "vida taskflow run-graph seed");
    assert_eq!(seed_parsed["payload"]["status"]["run_id"], "vida-pbi");
    assert_eq!(
        seed_parsed["payload"]["status"]["task_class"],
        "pbi_discussion"
    );
    assert_eq!(seed_parsed["payload"]["status"]["active_node"], "planning");
    assert_eq!(seed_parsed["payload"]["status"]["next_node"], "pm");
    assert_eq!(
        seed_parsed["payload"]["status"]["route_task_class"],
        "work-pool-pack"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["selected_backend"],
        "unknown"
    );
    assert_eq!(seed_parsed["payload"]["status"]["lane_id"], "pm_lane");
    assert_eq!(
        seed_parsed["payload"]["status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["handoff_state"],
        "awaiting_pm"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["checkpoint_kind"],
        "conversation_cursor"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["resume_target"],
        "dispatch.pm_lane"
    );
    assert_eq!(
        seed_parsed["payload"]["role_selection"]["conversational_mode"],
        "pbi_discussion"
    );

    let recovery = taskflow_recovery_status_with_timeout(&state_dir, "vida-pbi", true);
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "pm");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
}

#[test]
fn taskflow_run_graph_seed_builds_implementation_dispatch_state_for_default_route() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());
    let seed_stdout = String::from_utf8_lossy(&seed.stdout);
    let seed_parsed: serde_json::Value =
        serde_json::from_str(&seed_stdout).expect("run-graph seed json should parse");
    assert_eq!(seed_parsed["payload"]["status"]["run_id"], "vida-dev");
    assert_eq!(
        seed_parsed["payload"]["status"]["task_class"],
        "implementation"
    );
    assert_eq!(seed_parsed["payload"]["status"]["active_node"], "planning");
    assert_eq!(seed_parsed["payload"]["status"]["next_node"], "analysis");
    assert_eq!(
        seed_parsed["payload"]["status"]["route_task_class"],
        "implementation"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["selected_backend"],
        "internal_subagents"
    );
    assert_eq!(seed_parsed["payload"]["status"]["lane_id"], "analysis_lane");
    assert_eq!(
        seed_parsed["payload"]["status"]["lifecycle_stage"],
        "implementation_dispatch_ready"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["policy_gate"],
        "validation_report_required"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["handoff_state"],
        "awaiting_analysis"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["checkpoint_kind"],
        "execution_cursor"
    );
    assert_eq!(
        seed_parsed["payload"]["status"]["resume_target"],
        "dispatch.analysis_lane"
    );
    assert_eq!(
        seed_parsed["payload"]["role_selection"]["selected_role"],
        "orchestrator"
    );
    assert_eq!(
        seed_parsed["payload"]["role_selection"]["reason"],
        "auto_no_keyword_match"
    );

    let run_graph = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph status should run")
    });
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["next_node"],
        "analysis"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["policy_gate"],
        "validation_report_required"
    );
}

#[test]
fn taskflow_run_graph_advance_builds_coach_handoff_for_seeded_implementation() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph advance should run")
    });
    assert!(advance.status.success());
    let advance_stdout = String::from_utf8_lossy(&advance.stdout);
    let advance_parsed: serde_json::Value =
        serde_json::from_str(&advance_stdout).expect("run-graph advance json should parse");
    assert_eq!(advance_parsed["surface"], "vida taskflow run-graph advance");
    assert_eq!(advance_parsed["payload"]["status"]["run_id"], "vida-dev");
    assert_eq!(
        advance_parsed["payload"]["status"]["active_node"],
        "analysis"
    );
    assert_eq!(advance_parsed["payload"]["status"]["next_node"], "writer");
    assert_eq!(
        advance_parsed["payload"]["status"]["lane_id"],
        "analysis_lane"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["lifecycle_stage"],
        "analysis_active"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["policy_gate"],
        "targeted_verification"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["handoff_state"],
        "awaiting_writer"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.writer_lane"
    );
}

#[test]
fn taskflow_run_graph_advance_fails_closed_when_compiled_snapshot_lacks_implementation_route() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    overwrite_launcher_activation_snapshot(
        &state_dir,
        serde_json::json!({
            "role_selection": {
                "fallback_role": "orchestrator",
                "mode": "auto"
            },
            "agent_system": {
                "routing": {}
            },
            "autonomous_execution": {}
        }),
    );

    let advance = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph advance should run");
    assert!(!advance.status.success());

    assert!(
        !String::from_utf8_lossy(&advance.stdout).trim().is_empty()
            || !String::from_utf8_lossy(&advance.stderr).trim().is_empty()
    );
}

#[test]
fn taskflow_run_graph_advance_builds_spec_pack_handoff_for_seeded_scope_discussion() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-scope",
                "clarify spec scope",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-scope", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph advance should run")
    });
    assert!(advance.status.success());
    let advance_stdout = String::from_utf8_lossy(&advance.stdout);
    let advance_parsed: serde_json::Value =
        serde_json::from_str(&advance_stdout).expect("run-graph advance json should parse");
    assert_eq!(advance_parsed["surface"], "vida taskflow run-graph advance");
    assert_eq!(advance_parsed["payload"]["status"]["run_id"], "vida-scope");
    assert_eq!(
        advance_parsed["payload"]["status"]["task_class"],
        "scope_discussion"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["active_node"],
        "business_analyst"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["next_node"],
        "spec-pack"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["route_task_class"],
        "spec-pack"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["lane_id"],
        "business_analyst_lane"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["lifecycle_stage"],
        "conversation_active"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["handoff_state"],
        "awaiting_spec-pack"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["checkpoint_kind"],
        "conversation_cursor"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.spec-pack"
    );
}

#[test]
fn taskflow_run_graph_advance_builds_work_pool_pack_handoff_for_seeded_pbi_discussion() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-pbi",
                "prioritize backlog work pool",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-pbi", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph advance should run")
    });
    assert!(advance.status.success());
    let advance_stdout = String::from_utf8_lossy(&advance.stdout);
    let advance_parsed: serde_json::Value =
        serde_json::from_str(&advance_stdout).expect("run-graph advance json should parse");
    assert_eq!(advance_parsed["surface"], "vida taskflow run-graph advance");
    assert_eq!(advance_parsed["payload"]["status"]["run_id"], "vida-pbi");
    assert_eq!(
        advance_parsed["payload"]["status"]["task_class"],
        "pbi_discussion"
    );
    assert_eq!(advance_parsed["payload"]["status"]["active_node"], "pm");
    assert_eq!(
        advance_parsed["payload"]["status"]["next_node"],
        "work-pool-pack"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["route_task_class"],
        "work-pool-pack"
    );
    assert_eq!(advance_parsed["payload"]["status"]["lane_id"], "pm_lane");
    assert_eq!(
        advance_parsed["payload"]["status"]["lifecycle_stage"],
        "conversation_active"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["handoff_state"],
        "awaiting_work-pool-pack"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["checkpoint_kind"],
        "conversation_cursor"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.work-pool-pack"
    );
}

#[test]
fn taskflow_run_graph_advance_updates_status_and_recovery_for_seeded_scope_discussion() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-scope",
                "clarify spec scope",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-scope"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph advance should run")
    });
    assert!(advance.status.success());

    let run_graph = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-scope", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph status should run")
    });
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["active_node"],
        "business_analyst"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["next_node"],
        "spec-pack"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["resume_target"],
        "dispatch.spec-pack"
    );

    let recovery = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-scope", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("recovery status should run")
    });
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "spec-pack");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.spec-pack"
    );
    assert_eq!(
        recovery_parsed["recovery"]["policy_gate"],
        "single_task_scope_required"
    );
}

#[test]
fn taskflow_run_graph_advance_updates_status_and_recovery_for_seeded_pbi_discussion() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-pbi",
                "prioritize backlog work pool",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-pbi"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph advance should run")
    });
    assert!(advance.status.success());

    let run_graph = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-pbi", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph status should run")
    });
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(run_graph_parsed["run_graph_status"]["active_node"], "pm");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["next_node"],
        "work-pool-pack"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["resume_target"],
        "dispatch.work-pool-pack"
    );

    let recovery = taskflow_recovery_status_with_timeout(&state_dir, "vida-pbi", true);
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "work-pool-pack");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.work-pool-pack"
    );
    assert_eq!(
        recovery_parsed["recovery"]["policy_gate"],
        "single_task_scope_required"
    );
}

#[test]
fn taskflow_run_graph_advance_updates_status_and_recovery_for_seeded_implementation() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph advance should run")
    });
    assert!(advance.status.success());

    let run_graph = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph status should run")
    });
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["active_node"],
        "analysis"
    );
    assert_eq!(run_graph_parsed["run_graph_status"]["next_node"], "writer");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["policy_gate"],
        "targeted_verification"
    );

    let recovery = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("recovery status should run")
    });
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "writer");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.writer_lane"
    );
    assert_eq!(
        recovery_parsed["recovery"]["policy_gate"],
        "targeted_verification"
    );
}

#[test]
fn taskflow_run_graph_advance_builds_review_ensemble_handoff_after_coach_for_implementation() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let first_advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("first run-graph advance should run")
    });
    assert!(first_advance.status.success());

    let second_advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("second run-graph advance should run")
    });
    assert!(second_advance.status.success());
    let second_advance_stdout = String::from_utf8_lossy(&second_advance.stdout);
    let second_advance_parsed: serde_json::Value = serde_json::from_str(&second_advance_stdout)
        .expect("second run-graph advance json should parse");
    assert_eq!(
        second_advance_parsed["surface"],
        "vida taskflow run-graph advance"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["run_id"],
        "vida-dev"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["active_node"],
        "writer"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["next_node"],
        "coach"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["route_task_class"],
        "implementation"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["lane_id"],
        "writer_lane"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["lifecycle_stage"],
        "writer_active"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["policy_gate"],
        "review_findings"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["handoff_state"],
        "awaiting_coach"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.coach_lane"
    );
}

#[test]
fn taskflow_run_graph_second_advance_updates_status_and_recovery_for_implementation() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    let first_advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("first run-graph advance should run")
    });
    assert!(first_advance.status.success());

    let second_advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("second run-graph advance should run")
    });
    assert!(second_advance.status.success());

    let run_graph = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph status should run")
    });
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["active_node"],
        "writer"
    );
    assert_eq!(run_graph_parsed["run_graph_status"]["next_node"], "coach");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["policy_gate"],
        "review_findings"
    );

    let recovery = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("recovery status should run")
    });
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "coach");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.coach_lane"
    );
    assert_eq!(
        recovery_parsed["recovery"]["policy_gate"],
        "review_findings"
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["delegated_cycle_open"],
        true
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["delegated_cycle_state"],
        "handoff_pending"
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["continuation_signal"],
        "continue_routing_non_blocking"
    );
}

#[test]
fn taskflow_run_graph_third_advance_enters_review_ensemble_for_implementation() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..2 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-review run-graph advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let third_advance = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("third run-graph advance should run")
    });
    assert!(third_advance.status.success());
    let third_advance_stdout = String::from_utf8_lossy(&third_advance.stdout);
    let third_advance_parsed: serde_json::Value = serde_json::from_str(&third_advance_stdout)
        .expect("third run-graph advance json should parse");
    assert_eq!(
        third_advance_parsed["surface"],
        "vida taskflow run-graph advance"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["active_node"],
        "coach"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["next_node"],
        "review_ensemble"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["lane_id"],
        "coach_lane"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["lifecycle_stage"],
        "coach_active"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["policy_gate"],
        "review_findings"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["handoff_state"],
        "awaiting_review_ensemble"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.review_ensemble"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["recovery_ready"],
        true
    );
    assert_eq!(
        third_advance_parsed["delegation_gate"]["delegated_cycle_open"],
        true
    );
    assert_eq!(
        third_advance_parsed["delegation_gate"]["delegated_cycle_state"],
        "handoff_pending"
    );
    assert_eq!(
        third_advance_parsed["delegation_gate"]["continuation_signal"],
        "continue_routing_non_blocking"
    );
}

#[test]
fn taskflow_run_graph_third_advance_updates_status_and_recovery_for_review_ensemble() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("run-graph advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let run_graph = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph status should run")
    });
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["active_node"],
        "review_ensemble"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["next_node"],
        serde_json::Value::Null
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["lane_id"],
        "review_ensemble_lane"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["handoff_state"],
        "none"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["resume_target"],
        "none"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["recovery_ready"],
        false
    );

    let recovery = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-dev", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("recovery status should run")
    });
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(
        recovery_parsed["recovery"]["resume_node"],
        serde_json::Value::Null
    );
    assert_eq!(recovery_parsed["recovery"]["resume_target"], "none");
    assert_eq!(recovery_parsed["recovery"]["handoff_state"], "none");
    assert_eq!(
        recovery_parsed["recovery"]["policy_gate"],
        "review_findings"
    );
    assert_eq!(recovery_parsed["recovery"]["recovery_ready"], false);
}

#[test]
fn taskflow_run_graph_third_advance_fails_closed_for_wrong_review_handoff() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..2 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-review run-graph advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let corrupt = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "coach",
            "ready",
            "implementation",
            r#"{"next_node":"wrong_review"}"#,
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(corrupt.status.success());

    let output = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("wrong third advance should run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("implementation coach handoff"));
    assert!(stderr.contains("review_ensemble"));
    assert!(stderr.contains("wrong_review"));
}

#[test]
fn taskflow_run_graph_fourth_advance_enters_explicit_approval_wait_after_clean_review() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-approval advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_clean = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "clean",
            "implementation",
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_clean.status.success());

    let fourth_advance = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fourth run-graph advance should run");
    assert!(fourth_advance.status.success());
    let stdout = String::from_utf8_lossy(&fourth_advance.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("fourth run-graph advance json should parse");
    assert_eq!(
        parsed["payload"]["status"]["active_node"],
        "review_ensemble"
    );
    assert_eq!(parsed["payload"]["status"]["status"], "awaiting_approval");
    assert_eq!(
        parsed["payload"]["status"]["lifecycle_stage"],
        "approval_wait"
    );
    assert_eq!(
        parsed["payload"]["status"]["policy_gate"],
        "approval_required"
    );
    assert_eq!(parsed["payload"]["status"]["next_node"], "approval");
    assert_eq!(
        parsed["payload"]["status"]["resume_target"],
        "dispatch.approval"
    );
    assert_eq!(parsed["payload"]["status"]["recovery_ready"], true);
}

#[test]
fn taskflow_run_graph_fifth_advance_updates_status_and_recovery_after_explicit_approval() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-approval advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_clean = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "clean",
            "implementation",
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_clean.status.success());

    let fourth_advance = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("approval-wait advance should run");
    assert!(fourth_advance.status.success());

    let mark_approved = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "approved",
            "implementation",
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("approval update should run");
    assert!(mark_approved.status.success());

    let fifth_advance = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fifth run-graph advance should run");
    assert!(fifth_advance.status.success());

    let run_graph = taskflow_run_graph_status_with_timeout(&state_dir, "vida-dev", true);
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["active_node"],
        "review_ensemble"
    );
    assert_eq!(run_graph_parsed["run_graph_status"]["status"], "completed");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["policy_gate"],
        "not_required"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["resume_target"],
        "none"
    );
    assert_eq!(
        run_graph_parsed["run_graph_status"]["recovery_ready"],
        false
    );
    assert_eq!(
        run_graph_parsed["delegation_gate"]["delegated_cycle_open"],
        false
    );
    assert_eq!(
        run_graph_parsed["delegation_gate"]["local_exception_takeover_gate"],
        "delegated_cycle_clear"
    );
    assert_eq!(
        run_graph_parsed["delegation_gate"]["continuation_signal"],
        "continue_after_reports"
    );

    let recovery = taskflow_recovery_status_with_timeout(&state_dir, "vida-dev", true);
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(
        recovery_parsed["recovery"]["resume_node"],
        serde_json::Value::Null
    );
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "completed");
    assert_eq!(recovery_parsed["recovery"]["policy_gate"], "not_required");
    assert_eq!(recovery_parsed["recovery"]["resume_target"], "none");
    assert_eq!(recovery_parsed["recovery"]["recovery_ready"], false);
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["delegated_cycle_open"],
        false
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["reporting_pause_gate"],
        "closure_candidate"
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["continuation_signal"],
        "continue_after_reports"
    );
}

#[test]
fn taskflow_query_routes_approval_questions_to_run_graph_update() {
    let output = vida()
        .args([
            "taskflow", "query", "how", "do", "I", "approve", "a", "clean", "review?",
        ])
        .output()
        .expect("taskflow approval query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow query answer"));
    assert!(stdout.contains("record-approval"));
    assert!(stdout.contains("vida taskflow run-graph update"));
    assert!(stdout.contains("review_ensemble approved"));
}

#[test]
fn taskflow_run_graph_fourth_advance_fails_closed_for_review_findings() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-findings advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_findings = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "review_findings",
            "implementation",
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_findings.status.success());

    let output = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fourth findings advance should run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("blocked advance json should parse");
    assert_eq!(parsed["incident"]["code"], "run_graph_advance_blocked");
    assert_eq!(
        parsed["blockers"][0]["code"],
        "implementation_review_findings"
    );
    assert_eq!(parsed["blockers"][0]["source"], "run_graph_state");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("review findings require explicit scope/rework resolution"));
    assert!(stderr.contains("review_findings"));
}

#[test]
fn taskflow_run_graph_fourth_advance_fails_closed_for_changed_scope() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-scope advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_changed_scope = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "changed_scope",
            "implementation",
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_changed_scope.status.success());

    let output = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fourth changed-scope advance should run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("review findings require explicit scope/rework resolution"));
    assert!(stderr.contains("changed_scope"));
}

#[test]
fn taskflow_run_graph_fourth_advance_reenters_analysis_for_explicit_rework() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..3 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-rework advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_rework = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "rework_ready",
            "implementation",
            r#"{"next_node":"analysis"}"#,
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_rework.status.success());

    let fourth_advance = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fourth rework advance should run");
    assert!(fourth_advance.status.success());
    let stdout = String::from_utf8_lossy(&fourth_advance.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("fourth rework advance json should parse");
    assert_eq!(parsed["payload"]["status"]["active_node"], "analysis");
    assert_eq!(parsed["payload"]["status"]["next_node"], "writer");
    assert_eq!(parsed["payload"]["status"]["status"], "ready");
    assert_eq!(parsed["payload"]["status"]["lane_id"], "analysis_lane");
    assert_eq!(
        parsed["payload"]["status"]["lifecycle_stage"],
        "analysis_active"
    );
    assert_eq!(
        parsed["payload"]["status"]["policy_gate"],
        "targeted_verification"
    );
    assert_eq!(
        parsed["payload"]["status"]["handoff_state"],
        "awaiting_writer"
    );
    assert_eq!(
        parsed["payload"]["status"]["resume_target"],
        "dispatch.writer_lane"
    );
    assert_eq!(parsed["payload"]["status"]["recovery_ready"], true);
}

#[test]
fn taskflow_run_graph_fourth_rework_advance_updates_status_and_recovery() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-rework advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_rework = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "rework_ready",
            "implementation",
            r#"{"next_node":"analysis"}"#,
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_rework.status.success());

    let fourth_advance = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fourth rework advance should run");
    assert!(fourth_advance.status.success());

    let run_graph = taskflow_run_graph_status_with_timeout(&state_dir, "vida-dev", true);
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["active_node"],
        "analysis"
    );
    assert_eq!(run_graph_parsed["run_graph_status"]["next_node"], "writer");
    assert_eq!(run_graph_parsed["run_graph_status"]["status"], "ready");
    assert_eq!(
        run_graph_parsed["run_graph_status"]["resume_target"],
        "dispatch.writer_lane"
    );
    assert_eq!(run_graph_parsed["run_graph_status"]["recovery_ready"], true);

    let recovery = taskflow_recovery_status_with_timeout(&state_dir, "vida-dev", true);
    assert!(recovery.status.success());
    let recovery_stdout = String::from_utf8_lossy(&recovery.stdout);
    let recovery_parsed: serde_json::Value =
        serde_json::from_str(&recovery_stdout).expect("recovery status json should parse");
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "writer");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.writer_lane"
    );
    assert_eq!(
        recovery_parsed["recovery"]["policy_gate"],
        "targeted_verification"
    );
    assert_eq!(recovery_parsed["recovery"]["recovery_ready"], true);
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["delegated_cycle_open"],
        true
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["delegated_cycle_state"],
        "handoff_pending"
    );
    assert_eq!(
        recovery_parsed["recovery"]["delegation_gate"]["continuation_signal"],
        "continue_routing_non_blocking"
    );
}

#[test]
fn taskflow_run_graph_fourth_rework_advance_fails_closed_for_wrong_target() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let seed = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "run-graph",
                "seed",
                "vida-dev",
                "continue development",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("run-graph seed should run")
    });
    assert!(seed.status.success());

    for step in 0..4 {
        let advance = run_with_retry(|| {
            vida()
                .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-rework advance should run")
        });
        assert!(
            advance.status.success(),
            "advance step {step} should succeed"
        );
    }

    let mark_rework = vida()
        .args([
            "taskflow",
            "run-graph",
            "update",
            "vida-dev",
            "implementation",
            "review_ensemble",
            "rework_ready",
            "implementation",
            r#"{"next_node":"planning"}"#,
        ])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph update should run");
    assert!(mark_rework.status.success());

    let output = vida()
        .args(["taskflow", "run-graph", "advance", "vida-dev", "--json"])
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("fourth wrong-target advance should run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("explicit review rework loop"));
    assert!(stderr.contains("analysis"));
    assert!(stderr.contains("planning"));
}

#[test]
fn taskflow_proxy_help_supports_command_help_form() {
    let output = vida()
        .args(["taskflow", "run-graph", "--help"])
        .output()
        .expect("taskflow command help form should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: run-graph"));
    assert!(stdout.contains("Run-graph is not a second task queue"));
    assert!(stdout.contains("launcher-owned and in-process"));
    assert!(stdout.contains("vida taskflow run-graph seed <task_id> <request_text> [--json]"));
    assert!(stdout.contains("vida taskflow run-graph advance <task_id> [--json]"));
    assert!(stdout.contains("seeded implementation or seeded scope-discussion dispatch"));
    assert!(stdout.contains("vida taskflow run-graph status <task_id>"));
    assert!(stdout.contains("vida taskflow run-graph latest [--json]"));
}

#[test]
fn taskflow_proxy_help_supports_scheduler_command_help_form() {
    let output = vida()
        .args(["taskflow", "scheduler", "--help"])
        .output()
        .expect("taskflow scheduler help form should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: scheduler"));
    assert!(stdout.contains("vida taskflow scheduler dispatch"));
    assert!(stdout.contains("preview-first"));
}

#[test]
fn taskflow_query_help_is_launcher_owned() {
    let output = vida()
        .args(["taskflow", "query", "--help"])
        .output()
        .expect("taskflow query help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow query"));
    assert!(stdout.contains("deterministic and launcher-owned"));
    assert!(stdout.contains("what should I run next?"));
    assert!(stdout.contains(
        "Query/help output is advisory only and does not authorize stopping when a next lawful bounded step is already known."
    ));
}

#[test]
fn taskflow_query_recommends_ready_surface_for_next_step_questions() {
    let output = vida()
        .args(["taskflow", "query", "what", "should", "I", "run", "next?"])
        .output()
        .expect("taskflow query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow query answer"));
    assert!(stdout.contains("next-ready-slice"));
    assert!(stdout.contains("vida task next --json"));
}

#[test]
fn taskflow_query_recommends_doctor_for_health_questions() {
    let output = vida()
        .args([
            "taskflow", "query", "how", "do", "I", "diagnose", "runtime", "health?",
        ])
        .output()
        .expect("taskflow health query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("diagnose-runtime"));
    assert!(stdout.contains("vida taskflow doctor --json"));
}

#[test]
fn taskflow_query_recommends_create_surface_for_new_task_questions() {
    let output = vida()
        .args([
            "taskflow", "query", "how", "do", "I", "create", "a", "new", "task", "under", "this",
            "epic?",
        ])
        .output()
        .expect("taskflow create query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("create-task"));
    assert!(stdout.contains(
        "vida task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description \"...\" --json"
    ));
}

#[test]
fn taskflow_query_recommends_shell_safe_progress_surface_for_update_questions() {
    let output = vida()
        .args([
            "taskflow",
            "query",
            "how",
            "do",
            "I",
            "update",
            "task",
            "progress?",
        ])
        .output()
        .expect("taskflow update query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("record-progress"));
    assert!(stdout
        .contains("vida task update <task-id> --status in_progress --notes-file <path> --json"));
    assert!(stdout.contains("prefer `--notes-file` over inline shell quoting"));
}

#[test]
fn taskflow_query_recommends_next_display_id_surface_for_child_slot_questions() {
    let output = vida()
        .args([
            "taskflow", "query", "how", "do", "I", "get", "the", "next", "display", "id", "for",
            "a", "child", "task?",
        ])
        .output()
        .expect("taskflow display-id query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("next-display-id"));
    assert!(stdout.contains("vida task next-display-id <parent-display-id> --json"));
}

#[test]
fn taskflow_query_recommends_export_surface_for_jsonl_questions() {
    let output = vida()
        .args([
            "taskflow", "query", "how", "do", "I", "export", "the", "backlog", "to", "jsonl?",
        ])
        .output()
        .expect("taskflow export query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("export-runtime-store"));
    assert!(stdout.contains("vida task export-jsonl .vida/exports/tasks.snapshot.jsonl --json"));
}

#[test]
fn taskflow_query_recommends_gate_surface_for_gate_questions() {
    let output = vida()
        .args([
            "taskflow", "query", "how", "do", "I", "inspect", "the", "policy", "gate?",
        ])
        .output()
        .expect("taskflow gate query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("inspect-gate"));
    assert!(stdout.contains("vida taskflow recovery gate <run-id> --json"));
}

#[test]
fn taskflow_query_recommends_latest_recovery_surface_for_latest_questions() {
    let output = vida()
        .args([
            "taskflow", "query", "what", "is", "the", "latest", "recovery", "state?",
        ])
        .output()
        .expect("taskflow latest recovery query should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("inspect-latest-resumability"));
    assert!(stdout.contains("vida taskflow recovery latest --json"));
}

#[test]
fn docflow_proxy_help_is_runtime_specific() {
    let output = vida()
        .args(["docflow", "help"])
        .output()
        .expect("docflow proxy help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA DocFlow runtime family"));
    assert!(stdout.contains("repo/dev binary mode"));
    assert!(stdout.contains("installed mode"));
    assert!(stdout.contains("same in-process Rust DocFlow shell"));
    assert!(stdout.contains("Usage: docflow <COMMAND>"));
    assert!(stdout.contains("registry-write"));
    assert!(stdout.contains("artifact-impact"));
}

#[test]
fn docflow_root_help_surfaces_init_agent_bootstrap_contract() {
    let output = vida()
        .args(["docflow", "--help"])
        .output()
        .expect("docflow root help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida docflow init"));
    assert!(stdout.contains("without positional args prints agent bootstrap instructions"));
    assert!(stdout.contains("machine-readable JSON"));
    assert!(stdout.contains("<markdown_file> <artifact_path> <artifact_type> <change_note>"));
}

#[test]
fn taskflow_proxy_unsupported_top_level_subcommand_fails_closed_without_delegating_to_runtime() {
    let root = unique_state_dir();
    let script_path = format!("{root}/taskflow-proxy.sh");
    fs::create_dir_all(&root).expect("temp root should exist");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let output = vida()
        .args(["taskflow", "passthrough-probe", "--json"])
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("taskflow fail-closed path should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "Unsupported `vida taskflow passthrough-probe` subcommand. This launcher-owned top-level taskflow surface fails closed instead of delegating to the external TaskFlow runtime."
        ),
        "stderr was: {stderr}"
    );
    assert!(
        !stderr.contains("delegated-taskflow-binary-ran"),
        "stderr was: {stderr}"
    );
}

#[test]
fn taskflow_task_unsupported_subcommand_fails_closed_without_delegating_to_runtime() {
    let root = unique_state_dir();
    let script_path = format!("{root}/taskflow-proxy.sh");
    fs::create_dir_all(&root).expect("temp root should exist");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let output = vida()
        .args(["taskflow", "task", "donor-only-probe", "--json"])
        .env("VIDA_ROOT", &root)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("taskflow task fail-closed path should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to the external TaskFlow runtime."
        ),
        "stderr was: {stderr}"
    );
    assert!(
        !stderr.contains("delegated-taskflow-binary-ran"),
        "stderr was: {stderr}"
    );
}

#[test]
fn taskflow_task_ready_routes_through_local_db_bridge_without_taskflow_binary() {
    let root = unique_state_dir();
    let script_path = format!("{root}/delegated-taskflow-runtime");
    let seed_path = format!("{root}/seed.jsonl");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    fs::create_dir_all(&root).expect("temp root should exist");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    scaffold_runtime_project_root(&root, "# framework\n");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let seed = vida()
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
        .output()
        .expect("taskflow seed import should run");
    assert!(seed.status.success());

    let output = vida()
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
        .output()
        .expect("taskflow task ready bridge should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow task ready json should parse");
    assert_eq!(
        parsed
            .as_array()
            .expect("taskflow ready payload should be an array")
            .len(),
        0
    );
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_task_create_routes_through_local_db_bridge_with_display_id_allocation() {
    let root = unique_state_dir();
    let state_dir = format!("{root}/.vida/data/state");
    let script_path = format!("{root}/delegated-taskflow-runtime");
    let seed_path = format!("{root}/seed.jsonl");
    fs::create_dir_all(&root).expect("temp root should exist");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    scaffold_runtime_project_root(&root, "# framework\n");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let seed = vida()
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("taskflow seed import should run");
    assert!(seed.status.success());

    let create_epic = vida()
        .args([
            "taskflow",
            "task",
            "create",
            "vida-root",
            "Root",
            "--type",
            "epic",
            "--display-id",
            "vida-rf1.1",
            "--description",
            "root-epic",
            "--json",
        ])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("taskflow epic create should run");
    assert!(create_epic.status.success());

    let next_display = vida()
        .args([
            "taskflow",
            "task",
            "next-display-id",
            "vida-rf1.1",
            "--json",
        ])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("taskflow next display id should run");
    assert!(next_display.status.success());
    let next_display_stdout = String::from_utf8_lossy(&next_display.stdout);
    let next_display_json: serde_json::Value =
        serde_json::from_str(&next_display_stdout).expect("next display id json should parse");
    assert_eq!(next_display_json["valid"], true);
    let child_display_id = next_display_json["next_display_id"]
        .as_str()
        .expect("next display id should be present")
        .to_string();
    assert_eq!(child_display_id, "vida-rf1.1.1");

    let output = run_with_state_lock_retry(|| {
        vida()
            .args([
                "taskflow",
                "task",
                "create",
                "vida-child",
                "Child",
                "--parent-id",
                "vida-root",
                "--display-id",
                &child_display_id,
                "--description",
                "bridge-task",
                "--json",
            ])
            .current_dir(&root)
            .env("VIDA_ROOT", &root)
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_TASKFLOW_BIN", &script_path)
            .output()
            .expect("taskflow task create bridge should run")
    });

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow task create json should parse");
    assert_eq!(parsed["id"], "vida-child");
    assert_eq!(parsed["status"], "open");
    assert_eq!(parsed["display_id"], child_display_id);
    assert_eq!(parsed["description"], "bridge-task");
    assert_eq!(parsed["dependencies"][0]["depends_on_id"], "vida-root");
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn task_root_mutation_commands_use_authoritative_db_store_without_taskflow_binary() {
    let root = unique_state_dir();
    let state_dir = format!("{root}/.vida/data/state");
    let export_path = format!("{root}/tasks.snapshot.jsonl");
    let script_path = format!("{root}/delegated-taskflow-runtime");
    let seed_path = format!("{root}/seed.jsonl");
    fs::create_dir_all(&root).expect("temp root should exist");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    scaffold_runtime_project_root(&root, "# framework\n");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let seed = vida()
        .args(["task", "import-jsonl", &seed_path, "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("task seed import should run");
    assert!(seed.status.success());
    assert!(!String::from_utf8_lossy(&seed.stderr).contains("delegated-taskflow-binary-ran"));

    let create_epic = vida()
        .args([
            "task",
            "create",
            "vida-root",
            "Root",
            "--type",
            "epic",
            "--display-id",
            "vida-rf1.1",
            "--description",
            "root-epic",
            "--json",
        ])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("task epic create should run");
    assert!(create_epic.status.success());
    assert!(!String::from_utf8_lossy(&create_epic.stderr).contains("delegated-taskflow-binary-ran"));

    let next_display = vida()
        .args(["task", "next-display-id", "vida-rf1.1", "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("task next display id should run");
    assert!(next_display.status.success());
    let next_display_stdout = String::from_utf8_lossy(&next_display.stdout);
    let next_display_json: serde_json::Value =
        serde_json::from_str(&next_display_stdout).expect("next display id json should parse");
    assert_eq!(next_display_json["valid"], true);
    let child_display_id = next_display_json["next_display_id"]
        .as_str()
        .expect("next display id should be present")
        .to_string();
    assert_eq!(child_display_id, "vida-rf1.1.1");
    assert!(
        !String::from_utf8_lossy(&next_display.stderr).contains("delegated-taskflow-binary-ran")
    );

    let create_child = run_with_state_lock_retry(|| {
        vida()
            .args([
                "task",
                "create",
                "vida-child",
                "Child",
                "--parent-id",
                "vida-root",
                "--display-id",
                &child_display_id,
                "--description",
                "root-task",
                "--json",
            ])
            .current_dir(&root)
            .env("VIDA_ROOT", &root)
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_TASKFLOW_BIN", &script_path)
            .output()
            .expect("task create should run")
    });
    assert!(create_child.status.success());
    let create_child_stdout = String::from_utf8_lossy(&create_child.stdout);
    let create_child_json: serde_json::Value =
        serde_json::from_str(&create_child_stdout).expect("task create json should parse");
    assert_eq!(create_child_json["id"], "vida-child");
    assert_eq!(create_child_json["status"], "open");
    assert_eq!(create_child_json["description"], "root-task");
    assert!(
        !String::from_utf8_lossy(&create_child.stderr).contains("delegated-taskflow-binary-ran")
    );

    let ensure_child = run_with_state_lock_retry(|| {
        vida()
            .args([
                "task",
                "ensure",
                "vida-child",
                "Child",
                "--parent-id",
                "vida-root",
                "--display-id",
                &child_display_id,
                "--description",
                "root-task",
                "--json",
            ])
            .current_dir(&root)
            .env("VIDA_ROOT", &root)
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_TASKFLOW_BIN", &script_path)
            .output()
            .expect("task ensure should run")
    });
    assert!(ensure_child.status.success());
    let ensure_child_stdout = String::from_utf8_lossy(&ensure_child.stdout);
    let ensure_child_json: serde_json::Value =
        serde_json::from_str(&ensure_child_stdout).expect("task ensure json should parse");
    assert_eq!(ensure_child_json["id"], "vida-child");
    assert_eq!(ensure_child_json["status"], "open");
    assert_eq!(ensure_child_json["description"], "root-task");
    assert!(
        !String::from_utf8_lossy(&ensure_child.stderr).contains("delegated-taskflow-binary-ran")
    );

    let update = run_with_state_lock_retry(|| {
        vida()
            .args([
                "task",
                "update",
                "vida-child",
                "--status",
                "in_progress",
                "--notes",
                "root-surface-update",
                "--json",
            ])
            .current_dir(&root)
            .env("VIDA_ROOT", &root)
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_TASKFLOW_BIN", &script_path)
            .output()
            .expect("task update should run")
    });
    assert!(update.status.success());
    let update_stdout = String::from_utf8_lossy(&update.stdout);
    let update_json: serde_json::Value =
        serde_json::from_str(&update_stdout).expect("task update json should parse");
    assert_eq!(update_json["status"], "in_progress");
    assert_eq!(update_json["notes"], "root-surface-update");
    assert!(!String::from_utf8_lossy(&update.stderr).contains("delegated-taskflow-binary-ran"));

    let show = vida()
        .args(["task", "show", &child_display_id, "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("task show should run");
    assert!(show.status.success());
    let show_stdout = String::from_utf8_lossy(&show.stdout);
    let show_json: serde_json::Value =
        serde_json::from_str(&show_stdout).expect("task show json should parse");
    assert_eq!(show_json["surface"], "vida task show");
    assert_eq!(show_json["status"], "pass");
    assert_eq!(show_json["task"]["id"], "vida-child");
    assert_eq!(show_json["task"]["display_id"], child_display_id);
    assert_eq!(show_json["task"]["status"], "in_progress");
    assert!(!String::from_utf8_lossy(&show.stderr).contains("delegated-taskflow-binary-ran"));

    let close = run_with_state_lock_retry(|| {
        vida()
            .args(["task", "close", "vida-child", "--reason", "done", "--json"])
            .current_dir(&root)
            .env("VIDA_ROOT", &root)
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_TASKFLOW_BIN", &script_path)
            .output()
            .expect("task close should run")
    });
    assert!(close.status.success());
    let close_stdout = String::from_utf8_lossy(&close.stdout);
    let close_json: serde_json::Value =
        serde_json::from_str(&close_stdout).expect("task close json should parse");
    assert_eq!(close_json["status"], "pass");
    assert_eq!(close_json["task"]["status"], "closed");
    assert_eq!(close_json["task"]["close_reason"], "done");
    assert!(!String::from_utf8_lossy(&close.stderr).contains("delegated-taskflow-binary-ran"));

    let export = vida()
        .args(["task", "export-jsonl", &export_path, "--json"])
        .current_dir(&root)
        .env("VIDA_ROOT", &root)
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .output()
        .expect("task export should run");
    assert!(export.status.success());
    let export_stdout = String::from_utf8_lossy(&export.stdout);
    let export_json: serde_json::Value =
        serde_json::from_str(&export_stdout).expect("task export json should parse");
    assert_eq!(export_json["status"], "pass");
    assert_eq!(export_json["surface"], "vida task export-jsonl");
    assert_eq!(export_json["target_path"], export_path);
    assert_eq!(export_json["shared_fields"]["status"], "pass");
    assert_eq!(export_json["operator_contracts"]["status"], "pass");
    assert!(
        fs::metadata(&export_path).is_ok(),
        "export file should exist"
    );
    assert!(!String::from_utf8_lossy(&export.stderr).contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_proxy_resolves_repo_root_from_nested_project_pwd_without_env() {
    let root = unique_state_dir();
    let project_root = format!("{root}/project");
    let nested_pwd = format!("{project_root}/work/nested");
    let script_path = format!("{project_root}/delegated-taskflow-runtime");
    let seed_path = format!("{project_root}/seed.jsonl");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    scaffold_runtime_project_root(&project_root, "project");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let seed = vida()
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
        .output()
        .expect("taskflow seed import should resolve project root from nested cwd");
    assert!(seed.status.success());

    let output = vida()
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
        .output()
        .expect("taskflow proxy should resolve project root from nested cwd");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow task ready json should parse");
    assert_eq!(
        parsed
            .as_array()
            .expect("taskflow ready payload should be an array")
            .len(),
        0
    );
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_proxy_fails_closed_when_project_root_is_ambiguous() {
    let root = unique_state_dir();
    let outer = format!("{root}/outer");
    let inner = format!("{outer}/inner");
    let nested = format!("{inner}/work");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    scaffold_runtime_project_root(&outer, "outer");
    scaffold_runtime_project_root(&inner, "inner");

    let output = vida()
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&nested)
        .env_remove("VIDA_ROOT")
        .output()
        .expect("taskflow proxy should fail closed on ambiguous root");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Ambiguous VIDA project root"));
}

#[test]
fn installed_vida_resolves_taskflow_binary_from_its_bin_dir_and_project_root_from_pwd() {
    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let project_root = format!("{root}/project");
    let script_path = format!("{install_root}/bin/{}", donor_taskflow_runtime_name());
    let vida_path = format!("{install_root}/bin/vida");
    let helper_path = format!(
        "{install_root}/{}/helpers/turso_task_store.py",
        donor_taskflow_runtime_name()
    );
    let python_path = format!("{install_root}/.venv/bin/python3");
    let nested_pwd = format!("{project_root}/work/nested");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!(
        "{install_root}/{}/helpers",
        donor_taskflow_runtime_name()
    ))
    .expect("install helper dir should exist");
    fs::create_dir_all(format!("{install_root}/.venv/bin"))
        .expect("install python dir should exist");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    scaffold_runtime_project_root(&project_root, "project");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &vida_path);
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    write_executable_script(&python_path, "#!/bin/sh\nexec python3 \"$@\"\n");
    write_file(
        &helper_path,
        r#"#!/usr/bin/env python3
import json
import sys

args = sys.argv[1:]
if len(args) >= 3 and args[0] == "--db" and args[2] == "import-jsonl":
    print(json.dumps({
        "status": "ok",
        "imported_count": 0,
        "unchanged_count": 0,
        "updated_count": 0
    }))
elif len(args) >= 2 and args[0] == "--db" and args[2] == "ready":
    print("[]")
else:
    print(json.dumps({
        "status": "error",
        "reason": "unexpected_args",
        "args": args
    }))
    sys.exit(1)
"#,
    );

    let mut command = Command::new(&vida_path);
    command
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT");
    let output = command_output_with_retry(&mut command);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("installed vida task ready json should parse");
    assert_eq!(
        parsed
            .as_array()
            .expect("ready payload should be an array")
            .len(),
        0
    );
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn installed_vida_ready_filters_blocked_siblings_via_helper_backed_task_store() {
    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let project_root = format!("{root}/project");
    let state_dir = format!("{project_root}/.vida/data/state");
    let script_path = format!("{install_root}/bin/{}", donor_taskflow_runtime_name());
    let vida_path = format!("{install_root}/bin/vida");
    let nested_pwd = format!("{project_root}/work/nested");
    let seed_path = format!("{project_root}/seed.jsonl");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    scaffold_runtime_project_root(&project_root, "project");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &vida_path);
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    fs::write(
        &seed_path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-ready\",\"title\":\"Ready task\",\"description\":\"ready\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-ready\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-blocker\",\"title\":\"Blocker task\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-blocked\",\"title\":\"Blocked task\",\"description\":\"blocked\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-blocked\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-blocked\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("seed jsonl should be written");

    let import = run_with_state_lock_retry(|| {
        Command::new(&vida_path)
            .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
            .current_dir(&nested_pwd)
            .env_remove("VIDA_ROOT")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("installed import should run")
    });
    assert!(import.status.success());

    let ready = run_with_state_lock_retry(|| {
        Command::new(&vida_path)
            .args(["taskflow", "task", "ready", "--json"])
            .current_dir(&nested_pwd)
            .env_remove("VIDA_ROOT")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("installed ready should run")
    });

    assert!(ready.status.success());
    let stdout = String::from_utf8_lossy(&ready.stdout);
    let stderr = String::from_utf8_lossy(&ready.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("installed ready json should parse");
    let rows = parsed
        .as_array()
        .expect("installed ready payload should be an array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "vida-blocker");
    assert_eq!(rows[1]["id"], "vida-ready");
    assert!(rows.iter().any(|row| row["id"] == "vida-ready"));
    assert!(rows.iter().any(|row| row["id"] == "vida-blocker"));
    assert!(!rows.iter().any(|row| row["id"] == "vida-blocked"));
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn installed_vida_ready_orders_multiple_rows_and_filters_blocked_siblings() {
    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let project_root = format!("{root}/project");
    let state_dir = format!("{project_root}/.vida/data/state");
    let script_path = format!("{install_root}/bin/{}", donor_taskflow_runtime_name());
    let vida_path = format!("{install_root}/bin/vida");
    let nested_pwd = format!("{project_root}/work/nested");
    let seed_path = format!("{project_root}/seed-ordering.jsonl");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    scaffold_runtime_project_root(&project_root, "project");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &vida_path);
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    fs::write(
        &seed_path,
        concat!(
            "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-in-progress\",\"title\":\"In progress task\",\"description\":\"active\",\"status\":\"in_progress\",\"priority\":5,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-in-progress\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-blocker\",\"title\":\"Blocker task\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
            "{\"id\":\"vida-ready\",\"title\":\"Ready task\",\"description\":\"ready\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-ready\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
            "{\"id\":\"vida-blocked\",\"title\":\"Blocked task\",\"description\":\"blocked\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-blocked\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-blocked\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
        ),
    )
    .expect("seed ordering jsonl should be written");

    let import = run_with_state_lock_retry(|| {
        Command::new(&vida_path)
            .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
            .current_dir(&nested_pwd)
            .env_remove("VIDA_ROOT")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("installed import should run")
    });
    assert!(import.status.success());

    let ready = run_with_state_lock_retry(|| {
        Command::new(&vida_path)
            .args(["taskflow", "task", "ready", "--json"])
            .current_dir(&nested_pwd)
            .env_remove("VIDA_ROOT")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("installed ready should run")
    });

    assert!(ready.status.success());
    let stdout = String::from_utf8_lossy(&ready.stdout);
    let stderr = String::from_utf8_lossy(&ready.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("installed ready ordering json should parse");
    let rows = parsed
        .as_array()
        .expect("installed ready ordering payload should be an array");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["id"], "vida-in-progress");
    assert_eq!(rows[1]["id"], "vida-blocker");
    assert_eq!(rows[2]["id"], "vida-ready");
    assert!(!rows.iter().any(|row| row["id"] == "vida-blocked"));
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_task_bridge_keeps_missing_in_process_commands_off_delegated_runtime_in_project_and_installed_modes(
) {
    let project_root = format!("{}/project-aware", unique_state_dir());
    let project_state_dir = format!("{project_root}/.vida/data/state");
    let nested_pwd = format!("{project_root}/work/nested");
    let delegated_taskflow_bin = format!("{project_root}/delegated-taskflow-runtime");
    let export_path = format!("{project_root}/export/issues.jsonl");
    let seed_path = format!("{project_root}/seed.jsonl");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    scaffold_runtime_project_root(&project_root, "project");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    write_executable_script(
        &delegated_taskflow_bin,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let project_mode = |args: &[&str]| {
        run_with_state_lock_retry(|| {
            vida()
                .args(args)
                .current_dir(&nested_pwd)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &project_state_dir)
                .env("VIDA_TASKFLOW_BIN", &delegated_taskflow_bin)
                .output()
                .expect("project-aware bridge command should run")
        })
    };

    let project_import = project_mode(&["taskflow", "task", "import-jsonl", &seed_path, "--json"]);
    assert!(project_import.status.success());

    let project_create_epic = project_mode(&[
        "taskflow",
        "task",
        "create",
        "vida-root",
        "Root",
        "--type",
        "epic",
        "--display-id",
        "vida-rf1.1",
        "--description",
        "root-epic",
        "--json",
    ]);
    assert!(project_create_epic.status.success());

    let project_next_display_before = project_mode(&[
        "taskflow",
        "task",
        "next-display-id",
        "vida-rf1.1",
        "--json",
    ]);
    assert!(project_next_display_before.status.success());
    let project_next_display_before_stdout =
        String::from_utf8_lossy(&project_next_display_before.stdout);
    let project_next_display_before_json: serde_json::Value =
        serde_json::from_str(&project_next_display_before_stdout)
            .expect("project next-display-id pre-create json should parse");
    let project_child_display_id = project_next_display_before_json["next_display_id"]
        .as_str()
        .expect("project child display id should exist")
        .to_string();

    let project_create_child = project_mode(&[
        "taskflow",
        "task",
        "create",
        "vida-child",
        "Child",
        "--parent-id",
        "vida-root",
        "--display-id",
        project_child_display_id.as_str(),
        "--description",
        "bridge-task",
        "--json",
    ]);
    assert!(
        project_create_child.status.success(),
        "{}",
        String::from_utf8_lossy(&project_create_child.stderr)
    );

    let project_list = project_mode(&["taskflow", "task", "list", "--all", "--json"]);
    assert!(project_list.status.success());
    let project_list_stdout = String::from_utf8_lossy(&project_list.stdout);
    let project_list_stderr = String::from_utf8_lossy(&project_list.stderr);
    let project_list_json: serde_json::Value =
        serde_json::from_str(&project_list_stdout).expect("project list json should parse");
    let project_rows = task_rows_from_payload(&project_list_json, "project list");
    assert_eq!(project_rows.len(), 2);
    assert!(project_rows.iter().any(|row| row["id"] == "vida-root"));
    assert!(project_rows.iter().any(|row| row["id"] == "vida-child"));
    assert!(!project_list_stderr.contains("delegated-taskflow-binary-ran"));

    let project_show = project_mode(&["taskflow", "task", "show", "vida-child", "--json"]);
    assert!(project_show.status.success());
    let project_show_stdout = String::from_utf8_lossy(&project_show.stdout);
    let project_show_stderr = String::from_utf8_lossy(&project_show.stderr);
    let project_show_json: serde_json::Value =
        serde_json::from_str(&project_show_stdout).expect("project show json should parse");
    let project_show_task = project_show_json.get("task").unwrap_or(&project_show_json);
    assert_eq!(project_show_task["id"], "vida-child");
    assert_eq!(project_show_task["display_id"], project_child_display_id);
    assert_eq!(project_show_task["description"], "bridge-task");
    assert!(!project_show_stderr.contains("delegated-taskflow-binary-ran"));

    let project_update = project_mode(&[
        "taskflow",
        "task",
        "update",
        "vida-child",
        "--status",
        "in_progress",
        "--notes",
        "bridge proof",
        "--json",
    ]);
    let project_update_stdout = String::from_utf8_lossy(&project_update.stdout);
    let project_update_stderr = String::from_utf8_lossy(&project_update.stderr);
    assert!(project_update.status.success(), "{}", project_update_stderr);
    let project_update_json: serde_json::Value =
        serde_json::from_str(&project_update_stdout).expect("project update json should parse");
    let project_update_task = project_update_json
        .get("task")
        .unwrap_or(&project_update_json);
    assert_eq!(project_update_task["id"], "vida-child");
    assert_eq!(project_update_task["status"], "in_progress");
    assert_eq!(project_update_task["notes"], "bridge proof");
    assert!(!project_update_stderr.contains("delegated-taskflow-binary-ran"));

    let project_close = project_mode(&[
        "taskflow",
        "task",
        "close",
        "vida-child",
        "--reason",
        "done",
        "--json",
    ]);
    assert!(project_close.status.success());
    let project_close_stdout = String::from_utf8_lossy(&project_close.stdout);
    let project_close_stderr = String::from_utf8_lossy(&project_close.stderr);
    let project_close_json: serde_json::Value =
        serde_json::from_str(&project_close_stdout).expect("project close json should parse");
    let project_close_task = project_close_json
        .get("task")
        .unwrap_or(&project_close_json);
    assert_eq!(project_close_task["id"], "vida-child");
    assert_eq!(project_close_task["status"], "closed");
    assert_eq!(project_close_task["close_reason"], "done");
    assert!(!project_close_stderr.contains("delegated-taskflow-binary-ran"));

    let project_export =
        project_mode(&["taskflow", "task", "export-jsonl", &export_path, "--json"]);
    assert!(project_export.status.success());
    let project_export_stdout = String::from_utf8_lossy(&project_export.stdout);
    let project_export_stderr = String::from_utf8_lossy(&project_export.stderr);
    let project_export_json: serde_json::Value =
        serde_json::from_str(&project_export_stdout).expect("project export json should parse");
    assert_eq!(project_export_json["status"], "pass");
    assert_eq!(project_export_json["exported_count"], 2);
    assert_eq!(project_export_json["target_path"], export_path);
    let project_exported = fs::read_to_string(&export_path).expect("project export should exist");
    assert!(project_exported.contains("\"id\":\"vida-root\""));
    assert!(project_exported.contains("\"id\":\"vida-child\""));
    assert!(!project_export_stderr.contains("delegated-taskflow-binary-ran"));

    let install_root = format!("{}/install", unique_state_dir());
    let installed_project_root = format!("{install_root}/project");
    let installed_state_dir = format!("{installed_project_root}/.vida/data/state");
    let installed_nested_pwd = format!("{installed_project_root}/work/nested");
    let installed_taskflow_bin = format!("{install_root}/bin/{}", donor_taskflow_runtime_name());
    let installed_vida_bin = format!("{install_root}/bin/vida");
    let installed_export_path = format!("{installed_project_root}/export/issues.jsonl");
    let installed_seed_path = format!("{installed_project_root}/seed.jsonl");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(&installed_nested_pwd).expect("installed nested project dir should exist");
    scaffold_runtime_project_root(&installed_project_root, "project");
    fs::write(&installed_seed_path, "").expect("installed seed jsonl should be written");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &installed_vida_bin);
    write_executable_script(
        &installed_taskflow_bin,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let installed_mode = |args: &[&str]| {
        run_with_state_lock_retry(|| {
            Command::new(&installed_vida_bin)
                .args(args)
                .current_dir(&installed_nested_pwd)
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &installed_state_dir)
                .output()
                .expect("installed bridge command should run")
        })
    };

    let installed_import = installed_mode(&[
        "taskflow",
        "task",
        "import-jsonl",
        &installed_seed_path,
        "--json",
    ]);
    assert!(installed_import.status.success());

    let installed_create_epic = installed_mode(&[
        "taskflow",
        "task",
        "create",
        "vida-root",
        "Root",
        "--type",
        "epic",
        "--display-id",
        "vida-rf1.1",
        "--description",
        "root-epic",
        "--json",
    ]);
    assert!(installed_create_epic.status.success());

    let installed_next_display_before = installed_mode(&[
        "taskflow",
        "task",
        "next-display-id",
        "vida-rf1.1",
        "--json",
    ]);
    assert!(installed_next_display_before.status.success());
    let installed_next_display_before_stdout =
        String::from_utf8_lossy(&installed_next_display_before.stdout);
    let installed_next_display_before_json: serde_json::Value =
        serde_json::from_str(&installed_next_display_before_stdout)
            .expect("installed next-display-id pre-create json should parse");
    let installed_child_display_id = installed_next_display_before_json["next_display_id"]
        .as_str()
        .expect("installed child display id should exist")
        .to_string();

    let installed_create_child = installed_mode(&[
        "taskflow",
        "task",
        "create",
        "vida-child",
        "Child",
        "--parent-id",
        "vida-root",
        "--display-id",
        installed_child_display_id.as_str(),
        "--description",
        "bridge-task",
        "--json",
    ]);
    assert!(installed_create_child.status.success());

    let installed_ready = installed_mode(&["taskflow", "task", "ready", "--json"]);
    assert!(installed_ready.status.success());
    let installed_ready_stdout = String::from_utf8_lossy(&installed_ready.stdout);
    let installed_ready_stderr = String::from_utf8_lossy(&installed_ready.stderr);
    let installed_ready_json: serde_json::Value =
        serde_json::from_str(&installed_ready_stdout).expect("installed ready json should parse");
    let installed_ready_rows = task_rows_from_payload(&installed_ready_json, "installed ready");
    assert_eq!(installed_ready_rows.len(), 1);
    assert_eq!(installed_ready_rows[0]["id"], "vida-child");
    assert_eq!(
        installed_ready_rows[0]["display_id"],
        installed_child_display_id
    );
    assert!(!installed_ready_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_list = installed_mode(&["taskflow", "task", "list", "--all", "--json"]);
    assert!(installed_list.status.success());
    let installed_list_stdout = String::from_utf8_lossy(&installed_list.stdout);
    let installed_list_stderr = String::from_utf8_lossy(&installed_list.stderr);
    let installed_list_json: serde_json::Value =
        serde_json::from_str(&installed_list_stdout).expect("installed list json should parse");
    let installed_rows = task_rows_from_payload(&installed_list_json, "installed list");
    assert_eq!(installed_rows.len(), 2);
    assert!(installed_rows.iter().any(|row| row["id"] == "vida-root"));
    assert!(installed_rows.iter().any(|row| row["id"] == "vida-child"));
    assert!(!installed_list_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_show = installed_mode(&["taskflow", "task", "show", "vida-child", "--json"]);
    assert!(installed_show.status.success());
    let installed_show_stdout = String::from_utf8_lossy(&installed_show.stdout);
    let installed_show_stderr = String::from_utf8_lossy(&installed_show.stderr);
    let installed_show_json: serde_json::Value =
        serde_json::from_str(&installed_show_stdout).expect("installed show json should parse");
    let installed_show_task = installed_show_json
        .get("task")
        .unwrap_or(&installed_show_json);
    assert_eq!(installed_show_task["id"], "vida-child");
    assert_eq!(
        installed_show_task["display_id"],
        installed_child_display_id
    );
    assert_eq!(installed_show_task["description"], "bridge-task");
    assert!(!installed_show_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_update = installed_mode(&[
        "taskflow",
        "task",
        "update",
        "vida-child",
        "--status",
        "in_progress",
        "--notes",
        "bridge proof",
        "--json",
    ]);
    assert!(installed_update.status.success());
    let installed_update_stdout = String::from_utf8_lossy(&installed_update.stdout);
    let installed_update_stderr = String::from_utf8_lossy(&installed_update.stderr);
    let installed_update_json: serde_json::Value =
        serde_json::from_str(&installed_update_stdout).expect("installed update json should parse");
    let installed_update_task = installed_update_json
        .get("task")
        .unwrap_or(&installed_update_json);
    assert_eq!(installed_update_task["id"], "vida-child");
    assert_eq!(installed_update_task["status"], "in_progress");
    assert_eq!(installed_update_task["notes"], "bridge proof");
    assert!(!installed_update_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_close = installed_mode(&[
        "taskflow",
        "task",
        "close",
        "vida-child",
        "--reason",
        "done",
        "--json",
    ]);
    assert!(installed_close.status.success());
    let installed_close_stdout = String::from_utf8_lossy(&installed_close.stdout);
    let installed_close_stderr = String::from_utf8_lossy(&installed_close.stderr);
    let installed_close_json: serde_json::Value =
        serde_json::from_str(&installed_close_stdout).expect("installed close json should parse");
    let installed_close_task = installed_close_json
        .get("task")
        .unwrap_or(&installed_close_json);
    assert_eq!(installed_close_task["id"], "vida-child");
    assert_eq!(installed_close_task["status"], "closed");
    assert_eq!(installed_close_task["close_reason"], "done");
    assert!(!installed_close_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_export = installed_mode(&[
        "taskflow",
        "task",
        "export-jsonl",
        &installed_export_path,
        "--json",
    ]);
    assert!(installed_export.status.success());
    let installed_export_stdout = String::from_utf8_lossy(&installed_export.stdout);
    let installed_export_stderr = String::from_utf8_lossy(&installed_export.stderr);
    let installed_export_json: serde_json::Value =
        serde_json::from_str(&installed_export_stdout).expect("installed export json should parse");
    assert_eq!(installed_export_json["status"], "pass");
    assert_eq!(installed_export_json["exported_count"], 2);
    assert_eq!(installed_export_json["target_path"], installed_export_path);
    let installed_exported =
        fs::read_to_string(&installed_export_path).expect("installed export should exist");
    assert!(installed_exported.contains("\"id\":\"vida-root\""));
    assert!(installed_exported.contains("\"id\":\"vida-child\""));
    assert!(!installed_export_stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_consume_final_fails_closed_when_required_registry_is_missing() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "missing-registry-fail-closed",
        "Missing Registry Fail Closed",
    );
    write_file(
        &format!("{project_root}/vida.config.yaml"),
        r#"project:
  id: temp
agent_extensions:
  enabled: true
  registries:
    roles: missing/roles.yaml
    skills: docs/process/agent-extensions/skills.yaml
    profiles: docs/process/agent-extensions/profiles.yaml
    flows: docs/process/agent-extensions/flows.yaml
  enabled_framework_roles:
    - orchestrator
    - business_analyst
    - pm
    - worker
    - coach
    - verifier
  role_selection:
    mode: auto
    fallback_role: orchestrator
    conversation_modes:
      scope_discussion:
        enabled: true
        role: business_analyst
        tracked_flow_entry: spec-pack
  validation:
    require_registry_files: true
    require_profile_resolution: true
    require_flow_resolution: true
agent_system:
  mode: native
  state_owner: orchestrator_only
"#,
    );
    let boot = vida()
        .arg("boot")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should rerun");
    assert!(!boot.status.success());
    let output_text = String::from_utf8_lossy(&boot.stderr).to_string();
    assert!(output_text.contains("missing/roles.yaml"), "{output_text}");
    assert!(
        output_text.contains("Agent extension bundle validation failed")
            || output_text.contains("registry")
            || output_text.contains("roles"),
        "{output_text}"
    );
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn taskflow_consume_final_fails_closed_for_unresolved_tracked_flow_entry() {
    let (project_root, state_dir) = bootstrap_project_runtime(
        "unresolved-tracked-flow-fail-closed",
        "Unresolved Tracked Flow Fail Closed",
    );
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    write_file(
        &format!("{project_root}/vida.config.yaml"),
        &format!(
            r#"project:
  id: temp
agent_extensions:
  enabled: true
  registries:
    roles: {repo_root}/docs/process/agent-extensions/roles.yaml
    skills: {repo_root}/docs/process/agent-extensions/skills.yaml
    profiles: {repo_root}/docs/process/agent-extensions/profiles.yaml
    flows: {repo_root}/docs/process/agent-extensions/flows.yaml
  enabled_framework_roles:
    - orchestrator
    - business_analyst
    - pm
    - worker
    - coach
    - verifier
  role_selection:
    mode: auto
    fallback_role: orchestrator
    conversation_modes:
      scope_discussion:
        enabled: true
        role: business_analyst
        tracked_flow_entry: missing-pack
  validation:
    require_registry_files: true
    require_profile_resolution: true
    require_flow_resolution: true
agent_system:
  mode: native
  state_owner: orchestrator_only
"#
        ),
    );
    let boot = vida()
        .arg("boot")
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should rerun");
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    sync_protocol_binding(&state_dir);

    let output = vida()
        .args([
            "taskflow",
            "consume",
            "final",
            "clarify spec scope",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");

    assert!(!output.status.success());
    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_text.contains("missing-pack"), "{output_text}");
    assert!(
        output_text.contains("tracked flow")
            || output_text.contains("tracked_flow_entry")
            || output_text.contains("Agent extension bundle validation failed"),
        "{output_text}"
    );
    fs::remove_dir_all(project_root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_fails_closed_for_unsupported_command() {
    let output = vida()
        .args(["docflow", "nonexistent-command"])
        .output()
        .expect("docflow proxy should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unrecognized subcommand"));
}

#[test]
fn docflow_proxy_runs_check_in_process_when_profile_is_supported() {
    let output = vida()
        .args(["docflow", "check", "--profile", "active-canon"])
        .output()
        .expect("docflow in-process check should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
}

#[test]
fn docflow_proxy_runs_fastcheck_in_process_when_profile_is_supported() {
    let output = vida()
        .args(["docflow", "fastcheck", "--profile", "active-canon"])
        .output()
        .expect("docflow in-process fastcheck should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
}

#[test]
fn docflow_proxy_runs_activation_check_in_process_when_profile_is_supported() {
    let output = vida()
        .args(["docflow", "activation-check", "--profile", "active-canon"])
        .output()
        .expect("docflow in-process activation-check should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
}

#[test]
fn docflow_proxy_runs_protocol_coverage_in_process_when_profile_is_supported() {
    let output = vida()
        .args([
            "docflow",
            "protocol-coverage-check",
            "--profile",
            "active-canon",
        ])
        .output()
        .expect("docflow in-process protocol-coverage-check should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
}

#[test]
fn docflow_proxy_runs_readiness_check_in_process_when_profile_is_supported() {
    let output = vida()
        .args(["docflow", "readiness-check", "--profile", "active-canon"])
        .output()
        .expect("docflow in-process readiness-check should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
}

#[test]
fn docflow_proxy_runs_proofcheck_in_process_when_profile_is_supported() {
    let output = vida()
        .args(["docflow", "proofcheck", "--profile", "active-canon-strict"])
        .output()
        .expect("docflow in-process proofcheck should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
}

#[test]
fn docflow_proxy_runs_finalize_edit_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(format!("{root}/docs/process/a.changelog.jsonl"), "").expect("process changelog");

    let output = vida()
        .args([
            "docflow",
            "finalize-edit",
            "docs/process/a.md",
            "promote footer metadata",
            "--status",
            "canonical",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process finalize-edit should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
    assert!(stdout.contains("finalize-edit"));
    let updated = fs::read_to_string(format!("{root}/docs/process/a.md"))
        .expect("updated markdown should exist");
    assert!(updated.contains("status: canonical"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_touch_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(format!("{root}/docs/process/a.changelog.jsonl"), "").expect("process changelog");

    let output = vida()
        .args([
            "docflow",
            "touch",
            "docs/process/a.md",
            "record touch event",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process touch should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
    assert!(stdout.contains("touch"));
    let updated = fs::read_to_string(format!("{root}/docs/process/a.changelog.jsonl"))
        .expect("updated changelog should exist");
    assert!(updated.contains("\"reason\":\"record touch event\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_rename_artifact_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(format!("{root}/docs/process/a.changelog.jsonl"), "").expect("process changelog");

    let output = vida()
        .args([
            "docflow",
            "rename-artifact",
            "docs/process/a.md",
            "process/b",
            "rename artifact id",
            "--bump-version",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process rename-artifact should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
    assert!(stdout.contains("rename-artifact"));
    let updated = fs::read_to_string(format!("{root}/docs/process/a.md"))
        .expect("updated markdown should exist");
    assert!(updated.contains("artifact_path: process/b"));
    assert!(updated.contains("artifact_version: 2"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_init_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");

    let output = vida()
        .args([
            "docflow",
            "init",
            "docs/process/new.md",
            "process/new",
            "process_doc",
            "initialize artifact",
            "--title",
            "New Artifact",
            "--purpose",
            "Boot smoke init",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process init should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
    assert!(stdout.contains("init"));
    let updated =
        fs::read_to_string(format!("{root}/docs/process/new.md")).expect("markdown should exist");
    assert!(updated.contains("artifact_path: process/new"));
    assert!(updated.contains("# New Artifact"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_init_agent_bootstrap_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");

    let output = vida()
        .args(["docflow", "init"])
        .current_dir(&root)
        .output()
        .expect("docflow in-process init bootstrap should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
    assert!(stdout.contains("mode: agent_bootstrap"));
    assert!(stdout.contains("AGENTS.sidecar.md"));
    assert!(stdout.contains("docflow readiness-check --profile active-canon"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_init_agent_bootstrap_json_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");

    let output = vida()
        .args(["docflow", "init", "--json"])
        .current_dir(&root)
        .output()
        .expect("docflow in-process init bootstrap json should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("docflow init --json should render JSON");
    assert_eq!(
        payload.get("mode").and_then(|value| value.as_str()),
        Some("agent_bootstrap")
    );
    assert!(payload
        .pointer("/agent_startup/safe_first_commands")
        .and_then(|value| value.as_array())
        .is_some());
    assert!(payload.get("next_actions").is_some());

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_move_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/process/a.changelog.jsonl"),
        "{\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\"}\n",
    )
    .expect("process changelog");

    let output = vida()
        .args([
            "docflow",
            "move",
            "docs/process/a.md",
            "docs/product/spec/a.md",
            "move artifact",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process move should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docflow-proxy:"));
    assert!(stdout.contains("move"));
    let updated = fs::read_to_string(format!("{root}/docs/product/spec/a.md"))
        .expect("destination markdown should exist");
    assert!(updated.contains("source_path: docs/product/spec/a.md"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_changelog_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/process/a.changelog.jsonl"),
        "{\"ts\":\"2026-03-11T00:00:00Z\",\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\"}\n",
    )
    .expect("process changelog");

    let output = vida()
        .args([
            "docflow",
            "changelog",
            "docs/process/a.md",
            "--format",
            "jsonl",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process changelog should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"event\":\"artifact_initialized\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_task_summary_surfaces_in_process_when_supported() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/process/a.changelog.jsonl"),
        "{\"ts\":\"2026-03-11T00:00:00Z\",\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\",\"task_id\":\"vida-rf1\",\"actor\":\"codex\",\"scope\":\"bridge\",\"tags\":[\"docflow\"]}\n",
    )
    .expect("process changelog");

    let changelog_task = vida()
        .args([
            "docflow",
            "changelog-task",
            "--root",
            &root,
            "--task-id",
            "vida-rf1",
            "--format",
            "jsonl",
        ])
        .output()
        .expect("docflow in-process changelog-task should run");
    assert!(changelog_task.status.success());
    let changelog_stdout = String::from_utf8_lossy(&changelog_task.stdout);
    assert!(changelog_stdout.contains("\"task_id\":\"vida-rf1\""));

    let summary = vida()
        .args([
            "docflow",
            "task-summary",
            "--root",
            &root,
            "--task-id",
            "vida-rf1",
            "--format",
            "jsonl",
        ])
        .output()
        .expect("docflow in-process task-summary should run");
    assert!(summary.status.success());
    let summary_stdout = String::from_utf8_lossy(&summary.stdout);
    assert!(summary_stdout.contains("\"summary\":\"task\""));
    assert!(summary_stdout.contains("\"summary\":\"actor\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_runs_migrate_links_in_process_when_supported() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n[Link](b.md)\n\n-----\nartifact_path: process/a\nartifact_type: process_doc\nartifact_version: 1\nartifact_revision: old\nsource_path: docs/process/a.md\nstatus: draft\nchangelog_ref: a.changelog.jsonl\ncreated_at: 2026-03-11T00:00:00Z\nupdated_at: 2026-03-11T00:00:00Z\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/process/a.changelog.jsonl"),
        "{\"event\":\"artifact_initialized\",\"artifact_path\":\"process/a\"}\n",
    )
    .expect("process changelog");
    fs::write(format!("{root}/docs/process/c.md"), "# c\n").expect("new target should exist");

    let output = vida()
        .args([
            "docflow",
            "migrate-links",
            "docs/process/a.md",
            "b.md",
            "c.md",
            "rewrite links",
            "--format",
            "jsonl",
        ])
        .current_dir(&root)
        .output()
        .expect("docflow in-process migrate-links should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("quiet validation failed"));
    let updated = fs::read_to_string(format!("{root}/docs/process/a.md"))
        .expect("updated markdown should exist");
    assert!(updated.contains("[Link](c.md)"));
    let changelog = fs::read_to_string(format!("{root}/docs/process/a.changelog.jsonl"))
        .expect("updated changelog should exist");
    assert!(changelog.contains("\"event\":\"links_migrated\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_use_rust_cli_shell() {
    let output = vida()
        .args([
            "docflow",
            "overview",
            "--registry-count",
            "5",
            "--relation-count",
            "2",
        ])
        .output()
        .expect("docflow rust shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docflow overview"));
    assert!(stdout.contains("registry_rows: 5"));
    assert!(stdout.contains("relation_edges: 2"));
}

#[test]
fn docflow_proxy_can_run_rust_layer_status_surface() {
    let output = vida()
        .args(["docflow", "layer-status", "--layer", "6"])
        .output()
        .expect("docflow rust layer-status shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("layer-status"));
    assert!(stdout.contains("layer: 6"));
    assert!(stdout.contains("Layer name: Canonical Operator"));
    assert!(stdout.contains("Status: ✅"));
}

#[test]
fn docflow_proxy_can_run_rust_summary_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");
    fs::write(format!("{root}/docs/product/spec/b.md"), "# b\n").expect("spec markdown");

    let output = vida()
        .args(["docflow", "summary", "--root", &root])
        .output()
        .expect("docflow rust summary shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("summary"));
    assert!(stdout.contains("registry_rows: 2"));
    assert!(stdout.contains("relation_edges: 2"));
    assert!(stdout.contains("readiness: blocking"));
    assert!(stdout.contains("type[process_doc]: 1"));
    assert!(stdout.contains("type[product_spec]: 1"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_scan_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "scan", "--root", &root])
        .output()
        .expect("docflow rust scan shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"artifact_path\":\"docs/process/a.md\""));
    assert!(stdout.contains("\"artifact_type\":\"process_doc\""));
    assert!(stdout.contains("\"has_footer\":false"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_fastcheck_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "fastcheck", "--root", &root])
        .output()
        .expect("docflow rust fastcheck shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"artifact_path\":\"docs/process/a.md\""));
    assert!(stdout.contains("\"code\":\"missing_footer\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_doctor_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "doctor", "--root", &root])
        .output()
        .expect("docflow rust doctor shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"severity\":\"error\""));
    assert!(stdout.contains("\"path\":\"docs/process/a.md\""));
    assert!(stdout.contains("missing_footer"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_activation_check_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/vida/config/instructions"))
        .expect("instructions dir should be created");
    fs::write(
        format!("{root}/vida/config/instructions/runtime-instructions.synthetic-protocol.md"),
        "# synthetic\n",
    )
    .expect("protocol markdown");

    let output = vida()
        .args(["docflow", "activation-check", "--root", &root])
        .output()
        .expect("docflow rust activation-check shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "\"path\":\"vida/config/instructions/runtime-instructions.synthetic-protocol.md\""
    ));
    assert!(stdout.contains("missing_activation_binding"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_protocol_coverage_check_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/vida/config/instructions"))
        .expect("instructions dir should be created");
    fs::write(
        format!("{root}/vida/config/instructions/runtime-instructions.synthetic-protocol.md"),
        "# synthetic\n",
    )
    .expect("protocol markdown");

    let output = vida()
        .args(["docflow", "protocol-coverage-check", "--root", &root])
        .output()
        .expect("docflow rust protocol-coverage-check shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "\"path\":\"vida/config/instructions/runtime-instructions.synthetic-protocol.md\""
    ));
    assert!(stdout.contains("missing_activation_binding"));
    assert!(stdout.contains("missing_protocol_index_binding"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_proofcheck_surface() {
    let output = vida()
        .args(["docflow", "proofcheck", "--layer", "6"])
        .output()
        .expect("docflow rust proofcheck shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("proofcheck"));
    assert!(stdout.contains("layer: 6"));
    assert!(stdout.contains("files_mode: layer"));
    assert!(stdout.contains("fastcheck_rows:"));
    assert!(stdout.contains("protocol_coverage_rows:"));
    assert!(stdout.contains("readiness_rows:"));
    assert!(stdout.contains("doctor_error_rows:"));
}

#[test]
fn docflow_proxy_can_run_rust_validation_surface() {
    let output = vida()
        .args([
            "docflow",
            "validate-footer",
            "--path",
            "docs/process/test.md",
            "--content",
            "# title\n",
        ])
        .output()
        .expect("docflow rust validation shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validation"));
    assert!(stdout.contains("verdict: blocking"));
    assert!(stdout.contains("missing_footer"));
    assert!(stdout.contains("missing_project_doc_map_registration"));
}

#[test]
fn docflow_proxy_can_run_rust_readiness_surface() {
    let output = vida()
        .args([
            "docflow",
            "readiness",
            "--path",
            "docs/process/test.md",
            "--content",
            "# title\n",
        ])
        .output()
        .expect("docflow rust readiness shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("readiness"));
    assert!(stdout.contains("verdict: blocking"));
    assert!(stdout.contains("docs/process/test.md [blocking]"));
}

#[test]
fn docflow_proxy_can_run_rust_check_file_surface() {
    let path = format!("/tmp/vida-docflow-check-file-{}.md", std::process::id());
    fs::write(&path, "# title\n").expect("temp markdown should be written");

    let output = vida()
        .args(["docflow", "check-file", "--path", &path])
        .output()
        .expect("docflow rust check-file shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validation"));
    assert!(stdout.contains("issues: 1"));
    assert!(stdout.contains("[missing_footer]"));

    fs::remove_file(path).expect("temp markdown should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_readiness_file_surface() {
    let path = format!("/tmp/vida-docflow-readiness-file-{}.md", std::process::id());
    fs::write(&path, "# title\n").expect("temp markdown should be written");

    let output = vida()
        .args(["docflow", "readiness-file", "--path", &path])
        .output()
        .expect("docflow rust readiness-file shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("readiness"));
    assert!(stdout.contains("rows: 1"));
    assert!(stdout.contains("verdict: blocking"));

    fs::remove_file(path).expect("temp markdown should be removed");
}

#[test]
fn docflow_proxy_can_run_report_check_surface() {
    let path = format!("/tmp/vida-docflow-report-check-{}.log", std::process::id());
    fs::write(
        &path,
        "Thinking mode: STC.\nRequests: active=1 | in_work=1 | blocked=0\nAgents: active=0 | working=0 | waiting=0\n",
    )
    .expect("temp report should be written");

    let output = vida()
        .args(["docflow", "report-check", "--path", &path])
        .output()
        .expect("docflow rust report-check shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reporting"));
    assert!(stdout.contains("verdict: ok"));

    fs::remove_file(path).expect("temp report should be removed");
}

#[test]
fn docflow_proxy_prefers_active_repo_root_over_stale_env_root() {
    let foreign_root = unique_state_dir();
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    fs::create_dir_all(format!("{foreign_root}/docs/process")).expect("foreign process dir");
    fs::write(
        format!("{foreign_root}/docs/process/foreign.md"),
        "# foreign\n-----\nartifact_path: foreign\nartifact_type: process_doc\nartifact_version: '1'\nartifact_revision: '1'\nschema_version: '1'\nstatus: canonical\nsource_path: docs/process/foreign.md\ncreated_at: '2026-03-14T00:00:00+02:00'\nupdated_at: '2026-03-14T00:00:00+02:00'\nchangelog_ref: foreign.changelog.jsonl\n",
    )
    .expect("foreign markdown should be written");

    let output = vida()
        .current_dir(repo_root)
        .env("VIDA_ROOT", &foreign_root)
        .args([
            "docflow",
            "check-file",
            "--path",
            "docs/process/documentation-tooling-map.md",
        ])
        .output()
        .expect("docflow proxy should run from active repo root");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validation"));
    assert!(!stdout.contains("read_error"));
    assert!(!stdout.contains("foreign.md"));

    fs::remove_dir_all(foreign_root).expect("foreign root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_registry_scan_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");
    fs::write(format!("{root}/docs/product/spec/b.md"), "# b\n").expect("spec markdown");

    let output = vida()
        .args(["docflow", "registry-scan", "--root", &root])
        .output()
        .expect("docflow rust registry-scan shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("registry"));
    assert!(stdout.contains("total_rows: 2"));
    assert!(stdout.contains("docs/process/a.md [process_doc]"));
    assert!(stdout.contains("docs/product/spec/b.md [product_spec]"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_registry_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "registry", "--root", &root])
        .output()
        .expect("docflow rust registry shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"artifact_path\":\"docs/process/a.md\""));
    assert!(stdout.contains("\"artifact_type\":\"process_doc\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_registry_write_surface() {
    let root = unique_state_dir();
    let output = format!("{root}/registry.jsonl");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output_run = vida()
        .args([
            "docflow",
            "registry-write",
            "--root",
            &root,
            "--output",
            &output,
        ])
        .output()
        .expect("docflow rust registry-write shell should run");

    assert!(output_run.status.success());
    let stdout = String::from_utf8_lossy(&output_run.stdout);
    assert!(stdout.contains("registry-write"));
    assert!(stdout.contains("total_rows: 1"));
    assert!(stdout.contains(&format!("output: {output}")));

    let written = fs::read_to_string(&output).expect("registry jsonl should be written");
    assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_registry_write_canonical_surface() {
    let root = unique_state_dir();
    let output = format!("{root}/vida/config/docflow-registry.current.jsonl");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output_run = vida()
        .args(["docflow", "registry-write", "--root", &root, "--canonical"])
        .output()
        .expect("docflow rust registry-write canonical shell should run");

    assert!(output_run.status.success());
    let stdout = String::from_utf8_lossy(&output_run.stdout);
    assert!(stdout.contains("registry-write"));
    assert!(stdout.contains(&format!("output: {output}")));
    let written = fs::read_to_string(&output).expect("canonical registry jsonl should be written");
    assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_overview_scan_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");
    fs::write(format!("{root}/docs/product/spec/b.md"), "# b\n").expect("spec markdown");

    let output = vida()
        .args(["docflow", "overview-scan", "--root", &root])
        .output()
        .expect("docflow rust overview-scan shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docflow overview"));
    assert!(stdout.contains("registry_rows: 2"));
    assert!(stdout.contains("relation_edges: 2"));
    assert!(stdout.contains("readiness: ok"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_relations_scan_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");
    fs::write(format!("{root}/docs/product/spec/b.md"), "# b\n").expect("spec markdown");

    let output = vida()
        .args(["docflow", "relations-scan", "--root", &root])
        .output()
        .expect("docflow rust relations-scan shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("relations"));
    assert!(stdout.contains("total_edges: 2"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_validate_tree_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "validate-tree", "--root", &root])
        .output()
        .expect("docflow rust validate-tree shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validation-tree"));
    assert!(stdout.contains("scanned_rows: 1"));
    assert!(stdout.contains("verdict: blocking"));
    assert!(stdout.contains("docs/process/a.md"));
    assert!(stdout.contains("missing_footer"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_readiness_tree_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "readiness-tree", "--root", &root])
        .output()
        .expect("docflow rust readiness-tree shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("readiness-tree"));
    assert!(stdout.contains("scanned_rows: 1"));
    assert!(stdout.contains("rows: 1"));
    assert!(stdout.contains("verdict: blocking"));
    assert!(stdout.contains("docs/process/a.md [blocking]"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_readiness_check_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output = vida()
        .args(["docflow", "readiness-check", "--root", &root])
        .output()
        .expect("docflow rust readiness-check shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"artifact_path\":\"docs/process/a.md\""));
    assert!(stdout.contains("\"verdict\":\"blocking\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_readiness_write_surface() {
    let root = unique_state_dir();
    let output = format!("{root}/readiness.jsonl");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output_run = vida()
        .args([
            "docflow",
            "readiness-write",
            "--root",
            &root,
            "--output",
            &output,
        ])
        .output()
        .expect("docflow rust readiness-write shell should run");

    assert!(output_run.status.success());
    let stdout = String::from_utf8_lossy(&output_run.stdout);
    assert!(stdout.contains("readiness-write"));
    assert!(stdout.contains(&format!("output: {output}")));

    let written = fs::read_to_string(&output).expect("readiness jsonl should be written");
    assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));
    assert!(written.contains("\"verdict\":\"blocking\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_readiness_write_canonical_surface() {
    let root = unique_state_dir();
    let output = format!("{root}/vida/config/docflow-readiness.current.jsonl");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::write(format!("{root}/docs/process/a.md"), "# a\n").expect("process markdown");

    let output_run = vida()
        .args(["docflow", "readiness-write", "--root", &root, "--canonical"])
        .output()
        .expect("docflow rust readiness-write canonical shell should run");

    assert!(output_run.status.success());
    let stdout = String::from_utf8_lossy(&output_run.stdout);
    assert!(stdout.contains("readiness-write"));
    assert!(stdout.contains(&format!("output: {output}")));
    let written = fs::read_to_string(&output).expect("canonical readiness jsonl should be written");
    assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_links_surface() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n[b](../product/spec/b.md)\n\n-----\nartifact_path: docs/process/a.md\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/product/spec/b.md"),
        "# b\n\n-----\nartifact_path: docs/product/spec/b.md\n",
    )
    .expect("spec markdown");

    let output = vida()
        .args(["docflow", "links", "--path", "docs/process/a.md"])
        .current_dir(&root)
        .output()
        .expect("docflow rust links shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"path\":\"docs/process/a.md\""));
    assert!(stdout.contains("\"target\":\"../product/spec/b.md\""));
    assert!(stdout.contains("\"resolved\":\"docs/product/spec/b.md\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_deps_map_surface() {
    let root = unique_state_dir();
    scaffold_runtime_project_root(&root, "project");
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n[b](../product/spec/b.md)\n\n-----\nartifact_path: docs/process/a.md\nprojection_ref: docs/product/spec/b.md\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/product/spec/b.md"),
        "# b\n\n-----\nartifact_path: docs/product/spec/b.md\n",
    )
    .expect("spec markdown");

    let output = vida()
        .args(["docflow", "deps-map", "--path", "docs/process/a.md"])
        .current_dir(&root)
        .output()
        .expect("docflow rust deps-map shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"edge_type\":\"markdown_link\""));
    assert!(stdout.contains("\"edge_type\":\"projection_ref\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_artifact_impact_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n[b](../product/spec/b.md)\n\n-----\nartifact_path: docs/process/a.md\nprojection_ref: docs/product/spec/b.md\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/product/spec/b.md"),
        "# b\n\n-----\nartifact_path: docs/product/spec/b.md\n",
    )
    .expect("spec markdown");

    let output = vida()
        .args([
            "docflow",
            "artifact-impact",
            "--artifact",
            "docs/product/spec/b.md",
            "--root",
            &root,
            "--format",
            "jsonl",
        ])
        .output()
        .expect("docflow rust artifact-impact shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"command\":\"artifact-impact\""));
    assert!(stdout.contains("\"path\":\"docs/process/a.md\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_task_impact_surface() {
    let root = unique_state_dir();
    fs::create_dir_all(format!("{root}/docs/process")).expect("process dir should be created");
    fs::create_dir_all(format!("{root}/docs/product/spec")).expect("spec dir should be created");
    fs::write(
        format!("{root}/docs/process/a.md"),
        "# a\n\n[b](../product/spec/b.md)\n\n-----\nartifact_path: docs/process/a.md\n",
    )
    .expect("process markdown");
    fs::write(
        format!("{root}/docs/process/a.changelog.jsonl"),
        "{\"task_id\":\"vida-rf1.2.6\",\"artifact_path\":\"docs/process/a.md\"}\n",
    )
    .expect("process changelog");
    fs::write(
        format!("{root}/docs/product/spec/b.md"),
        "# b\n\n-----\nartifact_path: docs/product/spec/b.md\ncontract_ref: docs/process/a.md\n",
    )
    .expect("spec markdown");

    let output = vida()
        .args([
            "docflow",
            "task-impact",
            "--task-id",
            "vida-rf1.2.6",
            "--root",
            &root,
            "--format",
            "jsonl",
        ])
        .output()
        .expect("docflow rust task-impact shell should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"command\":\"task-impact\""));
    assert!(stdout.contains("\"source_artifact\":\"docs/process/a.md\""));
    assert!(stdout.contains("\"path\":\"docs/product/spec/b.md\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn installed_docflow_mode_supports_overview_surface() {
    let (root, mut command) = installed_vida();
    let output = command_output_with_retry(command.args([
        "docflow",
        "overview",
        "--registry-count",
        "5",
        "--relation-count",
        "2",
    ]));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docflow overview"));
    assert!(stdout.contains("registry_rows: 5"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn installed_docflow_mode_supports_mutation_and_check_commands() {
    let (root, _) = installed_vida();
    let binary = format!("{root}/vida-install/bin/vida");
    let design_doc = format!("{root}/docs/product/spec/example-design.md");

    let init = command_output_with_retry(Command::new(&binary).current_dir(&root).args([
        "docflow",
        "init",
        &design_doc,
        "product/spec/example-design",
        "product_spec",
        "initialize example design",
    ]));
    assert!(
        init.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(std::path::Path::new(&design_doc).is_file());

    let finalize = command_output_with_retry(Command::new(&binary).current_dir(&root).args([
        "docflow",
        "finalize-edit",
        &design_doc,
        "record bounded feature design",
    ]));
    assert!(
        finalize.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&finalize.stderr)
    );

    let check = command_output_with_retry(Command::new(&binary).current_dir(&root).args([
        "docflow",
        "check-file",
        "--path",
        &design_doc,
    ]));
    assert!(
        check.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn memory_surface_reports_effective_bundle() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = run_command_with_state_lock_retry(|| {
        let mut command = Command::new("timeout");
        command
            .args(["-k", "5s", "20s"])
            .arg(env!("CARGO_BIN_EXE_vida"))
            .arg("memory")
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida memory"));
    assert!(stdout.contains("effective instruction bundle root: framework-agent-definition"));
    assert!(stdout.contains("mandatory chain: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains("source version tuple: framework-agent-definition@v1, framework-instruction-contract@v1, framework-prompt-template-config@v1"));
    assert!(stdout.contains("receipt: not-persisted"));
}

#[test]
fn memory_surface_supports_color_render_mode() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["memory", "--render", "color"])
            .env("VIDA_STATE_DIR", &state_dir);
        command
    });
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida memory\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34meffective instruction bundle root\u{1b}[0m"));
}

#[test]
fn memory_surface_cli_render_overrides_env() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .args(["memory", "--render", "color_emoji"])
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_RENDER", "plain");
        command
    });
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("📘 vida memory"));
    assert!(stdout.contains("🔹"));
}

#[test]
fn memory_surface_fails_closed_on_invalid_render_env() {
    let state_dir = unique_state_dir();
    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = run_command_with_state_lock_retry(|| {
        let mut command = vida();
        command
            .arg("memory")
            .env("VIDA_STATE_DIR", &state_dir)
            .env("VIDA_RENDER", "invalid_mode");
        command
    });
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value") || stderr.contains("possible values"));
}

#[test]
fn memory_surface_fails_closed_on_uninitialized_state_dir() {
    let state_dir = unique_state_dir();

    let output = vida()
        .arg("memory")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("memory should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authoritative state directory is missing"));
    assert!(stderr.contains("VIDA_STATE_DIR=<temp-dir>"));
}

#[test]
fn memory_surface_fails_closed_when_governance_linkage_missing() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    seed_run_graph_status(
        &state_dir,
        "run-memory-guard",
        "memory_delete_required",
        "awaiting_coach",
        "sealed",
    );
    let output = memory_output_with_timeout_retry(&state_dir);
    assert!(!output.status.success());

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("memory governance")
            || combined.contains("run_graph")
            || combined.contains("Missing task")
            || combined.contains("LOCK is already locked"),
        "expected governance-related failure output, got: {combined}"
    );
}

#[test]
fn status_surface_reports_backend_and_bundle_receipt() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida status"));
    assert!(stdout.contains("backend: kv-surrealkv state-v1 instruction-v1"));
    assert!(stdout.contains(
        "state spine: initialized (state-v1, 8 entity surfaces, mutation root vida task)"
    ));
    assert!(stdout
        .contains("latest effective bundle receipt: effective-bundle-framework-agent-definition-"));
    assert!(stdout.contains("latest effective bundle root: framework-agent-definition"));
    assert!(stdout.contains("latest effective bundle artifact count: 3"));
    assert!(stdout.contains("boot compatibility: backward_compatible (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration state: backward_compatible / no_migration_required (normal_boot_allowed)"
    ));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
}

#[test]
fn status_surface_supports_color_emoji_render_mode_via_env() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "color_emoji")
        .output()
        .expect("status should run with color emoji render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("📘 vida status"));
    assert!(stdout.contains("✅") || stdout.contains("🔹"));
}

#[test]
fn status_surface_fails_closed_on_uninitialized_state_dir() {
    let state_dir = unique_state_dir();

    let output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authoritative state directory is missing"));
    assert!(stderr.contains("VIDA_STATE_DIR=<temp-dir>"));
}

#[test]
fn doctor_surface_reports_integrity_checks() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida doctor"));
    assert!(stdout.contains("storage metadata: pass (kv-surrealkv state-v1 instruction-v1)"));
    assert!(stdout.contains(
        "authoritative state spine: pass (state-v1, 8 entity surfaces, mutation root vida task)"
    ));
    assert!(stdout
        .contains("task store: pass (0 total, 0 open, 0 in_progress, 0 closed, 0 epics, 0 ready)"));
    assert!(stdout.contains(
        "run graph: pass (execution_plans=0, routed_runs=0, governance=0, resumability=0, reconciliation=0)"
    ));
    assert!(stdout.contains("latest run graph status: pass (none)"));
    assert!(stdout.contains("latest run graph recovery: pass (none)"));
    assert!(stdout.contains("latest run graph checkpoint: pass (none)"));
    assert!(stdout.contains("latest run graph gate: pass (none)"));
    assert!(stdout.contains("launcher/runtime paths: pass (vida="));
    assert!(stdout.contains("project_root="));
    assert!(stdout.contains("taskflow_surface=vida taskflow"));
    assert!(stdout.contains("dependency graph: pass (0 issues)"));
    assert!(stdout.contains("boot compatibility: pass (backward_compatible (normal_boot_allowed))"));
    assert!(stdout.contains(
        "migration preflight: pass (backward_compatible / no_migration_required (normal_boot_allowed))"
    ));
    assert!(stdout.contains(
        "migration receipts: pass (compatibility=1, application=0, verification=0, cutover=0, rollback=0)"
    ));
    assert!(stdout.contains("task reconciliation: pass (none)"));
    assert!(stdout.contains("task reconciliation rollup: pass (0 receipts)"));
    assert!(stdout.contains("taskflow snapshot bridge: pass (idle (no snapshot bridge receipts))"));
    assert!(stdout.contains("effective instruction bundle: pass (framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config)"));
}

#[test]
fn status_surface_supports_json_summary() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status json should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("status json should parse");
    assert_eq!(parsed["surface"], "vida status");
    assert!(parsed["trace_id"].is_null());
    assert!(parsed["workflow_class"].is_null());
    assert!(parsed["risk_tier"].is_null());
    assert!(
        parsed["host_agents"].is_object(),
        "host_agents must always be an object in status json"
    );
    assert_eq!(
        parsed["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
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
    let status = parsed["status"]
        .as_str()
        .expect("status should be a string");
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array");
    let next_actions = parsed["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    match status {
        "pass" => {
            assert!(
                blocker_codes.is_empty(),
                "status=pass must not include blocker_codes"
            );
            assert!(
                next_actions.is_empty(),
                "status=pass must not include next_actions"
            );
        }
        "blocked" => {
            assert!(
                !blocker_codes.is_empty(),
                "status=blocked must include blocker_codes"
            );
            assert!(
                !next_actions.is_empty(),
                "status=blocked must include next_actions"
            );
        }
        other => panic!("unsupported status value in release-1 operator contract: {other}"),
    }
    assert_eq!(parsed["storage_metadata"]["engine"], "surrealdb");
    assert_eq!(parsed["storage_metadata"]["backend"], "kv-surrealkv");
    assert_eq!(parsed["storage_metadata"]["namespace"], "vida");
    assert_eq!(parsed["storage_metadata"]["database"], "primary");
    assert_eq!(parsed["state_spine"]["state_schema_version"], 1);
    assert_eq!(parsed["state_spine"]["entity_surface_count"], 8);
    assert_eq!(
        parsed["state_spine"]["authoritative_mutation_root"],
        "vida task"
    );
    assert_eq!(
        parsed["latest_effective_bundle_receipt"]["root_artifact_id"],
        "framework-agent-definition"
    );
    assert_eq!(
        parsed["latest_effective_bundle_receipt"]["artifact_count"],
        3
    );
    assert_eq!(
        parsed["boot_compatibility"]["classification"],
        "backward_compatible"
    );
    assert_eq!(
        parsed["migration_state"]["migration_state"],
        "no_migration_required"
    );
    assert_eq!(
        parsed["migration_state"]["compatibility_class"],
        "backward_compatible"
    );
    assert!(parsed["migration_state"]["compatibility_classification"].is_null());
    assert_eq!(parsed["migration_receipts"]["compatibility_receipts"], 1);
    assert!(parsed["latest_task_reconciliation"].is_null());
    assert_eq!(parsed["task_reconciliation_rollup"]["total_receipts"], 0);
    assert!(parsed["task_reconciliation_rollup"]["latest_recorded_at"].is_null());
    assert!(parsed["task_reconciliation_rollup"]["latest_source_path"].is_null());
    assert_eq!(parsed["task_reconciliation_rollup"]["total_task_rows"], 0);
    assert_eq!(parsed["taskflow_snapshot_bridge"]["total_receipts"], 0);
    assert!(parsed["latest_run_graph_status"].is_null());
    assert!(parsed["latest_run_graph_recovery"].is_null());
    assert!(parsed["latest_run_graph_checkpoint"].is_null());
    assert!(parsed["latest_run_graph_gate"].is_null());
    assert_eq!(parsed["taskflow_snapshot_bridge"]["export_receipts"], 0);
    assert_eq!(
        parsed["taskflow_snapshot_bridge"]["memory_export_receipts"],
        0
    );
    assert_eq!(
        parsed["taskflow_snapshot_bridge"]["file_export_receipts"],
        0
    );
    assert!(parsed["taskflow_snapshot_bridge"]["latest_operation"].is_null());
    assert!(parsed["taskflow_snapshot_bridge"]["latest_source_path"].is_null());
    assert_eq!(parsed["protocol_binding"]["total_receipts"], 0);
    assert_eq!(parsed["protocol_binding"]["total_bindings"], 0);
    assert_eq!(parsed["protocol_binding"]["blocking_issue_count"], 0);
}

#[test]
fn status_surface_supports_compact_json_summary_view() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());
    let snapshot_path = seed_runtime_consumption_final_snapshot(&state_dir);

    let output = vida()
        .args(["status", "--summary", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status summary json should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("status summary json should parse");
    assert_eq!(parsed["surface"], "vida status");
    assert_eq!(parsed["view"], "summary");
    assert!(parsed["trace_id"].is_null());
    assert!(parsed["workflow_class"].is_null());
    assert!(parsed["risk_tier"].is_null());
    assert!(parsed.get("storage_metadata").is_none());
    assert!(parsed.get("latest_effective_bundle_receipt").is_none());
    assert_eq!(
        parsed["artifact_refs"]["root_session_write_guard_status"],
        "blocked_by_default"
    );
    assert_eq!(
        parsed["artifact_refs"]["runtime_consumption_latest_snapshot_path"],
        snapshot_path
    );
    assert!(parsed["blocker_codes"].as_array().is_some_and(|codes| {
        codes
            .iter()
            .any(|code| code == "incomplete_release_admission_operator_evidence")
    }));
    assert!(!parsed["blocker_codes"].as_array().is_some_and(|codes| {
        codes
            .iter()
            .any(|code| code == "missing_root_session_write_guard")
    }));
    assert!(parsed["state_spine"].is_object());
    assert!(parsed["project_activation"].is_object());
    assert!(parsed["protocol_binding"].is_object());
    assert!(parsed["host_agents"].is_object());
    assert_eq!(
        parsed["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        parsed["root_session_write_guard"]["lawful_write_surface"],
        "vida agent-init"
    );
    assert_eq!(
        parsed["root_session_write_guard"]["host_local_write_capability_is_not_authority"],
        true
    );
    assert_eq!(
        parsed["root_session_write_guard"]["explicit_user_ordered_agent_mode_is_sticky"],
        true
    );
    assert_eq!(
        parsed["root_session_write_guard"]["saturation_recovery_required_before_local_fallback"],
        true
    );
    assert_eq!(
        parsed["root_session_write_guard"]["local_fallback_without_lane_recovery_forbidden"],
        true
    );
}

#[test]
fn doctor_surface_supports_json_summary() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let output = doctor_with_timeout(&state_dir, &["doctor", "--json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor json should parse");
    assert_eq!(parsed["surface"], "vida doctor");
    assert!(parsed["trace_id"].is_null());
    assert!(parsed["workflow_class"].is_null());
    assert!(parsed["risk_tier"].is_null());
    assert_eq!(
        parsed["operator_contracts"]["contract_id"],
        "release-1-operator-contracts"
    );
    assert_eq!(
        parsed["operator_contracts"]["schema_version"],
        "release-1-v1"
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
    assert_eq!(parsed["storage_metadata"]["engine"], "surrealdb");
    assert_eq!(parsed["storage_metadata"]["backend"], "kv-surrealkv");
    assert_eq!(parsed["state_spine"]["entity_surface_count"], 8);
    assert_eq!(parsed["task_store"]["total_count"], 0);
    assert_eq!(parsed["task_store"]["ready_count"], 0);
    assert_eq!(parsed["run_graph"]["execution_plan_count"], 0);
    assert_eq!(parsed["dependency_graph"]["issue_count"], 0);
    assert_eq!(
        parsed["boot_compatibility"]["classification"],
        "backward_compatible"
    );
    assert_eq!(
        parsed["migration_preflight"]["migration_state"],
        "no_migration_required"
    );
    assert_eq!(
        parsed["migration_preflight"]["compatibility_class"],
        "backward_compatible"
    );
    assert!(parsed["migration_preflight"]["compatibility_classification"].is_null());
    assert_eq!(parsed["migration_receipts"]["compatibility_receipts"], 1);
    assert!(parsed["latest_task_reconciliation"].is_null());
    assert_eq!(parsed["task_reconciliation_rollup"]["total_receipts"], 0);
    assert!(parsed["task_reconciliation_rollup"]["latest_recorded_at"].is_null());
    assert!(parsed["task_reconciliation_rollup"]["latest_source_path"].is_null());
    assert_eq!(
        parsed["task_reconciliation_rollup"]["total_dependency_rows"],
        0
    );
    assert_eq!(parsed["taskflow_snapshot_bridge"]["total_receipts"], 0);
    assert_eq!(parsed["protocol_binding"]["total_receipts"], 0);
    assert_eq!(parsed["protocol_binding"]["total_bindings"], 0);
    assert!(parsed["latest_run_graph_status"].is_null());
    assert!(parsed["latest_run_graph_recovery"].is_null());
    assert!(parsed["latest_run_graph_checkpoint"].is_null());
    assert!(parsed["latest_run_graph_gate"].is_null());
    assert_eq!(parsed["taskflow_snapshot_bridge"]["replace_receipts"], 0);
    assert_eq!(
        parsed["taskflow_snapshot_bridge"]["memory_replace_receipts"],
        0
    );
    assert_eq!(
        parsed["taskflow_snapshot_bridge"]["file_replace_receipts"],
        0
    );
    assert!(parsed["taskflow_snapshot_bridge"]["latest_source_kind"].is_null());
    assert_eq!(parsed["taskflow_snapshot_bridge"]["total_stale_removed"], 0);
    assert_eq!(
        parsed["effective_instruction_bundle"]["root_artifact_id"],
        "framework-agent-definition"
    );
    assert_eq!(
        parsed["launcher_runtime_paths"]["vida"]
            .as_str()
            .expect("vida launcher should be a string"),
        "vida"
    );
    assert!(parsed["launcher_runtime_paths"]["taskflow_surface"]
        .as_str()
        .expect("taskflow surface should be a string")
        .contains("vida taskflow"));
}

#[test]
fn doctor_surface_supports_compact_json_summary_view() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());
    let snapshot_path = seed_runtime_consumption_final_snapshot(&state_dir);

    let output = doctor_with_timeout(&state_dir, &["doctor", "--summary", "--json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor summary json should parse");
    assert_eq!(parsed["surface"], "vida doctor");
    assert_eq!(parsed["view"], "summary");
    assert!(parsed["trace_id"].is_null());
    assert!(parsed["workflow_class"].is_null());
    assert!(parsed["risk_tier"].is_null());
    assert_eq!(parsed["runtime_consumption"]["latest_kind"], "final");
    assert_eq!(
        parsed["runtime_consumption"]["latest_snapshot_path"],
        snapshot_path
    );
    assert!(parsed.get("storage_metadata").is_none());
    assert!(parsed.get("task_store").is_none());
    assert_eq!(
        parsed["artifact_refs"]["root_session_write_guard_status"],
        "blocked_by_default"
    );
    assert!(parsed["dependency_graph"].is_object());
    assert!(parsed["runtime_consumption"].is_object());
    assert!(parsed["protocol_binding"].is_object());
    assert!(parsed["effective_instruction_bundle"].is_object());
    assert_eq!(
        parsed["root_session_write_guard"]["status"],
        "blocked_by_default"
    );
    assert_eq!(
        parsed["root_session_write_guard"]["lawful_write_surface"],
        "vida agent-init"
    );
    assert_eq!(
        parsed["root_session_write_guard"]["host_local_write_capability_is_not_authority"],
        true
    );
    assert_eq!(
        parsed["root_session_write_guard"]["explicit_user_ordered_agent_mode_is_sticky"],
        true
    );
    assert_eq!(
        parsed["root_session_write_guard"]["saturation_recovery_required_before_local_fallback"],
        true
    );
    assert_eq!(
        parsed["root_session_write_guard"]["local_fallback_without_lane_recovery_forbidden"],
        true
    );
}

#[test]
fn doctor_surface_fail_closed_when_release_admission_evidence_is_incomplete() {
    let state_dir = unique_state_dir();

    let boot = boot_with_retry(&state_dir);
    assert!(boot.status.success());

    let incomplete_snapshot_path = format!("{state_dir}/runtime-consumption/final-incomplete.json");
    write_file(
        &incomplete_snapshot_path,
        &serde_json::json!({
            "surface": "vida taskflow consume final",
            "payload": {
                "docflow_activation": {
                    "evidence": {
                        "registry": {"ok": true},
                        "check": {"ok": true},
                        "readiness": {"verdict": ""},
                    }
                },
                "closure_admission": {
                    "status": "admit",
                }
            }
        })
        .to_string(),
    );

    let output = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor json should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor json should parse");

    assert_eq!(parsed["status"], "blocked");
    let blocker_codes = parsed["blocker_codes"]
        .as_array()
        .expect("blocker_codes should be an array");
    assert!(blocker_codes.contains(&serde_json::Value::String(
        "incomplete_release_admission_operator_evidence".to_string()
    )));
}

#[test]
fn doctor_surface_supports_color_render_mode() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["doctor", "--render", "color"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run with color render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida doctor\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;32mpass\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34mtask store\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34mrun graph\u{1b}[0m"));
}

#[test]
fn doctor_surface_fails_closed_on_uninitialized_state_dir() {
    let state_dir = unique_state_dir();

    let output = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authoritative state directory is missing"));
    assert!(stderr.contains("VIDA_STATE_DIR=<temp-dir>"));
}

#[test]
fn installer_doctor_fails_closed_when_installed_helpers_are_missing() {
    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let bin_dir = format!("{root}/bin");
    let current_root = format!("{install_root}/current");
    let installer_script = format!("{install_root}/installer/install.sh");

    fs::create_dir_all(format!("{install_root}/installer")).expect("installer dir should exist");
    fs::create_dir_all(format!("{current_root}/bin")).expect("current bin dir should exist");
    fs::create_dir_all(format!("{current_root}/.venv/bin")).expect("venv dir should exist");
    fs::create_dir_all(format!("{current_root}/{}", donor_docflow_runtime_name()))
        .expect("docflow dir should exist");
    fs::create_dir_all(format!(
        "{current_root}/{}/helpers",
        donor_taskflow_runtime_name()
    ))
    .expect("helper dir should exist");
    fs::create_dir_all(&bin_dir).expect("bin dir should exist");

    write_executable_script(&format!("{bin_dir}/vida"), "#!/bin/sh\nexit 0\n");
    write_executable_script(
        &format!("{bin_dir}/{}", donor_taskflow_runtime_name()),
        "#!/bin/sh\nexit 0\n",
    );
    write_executable_script(
        &format!("{bin_dir}/{}", donor_docflow_runtime_name()),
        "#!/bin/sh\nexit 0\n",
    );
    write_executable_script(&installer_script, "#!/bin/sh\nexit 0\n");
    write_executable_script(
        &format!("{current_root}/bin/{}", donor_taskflow_runtime_name()),
        "#!/bin/sh\nexit 0\n",
    );
    write_executable_script(
        &format!("{current_root}/.venv/bin/python3"),
        "#!/bin/sh\nexit 0\n",
    );
    write_file(
        &format!(
            "{current_root}/{}/{}",
            donor_docflow_runtime_name(),
            donor_docflow_script_name()
        ),
        "print('ok')\n",
    );
    write_file(&format!("{current_root}/AGENTS.sidecar.md"), "sidecar\n");
    write_file(
        &format!(
            "{current_root}/{}/helpers/turso_task_store.py",
            donor_taskflow_runtime_name()
        ),
        "print('helper')\n",
    );
    write_file(&format!("{install_root}/env.sh"), "export VIDA_HOME=test\n");

    let output = Command::new("bash")
        .args([
            "install/install.sh",
            "doctor",
            "--root",
            &install_root,
            "--bin-dir",
            &bin_dir,
        ])
        .current_dir(repo_root())
        .output()
        .expect("installer doctor should run");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Missing bundled vida binary"));
    assert!(stderr.contains("Doctor found missing installation surfaces."));
}

#[test]
fn installer_install_populates_both_taskflow_helpers_in_current_layout() {
    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let bin_dir = format!("{root}/bin");
    let archive_path = format!("{root}/vida-stack-v-test.tar.gz");
    create_minimal_release_archive(&archive_path);

    let output = Command::new("bash")
        .args([
            "install/install.sh",
            "install",
            "--archive",
            &archive_path,
            "--root",
            &install_root,
            "--bin-dir",
            &bin_dir,
        ])
        .current_dir(repo_root())
        .env("HOME", format!("{root}/home"))
        .output()
        .expect("installer install should run");

    assert!(
        output.status.success(),
        "install should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::path::Path::new(&format!("{install_root}/current/bin/vida")).exists());
    assert!(std::path::Path::new(&format!(
        "{install_root}/current/install/assets/vida.config.yaml.template"
    ))
    .exists());
}

#[test]
fn reserved_families_fail_closed_without_help() {
    let command = "task";
    let output = vida()
        .arg(command)
        .output()
        .expect("reserved command should run");
    assert!(
        !output.status.success(),
        "{command} should stay fail-closed in Binary Foundation"
    );
}

#[test]
fn unknown_command_fails_closed() {
    let output = vida()
        .arg("unknown")
        .output()
        .expect("unknown command should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown command family `unknown`"));
}

#[test]
fn boot_with_extra_argument_fails_closed() {
    let output = vida()
        .args(["boot", "unexpected"])
        .output()
        .expect("boot with extra arg should run");
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported `vida boot` argument `unexpected`"));
}
