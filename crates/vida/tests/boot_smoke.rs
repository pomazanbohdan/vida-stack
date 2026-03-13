use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;

#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestLauncherActivationSnapshot {
    source: String,
    source_config_path: String,
    source_config_digest: String,
    captured_at: String,
    compiled_bundle: serde_json::Value,
    pack_router_keywords: serde_json::Value,
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
        fs::write(&self.path, &self.original_body).expect("guarded file should restore");
    }
}

fn vida() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vida"))
}

fn installed_vida() -> (String, Command) {
    let root = unique_state_dir();
    let install_root = format!("{root}/vida-install");
    let bin_dir = format!("{install_root}/bin");
    fs::create_dir_all(&bin_dir).expect("installed bin dir should exist");

    copy_executable(env!("CARGO_BIN_EXE_vida"), &format!("{bin_dir}/vida"));
    write_executable_script(
        &format!("{bin_dir}/taskflow-v0"),
        "#!/bin/sh\nprintf 'taskflow placeholder\\n'\n",
    );

    let mut command = Command::new(format!("{bin_dir}/vida"));
    command.current_dir(&root);
    (root, command)
}

fn unique_state_dir() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("/tmp/vida-test-state-{}-{}", std::process::id(), nanos)
}

fn repo_root() -> String {
    env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida")
        .to_string()
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

fn write_file(path: &str, body: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent).expect("parent dir should exist");
    }
    fs::write(path, body).expect("file should be written");
}

fn copy_file(from: &str, to: &str) {
    if let Some(parent) = std::path::Path::new(to).parent() {
        fs::create_dir_all(parent).expect("parent dir should exist");
    }
    fs::copy(from, to).expect("file should be copied");
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
    let config_body = fs::read(&config_path).expect("config should be readable for digest");
    let config_digest = blake3::hash(&config_body).to_hex().to_string();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(PathBuf::from(state_dir))
            .await
            .expect("state db should open");
        db.use_ns("vida")
            .use_db("primary")
            .await
            .expect("state namespace should open");
        let _: Option<TestLauncherActivationSnapshot> = db
            .upsert(("launcher_activation_snapshot", "launcher_live"))
            .content(TestLauncherActivationSnapshot {
                source: source.to_string(),
                source_config_path: config_path,
                source_config_digest: config_digest,
                captured_at: "2026-03-13T00:00:00Z".to_string(),
                compiled_bundle,
                pack_router_keywords: serde_json::json!({}),
            })
            .await
            .expect("launcher activation snapshot should update");
        drop(db);
    });
}

fn sync_protocol_binding(state_dir: &str) {
    let output = vida()
        .args(["taskflow", "protocol-binding", "sync", "--json"])
        .env("VIDA_STATE_DIR", state_dir)
        .output()
        .expect("protocol-binding sync should run");
    assert!(
        output.status.success(),
        "protocol-binding sync should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_minimal_release_archive(archive_path: &str) {
    let stage_root = format!("{}/release-stage", unique_state_dir());
    let package_root = format!("{stage_root}/vida-stack-v-test");
    let helper_dir = format!("{package_root}/taskflow-v0/helpers");
    let codex_dir = format!("{package_root}/codex-v0");
    let bin_dir = format!("{package_root}/bin");
    let template_dir = format!("{package_root}/docs/framework/templates");

    fs::create_dir_all(&helper_dir).expect("helper dir should exist");
    fs::create_dir_all(&codex_dir).expect("codex dir should exist");
    fs::create_dir_all(&bin_dir).expect("bin dir should exist");
    fs::create_dir_all(&template_dir).expect("template dir should exist");

    write_executable_script(
        &format!("{bin_dir}/taskflow-v0"),
        "#!/bin/sh\nprintf 'taskflow placeholder\\n'\n",
    );
    write_file(
        &format!("{codex_dir}/codex.py"),
        "print('codex placeholder')\n",
    );
    write_file(&format!("{codex_dir}/requirements-python.txt"), "");
    write_file(&format!("{package_root}/AGENTS.sidecar.md"), "sidecar\n");

    let root = repo_root();
    copy_file(
        &format!("{root}/taskflow-v0/helpers/turso_task_store.py"),
        &format!("{helper_dir}/turso_task_store.py"),
    );
    copy_file(
        &format!("{root}/taskflow-v0/helpers/toon_render.py"),
        &format!("{helper_dir}/toon_render.py"),
    );
    copy_file(
        &format!("{root}/docs/framework/templates/vida.config.yaml.template"),
        &format!("{template_dir}/vida.config.yaml.template"),
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

fn run_with_retry<F>(mut op: F) -> std::process::Output
where
    F: FnMut() -> std::process::Output,
{
    let mut last = None;
    for _ in 0..60 {
        let output = op();
        if output.status.success() {
            return output;
        }
        last = Some(output);
        thread::sleep(Duration::from_millis(100));
    }
    last.expect("retry helper should capture at least one output")
}

fn command_output_with_retry(command: &mut Command) -> std::process::Output {
    for _ in 0..60 {
        match command.output() {
            Ok(output) => return output,
            Err(error) if error.raw_os_error() == Some(26) => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => panic!("command should run: {error}"),
        }
    }

    panic!("command should run after executable file retry window");
}

#[test]
fn root_help_succeeds() {
    let output = vida().arg("--help").output().expect("root help should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: vida [COMMAND]"));
    assert!(stdout.contains("boot"));
    assert!(stdout.contains("task"));
    assert!(stdout.contains("memory"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("taskflow"));
    assert!(stdout.contains("docflow"));
}

#[test]
fn boot_succeeds() {
    let output = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", unique_state_dir())
        .output()
        .expect("boot should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida boot scaffold ready"));
    assert!(stdout.contains("authoritative state store: kv-surrealkv"));
    assert!(stdout.contains("authoritative state spine: initialized (state-v1, 8 entity surfaces, mutation root vida task)"));
    assert!(stdout.contains("framework instruction bundle: seeded"));
    assert!(stdout.contains("instruction source tree: _vida/instructions -> instruction_memory"));
    assert!(stdout.contains(
        "instruction ingest: 3 imported, 0 unchanged, 0 updated from _vida/instructions"
    ));
    assert!(stdout.contains("boot compatibility: compatible (normal_boot_allowed)"));
    assert!(stdout
        .contains("migration preflight: compatible / no_migration_required (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
    assert!(stdout.contains("effective instruction bundle: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains(
        "effective instruction bundle receipt: effective-bundle-framework-agent-definition-"
    ));
    assert!(stdout.contains(
        "framework memory ingest: 1 imported, 0 unchanged, 0 updated from _vida/framework-memory"
    ));
}

#[test]
fn boot_supports_color_render_mode() {
    let output = vida()
        .args(["boot", "--render", "color"])
        .env("VIDA_STATE_DIR", unique_state_dir())
        .output()
        .expect("boot should run with color render mode");
    assert!(output.status.success());

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
        "instruction ingest: 0 imported, 3 unchanged, 0 updated from _vida/instructions"
    ));
    assert!(stdout.contains("effective instruction bundle: framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config"));
    assert!(stdout.contains(
        "effective instruction bundle receipt: effective-bundle-framework-agent-definition-"
    ));
    assert!(stdout.contains(
        "framework memory ingest: 0 imported, 1 unchanged, 0 updated from _vida/framework-memory"
    ));
    assert!(stdout.contains("boot compatibility: compatible (normal_boot_allowed)"));
    assert!(stdout
        .contains("migration preflight: compatible / no_migration_required (normal_boot_allowed)"));
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

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow runtime family"));
    assert!(stdout.contains("`taskflow-v0 task` is the primary backlog store during the bridge"));
    assert!(stdout.contains("vida taskflow task ready --json"));
    assert!(stdout
        .contains("vida taskflow help [task|consume|run-graph|recovery|doctor|protocol-binding]"));
}

#[test]
fn taskflow_proxy_help_supports_task_topic() {
    let output = vida()
        .args(["taskflow", "help", "task"])
        .output()
        .expect("taskflow task topic help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VIDA TaskFlow help: task"));
    assert!(stdout.contains("JSONL is import/export compatibility only"));
    assert!(stdout.contains("vida taskflow task next-display-id <parent-display-id> --json"));
    assert!(stdout.contains(
        "vida taskflow task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description"
    ));
    assert!(stdout.contains("vida taskflow task update <task-id> --status in_progress --notes"));
    assert!(stdout
        .contains("vida taskflow task export-jsonl .vida/exports/tasks.snapshot.jsonl --json"));
    assert!(stdout.contains("Parent-child edges preserve epic/task structure"));
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
    assert!(stdout
        .contains("Bundle inspection and final closure loop are launcher-owned and in-process"));
}

#[test]
fn taskflow_doctor_routes_in_process_without_taskflow_binary() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);
    sync_protocol_binding(&state_dir);

    let delegated_taskflow_bin = format!("{state_dir}/delegated-taskflow-v0");
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
        parsed["launcher_runtime_paths"]["taskflow_runtime"],
        delegated_taskflow_bin
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

    let precheck = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding precheck should run");
    assert!(!precheck.status.success());
    let precheck_stdout = String::from_utf8_lossy(&precheck.stdout);
    let precheck_json: serde_json::Value = serde_json::from_str(&precheck_stdout)
        .expect("protocol-binding precheck json should parse");
    assert_eq!(precheck_json["ok"], false);
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

    let check = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding check should run");
    assert!(check.status.success());
    let check_stdout = String::from_utf8_lossy(&check.stdout);
    let check_json: serde_json::Value =
        serde_json::from_str(&check_stdout).expect("protocol-binding check json should parse");
    assert_eq!(check_json["ok"], true);
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

    let check = vida()
        .args(["taskflow", "protocol-binding", "check", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("protocol-binding check should run");
    assert!(!check.status.success());
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

    let text_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "latest"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery latest should run")
    });
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow recovery latest"));
    assert!(text_stdout.contains("recovery: none"));

    let json_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery latest json should run")
    });
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

    let output = run_with_retry(|| {
        vida()
            .args(["taskflow", "consume", "bundle", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume bundle json should run")
    });
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume bundle json should parse");
    assert_eq!(parsed["surface"], "vida taskflow consume bundle");
    assert_eq!(parsed["bundle"]["artifact_name"], "taskflow_runtime_bundle");
    assert_eq!(parsed["bundle"]["artifact_type"], "runtime_bundle");
    assert_eq!(
        parsed["bundle"]["control_core"]["root_artifact_id"],
        "framework-agent-definition"
    );
    assert_eq!(
        parsed["bundle"]["metadata"]["bundle_schema_version"],
        "release-1-v1"
    );
    assert_eq!(
        parsed["bundle"]["boot_compatibility"]["classification"],
        "compatible"
    );
    assert_eq!(
        parsed["bundle"]["migration_preflight"]["migration_state"],
        "no_migration_required"
    );
    let vida_root = parsed["bundle"]["vida_root"]
        .as_str()
        .expect("consume bundle should render vida_root");
    let config_path = parsed["bundle"]["config_path"]
        .as_str()
        .expect("consume bundle should render config_path");
    let project_root = parsed["bundle"]["launcher_runtime_paths"]["project_root"]
        .as_str()
        .expect("consume bundle should render project_root");
    assert_eq!(vida_root, project_root);
    assert_eq!(config_path, format!("{vida_root}/vida.config.yaml"));
    assert!(parsed["bundle"]["metadata"].is_object());
    assert!(parsed["bundle"]["control_core"].is_object());
    assert!(parsed["bundle"]["activation_bundle"].is_object());
    assert!(parsed["bundle"]["protocol_binding_registry"].is_object());
    assert!(parsed["bundle"]["cache_delivery_contract"].is_object());
    assert!(parsed["bundle"]["orchestrator_init_view"].is_object());
    assert!(parsed["bundle"]["agent_init_view"].is_object());
    assert!(parsed["bundle"]["protocol_binding_registry"]["protocols"].is_array());
    assert_eq!(
        parsed["bundle"]["protocol_binding_registry"]["binding_status"],
        "blocked"
    );
    assert_eq!(
        parsed["bundle"]["protocol_binding_registry"]["compiled_payload_import_evidence"]
            ["trusted"],
        true
    );
    assert!(parsed["bundle"]["cache_delivery_contract"]["cache_key_inputs"].is_object());
    assert!(parsed["bundle"]["cache_delivery_contract"]["invalidation_tuple"].is_object());
    assert!(parsed["bundle"]["cache_delivery_contract"]
        ["retrieval_only_optional_context_boundary"]
        .is_array());
    assert_eq!(
        parsed["bundle"]["activation_bundle"]["project_protocol_projections"]["status"],
        "compiled"
    );
    assert_eq!(
        parsed["bundle"]["orchestrator_init_view"]["surface"],
        "vida orchestrator-init"
    );
    assert_eq!(
        parsed["bundle"]["agent_init_view"]["surface"],
        "vida agent-init"
    );
    let snapshot_path = parsed["snapshot_path"]
        .as_str()
        .expect("consume bundle should report snapshot path");
    assert!(std::path::Path::new(snapshot_path).is_file());
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
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("orchestrator-init json should parse");
    assert_eq!(parsed["surface"], "vida orchestrator-init");
    assert_eq!(parsed["init"]["surface"], "vida orchestrator-init");
    assert!(parsed["init"]["project_startup_bundle"].is_object());
    assert!(parsed["init"]["project_startup_capsules"].is_array());
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
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("agent-init json should parse");
    assert_eq!(parsed["surface"], "vida agent-init");
    assert_eq!(parsed["init"]["surface"], "vida agent-init");
    assert_eq!(parsed["selection"]["selected_role"], "worker");
    assert!(parsed["init"]["allowed_non_orchestrator_roles"].is_array());
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

    let output = run_with_retry(|| {
        vida()
            .args(["taskflow", "consume", "bundle", "check", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume bundle check json should run")
    });
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume bundle check json should parse");
    assert_eq!(parsed["surface"], "vida taskflow consume bundle check");
    assert_eq!(parsed["check"]["ok"], true);
    assert_eq!(
        parsed["check"]["root_artifact_id"],
        "framework-agent-definition"
    );
    assert_eq!(parsed["check"]["boot_classification"], "compatible");
    assert_eq!(parsed["check"]["migration_state"], "no_migration_required");
    assert_eq!(parsed["check"]["blockers"], serde_json::json!([]));
    let snapshot_path = parsed["snapshot_path"]
        .as_str()
        .expect("consume bundle check should report snapshot path");
    assert!(std::path::Path::new(snapshot_path).is_file());
}

#[test]
fn taskflow_consume_bundle_check_fails_closed_without_protocol_binding_receipt() {
    let state_dir = unique_state_dir();

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
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume bundle check json should parse");
    assert_eq!(parsed["check"]["ok"], false);
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

#[test]
fn taskflow_consume_final_renders_direct_runtime_consumption_snapshot() {
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

    let output = run_with_retry(|| {
        vida()
            .args(["taskflow", "consume", "final", "probe closure", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume final json should run")
    });
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final json should parse");
    assert_eq!(parsed["surface"], "vida taskflow consume final");
    assert_eq!(parsed["payload"]["closure_authority"], "taskflow");
    assert_eq!(parsed["payload"]["request_text"], "probe closure");
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        "auto"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]["mode"],
        "native"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]["state_owner"],
        "orchestrator_only"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]
            ["max_parallel_agents"],
        4
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]["subagents"]
            ["internal_subagents"]["default_profile"],
        "internal_fast"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]["routing"]
            ["default"]["dispatch_required"],
        "external_first_when_eligible"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["codex_multi_agent"]["enabled"],
        true
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["codex_multi_agent"]["max_threads"],
        "4"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["codex_multi_agent"]["max_depth"],
        "2"
    );
    let codex_roles = parsed["payload"]["role_selection"]["compiled_bundle"]["codex_multi_agent"]
        ["roles"]
        .as_array()
        .expect("codex roles should be an array");
    assert!(codex_roles.iter().any(|row| {
        row["role_id"] == "development_implementer"
            && row["model"] == "gpt-5.4"
            && row["model_reasoning_effort"] == "low"
            && row["sandbox_mode"] == "workspace-write"
    }));
    assert!(codex_roles.iter().any(|row| {
        row["role_id"] == "development_coach"
            && row["model_reasoning_effort"] == "medium"
            && row["sandbox_mode"] == "read-only"
    }));
    assert!(codex_roles.iter().any(|row| {
        row["role_id"] == "development_verifier"
            && row["model_reasoning_effort"] == "low"
            && row["sandbox_mode"] == "read-only"
    }));
    assert!(codex_roles.iter().any(|row| {
        row["role_id"] == "development_escalation"
            && row["model_reasoning_effort"] == "high"
            && row["sandbox_mode"] == "read-only"
    }));
    assert_eq!(
        parsed["payload"]["role_selection"]["fallback_role"],
        "orchestrator"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["selected_role"],
        "orchestrator"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["conversational_mode"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["tracked_flow_entry"],
        serde_json::Value::Null
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["confidence"],
        "fallback"
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["reason"],
        "auto_no_keyword_match"
    );
    assert_eq!(parsed["payload"]["bundle_check"]["ok"], true);
    assert_eq!(parsed["payload"]["direct_consumption_ready"], true);
    assert_eq!(
        parsed["payload"]["docflow_activation"]["runtime_family"],
        "docflow"
    );
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["registry"]["ok"],
        true
    );
    assert!(
        parsed["payload"]["docflow_activation"]["evidence"]["registry"]["row_count"]
            .as_u64()
            .expect("registry row_count should be numeric")
            > 0
    );
    assert!(
        parsed["payload"]["docflow_activation"]["evidence"]["overview"]["registry_rows"]
            .as_u64()
            .expect("overview registry_rows should be numeric")
            > 0
    );
    assert!(
        parsed["payload"]["docflow_activation"]["evidence"]["registry"]["surface"]
            .as_str()
            .expect("registry surface should be a string")
            .starts_with("vida docflow registry --root ")
    );
    assert!(
        parsed["payload"]["docflow_activation"]["evidence"]["registry"]["output"]
            .as_str()
            .expect("registry output should be a string")
            .contains("\"artifact_path\":")
    );
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["check"]["ok"],
        true
    );
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["readiness"]["ok"],
        true
    );
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["readiness"]["verdict"],
        "ready"
    );
    let readiness_artifact_path = parsed["payload"]["docflow_activation"]["evidence"]["readiness"]
        ["artifact_path"]
        .as_str()
        .expect("readiness artifact path should be a string");
    assert!(readiness_artifact_path.ends_with("vida/config/codex-readiness.current.jsonl"));
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["proof"]["ok"],
        true
    );
    assert_eq!(parsed["payload"]["docflow_verdict"]["status"], "pass");
    assert_eq!(parsed["payload"]["docflow_verdict"]["ready"], true);
    assert_eq!(
        parsed["payload"]["docflow_verdict"]["blockers"],
        serde_json::json!([])
    );
    let proof_surfaces = parsed["payload"]["docflow_verdict"]["proof_surfaces"]
        .as_array()
        .expect("proof surfaces should be an array");
    assert_eq!(proof_surfaces.len(), 4);
    assert!(proof_surfaces
        .iter()
        .any(|value| value == "vida docflow check --profile active-canon"));
    assert!(proof_surfaces
        .iter()
        .any(|value| value == "vida docflow readiness-check --profile active-canon"));
    assert!(proof_surfaces
        .iter()
        .any(|value| value == "vida docflow proofcheck --profile active-canon"));
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "admit");
    assert_eq!(parsed["payload"]["closure_admission"]["admitted"], true);
    assert_eq!(
        parsed["payload"]["closure_admission"]["blockers"],
        serde_json::json!([])
    );
    let closure_proof_surfaces = parsed["payload"]["closure_admission"]["proof_surfaces"]
        .as_array()
        .expect("closure proof surfaces should be an array");
    assert!(closure_proof_surfaces
        .iter()
        .any(|value| value == "vida taskflow consume bundle check"));
    let snapshot_path = parsed["snapshot_path"]
        .as_str()
        .expect("consume final should report snapshot path");
    assert!(std::path::Path::new(snapshot_path).is_file());

    let status = run_with_retry(|| {
        vida()
            .args(["status", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("status json should run after consume final")
    });
    assert!(status.status.success());
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    let status_json: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status json should parse");
    assert_eq!(status_json["runtime_consumption"]["final_snapshots"], 1);
    assert_eq!(status_json["runtime_consumption"]["latest_kind"], "final");

    let doctor = run_with_retry(|| {
        vida()
            .args(["doctor", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("doctor json should run after consume final")
    });
    assert!(doctor.status.success());
    let doctor_stdout = String::from_utf8_lossy(&doctor.stdout);
    let doctor_json: serde_json::Value =
        serde_json::from_str(&doctor_stdout).expect("doctor json should parse");
    assert_eq!(doctor_json["runtime_consumption"]["final_snapshots"], 1);
    assert_eq!(doctor_json["runtime_consumption"]["latest_kind"], "final");
}

#[test]
fn consume_final_reports_explicit_docflow_blockers_when_repo_context_is_missing() {
    let state_dir = unique_state_dir();
    let root = unique_state_dir();
    let project_root = format!("{root}/project");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    write_file(&format!("{project_root}/AGENTS.md"), "project");
    write_file(&format!("{project_root}/vida/root-map.md"), "project");
    write_file(
        &format!("{project_root}/vida.config.yaml"),
        &format!(
            r#"project:
  id: temp
protocol_activation:
  agent_system: true
agent_extensions:
  enabled: true
  registries:
    roles: {repo_root}/docs/process/agent-extensions/roles.yaml
    skills: {repo_root}/docs/process/agent-extensions/skills.yaml
    profiles: {repo_root}/docs/process/agent-extensions/profiles.yaml
    flows: {repo_root}/docs/process/agent-extensions/flows.yaml
  enabled_framework_roles:
    - orchestrator
  enabled_project_roles: []
  enabled_project_skills: []
  enabled_project_profiles: []
  enabled_project_flows: []
  role_selection:
    mode: fixed
    fallback_role: orchestrator
  validation:
    require_registry_files: true
    require_profile_resolution: false
    require_flow_resolution: false
agent_system:
  mode: native
  state_owner: orchestrator_only
"#,
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_vida"))
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final should render blocking json payload");
    assert_eq!(parsed["payload"]["direct_consumption_ready"], false);
    assert_eq!(parsed["payload"]["docflow_verdict"]["status"], "block");
    assert_eq!(parsed["payload"]["docflow_verdict"]["ready"], false);
    assert_eq!(parsed["payload"]["closure_admission"]["status"], "block");
    assert_eq!(parsed["payload"]["closure_admission"]["admitted"], false);
    let blockers = parsed["payload"]["docflow_verdict"]["blockers"]
        .as_array()
        .expect("blockers should be an array");
    assert_eq!(
        parsed["payload"]["docflow_activation"]["evidence"]["readiness"]["verdict"],
        "blocked"
    );
    let readiness_artifact_path = parsed["payload"]["docflow_activation"]["evidence"]["readiness"]
        ["artifact_path"]
        .as_str()
        .expect("blocking readiness artifact path should be a string");
    assert!(readiness_artifact_path.ends_with("vida/config/codex-readiness.current.jsonl"));
    assert!(blockers
        .iter()
        .any(|value| value == "missing_docflow_activation"));
    assert!(blockers
        .iter()
        .any(|value| value == "missing_readiness_verdict"));
    assert!(blockers
        .iter()
        .any(|value| value == "missing_proof_verdict"));
    let closure_blockers = parsed["payload"]["closure_admission"]["blockers"]
        .as_array()
        .expect("closure blockers should be an array");
    assert!(closure_blockers
        .iter()
        .any(|value| value == "missing_closure_proof"));
    assert!(closure_blockers
        .iter()
        .any(|value| value == "restore_reconcile_not_green"));
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

    write_file(&format!("{project_root}/AGENTS.md"), "project");
    write_file(&format!("{project_root}/vida/root-map.md"), "project");

    let output = Command::new(env!("CARGO_BIN_EXE_vida"))
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");

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
}

#[test]
fn consume_final_refreshes_launcher_snapshot_when_config_digest_changes() {
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
    assert!(
        initial.status.success(),
        "{}{}",
        String::from_utf8_lossy(&initial.stdout),
        String::from_utf8_lossy(&initial.stderr)
    );
    let initial_stdout = String::from_utf8_lossy(&initial.stdout);
    let initial_parsed: serde_json::Value =
        serde_json::from_str(&initial_stdout).expect("initial consume final should render json");
    assert_eq!(
        initial_parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]
            ["max_parallel_agents"],
        serde_json::json!(4)
    );
    let initial_captured_at = initial_parsed["payload"]["runtime_bundle"]["metadata"]
        ["compiled_at"]
        .as_str()
        .expect("initial captured_at should exist")
        .to_string();

    let config_path = format!("{}/vida.config.yaml", repo_root());
    let restore_guard = FileRestoreGuard::new(config_path.clone());
    let original_config = restore_guard.original_body.clone();
    let updated_config = original_config
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("max_parallel_agents:") {
                "  max_parallel_agents: 5".to_string()
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
    fs::write(&config_path, &updated_config).expect("config should update");

    let output = vida()
        .args(["taskflow", "consume", "final", "probe closure", "--json"])
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("consume final should run");
    drop(restore_guard);

    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("consume final should render json payload");
    assert_eq!(
        parsed["payload"]["role_selection"]["selection_mode"],
        serde_json::json!("auto")
    );
    assert_eq!(
        parsed["payload"]["runtime_bundle"]["activation_source"],
        serde_json::json!("state_store")
    );
    assert_eq!(
        parsed["payload"]["runtime_bundle"]["config_path"],
        serde_json::json!(config_path)
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["agent_system"]
            ["max_parallel_agents"],
        serde_json::json!(5)
    );
    assert_ne!(
        parsed["payload"]["runtime_bundle"]["metadata"]["compiled_at"],
        serde_json::json!(initial_captured_at)
    );
}

#[test]
fn taskflow_consume_final_selects_scope_discussion_role_for_spec_queries() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let output = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "consume",
                "final",
                "clarify spec scope",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume final scope json should run")
    });
    assert!(output.status.success());

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
        11
    );
    assert_eq!(
        parsed["payload"]["role_selection"]["compiled_bundle"]["project_flows"]
            .as_array()
            .expect("project flows should be an array")
            .len(),
        2
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
    let matched_terms = parsed["payload"]["role_selection"]["matched_terms"]
        .as_array()
        .expect("matched terms should be an array");
    assert!(matched_terms.iter().any(|value| value == "clarify"));
    assert!(matched_terms.iter().any(|value| value == "spec"));
    assert!(matched_terms.iter().any(|value| value == "scope"));
}

#[test]
fn taskflow_consume_final_selects_pbi_discussion_role_for_backlog_queries() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let output = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "consume",
                "final",
                "prioritize backlog work pool",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume final pbi json should run")
    });
    assert!(output.status.success());

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
    let matched_terms = parsed["payload"]["role_selection"]["matched_terms"]
        .as_array()
        .expect("matched terms should be an array");
    assert!(matched_terms.iter().any(|value| value == "prioritize"));
    assert!(matched_terms.iter().any(|value| value == "backlog"));
    assert!(matched_terms.iter().any(|value| value == "work pool"));
}

#[test]
fn taskflow_consume_final_does_not_match_short_substring_false_positive() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let output = run_with_retry(|| {
        vida()
            .args(["taskflow", "consume", "final", "trace cache", "--json"])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume final false-positive probe should run")
    });
    assert!(output.status.success());

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
}

#[test]
fn taskflow_consume_final_does_not_match_ac_inside_incidental_words() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let output = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "consume",
                "final",
                "trace cache invalidation",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume final ac false-positive probe should run")
    });
    assert!(output.status.success());

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
}

#[test]
fn taskflow_consume_final_does_not_match_aspect_incidental_word() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env_remove("VIDA_ROOT")
        .env_remove("VIDA_HOME")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());
    sync_protocol_binding(&state_dir);

    let output = run_with_retry(|| {
        vida()
            .args([
                "taskflow",
                "consume",
                "final",
                "review one aspect of caching",
                "--json",
            ])
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow consume final aspect false-positive probe should run")
    });
    assert!(output.status.success());

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

    let text_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "checkpoint-latest"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery checkpoint-latest should run")
    });
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow recovery checkpoint-latest"));
    assert!(text_stdout.contains("checkpoint: none"));

    let json_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "checkpoint-latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery checkpoint-latest json should run")
    });
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

    let text_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "gate-latest"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery gate-latest should run")
    });
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow recovery gate-latest"));
    assert!(text_stdout.contains("gate: none"));

    let json_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "gate-latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery gate-latest json should run")
    });
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

    let text_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "latest"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow run-graph latest should run")
    });
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(text_stdout.contains("vida taskflow run-graph latest"));
    assert!(text_stdout.contains("status: none"));

    let json_output = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow run-graph latest json should run")
    });
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
            "{\"next_node\":\"coach\",\"selected_backend\":\"codex\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"policy_gate_required\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());

    let run_graph_latest = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow run-graph latest should run")
    });
    assert!(run_graph_latest.status.success());
    let run_graph_latest_stdout = String::from_utf8_lossy(&run_graph_latest.stdout);
    let run_graph_latest_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_latest_stdout).expect("run-graph latest should parse");
    assert_eq!(run_graph_latest_parsed["status"]["run_id"], "vida-a");
    assert_eq!(run_graph_latest_parsed["status"]["active_node"], "writer");
    assert_eq!(run_graph_latest_parsed["status"]["next_node"], "coach");
    assert_eq!(
        run_graph_latest_parsed["status"]["policy_gate"],
        "policy_gate_required"
    );
    assert_eq!(
        run_graph_latest_parsed["status"]["checkpoint_kind"],
        "execution_cursor"
    );

    let recovery_latest = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery latest should run")
    });
    assert!(recovery_latest.status.success());
    let recovery_latest_stdout = String::from_utf8_lossy(&recovery_latest.stdout);
    let recovery_latest_parsed: serde_json::Value =
        serde_json::from_str(&recovery_latest_stdout).expect("recovery latest should parse");
    assert_eq!(recovery_latest_parsed["recovery"]["run_id"], "vida-a");
    assert_eq!(recovery_latest_parsed["recovery"]["resume_node"], "coach");
    assert_eq!(recovery_latest_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_latest_parsed["recovery"]["resume_target"],
        "dispatch.writer_lane"
    );

    let checkpoint_latest = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "checkpoint-latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow checkpoint latest should run")
    });
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

    let gate_latest = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "gate-latest", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow gate latest should run")
    });
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
        "awaiting_coach"
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

    let status_output = vida()
        .args(["status", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status json should run");
    assert!(status_output.status.success());
    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    let status_parsed: serde_json::Value =
        serde_json::from_str(&status_stdout).expect("status json should parse");
    assert_eq!(
        status_parsed["latest_run_graph_checkpoint"]["run_id"],
        "vida-a"
    );
    assert_eq!(status_parsed["latest_run_graph_gate"]["run_id"], "vida-a");

    let doctor_output = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor json should run");
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
            "{\"next_node\":\"coach\",\"selected_backend\":\"codex\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"policy_gate_required\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());

    let status_output = vida()
        .arg("status")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("status should run");
    assert!(status_output.status.success());
    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_stdout.contains("latest run graph status: run=vida-a"));
    assert!(status_stdout.contains("latest run graph recovery: run=vida-a"));
    assert!(status_stdout.contains("latest run graph checkpoint: run=vida-a"));
    assert!(status_stdout.contains("latest run graph gate: run=vida-a"));
    assert!(status_stdout.contains("checkpoint=execution_cursor"));
    assert!(status_stdout.contains("gate=policy_gate_required"));

    let doctor_output = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(doctor_output.status.success());
    let doctor_stdout = String::from_utf8_lossy(&doctor_output.stdout);
    assert!(doctor_stdout.contains("latest run graph status: ok (run=vida-a"));
    assert!(doctor_stdout.contains("latest run graph recovery: ok (run=vida-a"));
    assert!(doctor_stdout.contains("latest run graph checkpoint: ok (run=vida-a"));
    assert!(doctor_stdout.contains("latest run graph gate: ok (run=vida-a"));
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
            "{\"next_node\":\"coach\",\"selected_backend\":\"codex\",\"lane_id\":\"writer_lane\",\"lifecycle_stage\":\"active\",\"policy_gate\":\"policy_gate_required\",\"handoff_state\":\"awaiting_coach\",\"context_state\":\"sealed\",\"checkpoint_kind\":\"execution_cursor\",\"resume_target\":\"dispatch.writer_lane\",\"recovery_ready\":true}",
        ])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("taskflow run-graph update should run");
    assert!(update.status.success());

    let run_graph_status = run_with_retry(|| {
        vida()
            .args(["taskflow", "run-graph", "status", "vida-a", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow run-graph status should run")
    });
    assert!(run_graph_status.status.success());
    let run_graph_status_stdout = String::from_utf8_lossy(&run_graph_status.stdout);
    let run_graph_status_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_status_stdout).expect("run-graph status should parse");
    assert_eq!(run_graph_status_parsed["run_id"], "vida-a");
    assert_eq!(run_graph_status_parsed["status"]["active_node"], "writer");
    assert_eq!(run_graph_status_parsed["status"]["next_node"], "coach");
    assert_eq!(
        run_graph_status_parsed["status"]["selected_backend"],
        "codex"
    );

    let recovery_status = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-a", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow recovery status should run")
    });
    assert!(recovery_status.status.success());
    let recovery_status_stdout = String::from_utf8_lossy(&recovery_status.stdout);
    let recovery_status_parsed: serde_json::Value =
        serde_json::from_str(&recovery_status_stdout).expect("recovery status should parse");
    assert_eq!(recovery_status_parsed["run_id"], "vida-a");
    assert_eq!(recovery_status_parsed["recovery"]["resume_node"], "coach");
    assert_eq!(recovery_status_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_status_parsed["recovery"]["policy_gate"],
        "policy_gate_required"
    );

    let checkpoint_status = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "checkpoint", "vida-a", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow checkpoint status should run")
    });
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

    let gate_status = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "gate", "vida-a", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("taskflow gate status should run")
    });
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
        "awaiting_coach"
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
        "internal_subagents"
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
        "internal_subagents"
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

    let recovery = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-pbi", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("recovery status should run")
    });
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
    assert_eq!(run_graph_parsed["status"]["next_node"], "analysis");
    assert_eq!(
        run_graph_parsed["status"]["policy_gate"],
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
    assert_eq!(advance_parsed["payload"]["status"]["next_node"], "coach");
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
        "awaiting_coach"
    );
    assert_eq!(
        advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.coach_lane"
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
        run_graph_parsed["status"]["active_node"],
        "business_analyst"
    );
    assert_eq!(run_graph_parsed["status"]["next_node"], "spec-pack");
    assert_eq!(
        run_graph_parsed["status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        run_graph_parsed["status"]["resume_target"],
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
    assert_eq!(run_graph_parsed["status"]["active_node"], "pm");
    assert_eq!(run_graph_parsed["status"]["next_node"], "work-pool-pack");
    assert_eq!(
        run_graph_parsed["status"]["policy_gate"],
        "single_task_scope_required"
    );
    assert_eq!(
        run_graph_parsed["status"]["resume_target"],
        "dispatch.work-pool-pack"
    );

    let recovery = run_with_retry(|| {
        vida()
            .args(["taskflow", "recovery", "status", "vida-pbi", "--json"])
            .env("VIDA_STATE_DIR", &state_dir)
            .output()
            .expect("recovery status should run")
    });
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
    assert_eq!(run_graph_parsed["status"]["active_node"], "analysis");
    assert_eq!(run_graph_parsed["status"]["next_node"], "coach");
    assert_eq!(
        run_graph_parsed["status"]["policy_gate"],
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
    assert_eq!(recovery_parsed["recovery"]["resume_node"], "coach");
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.coach_lane"
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
        "coach"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["next_node"],
        "review_ensemble"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["route_task_class"],
        "implementation"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["lane_id"],
        "coach_lane"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["lifecycle_stage"],
        "coach_active"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["policy_gate"],
        "review_findings"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["handoff_state"],
        "awaiting_review_ensemble"
    );
    assert_eq!(
        second_advance_parsed["payload"]["status"]["resume_target"],
        "dispatch.review_ensemble"
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
    assert_eq!(run_graph_parsed["status"]["active_node"], "coach");
    assert_eq!(run_graph_parsed["status"]["next_node"], "review_ensemble");
    assert_eq!(run_graph_parsed["status"]["policy_gate"], "review_findings");

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
        "review_ensemble"
    );
    assert_eq!(recovery_parsed["recovery"]["resume_status"], "ready");
    assert_eq!(
        recovery_parsed["recovery"]["resume_target"],
        "dispatch.review_ensemble"
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
        "review_ensemble"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["next_node"],
        serde_json::Value::Null
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["lane_id"],
        "review_ensemble_lane"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["lifecycle_stage"],
        "review_ensemble_active"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["policy_gate"],
        "review_findings"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["handoff_state"],
        "none"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["resume_target"],
        "none"
    );
    assert_eq!(
        third_advance_parsed["payload"]["status"]["recovery_ready"],
        false
    );
    assert_eq!(
        third_advance_parsed["delegation_gate"]["delegated_cycle_open"],
        true
    );
    assert_eq!(
        third_advance_parsed["delegation_gate"]["delegated_cycle_state"],
        "delegated_lane_active"
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

    for step in 0..3 {
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
    assert_eq!(run_graph_parsed["status"]["active_node"], "review_ensemble");
    assert_eq!(
        run_graph_parsed["status"]["next_node"],
        serde_json::Value::Null
    );
    assert_eq!(
        run_graph_parsed["status"]["lane_id"],
        "review_ensemble_lane"
    );
    assert_eq!(run_graph_parsed["status"]["handoff_state"], "none");
    assert_eq!(run_graph_parsed["status"]["resume_target"], "none");
    assert_eq!(run_graph_parsed["status"]["recovery_ready"], false);

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
fn taskflow_run_graph_fourth_advance_completes_clean_review_ensemble() {
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
                .expect("pre-completion advance should run")
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
    assert_eq!(parsed["payload"]["status"]["status"], "completed");
    assert_eq!(
        parsed["payload"]["status"]["lifecycle_stage"],
        "implementation_complete"
    );
    assert_eq!(parsed["payload"]["status"]["policy_gate"], "not_required");
    assert_eq!(
        parsed["payload"]["status"]["next_node"],
        serde_json::Value::Null
    );
    assert_eq!(parsed["payload"]["status"]["resume_target"], "none");
    assert_eq!(parsed["payload"]["status"]["recovery_ready"], false);
}

#[test]
fn taskflow_run_graph_fourth_advance_updates_status_and_recovery_after_clean_review() {
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
                .args(["taskflow", "run-graph", "advance", "vida-dev"])
                .env_remove("VIDA_ROOT")
                .env_remove("VIDA_HOME")
                .env("VIDA_STATE_DIR", &state_dir)
                .output()
                .expect("pre-completion advance should run")
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
        .expect("fourth run-graph advance should run");
    assert!(fourth_advance.status.success());

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", "vida-dev", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(run_graph_parsed["status"]["active_node"], "review_ensemble");
    assert_eq!(run_graph_parsed["status"]["status"], "completed");
    assert_eq!(run_graph_parsed["status"]["policy_gate"], "not_required");
    assert_eq!(run_graph_parsed["status"]["resume_target"], "none");
    assert_eq!(run_graph_parsed["status"]["recovery_ready"], false);
    assert_eq!(
        run_graph_parsed["delegation_gate"]["delegated_cycle_open"],
        false
    );
    assert_eq!(
        run_graph_parsed["delegation_gate"]["local_exception_takeover_gate"],
        "delegated_cycle_clear"
    );

    let recovery = vida()
        .args(["taskflow", "recovery", "status", "vida-dev", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery status should run");
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

    for step in 0..3 {
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

    for step in 0..3 {
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
    assert_eq!(parsed["payload"]["status"]["next_node"], "coach");
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
        "awaiting_coach"
    );
    assert_eq!(
        parsed["payload"]["status"]["resume_target"],
        "dispatch.coach_lane"
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

    for step in 0..3 {
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

    let run_graph = vida()
        .args(["taskflow", "run-graph", "status", "vida-dev", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("run-graph status should run");
    assert!(run_graph.status.success());
    let run_graph_stdout = String::from_utf8_lossy(&run_graph.stdout);
    let run_graph_parsed: serde_json::Value =
        serde_json::from_str(&run_graph_stdout).expect("run-graph status json should parse");
    assert_eq!(run_graph_parsed["status"]["active_node"], "analysis");
    assert_eq!(run_graph_parsed["status"]["next_node"], "coach");
    assert_eq!(run_graph_parsed["status"]["status"], "ready");
    assert_eq!(
        run_graph_parsed["status"]["resume_target"],
        "dispatch.coach_lane"
    );
    assert_eq!(run_graph_parsed["status"]["recovery_ready"], true);

    let recovery = vida()
        .args(["taskflow", "recovery", "status", "vida-dev", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("recovery status should run");
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
    assert!(stdout.contains("vida taskflow task ready --json"));
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
        "vida taskflow task create <task-id> <title> --parent-id <parent-id> --auto-display-from <parent-display-id> --description \"...\" --json"
    ));
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
    assert!(stdout.contains("vida taskflow task next-display-id <parent-display-id> --json"));
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
    assert!(stdout
        .contains("vida taskflow task export-jsonl .vida/exports/tasks.snapshot.jsonl --json"));
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
    assert!(stdout.contains("help|overview only"));
    assert!(stdout.contains("Usage: docflow <COMMAND>"));
    assert!(stdout.contains("registry-write"));
    assert!(stdout.contains("artifact-impact"));
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
            "Unsupported `vida taskflow passthrough-probe` subcommand. This launcher-owned top-level taskflow surface fails closed instead of delegating to taskflow-v0."
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
            "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to taskflow-v0."
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
    let script_path = format!("{root}/delegated-taskflow-v0");
    let seed_path = format!("{root}/seed.jsonl");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    fs::create_dir_all(&root).expect("temp root should exist");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let seed = vida()
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
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
            .expect("ready payload should be an array")
            .len(),
        0
    );
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_task_create_routes_through_local_db_bridge_with_display_id_allocation() {
    let root = unique_state_dir();
    let script_path = format!("{root}/delegated-taskflow-v0");
    let seed_path = format!("{root}/seed.jsonl");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    fs::create_dir_all(&root).expect("temp root should exist");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let seed = vida()
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
        .env("VIDA_ROOT", &root)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
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
        .env("VIDA_ROOT", &root)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
        .output()
        .expect("taskflow epic create should run");
    assert!(create_epic.status.success());

    let output = vida()
        .args([
            "taskflow",
            "task",
            "create",
            "vida-child",
            "Child",
            "--parent-id",
            "vida-root",
            "--auto-display-from",
            "vida-rf1.1",
            "--description",
            "bridge-task",
            "--json",
        ])
        .env("VIDA_ROOT", &root)
        .env("VIDA_TASKFLOW_BIN", &script_path)
        .env(
            "VIDA_V0_TURSO_PYTHON",
            format!("{repo_root}/.venv/bin/python3"),
        )
        .output()
        .expect("taskflow task create bridge should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("taskflow task create json should parse");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["created"], true);
    assert_eq!(parsed["task"]["id"], "vida-child");
    assert_eq!(parsed["task"]["display_id"], "vida-rf1.1.1");
    assert_eq!(parsed["task"]["description"], "bridge-task");
    assert_eq!(
        parsed["task"]["dependencies"][0]["depends_on_id"],
        "vida-root"
    );
    assert!(!stderr.contains("delegated-taskflow-binary-ran"));
}

#[test]
fn taskflow_proxy_resolves_repo_root_from_nested_project_pwd_without_env() {
    let root = unique_state_dir();
    let project_root = format!("{root}/project");
    let nested_pwd = format!("{project_root}/work/nested");
    let script_path = format!("{project_root}/delegated-taskflow-v0");
    let seed_path = format!("{project_root}/seed.jsonl");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("project AGENTS should exist");
    fs::write(format!("{project_root}/vida/root-map.md"), "project")
        .expect("project root-map should exist");
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
            .expect("ready payload should be an array")
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
    fs::create_dir_all(format!("{outer}/vida")).expect("outer vida dir should exist");
    fs::create_dir_all(format!("{inner}/vida")).expect("inner vida dir should exist");
    fs::create_dir_all(&nested).expect("nested dir should exist");
    fs::write(format!("{outer}/AGENTS.md"), "outer").expect("outer AGENTS should exist");
    fs::write(format!("{outer}/vida/root-map.md"), "outer").expect("outer root-map should exist");
    fs::write(format!("{inner}/AGENTS.md"), "inner").expect("inner AGENTS should exist");
    fs::write(format!("{inner}/vida/root-map.md"), "inner").expect("inner root-map should exist");

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
    let script_path = format!("{install_root}/bin/taskflow-v0");
    let vida_path = format!("{install_root}/bin/vida");
    let helper_path = format!("{install_root}/taskflow-v0/helpers/turso_task_store.py");
    let python_path = format!("{install_root}/.venv/bin/python3");
    let nested_pwd = format!("{project_root}/work/nested");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!("{install_root}/taskflow-v0/helpers"))
        .expect("install helper dir should exist");
    fs::create_dir_all(format!("{install_root}/.venv/bin"))
        .expect("install python dir should exist");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("project AGENTS should exist");
    fs::write(format!("{project_root}/vida/root-map.md"), "project")
        .expect("project root-map should exist");
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

    let output = Command::new(&vida_path)
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .output()
        .expect("installed vida should run");

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
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    let helper_source = format!("{repo_root}/taskflow-v0/helpers/turso_task_store.py");
    let python_source = format!("{repo_root}/.venv/bin/python3");

    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let project_root = format!("{root}/project");
    let script_path = format!("{install_root}/bin/taskflow-v0");
    let vida_path = format!("{install_root}/bin/vida");
    let helper_path = format!("{install_root}/taskflow-v0/helpers/turso_task_store.py");
    let python_path = format!("{install_root}/.venv/bin/python3");
    let nested_pwd = format!("{project_root}/work/nested");
    let seed_path = format!("{project_root}/seed.jsonl");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!("{install_root}/taskflow-v0/helpers"))
        .expect("install helper dir should exist");
    fs::create_dir_all(format!("{install_root}/.venv/bin"))
        .expect("install python dir should exist");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("project AGENTS should exist");
    fs::write(format!("{project_root}/vida/root-map.md"), "project")
        .expect("project root-map should exist");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &vida_path);
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    write_executable_script(
        &python_path,
        &format!("#!/bin/sh\nexec {python_source} \"$@\"\n"),
    );
    copy_file(&helper_source, &helper_path);
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

    let import = Command::new(&vida_path)
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .output()
        .expect("installed import should run");
    assert!(import.status.success());

    let ready = Command::new(&vida_path)
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .output()
        .expect("installed ready should run");

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
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    let helper_source = format!("{repo_root}/taskflow-v0/helpers/turso_task_store.py");
    let python_source = format!("{repo_root}/.venv/bin/python3");

    let root = unique_state_dir();
    let install_root = format!("{root}/install");
    let project_root = format!("{root}/project");
    let script_path = format!("{install_root}/bin/taskflow-v0");
    let vida_path = format!("{install_root}/bin/vida");
    let helper_path = format!("{install_root}/taskflow-v0/helpers/turso_task_store.py");
    let python_path = format!("{install_root}/.venv/bin/python3");
    let nested_pwd = format!("{project_root}/work/nested");
    let seed_path = format!("{project_root}/seed-ordering.jsonl");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!("{install_root}/taskflow-v0/helpers"))
        .expect("install helper dir should exist");
    fs::create_dir_all(format!("{install_root}/.venv/bin"))
        .expect("install python dir should exist");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("project AGENTS should exist");
    fs::write(format!("{project_root}/vida/root-map.md"), "project")
        .expect("project root-map should exist");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &vida_path);
    write_executable_script(
        &script_path,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    write_executable_script(
        &python_path,
        &format!("#!/bin/sh\nexec {python_source} \"$@\"\n"),
    );
    copy_file(&helper_source, &helper_path);
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

    let import = Command::new(&vida_path)
        .args(["taskflow", "task", "import-jsonl", &seed_path, "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .output()
        .expect("installed import should run");
    assert!(import.status.success());

    let ready = Command::new(&vida_path)
        .args(["taskflow", "task", "ready", "--json"])
        .current_dir(&nested_pwd)
        .env_remove("VIDA_ROOT")
        .output()
        .expect("installed ready should run");

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
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    let helper_source = format!("{repo_root}/taskflow-v0/helpers/turso_task_store.py");
    let python_source = format!("{repo_root}/.venv/bin/python3");

    let project_root = format!("{}/project-aware", unique_state_dir());
    let nested_pwd = format!("{project_root}/work/nested");
    let delegated_taskflow_bin = format!("{project_root}/delegated-taskflow-v0");
    let export_path = format!("{project_root}/export/issues.jsonl");
    let seed_path = format!("{project_root}/seed.jsonl");
    fs::create_dir_all(format!("{project_root}/vida")).expect("project vida dir should exist");
    fs::create_dir_all(&nested_pwd).expect("nested project dir should exist");
    fs::write(format!("{project_root}/AGENTS.md"), "project").expect("project AGENTS should exist");
    fs::write(format!("{project_root}/vida/root-map.md"), "project")
        .expect("project root-map should exist");
    fs::write(&seed_path, "").expect("seed jsonl should be written");
    write_executable_script(
        &delegated_taskflow_bin,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );

    let project_mode = |args: &[&str]| {
        vida()
            .args(args)
            .current_dir(&nested_pwd)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .env("VIDA_TASKFLOW_BIN", &delegated_taskflow_bin)
            .env("VIDA_V0_TURSO_PYTHON", &python_source)
            .output()
            .expect("project-aware bridge command should run")
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

    let project_create_child = project_mode(&[
        "taskflow",
        "task",
        "create",
        "vida-child",
        "Child",
        "--parent-id",
        "vida-root",
        "--auto-display-from",
        "vida-rf1.1",
        "--description",
        "bridge-task",
        "--json",
    ]);
    assert!(project_create_child.status.success());

    let project_list = project_mode(&["taskflow", "task", "list", "--all", "--json"]);
    assert!(project_list.status.success());
    let project_list_stdout = String::from_utf8_lossy(&project_list.stdout);
    let project_list_stderr = String::from_utf8_lossy(&project_list.stderr);
    let project_list_json: serde_json::Value =
        serde_json::from_str(&project_list_stdout).expect("project list json should parse");
    let project_rows = project_list_json
        .as_array()
        .expect("project list payload should be an array");
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
    assert_eq!(project_show_json["id"], "vida-child");
    assert_eq!(project_show_json["display_id"], "vida-rf1.1.1");
    assert_eq!(project_show_json["description"], "bridge-task");
    assert!(!project_show_stderr.contains("delegated-taskflow-binary-ran"));

    let project_next_display = project_mode(&[
        "taskflow",
        "task",
        "next-display-id",
        "vida-rf1.1",
        "--json",
    ]);
    assert!(project_next_display.status.success());
    let project_next_display_stdout = String::from_utf8_lossy(&project_next_display.stdout);
    let project_next_display_stderr = String::from_utf8_lossy(&project_next_display.stderr);
    let project_next_display_json: serde_json::Value =
        serde_json::from_str(&project_next_display_stdout)
            .expect("project next-display-id json should parse");
    assert_eq!(project_next_display_json["valid"], true);
    assert_eq!(project_next_display_json["next_display_id"], "vida-rf1.1.2");
    assert!(!project_next_display_stderr.contains("delegated-taskflow-binary-ran"));

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
    assert!(project_update.status.success());
    let project_update_stdout = String::from_utf8_lossy(&project_update.stdout);
    let project_update_stderr = String::from_utf8_lossy(&project_update.stderr);
    let project_update_json: serde_json::Value =
        serde_json::from_str(&project_update_stdout).expect("project update json should parse");
    assert_eq!(project_update_json["status"], "ok");
    assert_eq!(project_update_json["task"]["status"], "in_progress");
    assert_eq!(project_update_json["task"]["notes"], "bridge proof");
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
    assert_eq!(project_close_json["status"], "ok");
    assert_eq!(project_close_json["task"]["status"], "closed");
    assert_eq!(project_close_json["task"]["close_reason"], "done");
    assert!(!project_close_stderr.contains("delegated-taskflow-binary-ran"));

    let project_export =
        project_mode(&["taskflow", "task", "export-jsonl", &export_path, "--json"]);
    assert!(project_export.status.success());
    let project_export_stdout = String::from_utf8_lossy(&project_export.stdout);
    let project_export_stderr = String::from_utf8_lossy(&project_export.stderr);
    let project_export_json: serde_json::Value =
        serde_json::from_str(&project_export_stdout).expect("project export json should parse");
    assert_eq!(project_export_json["status"], "ok");
    assert_eq!(project_export_json["exported_count"], 2);
    assert_eq!(project_export_json["target_path"], export_path);
    let project_exported = fs::read_to_string(&export_path).expect("project export should exist");
    assert!(project_exported.contains("\"id\":\"vida-root\""));
    assert!(project_exported.contains("\"id\":\"vida-child\""));
    assert!(!project_export_stderr.contains("delegated-taskflow-binary-ran"));

    let install_root = format!("{}/install", unique_state_dir());
    let installed_project_root = format!("{install_root}/project");
    let installed_nested_pwd = format!("{installed_project_root}/work/nested");
    let installed_taskflow_bin = format!("{install_root}/bin/taskflow-v0");
    let installed_vida_bin = format!("{install_root}/bin/vida");
    let installed_python = format!("{install_root}/.venv/bin/python3");
    let installed_helper = format!("{install_root}/taskflow-v0/helpers/turso_task_store.py");
    let installed_export_path = format!("{installed_project_root}/export/issues.jsonl");
    let installed_seed_path = format!("{installed_project_root}/seed.jsonl");
    fs::create_dir_all(format!("{install_root}/bin")).expect("install bin dir should exist");
    fs::create_dir_all(format!("{install_root}/.venv/bin"))
        .expect("install python dir should exist");
    fs::create_dir_all(format!("{install_root}/taskflow-v0/helpers"))
        .expect("install helper dir should exist");
    fs::create_dir_all(format!("{installed_project_root}/vida"))
        .expect("installed project vida dir should exist");
    fs::create_dir_all(&installed_nested_pwd).expect("installed nested project dir should exist");
    fs::write(format!("{installed_project_root}/AGENTS.md"), "project")
        .expect("installed project AGENTS should exist");
    fs::write(
        format!("{installed_project_root}/vida/root-map.md"),
        "project",
    )
    .expect("installed project root-map should exist");
    fs::write(&installed_seed_path, "").expect("installed seed jsonl should be written");
    copy_executable(env!("CARGO_BIN_EXE_vida"), &installed_vida_bin);
    write_executable_script(
        &installed_taskflow_bin,
        "#!/bin/sh\necho delegated-taskflow-binary-ran >&2\nexit 23\n",
    );
    write_executable_script(
        &installed_python,
        &format!("#!/bin/sh\nexec {python_source} \"$@\"\n"),
    );
    copy_file(&helper_source, &installed_helper);

    let installed_mode = |args: &[&str]| {
        Command::new(&installed_vida_bin)
            .args(args)
            .current_dir(&installed_nested_pwd)
            .env_remove("VIDA_ROOT")
            .env_remove("VIDA_HOME")
            .output()
            .expect("installed bridge command should run")
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

    let installed_create_child = installed_mode(&[
        "taskflow",
        "task",
        "create",
        "vida-child",
        "Child",
        "--parent-id",
        "vida-root",
        "--auto-display-from",
        "vida-rf1.1",
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
    let installed_ready_rows = installed_ready_json
        .as_array()
        .expect("installed ready payload should be an array");
    assert_eq!(installed_ready_rows.len(), 1);
    assert_eq!(installed_ready_rows[0]["id"], "vida-child");
    assert_eq!(installed_ready_rows[0]["display_id"], "vida-rf1.1.1");
    assert!(!installed_ready_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_list = installed_mode(&["taskflow", "task", "list", "--all", "--json"]);
    assert!(installed_list.status.success());
    let installed_list_stdout = String::from_utf8_lossy(&installed_list.stdout);
    let installed_list_stderr = String::from_utf8_lossy(&installed_list.stderr);
    let installed_list_json: serde_json::Value =
        serde_json::from_str(&installed_list_stdout).expect("installed list json should parse");
    let installed_rows = installed_list_json
        .as_array()
        .expect("installed list payload should be an array");
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
    assert_eq!(installed_show_json["id"], "vida-child");
    assert_eq!(installed_show_json["display_id"], "vida-rf1.1.1");
    assert_eq!(installed_show_json["description"], "bridge-task");
    assert!(!installed_show_stderr.contains("delegated-taskflow-binary-ran"));

    let installed_next_display = installed_mode(&[
        "taskflow",
        "task",
        "next-display-id",
        "vida-rf1.1",
        "--json",
    ]);
    assert!(installed_next_display.status.success());
    let installed_next_display_stdout = String::from_utf8_lossy(&installed_next_display.stdout);
    let installed_next_display_stderr = String::from_utf8_lossy(&installed_next_display.stderr);
    let installed_next_display_json: serde_json::Value =
        serde_json::from_str(&installed_next_display_stdout)
            .expect("installed next-display-id json should parse");
    assert_eq!(installed_next_display_json["valid"], true);
    assert_eq!(
        installed_next_display_json["next_display_id"],
        "vida-rf1.1.2"
    );
    assert!(!installed_next_display_stderr.contains("delegated-taskflow-binary-ran"));

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
    assert_eq!(installed_update_json["status"], "ok");
    assert_eq!(installed_update_json["task"]["status"], "in_progress");
    assert_eq!(installed_update_json["task"]["notes"], "bridge proof");
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
    assert_eq!(installed_close_json["status"], "ok");
    assert_eq!(installed_close_json["task"]["status"], "closed");
    assert_eq!(installed_close_json["task"]["close_reason"], "done");
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
    assert_eq!(installed_export_json["status"], "ok");
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
    let state_dir = unique_state_dir();
    let root = unique_state_dir();
    let project_root = format!("{root}/project");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    let taskflow_bin = format!("{repo_root}/taskflow-v0/src/vida");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    write_file(&format!("{project_root}/AGENTS.md"), "project");
    write_file(&format!("{project_root}/vida/root-map.md"), "project");
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
  enabled_project_roles:
    - party_chat_facilitator
  enabled_project_skills:
    - party_chat_council_reasoning
  enabled_project_profiles:
    - party_chat_facilitator_profile
  enabled_project_flows:
    - party_chat_council_small
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

    let output = Command::new(env!("CARGO_BIN_EXE_vida"))
        .args([
            "taskflow",
            "consume",
            "final",
            "clarify spec scope",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &taskflow_bin)
        .output()
        .expect("consume final should run");

    assert!(!output.status.success());
    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_text.contains("missing/roles.yaml"));
    assert!(
        output_text.contains("Agent extension bundle validation failed")
            || output_text.contains("launcher activation snapshot")
    );
}

#[test]
fn taskflow_consume_final_fails_closed_for_unresolved_tracked_flow_entry() {
    let state_dir = unique_state_dir();
    let root = unique_state_dir();
    let project_root = format!("{root}/project");
    let repo_root = env!("CARGO_MANIFEST_DIR")
        .strip_suffix("/crates/vida")
        .expect("crate manifest dir should end with /crates/vida");
    let taskflow_bin = format!("{repo_root}/taskflow-v0/src/vida");

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    write_file(&format!("{project_root}/AGENTS.md"), "project");
    write_file(&format!("{project_root}/vida/root-map.md"), "project");
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
  enabled_project_roles:
    - party_chat_facilitator
  enabled_project_skills:
    - party_chat_council_reasoning
  enabled_project_profiles:
    - party_chat_facilitator_profile
  enabled_project_flows:
    - party_chat_council_small
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

    let output = Command::new(env!("CARGO_BIN_EXE_vida"))
        .args([
            "taskflow",
            "consume",
            "final",
            "clarify spec scope",
            "--json",
        ])
        .current_dir(&project_root)
        .env_remove("VIDA_ROOT")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_TASKFLOW_BIN", &taskflow_bin)
        .output()
        .expect("consume final should run");

    assert!(!output.status.success());
    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_text.contains("missing-pack"));
    assert!(
        output_text.contains("Agent extension bundle validation failed")
            || output_text.contains("launcher activation snapshot")
    );
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
        .env("VIDA_ROOT", &root)
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
        .env("VIDA_ROOT", &root)
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
        .env("VIDA_ROOT", &root)
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
        .env("VIDA_ROOT", &root)
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
fn docflow_proxy_runs_move_in_process_when_supported() {
    let root = unique_state_dir();
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
        .env("VIDA_ROOT", &root)
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
        .env("VIDA_ROOT", &root)
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
        .env("VIDA_ROOT", &root)
        .output()
        .expect("docflow in-process migrate-links should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"replacements\":1"));
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
    assert!(stdout.contains("issues: 1"));
    assert!(stdout.contains("verdict: blocking"));
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
    assert!(stdout.contains("rows: 1"));
    assert!(stdout.contains("verdict: blocking"));
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
    let output = format!("{root}/vida/config/codex-registry.current.jsonl");
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
    assert!(stdout.contains("issues: 1"));
    assert!(stdout.contains("verdict: blocking"));
    assert!(stdout.contains("docs/process/a.md [missing_footer]"));

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
    assert!(stdout.contains("rows: 1"));
    assert!(stdout.contains(&format!("output: {output}")));

    let written = fs::read_to_string(&output).expect("readiness jsonl should be written");
    assert!(written.contains("\"artifact_path\":\"docs/process/a.md\""));
    assert!(written.contains("\"verdict\":\"blocking\""));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn docflow_proxy_can_run_rust_readiness_write_canonical_surface() {
    let root = unique_state_dir();
    let output = format!("{root}/vida/config/codex-readiness.current.jsonl");
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
        .env("VIDA_ROOT", &root)
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
        .env("VIDA_ROOT", &root)
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
fn installed_docflow_compatibility_mode_supports_overview_only() {
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
fn installed_docflow_compatibility_mode_rejects_non_overview_commands() {
    let (root, mut command) = installed_vida();
    let output =
        command_output_with_retry(command.args(["docflow", "check", "--profile", "active-canon"]));

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("compatibility wrapper"));
    assert!(stderr.contains("help|overview only"));

    fs::remove_dir_all(root).expect("temp root should be removed");
}

#[test]
fn memory_surface_reports_effective_bundle() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("memory")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("memory should run");
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
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["memory", "--render", "color"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("memory should run with color render mode");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[1;36mvida memory\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[1;34meffective instruction bundle root\u{1b}[0m"));
}

#[test]
fn memory_surface_cli_render_overrides_env() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["memory", "--render", "color_emoji"])
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "plain")
        .output()
        .expect("memory should run with CLI render override");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("📘 vida memory"));
    assert!(stdout.contains("🔹"));
}

#[test]
fn memory_surface_fails_closed_on_invalid_render_env() {
    let state_dir = unique_state_dir();
    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("memory")
        .env("VIDA_STATE_DIR", &state_dir)
        .env("VIDA_RENDER", "invalid_mode")
        .output()
        .expect("memory should run with invalid render env");
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
    assert!(stdout.contains("boot compatibility: compatible (normal_boot_allowed)"));
    assert!(stdout
        .contains("migration state: compatible / no_migration_required (normal_boot_allowed)"));
    assert!(stdout.contains(
        "migration receipts: compatibility=1, application=0, verification=0, cutover=0, rollback=0"
    ));
}

#[test]
fn status_surface_supports_color_emoji_render_mode_via_env() {
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
}

#[test]
fn doctor_surface_reports_integrity_checks() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .arg("doctor")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida doctor"));
    assert!(stdout.contains("storage metadata: ok (kv-surrealkv state-v1 instruction-v1)"));
    assert!(stdout.contains(
        "authoritative state spine: ok (state-v1, 8 entity surfaces, mutation root vida task)"
    ));
    assert!(stdout
        .contains("task store: ok (0 total, 0 open, 0 in_progress, 0 closed, 0 epics, 0 ready)"));
    assert!(stdout.contains(
        "run graph: ok (execution_plans=0, routed_runs=0, governance=0, resumability=0, reconciliation=0)"
    ));
    assert!(stdout.contains("latest run graph status: ok (none)"));
    assert!(stdout.contains("latest run graph recovery: ok (none)"));
    assert!(stdout.contains("latest run graph checkpoint: ok (none)"));
    assert!(stdout.contains("latest run graph gate: ok (none)"));
    assert!(stdout.contains("launcher/runtime paths: ok (vida="));
    assert!(stdout.contains("project_root="));
    assert!(stdout.contains("taskflow_runtime="));
    assert!(stdout.contains("dependency graph: ok (0 issues)"));
    assert!(stdout.contains("boot compatibility: ok (compatible (normal_boot_allowed))"));
    assert!(stdout.contains(
        "migration preflight: ok (compatible / no_migration_required (normal_boot_allowed))"
    ));
    assert!(stdout.contains(
        "migration receipts: ok (compatibility=1, application=0, verification=0, cutover=0, rollback=0)"
    ));
    assert!(stdout.contains("task reconciliation: ok (none)"));
    assert!(stdout.contains("task reconciliation rollup: ok (0 receipts)"));
    assert!(stdout.contains("taskflow snapshot bridge: ok (idle (no snapshot bridge receipts))"));
    assert!(stdout.contains("effective instruction bundle: ok (framework-agent-definition -> framework-instruction-contract -> framework-prompt-template-config)"));
}

#[test]
fn status_surface_supports_json_summary() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
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
    assert_eq!(parsed["boot_compatibility"]["classification"], "compatible");
    assert_eq!(
        parsed["migration_state"]["migration_state"],
        "no_migration_required"
    );
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
fn doctor_surface_supports_json_summary() {
    let state_dir = unique_state_dir();

    let boot = vida()
        .arg("boot")
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("boot should run");
    assert!(boot.status.success());

    let output = vida()
        .args(["doctor", "--json"])
        .env("VIDA_STATE_DIR", &state_dir)
        .output()
        .expect("doctor json should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor json should parse");
    assert_eq!(parsed["surface"], "vida doctor");
    assert_eq!(parsed["storage_metadata"]["engine"], "surrealdb");
    assert_eq!(parsed["storage_metadata"]["backend"], "kv-surrealkv");
    assert_eq!(parsed["state_spine"]["entity_surface_count"], 8);
    assert_eq!(parsed["task_store"]["total_count"], 0);
    assert_eq!(parsed["task_store"]["ready_count"], 0);
    assert_eq!(parsed["run_graph"]["execution_plan_count"], 0);
    assert_eq!(parsed["dependency_graph"]["issue_count"], 0);
    assert_eq!(parsed["boot_compatibility"]["classification"], "compatible");
    assert_eq!(
        parsed["migration_preflight"]["migration_state"],
        "no_migration_required"
    );
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
    assert!(parsed["launcher_runtime_paths"]["vida"]
        .as_str()
        .expect("vida path should be a string")
        .contains("vida"));
    assert!(parsed["launcher_runtime_paths"]["taskflow_runtime"]
        .as_str()
        .expect("taskflow runtime should be a string")
        .contains("taskflow"));
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
    assert!(stdout.contains("\u{1b}[1;32mok\u{1b}[0m"));
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
    fs::create_dir_all(format!("{current_root}/codex-v0")).expect("codex dir should exist");
    fs::create_dir_all(format!("{current_root}/taskflow-v0/helpers"))
        .expect("helper dir should exist");
    fs::create_dir_all(&bin_dir).expect("bin dir should exist");

    write_executable_script(&format!("{bin_dir}/vida"), "#!/bin/sh\nexit 0\n");
    write_executable_script(&format!("{bin_dir}/taskflow-v0"), "#!/bin/sh\nexit 0\n");
    write_executable_script(&format!("{bin_dir}/codex-v0"), "#!/bin/sh\nexit 0\n");
    write_executable_script(&installer_script, "#!/bin/sh\nexit 0\n");
    write_executable_script(
        &format!("{current_root}/bin/taskflow-v0"),
        "#!/bin/sh\nexit 0\n",
    );
    write_executable_script(
        &format!("{current_root}/.venv/bin/python3"),
        "#!/bin/sh\nexit 0\n",
    );
    write_file(
        &format!("{current_root}/codex-v0/codex.py"),
        "print('ok')\n",
    );
    write_file(&format!("{current_root}/AGENTS.sidecar.md"), "sidecar\n");
    write_file(
        &format!("{current_root}/taskflow-v0/helpers/turso_task_store.py"),
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
    assert!(stdout.contains("Missing installed helper:"));
    assert!(stdout.contains("current/taskflow-v0/helpers/toon_render.py"));
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
    assert!(std::path::Path::new(&format!(
        "{install_root}/current/taskflow-v0/helpers/turso_task_store.py"
    ))
    .exists());
    assert!(std::path::Path::new(&format!(
        "{install_root}/current/taskflow-v0/helpers/toon_render.py"
    ))
    .exists());
}

#[test]
fn reserved_families_fail_closed_without_help() {
    for command in ["task"] {
        let output = vida()
            .arg(command)
            .output()
            .expect("reserved command should run");
        assert!(
            !output.status.success(),
            "{command} should stay fail-closed in Binary Foundation"
        );
    }
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
