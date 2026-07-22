use crate::state_store;
use crate::team_flow_authority_adapter::{
    TeamFlowAuthorityAvailability, TeamFlowAuthorityAvailabilityStatus,
};

#[derive(Debug, Clone)]
pub(crate) struct DevTeamSequenceStep {
    /// Stable TeamFlow node identity; never derive this from a dispatch alias.
    pub(crate) node_id: String,
    /// Configured external dispatch projection for receipts and packets.
    pub(crate) dispatch_target: String,
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
    /// Stable TeamFlow node identity selected for this task's bound flow.
    pub(crate) node_id: String,
    pub(crate) role_label: String,
    pub(crate) runtime_role: String,
    pub(crate) task_class: String,
    pub(crate) dispatch_target: String,
    pub(crate) sequence: Vec<DevTeamSequenceStep>,
}

#[derive(Debug, Clone)]
pub(crate) struct DevTeamSequenceResolution {
    pub(crate) authority: TeamFlowAuthorityAvailability,
    pub(crate) sequence: Vec<DevTeamSequenceStep>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfiguredDevTeamTaskRouteResolution {
    pub(crate) authority: TeamFlowAuthorityAvailability,
    pub(crate) route: Option<ConfiguredDevTeamTaskRoute>,
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
            if !keys.contains(&normalized.to_string()) {
                keys.push(normalized.to_string());
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
            next_action: "The configured TeamFlow snapshot has no included required task step."
                .to_string(),
            predecessor_role_label: None,
        };
    };
    let Some(receipt) = receipt else {
        return DevTeamReceiptGate {
            selected_step_index: Some(first_step_index),
            status: "initial_step_ready",
            blocker_code: None,
            next_action: "Materialize only the initial configured TeamFlow step.".to_string(),
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
                receipt.run_id
            )),
            next_action: format!("Repair malformed predecessor receipt for `{task_id}`."),
            predecessor_role_label: None,
        };
    }
    let Some(bound_run_id) = bound_run_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return DevTeamReceiptGate {
            selected_step_index: None,
            status: "blocked",
            blocker_code: Some(format!(
                "dev_team_predecessor_receipt_run_binding_missing:task={task_id}"
            )),
            next_action: format!("Resolve the authoritative run bound to `{task_id}`."),
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
            next_action: format!("Read the receipt for bound run `{bound_run_id}`."),
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
            next_action: format!("Repair receipt target `{}`.", receipt.dispatch_target),
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
            next_action: format!("Repair stale receipt `{}`.", receipt.dispatch_target),
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
            blocker_code: next_step_index
                .is_none()
                .then(|| format!("dev_team_sequence_completed:task={task_id}")),
            next_action: next_step_index.map_or_else(
                || format!("The configured TeamFlow sequence for `{task_id}` is complete."),
                |_| {
                    format!(
                        "Materialize only the configured successor of `{}`.",
                        receipt.dispatch_target
                    )
                },
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
                    "Repair configured {} transition `{}`.",
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
                "Configured {} transition authorizes `{}`.",
                transition.transition_kind, transition.target_role_label
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
            "Receipt state does not authorize a TeamFlow transition for `{}`.",
            receipt.dispatch_target
        ),
        predecessor_role_label: Some(receipt.dispatch_target.clone()),
    }
}

fn team_flow_bundle<'a>(activation_bundle: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
    activation_bundle
        .get("team_flow_authority")
        .map(|_| activation_bundle)
}

fn sequence_from_projection(
    projection: &crate::team_flow_authority_adapter::TeamFlowAuthorityProjection,
) -> Vec<DevTeamSequenceStep> {
    projection
        .ordered_nodes()
        .filter(|node| node.node.included)
        .map(|node| DevTeamSequenceStep {
            node_id: node.node.node_id.clone(),
            dispatch_target: node.dispatch_target.clone(),
            role_label: node.node.node_id.clone(),
            runtime_role: node.node.runtime_role.clone(),
            task_class: node.node.task_class.clone(),
            packet_template_kind: Some(node.packet_template_kind.clone()),
            closure_class: Some(node.closure_class.clone()),
            stage: Some(node.stage.clone()),
            completion_blocker: Some(node.completion_blocker.clone()),
            inclusion_rule: Some(node.node.inclusion_rule.clone()),
            requires_task: node.node.included && node.node.required,
            requires_user_approval: node.node.requires_user_approval,
            approval_policy: node.approval_policy.clone(),
            lifecycle_hook_templates: node.lifecycle_hook_templates.clone(),
            resume_transitions: node.resume_transitions.clone(),
            rework_transitions: node.rework_transitions.clone(),
        })
        .collect()
}

pub(crate) fn dev_team_sequence_resolution(
    activation_bundle: &serde_json::Value,
    flow_ref: Option<&str>,
) -> DevTeamSequenceResolution {
    let authority = team_flow_bundle(activation_bundle)
        .map(|bundle| {
            crate::team_flow_state_machine::team_flow_execution_authority_availability(
                bundle, flow_ref, None,
            )
        })
        .unwrap_or_else(|| TeamFlowAuthorityAvailability {
            status: TeamFlowAuthorityAvailabilityStatus::Unavailable,
            blocker: Some(
                crate::team_flow_authority_adapter::TEAM_FLOW_AUTHORITY_UNAVAILABLE_BLOCKER
                    .to_string(),
            ),
            projection: None,
        });
    let sequence = authority
        .projection
        .as_ref()
        .filter(|_| authority.is_ready())
        .map(sequence_from_projection)
        .unwrap_or_default();
    DevTeamSequenceResolution {
        authority,
        sequence,
    }
}

pub(crate) fn configured_dev_team_first_step_for_task_status(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> ConfiguredDevTeamTaskRouteResolution {
    // Resolve the task's configured flow before selecting its task-class node.
    // The default flow can contain a same-named role (for example `coder`) that
    // is not present in the bound flow; carrying the exact node id prevents a
    // later strict resolver from interpreting that stale role as an alias.
    let bound_flow_id = match selected_dev_team_flow_id_for_task(activation_bundle, task) {
        Ok(flow_id) => flow_id,
        Err(blocker) => {
            let mut authority = dev_team_sequence_resolution(activation_bundle, None).authority;
            authority.status = TeamFlowAuthorityAvailabilityStatus::Blocked;
            authority.blocker = Some(blocker);
            authority.projection = None;
            return ConfiguredDevTeamTaskRouteResolution {
                authority,
                route: None,
            };
        }
    };
    let mut resolution = dev_team_sequence_resolution(activation_bundle, Some(&bound_flow_id));
    if !resolution.authority.is_ready() {
        return ConfiguredDevTeamTaskRouteResolution {
            authority: resolution.authority,
            route: None,
        };
    }
    let task_value = match serde_json::to_value(task) {
        Ok(value) => value,
        Err(error) => {
            resolution.authority.status = TeamFlowAuthorityAvailabilityStatus::Blocked;
            resolution.authority.blocker =
                Some(format!("team_flow_execution_task_payload_invalid:{error}"));
            resolution.authority.projection = None;
            return ConfiguredDevTeamTaskRouteResolution {
                authority: resolution.authority,
                route: None,
            };
        }
    };
    let requested_class = crate::infer_task_class_from_task_payload(&task_value);
    let Some(step) = resolution
        .sequence
        .iter()
        .find(|step| step.requires_task && step.task_class == requested_class)
        .cloned()
    else {
        resolution.authority.status = TeamFlowAuthorityAvailabilityStatus::Blocked;
        resolution.authority.blocker = Some(format!(
            "team_flow_execution_task_class_not_configured:{requested_class}"
        ));
        resolution.authority.projection = None;
        return ConfiguredDevTeamTaskRouteResolution {
            authority: resolution.authority,
            route: None,
        };
    };
    let flow_id = resolution
        .authority
        .projection
        .as_ref()
        .map(|projection| projection.snapshot.flow_ref.clone());
    let route = ConfiguredDevTeamTaskRoute {
        flow_id,
        node_id: step.node_id.clone(),
        role_label: step.role_label.clone(),
        runtime_role: step.runtime_role.clone(),
        task_class: step.task_class.clone(),
        dispatch_target: step.dispatch_target.clone(),
        sequence: resolution.sequence,
    };
    ConfiguredDevTeamTaskRouteResolution {
        authority: resolution.authority,
        route: Some(route),
    }
}

pub(crate) fn configured_dev_team_first_step_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Option<ConfiguredDevTeamTaskRoute> {
    configured_dev_team_first_step_for_task_status(activation_bundle, task).route
}

fn route_lookup_blocker(
    namespace: &str,
    requested: &str,
    blocker: &crate::team_flow_authority_adapter::TeamFlowResolutionBlocker,
) -> String {
    let kind = blocker
        .code
        .strip_prefix("team_flow_node_resolution_")
        .unwrap_or(blocker.code.as_str());
    let candidates = if blocker.candidates.is_empty() {
        "<none>".to_string()
    } else {
        blocker.candidates.join(",")
    };
    format!("team_flow_{namespace}_{kind}:requested={requested}:candidates={candidates}")
}

pub(crate) fn configured_task_class_for_dispatch_target(
    activation_bundle: &serde_json::Value,
    flow_ref: Option<&str>,
    dispatch_target: &str,
) -> Result<String, String> {
    let dispatch_target = dispatch_target.trim();
    if dispatch_target.is_empty() {
        return Err("team_flow_dispatch_target_missing".to_string());
    }
    let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
        activation_bundle,
        flow_ref,
        None,
    )
    .map_err(|blocker| {
        format!(
            "team_flow_dispatch_target_authority_blocked:{}",
            blocker.code
        )
    })?;
    let node = authority
        .resolve_target(None, dispatch_target)
        .map_err(|blocker| route_lookup_blocker("dispatch_target", dispatch_target, &blocker))?;
    let task_class = node.task_class.trim();
    if task_class.is_empty() {
        return Err(format!(
            "team_flow_dispatch_target_task_class_missing:requested={dispatch_target}"
        ));
    }
    Ok(task_class.to_string())
}

pub(crate) fn configured_task_class_for_runtime_role(
    activation_bundle: &serde_json::Value,
    flow_ref: Option<&str>,
    runtime_role: &str,
) -> Result<String, String> {
    let runtime_role = runtime_role.trim();
    if runtime_role.is_empty() {
        return Err("team_flow_runtime_role_missing".to_string());
    }
    let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
        activation_bundle,
        flow_ref,
        None,
    )
    .map_err(|blocker| format!("team_flow_runtime_role_authority_blocked:{}", blocker.code))?;
    let node = authority
        .resolve_runtime_role(None, runtime_role)
        .map_err(|blocker| route_lookup_blocker("runtime_role", runtime_role, &blocker))?;
    let task_class = node.task_class.trim();
    if task_class.is_empty() {
        return Err(format!(
            "team_flow_runtime_role_task_class_missing:requested={runtime_role}"
        ));
    }
    Ok(task_class.to_string())
}

pub(crate) fn dev_team_sequence(activation_bundle: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence_resolution(activation_bundle, None).sequence
}

pub(crate) fn dev_team_sequence_for_flow_id(
    activation_bundle: &serde_json::Value,
    flow_id: &str,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence_resolution(activation_bundle, Some(flow_id)).sequence
}

pub(crate) fn dev_team_sequence_for_work_item(
    activation_bundle: &serde_json::Value,
    work_item_type: &str,
) -> Vec<DevTeamSequenceStep> {
    if !state_store::task_work_item_kind(work_item_type).flow_bindable {
        return Vec::new();
    }
    let authority = activation_bundle
        .get("team_flow_authority")
        .unwrap_or(activation_bundle);
    let Ok(flow_id) =
        resolve_work_item_flow_binding(authority, work_item_flow_lookup_keys(work_item_type))
    else {
        return Vec::new();
    };
    if validate_work_item_flow_binding(authority, &flow_id).is_err() {
        return Vec::new();
    }
    dev_team_sequence_for_flow_id(activation_bundle, &flow_id)
}

pub(crate) fn dev_team_sequence_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Vec<DevTeamSequenceStep> {
    selected_dev_team_flow_for_task(activation_bundle, task)
        .and_then(|flow| flow["flow_id"].as_str())
        .map(|flow_id| dev_team_sequence_for_flow_id(activation_bundle, flow_id))
        .unwrap_or_default()
}

pub(crate) fn selected_dev_team_flow_id_for_task(
    readiness: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Result<String, String> {
    let kind = state_store::task_work_item_kind(&task.issue_type);
    if !kind.flow_bindable {
        return Err("team_flow_authority_task_kind_not_flow_bindable".to_string());
    }
    let authority = readiness.get("team_flow_authority").unwrap_or(readiness);
    let binding = resolve_work_item_flow_binding(authority, task_flow_lookup_keys(task))?;
    validate_work_item_flow_binding(authority, &binding)?;
    selected_dev_team_flow_for_task(readiness, task)
        .and_then(|flow| flow["flow_id"].as_str())
        .map(str::to_string)
        .ok_or_else(|| "team_flow_authority_task_flow_selection_blocked".to_string())
}

/// Compatibility surface for the dispatch planner. Flow selection is owned by
/// the compiled TeamFlow authority; readiness JSON is deliberately not a
/// second parser or a fallback source.
pub(crate) fn selected_dev_team_flow_for_task<'a>(
    readiness: &'a serde_json::Value,
    task: &state_store::TaskRecord,
) -> Option<&'a serde_json::Value> {
    let authority = readiness.get("team_flow_authority")?;
    let flow_rows = authority["resolved_all_flow_payload"]["flows"].as_array()?;
    let flow_ref = resolve_work_item_flow_binding(authority, task_flow_lookup_keys(task)).ok()?;
    let selected_flow = flow_rows.iter().find(|flow| {
        flow["flow_id"].as_str() == Some(flow_ref.as_str())
            && flow["flow_policy"]["enabled"].as_bool() == Some(true)
    })?;
    let bundle = serde_json::json!({"team_flow_authority": authority});
    let projection = crate::team_flow_authority_adapter::compile_team_flow_authority(
        &bundle,
        Some(flow_ref.as_str()),
        None,
    )
    .ok()?;
    (projection.snapshot.flow_ref == flow_ref).then_some(selected_flow)
}

pub(crate) fn dev_team_flow_is_explicitly_sequential(
    readiness: &serde_json::Value,
    flow_id: Option<&str>,
) -> bool {
    let Some(flow_id) = flow_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    persisted_flow_rows(readiness)
        .into_iter()
        .find(|flow| {
            flow["flow_id"].as_str() == Some(flow_id)
                && flow["flow_policy"]["enabled"].as_bool() == Some(true)
        })
        .and_then(|flow| flow["flow_policy"]["sequential"].as_bool())
        .unwrap_or(false)
}

fn persisted_flow_rows(bundle_or_readiness: &serde_json::Value) -> Vec<&serde_json::Value> {
    let authority = bundle_or_readiness
        .get("team_flow_authority")
        .unwrap_or(bundle_or_readiness);
    authority["resolved_all_flow_payload"]["flows"]
        .as_array()
        .map(|flows| flows.iter().collect())
        .unwrap_or_default()
}

fn persisted_work_item_flow_bindings(
    bundle_or_readiness: &serde_json::Value,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    let authority = bundle_or_readiness
        .get("team_flow_authority")
        .unwrap_or(bundle_or_readiness);
    authority["resolved_all_flow_payload"]["work_item_flow_bindings"].as_object()
}

fn work_item_flow_lookup_keys(work_item_type: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let kind = state_store::task_work_item_kind(work_item_type);
    for value in [
        kind.canonical_issue_type.as_str(),
        kind.provider_issue_type.as_deref().unwrap_or_default(),
        work_item_type,
    ] {
        let value = value.trim().to_ascii_lowercase();
        if !value.is_empty() && !keys.contains(&value) {
            keys.push(value);
        }
    }
    keys
}

pub(crate) fn task_flow_lookup_keys(task: &state_store::TaskRecord) -> Vec<String> {
    let mut keys = work_item_flow_lookup_keys(&task.issue_type);
    if let Some(mapping) = task.provider_mapping.as_ref() {
        if let Some(provider_issue_type) = mapping.provider_issue_type.as_deref() {
            let provider_issue_type = provider_issue_type.trim().to_ascii_lowercase();
            if !provider_issue_type.is_empty() && !keys.contains(&provider_issue_type) {
                keys.push(provider_issue_type);
            }
        }
        let provider_canonical = state_store::canonical_work_item_issue_type(
            mapping.provider_issue_type.as_deref().unwrap_or_default(),
        );
        if !provider_canonical.is_empty() && !keys.contains(&provider_canonical) {
            keys.push(provider_canonical);
        }
    }
    keys
}

fn resolve_work_item_flow_binding(
    authority: &serde_json::Value,
    lookup_keys: Vec<String>,
) -> Result<String, String> {
    let bindings = persisted_work_item_flow_bindings(authority)
        .ok_or_else(|| "team_flow_authority_work_item_flow_bindings_missing".to_string())?;
    let mut matches = Vec::new();
    for key in lookup_keys {
        let Some(value) = bindings.get(&key) else {
            continue;
        };
        let flow_id = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("team_flow_authority_work_item_flow_binding_invalid:{key}"))?;
        matches.push((key, flow_id.to_string()));
    }
    if matches.is_empty() {
        return Err("team_flow_authority_work_item_flow_binding_missing".to_string());
    }
    let mut flow_ids = matches
        .iter()
        .map(|(_, flow_id)| flow_id.as_str())
        .collect::<Vec<_>>();
    flow_ids.sort_unstable();
    flow_ids.dedup();
    if flow_ids.len() > 1 {
        let keys = matches
            .iter()
            .map(|(key, flow_id)| format!("{key}={flow_id}"))
            .collect::<Vec<_>>()
            .join(",");
        return Err(format!(
            "team_flow_authority_work_item_flow_binding_ambiguous:{keys}"
        ));
    }
    Ok(flow_ids[0].to_string())
}

fn validate_work_item_flow_binding(
    authority: &serde_json::Value,
    flow_id: &str,
) -> Result<(), String> {
    let flow = persisted_flow_rows(authority)
        .into_iter()
        .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
        .ok_or_else(|| "team_flow_authority_unknown_flow".to_string())?;
    if flow["flow_policy"]["enabled"].as_bool() != Some(true) {
        return Err("team_flow_authority_flow_policy_disabled".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_bundle() -> serde_json::Value {
        crate::team_flow_authority_projection::test_support::canonical_compiled_bundle()
    }

    fn persisted_enabled_binding(authority: &serde_json::Value) -> (String, String) {
        let flows = authority["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("authority fixture should persist flows");
        authority["resolved_all_flow_payload"]["work_item_flow_bindings"]
            .as_object()
            .expect("authority fixture should persist explicit work-item bindings")
            .iter()
            .find_map(|(binding, flow_id)| {
                let flow_id = flow_id.as_str()?;
                flows
                    .iter()
                    .any(|flow| {
                        flow["flow_id"].as_str() == Some(flow_id)
                            && flow["flow_policy"]["enabled"].as_bool() == Some(true)
                    })
                    .then(|| (binding.clone(), flow_id.to_string()))
            })
            .expect("authority fixture should bind an enabled flow")
    }

    fn receipt(
        run_id: &str,
        target: &str,
        dispatch_status: &str,
        lane_status: &str,
    ) -> state_store::RunGraphDispatchReceipt {
        state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: target.to_string(),
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

    fn step(name: &str, _next: Option<&str>) -> DevTeamSequenceStep {
        DevTeamSequenceStep {
            node_id: name.to_string(),
            dispatch_target: name.to_string(),
            role_label: name.to_string(),
            runtime_role: "worker".to_string(),
            task_class: "implementation".to_string(),
            packet_template_kind: None,
            closure_class: None,
            stage: None,
            completion_blocker: None,
            inclusion_rule: Some("always".to_string()),
            requires_task: true,
            requires_user_approval: false,
            approval_policy: serde_json::Value::Null,
            lifecycle_hook_templates: serde_json::Value::Null,
            resume_transitions: serde_json::Value::Null,
            rework_transitions: serde_json::Value::Null,
        }
    }

    fn task_with_issue_type(issue_type: &str) -> state_store::TaskRecord {
        state_store::TaskRecord {
            id: "task".to_string(),
            display_id: None,
            title: "task".to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 1,
            issue_type: issue_type.to_string(),
            created_at: "2026-07-20T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-07-20T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: "test".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: Default::default(),
            planner_metadata: Default::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn task_with_provider_alias(
        issue_type: &str,
        provider: &str,
        provider_issue_type: &str,
    ) -> state_store::TaskRecord {
        let mut task = task_with_issue_type(issue_type);
        task.provider_mapping = Some(
            serde_json::from_value(serde_json::json!({
                "schema_version": 1,
                "provider": provider,
                "external_id": "provider-work-item-1",
                "provider_issue_type": provider_issue_type
            }))
            .expect("provider mapping fixture should deserialize"),
        );
        task
    }

    #[test]
    fn receipt_gate_follows_explicit_next_node() {
        let sequence = vec![step("first", Some("last")), step("last", None)];
        let completed = receipt("run-a", "first", "executed", "lane_completed");
        let gate = dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&completed));
        assert_eq!(gate.selected_step_index, Some(1));
    }

    #[test]
    fn receipt_gate_requires_configured_rework_transition() {
        let mut sequence = vec![step("first", Some("last")), step("last", None)];
        let blocked = receipt("run-a", "first", "blocked", "lane_blocked");
        assert_eq!(
            dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&blocked)).status,
            "blocked"
        );
        sequence[0].rework_transitions = serde_json::json!({"blocked": "first"});
        let gate = dev_team_receipt_gate(&sequence, "task-a", Some("run-a"), Some(&blocked));
        assert_eq!(gate.status, "configured_transition_authorized");
        assert_eq!(gate.selected_step_index, Some(0));
    }

    #[test]
    fn configured_task_class_lookups_follow_ordered_authority_nodes() {
        let bundle = authority_bundle();
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &bundle, None, None,
        )
        .expect("canonical authority must compile");
        let node = authority
            .ordered_nodes()
            .find(|node| node.node.included)
            .expect("canonical authority must expose an included node");
        assert_eq!(
            configured_task_class_for_dispatch_target(&bundle, None, &node.node.node_id)
                .expect("configured target must resolve"),
            node.node.task_class
        );
        assert_eq!(
            configured_task_class_for_runtime_role(&bundle, None, &node.node.runtime_role)
                .expect("configured runtime role must resolve"),
            node.node.task_class
        );
    }

    #[test]
    fn configured_task_class_lookups_fail_closed_for_missing_or_unknown_identity() {
        let bundle = authority_bundle();
        assert_eq!(
            configured_task_class_for_dispatch_target(&bundle, None, ""),
            Err("team_flow_dispatch_target_missing".to_string())
        );
        let unknown_target = "configured-target-that-does-not-exist";
        let target_error = configured_task_class_for_dispatch_target(&bundle, None, unknown_target)
            .expect_err("unknown configured target must fail closed");
        assert!(target_error.starts_with("team_flow_dispatch_target_missing:"));
        let role_error = configured_task_class_for_runtime_role(&bundle, None, "")
            .expect_err("missing configured runtime role must fail closed");
        assert_eq!(role_error, "team_flow_runtime_role_missing");
    }

    #[test]
    fn configured_runtime_role_lookup_surfaces_dynamic_ambiguity_when_present() {
        let bundle = authority_bundle();
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &bundle, None, None,
        )
        .expect("canonical authority must compile");
        let mut roles = std::collections::BTreeMap::<String, Vec<String>>::new();
        for node in authority.ordered_nodes().filter(|node| node.node.included) {
            roles
                .entry(node.node.runtime_role.clone())
                .or_default()
                .push(node.node.node_id.clone());
        }
        let Some((runtime_role, candidates)) = roles.into_iter().find(|(_, nodes)| nodes.len() > 1)
        else {
            return;
        };
        let error = configured_task_class_for_runtime_role(&bundle, None, &runtime_role)
            .expect_err("duplicate configured runtime role must fail closed");
        assert!(error.starts_with("team_flow_runtime_role_ambiguous:"));
        assert!(candidates.iter().all(|candidate| error.contains(candidate)));
    }

    #[test]
    fn sequence_uses_snapshot_flow_and_unknown_flow_fails_closed() {
        let bundle = authority_bundle();
        let default_flow = bundle["team_flow_authority"]["selected_config"]["authority_selection"]
            ["default_flow_id"]
            .as_str()
            .expect("authority fixture should select a default flow");
        let expected_lanes = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("authority fixture should persist flows")
            .iter()
            .find(|flow| flow["flow_id"].as_str() == Some(default_flow))
            .and_then(|flow| flow["lanes"].as_array())
            .expect("default flow should persist its lanes");
        let expected_sequence = expected_lanes
            .iter()
            .filter(|lane| lane["included"].as_bool() == Some(true))
            .map(|lane| {
                lane["node_id"]
                    .as_str()
                    .expect("persisted lane should define a node id")
            })
            .collect::<Vec<_>>();
        let actual_sequence = dev_team_sequence_for_flow_id(&bundle, default_flow);
        assert_eq!(
            actual_sequence
                .iter()
                .map(|step| step.role_label.as_str())
                .collect::<Vec<_>>(),
            expected_sequence
        );
        let unknown_flow = format!("{default_flow}_unknown");
        assert!(dev_team_sequence_for_flow_id(&bundle, &unknown_flow).is_empty());
    }

    #[test]
    fn selected_flow_uses_task_binding_and_fails_closed_without_binding() {
        let bundle = authority_bundle();
        let authority = bundle["team_flow_authority"].clone();
        let (work_item_type, bound_flow_id) = persisted_enabled_binding(&authority);
        let readiness = serde_json::json!({
            "team_flow_authority": authority,
            "work_item_flow_bindings": {(work_item_type.clone()): "stale-flow"},
            "flows": [
                {"flow_id": "stale-flow"}
            ]
        });
        let task = task_with_issue_type(&work_item_type);
        let selected = selected_dev_team_flow_for_task(&readiness, &task)
            .expect("configured work item binding should select a flow");
        assert_eq!(selected["flow_id"], bound_flow_id);
        assert_ne!(selected["flow_id"], "stale-flow");

        let unknown_task = task_with_issue_type(&format!("{work_item_type}_unknown"));
        assert!(selected_dev_team_flow_for_task(&readiness, &unknown_task).is_none());
        let authority_only_readiness = serde_json::json!({
            "team_flow_authority": readiness["team_flow_authority"].clone(),
        });
        assert_eq!(
            selected_dev_team_flow_for_task(&authority_only_readiness, &task)
                .expect("persisted authority binding should remain selectable")["flow_id"],
            bound_flow_id
        );
    }

    #[test]
    fn flow_policy_controls_sequentiality_and_ignores_mutable_readiness_projection() {
        let bundle = authority_bundle();
        let authority = bundle["team_flow_authority"].clone();
        let (binding, flow_id) = persisted_enabled_binding(&authority);
        let flow = authority["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("persisted flows")
            .iter()
            .find(|flow| flow["flow_id"].as_str() == Some(flow_id.as_str()))
            .expect("explicit binding should resolve an enabled flow");
        let readiness = serde_json::json!({
            "team_flow_authority": authority,
            "work_item_flow_bindings": {(binding.clone()): "stale-flow"},
            "flows": [{"flow_id": "stale-flow", "sequential": true}]
        });
        assert_eq!(
            dev_team_flow_is_explicitly_sequential(&readiness, Some(&flow_id)),
            flow["flow_policy"]["sequential"].as_bool().unwrap_or(false)
        );
        let selected = selected_dev_team_flow_for_task(&readiness, &task_with_issue_type(&binding))
            .expect("persisted explicit binding should select the bound flow");
        assert_eq!(selected["flow_id"], flow_id);
    }

    #[test]
    fn configured_task_route_preserves_bound_flow_and_exact_node_identity() {
        let bundle = authority_bundle();
        let authority = bundle["team_flow_authority"].clone();
        let default_flow = authority["selected_config"]["authority_selection"]["default_flow_id"]
            .as_str()
            .expect("default flow id")
            .to_string();
        let candidates = authority["resolved_all_flow_payload"]["work_item_flow_bindings"]
            .as_object()
            .expect("persisted work item bindings")
            .iter()
            .filter_map(|(work_item, flow)| {
                let flow_id = flow.as_str()?;
                (flow_id != default_flow).then(|| (work_item.clone(), flow_id.to_string()))
            })
            .collect::<Vec<_>>();
        let (_work_item, flow_id, route) = candidates
            .into_iter()
            .find_map(|(work_item, flow_id)| {
                let task = task_with_issue_type(&work_item);
                configured_dev_team_first_step_for_task(&bundle, &task)
                    .map(|route| (work_item, flow_id, route))
            })
            .expect("authority fixture should expose a non-default task-class binding");
        assert_eq!(route.flow_id.as_deref(), Some(flow_id.as_str()));
        assert!(!route.node_id.is_empty());
        let authority = crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &bundle,
            Some(&flow_id),
            None,
        )
        .expect("bound flow authority should compile");
        let node = authority
            .projection()
            .node(&route.node_id)
            .expect("route node id must belong to bound flow");
        assert_eq!(node.node.node_id, route.node_id);
        assert_eq!(node.node.node_id, route.role_label);
    }

    // ZOMBIE-D: Z/O/M/B/I/E/S/R/P/C for config authority, provider aliases,
    // typed fail-closed selection, and persisted sequence/replay parity.
    #[test]
    fn task_flow_binding_authority_projection_zombie_d_matrix() {
        let bundle = authority_bundle();
        let authority = bundle["team_flow_authority"].clone();
        let bindings = authority["resolved_all_flow_payload"]["work_item_flow_bindings"]
            .as_object()
            .expect("compiled authority should persist work-item bindings");
        let task_flow_id = bindings
            .get("task")
            .and_then(serde_json::Value::as_str)
            .expect("compiled authority should bind task")
            .to_string();
        let task = task_with_issue_type("task");
        assert_eq!(
            task_flow_lookup_keys(&task),
            vec!["task".to_string()],
            "lookup keys must not contain a static/default flow id"
        );

        // Fresh/explicit/alternate-projection: the persisted compiled binding wins
        // over an unrelated readiness projection supplied by another project.
        let alternate_projection = serde_json::json!({
            "team_flow_authority": authority,
            "work_item_flow_bindings": {"task": "alternate-project-flow"},
            "flows": [{"flow_id": "alternate-project-flow"}]
        });
        assert_eq!(
            selected_dev_team_flow_id_for_task(&alternate_projection, &task)
                .expect("persisted task binding should select"),
            task_flow_id
        );

        // Provider-neutral alias resolution uses the compiled canonical binding.
        let defect_flow_id = bindings
            .get("defect")
            .and_then(serde_json::Value::as_str)
            .expect("compiled authority should bind defect")
            .to_string();
        let provider_task = task_with_provider_alias("bug", "jira", "bug");
        assert_eq!(
            selected_dev_team_flow_id_for_task(&alternate_projection, &provider_task)
                .expect("provider alias should resolve to canonical defect"),
            defect_flow_id
        );

        // Missing and unknown bindings fail closed with typed blockers.
        let mut missing = alternate_projection.clone();
        missing["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            .as_object_mut()
            .expect("persisted bindings")
            .remove("task");
        assert_eq!(
            selected_dev_team_flow_id_for_task(&missing, &task),
            Err("team_flow_authority_work_item_flow_binding_missing".to_string())
        );
        let mut unknown = alternate_projection.clone();
        unknown["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            ["task"] = serde_json::json!("unknown-flow");
        assert_eq!(
            selected_dev_team_flow_id_for_task(&unknown, &task),
            Err("team_flow_authority_unknown_flow".to_string())
        );

        // Ambiguous aliases fail closed instead of taking the first map entry.
        let alternate_defect_flow = authority["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("persisted flows")
            .iter()
            .find_map(|flow| {
                let flow_id = flow["flow_id"].as_str()?;
                (flow_id != defect_flow_id).then_some(flow_id.to_string())
            })
            .expect("authority should expose an alternate flow");
        let mut ambiguous = alternate_projection.clone();
        ambiguous["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            ["bug"] = serde_json::json!(alternate_defect_flow);
        assert!(matches!(
            selected_dev_team_flow_id_for_task(&ambiguous, &provider_task),
            Err(error) if error.starts_with("team_flow_authority_work_item_flow_binding_ambiguous:")
        ));

        // Persisted/replay parity: selection, route, and sequence retain one flow id/node.
        let selected = selected_dev_team_flow_id_for_task(&alternate_projection, &task)
            .expect("task binding should replay");
        let sequence = dev_team_sequence_for_task(&alternate_projection, &task);
        let route = configured_dev_team_first_step_for_task(&alternate_projection, &task)
            .expect("task route should replay");
        assert_eq!(route.flow_id.as_deref(), Some(selected.as_str()));
        assert_eq!(
            sequence.first().map(|step| step.node_id.as_str()),
            route.sequence.first().map(|step| step.node_id.as_str())
        );
    }
}
