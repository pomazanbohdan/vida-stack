use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::taskflow_layer4::{print_taskflow_proxy_help, run_taskflow_query, taskflow_help_topic};
use crate::taskflow_run_graph::{
    run_taskflow_recovery, run_taskflow_run_graph, run_taskflow_run_graph_mutation,
};
use crate::taskflow_spec_bootstrap::run_taskflow_bootstrap_spec;
use crate::taskflow_task_bridge::{enforce_execution_preparation_contract_gate, proxy_state_dir};
use crate::{
    print_surface_header, print_surface_line, surface_render, taskflow_consume,
    taskflow_protocol_binding, Command, ProxyArgs, RenderMode, TaskCommand, TaskReadyArgs,
};
use clap::Parser;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GraphSummaryTaskRef {
    id: String,
    display_id: Option<String>,
    title: String,
    status: String,
    priority: u32,
    issue_type: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GraphSummaryWaveBucket {
    wave_id: String,
    task_count: usize,
    ready_count: usize,
    blocked_count: usize,
    primary_ready_task: Option<GraphSummaryTaskRef>,
    primary_blocked_task: Option<GraphSummaryTaskRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextAction {
    command: String,
    surface: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextWhyNotNow {
    category: String,
    summary: String,
    blocker_codes: Vec<String>,
    blocking_surface: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextCandidateContext {
    ready_head: Option<GraphSummaryTaskRef>,
    admissible_now: bool,
    admissibility_gate: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowNextDecision {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    recommended_command: Option<String>,
    recommended_surface: Option<String>,
    primary_ready_task: Option<GraphSummaryTaskRef>,
    candidate_task_context: TaskflowNextCandidateContext,
    why_not_now: Option<TaskflowNextWhyNotNow>,
    next_action: Option<TaskflowNextAction>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowSchedulerRejectedCandidate {
    task: GraphSummaryTaskRef,
    ready_now: bool,
    active_critical_path: bool,
    reasons: Vec<String>,
    blocked_by: Vec<crate::state_store::TaskDependencyStatus>,
    parallel_blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowSchedulerReservationPreview {
    reservation_id: String,
    task_id: String,
    task: GraphSummaryTaskRef,
    launch_role: String,
    launch_index: usize,
    reservation_status: String,
    reservation_persisted: bool,
    execute_supported: bool,
    execution_attempted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowSchedulerDispatchReceiptPreview {
    receipt_id: Option<String>,
    receipt_status: String,
    receipt_persisted: bool,
    dispatch_surface: String,
    dispatch_command: String,
    dispatch_status: String,
    execute_requested: bool,
    execute_supported: bool,
    execution_attempted: bool,
    selected_task_ids: Vec<String>,
    reservation_ids: Vec<String>,
    blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TaskflowSchedulerDispatchPlan {
    status: String,
    surface: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    dry_run: bool,
    execute_requested: bool,
    execute_supported: bool,
    execution_attempted: bool,
    execution_status: String,
    scope_task_id: Option<String>,
    requested_current_task_id: Option<String>,
    selected_current_task_id: Option<String>,
    selection_source: String,
    max_parallel_agents: u64,
    ready_count: usize,
    blocked_count: usize,
    selected_primary_task: Option<GraphSummaryTaskRef>,
    selected_parallel_tasks: Vec<GraphSummaryTaskRef>,
    selected_task_ids: Vec<String>,
    reservations: Vec<TaskflowSchedulerReservationPreview>,
    dispatch_receipt: TaskflowSchedulerDispatchReceiptPreview,
    rejected_candidates: Vec<TaskflowSchedulerRejectedCandidate>,
    scheduling: crate::state_store::TaskSchedulingProjection,
}

fn graph_summary_task_ref(task: &crate::state_store::TaskRecord) -> GraphSummaryTaskRef {
    GraphSummaryTaskRef {
        id: task.id.clone(),
        display_id: task.display_id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority,
        issue_type: task.issue_type.clone(),
    }
}

fn recovery_holds_active_bound_run(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|summary| summary.delegation_gate.delegated_cycle_open)
}

fn authoritative_dispatch_blocker_codes(
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let Some(dispatch) = dispatch else {
        return Vec::new();
    };
    let mut blocker_codes = Vec::new();
    if let Some(blocker_code) = dispatch
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    if matches!(dispatch.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(
            dispatch.lane_status.as_str(),
            "lane_blocked" | "lane_failed"
        )
    {
        blocker_codes.extend(
            dispatch
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes)
}

fn normalize_scheduler_max_parallel_agents(activation_bundle: &serde_json::Value) -> u64 {
    activation_bundle["agent_system"]["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn scheduler_rejection_reasons_from_blocked_by(
    blocked_by: &[crate::state_store::TaskDependencyStatus],
) -> Vec<String> {
    if blocked_by.is_empty() {
        return vec!["graph_blocked".to_string()];
    }

    blocked_by
        .iter()
        .map(|dependency| {
            format!(
                "blocked_by:{}:{}:{}",
                dependency.edge_type, dependency.depends_on_id, dependency.dependency_status
            )
        })
        .collect()
}

fn scheduler_reservation_preview(
    task: &GraphSummaryTaskRef,
    launch_role: &str,
    launch_index: usize,
) -> TaskflowSchedulerReservationPreview {
    TaskflowSchedulerReservationPreview {
        reservation_id: format!("scheduler-preview-{launch_role}-{launch_index}-{}", task.id),
        task_id: task.id.clone(),
        task: task.clone(),
        launch_role: launch_role.to_string(),
        launch_index,
        reservation_status: "preview_unpersisted".to_string(),
        reservation_persisted: false,
        execute_supported: false,
        execution_attempted: false,
    }
}

fn build_taskflow_scheduler_dispatch_plan(
    scheduling: crate::state_store::TaskSchedulingProjection,
    max_parallel_agents: u64,
    scope_task_id: Option<&str>,
    requested_current_task_id: Option<&str>,
    dry_run: bool,
    execute_requested: bool,
) -> TaskflowSchedulerDispatchPlan {
    let selected_current_candidate = if let Some(task_id) = requested_current_task_id {
        scheduling
            .ready
            .iter()
            .find(|candidate| candidate.task.id == task_id)
    } else {
        scheduling
            .ready
            .iter()
            .find(|candidate| candidate.active_critical_path)
            .or_else(|| scheduling.ready.first())
    };
    let selected_current_task_id =
        selected_current_candidate.map(|candidate| candidate.task.id.clone());
    let selection_source = if requested_current_task_id.is_some() {
        if selected_current_task_id.is_some() {
            "requested_current_task"
        } else {
            "requested_current_task_not_ready"
        }
    } else if selected_current_candidate.is_some_and(|candidate| candidate.active_critical_path) {
        "critical_path_ready_head"
    } else if selected_current_candidate.is_some() {
        "ready_head_fallback"
    } else {
        "no_ready_primary"
    }
    .to_string();

    let selected_primary_task =
        selected_current_candidate.map(|candidate| graph_summary_task_ref(&candidate.task));
    let parallel_capacity = max_parallel_agents.saturating_sub(1) as usize;
    let mut selected_parallel_tasks = Vec::new();
    let mut rejected_candidates = Vec::new();
    let mut remaining_parallel_capacity = parallel_capacity;

    for candidate in &scheduling.ready {
        if Some(candidate.task.id.as_str()) == selected_current_task_id.as_deref() {
            continue;
        }

        if candidate.ready_parallel_safe && remaining_parallel_capacity > 0 {
            selected_parallel_tasks.push(graph_summary_task_ref(&candidate.task));
            remaining_parallel_capacity -= 1;
            continue;
        }

        let reasons = if candidate.ready_parallel_safe {
            vec!["max_parallel_agents_cap_reached".to_string()]
        } else {
            candidate.parallel_blockers.clone()
        };
        rejected_candidates.push(TaskflowSchedulerRejectedCandidate {
            task: graph_summary_task_ref(&candidate.task),
            ready_now: candidate.ready_now,
            active_critical_path: candidate.active_critical_path,
            reasons,
            blocked_by: candidate.blocked_by.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        });
    }

    for candidate in &scheduling.blocked {
        rejected_candidates.push(TaskflowSchedulerRejectedCandidate {
            task: graph_summary_task_ref(&candidate.task),
            ready_now: candidate.ready_now,
            active_critical_path: candidate.active_critical_path,
            reasons: scheduler_rejection_reasons_from_blocked_by(&candidate.blocked_by),
            blocked_by: candidate.blocked_by.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        });
    }

    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();
    if selected_primary_task.is_none() {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::NoReadyTasks,
        ) {
            blocker_codes.push(code);
        }
        if requested_current_task_id.is_some() {
            blocker_codes.push("requested_current_task_not_ready".to_string());
        }
        next_actions.push(
            "Inspect `vida taskflow graph-summary --json` before attempting scheduler dispatch."
                .to_string(),
        );
    }
    if execute_requested {
        if let Some(code) = crate::contract_profile_adapter::blocker_code(
            crate::release1_contracts::BlockerCode::Unsupported,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "This wave is preview-first. Re-run `vida taskflow scheduler dispatch --json` without `--execute` and launch the selected bounded unit through normal delegated runtime flow."
                .to_string(),
        );
    }
    if let Some(task) = selected_primary_task.as_ref() {
        next_actions.push(format!(
            "Inspect the selected primary task with `vida task show {} --json` before delegated launch.",
            task.id
        ));
    }
    if let Some(task) = selected_parallel_tasks.first() {
        next_actions.push(format!(
            "Verify co-scheduling safety for `{}` with `vida taskflow graph-summary --json` before parallel launch.",
            task.id
        ));
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    }
    .to_string();
    let mut selected_task_ids = selected_primary_task
        .iter()
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    selected_task_ids.extend(selected_parallel_tasks.iter().map(|task| task.id.clone()));
    let mut reservations = Vec::new();
    if let Some(task) = selected_primary_task.as_ref() {
        reservations.push(scheduler_reservation_preview(task, "primary", 0));
    }
    reservations.extend(
        selected_parallel_tasks
            .iter()
            .enumerate()
            .map(|(index, task)| scheduler_reservation_preview(task, "parallel", index + 1)),
    );
    let reservation_ids = reservations
        .iter()
        .map(|reservation| reservation.reservation_id.clone())
        .collect::<Vec<_>>();
    let blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    let dispatch_receipt = TaskflowSchedulerDispatchReceiptPreview {
        receipt_id: None,
        receipt_status: if execute_requested {
            "unsupported_not_persisted"
        } else {
            "preview_not_persisted"
        }
        .to_string(),
        receipt_persisted: false,
        dispatch_surface: "vida taskflow scheduler dispatch".to_string(),
        dispatch_command: if execute_requested {
            "vida taskflow scheduler dispatch --execute --json"
        } else {
            "vida taskflow scheduler dispatch --json"
        }
        .to_string(),
        dispatch_status: status.clone(),
        execute_requested,
        execute_supported: false,
        execution_attempted: false,
        selected_task_ids: selected_task_ids.clone(),
        reservation_ids,
        blocker_codes: blocker_codes.clone(),
    };

    TaskflowSchedulerDispatchPlan {
        status,
        surface: "vida taskflow scheduler dispatch".to_string(),
        blocker_codes,
        next_actions,
        dry_run,
        execute_requested,
        execute_supported: false,
        execution_attempted: false,
        execution_status: if execute_requested {
            "unsupported"
        } else {
            "preview"
        }
        .to_string(),
        scope_task_id: scope_task_id.map(str::to_string),
        requested_current_task_id: requested_current_task_id.map(str::to_string),
        selected_current_task_id,
        selection_source,
        max_parallel_agents,
        ready_count: scheduling.ready.len(),
        blocked_count: scheduling.blocked.len(),
        selected_primary_task,
        selected_parallel_tasks,
        selected_task_ids,
        reservations,
        dispatch_receipt,
        rejected_candidates,
        scheduling,
    }
}

fn terminal_completed_without_next_unit(
    status: Option<&crate::state_store::RunGraphStatus>,
) -> bool {
    status.is_some_and(|status| {
        status.status == "completed"
            && status.lifecycle_stage == "closure_complete"
            && status
                .next_node
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
    })
}

fn explicit_task_binding_matches_status(
    binding: Option<&crate::state_store::RunGraphContinuationBinding>,
    status: Option<&crate::state_store::RunGraphStatus>,
) -> bool {
    let Some(binding) = binding else {
        return false;
    };
    let Some(status) = status else {
        return false;
    };
    binding.run_id == status.run_id
        && binding.binding_source == "explicit_continuation_bind_task"
        && binding
            .active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str)
            == Some("task_graph_task")
}

fn build_taskflow_next_decision(
    ready_head: Option<&crate::state_store::TaskRecord>,
    recovery_holds_active_bound_run: bool,
    recovery_present: bool,
    latest_runtime_consumption_kind: Option<&str>,
    scope_task_id: Option<&str>,
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    latest_run_graph_status: Option<&crate::state_store::RunGraphStatus>,
    explicit_binding: Option<&crate::state_store::RunGraphContinuationBinding>,
) -> TaskflowNextDecision {
    let ready_head = ready_head.map(graph_summary_task_ref);
    let completed_without_explicit_next_unit =
        terminal_completed_without_next_unit(latest_run_graph_status)
            && !explicit_task_binding_matches_status(explicit_binding, latest_run_graph_status);
    let admissibility_gate = if recovery_holds_active_bound_run {
        "delegated_cycle_runtime_gate".to_string()
    } else if completed_without_explicit_next_unit {
        "completed_without_explicit_next_bounded_unit".to_string()
    } else if ready_head.is_some() {
        "ready_now".to_string()
    } else {
        "no_ready_task".to_string()
    };
    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();
    let candidate_task_context = TaskflowNextCandidateContext {
        ready_head: ready_head.clone(),
        admissible_now: !(recovery_holds_active_bound_run || completed_without_explicit_next_unit),
        admissibility_gate,
    };

    let (recommended_command, recommended_surface, primary_ready_task, why_not_now, next_action) =
        if recovery_holds_active_bound_run {
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::OpenDelegatedCycle,
            ) {
                blocker_codes.push(code);
            }
            blocker_codes.extend(authoritative_dispatch_blocker_codes(dispatch));

            if latest_runtime_consumption_kind == Some("final") {
                let next_action = TaskflowNextAction {
                    command: "vida taskflow consume continue --json".to_string(),
                    surface: "vida taskflow consume continue".to_string(),
                    reason: "the delegated cycle is still open, so the next lawful step is continuation rather than selecting a new backlog slice".to_string(),
                };
                next_actions.push(format!(
                    "Continue the active bound run with `{}` before considering backlog ready-head work.",
                    next_action.command
                ));
                (
                    Some(next_action.command.clone()),
                    Some(next_action.surface.clone()),
                    None,
                    Some(TaskflowNextWhyNotNow {
                        category: "delegated_cycle_runtime_gate".to_string(),
                        summary: "A delegated execution cycle is still open, so backlog ready-head work is not admissible yet.".to_string(),
                        blocker_codes: blocker_codes.clone(),
                        blocking_surface: Some("vida taskflow recovery latest".to_string()),
                    }),
                    Some(next_action),
                )
            } else {
                let next_action = TaskflowNextAction {
                    command: "vida taskflow recovery latest --json".to_string(),
                    surface: "vida taskflow recovery latest".to_string(),
                    reason: "the delegated cycle is open and execution-preparation evidence is not yet finalized".to_string(),
                };
                next_actions.push(format!(
                    "Inspect the active bound recovery state with `{}` before considering backlog ready-head work.",
                    next_action.command
                ));
                (
                    Some(next_action.command.clone()),
                    Some(next_action.surface.clone()),
                    None,
                    Some(TaskflowNextWhyNotNow {
                        category: "delegated_cycle_runtime_gate".to_string(),
                        summary: "A delegated execution cycle is open and execution-preparation evidence is not finalized.".to_string(),
                        blocker_codes: blocker_codes.clone(),
                        blocking_surface: Some("vida taskflow recovery latest".to_string()),
                    }),
                    Some(next_action),
                )
            }
        } else if completed_without_explicit_next_unit {
            let run_id = latest_run_graph_status
                .map(|status| status.run_id.as_str())
                .unwrap_or("<run-id>");
            let next_action = TaskflowNextAction {
                command: format!(
                    "vida taskflow continuation bind {run_id} --task-id <task-id> --json"
                ),
                surface: "vida taskflow continuation bind".to_string(),
                reason: "the latest run is already closure_complete and no explicit continuation binding is admissible yet".to_string(),
            };
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::NoReadyTasks,
            ) {
                blocker_codes.push(code);
            }
            next_actions.push(format!(
                "Do not continue by heuristic after closure; bind the next bounded unit explicitly with `{}`.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                Some(TaskflowNextWhyNotNow {
                    category: "completed_without_explicit_next_bounded_unit".to_string(),
                    summary: "The latest run is closure_complete with no explicit admissible continuation binding, so `vida taskflow next` must fail closed.".to_string(),
                    blocker_codes: blocker_codes.clone(),
                    blocking_surface: Some("vida taskflow continuation bind".to_string()),
                }),
                Some(next_action),
            )
        } else if let Some(task) = ready_head.clone() {
            let next_action = TaskflowNextAction {
                command: format!("vida task show {} --json", task.id),
                surface: "vida task show".to_string(),
                reason: "a backlog slice is ready now; inspect the canonical task record before dispatch".to_string(),
            };
            next_actions.push(format!(
                "Inspect the primary ready task with `{}` before dispatch.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                Some(task),
                None,
                Some(next_action),
            )
        } else if recovery_present && latest_runtime_consumption_kind == Some("final") {
            let next_action = TaskflowNextAction {
                command: "vida taskflow consume continue --json".to_string(),
                surface: "vida taskflow consume continue".to_string(),
                reason: "no ready backlog slice exists, but the latest lawful delegated chain can still continue".to_string(),
            };
            next_actions.push(format!(
                "Continue the latest lawful delegated chain with `{}`.",
                next_action.command
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                None,
                Some(next_action),
            )
        } else {
            if let Some(code) = crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::NoReadyTasks,
            ) {
                blocker_codes.push(code);
            }
            let ready_command = if let Some(task_id) = scope_task_id {
                format!("vida task ready --scope {task_id} --json")
            } else {
                "vida task ready --json".to_string()
            };
            let next_action = TaskflowNextAction {
                command: ready_command.clone(),
                surface: "vida task ready".to_string(),
                reason: "no ready backlog slice is currently admissible".to_string(),
            };
            next_actions.push(format!(
                "No ready backlog slice is available right now; inspect `{ready_command}` and `vida taskflow recovery latest --json`."
            ));
            (
                Some(next_action.command.clone()),
                Some(next_action.surface.clone()),
                None,
                Some(TaskflowNextWhyNotNow {
                    category: "backlog_no_ready_task".to_string(),
                    summary: "No ready backlog slice is currently admissible.".to_string(),
                    blocker_codes: blocker_codes.clone(),
                    blocking_surface: Some("vida task ready".to_string()),
                }),
                Some(next_action),
            )
        };

    if recovery_present {
        next_actions.push(
            "Inspect the latest recovery projection with `vida taskflow recovery latest --json`."
                .to_string(),
        );
    }

    if recovery_present && latest_runtime_consumption_kind != Some("final") {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::ExecutionPreparationGateBlocked,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "Materialize final execution-preparation evidence with `vida taskflow consume final \"<request>\" --json` before attempting continuation."
                .to_string(),
        );
    }

    blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    let why_not_now = why_not_now.map(|mut summary| {
        summary.blocker_codes = blocker_codes.clone();
        summary
    });
    let status = if blocker_codes.is_empty() {
        "pass".to_string()
    } else {
        "blocked".to_string()
    };

    TaskflowNextDecision {
        status,
        blocker_codes,
        next_actions,
        recommended_command,
        recommended_surface,
        primary_ready_task,
        candidate_task_context,
        why_not_now,
        next_action,
    }
}

fn task_wave_label(
    task_id: &str,
    by_id: &BTreeMap<String, crate::state_store::TaskRecord>,
    memo: &mut BTreeMap<String, Option<String>>,
    active: &mut BTreeSet<String>,
) -> Option<String> {
    if let Some(cached) = memo.get(task_id) {
        return cached.clone();
    }
    if !active.insert(task_id.to_string()) {
        return None;
    }

    let resolved = by_id.get(task_id).and_then(|task| {
        task.execution_semantics
            .order_bucket
            .clone()
            .or_else(|| {
                task.labels
                    .iter()
                    .find(|label| label.as_str() == "wave" || label.starts_with("wave-"))
                    .cloned()
            })
            .or_else(|| {
                task.dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type == "parent-child")
                    .find_map(|dependency| {
                        task_wave_label(&dependency.depends_on_id, by_id, memo, active)
                    })
            })
    });

    active.remove(task_id);
    memo.insert(task_id.to_string(), resolved.clone());
    resolved
}

fn wave_sort_key(wave_id: &str) -> (usize, usize, &str) {
    if wave_id == "unassigned" {
        return (2, usize::MAX, wave_id);
    }
    if wave_id == "wave" {
        return (0, 0, wave_id);
    }
    if let Some(index) = wave_id.strip_prefix("wave-") {
        if let Ok(parsed) = index.parse::<usize>() {
            return (0, parsed, wave_id);
        }
    }
    (1, usize::MAX, wave_id)
}

fn build_graph_summary_waves(
    all_tasks: &[crate::state_store::TaskRecord],
    ready_tasks: &[crate::state_store::TaskRecord],
    blocked_tasks: &[crate::state_store::BlockedTaskRecord],
) -> Vec<GraphSummaryWaveBucket> {
    #[derive(Default)]
    struct WaveAccumulator {
        task_ids: BTreeSet<String>,
        ready_count: usize,
        blocked_count: usize,
        primary_ready_task: Option<GraphSummaryTaskRef>,
        primary_blocked_task: Option<GraphSummaryTaskRef>,
    }

    let by_id = all_tasks
        .iter()
        .cloned()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    let mut memo = BTreeMap::<String, Option<String>>::new();
    let mut active = BTreeSet::<String>::new();
    let mut buckets = BTreeMap::<String, WaveAccumulator>::new();

    for task in all_tasks {
        if task.issue_type == "epic" || task.status == "closed" {
            continue;
        }
        let wave_id = task_wave_label(&task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        buckets
            .entry(wave_id)
            .or_default()
            .task_ids
            .insert(task.id.clone());
    }

    for task in ready_tasks {
        let wave_id = task_wave_label(&task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        let bucket = buckets.entry(wave_id).or_default();
        bucket.ready_count += 1;
        if bucket.primary_ready_task.is_none() {
            bucket.primary_ready_task = Some(graph_summary_task_ref(task));
        }
    }

    for record in blocked_tasks {
        let wave_id = task_wave_label(&record.task.id, &by_id, &mut memo, &mut active)
            .unwrap_or_else(|| "unassigned".to_string());
        let bucket = buckets.entry(wave_id).or_default();
        bucket.blocked_count += 1;
        if bucket.primary_blocked_task.is_none() {
            bucket.primary_blocked_task = Some(graph_summary_task_ref(&record.task));
        }
    }

    let mut waves = buckets
        .into_iter()
        .map(|(wave_id, bucket)| GraphSummaryWaveBucket {
            wave_id,
            task_count: bucket.task_ids.len(),
            ready_count: bucket.ready_count,
            blocked_count: bucket.blocked_count,
            primary_ready_task: bucket.primary_ready_task,
            primary_blocked_task: bucket.primary_blocked_task,
        })
        .collect::<Vec<_>>();
    waves.sort_by(|left, right| {
        wave_sort_key(&left.wave_id)
            .cmp(&wave_sort_key(&right.wave_id))
            .then_with(|| right.ready_count.cmp(&left.ready_count))
            .then_with(|| right.blocked_count.cmp(&left.blocked_count))
            .then_with(|| left.wave_id.cmp(&right.wave_id))
    });
    waves
}

fn parse_taskflow_next_args(
    args: &[String],
) -> Result<(bool, Option<&str>, Option<PathBuf>), &'static str> {
    if !matches!(args.first().map(String::as_str), Some("next")) {
        return Err("Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]");
    }

    let mut as_json = false;
    let mut scope_task_id = None;
    let mut state_dir = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--scope" => {
                let Some(task_id) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                    );
                };
                scope_task_id = Some(task_id.as_str());
                index += 2;
            }
            "--state-dir" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(
                        "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                    );
                };
                state_dir = Some(PathBuf::from(path));
                index += 2;
            }
            "--help" | "-h" if index == 1 && args.len() == 2 => {
                return Ok((false, Some("__help__"), None));
            }
            _ => {
                return Err(
                    "Usage: vida taskflow next [--scope <task-id>] [--state-dir <path>] [--json]",
                );
            }
        }
    }

    Ok((as_json, scope_task_id, state_dir))
}

async fn route_taskflow_doctor(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Doctor(doctor_args)) => {
                crate::doctor_surface::run_doctor(doctor_args).await
            }
            _ => {
                eprintln!("Unsupported `vida taskflow doctor` routing request.");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_status(args: &[String]) -> ExitCode {
    let argv = std::iter::once("vida".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Status(mut status_args)) => {
                if status_args.state_dir.is_none() {
                    let state_dir = match resolve_taskflow_proxy_state_dir(None) {
                        Ok(state_dir) => state_dir,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    status_args.state_dir = Some(state_dir);
                }
                crate::status_surface::run_status(status_args).await
            }
            _ => {
                eprintln!("Unsupported `vida taskflow status` routing request.");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_ready(command: TaskReadyArgs) -> ExitCode {
    let state_dir = match resolve_taskflow_proxy_state_dir(command.state_dir) {
        Ok(state_dir) => state_dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    match crate::task_surface::ready_tasks_scoped_read_only(state_dir, command.scope.as_deref())
        .await
    {
        Ok(tasks) => {
            let payload =
                serde_json::to_value(&tasks).expect("taskflow ready payload should serialize");
            if surface_render::print_surface_json(
                &payload,
                command.json,
                "taskflow ready payload should render as json",
            ) {
                return ExitCode::SUCCESS;
            }

            print_surface_header(command.render, "vida taskflow task ready");
            if tasks.is_empty() {
                print_surface_line(command.render, "ready tasks", "none");
                return ExitCode::SUCCESS;
            }

            for task in tasks {
                println!("{}\t{}\t{}", task.id, task.status, task.title);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to compute taskflow ready tasks: {error}");
            ExitCode::from(1)
        }
    }
}

fn taskflow_task_subcommand_supported(subcommand: &str) -> bool {
    matches!(
        subcommand,
        "help"
            | "import-jsonl"
            | "replace-jsonl"
            | "export-jsonl"
            | "list"
            | "show"
            | "ready"
            | "next"
            | "next-display-id"
            | "create"
            | "update"
            | "close"
            | "split"
            | "spawn-blocker"
            | "deps"
            | "reverse-deps"
            | "blocked"
            | "children"
            | "reparent-children"
            | "move-children"
            | "tree"
            | "subtree"
            | "validate-graph"
            | "dep"
            | "critical-path"
    )
}

async fn run_taskflow_replan_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow replan split <task-id> --child <task-id>:<title> --child <task-id>:<title> --reason <text> [--apply] [--json]\n       vida taskflow replan spawn-blocker <task-id> <blocker-task-id> <title> --reason <text> [--apply] [--json]";
    match args.get(1).map(String::as_str) {
        None | Some("--help" | "-h") => {
            eprintln!("{usage}");
            return ExitCode::SUCCESS;
        }
        Some("split" | "spawn-blocker") => {}
        Some(_) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    }

    let apply = args.iter().any(|value| value == "--apply");
    let mut argv = vec!["vida".to_string(), "task".to_string()];
    argv.push(
        args.get(1)
            .expect("validated replan subcommand should exist")
            .clone(),
    );
    argv.extend(
        args.iter()
            .skip(2)
            .filter(|value| value.as_str() != "--apply")
            .cloned(),
    );
    if !apply {
        argv.push("--dry-run".to_string());
    }

    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Task(task_args)) => crate::task_surface::run_task(task_args).await,
            _ => {
                eprintln!(
                    "Unsupported `vida taskflow replan` arguments. This preview surface must map to canonical `vida task` mutation commands."
                );
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn route_taskflow_task(args: &[String]) -> ExitCode {
    if let Some(subcommand) = args.get(1).map(String::as_str) {
        if !subcommand.starts_with('-') && !taskflow_task_subcommand_supported(subcommand) {
            eprintln!(
                "Unsupported `vida taskflow task` subcommand. This launcher-owned task surface fails closed instead of delegating to the external TaskFlow runtime."
            );
            return ExitCode::from(2);
        }
    }

    let mut argv = vec!["vida".to_string(), "task".to_string()];
    if matches!(args.get(1).map(String::as_str), Some("close")) {
        argv.push("close".to_string());
        argv.push("--source".to_string());
        argv.push("vida taskflow task close".to_string());
        argv.extend(args.iter().skip(2).cloned());
    } else {
        argv.extend(args.iter().skip(1).cloned());
    }
    match super::Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Some(Command::Task(task_args)) => match task_args.command {
                TaskCommand::Ready(command) => route_taskflow_ready(command).await,
                _ => crate::task_surface::run_task(task_args).await,
            },
            _ => {
                eprintln!(
                    "Unsupported `vida taskflow task` subcommand. This compatibility alias must match the canonical `vida task` contract."
                );
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn resolve_taskflow_proxy_state_dir(state_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    match state_dir {
        Some(state_dir) => Ok(state_dir),
        None => crate::resolve_runtime_project_root()
            .map(|project_root| project_root.join(crate::state_store::default_state_dir())),
    }
}

pub(crate) async fn run_taskflow_next_surface(args: &[String]) -> ExitCode {
    let (as_json, scope_task_id, state_dir) = match parse_taskflow_next_args(args) {
        Ok((_, Some("__help__"), _)) => {
            print_taskflow_proxy_help(Some("next"));
            return ExitCode::SUCCESS;
        }
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };

    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(state_dir) => state_dir,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let runtime_consumption = match crate::runtime_consumption_summary(&state_dir) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("Failed to summarize runtime-consumption state: {error}");
            return ExitCode::from(1);
        }
    };

    let ready_tasks =
        match crate::task_surface::ready_tasks_scoped_read_only(state_dir.clone(), scope_task_id)
            .await
        {
            Ok(tasks) => tasks,
            Err(error) => {
                eprintln!("Failed to compute ready tasks: {error}");
                return ExitCode::from(1);
            }
        };

    let store = match crate::task_surface::open_read_only_task_store(state_dir).await {
        Ok(store) => Some(store),
        Err(error) => {
            let message = error.to_string();
            if message.contains("LOCK") || message.contains("lock") {
                None
            } else {
                eprintln!("Failed to open authoritative state store: {error}");
                return ExitCode::from(1);
            }
        }
    };
    let latest_run_graph = match store.as_ref() {
        Some(store) => match store.latest_run_graph_status().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest run-graph status: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let recovery = match store.as_ref() {
        Some(store) => match store.latest_run_graph_recovery_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest recovery summary: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let gate = match store.as_ref() {
        Some(store) => match store.latest_run_graph_gate_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest gate summary: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let dispatch = match store.as_ref() {
        Some(store) => match store.latest_run_graph_dispatch_receipt_summary().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest dispatch receipt summary: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    let explicit_binding = match store.as_ref() {
        Some(store) => match store.latest_explicit_run_graph_continuation_binding().await {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("Failed to read latest explicit continuation binding: {error}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    let recovery_holds_active_bound_run = recovery_holds_active_bound_run(recovery.as_ref());
    let decision = build_taskflow_next_decision(
        ready_tasks.first(),
        recovery_holds_active_bound_run,
        recovery.is_some(),
        runtime_consumption.latest_kind.as_deref(),
        scope_task_id,
        dispatch.as_ref(),
        latest_run_graph.as_ref(),
        explicit_binding.as_ref(),
    );
    let payload = serde_json::json!({
        "surface": "vida taskflow next",
        "status": decision.status,
        "blocker_codes": decision.blocker_codes,
        "why_not_now": decision.why_not_now,
        "next_action": decision.next_action,
        "next_actions": decision.next_actions,
        "recommended_command": decision.recommended_command,
        "recommended_surface": decision.recommended_surface,
        "scope_task_id": scope_task_id,
        "ready_count": ready_tasks.len(),
        "primary_ready_task": decision.primary_ready_task,
        "candidate_task_context": decision.candidate_task_context,
        "latest_run_graph": latest_run_graph,
        "continuation_binding": explicit_binding,
        "recovery": recovery,
        "gate": gate,
        "dispatch": dispatch,
        "runtime_consumption": runtime_consumption,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow next");
        crate::print_surface_line(RenderMode::Plain, "status", &decision.status);
        if !decision.blocker_codes.is_empty() {
            crate::print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &decision.blocker_codes.join(", "),
            );
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        if let Some(task_id) = scope_task_id {
            crate::print_surface_line(RenderMode::Plain, "scope_task_id", task_id);
        }
        if let Some(task) = payload
            .get("primary_ready_task")
            .and_then(|value| value.get("id"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", task);
            if let Some(title) = payload
                .get("primary_ready_task")
                .and_then(|value| value.get("title"))
                .and_then(serde_json::Value::as_str)
            {
                crate::print_surface_line(RenderMode::Plain, "title", title);
            }
        } else {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", "none");
        }
        if let Some(task) = payload
            .get("candidate_task_context")
            .and_then(|value| value.get("ready_head"))
            .and_then(|value| value.get("id"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "candidate_ready_task", task);
        }
        if let Some(gate) = payload
            .get("candidate_task_context")
            .and_then(|value| value.get("admissibility_gate"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "admissibility_gate", gate);
        }
        if let Some(summary) = payload
            .get("why_not_now")
            .and_then(|value| value.get("summary"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "why_not_now", summary);
        }
        if let Some(command) = payload["recommended_command"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "recommended_command", command);
        }
        if let Some(surface) = payload["recommended_surface"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "recommended_surface", surface);
        }
        if let Some(next_action) = payload
            .get("next_action")
            .and_then(|value| value.get("reason"))
            .and_then(serde_json::Value::as_str)
        {
            crate::print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if decision.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn run_taskflow_graph_summary(args: &[String]) -> ExitCode {
    let as_json = match args {
        [head] if head == "graph-summary" => false,
        [head, flag] if head == "graph-summary" && flag == "--json" => true,
        [head, flag] if head == "graph-summary" && matches!(flag.as_str(), "--help" | "-h") => {
            print_taskflow_proxy_help(Some("graph-summary"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("Usage: vida taskflow graph-summary [--json]");
            return ExitCode::from(2);
        }
    };

    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let ready_tasks = match store.ready_tasks_scoped(None).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute ready tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let blocked_tasks = match store.blocked_tasks().await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to compute blocked tasks: {error}");
            return ExitCode::from(1);
        }
    };
    let all_tasks = match store.list_tasks(None, true).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("Failed to list tasks for wave summary: {error}");
            return ExitCode::from(1);
        }
    };
    let critical_path = match store.critical_path().await {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to compute critical path: {error}");
            return ExitCode::from(1);
        }
    };
    let scheduling = match store.scheduling_projection_scoped(None, None).await {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("Failed to compute scheduling projection: {error}");
            return ExitCode::from(1);
        }
    };
    let waves = build_graph_summary_waves(&all_tasks, &ready_tasks, &blocked_tasks);

    let primary_ready_task = scheduling.ready.first().map(|candidate| {
        serde_json::json!({
            "task": graph_summary_task_ref(&candidate.task),
            "ready_now": candidate.ready_now,
            "ready_parallel_safe": candidate.ready_parallel_safe,
            "active_critical_path": candidate.active_critical_path,
            "parallel_blockers": candidate.parallel_blockers,
            "execution_semantics": candidate.task.execution_semantics,
        })
    });
    let primary_blocked_task = blocked_tasks.first().map(|record| {
        serde_json::json!({
            "id": record.task.id,
            "display_id": record.task.display_id,
            "title": record.task.title,
            "status": record.task.status,
            "priority": record.task.priority,
            "issue_type": record.task.issue_type,
            "blocker_count": record.blockers.len(),
            "blockers": record.blockers,
        })
    });

    let mut blocker_codes = Vec::<String>::new();
    let mut next_actions = Vec::<String>::new();

    if let Some(task) = ready_tasks.first() {
        next_actions.push(format!(
            "Inspect the primary ready task with `vida task show {} --json` before dispatch.",
            task.id
        ));
    }
    if let Some(record) = blocked_tasks.first() {
        next_actions.push(format!(
            "Inspect the highest-priority blocked task with `vida task deps {} --json` before resequencing.",
            record.task.id
        ));
    }
    if critical_path.length > 0 {
        next_actions.push(
            "Inspect the current graph bottleneck with `vida task critical-path --json` before parallelizing additional work."
                .to_string(),
        );
    }
    if ready_tasks.is_empty() {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::NoReadyTasks,
        ) {
            blocker_codes.push(code);
        }
    }
    if blocked_tasks.is_empty() && critical_path.length == 0 {
        if let Some(code) = crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::TaskGraphEmpty,
        ) {
            blocker_codes.push(code);
        }
        next_actions.push(
            "No active execution graph is present; inspect `vida task list --all --json` before sequencing new work."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let payload = serde_json::json!({
        "surface": "vida taskflow graph-summary",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "ready_count": ready_tasks.len(),
        "blocked_count": blocked_tasks.len(),
        "critical_path_length": critical_path.length,
        "current_task_id": scheduling.current_task_id,
        "primary_ready_task": primary_ready_task,
        "primary_blocked_task": primary_blocked_task,
        "scheduling": scheduling,
        "waves": waves,
        "critical_path": critical_path,
    });

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow graph-summary");
        crate::print_surface_line(RenderMode::Plain, "status", status);
        if !blocker_codes.is_empty() {
            crate::print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &blocker_codes.join(", "),
            );
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_count",
            &ready_tasks.len().to_string(),
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "blocked_count",
            &blocked_tasks.len().to_string(),
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "critical_path_length",
            &critical_path.length.to_string(),
        );
        crate::print_surface_line(RenderMode::Plain, "wave_count", &waves.len().to_string());
        if let Some(task) = ready_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_ready_task", &task.id);
        }
        if let Some(task_id) = payload["current_task_id"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "current_task_id", task_id);
        }
        if let Some(record) = blocked_tasks.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_blocked_task", &record.task.id);
        }
        if let Some(wave) = waves.first() {
            crate::print_surface_line(RenderMode::Plain, "primary_wave", &wave.wave_id);
        }
        if let Some(next_action) = next_actions.first() {
            crate::print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn graph_explain_task_ref(task: &crate::state_store::TaskRecord) -> serde_json::Value {
    serde_json::json!({
        "id": task.id,
        "display_id": task.display_id,
        "title": task.title,
        "status": task.status,
        "priority": task.priority,
        "issue_type": task.issue_type,
        "execution_semantics": task.execution_semantics,
    })
}

fn build_taskflow_graph_explain_payload(
    projection: &crate::state_store::TaskSchedulingProjection,
    scope_task_id: Option<&str>,
    target_task_id: Option<&str>,
) -> serde_json::Value {
    let selected_target = target_task_id
        .map(ToOwned::to_owned)
        .or_else(|| projection.current_task_id.clone())
        .or_else(|| {
            projection
                .ready
                .first()
                .map(|candidate| candidate.task.id.clone())
        })
        .or_else(|| {
            projection
                .blocked
                .first()
                .map(|candidate| candidate.task.id.clone())
        });
    let candidate = selected_target.as_deref().and_then(|task_id| {
        projection
            .ready
            .iter()
            .chain(projection.blocked.iter())
            .find(|candidate| candidate.task.id == task_id)
    });
    let candidate_missing = selected_target.is_some() && candidate.is_none();

    let ready_now = candidate
        .map(|candidate| candidate.ready_now)
        .unwrap_or(false);
    let ready_parallel_safe = candidate
        .map(|candidate| candidate.ready_parallel_safe)
        .unwrap_or(false);
    let blocked_by = candidate
        .map(|candidate| serde_json::json!(candidate.blocked_by))
        .unwrap_or_else(|| serde_json::json!([]));
    let parallel_blockers = candidate
        .map(|candidate| serde_json::json!(candidate.parallel_blockers))
        .unwrap_or_else(|| serde_json::json!([]));
    let active_critical_path = candidate
        .map(|candidate| candidate.active_critical_path)
        .unwrap_or(false);
    let selected_as_current = candidate
        .map(|candidate| Some(candidate.task.id.as_str()) == projection.current_task_id.as_deref())
        .unwrap_or(false);
    let selected_as_parallel_after_current = candidate
        .map(|candidate| {
            projection
                .parallel_candidates_after_current
                .iter()
                .any(|task| task.id == candidate.task.id)
        })
        .unwrap_or(false);

    let mut blocker_codes = Vec::<String>::new();
    let mut blocked_reasons = Vec::<String>::new();
    let mut ready_reasons = Vec::<String>::new();
    if candidate_missing {
        blocker_codes.push("task_not_in_graph_projection".to_string());
        blocked_reasons
            .push("target task is not present in the scoped scheduling projection".to_string());
    } else if let Some(candidate) = candidate {
        if candidate.ready_now {
            ready_reasons.push("no open non-parent dependency blockers".to_string());
        } else {
            blocker_codes.push("graph_blocked".to_string());
            blocked_reasons.extend(candidate.blocked_by.iter().map(|blocker| {
                format!(
                    "{} dependency `{}` is `{}`",
                    blocker.edge_type, blocker.depends_on_id, blocker.dependency_status
                )
            }));
        }
        if candidate.ready_now && !candidate.ready_parallel_safe {
            blocker_codes.extend(candidate.parallel_blockers.iter().cloned());
        }
    } else {
        blocker_codes.push("task_graph_empty".to_string());
        blocked_reasons
            .push("no open tasks are present in the scoped scheduling projection".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();

    let next_lawful_action = if let Some(candidate) = candidate {
        if candidate.ready_now {
            serde_json::json!({
                "surface": "vida taskflow scheduler dispatch",
                "command": "vida taskflow scheduler dispatch --json",
                "reason": "task is graph-ready; use scheduler dispatch to select the next bounded launch set"
            })
        } else {
            serde_json::json!({
                "surface": "vida task deps",
                "command": format!("vida task deps {} --json", candidate.task.id),
                "reason": "task is blocked by open graph dependencies"
            })
        }
    } else {
        serde_json::json!({
            "surface": "vida task ready",
            "command": "vida task ready --json",
            "reason": "no explainable target was found in the scoped projection"
        })
    };

    let status = if candidate_missing || !blocker_codes.is_empty() || !ready_now {
        "blocked"
    } else {
        "pass"
    };
    serde_json::json!({
        "surface": "vida taskflow graph explain",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": [next_lawful_action["reason"].clone()],
        "scope_task_id": scope_task_id,
        "task_id": selected_target,
        "current_task_id": projection.current_task_id,
        "task": candidate.map(|candidate| graph_explain_task_ref(&candidate.task)),
        "ready_now": ready_now,
        "ready_reasons": ready_reasons,
        "blocked_by": blocked_by,
        "blocked_reasons": blocked_reasons,
        "ready_parallel_safe": ready_parallel_safe,
        "parallel_blockers": parallel_blockers,
        "active_critical_path": active_critical_path,
        "selected_as_current": selected_as_current,
        "selected_as_parallel_after_current": selected_as_parallel_after_current,
        "parallel_candidates_after_current": projection
            .parallel_candidates_after_current
            .iter()
            .map(graph_explain_task_ref)
            .collect::<Vec<_>>(),
        "next_lawful_action": next_lawful_action,
        "projection_source": "StateStore::scheduling_projection_scoped",
        "truth_source": "canonical_task_graph_scheduler_projection",
    })
}

async fn run_taskflow_graph_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow graph explain [task-id] [--scope <task-id>] [--current-task-id <task-id>] [--json]";
    if matches!(
        args,
        [head, flag] if head == "graph" && matches!(flag.as_str(), "--help" | "-h")
    ) || matches!(
        args,
        [head, subcommand, flag]
            if head == "graph"
                && subcommand == "explain"
                && matches!(flag.as_str(), "--help" | "-h")
    ) {
        print_taskflow_proxy_help(Some("graph"));
        return ExitCode::SUCCESS;
    }
    if !matches!(args.first().map(String::as_str), Some("graph"))
        || !matches!(args.get(1).map(String::as_str), Some("explain"))
    {
        eprintln!("{usage}");
        return ExitCode::from(2);
    }

    let mut target_task_id = None::<String>;
    let mut scope_task_id = None::<String>;
    let mut current_task_id = None::<String>;
    let mut as_json = false;
    let mut index = 2usize;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--scope" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                scope_task_id = Some(value.clone());
                index += 2;
            }
            "--current-task-id" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                current_task_id = Some(value.clone());
                index += 2;
            }
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--help" | "-h" => {
                print_taskflow_proxy_help(Some("graph"));
                return ExitCode::SUCCESS;
            }
            value if !value.starts_with('-') && target_task_id.is_none() => {
                target_task_id = Some(value.to_string());
                index += 1;
            }
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        }
    }

    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let projection = match store
        .scheduling_projection_scoped(scope_task_id.as_deref(), current_task_id.as_deref())
        .await
    {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("Failed to compute graph explain projection: {error}");
            return ExitCode::from(1);
        }
    };
    let payload = build_taskflow_graph_explain_payload(
        &projection,
        scope_task_id.as_deref(),
        target_task_id.as_deref(),
    );

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        crate::print_surface_header(RenderMode::Plain, "vida taskflow graph explain");
        crate::print_surface_line(
            RenderMode::Plain,
            "status",
            payload["status"].as_str().unwrap_or("blocked"),
        );
        if let Some(task_id) = payload["task_id"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "task_id", task_id);
        }
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_now",
            if payload["ready_now"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
        );
        crate::print_surface_line(
            RenderMode::Plain,
            "ready_parallel_safe",
            if payload["ready_parallel_safe"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
        );
        if let Some(action) = payload["next_lawful_action"]["command"].as_str() {
            crate::print_surface_line(RenderMode::Plain, "next_lawful_action", action);
        }
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

async fn run_taskflow_scheduler_surface(args: &[String]) -> ExitCode {
    let usage = "Usage: vida taskflow scheduler dispatch [--scope <task-id>] [--current-task-id <task-id>] [--state-dir <path>] [--dry-run] [--execute] [--json]";
    if matches!(
        args,
        [head, flag] if head == "scheduler" && matches!(flag.as_str(), "--help" | "-h")
    ) {
        print_taskflow_proxy_help(Some("scheduler"));
        return ExitCode::SUCCESS;
    }
    if matches!(
        args,
        [head, subcommand, flag]
            if head == "scheduler"
                && subcommand == "dispatch"
                && matches!(flag.as_str(), "--help" | "-h")
    ) {
        print_taskflow_proxy_help(Some("scheduler"));
        return ExitCode::SUCCESS;
    }
    if !matches!(args.first().map(String::as_str), Some("scheduler"))
        || !matches!(args.get(1).map(String::as_str), Some("dispatch"))
    {
        eprintln!("{usage}");
        return ExitCode::from(2);
    }

    let mut scope_task_id = None::<String>;
    let mut current_task_id = None::<String>;
    let mut state_dir = None::<PathBuf>;
    let mut as_json = false;
    let mut dry_run = false;
    let mut execute_requested = false;

    let mut index = 2usize;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--scope" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                scope_task_id = Some(value.clone());
                index += 2;
            }
            "--current-task-id" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                current_task_id = Some(value.clone());
                index += 2;
            }
            "--state-dir" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{usage}");
                    return ExitCode::from(2);
                };
                state_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--json" => {
                as_json = true;
                index += 1;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            "--execute" => {
                execute_requested = true;
                index += 1;
            }
            _ => {
                eprintln!("{usage}");
                return ExitCode::from(2);
            }
        }
    }

    if dry_run && execute_requested {
        eprintln!("{usage}");
        return ExitCode::from(2);
    }

    let state_dir = match resolve_taskflow_proxy_state_dir(state_dir) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let store = match crate::task_surface::open_read_only_task_store(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let max_parallel_agents = match crate::build_taskflow_consume_bundle_payload(&store).await {
        Ok(payload) => normalize_scheduler_max_parallel_agents(&payload.activation_bundle),
        Err(_) => 1,
    };

    let initial_projection = match store
        .scheduling_projection_scoped(scope_task_id.as_deref(), current_task_id.as_deref())
        .await
    {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("Failed to compute scheduler projection: {error}");
            return ExitCode::from(1);
        }
    };
    let selected_primary_id = if let Some(task_id) = current_task_id.as_deref() {
        initial_projection
            .ready
            .iter()
            .find(|candidate| candidate.task.id == task_id)
            .map(|candidate| candidate.task.id.clone())
    } else {
        initial_projection
            .ready
            .iter()
            .find(|candidate| candidate.active_critical_path)
            .or_else(|| initial_projection.ready.first())
            .map(|candidate| candidate.task.id.clone())
    };

    let scheduling = if let Some(primary_id) = selected_primary_id.as_deref() {
        if initial_projection.current_task_id.as_deref() == Some(primary_id) {
            initial_projection
        } else {
            match store
                .scheduling_projection_scoped(scope_task_id.as_deref(), Some(primary_id))
                .await
            {
                Ok(projection) => projection,
                Err(error) => {
                    eprintln!("Failed to recompute scheduler projection: {error}");
                    return ExitCode::from(1);
                }
            }
        }
    } else {
        initial_projection
    };

    let plan = build_taskflow_scheduler_dispatch_plan(
        scheduling,
        max_parallel_agents,
        scope_task_id.as_deref(),
        current_task_id.as_deref(),
        dry_run || !execute_requested,
        execute_requested,
    );

    if as_json {
        crate::print_json_pretty(
            &serde_json::to_value(&plan).expect("scheduler dispatch plan should serialize"),
        );
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow scheduler dispatch");
        print_surface_line(RenderMode::Plain, "status", &plan.status);
        print_surface_line(
            RenderMode::Plain,
            "max_parallel_agents",
            &plan.max_parallel_agents.to_string(),
        );
        print_surface_line(
            RenderMode::Plain,
            "selection_source",
            &plan.selection_source,
        );
        print_surface_line(
            RenderMode::Plain,
            "selected_task_count",
            &plan.selected_task_ids.len().to_string(),
        );
        if let Some(task) = plan.selected_primary_task.as_ref() {
            print_surface_line(RenderMode::Plain, "selected_primary_task", &task.id);
        }
        if !plan.selected_parallel_tasks.is_empty() {
            print_surface_line(
                RenderMode::Plain,
                "selected_parallel_tasks",
                &plan
                    .selected_parallel_tasks
                    .iter()
                    .map(|task| task.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if !plan.blocker_codes.is_empty() {
            print_surface_line(
                RenderMode::Plain,
                "blocker_codes",
                &plan.blocker_codes.join(", "),
            );
        }
        if let Some(next_action) = plan.next_actions.first() {
            print_surface_line(RenderMode::Plain, "next_action", next_action);
        }
    }

    if plan.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteDiagnosticMode {
    Explain,
    ValidateRouting,
    ConfigActuationCensus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskflowRouteDiagnosticArgs {
    mode: RouteDiagnosticMode,
    run_id: Option<String>,
    dispatch_target: Option<String>,
    runtime_role: Option<String>,
    as_json: bool,
}

fn parse_taskflow_route_diagnostic_args(
    args: &[String],
) -> Result<TaskflowRouteDiagnosticArgs, &'static str> {
    let (mode, mut index) = match args {
        [head, subcommand, ..] if head == "route" && subcommand == "explain" => {
            (RouteDiagnosticMode::Explain, 2)
        }
        [head, ..] if head == "validate-routing" => (RouteDiagnosticMode::ValidateRouting, 1),
        [head, subcommand, ..] if head == "config-actuation" && subcommand == "census" => {
            (RouteDiagnosticMode::ConfigActuationCensus, 2)
        }
        _ => {
            return Err(
                "Usage: vida taskflow route explain [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow validate-routing [--run-id <run-id>] [--json]\n       vida taskflow config-actuation census [--run-id <run-id>] [--json]",
            );
        }
    };

    let mut parsed = TaskflowRouteDiagnosticArgs {
        mode,
        run_id: None,
        dispatch_target: None,
        runtime_role: None,
        as_json: false,
    };
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                parsed.as_json = true;
                index += 1;
            }
            "--run-id" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Missing value for --run-id");
                };
                parsed.run_id = Some(value.clone());
                index += 2;
            }
            "--dispatch-target" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Missing value for --dispatch-target");
                };
                parsed.dispatch_target = Some(value.clone());
                index += 2;
            }
            "--runtime-role" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("Missing value for --runtime-role");
                };
                parsed.runtime_role = Some(value.clone());
                index += 2;
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: vida taskflow route explain [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow validate-routing [--run-id <run-id>] [--json]\n       vida taskflow config-actuation census [--run-id <run-id>] [--json]",
                );
            }
            _ => {
                return Err(
                    "Usage: vida taskflow route explain [--run-id <run-id>] [--dispatch-target <target>|--runtime-role <role>] [--json]\n       vida taskflow validate-routing [--run-id <run-id>] [--json]\n       vida taskflow config-actuation census [--run-id <run-id>] [--json]",
                );
            }
        }
    }
    if parsed.dispatch_target.is_some() && parsed.runtime_role.is_some() {
        return Err("Use either --dispatch-target or --runtime-role, not both.");
    }
    if matches!(
        parsed.mode,
        RouteDiagnosticMode::ValidateRouting | RouteDiagnosticMode::ConfigActuationCensus
    ) && (parsed.dispatch_target.is_some() || parsed.runtime_role.is_some())
    {
        return Err(
            "vida taskflow validate-routing and config-actuation census inspect all routed lanes and do not accept --dispatch-target or --runtime-role.",
        );
    }
    Ok(parsed)
}

async fn latest_or_requested_dispatch_context(
    store: &crate::state_store::StateStore,
    run_id: Option<&str>,
) -> Result<Option<crate::state_store::RunGraphDispatchContext>, crate::state_store::StateStoreError>
{
    let run_id = match run_id {
        Some(run_id) => Some(run_id.to_string()),
        None => store.latest_run_graph_run_id().await?,
    };
    let Some(run_id) = run_id else {
        return Ok(None);
    };
    store.run_graph_dispatch_context(&run_id).await
}

fn string_array_from_payload(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn normalize_taskflow_diagnostic_operator_contract_payload(
    mut payload: serde_json::Value,
    fallback_blocked_next_action: &str,
) -> Result<serde_json::Value, String> {
    let blocker_codes = string_array_from_payload(&payload["blocker_codes"]);
    let blocker_codes = crate::operator_contracts::normalize_blocker_codes(
        &blocker_codes,
        crate::release_contract_adapters::canonical_blocker_codes,
        crate::contract_profile_adapter::blocker_code(
            crate::release1_contracts::BlockerCode::Unsupported,
        ),
    );
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else {
        let mut next_actions = string_array_from_payload(&payload["next_actions"]);
        if next_actions.is_empty() {
            next_actions.push(fallback_blocked_next_action.to_string());
        }
        next_actions
    };
    let artifact_refs = payload
        .get("artifact_refs")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        artifact_refs,
    )?;
    let Some(object) = payload.as_object_mut() else {
        return Err("taskflow diagnostic payload must be a JSON object".to_string());
    };
    object.insert(
        "status".to_string(),
        serde_json::Value::String(finalized.status.to_string()),
    );
    object.insert(
        "blocker_codes".to_string(),
        serde_json::to_value(finalized.blocker_codes)
            .expect("normalized blocker_codes should serialize"),
    );
    object.insert(
        "next_actions".to_string(),
        serde_json::to_value(finalized.next_actions)
            .expect("normalized next_actions should serialize"),
    );
    object.insert("artifact_refs".to_string(), finalized.artifact_refs);
    object.insert("shared_fields".to_string(), finalized.shared_fields);
    object.insert(
        "operator_contracts".to_string(),
        finalized.operator_contracts,
    );
    Ok(payload)
}

fn normalize_taskflow_route_diagnostic_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    normalize_taskflow_diagnostic_operator_contract_payload(
        payload,
        "inspect route diagnostic blockers with `vida taskflow route explain --json` or `vida taskflow validate-routing --json`",
    )
}

fn execution_plan_from_dispatch_context(
    context: &crate::state_store::RunGraphDispatchContext,
) -> Option<&serde_json::Value> {
    context
        .role_selection
        .get("execution_plan")
        .filter(|value| value.is_object())
}

fn selected_backend_readiness_payload(
    selected_backend: &str,
    preferred_profile_id: Option<&str>,
) -> Option<serde_json::Value> {
    let project_root = crate::state_store::repo_root();
    let overlay =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(&project_root).ok()?;
    let backend_entry =
        crate::yaml_lookup(&overlay, &["agent_system", "subagents", selected_backend])?;
    let backend_class = crate::yaml_lookup(backend_entry, &["subagent_backend_class"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if backend_class != "external_cli" {
        return None;
    }
    Some(
        crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
            selected_backend,
            backend_entry,
            preferred_profile_id,
        ),
    )
}

fn route_payload_for_dispatch_target(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> serde_json::Value {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        execution_plan,
        dispatch_target,
    );
    let mut payload =
        crate::taskflow_routing::route_explain_payload(execution_plan, dispatch_target, route);
    let preferred_profile_id = payload["selected_model_profile_id"].as_str();
    if let Some(selected_backend) = payload["selected_backend"].as_str() {
        if let Some(readiness) =
            selected_backend_readiness_payload(selected_backend, preferred_profile_id)
        {
            let readiness_blockers = if readiness["blocked"].as_bool().unwrap_or(false) {
                serde_json::json!([{
                    "backend_id": readiness["backend_id"].clone(),
                    "status": readiness["status"].clone(),
                    "blocker_code": readiness["blocker_code"].clone(),
                    "selected_model_profile": readiness["selected_model_profile"].clone(),
                    "next_actions": readiness["next_actions"].clone(),
                }])
            } else {
                serde_json::json!([])
            };
            if let Some(object) = payload.as_object_mut() {
                object.insert("selected_backend_readiness".to_string(), readiness);
                object.insert("readiness_blockers".to_string(), readiness_blockers);
            }
        }
    }
    let selected_backend = payload["selected_backend"].as_str();
    let admissible = selected_backend.map(|backend| {
        crate::runtime_dispatch_state::backend_is_admissible_or_runtime_selected_carrier_for_dispatch_target(
            execution_plan,
            backend,
            dispatch_target,
        )
    });
    let status = crate::taskflow_routing::route_explain_status(&payload, admissible);
    let blocker_codes = crate::taskflow_routing::route_explain_blocker_codes(&payload, admissible);
    if let Some(object) = payload.as_object_mut() {
        object.insert("status".to_string(), serde_json::Value::String(status));
        object.insert(
            "blocker_codes".to_string(),
            serde_json::to_value(blocker_codes)
                .expect("route explain blocker codes should serialize"),
        );
        object.insert(
            "selected_backend_admissible".to_string(),
            admissible.map_or(serde_json::Value::Null, serde_json::Value::Bool),
        );
    }
    payload
}

fn route_validate_targets(execution_plan: &serde_json::Value) -> Vec<String> {
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let mut targets =
        crate::taskflow_routing::dispatch_contract_execution_lane_sequence(dispatch_contract);
    if targets.is_empty() {
        targets.extend(
            ["implementation", "coach", "verification"]
                .into_iter()
                .map(str::to_string),
        );
    }
    let mut unique = BTreeSet::new();
    targets
        .into_iter()
        .map(|target| match target.as_str() {
            "implementer" | "analysis" => "implementation".to_string(),
            "execution_preparation" => "architecture".to_string(),
            _ => target,
        })
        .filter(|target| !target.trim().is_empty())
        .filter(|target| unique.insert(target.clone()))
        .collect()
}

fn value_configured(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(values) => !values.is_empty(),
    }
}

fn config_actuation_proof_status(
    route: &serde_json::Value,
    field: &str,
    configured: bool,
) -> &'static str {
    if !configured {
        return "not_configured";
    }
    if field.ends_with("model_selection_enabled")
        && route["model_selection_enabled"].as_bool() == Some(false)
    {
        return "validated_blocking";
    }
    if field.ends_with("candidate_scope")
        && !matches!(
            route["candidate_scope"].as_str(),
            Some("unified_carrier_model_profiles") | None
        )
    {
        return "validated_blocking";
    }
    if route["runtime_assignment_enabled"].as_bool() == Some(false)
        && field.starts_with("carrier_runtime_assignment.")
    {
        return "validated_blocking";
    }
    "actuated_or_validated"
}

fn config_actuation_census_row(
    route: &serde_json::Value,
    config_key: &str,
    value_key: &str,
    validator: &str,
    runtime_consumer: &str,
) -> serde_json::Value {
    let value = route
        .get(value_key)
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let configured = value_configured(&value);
    serde_json::json!({
        "config_key": config_key,
        "value": value,
        "configured": configured,
        "validator": validator,
        "runtime_consumer": runtime_consumer,
        "operator_surface": "vida taskflow route explain / vida taskflow validate-routing",
        "proof_status": config_actuation_proof_status(route, config_key, configured),
    })
}

fn config_actuation_census_rows_for_route(route: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut rows = vec![
        config_actuation_census_row(
            route,
            "executor_backend",
            "route_primary_backend",
            "selected_backend_from_execution_plan_route",
            "explicit_executor_backend_from_route",
        ),
        config_actuation_census_row(
            route,
            "fallback_executor_backend",
            "fallback_backend",
            "selected_backend_from_execution_plan_route",
            "fallback_executor_backend_from_route",
        ),
        config_actuation_census_row(
            route,
            "fanout_executor_backends",
            "fanout_backends",
            "selected_backend_from_execution_plan_route",
            "fanout_executor_backends_from_route",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.selected_backend_id",
            "runtime_assignment_backend",
            "route_explain_status / route_explain_blocker_codes",
            "runtime_assignment_backend_for_route",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.model_selection_enabled",
            "model_selection_enabled",
            "route_explain_status / route_explain_blocker_codes",
            "route_explain_status",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.candidate_scope",
            "candidate_scope",
            "route_explain_status / route_explain_blocker_codes",
            "route_explain_status",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.selected_model_profile_id",
            "selected_model_profile_id",
            "route_explain_payload",
            "selected_backend_readiness_payload",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.selected_reasoning_effort",
            "selected_reasoning_effort",
            "route_explain_payload",
            "selected_candidate_from_assignment",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.budget_policy",
            "budget_policy",
            "route_explain_payload",
            "selected_candidate_from_assignment",
        ),
        config_actuation_census_row(
            route,
            "carrier_runtime_assignment.max_budget_units",
            "max_budget_units",
            "route_explain_payload",
            "selected_candidate_from_assignment",
        ),
    ];
    rows.extend(
        route["route_field_truth"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|truth| {
                let field = truth["field"].as_str()?;
                Some(serde_json::json!({
                    "config_key": field,
                    "value": serde_json::Value::Null,
                    "configured": true,
                    "validator": "route_field_truth",
                    "runtime_consumer": serde_json::Value::Null,
                    "operator_surface": "vida taskflow validate-routing",
                    "proof_status": truth["truth"],
                    "effect": truth["effect"],
                }))
            }),
    );
    rows
}

pub(crate) fn model_profile_readiness_audit_payload_for_route(
    dispatch_target: &str,
    route: &serde_json::Value,
) -> serde_json::Value {
    let selected_model_profile_id = route["selected_model_profile_id"].clone();
    let selected_backend_readiness = route
        .get("selected_backend_readiness")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let readiness_blocked = selected_backend_readiness["blocked"].as_bool() == Some(true);
    let readiness_status = selected_backend_readiness["status"]
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            if selected_model_profile_id.as_str().is_some() {
                Some("unknown".to_string())
            } else {
                None
            }
        });
    let readiness_ready = selected_backend_readiness["blocked"]
        .as_bool()
        .map(|blocked| !blocked);
    let mut blocker_codes = Vec::new();
    if selected_model_profile_id.as_str().is_none() {
        blocker_codes.push("selected_model_profile_missing".to_string());
    }
    if readiness_blocked {
        blocker_codes.push("selected_model_profile_not_ready".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    let next_actions = if blocker_codes.is_empty() {
        Vec::<String>::new()
    } else {
        vec![
            "inspect selected_backend_readiness, selection_source_paths, and rejected_alternatives before enabling model-profile execution".to_string(),
        ]
    };
    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    serde_json::json!({
        "surface": "vida taskflow model-profile readiness audit",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "dispatch_target": dispatch_target,
        "route_status": route["status"],
        "selected_profile": {
            "profile_id": selected_model_profile_id,
            "model_ref": route["selected_model_ref"],
            "provider": route["selected_model_provider"],
            "reasoning_effort": route["selected_reasoning_effort"],
            "reasoning_control_mode": route["selected_reasoning_control_mode"],
            "selected_backend": route["selected_backend"],
            "selected_carrier_id": route["selected_carrier_id"],
            "readiness_status": readiness_status,
            "readiness_ready": readiness_ready,
            "readiness": selected_backend_readiness,
        },
        "source_paths": route["selection_source_paths"],
        "override_reasons": route["selection_override_reasons"],
        "selection_precedence": route["selection_precedence"],
        "selected_route_profile_mapping": route["selected_route_profile_mapping"],
        "selected_candidate": route["selected_candidate"],
        "candidate_pool": route["candidate_pool"],
        "rejected_alternatives": route["rejected_candidates"],
        "readiness_blockers": route["readiness_blockers"],
        "budget": {
            "policy": route["budget_policy"],
            "verdict": route["budget_verdict"],
            "max_budget_units": route["max_budget_units"],
            "selected_over_budget": route["selected_over_budget"],
            "scope": route["budget_scope"],
            "selection_budget": route["selection_budget"],
            "runtime_budget_ledger": route["runtime_budget_ledger"],
        },
    })
}

fn build_config_actuation_census_payload(
    context: &crate::state_store::RunGraphDispatchContext,
    execution_plan: &serde_json::Value,
) -> serde_json::Value {
    let routes = route_validate_targets(execution_plan)
        .into_iter()
        .map(|target| {
            let route = route_payload_for_dispatch_target(execution_plan, &target);
            let model_profile_readiness_audit =
                model_profile_readiness_audit_payload_for_route(&target, &route);
            let rows = config_actuation_census_rows_for_route(&route);
            serde_json::json!({
                "dispatch_target": target,
                "status": route["status"],
                "selected_backend": route["selected_backend"],
                "selection_source": route["selection_source"],
                "model_profile_readiness_audit": model_profile_readiness_audit,
                "rows": rows,
            })
        })
        .collect::<Vec<_>>();
    let row_count = routes
        .iter()
        .filter_map(|route| route["rows"].as_array().map(Vec::len))
        .sum::<usize>();
    serde_json::json!({
        "surface": "vida taskflow config-actuation census",
        "status": "pass",
        "blocker_codes": [],
        "run_id": context.run_id,
        "task_id": context.task_id,
        "scope": "routing_model_selection_keys",
        "route_count": routes.len(),
        "row_count": row_count,
        "routes": routes,
    })
}

async fn run_taskflow_route_diagnostic(args: &[String]) -> ExitCode {
    let parsed = match parse_taskflow_route_diagnostic_args(args) {
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    let store = match crate::state_store::StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let context = match latest_or_requested_dispatch_context(&store, parsed.run_id.as_deref()).await
    {
        Ok(Some(context)) => context,
        Ok(None) => {
            let payload = serde_json::json!({
                "surface": match parsed.mode {
                    RouteDiagnosticMode::Explain => "vida taskflow route explain",
                    RouteDiagnosticMode::ValidateRouting => "vida taskflow validate-routing",
                    RouteDiagnosticMode::ConfigActuationCensus => {
                        "vida taskflow config-actuation census"
                    }
                },
                "status": "blocked",
                "blocker_codes": ["run_graph_dispatch_context_missing"],
                "run_id": parsed.run_id,
            });
            let payload =
                normalize_taskflow_route_diagnostic_payload(payload).unwrap_or_else(|_| {
                    serde_json::json!({
                        "surface": "vida taskflow diagnostic",
                        "status": "blocked",
                        "blocker_codes": ["unsupported_blocker_code"],
                        "next_actions": ["inspect diagnostic blockers"]
                    })
                });
            crate::print_json_pretty(&payload);
            return ExitCode::from(1);
        }
        Err(error) => {
            eprintln!("Failed to read run-graph dispatch context: {error}");
            return ExitCode::from(1);
        }
    };
    let Some(execution_plan) = execution_plan_from_dispatch_context(&context) else {
        eprintln!(
            "Run `{}` has no object role_selection.execution_plan in dispatch context.",
            context.run_id
        );
        return ExitCode::from(1);
    };

    let payload = match parsed.mode {
        RouteDiagnosticMode::Explain => {
            let dispatch_target = match parsed.dispatch_target {
                Some(target) => target,
                None => match parsed.runtime_role.as_deref() {
                    Some(role) => match crate::taskflow_routing::dispatch_target_for_runtime_role(
                        execution_plan,
                        role,
                    ) {
                        Some(target) => target,
                        None => {
                            eprintln!(
                                "Unable to resolve dispatch target for runtime role `{role}`."
                            );
                            return ExitCode::from(1);
                        }
                    },
                    None => "implementation".to_string(),
                },
            };
            let explain = route_payload_for_dispatch_target(execution_plan, &dispatch_target);
            serde_json::json!({
                "surface": "vida taskflow route explain",
                "status": explain["status"],
                "blocker_codes": explain["blocker_codes"],
                "run_id": context.run_id,
                "task_id": context.task_id,
                "dispatch_target": dispatch_target,
                "route": explain,
            })
        }
        RouteDiagnosticMode::ValidateRouting => {
            let routes = route_validate_targets(execution_plan)
                .into_iter()
                .map(|target| route_payload_for_dispatch_target(execution_plan, &target))
                .collect::<Vec<_>>();
            let blocker_codes = routes
                .iter()
                .flat_map(|route| {
                    route["blocker_codes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let status = if blocker_codes.is_empty() {
                "pass"
            } else {
                "blocked"
            };
            serde_json::json!({
                "surface": "vida taskflow validate-routing",
                "status": status,
                "blocker_codes": blocker_codes,
                "run_id": context.run_id,
                "task_id": context.task_id,
                "route_count": routes.len(),
                "routes": routes,
            })
        }
        RouteDiagnosticMode::ConfigActuationCensus => {
            build_config_actuation_census_payload(&context, execution_plan)
        }
    };
    let payload = normalize_taskflow_route_diagnostic_payload(payload).unwrap_or_else(|_| {
        serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "status": "blocked",
            "blocker_codes": ["unsupported_blocker_code"],
            "next_actions": ["inspect diagnostic blockers"]
        })
    });

    if parsed.as_json {
        crate::print_json_pretty(&payload);
    } else {
        let surface = payload["surface"].as_str().unwrap_or("vida taskflow route");
        print_surface_header(RenderMode::Plain, surface);
        print_surface_line(
            RenderMode::Plain,
            "status",
            payload["status"].as_str().unwrap_or("unknown"),
        );
        if let Some(run_id) = payload["run_id"].as_str() {
            print_surface_line(RenderMode::Plain, "run_id", run_id);
        }
        if let Some(target) = payload["dispatch_target"].as_str() {
            print_surface_line(RenderMode::Plain, "dispatch_target", target);
        }
        if let Some(backend) = payload
            .get("route")
            .and_then(|route| route.get("selected_backend"))
            .and_then(serde_json::Value::as_str)
        {
            print_surface_line(RenderMode::Plain, "selected_backend", backend);
        }
        if let Some(source) = payload
            .get("route")
            .and_then(|route| route.get("selection_source"))
            .and_then(serde_json::Value::as_str)
        {
            print_surface_line(RenderMode::Plain, "selection_source", source);
        }
        if let Some(count) = payload["route_count"].as_u64() {
            print_surface_line(RenderMode::Plain, "route_count", &count.to_string());
        }
        if let Some(blockers) = payload["blocker_codes"]
            .as_array()
            .filter(|items| !items.is_empty())
        {
            let joined = blockers
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            print_surface_line(RenderMode::Plain, "blocker_codes", &joined);
        }
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_graph_summary_waves, build_taskflow_scheduler_dispatch_plan,
        taskflow_task_subcommand_supported, GraphSummaryWaveBucket,
    };
    use crate::state_store::{
        BlockedTaskRecord, TaskDependencyRecord, TaskDependencyStatus, TaskRecord,
        TaskSchedulingCandidate, TaskSchedulingProjection,
    };

    fn task(
        id: &str,
        issue_type: &str,
        status: &str,
        priority: u32,
        labels: &[&str],
        dependencies: Vec<TaskDependencyRecord>,
    ) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: format!("task {id}"),
            description: String::new(),
            issue_type: issue_type.to_string(),
            status: status.to_string(),
            priority,
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            updated_at: "0".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            dependencies,
        }
    }

    fn parent_dependency(issue_id: &str, depends_on_id: &str) -> TaskDependencyRecord {
        TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: "parent-child".to_string(),
            created_at: "0".to_string(),
            created_by: "test".to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        }
    }

    fn scheduling_candidate(
        task: TaskRecord,
        ready_now: bool,
        ready_parallel_safe: bool,
        active_critical_path: bool,
        blocked_by: Vec<TaskDependencyStatus>,
        parallel_blockers: Vec<&str>,
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task,
            ready_now,
            ready_parallel_safe,
            blocked_by,
            active_critical_path,
            parallel_blockers: parallel_blockers.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn graph_summary_waves_follow_parent_chain_wave_labels() {
        let wave_epic = task("wave-0-epic", "epic", "open", 1, &["wave-0"], Vec::new());
        let child_ready = task(
            "r1-ready",
            "task",
            "open",
            1,
            &[],
            vec![parent_dependency("r1-ready", "wave-0-epic")],
        );
        let child_blocked = task(
            "r1-blocked",
            "task",
            "in_progress",
            2,
            &[],
            vec![parent_dependency("r1-blocked", "wave-0-epic")],
        );
        let unassigned = task("r1-unassigned", "task", "open", 3, &[], Vec::new());

        let all_tasks = vec![
            wave_epic.clone(),
            child_ready.clone(),
            child_blocked.clone(),
            unassigned.clone(),
        ];
        let ready_tasks = vec![child_ready.clone(), unassigned.clone()];
        let blocked_tasks = vec![BlockedTaskRecord {
            task: child_blocked.clone(),
            blockers: Vec::new(),
        }];

        let waves = build_graph_summary_waves(&all_tasks, &ready_tasks, &blocked_tasks);
        assert_eq!(
            waves,
            vec![
                GraphSummaryWaveBucket {
                    wave_id: "wave-0".to_string(),
                    task_count: 2,
                    ready_count: 1,
                    blocked_count: 1,
                    primary_ready_task: Some(super::graph_summary_task_ref(&child_ready)),
                    primary_blocked_task: Some(super::graph_summary_task_ref(&child_blocked)),
                },
                GraphSummaryWaveBucket {
                    wave_id: "unassigned".to_string(),
                    task_count: 1,
                    ready_count: 1,
                    blocked_count: 0,
                    primary_ready_task: Some(super::graph_summary_task_ref(&unassigned)),
                    primary_blocked_task: None,
                },
            ]
        );
    }

    #[test]
    fn graph_summary_waves_prefer_order_bucket_over_labels() {
        let mut explicit_bucket = task("bucket-task", "task", "open", 1, &["wave-99"], Vec::new());
        explicit_bucket.execution_semantics.order_bucket = Some("wave-2".to_string());

        let waves = build_graph_summary_waves(
            std::slice::from_ref(&explicit_bucket),
            std::slice::from_ref(&explicit_bucket),
            &[],
        );
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].wave_id, "wave-2");
        assert_eq!(waves[0].ready_count, 1);
    }

    #[test]
    fn scheduler_dispatch_plan_prefers_critical_path_and_respects_parallel_cap() {
        let mut primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        primary.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        primary.execution_semantics.order_bucket = Some("wave-a".to_string());
        primary.execution_semantics.parallel_group = Some("docs".to_string());
        primary.execution_semantics.conflict_domain = Some("critical".to_string());

        let mut sibling_a = task("parallel-a", "task", "open", 2, &[], Vec::new());
        sibling_a.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        sibling_a.execution_semantics.order_bucket = Some("wave-a".to_string());
        sibling_a.execution_semantics.parallel_group = Some("docs".to_string());
        sibling_a.execution_semantics.conflict_domain = Some("parallel-a".to_string());

        let mut sibling_b = task("parallel-b", "task", "open", 3, &[], Vec::new());
        sibling_b.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        sibling_b.execution_semantics.order_bucket = Some("wave-a".to_string());
        sibling_b.execution_semantics.parallel_group = Some("docs".to_string());
        sibling_b.execution_semantics.conflict_domain = Some("parallel-b".to_string());

        let mut unsafe_sibling = task("sequential-only", "task", "open", 4, &[], Vec::new());
        unsafe_sibling.execution_semantics.execution_mode = Some("sequential".to_string());

        let blocked_dependency = TaskDependencyStatus {
            issue_id: "blocked".to_string(),
            depends_on_id: "dep-1".to_string(),
            edge_type: "depends-on".to_string(),
            dependency_status: "open".to_string(),
            dependency_issue_type: Some("task".to_string()),
        };
        let blocked = task("blocked", "task", "open", 5, &[], Vec::new());

        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![
                scheduling_candidate(primary.clone(), true, false, true, Vec::new(), vec![]),
                scheduling_candidate(sibling_a.clone(), true, true, false, Vec::new(), vec![]),
                scheduling_candidate(sibling_b.clone(), true, true, false, Vec::new(), vec![]),
                scheduling_candidate(
                    unsafe_sibling.clone(),
                    true,
                    false,
                    false,
                    Vec::new(),
                    vec!["current_execution_mode_not_parallel_safe"],
                ),
            ],
            blocked: vec![scheduling_candidate(
                blocked.clone(),
                false,
                false,
                false,
                vec![blocked_dependency.clone()],
                vec!["graph_blocked"],
            )],
            parallel_candidates_after_current: vec![sibling_a.clone(), sibling_b.clone()],
        };

        let plan = build_taskflow_scheduler_dispatch_plan(projection, 2, None, None, true, false);

        assert_eq!(plan.status, "pass");
        assert_eq!(plan.max_parallel_agents, 2);
        assert!(!plan.execute_requested);
        assert!(!plan.execute_supported);
        assert!(!plan.execution_attempted);
        assert_eq!(plan.execution_status, "preview");
        assert_eq!(plan.selection_source, "critical_path_ready_head");
        assert_eq!(
            plan.selected_primary_task
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("critical-ready")
        );
        assert_eq!(plan.selected_task_ids, vec!["critical-ready", "parallel-a"]);
        assert_eq!(plan.selected_parallel_tasks.len(), 1);
        assert_eq!(plan.selected_parallel_tasks[0].id, "parallel-a");
        assert_eq!(plan.reservations.len(), 2);
        assert_eq!(plan.reservations[0].task_id, "critical-ready");
        assert_eq!(plan.reservations[0].launch_role, "primary");
        assert_eq!(plan.reservations[0].launch_index, 0);
        assert_eq!(
            plan.reservations[0].reservation_id,
            "scheduler-preview-primary-0-critical-ready"
        );
        assert_eq!(
            plan.reservations[0].reservation_status,
            "preview_unpersisted"
        );
        assert!(!plan.reservations[0].reservation_persisted);
        assert!(!plan.reservations[0].execution_attempted);
        assert_eq!(plan.reservations[1].task_id, "parallel-a");
        assert_eq!(plan.reservations[1].launch_role, "parallel");
        assert_eq!(plan.reservations[1].launch_index, 1);
        assert_eq!(plan.dispatch_receipt.receipt_id, None);
        assert_eq!(
            plan.dispatch_receipt.receipt_status,
            "preview_not_persisted"
        );
        assert!(!plan.dispatch_receipt.receipt_persisted);
        assert_eq!(plan.dispatch_receipt.dispatch_status, "pass");
        assert_eq!(
            plan.dispatch_receipt.selected_task_ids,
            vec!["critical-ready", "parallel-a"]
        );
        assert_eq!(
            plan.dispatch_receipt.reservation_ids,
            vec![
                "scheduler-preview-primary-0-critical-ready",
                "scheduler-preview-parallel-1-parallel-a"
            ]
        );
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task.id == "parallel-b"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "max_parallel_agents_cap_reached")
        }));
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task.id == "sequential-only"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "current_execution_mode_not_parallel_safe")
        }));
        assert!(plan.rejected_candidates.iter().any(|candidate| {
            candidate.task.id == "blocked"
                && candidate
                    .reasons
                    .iter()
                    .any(|reason| reason == "blocked_by:depends-on:dep-1:open")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_fails_closed_when_execute_is_requested() {
        let primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![scheduling_candidate(
                primary,
                true,
                false,
                true,
                Vec::new(),
                vec![],
            )],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let plan = build_taskflow_scheduler_dispatch_plan(projection, 1, None, None, false, true);

        assert_eq!(plan.status, "blocked");
        assert!(plan
            .blocker_codes
            .iter()
            .any(|code| code == "unsupported_blocker_code"));
        assert_eq!(plan.max_parallel_agents, 1);
        assert!(plan.execute_requested);
        assert!(!plan.execute_supported);
        assert!(!plan.execution_attempted);
        assert_eq!(plan.execution_status, "unsupported");
        assert!(!plan.dry_run);
        assert_eq!(plan.selected_task_ids, vec!["critical-ready"]);
        assert_eq!(plan.reservations.len(), 1);
        assert_eq!(
            plan.reservations[0].reservation_status,
            "preview_unpersisted"
        );
        assert!(!plan.reservations[0].reservation_persisted);
        assert_eq!(
            plan.dispatch_receipt.receipt_status,
            "unsupported_not_persisted"
        );
        assert_eq!(plan.dispatch_receipt.dispatch_status, "blocked");
        assert!(plan.dispatch_receipt.execute_requested);
        assert!(!plan.dispatch_receipt.execute_supported);
        assert!(!plan.dispatch_receipt.execution_attempted);
        assert!(!plan.dispatch_receipt.receipt_persisted);
        assert!(plan
            .dispatch_receipt
            .blocker_codes
            .iter()
            .any(|code| code == "unsupported_blocker_code"));
        assert!(plan.next_actions.iter().any(|action| {
            action.contains("preview-first")
                && action.contains("without `--execute`")
                && action.contains("normal delegated runtime flow")
        }));
    }

    #[test]
    fn scheduler_dispatch_plan_serializes_reservation_and_receipt_projection_fields() {
        let primary = task("critical-ready", "task", "open", 1, &[], Vec::new());
        let parallel = task("parallel-ready", "task", "open", 2, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("critical-ready".to_string()),
            ready: vec![
                scheduling_candidate(primary, true, false, true, Vec::new(), vec![]),
                scheduling_candidate(parallel, true, true, false, Vec::new(), vec![]),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let plan = build_taskflow_scheduler_dispatch_plan(projection, 2, None, None, true, false);
        let payload = serde_json::to_value(&plan).expect("scheduler plan should serialize");

        assert_eq!(
            payload["reservations"][0]["reservation_id"],
            "scheduler-preview-primary-0-critical-ready"
        );
        assert_eq!(
            payload["reservations"][1]["reservation_id"],
            "scheduler-preview-parallel-1-parallel-ready"
        );
        assert_eq!(payload["reservations"][0]["reservation_persisted"], false);
        assert_eq!(
            payload["dispatch_receipt"]["receipt_id"],
            serde_json::Value::Null
        );
        assert_eq!(
            payload["dispatch_receipt"]["receipt_status"],
            "preview_not_persisted"
        );
        assert_eq!(payload["dispatch_receipt"]["receipt_persisted"], false);
        assert_eq!(
            payload["dispatch_receipt"]["reservation_ids"],
            serde_json::json!([
                "scheduler-preview-primary-0-critical-ready",
                "scheduler-preview-parallel-1-parallel-ready"
            ])
        );
        assert_eq!(payload["dispatch_receipt"]["execution_attempted"], false);
    }

    #[test]
    fn graph_explain_payload_reports_ready_blocked_and_parallel_truth() {
        let mut current = task("current", "task", "open", 1, &[], Vec::new());
        current.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        current.execution_semantics.order_bucket = Some("wave-a".to_string());
        current.execution_semantics.parallel_group = Some("docs".to_string());
        current.execution_semantics.conflict_domain = Some("current".to_string());

        let mut sibling = task("sibling", "task", "open", 2, &[], Vec::new());
        sibling.execution_semantics.execution_mode = Some("parallel_safe".to_string());
        sibling.execution_semantics.order_bucket = Some("wave-a".to_string());
        sibling.execution_semantics.parallel_group = Some("docs".to_string());
        sibling.execution_semantics.conflict_domain = Some("sibling".to_string());

        let blocker = TaskDependencyStatus {
            issue_id: "blocked".to_string(),
            depends_on_id: "current".to_string(),
            edge_type: "blocks".to_string(),
            dependency_status: "open".to_string(),
            dependency_issue_type: Some("task".to_string()),
        };
        let blocked = task("blocked", "task", "open", 3, &[], Vec::new());
        let projection = TaskSchedulingProjection {
            current_task_id: Some("current".to_string()),
            ready: vec![
                scheduling_candidate(
                    current,
                    true,
                    false,
                    true,
                    Vec::new(),
                    vec!["parallel_conflict"],
                ),
                scheduling_candidate(sibling.clone(), true, true, false, Vec::new(), vec![]),
            ],
            blocked: vec![scheduling_candidate(
                blocked,
                false,
                false,
                false,
                vec![blocker],
                vec!["graph_blocked"],
            )],
            parallel_candidates_after_current: vec![sibling],
        };

        let ready_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("sibling"));
        assert_eq!(ready_payload["surface"], "vida taskflow graph explain");
        assert_eq!(ready_payload["status"], "pass");
        assert_eq!(ready_payload["ready_now"], true);
        assert_eq!(ready_payload["ready_parallel_safe"], true);
        assert_eq!(ready_payload["selected_as_parallel_after_current"], true);
        assert_eq!(
            ready_payload["next_lawful_action"]["surface"],
            "vida taskflow scheduler dispatch"
        );

        let blocked_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("blocked"));
        assert_eq!(blocked_payload["status"], "blocked");
        assert_eq!(blocked_payload["ready_now"], false);
        assert!(blocked_payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| codes.contains(&serde_json::json!("graph_blocked"))));
        assert_eq!(blocked_payload["blocked_by"][0]["depends_on_id"], "current");
        assert_eq!(
            blocked_payload["next_lawful_action"]["surface"],
            "vida task deps"
        );

        let missing_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("missing"));
        assert_eq!(missing_payload["status"], "blocked");
        assert_eq!(missing_payload["ready_now"], false);
        assert!(missing_payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| {
                codes.contains(&serde_json::json!("task_not_in_graph_projection"))
            }));

        let parallel_blocked_payload =
            super::build_taskflow_graph_explain_payload(&projection, None, Some("current"));
        assert_eq!(parallel_blocked_payload["status"], "blocked");
        assert_eq!(parallel_blocked_payload["ready_now"], true);
        assert!(parallel_blocked_payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()));
    }

    #[test]
    fn taskflow_task_subcommand_supports_replace_jsonl() {
        assert!(taskflow_task_subcommand_supported("replace-jsonl"));
    }

    #[test]
    fn taskflow_task_subcommand_supports_task_graph_parity_commands() {
        for subcommand in [
            "children",
            "reparent-children",
            "move-children",
            "tree",
            "subtree",
            "validate-graph",
            "critical-path",
        ] {
            assert!(
                taskflow_task_subcommand_supported(subcommand),
                "`vida taskflow task {subcommand}` should remain in parity with `vida task {subcommand}`"
            );
        }
    }

    #[test]
    fn route_diagnostic_parser_accepts_route_explain_json() {
        let args = vec![
            "route".to_string(),
            "explain".to_string(),
            "--dispatch-target".to_string(),
            "implementation".to_string(),
            "--json".to_string(),
        ];
        let parsed = super::parse_taskflow_route_diagnostic_args(&args).unwrap();
        assert_eq!(parsed.mode, super::RouteDiagnosticMode::Explain);
        assert_eq!(parsed.dispatch_target.as_deref(), Some("implementation"));
        assert!(parsed.as_json);
    }

    #[test]
    fn route_diagnostic_parser_accepts_config_actuation_census() {
        let args = vec![
            "config-actuation".to_string(),
            "census".to_string(),
            "--json".to_string(),
        ];
        let parsed = super::parse_taskflow_route_diagnostic_args(&args).unwrap();
        assert_eq!(
            parsed.mode,
            super::RouteDiagnosticMode::ConfigActuationCensus
        );
        assert!(parsed.as_json);
    }

    #[test]
    fn taskflow_diagnostic_operator_contract_normalizer_derives_blocked_truth() {
        let payload = serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "status": "pass",
            "blocker_codes": [" migration_required "],
            "next_actions": [" Run migration preflight "],
            "artifact_refs": {
                "source": "unit-test"
            }
        });

        let normalized = super::normalize_taskflow_diagnostic_operator_contract_payload(
            payload,
            "inspect diagnostic blockers",
        )
        .expect("diagnostic payload should normalize");

        assert_eq!(normalized["status"], "blocked");
        assert_eq!(
            normalized["blocker_codes"],
            serde_json::json!(["migration_required"])
        );
        assert_eq!(
            normalized["next_actions"],
            serde_json::json!(["run migration preflight"])
        );
        assert_eq!(normalized["shared_fields"]["status"], normalized["status"]);
        assert_eq!(
            normalized["operator_contracts"]["blocker_codes"],
            normalized["blocker_codes"]
        );
        assert_eq!(
            normalized["operator_contracts"]["next_actions"],
            normalized["next_actions"]
        );
        assert_eq!(normalized["artifact_refs"]["source"], "unit-test");
    }

    #[test]
    fn taskflow_diagnostic_operator_contract_normalizer_derives_pass_truth() {
        let payload = serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "status": "blocked",
            "blocker_codes": [],
            "next_actions": [" stale action "]
        });

        let normalized = super::normalize_taskflow_diagnostic_operator_contract_payload(
            payload,
            "inspect diagnostic blockers",
        )
        .expect("diagnostic payload should normalize");

        assert_eq!(normalized["status"], "pass");
        assert_eq!(normalized["blocker_codes"], serde_json::json!([]));
        assert_eq!(normalized["next_actions"], serde_json::json!([]));
        assert_eq!(normalized["shared_fields"]["status"], "pass");
        assert_eq!(normalized["operator_contracts"]["status"], "pass");
    }

    #[test]
    fn taskflow_diagnostic_operator_contract_normalizer_falls_back_for_unknown_blockers() {
        let payload = serde_json::json!({
            "surface": "vida taskflow diagnostic",
            "blocker_codes": ["not_registry_backed"],
            "next_actions": []
        });

        let normalized = super::normalize_taskflow_diagnostic_operator_contract_payload(
            payload,
            "inspect diagnostic blockers",
        )
        .expect("diagnostic payload should normalize");

        assert_eq!(normalized["status"], "blocked");
        assert_eq!(
            normalized["blocker_codes"],
            serde_json::json!(["unsupported_blocker_code"])
        );
        assert_eq!(
            normalized["next_actions"],
            serde_json::json!(["inspect diagnostic blockers"])
        );
    }

    #[test]
    fn route_diagnostic_payload_normalizer_applies_pass_contract_parity() {
        let payload = serde_json::json!({
            "surface": "vida taskflow route explain",
            "status": "blocked",
            "blocker_codes": [],
            "next_actions": [" stale action "],
            "route": {
                "status": "pass",
                "blocker_codes": []
            }
        });

        let normalized = super::normalize_taskflow_route_diagnostic_payload(payload)
            .expect("route diagnostic payload should normalize");

        assert_eq!(normalized["status"], "pass");
        assert_eq!(normalized["blocker_codes"], serde_json::json!([]));
        assert_eq!(normalized["next_actions"], serde_json::json!([]));
        assert_eq!(normalized["shared_fields"]["status"], normalized["status"]);
        assert_eq!(
            normalized["operator_contracts"]["blocker_codes"],
            normalized["blocker_codes"]
        );
        assert_eq!(
            normalized["operator_contracts"]["next_actions"],
            normalized["next_actions"]
        );
    }

    #[test]
    fn config_diagnostic_payload_normalizer_applies_blocked_contract_parity() {
        let payload = serde_json::json!({
            "surface": "vida taskflow config-actuation census",
            "status": "pass",
            "blocker_codes": ["route_missing"],
            "next_actions": [],
            "routes": []
        });

        let normalized = super::normalize_taskflow_route_diagnostic_payload(payload)
            .expect("config diagnostic payload should normalize");

        assert_eq!(normalized["status"], "blocked");
        assert_eq!(
            normalized["blocker_codes"],
            serde_json::json!(["unsupported_blocker_code"])
        );
        assert_eq!(
            normalized["next_actions"],
            serde_json::json!(["inspect route diagnostic blockers with `vida taskflow route explain --json` or `vida taskflow validate-routing --json`"])
        );
        assert_eq!(normalized["shared_fields"]["status"], normalized["status"]);
        assert_eq!(
            normalized["operator_contracts"]["blocker_codes"],
            normalized["blocker_codes"]
        );
        assert_eq!(
            normalized["operator_contracts"]["next_actions"],
            normalized["next_actions"]
        );
    }

    #[test]
    fn route_payload_accepts_runtime_selected_internal_host_carrier_without_matrix_row() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                },
                {
                    "backend_id": "opencode_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "implementation": false
                    }
                }
            ],
            "development_flow": {
                "implementation": {
                    "executor_backend": "opencode_cli",
                    "fallback_executor_backend": "internal_subagents",
                    "carrier_runtime_assignment": {
                        "selected_backend_id": "junior",
                        "selected_carrier_id": "junior",
                        "selected_agent_id": "junior",
                        "selected_tier": "junior",
                        "selected_model_provider": "openai",
                        "selected_model_profile_id": "codex_gpt54_low_write"
                    }
                }
            }
        });

        let payload = super::route_payload_for_dispatch_target(&execution_plan, "implementer");

        assert_eq!(payload["selected_backend"].as_str(), Some("junior"));
        assert_eq!(payload["selected_backend_admissible"].as_bool(), Some(true));
        assert_eq!(payload["status"].as_str(), Some("pass"));
        assert!(payload["blocker_codes"]
            .as_array()
            .is_some_and(|codes| codes.is_empty()));
    }

    #[test]
    fn config_actuation_census_reports_bounded_route_model_selection_rows() {
        let context = crate::state_store::RunGraphDispatchContext {
            run_id: "run-config-census".to_string(),
            task_id: "task-config-census".to_string(),
            request_text: "inspect config actuation".to_string(),
            role_selection: serde_json::json!({}),
            recorded_at: "0".to_string(),
        };
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "execution_lane_sequence": ["implementation"],
                    "lane_catalog": {
                        "implementation": {
                            "executor_backend": "opencode_cli",
                            "fallback_executor_backend": "internal_subagents",
                            "fanout_executor_backends": ["middle", "senior"],
                            "carrier_runtime_assignment": {
                                "enabled": true,
                                "selected_backend_id": "junior",
                                "selected_carrier_id": "junior",
                                "selected_model_profile_id": "codex_gpt54_low_write",
                                "selected_reasoning_effort": "low",
                                "model_selection_enabled": true,
                                "candidate_scope": "unified_carrier_model_profiles",
                                "budget_policy": "tier_budget_guard",
                                "max_budget_units": 4
                            }
                        }
                    }
                }
            }
        });

        let payload = super::build_config_actuation_census_payload(&context, &execution_plan);

        assert_eq!(payload["surface"], "vida taskflow config-actuation census");
        assert_eq!(payload["scope"], "routing_model_selection_keys");
        assert_eq!(payload["route_count"], 1);
        assert_eq!(
            payload["routes"][0]["model_profile_readiness_audit"]["surface"],
            "vida taskflow model-profile readiness audit"
        );
        assert!(payload["row_count"]
            .as_u64()
            .is_some_and(|count| count >= 10));
        let rows = payload["routes"][0]["rows"]
            .as_array()
            .expect("config actuation rows should render");
        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.selected_model_profile_id"
                && row["runtime_consumer"] == "selected_backend_readiness_payload"
                && row["proof_status"] == "actuated_or_validated"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "executor_backend"
                && row["runtime_consumer"] == "explicit_executor_backend_from_route"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.candidate_scope"
                && row["validator"] == "route_explain_status / route_explain_blocker_codes"
                && row["proof_status"] == "actuated_or_validated"
        }));
    }

    #[test]
    fn config_actuation_census_marks_blocking_and_non_behavioral_route_fields() {
        let route = serde_json::json!({
            "runtime_assignment_enabled": true,
            "model_selection_enabled": false,
            "candidate_scope": "legacy_backend_pool",
            "route_field_truth": [
                {
                    "field": "external_first_required",
                    "truth": "rejected_no_runtime_consumer",
                    "effect": "validate-routing blocks the route until the field is removed or wired to a concrete consumer"
                }
            ]
        });

        let rows = super::config_actuation_census_rows_for_route(&route);

        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.model_selection_enabled"
                && row["proof_status"] == "validated_blocking"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "carrier_runtime_assignment.candidate_scope"
                && row["proof_status"] == "validated_blocking"
        }));
        assert!(rows.iter().any(|row| {
            row["config_key"] == "external_first_required"
                && row["proof_status"] == "rejected_no_runtime_consumer"
                && row["operator_surface"] == "vida taskflow validate-routing"
        }));
    }

    #[test]
    fn model_profile_readiness_audit_payload_reports_ready_selection_truth() {
        let route = serde_json::json!({
            "status": "pass",
            "selected_backend": "junior",
            "selected_carrier_id": "junior",
            "selected_model_profile_id": "codex_gpt54_low_write",
            "selected_model_ref": "gpt-5.4",
            "selected_model_provider": "openai",
            "selected_reasoning_effort": "low",
            "selected_reasoning_control_mode": "fixed",
            "selected_backend_readiness": {
                "backend_id": "junior",
                "blocked": false,
                "status": "pass",
                "blocker_code": null,
                "selected_model_profile": "codex_gpt54_low_write",
                "next_actions": []
            },
            "selection_source_paths": {
                "selected_model_profile_id": "carrier_runtime.roles[junior].model_profiles.codex_gpt54_low_write.profile_id"
            },
            "selection_override_reasons": ["route_profile_mapping"],
            "selection_precedence": ["route_profile_mapping", "role_default"],
            "selected_route_profile_mapping": {
                "runtime_role": "worker",
                "profile_id": "codex_gpt54_low_write"
            },
            "selected_candidate": {
                "profile_id": "codex_gpt54_low_write",
                "selected": true
            },
            "candidate_pool": [
                {
                    "profile_id": "codex_gpt54_low_write",
                    "selected": true
                },
                {
                    "profile_id": "codex_spark_high_readonly",
                    "selected": false
                }
            ],
            "rejected_candidates": [
                {
                    "profile_id": "codex_spark_high_readonly",
                    "reason": "write_scope_required"
                }
            ],
            "readiness_blockers": [],
            "budget_policy": "tier_budget_guard",
            "budget_verdict": "within_budget",
            "max_budget_units": 4,
            "selected_over_budget": false,
            "budget_scope": "task",
            "selection_budget": {
                "remaining_units": 3
            },
            "runtime_budget_ledger": {
                "spent_units": 1
            }
        });

        let payload =
            super::model_profile_readiness_audit_payload_for_route("implementation", &route);

        assert_eq!(
            payload["surface"],
            "vida taskflow model-profile readiness audit"
        );
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"], serde_json::json!([]));
        assert_eq!(
            payload["selected_profile"]["profile_id"],
            "codex_gpt54_low_write"
        );
        assert_eq!(payload["selected_profile"]["readiness_status"], "pass");
        assert_eq!(payload["selected_profile"]["readiness_ready"], true);
        assert_eq!(
            payload["source_paths"]["selected_model_profile_id"],
            "carrier_runtime.roles[junior].model_profiles.codex_gpt54_low_write.profile_id"
        );
        assert_eq!(
            payload["override_reasons"],
            serde_json::json!(["route_profile_mapping"])
        );
        assert_eq!(
            payload["rejected_alternatives"][0]["profile_id"],
            "codex_spark_high_readonly"
        );
        assert_eq!(payload["budget"]["policy"], "tier_budget_guard");
    }

    #[test]
    fn model_profile_readiness_audit_payload_blocks_unready_or_missing_selection() {
        let unready_route = serde_json::json!({
            "status": "blocked",
            "selected_backend": "junior",
            "selected_carrier_id": "junior",
            "selected_model_profile_id": "codex_gpt54_low_write",
            "selected_model_ref": "gpt-5.4",
            "selected_model_provider": "openai",
            "selected_reasoning_effort": "low",
            "selected_reasoning_control_mode": "fixed",
            "selected_backend_readiness": {
                "backend_id": "junior",
                "blocked": true,
                "status": "blocked",
                "blocker_code": "external_cli_missing_api_key",
                "selected_model_profile": "codex_gpt54_low_write",
                "next_actions": ["configure OPENAI_API_KEY"]
            },
            "selection_source_paths": {},
            "selection_override_reasons": [],
            "selection_precedence": [],
            "selected_route_profile_mapping": null,
            "selected_candidate": null,
            "candidate_pool": [],
            "rejected_candidates": [],
            "readiness_blockers": [
                {
                    "blocker_code": "external_cli_missing_api_key"
                }
            ],
            "budget_policy": null,
            "budget_verdict": null,
            "max_budget_units": null,
            "selected_over_budget": null,
            "budget_scope": null,
            "selection_budget": null,
            "runtime_budget_ledger": null
        });
        let missing_route = serde_json::json!({
            "status": "blocked",
            "selected_backend": "junior",
            "selected_carrier_id": "junior",
            "selected_model_ref": null,
            "selected_model_provider": null,
            "selected_reasoning_effort": null,
            "selected_reasoning_control_mode": null,
            "selection_source_paths": {},
            "selection_override_reasons": [],
            "selection_precedence": [],
            "selected_route_profile_mapping": null,
            "selected_candidate": null,
            "candidate_pool": [],
            "rejected_candidates": [],
            "readiness_blockers": [],
            "budget_policy": null,
            "budget_verdict": null,
            "max_budget_units": null,
            "selected_over_budget": null,
            "budget_scope": null,
            "selection_budget": null,
            "runtime_budget_ledger": null
        });

        let unready = super::model_profile_readiness_audit_payload_for_route(
            "implementation",
            &unready_route,
        );
        let missing = super::model_profile_readiness_audit_payload_for_route(
            "implementation",
            &missing_route,
        );

        assert_eq!(unready["status"], "blocked");
        assert_eq!(
            unready["blocker_codes"],
            serde_json::json!(["selected_model_profile_not_ready"])
        );
        assert_eq!(
            unready["selected_profile"]["readiness"]["blocker_code"],
            "external_cli_missing_api_key"
        );
        assert!(unready["next_actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()));
        assert_eq!(missing["status"], "blocked");
        assert_eq!(
            missing["blocker_codes"],
            serde_json::json!(["selected_model_profile_missing"])
        );
        assert!(missing["selected_profile"]["readiness_status"].is_null());
    }

    #[test]
    fn validate_routing_targets_fall_back_when_contract_is_missing() {
        let targets = super::route_validate_targets(&serde_json::json!({}));
        assert_eq!(targets, vec!["implementation", "coach", "verification"]);
    }

    #[test]
    fn validate_routing_targets_canonicalize_legacy_dispatch_lane_names() {
        let execution_plan = serde_json::json!({
            "development_flow": {
                "dispatch_contract": {
                    "execution_lane_sequence": ["implementer", "coach", "execution_preparation"]
                }
            }
        });

        let targets = super::route_validate_targets(&execution_plan);
        assert_eq!(targets, vec!["implementation", "coach", "architecture"]);
    }

    #[test]
    fn recovery_holds_active_bound_run_when_delegated_cycle_is_open() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: None,
            resume_status: "ready".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "validation_report_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "non_blocking_only".to_string(),
                continuation_signal: "continue_routing_non_blocking".to_string(),
                blocker_code: Some("open_delegated_cycle".to_string()),
                lifecycle_stage: "coach_active".to_string(),
            },
        };

        assert!(super::recovery_holds_active_bound_run(Some(&recovery)));
    }

    #[test]
    fn recovery_does_not_hold_ready_head_when_delegated_cycle_is_clear() {
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            active_node: "closure".to_string(),
            lifecycle_stage: "closure_complete".to_string(),
            resume_node: None,
            resume_status: "completed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "none".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "closure".to_string(),
                delegated_cycle_open: false,
                delegated_cycle_state: "clear".to_string(),
                local_exception_takeover_gate: "delegated_cycle_clear".to_string(),
                reporting_pause_gate: "clear".to_string(),
                continuation_signal: "none".to_string(),
                blocker_code: None,
                lifecycle_stage: "closure_complete".to_string(),
            },
        };

        assert!(!super::recovery_holds_active_bound_run(Some(&recovery)));
        assert!(!super::recovery_holds_active_bound_run(None));
    }

    #[test]
    fn authoritative_dispatch_blocker_codes_include_primary_and_downstream_blockers() {
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/packet.json".to_string()),
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
            activation_runtime_role: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: Some("/tmp/downstream-result.json".to_string()),
            downstream_dispatch_packet_path: Some("/tmp/downstream-packet.json".to_string()),
            downstream_dispatch_trace_path: Some("/tmp/downstream-trace.json".to_string()),
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: Some("coach".to_string()),
            downstream_dispatch_note: Some("after coach, verify".to_string()),
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
        };

        let blocker_codes = super::authoritative_dispatch_blocker_codes(Some(&dispatch));
        assert_eq!(blocker_codes.len(), 2);
        assert!(blocker_codes
            .iter()
            .any(|code| code == "timeout_without_takeover_authority"));
        assert!(blocker_codes
            .iter()
            .any(|code| code == "pending_review_clean_evidence"));
    }

    fn sample_task(task_id: &str) -> crate::state_store::TaskRecord {
        crate::state_store::TaskRecord {
            id: task_id.to_string(),
            display_id: Some("vida-1".to_string()),
            title: "Sample Task".to_string(),
            description: "sample".to_string(),
            status: "open".to_string(),
            priority: 2,
            issue_type: "task".to_string(),
            created_at: "2026-04-18T00:00:00Z".to_string(),
            created_by: "tester".to_string(),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
            planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn taskflow_next_decision_surfaces_candidate_when_runtime_gate_blocks_ready_head() {
        let dispatch = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_target: "worker".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            dispatch_surface: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            selected_backend: Some("internal_subagents".to_string()),
            exception_path_receipt_id: None,
            supersedes_receipt_id: None,
            recorded_at: "2026-04-18T00:00:00Z".to_string(),
            activation_runtime_role: None,
            activation_agent_type: None,
            activation_evidence: serde_json::Value::Null,
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            downstream_dispatch_ready: false,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_packet_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_active_target: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_last_target: None,
            downstream_dispatch_note: None,
            downstream_dispatch_blockers: Vec::new(),
        };

        let decision = super::build_taskflow_next_decision(
            Some(&sample_task("task-1")),
            true,
            true,
            Some("bundle-check"),
            Some("epic-1"),
            Some(&dispatch),
            None,
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(
            decision
                .candidate_task_context
                .ready_head
                .as_ref()
                .map(|task| task.id.as_str()),
            Some("task-1")
        );
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "delegated_cycle_runtime_gate"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("delegated_cycle_runtime_gate")
        );
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow recovery latest --json")
        );
    }

    #[test]
    fn taskflow_next_decision_reports_no_ready_task_with_explicit_next_action() {
        let decision = super::build_taskflow_next_decision(
            None,
            false,
            false,
            None,
            Some("epic-1"),
            None,
            None,
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(decision.primary_ready_task.is_none());
        assert_eq!(decision.candidate_task_context.ready_head, None);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "no_ready_task"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("backlog_no_ready_task")
        );
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida task ready --scope epic-1 --json")
        );
        assert!(decision
            .blocker_codes
            .iter()
            .any(|code| code == "no_ready_tasks"));
    }

    #[test]
    fn taskflow_next_decision_fails_closed_after_closure_without_explicit_binding() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-closure",
            "task-closure",
            "implementation",
        );
        status.active_node = "closure".to_string();
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        let decision = super::build_taskflow_next_decision(
            None,
            false,
            true,
            Some("final"),
            None,
            None,
            Some(&status),
            None,
        );

        assert_eq!(decision.status, "blocked");
        assert!(!decision.candidate_task_context.admissible_now);
        assert_eq!(
            decision.candidate_task_context.admissibility_gate,
            "completed_without_explicit_next_bounded_unit"
        );
        assert_eq!(
            decision
                .why_not_now
                .as_ref()
                .map(|value| value.category.as_str()),
            Some("completed_without_explicit_next_bounded_unit")
        );
        assert_eq!(
            decision
                .next_action
                .as_ref()
                .map(|value| value.command.as_str()),
            Some("vida taskflow continuation bind run-closure --task-id <task-id> --json")
        );
    }
}

pub(crate) async fn run_taskflow_proxy(args: ProxyArgs) -> ExitCode {
    if matches!(args.args.first().map(String::as_str), Some("query")) {
        return run_taskflow_query(&args.args);
    }

    if let Some(topic) = taskflow_help_topic(&args.args) {
        print_taskflow_proxy_help(topic);
        return ExitCode::SUCCESS;
    }

    if matches!(args.args.first().map(String::as_str), Some("recovery")) {
        return run_taskflow_recovery(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("task")) {
        return route_taskflow_task(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("next")) {
        return run_taskflow_next_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("graph-summary")) {
        return run_taskflow_graph_summary(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("graph")) {
        return run_taskflow_graph_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("plan")) {
        return crate::taskflow_plan_graph::run_taskflow_plan(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("replan")) {
        return run_taskflow_replan_surface(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("scheduler")) {
        return run_taskflow_scheduler_surface(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("route" | "validate-routing" | "config-actuation")
    ) {
        return run_taskflow_route_diagnostic(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("doctor")) {
        return route_taskflow_doctor(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("status")) {
        return route_taskflow_status(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("bootstrap-spec")
    ) {
        return run_taskflow_bootstrap_spec(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("protocol-binding")
    ) {
        return taskflow_protocol_binding::run_taskflow_protocol_binding(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("continuation")) {
        return crate::taskflow_continuation::run_taskflow_continuation(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("packet")) {
        return crate::taskflow_packet::run_taskflow_packet(&args.args).await;
    }

    if matches!(
        args.args.first().map(String::as_str),
        Some("artifact" | "artifacts")
    ) {
        return crate::taskflow_artifacts::run_taskflow_artifacts(&args.args).await;
    }

    if matches!(args.args.first().map(String::as_str), Some("consume")) {
        let consume_subcommand = args.args.get(1).map(String::as_str);
        if matches!(consume_subcommand, Some("continue" | "advance")) {
            let state_root = proxy_state_dir();
            if let Err(error) = enforce_execution_preparation_contract_gate(&state_root) {
                eprintln!(
                    "{error}\nFail-closed: `vida taskflow consume {}` requires release-1 execution-preparation evidence/contract.",
                    consume_subcommand.unwrap_or("unknown")
                );
                return ExitCode::from(1);
            }
        }
        if matches!(
            consume_subcommand,
            None | Some(
                "bundle" | "agent-system" | "final" | "continue" | "advance" | "--help" | "-h"
            )
        ) {
            return taskflow_consume::run_taskflow_consume(&args.args).await;
        }
    }

    if matches!(args.args.first().map(String::as_str), Some("run-graph")) {
        if matches!(
            args.args.get(1).map(String::as_str),
            Some("status" | "latest" | "diagnose" | "diagnose-latest" | "--help" | "-h")
        ) {
            return run_taskflow_run_graph(&args.args).await;
        }
        if matches!(
            args.args.get(1).map(String::as_str),
            Some("seed" | "advance" | "dispatch-init" | "init" | "update")
        ) {
            return run_taskflow_run_graph_mutation(&args.args).await;
        }
    }

    let subcommand = args.args.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "Unsupported `vida taskflow {subcommand}` subcommand. This launcher-owned top-level taskflow surface fails closed instead of delegating to the external TaskFlow runtime."
    );
    ExitCode::from(2)
}
