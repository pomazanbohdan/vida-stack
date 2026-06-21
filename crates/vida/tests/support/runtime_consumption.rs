#![allow(dead_code, clippy::suspicious_open_options, clippy::too_many_arguments)]

use std::path::PathBuf;
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
    let state_dir = std::env::var(RECEIPT_HELPER_STATE_DIR_ENV)
        .expect("runtime receipt helper state dir should be set");
    let run_id = std::env::var(RECEIPT_HELPER_RUN_ID_ENV)
        .expect("runtime receipt helper run id should be set");
    let dispatch_target = std::env::var(RECEIPT_HELPER_DISPATCH_TARGET_ENV)
        .expect("runtime receipt helper dispatch target should be set");
    let dispatch_packet_path = std::env::var(RECEIPT_HELPER_DISPATCH_PACKET_PATH_ENV)
        .expect("runtime receipt helper dispatch packet path should be set");
    let downstream_target = std::env::var(RECEIPT_HELPER_DOWNSTREAM_TARGET_ENV)
        .expect("runtime receipt helper downstream target should be set");
    let downstream_packet_path = std::env::var(RECEIPT_HELPER_DOWNSTREAM_PACKET_PATH_ENV)
        .expect("runtime receipt helper downstream packet path should be set");
    let result_path = std::env::var(RECEIPT_HELPER_RESULT_PATH_ENV)
        .expect("runtime receipt helper result path should be set");
    let downstream_ready = std::env::var(RECEIPT_HELPER_DOWNSTREAM_READY_ENV)
        .map(|value| value == "true")
        .unwrap_or(true);
    let downstream_status = std::env::var(RECEIPT_HELPER_DOWNSTREAM_STATUS_ENV)
        .unwrap_or_else(|_| "packet_ready".to_string());
    let downstream_blockers = std::env::var(RECEIPT_HELPER_DOWNSTREAM_BLOCKERS_ENV)
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
    let dispatch_status = std::env::var(RECEIPT_HELPER_DISPATCH_STATUS_ENV)
        .unwrap_or_else(|_| "executed".to_string());
    let lane_status = std::env::var(RECEIPT_HELPER_LANE_STATUS_ENV)
        .unwrap_or_else(|_| "lane_completed".to_string());
    let blocker_code = std::env::var(RECEIPT_HELPER_BLOCKER_CODE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let dispatch_surface = std::env::var(RECEIPT_HELPER_DISPATCH_SURFACE_ENV)
        .unwrap_or_else(|_| "vida agent-init".to_string());
    let dispatch_command = std::env::var(RECEIPT_HELPER_DISPATCH_COMMAND_ENV)
        .unwrap_or_else(|_| "vida agent-init".to_string());
    let task_class =
        std::env::var(RECEIPT_HELPER_TASK_CLASS_ENV).unwrap_or_else(|_| dispatch_target.clone());
    let lifecycle_stage = std::env::var(RECEIPT_HELPER_LIFECYCLE_STAGE_ENV)
        .unwrap_or_else(|_| format!("{dispatch_target}_active"));
    let handoff_state = std::env::var(RECEIPT_HELPER_HANDOFF_STATE_ENV)
        .unwrap_or_else(|_| format!("awaiting_{downstream_target}"));
    let resume_target = std::env::var(RECEIPT_HELPER_RESUME_TARGET_ENV)
        .unwrap_or_else(|_| format!("dispatch.{dispatch_target}"));

    persist_ready_downstream_receipt(
        &state_dir,
        &run_id,
        &dispatch_target,
        &dispatch_packet_path,
        &downstream_target,
        &downstream_packet_path,
        &result_path,
        &dispatch_status,
        &lane_status,
        blocker_code,
        &dispatch_surface,
        &dispatch_command,
        &task_class,
        &lifecycle_stage,
        &handoff_state,
        &resume_target,
        downstream_ready,
        &downstream_status,
        downstream_blockers,
    );
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
    lifecycle_stage: &str,
    handoff_state: &str,
    resume_target: &str,
    downstream_ready: bool,
    downstream_status: &str,
    downstream_blockers: Vec<String>,
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
        let active_node = std::env::var(RECEIPT_HELPER_ACTIVE_NODE_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| dispatch_target.to_string());
        let _: Option<TestExecutionPlanStateRow> = db
            .upsert(("execution_plan_state", run_id))
            .content(TestExecutionPlanStateRow {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                task_class: task_class.to_string(),
                active_node,
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
            downstream_dispatch_active_target: None,
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
