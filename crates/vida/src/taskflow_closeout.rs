use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;

use crate::operator_toon_report::OperatorToonField;
use crate::state_store::{work_item_is_program_container, StateStore, StateStoreError, TaskRecord};

#[derive(Debug, Clone)]
struct TaskflowCloseoutCommand {
    json: bool,
    compact: bool,
    view: String,
    fields: Option<String>,
    state_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskflowCloseoutNextAction {
    None,
    CloseEpic,
    Reconcile,
    RecoverLane,
    RunGate,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskflowCloseoutSummary {
    ready_count: usize,
    open_count: usize,
    active_agents_count: usize,
    active_lanes_count: usize,
    active_bounded_unit: serde_json::Value,
    continuation_required_now: bool,
    stale_run_graph_present: bool,
    root_local_write_allowed: bool,
    all_epics_closed: bool,
    next_action: TaskflowCloseoutNextAction,
}

#[derive(Debug, Clone, Serialize)]
struct TaskflowCloseoutPayload {
    surface: &'static str,
    status: &'static str,
    view: &'static str,
    ready_count: usize,
    open_count: usize,
    active_agents_count: usize,
    active_lanes_count: usize,
    active_bounded_unit: serde_json::Value,
    continuation_required_now: bool,
    stale_run_graph_present: bool,
    root_local_write_allowed: bool,
    all_epics_closed: bool,
    next_action: TaskflowCloseoutNextAction,
}

#[derive(Debug, Clone)]
struct AgentLaneCounts {
    active_agents_count: usize,
    active_lanes_count: usize,
}

pub(crate) async fn run_taskflow_closeout(args: &[String]) -> ExitCode {
    let command = match parse_taskflow_closeout_args(args) {
        Ok(command) => command,
        Err(message) if message == "help" => {
            print_taskflow_closeout_help();
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let store = match StateStore::open_existing_read_only(command.state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            let payload = serde_json::json!({
                "surface": "vida taskflow closeout",
                "status": "blocked",
                "view": if command.compact { "compact" } else { "compact" },
                "blocker_codes": ["state_store_unavailable"],
                "next_action": "reconcile",
                "reason": format!("open authoritative state store before closeout summary: {error}"),
            });
            if command.json {
                crate::print_json_pretty(&payload);
            } else {
                crate::operator_toon_report::print(
                    "vida taskflow closeout",
                    vec![
                        OperatorToonField::text("status", "blocked"),
                        OperatorToonField::value(
                            "blocker_codes",
                            serde_json::json!(["state_store_unavailable"]),
                        ),
                        OperatorToonField::text("next_action", "reconcile"),
                    ],
                );
            }
            return ExitCode::from(1);
        }
    };

    match build_taskflow_closeout_summary(&store).await {
        Ok(summary) => {
            let mut payload = TaskflowCloseoutPayload::from_summary(summary);
            payload.view = match command.view.as_str() {
                "full" => "full",
                "summary" => "summary",
                _ => "compact",
            };
            print_taskflow_closeout_payload(&payload, command.json, command.fields.as_deref());
            ExitCode::SUCCESS
        }
        Err(error) => {
            let payload = serde_json::json!({
                "surface": "vida taskflow closeout",
                "status": "blocked",
                "view": "compact",
                "blocker_codes": ["closeout_summary_unavailable"],
                "next_action": "reconcile",
                "reason": error.to_string(),
            });
            if command.json {
                crate::print_json_pretty(&payload);
            } else {
                crate::operator_toon_report::print(
                    "vida taskflow closeout",
                    vec![
                        OperatorToonField::text("status", "blocked"),
                        OperatorToonField::value(
                            "blocker_codes",
                            serde_json::json!(["closeout_summary_unavailable"]),
                        ),
                        OperatorToonField::text("next_action", "reconcile"),
                    ],
                );
            }
            ExitCode::from(1)
        }
    }
}

impl TaskflowCloseoutPayload {
    fn from_summary(summary: TaskflowCloseoutSummary) -> Self {
        Self {
            surface: "vida taskflow closeout",
            status: "pass",
            view: "compact",
            ready_count: summary.ready_count,
            open_count: summary.open_count,
            active_agents_count: summary.active_agents_count,
            active_lanes_count: summary.active_lanes_count,
            active_bounded_unit: summary.active_bounded_unit,
            continuation_required_now: summary.continuation_required_now,
            stale_run_graph_present: summary.stale_run_graph_present,
            root_local_write_allowed: summary.root_local_write_allowed,
            all_epics_closed: summary.all_epics_closed,
            next_action: summary.next_action,
        }
    }
}

fn parse_taskflow_closeout_args(args: &[String]) -> Result<TaskflowCloseoutCommand, String> {
    let mut json = false;
    let mut compact = false;
    let mut view = "compact".to_string();
    let mut fields: Option<String> = None;
    let mut state_dir: Option<PathBuf> = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" => return Err("help".to_string()),
            "--json" => json = true,
            "--compact" => {
                compact = true;
                view = "compact".to_string();
            }
            "--view" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("missing value for --view".to_string());
                };
                let normalized = value.trim();
                match normalized {
                    "compact" | "summary" | "full" => view = normalized.to_string(),
                    _ => return Err(format!("unsupported vida taskflow closeout view `{value}`")),
                }
            }
            "--fields" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("missing value for --fields".to_string());
                };
                fields = Some(value.clone());
            }
            "--state-dir" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("missing value for --state-dir".to_string());
                };
                state_dir = Some(PathBuf::from(value));
            }
            value => {
                return Err(format!(
                    "unexpected vida taskflow closeout argument `{value}`"
                ))
            }
        }
        index += 1;
    }

    Ok(TaskflowCloseoutCommand {
        json,
        compact: compact || !json || view == "compact" || view == "summary",
        view,
        fields,
        state_dir: state_dir.unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir),
    })
}

fn print_taskflow_closeout_help() {
    println!("VIDA TaskFlow help: closeout");
    println!();
    println!("Purpose:");
    println!(
        "  Summarize session/epic closeout readiness from authoritative TaskFlow runtime state."
    );
    println!(
        "  This is a read-only compact operator surface; it never settles, reconciles, closes, or dispatches work."
    );
    println!();
    println!("Canonical command:");
    println!(
        "  vida taskflow closeout [--compact] [--view compact|summary|full] [--fields <field,...>] [--state-dir <path>] [--json]"
    );
    println!("  Default human output is compact TOON; --json emits machine-readable JSON.");
    println!("  Use --view compact or --view summary for the compact closeout field family.");
    println!("  Use --fields to select a smaller top-level field set.");
    println!();
    println!("Returned fields:");
    println!(
        "  ready_count, open_count, active_agents_count, active_lanes_count, active_bounded_unit, continuation_required_now, stale_run_graph_present, root_local_write_allowed, all_epics_closed, next_action"
    );
    println!();
    println!("next_action enum:");
    println!("  none | close_epic | reconcile | recover_lane | run_gate");
}

pub(crate) async fn build_taskflow_closeout_summary(
    store: &StateStore,
) -> Result<TaskflowCloseoutSummary, StateStoreError> {
    let all_tasks = store.all_tasks().await?;
    let task_store = store.task_store_summary().await?;
    let latest_status = store.latest_run_graph_status().await?;
    let latest_receipt = store.latest_run_graph_dispatch_receipt_summary().await?;
    let latest_recovery = store.latest_run_graph_recovery_summary().await?;
    let explicit_binding = match store
        .latest_explicit_run_graph_continuation_binding_for_current_session()
        .await?
    {
        Some(binding) => Some(binding),
        None => {
            store
                .latest_explicit_run_graph_continuation_binding()
                .await?
        }
    };

    let (latest_run_graph_task_closed, latest_run_graph_task_missing) = match latest_status.as_ref()
    {
        Some(status) => {
            let verdict =
                crate::taskflow_run_graph_task_authority::run_graph_task_authority_verdict(
                    store, status,
                )
                .await?;
            (
                verdict.stale_for_active_projection(),
                verdict.task_missing(),
            )
        }
        None => (false, false),
    };

    let continuation_binding =
        crate::continuation_binding_summary::build_continuation_binding_summary_with_task_authority(
            explicit_binding.as_ref(),
            latest_status.as_ref(),
            latest_recovery.as_ref(),
            latest_receipt.as_ref(),
            crate::latest_terminal_consume_continue_snapshot_run_id(store.root())
                .ok()
                .flatten()
                .as_deref(),
            false,
            task_store.open_count == 0
                && task_store.in_progress_count == 0
                && task_store.ready_count == 0,
            latest_run_graph_task_closed,
            latest_run_graph_task_missing,
        );
    let taskflow_active_candidates =
        crate::continuation_binding_summary::taskflow_active_candidates_from_tasks(
            &all_tasks
                .iter()
                .filter(|task| task.status == "in_progress")
                .cloned()
                .collect::<Vec<_>>(),
        );
    let continuation_binding = crate::continuation_binding_summary::add_taskflow_active_work_truth(
        continuation_binding,
        taskflow_active_candidates,
    );
    let active_bounded_unit = continuation_binding
        .get("active_bounded_unit")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let continuation_required_now = continuation_binding
        .get("continuation_required_now")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let stale_run_graph_present = latest_run_graph_task_closed || latest_run_graph_task_missing;
    let agent_lane_counts = agent_lane_counts(latest_status.as_ref(), latest_receipt.as_ref());
    let root_local_write_allowed = merged_root_write_allowed(
        store,
        latest_receipt.as_ref(),
        latest_recovery.as_ref(),
        stale_run_graph_present,
    );
    let all_epics_closed = all_program_containers_closed(&all_tasks);
    let closeable_epic_present = closeable_epic_present(&all_tasks);
    let next_action = closeout_next_action(
        stale_run_graph_present,
        agent_lane_counts.active_agents_count,
        agent_lane_counts.active_lanes_count,
        continuation_required_now,
        closeable_epic_present,
        task_store.ready_count,
        task_store.open_count,
        all_epics_closed,
    );

    Ok(TaskflowCloseoutSummary {
        ready_count: task_store.ready_count,
        open_count: task_store.open_count,
        active_agents_count: agent_lane_counts.active_agents_count,
        active_lanes_count: agent_lane_counts.active_lanes_count,
        active_bounded_unit,
        continuation_required_now,
        stale_run_graph_present,
        root_local_write_allowed,
        all_epics_closed,
        next_action,
    })
}

fn agent_lane_counts(
    latest_status: Option<&crate::state_store::RunGraphStatus>,
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> AgentLaneCounts {
    let current_run_id = latest_status
        .map(|status| status.run_id.as_str())
        .or_else(|| latest_receipt.map(|receipt| receipt.run_id.as_str()));
    let latest_receipt = latest_receipt.filter(|receipt| {
        current_run_id
            .map(|run_id| run_id == receipt.run_id)
            .unwrap_or(true)
    });
    let active_lanes_count = latest_status
        .filter(|status| {
            !matches!(
                status.lifecycle_stage.as_str(),
                "closure_complete" | "completed" | "lane_completed"
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let active_agents_count = latest_receipt
        .filter(|receipt| {
            matches!(
                receipt.dispatch_status.as_str(),
                "routed" | "pending" | "bridge_request_pending" | "blocked"
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    AgentLaneCounts {
        active_agents_count,
        active_lanes_count,
    }
}

fn merged_root_write_allowed(
    store: &StateStore,
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    latest_run_graph_task_stale: bool,
) -> bool {
    let guard =
        crate::status_surface_write_guard::root_session_write_guard_summary_from_snapshot_path(
            crate::latest_final_runtime_consumption_snapshot_path(store.root())
                .ok()
                .flatten()
                .as_deref(),
        );
    let guard = crate::status_surface_write_guard::merge_live_exception_takeover_write_guard_with_task_authority(
        guard,
        store.root(),
        latest_receipt,
        latest_recovery,
        latest_run_graph_task_stale,
    );
    guard
        .get("root_local_write_allowed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn all_program_containers_closed(tasks: &[TaskRecord]) -> bool {
    tasks
        .iter()
        .filter(|task| work_item_is_program_container(&task.issue_type))
        .all(|task| StateStore::task_status_is_closed_like(&task.status))
}

fn closeable_epic_present(tasks: &[TaskRecord]) -> bool {
    tasks
        .iter()
        .filter(|task| work_item_is_program_container(&task.issue_type))
        .filter(|task| !StateStore::task_status_is_closed_like(&task.status))
        .any(|task| {
            StateStore::task_progress_summary_from_rows(tasks, &task.id)
                .map(|summary| summary.closure_candidate)
                .unwrap_or(false)
        })
}

fn closeout_next_action(
    stale_run_graph_present: bool,
    active_agents_count: usize,
    active_lanes_count: usize,
    continuation_required_now: bool,
    closeable_epic_present: bool,
    ready_count: usize,
    open_count: usize,
    all_epics_closed: bool,
) -> TaskflowCloseoutNextAction {
    if stale_run_graph_present {
        TaskflowCloseoutNextAction::Reconcile
    } else if active_agents_count > 0 || active_lanes_count > 0 || continuation_required_now {
        TaskflowCloseoutNextAction::RecoverLane
    } else if closeable_epic_present {
        TaskflowCloseoutNextAction::CloseEpic
    } else if ready_count > 0 || open_count > 0 || !all_epics_closed {
        TaskflowCloseoutNextAction::RunGate
    } else {
        TaskflowCloseoutNextAction::None
    }
}

fn print_taskflow_closeout_payload(
    payload: &TaskflowCloseoutPayload,
    json: bool,
    fields: Option<&str>,
) {
    let payload = serde_json::to_value(payload).expect("closeout payload should serialize");
    let payload = crate::operator_toon_report::select_fields(payload, fields);
    if json {
        crate::print_json_pretty(&payload);
        return;
    }

    println!(
        "{}",
        crate::operator_toon_report::render_value("vida taskflow closeout", payload)
    );
}
