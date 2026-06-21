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
                packet_template_kind: json_string_field(route, "packet_template_kind"),
                closure_class: json_string_field(route, "closure_class"),
                stage: json_string_field(route, "stage"),
                completion_blocker: json_string_field(route, "completion_blocker"),
                inclusion_rule: json_string_field(route, "inclusion_rule"),
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
        .flat_map(|role| {
            ["role_id", "canonical_role_id"]
                .into_iter()
                .filter_map(move |field| role[field].as_str().map(|id| (id.to_string(), role)))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    flow["ordered_steps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|step| {
            let role_id = step["role_id"].as_str()?;
            let role = roles.get(role_id).copied();
            let runtime_role = step["runtime_role"]
                .as_str()
                .or_else(|| role.and_then(|role| role["runtime_role"].as_str()))
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)?;
            let task_class = step["task_class"]
                .as_str()
                .or_else(|| {
                    role.and_then(|role| {
                        role["task_classes"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .find(|value| !value.trim().is_empty())
                    })
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
    if task.issue_type.trim() == "task" {
        if !task.planner_metadata.owned_paths.is_empty() && task.labels.is_empty() {
            if let Some(step) = sequence.iter().find(|step| {
                step.requires_task
                    && !matches!(step.task_class.trim(), "specification" | "planning")
            }) {
                return Some(step.clone());
            }
        }
        if !task.labels.is_empty() {
            let task_value = serde_json::to_value(task).unwrap_or(serde_json::Value::Null);
            let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
            if let Some(step) = sequence.iter().find(|step| {
                step.requires_task && step.task_class.trim() == inferred_task_class.as_str()
            }) {
                return Some(step.clone());
            }
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
        .flat_map(|role| {
            ["role_id", "canonical_role_id"]
                .into_iter()
                .filter_map(move |field| role[field].as_str().map(|id| (id.to_string(), role)))
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
                let role = roles.get(role_id).copied();
                let runtime_role = step["runtime_role"]
                    .as_str()
                    .or_else(|| role.and_then(|role| role["runtime_role"].as_str()))
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)?;
                let task_class = step["task_class"]
                    .as_str()
                    .or_else(|| {
                        role.and_then(|role| {
                            role["task_classes"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(serde_json::Value::as_str)
                                .find(|value| !value.trim().is_empty())
                        })
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
                packet_template_kind: json_string_field(role, "packet_template_kind"),
                closure_class: json_string_field(role, "closure_class"),
                stage: json_string_field(role, "stage"),
                completion_blocker: json_string_field(role, "completion_blocker"),
                inclusion_rule: json_string_field(role, "inclusion_rule"),
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

fn policy_field_from_step_or_role(
    step: &serde_json::Value,
    role: Option<&serde_json::Value>,
    key: &str,
) -> Option<String> {
    json_string_field(step, key).or_else(|| role.and_then(|role| json_string_field(role, key)))
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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
