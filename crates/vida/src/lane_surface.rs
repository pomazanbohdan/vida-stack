use std::process::ExitCode;

use serde::Serialize;

use crate::taskflow_task_bridge::proxy_state_dir;
use crate::{ProxyArgs, state_store::StateStore};

#[derive(Serialize)]
struct LaneEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    run_id: String,
    lane_id: Option<String>,
    runtime_role: Option<String>,
    lane_status: String,
    selected_backend: Option<String>,
    dispatch_status: String,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
}

#[derive(Serialize)]
struct BlockedLaneEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    reason: String,
}

enum LaneCommand<'a> {
    ShowLatest {
        as_json: bool,
    },
    ShowRun {
        run_id: &'a str,
        as_json: bool,
    },
    ExceptionTakeover {
        run_id: &'a str,
        receipt_id: &'a str,
        as_json: bool,
    },
}

fn lane_usage() -> &'static str {
    "Usage: vida lane show <run-id> [--json]\n       vida lane show --latest [--json]\n       vida lane exception-takeover <run-id> --receipt-id <id> [--json]"
}

fn parse_lane_args<'a>(args: &'a [String]) -> Result<LaneCommand<'a>, String> {
    match args {
        [] => Err(lane_usage().to_string()),
        [flag] => {
            if matches!(flag.as_str(), "-h" | "--help") {
                Err(lane_usage().to_string())
            } else {
                Err(lane_usage().to_string())
            }
        }
        [head, rest @ ..] if head == "show" => {
            let mut as_json = false;
            let mut latest = false;
            let mut run_id = None;
            for arg in rest {
                match arg.as_str() {
                    "--json" => as_json = true,
                    "--latest" => latest = true,
                    value if !value.starts_with('-') && run_id.is_none() => run_id = Some(value),
                    _ => return Err(lane_usage().to_string()),
                }
            }
            if latest {
                if run_id.is_some() {
                    return Err(lane_usage().to_string());
                }
                return Ok(LaneCommand::ShowLatest { as_json });
            }
            let Some(run_id) = run_id else {
                return Err(lane_usage().to_string());
            };
            Ok(LaneCommand::ShowRun { run_id, as_json })
        }
        [head, run_id, rest @ ..] if head == "exception-takeover" => {
            let mut as_json = false;
            let mut receipt_id = None;
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--json" => {
                        as_json = true;
                        index += 1;
                    }
                    "--receipt-id" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        receipt_id = Some(value.as_str());
                        index += 2;
                    }
                    _ => return Err(lane_usage().to_string()),
                }
            }
            let Some(receipt_id) = receipt_id else {
                return Err(lane_usage().to_string());
            };
            Ok(LaneCommand::ExceptionTakeover {
                run_id,
                receipt_id,
                as_json,
            })
        }
        _ => Err(lane_usage().to_string()),
    }
}

#[cfg(test)]
fn exception_takeover_allowed(
    receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    crate::release1_contracts::exception_takeover_state(
        Some("pending-exception-receipt"),
        receipt.supersedes_receipt_id.as_deref(),
        recovery.map(|recovery| recovery.delegation_gate.local_exception_takeover_gate.as_str()),
    )
    .is_active()
}

fn build_lane_envelope(
    summary: crate::state_store::RunGraphDispatchReceiptSummary,
    status: Option<crate::state_store::RunGraphStatus>,
    blocked: bool,
    blocker_code: Option<&str>,
    next_actions: Vec<String>,
) -> LaneEnvelope {
    let run_id = summary.run_id.clone();
    let dispatch_packet_path = summary.dispatch_packet_path.clone();
    let dispatch_result_path = summary.dispatch_result_path.clone();
    let exception_path_receipt_id = summary.exception_path_receipt_id.clone();
    let supersedes_receipt_id = summary.supersedes_receipt_id.clone();
    let lane_status = summary.lane_status.clone();
    let dispatch_status = summary.dispatch_status.clone();
    let selected_backend = status
        .as_ref()
        .map(|status| status.selected_backend.clone())
        .or(summary.selected_backend.clone());
    LaneEnvelope {
        surface: "vida lane",
        status: if blocked { "blocked" } else { "pass" },
        trace_id: None,
        workflow_class: None,
        risk_tier: None,
        artifact_refs: serde_json::json!({
            "latest_run_graph_dispatch_receipt_id": run_id.clone(),
            "exception_path_receipt_id": exception_path_receipt_id.clone(),
            "dispatch_packet_path": dispatch_packet_path.clone(),
            "dispatch_result_path": dispatch_result_path.clone(),
        }),
        next_actions,
        blocker_codes: blocker_code
            .map(|code| vec![code.to_string()])
            .unwrap_or_default(),
        run_id,
        lane_id: status.as_ref().map(|status| status.lane_id.clone()),
        runtime_role: status
            .as_ref()
            .map(|status| status.task_class.clone())
            .or(summary.activation_runtime_role.clone()),
        lane_status,
        selected_backend,
        dispatch_status,
        supersedes_receipt_id,
        exception_path_receipt_id,
    }
}

fn emit_lane_envelope(envelope: &LaneEnvelope, as_json: bool) -> ExitCode {
    if crate::surface_render::print_surface_json(envelope, as_json, "lane surface should serialize")
    {
        return if envelope.status == "pass" {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(2)
        };
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(crate::RenderMode::Plain, "run_id", &envelope.run_id);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "lane_status",
        &envelope.lane_status,
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "dispatch_status",
        &envelope.dispatch_status,
    );
    if let Some(receipt_id) = envelope.exception_path_receipt_id.as_deref() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "exception_path_receipt_id",
            receipt_id,
        );
    }
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    if envelope.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn emit_blocked_lane_envelope(as_json: bool) -> ExitCode {
    let envelope = BlockedLaneEnvelope {
        surface: "vida lane",
        status: "blocked",
        trace_id: None,
        workflow_class: None,
        risk_tier: None,
        artifact_refs: serde_json::json!([]),
        next_actions: vec![
            "Use `vida lane show --latest --json` or `vida lane show <run-id> --json` once a lane receipt exists."
                .to_string(),
        ],
        blocker_codes: vec!["unsupported_blocker_code".to_string()],
        reason: "vida lane requires a bounded subcommand; the root surface fails closed instead of inferring one."
            .to_string(),
    };

    if crate::surface_render::print_surface_json(
        &envelope,
        as_json,
        "blocked lane surface should serialize",
    ) {
        return ExitCode::from(2);
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "blocker_codes",
        &envelope.blocker_codes.join(", "),
    );
    crate::print_surface_line(crate::RenderMode::Plain, "reason", &envelope.reason);
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    ExitCode::from(2)
}

pub(crate) async fn run_lane(args: ProxyArgs) -> ExitCode {
    if args.args.is_empty() || args.args.iter().all(|arg| arg.starts_with('-')) {
        return emit_blocked_lane_envelope(args.args.iter().any(|arg| arg == "--json"));
    }

    let command = match parse_lane_args(&args.args) {
        Ok(command) => command,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    let state_dir = proxy_state_dir();
    let store = match StateStore::open_existing(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    match command {
        LaneCommand::ShowLatest { as_json } => {
            let Some(summary) = (match store.latest_run_graph_dispatch_receipt_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("Failed to read latest lane receipt summary: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("No lane receipt found.");
                return ExitCode::from(2);
            };
            let status = match store.run_graph_status(&summary.run_id).await {
                Ok(status) => Some(status),
                Err(_) => None,
            };
            let envelope = build_lane_envelope(summary, status, false, None, Vec::new());
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::ShowRun { run_id, as_json } => {
            let Some(receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing lane receipt for `{run_id}`.");
                return ExitCode::from(2);
            };
            let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let status = match store.run_graph_status(run_id).await {
                Ok(status) => Some(status),
                Err(_) => None,
            };
            let envelope = build_lane_envelope(summary, status, false, None, Vec::new());
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::ExceptionTakeover {
            run_id,
            receipt_id,
            as_json,
        } => {
            let Some(mut receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing lane receipt for `{run_id}`.");
                return ExitCode::from(2);
            };
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            receipt.exception_path_receipt_id = Some(receipt_id.to_string());
            let takeover_active = crate::release1_contracts::exception_takeover_state(
                receipt.exception_path_receipt_id.as_deref(),
                receipt.supersedes_receipt_id.as_deref(),
                recovery.as_ref().map(|recovery| {
                    recovery.delegation_gate.local_exception_takeover_gate.as_str()
                }),
            )
            .is_active();
            receipt.lane_status = if takeover_active {
                crate::LaneStatus::LaneExceptionTakeover.as_str().to_string()
            } else {
                crate::derive_lane_status(
                    &receipt.dispatch_status,
                    receipt.supersedes_receipt_id.as_deref(),
                    receipt.exception_path_receipt_id.as_deref(),
                )
                .as_str()
                .to_string()
            };
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist exception takeover receipt: {error}");
                return ExitCode::from(1);
            }
            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let status = store.run_graph_status(run_id).await.ok();
            let envelope = if takeover_active {
                build_lane_envelope(updated_summary, status, false, None, Vec::new())
            } else {
                build_lane_envelope(
                    updated_summary,
                    status,
                    true,
                    Some("open_delegated_cycle"),
                    vec![
                        "Exception-path receipt recorded; delegated cycle is still open, so root-local write remains blocked."
                            .to_string(),
                    ],
                )
            };
            emit_lane_envelope(&envelope, as_json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn lane_surface_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn wait_for_state_unlock(state_dir: &std::path::Path) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let direct_lock_path = state_dir.join("LOCK");
        while direct_lock_path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    struct ProxyStateDirOverrideGuard;

    impl ProxyStateDirOverrideGuard {
        fn install(path: std::path::PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for ProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
    }

    fn sample_receipt(dispatch_status: &str) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-lane-test".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: dispatch_status.to_string(),
            lane_status: String::new(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
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
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-09T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn parse_lane_show_latest_supports_json() {
        let args = vec![
            "show".to_string(),
            "--latest".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane show latest should parse");
        assert!(matches!(command, LaneCommand::ShowLatest { as_json: true }));
    }

    #[test]
    fn exception_takeover_requires_more_than_a_recorded_receipt_when_recovery_is_missing() {
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(
            sample_receipt("executed"),
        );
        assert!(!exception_takeover_allowed(&summary, None));
    }

    #[tokio::test]
    async fn lane_exception_takeover_records_receipt_without_activating_local_write() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-test";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "spec-pack".to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "spec_pack_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        store
            .record_run_graph_dispatch_receipt(&sample_receipt("executed"))
            .await
            .expect("persist dispatch receipt");

        let before = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt before");
        assert_eq!(
            before.and_then(|receipt| receipt.exception_path_receipt_id),
            None
        );

        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "exception-takeover".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "receipt-1".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after lane command");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(
            after.exception_path_receipt_id.as_deref(),
            Some("receipt-1")
        );
        assert_eq!(after.lane_status, "lane_exception_recorded");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_exception_takeover_activates_once_delegated_cycle_is_clear() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-clear-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-clear";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "closure".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "closure_pending".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.handoff_state = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "exception-takeover".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "receipt-clear-1".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after lane command");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(
            after.exception_path_receipt_id.as_deref(),
            Some("receipt-clear-1")
        );
        assert_eq!(after.lane_status, "lane_exception_takeover");

        let _ = std::fs::remove_dir_all(&root);
    }
}
