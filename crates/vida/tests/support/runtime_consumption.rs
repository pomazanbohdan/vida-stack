#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;

pub(crate) const RECEIPT_HELPER_STATE_DIR_ENV: &str = "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_STATE_DIR";
pub(crate) const RECEIPT_HELPER_RUN_ID_ENV: &str = "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_RUN_ID";
pub(crate) const RECEIPT_HELPER_DISPATCH_TARGET_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DISPATCH_TARGET";
pub(crate) const RECEIPT_HELPER_ACTIVE_NODE_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_ACTIVE_NODE";
pub(crate) const RECEIPT_HELPER_DISPATCH_PACKET_PATH_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DISPATCH_PACKET_PATH";
pub(crate) const RECEIPT_HELPER_DOWNSTREAM_TARGET_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DOWNSTREAM_TARGET";
pub(crate) const RECEIPT_HELPER_DOWNSTREAM_PACKET_PATH_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DOWNSTREAM_PACKET_PATH";
pub(crate) const RECEIPT_HELPER_RESULT_PATH_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_RESULT_PATH";
pub(crate) const RECEIPT_HELPER_DOWNSTREAM_READY_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DOWNSTREAM_READY";
pub(crate) const RECEIPT_HELPER_DOWNSTREAM_STATUS_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DOWNSTREAM_STATUS";
pub(crate) const RECEIPT_HELPER_DOWNSTREAM_BLOCKERS_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DOWNSTREAM_BLOCKERS";
pub(crate) const RECEIPT_HELPER_DOWNSTREAM_ACTIVE_TARGET_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DOWNSTREAM_ACTIVE_TARGET";
pub(crate) const RECEIPT_HELPER_DISPATCH_STATUS_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DISPATCH_STATUS";
pub(crate) const RECEIPT_HELPER_LANE_STATUS_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_LANE_STATUS";
pub(crate) const RECEIPT_HELPER_BLOCKER_CODE_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_BLOCKER_CODE";
pub(crate) const RECEIPT_HELPER_DISPATCH_SURFACE_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DISPATCH_SURFACE";
pub(crate) const RECEIPT_HELPER_DISPATCH_COMMAND_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DISPATCH_COMMAND";
pub(crate) const RECEIPT_HELPER_TASK_CLASS_ENV: &str = "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_TASK_CLASS";
pub(crate) const RECEIPT_HELPER_LIFECYCLE_STAGE_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_LIFECYCLE_STAGE";
pub(crate) const RECEIPT_HELPER_HANDOFF_STATE_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_HANDOFF_STATE";
pub(crate) const RECEIPT_HELPER_RESUME_TARGET_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_RESUME_TARGET";
pub(crate) const PROTOCOL_BINDING_CLEAR_STATE_DIR_ENV: &str =
    "VIDA_BOOT_SMOKE_PROTOCOL_BINDING_CLEAR_STATE_DIR";
pub(crate) const RUN_GRAPH_DELETE_STATE_DIR_ENV: &str =
    "VIDA_BOOT_SMOKE_RUN_GRAPH_DELETE_STATE_DIR";
pub(crate) const RUN_GRAPH_DELETE_TABLE_ENV: &str = "VIDA_BOOT_SMOKE_RUN_GRAPH_DELETE_TABLE";
pub(crate) const RUN_GRAPH_DELETE_RUN_ID_ENV: &str = "VIDA_BOOT_SMOKE_RUN_GRAPH_DELETE_RUN_ID";

const MAX_OPEN_RETRIES: usize = 20;
static UNIQUE_FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct RuntimeReceiptFixture {
    pub(crate) state_dir: String,
    pub(crate) run_id: String,
    pub(crate) dispatch_target: String,
    pub(crate) active_node: Option<String>,
    pub(crate) dispatch_packet_path: String,
    pub(crate) downstream_target: String,
    pub(crate) downstream_packet_path: String,
    pub(crate) result_path: String,
    pub(crate) downstream_ready: bool,
    pub(crate) downstream_status: String,
    pub(crate) downstream_blockers: Vec<String>,
    pub(crate) downstream_active_target: Option<String>,
    pub(crate) dispatch_status: String,
    pub(crate) lane_status: String,
    pub(crate) blocker_code: Option<String>,
    pub(crate) dispatch_surface: String,
    pub(crate) dispatch_command: String,
    pub(crate) task_class: String,
    pub(crate) lifecycle_stage: String,
    pub(crate) handoff_state: String,
    pub(crate) resume_target: String,
}

impl RuntimeReceiptFixture {
    pub(crate) fn ready_downstream(
        state_dir: impl Into<String>,
        run_id: impl Into<String>,
        dispatch_target: impl Into<String>,
        downstream_target: impl Into<String>,
    ) -> Self {
        let run_id = run_id.into();
        let dispatch_target = dispatch_target.into();
        let downstream_target = downstream_target.into();
        let dispatch_packet_path = format!("runtime-consumption/dispatch-packets/{run_id}.json");
        let downstream_packet_path =
            format!("runtime-consumption/dispatch-packets/{run_id}-{downstream_target}.json");
        let result_path = format!("runtime-consumption/dispatch-results/{run_id}.json");
        Self {
            state_dir: state_dir.into(),
            run_id,
            dispatch_target: dispatch_target.clone(),
            active_node: None,
            dispatch_packet_path,
            downstream_target: downstream_target.clone(),
            downstream_packet_path,
            result_path,
            downstream_ready: true,
            downstream_status: "packet_ready".to_string(),
            downstream_blockers: Vec::new(),
            downstream_active_target: None,
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            blocker_code: None,
            dispatch_surface: "vida agent-init".to_string(),
            dispatch_command: "vida agent-init".to_string(),
            task_class: dispatch_target.clone(),
            lifecycle_stage: format!("{dispatch_target}_active"),
            handoff_state: format!("awaiting_{downstream_target}"),
            resume_target: format!("dispatch.{dispatch_target}"),
        }
    }

    pub(crate) fn from_env() -> Self {
        let state_dir = std::env::var(RECEIPT_HELPER_STATE_DIR_ENV)
            .expect("runtime receipt helper state dir should be set");
        let run_id = std::env::var(RECEIPT_HELPER_RUN_ID_ENV)
            .expect("runtime receipt helper run id should be set");
        let dispatch_target = std::env::var(RECEIPT_HELPER_DISPATCH_TARGET_ENV)
            .expect("runtime receipt helper dispatch target should be set");
        let downstream_target = std::env::var(RECEIPT_HELPER_DOWNSTREAM_TARGET_ENV)
            .expect("runtime receipt helper downstream target should be set");
        let mut fixture =
            Self::ready_downstream(state_dir, run_id, dispatch_target, downstream_target);
        fixture.dispatch_packet_path = std::env::var(RECEIPT_HELPER_DISPATCH_PACKET_PATH_ENV)
            .expect("runtime receipt helper dispatch packet path should be set");
        fixture.downstream_packet_path = std::env::var(RECEIPT_HELPER_DOWNSTREAM_PACKET_PATH_ENV)
            .expect("runtime receipt helper downstream packet path should be set");
        fixture.result_path = std::env::var(RECEIPT_HELPER_RESULT_PATH_ENV)
            .expect("runtime receipt helper result path should be set");
        fixture.active_node = std::env::var(RECEIPT_HELPER_ACTIVE_NODE_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        fixture.downstream_ready = std::env::var(RECEIPT_HELPER_DOWNSTREAM_READY_ENV)
            .map(|value| value == "true")
            .unwrap_or(true);
        fixture.downstream_status = std::env::var(RECEIPT_HELPER_DOWNSTREAM_STATUS_ENV)
            .unwrap_or_else(|_| "packet_ready".to_string());
        fixture.downstream_blockers = std::env::var(RECEIPT_HELPER_DOWNSTREAM_BLOCKERS_ENV)
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        fixture.downstream_active_target =
            std::env::var(RECEIPT_HELPER_DOWNSTREAM_ACTIVE_TARGET_ENV)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
        fixture.dispatch_status = std::env::var(RECEIPT_HELPER_DISPATCH_STATUS_ENV)
            .unwrap_or_else(|_| "executed".to_string());
        fixture.lane_status = std::env::var(RECEIPT_HELPER_LANE_STATUS_ENV)
            .unwrap_or_else(|_| "lane_completed".to_string());
        fixture.blocker_code = std::env::var(RECEIPT_HELPER_BLOCKER_CODE_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        fixture.dispatch_surface = std::env::var(RECEIPT_HELPER_DISPATCH_SURFACE_ENV)
            .unwrap_or_else(|_| "vida agent-init".to_string());
        fixture.dispatch_command = std::env::var(RECEIPT_HELPER_DISPATCH_COMMAND_ENV)
            .unwrap_or_else(|_| "vida agent-init".to_string());
        fixture.task_class = std::env::var(RECEIPT_HELPER_TASK_CLASS_ENV)
            .unwrap_or_else(|_| fixture.dispatch_target.clone());
        fixture.lifecycle_stage = std::env::var(RECEIPT_HELPER_LIFECYCLE_STAGE_ENV)
            .unwrap_or_else(|_| format!("{}_active", fixture.dispatch_target));
        fixture.handoff_state = std::env::var(RECEIPT_HELPER_HANDOFF_STATE_ENV)
            .unwrap_or_else(|_| format!("awaiting_{}", fixture.downstream_target));
        fixture.resume_target = std::env::var(RECEIPT_HELPER_RESUME_TARGET_ENV)
            .unwrap_or_else(|_| format!("dispatch.{}", fixture.dispatch_target));
        fixture
    }

    pub(crate) fn persist(&self) {
        persist_ready_downstream_receipt(
            &self.state_dir,
            &self.run_id,
            &self.dispatch_target,
            &self.dispatch_packet_path,
            &self.downstream_target,
            &self.downstream_packet_path,
            &self.result_path,
            &self.dispatch_status,
            &self.lane_status,
            self.blocker_code.clone(),
            &self.dispatch_surface,
            &self.dispatch_command,
            &self.task_class,
            self.active_node.as_deref(),
            &self.lifecycle_stage,
            &self.handoff_state,
            &self.resume_target,
            self.downstream_ready,
            &self.downstream_status,
            self.downstream_blockers.clone(),
            self.downstream_active_target.clone(),
        );
    }
}

pub(crate) struct PersistentRuntimeFixture {
    project_root: Option<PathBuf>,
    state_dir: PathBuf,
}

impl PersistentRuntimeFixture {
    pub(crate) fn state_only(label: &str) -> Self {
        let state_dir = unique_fixture_path(label);
        std::fs::create_dir_all(&state_dir).expect("runtime fixture state dir should exist");
        Self {
            project_root: None,
            state_dir,
        }
    }

    pub(crate) fn project_bound(label: &str) -> Self {
        let project_root = unique_fixture_path(label);
        let state_dir = project_root.join(".vida").join("data").join("state");
        std::fs::create_dir_all(&state_dir).expect("runtime fixture state dir should exist");
        write_project_files(&project_root);
        Self {
            project_root: Some(project_root),
            state_dir,
        }
    }

    pub(crate) fn project_bound_with_canonical_sources(label: &str, canonical_root: &Path) -> Self {
        let fixture = Self::project_bound(label);
        let project_root = fixture
            .project_root
            .as_deref()
            .expect("project-bound fixture should expose a project root");
        copy_canonical_project_sources(project_root, canonical_root);
        fixture
    }

    pub(crate) fn project_shell(label: &str) -> Self {
        let project_root = unique_fixture_path(label);
        let state_dir = project_root.join(".vida").join("data").join("state");
        write_project_files(&project_root);
        Self {
            project_root: Some(project_root),
            state_dir,
        }
    }

    pub(crate) fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub(crate) fn state_dir_string(&self) -> String {
        self.state_dir.display().to_string()
    }

    pub(crate) fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    pub(crate) fn boot(&self) {
        let output = self.capture(&["boot"]);
        assert!(
            output.status.success(),
            "boot should succeed: stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub(crate) fn cmd(&self, args: &[&str]) -> Command {
        let mut command = vida_command();
        command.args(args).env("VIDA_STATE_DIR", &self.state_dir);
        if let Some(project_root) = &self.project_root {
            command
                .current_dir(project_root)
                .env("VIDA_ROOT", project_root);
        }
        command
    }

    pub(crate) fn capture(&self, args: &[&str]) -> Output {
        run_with_state_lock_retry(|| self.cmd(args))
    }

    pub(crate) fn capture_with_state_dir(&self, args: &[&str], state_dir: Option<&Path>) -> Output {
        let project_root = self
            .project_root
            .as_ref()
            .expect("runtime fixture project root should exist");
        run_with_state_lock_retry(|| {
            let mut command = vida_command();
            command
                .args(args)
                .current_dir(project_root)
                .env_remove("VIDA_STATE_DIR");
            if let Some(state_dir) = state_dir {
                command.env("VIDA_STATE_DIR", state_dir);
            }
            command.env("VIDA_ROOT", project_root);
            command
        })
    }

    pub(crate) fn json_success(&self, args: &[&str]) -> serde_json::Value {
        let output = self.capture(args);
        assert!(
            output.status.success(),
            "{} should succeed: stdout={}; stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        parse_json_output(args, &output)
    }

    pub(crate) fn json_allow_failure(&self, args: &[&str]) -> (serde_json::Value, bool) {
        let output = self.capture(args);
        (parse_json_output(args, &output), output.status.success())
    }

    pub(crate) fn json_success_with_state_dir(
        &self,
        args: &[&str],
        state_dir: Option<&Path>,
    ) -> serde_json::Value {
        let output = self.capture_with_state_dir(args, state_dir);
        assert!(
            output.status.success(),
            "{} should succeed: stdout={}; stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        parse_json_output(args, &output)
    }

    pub(crate) fn output_success_with_state_dir(
        &self,
        args: &[&str],
        state_dir: Option<&Path>,
    ) -> Output {
        let output = self.capture_with_state_dir(args, state_dir);
        assert!(
            output.status.success(),
            "{} should succeed: stdout={}; stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    pub(crate) fn write_project_config(&self, contents: impl AsRef<str>) {
        let project_root = self
            .project_root
            .as_ref()
            .expect("runtime fixture project root should exist");
        std::fs::write(project_root.join("vida.config.yaml"), contents.as_ref())
            .expect("runtime fixture vida.config.yaml should be written");
    }

    pub(crate) fn create_epic_parent(&self, epic_id: &str, title: &str) -> serde_json::Value {
        self.json_success(&["task", "create", epic_id, title, "--type", "epic", "--json"])
    }

    pub(crate) fn create_task(
        &self,
        task_id: &str,
        title: &str,
        parent_id: &str,
    ) -> serde_json::Value {
        self.json_success(&[
            "task",
            "create",
            task_id,
            title,
            "--parent-id",
            parent_id,
            "--json",
        ])
    }

    pub(crate) fn create_task_with_owned_path(
        &self,
        task_id: &str,
        title: &str,
        parent_id: &str,
        owned_path: &str,
    ) -> serde_json::Value {
        self.json_success(&[
            "task",
            "create",
            task_id,
            title,
            "--parent-id",
            parent_id,
            "--owned-path",
            owned_path,
            "--json",
        ])
    }

    pub(crate) fn create_run_graph_backing_task(&self, run_id: &str) {
        let epic_id = format!("{run_id}-epic");
        self.create_epic_parent(&epic_id, &format!("{run_id} epic"));
        self.create_task(run_id, &format!("{run_id} task"), &epic_id);
    }

    pub(crate) fn create_authority_bound_run_graph_task(&self, run_id: &str) {
        let epic_id = format!("{run_id}-epic");
        self.create_epic_parent(&epic_id, &format!("{run_id} epic"));
        self.create_task_with_owned_path(
            run_id,
            &format!("{run_id} task"),
            &epic_id,
            &format!("docs/{run_id}.md"),
        );
    }

    pub(crate) fn runtime_consumption_path(&self, kind: &str, name: &str) -> PathBuf {
        self.state_dir
            .join("runtime-consumption")
            .join(kind)
            .join(name)
    }

    pub(crate) fn write_runtime_json(
        &self,
        kind: &str,
        name: &str,
        value: &serde_json::Value,
    ) -> PathBuf {
        let path = self.runtime_consumption_path(kind, name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("runtime fixture artifact dir should exist");
        }
        std::fs::write(
            &path,
            serde_json::to_string_pretty(value).expect("runtime fixture json should serialize"),
        )
        .expect("runtime fixture json should be written");
        path
    }

    pub(crate) fn receipt(
        &self,
        run_id: &str,
        dispatch_target: &str,
        downstream_target: &str,
    ) -> RuntimeReceiptFixture {
        RuntimeReceiptFixture::ready_downstream(
            self.state_dir_string(),
            run_id,
            dispatch_target,
            downstream_target,
        )
    }

    pub(crate) fn persist_receipt(&self, fixture: &RuntimeReceiptFixture) {
        fixture.persist();
    }

    pub(crate) fn delete_row(&self, table: &str, id: &str) {
        delete_run_graph_row(&self.state_dir_string(), table, id);
    }
}

impl Drop for PersistentRuntimeFixture {
    fn drop(&mut self) {
        let root = self
            .project_root
            .as_ref()
            .unwrap_or(&self.state_dir)
            .to_path_buf();
        let _ = std::fs::remove_dir_all(root);
    }
}

#[allow(dead_code)]
#[derive(serde::Serialize, serde::Deserialize, SurrealValue)]
struct TestRunGraphDispatchReceiptRow {
    run_id: String,
    dispatch_target: String,
    dispatch_status: String,
    lane_status: Option<String>,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
    dispatch_kind: String,
    dispatch_surface: Option<String>,
    dispatch_command: Option<String>,
    dispatch_packet_path: Option<String>,
    dispatch_result_path: Option<String>,
    blocker_code: Option<String>,
    downstream_dispatch_target: Option<String>,
    downstream_dispatch_command: Option<String>,
    downstream_dispatch_note: Option<String>,
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: Vec<String>,
    downstream_dispatch_packet_path: Option<String>,
    downstream_dispatch_status: Option<String>,
    downstream_dispatch_result_path: Option<String>,
    downstream_dispatch_trace_path: Option<String>,
    downstream_dispatch_executed_count: u32,
    downstream_dispatch_active_target: Option<String>,
    downstream_dispatch_last_target: Option<String>,
    activation_agent_type: Option<String>,
    activation_runtime_role: Option<String>,
    selected_backend: Option<String>,
    recorded_at: String,
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

pub(crate) fn persist_ready_downstream_receipt_from_env() {
    RuntimeReceiptFixture::from_env().persist();
}

pub(crate) fn clear_protocol_binding_receipts_from_env() {
    let state_dir = std::env::var(PROTOCOL_BINDING_CLEAR_STATE_DIR_ENV)
        .expect("protocol binding clear helper state dir should be set");
    clear_protocol_binding_receipts(&state_dir);
}

pub(crate) fn delete_run_graph_row_from_env() {
    let state_dir = std::env::var(RUN_GRAPH_DELETE_STATE_DIR_ENV)
        .expect("run graph delete helper state dir should be set");
    let table = std::env::var(RUN_GRAPH_DELETE_TABLE_ENV)
        .expect("run graph delete helper table should be set");
    let run_id = std::env::var(RUN_GRAPH_DELETE_RUN_ID_ENV)
        .expect("run graph delete helper run id should be set");
    delete_run_graph_row(&state_dir, &table, &run_id);
}

pub(crate) fn delete_run_graph_row(state_dir: &str, table: &str, run_id: &str) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db = open_state_db_with_retry(state_dir).await;
        let _: Option<serde_json::Value> = db
            .delete((table, run_id))
            .await
            .expect("run graph test row should delete");
        drop(db);
    });
    runtime.shutdown_timeout(Duration::from_millis(250));
}

fn clear_protocol_binding_receipts(state_dir: &str) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db = open_state_db_with_retry(state_dir).await;
        db.query("DELETE protocol_binding_receipt;")
            .await
            .expect("protocol binding receipts should clear");
        drop(db);
    });
    runtime.shutdown_timeout(Duration::from_millis(250));
}

fn persist_ready_downstream_receipt(
    state_dir: &str,
    run_id: &str,
    dispatch_target: &str,
    dispatch_packet_path: &str,
    downstream_target: &str,
    downstream_packet_path: &str,
    result_path: &str,
    dispatch_status: &str,
    lane_status: &str,
    blocker_code: Option<String>,
    dispatch_surface: &str,
    dispatch_command: &str,
    task_class: &str,
    active_node: Option<&str>,
    lifecycle_stage: &str,
    handoff_state: &str,
    resume_target: &str,
    downstream_ready: bool,
    downstream_status: &str,
    downstream_blockers: Vec<String>,
    downstream_active_target: Option<String>,
) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db = open_state_db_with_retry(state_dir).await;
        let updated_at = "2026-06-05T00:00:00Z".to_string();
        let status_label = if dispatch_status == "executed" {
            "ready"
        } else {
            "blocked"
        };
        let _: Option<TestRoutedRunStateRow> = db
            .upsert(("routed_run_state", run_id))
            .content(TestRoutedRunStateRow {
                run_id: run_id.to_string(),
                route_task_class: task_class.to_string(),
                selected_backend: "internal_subagents".to_string(),
                lane_id: format!("{dispatch_target}_lane"),
                lifecycle_stage: lifecycle_stage.to_string(),
                updated_at: updated_at.clone(),
            })
            .await
            .expect("runtime receipt helper should persist routed run state");
        let _: Option<TestGovernanceStateRow> = db
            .upsert(("governance_state", run_id))
            .content(TestGovernanceStateRow {
                run_id: run_id.to_string(),
                policy_gate: "test_fixture".to_string(),
                handoff_state: handoff_state.to_string(),
                context_state: "active".to_string(),
                updated_at: updated_at.clone(),
            })
            .await
            .expect("runtime receipt helper should persist governance state");
        let _: Option<TestResumabilityCapsuleRow> = db
            .upsert(("resumability_capsule", run_id))
            .content(TestResumabilityCapsuleRow {
                run_id: run_id.to_string(),
                checkpoint_kind: "runtime_consumption_test_fixture".to_string(),
                resume_target: resume_target.to_string(),
                recovery_ready: dispatch_status == "executed",
                updated_at: updated_at.clone(),
            })
            .await
            .expect("runtime receipt helper should persist resumability capsule");
        let _: Option<TestExecutionPlanStateRow> = db
            .upsert(("execution_plan_state", run_id))
            .content(TestExecutionPlanStateRow {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                task_class: task_class.to_string(),
                active_node: active_node.unwrap_or(dispatch_target).to_string(),
                next_node: Some(downstream_target.to_string()),
                status: status_label.to_string(),
                updated_at,
            })
            .await
            .expect("runtime receipt helper should persist execution plan state");
        let row = TestRunGraphDispatchReceiptRow {
            run_id: run_id.to_string(),
            dispatch_target: dispatch_target.to_string(),
            dispatch_status: dispatch_status.to_string(),
            lane_status: Some(lane_status.to_string()),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some(dispatch_surface.to_string()),
            dispatch_command: Some(dispatch_command.to_string()),
            dispatch_packet_path: Some(dispatch_packet_path.to_string()),
            dispatch_result_path: Some(result_path.to_string()),
            blocker_code,
            downstream_dispatch_target: Some(downstream_target.to_string()),
            downstream_dispatch_command: Some("vida taskflow consume continue".to_string()),
            downstream_dispatch_note: Some("receipt-backed downstream packet fixture".to_string()),
            downstream_dispatch_ready: downstream_ready,
            downstream_dispatch_blockers: downstream_blockers,
            downstream_dispatch_packet_path: Some(downstream_packet_path.to_string()),
            downstream_dispatch_status: Some(downstream_status.to_string()),
            downstream_dispatch_result_path: Some(result_path.to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: downstream_active_target,
            downstream_dispatch_last_target: Some(dispatch_target.to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some(dispatch_target.to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-05T00:00:00Z".to_string(),
        };
        let _: Option<TestRunGraphDispatchReceiptRow> = db
            .upsert(("run_graph_dispatch_receipt", run_id))
            .content(row)
            .await
            .expect("runtime receipt helper should persist dispatch receipt");
        drop(db);
    });
    runtime.shutdown_timeout(Duration::from_millis(250));
}

async fn open_state_db_with_retry(state_dir: &str) -> Surreal<Db> {
    let mut last_error = None;
    for attempt in 0..MAX_OPEN_RETRIES {
        match Surreal::new::<SurrealKv>(PathBuf::from(state_dir)).await {
            Ok(db) => match db.use_ns("vida").use_db("primary").await {
                Ok(_) => return db,
                Err(error) if is_lock_error(&error.to_string()) => {
                    last_error = Some(error.to_string());
                    drop(db);
                    tokio::time::sleep(backoff_delay(attempt)).await;
                }
                Err(error) => panic!("runtime receipt helper state namespace should open: {error}"),
            },
            Err(error) if is_lock_error(&error.to_string()) => {
                last_error = Some(error.to_string());
                tokio::time::sleep(backoff_delay(attempt)).await;
            }
            Err(error) => panic!("runtime receipt helper state db should open: {error}"),
        }
    }

    panic!(
        "runtime receipt helper state db should open after retries: {}",
        last_error.unwrap_or_else(|| "unknown lock error".to_string())
    );
}

fn is_lock_error(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("lock")
        || lowered.contains("being used by another process")
        || lowered.contains("access is denied")
        || lowered.contains("could not acquire")
}

fn backoff_delay(attempt: usize) -> Duration {
    let millis = 25_u64.saturating_mul((attempt as u64).saturating_add(1));
    Duration::from_millis(millis.min(250))
}

fn unique_fixture_path(label: &str) -> PathBuf {
    let counter = UNIQUE_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    external_temp_dir().join(format!(
        "vida-{label}-{}-{nanos}-{counter}",
        std::process::id()
    ))
}

fn external_temp_dir() -> PathBuf {
    std::env::var_os("VIDA_TEST_EXTERNAL_TMP")
        .map(PathBuf::from)
        .or_else(|| outside_repo_temp_base(std::env::temp_dir()))
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|path| path.join("Temp"))
        })
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|path| path.join("AppData").join("Local").join("Temp"))
        })
        .unwrap_or_else(std::env::temp_dir)
}

fn outside_repo_temp_base(temp_dir: PathBuf) -> Option<PathBuf> {
    temp_dir
        .ancestors()
        .find(|path| {
            path.join("AGENTS.md").exists()
                && path.join("AGENTS.sidecar.md").exists()
                && path.join(".vida").exists()
        })
        .and_then(Path::parent)
        .map(|path| path.join(".vida-test-temp"))
}

fn write_project_files(project_root: &Path) {
    std::fs::create_dir_all(project_root.join(".vida").join("config"))
        .expect("runtime fixture project config dir should exist");
    std::fs::create_dir_all(project_root.join(".vida").join("db"))
        .expect("runtime fixture project db dir should exist");
    std::fs::create_dir_all(project_root.join(".vida").join("project"))
        .expect("runtime fixture project marker dir should exist");
    std::fs::write(project_root.join("AGENTS.md"), "# Test agents\n")
        .expect("runtime fixture AGENTS.md should be written");
    std::fs::write(project_root.join("AGENTS.sidecar.md"), "# Test sidecar\n")
        .expect("runtime fixture AGENTS.sidecar.md should be written");
    std::fs::write(project_root.join("vida.config.yaml"), "project_id: test\n")
        .expect("runtime fixture vida.config.yaml should be written");
}

fn copy_canonical_project_sources(project_root: &Path, canonical_root: &Path) {
    let canonical_root = std::fs::canonicalize(canonical_root)
        .expect("canonical source project root should resolve");
    let canonical_config = canonical_root.join("vida.config.yaml");
    let target_config = project_root.join("vida.config.yaml");
    std::fs::copy(&canonical_config, &target_config)
        .expect("canonical vida.config.yaml should copy into project fixture");
    let config: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(&canonical_config)
            .expect("canonical vida.config.yaml should be readable"),
    )
    .expect("canonical vida.config.yaml should parse");
    let registries = config
        .get("agent_extensions")
        .and_then(|value| value.get("registries"))
        .and_then(serde_yaml::Value::as_mapping)
        .expect("canonical config should declare agent extension registries");
    for relative in registries.values().filter_map(serde_yaml::Value::as_str) {
        copy_canonical_source_file(&canonical_root, project_root, relative);
    }
    for relative in [
        "vida/config/instructions/bundles/framework-source",
        "vida/config/instructions/bundles/framework-memory-source",
    ] {
        copy_canonical_source_tree(&canonical_root, project_root, relative);
    }
}

fn copy_canonical_source_file(canonical_root: &Path, project_root: &Path, relative: &str) {
    let source = canonical_root.join(relative);
    let target = project_root.join(relative);
    assert!(
        source.starts_with(canonical_root),
        "canonical registry source must remain under canonical project root: {relative}"
    );
    assert!(
        target.starts_with(project_root),
        "fixture registry target must remain under fixture project root: {relative}"
    );
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).expect("fixture registry target parent should exist");
    }
    std::fs::copy(&source, &target).unwrap_or_else(|error| {
        panic!("canonical registry source should copy ({relative}): {error}")
    });
}

fn copy_canonical_source_tree(canonical_root: &Path, project_root: &Path, relative: &str) {
    let source = canonical_root.join(relative);
    let target = project_root.join(relative);
    assert!(
        source.starts_with(canonical_root),
        "canonical instruction source must remain under canonical project root: {relative}"
    );
    assert!(
        target.starts_with(project_root),
        "fixture instruction target must remain under fixture project root: {relative}"
    );
    copy_canonical_source_tree_at(&source, &target, relative);
}

fn copy_canonical_source_tree_at(source: &Path, target: &Path, relative: &str) {
    std::fs::create_dir_all(target).unwrap_or_else(|error| {
        panic!("canonical instruction target should be creatable ({relative}): {error}")
    });
    for entry in std::fs::read_dir(source).unwrap_or_else(|error| {
        panic!("canonical instruction source should be readable ({relative}): {error}")
    }) {
        let entry = entry.expect("canonical instruction source entry should resolve");
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry
            .file_type()
            .expect("canonical instruction source entry type should resolve")
            .is_dir()
        {
            copy_canonical_source_tree_at(&source_path, &target_path, relative);
        } else {
            std::fs::copy(&source_path, &target_path).unwrap_or_else(|error| {
                panic!("canonical instruction source should copy ({relative}): {error}")
            });
        }
    }
}

fn vida_command() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn run_with_state_lock_retry<F>(mut build: F) -> Output
where
    F: FnMut() -> Command,
{
    let mut last_output = None;
    for attempt in 0..6 {
        let output = build()
            .output()
            .unwrap_or_else(|error| panic!("vida command should run: {error}"));
        if !is_state_lock_output(&output) {
            return output;
        }
        last_output = Some(output);
        std::thread::sleep(Duration::from_millis(150 * (attempt + 1)));
    }
    last_output.expect("state lock retry should record output")
}

fn is_state_lock_output(output: &Output) -> bool {
    if output.status.success() {
        return false;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    is_state_lock_output_text(&stdout, &stderr)
}

fn is_state_lock_output_text(stdout: &str, stderr: &str) -> bool {
    has_retryable_state_lock_signal(stdout, stderr)
        || stderr.contains("authoritative_state_required_for_mutation")
        || stdout.contains("authoritative_state_required_for_mutation")
}

pub(crate) fn has_retryable_state_lock_signal(stdout: &str, stderr: &str) -> bool {
    [stdout, stderr].iter().any(|stream| {
        stream
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .any(|token| {
                matches!(
                    token,
                    "state_store_read_lock_contention" | "authoritative_state_store_locked"
                )
            })
            || stream.contains("timed out while waiting for authoritative datastore lock")
    })
}

fn parse_json_output(args: &[&str], output: &Output) -> serde_json::Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_lock_output_text_retries_only_transient_lock_blockers() {
        for retryable in [
            r#"{"blocker_codes":["authoritative_state_store_locked"]}"#,
            "blocker_codes[1]: state_store_read_lock_contention",
            "timed out while waiting for authoritative datastore lock",
        ] {
            assert!(
                is_state_lock_output_text(retryable, ""),
                "transient lock signal should be retryable: {retryable}"
            );
        }

        for non_retryable in [
            r#"{"blocker_codes":["authoritative_state_store_open_failed"],"error_kind":"permission_access"}"#,
            r#"{"blocker_codes":["authoritative_state_store_open_failed"],"error_kind":"storage_corruption"}"#,
            r#"{"blocker_codes":["authoritative_state_store_open_failed"],"error_kind":"unknown"}"#,
            r#"{"blocker_codes":["state_store_surrealkv_wal_replay_corruption"]}"#,
        ] {
            assert!(
                !is_state_lock_output_text(non_retryable, ""),
                "non-lock state failure must not be retried: {non_retryable}"
            );
        }
    }
}
