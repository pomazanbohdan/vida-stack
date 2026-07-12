use crate::state_store;

#[derive(Debug, Clone)]
pub(crate) struct DevTeamSequenceStep {
    pub(crate) role_label: String,
    pub(crate) runtime_role: String,
    pub(crate) task_class: String,
    pub(crate) packet_template_kind: Option<String>,
    pub(crate) closure_class: Option<String>,
    pub(crate) stage: Option<String>,
    pub(crate) completion_blocker: Option<String>,
    pub(crate) inclusion_rule: Option<String>,
    pub(crate) requires_task: bool,
    pub(crate) requires_user_approval: bool,
    pub(crate) approval_policy: serde_json::Value,
    pub(crate) lifecycle_hook_templates: serde_json::Value,
    pub(crate) resume_transitions: serde_json::Value,
    pub(crate) rework_transitions: serde_json::Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfiguredDevTeamTaskRoute {
    pub(crate) flow_id: Option<String>,
    pub(crate) role_label: String,
    pub(crate) runtime_role: String,
    pub(crate) task_class: String,
    pub(crate) dispatch_target: String,
    pub(crate) sequence: Vec<DevTeamSequenceStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DevTeamReceiptGate {
    pub(crate) selected_step_index: Option<usize>,
    pub(crate) status: &'static str,
    pub(crate) blocker_code: Option<String>,
    pub(crate) next_action: String,
    pub(crate) predecessor_role_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfiguredReceiptTransition {
    transition_kind: &'static str,
    event: String,
    target_role_label: String,
}

fn receipt_state_keys(receipt: &state_store::RunGraphDispatchReceipt) -> Vec<String> {
    let mut keys = Vec::new();
    for state in [&receipt.dispatch_status, &receipt.lane_status] {
        let state = state.trim().to_ascii_lowercase();
        if state.is_empty() {
            continue;
        }
        if !keys.contains(&state) {
            keys.push(state.clone());
        }
        if let Some(normalized) = state.strip_prefix("lane_") {
            let normalized = normalized.to_string();
            if !keys.contains(&normalized) {
                keys.push(normalized);
            }
        }
    }
    keys
}

fn configured_receipt_transition(
    step: &DevTeamSequenceStep,
    receipt: &state_store::RunGraphDispatchReceipt,
) -> Option<ConfiguredReceiptTransition> {
    let state_keys = receipt_state_keys(receipt);
    [
        ("rework", &step.rework_transitions),
        ("resume", &step.resume_transitions),
    ]
    .into_iter()
    .find_map(|(transition_kind, transitions)| {
        let transitions = transitions.as_object()?;
        state_keys.iter().find_map(|event| {
            transitions
                .get(event)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|target| !target.is_empty())
                .map(|target_role_label| ConfiguredReceiptTransition {
                    transition_kind,
                    event: event.clone(),
                    target_role_label: target_role_label.to_string(),
                })
        })
    })
}

fn receipt_failure_class(receipt: &state_store::RunGraphDispatchReceipt) -> &'static str {
    let states = receipt_state_keys(receipt).join(" ");
    if states.contains("exception") {
        "exception"
    } else if states.contains("failed")
        || states.contains("failure")
        || states.contains("error")
        || states.contains("rejected")
    {
        "failed"
    } else if states.contains("blocked") {
        "blocked"
    } else {
        "unknown_state"
    }
}

pub(crate) fn dev_team_receipt_gate(
    sequence: &[DevTeamSequenceStep],
    task_id: &str,
    bound_run_id: Option<&str>,
    receipt: Option<&state_store::RunGraphDispatchReceipt>,
) -> DevTeamReceiptGate {
    let task_step_indexes = sequence
        .iter()
        .enumerate()
        .filter_map(|(index, step)| step.requires_task.then_some(index))
        .collect::<Vec<_>>();
    let Some(first_step_index) = task_step_indexes.first().copied() else {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some("configured_dev_team_sequence_requires_task_step".to_string()),
            next_action:
                "Configure a task-bound dev-team step before materializing a dispatch packet."
                    .to_string(),
            predecessor_role_label: None,
        };
    };
    let Some(receipt) = receipt else {
        return DevTeamReceiptGate {
            selected_step_index: Some(first_step_index),
            status: "initial_step_ready",
            blocker_code: None,
            next_action: "Materialize only the initial dev-team step; later roles require a completed same-task predecessor receipt."
                .to_string(),
            predecessor_role_label: None,
        };
    };
    if receipt.run_id.trim().is_empty()
        || receipt.dispatch_target.trim().is_empty()
        || receipt.dispatch_status.trim().is_empty()
        || receipt.lane_status.trim().is_empty()
    {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some(format!(
                "dev_team_predecessor_receipt_malformed:task={task_id}:receipt_run={}",
                receipt.run_id,
            )),
            next_action: format!(
                "Repair malformed predecessor receipt state for TaskFlow task `{task_id}` before materializing another sequential dev-team packet."
            ),
            predecessor_role_label: None,
        };
    }
    let Some(bound_run_id) = bound_run_id
        .map(str::trim)
        .filter(|run_id| !run_id.is_empty())
    else {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some(format!(
                "dev_team_predecessor_receipt_run_binding_missing:task={task_id}"
            )),
            next_action: format!(
                "Resolve the authoritative latest run bound to TaskFlow task `{task_id}` before using predecessor receipt evidence."
            ),
            predecessor_role_label: None,
        };
    };
    if receipt.run_id != bound_run_id {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some(format!(
                "dev_team_predecessor_receipt_run_mismatch:task={task_id}:bound_run={bound_run_id}:receipt_run={}",
                receipt.run_id
            )),
            next_action: format!(
                "Read the predecessor receipt for authoritative run `{bound_run_id}` bound to TaskFlow task `{task_id}` before materializing another sequential dev-team packet."
            ),
            predecessor_role_label: None,
        };
    }
    let Some(receipt_step_index) = task_step_indexes
        .iter()
        .copied()
        .find(|index| sequence[*index].role_label == receipt.dispatch_target)
    else {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some(format!(
                "dev_team_predecessor_receipt_target_not_in_sequence:task={task_id}:target={}",
                receipt.dispatch_target
            )),
            next_action: format!(
                "Repair stale receipt target `{}` or resume the current TaskFlow task `{task_id}` before materializing another dev-team packet.",
                receipt.dispatch_target
            ),
            predecessor_role_label: None,
        };
    };
    if receipt.recorded_at.trim().is_empty() || receipt.lane_status == "lane_superseded" {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some(format!(
                "dev_team_predecessor_receipt_stale:task={task_id}:target={}",
                receipt.dispatch_target
            )),
            next_action: format!(
                "Repair or replace the stale `{}` receipt for TaskFlow task `{task_id}` before materializing a future dev-team role.",
                receipt.dispatch_target
            ),
            predecessor_role_label: Some(receipt.dispatch_target.clone()),
        };
    }
    if receipt.dispatch_status == "executed" && receipt.lane_status == "lane_completed" {
        let next_step_index = task_step_indexes
            .iter()
            .copied()
            .find(|index| *index > receipt_step_index);
        return DevTeamReceiptGate {
            selected_step_index: next_step_index,
            status: if next_step_index.is_some() {
                "predecessor_completed"
            } else {
                "sequence_completed"
            },
            blocker_code: next_step_index.is_none().then(|| {
                format!("dev_team_sequence_completed:task={task_id}")
            }),
            next_action: next_step_index.map_or_else(
                || format!("The configured dev-team sequence for TaskFlow task `{task_id}` is complete."),
                |_| format!(
                    "The `{}` receipt is completed for TaskFlow task `{task_id}`; materialize only its configured successor.",
                    receipt.dispatch_target
                ),
            ),
            predecessor_role_label: Some(receipt.dispatch_target.clone()),
        };
    }
    if let Some(transition) = configured_receipt_transition(&sequence[receipt_step_index], receipt)
    {
        let target_step_index = task_step_indexes
            .iter()
            .copied()
            .find(|index| sequence[*index].role_label == transition.target_role_label);
        let Some(target_step_index) = target_step_index else {
            return DevTeamReceiptGate {
                selected_step_index: None,
                status: "blocked",
                blocker_code: Some(format!(
                    "dev_team_predecessor_receipt_transition_target_not_in_sequence:task={task_id}:target={}",
                    transition.target_role_label
                )),
                next_action: format!(
                    "Repair configured {} transition `{}` so it targets a task-bound role in the sequential dev-team flow for `{task_id}`.",
                    transition.transition_kind, transition.event
                ),
                predecessor_role_label: Some(receipt.dispatch_target.clone()),
            };
        };
        return DevTeamReceiptGate {
            selected_step_index: Some(target_step_index),
            status: "configured_transition_authorized",
            blocker_code: None,
            next_action: format!(
                "Configured {} transition `{}` authorizes `{}` for TaskFlow task `{task_id}`; materialize only that sequential step.",
                transition.transition_kind, transition.event, transition.target_role_label
            ),
            predecessor_role_label: Some(receipt.dispatch_target.clone()),
        };
    }
    let failure_class = receipt_failure_class(receipt);
    DevTeamReceiptGate {
        selected_step_index: None,
        status: "blocked",
        blocker_code: Some(format!(
            "dev_team_predecessor_receipt_{failure_class}_fail_closed:task={task_id}:target={}",
            receipt.dispatch_target
        )),
        next_action: format!(
            "Receipt state dispatch_status=`{}` lane_status=`{}` does not authorize resume or rework for `{}` on TaskFlow task `{task_id}`; configure an explicit matching transition or repair the receipt before retrying.",
            receipt.dispatch_status, receipt.lane_status, receipt.dispatch_target
        ),
        predecessor_role_label: Some(receipt.dispatch_target.clone()),
    }
}

pub(crate) fn configured_dev_team_first_step_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Option<ConfiguredDevTeamTaskRoute> {
    let readiness = &activation_bundle["dev_team_readiness"];
    let flow_id = selected_dev_team_flow_for_task(readiness, task)
        .and_then(|flow| flow["flow_id"].as_str())
        .map(str::to_string);
    let sequence = dev_team_sequence_for_task(activation_bundle, task);
    let step = selected_dev_team_step_for_task(task, &sequence)?;
    Some(ConfiguredDevTeamTaskRoute {
        flow_id,
        dispatch_target: step.role_label.clone(),
        role_label: step.role_label,
        runtime_role: step.runtime_role,
        task_class: step.task_class,
        sequence,
    })
}

pub(crate) fn selected_dev_team_flow_for_task<'a>(
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

pub(crate) fn dev_team_flow_is_explicitly_sequential(
    readiness: &serde_json::Value,
    flow_id: Option<&str>,
) -> bool {
    flow_id
        .and_then(|flow_id| {
            readiness["flows"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
        })
        .and_then(|flow| flow["sequential"].as_bool())
        == Some(true)
}

pub(crate) fn selected_dev_team_flow_for_lookup_key<'a>(
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

pub(crate) fn selected_dev_team_flow_for_work_item<'a>(
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

pub(crate) fn dev_team_sequence_from_readiness(
    readiness: &serde_json::Value,
    work_item_type: Option<&str>,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence_from_readiness_with_default(readiness, work_item_type, true)
}

pub(crate) fn dev_team_sequence_from_readiness_lookup_key(
    readiness: &serde_json::Value,
    work_item_type: &str,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence_from_readiness_with_default(readiness, Some(work_item_type), false)
}

pub(crate) fn dev_team_sequence(activation_bundle: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
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

    execution_lane_sequence
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
                packet_template_kind: crate::json_trimmed_string_field(
                    route,
                    "packet_template_kind",
                ),
                closure_class: crate::json_trimmed_string_field(route, "closure_class"),
                stage: crate::json_trimmed_string_field(route, "stage"),
                completion_blocker: crate::json_trimmed_string_field(route, "completion_blocker"),
                inclusion_rule: crate::json_trimmed_string_field(route, "inclusion_rule"),
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

pub(crate) fn dev_team_sequence_for_flow_id(
    readiness: &serde_json::Value,
    flow_id: &str,
) -> Vec<DevTeamSequenceStep> {
    let flow_id = flow_id.trim();
    if flow_id.is_empty() {
        return Vec::new();
    }
    let Some(flow) = readiness["flows"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
    else {
        return Vec::new();
    };
    dev_team_sequence_from_explicit_flow(readiness, flow)
}

pub(crate) fn dev_team_sequence_for_work_item(
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

pub(crate) fn dev_team_sequence_for_task(
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

fn dev_team_sequence_from_explicit_flow(
    readiness: &serde_json::Value,
    flow: &serde_json::Value,
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
    configured_flow_steps(flow)
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
                packet_template_kind: policy_field_from_step_or_role(
                    step,
                    role,
                    "packet_template_kind",
                ),
                closure_class: policy_field_from_step_or_role(step, role, "closure_class"),
                stage: policy_field_from_step_or_role(step, role, "stage"),
                completion_blocker: policy_field_from_step_or_role(
                    step,
                    role,
                    "completion_blocker",
                ),
                inclusion_rule: policy_field_from_step_or_role(step, role, "inclusion_rule"),
                requires_task: role_id != "release_closure" && role_id != "terminal_closure",
                requires_user_approval: step["requires_user_approval"].as_bool().unwrap_or(false),
                approval_policy: step["approval_policy"].clone(),
                lifecycle_hook_templates: step["lifecycle_hook_templates"].clone(),
                resume_transitions: step["resume_transitions"].clone(),
                rework_transitions: step["rework_transitions"].clone(),
            })
        })
        .collect()
}

fn selected_dev_team_step_for_task(
    task: &state_store::TaskRecord,
    sequence: &[DevTeamSequenceStep],
) -> Option<DevTeamSequenceStep> {
    let canonical_issue_type = state_store::canonical_work_item_issue_type(&task.issue_type);
    if matches!(canonical_issue_type.as_str(), "task" | "subtask") {
        if !task.planner_metadata.owned_paths.is_empty() && task.labels.is_empty() {
            if let Some(step) = sequence.iter().find(|step| {
                step.requires_task
                    && !matches!(step.task_class.trim(), "specification" | "planning")
            }) {
                return Some(step.clone());
            }
        }
        let task_value = serde_json::to_value(task).unwrap_or(serde_json::Value::Null);
        let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
        if let Some(step) = sequence.iter().find(|step| {
            step.requires_task && step.task_class.trim() == inferred_task_class.as_str()
        }) {
            return Some(step.clone());
        }
    }
    sequence.iter().find(|step| step.requires_task).cloned()
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
        .map(configured_flow_steps)
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
                    packet_template_kind: policy_field_from_step_or_role(
                        step,
                        role,
                        "packet_template_kind",
                    ),
                    closure_class: policy_field_from_step_or_role(step, role, "closure_class"),
                    stage: policy_field_from_step_or_role(step, role, "stage"),
                    completion_blocker: policy_field_from_step_or_role(
                        step,
                        role,
                        "completion_blocker",
                    ),
                    inclusion_rule: policy_field_from_step_or_role(step, role, "inclusion_rule"),
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
                packet_template_kind: crate::json_trimmed_string_field(
                    role,
                    "packet_template_kind",
                ),
                closure_class: crate::json_trimmed_string_field(role, "closure_class"),
                stage: crate::json_trimmed_string_field(role, "stage"),
                completion_blocker: crate::json_trimmed_string_field(role, "completion_blocker"),
                inclusion_rule: crate::json_trimmed_string_field(role, "inclusion_rule"),
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
                packet_template_kind: None,
                closure_class: None,
                stage: None,
                completion_blocker: None,
                inclusion_rule: None,
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

fn configured_flow_steps(flow: &serde_json::Value) -> Vec<serde_json::Value> {
    let ordered_steps = flow["ordered_steps"]
        .as_array()
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    if !ordered_steps.is_empty() {
        return ordered_steps;
    }
    flow["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|step| {
            step.as_str()
                .map(|role_id| serde_json::json!({ "role_id": role_id }))
                .or_else(|| step.as_object().map(|_| step.clone()))
        })
        .collect()
}

fn policy_field_from_step_or_role(
    step: &serde_json::Value,
    role: &serde_json::Value,
    key: &str,
) -> Option<String> {
    crate::json_trimmed_string_field(step, key)
        .or_else(|| crate::json_trimmed_string_field(role, key))
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

pub(crate) fn task_flow_lookup_keys(task: &state_store::TaskRecord) -> Vec<String> {
    let mut keys = Vec::new();
    let task_value = serde_json::to_value(task).unwrap_or(serde_json::Value::Null);
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    let work_item_kind = state_store::task_work_item_kind(&task.issue_type);
    let explicit_kind_is_generic_task = work_item_kind.canonical_issue_type == "task";
    if !explicit_kind_is_generic_task {
        push_work_item_lookup_keys(&mut keys, &work_item_kind, &task.issue_type);
    }
    if inferred_task_class != "implementation" {
        push_unique_lookup_key(&mut keys, inferred_task_class);
    }
    if explicit_kind_is_generic_task {
        push_work_item_lookup_keys(&mut keys, &work_item_kind, &task.issue_type);
    }
    keys
}

fn push_work_item_lookup_keys(
    keys: &mut Vec<String>,
    work_item_kind: &state_store::TaskWorkItemKind,
    issue_type: &str,
) {
    push_unique_lookup_key(keys, &work_item_kind.canonical_issue_type);
    if let Some(provider_issue_type) = &work_item_kind.provider_issue_type {
        push_unique_lookup_key(keys, provider_issue_type);
    }
    push_unique_lookup_key(keys, issue_type);
    push_unique_lookup_key(keys, &work_item_kind.default_flow_binding);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_team_sequence_accepts_flow_steps_shorthand() {
        let readiness = serde_json::json!({
            "default_flow_id": "runtime_defect_remediation",
            "work_item_flow_bindings": {
                "runtime_defect": "runtime_defect_remediation"
            },
            "roles": [
                {"role_id": "specifier", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "coder", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "refactorer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
            ],
            "flows": [
                {
                    "flow_id": "runtime_defect_remediation",
                    "enabled": true,
                    "work_item_bindings": ["runtime_defect"],
                    "steps": [
                        "specifier",
                        {"role_id": "coder", "task_class": "implementation"},
                        {"role_id": "refactorer", "task_class": "implementation"},
                        {"role_id": "architect", "task_class": "architecture"}
                    ]
                }
            ]
        });

        let sequence = dev_team_sequence_from_readiness(&readiness, Some("runtime_defect"));

        assert_eq!(sequence.len(), 4);
        assert_eq!(sequence[0].role_label, "specifier");
        assert_eq!(sequence[0].runtime_role, "business_analyst");
        assert_eq!(sequence[0].task_class, "specification");
        assert_eq!(sequence[1].role_label, "coder");
        assert_eq!(sequence[1].runtime_role, "worker");
        assert_eq!(sequence[1].task_class, "implementation");
        assert_eq!(sequence[2].role_label, "refactorer");
        assert_eq!(sequence[2].runtime_role, "worker");
        assert_eq!(sequence[2].task_class, "implementation");
        assert_eq!(sequence[3].role_label, "architect");
        assert_eq!(sequence[3].runtime_role, "solution_architect");
        assert_eq!(sequence[3].task_class, "architecture");
    }

    #[test]
    fn dev_team_sequence_uses_shared_trimmed_json_string_fields() {
        let readiness = serde_json::json!({
            "default_flow_id": "runtime_defect_remediation",
            "roles": [
                {
                    "role_id": "coder",
                    "runtime_role": "worker",
                    "task_classes": ["implementation"],
                    "packet_template_kind": " delivery_task_packet ",
                    "closure_class": "   "
                }
            ],
            "flows": [
                {
                    "flow_id": "runtime_defect_remediation",
                    "enabled": true,
                    "work_item_bindings": ["runtime_defect"],
                    "steps": [
                        {
                            "role_id": "coder",
                            "task_class": "implementation",
                            "stage": " implement ",
                            "completion_blocker": "   ",
                            "inclusion_rule": 7
                        }
                    ]
                }
            ]
        });

        let sequence = dev_team_sequence_from_readiness(&readiness, Some("runtime_defect"));

        assert_eq!(sequence.len(), 1);
        assert_eq!(
            sequence[0].packet_template_kind.as_deref(),
            Some("delivery_task_packet")
        );
        assert_eq!(sequence[0].stage.as_deref(), Some("implement"));
        assert_eq!(sequence[0].closure_class, None);
        assert_eq!(sequence[0].completion_blocker, None);
        assert_eq!(sequence[0].inclusion_rule, None);
    }

    fn receipt(
        run_id: &str,
        dispatch_target: &str,
        dispatch_status: &str,
        lane_status: &str,
    ) -> state_store::RunGraphDispatchReceipt {
        state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: dispatch_target.to_string(),
            dispatch_status: dispatch_status.to_string(),
            lane_status: lane_status.to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: None,
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
            selected_backend: None,
            recorded_at: "2026-07-10T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn dev_team_receipt_gate_requires_bound_run_completion_and_explicit_transitions() {
        let mut sequence = ["developer", "coach", "tester", "reviewer"]
            .into_iter()
            .map(|role_label| DevTeamSequenceStep {
                role_label: role_label.to_string(),
                runtime_role: "worker".to_string(),
                task_class: "implementation".to_string(),
                packet_template_kind: None,
                closure_class: None,
                stage: None,
                completion_blocker: None,
                inclusion_rule: None,
                requires_task: true,
                requires_user_approval: false,
                approval_policy: serde_json::Value::Null,
                lifecycle_hook_templates: serde_json::Value::Null,
                resume_transitions: serde_json::Value::Null,
                rework_transitions: serde_json::Value::Null,
            })
            .collect::<Vec<_>>();
        let completed = |role| receipt("run-a", role, "executed", "lane_completed");

        for (receipt, expected_index) in [
            (None, 0),
            (Some(completed("developer")), 1),
            (Some(completed("coach")), 2),
            (Some(completed("tester")), 3),
        ] {
            let gate = dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), receipt.as_ref());
            assert_eq!(gate.selected_step_index, Some(expected_index), "{gate:?}");
            assert!(gate.blocker_code.is_none(), "{gate:?}");
        }

        let blocked = receipt("run-a", "developer", "blocked", "lane_blocked");
        let blocked_gate =
            dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&blocked));
        assert_eq!(blocked_gate.status, "blocked");
        assert!(blocked_gate.blocker_code.as_deref().is_some_and(|code| {
            code.starts_with("dev_team_predecessor_receipt_blocked_fail_closed:")
        }));

        sequence[0].rework_transitions = serde_json::json!({"blocked": "developer"});
        let authorized_gate =
            dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&blocked));
        assert_eq!(authorized_gate.status, "configured_transition_authorized");
        assert_eq!(authorized_gate.selected_step_index, Some(0));
        assert!(authorized_gate.blocker_code.is_none());

        let wrong_run = receipt("run-b", "developer", "executed", "lane_completed");
        let wrong_run_gate =
            dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&wrong_run));
        assert!(
            wrong_run_gate
                .blocker_code
                .as_deref()
                .is_some_and(|code| code.starts_with("dev_team_predecessor_receipt_run_mismatch:"))
        );

        let stale = receipt("run-a", "developer", "executed", "lane_superseded");
        let stale_gate = dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&stale));
        assert!(
            stale_gate
                .blocker_code
                .as_deref()
                .is_some_and(|code| code.starts_with("dev_team_predecessor_receipt_stale:"))
        );

        for (mut invalid, blocker_prefix) in [
            (
                receipt("run-a", "coach", "failed", "lane_failed"),
                "dev_team_predecessor_receipt_failed_fail_closed:",
            ),
            (
                receipt("run-a", "coach", "exception", "lane_exception_takeover"),
                "dev_team_predecessor_receipt_exception_fail_closed:",
            ),
            (
                receipt("run-a", "coach", "pending", "lane_active"),
                "dev_team_predecessor_receipt_unknown_state_fail_closed:",
            ),
            (
                receipt("run-a", "coach", "", "lane_active"),
                "dev_team_predecessor_receipt_malformed:",
            ),
        ] {
            if invalid.dispatch_status == "failed" {
                invalid.blocker_code = Some("execution_failed".to_string());
            }
            let gate = dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&invalid));
            assert_eq!(gate.status, "blocked", "{gate:?}");
            assert!(
                gate.blocker_code
                    .as_deref()
                    .is_some_and(|code| code.starts_with(blocker_prefix)),
                "{gate:?}"
            );
        }
    }
}
