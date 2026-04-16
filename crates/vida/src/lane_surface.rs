use std::process::ExitCode;

use serde::Serialize;

use crate::taskflow_task_bridge::proxy_state_dir;
use crate::{state_store::StateStore, ProxyArgs};

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
    Complete {
        run_id: &'a str,
        receipt_id: &'a str,
        as_json: bool,
    },
    ExceptionTakeover {
        run_id: &'a str,
        receipt_id: &'a str,
        as_json: bool,
    },
}

fn lane_usage() -> &'static str {
    "Usage: vida lane show <run-id> [--json]\n       vida lane show --latest [--json]\n       vida lane complete <run-id> --receipt-id <id> [--json]\n       vida lane exception-takeover <run-id> --receipt-id <id> [--json]"
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
        [head, run_id, rest @ ..] if head == "complete" => {
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
            Ok(LaneCommand::Complete {
                run_id,
                receipt_id,
                as_json,
            })
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
        recovery.map(|recovery| {
            recovery
                .delegation_gate
                .local_exception_takeover_gate
                .as_str()
        }),
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
    let downstream_dispatch_packet_path = summary.downstream_dispatch_packet_path.clone();
    let downstream_dispatch_result_path = summary.downstream_dispatch_result_path.clone();
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
            "downstream_dispatch_packet_path": downstream_dispatch_packet_path.clone(),
            "downstream_dispatch_result_path": downstream_dispatch_result_path.clone(),
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

struct LaneShowTruth {
    blocked: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
}

fn derive_lane_show_truth(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> LaneShowTruth {
    let mut blocked = matches!(summary.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(summary.lane_status.as_str(), "lane_blocked" | "lane_failed");
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();

    if let Some(blocker_code) = summary
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }

    if blocked {
        blocker_codes.extend(
            summary
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }

    if summary.lane_status == crate::LaneStatus::LaneExceptionRecorded.as_str() {
        blocked = true;
        if recovery.is_some_and(|recovery| {
            recovery.delegation_gate.local_exception_takeover_gate == "blocked_open_delegated_cycle"
                || recovery.delegation_gate.delegated_cycle_open
        }) {
            blocker_codes.push("open_delegated_cycle".to_string());
            next_actions.push(
                "Exception-path receipt recorded; delegated cycle is still open, so root-local write remains blocked."
                    .to_string(),
            );
        }
    }

    LaneShowTruth {
        blocked,
        blocker_codes: crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes),
        next_actions,
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

fn read_lane_packet(path: &str) -> Result<serde_json::Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted lane packet `{path}`: {error}"))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to decode persisted lane packet `{path}`: {error}"))
}

fn write_lane_packet(path: &str, packet: &serde_json::Value) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(packet)
        .map_err(|error| format!("Failed to encode persisted lane packet `{path}`: {error}"))?;
    std::fs::write(path, encoded)
        .map_err(|error| format!("Failed to write persisted lane packet `{path}`: {error}"))
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
            let recovery = store.run_graph_recovery_summary(&summary.run_id).await.ok();
            let truth = derive_lane_show_truth(&summary, recovery.as_ref());
            let envelope = build_lane_envelope(
                summary,
                status,
                truth.blocked,
                truth.blocker_codes.first().map(String::as_str),
                truth.next_actions,
            );
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
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let truth = derive_lane_show_truth(&summary, recovery.as_ref());
            let envelope = build_lane_envelope(
                summary,
                status,
                truth.blocked,
                truth.blocker_codes.first().map(String::as_str),
                truth.next_actions,
            );
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::Complete {
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
            let Some(packet_path) = receipt.downstream_dispatch_packet_path.clone() else {
                eprintln!(
                    "Lane `{run_id}` has no persisted downstream dispatch packet for bounded completion evidence."
                );
                return ExitCode::from(2);
            };
            let mut packet = match read_lane_packet(&packet_path) {
                Ok(packet) => packet,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            let completed_target = packet
                .get("downstream_dispatch_active_target")
                .and_then(serde_json::Value::as_str)
                .or(receipt.downstream_dispatch_active_target.as_deref())
                .or(receipt.downstream_dispatch_last_target.as_deref())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(receipt.dispatch_target.as_str())
                .to_string();
            let completion_result_path =
                match crate::runtime_dispatch_state::write_runtime_lane_completion_result(
                    store.root(),
                    run_id,
                    &completed_target,
                    receipt_id,
                    &packet_path,
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            packet["downstream_dispatch_ready"] = serde_json::json!(true);
            packet["downstream_dispatch_blockers"] = serde_json::json!([]);
            packet["downstream_dispatch_status"] = serde_json::json!("packet_ready");
            packet["downstream_dispatch_result_path"] =
                serde_json::json!(completion_result_path.clone());
            packet["downstream_lane_status"] = serde_json::json!("packet_ready");
            packet["downstream_dispatch_active_target"] = serde_json::json!(completed_target);
            if let Err(error) = write_lane_packet(&packet_path, &packet) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }

            receipt.downstream_dispatch_ready = true;
            receipt.downstream_dispatch_blockers.clear();
            receipt.downstream_dispatch_status = Some("packet_ready".to_string());
            receipt.downstream_dispatch_result_path = Some(completion_result_path);
            receipt.downstream_dispatch_active_target = Some(completed_target.clone());
            receipt.downstream_dispatch_last_target = Some(completed_target);
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist lane completion evidence: {error}");
                return ExitCode::from(1);
            }

            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let status = store.run_graph_status(run_id).await.ok();
            let envelope = build_lane_envelope(updated_summary, status, false, None, Vec::new());
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
                    recovery
                        .delegation_gate
                        .local_exception_takeover_gate
                        .as_str()
                }),
            )
            .is_active();
            receipt.lane_status = if takeover_active {
                crate::LaneStatus::LaneExceptionTakeover
                    .as_str()
                    .to_string()
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
    fn parse_lane_complete_supports_receipt_id_and_json() {
        let args = vec![
            "complete".to_string(),
            "run-1".to_string(),
            "--receipt-id".to_string(),
            "receipt-1".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane complete should parse");
        assert!(matches!(
            command,
            LaneCommand::Complete {
                run_id: "run-1",
                receipt_id: "receipt-1",
                as_json: true
            }
        ));
    }

    #[test]
    fn exception_takeover_requires_more_than_a_recorded_receipt_when_recovery_is_missing() {
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(
            sample_receipt("executed"),
        );
        assert!(!exception_takeover_allowed(&summary, None));
    }

    #[test]
    fn derive_lane_show_truth_marks_blocked_dispatch_receipts_as_blocked() {
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(
            sample_receipt("blocked"),
        );

        let truth = derive_lane_show_truth(&summary, None);

        assert!(truth.blocked);
        assert!(truth.next_actions.is_empty());
    }

    #[test]
    fn derive_lane_show_truth_blocks_exception_recorded_open_cycle() {
        let mut receipt = sample_receipt("executed");
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "specification",
            "scope_discussion",
        );
        status.active_node = "implementer".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.status = "ready".to_string();
        let recovery = crate::state_store::RunGraphRecoverySummary::from_status(status);

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"open_delegated_cycle".to_string()));
    }

    #[tokio::test]
    async fn lane_show_run_fails_closed_for_exception_recorded_open_cycle() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-show-run-open-cycle-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-show-open-cycle";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "implementer".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.status = "ready".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception-recorded receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec!["show".to_string(), run_id.to_string(), "--json".to_string()],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let _ = std::fs::remove_dir_all(&root);
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

    #[tokio::test]
    async fn lane_complete_records_receipt_backed_downstream_completion_evidence() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-complete";
        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-lane-complete.json");
        std::fs::create_dir_all(
            packet_path
                .parent()
                .expect("downstream packet path should have parent"),
        )
        .expect("create downstream packet dir");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_active_target": "implementer",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_implementation_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_note =
            Some("after `implementer` evidence is recorded, activate `coach`".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "completion-1".to_string(),
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
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());
        assert_eq!(
            after.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        let result_path = after
            .downstream_dispatch_result_path
            .clone()
            .expect("completion result path should be recorded");
        let result = std::fs::read_to_string(&result_path).expect("read completion result");
        let result_json: serde_json::Value =
            serde_json::from_str(&result).expect("completion result should be json");
        assert_eq!(
            result_json["artifact_kind"],
            "runtime_lane_completion_result"
        );
        assert_eq!(result_json["completion_receipt_id"], "completion-1");

        let packet = std::fs::read_to_string(&packet_path).expect("read updated packet");
        let packet_json: serde_json::Value =
            serde_json::from_str(&packet).expect("updated packet should be json");
        assert_eq!(packet_json["downstream_dispatch_ready"], true);
        assert_eq!(packet_json["downstream_dispatch_status"], "packet_ready");
        assert_eq!(packet_json["downstream_lane_status"], "packet_ready");
        assert_eq!(packet_json["downstream_dispatch_result_path"], result_path);

        let _ = std::fs::remove_dir_all(&root);
    }
}
