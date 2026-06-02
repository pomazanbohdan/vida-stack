use std::process::ExitCode;

use crate::launcher_activation_snapshot::capture_launcher_activation_snapshot;
use crate::{
    state_store, state_store::StateStore, AgentArgs, AgentCommand, AgentDispatchNextArgs,
    AgentSelectArgs,
};

const AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(300);

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLaneSelectionTruth {
    selected_carrier: String,
    selected_backend: String,
    selected_model_profile: String,
    selected_model_ref: String,
    selected_reasoning_effort: String,
    rate: u64,
    estimated_task_price_units: u64,
    budget_verdict: String,
    selection_source_paths: serde_json::Value,
    pricing_readiness: serde_json::Value,
    runtime_role: String,
    task_class: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLanePreview {
    lane_index: usize,
    task_id: String,
    title: String,
    role_label: String,
    runtime_role: String,
    task_class: String,
    dispatch_command: String,
    dispatch_command_kind: String,
    receipt_backed_execution_command: String,
    ready_parallel_safe: bool,
    selection_reason: String,
    selection_truth: AgentDispatchLaneSelectionTruth,
    requires_user_approval: bool,
    approval_gate: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchBlockedCandidate {
    task_id: String,
    title: String,
    ready_now: bool,
    ready_parallel_safe: bool,
    reasons: Vec<String>,
    parallel_blockers: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchNextPreview {
    status: String,
    mode: String,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    effective_max_parallel_agents: usize,
    lanes_selected: usize,
    selected_lanes: Vec<AgentDispatchLanePreview>,
    blocked_candidates: Vec<AgentDispatchBlockedCandidate>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    execute_supported: bool,
    execution_attempted: bool,
    parallelization_planner: serde_json::Value,
    carrier_selection_api: serde_json::Value,
    flow_projection: serde_json::Value,
    source_surfaces: Vec<String>,
}

fn agent_dispatch_source_surfaces() -> Vec<String> {
    vec![
        "vida agent dispatch-next".to_string(),
        "StateStore::scheduling_projection_scoped".to_string(),
        "vida taskflow graph-summary --json".to_string(),
        "vida taskflow scheduler dispatch --json".to_string(),
        "vida agent select --runtime-role <role> --task-class <class> --json".to_string(),
        "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            .to_string(),
        "vida agent-init --role worker <task-id> --json".to_string(),
        "vida agent-init --role <runtime-role> <task-id> --json".to_string(),
    ]
}

fn build_parallelization_planner(
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
) -> serde_json::Value {
    let ready_parallel_safe = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .count();
    let independent_failures = projection
        .blocked
        .iter()
        .filter(|candidate| !candidate.ready_now)
        .count();
    let triggers = [
        (
            "coverage_or_test_expansion",
            projection.ready.iter().any(|candidate| {
                let title = candidate.task.title.to_ascii_lowercase();
                let work_item_keys = task_flow_lookup_keys(&candidate.task).join(" ");
                let labels = candidate
                    .task
                    .labels
                    .iter()
                    .map(|label| label.to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join(" ");
                title.contains("test")
                    || title.contains("coverage")
                    || work_item_keys.contains("verification")
                    || labels.contains("verification")
                    || labels.contains("quality")
            }),
        ),
        (
            "three_or_more_independent_failures",
            independent_failures >= 3,
        ),
        (
            "parallel_safe_ready_candidates",
            ready_parallel_safe >= 2 && configured_max_parallel_agents > 1,
        ),
    ];
    let active_triggers = triggers
        .into_iter()
        .filter_map(|(trigger, active)| active.then(|| trigger.to_string()))
        .collect::<Vec<_>>();
    let packet_proposals = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .take(lanes_requested.min(configured_max_parallel_agents.max(1)))
        .map(|candidate| {
            serde_json::json!({
                "task_id": candidate.task.id,
                "title": candidate.task.title,
                "proposal_kind": "parallel_safe_dispatch_packet_preview",
                "materializes_packet": false,
                "next_surface": "vida agent-init",
                "reason": "candidate is ready and parallel-safe under TaskFlow scheduling projection"
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if packet_proposals.is_empty() { "no_packet_proposals" } else { "proposals_available" },
        "mode": "preview_only",
        "triggers": active_triggers,
        "ready_parallel_safe_count": ready_parallel_safe,
        "independent_failure_count": independent_failures,
        "packet_proposals": packet_proposals,
        "materializes_packets": false,
        "next_action": if ready_parallel_safe > 0 {
            "review selected lanes and launch with the shown `vida agent-init` command only after operator approval"
        } else {
            "add or unblock parallel-safe execution semantics before expecting planner proposals"
        }
    })
}

fn build_carrier_selection_api_descriptor(
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let dev_team_roles = activation_bundle["dev_team_readiness"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| {
            let api_id = role["role_id"].as_str()?.trim();
            let runtime_role = role["runtime_role"].as_str()?.trim();
            let task_class = role["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .find(|value| !value.is_empty())?;
            if api_id.is_empty() || runtime_role.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "api_id": api_id,
                "runtime_role": runtime_role,
                "task_class": task_class,
                "selection_surface": "vida agent select",
                "selection_materialized": false,
                "selection_reason": "dispatch_next_preview_exposes_selection_api_without_embedding_full_assignment",
                "command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class} --json")
            }))
        })
        .collect::<Vec<_>>();
    let first_class = if dev_team_roles.is_empty() {
        activation_bundle["carrier_runtime"]["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| {
                let api_id = role["role_id"].as_str()?.trim();
                let runtime_role = role["default_runtime_role"]
                    .as_str()
                    .or_else(|| {
                        role["runtime_roles"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .find(|value| !value.trim().is_empty())
                    })?
                    .trim();
                let task_class = role["task_classes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .find(|value| !value.trim().is_empty())?
                    .trim();
                if api_id.is_empty() || runtime_role.is_empty() || task_class.is_empty() {
                    return None;
                }
                Some(serde_json::json!({
                    "api_id": api_id,
                    "runtime_role": runtime_role,
                    "task_class": task_class,
                    "selection_surface": "vida agent select",
                    "selection_materialized": false,
                    "selection_reason": "dispatch_next_preview_exposes_selection_api_without_embedding_full_assignment",
                    "command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class} --json")
                }))
            })
            .collect::<Vec<_>>()
    } else {
        dev_team_roles
    };
    serde_json::json!({
        "surface": "vida agent select",
        "mode": "config_driven_runtime_assignment",
        "status": if first_class.is_empty() { "blocked" } else { "pass" },
        "blocker_codes": if first_class.is_empty() {
            vec!["carrier_selection_api_requires_configured_dev_team_roles"]
        } else {
            Vec::<&str>::new()
        },
        "first_class_carriers": first_class,
        "manual_host_tool_choice_required": false,
        "embedded_assignment_diagnostics": false,
        "diagnostics_note": "Run the listed `vida agent select` command for full carrier/model/cost assignment diagnostics.",
    })
}

fn non_dev_team_flow_projection() -> serde_json::Value {
    serde_json::json!({
        "status": "not_applicable",
        "reason": "dev_team_preview_not_enabled",
        "diagnostic_only": true,
    })
}

fn lifecycle_hook_event_stream(
    selected_flow: Option<&serde_json::Value>,
    sequence: &[DevTeamSequenceStep],
) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    if let Some(flow) = selected_flow {
        for hook in flow["lifecycle_hook_templates"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            events.push(serde_json::json!({
                "scope": "flow",
                "template_id": hook,
                "authority": "diagnostic_event_stream",
                "configured_from": "dev_team.flows.lifecycle_hook_templates",
            }));
        }
    }
    for (index, step) in sequence.iter().enumerate() {
        for hook in step
            .lifecycle_hook_templates
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            events.push(serde_json::json!({
                "scope": "step",
                "step_index": index,
                "role_label": step.role_label,
                "template_id": hook,
                "authority": "diagnostic_event_stream",
                "configured_from": "dev_team.flows.steps.lifecycle_hook_templates",
            }));
        }
    }
    events
}

fn build_dev_team_flow_projection(
    activation_bundle: &serde_json::Value,
    selected_flow_id: Option<&str>,
    sequence: &[DevTeamSequenceStep],
    selected_lanes: &[AgentDispatchLanePreview],
    blocker_codes: &[String],
) -> serde_json::Value {
    let readiness = &activation_bundle["dev_team_readiness"];
    let selected_flow = selected_flow_id.and_then(|flow_id| {
        readiness["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
    });
    let current_lane = selected_lanes.first();
    let current_step = current_lane
        .map(|lane| {
            serde_json::json!({
                "role_label": lane.role_label,
                "runtime_role": lane.runtime_role,
                "task_class": lane.task_class,
                "task_id": lane.task_id,
                "dispatch_command": lane.dispatch_command,
                "dispatch_command_kind": lane.dispatch_command_kind,
                "receipt_status": {
                    "receipt_backed": false,
                    "receipt_path": null,
                    "status": "preview_only"
                },
                "proof_state": {
                    "status": "pending_dispatch",
                    "diagnostic_only": true
                },
                "approval_gate": lane.approval_gate,
            })
        })
        .or_else(|| {
            sequence.first().map(|step| {
                serde_json::json!({
                    "role_label": step.role_label,
                    "runtime_role": step.runtime_role,
                    "task_class": step.task_class,
                    "task_id": null,
                    "dispatch_command": null,
                    "dispatch_command_kind": null,
                    "receipt_status": {
                        "receipt_backed": false,
                        "receipt_path": null,
                        "status": "not_selected"
                    },
                    "proof_state": {
                        "status": "not_started",
                        "diagnostic_only": true
                    },
                    "approval_gate": {
                        "required": step.requires_user_approval,
                        "status": if step.requires_user_approval {
                            "approval_required_after_step_completion"
                        } else {
                            "not_required"
                        },
                        "policy": step.approval_policy,
                    },
                })
            })
        })
        .unwrap_or(serde_json::Value::Null);
    let approval_waits = selected_lanes
        .iter()
        .filter(|lane| lane.requires_user_approval)
        .map(|lane| {
            serde_json::json!({
                "task_id": lane.task_id,
                "role_label": lane.role_label,
                "status": "approval_required_after_step_completion",
                "policy": lane.approval_gate["policy"],
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if blocker_codes.is_empty() { "ready" } else { "blocked" },
        "flow_id": selected_flow.and_then(|flow| flow["flow_id"].as_str()),
        "flow_class": selected_flow.and_then(|flow| flow["flow_class"].as_str()),
        "work_item_bindings": selected_flow
            .map(|flow| flow["work_item_bindings"].clone())
            .unwrap_or(serde_json::Value::Null),
        "adapter_projection": selected_flow
            .map(|flow| flow["adapter_projection"].clone())
            .unwrap_or(serde_json::Value::Null),
        "adapter_projection_source": "dev_team.flows.adapter_projection",
        "adapter_projection_is_data_only": true,
        "proof_gates": selected_flow
            .map(|flow| flow["proof_gates"].clone())
            .unwrap_or(serde_json::Value::Null),
        "current_step": current_step,
        "steps": sequence.iter().enumerate().map(|(index, step)| {
            serde_json::json!({
                "index": index,
                "role_label": step.role_label,
                "runtime_role": step.runtime_role,
                "task_class": step.task_class,
                "requires_user_approval": step.requires_user_approval,
                "approval_policy": step.approval_policy,
                "lifecycle_hook_templates": step.lifecycle_hook_templates,
                "resume_transitions": step.resume_transitions,
                "rework_transitions": step.rework_transitions,
            })
        }).collect::<Vec<_>>(),
        "approval_waits": approval_waits,
        "lifecycle_hook_event_stream": lifecycle_hook_event_stream(selected_flow, sequence),
        "receipt_status": {
            "receipt_backed": false,
            "receipt_path": null,
            "status": "preview_only"
        },
        "proof_state": {
            "status": "pending_dispatch",
            "diagnostic_only": true
        },
        "diagnostic_only": true,
    })
}

#[derive(Debug, Clone)]
struct DevTeamSequenceStep {
    role_label: String,
    runtime_role: String,
    task_class: String,
    requires_task: bool,
    requires_user_approval: bool,
    approval_policy: serde_json::Value,
    lifecycle_hook_templates: serde_json::Value,
    resume_transitions: serde_json::Value,
    rework_transitions: serde_json::Value,
}

fn flow_matches_work_item_type(flow: &serde_json::Value, work_item_type: &str) -> bool {
    let lookup_keys = work_item_type_lookup_keys(work_item_type);
    work_item_binding_values(&flow["work_item_bindings"]).any(|value| {
        let binding_keys = work_item_type_lookup_keys(&value);
        binding_keys
            .iter()
            .any(|binding_key| lookup_keys.iter().any(|key| key == binding_key))
    })
}

fn flow_has_exact_work_item_binding(flow: &serde_json::Value, work_item_type: &str) -> bool {
    let exact_key = work_item_type.trim().to_ascii_lowercase();
    !exact_key.is_empty()
        && work_item_binding_values(&flow["work_item_bindings"])
            .any(|value| value.trim().eq_ignore_ascii_case(&exact_key))
}

fn work_item_binding_values(bindings: &serde_json::Value) -> impl Iterator<Item = String> + '_ {
    match bindings {
        serde_json::Value::String(value) => {
            Box::new(split_work_item_binding_value(value)) as Box<dyn Iterator<Item = String>>
        }
        serde_json::Value::Array(values) => Box::new(
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .flat_map(split_work_item_binding_value),
        ),
        _ => Box::new(std::iter::empty()),
    }
}

fn split_work_item_binding_value(value: &str) -> impl Iterator<Item = String> + '_ {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn single_in_progress_task_id_from_rows(rows: &[state_store::TaskRecord]) -> Option<&str> {
    let mut candidates = rows
        .iter()
        .filter(|task| task.status == "in_progress" && task.issue_type != "epic");
    let candidate = candidates.next()?;
    candidates.next().is_none().then_some(candidate.id.as_str())
}

fn work_item_type_lookup_keys(work_item_type: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let lower = work_item_type.trim().to_ascii_lowercase();
    push_unique_lookup_key(&mut keys, lower);
    let normalized = state_store::canonical_work_item_issue_type(work_item_type);
    push_unique_lookup_key(&mut keys, normalized);
    keys
}

fn push_unique_lookup_key(keys: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    let normalized = value.trim().to_ascii_lowercase();
    if !normalized.is_empty() && !keys.iter().any(|key| key == &normalized) {
        keys.push(normalized);
    }
}

fn task_flow_lookup_keys(task: &state_store::TaskRecord) -> Vec<String> {
    let mut keys = Vec::new();
    let task_value = serde_json::to_value(task).unwrap_or(serde_json::Value::Null);
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    if inferred_task_class != "implementation" {
        push_unique_lookup_key(&mut keys, inferred_task_class);
    }
    let work_item_kind = state_store::task_work_item_kind(&task.issue_type);
    push_unique_lookup_key(&mut keys, work_item_kind.canonical_issue_type);
    if let Some(provider_issue_type) = work_item_kind.provider_issue_type {
        push_unique_lookup_key(&mut keys, provider_issue_type);
    }
    push_unique_lookup_key(&mut keys, &task.issue_type);
    push_unique_lookup_key(&mut keys, work_item_kind.default_flow_binding);
    keys
}

fn selected_dev_team_flow_for_task<'a>(
    readiness: &'a serde_json::Value,
    task: &state_store::TaskRecord,
) -> Option<&'a serde_json::Value> {
    for lookup_key in task_flow_lookup_keys(task) {
        if let Some(flow) = selected_dev_team_flow_for_lookup_key(readiness, &lookup_key) {
            return Some(flow);
        }
    }
    selected_dev_team_flow_for_work_item(readiness, None)
}

fn selected_dev_team_flow_for_lookup_key<'a>(
    readiness: &'a serde_json::Value,
    work_item_type: &str,
) -> Option<&'a serde_json::Value> {
    let flows = readiness["flows"].as_array()?;
    for lookup_key in work_item_type_lookup_keys(work_item_type) {
        if let Some(flow_id) = readiness["work_item_flow_bindings"]
            .get(&lookup_key)
            .and_then(serde_json::Value::as_str)
        {
            if let Some(flow) = flows
                .iter()
                .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
            {
                return Some(flow);
            }
        }
    }
    flows
        .iter()
        .find(|flow| flow_has_exact_work_item_binding(flow, work_item_type))
        .or_else(|| {
            flows
                .iter()
                .find(|flow| flow_matches_work_item_type(flow, work_item_type))
        })
}

fn selected_dev_team_flow_for_work_item<'a>(
    readiness: &'a serde_json::Value,
    work_item_type: Option<&str>,
) -> Option<&'a serde_json::Value> {
    let flows = readiness["flows"].as_array()?;
    let normalized_type = work_item_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    if let Some(work_item_type) = normalized_type.as_deref() {
        if let Some(flow) = selected_dev_team_flow_for_lookup_key(readiness, work_item_type) {
            return Some(flow);
        }
    }
    readiness["default_flow_id"]
        .as_str()
        .and_then(|flow_id| {
            flows
                .iter()
                .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
        })
        .or_else(|| {
            flows
                .iter()
                .find(|flow| flow["default"].as_bool().unwrap_or(false))
        })
}

fn dev_team_sequence_from_readiness(
    readiness: &serde_json::Value,
    work_item_type: Option<&str>,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence_from_readiness_with_default(readiness, work_item_type, true)
}

fn dev_team_sequence_from_readiness_lookup_key(
    readiness: &serde_json::Value,
    work_item_type: &str,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence_from_readiness_with_default(readiness, Some(work_item_type), false)
}

fn dev_team_sequence_from_readiness_with_default(
    readiness: &serde_json::Value,
    work_item_type: Option<&str>,
    default_on_miss: bool,
) -> Vec<DevTeamSequenceStep> {
    let roles = readiness["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| {
            let role_id = role["role_id"].as_str()?;
            Some((role_id.to_string(), role))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let selected_flow = match (work_item_type, default_on_miss) {
        (Some(work_item_type), false) => {
            selected_dev_team_flow_for_lookup_key(readiness, work_item_type)
        }
        _ => selected_dev_team_flow_for_work_item(readiness, work_item_type),
    };
    if selected_flow.is_none() && !default_on_miss {
        return Vec::new();
    }
    if let Some(steps) = selected_flow
        .and_then(|flow| flow["ordered_steps"].as_array())
        .filter(|steps| !steps.is_empty())
    {
        return steps
            .iter()
            .filter_map(|step| {
                let role_id = step["role_id"].as_str()?;
                let role = roles.get(role_id)?;
                let runtime_role = step["runtime_role"]
                    .as_str()
                    .or_else(|| role["runtime_role"].as_str())
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)?;
                let task_class = step["task_class"]
                    .as_str()
                    .or_else(|| {
                        role["task_classes"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .find(|value| !value.trim().is_empty())
                    })
                    .map(str::to_string)?;
                Some(DevTeamSequenceStep {
                    role_label: role_id.to_string(),
                    runtime_role,
                    task_class,
                    requires_task: role_id != "release_closure" && role_id != "terminal_closure",
                    requires_user_approval: step["requires_user_approval"]
                        .as_bool()
                        .unwrap_or(false),
                    approval_policy: step["approval_policy"].clone(),
                    lifecycle_hook_templates: step["lifecycle_hook_templates"].clone(),
                    resume_transitions: step["resume_transitions"].clone(),
                    rework_transitions: step["rework_transitions"].clone(),
                })
            })
            .collect();
    }
    let sequence = readiness["sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    if sequence.is_empty() || roles.is_empty() {
        return Vec::new();
    }
    sequence
        .into_iter()
        .filter_map(|role_id| {
            let role = roles.get(role_id)?;
            let runtime_role = role["runtime_role"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)?;
            let task_class = role["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .find(|value| !value.trim().is_empty())
                .map(str::to_string)?;
            Some(DevTeamSequenceStep {
                role_label: role_id.to_string(),
                runtime_role,
                task_class,
                requires_task: role_id != "release_closure" && role_id != "terminal_closure",
                requires_user_approval: false,
                approval_policy: serde_json::Value::Null,
                lifecycle_hook_templates: serde_json::Value::Null,
                resume_transitions: serde_json::Value::Null,
                rework_transitions: serde_json::Value::Null,
            })
        })
        .collect()
}

fn dev_team_sequence_from_carrier_runtime(
    activation_bundle: &serde_json::Value,
) -> Vec<DevTeamSequenceStep> {
    activation_bundle["carrier_runtime"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| {
            let role_id = role["role_id"].as_str()?.trim();
            if role_id.is_empty() {
                return None;
            }
            let runtime_role = role["default_runtime_role"]
                .as_str()
                .or_else(|| {
                    role["runtime_roles"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .find(|value| !value.trim().is_empty())
                })?
                .trim()
                .to_string();
            let task_class = role["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .find(|value| !value.trim().is_empty())?
                .trim()
                .to_string();
            Some(DevTeamSequenceStep {
                role_label: role_id.to_string(),
                runtime_role,
                task_class,
                requires_task: true,
                requires_user_approval: false,
                approval_policy: serde_json::Value::Null,
                lifecycle_hook_templates: serde_json::Value::Null,
                resume_transitions: serde_json::Value::Null,
                rework_transitions: serde_json::Value::Null,
            })
        })
        .collect()
}

fn dev_team_sequence(activation_bundle: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
    let readiness_sequence =
        dev_team_sequence_from_readiness(&activation_bundle["dev_team_readiness"], None);
    if !readiness_sequence.is_empty() {
        return readiness_sequence;
    }
    let carrier_sequence = dev_team_sequence_from_carrier_runtime(activation_bundle);
    if !carrier_sequence.is_empty() {
        return carrier_sequence;
    }
    let Some(development_flow) = activation_bundle.get("development_flow") else {
        return Vec::new();
    };
    let Some(dispatch_contract) = development_flow.get("dispatch_contract") else {
        return Vec::new();
    };
    let execution_lane_sequence =
        crate::dispatch_contract_execution_lane_sequence(dispatch_contract);
    if execution_lane_sequence.is_empty() {
        return Vec::new();
    }

    let steps = execution_lane_sequence
        .into_iter()
        .filter_map(|dispatch_target| {
            let route = crate::dispatch_contract_lane(activation_bundle, &dispatch_target)?;
            let activation = crate::dispatch_contract_lane_activation(route);
            let runtime_role = activation
                .get("activation_runtime_role")
                .or_else(|| route.get("runtime_role"))
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)?;
            let task_class = activation
                .get("task_class")
                .or_else(|| route.get("task_class"))
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)?;
            Some(DevTeamSequenceStep {
                role_label: dispatch_target,
                runtime_role,
                task_class,
                requires_task: true,
                requires_user_approval: false,
                approval_policy: serde_json::Value::Null,
                lifecycle_hook_templates: serde_json::Value::Null,
                resume_transitions: serde_json::Value::Null,
                rework_transitions: serde_json::Value::Null,
            })
        })
        .collect::<Vec<_>>();
    steps
}

fn dev_team_sequence_for_work_item(
    activation_bundle: &serde_json::Value,
    work_item_type: &str,
) -> Vec<DevTeamSequenceStep> {
    let readiness_sequence = dev_team_sequence_from_readiness(
        &activation_bundle["dev_team_readiness"],
        Some(work_item_type),
    );
    if readiness_sequence.is_empty() {
        dev_team_sequence(activation_bundle)
    } else {
        readiness_sequence
    }
}

fn dev_team_sequence_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Vec<DevTeamSequenceStep> {
    for lookup_key in task_flow_lookup_keys(task) {
        let readiness_sequence = dev_team_sequence_from_readiness_lookup_key(
            &activation_bundle["dev_team_readiness"],
            &lookup_key,
        );
        if !readiness_sequence.is_empty() {
            return readiness_sequence;
        }
    }
    dev_team_sequence(activation_bundle)
}

fn configured_max_parallel_agents_from_activation_bundle(
    activation_bundle: &serde_json::Value,
) -> usize {
    activation_bundle["agent_system"]["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1)
}

fn dev_team_required_task_steps_to_preview(
    activation_bundle: &serde_json::Value,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
) -> usize {
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents.max(1));
    dev_team_sequence(activation_bundle)
        .into_iter()
        .take(effective_max_parallel_agents)
        .filter(|step| step.requires_task)
        .count()
}

fn agent_init_command(
    task_id: &str,
    state_dir: Option<&std::path::Path>,
    runtime_role: &str,
) -> String {
    let runtime_role = if runtime_role.trim().is_empty() {
        "worker"
    } else {
        runtime_role
    };
    let mut command = format!(
        "vida agent-init --role {} {} --json",
        crate::shell_quote(runtime_role),
        crate::shell_quote(task_id)
    );
    if let Some(state_dir) = state_dir {
        command.push_str(" --state-dir ");
        command.push_str(&crate::shell_quote(&state_dir.display().to_string()));
    }
    command
}

fn receipt_backed_execution_command_hint(task_id: &str) -> String {
    format!(
        "vida taskflow run-graph dispatch-init {} --json, then vida agent-init --dispatch-packet <packet-path> --execute-dispatch --json",
        crate::shell_quote(task_id)
    )
}

fn required_string_field(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload[key]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn selection_truth_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    selection_truth_for_task_with_role_and_class(activation_bundle, task, "worker", None, None)
}

fn selection_truth_for_task_with_role_and_class(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
    conversation_role: &str,
    runtime_role_override: Option<&str>,
    task_class_override: Option<&str>,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    let task_value = serde_json::to_value(task)
        .map_err(|error| format!("task_record_serialization_failed:{error}"))?;
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    let task_class = task_class_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or(inferred_task_class);
    let runtime_role = runtime_role_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| crate::runtime_role_for_task_class(&task_class).to_string());
    let assignment = crate::build_runtime_assignment_preview_from_resolved_constraints(
        activation_bundle,
        conversation_role,
        &task_class,
        &runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        let reason = required_string_field(&assignment, "reason")
            .unwrap_or_else(|| "runtime_assignment_disabled".to_string());
        return Err(reason);
    }

    let selected_carrier = required_string_field(&assignment, "selected_carrier_id")
        .ok_or_else(|| "selected_carrier_id_missing".to_string())?;
    let selected_backend = required_string_field(&assignment, "selected_backend_id")
        .ok_or_else(|| "selected_backend_id_missing".to_string())?;
    let selected_model_profile = required_string_field(&assignment, "selected_model_profile_id")
        .ok_or_else(|| "selected_model_profile_id_missing".to_string())?;
    let selected_model_ref = required_string_field(&assignment, "selected_model_ref")
        .ok_or_else(|| "selected_model_ref_missing".to_string())?;
    let selected_reasoning_effort = required_string_field(&assignment, "selected_reasoning_effort")
        .ok_or_else(|| "selected_reasoning_effort_missing".to_string())?;
    let budget_verdict = required_string_field(&assignment, "budget_verdict")
        .ok_or_else(|| "budget_verdict_missing".to_string())?;
    let rate = assignment["rate"]
        .as_u64()
        .ok_or_else(|| "rate_missing".to_string())?;
    let estimated_task_price_units = assignment["estimated_task_price_units"]
        .as_u64()
        .ok_or_else(|| "estimated_task_price_units_missing".to_string())?;

    Ok(AgentDispatchLaneSelectionTruth {
        selected_carrier,
        selected_backend,
        selected_model_profile,
        selected_model_ref,
        selected_reasoning_effort,
        rate,
        estimated_task_price_units,
        budget_verdict,
        selection_source_paths: assignment["selection_source_paths"].clone(),
        pricing_readiness: assignment["pricing_readiness"].clone(),
        runtime_role,
        task_class,
    })
}

fn blocked_candidate(
    candidate: &state_store::TaskSchedulingCandidate,
    reasons: Vec<String>,
) -> AgentDispatchBlockedCandidate {
    AgentDispatchBlockedCandidate {
        task_id: candidate.task.id.clone(),
        title: candidate.task.title.clone(),
        ready_now: candidate.ready_now,
        ready_parallel_safe: candidate.ready_parallel_safe,
        reasons,
        parallel_blockers: candidate.parallel_blockers.clone(),
    }
}

fn explicit_task_graph_continuation_task_id(
    binding: Option<&state_store::RunGraphContinuationBinding>,
) -> Option<&str> {
    let binding = binding?;
    if binding.status != "bound" || binding.binding_source != "explicit_continuation_bind_task" {
        return None;
    }
    if binding
        .active_bounded_unit
        .get("kind")
        .and_then(serde_json::Value::as_str)
        != Some("task_graph_task")
    {
        return None;
    }
    binding
        .active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
        .or_else(|| {
            let task_id = binding.task_id.trim();
            (!task_id.is_empty()).then_some(task_id)
        })
}

fn build_agent_dispatch_next_preview(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    dev_team: bool,
) -> AgentDispatchNextPreview {
    if dev_team {
        build_agent_dispatch_next_preview_dev_team(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
        )
    } else {
        build_agent_dispatch_next_preview_standard(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
        )
    }
}

fn build_agent_dispatch_next_preview_standard(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);

    let Some(primary) = projection.ready.first() else {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(
            "Inspect `vida task ready --json` and resolve blockers before previewing agent dispatch."
                .to_string(),
        );
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        let flow_projection = non_dev_team_flow_projection();
        return AgentDispatchNextPreview {
            status: "blocked".to_string(),
            mode: "preview".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            parallelization_planner: build_parallelization_planner(
                projection,
                lanes_requested,
                configured_max_parallel_agents,
            ),
            carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
            flow_projection,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    };

    if effective_max_parallel_agents > 0 {
        match selection_truth_for_task(activation_bundle, &primary.task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: 1,
                task_id: primary.task.id.clone(),
                title: primary.task.title.clone(),
                role_label: "default".to_string(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &primary.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &primary.task.id,
                ),
                ready_parallel_safe: primary.ready_parallel_safe,
                selection_reason: "primary_ready_task".to_string(),
                selection_truth,
                requires_user_approval: false,
                approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                primary.task.id, reason
            )),
        }
    }

    let mut remaining = effective_max_parallel_agents.saturating_sub(selected_lanes.len());
    for candidate in projection.ready.iter().skip(1) {
        if candidate.ready_parallel_safe && remaining > 0 {
            match selection_truth_for_task(activation_bundle, &candidate.task) {
                Ok(selection_truth) => {
                    selected_lanes.push(AgentDispatchLanePreview {
                        lane_index: selected_lanes.len() + 1,
                        task_id: candidate.task.id.clone(),
                        title: candidate.task.title.clone(),
                        role_label: "parallel".to_string(),
                        runtime_role: selection_truth.runtime_role.clone(),
                        task_class: selection_truth.task_class.clone(),
                        dispatch_command: agent_init_command(
                            &candidate.task.id,
                            explicit_state_dir,
                            &selection_truth.runtime_role,
                        ),
                        dispatch_command_kind: "startup_activation_view_only".to_string(),
                        receipt_backed_execution_command: receipt_backed_execution_command_hint(
                            &candidate.task.id,
                        ),
                        ready_parallel_safe: candidate.ready_parallel_safe,
                        selection_reason: "parallel_safe_ready_task".to_string(),
                        selection_truth,
                        requires_user_approval: false,
                        approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
                    });
                    remaining -= 1;
                }
                Err(reason) => blocker_codes.push(format!(
                    "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                    candidate.task.id, reason
                )),
            }
            continue;
        }

        let reasons = if candidate.ready_parallel_safe {
            vec!["effective_max_parallel_agents_cap_reached".to_string()]
        } else if candidate.parallel_blockers.is_empty() {
            vec!["parallel_safety_not_established".to_string()]
        } else {
            candidate.parallel_blockers.clone()
        };
        blocked_candidates.push(blocked_candidate(candidate, reasons));
    }

    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    let unsafe_ready_candidates = blocked_candidates
        .iter()
        .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe);
    if effective_max_parallel_agents > 1 && unsafe_ready_candidates && selected_lanes.is_empty() {
        blocker_codes.push("ambiguous_unsafe_parallel_candidates".to_string());
        next_actions.push(
            "Some ready candidates are not parallel-safe; reduce to `--lanes 1` or fix execution semantics/conflicts before multi-lane dispatch."
                .to_string(),
        );
    } else if effective_max_parallel_agents > 1 && unsafe_ready_candidates {
        next_actions.push(
            "Some ready candidates are not parallel-safe; they remain blocked candidates and are not selected for this preview."
                .to_string(),
        );
    }
    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one chosen lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner: build_parallelization_planner(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
        ),
        carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
        flow_projection: non_dev_team_flow_projection(),
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn build_agent_dispatch_next_preview_dev_team(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();
    let mut selected_ready_candidates = match projection.current_task_id.as_deref() {
        Some(current_task_id) => projection
            .ready
            .iter()
            .filter(|candidate| candidate.task.id == current_task_id)
            .collect::<Vec<_>>(),
        None => projection.ready.iter().collect::<Vec<_>>(),
    };
    if let Some(current_task_id) = projection.current_task_id.as_deref() {
        selected_ready_candidates.sort_by_key(|candidate| {
            if candidate.task.id == current_task_id {
                0
            } else {
                1
            }
        });
    }
    let ready_flow_ids = selected_ready_candidates
        .iter()
        .filter(|candidate| candidate.ready_now)
        .filter_map(|candidate| {
            selected_dev_team_flow_for_task(
                &activation_bundle["dev_team_readiness"],
                &candidate.task,
            )
            .and_then(|flow| flow["flow_id"].as_str())
            .map(str::to_string)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let sequence = if ready_flow_ids.len() == 1 {
        selected_ready_candidates
            .iter()
            .find(|candidate| candidate.ready_now)
            .map(|candidate| dev_team_sequence_for_task(activation_bundle, &candidate.task))
            .unwrap_or_else(|| dev_team_sequence(activation_bundle))
    } else {
        dev_team_sequence(activation_bundle)
    };
    let selected_flow_id = if ready_flow_ids.len() == 1 {
        ready_flow_ids.iter().next().map(String::as_str)
    } else {
        activation_bundle["dev_team_readiness"]["default_flow_id"].as_str()
    };

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    if sequence.is_empty() {
        blocker_codes.push("configured_dev_team_sequence_required".to_string());
        next_actions.push(
            "Configure dev_team_readiness roles/sequence or dispatch_contract lanes before previewing dev-team dispatch."
                .to_string(),
        );
    }
    if projection.current_task_id.is_none() && ready_flow_ids.len() > 1 {
        blocker_codes.push("ambiguous_work_item_flow_selection".to_string());
        next_actions.push(
            "Ready task candidates map to multiple configured dev_team flows; narrow the task scope or dispatch one flow class at a time."
                .to_string(),
        );
    }

    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);
    let steps_to_preview = sequence
        .iter()
        .cloned()
        .take(effective_max_parallel_agents)
        .collect::<Vec<_>>();
    if projection.ready.is_empty() {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(
            "Inspect `vida task ready --json` and resolve blockers before previewing dev-team dispatch."
                .to_string(),
        );
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        let flow_projection = build_dev_team_flow_projection(
            activation_bundle,
            selected_flow_id,
            &sequence,
            &selected_lanes,
            &blocker_codes,
        );
        return AgentDispatchNextPreview {
            status: "blocked".to_string(),
            mode: "preview-dev-team".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            parallelization_planner: build_parallelization_planner(
                projection,
                lanes_requested,
                configured_max_parallel_agents,
            ),
            carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
            flow_projection,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    }

    let current_ready_candidate = (selected_ready_candidates.len() == 1)
        .then(|| projection.current_task_id.as_deref())
        .flatten()
        .and_then(|task_id| {
            selected_ready_candidates
                .iter()
                .copied()
                .find(|candidate| candidate.task.id == task_id && candidate.ready_now)
        });
    let mut ready_index = 0;
    for (step_index, step) in steps_to_preview.into_iter().enumerate() {
        if !step.requires_task {
            next_actions.push(format!(
                "dev-team step [{}] {} is closure-oriented and does not emit a runtime launch command.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
            continue;
        }
        if step.requires_user_approval {
            next_actions.push(format!(
                "dev-team step [{}] {} will pause after receipt-backed completion for configured user approval before the next role starts.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
        }
        let candidate = if let Some(candidate) = current_ready_candidate {
            candidate
        } else {
            let Some(candidate) = selected_ready_candidates.get(ready_index).copied() else {
                blocker_codes.push(format!(
                    "dev_team_step_missing_ready_task:position={}:{}",
                    step_index + 1,
                    step.role_label
                ));
                break;
            };
            ready_index += 1;
            candidate
        };
        if !candidate.ready_now {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["task_not_ready_for_dev_team_step".to_string()],
            ));
            continue;
        }
        match selection_truth_for_task_with_role_and_class(
            activation_bundle,
            &candidate.task,
            &step.runtime_role,
            Some(&step.runtime_role),
            Some(&step.task_class),
        ) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: selected_lanes.len() + 1,
                task_id: candidate.task.id.clone(),
                title: candidate.task.title.clone(),
                role_label: step.role_label.clone(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &candidate.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &candidate.task.id,
                ),
                ready_parallel_safe: candidate.ready_parallel_safe,
                selection_reason: format!("dev_team_step_{}:{}", step_index + 1, step.role_label),
                selection_truth,
                requires_user_approval: step.requires_user_approval,
                approval_gate: serde_json::json!({
                    "required": step.requires_user_approval,
                    "status": if step.requires_user_approval {
                        "approval_required_after_step_completion"
                    } else {
                        "not_required"
                    },
                    "policy": step.approval_policy,
                    "lifecycle_hook_templates": step.lifecycle_hook_templates,
                    "resume_transitions": step.resume_transitions,
                    "rework_transitions": step.rework_transitions,
                    "prompt_template_source": if step.requires_user_approval {
                        "dev_team.flows.steps.approval_policy"
                    } else {
                        "none"
                    },
                }),
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                candidate.task.id, reason
            )),
        }
    }

    let blocked_ready_parallel = projection
        .ready
        .iter()
        .filter(|candidate| {
            Some(candidate.task.id.as_str()) != projection.current_task_id.as_deref()
                && !candidate.ready_parallel_safe
        })
        .collect::<Vec<_>>();
    for candidate in blocked_ready_parallel {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["parallel_safety_not_established".to_string()],
        ));
    }
    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one configured dev-team step; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
        next_actions.push(
            "The shown `vida agent-init --role` command is startup activation view only; receipt-backed execution requires a dispatch packet and `--execute-dispatch`."
                .to_string(),
        );
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    };
    let flow_projection = build_dev_team_flow_projection(
        activation_bundle,
        selected_flow_id,
        &sequence,
        &selected_lanes,
        &blocker_codes,
    );

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview-dev-team".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner: build_parallelization_planner(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
        ),
        carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
        flow_projection,
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn scheduler_task_record<'a>(
    plan: &'a crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    task_id: &str,
) -> Option<&'a state_store::TaskRecord> {
    plan.scheduling
        .ready
        .iter()
        .chain(plan.scheduling.blocked.iter())
        .find(|candidate| candidate.task.id == task_id)
        .map(|candidate| &candidate.task)
}

fn scheduler_task_parallel_safety(
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    task_id: &str,
) -> bool {
    plan.scheduling
        .ready
        .iter()
        .chain(plan.scheduling.blocked.iter())
        .find(|candidate| candidate.task.id == task_id)
        .is_some_and(|candidate| candidate.ready_parallel_safe)
}

fn build_agent_dispatch_next_preview_from_scheduler_plan(
    activation_bundle: &serde_json::Value,
    plan: crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    lanes_requested: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = plan.blocker_codes.clone();
    let mut next_actions = plan.next_actions.clone();
    let mut selected_lanes = Vec::new();
    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let blocked_candidates = plan
        .rejected_candidates
        .iter()
        .map(|candidate| AgentDispatchBlockedCandidate {
            task_id: candidate.task_id.clone(),
            title: candidate.task.title.clone(),
            ready_now: candidate.ready_now,
            ready_parallel_safe: candidate.ready_now && candidate.parallel_blockers.is_empty(),
            reasons: candidate.reasons.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        })
        .collect::<Vec<_>>();

    for (index, reservation) in plan.reservations.iter().enumerate() {
        let Some(task) = scheduler_task_record(&plan, &reservation.task_id) else {
            blocker_codes.push(format!(
                "selected_lane_task_record_missing:task={}",
                reservation.task_id
            ));
            continue;
        };
        match selection_truth_for_task(activation_bundle, task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: index + 1,
                task_id: reservation.task_id.clone(),
                title: reservation.task.title.clone(),
                role_label: if reservation.launch_role == "primary" {
                    "default".to_string()
                } else {
                    reservation.launch_role.clone()
                },
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &reservation.task_id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &reservation.task_id,
                ),
                ready_parallel_safe: scheduler_task_parallel_safety(&plan, &reservation.task_id),
                selection_reason: if reservation.launch_role == "primary" {
                    "scheduler_primary_ready_task".to_string()
                } else {
                    "scheduler_parallel_safe_ready_task".to_string()
                },
                selection_truth,
                requires_user_approval: false,
                approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
            }),
            Err(reason) => blocker_codes.push(format!(
                "selected_lane_runtime_assignment_truth_missing:task={}:{}",
                reservation.task_id, reason
            )),
        }
    }

    if blocker_codes
        .iter()
        .any(|code| code.starts_with("selected_lane_runtime_assignment_truth_missing:"))
        || blocker_codes
            .iter()
            .any(|code| code.starts_with("selected_lane_task_record_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push("selected_lane_runtime_assignment_truth_required".to_string());
        next_actions.push(
            "Selection truth is incomplete for at least one scheduler-selected lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if plan.max_parallel_agents > 1
        && blocked_candidates
            .iter()
            .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe)
    {
        next_actions.push(
            "Some ready candidates are not parallel-safe; they remain blocked candidates and are not selected for this preview."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }
    if lanes_requested == 0 {
        selected_lanes.clear();
    }

    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
    }
    .to_string();
    let configured_parallel =
        usize::try_from(plan.configured_max_parallel_agents).unwrap_or(usize::MAX);
    let effective_parallel = if lanes_requested == 0 {
        0
    } else {
        usize::try_from(plan.max_parallel_agents).unwrap_or(usize::MAX)
    };
    let parallelization_planner =
        build_parallelization_planner(&plan.scheduling, lanes_requested, effective_parallel);
    AgentDispatchNextPreview {
        status,
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents: configured_parallel,
        effective_max_parallel_agents: effective_parallel,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner,
        carrier_selection_api: build_carrier_selection_api_descriptor(activation_bundle),
        flow_projection: non_dev_team_flow_projection(),
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn apply_continuation_dispatch_gate_to_preview(
    preview: &mut AgentDispatchNextPreview,
    gate: &crate::taskflow_proxy::TaskflowContinuationDispatchGate,
) {
    if gate.admissible {
        return;
    }

    preview.status = "blocked".to_string();
    preview.selected_lanes.clear();
    preview.lanes_selected = 0;
    for blocker in &gate.blocker_codes {
        if !preview.blocker_codes.iter().any(|value| value == blocker) {
            preview.blocker_codes.push(blocker.clone());
        }
    }
    preview.next_actions.clear();
    for action in &gate.next_actions {
        if !preview.next_actions.iter().any(|value| value == action) {
            preview.next_actions.push(action.clone());
        }
    }
    if preview.next_actions.is_empty() {
        preview.next_actions.push(
            crate::status_surface_signals::continuation_binding_ambiguous_next_action().to_string(),
        );
    }
    fail_closed_flow_projection_for_continuation_gate(preview);
    if let Some(planner) = preview.parallelization_planner.as_object_mut() {
        planner.insert(
            "status".to_string(),
            serde_json::json!("no_packet_proposals"),
        );
        planner.insert("packet_proposals".to_string(), serde_json::json!([]));
        planner.insert("materializes_packets".to_string(), serde_json::json!(false));
        planner.insert("diagnostic_only".to_string(), serde_json::json!(true));
        planner.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
    }
}

fn fail_closed_flow_projection_for_continuation_gate(preview: &mut AgentDispatchNextPreview) {
    let blocked_proof_state = serde_json::json!({
        "status": "blocked_by_continuation_gate",
        "diagnostic_only": true
    });
    if let Some(flow_projection) = preview.flow_projection.as_object_mut() {
        flow_projection.insert("status".to_string(), serde_json::json!("blocked"));
        flow_projection.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        flow_projection.insert(
            "blocker_codes".to_string(),
            serde_json::json!(preview.blocker_codes),
        );
        flow_projection.insert(
            "next_actions".to_string(),
            serde_json::json!(preview.next_actions),
        );
        flow_projection.insert("proof_state".to_string(), blocked_proof_state.clone());
        if let Some(current_step) = flow_projection
            .get_mut("current_step")
            .and_then(serde_json::Value::as_object_mut)
        {
            current_step.insert("dispatch_command".to_string(), serde_json::Value::Null);
            current_step.insert("dispatch_command_kind".to_string(), serde_json::Value::Null);
            current_step.insert("proof_state".to_string(), blocked_proof_state);
            current_step.insert(
                "blocked_by_continuation_gate".to_string(),
                serde_json::json!(true),
            );
        }
    }
}

fn safe_agent_dispatch_projection_component(value: &str) -> String {
    let mut safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    safe.truncate(120);
    if safe.is_empty() {
        "none".to_string()
    } else {
        safe
    }
}

fn agent_dispatch_next_projection_name(command: &AgentDispatchNextArgs) -> String {
    format!(
        "agent-dispatch-next-mode-{}-lanes-{}-scope-{}-current-{}-latest",
        if command.dev_team {
            "dev-team"
        } else {
            "scheduler"
        },
        command.lanes,
        safe_agent_dispatch_projection_component(command.scope.as_deref().unwrap_or("default")),
        safe_agent_dispatch_projection_component(
            command.current_task_id.as_deref().unwrap_or("default")
        ),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AgentDispatchNextCurrentTaskIds<'a> {
    preview_current_task_id: Option<&'a str>,
    scheduler_current_task_id: Option<&'a str>,
}

fn resolve_agent_dispatch_next_current_task_ids<'a>(
    requested_current_task_id: Option<&'a str>,
    explicit_bound_current_task_id: Option<&'a str>,
    taskflow_single_in_progress_task_id: Option<&'a str>,
) -> AgentDispatchNextCurrentTaskIds<'a> {
    AgentDispatchNextCurrentTaskIds {
        preview_current_task_id: requested_current_task_id
            .or(explicit_bound_current_task_id)
            .or(taskflow_single_in_progress_task_id),
        scheduler_current_task_id: requested_current_task_id
            .or(taskflow_single_in_progress_task_id),
    }
}

fn emit_agent_dispatch_next_preview(
    command: &AgentDispatchNextArgs,
    state_dir: &std::path::Path,
    projection_name: &str,
    preview: AgentDispatchNextPreview,
) -> ExitCode {
    if command.json {
        let payload =
            serde_json::to_value(&preview).expect("agent dispatch-next preview should serialize");
        crate::print_json_pretty(&payload);
        crate::operator_projection_cache::write_json_projection(
            state_dir,
            projection_name,
            &payload,
        );
    } else {
        println!("agent dispatch-next: {}", preview.status);
        println!("lanes selected: {}", preview.lanes_selected);
        println!(
            "preview only: review carrier/model/cost selection truth before launching any `vida agent-init` command"
        );
        for lane in &preview.selected_lanes {
            println!(
                "lane {} [{}]: {} [{} / {} / rate={} / est_cost={}]",
                lane.lane_index,
                lane.role_label,
                lane.task_id,
                lane.selection_truth.selected_carrier,
                lane.selection_truth.selected_model_ref,
                lane.selection_truth.rate,
                lane.selection_truth.estimated_task_price_units
            );
        }
        if !preview.blocker_codes.is_empty() {
            println!("blockers: {}", preview.blocker_codes.join(", "));
        }
    }
    if preview.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(crate) async fn run_agent(args: AgentArgs) -> ExitCode {
    match args.command {
        AgentCommand::DispatchNext(command) => run_agent_dispatch_next(command).await,
        AgentCommand::Select(command) => run_agent_select(command).await,
    }
}

async fn run_agent_select(command: AgentSelectArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => {
            let activation_bundle = match crate::build_taskflow_consume_bundle_payload(&store).await
            {
                Ok(payload) => payload.activation_bundle,
                Err(error) => {
                    eprintln!("Failed to load activation bundle for carrier selection: {error}");
                    return ExitCode::from(1);
                }
            };
            let selection = crate::build_runtime_assignment_from_resolved_constraints(
                &activation_bundle,
                &command.conversation_role,
                &command.task_class,
                &command.runtime_role,
            );
            let status = if selection["enabled"].as_bool().unwrap_or(false) {
                "pass"
            } else {
                "blocked"
            };
            let payload = serde_json::json!({
                "surface": "vida agent select",
                "status": status,
                "mode": "config_driven_runtime_assignment",
                "runtime_role": command.runtime_role,
                "task_class": command.task_class,
                "conversation_role": command.conversation_role,
                "selection": selection,
                "manual_host_tool_choice_required": false,
                "source_surfaces": [
                    "vida.config.yaml",
                    "build_runtime_assignment_from_resolved_constraints",
                    "carrier_runtime.roles"
                ],
            });
            if command.json {
                crate::print_json_pretty(&payload);
            } else {
                println!(
                    "agent select: {}",
                    payload["status"].as_str().unwrap_or("unknown")
                );
                if let Some(carrier) = payload["selection"]["selected_carrier_id"].as_str() {
                    println!("selected carrier: {carrier}");
                }
                if let Some(profile) = payload["selection"]["selected_model_profile_id"].as_str() {
                    println!("selected model profile: {profile}");
                }
            }
            if status == "pass" {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_agent_dispatch_next(command: AgentDispatchNextArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let explicit_state_dir = command.state_dir.as_deref();
    let projection_name = agent_dispatch_next_projection_name(&command);
    let cache_read_allowed = command.current_task_id.is_some();
    if command.json && cache_read_allowed {
        if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
            &state_dir,
            &projection_name,
        ) {
            println!("{cached}");
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                &state_dir,
                &projection_name,
                AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE,
            )
        {
            println!("{cached}");
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_state_stale_recent_json_projection(
                &state_dir,
                &projection_name,
                AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE,
            )
        {
            if let Some(overlay) =
                crate::operator_projection_cache::read_runtime_continuation_binding_overlay(
                    &state_dir,
                )
            {
                if let Some(rendered) =
                    crate::operator_projection_cache::apply_runtime_continuation_binding_overlay_to_payload(
                        &state_dir,
                        &cached,
                        &overlay,
                    )
                {
                    println!("{rendered}");
                    return ExitCode::SUCCESS;
                }
            }
        }
    }
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => {
            let mut activation_bundle =
                match crate::read_or_sync_launcher_activation_snapshot(&store).await {
                    Ok(snapshot) => snapshot.compiled_bundle,
                    Err(error) => {
                        eprintln!(
                            "Failed to load activation bundle for agent dispatch preview: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
            let explicit_binding = if command.current_task_id.is_none() {
                match store
                    .latest_explicit_run_graph_continuation_binding_for_current_session()
                    .await
                {
                    Ok(binding) => binding,
                    Err(error) => {
                        eprintln!("Failed to read latest explicit continuation binding: {error}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                None
            };
            let explicit_bound_current_task_id =
                explicit_task_graph_continuation_task_id(explicit_binding.as_ref())
                    .map(str::to_string);
            let taskflow_single_in_progress_task_id =
                if command.current_task_id.is_none() && explicit_bound_current_task_id.is_none() {
                    StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root())
                        .ok()
                        .and_then(|rows| {
                            single_in_progress_task_id_from_rows(&rows).map(str::to_string)
                        })
                } else {
                    None
                };
            let resolved_current_task_ids = resolve_agent_dispatch_next_current_task_ids(
                command.current_task_id.as_deref(),
                explicit_bound_current_task_id.as_deref(),
                taskflow_single_in_progress_task_id.as_deref(),
            );
            let preview = if command.dev_team {
                let configured_max_parallel_agents =
                    configured_max_parallel_agents_from_activation_bundle(&activation_bundle);
                let projection =
                    match StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root()) {
                        Ok(rows) => {
                            let critical_path_ids = match StateStore::critical_path_from_rows(&rows)
                            {
                                Ok(path) => path
                                    .nodes
                                    .into_iter()
                                    .map(|node| node.id)
                                    .collect::<std::collections::BTreeSet<_>>(),
                                Err(_) => std::collections::BTreeSet::new(),
                            };
                            match StateStore::scheduling_projection_scoped_from_rows(
                                &rows,
                                command.scope.as_deref(),
                                resolved_current_task_ids.preview_current_task_id,
                                &critical_path_ids,
                            ) {
                                Ok(projection) => projection,
                                Err(error) => {
                                    eprintln!("Failed to compute agent dispatch preview: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        Err(_) => match store
                            .scheduling_projection_scoped(
                                command.scope.as_deref(),
                                resolved_current_task_ids.preview_current_task_id,
                            )
                            .await
                        {
                            Ok(projection) => projection,
                            Err(error) => {
                                eprintln!("Failed to compute agent dispatch preview: {error}");
                                return ExitCode::from(1);
                            }
                        },
                    };
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
                let required_task_steps = dev_team_required_task_steps_to_preview(
                    &activation_bundle,
                    command.lanes,
                    configured_max_parallel_agents,
                );
                let continuation_gate_required = command.current_task_id.is_none()
                    && required_task_steps > 0
                    && projection.ready.len() >= required_task_steps;
                let continuation_gate = if continuation_gate_required {
                    match crate::taskflow_proxy::build_taskflow_continuation_dispatch_gate_from_store(
                        &store,
                        &state_dir,
                        command.scope.as_deref(),
                    )
                    .await
                    {
                        Ok(gate) => gate,
                        Err(error) => {
                            eprintln!("Failed to compute agent continuation gate: {error}");
                            return ExitCode::from(1);
                        }
                    }
                } else {
                    None
                };
                drop(store);
                let mut preview = build_agent_dispatch_next_preview(
                    &activation_bundle,
                    &projection,
                    command.lanes,
                    configured_max_parallel_agents,
                    explicit_state_dir,
                    true,
                );
                if let Some(gate) = continuation_gate {
                    apply_continuation_dispatch_gate_to_preview(&mut preview, &gate);
                }
                preview
            } else {
                let requested_parallel_limit = u64::try_from(command.lanes).ok();
                let plan =
                    match crate::taskflow_proxy::build_taskflow_scheduler_dispatch_plan_from_store(
                        &store,
                        &state_dir,
                        command.scope.as_deref(),
                        resolved_current_task_ids.scheduler_current_task_id,
                        requested_parallel_limit,
                        true,
                        false,
                    )
                    .await
                    {
                        Ok(plan) => plan,
                        Err(error) => {
                            eprintln!("Failed to compute agent dispatch preview: {error}");
                            return ExitCode::from(1);
                        }
                    };
                drop(store);
                build_agent_dispatch_next_preview_from_scheduler_plan(
                    &activation_bundle,
                    plan,
                    command.lanes,
                    explicit_state_dir,
                )
            };
            emit_agent_dispatch_next_preview(&command, &state_dir, &projection_name, preview)
        }
        Err(error) => {
            if command.dev_team {
                let Some(current_task_id) = command.current_task_id.as_deref() else {
                    eprintln!("Failed to open authoritative state store: {error}");
                    return ExitCode::from(1);
                };
                let mut activation_bundle = match capture_launcher_activation_snapshot() {
                    Ok(snapshot) => snapshot.compiled_bundle,
                    Err(snapshot_error) => {
                        eprintln!(
                            "Failed to load activation bundle for agent dispatch preview: {snapshot_error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                let rows = match StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_dir) {
                    Ok(rows) => rows,
                    Err(fresh_error) => {
                        let snapshot_path =
                            StateStore::canonical_task_snapshot_path_for_state_root(&state_dir);
                        match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
                            Ok(rows) => rows,
                            Err(snapshot_error) => {
                                eprintln!("Failed to open authoritative state store: {error}");
                                eprintln!(
                                    "Failed to read canonical task snapshot after authoritative open failure: {snapshot_error}; fresh snapshot error: {fresh_error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                };
                let critical_path_ids = match StateStore::critical_path_from_rows(&rows) {
                    Ok(path) => path
                        .nodes
                        .into_iter()
                        .map(|node| node.id)
                        .collect::<std::collections::BTreeSet<_>>(),
                    Err(_) => std::collections::BTreeSet::new(),
                };
                let projection = match StateStore::scheduling_projection_scoped_from_rows(
                    &rows,
                    command.scope.as_deref(),
                    Some(current_task_id),
                    &critical_path_ids,
                ) {
                    Ok(projection) => projection,
                    Err(projection_error) => {
                        eprintln!("Failed to compute agent dispatch preview: {projection_error}");
                        return ExitCode::from(1);
                    }
                };
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
                let configured_max_parallel_agents =
                    configured_max_parallel_agents_from_activation_bundle(&activation_bundle);
                let mut preview = build_agent_dispatch_next_preview(
                    &activation_bundle,
                    &projection,
                    command.lanes,
                    configured_max_parallel_agents,
                    explicit_state_dir,
                    true,
                );
                preview.source_surfaces.push(
                    "StateStore::read_fresh_tasks_from_jsonl_snapshot(authoritative-open-fallback)"
                        .to_string(),
                );
                emit_agent_dispatch_next_preview(&command, &state_dir, &projection_name, preview)
            } else {
                eprintln!("Failed to open authoritative state store: {error}");
                ExitCode::from(1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_continuation_dispatch_gate_to_preview, build_agent_dispatch_next_preview,
        dev_team_sequence, dev_team_sequence_for_task, dev_team_sequence_for_work_item,
        resolve_agent_dispatch_next_current_task_ids, single_in_progress_task_id_from_rows,
        state_store,
    };
    use crate::state_store::{
        CreateTaskRequest, TaskExecutionSemantics, TaskRecord, TaskSchedulingCandidate,
        TaskSchedulingProjection,
    };
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, EnvVarGuard};
    use crate::AgentDispatchNextArgs;
    use std::process::ExitCode;

    #[test]
    fn agent_dispatch_next_scheduler_keeps_explicit_binding_implicit() {
        let resolved =
            resolve_agent_dispatch_next_current_task_ids(None, Some("explicit-bound"), None);

        assert_eq!(resolved.preview_current_task_id, Some("explicit-bound"));
        assert_eq!(resolved.scheduler_current_task_id, None);
    }

    #[test]
    fn agent_dispatch_next_scheduler_preserves_operator_requested_current_task() {
        let resolved = resolve_agent_dispatch_next_current_task_ids(
            Some("requested"),
            Some("explicit-bound"),
            Some("single-in-progress"),
        );

        assert_eq!(resolved.preview_current_task_id, Some("requested"));
        assert_eq!(resolved.scheduler_current_task_id, Some("requested"));
    }

    #[test]
    fn agent_dispatch_next_scheduler_preserves_single_in_progress_fallback() {
        let resolved =
            resolve_agent_dispatch_next_current_task_ids(None, None, Some("single-in-progress"));

        assert_eq!(resolved.preview_current_task_id, Some("single-in-progress"));
        assert_eq!(
            resolved.scheduler_current_task_id,
            Some("single-in-progress")
        );
    }

    trait StateStoreFixtureTaskExt {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        >;
    }

    impl StateStoreFixtureTaskExt for crate::StateStore {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        > {
            Box::pin(async move {
                let crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id,
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                } = request;
                let generated_parent_id = (issue_type != "epic" && parent_id.is_none())
                    .then(|| format!("{task_id}-fixture-parent"));
                if let Some(parent_task_id) = generated_parent_id.as_deref() {
                    let parent_labels: Vec<String> = Vec::new();
                    let parent_status = if matches!(status.trim(), "closed" | "completed") {
                        "closed"
                    } else {
                        "open"
                    };
                    self.create_task(crate::state_store::CreateTaskRequest {
                        task_id: parent_task_id,
                        title: "Fixture parent epic",
                        display_id: None,
                        description: "Test-only parent epic for strict task hierarchy fixtures",
                        issue_type: "epic",
                        status: parent_status,
                        priority,
                        parent_id: None,
                        labels: &parent_labels,
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by,
                        source_repo,
                    })
                    .await?;
                }
                self.create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id: parent_id.or(generated_parent_id.as_deref()),
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                })
                .await
            })
        }
    }

    fn task_with_labels(id: &str, title: &str, labels: &[&str]) -> TaskRecord {
        task_with_labels_and_type(id, title, labels, "task")
    }

    fn task_with_labels_and_type(
        id: &str,
        title: &str,
        labels: &[&str],
        issue_type: &str,
    ) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: title.to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 2,
            issue_type: issue_type.to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn candidate(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
    ) -> TaskSchedulingCandidate {
        candidate_with_labels(
            id,
            title,
            ready_now,
            ready_parallel_safe,
            parallel_blockers,
            &[],
        )
    }

    fn candidate_with_labels(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
        labels: &[&str],
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels(id, title, labels),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers,
        }
    }

    fn candidate_with_type(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        issue_type: &str,
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels_and_type(id, title, &[], issue_type),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers: Vec::new(),
        }
    }

    #[test]
    fn single_in_progress_task_id_from_rows_selects_only_non_epic_active_task() {
        let mut active = task_with_labels_and_type("task-active", "Active task", &[], "task");
        active.status = "in_progress".to_string();
        let mut epic = task_with_labels_and_type("epic-active", "Active epic", &[], "epic");
        epic.status = "in_progress".to_string();

        assert_eq!(
            single_in_progress_task_id_from_rows(&[epic, active]),
            Some("task-active")
        );
    }

    #[test]
    fn single_in_progress_task_id_from_rows_fails_closed_for_multiple_active_tasks() {
        let mut first = task_with_labels_and_type("task-first", "First task", &[], "task");
        first.status = "in_progress".to_string();
        let mut second = task_with_labels_and_type("task-second", "Second task", &[], "task");
        second.status = "in_progress".to_string();

        assert_eq!(single_in_progress_task_id_from_rows(&[first, second]), None);
    }

    #[test]
    fn single_in_progress_task_id_from_rows_fails_closed_without_active_task() {
        assert_eq!(
            single_in_progress_task_id_from_rows(&[task_with_labels_and_type(
                "task-open",
                "Open task",
                &[],
                "task",
            )]),
            None
        );
    }

    fn activation_bundle_with_worker_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "junior",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation", "verification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "write_scope": "scoped_only",
                        "model_profiles": {
                            "gpt-5.5-low": {
                                "profile_id": "gpt-5.5-low",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation", "verification"],
                                "normalized_cost_units": 1
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "junior": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational"
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_dev_team_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "analyst-seat",
                        "tier": "senior",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["specification"],
                        "model_profiles": {
                            "analyst": {
                                "profile_id": "analyst-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["business_analyst"],
                                "task_classes": ["specification"],
                                "normalized_cost_units": 1
                            }
                        }
                    },
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    },
                    {
                        "role_id": "coach-seat",
                        "tier": "middle",
                        "default_runtime_role": "coach",
                        "runtime_roles": ["coach"],
                        "task_classes": ["coach"],
                        "normalized_cost_units": 3,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["coach"],
                        "model_profiles": {
                            "coach": {
                                "profile_id": "coach-profile",
                                "model_ref": "gpt-5.5-coach",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["coach"],
                                "task_classes": ["coach"],
                                "normalized_cost_units": 3
                            }
                        }
                    },
                    {
                        "role_id": "verifier-seat",
                        "tier": "middle",
                        "default_runtime_role": "verifier",
                        "runtime_roles": ["verifier", "prover"],
                        "task_classes": ["verification"],
                        "normalized_cost_units": 4,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["verification"],
                        "model_profiles": {
                            "prover": {
                                "profile_id": "verifier-profile",
                                "model_ref": "gpt-5.3",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["verifier", "prover"],
                                "task_classes": ["verification"],
                                "normalized_cost_units": 4
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "analyst-seat": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        },
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        },
                        "coach-seat": {
                            "effective_score": 74,
                            "lifecycle_state": "active"
                        },
                        "verifier-seat": {
                            "effective_score": 76,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_role_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_model_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "",
                                "model_ref": "",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_price_data_blocked() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": false
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_price_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn assertion_message_contains_actionable_blocker(blocker_codes: &[String], task_id: &str) {
        let expected_prefix =
            format!("selected_lane_runtime_assignment_truth_missing:task={task_id}:");
        assert!(blocker_codes
            .iter()
            .any(|code| code.starts_with(&expected_prefix)));
    }

    #[test]
    fn agent_dispatch_next_preview_selects_parallel_safe_lanes_with_commands() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            2,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            false,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 2);
        assert_eq!(preview.configured_max_parallel_agents, 4);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert_eq!(preview.selected_lanes[0].task_class, "implementation");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "junior"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert!(preview.selected_lanes[0]
            .selection_truth
            .selection_source_paths["selected_rate"]
            .as_str()
            .is_some_and(
                |path| path.starts_with("carrier_runtime.roles[junior].model_profiles.")
                    && path.ends_with(".normalized_cost_units")
            ));
        assert_eq!(
            preview.selected_lanes[0].selection_truth.pricing_readiness["pricing_freshness_status"],
            "missing"
        );
        assert!(preview.selected_lanes[1]
            .dispatch_command
            .contains("--state-dir /tmp/vida-state"));
        assert_eq!(
            preview.parallelization_planner["status"],
            "proposals_available"
        );
        assert_eq!(
            preview.parallelization_planner["materializes_packets"],
            false
        );
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| proposals.len() == 2));
        assert_eq!(
            preview.carrier_selection_api["surface"],
            "vida agent select"
        );
        assert_eq!(preview.carrier_selection_api["status"], "pass");
        assert!(preview.carrier_selection_api["first_class_carriers"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["api_id"] == "junior")));
    }

    #[test]
    fn agent_dispatch_next_preview_blocks_no_ready_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: None,
            ready: Vec::new(),
            blocked: vec![candidate(
                "task-blocked",
                "Blocked",
                false,
                false,
                vec!["graph_blocked".to_string()],
            )],
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert_eq!(preview.blocker_codes, vec!["no_ready_task_candidates"]);
        assert_eq!(preview.blocked_candidates[0].task_id, "task-blocked");
    }

    #[test]
    fn agent_dispatch_next_preview_selects_primary_and_reports_unsafe_parallel_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate(
                    "task-b",
                    "Task B",
                    true,
                    false,
                    vec!["execution_mode_not_parallel_safe".to_string()],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(
            preview.selected_lanes[0].dispatch_command_kind,
            "startup_activation_view_only"
        );
        assert!(preview.selected_lanes[0]
            .receipt_backed_execution_command
            .contains("--execute-dispatch"));
        assert!(preview.blocker_codes.is_empty());
        assert_eq!(preview.blocked_candidates[0].task_id, "task-b");
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("remain blocked candidates and are not selected")));
    }

    #[test]
    fn agent_dispatch_next_preview_clamps_requested_lanes_to_configured_max() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
                candidate("task-d", "Task D", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            2,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 4);
        assert_eq!(preview.configured_max_parallel_agents, 2);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert!(preview.blocked_candidates.iter().any(
            |candidate| candidate.reasons == vec!["effective_max_parallel_agents_cap_reached"]
        ));
    }

    #[test]
    fn agent_dispatch_next_preview_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({}),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
    }

    #[test]
    fn dev_team_sequence_uses_configured_flow_ordered_step_overrides() {
        let sequence = dev_team_sequence(&serde_json::json!({
            "dev_team_readiness": {
                "default_flow_id": "debug_flow",
                "roles": [
                    {
                        "role_id": "analyst",
                        "runtime_role": "business_analyst",
                        "task_classes": ["specification"]
                    },
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "task_classes": ["implementation"]
                    }
                ],
                "sequence": ["developer"],
                "flows": [
                    {
                        "flow_id": "debug_flow",
                        "enabled": true,
                        "default": true,
                        "ordered_steps": [
                            {
                                "role_id": "analyst",
                                "runtime_role": "solution_architect",
                                "task_class": "architecture"
                            },
                            {
                                "role_id": "developer"
                            }
                        ]
                    }
                ]
            }
        }));

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].role_label, "analyst");
        assert_eq!(sequence[0].runtime_role, "solution_architect");
        assert_eq!(sequence[0].task_class, "architecture");
        assert_eq!(sequence[1].role_label, "developer");
        assert_eq!(sequence[1].runtime_role, "worker");
        assert_eq!(sequence[1].task_class, "implementation");
    }

    #[test]
    fn development_flow_binding_selects_sequence_by_work_item_type() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [
                                {"role_id": "developer"}
                            ]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [
                                {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                                {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                            ]
                        }
                    ]
                }
            }),
            "defect",
        );

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].role_label, "analyst");
        assert_eq!(sequence[0].task_class, "specification");
        assert_eq!(sequence[1].role_label, "tester");
        assert_eq!(sequence[1].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_selects_sequence_by_canonical_work_item_alias() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "tester");
        assert_eq!(sequence[0].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_prefers_explicit_alias_binding_over_canonical_binding() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair",
                        "bug": "bug_triage"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                        {"role_id": "triager", "runtime_role": "business_analyst", "task_classes": ["triage"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        },
                        {
                            "flow_id": "bug_triage",
                            "enabled": true,
                            "work_item_bindings": ["bug"],
                            "ordered_steps": [{"role_id": "triager"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "triager");
        assert_eq!(sequence[0].task_class, "triage");
    }

    #[test]
    fn development_flow_fallback_prefers_explicit_alias_binding_over_canonical_binding() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                        {"role_id": "triager", "runtime_role": "business_analyst", "task_classes": ["triage"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        },
                        {
                            "flow_id": "bug_triage",
                            "enabled": true,
                            "work_item_bindings": ["bug"],
                            "ordered_steps": [{"role_id": "triager"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "triager");
        assert_eq!(sequence[0].task_class, "triage");
    }

    #[test]
    fn task_sequence_skips_default_on_inferred_key_miss_before_canonical_work_item() {
        let task = task_with_labels_and_type(
            "defect-review",
            "Verify defect remediation",
            &["verification"],
            "defect",
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "tester");
        assert_eq!(sequence[0].runtime_role, "verifier");
        assert_eq!(sequence[0].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_prefers_task_class_for_generic_task_kind() {
        let task = task_with_labels(
            "architecture-task",
            "Architecture migration task",
            &["architecture"],
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery",
                        "architecture": "architecture_design"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "architecture_design",
                            "enabled": true,
                            "work_item_bindings": ["architecture"],
                            "ordered_steps": [{"role_id": "architect", "runtime_role": "solution_architect", "task_class": "architecture"}]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "architect");
        assert_eq!(sequence[0].runtime_role, "solution_architect");
        assert_eq!(sequence[0].task_class, "architecture");
    }

    #[test]
    fn development_flow_binding_selects_sequence_from_scalar_comma_bindings() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "minimal",
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "coach", "runtime_role": "coach", "task_classes": ["coach"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "minimal",
                            "enabled": true,
                            "default": true,
                            "work_item_bindings": "task",
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "reviewed",
                            "enabled": true,
                            "work_item_bindings": "epic,task",
                            "ordered_steps": [{"role_id": "coach"}]
                        }
                    ]
                }
            }),
            "epic",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "coach");
        assert_eq!(sequence[0].task_class, "coach");
    }

    #[test]
    fn development_flow_binding_blocks_mixed_ready_flow_classes_without_current_task() {
        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({
                "agent_system": {"max_parallel_agents": 2},
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery",
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            &TaskSchedulingProjection {
                current_task_id: None,
                ready: vec![
                    candidate_with_type("task-a", "Task A", true, true, "task"),
                    candidate_with_type("defect-a", "Defect A", true, true, "defect"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"ambiguous_work_item_flow_selection".to_string()));
    }

    #[test]
    fn development_flow_binding_uses_current_task_before_mixed_ready_flow_classes() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery",
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [{"role_id": "developer"}]
                },
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [{"role_id": "tester"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![
                    candidate_with_type("task-a", "Task A", true, true, "task"),
                    candidate_with_type("defect-a", "Defect A", true, true, "defect"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[0].role_label, "tester");
        assert!(!preview
            .blocker_codes
            .contains(&"ambiguous_work_item_flow_selection".to_string()));
    }

    #[test]
    fn development_flow_binding_orders_current_task_first_with_same_ready_flow_class() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [{"role_id": "developer"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("task-active".to_string()),
                ready: vec![
                    candidate_with_type("task-other", "Other task", true, true, "task"),
                    candidate_with_type("task-active", "Active task", true, true, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "task-active");
        assert_eq!(preview.selected_lanes[0].role_label, "developer");
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_honors_current_task_for_same_flow_ready_candidates() {
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &TaskSchedulingProjection {
                current_task_id: Some("zzz-bound".to_string()),
                ready: vec![
                    candidate_with_type("aaa-other", "Other specification", true, true, "task"),
                    candidate_with_type("zzz-bound", "Bound specification", true, true, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "zzz-bound");
        assert!(preview.selected_lanes[0]
            .dispatch_command
            .contains("vida agent-init --role business_analyst zzz-bound --json"));
    }

    #[test]
    fn development_flow_binding_reuses_current_task_for_ordered_role_steps() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "defect_repair",
            "work_item_flow_bindings": {
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "flows": [
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![candidate_with_type(
                    "defect-a", "Defect A", true, false, "defect",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 2);
        assert_eq!(preview.selected_lanes[0].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[0].role_label, "analyst");
        assert_eq!(preview.selected_lanes[0].task_class, "specification");
        assert_eq!(preview.selected_lanes[1].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[1].role_label, "tester");
        assert_eq!(preview.selected_lanes[1].task_class, "verification");
        assert!(preview.blocker_codes.is_empty());
    }

    #[test]
    fn flow_projection_projects_user_approval_step_gate_and_rework_policy() {
        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({
                "agent_system": {"max_parallel_agents": 1},
                "carrier_runtime": {
                    "roles": [{
                        "role_id": "middle",
                        "tier": "middle",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "rate": 4,
                        "model": "gpt-5.5",
                        "model_provider": "openai",
                        "model_reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "readiness": {"status": "ready"},
                        "lifecycle": {"state": "ready"}
                    }]
                },
                "dev_team_readiness": {
                    "default_flow_id": "approval_flow",
                    "roles": [{
                        "role_id": "analyst",
                        "runtime_role": "business_analyst",
                        "task_classes": ["specification"]
                    }],
                    "flows": [{
                        "flow_id": "approval_flow",
                        "enabled": true,
                        "default": true,
                        "ordered_steps": [{
                            "role_id": "analyst",
                            "runtime_role": "business_analyst",
                            "task_class": "specification",
                            "requires_user_approval": true,
                            "approval_policy": {
                                "mode": "user_review_required",
                                "prompt_template": "review_document_before_next_role"
                            },
                            "lifecycle_hook_templates": ["approval_wait", "approval_complete"],
                            "resume_transitions": {"approved": "developer"},
                            "rework_transitions": {"rework": "analyst"}
                        }]
                    }]
                }
            }),
            &TaskSchedulingProjection {
                current_task_id: Some("task-approval".to_string()),
                ready: vec![candidate_with_type(
                    "task-approval",
                    "Approval task",
                    true,
                    true,
                    "task",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.selected_lanes.len(), 1);
        let lane = &preview.selected_lanes[0];
        assert!(lane.requires_user_approval);
        assert_eq!(
            lane.approval_gate["status"],
            "approval_required_after_step_completion"
        );
        assert_eq!(
            lane.approval_gate["policy"]["prompt_template"],
            "review_document_before_next_role"
        );
        assert_eq!(
            lane.approval_gate["rework_transitions"]["rework"],
            "analyst"
        );
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("will pause after receipt-backed completion")));
        assert_eq!(preview.flow_projection["flow_id"], "approval_flow");
        assert_eq!(
            preview.flow_projection["current_step"]["role_label"],
            "analyst"
        );
        assert_eq!(
            preview.flow_projection["current_step"]["receipt_status"]["status"],
            "preview_only"
        );
        assert_eq!(
            preview.flow_projection["approval_waits"][0]["policy"]["prompt_template"],
            "review_document_before_next_role"
        );
        assert_eq!(
            preview.flow_projection["lifecycle_hook_event_stream"][0]["template_id"],
            "approval_wait"
        );
        assert_eq!(
            preview.flow_projection["adapter_projection_source"],
            "dev_team.flows.adapter_projection"
        );
        assert_eq!(
            preview.flow_projection["adapter_projection_is_data_only"],
            true
        );
    }

    #[test]
    fn agent_dispatch_next_preview_renders_configured_dev_team_sequence() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate_with_labels(
                    "task-analyst",
                    "Specification task",
                    true,
                    true,
                    Vec::new(),
                    &["documentation"],
                ),
                candidate_with_labels(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                    &[],
                ),
                candidate_with_labels(
                    "task-coach",
                    "Coach review task",
                    true,
                    true,
                    Vec::new(),
                    &["coach"],
                ),
                candidate_with_labels(
                    "task-tester",
                    "Tester verification",
                    true,
                    true,
                    Vec::new(),
                    &["tester"],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            4,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert_eq!(preview.selected_lanes[0].role_label, "analyst-seat");
        assert_eq!(preview.selected_lanes[1].role_label, "developer-seat");
        assert_eq!(preview.selected_lanes[2].role_label, "coach-seat");
        assert_eq!(preview.selected_lanes[3].role_label, "verifier-seat");
        assert_eq!(preview.selected_lanes[0].task_id, "task-analyst");
        assert_eq!(preview.selected_lanes[1].task_id, "task-developer");
        assert_eq!(preview.selected_lanes[2].task_id, "task-coach");
        assert_eq!(preview.selected_lanes[3].task_id, "task-tester");
        assert_eq!(preview.selected_lanes[0].runtime_role, "business_analyst");
        assert_eq!(preview.selected_lanes[1].runtime_role, "worker");
        assert_eq!(preview.selected_lanes[2].runtime_role, "coach");
        assert_eq!(preview.selected_lanes[3].runtime_role, "verifier");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.task_class,
            "specification"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.task_class,
            "implementation"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.task_class,
            "coach"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.task_class,
            "verification"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "analyst-seat"
        );
        assert_eq!(
            preview.selected_lanes[0]
                .selection_truth
                .selected_model_profile,
            "analyst-profile"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_carrier,
            "developer-seat"
        );
        assert_eq!(
            preview.selected_lanes[1]
                .selection_truth
                .selected_model_profile,
            "developer-profile"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[1].selection_truth.rate, 2);
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_carrier,
            "coach-seat"
        );
        assert_eq!(
            preview.selected_lanes[2]
                .selection_truth
                .selected_model_profile,
            "coach-profile"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_model_ref,
            "gpt-5.5-coach"
        );
        assert_eq!(preview.selected_lanes[2].selection_truth.rate, 3);
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_carrier,
            "verifier-seat"
        );
        assert_eq!(
            preview.selected_lanes[3]
                .selection_truth
                .selected_model_profile,
            "verifier-profile"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_model_ref,
            "gpt-5.3"
        );
        assert_eq!(preview.selected_lanes[3].selection_truth.rate, 4);
        assert!(
            preview.selected_lanes[0]
                .dispatch_command
                .contains("vida agent-init --role business_analyst task-analyst --json --state-dir /tmp/vida-state")
        );
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_uses_only_configured_registry_roles() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate("task-analyst", "Specification task", true, true, Vec::new()),
                candidate(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                ),
                candidate("task-coach", "Coach review task", true, true, Vec::new()),
                candidate("task-tester", "Tester verification", true, true, Vec::new()),
                candidate("task-unused", "Unused final task", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            5,
            5,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert!(!preview
            .next_actions
            .iter()
            .any(|action| action.contains("closure-oriented")));
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_role_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_role_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_carrier_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_model_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_model_profile_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_price_policy() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_price_data_blocked(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_rate_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_price_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_rate_missing")));
        assert!(preview.blocked_candidates.is_empty());
    }

    #[test]
    fn agent_dispatch_next_preview_exposes_dispatch_flow_discovery_surfaces() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert!(preview.source_surfaces.iter().any(|surface| {
            surface == "vida taskflow graph-summary --json"
                || surface == "vida taskflow scheduler dispatch --json"
        }));
        assert!(
            preview.source_surfaces.iter().any(
                |surface| surface
                    == "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            )
        );
        assert!(preview
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida agent-init --role worker <task-id> --json"));
    }

    #[test]
    fn agent_dispatch_next_preview_gate_clears_selected_lanes() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.lanes_selected, 1);
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| !proposals.is_empty()));

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "terminal_continue_snapshot_without_next_bounded_unit"
                    .to_string(),
                blocker_codes: vec![
                    "terminal_continue_snapshot_without_next_bounded_unit".to_string(),
                    "continuation_binding_ambiguous".to_string(),
                ],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert!(preview
            .blocker_codes
            .contains(&"terminal_continue_snapshot_without_next_bounded_unit".to_string()));
        assert!(preview
            .blocker_codes
            .contains(&"continuation_binding_ambiguous".to_string()));
        assert!(preview
            .next_actions
            .contains(&"bind an explicit next bounded unit".to_string()));
        assert_eq!(
            preview.parallelization_planner["status"],
            "no_packet_proposals"
        );
        assert_eq!(preview.parallelization_planner["diagnostic_only"], true);
        assert_eq!(
            preview.parallelization_planner["blocked_by_continuation_gate"],
            true
        );
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| proposals.is_empty()));
    }

    #[test]
    fn continuation_gate_blocks_flow_projection_dispatch_state() {
        let mut activation_bundle = activation_bundle_with_worker_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "implementation_flow",
            "roles": [{
                "role_id": "worker",
                "runtime_role": "worker",
                "task_classes": ["implementation"]
            }],
            "flows": [{
                "flow_id": "implementation_flow",
                "enabled": true,
                "default": true,
                "ordered_steps": [{
                    "role_id": "worker",
                    "runtime_role": "worker",
                    "task_class": "implementation"
                }]
            }]
        });
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview =
            build_agent_dispatch_next_preview(&activation_bundle, &projection, 1, 4, None, true);
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.flow_projection["status"], "ready");
        assert_eq!(
            preview.flow_projection["current_step"]["proof_state"]["status"],
            "pending_dispatch"
        );
        assert!(preview.flow_projection["current_step"]["dispatch_command"].is_string());

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "continuation_binding_ambiguous".to_string(),
                blocker_codes: vec!["continuation_binding_ambiguous".to_string()],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert_eq!(
            preview.flow_projection["proof_state"]["status"],
            "blocked_by_continuation_gate"
        );
        assert_eq!(
            preview.flow_projection["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.flow_projection["blocker_codes"],
            serde_json::json!(["continuation_binding_ambiguous"])
        );
        assert_eq!(
            preview.flow_projection["next_actions"],
            serde_json::json!(["bind an explicit next bounded unit"])
        );
        assert!(preview.flow_projection["current_step"]["dispatch_command"].is_null());
        assert!(preview.flow_projection["current_step"]["dispatch_command_kind"].is_null());
        assert_eq!(
            preview.flow_projection["current_step"]["proof_state"]["status"],
            "blocked_by_continuation_gate"
        );
        assert_eq!(
            preview.flow_projection["current_step"]["blocked_by_continuation_gate"],
            true
        );
    }

    #[test]
    fn agent_dispatch_next_command_uses_configured_runtime_selection_truth() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id: "task-ready",
                    title: "Ready task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 2,
                    parent_id: None,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: ".",
                })
                .await
                .expect("task should create");
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let _vida_root = EnvVarGuard::unset("VIDA_ROOT");
        let code = runtime.block_on(crate::run(cli(&[
            "agent",
            "dispatch-next",
            "--lanes",
            "1",
            "--state-dir",
            harness.path().to_str().expect("state dir should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::SUCCESS);
    }
}
