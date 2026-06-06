use std::path::PathBuf;
use std::time::Duration;

use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;

pub(crate) const RECEIPT_HELPER_STATE_DIR_ENV: &str = "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_STATE_DIR";
pub(crate) const RECEIPT_HELPER_RUN_ID_ENV: &str = "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_RUN_ID";
pub(crate) const RECEIPT_HELPER_DISPATCH_TARGET_ENV: &str =
    "VIDA_BOOT_SMOKE_RUNTIME_RECEIPT_DISPATCH_TARGET";
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
pub(crate) const PROTOCOL_BINDING_CLEAR_STATE_DIR_ENV: &str =
    "VIDA_BOOT_SMOKE_PROTOCOL_BINDING_CLEAR_STATE_DIR";

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

    persist_ready_downstream_receipt(
        &state_dir,
        &run_id,
        &dispatch_target,
        &dispatch_packet_path,
        &downstream_target,
        &downstream_packet_path,
        &result_path,
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
    downstream_ready: bool,
    downstream_status: &str,
    downstream_blockers: Vec<String>,
) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime.block_on(async {
        let db = open_state_db_with_retry(state_dir).await;
        let row = TestRunGraphDispatchReceiptRow {
            run_id: run_id.to_string(),
            dispatch_target: dispatch_target.to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: Some("lane_completed".to_string()),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(dispatch_packet_path.to_string()),
            dispatch_result_path: Some(result_path.to_string()),
            blocker_code: None,
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
            selected_backend: Some("middle".to_string()),
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
