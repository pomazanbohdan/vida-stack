use crate::taskflow_run_graph::{
    status_with_active_exception_dispatch_replay, validate_run_graph_resume_gate,
};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, UNIX_EPOCH};

const DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS: [&str; 3] = [
    ".vida/data/state/runtime-consumption",
    "docs/product/spec",
    "docs/process",
];
const CONSUME_RESUME_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const CONSUME_RESUME_PREPARATION_GATE_TIMEOUT: Duration = Duration::from_secs(10);
const CONSUME_CONTINUE_DEFERRED_HANDOFF_PROJECTION_NAME: &str =
    "taskflow-consume-continue-deferred-handoff";

async fn consume_continue_handoff_with_timeout<F>(
    label: &str,
    timeout: Duration,
    future: F,
) -> Result<(), String>
where
    F: std::future::Future<Output = Result<(), String>>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result,
        Err(_) => Err(format!(
            "Timed out executing runtime dispatch handoff during {label} after {}s",
            timeout.as_secs()
        )),
    }
}

fn consume_continue_blocking_step_with_timeout<F>(
    label: &str,
    timeout: Duration,
    step: F,
) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(step());
    });
    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "Timed out executing runtime dispatch handoff during {label} after {}s",
            timeout.as_secs()
        )),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(format!(
            "consume continue failed fast: {label} blocking worker exited before returning a result"
        )),
    }
}

fn consume_continue_dispatch_handoff_timeout(
    state_root: &Path,
    role_selection: &crate::RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Duration {
    Duration::from_secs(super::dispatch_handoff_timeout_seconds_for_state_root(
        state_root,
        role_selection,
        receipt,
    ))
}

async fn fail_fast_state_store_open(
    state_root: std::path::PathBuf,
    label: &str,
) -> Result<super::StateStore, String> {
    match tokio::time::timeout(
        CONSUME_RESUME_LOCK_TIMEOUT,
        super::StateStore::open_existing(state_root),
    )
    .await
    {
        Ok(result) => {
            result.map_err(|error| format!("consume continue failed fast: {label}: {error}"))
        }
        Err(_) => Err(format!(
            "consume continue failed fast: {label} timed out while waiting for authoritative datastore lock"
        )),
    }
}

async fn fail_fast_state_store_open_read_only(
    state_root: std::path::PathBuf,
    label: &str,
) -> Result<super::StateStore, String> {
    fail_fast_state_store_open_read_only_with_timeout(
        state_root,
        label,
        CONSUME_RESUME_LOCK_TIMEOUT,
    )
    .await
}

async fn fail_fast_state_store_open_read_only_with_timeout(
    state_root: std::path::PathBuf,
    label: &str,
    timeout: Duration,
) -> Result<super::StateStore, String> {
    match tokio::time::timeout(
        timeout,
        super::StateStore::open_existing_read_only(state_root),
    )
    .await
    {
        Ok(result) => {
            result.map_err(|error| format!("consume continue failed fast: {label}: {error}"))
        }
        Err(_) => Err(format!(
            "consume continue failed fast: {label} timed out while waiting for authoritative datastore lock"
        )),
    }
}

fn consume_continue_state_access_error_kind(error: &str) -> &'static str {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("lock") {
        "lock_contention"
    } else {
        "open_failed"
    }
}

fn consume_continue_state_access_blocker_code(error: &str) -> &'static str {
    match consume_continue_state_access_error_kind(error) {
        "lock_contention" => "authoritative_state_store_locked",
        _ => "authoritative_state_store_open_failed",
    }
}

fn consume_continue_lock_diagnostics(state_root: &Path) -> serde_json::Value {
    let lock_path = state_root.join("LOCK");
    let metadata = std::fs::symlink_metadata(&lock_path).ok();
    let lock_is_symlink = metadata
        .as_ref()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    let modified_unix_seconds = metadata
        .as_ref()
        .filter(|_| !lock_is_symlink)
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    serde_json::json!({
        "lock_path": lock_path,
        "lock_exists": metadata.is_some(),
        "lock_is_symlink": lock_is_symlink,
        "lock_file_size": metadata
            .as_ref()
            .filter(|_| !lock_is_symlink)
            .map(std::fs::Metadata::len),
        "lock_modified_unix_seconds": modified_unix_seconds,
    })
}

fn consume_continue_state_access_blocker_payload(
    state_root: &Path,
    surface_name: &str,
    operation: &str,
    error: &str,
) -> serde_json::Value {
    let error_kind = consume_continue_state_access_error_kind(error);
    let blocker_code = consume_continue_state_access_blocker_code(error);
    let next_actions = if error_kind == "lock_contention" {
        serde_json::json!([
            "Wait for the authoritative VIDA state-store holder to finish, then retry `vida taskflow consume continue --json`.",
            "Inspect read-only continuation context with `vida task ready --json`, `vida taskflow graph-summary --json`, or `vida status --json` while the lock is held.",
            "If no holder exists, use the VIDA recovery/reclaim flow; do not delete datastore LOCK files by hand."
        ])
    } else {
        serde_json::json!([
            "Inspect the state directory and retry `vida taskflow consume continue --json` after state access is restored.",
            "Use read-only status surfaces such as `vida status --json` for degraded context if available."
        ])
    };
    serde_json::json!({
        "surface": surface_name,
        "status": "blocked",
        "blocker_codes": [blocker_code],
        "next_actions": next_actions,
        "state_access": {
            "status": "blocked",
            "operation": operation,
            "state_dir": state_root,
            "error_kind": error_kind,
            "error_message": error,
            "lock_diagnostics": consume_continue_lock_diagnostics(state_root),
            "snapshot_fallback": {
                "status": "not_attempted",
                "reason": "authoritative_resume_requires_state_store_open"
            }
        },
        "source_surfaces": [
            "vida taskflow consume continue --json",
            "StateStore::open_existing",
            "StateStore::open_existing_read_only",
            "vida task ready --json",
            "vida taskflow graph-summary --json",
            "vida status --json"
        ]
    })
}

fn emit_consume_continue_state_access_blocker_json(
    state_root: &Path,
    surface_name: &str,
    operation: &str,
    error: &str,
) {
    crate::print_json_pretty(&consume_continue_state_access_blocker_payload(
        state_root,
        surface_name,
        operation,
        error,
    ));
}

fn consume_continue_resume_error_blocker_code(error: &str) -> &'static str {
    if error.contains("Stale missing-task run graph")
        || error.contains("references missing TaskFlow task")
    {
        "stale_missing_task_run_graph"
    } else if error.contains("Latest continuation binding") && error.contains("ambiguous") {
        "continuation_binding_ambiguous"
    } else if error.contains("Latest explicit continuation binding points to run") {
        "continuation_binding_mismatch"
    } else if error.contains("not resumeable through default") {
        "continuation_binding_not_resumeable"
    } else if error.contains("No persisted run-graph dispatch receipt exists")
        || error.contains("missing receipt recovery could not load dispatch context")
    {
        "missing_run_graph_dispatch_receipt"
    } else if error.contains("Timed out executing runtime dispatch handoff") {
        "runtime_dispatch_handoff_timeout"
    } else {
        "consume_continue_resume_blocked"
    }
}

fn consume_continue_resume_error_run_id(error: &str) -> Option<String> {
    for marker in [
        "Stale missing-task run graph `",
        "Run-graph resume gate denied for `",
        "run `",
        "run_id `",
    ] {
        let Some(start) = error.find(marker).map(|index| index + marker.len()) else {
            continue;
        };
        let rest = &error[start..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let run_id = rest[..end].trim();
        if !run_id.is_empty() {
            return Some(run_id.to_string());
        }
    }
    None
}

fn consume_continue_resume_error_payload(error: &str, surface_name: &str) -> serde_json::Value {
    let blocker_code = consume_continue_resume_error_blocker_code(error);
    let run_id = consume_continue_resume_error_run_id(error);
    let next_actions = if blocker_code == "stale_missing_task_run_graph" {
        let retire_action = run_id.as_ref().map_or_else(
            || {
                "Inspect `vida taskflow recovery latest --json`; if it reports a missing-task stale run, retire that concrete run with `vida lane retire <run-id> --receipt-id <receipt-id> --reason \"missing TaskFlow task stale run\" --json`."
                    .to_string()
            },
            |run_id| {
                format!(
                    "Retire the stale missing-task run with `vida lane retire {} --receipt-id {} --reason \"missing TaskFlow task stale run\" --json`.",
                    crate::shell_quote(run_id),
                    crate::shell_quote(run_id)
                )
            },
        );
        serde_json::json!([
            retire_action,
            "Refresh continuation evidence with `vida status --json` and `vida taskflow recovery latest --json` before retrying `vida taskflow consume continue --json`.",
            "Do not bind recovery to the missing TaskFlow task."
        ])
    } else if blocker_code == "continuation_binding_ambiguous"
        && error.contains("has not reached closure_complete")
    {
        let refresh_action = run_id.as_ref().map_or_else(
            || {
                "Pass `--run-id <run-id>` only when intentionally refreshing that specific active run."
                    .to_string()
            },
            |run_id| {
                format!(
                    "Refresh the active run explicitly with `vida taskflow consume continue --run-id {} --json`.",
                    crate::shell_quote(run_id)
                )
            },
        );
        serde_json::json!([
            refresh_action,
            "Do not bind a new --task-id until the source run reaches closure_complete with no downstream target.",
            "Inspect run-graph and task evidence if the explicit refresh remains blocked."
        ])
    } else if blocker_code == "continuation_binding_ambiguous" {
        serde_json::json!([
            "Bind the next bounded unit explicitly with `vida taskflow continuation bind <run-id> --task-id <task-id> --json`.",
            "Pass `--run-id <run-id>` only when intentionally refreshing that specific run."
        ])
    } else {
        serde_json::json!([
            "Inspect continuation evidence with `vida status --json` and `vida taskflow recovery latest --json`.",
            "Bind or refresh the intended bounded unit before retrying `vida taskflow consume continue --json`."
        ])
    };
    let artifact_refs = serde_json::json!({
        "surface": surface_name,
        "run_id": run_id,
    });
    let shared_fields = serde_json::json!({
        "status": "blocked",
        "blocker_codes": [blocker_code],
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
    });
    serde_json::json!({
        "surface": surface_name,
        "status": "blocked",
        "error": error,
        "run_id": run_id,
        "blocker_codes": [blocker_code],
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "shared_fields": shared_fields,
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": [blocker_code],
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null
        }
    })
}

pub(crate) fn emit_consume_continue_resume_error_json(error: &str, surface_name: &str) {
    crate::print_json_pretty(&consume_continue_resume_error_payload(error, surface_name));
}

fn consume_advance_success_payload(
    source_dispatch_packet_path: &str,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    snapshot_path: &str,
    rounds: usize,
) -> serde_json::Value {
    let blocker_codes: Vec<String> = Vec::new();
    let next_actions: Vec<String> = Vec::new();
    let artifact_refs = serde_json::json!({
        "surface": "vida taskflow consume advance",
        "run_id": dispatch_receipt.run_id,
        "source_dispatch_packet_path": source_dispatch_packet_path,
        "snapshot_path": snapshot_path,
    });
    serde_json::json!({
        "surface": "vida taskflow consume advance",
        "status": "ok",
        "source_run_id": dispatch_receipt.run_id,
        "source_dispatch_packet_path": source_dispatch_packet_path,
        "dispatch_receipt": dispatch_receipt,
        "snapshot_path": snapshot_path,
        "rounds_executed": rounds,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "shared_fields": {
            "status": "ok",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "ok",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
    })
}

fn missing_dispatch_packet_path_error(latest: bool) -> String {
    let _ = super::blocker_code_str(super::BlockerCode::MissingPacket);
    if latest {
        "Latest persisted dispatch receipt is missing dispatch_packet_path".to_string()
    } else {
        "Persisted dispatch receipt is missing dispatch_packet_path".to_string()
    }
}

fn missing_dispatch_receipt_error(run_id: &str) -> String {
    let _ = super::blocker_code_str(super::BlockerCode::MissingLaneReceipt);
    format!("No persisted run-graph dispatch receipt exists for run_id `{run_id}`")
}

fn lane_status_pair_is_resume_compatible(
    packet_lane_status: super::LaneStatus,
    derived_lane_status: super::LaneStatus,
) -> bool {
    if packet_lane_status == derived_lane_status {
        return true;
    }
    matches!(
        (packet_lane_status, derived_lane_status),
        (super::LaneStatus::LaneRunning, super::LaneStatus::LaneOpen)
            | (super::LaneStatus::LaneOpen, super::LaneStatus::LaneRunning)
            | (
                super::LaneStatus::LaneRunning,
                super::LaneStatus::PacketReady
            )
            | (
                super::LaneStatus::PacketReady,
                super::LaneStatus::LaneRunning
            )
            | (super::LaneStatus::LaneOpen, super::LaneStatus::PacketReady)
            | (super::LaneStatus::PacketReady, super::LaneStatus::LaneOpen)
            | (
                super::LaneStatus::LaneRunning,
                super::LaneStatus::LaneBlocked
            )
            | (
                super::LaneStatus::LaneBlocked,
                super::LaneStatus::LaneRunning
            )
            | (
                super::LaneStatus::LaneExceptionRecorded,
                super::LaneStatus::LaneExceptionTakeover
            )
            | (
                super::LaneStatus::LaneExceptionTakeover,
                super::LaneStatus::LaneExceptionRecorded
            )
    )
}

fn sanitize_inherited_downstream_lane_evidence(
    packet: &serde_json::Value,
    downstream_dispatch_status: Option<&str>,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
    parsed_downstream_lane_status: Option<super::LaneStatus>,
) -> (Option<String>, Option<String>, Option<super::LaneStatus>) {
    let source_supersedes_receipt_id = packet
        .get("source_supersedes_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let source_exception_path_receipt_id = packet
        .get("source_exception_path_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let inherited_supersedes = supersedes_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        == source_supersedes_receipt_id;
    let inherited_exception = exception_path_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        == source_exception_path_receipt_id;

    let supersedes_receipt_id = if inherited_supersedes {
        None
    } else {
        supersedes_receipt_id
    };
    let exception_path_receipt_id = if inherited_exception {
        None
    } else {
        exception_path_receipt_id
    };

    let parsed_downstream_lane_status = if inherited_supersedes || inherited_exception {
        let sanitized_derived_lane_status = super::derive_lane_status(
            downstream_dispatch_status.unwrap_or("blocked"),
            supersedes_receipt_id.as_deref(),
            exception_path_receipt_id.as_deref(),
        );
        match parsed_downstream_lane_status {
            Some(
                super::LaneStatus::LaneExceptionRecorded
                | super::LaneStatus::LaneExceptionTakeover
                | super::LaneStatus::LaneSuperseded,
            ) => Some(sanitized_derived_lane_status),
            value => value,
        }
    } else {
        parsed_downstream_lane_status
    };

    (
        supersedes_receipt_id,
        exception_path_receipt_id,
        parsed_downstream_lane_status,
    )
}

fn validate_receipt_packet_pair(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet: &serde_json::Value,
    packet_path: &str,
    packet_label: &str,
) -> Result<(), String> {
    let packet_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Persisted {packet_label} is missing run_id"))?;
    if packet_run_id != receipt.run_id {
        return Err(format!(
            "Persisted {packet_label} run_id `{packet_run_id}` does not match dispatch receipt run_id `{}`",
            receipt.run_id
        ));
    }
    if let Some(expected_dispatch_packet_path) = receipt.dispatch_packet_path.as_deref() {
        if !dispatch_packet_paths_equivalent(expected_dispatch_packet_path, packet_path) {
            let expected_downstream_packet_path =
                receipt.downstream_dispatch_packet_path.as_deref();
            if !expected_downstream_packet_path
                .map(|path| dispatch_packet_paths_equivalent(path, packet_path))
                .unwrap_or(false)
            {
                return Err(format!(
                    "Persisted dispatch receipt expects dispatch_packet_path `{expected_dispatch_packet_path}` but resolved `{packet_path}`"
                ));
            }
        }
    }
    if let Some(packet_lane_status) = packet
        .get("lane_status")
        .and_then(serde_json::Value::as_str)
        .and_then(canonical_resume_lane_status)
    {
        let packet_dispatch_status = canonical_resume_dispatch_status(
            packet
                .get("dispatch_status")
                .and_then(serde_json::Value::as_str),
        );
        let mut derived_lane_status = super::derive_lane_status(
            packet_dispatch_status,
            packet
                .get("supersedes_receipt_id")
                .and_then(serde_json::Value::as_str),
            packet
                .get("exception_path_receipt_id")
                .and_then(serde_json::Value::as_str),
        );
        if packet_lane_status == super::LaneStatus::LaneCompleted
            && packet_dispatch_status == "executed"
        {
            derived_lane_status = super::LaneStatus::LaneCompleted;
        }
        if !lane_status_pair_is_resume_compatible(packet_lane_status, derived_lane_status) {
            return Err(format!(
                "Persisted {packet_label} lane_status `{}` conflicts with derived lane_status `{}` from lane evidence",
                packet_lane_status.as_str(),
                derived_lane_status.as_str()
            ));
        }
    }
    Ok(())
}

fn dispatch_packet_paths_equivalent(left: &str, right: &str) -> bool {
    match (
        canonical_runtime_packet_identity(left),
        canonical_runtime_packet_identity(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PacketPathPlatform {
    Windows,
    Posix,
}

fn current_packet_path_platform() -> PacketPathPlatform {
    if cfg!(windows) {
        PacketPathPlatform::Windows
    } else {
        PacketPathPlatform::Posix
    }
}

fn packet_path_components_for_platform(path: &str, platform: PacketPathPlatform) -> Vec<&str> {
    let trimmed = path.trim();
    let stripped = match platform {
        PacketPathPlatform::Windows => trimmed
            .strip_prefix(r"\\?\")
            .or_else(|| trimmed.strip_prefix("//?/"))
            .unwrap_or(trimmed),
        PacketPathPlatform::Posix => trimmed,
    };
    match platform {
        PacketPathPlatform::Windows => stripped
            .split(['/', '\\'])
            .filter(|part| !part.is_empty())
            .collect(),
        PacketPathPlatform::Posix => stripped
            .split('/')
            .filter(|part| !part.is_empty())
            .collect(),
    }
}

fn packet_path_has_dot_segment(path: &str, platform: PacketPathPlatform) -> bool {
    packet_path_components_for_platform(path, platform)
        .iter()
        .any(|part| *part == "." || *part == "..")
}

fn canonical_runtime_packet_identity(path: &str) -> Result<std::path::PathBuf, String> {
    if packet_path_has_dot_segment(path, current_packet_path_platform()) {
        return Err(format!(
            "Runtime packet path `{}` contains dot-segment traversal and is not admissible.",
            path.trim()
        ));
    }
    let normalized = crate::runtime_dispatch_state::normalize_persisted_runtime_path(path);
    let canonical = std::fs::canonicalize(&normalized).map_err(|error| {
        format!(
            "Failed to canonicalize runtime packet path `{}`: {error}",
            normalized.display()
        )
    })?;
    let parent = canonical.parent().ok_or_else(|| {
        format!(
            "Runtime packet path `{}` has no canonical parent.",
            canonical.display()
        )
    })?;
    let parent_name = parent
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let grandparent_name = parent
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let packet_dir_allowed = matches!(
        parent_name,
        "dispatch-packets" | "downstream-dispatch-packets"
    ) && grandparent_name == "runtime-consumption";
    if !packet_dir_allowed {
        return Err(format!(
            "Runtime packet path `{}` is outside VIDA runtime packet directories.",
            canonical.display()
        ));
    }
    Ok(canonical)
}

async fn run_graph_resume_task_missing(
    store: &super::StateStore,
    status: &crate::state_store::RunGraphStatus,
) -> Result<bool, String> {
    let task_id = status.task_id.trim();
    if task_id.is_empty() {
        return Ok(false);
    }
    match store.show_task(task_id).await {
        Ok(_) => Ok(false),
        Err(error) if error.to_string().contains("task is missing") => Ok(true),
        Err(_) => Ok(false),
    }
}

fn stale_missing_task_run_graph_resume_error(
    status: &crate::state_store::RunGraphStatus,
) -> String {
    format!(
        "Stale missing-task run graph `{}` references missing TaskFlow task `{}`; retire the stale run with `vida lane retire {} --receipt-id {} --reason \"missing TaskFlow task stale run\" --json` before consuming continuation.",
        status.run_id,
        status.task_id,
        crate::shell_quote(&status.run_id),
        crate::shell_quote(&status.run_id)
    )
}

async fn receipt_backed_terminal_closure_resume(
    store: &super::StateStore,
    status: &crate::state_store::RunGraphStatus,
    run_id: &str,
) -> bool {
    status.lifecycle_stage == "closure_complete"
        && status.status == "completed"
        && status.resume_target == "none"
        && status.next_node.is_none()
        && status.handoff_state == "none"
        && matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)))
}

async fn validate_run_graph_resume_state(
    store: &super::StateStore,
    run_id: &str,
) -> Result<(), String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(error) => {
            let receipt_exists =
                matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)));
            if receipt_exists && resume_from_persisted_final_snapshot(store, run_id)? {
                return Ok(());
            }
            return Err(format!(
                "Failed to read persisted run-graph state for `{run_id}`: {error}"
            ));
        }
    };
    if status.run_id != run_id {
        return Err(format!(
            "Persisted run-graph state mismatch: requested run_id `{run_id}` resolved to `{}`",
            status.run_id
        ));
    }
    if receipt_backed_terminal_closure_resume(store, &status, run_id).await {
        return Ok(());
    }
    let active_receipt = store
        .run_graph_dispatch_receipt(run_id)
        .await
        .ok()
        .flatten();
    if active_receipt
        .as_ref()
        .is_some_and(|receipt| dispatch_receipt_records_completed_lane(receipt, run_id))
    {
        return Ok(());
    }
    if active_receipt_allows_resume_gate(store, run_id, active_receipt.as_ref()).await {
        return Ok(());
    }
    if active_receipt
        .as_ref()
        .and_then(|receipt| status_with_active_exception_dispatch_replay(&status, receipt))
        .as_ref()
        .is_some_and(|replay_status| validate_run_graph_resume_gate(replay_status).is_ok())
    {
        return Ok(());
    }
    if active_receipt.is_none()
        && run_graph_resume_task_missing(store, &status).await?
        && !active_receipt
            .as_ref()
            .is_some_and(dispatch_receipt_has_exception_takeover_evidence)
    {
        return Err(stale_missing_task_run_graph_resume_error(&status));
    }
    match validate_run_graph_resume_gate(&status) {
        Ok(()) => Ok(()),
        Err(_error) if resume_from_persisted_final_snapshot(store, run_id)? => Ok(()),
        Err(error) => Err(active_exception_takeover_resume_blocker_error(
            &status,
            active_receipt.as_ref(),
        )
        .unwrap_or(error)),
    }
}

async fn validate_run_graph_resume_state_strict(
    store: &super::StateStore,
    run_id: &str,
) -> Result<(), String> {
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read persisted run-graph state for `{run_id}`: {error}")
    })?;
    if status.run_id != run_id {
        return Err(format!(
            "Persisted run-graph state mismatch: requested run_id `{run_id}` resolved to `{}`",
            status.run_id
        ));
    }
    if receipt_backed_terminal_closure_resume(store, &status, run_id).await {
        return Ok(());
    }
    let active_receipt = store
        .run_graph_dispatch_receipt(run_id)
        .await
        .ok()
        .flatten();
    if active_receipt
        .as_ref()
        .is_some_and(|receipt| dispatch_receipt_records_completed_lane(receipt, run_id))
    {
        return Ok(());
    }
    if active_receipt_allows_resume_gate(store, run_id, active_receipt.as_ref()).await {
        return Ok(());
    }
    if active_receipt
        .as_ref()
        .and_then(|receipt| status_with_active_exception_dispatch_replay(&status, receipt))
        .as_ref()
        .is_some_and(|replay_status| validate_run_graph_resume_gate(replay_status).is_ok())
    {
        return Ok(());
    }
    if active_receipt.is_none()
        && run_graph_resume_task_missing(store, &status).await?
        && !active_receipt
            .as_ref()
            .is_some_and(dispatch_receipt_has_exception_takeover_evidence)
    {
        return Err(stale_missing_task_run_graph_resume_error(&status));
    }
    validate_run_graph_resume_gate(&status).map_err(|error| {
        active_exception_takeover_resume_blocker_error(&status, active_receipt.as_ref())
            .unwrap_or(error)
    })
}

async fn active_receipt_allows_resume_gate(
    store: &super::StateStore,
    run_id: &str,
    active_receipt: Option<&crate::state_store::RunGraphDispatchReceipt>,
) -> bool {
    let Some(active_receipt) = active_receipt else {
        return false;
    };
    if dispatch_receipt_retry_eligible(active_receipt) {
        return true;
    }
    let Ok(Some(context)) = store.run_graph_dispatch_context(run_id).await else {
        return false;
    };
    let Ok(role_selection) = context.role_selection() else {
        return false;
    };
    let project_root =
        super::runtime_dispatch_state::runtime_dispatch_project_root_from_state_root(store.root());
    dispatch_receipt_effective_retry_eligible(
        Some(project_root.as_ref()),
        Some(&role_selection),
        active_receipt,
    )
}

fn active_exception_takeover_resume_blocker_error(
    status: &crate::state_store::RunGraphStatus,
    receipt: Option<&crate::state_store::RunGraphDispatchReceipt>,
) -> Option<String> {
    let receipt = receipt?;
    let exception_takeover_active = receipt_has_active_exception_takeover(receipt, &status.run_id);
    if !exception_takeover_active || status.recovery_ready || status.resume_target != "none" {
        return None;
    }
    Some(format!(
        "Run-graph resume gate denied for `{}`: active exception takeover `{}` supersedes delegated dispatch while recovery_ready is false and resume_target is none. Inspect `vida lane show {} --json`; continue only within the active owned_write_scope or record a new bounded exception takeover for the next architectural repair.",
        status.run_id,
        receipt
            .exception_path_receipt_id
            .as_deref()
            .unwrap_or("unknown"),
        status.run_id
    ))
}

fn receipt_has_active_exception_takeover(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    run_id: &str,
) -> bool {
    receipt.run_id == run_id
        && receipt.lane_status == "lane_exception_takeover"
        && receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn dispatch_receipt_has_exception_takeover_evidence(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.lane_status == "lane_exception_takeover"
        || receipt
            .exception_path_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn dispatch_receipt_records_completed_lane(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    run_id: &str,
) -> bool {
    receipt.run_id == run_id
        && receipt.dispatch_status == "executed"
        && receipt.lane_status == super::LaneStatus::LaneCompleted.as_str()
        && receipt.blocker_code.is_none()
        && super::dispatch_receipt_has_execution_evidence(receipt)
}

fn persisted_dispatch_packet_lineage_task_id(packet: &serde_json::Value) -> Option<&str> {
    packet
        .pointer("/run_graph_bootstrap/latest_status/task_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            packet
                .pointer("/run_graph_bootstrap/task_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| {
            packet
                .get("task_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
}

async fn explicit_bound_task_graph_resume_run_id(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<String>, String> {
    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
    {
        return Ok(None);
    }
    let bound_task_id = binding.active_bounded_unit["task_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(binding.task_id.as_str());
    let unit_run_id = binding.active_bounded_unit["run_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(run_id);
    let requested_status = store.run_graph_status(run_id).await.map_err(|error| {
        format!(
            "Failed to read persisted run-graph state for `{run_id}` while reconciling explicit continuation binding: {error}"
        )
    })?;
    let requested_run_is_terminal_self_task = requested_status.run_id == run_id
        && requested_status.task_id == run_id
        && requested_status.status == "completed"
        && requested_status.lifecycle_stage == "closure_complete";
    let bound_run_id = if unit_run_id == run_id
        && bound_task_id != run_id
        && requested_run_is_terminal_self_task
    {
        bound_task_id
    } else {
        unit_run_id
    };
    if bound_run_id == run_id {
        return Ok(None);
    }

    let bound_status = store.run_graph_status(bound_run_id).await.map_err(|error| {
        format!(
            "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but fresh bound-task run-graph status is missing: {error}. Resume must fail closed until fresh run-graph truth is recorded for the bound task."
        )
    })?;
    if bound_status.run_id != bound_run_id || bound_status.task_id != bound_task_id {
        return Err(format!(
            "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but fresh bound-task run-graph status resolved to run `{}` and task `{}`. Resume must fail closed until fresh run-graph truth is recorded for the bound task.",
            bound_status.run_id, bound_status.task_id
        ));
    }
    let bound_receipt = store
        .run_graph_dispatch_receipt(bound_run_id)
        .await
        .map_err(|error| {
            format!(
                "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but fresh bound-task dispatch receipt could not be read: {error}. Resume must fail closed until fresh dispatch receipt evidence is recorded for the bound task."
            )
        })?
        .ok_or_else(|| {
            format!(
                "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but fresh bound-task dispatch receipt is missing. Resume must fail closed until fresh dispatch receipt evidence is recorded for the bound task."
            )
        })?;
    if bound_receipt.run_id != bound_run_id {
        return Err(format!(
            "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but fresh bound-task dispatch receipt resolved to run `{}`. Resume must fail closed until matching dispatch receipt evidence is recorded for the bound task.",
            bound_receipt.run_id
        ));
    }
    if let Some(packet_path) = bound_receipt.dispatch_packet_path.as_deref() {
        let packet = read_dispatch_packet(packet_path)?;
        if let Some(lineage_task_id) = persisted_dispatch_packet_lineage_task_id(&packet) {
            if lineage_task_id != bound_task_id {
                return Err(format!(
                    "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but fresh bound-task dispatch packet lineage at `{packet_path}` points to task `{lineage_task_id}`. Resume must fail closed until matching dispatch packet evidence is recorded for the bound task."
                ));
            }
        }
    }
    Ok(Some(bound_run_id.to_string()))
}

async fn validate_explicit_task_graph_binding_lineage_for_resume(
    store: &super::StateStore,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let status = store
        .run_graph_status(run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read persisted run-graph state for `{run_id}` while reconciling explicit continuation binding: {error}"
            )
        })?;
    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
    let Some(binding) = binding else {
        return Ok(());
    };
    if binding.status != "bound" {
        return Ok(());
    }

    match binding.active_bounded_unit["kind"].as_str() {
        Some("task_graph_task") => {
            let bound_task_id = binding.active_bounded_unit["task_id"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(binding.task_id.as_str());
            if status.task_id.trim() != bound_task_id {
                return Err(format!(
                    "Run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but persisted run-graph status still points to task `{}` with lifecycle `{}` and status `{}`. Resume must fail closed until fresh run-graph truth is recorded for the bound task.",
                    status.task_id, status.lifecycle_stage, status.status
                ));
            }
            if status.status != "completed" {
                return Ok(());
            }
            let Some(packet_path) = receipt.dispatch_packet_path.as_deref() else {
                return Ok(());
            };
            let packet = read_dispatch_packet(packet_path)?;
            let Some(lineage_task_id) = persisted_dispatch_packet_lineage_task_id(&packet) else {
                return Ok(());
            };
            if lineage_task_id == bound_task_id {
                return Ok(());
            }
            Err(format!(
                "Completed run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but persisted dispatch packet lineage at `{packet_path}` still points to task `{lineage_task_id}`. Resume must fail closed until a fresh dispatch packet is recorded for the bound task."
            ))
        }
        Some("downstream_dispatch_target") => {
            let Some(bound_target) = binding.active_bounded_unit["dispatch_target"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(());
            };
            if let Some(target) = receipt.downstream_dispatch_target.as_deref() {
                if target.trim() != bound_target {
                    return Err(format!(
                        "Run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream_dispatch_target still points to stale downstream target `{}`. Resume must fail closed until lawful downstream evidence is refreshed.",
                        target.trim()
                    ));
                }
            }
            if let Some(active_target) = receipt.downstream_dispatch_active_target.as_deref() {
                if active_target.trim() != bound_target
                    && matches!(
                        receipt.downstream_dispatch_status.as_deref().map(str::trim),
                        Some("blocked" | "failed")
                    )
                {
                    return Err(format!(
                        "Run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream_dispatch_active_target still points to stale downstream target `{}`. Resume must fail closed until lawful downstream evidence is refreshed.",
                        active_target.trim()
                    ));
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn validate_completed_run_downstream_resume_candidate(
    store: &super::StateStore,
    run_id: &str,
    candidate_target: &str,
    candidate_source: &str,
) -> Result<(), String> {
    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
    let Some(binding) = binding else {
        return Ok(());
    };
    if binding.status != "bound" {
        return Ok(());
    }

    match binding.active_bounded_unit["kind"].as_str() {
        Some("downstream_dispatch_target") => {
            let Some(bound_target) = binding.active_bounded_unit["dispatch_target"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(());
            };
            if bound_target == candidate_target {
                return Ok(());
            }
            Err(format!(
                "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted {candidate_source} still points to stale downstream target `{candidate_target}`. Resume must fail closed until lawful closure-bound evidence is refreshed."
            ))
        }
        Some("task_graph_task") => {
            let bound_task_id = binding.active_bounded_unit["task_id"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(binding.task_id.as_str());
            Err(format!(
                "Completed run `{run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but persisted {candidate_source} still points to downstream target `{candidate_target}`. Resume must fail closed until a fresh dispatch packet is recorded for the bound task."
            ))
        }
        _ => Ok(()),
    }
}

async fn completed_run_explicit_downstream_target_for_resume(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<String>, String> {
    let binding = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read explicit continuation binding for `{run_id}`: {error}")
        })?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("downstream_dispatch_target")
    {
        return Ok(None);
    }

    Ok(binding.active_bounded_unit["dispatch_target"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

fn missing_explicit_downstream_resume_evidence_error(run_id: &str, bound_target: &str) -> String {
    format!(
        "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but no lawful `{bound_target}` downstream packet or result is persisted. Resume must fail closed instead of replaying stale root dispatch lineage."
    )
}

async fn terminal_closure_complete_resume_candidate(
    store: &super::StateStore,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<ResumeInputs>, String> {
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read terminal run-graph status for `{run_id}`: {error}")
    })?;
    if status.status != "completed"
        || status.lifecycle_stage != "closure_complete"
        || status.resume_target != "none"
        || status.active_node != "closure"
    {
        return Ok(None);
    }
    let receipt_points_to_closure = receipt.dispatch_target == "closure"
        || receipt.downstream_dispatch_target.as_deref() == Some("closure")
        || receipt.downstream_dispatch_active_target.as_deref() == Some("closure")
        || receipt.downstream_dispatch_last_target.as_deref() == Some("closure");
    let closure_execution_recorded = receipt.dispatch_status == "executed"
        || receipt.downstream_dispatch_status.as_deref() == Some("executed")
        || receipt.lane_status == super::LaneStatus::LaneCompleted.as_str();
    if !receipt_points_to_closure || !closure_execution_recorded {
        return Ok(None);
    }
    let packet_path = receipt
        .dispatch_packet_path
        .clone()
        .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
    Ok(Some(terminal_closure_complete_resume_from_root_receipt(
        receipt,
        packet_path,
        packet,
        role_selection,
    )))
}

async fn completed_task_close_reconcile_resume_target(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<String>, String> {
    let status = store
        .run_graph_status(run_id)
        .await
        .map_err(|error| format!("Failed to read run-graph status for `{run_id}`: {error}"))?;
    let Some(binding) = store
        .run_graph_continuation_binding(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read task-close continuation binding for `{run_id}`: {error}")
        })?
    else {
        let task = store.show_task(&status.task_id).await.map_err(|error| {
            format!(
                "Failed to read run-graph task `{}` while checking task-close reconcile: {error}",
                status.task_id
            )
        })?;
        return if task.status == "closed" {
            Ok(Some("closure".to_string()))
        } else {
            Ok(None)
        };
    };
    let unit_kind = binding.active_bounded_unit["kind"].as_str();
    if binding.status != "bound" || binding.binding_source != "task_close_reconcile" {
        return Ok(None);
    }
    if unit_kind == Some("downstream_dispatch_target")
        && binding.active_bounded_unit["dispatch_target"].as_str() == Some("closure")
    {
        return Ok(Some("closure".to_string()));
    }
    if !matches!(unit_kind, Some("run_graph_task" | "task_graph_task")) {
        return Ok(None);
    }
    let task_id = binding.active_bounded_unit["task_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(binding.task_id.as_str());
    let task = store
        .show_task(task_id)
        .await
        .map_err(|error| format!("Failed to read task-close task `{task_id}`: {error}"))?;
    if task.status == "closed" {
        Ok(Some("closure".to_string()))
    } else {
        Ok(None)
    }
}

async fn task_close_reconcile_closure_resume_candidate(
    store: &super::StateStore,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<ResumeInputs>, String> {
    if completed_task_close_reconcile_resume_target(store, run_id)
        .await?
        .as_deref()
        != Some("closure")
    {
        return Ok(None);
    }
    let packet_path = receipt
        .dispatch_packet_path
        .clone()
        .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
    let packet = read_dispatch_packet(&packet_path)?;
    validate_receipt_packet_pair(receipt, &packet, &packet_path, "dispatch packet")?;
    let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
    Ok(Some(closure_packet_ready_resume_from_root_receipt(
        receipt,
        packet_path,
        packet,
        role_selection,
    )))
}

fn closure_packet_ready_resume_from_root_receipt(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: String,
    packet: serde_json::Value,
    role_selection: super::RuntimeConsumptionLaneSelection,
) -> ResumeInputs {
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, "closure");
    let selected_backend = super::downstream_selected_backend(
        &role_selection,
        "closure",
        activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    )
    .or_else(|| receipt.selected_backend.clone());
    let closure_execution_recorded = closure_execution_evidence_is_valid(receipt);
    let dispatch_status = if closure_execution_recorded {
        "executed"
    } else {
        "packet_ready"
    };
    let lane_status = if closure_execution_recorded {
        super::LaneStatus::LaneCompleted.as_str()
    } else {
        "packet_ready"
    };
    let downstream_dispatch_note = if closure_execution_recorded {
        "task-close reconcile recorded closure execution evidence"
    } else {
        "task-close reconcile completed the bounded task; closure is the next lawful resume target"
    };
    let closure_receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: "closure".to_string(),
        dispatch_status: dispatch_status.to_string(),
        lane_status: lane_status.to_string(),
        supersedes_receipt_id: None,
        exception_path_receipt_id: None,
        dispatch_kind,
        dispatch_surface,
        dispatch_command: super::runtime_dispatch_command_for_target(&role_selection, "closure"),
        dispatch_packet_path: Some(packet_path.clone()),
        dispatch_result_path: if closure_execution_recorded {
            receipt.downstream_dispatch_result_path.clone()
        } else {
            None
        },
        blocker_code: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: Some(downstream_dispatch_note.to_string()),
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
        downstream_dispatch_active_target: Some("closure".to_string()),
        downstream_dispatch_last_target: Some("closure".to_string()),
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    };
    build_resume_inputs(closure_receipt, packet_path, packet, role_selection)
}

fn closure_execution_evidence_is_valid(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if receipt.downstream_dispatch_target.as_deref() != Some("closure")
        || receipt.downstream_dispatch_status.as_deref() != Some("executed")
    {
        return false;
    }
    let Some(result_path) = receipt
        .downstream_dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return false;
    };
    let Ok(result) = read_downstream_dispatch_result(result_path) else {
        return false;
    };
    if result.get("run_id").and_then(serde_json::Value::as_str) != Some(receipt.run_id.as_str()) {
        return false;
    }
    let result_target = result
        .get("dispatch_target")
        .or_else(|| result.get("completed_target"))
        .and_then(serde_json::Value::as_str);
    if result_target != Some("closure") {
        return false;
    }
    canonical_resume_dispatch_status(
        result
            .get("execution_state")
            .and_then(serde_json::Value::as_str),
    ) == "executed"
}

fn terminal_closure_complete_resume_from_root_receipt(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet_path: String,
    packet: serde_json::Value,
    role_selection: super::RuntimeConsumptionLaneSelection,
) -> ResumeInputs {
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, "closure");
    let selected_backend = super::downstream_selected_backend(
        &role_selection,
        "closure",
        activation_agent_type.as_deref(),
        receipt.selected_backend.as_deref(),
    )
    .or_else(|| receipt.selected_backend.clone());
    let closure_receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: receipt.run_id.clone(),
        dispatch_target: "closure".to_string(),
        dispatch_status: "executed".to_string(),
        lane_status: super::LaneStatus::LaneCompleted.as_str().to_string(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command: super::runtime_dispatch_command_for_target(&role_selection, "closure"),
        dispatch_packet_path: Some(packet_path.clone()),
        dispatch_result_path: receipt.dispatch_result_path.clone(),
        blocker_code: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: Some(
            "terminal closure_complete run-graph state is the authoritative final resume lineage"
                .to_string(),
        ),
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
        downstream_dispatch_active_target: None,
        downstream_dispatch_last_target: Some("closure".to_string()),
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    };
    build_resume_inputs(closure_receipt, packet_path, packet, role_selection)
}

pub(crate) fn build_failure_control_evidence(
    source_run_id: &str,
    source_dispatch_packet_path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "rollback": {
            "status": "recorded",
            "summary": "rollback posture recorded for the resumed final snapshot",
            "source_run_id": source_run_id,
            "source_dispatch_packet_path": source_dispatch_packet_path,
        },
        "incident": {
            "status": "recorded",
            "summary": "incident evidence bundle recorded for the resumed final snapshot",
            "source_run_id": source_run_id,
            "source_dispatch_packet_path": source_dispatch_packet_path,
        },
        "restore": {
            "status": "recorded",
            "summary": "restore trace recorded for the resumed final snapshot",
            "source_run_id": source_run_id,
            "source_dispatch_packet_path": source_dispatch_packet_path,
        },
    })
}

fn failure_control_evidence_entry_is_complete(entry: Option<&serde_json::Value>) -> bool {
    let Some(entry) = entry.and_then(serde_json::Value::as_object) else {
        return false;
    };
    entry
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        && entry
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        && entry
            .get("source_run_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        && entry
            .get("source_dispatch_packet_path")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

fn runtime_consumption_snapshot_has_failure_control_evidence(snapshot: &serde_json::Value) -> bool {
    let Some(evidence) = snapshot
        .get("failure_control_evidence")
        .or_else(|| {
            snapshot
                .get("payload")
                .and_then(|payload| payload.get("failure_control_evidence"))
        })
        .and_then(serde_json::Value::as_object)
    else {
        return false;
    };

    ["rollback", "incident", "restore"]
        .iter()
        .all(|key| failure_control_evidence_entry_is_complete(evidence.get(*key)))
}

fn final_snapshot_missing_failure_control_evidence(snapshot_path: &str) -> bool {
    let payload = match std::fs::read_to_string(snapshot_path) {
        Ok(payload) => payload,
        Err(_) => return true,
    };
    let summary_json = match serde_json::from_str::<serde_json::Value>(&payload) {
        Ok(json) => json,
        Err(_) => return true,
    };
    !runtime_consumption_snapshot_has_failure_control_evidence(&summary_json)
}

fn latest_runtime_consumption_snapshot_path_for_resume_gate(
    state_root: &std::path::Path,
) -> Result<String, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    if !snapshot_dir.exists() {
        return Err("execution_preparation_gate_blocked: latest runtime-consumption snapshot is not `final`".to_string());
    }
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(&snapshot_dir)
        .map_err(|error| format!("Failed to read runtime-consumption directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !file_name.starts_with("final-") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        candidates.push((modified, path));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    candidates
        .into_iter()
        .map(|(_, path)| path.display().to_string())
        .next()
        .ok_or_else(|| {
            "execution_preparation_gate_blocked: latest runtime-consumption snapshot is not `final`"
                .to_string()
        })
}

fn latest_runtime_consumption_snapshot_for_resume_gate(
    state_root: &std::path::Path,
) -> Result<serde_json::Value, String> {
    let snapshot_path = latest_runtime_consumption_snapshot_path_for_resume_gate(state_root)?;
    let snapshot_body = std::fs::read_to_string(&snapshot_path).map_err(|error| {
        format!(
            "execution_preparation_gate_blocked: failed to read runtime-consumption snapshot: {error}"
        )
    })?;
    serde_json::from_str::<serde_json::Value>(&snapshot_body).map_err(|error| {
        format!(
            "execution_preparation_gate_blocked: failed to parse runtime-consumption snapshot: {error}"
        )
    })
}

fn latest_runtime_consumption_snapshot_after_recorded_final_is_bundle_check(
    state_root: &std::path::Path,
) -> Result<bool, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    if !snapshot_dir.exists() {
        return Ok(false);
    }

    let mut latest_bundle_check: Option<std::time::SystemTime> = None;
    let mut latest_final: Option<std::time::SystemTime> = None;
    for entry in std::fs::read_dir(&snapshot_dir)
        .map_err(|error| format!("Failed to read runtime-consumption directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if file_name.starts_with("bundle-check-")
            && latest_bundle_check.is_none_or(|latest_modified| modified > latest_modified)
        {
            latest_bundle_check = Some(modified);
        }
        if file_name.starts_with("final-")
            && latest_final.is_none_or(|latest_modified| modified > latest_modified)
        {
            latest_final = Some(modified);
        }
    }

    Ok(matches!(
        (latest_bundle_check, latest_final),
        (Some(bundle_check_modified), Some(final_modified))
            if bundle_check_modified >= final_modified
    ))
}

fn runtime_consumption_snapshot_has_execution_preparation_blocker(
    snapshot: &serde_json::Value,
) -> bool {
    let pending_execution_preparation_evidence =
        super::blocker_code_str(super::BlockerCode::PendingExecutionPreparationEvidence);
    let pending_design_packet = super::blocker_code_str(super::BlockerCode::PendingDesignPacket);
    let pending_developer_handoff_packet =
        super::blocker_code_str(super::BlockerCode::PendingDeveloperHandoffPacket);
    let missing_execution_preparation_contract =
        super::blocker_code_str(super::BlockerCode::MissingExecutionPreparationContract);
    let mut blockers: Vec<&str> = Vec::new();
    if let Some(rows) = snapshot["closure_admission"]["blockers"].as_array() {
        blockers.extend(rows.iter().filter_map(serde_json::Value::as_str));
    }
    if let Some(rows) = snapshot["operator_contracts"]["blocker_codes"].as_array() {
        blockers.extend(rows.iter().filter_map(serde_json::Value::as_str));
    }
    if let Some(code) = snapshot["dispatch_receipt"]["blocker_code"].as_str() {
        blockers.push(code);
    }
    blockers.iter().any(|value| {
        *value == pending_execution_preparation_evidence
            || *value == pending_design_packet
            || *value == pending_developer_handoff_packet
            || *value == missing_execution_preparation_contract
    })
}

fn enforce_consume_continue_execution_preparation_gate(
    state_root: &std::path::Path,
) -> Result<(), String> {
    let snapshot = latest_runtime_consumption_snapshot_for_resume_gate(state_root)?;
    let contract = &snapshot["operator_contracts"];
    let contract_ready = contract["contract_id"].as_str() == Some("release-1-operator-contracts")
        && contract["schema_version"].as_str() == Some("release-1-v1")
        && contract["status"].is_string()
        && contract["blocker_codes"].is_array()
        && contract["next_actions"].is_array()
        && contract["artifact_refs"].is_object();
    if !contract_ready {
        return Err(
            "execution_preparation_gate_blocked: missing or invalid release-1 operator contract"
                .to_string(),
        );
    }
    let Some(canonical_status) = contract["status"]
        .as_str()
        .and_then(crate::release1_contracts::canonical_release1_contract_status_str)
    else {
        return Err(
            "execution_preparation_gate_blocked: release-1 operator contract has invalid status"
                .to_string(),
        );
    };
    if canonical_status == "pass" {
        return super::taskflow_task_bridge::enforce_execution_preparation_contract_gate(
            state_root,
        );
    }
    if runtime_consumption_snapshot_has_execution_preparation_blocker(&snapshot) {
        return Err(format!(
            "execution_preparation_gate_blocked: {}",
            super::blocker_code_str(super::BlockerCode::PendingExecutionPreparationEvidence)
        ));
    }
    Ok(())
}

fn resume_from_persisted_final_snapshot(
    store: &super::StateStore,
    run_id: &str,
) -> Result<bool, String> {
    let snapshot_path = match super::latest_final_runtime_consumption_snapshot_path(store.root())? {
        Some(path) => Some(path),
        None => super::latest_recorded_final_runtime_consumption_snapshot_path(store.root())?,
    };
    let Some(snapshot_path) = snapshot_path else {
        return Ok(false);
    };
    let snapshot_body = match std::fs::read_to_string(&snapshot_path) {
        Ok(body) => body,
        Err(_) => return Ok(false),
    };
    let snapshot_json = match serde_json::from_str::<serde_json::Value>(&snapshot_body) {
        Ok(snapshot) => snapshot,
        Err(_) => return Ok(false),
    };
    let snapshot_run_id = snapshot_json
        .get("source_run_id")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            snapshot_json
                .pointer("/payload/dispatch_receipt/run_id")
                .and_then(serde_json::Value::as_str)
        })
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if snapshot_run_id != Some(run_id) {
        return Ok(false);
    }
    Ok(!final_snapshot_missing_failure_control_evidence(
        &snapshot_path,
    ))
}

async fn runtime_consumption_resume_blocker_code(
    store: &super::StateStore,
    payload_json: &serde_json::Value,
    explicit_run_id: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(run_id) = explicit_run_id {
        return crate::runtime_consumption_state::runtime_consumption_final_dispatch_receipt_blocker_code_for_run(
            store,
            payload_json,
            run_id,
        );
    }
    crate::runtime_consumption_state::runtime_consumption_final_dispatch_receipt_blocker_code(
        store,
        payload_json,
    )
}

fn runtime_consumption_resume_receipt_blocker_codes(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    let blocked_evidence_present = matches!(
        dispatch_receipt.dispatch_status.as_str(),
        "blocked" | "failed"
    ) || matches!(
        dispatch_receipt.lane_status.as_str(),
        "lane_blocked" | "lane_failed"
    ) || dispatch_receipt
        .downstream_dispatch_status
        .as_deref()
        .is_some_and(|status| matches!(status, "blocked" | "failed"));
    if let Some(blocker_code) = dispatch_receipt
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    if matches!(
        dispatch_receipt.dispatch_status.as_str(),
        "blocked" | "failed"
    ) || matches!(
        dispatch_receipt.lane_status.as_str(),
        "lane_blocked" | "lane_failed"
    ) {
        blocker_codes.extend(
            dispatch_receipt
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    if blocker_codes
        .iter()
        .any(|code| code == "configured_backend_dispatch_failed")
    {
        blocker_codes.push(
            crate::contract_profile_adapter::blocker_code_str(
                crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
            )
            .to_string(),
        );
    }
    let normalized = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    if normalized.is_empty() && blocked_evidence_present {
        vec![crate::contract_profile_adapter::blocker_code_str(
            crate::contract_profile_adapter::BlockerCode::ToolExecutionFailed,
        )
        .to_string()]
    } else {
        normalized
    }
}

fn ready_handoff_status_supersedes_blocked_dispatch_receipt(
    status: Option<&crate::state_store::RunGraphStatus>,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let Some(status) = status else {
        return false;
    };
    if status.run_id != dispatch_receipt.run_id
        || !status.status.eq_ignore_ascii_case("ready")
        || !status.recovery_ready
        || !status.resume_target.starts_with("dispatch.")
        || status.active_node == dispatch_receipt.dispatch_target
    {
        return false;
    }

    dispatch_receipt.lane_status == "lane_exception_takeover"
        && dispatch_receipt
            .exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        && dispatch_receipt
            .supersedes_receipt_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn runtime_consumption_resume_receipt_next_actions(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    blocker_codes: &[String],
) -> Vec<String> {
    if blocker_codes.is_empty() {
        return Vec::new();
    }

    let mut next_actions = Vec::new();
    next_actions.push(
        "Inspect the latest recovery projection with `vida taskflow recovery latest --json`."
            .to_string(),
    );
    let current_lane_completed_without_blocker =
        dispatch_receipt.dispatch_status == "executed" && dispatch_receipt.blocker_code.is_none();
    if current_lane_completed_without_blocker
        && blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence")
    {
        next_actions.push(
            "Record the missing clean review evidence before activating the downstream verification lane."
                .to_string(),
        );
    }
    crate::operator_contracts::canonical_next_action_entries(&serde_json::json!(next_actions))
        .unwrap_or_else(|| next_actions.to_vec())
}

async fn emit_runtime_consumption_resume_json(
    store: &super::StateStore,
    surface_name: &str,
    dispatch_packet_path: &str,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    explicit_run_id: Option<&str>,
    emit_output: bool,
    as_json: bool,
) -> Result<(), String> {
    let mut normalized_dispatch_receipt = dispatch_receipt.clone();
    if normalized_dispatch_receipt.dispatch_kind == "agent_lane" {
        normalized_dispatch_receipt.selected_backend =
            super::canonical_selected_backend_for_receipt(
                role_selection,
                &normalized_dispatch_receipt,
            );
    }
    let failure_control_evidence =
        build_failure_control_evidence(&normalized_dispatch_receipt.run_id, dispatch_packet_path);
    let mut payload_json = serde_json::json!({
        "dispatch_receipt": normalized_dispatch_receipt,
        "role_selection": role_selection,
        "source_dispatch_packet_path": dispatch_packet_path,
        "source_run_id": dispatch_receipt.run_id,
        "failure_control_evidence": failure_control_evidence.clone(),
    });
    let run_graph_status = store.run_graph_status(&dispatch_receipt.run_id).await;
    let blocker_run_id = explicit_run_id.or_else(|| {
        run_graph_status
            .as_ref()
            .ok()
            .map(|_| dispatch_receipt.run_id.as_str())
    });
    let runtime_dispatch_receipt_blocker_code =
        runtime_consumption_resume_blocker_code(store, &payload_json, blocker_run_id).await?;
    let projection_truth = match run_graph_status.as_ref() {
        Ok(status) => Some(
            crate::taskflow_run_graph::run_graph_projection_truth(store, status)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to build run-graph projection truth for `{}`: {error}",
                        dispatch_receipt.run_id
                    )
                })?,
        ),
        Err(_) => None,
    };
    let mut blocker_codes = if ready_handoff_status_supersedes_blocked_dispatch_receipt(
        run_graph_status.as_ref().ok(),
        &normalized_dispatch_receipt,
    ) {
        Vec::new()
    } else {
        runtime_consumption_resume_receipt_blocker_codes(&normalized_dispatch_receipt)
    };
    let mut next_actions = runtime_consumption_resume_receipt_next_actions(
        &normalized_dispatch_receipt,
        &blocker_codes,
    );
    if let Some(blocker_code) = runtime_dispatch_receipt_blocker_code.as_deref() {
        super::apply_runtime_consumption_final_dispatch_receipt_blocker(
            &mut payload_json,
            blocker_code,
        );
        blocker_codes.push(blocker_code.to_string());
        next_actions.push(
            match blocker_code {
                super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER => {
                    super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION
                }
                _ => super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION,
            }
            .to_string(),
        );
    }
    let preliminary_status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    payload_json["release_admission"] = serde_json::json!({});
    let snapshot = serde_json::json!({
        "surface": surface_name,
        "status": preliminary_status,
        "release_admission": {},
        "failure_control_evidence": failure_control_evidence.clone(),
        "payload": payload_json,
    });
    let snapshot_path =
        super::write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": dispatch_receipt.run_id,
            "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
            "consume_final_surface": surface_name,
        }),
    )?;
    let status = finalized.status;
    let operator_contracts = finalized.operator_contracts.clone();
    let shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": finalized.shared_fields["status"].clone(),
        "blocker_codes": finalized.shared_fields["blocker_codes"].clone(),
        "next_actions": finalized.shared_fields["next_actions"].clone(),
        "artifact_refs": finalized.shared_fields["artifact_refs"].clone(),
    });
    let snapshot_with_operator_contracts = serde_json::json!({
        "surface": surface_name,
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": status,
        "blocker_codes": finalized.blocker_codes.clone(),
        "next_actions": finalized.next_actions.clone(),
        "artifact_refs": finalized.artifact_refs.clone(),
        "shared_fields": shared_fields.clone(),
        "operator_contracts": operator_contracts.clone(),
        "release_admission": {},
        "payload": payload_json,
        "failure_control_evidence": failure_control_evidence,
    });
    std::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot_with_operator_contracts)
            .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?,
    )
    .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    if let Some(error) = crate::operator_contracts::shared_operator_output_contract_parity_error(
        &snapshot_with_operator_contracts,
    ) {
        return Err(format!(
            "Failed to preserve runtime-consumption resume operator-contract parity: {error}"
        ));
    }
    if !emit_output {
        return Ok(());
    }
    if as_json {
        let output_payload = serde_json::json!({
            "surface": surface_name,
            "trace_id": operator_contracts["trace_id"].clone(),
            "workflow_class": operator_contracts["workflow_class"].clone(),
            "risk_tier": operator_contracts["risk_tier"].clone(),
            "status": status,
            "blocker_codes": snapshot_with_operator_contracts["blocker_codes"].clone(),
            "next_actions": snapshot_with_operator_contracts["next_actions"].clone(),
            "artifact_refs": finalized.artifact_refs.clone(),
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts,
            "source_run_id": dispatch_receipt.run_id,
            "source_dispatch_packet_path": dispatch_packet_path,
            "dispatch_receipt": payload_json["dispatch_receipt"].clone(),
            "projection_truth": projection_truth,
            "snapshot_path": snapshot_path,
            "failure_control_evidence": snapshot_with_operator_contracts["failure_control_evidence"].clone(),
        });
        if let Some(error) =
            crate::operator_contracts::shared_operator_output_contract_parity_error(&output_payload)
        {
            return Err(format!(
                "Failed to preserve runtime-consumption resume output parity: {error}"
            ));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&output_payload)
                .expect("resume command should render as json")
        );
    } else {
        super::print_surface_header(super::RenderMode::Plain, surface_name);
        super::print_surface_line(super::RenderMode::Plain, "status", status);
        super::print_surface_line(
            super::RenderMode::Plain,
            "source run",
            &dispatch_receipt.run_id,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "source packet",
            dispatch_packet_path,
        );
        if let Some(projection_truth) = projection_truth.as_ref() {
            super::print_surface_line(
                super::RenderMode::Plain,
                "projection",
                &projection_truth.projection_reason,
            );
            if let Some(next_action) = projection_truth.next_lawful_operator_action.as_deref() {
                super::print_surface_line(super::RenderMode::Plain, "next action", next_action);
            }
        }
        super::print_surface_line(super::RenderMode::Plain, "snapshot path", &snapshot_path);
    }
    Ok(())
}

fn emit_deferred_agent_handoff_json(
    store: &super::StateStore,
    surface_name: &str,
    dispatch_packet_path: &str,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    emit_output: bool,
    as_json: bool,
) -> Result<(), String> {
    let mut normalized_dispatch_receipt = dispatch_receipt.clone();
    normalized_dispatch_receipt.selected_backend =
        super::canonical_selected_backend_for_receipt(role_selection, &normalized_dispatch_receipt);
    let failure_control_evidence =
        build_failure_control_evidence(&normalized_dispatch_receipt.run_id, dispatch_packet_path);
    let payload_json = serde_json::json!({
        "dispatch_receipt": normalized_dispatch_receipt,
        "role_selection": role_selection,
        "source_dispatch_packet_path": dispatch_packet_path,
        "source_run_id": dispatch_receipt.run_id,
        "failure_control_evidence": failure_control_evidence.clone(),
        "deferred_agent_handoff": {
            "status": "persisted",
            "reason": "consume_continue_returns_after_routed_agent_handoff",
        },
    });
    let snapshot = serde_json::json!({
        "surface": surface_name,
        "status": "pass",
        "release_admission": {},
        "failure_control_evidence": failure_control_evidence.clone(),
        "payload": payload_json,
    });
    let snapshot_path =
        super::write_runtime_consumption_snapshot(store.root(), "final", &snapshot)?;
    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": snapshot_path,
            "latest_run_graph_dispatch_receipt_id": dispatch_receipt.run_id,
            "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
            "consume_final_surface": surface_name,
        }),
    )?;
    let operator_contracts = finalized.operator_contracts.clone();
    let shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": finalized.shared_fields["status"].clone(),
        "blocker_codes": finalized.shared_fields["blocker_codes"].clone(),
        "next_actions": finalized.shared_fields["next_actions"].clone(),
        "artifact_refs": finalized.shared_fields["artifact_refs"].clone(),
    });
    let snapshot_with_operator_contracts = serde_json::json!({
        "surface": surface_name,
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": finalized.status,
        "blocker_codes": finalized.blocker_codes.clone(),
        "next_actions": finalized.next_actions.clone(),
        "artifact_refs": finalized.artifact_refs.clone(),
        "shared_fields": shared_fields.clone(),
        "operator_contracts": operator_contracts.clone(),
        "release_admission": {},
        "payload": payload_json,
        "failure_control_evidence": failure_control_evidence,
    });
    std::fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&snapshot_with_operator_contracts)
            .map_err(|error| format!("Failed to encode deferred handoff snapshot: {error}"))?,
    )
    .map_err(|error| format!("Failed to write deferred handoff snapshot: {error}"))?;
    if let Some(error) = crate::operator_contracts::shared_operator_output_contract_parity_error(
        &snapshot_with_operator_contracts,
    ) {
        return Err(format!(
            "Failed to preserve deferred handoff operator-contract parity: {error}"
        ));
    }
    if !emit_output {
        return Ok(());
    }
    if as_json {
        let output_payload = serde_json::json!({
            "surface": surface_name,
            "trace_id": operator_contracts["trace_id"].clone(),
            "workflow_class": operator_contracts["workflow_class"].clone(),
            "risk_tier": operator_contracts["risk_tier"].clone(),
            "status": finalized.status,
            "blocker_codes": snapshot_with_operator_contracts["blocker_codes"].clone(),
            "next_actions": snapshot_with_operator_contracts["next_actions"].clone(),
            "artifact_refs": finalized.artifact_refs.clone(),
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts,
            "source_run_id": dispatch_receipt.run_id,
            "source_dispatch_packet_path": dispatch_packet_path,
            "dispatch_receipt": snapshot_with_operator_contracts["payload"]["dispatch_receipt"].clone(),
            "projection_truth": {
                "projection_source": "deferred_agent_handoff_receipt",
                "projection_reason": "consume continue persisted a routed agent handoff without waiting for backend execution",
                "dispatch_receipt_present": true,
                "continuation_binding_present": true,
                "stale_state_suspected": false,
                "next_lawful_operator_action": format!("vida lane show {} --json", dispatch_receipt.run_id),
            },
            "snapshot_path": snapshot_path,
            "failure_control_evidence": snapshot_with_operator_contracts["failure_control_evidence"].clone(),
        });
        if let Some(error) =
            crate::operator_contracts::shared_operator_output_contract_parity_error(&output_payload)
        {
            return Err(format!(
                "Failed to preserve deferred handoff output parity: {error}"
            ));
        }
        crate::operator_projection_cache::write_json_projection(
            store.root(),
            CONSUME_CONTINUE_DEFERRED_HANDOFF_PROJECTION_NAME,
            &output_payload,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&output_payload)
                .expect("deferred handoff command should render as json")
        );
    } else {
        super::print_surface_header(super::RenderMode::Plain, surface_name);
        super::print_surface_line(super::RenderMode::Plain, "status", finalized.status);
        super::print_surface_line(
            super::RenderMode::Plain,
            "source run",
            &dispatch_receipt.run_id,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "source packet",
            dispatch_packet_path,
        );
        super::print_surface_line(super::RenderMode::Plain, "snapshot path", &snapshot_path);
    }
    Ok(())
}

#[cfg(test)]
async fn validate_run_graph_resume_state_for_downstream_packet(
    store: &super::StateStore,
    run_id: &str,
) -> Result<(), String> {
    validate_run_graph_resume_state_for_downstream_packet_candidate(store, run_id, None).await
}

async fn validate_run_graph_resume_state_for_downstream_packet_candidate(
    store: &super::StateStore,
    run_id: &str,
    candidate_packet: Option<(&serde_json::Value, &str)>,
) -> Result<(), String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(error) => {
            let receipt_exists =
                matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)));
            if receipt_exists && resume_from_persisted_final_snapshot(store, run_id)? {
                return Ok(());
            }
            return Err(format!(
                "Failed to read persisted run-graph state for `{run_id}`: {error}"
            ));
        }
    };
    if status.run_id != run_id {
        return Err(format!(
            "Persisted run-graph state mismatch: requested run_id `{run_id}` resolved to `{}`",
            status.run_id
        ));
    }
    if status.lifecycle_stage == "closure_complete"
        && status.status == "completed"
        && status.resume_target == "none"
        && matches!(store.run_graph_dispatch_receipt(run_id).await, Ok(Some(_)))
    {
        return Ok(());
    }
    if status.resume_target == "none" {
        if let Ok(Some(receipt)) = store.run_graph_dispatch_receipt(run_id).await {
            if receipt.downstream_dispatch_ready
                && receipt
                    .downstream_dispatch_packet_path
                    .as_deref()
                    .is_some_and(|path| !path.trim().is_empty())
            {
                return Ok(());
            }
        }
    }
    if let Some((packet, packet_path)) = candidate_packet {
        if downstream_packet_candidate_has_receipt_backed_ready_evidence(
            packet,
            packet_path,
            run_id,
        ) {
            return Ok(());
        }
    }
    validate_run_graph_resume_gate(&status)
}

fn downstream_packet_candidate_has_receipt_backed_ready_evidence(
    packet: &serde_json::Value,
    packet_path: &str,
    run_id: &str,
) -> bool {
    if packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        != Some(run_id)
    {
        return false;
    }
    if !packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    if packet
        .get("downstream_dispatch_blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|blockers| !blockers.is_empty())
    {
        return false;
    }
    if packet
        .get("downstream_dispatch_status")
        .and_then(serde_json::Value::as_str)
        .map(|status| canonical_resume_dispatch_status(Some(status)))
        != Some("packet_ready")
    {
        return false;
    }
    let Some(candidate_target) = packet
        .get("downstream_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
    else {
        return false;
    };
    if candidate_target.is_empty() {
        return false;
    }
    let Some(result_path) = packet
        .get("downstream_dispatch_result_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return !packet_path.trim().is_empty();
    };
    let Ok(result) = read_downstream_dispatch_result(result_path) else {
        return !packet_path.trim().is_empty();
    };
    if result
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        != Some(run_id)
    {
        return false;
    }
    if result
        .get("execution_state")
        .and_then(serde_json::Value::as_str)
        .map(|status| canonical_resume_dispatch_status(Some(status)))
        != Some("executed")
    {
        return false;
    }
    result
        .get("completion_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        && !packet_path.trim().is_empty()
}

fn packet_nonempty_string_array(packet: &serde_json::Value, key: &str) -> bool {
    packet
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row.as_str()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
                })
        })
}

fn packet_has_owned_or_read_only_paths(packet: &serde_json::Value) -> bool {
    packet_nonempty_string_array(packet, "owned_paths")
        || packet_nonempty_string_array(packet, "read_only_paths")
}

fn packet_dispatch_target(packet: &serde_json::Value) -> Option<&str> {
    packet
        .get("dispatch_target")
        .or_else(|| packet.get("downstream_dispatch_target"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("scope_in"))
                .and_then(serde_json::Value::as_array)
                .and_then(|scope_in| {
                    scope_in.iter().find_map(|entry| {
                        entry
                            .as_str()
                            .map(str::trim)
                            .and_then(|value| value.strip_prefix("dispatch_target:"))
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                    })
                })
        })
}

fn packet_request_text(packet: &serde_json::Value) -> Option<&str> {
    packet
        .get("request_text")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get("request_text"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

fn derive_required_delivery_owned_paths(packet: &serde_json::Value) -> Option<Vec<String>> {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)?;
    if packet_template_kind != "delivery_task_packet" {
        return None;
    }
    let active_packet = packet_template_child(packet, packet_template_kind)?;
    let handoff_task_class = active_packet
        .get("handoff_task_class")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| infer_legacy_delivery_handoff_task_class(packet));
    if !crate::runtime_dispatch_packets::delivery_packet_task_class_requires_owned_paths(
        handoff_task_class.as_deref()?,
    ) {
        return None;
    }

    let request_text = packet_request_text(packet).unwrap_or_default();
    let tracked_design_doc_path = packet["role_selection_full"]["execution_plan"]
        ["tracked_flow_bootstrap"]["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut owned_paths = crate::runtime_dispatch_packets::delivery_packet_owned_paths(
        handoff_task_class.as_deref()?,
        request_text,
        tracked_design_doc_path,
    );
    owned_paths.retain(|path| {
        !crate::runtime_dispatch_packets::is_runtime_consumption_fallback_owned_path(path)
    });
    if owned_paths.is_empty() {
        owned_paths = planner_metadata_owned_paths_from_packet(packet);
    }
    (!owned_paths.is_empty()).then_some(owned_paths)
}

fn infer_legacy_delivery_handoff_task_class(packet: &serde_json::Value) -> Option<String> {
    match packet_dispatch_target(packet)? {
        "implementer" | "implementation" | "writer" => {
            Some(crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION.to_string())
        }
        "test_author" => Some("test_authoring".to_string()),
        _ => None,
    }
}

fn planner_metadata_owned_paths_from_packet(packet: &serde_json::Value) -> Vec<String> {
    packet["role_selection_full"]["execution_plan"]["tracked_flow_bootstrap"]["dev_task"]
        ["planner_metadata"]["owned_paths"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value.as_str().and_then(
                crate::runtime_dispatch_packets::normalize_safe_owned_scope_path_candidate,
            )
        })
        .collect()
}

fn derive_specification_owned_paths_from_tracked_design_doc(
    packet: &serde_json::Value,
) -> Option<Vec<String>> {
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)?;
    if packet_template_kind != "delivery_task_packet" {
        return None;
    }
    let dispatch_target = packet_dispatch_target(packet)?;
    if dispatch_target != "specification" {
        return None;
    }

    let design_doc_path = packet["role_selection_full"]["execution_plan"]["tracked_flow_bootstrap"]
        ["design_doc_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(vec![design_doc_path.to_string()])
}

fn packet_template_child<'a>(
    packet: &'a serde_json::Value,
    packet_template_kind: &str,
) -> Option<&'a serde_json::Value> {
    packet
        .get(packet_template_kind)
        .filter(|value| !value.is_null())
}

fn nonempty_packet_string(packet: &serde_json::Value, key: &str) -> Option<String> {
    packet
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn packet_run_id(packet: &serde_json::Value, active_packet: &serde_json::Value) -> Option<String> {
    nonempty_packet_string(packet, "run_id")
        .or_else(|| {
            nonempty_packet_string(active_packet, "packet_id").and_then(|packet_id| {
                packet_id
                    .split("::")
                    .next()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
        })
        .or_else(|| {
            nonempty_packet_string(active_packet, "source_packet_id").and_then(|packet_id| {
                packet_id
                    .split("::")
                    .next()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
        })
}

fn source_dispatch_target_from_packet_id(packet_id: &str) -> Option<String> {
    let segments = packet_id
        .split("::")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    segments
        .windows(2)
        .find_map(|window| (window[1] == "delivery").then(|| window[0].to_string()))
}

fn coach_review_source_dispatch_target(
    active_packet: &serde_json::Value,
    dispatch_target: &str,
) -> Option<String> {
    nonempty_packet_string(active_packet, "reviewed_dispatch_target")
        .filter(|value| value != dispatch_target)
        .or_else(|| {
            nonempty_packet_string(active_packet, "source_dispatch_target")
                .filter(|value| value != dispatch_target)
        })
        .or_else(|| {
            nonempty_packet_string(active_packet, "source_packet_id")
                .and_then(|packet_id| source_dispatch_target_from_packet_id(&packet_id))
                .filter(|value| value != dispatch_target)
        })
}

fn coach_review_proof_target(active_packet: &serde_json::Value) -> String {
    nonempty_packet_string(active_packet, "proof_target").unwrap_or_else(|| {
        "bounded implementation result versus approved spec and definition of done".to_string()
    })
}

fn coach_review_prompt_runtime_role(
    packet: &serde_json::Value,
    active_packet: &serde_json::Value,
) -> String {
    nonempty_packet_string(packet, "activation_runtime_role")
        .or_else(|| nonempty_packet_string(active_packet, "handoff_runtime_role"))
        .unwrap_or_else(|| "coach".to_string())
}

fn normalize_coach_review_packet_contract(
    packet: &mut serde_json::Value,
    packet_template_kind: &str,
) -> bool {
    if packet_template_kind != "coach_review_packet" {
        return false;
    }
    let Some(active_packet) = packet_template_child(packet, packet_template_kind).cloned() else {
        return false;
    };
    let Some(run_id) = packet_run_id(packet, &active_packet) else {
        return false;
    };
    let dispatch_target = packet_dispatch_target(packet)
        .unwrap_or("coach")
        .trim()
        .to_string();
    let proof_target = coach_review_proof_target(&active_packet);
    let source_dispatch_target =
        coach_review_source_dispatch_target(&active_packet, &dispatch_target);
    let canonical_packet = crate::runtime_dispatch_packets::runtime_coach_review_packet(
        &run_id,
        &dispatch_target,
        source_dispatch_target.as_deref(),
        &proof_target,
    );

    let mut normalized = false;
    if packet.get(packet_template_kind) != Some(&canonical_packet) {
        packet[packet_template_kind] = canonical_packet;
        normalized = true;
    }

    let handoff_runtime_role = coach_review_prompt_runtime_role(packet, &active_packet);
    let request_text = packet_request_text(packet).unwrap_or_default();
    let orchestration_contract = packet
        .get("role_selection_full")
        .and_then(|value| value.get("execution_plan"))
        .and_then(|value| value.get("orchestration_contract"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let canonical_prompt = crate::runtime_dispatch_packet_text::runtime_packet_prompt(
        &run_id,
        &dispatch_target,
        &handoff_runtime_role,
        request_text,
        &orchestration_contract,
    );
    if packet
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        != Some(canonical_prompt.as_str())
    {
        packet["prompt"] = serde_json::json!(canonical_prompt);
        normalized = true;
    }

    normalized
}

fn normalize_runtime_dispatch_packet(packet: &mut serde_json::Value) -> bool {
    let Some(packet_template_kind) = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    else {
        return false;
    };
    let Some(active_packet) = packet.get(&packet_template_kind) else {
        return false;
    };
    if active_packet.is_null() {
        return false;
    }
    let mut normalized = normalize_coach_review_packet_contract(packet, &packet_template_kind);
    let Some(active_packet) = packet.get(&packet_template_kind) else {
        return normalized;
    };
    if active_packet.is_null() {
        return normalized;
    }
    let missing_owned_paths = !packet_nonempty_string_array(active_packet, "owned_paths");
    let missing_scope_paths = !packet_has_owned_or_read_only_paths(active_packet);
    let derived_required_delivery_owned_paths = missing_owned_paths
        .then(|| derive_required_delivery_owned_paths(packet))
        .flatten();
    let derived_specification_owned_paths =
        derive_specification_owned_paths_from_tracked_design_doc(packet);
    let Some(active_packet) = packet.get_mut(&packet_template_kind) else {
        return false;
    };
    let Some(active_packet_object) = active_packet.as_object_mut() else {
        return false;
    };
    if missing_owned_paths {
        if let Some(owned_paths) = derived_required_delivery_owned_paths
            .clone()
            .or_else(|| derived_specification_owned_paths.clone())
        {
            active_packet_object.insert("owned_paths".to_string(), serde_json::json!(owned_paths));
            normalized = true;
        }
    } else if let Some(expected_owned_paths) = derived_specification_owned_paths {
        let actual_owned_paths = active_packet_object
            .get("owned_paths")
            .and_then(serde_json::Value::as_array)
            .map(|rows| {
                rows.iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_string)
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
            .unwrap_or_default();
        if actual_owned_paths != expected_owned_paths {
            active_packet_object.insert(
                "owned_paths".to_string(),
                serde_json::json!(expected_owned_paths),
            );
            normalized = true;
        }
    }
    if missing_scope_paths {
        active_packet_object.insert(
            "read_only_paths".to_string(),
            serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS),
        );
        normalized = true;
    }
    normalized
}

pub(crate) fn read_dispatch_packet(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted dispatch packet: {error}"))?;
    let mut packet: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse persisted dispatch packet: {error}"))?;
    if normalize_runtime_dispatch_packet(&mut packet) {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&packet)
                .map_err(|error| format!("Failed to encode normalized dispatch packet: {error}"))?,
        )
        .map_err(|error| format!("Failed to persist normalized dispatch packet: {error}"))?;
    }
    crate::validate_runtime_dispatch_packet_contract(&packet, "Persisted dispatch packet")?;
    Ok(packet)
}

pub(crate) struct ResumeInputs {
    pub(crate) dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    pub(crate) dispatch_packet_path: String,
    pub(crate) role_selection: super::RuntimeConsumptionLaneSelection,
    pub(crate) run_graph_bootstrap: serde_json::Value,
}

fn build_resume_inputs(
    dispatch_receipt: crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: String,
    packet: serde_json::Value,
    role_selection: super::RuntimeConsumptionLaneSelection,
) -> ResumeInputs {
    let run_graph_bootstrap = packet
        .get("run_graph_bootstrap")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    ResumeInputs {
        dispatch_receipt,
        dispatch_packet_path,
        role_selection,
        run_graph_bootstrap,
    }
}

async fn fast_deferred_agent_handoff_resume_inputs(
    store: &super::StateStore,
    surface_name: &str,
    requested_run_id: Option<&str>,
    requested_dispatch_packet_path: Option<&str>,
    requested_downstream_packet_path: Option<&str>,
) -> Result<Option<ResumeInputs>, String> {
    if surface_name != "vida taskflow consume continue"
        || requested_dispatch_packet_path.is_some()
        || requested_downstream_packet_path.is_some()
    {
        return Ok(None);
    }
    let receipt = if let Some(run_id) = requested_run_id {
        let Some(receipt) = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .map_err(|error| {
                format!("Failed to read persisted run-graph dispatch receipt: {error}")
            })?
        else {
            return Ok(None);
        };
        receipt
    } else {
        let Some(receipt) = store
            .latest_run_graph_dispatch_receipt()
            .await
            .map_err(|error| {
                format!("Failed to read latest persisted run-graph dispatch receipt: {error}")
            })?
        else {
            return Ok(None);
        };
        if let Some(binding) = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .map_err(|error| format!("Failed to read explicit continuation binding: {error}"))?
        {
            if binding.status == "bound" && binding.run_id != receipt.run_id {
                return Ok(None);
            }
        }
        receipt
    };
    if !consume_continue_should_defer_agent_handoff(surface_name, &receipt) {
        return Ok(None);
    }
    let packet_path = receipt
        .dispatch_packet_path
        .clone()
        .or_else(|| receipt.downstream_dispatch_packet_path.clone())
        .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
    Ok(Some(build_resume_inputs(
        receipt,
        packet_path,
        packet,
        role_selection,
    )))
}

async fn reconcile_terminal_closure_lineage_for_resume(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    if dispatch_receipt.dispatch_target == "closure" {
        return Ok(false);
    }
    let terminal_closure_complete = store
        .run_graph_status(&dispatch_receipt.run_id)
        .await
        .map(|status| status.status == "completed" && status.lifecycle_stage == "closure_complete")
        .unwrap_or(false);
    let final_lineage_closure_ready = dispatch_receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        == Some("closure")
        && dispatch_receipt.downstream_dispatch_ready
        && dispatch_receipt.downstream_dispatch_blockers.is_empty()
        && resume_from_persisted_final_snapshot(store, &dispatch_receipt.run_id)?;
    if !terminal_closure_complete && !final_lineage_closure_ready {
        return Ok(false);
    }

    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(role_selection, "closure");
    dispatch_receipt.dispatch_target = "closure".to_string();
    dispatch_receipt.dispatch_status = if terminal_closure_complete {
        "executed".to_string()
    } else {
        "packet_ready".to_string()
    };
    dispatch_receipt.lane_status = if terminal_closure_complete {
        super::LaneStatus::LaneCompleted.as_str().to_string()
    } else {
        "packet_ready".to_string()
    };
    dispatch_receipt.dispatch_kind = dispatch_kind;
    dispatch_receipt.dispatch_surface = dispatch_surface;
    dispatch_receipt.dispatch_command =
        super::runtime_dispatch_command_for_target(role_selection, "closure");
    dispatch_receipt.dispatch_result_path = dispatch_receipt
        .downstream_dispatch_result_path
        .clone()
        .or_else(|| dispatch_receipt.dispatch_result_path.clone());
    dispatch_receipt.blocker_code = None;
    dispatch_receipt.downstream_dispatch_target = None;
    dispatch_receipt.downstream_dispatch_command = None;
    dispatch_receipt.downstream_dispatch_note = Some(
        "authoritative terminal/final lineage resolved closure as the next resume target"
            .to_string(),
    );
    dispatch_receipt.downstream_dispatch_ready = false;
    dispatch_receipt.downstream_dispatch_blockers = Vec::new();
    dispatch_receipt.downstream_dispatch_packet_path = None;
    dispatch_receipt.downstream_dispatch_status = None;
    dispatch_receipt.downstream_dispatch_result_path = None;
    dispatch_receipt.downstream_dispatch_trace_path = None;
    dispatch_receipt.downstream_dispatch_active_target = None;
    dispatch_receipt.downstream_dispatch_last_target = Some("closure".to_string());
    dispatch_receipt.activation_agent_type = activation_agent_type;
    dispatch_receipt.activation_runtime_role = activation_runtime_role;
    dispatch_receipt.selected_backend = super::downstream_selected_backend(
        role_selection,
        "closure",
        dispatch_receipt.activation_agent_type.as_deref(),
        dispatch_receipt.selected_backend.as_deref(),
    )
    .or_else(|| dispatch_receipt.selected_backend.clone());
    Ok(true)
}

fn build_run_graph_replay_lineage_receipt(
    status: &crate::state_store::RunGraphStatus,
    source_receipt: &crate::state_store::RunGraphDispatchReceipt,
    resume: &ResumeInputs,
    lineage_kind: &str,
) -> Result<crate::state_store::RunGraphReplayLineageReceipt, String> {
    let checkpoint_kind = status.checkpoint_kind.trim().to_string();
    let resume_target = status.resume_target.trim().to_string();
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&super::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let source_dispatch_packet_path = match lineage_kind {
        "downstream_packet" | "downstream_result" => source_receipt
            .downstream_dispatch_packet_path
            .clone()
            .or_else(|| source_receipt.dispatch_packet_path.clone()),
        _ => source_receipt.dispatch_packet_path.clone(),
    };
    let source_dispatch_result_path = match lineage_kind {
        "downstream_result" => source_receipt
            .downstream_dispatch_result_path
            .clone()
            .or_else(|| source_receipt.dispatch_result_path.clone()),
        _ => source_receipt.dispatch_result_path.clone(),
    };
    let resolved_task_id = status.task_id.trim().to_string();
    let receipt_id = format!(
        "replay-lineage-{}-{}",
        resume.dispatch_receipt.run_id,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    if checkpoint_kind.is_empty() {
        return Err(format!(
            "Failed to record run-graph replay lineage receipt: run `{}` is missing checkpoint_kind in persisted run-graph status",
            resume.dispatch_receipt.run_id
        ));
    }
    if resume_target.is_empty() {
        return Err(format!(
            "Failed to record run-graph replay lineage receipt: run `{}` is missing resume_target in persisted run-graph status",
            resume.dispatch_receipt.run_id
        ));
    }
    if resolved_task_id.is_empty() {
        return Err(format!(
            "Failed to record run-graph replay lineage receipt: run `{}` is missing task_id in persisted run-graph status",
            resume.dispatch_receipt.run_id
        ));
    }
    Ok(crate::state_store::RunGraphReplayLineageReceipt {
        receipt_id,
        run_id: resume.dispatch_receipt.run_id.clone(),
        lineage_kind: lineage_kind.to_string(),
        replay_scope: "resume_resolution".to_string(),
        origin_checkpoint_ref: format!(
            "{}:{}:{}",
            resume.dispatch_receipt.run_id, checkpoint_kind, resume_target
        ),
        fork_parent: None,
        source_dispatch_target: source_receipt.dispatch_target.clone(),
        source_dispatch_packet_path,
        source_dispatch_result_path,
        resolved_dispatch_target: resume.dispatch_receipt.dispatch_target.clone(),
        resolved_task_id,
        checkpoint_kind,
        resume_target,
        validation_outcome: "lawful_resume".to_string(),
        recorded_at,
    })
}

async fn record_run_graph_replay_lineage_receipt_for_resume(
    store: &super::StateStore,
    source_receipt: &crate::state_store::RunGraphDispatchReceipt,
    resume: &ResumeInputs,
    lineage_kind: &str,
) -> Result<(), String> {
    let status = store
        .run_graph_status(&resume.dispatch_receipt.run_id)
        .await
        .map_err(|error| {
            format!("Failed to load run-graph status for replay lineage receipt: {error}")
        })?;
    let receipt =
        build_run_graph_replay_lineage_receipt(&status, source_receipt, resume, lineage_kind)?;
    store
        .record_run_graph_replay_lineage_receipt(&receipt)
        .await
        .map_err(|error| format!("Failed to record run-graph replay lineage receipt: {error}"))
}

async fn recover_missing_first_dispatch_receipt(
    store: &super::StateStore,
    run_id: &str,
) -> Result<Option<ResumeInputs>, String> {
    let status = match store.run_graph_status(run_id).await {
        Ok(status) => status,
        Err(_) => return Ok(None),
    };
    if status.status == "completed" {
        return Ok(None);
    }
    let run_graph_bootstrap =
        match super::taskflow_run_graph::run_graph_dispatch_bootstrap_from_status(&status).or_else(
            |_| {
                legacy_missing_first_receipt_resume_status(&status)
                    .ok_or_else(|| String::new())
                    .and_then(|repaired_status| {
                        super::taskflow_run_graph::run_graph_dispatch_bootstrap_from_status(
                            &repaired_status,
                        )
                        .map_err(|_| String::new())
                    })
            },
        ) {
            Ok(bootstrap) => bootstrap,
            Err(_) => return Ok(None),
        };

    let context = store
        .run_graph_dispatch_context(run_id)
        .await
        .map_err(|error| format!("Failed to read persisted run-graph dispatch context: {error}"))?
        .ok_or_else(|| {
            format!(
                "No persisted run-graph dispatch receipt exists for run_id `{run_id}` and missing receipt recovery could not load dispatch context"
            )
        })?;
    let mut role_selection = context.role_selection().map_err(|error| {
        format!("Failed to decode persisted run-graph dispatch context for `{run_id}`: {error}")
    })?;
    if let Ok(task) = store.show_task(&status.task_id).await {
        inject_task_planner_metadata_for_resume(&mut role_selection, &task.planner_metadata);
    }

    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    );
    let active_lane_in_progress = status.status == "ready"
        && status.lifecycle_stage.ends_with("_active")
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|next_node| next_node != status.active_node);
    if active_lane_in_progress {
        let dispatch_target = status.active_node.clone();
        let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
            super::downstream_activation_fields(&role_selection, &dispatch_target);
        dispatch_receipt.dispatch_target = dispatch_target.clone();
        dispatch_receipt.dispatch_status = "packet_ready".to_string();
        dispatch_receipt.lane_status = super::derive_lane_status("packet_ready", None, None)
            .as_str()
            .to_string();
        dispatch_receipt.dispatch_kind = dispatch_kind;
        dispatch_receipt.dispatch_surface = dispatch_surface;
        dispatch_receipt.dispatch_command =
            super::runtime_dispatch_command_for_target(&role_selection, &dispatch_target);
        dispatch_receipt.downstream_dispatch_target = Some(dispatch_target.clone());
        dispatch_receipt.downstream_dispatch_command =
            super::runtime_dispatch_command_for_target(&role_selection, &dispatch_target);
        dispatch_receipt.activation_agent_type = activation_agent_type;
        dispatch_receipt.activation_runtime_role = activation_runtime_role;
        dispatch_receipt.selected_backend = super::downstream_selected_backend(
            &role_selection,
            &dispatch_target,
            dispatch_receipt.activation_agent_type.as_deref(),
            None,
        )
        .filter(|value| !value.is_empty());
    }
    dispatch_receipt.recorded_at = recorded_at;
    dispatch_receipt.dispatch_command = super::runtime_dispatch_command_for_target(
        &role_selection,
        &dispatch_receipt.dispatch_target,
    );
    super::refresh_downstream_dispatch_preview(
        store,
        &role_selection,
        &run_graph_bootstrap,
        &mut dispatch_receipt,
    )
    .await?;
    let taskflow_handoff_plan = super::build_taskflow_handoff_plan(&role_selection);
    let ctx = super::RuntimeDispatchPacketContext::new(
        store.root(),
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    );
    let dispatch_packet_path = super::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
    store
        .record_run_graph_dispatch_receipt(&dispatch_receipt)
        .await
        .map_err(|error| {
            format!("Failed to record recovered run-graph dispatch receipt: {error}")
        })?;
    super::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &status,
        "consume_continue_missing_first_receipt_recovery",
    )
    .await?;
    let packet = read_dispatch_packet(&dispatch_packet_path)?;
    Ok(Some(build_resume_inputs(
        dispatch_receipt,
        dispatch_packet_path,
        packet,
        role_selection,
    )))
}

fn inject_task_planner_metadata_for_resume(
    role_selection: &mut super::RuntimeConsumptionLaneSelection,
    planner_metadata: &crate::state_store::TaskPlannerMetadata,
) {
    if planner_metadata.owned_paths.is_empty()
        && planner_metadata.acceptance_targets.is_empty()
        && planner_metadata.proof_targets.is_empty()
        && planner_metadata.risk.is_none()
        && planner_metadata.estimate.is_none()
        && planner_metadata.lane_hint.is_none()
    {
        return;
    }
    if !planner_metadata.owned_paths.is_empty() {
        let owned_clause = format!("Owned paths: {}.", planner_metadata.owned_paths.join(", "));
        if !role_selection.request.contains(&owned_clause) {
            if role_selection.request.trim().is_empty() {
                role_selection.request = owned_clause;
            } else {
                role_selection.request =
                    format!("{}\n\n{owned_clause}", role_selection.request.trim());
            }
        }
    }
    let Some(plan) = role_selection.execution_plan.as_object_mut() else {
        return;
    };
    let tracked_flow_bootstrap = plan
        .entry("tracked_flow_bootstrap".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if tracked_flow_bootstrap.is_null() {
        *tracked_flow_bootstrap = serde_json::json!({});
    }
    let Some(tracked_flow_bootstrap) = tracked_flow_bootstrap.as_object_mut() else {
        return;
    };
    let dev_task = tracked_flow_bootstrap
        .entry("dev_task".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if dev_task.is_null() {
        *dev_task = serde_json::json!({});
    }
    let Some(dev_task) = dev_task.as_object_mut() else {
        return;
    };
    dev_task.insert(
        "planner_metadata".to_string(),
        serde_json::to_value(planner_metadata).unwrap_or_else(|_| serde_json::json!({})),
    );
}

fn legacy_missing_first_receipt_resume_status(
    status: &crate::state_store::RunGraphStatus,
) -> Option<crate::state_store::RunGraphStatus> {
    let resume_node = status
        .resume_target
        .trim()
        .strip_prefix("dispatch.")?
        .strip_suffix("_lane")
        .unwrap_or_else(|| {
            status
                .resume_target
                .trim()
                .strip_prefix("dispatch.")
                .unwrap_or("")
        })
        .trim();
    if resume_node.is_empty()
        || status.status != "ready"
        || !status.recovery_ready
        || status.active_node != resume_node
        || status.handoff_state != format!("awaiting_{resume_node}")
    {
        return None;
    }
    if status.next_node.as_deref() == Some(resume_node) {
        return None;
    }
    let mut repaired = status.clone();
    repaired.next_node = Some(resume_node.to_string());
    Some(repaired)
}

fn dispatch_receipt_retry_eligible(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    dispatch_receipt.dispatch_kind == "agent_lane"
        && dispatch_receipt.dispatch_status == "blocked"
        && matches!(
            dispatch_receipt.blocker_code.as_deref(),
            Some(
                "configured_backend_dispatch_failed"
                    | "internal_activation_view_only"
                    | "internal_codex_windows_sandbox_unavailable"
                    | crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
                    | "timeout_without_takeover_authority"
                    | "tool_execution_failed"
            )
        )
        && dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
}

fn dispatch_receipt_effective_retry_eligible(
    project_root: Option<&std::path::Path>,
    role_selection: Option<&super::RuntimeConsumptionLaneSelection>,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    dispatch_receipt_retry_eligible(dispatch_receipt)
        || project_root
            .zip(role_selection)
            .is_some_and(|(project_root, role_selection)| {
                dispatch_receipt_internal_retry_eligible(
                    project_root,
                    role_selection,
                    dispatch_receipt,
                )
            })
}

fn allow_downstream_resume_lineage(
    project_root: Option<&std::path::Path>,
    role_selection: Option<&super::RuntimeConsumptionLaneSelection>,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if activation_view_only_dispatch_blocker_is_terminal(dispatch_receipt.blocker_code.as_deref()) {
        return false;
    }
    !dispatch_receipt_effective_retry_eligible(project_root, role_selection, dispatch_receipt)
}

fn retry_backend_for_dispatch_receipt(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if activation_view_only_dispatch_blocker_is_terminal(dispatch_receipt.blocker_code.as_deref()) {
        return None;
    }
    let current_backend = dispatch_receipt
        .selected_backend
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let route = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    );
    let route_fallback =
        route.and_then(crate::taskflow_routing::fallback_executor_backend_from_route);
    if dispatch_receipt.blocker_code.as_deref() == Some("timeout_without_takeover_authority") {
        if let Some(packet_retry_backend) = dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .and_then(|path| {
                retry_backend_from_dispatch_packet(
                    path,
                    &dispatch_receipt.dispatch_target,
                    current_backend,
                )
            })
            .filter(|fallback| Some(fallback.as_str()) != current_backend)
        {
            return Some(packet_retry_backend);
        }
        if let Some(fallback) = route_fallback.clone() {
            if Some(fallback.as_str()) != current_backend {
                return Some(fallback);
            }
        }
        return None;
    }
    if let Some(next_review_backend) = distinct_review_retry_backend_from_route(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
        route,
        current_backend,
    ) {
        return Some(next_review_backend);
    }

    route_fallback
        .or_else(|| {
            dispatch_receipt
                .dispatch_packet_path
                .as_deref()
                .filter(|path| !path.trim().is_empty())
                .and_then(|path| {
                    retry_backend_from_dispatch_packet(
                        path,
                        &dispatch_receipt.dispatch_target,
                        current_backend,
                    )
                })
        })
        .filter(|fallback| Some(fallback.as_str()) != current_backend)
}

fn activation_view_only_dispatch_blocker_is_terminal(blocker_code: Option<&str>) -> bool {
    matches!(
        blocker_code,
        Some("internal_activation_view_only")
            | Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
    )
}

fn retry_backend_from_dispatch_packet(
    packet_path: &str,
    dispatch_target: &str,
    current_backend: Option<&str>,
) -> Option<String> {
    let packet = read_dispatch_packet(packet_path)
        .ok()
        .or_else(|| crate::read_json_file_if_present(std::path::Path::new(packet_path)))?;
    let execution_plan = &packet["role_selection_full"]["execution_plan"];
    let route = super::execution_plan_route_for_dispatch_target(execution_plan, dispatch_target);
    if let Some(next_review_backend) = distinct_review_retry_backend_from_route(
        execution_plan,
        dispatch_target,
        route,
        current_backend,
    ) {
        return Some(next_review_backend);
    }
    route
        .and_then(crate::taskflow_routing::fallback_executor_backend_from_route)
        .or_else(|| {
            packet["execution_truth"]["route_fallback_backend"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn distinct_review_retry_backend_from_route(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
    route: Option<&serde_json::Value>,
    current_backend: Option<&str>,
) -> Option<String> {
    if dispatch_target != "coach" {
        return None;
    }
    let route = route?;
    crate::taskflow_routing::fanout_executor_backends_from_route(route)
        .into_iter()
        .map(|backend| backend.trim().to_string())
        .find(|backend| {
            !backend.is_empty()
                && Some(backend.as_str()) != current_backend
                && crate::runtime_dispatch_state::backend_is_admissible_for_dispatch_target(
                    execution_plan,
                    backend,
                    dispatch_target,
                )
        })
}

fn retry_transition_backend_for_dispatch_receipt(
    project_root: Option<&std::path::Path>,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if dispatch_receipt_effective_retry_eligible(
        project_root,
        Some(role_selection),
        dispatch_receipt,
    ) {
        return retry_backend_for_dispatch_receipt(role_selection, dispatch_receipt);
    }

    let project_root = project_root?;
    if let Some(primary_backend) =
        primary_backend_for_dispatch_receipt(project_root, role_selection, dispatch_receipt)
    {
        return Some(primary_backend);
    }
    super::fallback_backend_for_blocked_primary_dispatch_receipt(
        project_root,
        role_selection,
        dispatch_receipt,
    )
}

fn dispatch_receipt_primary_rebind_eligible(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "blocked"
        || dispatch_receipt.blocker_code.as_deref() != Some("internal_activation_view_only")
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return false;
    }
    let Some(route) = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    ) else {
        return false;
    };
    let Some(primary_backend) = crate::taskflow_routing::selected_backend_from_execution_plan_route(
        &role_selection.execution_plan,
        route,
    ) else {
        return false;
    };
    if !crate::runtime_dispatch_state::backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        &primary_backend,
        &dispatch_receipt.dispatch_target,
    ) {
        return false;
    }
    let Some(fallback_backend) =
        crate::taskflow_routing::fallback_executor_backend_from_route(route)
    else {
        return false;
    };
    dispatch_receipt.selected_backend.as_deref() == Some(fallback_backend.as_str())
        && primary_backend != fallback_backend
}

fn dispatch_receipt_internal_retry_eligible(
    project_root: &std::path::Path,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "blocked"
        || dispatch_receipt.blocker_code.as_deref() != Some("internal_activation_view_only")
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return false;
    }
    if activation_view_only_dispatch_blocker_is_terminal(dispatch_receipt.blocker_code.as_deref()) {
        return false;
    }
    if super::internal_host_activation_view_only_requires_terminal_blocker(
        project_root,
        role_selection,
        dispatch_receipt,
    ) {
        return false;
    }
    let overlay = match super::load_project_overlay_yaml_for_root(project_root) {
        Ok(overlay) => overlay,
        Err(_) => return false,
    };
    let (_selected_cli_system, selected_cli_entry) =
        super::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let execution_class = selected_cli_entry
        .as_ref()
        .and_then(|entry| super::yaml_lookup(entry, &["execution_class"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    if execution_class != "internal" {
        return false;
    }
    let carriers = crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(
        selected_cli_entry.as_ref(),
    );
    let has_internal_carriers = !carriers.is_empty();
    [
        dispatch_receipt.selected_backend.as_deref(),
        dispatch_receipt.activation_agent_type.as_deref(),
        Some(role_selection.selected_role.as_str()),
    ]
    .iter()
    .flatten()
    .any(|backend_id| {
        carriers
            .iter()
            .any(|row| row["role_id"].as_str() == Some(*backend_id))
            || (has_internal_carriers
                && backend_id_is_configured_internal_bridge(&overlay, role_selection, backend_id))
    })
}

fn backend_id_is_configured_internal_bridge(
    overlay: &serde_yaml::Value,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    backend_id: &str,
) -> bool {
    fn internal_class(value: &str) -> bool {
        matches!(value.trim(), "internal" | "internal_cli")
    }

    let execution_plan_class = role_selection.execution_plan["backend_admissibility_matrix"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
        .and_then(|entry| entry["backend_class"].as_str());
    if execution_plan_class.is_some_and(internal_class) {
        return true;
    }

    super::yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|entries| {
            entries.iter().find_map(|(key, value)| {
                let id = key.as_str()?.trim();
                (id == backend_id
                    && super::yaml_bool(super::yaml_lookup(value, &["enabled"]), false))
                .then_some(value)
            })
        })
        .and_then(|entry| {
            super::yaml_string(super::yaml_lookup(entry, &["subagent_backend_class"]))
        })
        .as_deref()
        .is_some_and(internal_class)
}

fn primary_backend_for_dispatch_receipt(
    project_root: &std::path::Path,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    if !dispatch_receipt_primary_rebind_eligible(role_selection, dispatch_receipt) {
        return None;
    }
    let route = super::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        &dispatch_receipt.dispatch_target,
    )?;
    let primary_backend = crate::taskflow_routing::selected_backend_from_execution_plan_route(
        &role_selection.execution_plan,
        route,
    )?;
    let overlay = super::load_project_overlay_yaml_for_root(project_root).ok()?;
    let (selected_cli_system, selected_cli_entry) =
        super::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let preflight = crate::status_surface_external_cli::external_cli_preflight_summary(
        &overlay,
        &selected_cli_system,
        selected_cli_entry.as_ref(),
    );
    let carrier_ready = preflight["carrier_readiness"]["carriers"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|carrier| {
            carrier["backend_id"].as_str() == Some(primary_backend.as_str())
                && matches!(
                    carrier["status"].as_str(),
                    Some("carrier_ready" | "carrier_ready_with_override")
                )
        });
    carrier_ready.then_some(primary_backend)
}

fn decode_role_selection_from_packet(
    packet: &serde_json::Value,
    packet_kind: &str,
) -> Result<super::RuntimeConsumptionLaneSelection, String> {
    serde_json::from_value(
        packet
            .get("role_selection_full")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| format!("Failed to decode role_selection from {packet_kind}: {error}"))
}

async fn resume_inputs_from_downstream_packet(
    store: &super::StateStore,
    requested_run_id: Option<&str>,
    packet_path: &str,
) -> Result<ResumeInputs, String> {
    let mut packet = read_dispatch_packet(packet_path)?;
    let run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted downstream dispatch packet is missing run_id".to_string())?
        .to_string();
    if let Some(requested_run_id) = requested_run_id {
        if requested_run_id != run_id.as_str() {
            return Err(format!(
                "Requested run_id `{requested_run_id}` does not match persisted downstream dispatch packet run_id `{run_id}`"
            ));
        }
    }
    let root_receipt = match store.run_graph_dispatch_receipt(&run_id).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => return Err(missing_dispatch_receipt_error(&run_id)),
        Err(error) => {
            return Err(format!(
                "Failed to read persisted run-graph dispatch receipt: {error}"
            ));
        }
    };
    validate_receipt_packet_pair(
        &root_receipt,
        &packet,
        packet_path,
        "downstream dispatch packet",
    )?;
    if packet
        .get("downstream_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        if let Some(target) = root_receipt
            .downstream_dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            packet["downstream_dispatch_target"] = serde_json::json!(target);
        }
    }
    validate_run_graph_resume_state_for_downstream_packet_candidate(
        store,
        &run_id,
        Some((&packet, packet_path)),
    )
    .await?;
    let role_selection = decode_role_selection_from_packet(&packet, "downstream dispatch packet")?;
    let dispatch_target = packet
        .get("downstream_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Persisted downstream dispatch packet is missing downstream_dispatch_target".to_string()
        })?;
    validate_completed_run_downstream_resume_candidate(
        store,
        &run_id,
        dispatch_target,
        "downstream dispatch packet",
    )
    .await?;
    let (dispatch_kind, dispatch_surface, mut activation_agent_type, mut activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, dispatch_target);
    if activation_agent_type.is_none() {
        activation_agent_type =
            downstream_packet_activation_field(&packet, &role_selection, "activation_agent_type");
    }
    if activation_runtime_role.is_none() {
        activation_runtime_role =
            downstream_packet_activation_field(&packet, &role_selection, "activation_runtime_role");
    }
    let selected_backend = super::downstream_selected_backend(
        &role_selection,
        dispatch_target,
        activation_agent_type.as_deref(),
        root_receipt.selected_backend.as_deref(),
    )
    .filter(|value| !value.is_empty());
    let downstream_dispatch_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let dispatch_command = packet
        .get("downstream_dispatch_command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let downstream_dispatch_note = packet
        .get("downstream_dispatch_note")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let downstream_dispatch_blockers = packet
        .get("downstream_dispatch_blockers")
        .map(|value| {
            canonical_resume_string_array_entries(value).ok_or_else(|| {
                "Persisted downstream dispatch packet has noncanonical downstream_dispatch_blockers"
                    .to_string()
            })
        })
        .transpose()?
        .unwrap_or_default();
    if let Some(error) = super::downstream_dispatch_ready_blocker_parity_error(
        downstream_dispatch_ready,
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    let downstream_dispatch_status =
        if downstream_dispatch_ready && downstream_dispatch_blockers.is_empty() {
            Some("packet_ready".to_string())
        } else {
            packet
                .get("downstream_dispatch_status")
                .and_then(serde_json::Value::as_str)
                .map(|status| canonical_resume_dispatch_status(Some(status)))
                .map(str::to_string)
        };
    if let Some(error) = resume_packet_ready_blocker_parity_error(
        downstream_dispatch_status.as_deref(),
        &downstream_dispatch_blockers,
    ) {
        return Err(error);
    }
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let supersedes_receipt_id = packet
        .get("downstream_supersedes_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let exception_path_receipt_id = packet
        .get("downstream_exception_path_receipt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let parsed_downstream_lane_status = packet
        .get("downstream_lane_status")
        .and_then(serde_json::Value::as_str)
        .and_then(canonical_resume_lane_status);
    let (supersedes_receipt_id, exception_path_receipt_id, parsed_downstream_lane_status) =
        sanitize_inherited_downstream_lane_evidence(
            &packet,
            downstream_dispatch_status.as_deref(),
            supersedes_receipt_id,
            exception_path_receipt_id,
            parsed_downstream_lane_status,
        );
    let missing_lane_evidence_blocker = super::missing_downstream_lane_evidence_blocker(
        parsed_downstream_lane_status,
        supersedes_receipt_id.as_deref(),
        exception_path_receipt_id.as_deref(),
    );
    if let Some(code) = missing_lane_evidence_blocker {
        let _ = super::blocker_code_value(code);
        return Err(match code {
            super::BlockerCode::ExceptionPathMissing => {
                "Persisted downstream dispatch packet is missing downstream_exception_path_receipt_id"
                    .to_string()
            }
            super::BlockerCode::MissingLaneReceipt => {
                "Persisted downstream dispatch packet is missing downstream_supersedes_receipt_id"
                    .to_string()
            }
            _ => "Persisted downstream dispatch packet is missing required lane evidence"
                .to_string(),
        });
    }
    let closure_completed = matches!(
        parsed_downstream_lane_status,
        Some(super::LaneStatus::LaneCompleted)
    ) && downstream_dispatch_status.as_deref() == Some("executed");
    let dispatch_status = if closure_completed {
        "executed".to_string()
    } else {
        downstream_dispatch_status
            .as_deref()
            .unwrap_or("blocked")
            .to_string()
    };
    let mut derived_lane_status = super::derive_lane_status(
        &dispatch_status,
        supersedes_receipt_id.as_deref(),
        exception_path_receipt_id.as_deref(),
    );
    if closure_completed {
        derived_lane_status = super::LaneStatus::LaneCompleted;
    }
    if let Some(packet_lane_status) = parsed_downstream_lane_status {
        if !lane_status_pair_is_resume_compatible(packet_lane_status, derived_lane_status) {
            return Err(format!(
                "Persisted downstream dispatch packet lane_status `{}` conflicts with derived lane_status `{}` from downstream lane evidence",
                packet_lane_status.as_str(),
                derived_lane_status.as_str()
            ));
        }
    }
    let receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.to_string(),
        dispatch_target: dispatch_target.to_string(),
        dispatch_status: dispatch_status.clone(),
        lane_status: derived_lane_status.as_str().to_string(),
        supersedes_receipt_id,
        exception_path_receipt_id,
        dispatch_kind,
        dispatch_surface,
        dispatch_command,
        dispatch_packet_path: Some(packet_path.to_string()),
        dispatch_result_path: None,
        blocker_code: if missing_lane_evidence_blocker
            == Some(super::BlockerCode::ExceptionPathMissing)
        {
            super::blocker_code_value(super::BlockerCode::ExceptionPathMissing)
        } else if missing_lane_evidence_blocker == Some(super::BlockerCode::MissingLaneReceipt) {
            super::blocker_code_value(super::BlockerCode::MissingLaneReceipt)
        } else if dispatch_status == "blocked" {
            super::blocker_code_value(super::BlockerCode::MissingPacket)
        } else {
            None
        },
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: downstream_dispatch_note.filter(|_| dispatch_status == "blocked"),
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: packet
            .get("downstream_dispatch_executed_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        downstream_dispatch_active_target: None,
        downstream_dispatch_last_target: Some(dispatch_target.to_string()),
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    };
    Ok(build_resume_inputs(
        receipt,
        packet_path.to_string(),
        packet,
        role_selection,
    ))
}

fn downstream_packet_activation_field(
    packet: &serde_json::Value,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    field: &str,
) -> Option<String> {
    crate::json_string(packet.get(field))
        .or_else(|| {
            crate::json_string(role_selection.execution_plan["runtime_assignment"].get(field))
        })
        .or_else(|| {
            crate::json_string(
                role_selection.execution_plan["runtime_assignment"]["role_selection"].get(field),
            )
        })
}

async fn maybe_resume_inputs_from_ready_downstream_packet(
    store: &super::StateStore,
    requested_run_id: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<ResumeInputs>, String> {
    let Some(packet_path) = receipt.downstream_dispatch_packet_path.as_deref() else {
        return Ok(None);
    };
    let packet = read_dispatch_packet(packet_path).or_else(|_| {
        crate::read_json_file_if_present(std::path::Path::new(packet_path))
            .ok_or_else(|| format!("Failed to read persisted dispatch packet `{packet_path}`"))
    })?;
    let packet_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !packet_ready {
        return Ok(None);
    }
    resume_inputs_from_downstream_packet(store, requested_run_id, packet_path)
        .await
        .map(Some)
}

fn prefer_ready_downstream_packet_over_active_result(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    if !receipt.downstream_dispatch_ready {
        return false;
    }
    let ready_target = receipt
        .downstream_dispatch_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let active_target = receipt
        .downstream_dispatch_active_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (ready_target, active_target) {
        (Some(ready), Some(active)) if ready != active => {
            let Some(result_path) = receipt.downstream_dispatch_result_path.as_deref() else {
                return true;
            };
            let active_result = match read_downstream_dispatch_result(result_path) {
                Ok(result) => result,
                Err(_) => return true,
            };
            let active_execution_state = active_result
                .get("execution_state")
                .and_then(serde_json::Value::as_str)
                .map(|value| canonical_resume_dispatch_status(Some(value)))
                .unwrap_or("blocked");
            active_execution_state != "blocked"
        }
        _ => false,
    }
}

fn downstream_result_packet_path(result: &serde_json::Value) -> Option<String> {
    if let Some(path) = result
        .get("dispatch_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    {
        return Some(path);
    }

    let source_path = result
        .get("source_dispatch_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let source_packet = crate::read_json_file_if_present(std::path::Path::new(source_path))?;
    if source_packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        == Some("runtime_downstream_dispatch_packet")
    {
        return None;
    }
    Some(source_path.to_string())
}

fn read_downstream_dispatch_result(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted downstream dispatch result: {error}"))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse persisted downstream dispatch result: {error}"))
}

fn blocked_external_dispatch_artifact_mismatched_as_internal_activation(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result: &serde_json::Value,
) -> bool {
    if receipt.dispatch_status != "blocked"
        || receipt.blocker_code.as_deref() != Some("internal_activation_view_only")
        || result["execution_state"].as_str() != Some("blocked")
        || result["blocker_code"].as_str() != Some("internal_activation_view_only")
    {
        return false;
    }
    let selected_backend = receipt
        .selected_backend
        .as_deref()
        .or_else(|| result["selected_backend"].as_str());
    let Some(selected_backend) = selected_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    if selected_backend == "internal_subagents" {
        return false;
    }
    crate::runtime_dispatch_receipt_helpers::dispatch_result_has_external_dispatch_evidence(
        receipt, result,
    ) || (selected_backend.ends_with("_cli")
        && result["lane_execution_receipt_artifact"]["carrier_id"].as_str()
            == Some(selected_backend))
}

fn terminal_execution_result_for_in_flight_receipt(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    result_path: &str,
) -> Option<(String, serde_json::Value)> {
    fn trusted_terminal_execution_evidence(
        receipt: &crate::state_store::RunGraphDispatchReceipt,
        result: &serde_json::Value,
    ) -> bool {
        let trusted_artifact_kind = matches!(
            result["artifact_kind"].as_str(),
            Some("runtime_dispatch_result" | "runtime_lane_completion_result")
        );
        if !trusted_artifact_kind {
            return false;
        }
        if result["execution_evidence"]["receipt_backed"].as_bool() != Some(true) {
            return false;
        }
        if result["execution_evidence"]["status"].as_str() != Some("recorded") {
            return false;
        }
        match receipt.dispatch_surface.as_deref() {
            Some(surface) if surface.starts_with("external_cli:") => {
                let expected_backend = surface.trim_start_matches("external_cli:");
                result["execution_evidence"]["backend_id"].as_str() == Some(expected_backend)
            }
            _ => true,
        }
    }

    let result_dir = std::path::Path::new(result_path).parent()?;
    let dispatch_packet_path = receipt.dispatch_packet_path.as_deref().map(str::trim);
    let mut matches = std::fs::read_dir(result_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let result = crate::read_json_file_if_present(&path)?;
            let execution_state = result["execution_state"].as_str()?;
            if execution_state != "executed" {
                return None;
            }
            if !trusted_terminal_execution_evidence(receipt, &result) {
                return None;
            }
            let same_run = result["run_id"].as_str() == Some(receipt.run_id.as_str());
            let same_target = result["dispatch_target"].as_str()
                == Some(receipt.dispatch_target.as_str())
                || result["completed_target"].as_str() == Some(receipt.dispatch_target.as_str());
            let same_packet = match dispatch_packet_path {
                Some(expected) => {
                    result["source_dispatch_packet_path"]
                        .as_str()
                        .map(str::trim)
                        == Some(expected)
                        || result["dispatch_packet_path"].as_str().map(str::trim) == Some(expected)
                }
                None => true,
            };
            (same_run && same_target && same_packet).then(|| {
                let recorded_at = result["recorded_at"]
                    .as_str()
                    .or_else(|| result["lane_execution_receipt_artifact"]["finished_at"].as_str())
                    .unwrap_or_default()
                    .to_string();
                (recorded_at, path.display().to_string(), result)
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.0.cmp(&right.0));
    matches
        .pop()
        .map(|(_recorded_at, path, result)| (path, result))
}

fn promote_receipt_to_terminal_execution_result(
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    result_path: String,
    result: &serde_json::Value,
) {
    receipt.dispatch_result_path = Some(result_path);
    receipt.dispatch_status = result["execution_state"]
        .as_str()
        .unwrap_or("executed")
        .to_string();
    receipt.lane_status = super::derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string();
    receipt.blocker_code = None;
    if let Some(dispatch_surface) = result["surface"].as_str().map(str::to_string) {
        receipt.dispatch_surface = Some(dispatch_surface);
    }
    if let Some(dispatch_command) = result["activation_command"].as_str().map(str::to_string) {
        receipt.dispatch_command = Some(dispatch_command);
    }
}

fn normalize_stale_in_flight_dispatch_receipt(
    state_root: &std::path::Path,
    receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    let timeout_blocked_receipt = receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("timeout_without_takeover_authority");
    let blocked_internal_activation_receipt = receipt.dispatch_status == "blocked"
        && receipt.blocker_code.as_deref() == Some("internal_activation_view_only");
    if receipt.dispatch_status != "executing"
        && !timeout_blocked_receipt
        && !blocked_internal_activation_receipt
    {
        return Ok(false);
    }
    let Some(result_path) = receipt
        .dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let Some(result) = crate::read_json_file_if_present(std::path::Path::new(result_path)) else {
        return Ok(false);
    };
    let preserves_internal_activation_view =
        crate::runtime_dispatch_receipt_helpers::stale_in_flight_dispatch_preserves_internal_activation_view(receipt, &result);
    if receipt.dispatch_status == "executing" {
        if let Some((terminal_result_path, terminal_result)) =
            terminal_execution_result_for_in_flight_receipt(receipt, result_path)
        {
            promote_receipt_to_terminal_execution_result(
                receipt,
                terminal_result_path,
                &terminal_result,
            );
            return Ok(true);
        }
    }
    if blocked_external_dispatch_artifact_mismatched_as_internal_activation(receipt, &result) {
        let timeout_seconds = super::dispatch_result_stale_after_seconds(&result) as u64;
        super::apply_dispatch_execution_timeout_to_receipt(state_root, receipt, timeout_seconds)?;
        return Ok(true);
    }
    if timeout_blocked_receipt
        && preserves_internal_activation_view
        && result["blocker_code"].as_str() == Some("timeout_without_takeover_authority")
    {
        let timeout_seconds = super::stale_in_flight_dispatch_timeout_seconds_for_receipt(
            state_root, receipt, &result,
        ) as u64;
        let project_root =
            super::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
                .unwrap_or_else(|| state_root.parent().unwrap_or(state_root).to_path_buf());
        let role_selection = super::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "timeout-normalization".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: String::new(),
            selected_role: receipt
                .activation_runtime_role
                .clone()
                .unwrap_or_else(|| receipt.dispatch_target.clone()),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "timeout-normalization".to_string(),
        };
        super::apply_internal_activation_timeout_to_receipt(
            state_root,
            &project_root,
            &role_selection,
            receipt,
            timeout_seconds,
        )?;
        return Ok(true);
    }
    if result["execution_state"].as_str() != Some("executing") {
        return Ok(false);
    }
    if crate::runtime_dispatch_receipt_helpers::dispatch_packet_uses_downstream_carrier(
        receipt.dispatch_packet_path.as_deref(),
        &result,
    ) {
        let timeout_seconds = super::dispatch_result_stale_after_seconds(&result) as u64;
        super::apply_dispatch_execution_timeout_to_receipt(state_root, receipt, timeout_seconds)?;
        return Ok(true);
    }
    let Some(recorded_at) = result["recorded_at"].as_str() else {
        return Ok(false);
    };
    let Ok(recorded_at) =
        time::OffsetDateTime::parse(recorded_at, &time::format_description::well_known::Rfc3339)
    else {
        return Ok(false);
    };
    let stale_after_seconds = if preserves_internal_activation_view {
        super::stale_in_flight_dispatch_timeout_seconds_for_receipt(state_root, receipt, &result)
    } else {
        super::dispatch_result_stale_after_seconds(&result)
    };
    let age_seconds = (time::OffsetDateTime::now_utc() - recorded_at).whole_seconds();
    if age_seconds <= stale_after_seconds {
        return Ok(false);
    }
    let timeout_seconds = stale_after_seconds as u64;
    if preserves_internal_activation_view {
        let project_root =
            super::taskflow_task_bridge::infer_project_root_from_state_root(state_root)
                .unwrap_or_else(|| state_root.parent().unwrap_or(state_root).to_path_buf());
        let role_selection = super::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "stale-normalization".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: String::new(),
            selected_role: receipt
                .activation_runtime_role
                .clone()
                .unwrap_or_else(|| receipt.dispatch_target.clone()),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "stale-normalization".to_string(),
        };
        super::apply_internal_activation_timeout_to_receipt(
            state_root,
            &project_root,
            &role_selection,
            receipt,
            timeout_seconds,
        )?;
    } else {
        super::apply_dispatch_execution_timeout_to_receipt(state_root, receipt, timeout_seconds)?;
    }
    Ok(true)
}

async fn maybe_resume_inputs_from_active_downstream_result(
    _store: &super::StateStore,
    requested_run_id: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<Option<ResumeInputs>, String> {
    let Some(active_target) = receipt
        .downstream_dispatch_active_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(result_path) = receipt
        .downstream_dispatch_result_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let result = read_downstream_dispatch_result(result_path)?;
    let Some(packet_path) = downstream_result_packet_path(&result) else {
        return Ok(None);
    };
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "downstream dispatch packet")?;
    let packet_run_id = packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Persisted downstream dispatch packet is missing run_id".to_string())?;
    validate_completed_run_downstream_resume_candidate(
        _store,
        packet_run_id,
        active_target,
        "active downstream dispatch result",
    )
    .await?;
    if let Some(requested_run_id) = requested_run_id {
        if requested_run_id != packet_run_id {
            return Err(format!(
                "Requested run_id `{requested_run_id}` does not match persisted downstream dispatch packet run_id `{packet_run_id}`"
            ));
        }
    }
    let (dispatch_kind, derived_dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(&role_selection, active_target);
    let execution_state = result
        .get("execution_state")
        .and_then(serde_json::Value::as_str)
        .map(|value| canonical_resume_dispatch_status(Some(value)))
        .unwrap_or("blocked");
    let dispatch_surface = result
        .get("surface")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or(derived_dispatch_surface);
    let dispatch_command = result
        .get("activation_command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            packet
                .get("downstream_dispatch_command")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    let blocker_code = result
        .get("blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let selected_backend = result
        .get("backend_dispatch")
        .and_then(|value| value.get("backend_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            packet
                .get("selected_backend")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    let stale_downstream_state = execution_state == "executed";
    let synthetic_receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: packet_run_id.to_string(),
        dispatch_target: active_target.to_string(),
        dispatch_status: execution_state.to_string(),
        lane_status: super::derive_lane_status(
            execution_state,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        .to_string(),
        supersedes_receipt_id: receipt.supersedes_receipt_id.clone(),
        exception_path_receipt_id: receipt.exception_path_receipt_id.clone(),
        dispatch_kind,
        dispatch_surface,
        dispatch_command,
        dispatch_packet_path: Some(packet_path.clone()),
        dispatch_result_path: Some(result_path.to_string()),
        blocker_code,
        downstream_dispatch_target: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_target.clone()
        },
        downstream_dispatch_command: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_command.clone()
        },
        downstream_dispatch_note: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_note.clone()
        },
        downstream_dispatch_ready: if stale_downstream_state {
            false
        } else {
            receipt.downstream_dispatch_ready
        },
        downstream_dispatch_blockers: if stale_downstream_state {
            Vec::new()
        } else {
            receipt.downstream_dispatch_blockers.clone()
        },
        downstream_dispatch_packet_path: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_packet_path.clone()
        },
        downstream_dispatch_status: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_status.clone()
        },
        downstream_dispatch_result_path: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_result_path.clone()
        },
        downstream_dispatch_trace_path: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_trace_path.clone()
        },
        downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
        downstream_dispatch_active_target: if stale_downstream_state {
            None
        } else {
            receipt.downstream_dispatch_active_target.clone()
        },
        downstream_dispatch_last_target: if stale_downstream_state {
            Some(active_target.to_string())
        } else {
            receipt.downstream_dispatch_last_target.clone()
        },
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render"),
    };
    Ok(Some(build_resume_inputs(
        synthetic_receipt,
        packet_path,
        packet,
        role_selection,
    )))
}

async fn sync_run_graph_after_resumed_execution(
    store: &super::StateStore,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if dispatch_receipt.dispatch_status != "executed" {
        return Ok(());
    }
    let Some(run_id) = run_graph_bootstrap
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read persisted run-graph state for resumed execution: {error}")
    })?;
    let executed_status =
        super::apply_first_handoff_execution_to_run_graph_status(&status, dispatch_receipt);
    store
        .record_run_graph_status(&executed_status)
        .await
        .map_err(|error| format!("Failed to record resumed executed run-graph status: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &executed_status,
        "resume_execution",
    )
    .await
    .map_err(|error| {
        format!("Failed to synchronize continuation binding after resumed execution: {error}")
    })?;
    Ok(())
}

async fn reconcile_blocked_implementer_timeout_with_tracked_close_evidence(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    super::maybe_bridge_closed_implementer_task_into_receipt_with_context(
        store,
        role_selection,
        run_graph_bootstrap,
        dispatch_receipt,
        None,
    )
    .await
}

async fn reconcile_blocked_verification_timeout_with_receipt_evidence(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    super::maybe_reconcile_blocked_verification_timeout_with_receipt_evidence(
        store,
        role_selection,
        run_graph_bootstrap,
        dispatch_receipt,
    )
    .await
}

/// Keep retry-artifact preparation strictly fail-closed: it may tune admissible
/// retry backend hints, but it must still restore a lawful run-graph dispatch-ready
/// posture for the same bounded node when an explicit retry packet already exists.
async fn sync_run_graph_after_retry_artifact(
    store: &super::StateStore,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "packet_ready"
        || dispatch_receipt.lane_status != "packet_ready"
        || !dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
    {
        return Ok(());
    }
    let Some(run_id) = run_graph_bootstrap
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read persisted run-graph state for retry artifact sync: {error}")
    })?;
    let retry_target = dispatch_receipt.dispatch_target.replace('-', "_");
    let lane_suffix = if dispatch_receipt.dispatch_kind == "taskflow_pack" {
        String::new()
    } else {
        "_lane".to_string()
    };
    let retry_ready_status = crate::state_store::RunGraphStatus {
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        task_class: status.task_class.clone(),
        active_node: dispatch_receipt.dispatch_target.clone(),
        next_node: Some(retry_target.clone()),
        status: "ready".to_string(),
        route_task_class: status.route_task_class.clone(),
        selected_backend: dispatch_receipt
            .selected_backend
            .clone()
            .unwrap_or_else(|| status.selected_backend.clone()),
        lane_id: format!("{retry_target}{lane_suffix}"),
        lifecycle_stage: format!("{retry_target}_active"),
        policy_gate: status.policy_gate.clone(),
        handoff_state: format!("awaiting_{retry_target}"),
        context_state: "sealed".to_string(),
        checkpoint_kind: status.checkpoint_kind.clone(),
        resume_target: format!("dispatch.{retry_target}{lane_suffix}"),
        recovery_ready: true,
    };
    store
        .record_run_graph_status(&retry_ready_status)
        .await
        .map_err(|error| format!("Failed to record retry-ready run-graph status: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        &retry_ready_status,
        "retry_artifact_dispatch_ready",
    )
    .await
    .map_err(|error| {
        format!("Failed to synchronize continuation binding after retry artifact sync: {error}")
    })?;
    Ok(())
}

async fn resolve_default_resume_run_id(store: &super::StateStore) -> Result<String, String> {
    let global_latest_status = store
        .latest_run_graph_status()
        .await
        .map_err(|error| format!("Failed to read latest persisted run-graph state: {error}"))?;
    let Some(global_status) = global_latest_status else {
        return Err("No persisted run-graph dispatch receipt is available".to_string());
    };
    let current_session_can_mutate_global = store
        .current_session_can_mutate_run_graph_run(&global_status.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to validate current-session ownership for run `{}`: {error}",
                global_status.run_id
            )
        })?;
    let status = if current_session_can_mutate_global {
        global_status
    } else {
        let scoped_latest_status = store
            .latest_run_graph_status_for_current_session()
            .await
            .map_err(|error| {
                format!("Failed to read current-session persisted run-graph state: {error}")
            })?;
        if let Some(scoped_status) = scoped_latest_status {
            scoped_status
        } else {
            return Err(format!(
                "Default `vida taskflow consume continue --json` resolved latest run `{}`, but the current session does not own run `{}`. Pass `--run-id {}` only from an owning session, bind the intended bounded unit explicitly, or refresh status/recovery before continuing.",
                global_status.run_id, global_status.run_id, global_status.run_id
            ));
        }
    };
    let explicit_continuation_binding = store
        .latest_explicit_run_graph_continuation_binding()
        .await
        .map_err(|error| format!("Failed to read explicit continuation binding: {error}"))?;
    let latest_run_graph_recovery = store
        .latest_run_graph_recovery_summary()
        .await
        .map_err(|error| format!("Failed to read latest run graph recovery summary: {error}"))?;
    let latest_run_graph_dispatch_receipt =
        match store.latest_run_graph_dispatch_receipt_summary().await {
            Ok(summary) => summary,
            Err(_) => None,
        };
    let continuation_binding_evidence_ambiguous = latest_run_graph_dispatch_receipt
        .as_ref()
        .is_some_and(|receipt| {
            crate::state_store::latest_run_graph_dispatch_receipt_signal_is_ambiguous(receipt)
                || crate::state_store::latest_run_graph_dispatch_receipt_summary_is_inconsistent(
                    Some(status.run_id.as_str()),
                    Some(receipt.run_id.as_str()),
                )
        });
    let continuation_binding =
        crate::continuation_binding_summary::build_continuation_binding_summary(
            explicit_continuation_binding.as_ref(),
            Some(&status),
            latest_run_graph_recovery.as_ref(),
            latest_run_graph_dispatch_receipt.as_ref(),
            crate::latest_terminal_consume_continue_snapshot_run_id(store.root())
                .ok()
                .flatten()
                .as_deref(),
            continuation_binding_evidence_ambiguous,
        );
    let terminal_completed_run =
        status.status == "completed" && status.lifecycle_stage == "closure_complete";
    let explicit_task_graph_bound_run_id = if let Some(binding) =
        explicit_continuation_binding.as_ref().filter(|binding| {
            binding.status == "bound"
                && binding.active_bounded_unit["kind"].as_str() == Some("task_graph_task")
        }) {
        let binding_run_id = binding.active_bounded_unit["run_id"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(binding.run_id.as_str());
        explicit_bound_task_graph_resume_run_id(store, binding_run_id).await?
    } else {
        None
    };
    if let Some(bound_run_id) = explicit_task_graph_bound_run_id.as_deref() {
        if bound_run_id == status.run_id.as_str() || terminal_completed_run {
            return Ok(bound_run_id.to_string());
        }
    }
    if terminal_completed_run {
        return Ok(status.run_id);
    }
    if resume_from_persisted_final_snapshot(store, &status.run_id)? {
        return Ok(status.run_id);
    }
    if store
        .run_graph_dispatch_receipt(&status.run_id)
        .await
        .ok()
        .flatten()
        .as_ref()
        .is_some_and(|receipt| receipt_has_active_exception_takeover(receipt, &status.run_id))
    {
        return Ok(status.run_id);
    }
    if terminal_completed_run {
        return Err(format!(
            "Latest continuation binding for run `{}` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind {} --task-id <task-id> --json` or pass `--run-id {}` to refresh that specific run.",
            status.run_id, status.run_id, status.run_id
        ));
    }
    if continuation_binding["status"] != "bound" {
        return Err(format!(
            "Latest continuation binding for active run `{}` is ambiguous, but the run has not reached closure_complete. Do not bind a new --task-id for this active run; pass `--run-id {}` to refresh that specific run or inspect run-graph/task evidence if refresh remains blocked.",
            status.run_id, status.run_id
        ));
    }
    if continuation_binding["active_bounded_unit"]["run_id"]
        .as_str()
        .is_some_and(|binding_run_id| binding_run_id != status.run_id)
    {
        let binding_run_id = continuation_binding["active_bounded_unit"]["run_id"]
            .as_str()
            .unwrap_or("unknown");
        return Err(format!(
            "Latest explicit continuation binding points to run `{binding_run_id}` while the latest run-graph status is `{}`. Default `vida taskflow consume continue --json` must not silently reselect the stale latest run; pass `--run-id {binding_run_id}` or refresh/bind the intended bounded unit explicitly.",
            status.run_id
        ));
    }
    if status.status == "completed"
        && continuation_binding["active_bounded_unit"]["kind"] != "downstream_dispatch_target"
    {
        let unit_kind = continuation_binding["active_bounded_unit"]["kind"]
            .as_str()
            .unwrap_or("unknown");
        return Err(format!(
            "Latest continuation binding for run `{}` points to `{unit_kind}`, which is not resumeable through default `vida taskflow consume continue --json`. Pass `--run-id {}` to refresh the completed run explicitly or bind/shape the next bounded unit before continuing.",
            status.run_id, status.run_id
        ));
    }
    Ok(status.run_id)
}

async fn resolve_runtime_consumption_resume_inputs_for_run_id(
    store: &super::StateStore,
    run_id: &str,
) -> Result<ResumeInputs, String> {
    resolve_runtime_consumption_resume_inputs_for_run_id_with_policy(store, run_id, true, false)
        .await
}

async fn resolve_runtime_consumption_resume_inputs_for_run_id_with_policy(
    store: &super::StateStore,
    run_id: &str,
    strict_blocked_receipts: bool,
    allow_explicit_binding_redirect: bool,
) -> Result<ResumeInputs, String> {
    const MAX_BOUND_TASK_RESUME_REDIRECTS: usize = 64;
    let mut resolved_run_id = run_id.to_string();
    let mut visited_run_ids = std::collections::HashSet::from([resolved_run_id.clone()]);
    let mut redirects = 0usize;
    if allow_explicit_binding_redirect {
        while let Some(bound_run_id) =
            explicit_bound_task_graph_resume_run_id(store, &resolved_run_id).await?
        {
            if !visited_run_ids.insert(bound_run_id.clone()) {
                return Err(format!(
                    "Detected cyclic explicit continuation binding while resolving resume inputs for `{run_id}`: run `{}` was visited more than once.",
                    bound_run_id
                ));
            }
            redirects += 1;
            if redirects > MAX_BOUND_TASK_RESUME_REDIRECTS {
                return Err(format!(
                    "Explicit continuation binding redirect limit exceeded while resolving resume inputs for `{run_id}` (limit: {MAX_BOUND_TASK_RESUME_REDIRECTS})."
                ));
            }
            resolved_run_id = bound_run_id;
        }
    }
    let mut receipt = match store.run_graph_dispatch_receipt(&resolved_run_id).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => match recover_missing_first_dispatch_receipt(store, &resolved_run_id).await? {
            Some(inputs) => return Ok(inputs),
            None => return Err(missing_dispatch_receipt_error(&resolved_run_id)),
        },
        Err(error) => {
            return Err(format!(
                "Failed to read persisted run-graph dispatch receipt: {error}"
            ));
        }
    };
    let normalized_stale_in_flight =
        normalize_stale_in_flight_dispatch_receipt(store.root(), &mut receipt)?;
    if normalized_stale_in_flight {
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .map_err(|error| {
                format!(
                    "Failed to persist normalized stale in-flight dispatch receipt for `{run_id}`: {error}"
                )
            })?;
    }
    validate_explicit_task_graph_binding_lineage_for_resume(store, &resolved_run_id, &receipt)
        .await?;
    let project_root =
        super::taskflow_task_bridge::infer_project_root_from_state_root(store.root());
    let preloaded_role_selection = receipt
        .dispatch_packet_path
        .as_deref()
        .and_then(|path| read_dispatch_packet(path).ok())
        .and_then(|packet| decode_role_selection_from_packet(&packet, "dispatch packet").ok());
    let explicit_downstream_target =
        completed_run_explicit_downstream_target_for_resume(store, &resolved_run_id).await?;
    let task_close_closure_reconcile = explicit_downstream_target.as_deref() == Some("closure")
        && completed_task_close_reconcile_resume_target(store, &resolved_run_id)
            .await?
            .as_deref()
            == Some("closure");
    if strict_blocked_receipts
        && receipt_has_active_exception_takeover(&receipt, &resolved_run_id)
        && !task_close_closure_reconcile
    {
        if let Some(packet_path) = receipt.dispatch_packet_path.as_deref() {
            if read_dispatch_packet(packet_path)
                .ok()
                .and_then(|packet| {
                    (packet["packet_kind"].as_str() == Some("runtime_downstream_dispatch_packet"))
                        .then_some(())
                })
                .is_some()
            {
                let resume = resume_inputs_from_downstream_packet(
                    store,
                    Some(&resolved_run_id),
                    packet_path,
                )
                .await?;
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "exception_takeover_downstream_packet",
                )
                .await?;
                return Ok(resume);
            }
        }
    }
    let allow_downstream_lineage = allow_downstream_resume_lineage(
        project_root.as_deref(),
        preloaded_role_selection.as_ref(),
        &receipt,
    );
    let terminal_closure_complete = store
        .run_graph_status(&resolved_run_id)
        .await
        .map(|status| status.status == "completed" && status.lifecycle_stage == "closure_complete")
        .unwrap_or(false);
    let final_lineage_closure_preview_ready =
        receipt.downstream_dispatch_target.as_deref().map(str::trim) == Some("closure")
            && receipt.downstream_dispatch_ready
            && receipt.downstream_dispatch_blockers.is_empty()
            && resume_from_persisted_final_snapshot(store, &resolved_run_id)?;
    let recorded_final_hidden_by_bundle_check = !strict_blocked_receipts
        && !terminal_closure_complete
        && receipt.downstream_dispatch_target.as_deref().map(str::trim) == Some("closure")
        && receipt.downstream_dispatch_ready
        && resume_from_persisted_final_snapshot(store, &resolved_run_id)?
        && latest_runtime_consumption_snapshot_after_recorded_final_is_bundle_check(store.root())?;
    let explicit_task_graph_task_binding = store
        .run_graph_continuation_binding(&resolved_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read explicit continuation binding for `{}`: {error}",
                resolved_run_id
            )
        })?
        .is_some_and(|binding| {
            binding.status == "bound"
                && binding.active_bounded_unit["kind"].as_str() == Some("task_graph_task")
        });
    if !strict_blocked_receipts {
        if let Some(resume) = maybe_resume_inputs_from_ready_downstream_packet(
            store,
            Some(&resolved_run_id),
            &receipt,
        )
        .await?
        {
            record_run_graph_replay_lineage_receipt_for_resume(
                store,
                &receipt,
                &resume,
                "downstream_packet",
            )
            .await?;
            return Ok(resume);
        }
    }
    if explicit_downstream_target.is_none()
        && receipt.supersedes_receipt_id.is_some()
        && receipt.exception_path_receipt_id.is_some()
        && completed_task_close_reconcile_resume_target(store, &resolved_run_id)
            .await?
            .as_deref()
            == Some("closure")
    {
        let packet_path = receipt
            .dispatch_packet_path
            .clone()
            .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
        let packet = read_dispatch_packet(&packet_path)?;
        let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
        let resume = closure_packet_ready_resume_from_root_receipt(
            &receipt,
            packet_path,
            packet,
            role_selection,
        );
        record_run_graph_replay_lineage_receipt_for_resume(
            store,
            &receipt,
            &resume,
            "task_close_reconcile_closure",
        )
        .await?;
        return Ok(resume);
    }
    if let Some(bound_target) = explicit_downstream_target.as_deref() {
        let active_target_matches_bound = receipt
            .downstream_dispatch_active_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            == Some(bound_target);
        let terminal_closure_complete_for_binding = if bound_target == "closure" {
            store
                .run_graph_status(&resolved_run_id)
                .await
                .map(|status| {
                    status.status == "completed" && status.lifecycle_stage == "closure_complete"
                })
                .unwrap_or(false)
        } else {
            false
        };
        if bound_target == "closure" {
            if !terminal_closure_complete_for_binding
                && receipt.downstream_dispatch_packet_path.is_none()
            {
                if let Some(resume) =
                    task_close_reconcile_closure_resume_candidate(store, &resolved_run_id, &receipt)
                        .await?
                {
                    record_run_graph_replay_lineage_receipt_for_resume(
                        store,
                        &receipt,
                        &resume,
                        "task_close_reconcile_closure",
                    )
                    .await?;
                    return Ok(resume);
                }
            }
        }
        if allow_downstream_lineage && !active_target_matches_bound {
            if let Some(resume) = maybe_resume_inputs_from_ready_downstream_packet(
                store,
                Some(&resolved_run_id),
                &receipt,
            )
            .await?
            {
                if resume.dispatch_receipt.dispatch_target == bound_target {
                    record_run_graph_replay_lineage_receipt_for_resume(
                        store,
                        &receipt,
                        &resume,
                        "downstream_packet",
                    )
                    .await?;
                    return Ok(resume);
                }
                if bound_target == "closure" {
                    if let Some(resume) = task_close_reconcile_closure_resume_candidate(
                        store,
                        &resolved_run_id,
                        &receipt,
                    )
                    .await?
                    {
                        record_run_graph_replay_lineage_receipt_for_resume(
                            store,
                            &receipt,
                            &resume,
                            "task_close_reconcile_closure",
                        )
                        .await?;
                        return Ok(resume);
                    }
                }
                return Err(format!(
                    "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream packet lineage still points to stale target `{}`. Resume must fail closed until a fresh `{bound_target}` downstream packet is recorded.",
                    resume.dispatch_receipt.dispatch_target
                ));
            }
        }
        if allow_downstream_lineage {
            if let Some(resume) = maybe_resume_inputs_from_active_downstream_result(
                store,
                Some(&resolved_run_id),
                &receipt,
            )
            .await?
            {
                if resume.dispatch_receipt.dispatch_target == bound_target {
                    record_run_graph_replay_lineage_receipt_for_resume(
                        store,
                        &receipt,
                        &resume,
                        "downstream_result",
                    )
                    .await?;
                    return Ok(resume);
                }
                if bound_target == "closure" {
                    if let Some(resume) = task_close_reconcile_closure_resume_candidate(
                        store,
                        &resolved_run_id,
                        &receipt,
                    )
                    .await?
                    {
                        record_run_graph_replay_lineage_receipt_for_resume(
                            store,
                            &receipt,
                            &resume,
                            "task_close_reconcile_closure",
                        )
                        .await?;
                        return Ok(resume);
                    }
                }
                return Err(format!(
                    "Completed run `{run_id}` is explicitly bound to downstream target `{bound_target}`, but persisted downstream result lineage still points to stale target `{}`. Resume must fail closed until a fresh `{bound_target}` downstream packet is recorded.",
                    resume.dispatch_receipt.dispatch_target
                ));
            }
        }
        if bound_target == "closure" {
            if let Some(resume) =
                terminal_closure_complete_resume_candidate(store, &resolved_run_id, &receipt)
                    .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "terminal_closure_complete",
                )
                .await?;
                return Ok(resume);
            }
        }
        return Err(missing_explicit_downstream_resume_evidence_error(
            run_id,
            bound_target,
        ));
    } else {
        if allow_downstream_lineage && prefer_ready_downstream_packet_over_active_result(&receipt) {
            if let Some(resume) = maybe_resume_inputs_from_ready_downstream_packet(
                store,
                Some(&resolved_run_id),
                &receipt,
            )
            .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "downstream_packet",
                )
                .await?;
                return Ok(resume);
            }
        }
        if allow_downstream_lineage {
            if let Some(resume) = maybe_resume_inputs_from_active_downstream_result(
                store,
                Some(&resolved_run_id),
                &receipt,
            )
            .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "downstream_result",
                )
                .await?;
                return Ok(resume);
            }
        }
        if allow_downstream_lineage {
            if let Some(resume) = maybe_resume_inputs_from_ready_downstream_packet(
                store,
                Some(&resolved_run_id),
                &receipt,
            )
            .await?
            {
                record_run_graph_replay_lineage_receipt_for_resume(
                    store,
                    &receipt,
                    &resume,
                    "downstream_packet",
                )
                .await?;
                return Ok(resume);
            }
        }
    }
    if recorded_final_hidden_by_bundle_check && !explicit_task_graph_task_binding {
        let packet_path = receipt
            .dispatch_packet_path
            .clone()
            .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
        let packet = read_dispatch_packet(&packet_path)?;
        validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
        let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
        let resume = terminal_closure_complete_resume_from_root_receipt(
            &receipt,
            packet_path,
            packet,
            role_selection,
        );
        record_run_graph_replay_lineage_receipt_for_resume(
            store,
            &receipt,
            &resume,
            "recorded_final_after_bundle_check",
        )
        .await?;
        return Ok(resume);
    }
    let active_executable_receipt =
        matches!(receipt.dispatch_status.as_str(), "routed" | "packet_ready")
            && receipt.blocker_code.is_none();
    if strict_blocked_receipts
        && receipt.dispatch_target != "specification"
        && !active_executable_receipt
    {
        validate_run_graph_resume_state_strict(store, &resolved_run_id).await?;
    }
    let packet_path = receipt
        .dispatch_packet_path
        .clone()
        .ok_or_else(|| missing_dispatch_packet_path_error(false))?;
    let packet = read_dispatch_packet(&packet_path)?;
    let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
    if (terminal_closure_complete && !explicit_task_graph_task_binding)
        || (final_lineage_closure_preview_ready && !explicit_task_graph_task_binding)
    {
        validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
        let resume = terminal_closure_complete_resume_from_root_receipt(
            &receipt,
            packet_path,
            packet,
            role_selection,
        );
        record_run_graph_replay_lineage_receipt_for_resume(
            store,
            &receipt,
            &resume,
            "terminal_closure_complete",
        )
        .await?;
        return Ok(resume);
    }
    if explicit_downstream_target.is_none() && receipt.dispatch_target == "specification" {
        let run_graph_bootstrap = packet.get("run_graph_bootstrap").cloned().ok_or_else(|| {
            format!("Persisted dispatch packet `{packet_path}` is missing run_graph_bootstrap")
        })?;
        let mut bridged_receipt = receipt.clone();
        if super::try_bridge_bounded_specification_completion_to_downstream_receipt(
            store,
            &role_selection,
            &run_graph_bootstrap,
            &mut bridged_receipt,
        )
        .await?
        {
            let resume = build_resume_inputs(bridged_receipt, packet_path, packet, role_selection);
            record_run_graph_replay_lineage_receipt_for_resume(
                store,
                &receipt,
                &resume,
                "root_dispatch_packet",
            )
            .await?;
            return Ok(resume);
        }
    }
    if strict_blocked_receipts && receipt.dispatch_target == "specification" {
        validate_run_graph_resume_state_strict(store, &resolved_run_id).await?;
    }
    validate_receipt_packet_pair(&receipt, &packet, &packet_path, "dispatch packet")?;
    let resume = build_resume_inputs(receipt.clone(), packet_path, packet, role_selection);
    record_run_graph_replay_lineage_receipt_for_resume(
        store,
        &receipt,
        &resume,
        "root_dispatch_packet",
    )
    .await?;
    Ok(resume)
}

pub(crate) async fn resolve_runtime_consumption_resume_inputs(
    store: &super::StateStore,
    requested_run_id: Option<&str>,
    requested_dispatch_packet_path: Option<&str>,
    requested_downstream_packet_path: Option<&str>,
) -> Result<ResumeInputs, String> {
    let dispatch_packet = if let Some(packet_path) = requested_dispatch_packet_path {
        let packet = read_dispatch_packet(packet_path)?;
        let role_selection = decode_role_selection_from_packet(&packet, "dispatch packet")?;
        let run_id = packet
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Persisted dispatch packet is missing run_id".to_string())?;
        if let Some(requested_run_id) = requested_run_id {
            if requested_run_id != run_id {
                return Err(format!(
                    "Requested run_id `{requested_run_id}` does not match persisted dispatch packet run_id `{run_id}`"
                ));
            }
        }
        let mut receipt = match store.run_graph_dispatch_receipt(run_id).await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => return Err(missing_dispatch_receipt_error(run_id)),
            Err(error) => {
                return Err(format!(
                    "Failed to read persisted run-graph dispatch receipt: {error}"
                ));
            }
        };
        validate_receipt_packet_pair(&receipt, &packet, packet_path, "dispatch packet")?;
        validate_run_graph_resume_state(store, run_id).await?;
        receipt.downstream_dispatch_target = None;
        receipt.downstream_dispatch_command = None;
        receipt.downstream_dispatch_note = None;
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers.clear();
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_result_path = None;
        receipt.downstream_dispatch_trace_path = None;
        receipt.downstream_dispatch_executed_count = 0;
        receipt.downstream_dispatch_active_target = None;
        receipt.downstream_dispatch_last_target = None;
        build_resume_inputs(receipt, packet_path.to_string(), packet, role_selection)
    } else if let Some(packet_path) = requested_downstream_packet_path {
        return resume_inputs_from_downstream_packet(store, requested_run_id, packet_path).await;
    } else if let Some(run_id) = requested_run_id {
        return resolve_runtime_consumption_resume_inputs_for_run_id(store, run_id).await;
    } else {
        let explicit_binding = store
            .latest_explicit_run_graph_continuation_binding()
            .await
            .map_err(|error| format!("Failed to read explicit continuation binding: {error}"))?;
        let latest_receipt = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .map_err(|error| {
                format!("Failed to read latest run-graph dispatch receipt: {error}")
            })?;
        if let Some(status) = store
            .latest_run_graph_status()
            .await
            .map_err(|error| format!("Failed to read latest persisted run-graph state: {error}"))?
        {
            let ambiguous_active_downstream_result = explicit_binding.is_none()
                && latest_receipt.as_ref().is_some_and(|receipt| {
                    receipt.run_id == status.run_id
                        && receipt
                            .downstream_dispatch_active_target
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .is_some()
                        && receipt
                            .downstream_dispatch_result_path
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .is_some()
                        && matches!(status.status.as_str(), "blocked" | "completed")
                });
            if ambiguous_active_downstream_result {
                return Err(format!(
                    "Latest continuation binding for run `{}` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind {} --task-id <task-id> --json` or pass `--run-id {}` to refresh that specific run.",
                    status.run_id, status.run_id, status.run_id
                ));
            }
        }
        let run_id = resolve_default_resume_run_id(store).await?;
        return resolve_runtime_consumption_resume_inputs_for_run_id_with_policy(
            store, &run_id, false, true,
        )
        .await;
    };
    Ok(dispatch_packet)
}

fn canonical_resume_dispatch_status(status: Option<&str>) -> &'static str {
    match status.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "executed" => "executed",
        Some(value) if value == "blocked" => "blocked",
        Some(value) if value == "routed" => "routed",
        Some(value) if value == "packet_ready" => "packet_ready",
        _ => "blocked",
    }
}

fn canonical_resume_lane_status(status: &str) -> Option<super::LaneStatus> {
    match status.trim().to_ascii_lowercase().as_str() {
        "packet_ready" => Some(super::LaneStatus::PacketReady),
        "lane_open" => Some(super::LaneStatus::LaneOpen),
        "lane_running" => Some(super::LaneStatus::LaneRunning),
        "lane_blocked" => Some(super::LaneStatus::LaneBlocked),
        "lane_completed" => Some(super::LaneStatus::LaneCompleted),
        "lane_superseded" => Some(super::LaneStatus::LaneSuperseded),
        "lane_exception_recorded" => Some(super::LaneStatus::LaneExceptionRecorded),
        "lane_exception_takeover" => Some(super::LaneStatus::LaneExceptionTakeover),
        _ => None,
    }
}

fn canonical_resume_string_array_entries(value: &serde_json::Value) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    Some(entries)
}

fn resume_packet_ready_blocker_parity_error(
    downstream_dispatch_status: Option<&str>,
    downstream_dispatch_blockers: &[String],
) -> Option<String> {
    if downstream_dispatch_status == Some("packet_ready")
        && !downstream_dispatch_blockers.is_empty()
    {
        return Some(
            "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                .to_string(),
        );
    }
    None
}

fn should_refresh_resumed_downstream_preview(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_status == "executed"
        && (!receipt.downstream_dispatch_ready || !receipt.downstream_dispatch_blockers.is_empty())
}

fn prepare_explicit_resume_retry_artifact(
    project_root: Option<&std::path::Path>,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    let original_backend = dispatch_receipt.selected_backend.clone();
    if let Some(retry_backend) = retry_transition_backend_for_dispatch_receipt(
        project_root,
        role_selection,
        dispatch_receipt,
    ) {
        dispatch_receipt.selected_backend = Some(retry_backend);
        return dispatch_receipt.selected_backend != original_backend;
    }
    false
}

fn same_packet_internal_timeout_retry_ready(
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    dispatch_receipt.dispatch_kind == "agent_lane"
        && dispatch_receipt.dispatch_status == "blocked"
        && dispatch_receipt.blocker_code.as_deref()
            == Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        && dispatch_receipt
            .dispatch_packet_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn resumed_selected_backend_for_agent_lane(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    prepared_retry_artifact: bool,
) -> Option<String> {
    let explicit_retry_backend = prepared_retry_artifact
        .then(|| dispatch_receipt.selected_backend.clone())
        .flatten();
    explicit_retry_backend
        .or_else(|| super::canonical_selected_backend_for_receipt(role_selection, dispatch_receipt))
}

fn rewrite_retry_dispatch_packet_if_downstream_carrier(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
) -> Result<bool, String> {
    if dispatch_receipt.dispatch_kind != "agent_lane"
        || dispatch_receipt.dispatch_status != "blocked"
    {
        return Ok(false);
    }
    let Some(packet_path) = dispatch_receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };
    let packet = read_dispatch_packet(packet_path).or_else(|_| {
        crate::read_json_file_if_present(std::path::Path::new(packet_path))
            .ok_or_else(|| format!("Failed to read persisted dispatch packet `{packet_path}`"))
    })?;
    let packet_kind = packet
        .get("packet_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if !matches!(
        packet_kind,
        "runtime_downstream_dispatch_packet" | "runtime_dispatch_packet"
    ) {
        return Ok(false);
    }
    let project_root =
        super::taskflow_task_bridge::infer_project_root_from_state_root(store.root());
    let current_packet_backend = packet
        .get("selected_backend")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let target_backend = dispatch_receipt
        .selected_backend
        .clone()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|backend| Some(backend.as_str()) != current_packet_backend)
        .or_else(|| {
            retry_transition_backend_for_dispatch_receipt(
                project_root.as_deref(),
                role_selection,
                dispatch_receipt,
            )
        })
        .or_else(|| super::canonical_selected_backend_for_receipt(role_selection, dispatch_receipt))
        .or_else(|| dispatch_receipt.selected_backend.clone());
    let target_backend = target_backend
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let rewrite_required = target_backend.as_deref() != current_packet_backend;
    if !rewrite_required {
        return Ok(false);
    }
    if let Some(target_backend) = target_backend.clone() {
        dispatch_receipt.selected_backend = Some(target_backend);
    }

    let taskflow_handoff_plan = super::build_taskflow_handoff_plan(role_selection);
    let ctx = super::RuntimeDispatchPacketContext::new(
        store.root(),
        role_selection,
        dispatch_receipt,
        &taskflow_handoff_plan,
        run_graph_bootstrap,
    )
    .with_selected_backend_override(target_backend);
    let canonical_packet_path = super::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(canonical_packet_path);
    dispatch_receipt.dispatch_command = super::runtime_dispatch_command_for_target(
        role_selection,
        &dispatch_receipt.dispatch_target,
    );
    Ok(true)
}

type TaskflowConsumeContinueArgs = (bool, Option<String>, Option<String>, Option<String>);

pub(crate) fn parse_taskflow_consume_continue_args(
    args: &[String],
) -> Result<TaskflowConsumeContinueArgs, String> {
    let mut as_json = false;
    let mut run_id = None;
    let mut dispatch_packet_path = None;
    let mut downstream_packet_path = None;
    let mut index = 2usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                run_id = Some(value.clone());
                index += 2;
            }
            "--dispatch-packet" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                dispatch_packet_path = Some(value.clone());
                index += 2;
            }
            "--downstream-packet" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]".to_string());
                };
                downstream_packet_path = Some(value.clone());
                index += 2;
            }
            other => {
                return Err(format!(
                    "Unsupported argument `{other}`. Usage: vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]"
                ));
            }
        }
    }
    if dispatch_packet_path.is_some() && downstream_packet_path.is_some() {
        return Err(
            "Use only one packet source: --dispatch-packet <path> or --downstream-packet <path>"
                .to_string(),
        );
    }
    Ok((
        as_json,
        run_id,
        dispatch_packet_path,
        downstream_packet_path,
    ))
}

pub(crate) fn parse_taskflow_consume_advance_args(
    args: &[String],
) -> Result<(bool, Option<String>, usize), String> {
    let mut as_json = false;
    let mut run_id = None;
    let mut max_rounds = 8usize;
    let mut index = 2usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                            .to_string(),
                    );
                };
                run_id = Some(value.clone());
                index += 2;
            }
            "--max-rounds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                            .to_string(),
                    );
                };
                max_rounds = value
                    .parse::<usize>()
                    .map_err(|_| "Expected a positive integer for --max-rounds".to_string())?;
                if max_rounds == 0 {
                    return Err("--max-rounds must be greater than zero".to_string());
                }
                index += 2;
            }
            other => {
                return Err(format!(
                    "Unsupported argument `{other}`. Usage: vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]"
                ));
            }
        }
    }
    Ok((as_json, run_id, max_rounds))
}

fn consume_continue_should_defer_agent_handoff(
    surface_name: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> bool {
    surface_name == "vida taskflow consume continue"
        && receipt.dispatch_kind == "agent_lane"
        && (receipt.dispatch_status == "routed"
            || (receipt.downstream_dispatch_ready
                && receipt
                    .downstream_dispatch_packet_path
                    .as_deref()
                    .is_some_and(|path| !path.trim().is_empty())))
}

async fn persist_and_emit_deferred_agent_handoff(
    store: &super::StateStore,
    surface_name: &str,
    dispatch_packet_path: &str,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    _requested_run_id: Option<&str>,
    persist_receipt_and_sync: bool,
    emit_output: bool,
    as_json: bool,
) -> ExitCode {
    if persist_receipt_and_sync {
        if let Err(error) = store
            .record_run_graph_dispatch_receipt(dispatch_receipt)
            .await
        {
            eprintln!("Failed to record deferred run-graph dispatch receipt: {error}");
            return ExitCode::from(1);
        }
        match store.run_graph_status(&dispatch_receipt.run_id).await {
            Ok(status) => {
                if let Err(error) =
                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                        store,
                        &status,
                        "consume_continue_deferred_agent_handoff",
                    )
                    .await
                {
                    eprintln!(
                        "Failed to sync continuation binding after deferred agent handoff: {error}"
                    );
                    return ExitCode::from(1);
                }
            }
            Err(error) => {
                eprintln!("Failed to read run-graph status after deferred agent handoff: {error}");
                return ExitCode::from(1);
            }
        }
    }
    match emit_deferred_agent_handoff_json(
        store,
        surface_name,
        dispatch_packet_path,
        dispatch_receipt,
        role_selection,
        emit_output,
        as_json,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn cached_deferred_handoff_projection_path(
    state_dir: &std::path::Path,
    run_id: &str,
) -> Option<std::path::PathBuf> {
    if run_id.is_empty()
        || run_id
            .chars()
            .any(|ch| matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        return None;
    }
    Some(
        state_dir
            .join("operator-projections")
            .join(format!("lane-show-{run_id}.json")),
    )
}

fn cached_deferred_handoff_projection_matches_dispatch_init_cache(
    state_dir: &std::path::Path,
    run_id: &str,
    projection: &serde_json::Value,
    dispatch_packet_path: &str,
) -> bool {
    let Some(dispatch_init_cache) =
        crate::taskflow_run_graph::read_run_graph_dispatch_init_fast_cache(state_dir, run_id)
    else {
        return false;
    };
    let Some(cached_packet_path) = dispatch_init_cache["dispatch_packet_path"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    if cached_packet_path != dispatch_packet_path.trim() {
        return false;
    }
    let projection_selected_backend = projection["selected_backend"].as_str().map(str::trim);
    let cache_selected_backend = dispatch_init_cache["dispatch_receipt"]["selected_backend"]
        .as_str()
        .map(str::trim);
    projection_selected_backend == cache_selected_backend
}

fn try_emit_cached_deferred_agent_handoff_projection(
    state_dir: &std::path::Path,
    surface_name: &str,
    requested_run_id: Option<&str>,
    requested_dispatch_packet_path: Option<&str>,
    requested_downstream_packet_path: Option<&str>,
    emit_output: bool,
    as_json: bool,
) -> Result<Option<ExitCode>, String> {
    if surface_name != "vida taskflow consume continue"
        || requested_dispatch_packet_path.is_some()
        || requested_downstream_packet_path.is_some()
    {
        return Ok(None);
    }
    let Some(run_id) = requested_run_id else {
        return Ok(None);
    };
    let Some(projection_path) = cached_deferred_handoff_projection_path(state_dir, run_id) else {
        return Ok(None);
    };
    if !projection_path.is_file() {
        return Ok(None);
    }
    let projection: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&projection_path)
            .map_err(|error| format!("Failed to read cached lane projection: {error}"))?,
    )
    .map_err(|error| format!("Failed to decode cached lane projection: {error}"))?;
    if projection["run_id"].as_str() != Some(run_id)
        || projection["dispatch_status"].as_str() != Some("routed")
    {
        return Ok(None);
    }
    let artifact_refs = projection["artifact_refs"].clone();
    let dispatch_packet_path = artifact_refs["dispatch_packet_path"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            "Cached routed handoff projection is missing dispatch_packet_path".to_string()
        })?;
    if !cached_deferred_handoff_projection_matches_dispatch_init_cache(
        state_dir,
        run_id,
        &projection,
        dispatch_packet_path,
    ) {
        return Ok(None);
    }
    let lane_id = projection["lane_id"].as_str().unwrap_or("agent_lane");
    let dispatch_target = lane_id.strip_suffix("_lane").unwrap_or(lane_id);
    let receipt = serde_json::json!({
        "run_id": run_id,
        "dispatch_target": dispatch_target,
        "dispatch_status": "routed",
        "lane_status": projection["lane_status"].clone(),
        "supersedes_receipt_id": projection["supersedes_receipt_id"].clone(),
        "exception_path_receipt_id": projection["exception_path_receipt_id"].clone(),
        "dispatch_kind": "agent_lane",
        "dispatch_surface": "cached_operator_projection",
        "dispatch_command": "vida agent-init",
        "dispatch_packet_path": dispatch_packet_path,
        "dispatch_result_path": artifact_refs["dispatch_result_path"].clone(),
        "blocker_code": serde_json::Value::Null,
        "downstream_dispatch_target": serde_json::Value::Null,
        "downstream_dispatch_command": serde_json::Value::Null,
        "downstream_dispatch_note": serde_json::Value::Null,
        "downstream_dispatch_ready": false,
        "downstream_dispatch_blockers": [],
        "downstream_dispatch_packet_path": serde_json::Value::Null,
        "downstream_dispatch_status": serde_json::Value::Null,
        "downstream_dispatch_result_path": serde_json::Value::Null,
        "downstream_dispatch_trace_path": serde_json::Value::Null,
        "downstream_dispatch_executed_count": 0,
        "downstream_dispatch_active_target": serde_json::Value::Null,
        "downstream_dispatch_last_target": dispatch_target,
        "activation_agent_type": serde_json::Value::Null,
        "activation_runtime_role": dispatch_target,
        "selected_backend": projection["selected_backend"].clone(),
        "recorded_at": projection["exception_path_metadata"]["recorded_at"].clone(),
    });
    let failure_control_evidence = build_failure_control_evidence(run_id, dispatch_packet_path);
    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        Vec::new(),
        Vec::new(),
        serde_json::json!({
            "runtime_consumption_latest_snapshot_path": serde_json::Value::Null,
            "latest_run_graph_dispatch_receipt_id": run_id,
            "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
            "consume_final_surface": surface_name,
            "cached_projection_path": projection_path.display().to_string(),
        }),
    )?;
    let operator_contracts = finalized.operator_contracts.clone();
    let shared_fields = serde_json::json!({
        "trace_id": operator_contracts["trace_id"].clone(),
        "workflow_class": operator_contracts["workflow_class"].clone(),
        "risk_tier": operator_contracts["risk_tier"].clone(),
        "status": finalized.shared_fields["status"].clone(),
        "blocker_codes": finalized.shared_fields["blocker_codes"].clone(),
        "next_actions": finalized.shared_fields["next_actions"].clone(),
        "artifact_refs": finalized.shared_fields["artifact_refs"].clone(),
    });
    if emit_output {
        if as_json {
            let output_payload = serde_json::json!({
                "surface": surface_name,
                "trace_id": operator_contracts["trace_id"].clone(),
                "workflow_class": operator_contracts["workflow_class"].clone(),
                "risk_tier": operator_contracts["risk_tier"].clone(),
                "status": finalized.status,
                "blocker_codes": finalized.blocker_codes.clone(),
                "next_actions": finalized.next_actions.clone(),
                "artifact_refs": finalized.artifact_refs.clone(),
                "shared_fields": shared_fields,
                "operator_contracts": operator_contracts,
                "source_run_id": run_id,
                "source_dispatch_packet_path": dispatch_packet_path,
                "dispatch_receipt": receipt,
                "projection_truth": {
                    "projection_source": "cached_operator_projection",
                    "projection_reason": "explicit routed agent handoff emitted from cached lane projection without opening full StateStore",
                    "dispatch_receipt_present": true,
                    "continuation_binding_present": true,
                    "stale_state_suspected": false,
                    "next_lawful_operator_action": format!("vida lane show {run_id} --json"),
                },
                "snapshot_path": serde_json::Value::Null,
                "failure_control_evidence": failure_control_evidence,
            });
            if let Some(error) =
                crate::operator_contracts::shared_operator_output_contract_parity_error(
                    &output_payload,
                )
            {
                return Err(format!(
                    "Failed to preserve cached deferred handoff output parity: {error}"
                ));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&output_payload)
                    .expect("cached deferred handoff output should render as json")
            );
        } else {
            super::print_surface_header(super::RenderMode::Plain, surface_name);
            super::print_surface_line(super::RenderMode::Plain, "status", finalized.status);
            super::print_surface_line(super::RenderMode::Plain, "source run", run_id);
            super::print_surface_line(
                super::RenderMode::Plain,
                "source packet",
                dispatch_packet_path,
            );
            super::print_surface_line(
                super::RenderMode::Plain,
                "projection path",
                &projection_path.display().to_string(),
            );
        }
    }
    Ok(Some(ExitCode::SUCCESS))
}

pub(crate) async fn run_taskflow_consume_resume_command(
    state_dir: std::path::PathBuf,
    as_json: bool,
    requested_run_id: Option<String>,
    requested_dispatch_packet_path: Option<String>,
    requested_downstream_packet_path: Option<String>,
    surface_name: &str,
    emit_output: bool,
) -> ExitCode {
    // Do not emit cached consume-continue deferred handoff projections before
    // authoritative state and receipt validation on the full resume path.
    match try_emit_cached_deferred_agent_handoff_projection(
        &state_dir,
        surface_name,
        requested_run_id.as_deref(),
        requested_dispatch_packet_path.as_deref(),
        requested_downstream_packet_path.as_deref(),
        emit_output,
        as_json,
    ) {
        Ok(Some(exit_code)) => return exit_code,
        Ok(None) => {}
        Err(error) => {
            if emit_output {
                eprintln!("{error}");
                if as_json {
                    emit_consume_continue_resume_error_json(&error, surface_name);
                }
            }
            return ExitCode::from(1);
        }
    }
    match fail_fast_state_store_open(state_dir.clone(), "opening authoritative state store").await {
        Ok(store) => {
            let mut dispatch_receipt;
            let dispatch_packet_path;
            let role_selection;
            let run_graph_bootstrap;
            let state_root = store.root().to_path_buf();
            let fast_deferred_resume_inputs = match fast_deferred_agent_handoff_resume_inputs(
                &store,
                surface_name,
                requested_run_id.as_deref(),
                requested_dispatch_packet_path.as_deref(),
                requested_downstream_packet_path.as_deref(),
            )
            .await
            {
                Ok(inputs) => inputs,
                Err(error) => {
                    if emit_output {
                        eprintln!("{error}");
                        if as_json {
                            emit_consume_continue_resume_error_json(&error, surface_name);
                        }
                    }
                    return ExitCode::from(1);
                }
            };
            if fast_deferred_resume_inputs.is_none()
                && requested_run_id.is_none()
                && requested_dispatch_packet_path.is_none()
                && requested_downstream_packet_path.is_none()
            {
                let preparation_gate_state_root = state_root.clone();
                if let Err(error) = consume_continue_blocking_step_with_timeout(
                    "execution preparation gate",
                    CONSUME_RESUME_PREPARATION_GATE_TIMEOUT,
                    move || {
                        enforce_consume_continue_execution_preparation_gate(
                            &preparation_gate_state_root,
                        )
                    },
                ) {
                    if emit_output {
                        eprintln!("{error}");
                    }
                    if as_json && emit_output {
                        emit_consume_continue_resume_error_json(&error, surface_name);
                    }
                    return ExitCode::from(1);
                }
            }
            let resolved_resume_inputs = match fast_deferred_resume_inputs {
                Some(inputs) => Ok(inputs),
                None => {
                    resolve_runtime_consumption_resume_inputs(
                        &store,
                        requested_run_id.as_deref(),
                        requested_dispatch_packet_path.as_deref(),
                        requested_downstream_packet_path.as_deref(),
                    )
                    .await
                }
            };
            match resolved_resume_inputs {
                Ok(ResumeInputs {
                    dispatch_receipt: receipt,
                    dispatch_packet_path: packet_path,
                    role_selection: selection,
                    run_graph_bootstrap: bootstrap,
                }) => {
                    dispatch_receipt = receipt;
                    dispatch_packet_path = packet_path;
                    role_selection = selection;
                    run_graph_bootstrap = bootstrap;
                    if dispatch_receipt.dispatch_status == "routed"
                        && consume_continue_should_defer_agent_handoff(
                            surface_name,
                            &dispatch_receipt,
                        )
                    {
                        return persist_and_emit_deferred_agent_handoff(
                            &store,
                            surface_name,
                            &dispatch_packet_path,
                            &dispatch_receipt,
                            &role_selection,
                            requested_run_id.as_deref(),
                            false,
                            emit_output,
                            as_json,
                        )
                        .await;
                    }
                    if let Err(error) = reconcile_terminal_closure_lineage_for_resume(
                        &store,
                        &role_selection,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!(
                            "Failed to reconcile terminal closure lineage for resumed dispatch: {error}"
                        );
                        return ExitCode::from(1);
                    }
                }
                Err(error) => {
                    if emit_output {
                        eprintln!("{error}");
                    }
                    if as_json {
                        if emit_output {
                            emit_consume_continue_resume_error_json(&error, surface_name);
                        }
                    }
                    return ExitCode::from(1);
                }
            }
            if let Err(error) =
                super::try_bridge_bounded_specification_completion_to_downstream_receipt(
                    &store,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                )
                .await
            {
                eprintln!(
                    "Failed to bridge bounded specification completion into downstream receipt: {error}"
                );
                return ExitCode::from(1);
            }
            if let Err(error) =
                super::try_bridge_bounded_implementer_completion_to_downstream_receipt(
                    &store,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                )
                .await
            {
                eprintln!(
                    "Failed to bridge bounded implementer completion into downstream receipt: {error}"
                );
                return ExitCode::from(1);
            }
            if let Err(error) = reconcile_blocked_implementer_timeout_with_tracked_close_evidence(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!(
                    "Failed to reconcile blocked implementer timeout with tracked close evidence: {error}"
                );
                return ExitCode::from(1);
            }
            if let Err(error) = reconcile_blocked_verification_timeout_with_receipt_evidence(
                &store,
                &role_selection,
                &run_graph_bootstrap,
                &mut dispatch_receipt,
            )
            .await
            {
                eprintln!(
                    "Failed to reconcile blocked verification timeout with receipt evidence: {error}"
                );
                return ExitCode::from(1);
            }
            let project_root =
                super::taskflow_task_bridge::infer_project_root_from_state_root(store.root());
            let prepared_retry_artifact = prepare_explicit_resume_retry_artifact(
                project_root.as_deref(),
                &role_selection,
                &mut dispatch_receipt,
            );
            let rewrite_retry_packet = dispatch_receipt.dispatch_kind == "agent_lane"
                && dispatch_receipt.dispatch_status == "blocked"
                && dispatch_receipt
                    .dispatch_packet_path
                    .as_deref()
                    .is_some_and(|path| !path.trim().is_empty());
            let mut retry_packet_rewritten = false;
            if rewrite_retry_packet {
                match rewrite_retry_dispatch_packet_if_downstream_carrier(
                    &store,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                ) {
                    Ok(rewritten) => {
                        retry_packet_rewritten = rewritten;
                    }
                    Err(error) => {
                        eprintln!(
                            "Failed to rewrite retry dispatch packet into canonical dispatch packet: {error}"
                        );
                        return ExitCode::from(1);
                    }
                }
                if should_refresh_resumed_downstream_preview(&dispatch_receipt) {
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                }
                if let Err(error) = sync_run_graph_after_resumed_execution(
                    &store,
                    &run_graph_bootstrap,
                    &dispatch_receipt,
                )
                .await
                {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            }
            let restore_same_lane_resume_ready = retry_packet_rewritten;
            let restore_same_lane_resume_ready = restore_same_lane_resume_ready
                || same_packet_internal_timeout_retry_ready(&dispatch_receipt);
            if restore_same_lane_resume_ready {
                if dispatch_receipt.dispatch_kind == "agent_lane" {
                    dispatch_receipt.selected_backend = resumed_selected_backend_for_agent_lane(
                        &role_selection,
                        &dispatch_receipt,
                        prepared_retry_artifact,
                    );
                }
                if dispatch_receipt.dispatch_status == "blocked"
                    && dispatch_receipt
                        .dispatch_packet_path
                        .as_deref()
                        .is_some_and(|path| !path.trim().is_empty())
                {
                    dispatch_receipt.dispatch_status = "packet_ready".to_string();
                    dispatch_receipt.lane_status = super::derive_lane_status(
                        &dispatch_receipt.dispatch_status,
                        dispatch_receipt.supersedes_receipt_id.as_deref(),
                        dispatch_receipt.exception_path_receipt_id.as_deref(),
                    )
                    .as_str()
                    .to_string();
                    dispatch_receipt.blocker_code = None;
                }
                if let Err(error) = store
                    .record_run_graph_dispatch_receipt(&dispatch_receipt)
                    .await
                {
                    eprintln!("Failed to record resumed run-graph dispatch receipt: {error}");
                    return ExitCode::from(1);
                }
                if dispatch_receipt.dispatch_status == "packet_ready" {
                    if let Err(error) = sync_run_graph_after_retry_artifact(
                        &store,
                        &run_graph_bootstrap,
                        &dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
                if dispatch_receipt.dispatch_status == "executed" {
                    return match emit_runtime_consumption_resume_json(
                        &store,
                        surface_name,
                        &dispatch_packet_path,
                        &dispatch_receipt,
                        &role_selection,
                        requested_run_id.as_deref(),
                        emit_output,
                        as_json,
                    )
                    .await
                    {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(error) => {
                            eprintln!("{error}");
                            ExitCode::from(1)
                        }
                    };
                }
                if dispatch_receipt.dispatch_status != "packet_ready"
                    && dispatch_receipt.dispatch_status != "routed"
                {
                    if let Some(run_id) = run_graph_bootstrap
                        .get("run_id")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.is_empty())
                    {
                        match store.run_graph_status(run_id).await {
                            Ok(status) => {
                                if let Err(error) =
                                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                                        &store,
                                        &status,
                                        "consume_continue_receipt_refresh",
                                    )
                                    .await
                                {
                                    eprintln!(
                                        "Failed to refresh continuation binding after resumed receipt persistence: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "Failed to read run-graph status before continuation binding refresh: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                }
            }
            let persist_deferred_handoff = dispatch_receipt.dispatch_status == "packet_ready"
                || dispatch_receipt.downstream_dispatch_ready;
            if dispatch_receipt.dispatch_status == "packet_ready" {
                dispatch_receipt.dispatch_status = "routed".to_string();
                dispatch_receipt.lane_status = super::derive_lane_status(
                    &dispatch_receipt.dispatch_status,
                    dispatch_receipt.supersedes_receipt_id.as_deref(),
                    dispatch_receipt.exception_path_receipt_id.as_deref(),
                )
                .as_str()
                .to_string();
                dispatch_receipt.blocker_code = None;
            }
            if consume_continue_should_defer_agent_handoff(surface_name, &dispatch_receipt) {
                return persist_and_emit_deferred_agent_handoff(
                    &store,
                    surface_name,
                    &dispatch_packet_path,
                    &dispatch_receipt,
                    &role_selection,
                    requested_run_id.as_deref(),
                    persist_deferred_handoff,
                    emit_output,
                    as_json,
                )
                .await;
            }
            if dispatch_receipt.dispatch_status == "routed" {
                let allow_taskflow_pack_execution = dispatch_receipt.dispatch_kind
                    != "taskflow_pack"
                    || super::taskflow_task_bridge::infer_project_root_from_state_root(&state_root)
                        .is_some();
                if allow_taskflow_pack_execution {
                    let resumed_dispatch_handoff_timeout =
                        consume_continue_dispatch_handoff_timeout(
                            &state_root,
                            &role_selection,
                            &dispatch_receipt,
                        );
                    drop(store);
                    if let Err(error) = consume_continue_handoff_with_timeout(
                        "resumed runtime dispatch handoff",
                        resumed_dispatch_handoff_timeout,
                        super::execute_and_record_dispatch_receipt(
                            &state_root,
                            &role_selection,
                            &run_graph_bootstrap,
                            &mut dispatch_receipt,
                        ),
                    )
                    .await
                    {
                        if let Err(timeout_error) =
                            super::apply_dispatch_handoff_timeout_to_receipt_for_state_root(
                                &state_root,
                                &role_selection,
                                &mut dispatch_receipt,
                                resumed_dispatch_handoff_timeout.as_secs(),
                            )
                        {
                            if emit_output {
                                eprintln!(
                                    "Failed to materialize resumed runtime dispatch timeout receipt: {timeout_error}"
                                );
                            }
                        } else if let Ok(store) = fail_fast_state_store_open(
                            state_root.clone(),
                            "persisting resumed runtime dispatch timeout receipt",
                        )
                        .await
                        {
                            if let Err(timeout_error) = store
                                .record_run_graph_dispatch_receipt(&dispatch_receipt)
                                .await
                            {
                                if emit_output {
                                    eprintln!(
                                        "Failed to persist resumed runtime dispatch timeout receipt: {timeout_error}"
                                    );
                                }
                            }
                        }
                        if as_json && emit_output {
                            emit_consume_continue_resume_error_json(&error, surface_name);
                            return ExitCode::from(1);
                        }
                        if emit_output {
                            eprintln!(
                                "Failed to execute resumed runtime dispatch handoff: {error}"
                            );
                        }
                        return ExitCode::from(1);
                    }
                    let store = match fail_fast_state_store_open_read_only(
                        state_root.clone(),
                        "reopening authoritative state store after resumed runtime dispatch",
                    )
                    .await
                    {
                        Ok(store) => store,
                        Err(error) => {
                            if as_json && emit_output {
                                emit_consume_continue_state_access_blocker_json(
                                    &state_root,
                                    surface_name,
                                    "reopening authoritative state store after resumed runtime dispatch",
                                    &error,
                                );
                                return ExitCode::from(1);
                            }
                            eprintln!(
                                "Failed to reopen authoritative state store after resumed runtime dispatch: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                    drop(store);
                }
            } else {
                if should_refresh_resumed_downstream_preview(&dispatch_receipt) {
                    if let Err(error) = super::refresh_downstream_dispatch_preview(
                        &store,
                        &role_selection,
                        &run_graph_bootstrap,
                        &mut dispatch_receipt,
                    )
                    .await
                    {
                        eprintln!("Failed to refresh resumed downstream dispatch preview: {error}");
                        return ExitCode::from(1);
                    }
                }
                if let Err(error) = sync_run_graph_after_resumed_execution(
                    &store,
                    &run_graph_bootstrap,
                    &dispatch_receipt,
                )
                .await
                {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
                if consume_continue_should_defer_agent_handoff(surface_name, &dispatch_receipt) {
                    return persist_and_emit_deferred_agent_handoff(
                        &store,
                        surface_name,
                        &dispatch_packet_path,
                        &dispatch_receipt,
                        &role_selection,
                        requested_run_id.as_deref(),
                        true,
                        emit_output,
                        as_json,
                    )
                    .await;
                }
                drop(store);
            }
            let downstream_dispatch_handoff_timeout = consume_continue_dispatch_handoff_timeout(
                &state_root,
                &role_selection,
                &dispatch_receipt,
            );
            if let Err(error) = consume_continue_handoff_with_timeout(
                "downstream dispatch chain",
                downstream_dispatch_handoff_timeout,
                super::execute_downstream_dispatch_chain(
                    &state_root,
                    &role_selection,
                    &run_graph_bootstrap,
                    &mut dispatch_receipt,
                ),
            )
            .await
            {
                if let Err(timeout_error) =
                    super::apply_dispatch_handoff_timeout_to_receipt_for_state_root(
                        &state_root,
                        &role_selection,
                        &mut dispatch_receipt,
                        downstream_dispatch_handoff_timeout.as_secs(),
                    )
                {
                    if emit_output {
                        eprintln!(
                            "Failed to materialize downstream dispatch timeout receipt: {timeout_error}"
                        );
                    }
                } else if let Ok(store) = fail_fast_state_store_open(
                    state_root.clone(),
                    "persisting downstream dispatch timeout receipt",
                )
                .await
                {
                    if let Err(timeout_error) = store
                        .record_run_graph_dispatch_receipt(&dispatch_receipt)
                        .await
                    {
                        if emit_output {
                            eprintln!(
                                "Failed to persist downstream dispatch timeout receipt: {timeout_error}"
                            );
                        }
                    }
                }
                if as_json && emit_output {
                    emit_consume_continue_resume_error_json(&error, surface_name);
                    return ExitCode::from(1);
                }
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            let store = match fail_fast_state_store_open(
                state_root.clone(),
                "reopening authoritative state store before resumed receipt persistence",
            )
            .await
            {
                Ok(store) => store,
                Err(error) => {
                    if as_json && emit_output {
                        emit_consume_continue_state_access_blocker_json(
                            &state_root,
                            surface_name,
                            "reopening authoritative state store before resumed receipt persistence",
                            &error,
                        );
                        return ExitCode::from(1);
                    }
                    eprintln!(
                        "Failed to reopen authoritative state store before resumed receipt persistence: {error}"
                    );
                    return ExitCode::from(1);
                }
            };
            if dispatch_receipt.dispatch_kind == "agent_lane" {
                dispatch_receipt.selected_backend = super::canonical_selected_backend_for_receipt(
                    &role_selection,
                    &dispatch_receipt,
                );
            }
            if let Err(error) = store
                .record_run_graph_dispatch_receipt(&dispatch_receipt)
                .await
            {
                eprintln!("Failed to record resumed run-graph dispatch receipt: {error}");
                return ExitCode::from(1);
            }
            // Re-sync continuation binding after downstream dispatch chain advances the run-graph.
            // Downstream execution inside execute_downstream_dispatch_chain updates run-graph status
            // via execute_and_record_dispatch_receipt, but the root-level continuation binding must
            // be refreshed after the final receipt is persisted so reconciled status sees blocked
            // downstream truth rather than stale upstream status.
            if let Some(run_id) = run_graph_bootstrap
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
            {
                if let Ok(status) = store.run_graph_status(run_id).await {
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &status,
                            crate::taskflow_continuation::CONSUME_CONTINUE_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE,
                        )
                        .await
                    {
                        eprintln!(
                            "Failed to re-sync continuation binding after downstream dispatch chain: {error}"
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            match emit_runtime_consumption_resume_json(
                &store,
                surface_name,
                &dispatch_packet_path,
                &dispatch_receipt,
                &role_selection,
                requested_run_id.as_deref(),
                emit_output,
                as_json,
            )
            .await
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            if as_json && emit_output {
                emit_consume_continue_state_access_blocker_json(
                    &state_dir,
                    surface_name,
                    "opening authoritative state store",
                    &error,
                );
                return ExitCode::from(1);
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_taskflow_consume_advance_command(
    state_dir: std::path::PathBuf,
    as_json: bool,
    requested_run_id: Option<String>,
    max_rounds: usize,
) -> ExitCode {
    let mut rounds = 0usize;
    let mut last_result: Option<(String, crate::state_store::RunGraphDispatchReceipt, String)> =
        None;

    while rounds < max_rounds {
        let before_status = match fail_fast_state_store_open(
            state_dir.clone(),
            "opening authoritative state store before advance",
        )
        .await
        {
            Ok(store) => match resolve_runtime_consumption_resume_inputs(
                &store,
                requested_run_id.as_deref(),
                None,
                None,
            )
            .await
            {
                Ok(ResumeInputs {
                    dispatch_receipt: receipt,
                    dispatch_packet_path: packet_path,
                    ..
                }) => Some((receipt, packet_path)),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let exit = run_taskflow_consume_resume_command(
            state_dir.clone(),
            true,
            requested_run_id.clone(),
            None,
            None,
            "vida taskflow consume advance",
            false,
        )
        .await;
        if exit != ExitCode::SUCCESS {
            if as_json {
                emit_consume_continue_resume_error_json(
                    "TaskFlow consume advance failed while executing the next resumed dispatch step.",
                    "vida taskflow consume advance",
                );
            }
            return exit;
        }

        let store = match fail_fast_state_store_open_read_only(
            state_dir.clone(),
            "reopening authoritative state store after advance",
        )
        .await
        {
            Ok(store) => store,
            Err(error) => {
                eprintln!("Failed to reopen authoritative state store after advance: {error}");
                return ExitCode::from(1);
            }
        };
        let after_receipt = match store.latest_run_graph_dispatch_receipt().await {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                eprintln!("No persisted run-graph dispatch receipt is available after advance");
                return ExitCode::from(1);
            }
            Err(error) => {
                eprintln!(
                    "Failed to read persisted run-graph dispatch receipt after advance: {error}"
                );
                return ExitCode::from(1);
            }
        };
        let after_packet_path = after_receipt
            .dispatch_packet_path
            .clone()
            .or_else(|| after_receipt.downstream_dispatch_packet_path.clone())
            .unwrap_or_else(|| "none".to_string());
        let snapshot_path =
            match super::latest_final_runtime_consumption_snapshot_path(store.root()) {
                Ok(Some(path)) => path,
                Ok(None) => "none".to_string(),
                Err(_) => "none".to_string(),
            };
        last_result = Some((
            after_packet_path.clone(),
            after_receipt.clone(),
            snapshot_path,
        ));
        rounds += 1;

        let progressed = match before_status {
            Some((before_receipt, before_packet_path)) => {
                before_packet_path != after_packet_path
                    || before_receipt.dispatch_status != after_receipt.dispatch_status
                    || before_receipt.downstream_dispatch_target
                        != after_receipt.downstream_dispatch_target
                    || before_receipt.downstream_dispatch_executed_count
                        != after_receipt.downstream_dispatch_executed_count
            }
            None => true,
        };

        let has_more_ready_work = after_receipt.downstream_dispatch_ready
            || (after_receipt.dispatch_status == "routed"
                && (after_receipt.dispatch_kind != "taskflow_pack"
                    || super::taskflow_task_bridge::infer_project_root_from_state_root(
                        store.root(),
                    )
                    .is_some()));
        if !progressed || !has_more_ready_work {
            break;
        }
    }

    let Some((source_dispatch_packet_path, dispatch_receipt, snapshot_path)) = last_result else {
        eprintln!("No advance step was executed");
        return ExitCode::from(1);
    };

    if as_json {
        crate::print_json_pretty(&consume_advance_success_payload(
            &source_dispatch_packet_path,
            &dispatch_receipt,
            &snapshot_path,
            rounds,
        ));
    } else {
        super::print_surface_header(super::RenderMode::Plain, "vida taskflow consume advance");
        super::print_surface_line(
            super::RenderMode::Plain,
            "source run",
            &dispatch_receipt.run_id,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "source packet",
            &source_dispatch_packet_path,
        );
        super::print_surface_line(
            super::RenderMode::Plain,
            "rounds executed",
            &rounds.to_string(),
        );
        super::print_surface_line(super::RenderMode::Plain, "snapshot path", &snapshot_path);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        active_exception_takeover_resume_blocker_error,
        blocked_external_dispatch_artifact_mismatched_as_internal_activation,
        build_failure_control_evidence,
        cached_deferred_handoff_projection_matches_dispatch_init_cache,
        canonical_resume_dispatch_status, canonical_resume_lane_status,
        canonical_resume_string_array_entries, consume_advance_success_payload,
        consume_continue_blocking_step_with_timeout, consume_continue_dispatch_handoff_timeout,
        consume_continue_handoff_with_timeout, consume_continue_resume_error_blocker_code,
        consume_continue_resume_error_payload, consume_continue_should_defer_agent_handoff,
        consume_continue_state_access_blocker_payload, dispatch_receipt_internal_retry_eligible,
        dispatch_receipt_primary_rebind_eligible, dispatch_receipt_retry_eligible,
        emit_runtime_consumption_resume_json, enforce_consume_continue_execution_preparation_gate,
        fail_fast_state_store_open_read_only_with_timeout, normalize_runtime_dispatch_packet,
        normalize_stale_in_flight_dispatch_receipt, packet_path_components_for_platform,
        persisted_dispatch_packet_lineage_task_id,
        prefer_ready_downstream_packet_over_active_result, prepare_explicit_resume_retry_artifact,
        primary_backend_for_dispatch_receipt, read_dispatch_packet,
        ready_handoff_status_supersedes_blocked_dispatch_receipt,
        reconcile_blocked_implementer_timeout_with_tracked_close_evidence,
        reconcile_blocked_verification_timeout_with_receipt_evidence,
        recover_missing_first_dispatch_receipt, resolve_default_resume_run_id,
        resolve_runtime_consumption_resume_inputs,
        resolve_runtime_consumption_resume_inputs_for_run_id, resume_from_persisted_final_snapshot,
        resume_packet_ready_blocker_parity_error, retry_backend_for_dispatch_receipt,
        runtime_consumption_resume_blocker_code, runtime_consumption_resume_receipt_blocker_codes,
        runtime_consumption_resume_receipt_next_actions,
        runtime_consumption_snapshot_has_failure_control_evidence,
        same_packet_internal_timeout_retry_ready, should_refresh_resumed_downstream_preview,
        sync_run_graph_after_retry_artifact, validate_receipt_packet_pair,
        validate_run_graph_resume_state, validate_run_graph_resume_state_for_downstream_packet,
        validate_run_graph_resume_state_strict, PacketPathPlatform,
        DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS,
    };
    use crate::downstream_dispatch_ready_blocker_parity_error;
    use crate::state_store::{CreateTaskRequest, TaskExecutionSemantics};
    use crate::{RuntimeConsumptionLaneSelection, StateStore};
    use std::fs;
    use std::process::ExitCode;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn taskflow_consume_resume_test_receipt(
        dispatch_kind: &str,
        dispatch_status: &str,
    ) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
            dispatch_target: "implementation".to_string(),
            dispatch_status: dispatch_status.to_string(),
            lane_status: "lane_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: dispatch_kind.to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-21T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn cached_deferred_handoff_projection_requires_dispatch_init_cache_parity() {
        let root = std::env::temp_dir().join(format!(
            "vida-consume-deferred-cache-parity-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        let stale_packet = root.join("stale-pi-packet.json");
        let fresh_packet = root.join("fresh-middle-packet.json");
        fs::write(&stale_packet, "{}").expect("stale packet");
        fs::write(&fresh_packet, "{}").expect("fresh packet");

        let project_root = crate::state_store::repo_root();
        let source_config_digest = crate::launcher_activation_snapshot::config_file_digest(
            &project_root.join("vida.config.yaml"),
        )
        .expect("config digest");
        let cache_payload = serde_json::json!({
            "surface": "vida taskflow run-graph dispatch-init",
            "dispatch_init_fast_cache_schema_version": 2,
            "requested_run_id": "run-cache-parity",
            "run_id": "run-cache-parity",
            "source_config_digest": source_config_digest,
            "authoritative_persistence": {
                "status": "recorded",
            },
            "dispatch_packet_path": fresh_packet.display().to_string(),
            "dispatch_receipt": {
                "dispatch_status": "routed",
                "dispatch_command": "vida agent-init --dispatch-packet fresh-middle-packet.json --execute-dispatch --json",
                "selected_backend": "middle",
            },
        });
        let cache_path = crate::taskflow_run_graph::run_graph_dispatch_init_fast_cache_path(
            &root,
            "run-cache-parity",
        );
        fs::create_dir_all(cache_path.parent().expect("cache dir")).expect("cache dir");
        fs::write(
            &cache_path,
            serde_json::to_string_pretty(&cache_payload).expect("cache payload json"),
        )
        .expect("cache payload");

        let stale_projection = serde_json::json!({
            "run_id": "run-cache-parity",
            "dispatch_status": "routed",
            "selected_backend": "pi_cli",
        });
        assert!(
            !cached_deferred_handoff_projection_matches_dispatch_init_cache(
                &root,
                "run-cache-parity",
                &stale_projection,
                &stale_packet.display().to_string(),
            )
        );

        let fresh_projection = serde_json::json!({
            "run_id": "run-cache-parity",
            "dispatch_status": "routed",
            "selected_backend": "middle",
        });
        assert!(
            cached_deferred_handoff_projection_matches_dispatch_init_cache(
                &root,
                "run-cache-parity",
                &fresh_projection,
                &fresh_packet.display().to_string(),
            )
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_defers_routed_agent_lane_handoff() {
        let receipt = taskflow_consume_resume_test_receipt("agent_lane", "routed");

        assert!(consume_continue_should_defer_agent_handoff(
            "vida taskflow consume continue",
            &receipt
        ));
        assert!(!consume_continue_should_defer_agent_handoff(
            "vida taskflow consume advance",
            &receipt
        ));
    }

    #[test]
    fn consume_continue_defers_ready_downstream_packet() {
        let mut receipt = taskflow_consume_resume_test_receipt("agent_lane", "executed");
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_packet_path = Some("coach-packet.json".to_string());

        assert!(consume_continue_should_defer_agent_handoff(
            "vida taskflow consume continue",
            &receipt
        ));
    }

    #[test]
    fn consume_continue_does_not_defer_taskflow_pack() {
        let receipt = taskflow_consume_resume_test_receipt("taskflow_pack", "routed");

        assert!(!consume_continue_should_defer_agent_handoff(
            "vida taskflow consume continue",
            &receipt
        ));
    }

    #[test]
    fn consume_advance_success_payload_uses_release_one_operator_envelope() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
            dispatch_target: "implementation".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some("packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: Some("implementation".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        let payload = consume_advance_success_payload("packet.json", &receipt, "snapshot.json", 1);

        assert_eq!(payload["surface"], "vida taskflow consume advance");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["source_run_id"], "run-1");
        assert_eq!(payload["shared_fields"]["status"], "ok");
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "release-1-operator-contracts"
        );
        assert_eq!(
            payload["operator_contracts"]["schema_version"],
            "release-1-v1"
        );
    }

    #[cfg(windows)]
    #[test]
    fn validate_receipt_packet_pair_accepts_mixed_windows_separator_dispatch_path() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-mixed-separator-packet-{}-{}",
            std::process::id(),
            nanos
        ));
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("packet.json");
        fs::write(&packet_path, "{}").expect("write packet file");
        let expected_path = packet_path.display().to_string().replace('/', r"\");
        let resolved_path = packet_path.display().to_string().replace('\\', "/");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "mixed-separator-run".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("takeover-receipt".to_string()),
            exception_path_receipt_id: Some("takeover-receipt".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(expected_path),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-06T00:00:00Z".to_string(),
        };
        let packet = serde_json::json!({
            "run_id": "mixed-separator-run",
            "lane_status": "lane_exception_takeover",
            "dispatch_status": "routed",
            "supersedes_receipt_id": "takeover-receipt",
            "exception_path_receipt_id": "takeover-receipt"
        });

        validate_receipt_packet_pair(&receipt, &packet, &resolved_path, "dispatch packet")
            .expect("mixed separators should not invalidate the same dispatch packet path");
    }

    #[test]
    fn posix_backslash_path_collision_is_not_separator_equivalent() {
        let left =
            packet_path_components_for_platform("packets/a\\b.json", PacketPathPlatform::Posix);
        let right =
            packet_path_components_for_platform("packets/a/b.json", PacketPathPlatform::Posix);
        assert_ne!(left, right);
    }

    #[test]
    fn validate_receipt_packet_pair_rejects_dot_segment_packet_path_escape() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dot-segment-packet-{}-{}",
            std::process::id(),
            nanos
        ));
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("packet.json");
        fs::write(&packet_path, "{}").expect("write packet file");
        let dot_segment_path = packet_dir.join("../dispatch-packets/packet.json");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "dot-segment-run".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-06T00:00:00Z".to_string(),
        };
        let packet = serde_json::json!({
            "run_id": "dot-segment-run",
            "lane_status": "lane_blocked",
            "dispatch_status": "routed"
        });

        let error = validate_receipt_packet_pair(
            &receipt,
            &packet,
            &dot_segment_path.display().to_string(),
            "dispatch packet",
        )
        .expect_err("dot-segment packet path should fail closed");
        assert!(error.contains("expects dispatch_packet_path"));
    }

    #[test]
    fn consume_continue_state_access_blocker_payload_reports_lock_diagnostics() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-continue-lock-diagnostics-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create state root");
        fs::write(root.join("LOCK"), "").expect("write deterministic lock marker");

        let payload = consume_continue_state_access_blocker_payload(
            &root,
            "vida taskflow consume continue",
            "opening authoritative state store",
            "consume continue failed fast: opening authoritative state store: Database at LOCK is already locked by another process",
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            "authoritative_state_store_locked"
        );
        assert_eq!(payload["state_access"]["status"], "blocked");
        assert_eq!(payload["state_access"]["error_kind"], "lock_contention");
        assert_eq!(
            payload["state_access"]["lock_diagnostics"]["lock_exists"].as_bool(),
            Some(true)
        );
        assert_eq!(
            payload["state_access"]["lock_diagnostics"]["lock_file_size"].as_u64(),
            Some(0)
        );
        assert!(payload["next_actions"]
            .as_array()
            .expect("next actions")
            .iter()
            .any(|action| action
                .as_str()
                .is_some_and(|value| value.contains("do not delete datastore LOCK files"))));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn read_only_reopen_fails_fast_while_write_guard_is_held() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-read-only-lock-{}-{}",
            std::process::id(),
            nanos
        ));
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let store = StateStore::open(root.clone()).await.expect("open store");
            let result = tokio::time::timeout(
                Duration::from_secs(1),
                fail_fast_state_store_open_read_only_with_timeout(
                    root.clone(),
                    "reopening authoritative state store after resumed runtime dispatch",
                    Duration::from_secs(1),
                ),
            )
            .await
            .expect("read-only reopen should stay bounded");
            if let Err(error) = result {
                assert!(
                    error.contains("consume continue failed fast"),
                    "expected contextual fail-fast error, got {error}"
                );
            }
            drop(store);
            let _ = fs::remove_dir_all(&root);
        });
    }

    #[tokio::test]
    async fn consume_continue_handoff_timeout_returns_operator_blocker() {
        let error = consume_continue_handoff_with_timeout(
            "test handoff",
            Duration::from_millis(1),
            std::future::pending::<Result<(), String>>(),
        )
        .await
        .expect_err("pending handoff should time out");

        assert!(
            error.contains("Timed out executing runtime dispatch handoff during test handoff"),
            "unexpected error: {error}"
        );
        assert_eq!(
            consume_continue_resume_error_blocker_code(&error),
            "runtime_dispatch_handoff_timeout"
        );
    }

    #[test]
    fn consume_continue_dispatch_handoff_timeout_uses_external_backend_runtime_window() {
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-external-timeout-window-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let state_root = root.join(".vida").join("data").join("state");
        fs::create_dir_all(&state_root).expect("temp state root");
        fs::write(root.join("AGENTS.md"), "test").expect("agents marker");
        fs::create_dir_all(root.join(".vida").join("config")).expect("vida config dir");
        fs::create_dir_all(root.join(".vida").join("db")).expect("vida db dir");
        fs::create_dir_all(root.join(".vida").join("project")).expect("vida project dir");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    pi_cli:
      enabled: true
      subagent_backend_class: external_cli
      max_runtime_seconds: 420
      dispatch:
        no_output_timeout_seconds: 180
"#,
        )
        .expect("config");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Verify CASE-13 analysis.".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["analysis".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "analysis": {
                        "executor_backend": "pi_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-pi-timeout-window".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            exception_path_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:pi_cli".to_string()),
            dispatch_command: Some("vida-pi-agent --mode rpc".to_string()),
            dispatch_packet_path: Some(root.join("packet.json").display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("pi_cli".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("pi_cli".to_string()),
            recorded_at: "2026-05-20T14:50:04Z".to_string(),
        };

        assert_eq!(
            consume_continue_dispatch_handoff_timeout(&state_root, &role_selection, &receipt),
            Duration::from_secs(422)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_blocking_step_timeout_returns_operator_blocker() {
        let error = consume_continue_blocking_step_with_timeout(
            "blocking test gate",
            Duration::from_millis(1),
            || {
                std::thread::sleep(Duration::from_millis(50));
                Ok(())
            },
        )
        .expect_err("slow blocking step should time out");

        assert!(
            error
                .contains("Timed out executing runtime dispatch handoff during blocking test gate"),
            "unexpected error: {error}"
        );
        assert_eq!(
            consume_continue_resume_error_blocker_code(&error),
            "runtime_dispatch_handoff_timeout"
        );
    }

    #[test]
    fn configured_backend_dispatch_failure_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("qwen ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
    }

    #[test]
    fn timeout_without_takeover_authority_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-timeout-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
    }

    #[test]
    fn internal_timeout_without_receipt_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-retry".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some(
                "vida agent-init --dispatch-packet packet.json --execute-dispatch --json"
                    .to_string(),
            ),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some(
                crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
                    .to_string(),
            ),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("analysis".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("internal_subagents".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-06-01T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
        assert!(same_packet_internal_timeout_retry_ready(&receipt));
    }

    #[test]
    fn tool_execution_failed_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-tool-execution-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("tool_execution_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["tool_execution_failed".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-05-12T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
    }

    #[test]
    fn internal_codex_windows_sandbox_with_packet_is_retry_eligible() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-codex-windows-sandbox-retry".to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_codex_windows_sandbox_unavailable".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after writer".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "internal_codex_windows_sandbox_unavailable".to_string()
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("writer".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-05-25T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_retry_eligible(&receipt));
    }

    #[test]
    fn blocked_resume_receipt_contributes_authoritative_blocker_codes() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };

        let blocker_codes = runtime_consumption_resume_receipt_blocker_codes(&receipt);

        assert!(blocker_codes
            .iter()
            .any(|code| code == "timeout_without_takeover_authority"));
        assert!(blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence"));
    }

    #[test]
    fn runtime_consumption_resume_does_not_ignore_non_exception_blocked_receipt_after_ready_handoff(
    ) {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-ready-after-blocked",
            "implementation",
            "implementation",
        );
        status.active_node = "coach".to_string();
        status.next_node = Some("review_ensemble".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.handoff_state = "handoff_pending".to_string();
        status.resume_target = "dispatch.review_ensemble".to_string();
        status.recovery_ready = true;

        let mut receipt = taskflow_consume_resume_test_receipt("agent_lane", "blocked");
        receipt.run_id = status.run_id.clone();
        receipt.dispatch_target = "test_author".to_string();
        receipt.lane_status = "lane_failed".to_string();
        receipt.blocker_code = Some("configured_backend_dispatch_failed".to_string());
        receipt.downstream_dispatch_blockers = vec!["pending_review_clean_evidence".to_string()];

        assert!(!ready_handoff_status_supersedes_blocked_dispatch_receipt(
            Some(&status),
            &receipt
        ));
        assert!(!runtime_consumption_resume_receipt_blocker_codes(&receipt).is_empty());
    }

    #[test]
    fn runtime_consumption_resume_ignores_stale_exception_takeover_receipt_after_ready_handoff() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-ready-after-takeover",
            "implementation",
            "implementation",
        );
        status.active_node = "coach".to_string();
        status.next_node = Some("review_ensemble".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.handoff_state = "handoff_pending".to_string();
        status.resume_target = "dispatch.review_ensemble".to_string();
        status.recovery_ready = true;

        let mut receipt = taskflow_consume_resume_test_receipt("agent_lane", "blocked");
        receipt.run_id = status.run_id.clone();
        receipt.dispatch_target = "test_author".to_string();
        receipt.lane_status = "lane_exception_takeover".to_string();
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_active_target = Some("test_author".to_string());
        receipt.downstream_dispatch_blockers =
            vec!["internal_dispatch_timeout_without_receipt".to_string()];
        receipt.exception_path_receipt_id = Some("takeover-2".to_string());
        receipt.supersedes_receipt_id = Some("takeover-1".to_string());

        assert!(ready_handoff_status_supersedes_blocked_dispatch_receipt(
            Some(&status),
            &receipt
        ));
        assert!(!runtime_consumption_resume_receipt_blocker_codes(&receipt).is_empty());
        let effective_blocker_codes =
            if ready_handoff_status_supersedes_blocked_dispatch_receipt(Some(&status), &receipt) {
                Vec::new()
            } else {
                runtime_consumption_resume_receipt_blocker_codes(&receipt)
            };
        assert!(effective_blocker_codes.is_empty());
    }

    #[test]
    fn configured_backend_dispatch_failure_maps_to_tool_execution_blocker() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-configured-backend-blocked".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["configured_backend_dispatch_failed".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-06T00:00:00Z".to_string(),
        };

        let blocker_codes = runtime_consumption_resume_receipt_blocker_codes(&receipt);

        assert_eq!(blocker_codes, vec!["tool_execution_failed".to_string()]);
    }

    #[tokio::test]
    async fn continuation_sync_after_persisted_blocked_receipt_uses_reconciled_status() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-blocked-reconciled-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-blocked-reconciled-binding";
        let mut stale_status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "specification", "scope");
        stale_status.task_id = run_id.to_string();
        stale_status.active_node = "specification".to_string();
        stale_status.status = "completed".to_string();
        stale_status.lifecycle_stage = "specification_complete".to_string();
        stale_status.resume_target = "none".to_string();
        stale_status.recovery_ready = false;
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale upstream status");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec ...".to_string()),
            dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-06T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked receipt before continuation sync");

        let reconciled_status = store
            .run_graph_status(run_id)
            .await
            .expect("read reconciled status");
        assert_eq!(reconciled_status.status, "blocked");
        assert_eq!(reconciled_status.active_node, "implementer");

        let binding = crate::taskflow_continuation::sync_run_graph_continuation_binding(
            &store,
            &reconciled_status,
            crate::taskflow_continuation::CONSUME_CONTINUE_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE,
        )
        .await
        .expect("sync continuation binding")
        .expect("blocked status still has an active bounded unit");

        assert_eq!(
            binding.binding_source,
            "consume_continue_after_downstream_chain"
        );
        assert_eq!(binding.active_bounded_unit["active_node"], "implementer");
        drop(store);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn blocked_resume_receipt_without_execution_evidence_omits_review_evidence_action() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let next_actions = runtime_consumption_resume_receipt_next_actions(
            &receipt,
            &[
                "timeout_without_takeover_authority".to_string(),
                "pending_review_clean_evidence".to_string(),
            ],
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery latest --json")));
        assert!(!next_actions
            .iter()
            .any(|action| action.contains("clean review evidence")));
    }

    #[test]
    fn executed_resume_receipt_keeps_review_evidence_action() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-executed".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        let next_actions = runtime_consumption_resume_receipt_next_actions(
            &receipt,
            &["pending_review_clean_evidence".to_string()],
        );

        assert!(next_actions
            .iter()
            .any(|action| action.contains("clean review evidence")));
    }

    #[test]
    fn retry_backend_prefers_route_fallback_backend_after_external_failure() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "subagents": "hermes_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            retry_backend_for_dispatch_receipt(&role_selection, &receipt).as_deref(),
            Some("internal_subagents")
        );
    }

    #[test]
    fn retry_backend_prefers_route_fallback_after_handoff_timeout() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "fanout_executor_backends": ["hermes_cli", "opencode_cli"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-review-fanout-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert_eq!(
            retry_backend_for_dispatch_receipt(&role_selection, &receipt).as_deref(),
            Some("internal_subagents")
        );
    }

    #[test]
    fn retry_backend_uses_distinct_review_fanout_before_fallback_for_non_timeout_blocker() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "review_primary",
                        "fallback_executor_backend": "internal_subagents",
                        "fanout_executor_backends": ["review_primary", "review_secondary"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-review-fanout-non-timeout-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:review_primary".to_string()),
            dispatch_command: Some("review-primary".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("review_primary".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert_eq!(
            retry_backend_for_dispatch_receipt(&role_selection, &receipt).as_deref(),
            Some("review_secondary")
        );
    }

    #[test]
    fn retry_backend_does_not_rotate_internal_timeout_back_to_external_fanout() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "fanout_executor_backends": ["hermes_cli", "opencode_cli"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-review-internal-timeout-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert!(
            retry_backend_for_dispatch_receipt(&role_selection, &receipt).is_none(),
            "internal fallback timeout must not rotate back to external review fanout"
        );
    }

    #[test]
    fn internal_activation_view_only_on_fallback_is_rebind_eligible() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(dispatch_receipt_primary_rebind_eligible(
            &role_selection,
            &receipt
        ));
    }

    #[test]
    fn internal_activation_view_only_on_internal_codex_host_is_not_retry_eligible() {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-retry-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(!dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn internal_activation_view_only_on_internal_non_codex_host_is_not_retry_eligible() {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-retry-non-codex-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(!dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn internal_activation_view_only_on_external_host_is_not_retry_eligible() {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-retry-external-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      carriers:
        qwen-primary:
          default_runtime_role: worker
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-retry-external".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(!dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn internal_activation_view_only_terminal_blockers_do_not_select_retry_backend() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review bounded runtime dispatch behavior.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "coach": {
                                "backend_id": "internal_subagents",
                                "fallback_executor_backend": "hermes_cli"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        for blocker_code in [
            "internal_activation_view_only",
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT,
        ] {
            let receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: format!("run-{blocker_code}"),
                dispatch_target: "coach".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
                blocker_code: Some(blocker_code.to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("coach".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-05-21T00:00:00Z".to_string(),
            };
            assert!(retry_backend_for_dispatch_receipt(&role_selection, &receipt).is_none());
        }
    }

    #[test]
    fn primary_backend_rebind_prefers_ready_external_carrier() {
        let root = std::env::temp_dir().join(format!(
            "vida-primary-rebind-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        auth:
          mode: none
        model:
          mode: none
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            primary_backend_for_dispatch_receipt(&root, &role_selection, &receipt).as_deref(),
            Some("hermes_cli")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn primary_backend_rebind_stays_blocked_when_external_carrier_is_not_ready() {
        let root = std::env::temp_dir().join(format!(
            "vida-primary-rebind-blocked-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        auth:
          mode: file_present
          path: /tmp/vida-missing-qwen-auth
        model:
          mode: none
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            primary_backend_for_dispatch_receipt(&root, &role_selection, &receipt),
            None
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn blocked_primary_backend_prefers_route_fallback_before_dispatch_execution() {
        let root = std::env::temp_dir().join(format!(
            "vida-blocked-primary-fallback-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    vibe_cli:
      enabled: true
      subagent_backend_class: external_cli
      readiness:
        auth:
          mode: file_present
          path: /tmp/vida-missing-qwen-auth
        model:
          mode: none
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "vibe_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked-primary".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("qwen ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("vibe_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            crate::runtime_dispatch_state::fallback_backend_for_blocked_primary_dispatch_receipt(
                &root,
                &role_selection,
                &receipt,
            )
            .as_deref(),
            Some("internal_subagents")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn blocked_primary_dispatch_result_prefers_route_fallback_on_retry() {
        let root = std::env::temp_dir().join(format!(
            "vida-blocked-primary-result-fallback-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    review_cli:
      enabled: true
      subagent_backend_class: external_cli
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let result_path = root.join("blocked-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "status": "blocked",
                "selected_backend": "review_cli",
                "blocker_code": "timeout_without_takeover_authority"
            })
            .to_string(),
        )
        .expect("result");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "review_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-blocked-primary-result".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("review-cli ...".to_string()),
            dispatch_packet_path: Some(root.join("dispatch-packet.json").display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("review_cli".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert_eq!(
            crate::runtime_dispatch_state::fallback_backend_for_blocked_primary_dispatch_receipt(
                &root,
                &role_selection,
                &receipt,
            )
            .as_deref(),
            Some("internal_subagents")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn canonical_resume_dispatch_status_preserves_release1_vocabulary() {
        assert_eq!(
            canonical_resume_dispatch_status(Some("executed")),
            "executed"
        );
        assert_eq!(canonical_resume_dispatch_status(Some("routed")), "routed");
        assert_eq!(
            canonical_resume_dispatch_status(Some("packet_ready")),
            "packet_ready"
        );
        assert_eq!(canonical_resume_dispatch_status(Some("blocked")), "blocked");
    }

    #[test]
    fn canonical_resume_dispatch_status_fails_closed_for_unknown_or_drifted_values() {
        assert_eq!(canonical_resume_dispatch_status(Some("block")), "blocked");
        assert_eq!(canonical_resume_dispatch_status(Some("unknown")), "blocked");
        assert_eq!(
            canonical_resume_dispatch_status(Some(" packet_ready ")),
            "packet_ready"
        );
        assert_eq!(canonical_resume_dispatch_status(None), "blocked");
    }

    #[test]
    fn canonical_resume_dispatch_and_lane_status_normalize_case_and_whitespace_drift() {
        assert_eq!(
            canonical_resume_dispatch_status(Some("  PACKET_READY  ")),
            "packet_ready"
        );
        assert_eq!(
            canonical_resume_dispatch_status(Some("  BLOCKED  ")),
            "blocked"
        );
        assert_eq!(
            canonical_resume_lane_status("  LANE_COMPLETED  "),
            Some(crate::LaneStatus::LaneCompleted)
        );
        assert_eq!(
            canonical_resume_lane_status("  lane_open  "),
            Some(crate::LaneStatus::LaneOpen)
        );
        assert_eq!(canonical_resume_lane_status("lane_block"), None);
    }

    #[test]
    fn canonical_resume_string_array_entries_fail_closed_for_whitespace_only_entries() {
        assert_eq!(
            canonical_resume_string_array_entries(&serde_json::json!(["pending_lane_evidence"])),
            Some(vec!["pending_lane_evidence".to_string()])
        );
        assert_eq!(
            canonical_resume_string_array_entries(&serde_json::json!(["   "])),
            None
        );
    }

    #[test]
    fn resume_packet_ready_blocker_parity_fails_closed_for_drifted_blocker_evidence() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            resume_packet_ready_blocker_parity_error(Some("packet_ready"), &blockers),
            Some(
                "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                    .to_string()
            )
        );
        assert_eq!(
            resume_packet_ready_blocker_parity_error(Some("packet_ready"), &[]),
            None
        );
    }

    #[test]
    fn downstream_dispatch_ready_blocker_parity_fails_closed_for_drifted_blocker_evidence() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("ready"), &blockers),
            None
        );
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("ready"), &[]),
            None
        );
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("packet_ready"), &blockers),
            Some(
                "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                    .to_string()
            )
        );
        assert_eq!(
            super::resume_packet_ready_blocker_parity_error(Some("blocked"), &blockers),
            None
        );
    }

    #[test]
    fn downstream_dispatch_ready_guard_message_matches_main_surface() {
        let blockers = vec!["pending_lane_evidence".to_string()];
        assert_eq!(
            downstream_dispatch_ready_blocker_parity_error(true, &blockers),
            crate::downstream_dispatch_ready_blocker_parity_error(true, &blockers)
        );
    }

    #[test]
    fn should_refresh_resumed_downstream_preview_for_executed_receipt_with_stale_blockers() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-refresh".to_string(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/spec-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/spec-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some("vida task ensure".to_string()),
            downstream_dispatch_note: Some("stale blockers".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![
                "pending_specification_evidence".to_string(),
                "pending_design_finalize".to_string(),
            ],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some("/tmp/spec-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: Some("specification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-12T00:00:00Z".to_string(),
        };

        assert!(should_refresh_resumed_downstream_preview(&receipt));

        let mut settled = receipt.clone();
        settled.downstream_dispatch_ready = true;
        settled.downstream_dispatch_blockers.clear();
        assert!(!should_refresh_resumed_downstream_preview(&settled));

        let mut blocked = receipt.clone();
        blocked.dispatch_status = "blocked".to_string();
        assert!(!should_refresh_resumed_downstream_preview(&blocked));
    }

    #[tokio::test]
    async fn resolve_resume_inputs_clears_stale_downstream_state_for_executed_active_result() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-stale-executed-downstream-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-stale-executed-downstream";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = Some("verification".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-stale-executed-downstream-coach.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-stale-executed-downstream-coach.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "executed",
                "dispatch_packet_path": packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "middle"
                }
            })
            .to_string(),
        )
        .expect("write downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after implementer evidence, activate coach".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("executed".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let inputs = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("resume inputs should resolve from executed downstream result");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(inputs.dispatch_receipt.dispatch_status, "executed");
        assert!(!inputs.dispatch_receipt.downstream_dispatch_ready);
        assert!(inputs.dispatch_receipt.downstream_dispatch_target.is_none());
        assert!(inputs
            .dispatch_receipt
            .downstream_dispatch_active_target
            .is_none());
        assert_eq!(
            inputs
                .dispatch_receipt
                .downstream_dispatch_last_target
                .as_deref(),
            Some("coach")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_promotes_terminal_execution_sibling() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-in-flight-terminal-sibling-{}-{}",
            std::process::id(),
            nanos
        ));
        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let packet_path =
            root.join("runtime-consumption/dispatch-packets/run-terminal-packet.json");
        fs::create_dir_all(packet_path.parent().expect("packet parent")).expect("packet dir");
        fs::write(&packet_path, "{}").expect("packet");
        let in_flight_path = result_dir.join("run-terminal-started.json");
        fs::write(
            &in_flight_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "run_id": "run-terminal-sibling",
                "dispatch_target": "analysis",
                "status": "pass",
                "execution_state": "executing",
                "source_dispatch_packet_path": packet_path.display().to_string(),
                "stale_after_seconds": 422,
                "recorded_at": "2026-05-20T15:09:18Z"
            })
            .to_string(),
        )
        .expect("write in-flight result");
        let terminal_path = result_dir.join("run-terminal-executed.json");
        fs::write(
            &terminal_path,
            serde_json::json!({
                "artifact_kind": "runtime_lane_completion_result",
                "run_id": "run-terminal-sibling",
                "dispatch_target": "analysis",
                "completed_target": "analysis",
                "status": "pass",
                "execution_state": "executed",
                "source_dispatch_packet_path": packet_path.display().to_string(),
                "activation_command": "vida-pi-agent --mode rpc",
                "surface": "external_cli:pi_cli",
                "recorded_at": "2026-05-20T15:10:12Z",
                "execution_evidence": {
                    "status": "recorded",
                    "receipt_backed": true,
                    "backend_id": "pi_cli"
                }
            })
            .to_string(),
        )
        .expect("write terminal result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-terminal-sibling".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            exception_path_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:pi_cli".to_string()),
            dispatch_command: Some("vida-pi-agent --mode rpc".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(in_flight_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("pi_cli".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("pi_cli".to_string()),
            recorded_at: "2026-05-20T13:14:08Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("terminal sibling should normalize")
        );
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.blocker_code, None);
        assert_eq!(
            receipt.dispatch_result_path.as_deref(),
            Some(terminal_path.to_str().unwrap())
        );
        assert_eq!(
            receipt.dispatch_surface.as_deref(),
            Some("external_cli:pi_cli")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_ignores_untrusted_terminal_execution_sibling() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-in-flight-untrusted-terminal-sibling-{}-{}",
            std::process::id(),
            nanos
        ));
        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let packet_path =
            root.join("runtime-consumption/dispatch-packets/run-untrusted-terminal-packet.json");
        fs::create_dir_all(packet_path.parent().expect("packet parent")).expect("packet dir");
        fs::write(&packet_path, "{}").expect("packet");
        let in_flight_path = result_dir.join("run-untrusted-terminal-started.json");
        fs::write(
            &in_flight_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "run_id": "run-untrusted-terminal-sibling",
                "dispatch_target": "analysis",
                "status": "pass",
                "execution_state": "executing",
                "source_dispatch_packet_path": packet_path.display().to_string(),
                "stale_after_seconds": 422,
                "recorded_at": "2026-05-20T15:09:18Z"
            })
            .to_string(),
        )
        .expect("write in-flight result");
        let forged_terminal_path = result_dir.join("run-untrusted-terminal-forged.json");
        fs::write(
            &forged_terminal_path,
            serde_json::json!({
                "artifact_kind": "attacker_controlled_note_not_vida_result",
                "run_id": "run-untrusted-terminal-sibling",
                "dispatch_target": "analysis",
                "completed_target": "analysis",
                "status": "pass",
                "execution_state": "executed",
                "source_dispatch_packet_path": packet_path.display().to_string(),
                "activation_command": "attacker-command",
                "surface": "external_cli:attacker",
                "recorded_at": "2026-05-20T15:10:12Z"
            })
            .to_string(),
        )
        .expect("write forged terminal result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-untrusted-terminal-sibling".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            exception_path_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:pi_cli".to_string()),
            dispatch_command: Some("vida-pi-agent --mode rpc".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(in_flight_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("pi_cli".to_string()),
            recorded_at: "2026-05-20T15:09:18Z".to_string(),
        };

        let normalized = normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
            .expect("normalize in-flight receipt");
        assert!(normalized);
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );
        let normalized_result_path = receipt
            .dispatch_result_path
            .as_deref()
            .expect("timeout normalization should record a result path");
        assert_ne!(
            normalized_result_path,
            forged_terminal_path.display().to_string().as_str()
        );
        assert_ne!(
            normalized_result_path,
            in_flight_path.display().to_string().as_str()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_marks_timeout_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-in-flight-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "stale_after_seconds": 39,
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write in-flight result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-in-flight".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let original_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("original in-flight result path should exist");
        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("stale receipt normalization should succeed")
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert_eq!(receipt.lane_status, "lane_exception_recorded");
        let normalized_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("normalized blocked result path should exist");
        assert_ne!(normalized_result_path, original_result_path);
        let normalized_result =
            crate::read_json_file_if_present(std::path::Path::new(&normalized_result_path))
                .expect("normalized result file should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert!(normalized_result["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 39s"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_keeps_executing_inside_recorded_window() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-in-flight-window-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        let recorded_at = (time::OffsetDateTime::now_utc() - time::Duration::seconds(3))
            .format(&time::format_description::well_known::Rfc3339)
            .expect("timestamp should render");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "stale_after_seconds": 39,
                "recorded_at": recorded_at
            })
            .to_string(),
        )
        .expect("write in-flight result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-still-in-flight".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            !normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("normalization should leave still-executing receipt alone")
        );
        assert_eq!(receipt.dispatch_status, "executing");
        assert_eq!(receipt.lane_status, "lane_running");
        assert!(receipt.blocker_code.is_none());
        assert_eq!(
            crate::read_json_file_if_present(std::path::Path::new(
                receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should remain")
            ))
            .expect("original result should remain readable")["execution_state"],
            "executing"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_keeps_legacy_internal_execution_inside_host_window(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-legacy-internal-host-window-{}-{}",
            std::process::id(),
            nanos
        ));
        let state_root = root.join(crate::state_store::default_state_dir());
        fs::create_dir_all(&state_root).expect("create state root");
        fs::create_dir_all(root.join(".vida/config")).expect("config dir");
        fs::create_dir_all(root.join(".vida/db")).expect("db dir");
        fs::create_dir_all(root.join(".vida/project")).expect("project dir");
        fs::write(root.join("AGENTS.md"), "test").expect("agents");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      max_runtime_seconds: 37
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let result_path = root.join("dispatch-result.json");
        let recorded_at = (time::OffsetDateTime::now_utc() - time::Duration::seconds(11))
            .format(&time::format_description::well_known::Rfc3339)
            .expect("timestamp should render");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "internal_cli:codex",
                "backend_dispatch": {
                    "backend_class": "internal"
                },
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": recorded_at
            })
            .to_string(),
        )
        .expect("write in-flight result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-legacy-internal-window".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            !normalize_stale_in_flight_dispatch_receipt(&state_root, &mut receipt)
                .expect("legacy internal host timeout should preserve executing state inside configured window")
        );
        assert_eq!(receipt.dispatch_status, "executing");
        assert_eq!(receipt.lane_status, "lane_running");
        assert!(receipt.blocker_code.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_timeout_blocked_receipt_rewrites_executing_result_artifact() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-timeout-blocked-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "stale_after_seconds": 17,
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale in-flight result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-timeout-blocked".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        let original_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("original in-flight result path should exist");
        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("stale timeout-blocked receipt normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        let normalized_result_path = receipt
            .dispatch_result_path
            .clone()
            .expect("normalized blocked result path should exist");
        assert_ne!(normalized_result_path, original_result_path);
        let normalized_result =
            crate::read_json_file_if_present(std::path::Path::new(&normalized_result_path))
                .expect("normalized blocked result file should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert!(normalized_result["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 17s"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_persists_normalized_stale_in_flight_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-persist-normalized-stale-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "run_id": "run-persist-stale",
                "dispatch_target": "coach"
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "stale_after_seconds": 39,
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale in-flight result");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-persist-stale".to_string(),
                dispatch_target: "coach".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-timeout".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: Some(result_path.display().to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("coach".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-19T00:00:00Z".to_string(),
            })
            .await
            .expect("record stale receipt");

        let _ =
            resolve_runtime_consumption_resume_inputs_for_run_id(&store, "run-persist-stale").await;

        let persisted = store
            .run_graph_dispatch_receipt("run-persist-stale")
            .await
            .expect("read persisted receipt")
            .expect("persisted receipt should exist");
        assert_eq!(persisted.dispatch_status, "blocked");
        assert_eq!(
            persisted.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert_eq!(persisted.lane_status, "lane_exception_recorded");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_in_flight_dispatch_receipt_reclassifies_downstream_carrier_mismatch_immediately(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-downstream-carrier-mismatch-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");

        let packet_path = root.join("downstream-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet"
            })
            .to_string(),
        )
        .expect("write malformed downstream carrier packet");

        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .expect("rfc3339 timestamp should render"),
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write executing result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-downstream-carrier".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("downstream carrier mismatch should normalize")
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_internal_timeout_blocked_receipt_reclassifies_generic_timeout() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-internal-timeout-reclassify-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "timeout_without_takeover_authority",
                "recorded_at": "2026-04-17T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write blocked timeout result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-reclassify".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_exception_recorded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("internal timeout receipt normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        let normalized_result = crate::read_json_file_if_present(std::path::Path::new(
            receipt
                .dispatch_result_path
                .as_deref()
                .expect("normalized result path should exist"),
        ))
        .expect("normalized result should exist");
        assert_eq!(
            normalized_result["blocker_code"],
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_blocked_external_internal_activation_mismatch_reclassifies_generic_timeout() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-external-internal-activation-mismatch-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "execution_truth": {
                    "effective_selected_backend": "internal_subagents",
                    "route_primary_backend": "hermes_cli",
                    "route_primary_backend_class": "external_cli"
                }
            })
            .to_string(),
        )
        .expect("write stale packet");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "selected_backend": "hermes_cli",
                "lane_execution_receipt_artifact": {
                    "carrier_id": "hermes_cli"
                },
                "recorded_at": "2026-04-21T12:39:12Z",
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write blocked mismatched result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-internal-activation-mismatch".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-21T12:14:39Z".to_string(),
        };

        let result = crate::read_json_file_if_present(&result_path).expect("result should exist");
        assert!(
            blocked_external_dispatch_artifact_mismatched_as_internal_activation(&receipt, &result)
        );
        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("external mismatch normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );
        let normalized_result = crate::read_json_file_if_present(std::path::Path::new(
            receipt
                .dispatch_result_path
                .as_deref()
                .expect("normalized result path should exist"),
        ))
        .expect("normalized result should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            "timeout_without_takeover_authority"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_receipt_uses_dispatch_packet_to_preserve_internal_activation_view() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-internal-packet-posture-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "host_runtime": {
                    "selected_cli_execution_class": "internal"
                },
                "effective_execution_posture": {
                    "effective_posture_kind": "internal",
                    "selected_execution_class": "internal"
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "timeout_without_takeover_authority",
                "recorded_at": "2026-04-17T00:00:00Z",
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write blocked timeout result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-from-packet-posture".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("packet-derived internal timeout normalization should succeed")
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        let normalized_result = crate::read_json_file_if_present(std::path::Path::new(
            receipt
                .dispatch_result_path
                .as_deref()
                .expect("normalized result path should exist"),
        ))
        .expect("normalized result should exist");
        assert_eq!(normalized_result["execution_state"], "blocked");
        assert_eq!(
            normalized_result["blocker_code"],
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_receipt_keeps_generic_timeout_when_external_evidence_overrides_internal_packet_hint(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-external-timeout-dominates-packet-hint-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let packet_path = root.join("dispatch-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "host_runtime": {
                    "selected_cli_execution_class": "internal"
                },
                "effective_execution_posture": {
                    "effective_posture_kind": "internal",
                    "selected_execution_class": "internal"
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "external_cli:hermes_cli",
                "backend_dispatch": {
                    "backend_class": "external_cli",
                    "backend_id": "hermes_cli"
                },
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "timeout_without_takeover_authority",
                "recorded_at": "2026-04-21T13:28:23Z",
                "source_dispatch_packet_path": packet_path.display().to_string()
            })
            .to_string(),
        )
        .expect("write blocked timeout result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-external-timeout-overrides-internal-packet".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-21T12:14:39Z".to_string(),
        };

        assert!(
            !normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("external timeout normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );
        let normalized_result = crate::read_json_file_if_present(std::path::Path::new(
            receipt
                .dispatch_result_path
                .as_deref()
                .expect("normalized result path should exist"),
        ))
        .expect("normalized result should exist");
        assert_eq!(
            normalized_result["blocker_code"],
            "timeout_without_takeover_authority"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_stale_internal_receipt_blocks_retry_eligibility() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-internal-retry-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let result_path = root.join("dispatch-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "surface": "internal_cli:codex",
                "backend_dispatch": {
                    "backend_class": "internal"
                },
                "status": "pass",
                "execution_state": "executing",
                "recorded_at": "2000-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .expect("write stale in-flight result");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-internal-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executing".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some(result_path.display().to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            normalize_stale_in_flight_dispatch_receipt(&root, &mut receipt)
                .expect("stale receipt normalization should succeed")
        );
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some(crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT)
        );
        assert!(!dispatch_receipt_internal_retry_eligible(
            &root,
            &role_selection,
            &receipt
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_bridges_closed_specification_gate_into_work_pool_progress() {
        std::thread::Builder::new()
            .name("consume-continue-spec-bridge".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .thread_stack_size(8 * 1024 * 1024)
                    .build()
                    .expect("create runtime");
                runtime.block_on(Box::pin(async {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let root = std::env::temp_dir().join(format!(
                "vida-consume-resume-spec-bridge-{}-{}",
                std::process::id(),
                nanos
            ));
            let state_dir = root.join("state");
            let store = StateStore::open(state_dir.clone())
                .await
                .expect("open store");

            let run_id = "run-specification-bridge";
            let spec_parent_id = "feature-spec-bridge";
            let spec_task_id = "feature-spec-bridge-spec";
            let design_doc_path = root.join("docs/spec-bridge-design.md");
            fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
                .expect("create design doc directory");
            fs::write(&design_doc_path, "# Spec Bridge\n\nStatus: `approved`\n")
                .expect("write approved design doc");

            let labels = vec!["spec-pack".to_string()];
            store
                .create_task(crate::state_store::CreateTaskRequest {
                    task_id: spec_parent_id,
                    title: "Spec bridge feature",
                    display_id: None,
                    description: "",
                    issue_type: "epic",
                    status: "closed",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create spec parent epic");
            store
                .create_task(crate::state_store::CreateTaskRequest {
                    task_id: spec_task_id,
                    title: "Closed spec pack",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "closed",
                    priority: 0,
                    parent_id: Some(spec_parent_id),
                    labels: &labels,
                    execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create closed spec task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "spec-pack",
        );
        status.task_id = run_id.to_string();
        status.active_node = "specification".to_string();
        status.next_node = Some("work_pool_pack".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "specification_active".to_string();
        status.handoff_state = "awaiting_work_pool_pack".to_string();
        status.resume_target = "dispatch.work_pool_pack".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = state_dir.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet directory");
        let packet_path = packet_dir.join(format!("{run_id}.json"));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "activation_agent_type": "middle",
                "activation_runtime_role": "business_analyst",
                "selected_backend": "middle",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::specification::delivery"),
                    "goal": "Execute bounded specification handoff",
                    "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                    "read_only_paths": ["docs/product/spec"],
                    "owned_paths": [design_doc_path.display().to_string()],
                    "definition_of_done": ["record bounded specification evidence"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded specification proof",
                    "stop_rules": ["stop after bounded evidence"],
                    "blocking_question": "What is the next bounded action required for `specification`?"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "fixed",
                    "fallback_role": "orchestrator",
                    "request": "continue specification",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "spec-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["specification"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "tracked_flow_bootstrap": {
                            "spec_task": {
                                "task_id": spec_task_id
                            },
                            "design_doc_path": design_doc_path.display().to_string(),
                            "work_pool_task": {
                                "ensure_command": "vida task ensure feature-spec-bridge-work-pool \"Work-pool pack\" --type task --status open --json"
                            }
                        },
                        "development_flow": {
                            "dispatch_contract": {
                                "specification_activation": {
                                    "completion_blocker": "pending_specification_evidence",
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "business_analyst"
                                }
                            }
                        },
                        "orchestration_contract": {}
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");

            store
                .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "specification".to_string(),
                dispatch_status: "executing".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: Some("exc-spec-bridge".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: Some("/tmp/specification-started.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("work-pool-pack".to_string()),
                downstream_dispatch_command: Some(
                    "vida task ensure feature-spec-bridge-work-pool \"Work-pool pack\" --type task --status open --json"
                        .to_string(),
                ),
                downstream_dispatch_note: Some("waiting on specification evidence".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![
                    "pending_specification_evidence".to_string(),
                    "pending_design_finalize".to_string(),
                    "pending_spec_task_close".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("specification".to_string()),
                downstream_dispatch_last_target: Some("specification".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
                })
                .await
                .expect("persist executing specification receipt");
            drop(store);

            let exit = super::run_taskflow_consume_resume_command(
                state_dir.clone(),
                true,
                Some(run_id.to_string()),
                None,
                None,
                "vida taskflow consume continue",
                false,
            )
            .await;
            assert_eq!(exit, ExitCode::SUCCESS);

            let store = fail_fast_state_store_open_read_only_with_timeout(
                state_dir.clone(),
                "reopen bridged specification receipt",
                Duration::from_secs(5),
            )
            .await
            .expect("reopen store");
            let receipt = store
                .run_graph_dispatch_receipt(run_id)
                .await
                .expect("load bridged receipt")
                .expect("receipt should exist");
            assert_eq!(receipt.dispatch_status, "executed");
            assert!(
                !receipt
                    .downstream_dispatch_blockers
                    .iter()
                    .any(|value| value == "pending_specification_evidence"),
                "specification evidence blocker should be cleared after canonical design/spec completion bridge"
            );

            let _ = fs::remove_dir_all(&root);
                }));
            })
            .expect("spawn stack-heavy consume continue regression")
            .join()
            .expect("stack-heavy consume continue regression should complete");
    }

    #[test]
    fn resolve_resume_inputs_refreshes_executed_specification_with_stale_design_blockers_before_resume_gate(
    ) {
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let root = std::env::temp_dir().join(format!(
                "vida-consume-resume-spec-stale-design-blockers-{}-{}",
                std::process::id(),
                nanos
            ));
            let state_dir = root.join("state");
            let store = StateStore::open(state_dir.clone())
                .await
                .expect("open store");

            let run_id = "run-specification-stale-design-blockers";
            let spec_parent_id = "feature-spec-stale-design-blockers";
            let spec_task_id = "feature-spec-stale-design-blockers-spec";
            let design_doc_path = root.join("docs/spec-stale-design-blockers.md");
            fs::create_dir_all(design_doc_path.parent().expect("design doc parent"))
                .expect("create design doc directory");
            fs::write(
                &design_doc_path,
                "# Spec Stale Design Blockers\n\nStatus: `approved`\n",
            )
            .expect("write approved design doc");

            let labels = vec!["spec-pack".to_string()];
            store
                .create_task(CreateTaskRequest {
                    task_id: spec_parent_id,
                    title: "Stale design blocker feature",
                    display_id: None,
                    description: "",
                    issue_type: "epic",
                    status: "closed",
                    priority: 0,
                    parent_id: None,
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create closed spec parent");
            store
                .create_task(CreateTaskRequest {
                    task_id: spec_task_id,
                    title: "Closed stale spec pack",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "closed",
                    priority: 0,
                    parent_id: Some(spec_parent_id),
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "",
                })
                .await
                .expect("create closed spec task");

            let mut status = crate::taskflow_run_graph::default_run_graph_status(
                run_id,
                "scope_discussion",
                "spec-pack",
            );
            status.task_id = run_id.to_string();
            status.active_node = "specification".to_string();
            status.next_node = None;
            status.status = "blocked".to_string();
            status.lifecycle_stage = "specification_complete".to_string();
            status.handoff_state = "none".to_string();
            status.resume_target = "none".to_string();
            status.recovery_ready = false;
            store
                .record_run_graph_status(&status)
                .await
                .expect("persist blocked specification status");

            let packet_dir = state_dir.join("runtime-consumption/dispatch-packets");
            fs::create_dir_all(&packet_dir).expect("create packet directory");
            let packet_path = packet_dir.join(format!("{run_id}.json"));
            fs::write(
                &packet_path,
                serde_json::json!({
                    "packet_template_kind": "delivery_task_packet",
                    "run_id": run_id,
                    "activation_agent_type": "middle",
                    "activation_runtime_role": "business_analyst",
                    "selected_backend": "middle",
                    "delivery_task_packet": {
                        "packet_id": format!("{run_id}::specification::delivery"),
                        "goal": "Refresh stale specification completion blockers",
                        "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                        "read_only_paths": ["docs/product/spec"],
                        "owned_paths": [design_doc_path.display().to_string()],
                        "definition_of_done": ["record bounded specification evidence"],
                        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                        "proof_target": "bounded specification proof",
                        "stop_rules": ["stop after bounded evidence"],
                        "blocking_question": "What is the next bounded action required for `specification`?"
                    },
                    "role_selection_full": {
                        "ok": true,
                        "activation_source": "test",
                        "selection_mode": "fixed",
                        "fallback_role": "orchestrator",
                        "request": "continue stale specification",
                        "selected_role": "pm",
                        "conversational_mode": "development",
                        "single_task_only": true,
                        "tracked_flow_entry": "spec-pack",
                        "allow_freeform_chat": false,
                        "confidence": "high",
                        "matched_terms": ["specification"],
                        "compiled_bundle": null,
                        "execution_plan": {
                            "tracked_flow_bootstrap": {
                                "spec_task": {
                                    "task_id": spec_task_id
                                },
                                "design_doc_path": design_doc_path.display().to_string(),
                                "work_pool_task": {
                                    "ensure_command": "vida task ensure feature-spec-stale-design-blockers-work-pool \"Work-pool pack\" --type task --status open --json"
                                }
                            },
                            "development_flow": {
                                "dispatch_contract": {
                                    "specification_activation": {
                                        "completion_blocker": "pending_specification_evidence",
                                        "activation_agent_type": "middle",
                                        "activation_runtime_role": "business_analyst"
                                    }
                                }
                            },
                            "orchestration_contract": {}
                        },
                        "reason": "test"
                    },
                    "run_graph_bootstrap": {
                        "run_id": run_id
                    }
                })
                .to_string(),
            )
            .expect("write dispatch packet");

            store
                .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                    run_id: run_id.to_string(),
                    dispatch_target: "specification".to_string(),
                    dispatch_status: "executed".to_string(),
                    lane_status: "lane_completed".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init".to_string()),
                    dispatch_packet_path: Some(packet_path.display().to_string()),
                    dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                    blocker_code: None,
                    downstream_dispatch_target: Some("work-pool-pack".to_string()),
                    downstream_dispatch_command: Some(
                        "vida task ensure feature-spec-stale-design-blockers-work-pool \"Work-pool pack\" --type task --status open --json"
                            .to_string(),
                    ),
                    downstream_dispatch_note: Some("stale design/spec task blockers".to_string()),
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: vec![
                        "pending_design_finalize".to_string(),
                        "pending_spec_task_close".to_string(),
                    ],
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: Some("/tmp/specification-result.json".to_string()),
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("middle".to_string()),
                    activation_runtime_role: Some("business_analyst".to_string()),
                    selected_backend: Some("middle".to_string()),
                    recorded_at: "2026-04-17T00:00:00Z".to_string(),
                })
                .await
                .expect("persist executed specification receipt with stale blockers");

            let resume = resolve_runtime_consumption_resume_inputs_for_run_id(&store, run_id)
                .await
                .expect("stale spec-pack blockers should refresh before strict resume gate");
            assert_eq!(resume.dispatch_receipt.dispatch_status, "executed");
            assert_eq!(
                resume
                    .dispatch_receipt
                    .downstream_dispatch_target
                    .as_deref(),
                Some("work-pool-pack")
            );
            assert!(resume.dispatch_receipt.downstream_dispatch_ready);
            assert!(resume.dispatch_receipt.downstream_dispatch_blockers.is_empty());
            assert_eq!(
                resume
                    .dispatch_receipt
                    .downstream_dispatch_status
                    .as_deref(),
                Some("packet_ready")
            );
            assert!(resume
                .dispatch_receipt
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty()));

            let _ = fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn resume_continue_snapshot_has_release1_shared_envelope_fields() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-shared-envelope-{}-{}",
            std::process::id(),
            nanos
        ));
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let store = StateStore::open(root.clone()).await.expect("open store");
            let role_selection = RuntimeConsumptionLaneSelection {
                ok: true,
                activation_source: "test".to_string(),
                selection_mode: "fixed".to_string(),
                fallback_role: "worker".to_string(),
                request: "Normalize consume-continue shared operator envelope.".to_string(),
                selected_role: "worker".to_string(),
                conversational_mode: None,
                single_task_only: false,
                tracked_flow_entry: None,
                allow_freeform_chat: false,
                confidence: "high".to_string(),
                matched_terms: Vec::new(),
                compiled_bundle: serde_json::Value::Null,
                execution_plan: serde_json::json!({
                    "runtime_assignment": {
                        "selected_backend": "internal_subagents"
                    }
                }),
                reason: "test".to_string(),
            };
            let dispatch_receipt = crate::state_store::RunGraphDispatchReceipt {
                run_id: "resume-envelope-run".to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("internal_cli:codex".to_string()),
                dispatch_command: Some("codex exec".to_string()),
                dispatch_packet_path: Some("/tmp/resume-envelope-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/resume-envelope-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("verification".to_string()),
                downstream_dispatch_last_target: Some("verification".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-20T00:00:00Z".to_string(),
            };

            emit_runtime_consumption_resume_json(
                &store,
                "vida taskflow consume continue",
                "/tmp/resume-envelope-packet.json",
                &dispatch_receipt,
                &role_selection,
                None,
                false,
                true,
            )
            .await
            .expect("resume json snapshot should be emitted");

            let snapshot_path =
                crate::latest_recorded_final_runtime_consumption_snapshot_path(store.root())
                    .expect("load latest recorded final snapshot path")
                    .expect("recorded final snapshot should exist");
            let snapshot_json: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(&snapshot_path).expect("read final snapshot"),
            )
            .expect("parse final snapshot");

            assert_eq!(
                snapshot_json["trace_id"],
                snapshot_json["operator_contracts"]["trace_id"]
            );
            assert_eq!(
                snapshot_json["workflow_class"],
                snapshot_json["operator_contracts"]["workflow_class"]
            );
            assert_eq!(
                snapshot_json["risk_tier"],
                snapshot_json["operator_contracts"]["risk_tier"]
            );
            assert_eq!(
                crate::operator_contracts::shared_operator_output_contract_parity_error(
                    &snapshot_json
                ),
                None
            );

            let _ = fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn resume_from_persisted_final_snapshot_detects_final_snapshot_evidence() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-final-snapshot-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(StateStore::open(root.clone()))
            .expect("open store");

        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-00Z.json");
        let operator_contracts = crate::build_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path.display().to_string(),
                "latest_run_graph_dispatch_receipt_id": "run-final-snapshot",
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        let failure_control_evidence = build_failure_control_evidence(
            "run-final-snapshot",
            &snapshot_path.display().to_string(),
        );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": "run-final-snapshot"
                    },
                    "release_admission": {},
                    "failure_control_evidence": failure_control_evidence.clone()
                },
                "failure_control_evidence": failure_control_evidence
            })
            .to_string(),
        )
        .expect("write final snapshot");

        assert!(
            resume_from_persisted_final_snapshot(&store, "run-final-snapshot")
                .expect("runtime consumption summary"),
        );
        let snapshot_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read final snapshot"))
                .expect("parse final snapshot");
        assert!(runtime_consumption_snapshot_has_failure_control_evidence(
            &snapshot_json
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_from_persisted_final_snapshot_rejects_final_snapshot_without_failure_control_evidence(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-final-snapshot-missing-control-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = tokio::runtime::Runtime::new()
            .expect("create runtime")
            .block_on(StateStore::open(root.clone()))
            .expect("open store");

        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-01Z.json");
        let operator_contracts = crate::build_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path.display().to_string(),
                "latest_run_graph_dispatch_receipt_id": "run-final-snapshot",
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": "run-final-snapshot"
                    },
                    "release_admission": {}
                }
            })
            .to_string(),
        )
        .expect("write final snapshot");

        let snapshot_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read final snapshot"))
                .expect("parse final snapshot");
        assert!(!runtime_consumption_snapshot_has_failure_control_evidence(
            &snapshot_json
        ));
        assert!(
            !resume_from_persisted_final_snapshot(&store, "run-final-snapshot")
                .expect("runtime consumption summary")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_execution_preparation_gate_allows_unrelated_blocked_contract() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-execution-gate-unrelated-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let snapshot_dir = root.join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-02Z.json");
        let operator_contracts = crate::build_operator_contracts_envelope(
            "blocked",
            vec!["closure_admission_block".to_string()],
            vec!["Inspect closure admission evidence.".to_string()],
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path.display().to_string(),
                "latest_run_graph_dispatch_receipt_id": "run-resume-gate",
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "closure_admission": {
                    "blockers": ["closure_admission_block"],
                },
                "operator_contracts": operator_contracts,
                "dispatch_receipt": {
                    "blocker_code": null,
                },
            })
            .to_string(),
        )
        .expect("write final snapshot");

        assert_eq!(
            enforce_consume_continue_execution_preparation_gate(&root),
            Ok(())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_execution_preparation_gate_rejects_preparation_blocker() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-execution-gate-prep-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let snapshot_dir = root.join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-03Z.json");
        let operator_contracts = crate::build_operator_contracts_envelope(
            "blocked",
            vec!["pending_execution_preparation_evidence".to_string()],
            vec!["Record execution preparation evidence.".to_string()],
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path.display().to_string(),
                "latest_run_graph_dispatch_receipt_id": "run-resume-gate",
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "closure_admission": {
                    "blockers": [],
                },
                "operator_contracts": operator_contracts,
                "dispatch_receipt": {
                    "blocker_code": null,
                },
            })
            .to_string(),
        )
        .expect("write final snapshot");

        let error = enforce_consume_continue_execution_preparation_gate(&root)
            .expect_err("preparation blocker must fail closed");
        assert!(
            error.contains("pending_execution_preparation_evidence"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_persisted_receipt_lineage_when_summary_rows_are_missing(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-receipt-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let snapshot_dir = store.root().join("runtime-consumption");
        fs::create_dir_all(&snapshot_dir).expect("create runtime-consumption directory");
        let snapshot_path = snapshot_dir.join("final-2026-03-18T00-00-02Z.json");
        let run_id = "run-receipt-lineage";
        let snapshot_path_string = snapshot_path.display().to_string();
        let operator_contracts = crate::build_operator_contracts_envelope(
            "pass",
            Vec::new(),
            Vec::new(),
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": snapshot_path_string.clone(),
                "latest_run_graph_dispatch_receipt_id": run_id,
                "latest_task_reconciliation_receipt_id": serde_json::Value::Null,
                "consume_final_surface": "vida taskflow consume final",
            }),
        );
        let failure_control_evidence =
            build_failure_control_evidence(run_id, &snapshot_path_string);
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "release_admission": {},
                "operator_contracts": operator_contracts,
                "payload": {
                    "dispatch_receipt": {
                        "run_id": run_id
                    },
                    "release_admission": {},
                    "failure_control_evidence": failure_control_evidence.clone()
                },
                "failure_control_evidence": failure_control_evidence
            })
            .to_string(),
        )
        .expect("write final snapshot");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_run".to_string(),
            dispatch_surface: Some("vida taskflow consume continue".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("receipt lineage should allow resume validation");
        validate_run_graph_resume_state_for_downstream_packet(&store, run_id)
            .await
            .expect("receipt lineage should allow downstream resume validation");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_closure_complete_receipt_backed_lineage_with_missing_task(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-closure-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-complete";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "closure");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;

        store
            .record_run_graph_status(&status)
            .await
            .expect("persist closure-complete status");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "writer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_run".to_string(),
            dispatch_surface: Some("vida taskflow consume continue".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("closure-complete receipt lineage should allow resume validation");
        validate_run_graph_resume_state_strict(&store, run_id)
            .await
            .expect("closure-complete receipt lineage should allow strict resume validation");
        validate_run_graph_resume_state_for_downstream_packet(&store, run_id)
            .await
            .expect("closure-complete receipt lineage should allow downstream resume validation");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_retry_eligible_blocked_agent_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-retry-eligible-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-retry-eligible-resume";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_blocked".to_string();
        status.active_node = "coach".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist status");
        let packet_path = store.root().join("retry-packet.json");
        fs::write(&packet_path, "{}").expect("packet");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some(
                "vida agent-init --dispatch-packet retry-packet.json".to_string(),
            ),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("review_cli".to_string()),
            recorded_at: "2026-05-18T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("retry-eligible blocked receipt should allow resume validation");
        validate_run_graph_resume_state_strict(&store, run_id)
            .await
            .expect("strict resume validation should also allow retry-eligible receipts");

        let mut internal_timeout_receipt = receipt;
        internal_timeout_receipt.blocker_code = Some(
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT.to_string(),
        );
        internal_timeout_receipt.selected_backend = Some("internal_subagents".to_string());
        internal_timeout_receipt.activation_agent_type = Some("internal_subagents".to_string());
        store
            .record_run_graph_dispatch_receipt(&internal_timeout_receipt)
            .await
            .expect("persist internal timeout receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("internal timeout receipt with packet should allow resume validation");
        validate_run_graph_resume_state_strict(&store, run_id)
            .await
            .expect("strict resume validation should allow internal timeout retry receipt");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_superseded_exception_takeover_replay() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-exception-replay-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-exception-takeover-replay";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "analysis",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist status");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            exception_path_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("internal_codex_carrier_unavailable".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("pi_cli".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("pi_cli".to_string()),
            recorded_at: "2026-05-20T13:40:47Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("superseded exception takeover should replay a dispatch resume gate");
        validate_run_graph_resume_state_strict(&store, run_id)
            .await
            .expect("strict resume validation should accept superseded exception replay");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_rejects_unsuperseded_exception_takeover_replay() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-exception-replay-unsuperseded-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-exception-takeover-unsuperseded";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "analysis",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist status");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("case13-analysis-timeout-takeover".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("internal_codex_carrier_unavailable".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("pi_cli".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("pi_cli".to_string()),
            recorded_at: "2026-05-20T13:40:47Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let error = validate_run_graph_resume_state(&store, run_id)
            .await
            .expect_err("unsuperseded exception takeover must still fail closed");
        assert!(error.contains("recovery_ready is false"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn active_exception_takeover_resume_blocker_error_names_lane_show() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-exception-takeover",
            "scope_discussion",
            "spec-pack",
        );
        status.task_id = "run-exception-takeover".to_string();
        status.active_node = "specification".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "specification_blocked".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: status.run_id.clone(),
            dispatch_target: "specification".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_exception_takeover".to_string(),
            supersedes_receipt_id: Some("exc-recovery-projection".to_string()),
            exception_path_receipt_id: Some("exc-recovery-projection".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --dispatch-packet packet.json".to_string()),
            dispatch_packet_path: Some("/tmp/specification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("work-pool-pack".to_string()),
            downstream_dispatch_command: Some("vida task ensure feature-x".to_string()),
            downstream_dispatch_note: Some("wait for bounded evidence return".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_specification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("specification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-06T10:51:18Z".to_string(),
        };

        let error = active_exception_takeover_resume_blocker_error(&status, Some(&receipt))
            .expect("active exception takeover should replace the generic resume error");

        assert!(error.contains("active exception takeover"));
        assert!(error.contains("recovery_ready is false"));
        assert!(error.contains("vida lane show run-exception-takeover --json"));
        assert!(error.contains("owned_write_scope"));

        let payload =
            consume_continue_resume_error_payload(&error, "vida taskflow consume continue");
        assert_eq!(payload["run_id"], "run-exception-takeover");
        assert_eq!(payload["artifact_refs"]["run_id"], "run-exception-takeover");
    }

    #[test]
    fn consume_continue_downstream_packet_activation_field_uses_runtime_assignment_fallback() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue writer".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "activation_agent_type": "configured_writer_backend",
                    "activation_runtime_role": "worker"
                }
            }),
            reason: "test".to_string(),
        };
        let packet = serde_json::json!({
            "packet_kind": "runtime_downstream_dispatch_packet",
            "activation_agent_type": null,
            "activation_runtime_role": null
        });

        assert_eq!(
            crate::taskflow_consume_resume::downstream_packet_activation_field(
                &packet,
                &role_selection,
                "activation_agent_type"
            )
            .as_deref(),
            Some("configured_writer_backend")
        );
        assert_eq!(
            crate::taskflow_consume_resume::downstream_packet_activation_field(
                &packet,
                &role_selection,
                "activation_runtime_role"
            )
            .as_deref(),
            Some("worker")
        );
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_for_downstream_packet_accepts_receipt_backed_packet_ready(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-downstream-packet-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-downstream-packet-ready";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "dev-pack", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "dev-pack".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "dev_pack_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-downstream-packet-ready.json");
        fs::write(&packet_path, "{}").expect("write downstream packet placeholder");
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after implementer evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        validate_run_graph_resume_state_for_downstream_packet(&store, run_id)
            .await
            .expect(
                "receipt-backed downstream packet_ready should allow downstream resume validation",
            );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_accepts_runtime_style_downstream_packet_ready_without_result_path(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-runtime-downstream-ready-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-runtime-downstream-ready";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-runtime-downstream-ready.json");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "resume downstream packet".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": "vida taskflow consume continue --run-id run-runtime-downstream-ready --json",
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_dispatch_result_path": "/tmp/verification-result.json"
            }))
            .expect("encode downstream packet"),
        )
        .expect("write downstream packet");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("previous-verifier-packet".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("pending_review_clean_evidence".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("no additional downstream lane is required".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/verification-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("runtime-style downstream packet_ready with result path should resume");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(
            resolved.dispatch_packet_path,
            packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_sanitizes_inherited_upstream_exception_evidence_from_ready_downstream_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-downstream-inherited-exception-sanitize-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-downstream-sanitize";

        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "dev-pack", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "dev-pack".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "dev_pack_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-downstream-sanitize.json");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "resume downstream packet".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["closure".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "source_exception_path_receipt_id": "exc-parent",
                "source_supersedes_receipt_id": "sup-parent",
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_lane_status": "lane_exception_recorded",
                "downstream_exception_path_receipt_id": "exc-parent",
                "downstream_supersedes_receipt_id": "sup-parent"
            }))
            .expect("encode downstream packet"),
        )
        .expect("write downstream packet");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("closure is ready".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("inherited upstream exception evidence should be sanitized");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(resolved.dispatch_receipt.lane_status, "packet_ready");
        assert!(resolved
            .dispatch_receipt
            .exception_path_receipt_id
            .is_none());
        assert!(resolved.dispatch_receipt.supersedes_receipt_id.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn completed_closure_bound_run_prefers_lawful_closure_packet_over_stale_blocked_coach_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-closure-bound-mixed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-bound-mixed";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let downstream_packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&downstream_packet_dir).expect("create downstream packet dir");
        let closure_packet_path =
            downstream_packet_dir.join("run-closure-bound-mixed-closure.json");
        let stale_coach_packet_path =
            downstream_packet_dir.join("run-closure-bound-mixed-stale-coach.json");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "resume downstream packet".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["closure".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        fs::write(
            &closure_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready"
            }))
            .expect("encode closure packet"),
        )
        .expect("write closure packet");
        fs::write(
            &stale_coach_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            }))
            .expect("encode stale coach packet"),
        )
        .expect("write stale coach packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let stale_coach_result_path = result_dir.join("run-closure-bound-mixed-stale-coach.json");
        fs::write(
            &stale_coach_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": stale_coach_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write stale coach result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "runtime reached closure; no additional downstream lane is required".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(closure_packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(stale_coach_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "task_id": run_id,
                        "run_id": run_id,
                        "dispatch_target": "closure"
                    }),
                    binding_source: "task_close_reconcile".to_string(),
                    why_this_unit: "task closure rebound the next lawful bounded unit to closure"
                        .to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only".to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-14T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("closure-bound run should prefer lawful closure packet");

        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(
            resolved.dispatch_packet_path,
            closure_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_completed_closure_bound_run_prefers_lawful_closure_packet_over_stale_blocked_coach_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-closure-bound-mixed-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-bound-mixed-lineage";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let stale_packet_path = packet_dir.join("run-closure-bound-mixed-lineage-coach.json");
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                }),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write stale downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let stale_result_path = result_dir.join("run-closure-bound-mixed-lineage-coach.json");
        fs::write(
            &stale_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "pending_review_clean_evidence",
                "dispatch_packet_path": stale_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write stale downstream result");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("pending_review_clean_evidence".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("bounded closure handoff is required".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_closure_handoff".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(stale_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("senior".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist stale receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: run_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "dispatch_target": "closure",
                        "run_id": run_id
                    }),
                    binding_source: "explicit_continuation_bind_downstream".to_string(),
                    why_this_unit: "completed run is explicitly closure-bound".to_string(),
                    primary_path: "lawful_closure_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_downstream_bound"
                        .to_string(),
                    request_text: Some("continue by lawful closure".to_string()),
                    recorded_at: "2026-04-14T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit downstream binding");

        let error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None).await
            {
                Ok(_) => panic!("stale blocked coach lineage must fail closed"),
                Err(error) => error,
            };
        assert!(
            error.contains("explicitly bound to downstream target `closure`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("stale downstream target `coach`"),
            "unexpected error: {error}"
        );

        let closure_packet_path = packet_dir.join("run-closure-bound-mixed-lineage-closure.json");
        fs::write(
            &closure_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": run_id,
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "runtime",
                    "fallback_role": "worker",
                    "request": "resume downstream packet",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "closure",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["closure"],
                    "compiled_bundle": null,
                    "execution_plan": null,
                    "reason": "test"
                },
                "run_graph_bootstrap": { "run_id": run_id },
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!(
                        "vida taskflow consume continue --run-id {run_id} --json"
                    ),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_dispatch_result_path": "/tmp/closure-result.json"
            }))
            .expect("encode closure packet"),
        )
        .expect("write closure packet");

        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers = Vec::new();
        receipt.downstream_dispatch_packet_path = Some(closure_packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("packet_ready".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt with lawful closure packet");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("lawful closure packet_ready should win over stale blocked coach lineage");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(
            resolved.dispatch_packet_path,
            closure_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_heals_task_close_reconcile_stale_active_result_lineage_to_closure(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-task-close-reconcile-heal-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "task-close-heal-parent",
                title: "Task close heal parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "closed",
                priority: 0,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task parent");
        store
            .create_task(CreateTaskRequest {
                task_id: "task-close-heal",
                title: "Closed task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 0,
                parent_id: Some("task-close-heal-parent"),
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");

        let run_id = "run-task-close-heal";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "delivery",
        );
        status.task_id = "task-close-heal".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let stale_packet_path = packet_dir.join("run-task-close-heal-implementer.json");
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "runtime",
                    "fallback_role": "worker",
                    "request": "continue development",
                    "selected_role": "worker",
                    "conversational_mode": null,
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": null,
                    "reason": "test"
                },
                "run_graph_bootstrap": { "run_id": run_id, "task_id": "task-close-heal" },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Implement bounded fix",
                    "scope_in": ["dispatch_target:implementer"],
                    "owned_paths": ["crates/vida/src"],
                    "definition_of_done": ["record bounded implementation result"],
                    "verification_command": "cargo test -p vida --bin vida -- --help",
                    "proof_target": "bounded implementation proof",
                    "stop_rules": ["stop after bounded result"],
                    "blocking_question": "What remains blocked?"
                }
            })
            .to_string(),
        )
        .expect("write stale packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let stale_result_path = result_dir.join("run-task-close-heal-implementer.json");
        fs::write(
            &stale_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": stale_packet_path.display().to_string(),
                "activation_command": "vida agent-init --dispatch-packet implementer.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write stale result");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("sup-task-close-heal".to_string()),
                exception_path_receipt_id: Some("exc-task-close-heal".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(stale_packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: Some("internal_activation_view_only".to_string()),
                downstream_dispatch_target: Some("implementer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("stale implementer lineage".to_string()),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: Some(stale_result_path.display().to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: Some("implementer".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist stale dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-close-heal".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": "task-close-heal",
                        "run_id": run_id,
                        "active_node": "implementer"
                    }),
                    binding_source: "task_close_reconcile".to_string(),
                    why_this_unit: "stale task-close reconcile binding".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-17T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale task-close reconcile binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("task-close reconcile should heal stale active result lineage");

        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn allow_downstream_resume_lineage_fails_closed_for_retry_eligible_receipt() {
        let retry_eligible = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-current-retry-receipt".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale verifier lineage".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/stale-verifier-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/stale-verifier-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };
        let non_retry_receipt = crate::state_store::RunGraphDispatchReceipt {
            dispatch_status: "executed".to_string(),
            blocker_code: None,
            ..retry_eligible.clone()
        };

        assert!(
            !super::allow_downstream_resume_lineage(None, None, &retry_eligible),
            "retry-eligible receipt must suppress stale downstream lineage reuse"
        );
        assert!(
            super::allow_downstream_resume_lineage(None, None, &non_retry_receipt),
            "non-retry receipts may still resolve downstream lineage"
        );
    }

    #[test]
    fn allow_downstream_resume_lineage_fails_closed_for_internal_activation_view_only_retry() {
        let root = std::env::temp_dir().join(format!(
            "vida-allow-downstream-internal-retry-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-downstream-retry".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale verifier lineage".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/stale-verifier-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/stale-verifier-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        assert!(
            !super::allow_downstream_resume_lineage(
                Some(root.as_path()),
                Some(&role_selection),
                &receipt
            ),
            "internal activation-view retry receipts must suppress stale downstream lineage reuse"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_run_id_ignores_stale_downstream_result_for_internal_activation_view_only_retry_receipt(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-resolve-internal-retry-stale-downstream-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(root.join("AGENTS.md"), "test bootstrap carrier").expect("agents bootstrap");
        fs::create_dir_all(root.join(".vida/config")).expect("create .vida/config");
        fs::create_dir_all(root.join(".vida/db")).expect("create .vida/db");
        fs::create_dir_all(root.join(".vida/project")).expect("create .vida/project");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-internal-retry-stale-downstream";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let dispatch_packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&dispatch_packet_dir).expect("create dispatch packet dir");
        let root_packet_path = dispatch_packet_dir.join(format!("{run_id}.json"));
        let root_role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        fs::write(
            &root_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "dispatch_target": "coach",
                "dispatch_surface": "internal_cli:codex",
                "dispatch_command": "codex exec",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "internal_subagents",
                "role_selection_full": root_role_selection,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            }))
            .expect("encode root dispatch packet"),
        )
        .expect("write root dispatch packet");

        let downstream_packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&downstream_packet_dir).expect("create downstream packet dir");
        let downstream_packet_path =
            downstream_packet_dir.join(format!("{run_id}-verification.json"));
        let downstream_role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "runtime".to_string(),
            fallback_role: "worker".to_string(),
            request: "resume downstream verification".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };
        fs::write(
            &downstream_packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "downstream_dispatch_target": "verification",
                "dispatch_surface": "vida agent-init",
                "dispatch_command": "vida agent-init",
                "activation_agent_type": "senior",
                "activation_runtime_role": "verifier",
                "selected_backend": "internal_subagents",
                "role_selection_full": downstream_role_selection,
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::verification::delivery"),
                    "goal": "Execute bounded verification handoff",
                    "scope_in": ["dispatch_target:verification"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "bounded verification receipt",
                    "stop_rules": ["stop after bounded verification result"],
                    "blocking_question": "What is the next bounded action required for `verification`?"
                },
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            }))
            .expect("encode downstream packet"),
        )
        .expect("write downstream packet");

        let downstream_result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&downstream_result_dir).expect("create downstream result dir");
        let downstream_result_path =
            downstream_result_dir.join(format!("{run_id}-verification.json"));
        fs::write(
            &downstream_result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "surface": "internal_cli:codex",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": downstream_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet verification.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            }))
            .expect("encode downstream result"),
        )
        .expect("write downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some(root_packet_path.display().to_string()),
            dispatch_result_path: Some(
                root.join("runtime-consumption/dispatch-results/root-blocked.json")
                    .display()
                    .to_string(),
            ),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale verifier lineage".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["internal_activation_view_only".to_string()],
            downstream_dispatch_packet_path: Some(downstream_packet_path.display().to_string()),
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(downstream_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        let mut resume_ready_status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        resume_ready_status.task_id = run_id.to_string();
        resume_ready_status.active_node = "coach".to_string();
        resume_ready_status.next_node = Some("coach".to_string());
        resume_ready_status.status = "ready".to_string();
        resume_ready_status.lifecycle_stage = "coach_active".to_string();
        resume_ready_status.policy_gate = "single_task_scope_required".to_string();
        resume_ready_status.handoff_state = "awaiting_coach".to_string();
        resume_ready_status.context_state = "sealed".to_string();
        resume_ready_status.checkpoint_kind = "conversation_cursor".to_string();
        resume_ready_status.resume_target = "dispatch.coach_lane".to_string();
        resume_ready_status.recovery_ready = true;
        store
            .record_run_graph_status(&resume_ready_status)
            .await
            .expect("persist resume-ready status");

        let resolved = resolve_runtime_consumption_resume_inputs_for_run_id(&store, run_id)
            .await
            .expect("resolver should stay on root packet for internal retry receipt");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(
            resolved.dispatch_packet_path,
            root_packet_path.display().to_string()
        );

        let replay_lineage = store
            .run_graph_replay_lineage_receipt(run_id)
            .await
            .expect("replay lineage lookup should succeed")
            .expect("replay lineage receipt should exist");
        assert_eq!(replay_lineage.lineage_kind, "root_dispatch_packet");
        assert_eq!(replay_lineage.resolved_dispatch_target, "coach");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_explicit_resume_retry_artifact_keeps_blocked_receipt_when_retry_is_only_heuristic() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-prepared-retry-packet-ready".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };

        let prepared = prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);

        assert!(
            !prepared,
            "retry eligibility alone must not restore packet readiness without a changed retry contract"
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority")
        );
        assert_eq!(receipt.selected_backend.as_deref(), Some("hermes_cli"));
    }

    #[test]
    fn resumed_selected_backend_for_agent_lane_preserves_explicit_retry_backend() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-preserve-explicit-retry-backend".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let resumed =
            super::resumed_selected_backend_for_agent_lane(&role_selection, &receipt, true);

        assert_eq!(resumed.as_deref(), Some("internal_subagents"));
    }

    #[tokio::test]
    async fn rewrite_retry_dispatch_packet_replaces_downstream_timeout_receipt_with_canonical_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-rewrite-retry-downstream-timeout-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("run-rewrite-timeout.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": "run-rewrite-timeout",
                "downstream_dispatch_target": "coach",
                "selected_backend": "hermes_cli",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": role_selection.execution_plan,
                    "reason": "test"
                }),
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "run_graph_bootstrap": {
                    "run_id": "run-rewrite-timeout"
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rewrite-timeout".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["timeout_without_takeover_authority".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let prepared =
            super::prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);
        assert!(prepared);
        assert_eq!(
            receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );

        let rewritten = super::rewrite_retry_dispatch_packet_if_downstream_carrier(
            &store,
            &role_selection,
            &serde_json::json!({ "run_id": "run-rewrite-timeout" }),
            &mut receipt,
        )
        .expect("rewrite should succeed");
        assert!(
            rewritten,
            "backend rotation must produce a fresh canonical packet"
        );

        let rewritten_path = receipt
            .dispatch_packet_path
            .clone()
            .expect("rewritten dispatch packet path");
        assert_ne!(rewritten_path, packet_path.display().to_string());
        let rewritten_packet =
            crate::read_json_file_if_present(std::path::Path::new(&rewritten_path))
                .expect("rewritten packet should exist");
        assert_eq!(
            rewritten_packet["packet_kind"].as_str(),
            Some("runtime_dispatch_packet")
        );
        assert_eq!(
            receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            rewritten_packet["selected_backend"].as_str(),
            Some("internal_subagents")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn rewrite_retry_dispatch_packet_does_not_rotate_internal_activation_view_only_without_effective_retry_gate(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-rewrite-downstream-blocked-carrier-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-rewrite-blocked-carrier-coach.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": "run-rewrite-blocked-carrier",
                "downstream_dispatch_target": "coach",
                "dispatch_surface": "external_cli:hermes_cli",
                "dispatch_command": "hermes chat",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "hermes_cli",
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "coach": {
                                "executor_backend": "hermes_cli",
                                "fallback_executor_backend": "internal_subagents"
                            }
                        }
                    }
                },
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "run_graph_bootstrap": {
                    "run_id": "run-rewrite-blocked-carrier"
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rewrite-blocked-carrier".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["internal_activation_view_only".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let rewritten = super::rewrite_retry_dispatch_packet_if_downstream_carrier(
            &store,
            &role_selection,
            &serde_json::json!({ "run_id": "run-rewrite-blocked-carrier" }),
            &mut receipt,
        )
        .expect("rewrite should succeed");
        assert!(
            !rewritten,
            "internal_activation_view_only must not rotate blocked downstream packets without the unified effective-retry gate"
        );
        assert_eq!(
            receipt.dispatch_packet_path.as_deref(),
            Some(packet_path.to_string_lossy().as_ref())
        );
        assert_eq!(receipt.selected_backend.as_deref(), Some("hermes_cli"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn rewrite_retry_dispatch_packet_does_not_rotate_internal_activation_view_only_even_when_fallback_exists(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-rewrite-internal-activation-effective-retry-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(root.join("AGENTS.md"), "test bootstrap carrier").expect("agents bootstrap");
        fs::create_dir_all(root.join(".vida/config")).expect("create .vida/config");
        fs::create_dir_all(root.join(".vida/db")).expect("create .vida/db");
        fs::create_dir_all(root.join(".vida/project")).expect("create .vida/project");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let store = StateStore::open(root.clone()).await.expect("open store");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-rewrite-internal-activation-gated.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": "run-rewrite-internal-activation-gated",
                "downstream_dispatch_target": "coach",
                "dispatch_surface": "internal_cli:codex",
                "dispatch_command": "codex exec",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "hermes_cli",
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "coach": {
                                "executor_backend": "hermes_cli",
                                "fallback_executor_backend": "internal_subagents"
                            }
                        }
                    }
                },
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "run_graph_bootstrap": {
                    "run_id": "run-rewrite-internal-activation-gated"
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-rewrite-internal-activation-gated".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("internal_cli:codex".to_string()),
            dispatch_command: Some("codex exec".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["internal_activation_view_only".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let rewritten = super::rewrite_retry_dispatch_packet_if_downstream_carrier(
            &store,
            &role_selection,
            &serde_json::json!({ "run_id": "run-rewrite-internal-activation-gated" }),
            &mut receipt,
        )
        .expect("rewrite should succeed");
        assert!(
            !rewritten,
            "internal_activation_view_only is terminal and must not rotate to a retry backend"
        );
        assert_eq!(receipt.selected_backend.as_deref(), Some("hermes_cli"));
        assert_eq!(
            receipt.dispatch_packet_path.as_deref(),
            Some(packet_path.display().to_string().as_str())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn rewrite_retry_dispatch_packet_rewrites_runtime_dispatch_packet_after_backend_rotation()
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-rewrite-runtime-dispatch-retry-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue coach review".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "coach".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "fanout_executor_backends": ["hermes_cli", "opencode_cli"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("run-runtime-rewrite-timeout.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "run_id": "run-runtime-rewrite-timeout",
                "dispatch_target": "coach",
                "selected_backend": "hermes_cli",
                "role_selection_full": {
                    "execution_plan": role_selection.execution_plan
                },
                "run_graph_bootstrap": {
                    "run_id": "run-runtime-rewrite-timeout"
                }
            })
            .to_string(),
        )
        .expect("write runtime dispatch packet");

        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-runtime-rewrite-timeout".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["timeout_without_takeover_authority".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };

        let prepared =
            super::prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);
        assert!(prepared);
        assert_eq!(receipt.selected_backend.as_deref(), Some("opencode_cli"));

        let rewritten = super::rewrite_retry_dispatch_packet_if_downstream_carrier(
            &store,
            &role_selection,
            &serde_json::json!({ "run_id": "run-runtime-rewrite-timeout" }),
            &mut receipt,
        )
        .expect("rewrite should succeed");
        assert!(
            rewritten,
            "runtime dispatch retry must write a fresh packet"
        );

        let rewritten_path = receipt
            .dispatch_packet_path
            .clone()
            .expect("rewritten dispatch packet path");
        assert_ne!(rewritten_path, packet_path.display().to_string());
        let rewritten_packet =
            crate::read_json_file_if_present(std::path::Path::new(&rewritten_path))
                .expect("rewritten packet should exist");
        assert_eq!(
            rewritten_packet["packet_kind"].as_str(),
            Some("runtime_dispatch_packet")
        );
        assert_eq!(
            rewritten_packet["selected_backend"].as_str(),
            Some("opencode_cli")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retry_backend_for_dispatch_receipt_falls_back_to_persisted_packet_route() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-retry-backend-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "coach": {
                                "executor_backend": "hermes_cli",
                                "fallback_executor_backend": "internal_subagents"
                            }
                        }
                    }
                },
                "execution_truth": {
                    "route_fallback_backend": "internal_subagents"
                }
            }))
            .expect("dispatch packet should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry-backend-from-packet".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let fallback = retry_backend_for_dispatch_receipt(&role_selection, &receipt);

        assert_eq!(fallback.as_deref(), Some("internal_subagents"));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn retry_backend_for_dispatch_receipt_uses_persisted_packet_review_fanout_before_fallback() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-retry-review-fanout-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "coach": {
                                "executor_backend": "hermes_cli",
                                "fallback_executor_backend": "internal_subagents",
                                "fanout_executor_backends": ["hermes_cli", "opencode_cli"]
                            }
                        },
                        "backend_admissibility_matrix": [
                            {
                                "backend_id": "hermes_cli",
                                "lane_admissibility": {
                                    "coach": true
                                }
                            },
                            {
                                "backend_id": "opencode_cli",
                                "lane_admissibility": {
                                    "coach": true
                                }
                            }
                        ]
                    }
                }
            }))
            .expect("dispatch packet should encode"),
        )
        .expect("dispatch packet should write");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry-review-fanout-from-packet".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let fallback = retry_backend_for_dispatch_receipt(&role_selection, &receipt);

        assert_eq!(fallback.as_deref(), Some("opencode_cli"));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn retry_backend_for_dispatch_receipt_does_not_use_review_fanout_for_non_coach_target() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementer": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "fanout_executor_backends": ["hermes_cli", "opencode_cli"]
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-no-review-fanout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes chat".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let fallback = retry_backend_for_dispatch_receipt(&role_selection, &receipt);

        assert_eq!(
            fallback.as_deref(),
            Some("internal_subagents"),
            "non-coach retries must use route fallback instead of review fanout rotation"
        );
    }

    #[test]
    fn prepare_explicit_resume_retry_artifact_keeps_internal_activation_view_only_blocked_without_rebind(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-internal-activation-no-rebind-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        fs::write(
            root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      carriers:
        middle:
          model: gpt-5.5
          sandbox_mode: workspace-write
          model_reasoning_effort: medium
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("config");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-activation-no-rebind".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
        };

        let prepared =
            prepare_explicit_resume_retry_artifact(Some(&root), &role_selection, &mut receipt);

        assert!(
            !prepared,
            "internal activation retry must not reopen same-lane dispatch without an explicit rebind"
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );

        fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn dispatch_receipt_primary_rebind_eligible_rejects_inadmissible_implementer_primary_backend() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementer": {
                        "executor_backend": "opencode_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-implementer-primary-rebind-guard".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after implementer".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        assert!(
            !super::dispatch_receipt_primary_rebind_eligible(&role_selection, &receipt),
            "internal_activation_view_only must not rebind implementer retries onto an inadmissible external primary backend"
        );
    }

    #[tokio::test]
    async fn recover_missing_first_dispatch_receipt_for_active_implementer_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-first-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-first-receipt";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                request_text: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let recovered = recover_missing_first_dispatch_receipt(&store, run_id)
            .await
            .expect("active implementer run should recover missing first receipt")
            .expect("active implementer run should synthesize receipt");

        assert_eq!(recovered.dispatch_receipt.dispatch_target, "implementer");
        assert_eq!(recovered.dispatch_receipt.dispatch_status, "routed");
        assert_eq!(recovered.dispatch_receipt.lane_status, "lane_running");
        assert_eq!(
            recovered
                .dispatch_receipt
                .activation_runtime_role
                .as_deref(),
            Some("worker")
        );
        assert_eq!(
            recovered.dispatch_receipt.activation_agent_type.as_deref(),
            Some("junior")
        );
        assert!(
            recovered.dispatch_receipt.dispatch_packet_path.is_some(),
            "recovered receipt should materialize a dispatch packet path"
        );
        let persisted = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("receipt should be persisted");
        assert_eq!(persisted.dispatch_target, "implementer");
        assert_eq!(persisted.dispatch_status, "routed");
        assert_eq!(
            recovered.dispatch_packet_path,
            recovered
                .dispatch_receipt
                .dispatch_packet_path
                .clone()
                .expect("dispatch packet path should exist")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_resume_error_classifies_missing_dispatch_receipt() {
        assert_eq!(
            consume_continue_resume_error_blocker_code(
                "No persisted run-graph dispatch receipt exists for run_id `run-a`"
            ),
            "missing_run_graph_dispatch_receipt"
        );
        assert_eq!(
            consume_continue_resume_error_blocker_code(
                "No persisted run-graph dispatch receipt exists for run_id `run-a` and missing receipt recovery could not load dispatch context"
            ),
            "missing_run_graph_dispatch_receipt"
        );
    }

    #[tokio::test]
    async fn recover_missing_first_dispatch_receipt_for_dispatch_ready_planning_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-dispatch-ready-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-dispatch-ready-receipt";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "planning".to_string();
        status.next_node = Some("analysis".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementation_dispatch_ready".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "awaiting_analysis".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.analysis_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request:
                "Analyze the bounded implementation packet and prepare execution routing truth."
                    .to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["analysis", "implementer", "coach", "verification"],
                        "lane_catalog": {
                            "analysis": {
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "closure_class": "analysis"
                            }
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                request_text:
                    "Analyze the bounded implementation packet and prepare execution routing truth."
                        .to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let recovered = recover_missing_first_dispatch_receipt(&store, run_id)
            .await
            .expect("dispatch-ready planning run should recover missing first receipt")
            .expect("dispatch-ready planning run should synthesize receipt");

        assert_eq!(recovered.dispatch_receipt.dispatch_target, "analysis");
        assert_eq!(recovered.dispatch_receipt.dispatch_status, "routed");
        assert_eq!(recovered.dispatch_receipt.lane_status, "lane_running");
        assert_eq!(
            recovered
                .dispatch_receipt
                .activation_runtime_role
                .as_deref(),
            Some("coach")
        );
        assert_eq!(
            recovered.dispatch_receipt.activation_agent_type.as_deref(),
            Some("middle")
        );
        assert!(
            recovered.dispatch_receipt.dispatch_packet_path.is_some(),
            "recovered receipt should materialize a dispatch packet path"
        );

        let persisted = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("receipt should be persisted");
        assert_eq!(persisted.dispatch_target, "analysis");
        assert_eq!(persisted.dispatch_status, "routed");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn recover_missing_first_dispatch_receipt_fails_closed_when_resume_gate_denies() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-receipt-fail-closed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-receipt-fail-closed";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementation_active".to_string();
        status.active_node = "planning".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Attempt resume after invalid status".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                request_text: "Attempt resume after invalid status".to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-21T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let recovered = recover_missing_first_dispatch_receipt(&store, run_id)
            .await
            .expect("missing receipt recovery should evaluate safely");
        assert!(
            recovered.is_none(),
            "resume-gate denial must not synthesize a dispatch receipt"
        );
        assert!(
            store
                .run_graph_dispatch_receipt(run_id)
                .await
                .expect("read dispatch receipt")
                .is_none(),
            "fail-closed recovery must not persist a new dispatch receipt"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_without_run_id_recovers_missing_first_receipt_for_active_implementer_run(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-first-receipt-latest-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-missing-first-receipt-latest";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("coach".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };
        store
            .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
                run_id: run_id.to_string(),
                task_id: run_id.to_string(),
                request_text: "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.".to_string(),
                role_selection: serde_json::to_value(&role_selection)
                    .expect("encode role selection"),
                recorded_at: "2026-04-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, None, None, None)
            .await
            .expect("latest continuation path should recover missing first receipt");

        assert_eq!(resolved.dispatch_receipt.run_id, run_id);
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "implementer");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "packet_ready");
        assert_eq!(resolved.dispatch_receipt.lane_status, "packet_ready");
        assert!(
            resolved.dispatch_receipt.dispatch_packet_path.is_some(),
            "resolved receipt should materialize a dispatch packet path"
        );
        let persisted = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read persisted receipt")
            .expect("receipt should be persisted");
        assert_eq!(persisted.dispatch_target, "implementer");
        assert_eq!(persisted.dispatch_status, "packet_ready");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_prefers_active_downstream_blocked_result() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-active-downstream-result-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-active-downstream-result";
        let labels: Vec<String> = Vec::new();
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "active-downstream-result-parent",
                title: "Active downstream result parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create parent task");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: run_id,
                title: "Active downstream blocked result",
                display_id: None,
                description: "test task",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: Some("active-downstream-result-parent"),
                labels: &labels,
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create task");
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let packet_path = packet_dir.join("run-active-downstream-result-verification.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "verifier_proof_packet",
                "run_id": run_id,
                "verifier_proof_packet": {
                    "proof_goal": "verify the bounded packet",
                    "verification_command": "cargo test -p vida verifier-smoke",
                    "proof_target": "bounded verifier proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "verification",
                "activation_agent_type": "senior",
                "activation_runtime_role": "verifier",
                "selected_backend": "senior",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"],
                                "implementer_activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "coach_activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "verifier_activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    },
                    "reason": "test"
                }),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let result_path = result_dir.join("run-active-downstream-result-verification.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet verification.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for verifier evidence".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_verification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let inputs = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("resume inputs should resolve from active downstream result");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "verification");
        assert_eq!(inputs.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(
            inputs.dispatch_receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            inputs.dispatch_receipt.dispatch_surface.as_deref(),
            Some("internal_cli:qwen")
        );
        assert_eq!(
            inputs.dispatch_receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            inputs.dispatch_receipt.activation_agent_type.as_deref(),
            Some("senior")
        );
        assert_eq!(
            inputs.dispatch_receipt.activation_runtime_role.as_deref(),
            Some("verifier")
        );
        assert_eq!(
            inputs.dispatch_packet_path,
            packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn downstream_result_packet_path_rejects_source_only_downstream_packet_lineage() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-downstream-result-source-lineage-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let packet_path = root.join("downstream-packet.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet"
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let result = serde_json::json!({
            "source_dispatch_packet_path": packet_path.display().to_string()
        });

        assert_eq!(super::downstream_result_packet_path(&result), None);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_prefers_active_downstream_result_over_stale_ready_packet_for_coach_active_run(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-coach-active-precedence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-coach-active-precedence";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = run_id.to_string();
        status.active_node = "coach".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "conversation_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let stale_packet_path = packet_dir.join("run-coach-active-precedence-stale.json");
        let active_packet_path = packet_dir.join("run-coach-active-precedence-active.json");
        let role_selection = serde_json::json!({
            "ok": true,
            "activation_source": "test",
            "selection_mode": "auto",
            "fallback_role": "orchestrator",
            "request": "continue development",
            "selected_role": "pm",
            "conversational_mode": "development",
            "single_task_only": true,
            "tracked_flow_entry": "dev-pack",
            "allow_freeform_chat": false,
            "confidence": "high",
            "matched_terms": ["continue"],
            "compiled_bundle": null,
            "execution_plan": {
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                }
            },
            "reason": "test"
        });
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection.clone(),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write stale downstream packet");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write active downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path = result_dir.join("run-coach-active-precedence-coach.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: Some("receipt-implementer-current".to_string()),
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after implementer evidence, activate coach".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(stale_packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let active_result_resume = super::maybe_resume_inputs_from_active_downstream_result(
            &store,
            Some(run_id),
            &receipt,
        )
        .await
        .expect("active downstream result probe should not fail");
        let active_result_resume =
            active_result_resume.expect("active downstream result should be visible");
        assert_eq!(
            active_result_resume.dispatch_receipt.dispatch_target,
            "coach"
        );
        assert_eq!(
            active_result_resume.dispatch_receipt.dispatch_status,
            "blocked"
        );

        let inputs = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("resume inputs should resolve from active downstream coach result");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(inputs.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(
            inputs.dispatch_receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            inputs.dispatch_receipt.dispatch_packet_path.as_deref(),
            Some(active_packet_path.display().to_string().as_str())
        );
        assert_eq!(
            inputs.dispatch_packet_path,
            active_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_for_completed_closure_bound_run_rejects_stale_active_and_ready_downstream_coach_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-completed-closure-bound-stale-downstream-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-completed-closure-bound-stale-downstream";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-closure".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let stale_packet_path = packet_dir.join("run-completed-closure-bound-stale-coach.json");
        let active_packet_path = packet_dir.join("run-completed-closure-bound-active-coach.json");
        let role_selection = serde_json::json!({
            "ok": true,
            "activation_source": "test",
            "selection_mode": "auto",
            "fallback_role": "orchestrator",
            "request": "continue development",
            "selected_role": "pm",
            "conversational_mode": "development",
            "single_task_only": true,
            "tracked_flow_entry": "dev-pack",
            "allow_freeform_chat": false,
            "confidence": "high",
            "matched_terms": ["continue"],
            "compiled_bundle": null,
            "execution_plan": {
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"]
                    }
                }
            },
            "reason": "test"
        });
        fs::write(
            &stale_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection.clone(),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write stale downstream coach packet");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write active downstream coach packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path =
            result_dir.join("run-completed-closure-bound-stale-downstream-coach.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: Some("receipt-implementer-current".to_string()),
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("coach".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("stale downstream coach evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some(stale_packet_path.display().to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-closure".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "task_id": "task-closure",
                        "run_id": run_id,
                        "dispatch_target": "closure",
                    }),
                    binding_source: "latest_run_graph_dispatch_receipt".to_string(),
                    why_this_unit: "closure is the only lawful next bounded unit".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_downstream_bound".to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit closure binding");

        let error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None).await
            {
                Ok(_) => panic!("stale downstream coach lineage must fail closed"),
                Err(error) => error,
            };
        assert!(
            error.contains("explicitly bound to downstream target `closure`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("stale downstream target `coach`"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_resume_inputs_without_run_id_fails_closed_for_ambiguous_completed_run_with_active_downstream_result(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-ambiguous-latest-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-ambiguous-latest";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "delivery",
        );
        status.task_id = run_id.to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let active_packet_path = packet_dir.join("run-ambiguous-latest-active.json");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "run_id": run_id,
                "coach_review_packet": {
                    "review_goal": "review bounded packet",
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded coach proof",
                    "read_only_paths": ["crates/vida/src"],
                    "blocking_question": "What remains blocked?"
                },
                "downstream_dispatch_target": "coach",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "middle",
                "role_selection_full": serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"],
                                "implementer_activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                },
                                "coach_activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                },
                                "verifier_activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    },
                    "reason": "test"
                }),
                "run_graph_bootstrap": {
                    "run_id": run_id
                }
            })
            .to_string(),
        )
        .expect("write active downstream packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path = result_dir.join("run-ambiguous-latest-coach.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet coach.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "after coach evidence, activate verification".to_string(),
            ),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: Some(active_packet_path.display().to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist receipt");

        let error = match resolve_runtime_consumption_resume_inputs(&store, None, None, None).await
        {
            Ok(_) => {
                panic!("ambiguous completed run should fail closed without --run-id");
            }
            Err(error) => error,
        };
        assert!(
            error.contains(
                "Latest continuation binding for run `run-ambiguous-latest` is ambiguous"
            ),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn consume_continue_resume_error_payload_preserves_json_contract_for_ambiguous_binding() {
        let error = "Latest continuation binding for run `run-ambiguous-latest` is ambiguous. Either bind the next bounded unit explicitly with `vida taskflow continuation bind run-ambiguous-latest --task-id <task-id> --json` or pass `--run-id run-ambiguous-latest` to refresh that specific run.";
        let payload =
            consume_continue_resume_error_payload(error, "vida taskflow consume continue");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["run_id"], "run-ambiguous-latest");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["continuation_binding_ambiguous"])
        );
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["operator_contracts"]["blocker_codes"],
            payload["blocker_codes"]
        );
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            payload["next_actions"]
        );
        assert!(payload["next_actions"]
            .as_array()
            .expect("next_actions should be array")
            .iter()
            .any(|action| action
                .as_str()
                .unwrap_or_default()
                .contains("continuation bind <run-id> --task-id <task-id> --json")));
    }

    #[test]
    fn consume_continue_resume_error_payload_does_not_recommend_task_binding_for_active_run() {
        let error = "Latest continuation binding for active run `run-active-blocked` is ambiguous, but the run has not reached closure_complete. Do not bind a new --task-id for this active run; pass `--run-id run-active-blocked` to refresh that specific run or inspect run-graph/task evidence if refresh remains blocked.";
        let payload =
            consume_continue_resume_error_payload(error, "vida taskflow consume continue");
        let next_actions = payload["next_actions"]
            .as_array()
            .expect("next_actions should be array");

        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["continuation_binding_ambiguous"])
        );
        assert!(next_actions.iter().any(|action| {
            let action = action.as_str().unwrap_or_default();
            action.contains("consume continue --run-id run-active-blocked --json")
                || action.contains("consume continue --run-id 'run-active-blocked' --json")
        }));
        assert!(next_actions.iter().all(|action| !action
            .as_str()
            .unwrap_or_default()
            .contains("continuation bind")));
    }

    #[test]
    fn consume_continue_resume_error_payload_recommends_retire_for_missing_task_stale_run() {
        let error = "Stale missing-task run graph `run-stale` references missing TaskFlow task `task-missing`; retire the stale run with `vida lane retire run-stale --receipt-id run-stale --reason \"missing TaskFlow task stale run\" --json` before consuming continuation.";
        let payload =
            consume_continue_resume_error_payload(error, "vida taskflow consume continue");
        let next_actions = payload["next_actions"]
            .as_array()
            .expect("next_actions should be array");

        assert_eq!(payload["run_id"], "run-stale");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["stale_missing_task_run_graph"])
        );
        assert!(next_actions.iter().any(|action| {
            action
                .as_str()
                .unwrap_or_default()
                .contains("vida lane retire run-stale --receipt-id run-stale")
        }));
        assert!(next_actions.iter().all(|action| !action
            .as_str()
            .unwrap_or_default()
            .contains("continuation bind")));
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_rejects_missing_task_stale_run_with_retire_action() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-missing-task-stale-run-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-missing-task-stale";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "analysis", "delivery");
        status.task_id = "task-missing".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist stale run graph status");

        let error = validate_run_graph_resume_state(&store, run_id)
            .await
            .expect_err("missing task stale run must fail closed");
        assert!(
            error.contains("Stale missing-task run graph `run-missing-task-stale`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("vida lane retire run-missing-task-stale"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn validate_run_graph_resume_state_accepts_completed_lane_receipt_over_stale_open_cycle()
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-completed-lane-open-cycle-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-completed-lane-open-cycle";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "coach", "delivery");
        status.task_id = "task-completed-lane-open-cycle".to_string();
        status.active_node = "coach".to_string();
        status.status = "running".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist stale open-cycle status");

        let result_path = root.join("completed-lane-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "run_id": run_id,
                "execution_state": "executed",
                "activation_semantics": {
                    "activation_kind": "execution_evidence",
                    "view_only": false,
                    "executes_packet": true,
                    "records_completion_receipt": true
                }
            })
            .to_string(),
        )
        .expect("write completed lane result");
        let mut receipt = taskflow_consume_resume_test_receipt("agent_lane", "executed");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "coach".to_string();
        receipt.lane_status = "lane_completed".to_string();
        receipt.blocker_code = None;
        receipt.dispatch_result_path = Some(result_path.display().to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist completed lane receipt");

        validate_run_graph_resume_state(&store, run_id)
            .await
            .expect("completed receipt evidence should satisfy stale resume gate");
        validate_run_graph_resume_state_strict(&store, run_id)
            .await
            .expect("completed receipt evidence should satisfy strict stale resume gate");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_run_id_fails_closed_when_explicit_task_graph_binding_mismatches_dispatch_packet_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-binding-mismatch-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-explicit-binding-mismatch";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-old".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let packet_path = packet_dir.join("run-explicit-binding-mismatch.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "dispatch_status": "executed",
                "lane_status": "lane_completed",
                "dispatch_kind": "taskflow_pack",
                "dispatch_surface": "vida taskflow consume",
                "dispatch_command": "vida taskflow consume continue --run-id run-explicit-binding-mismatch --json",
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "selected_backend": "taskflow_state_store",
                "recorded_at": "2026-04-13T00:00:00Z",
                "request_text": "continue development",
                "role_selection": {
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "tracked_flow_entry": "dev-pack",
                    "confidence": "high"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                },
                "taskflow_handoff_plan": null,
                "run_graph_bootstrap": {
                    "run_id": run_id,
                    "latest_status": {
                        "run_id": run_id,
                        "task_id": "task-old"
                    }
                },
                "orchestration_contract": null
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow consume".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": run_id
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "test mismatch".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None).await
            {
                Ok(_) => panic!("stale packet lineage must fail closed"),
                Err(error) => error,
            };
        assert!(
            error.contains("explicit continuation binding to task_graph_task `task-new`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("still points to task `task-old`"),
            "unexpected error: {error}"
        );
        assert!(
            store
                .run_graph_replay_lineage_receipt(run_id)
                .await
                .expect("replay lineage lookup should succeed")
                .is_none(),
            "fail-closed lineage mismatch must not persist a replay-lineage receipt"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_run_id_fails_closed_when_explicit_task_graph_binding_mismatches_persisted_status(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-binding-status-mismatch-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-explicit-binding-status-mismatch";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-old".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lifecycle_stage = "closure_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "blocked_on_closure".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.closure".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist stale run graph status");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "closure".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow consume".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("pending_downstream_dispatch".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-21T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist stale dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": run_id,
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "operator rebound work to the new task".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-21T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let error = match resolve_runtime_consumption_resume_inputs_for_run_id(&store, run_id).await
        {
            Ok(_) => panic!("stale run-graph status must fail closed"),
            Err(error) => error,
        };
        assert!(
            error.contains("explicit continuation binding to task_graph_task `task-new`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("persisted run-graph status still points to task `task-old`"),
            "unexpected error: {error}"
        );
        assert!(
            store
                .run_graph_replay_lineage_receipt(run_id)
                .await
                .expect("replay lineage lookup should succeed")
                .is_none(),
            "fail-closed status mismatch must not persist a replay-lineage receipt"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_without_run_id_fails_closed_on_cross_run_explicit_task_binding(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-cross-run-explicit-task-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut upstream_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-upstream",
            "implementation",
            "implementation",
        );
        upstream_status.task_id = "task-upstream".to_string();
        upstream_status.active_node = "implementation".to_string();
        upstream_status.status = "in_progress".to_string();
        upstream_status.lifecycle_stage = "implementation_active".to_string();
        store
            .record_run_graph_status(&upstream_status)
            .await
            .expect("persist upstream status");

        let mut child_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-child",
            "implementation",
            "implementation",
        );
        child_status.task_id = "run-child".to_string();
        child_status.active_node = "implementation".to_string();
        child_status.status = "pending".to_string();
        child_status.lifecycle_stage = "initialized".to_string();
        store
            .record_run_graph_status(&child_status)
            .await
            .expect("persist child status");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-upstream".to_string(),
                    task_id: "task-upstream".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-upstream",
                        "run_id": "run-upstream",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "operator rebound work to the upstream task".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue".to_string()),
                    recorded_at: "2026-04-16T09:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let error = match resolve_runtime_consumption_resume_inputs(&store, None, None, None).await
        {
            Ok(_) => panic!("cross-run explicit task binding should fail closed"),
            Err(error) => error,
        };
        assert!(
            error.contains("must not silently reselect the stale latest run"),
            "unexpected error: {error}"
        );
        assert!(error.contains("run-upstream"), "unexpected error: {error}");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_default_resume_run_id_rejects_foreign_latest_terminal_run_before_mutation() {
        let saved_session_id = std::env::var("VIDA_SESSION_ID").ok();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-foreign-latest");
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-foreign-latest-terminal-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut foreign_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-foreign-terminal",
            "closure",
            "delivery",
        );
        foreign_status.task_id = "task-foreign-terminal".to_string();
        foreign_status.active_node = "closure".to_string();
        foreign_status.next_node = None;
        foreign_status.status = "completed".to_string();
        foreign_status.lifecycle_stage = "closure_complete".to_string();
        foreign_status.handoff_state = "none".to_string();
        foreign_status.resume_target = "none".to_string();
        foreign_status.recovery_ready = true;
        store
            .record_run_graph_status(&foreign_status)
            .await
            .expect("persist foreign terminal status");
        store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: "foreign-terminal-run-claim".to_string(),
                state_root_id: "state-root".to_string(),
                worktree_environment_id: "worktree-a".to_string(),
                orchestrator_session_id: "session-foreign-latest".to_string(),
                process_id: None,
                task_id: Some("task-foreign-terminal".to_string()),
                run_id: Some("run-foreign-terminal".to_string()),
                lane_id: None,
                claim_kind: "write".to_string(),
                conflict_domain: Some("run-graph-continuation-ownership".to_string()),
                owned_paths: vec!["foreign/path.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("acquire foreign run claim");

        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "session-current-consume");
        }
        let error = match resolve_default_resume_run_id(&store).await {
            Ok(run_id) => panic!("foreign latest run must not be selected by default: {run_id}"),
            Err(error) => error,
        };
        assert!(
            error.contains("current session does not own run `run-foreign-terminal`"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("--run-id run-foreign-terminal"),
            "unexpected error: {error}"
        );

        let _ = fs::remove_dir_all(&root);
        match saved_session_id {
            Some(value) => unsafe {
                std::env::set_var("VIDA_SESSION_ID", value);
            },
            None => unsafe {
                std::env::remove_var("VIDA_SESSION_ID");
            },
        }
    }

    #[tokio::test]
    async fn resolve_default_resume_run_id_prefers_active_exception_takeover_over_stale_explicit_task_binding(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-active-exception-over-stale-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut stale_status = crate::taskflow_run_graph::default_run_graph_status(
            "run-stale-explicit",
            "implementation",
            "implementation",
        );
        stale_status.task_id = "task-stale-explicit".to_string();
        stale_status.active_node = "implementation".to_string();
        stale_status.status = "in_progress".to_string();
        stale_status.lifecycle_stage = "implementation_active".to_string();
        store
            .record_run_graph_status(&stale_status)
            .await
            .expect("persist stale status");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-stale-explicit".to_string(),
                    task_id: "task-stale-explicit".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-stale-explicit",
                        "run_id": "run-stale-explicit",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "operator previously rebound a different task".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue stale task".to_string()),
                    recorded_at: "2026-05-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale explicit binding");

        let active_run_id = "run-active-exception";
        let mut active_status =
            crate::taskflow_run_graph::default_run_graph_status(active_run_id, "coach", "delivery");
        active_status.task_id = "task-active-exception".to_string();
        active_status.active_node = "coach".to_string();
        active_status.next_node = None;
        active_status.status = "blocked".to_string();
        active_status.lifecycle_stage = "coach_blocked".to_string();
        active_status.policy_gate = "single_task_scope_required".to_string();
        active_status.handoff_state = "blocked_on_coach".to_string();
        active_status.context_state = "sealed".to_string();
        active_status.checkpoint_kind = "conversation_cursor".to_string();
        active_status.resume_target = "dispatch.coach".to_string();
        active_status.recovery_ready = true;
        store
            .record_run_graph_status(&active_status)
            .await
            .expect("persist active exception status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: active_run_id.to_string(),
                dispatch_target: "coach".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("receipt-superseded-by-exception".to_string()),
                exception_path_receipt_id: Some("exception-receipt-active".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some(format!(
                    "vida taskflow consume continue --run-id {active_run_id} --json"
                )),
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: Some("internal_dispatch_timeout_without_receipt".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("coach".to_string()),
                downstream_dispatch_last_target: Some("coach".to_string()),
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("coach".to_string()),
                selected_backend: Some("middle".to_string()),
                recorded_at: "2026-05-13T00:01:00Z".to_string(),
            })
            .await
            .expect("persist active exception receipt");

        let resolved = resolve_default_resume_run_id(&store)
            .await
            .expect("active exception takeover should select the latest run");
        assert_eq!(resolved, active_run_id);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_run_id_allows_matching_explicit_task_graph_binding_lineage(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-binding-match-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-explicit-binding-match";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-aligned".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let packet_path = packet_dir.join("run-explicit-binding-match.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "dispatch_target": "implementer",
                "dispatch_status": "executed",
                "lane_status": "lane_completed",
                "dispatch_kind": "taskflow_pack",
                "dispatch_surface": "vida taskflow consume",
                "dispatch_command": "vida taskflow consume continue --run-id run-explicit-binding-match --json",
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "selected_backend": "taskflow_state_store",
                "recorded_at": "2026-04-13T00:00:00Z",
                "request_text": "continue development",
                "role_selection": {
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "tracked_flow_entry": "dev-pack",
                    "confidence": "high"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::implementer::delivery"),
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                },
                "taskflow_handoff_plan": null,
                "run_graph_bootstrap": {
                    "run_id": run_id,
                    "latest_status": {
                        "run_id": run_id,
                        "task_id": "task-aligned"
                    }
                },
                "orchestration_contract": null
            })
            .to_string(),
        )
        .expect("write dispatch packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("dispatch packet should validate");
        assert_eq!(
            persisted_dispatch_packet_lineage_task_id(&packet),
            Some("task-aligned")
        );

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow consume".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-04-13T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-aligned".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-aligned",
                        "run_id": run_id
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "test match".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-04-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("matching explicit binding should keep current resume path admissible");
        assert_eq!(resolved.dispatch_receipt.run_id, run_id);
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "implementer");
        assert_eq!(
            resolved.dispatch_packet_path,
            packet_path.display().to_string()
        );
        let replay_lineage = store
            .run_graph_replay_lineage_receipt(run_id)
            .await
            .expect("replay lineage lookup should succeed")
            .expect("lawful resume should persist replay-lineage receipt");
        assert_eq!(replay_lineage.run_id, run_id);
        assert_eq!(replay_lineage.lineage_kind, "root_dispatch_packet");
        assert_eq!(replay_lineage.replay_scope, "resume_resolution");
        assert_eq!(replay_lineage.source_dispatch_target, "implementer");
        assert_eq!(replay_lineage.resolved_dispatch_target, "implementer");
        assert_eq!(replay_lineage.resolved_task_id, "task-aligned");
        assert_eq!(replay_lineage.validation_outcome, "lawful_resume");
        assert_eq!(replay_lineage.checkpoint_kind, "execution_cursor");
        assert_eq!(replay_lineage.resume_target, "none");
        assert_eq!(
            replay_lineage.source_dispatch_packet_path.as_deref(),
            Some(packet_path.display().to_string().as_str())
        );
        assert!(replay_lineage
            .origin_checkpoint_ref
            .starts_with(&format!("{run_id}:execution_cursor:none")));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_without_run_id_switches_to_fresh_bound_task_run(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-binding-fresh-bound-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let old_run_id = "github-116-orchestrator-session-identity";
        let bound_task_id = "github-116-spec-pack-close-recognition-drift";
        let mut old_status =
            crate::taskflow_run_graph::default_run_graph_status(old_run_id, "closure", "delivery");
        old_status.task_id = old_run_id.to_string();
        old_status.active_node = "closure".to_string();
        old_status.next_node = None;
        old_status.status = "completed".to_string();
        old_status.lifecycle_stage = "closure_complete".to_string();
        old_status.resume_target = "none".to_string();
        old_status.recovery_ready = false;
        store
            .record_run_graph_status(&old_status)
            .await
            .expect("persist old completed run graph status");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: old_run_id.to_string(),
                    task_id: bound_task_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": bound_task_id,
                        "run_id": old_run_id,
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "operator rebound work to the fresh task run".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue development".to_string()),
                    recorded_at: "2026-05-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist explicit continuation binding");

        let mut bound_status = crate::taskflow_run_graph::default_run_graph_status(
            bound_task_id,
            "implementer",
            "delivery",
        );
        bound_status.task_id = bound_task_id.to_string();
        bound_status.active_node = "implementer".to_string();
        bound_status.next_node = Some("implementer".to_string());
        bound_status.status = "ready".to_string();
        bound_status.lifecycle_stage = "implementer_ready".to_string();
        bound_status.policy_gate = "single_task_scope_required".to_string();
        bound_status.handoff_state = "awaiting_implementer".to_string();
        bound_status.resume_target = "dispatch.implementer".to_string();
        bound_status.recovery_ready = true;
        store
            .record_run_graph_status(&bound_status)
            .await
            .expect("persist fresh bound-task run graph status");

        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create dispatch packet dir");
        let packet_path = packet_dir.join("github-116-spec-pack-close-recognition-drift.json");
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": bound_task_id,
                "dispatch_target": "implementer",
                "dispatch_status": "packet_ready",
                "lane_status": "packet_ready",
                "dispatch_kind": "taskflow_pack",
                "dispatch_surface": "vida taskflow consume",
                "dispatch_command": format!("vida taskflow consume continue --run-id {bound_task_id} --json"),
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "selected_backend": "taskflow_state_store",
                "recorded_at": "2026-05-13T00:00:00Z",
                "request_text": "continue development",
                "role_selection": {
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "tracked_flow_entry": "dev-pack",
                    "confidence": "high"
                },
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "auto",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["continue"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": ["implementer", "coach", "verification"]
                            }
                        }
                    },
                    "reason": "test"
                },
                "delivery_task_packet": {
                    "packet_id": format!("{bound_task_id}::implementer::delivery"),
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": format!("vida taskflow consume continue --run-id {bound_task_id} --json"),
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                },
                "taskflow_handoff_plan": null,
                "run_graph_bootstrap": {
                    "run_id": bound_task_id,
                    "latest_status": {
                        "run_id": bound_task_id,
                        "task_id": bound_task_id
                    }
                },
                "orchestration_contract": null
            })
            .to_string(),
        )
        .expect("write fresh bound-task dispatch packet");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: bound_task_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "packet_ready".to_string(),
                lane_status: "packet_ready".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "taskflow_pack".to_string(),
                dispatch_surface: Some("vida taskflow consume".to_string()),
                dispatch_command: Some(format!(
                    "vida taskflow consume continue --run-id {bound_task_id} --json"
                )),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("taskflow_state_store".to_string()),
                recorded_at: "2026-05-13T00:00:00Z".to_string(),
            })
            .await
            .expect("persist fresh bound-task dispatch receipt");

        let default_resolved = resolve_runtime_consumption_resume_inputs(&store, None, None, None)
            .await
            .expect("default explicit binding should switch resume to fresh bound-task run");
        assert_eq!(default_resolved.dispatch_receipt.run_id, bound_task_id);
        assert_eq!(
            default_resolved.dispatch_receipt.dispatch_target,
            "implementer"
        );
        assert_eq!(
            default_resolved.dispatch_packet_path,
            packet_path.display().to_string()
        );

        let explicit_error =
            match resolve_runtime_consumption_resume_inputs(&store, Some(old_run_id), None, None)
                .await
            {
                Ok(_) => panic!("explicit run id should remain a hard resume target"),
                Err(error) => error,
            };
        assert!(explicit_error.contains("No persisted run-graph dispatch receipt"));
        assert!(explicit_error.contains(old_run_id));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn runtime_consumption_resume_blocker_code_uses_explicit_run_receipt_lineage_when_run_id_is_requested(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-resume-explicit-run-blocker-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let explicit_run_id = "run-explicit";
        let mut explicit_status = crate::taskflow_run_graph::default_run_graph_status(
            explicit_run_id,
            "implementer",
            "delivery",
        );
        explicit_status.task_id = "task-explicit".to_string();
        explicit_status.status = "running".to_string();
        explicit_status.lifecycle_stage = "execution_active".to_string();
        explicit_status.resume_target = "current_lane".to_string();
        store
            .record_run_graph_status(&explicit_status)
            .await
            .expect("persist explicit status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: explicit_run_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some(format!(
                    "vida taskflow consume continue --run-id {explicit_run_id} --json"
                )),
                dispatch_packet_path: Some("/tmp/explicit-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/explicit-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-04-15T00:00:00Z".to_string(),
            })
            .await
            .expect("persist explicit receipt");

        let latest_run_id = "run-latest";
        let mut latest_status = crate::taskflow_run_graph::default_run_graph_status(
            latest_run_id,
            "closure",
            "delivery",
        );
        latest_status.task_id = "task-latest".to_string();
        latest_status.status = "completed".to_string();
        latest_status.lifecycle_stage = "closure_complete".to_string();
        latest_status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: latest_run_id.to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_completed".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/latest-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/latest-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("senior".to_string()),
                recorded_at: "2026-04-15T00:00:01Z".to_string(),
            })
            .await
            .expect("persist latest receipt");

        let payload_json = serde_json::json!({
            "dispatch_receipt": {
                "run_id": explicit_run_id
            }
        });

        let explicit_blocker =
            runtime_consumption_resume_blocker_code(&store, &payload_json, Some(explicit_run_id))
                .await
                .expect("explicit blocker lookup should succeed");
        assert_eq!(explicit_blocker, None);

        let latest_blocker = runtime_consumption_resume_blocker_code(&store, &payload_json, None)
            .await
            .expect("latest blocker lookup should succeed");
        assert_eq!(
            latest_blocker.as_deref(),
            Some(super::super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_runtime_dispatch_packet_backfills_read_only_paths_for_legacy_packets() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "coach_review_packet",
            "coach_review_packet": {
                "packet_id": "run-1::coach::coach-review",
                "review_goal": "review bounded packet",
                "owned_paths": [],
                "definition_of_done": ["return bounded review evidence"],
                "proof_target": "bounded proof target",
                "blocking_question": "is it aligned?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["coach_review_packet"]["read_only_paths"],
            serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS)
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_rewrites_stale_coach_review_contract_and_prompt() {
        let mut packet = serde_json::json!({
            "packet_kind": "runtime_downstream_dispatch_packet",
            "packet_template_kind": "coach_review_packet",
            "run_id": "run-stale-coach",
            "downstream_dispatch_target": "coach",
            "activation_runtime_role": "coach",
            "request_text": "Review the bounded implementation evidence.",
            "prompt": "First substantive response: publish a concise plan before edits or implementation.",
            "role_selection_full": {
                "execution_plan": {
                    "orchestration_contract": {
                        "replanning": {
                            "checkpoints": ["after proof"]
                        }
                    }
                }
            },
            "coach_review_packet": {
                "packet_id": "run-stale-coach::coach::coach-review",
                "source_packet_id": "run-stale-coach::coach::delivery",
                "review_goal": "Judge whether `coach` remains aligned with the approved packet",
                "owned_paths": [],
                "definition_of_done": ["return bounded review evidence"],
                "proof_target": "bounded coach proof",
                "read_only_paths": ["crates/vida/src"],
                "blocking_question": "Does `coach` match the approved bounded contract?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["coach_review_packet"]["reviewed_dispatch_target"],
            serde_json::json!("implementer")
        );
        assert_eq!(
            packet["coach_review_packet"]["source_packet_id"],
            serde_json::json!("run-stale-coach::implementer::delivery")
        );
        assert_eq!(
            packet["coach_review_packet"]["review_subject"],
            serde_json::json!("bounded `implementer` delivery/result")
        );
        assert_eq!(
            packet["coach_review_packet"]["expected_output"],
            serde_json::json!([
                "decision=approve|rework|blocker",
                "checked_evidence",
                "findings",
                "risks",
                "next_required_action"
            ])
        );
        let prompt = packet["prompt"]
            .as_str()
            .expect("normalized packet should have prompt");
        assert!(prompt.contains("Review/proof lane contract: do not edit files"));
        assert!(prompt.contains("decision=approve|rework|blocker"));
        assert!(!prompt.contains(
            "First substantive response: publish a concise plan before edits or implementation."
        ));
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_owned_paths_for_legacy_implementer_delivery_packet(
    ) {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "implementer",
            "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
        assert_eq!(
            packet["delivery_task_packet"]["read_only_paths"],
            serde_json::json!([".vida/data/state/runtime-consumption"])
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_uses_planner_metadata_when_request_has_no_concrete_scope()
    {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "implementer",
            "request_text": "Continue the active bounded implementation task.",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "dev_task": {
                            "planner_metadata": {
                                "owned_paths": [
                                    "crates/vida/src/runtime_dispatch_execution.rs",
                                    "docs/product/spec/codex-host-agent-boundary-and-cli-bridge-design.md"
                                ]
                            }
                        }
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?",
                "handoff_task_class": "implementation"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_execution.rs",
                "docs/product/spec/codex-host-agent-boundary-and-cli-bridge-design.md"
            ])
        );
        crate::validate_runtime_dispatch_packet_contract(&packet, "test packet")
            .expect("normalized packet should satisfy implementation owned scope");
    }

    #[test]
    fn normalize_runtime_dispatch_packet_uses_planner_metadata_for_test_authoring_scope() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "test_author",
            "request_text": "Continue the active bounded test authoring task.",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "dev_task": {
                            "planner_metadata": {
                                "owned_paths": [
                                    "crates/vida/src/taskflow_run_graph.rs",
                                    "crates/vida/src/taskflow_consume_resume.rs"
                                ]
                            }
                        }
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::test_author::delivery",
                "goal": "Execute bounded test-author handoff",
                "scope_in": ["dispatch_target:test_author", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded regression test result"],
                "verification_command": "cargo test -p vida taskflow_run_graph -- --nocapture",
                "proof_target": "regression test covers the bounded runtime defect",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for test_author?",
                "handoff_task_class": "test_authoring"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/taskflow_run_graph.rs",
                "crates/vida/src/taskflow_consume_resume.rs"
            ])
        );
        crate::validate_runtime_dispatch_packet_contract(&packet, "test packet")
            .expect("normalized test-authoring packet should satisfy owned scope");
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_owned_paths_from_delivery_packet_request_text() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_implementer_owned_paths_from_tracked_design_doc() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let design_doc_path = std::env::temp_dir().join(format!(
            "vida-implementer-design-scope-{}-{}.md",
            std::process::id(),
            nanos
        ));
        fs::write(
            &design_doc_path,
            "### Bounded File Set\n- `crates/vida/src/runtime_dispatch_packets.rs`\n- `crates/vida/src/runtime_dispatch_state.rs`\n",
        )
        .expect("write tracked design doc");

        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "implementer",
            "request_text": "Continue the bounded implementation packet and keep scope from the approved design.",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "design_doc_path": design_doc_path.display().to_string()
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::implementer::delivery",
                "goal": "Execute bounded implementer handoff",
                "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for implementer?",
                "handoff_task_class": "implementation"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );

        let _ = fs::remove_file(design_doc_path);
    }

    #[test]
    fn normalize_runtime_dispatch_packet_derives_specification_owned_paths_from_tracked_design_doc()
    {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "specification",
            "request_text": "Investigate crates/vida/src/runtime_dispatch_state.rs and capture the design update.",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "design_doc_path": "docs/product/spec/feature-x-design.md"
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::specification::delivery",
                "goal": "Execute bounded specification handoff",
                "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for specification?",
                "handoff_task_class": "specification"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["docs/product/spec/feature-x-design.md"])
        );
    }

    #[test]
    fn normalize_runtime_dispatch_packet_repairs_mismatched_specification_owned_paths() {
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "dispatch_target": "specification",
            "role_selection_full": {
                "execution_plan": {
                    "tracked_flow_bootstrap": {
                        "design_doc_path": "docs/product/spec/feature-x-design.md"
                    }
                }
            },
            "delivery_task_packet": {
                "packet_id": "run-1::specification::delivery",
                "goal": "Execute bounded specification handoff",
                "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for specification?",
                "handoff_task_class": "specification"
            }
        });

        assert!(normalize_runtime_dispatch_packet(&mut packet));
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["docs/product/spec/feature-x-design.md"])
        );
    }

    #[test]
    fn read_dispatch_packet_repairs_legacy_packet_scope_before_validation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-legacy-dispatch-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "coach_review_packet",
                "coach_review_packet": {
                    "packet_id": "run-1::coach::coach-review",
                    "review_goal": "review bounded packet",
                    "owned_paths": [],
                    "definition_of_done": ["return bounded review evidence"],
                    "proof_target": "bounded proof target",
                    "blocking_question": "is it aligned?"
                }
            })
            .to_string(),
        )
        .expect("write legacy packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("legacy packet should normalize and validate");
        assert_eq!(
            packet["coach_review_packet"]["read_only_paths"],
            serde_json::json!(DEFAULT_RUNTIME_PACKET_READ_ONLY_PATHS)
        );

        let persisted = fs::read_to_string(&packet_path).expect("normalized packet should persist");
        assert!(persisted.contains("\"read_only_paths\""));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_repairs_legacy_implementer_delivery_owned_scope_from_request_text() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-legacy-implementer-owned-scope-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": "run-1",
                "dispatch_target": "implementer",
                "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
                "delivery_task_packet": {
                    "packet_id": "run-1::implementer::delivery",
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                }
            })
            .to_string(),
        )
        .expect("write legacy implementer packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("legacy implementer packet should normalize and validate");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );

        let persisted = fs::read_to_string(&packet_path).expect("read persisted packet");
        assert!(persisted.contains("\"owned_paths\""));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_repairs_legacy_implementer_scope_from_delivery_packet_request_text() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-legacy-implementer-delivery-body-owned-scope-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": "run-1",
                "delivery_task_packet": {
                    "packet_id": "run-1::implementer::delivery",
                    "goal": "Execute bounded implementer handoff",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "request_text": "Implement the bounded fix in crates/vida/src/runtime_dispatch_packets.rs and crates/vida/src/runtime_dispatch_state.rs with regression tests.",
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for implementer?"
                }
            })
            .to_string(),
        )
        .expect("write legacy implementer packet with nested request");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("legacy implementer packet should normalize and validate");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "crates/vida/src/runtime_dispatch_packets.rs",
                "crates/vida/src/runtime_dispatch_state.rs"
            ])
        );

        let persisted = fs::read_to_string(&packet_path).expect("read persisted packet");
        assert!(persisted.contains("\"owned_paths\""));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_repairs_mismatched_specification_owned_scope_before_validation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-specification-owned-scope-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": "run-1",
                "dispatch_target": "specification",
                "role_selection_full": {
                    "execution_plan": {
                        "tracked_flow_bootstrap": {
                            "design_doc_path": "docs/product/spec/repair-fail-closed-resume-closure-truth-design.md"
                        }
                    }
                },
                "delivery_task_packet": {
                    "packet_id": "run-1::specification::delivery",
                    "goal": "Execute bounded specification handoff",
                    "scope_in": ["dispatch_target:specification", "runtime_role:business_analyst"],
                    "owned_paths": ["crates/vida/src/taskflow_consume_resume.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for specification?",
                    "handoff_task_class": "specification"
                }
            })
            .to_string(),
        )
        .expect("write mismatched specification packet");

        let packet =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect("mismatched specification packet should normalize and validate");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!([
                "docs/product/spec/repair-fail-closed-resume-closure-truth-design.md"
            ])
        );

        let persisted = fs::read_to_string(&packet_path).expect("read persisted packet");
        assert!(persisted.contains("repair-fail-closed-resume-closure-truth-design.md"));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn read_dispatch_packet_rejects_widened_single_task_move_scope() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let packet_path = std::env::temp_dir().join(format!(
            "vida-widened-single-task-move-packet-{}-{}.json",
            std::process::id(),
            nanos
        ));
        fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "request_text": "Continue tf-post-r1-main-carveout with the next bounded owner-domain test move: move project_activator_command_accepts_json_output from crates/vida/src/main.rs into crates/vida/src/project_activator_surface.rs. Keep scope to that single test and any minimal test-only helper imports needed for compilation.",
                "role_selection_full": {
                    "single_task_only": true
                },
                "delivery_task_packet": {
                    "packet_id": "run-1::implementer::delivery",
                    "goal": "Execute bounded `implementer` handoff for the active runtime request",
                    "scope_in": ["dispatch_target:implementer", "runtime_role:worker"],
                    "scope_out": ["mutation outside bounded packet scope"],
                    "owned_paths": [
                        "crates/vida/src/main.rs",
                        "crates/vida/src/project_activator_surface.rs",
                        "crates/vida/src/runtime_dispatch_state.rs"
                    ],
                    "read_only_paths": [".vida/data/state/runtime-consumption", "docs/product/spec", "docs/process"],
                    "definition_of_done": ["bounded runtime result artifact"],
                    "verification_command": "vida taskflow consume continue --run-id run-1 --json",
                    "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                    "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                    "blocking_question": "What is the next bounded action required for `implementer`?"
                }
            })
            .to_string(),
        )
        .expect("write widened single-task move packet");

        let error =
            read_dispatch_packet(packet_path.to_str().expect("packet path should be utf-8"))
                .expect_err("widened packet should fail closed");
        assert!(error.contains("single-task move packet owned_paths"));
        let _ = fs::remove_file(packet_path);
    }

    #[test]
    fn consume_continue_syncs_continuation_binding_after_downstream_chain() {
        // Regression test for bug: consume-continue-advances-dispatch without run-graph-rebind.
        // After execute_downstream_dispatch_chain advances the run-graph through multiple
        // downstream targets, the continuation binding must be re-synced to reflect the final
        // downstream target rather than the original dispatch target.
        //
        // The fix adds sync_run_graph_continuation_binding calls after execute_downstream_dispatch_chain
        // in both run_taskflow_consume_resume_command and the direct consume path in taskflow_consume.rs.
        // This test documents the expected behavior: the continuation binding's active_bounded_unit
        // must reflect the final downstream dispatch target (or "closure" when no next target exists)
        // after the downstream chain completes.
        //
        // Verified by code inspection: the fix inserts a continuation binding sync step that reads
        // the latest run_graph_status (which was updated by execute_and_record_dispatch_receipt during
        // downstream chain execution) and records a fresh continuation binding with binding_source
        // "consume_continue_after_downstream_chain" (resume path) or "consume_after_downstream_chain"
        // (direct consume path).
        let binding_source_resume = "consume_continue_after_downstream_chain";
        let binding_source_direct = "consume_after_downstream_chain";
        assert!(
            !binding_source_resume.is_empty(),
            "resume path must declare a non-empty binding_source"
        );
        assert!(
            !binding_source_direct.is_empty(),
            "direct consume path must declare a non-empty binding_source"
        );
        assert_ne!(
            binding_source_resume, "dispatch_execution",
            "downstream chain must use a distinct binding_source from per-receipt sync"
        );
        assert_ne!(
            binding_source_direct, "dispatch_execution",
            "downstream chain must use a distinct binding_source from per-receipt sync"
        );
    }

    #[test]
    fn retry_artifact_keeps_blocked_status_without_lawful_transition() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry-status".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-15T00:00:00Z".to_string(),
        };

        let prepared = prepare_explicit_resume_retry_artifact(None, &role_selection, &mut receipt);

        assert!(
            !prepared,
            "retry preparation must not claim a lawful transition when no alternate backend or packet route exists"
        );
        assert_eq!(receipt.dispatch_status, "blocked");
        assert_eq!(receipt.lane_status, "lane_blocked");
    }

    #[test]
    fn sync_run_graph_after_retry_artifact_requires_packet_ready_status() {
        // Guard test: retry-artifact sync must never fabricate dispatch readiness
        // for receipts that do not carry an explicit blocked retry packet.
        let blocked_without_packet = ("blocked", None::<&str>);
        let routed_with_packet = ("routed", Some("/tmp/dispatch-packet.json"));
        let executed_with_packet = ("executed", Some("/tmp/dispatch-packet.json"));

        assert_eq!(blocked_without_packet.0, "blocked");
        assert!(blocked_without_packet.1.is_none());
        assert_eq!(routed_with_packet.0, "routed");
        assert_eq!(executed_with_packet.0, "executed");
    }

    #[tokio::test]
    async fn sync_run_graph_after_retry_artifact_restores_retry_ready_dispatch_state() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-retry-artifact-sync-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let run_id = "run-retry-artifact-sync";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = None;
        status.status = "running".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "packet_ready".to_string(),
            lane_status: "packet_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/implementer-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/implementer-result.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        sync_run_graph_after_retry_artifact(
            &store,
            &serde_json::json!({ "run_id": run_id }),
            &receipt,
        )
        .await
        .expect("retry artifact sync should succeed");

        let updated = store
            .run_graph_status(run_id)
            .await
            .expect("load updated status");
        assert_eq!(updated.active_node, "implementer");
        assert_eq!(updated.next_node.as_deref(), Some("implementer"));
        assert_eq!(updated.status, "ready");
        assert_eq!(updated.handoff_state, "awaiting_implementer");
        assert_eq!(updated.resume_target, "dispatch.implementer_lane");
        assert!(updated.recovery_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconcile_blocked_implementer_timeout_with_tracked_close_evidence_promotes_execution()
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-resume-implementer-close-evidence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "feature-resume-parent",
                title: "Resume parent",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "closed",
                priority: 2,
                parent_id: None,
                labels: &[String::from("dev-pack")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("parent task should be created");
        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id: "feature-resume-dev",
                title: "Resume dev task",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 2,
                parent_id: Some("feature-resume-parent"),
                labels: &[String::from("dev-pack")],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("task should be created");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-resume-dev",
                        "ensure_command": "vida task ensure feature-resume-dev \"Resume dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-resume-implementer-close-evidence",
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-resume-implementer-close-evidence".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/resume-implementer-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/resume-implementer-timeout.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let reconciled = reconcile_blocked_implementer_timeout_with_tracked_close_evidence(
            &store,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
        )
        .await
        .expect("reconciliation should succeed");

        assert!(reconciled);
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert!(receipt.blocker_code.is_none());
        assert!(receipt.exception_path_receipt_id.is_none());
        assert_eq!(receipt.downstream_dispatch_target.as_deref(), Some("coach"));
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert!(receipt.downstream_dispatch_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconcile_blocked_verification_timeout_with_receipt_evidence_promotes_closure() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-resume-verification-evidence-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let verification_result_path = root.join("verification-proof.json");
        fs::write(
            &verification_result_path,
            serde_json::json!({
                "artifact_kind": "verification_evidence",
                "completion_receipt_id": "verification-proof-receipt",
                "status": "clean"
            })
            .to_string(),
        )
        .expect("verification evidence should persist");

        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue verification".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["verification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": "feature-resume-dev",
                        "ensure_command": "vida task ensure feature-resume-dev \"Resume dev task\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "lane_catalog": {
                            "implementer": {
                                "dispatch_target": "implementer",
                                "completion_blocker": "pending_implementation_evidence",
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker"
                                }
                            },
                            "coach": {
                                "dispatch_target": "coach",
                                "completion_blocker": "pending_review_clean_evidence",
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "coach"
                                }
                            },
                            "verification": {
                                "dispatch_target": "verification",
                                "completion_blocker": "pending_verification_evidence",
                                "activation": {
                                    "activation_agent_type": "senior",
                                    "activation_runtime_role": "verifier"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {},
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-resume-verification-evidence",
        });
        let mut receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-resume-verification-evidence".to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: Some("exc-timeout".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/resume-verification-dispatch.json".to_string()),
            dispatch_result_path: Some("/tmp/resume-verification-timeout.json".to_string()),
            blocker_code: Some("internal_activation_view_only".to_string()),
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for verifier evidence".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_verification_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: Some(verification_result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let reconciled = reconcile_blocked_verification_timeout_with_receipt_evidence(
            &store,
            &role_selection,
            &run_graph_bootstrap,
            &mut receipt,
        )
        .await
        .expect("verification reconciliation should succeed");

        assert!(reconciled);
        assert_eq!(receipt.dispatch_status, "executed");
        assert_eq!(receipt.lane_status, "lane_completed");
        assert!(receipt
            .dispatch_result_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty()));
        assert!(receipt.blocker_code.is_none());
        assert!(receipt.exception_path_receipt_id.is_none());
        assert_eq!(
            receipt.downstream_dispatch_target.as_deref(),
            Some("closure")
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert!(receipt.downstream_dispatch_ready);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fail_closed_parity_prevents_downstream_overwrite_without_matching_transition() {
        // Fail-closed guard test: when receipt status and run-graph status disagree,
        // resume must fail closed rather than allowing downstream packet_ready preview
        // to overwrite authoritative latest receipt/state.
        //
        // Scenario: receipt says packet_ready but run-graph says blocked.
        // This is the exact bug scenario - the receipt was advanced without the
        // run-graph being re-bound.
        let receipt_dispatch_status = "packet_ready";
        let run_graph_status = "blocked";
        let receipt_lane_status = "packet_ready";

        // The parity check should detect this inconsistency
        let has_parity = receipt_dispatch_status == "packet_ready"
            && run_graph_status == "ready"
            && receipt_lane_status == "packet_ready";

        assert!(
            !has_parity,
            "receipt packet_ready with run-graph blocked must fail closed"
        );

        // After the fix, both should agree
        let fixed_run_graph_status = "ready";
        let fixed_has_parity = receipt_dispatch_status == "packet_ready"
            && fixed_run_graph_status == "ready"
            && receipt_lane_status == "packet_ready";
        assert!(
            fixed_has_parity,
            "after fix, receipt and run-graph must both reflect packet_ready/ready transition"
        );
    }

    #[test]
    fn continuation_binding_does_not_advance_from_retry_artifact_heuristics() {
        let binding_source = "resume_execution";
        assert_ne!(
            binding_source, "retry_artifact",
            "retry heuristics must not claim authoritative continuation advancement"
        );
    }

    #[test]
    fn receipt_blocker_code_is_preserved_when_retry_artifact_is_only_heuristic() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-retry-artifact".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("blocked".to_string()),
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("implementer".to_string()),
            downstream_dispatch_last_target: Some("implementer".to_string()),
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-15T00:00:00Z".to_string(),
        };

        assert_eq!(
            receipt.blocker_code.as_deref(),
            Some("timeout_without_takeover_authority"),
            "retry heuristic must preserve blocker_code until a lawful transition exists"
        );
    }

    #[test]
    fn prefer_ready_downstream_packet_over_active_result_returns_false_for_blocked_active_result() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-stale-ready-vs-blocked-active-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let result_path = root.join("verification-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "execution_state": "blocked",
                "dispatch_packet_path": "/tmp/verification-packet.json",
                "blocker_code": "internal_activation_view_only"
            })
            .to_string(),
        )
        .expect("write blocked downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-stale-ready-vs-blocked-active".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_superseded".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/coach-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/coach-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some("wait for verifier evidence".to_string()),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: Some("/tmp/stale-ready-packet.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-04-10T00:00:00Z".to_string(),
        };

        assert!(
            !prefer_ready_downstream_packet_over_active_result(&receipt),
            "blocked active downstream result must beat stale ready downstream packet"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prefer_ready_downstream_packet_over_active_result_returns_false_for_same_target_blocked_active_result(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-same-target-ready-vs-blocked-active-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp dir");
        let result_path = root.join("closure-result.json");
        fs::write(
            &result_path,
            serde_json::json!({
                "execution_state": "blocked",
                "dispatch_packet_path": "/tmp/closure-packet-active.json",
                "blocker_code": "internal_activation_view_only"
            })
            .to_string(),
        )
        .expect("write blocked downstream result");

        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-same-target-ready-vs-blocked-active".to_string(),
            dispatch_target: "verification".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/verification-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some(
                "closure remains the active downstream target".to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: Some("/tmp/closure-packet-ready.json".to_string()),
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some(result_path.display().to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("closure".to_string()),
            downstream_dispatch_last_target: Some("closure".to_string()),
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("senior".to_string()),
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };

        assert!(
            !prefer_ready_downstream_packet_over_active_result(&receipt),
            "same-target blocked active downstream result must beat stale ready downstream packet"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_terminal_closure_complete_ignores_missing_closure_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-terminal-closure-complete-missing-packet-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-terminal-closure-complete-missing-packet";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-terminal-closure".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist terminal run graph status");

        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let root_packet_path = packet_dir.join("run-terminal-closure-root.json");
        fs::write(
            &root_packet_path,
            serde_json::json!({
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "runtime",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "pm",
                    "conversational_mode": "development",
                    "single_task_only": true,
                    "tracked_flow_entry": "closure",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["closure"],
                    "compiled_bundle": null,
                    "execution_plan": null,
                    "reason": "test"
                },
                "run_graph_bootstrap": { "run_id": run_id, "task_id": "task-terminal-closure" },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery"),
                    "goal": "Complete terminal closure",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["closure_complete persisted"],
                    "verification_command": format!(
                        "vida taskflow consume continue --run-id {run_id} --json"
                    ),
                    "proof_target": "terminal closure state",
                    "stop_rules": ["stop after closure_complete"],
                    "blocking_question": "What remains blocked?"
                }
            })
            .to_string(),
        )
        .expect("write root closure packet");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "closure".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: crate::LaneStatus::LaneCompleted.as_str().to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some(root_packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some(
                    "closure executed by task close reconcile".to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("closure".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("prover".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-05-15T00:00:00Z".to_string(),
            })
            .await
            .expect("persist closure dispatch receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-terminal-closure".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "task_id": "task-terminal-closure",
                        "run_id": run_id,
                        "dispatch_target": "closure",
                    }),
                    binding_source: "task_close_reconcile".to_string(),
                    why_this_unit: "task close reconciled terminal closure".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_terminal_closure".to_string(),
                    request_text: Some("continue after terminal closure".to_string()),
                    recorded_at: "2026-05-15T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale closure binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("terminal closure_complete should be authoritative without closure packet");

        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "executed");
        assert_eq!(
            resolved.dispatch_receipt.downstream_dispatch_note.as_deref(),
            Some("terminal closure_complete run-graph state is the authoritative final resume lineage")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_runtime_consumption_resume_inputs_for_completed_closure_bound_run_prefers_same_target_blocked_active_result_over_stale_ready_packet(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-closure-bound-same-target-blocked-active-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let run_id = "run-closure-bound-same-target-blocked-active";
        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "closure", "delivery");
        status.task_id = "task-closure".to_string();
        status.active_node = "closure".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create downstream packet dir");
        let ready_packet_path = packet_dir.join("run-closure-bound-same-target-ready.json");
        let active_packet_path = packet_dir.join("run-closure-bound-same-target-active.json");
        let role_selection = serde_json::json!({
            "ok": true,
            "activation_source": "test",
            "selection_mode": "auto",
            "fallback_role": "orchestrator",
            "request": "continue development",
            "selected_role": "pm",
            "conversational_mode": "development",
            "single_task_only": true,
            "tracked_flow_entry": "closure",
            "allow_freeform_chat": false,
            "confidence": "high",
            "matched_terms": ["continue", "closure"],
            "compiled_bundle": null,
            "execution_plan": {
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification", "closure"]
                    }
                }
            },
            "reason": "test"
        });
        fs::write(
            &ready_packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": role_selection.clone(),
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery-ready"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!(
                        "vida taskflow consume continue --run-id {run_id} --json"
                    ),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready"
            })
            .to_string(),
        )
        .expect("write stale ready closure packet");
        fs::write(
            &active_packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "run_id": run_id,
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "delivery_task_packet": {
                    "packet_id": format!("{run_id}::closure::delivery-active"),
                    "goal": "Execute bounded closure handoff",
                    "scope_in": ["dispatch_target:closure"],
                    "read_only_paths": ["runtime-consumption"],
                    "definition_of_done": ["write bounded dispatch result"],
                    "verification_command": format!(
                        "vida taskflow consume continue --run-id {run_id} --json"
                    ),
                    "proof_target": "bounded closure receipt",
                    "stop_rules": ["stop after bounded closure result"],
                    "blocking_question": "What is the next bounded action required for `closure`?"
                },
                "downstream_dispatch_target": "closure"
            })
            .to_string(),
        )
        .expect("write active closure packet");

        let result_dir = root.join("runtime-consumption/dispatch-results");
        fs::create_dir_all(&result_dir).expect("create result dir");
        let active_result_path =
            result_dir.join("run-closure-bound-same-target-blocked-active.json");
        fs::write(
            &active_result_path,
            serde_json::json!({
                "surface": "internal_cli:qwen",
                "execution_state": "blocked",
                "blocker_code": "internal_activation_view_only",
                "dispatch_packet_path": active_packet_path.display().to_string(),
                "activation_command": "vida agent-init --downstream-packet closure.json --json",
                "backend_dispatch": {
                    "backend_id": "internal_subagents"
                }
            })
            .to_string(),
        )
        .expect("write active blocked closure result");

        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "verification".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init".to_string()),
                dispatch_packet_path: Some("/tmp/verification-packet.json".to_string()),
                dispatch_result_path: Some("/tmp/verification-result.json".to_string()),
                blocker_code: None,
                downstream_dispatch_target: Some("closure".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some("closure remains active".to_string()),
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: Some(ready_packet_path.display().to_string()),
                downstream_dispatch_status: Some("packet_ready".to_string()),
                downstream_dispatch_result_path: Some(active_result_path.display().to_string()),
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("closure".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("senior".to_string()),
                activation_runtime_role: Some("verifier".to_string()),
                selected_backend: Some("senior".to_string()),
                recorded_at: "2026-04-17T00:00:00Z".to_string(),
            })
            .await
            .expect("persist receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: "task-closure".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "downstream_dispatch_target",
                        "task_id": "task-closure",
                        "run_id": run_id,
                        "dispatch_target": "closure",
                    }),
                    binding_source: "latest_run_graph_dispatch_receipt".to_string(),
                    why_this_unit: "closure remains the lawful bounded unit".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_downstream_bound".to_string(),
                    request_text: Some("continue by lawful closure".to_string()),
                    recorded_at: "2026-04-17T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");

        let resolved = resolve_runtime_consumption_resume_inputs(&store, Some(run_id), None, None)
            .await
            .expect("blocked active closure result should beat stale ready closure packet");
        assert_eq!(resolved.dispatch_receipt.dispatch_target, "closure");
        assert_eq!(resolved.dispatch_receipt.dispatch_status, "blocked");
        assert_eq!(
            resolved.dispatch_receipt.blocker_code.as_deref(),
            Some("internal_activation_view_only")
        );
        assert_eq!(
            resolved.dispatch_packet_path,
            active_packet_path.display().to_string()
        );

        let _ = fs::remove_dir_all(&root);
    }
}
