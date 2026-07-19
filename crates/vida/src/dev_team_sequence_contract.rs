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
                next_action: format!("Repair configured {} transition `{}`.", transition.transition_kind, transition.event),
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
        .or_else(|| activation_bundle.get("compiled_agent_extension_bundle"))
}

fn snapshot_sequence(activation_bundle: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
    let Some(bundle) = team_flow_bundle(activation_bundle) else {
        return Vec::new();
    };
    let Ok(projection) =
        crate::team_flow_authority_adapter::compile_team_flow_authority(bundle, None, None)
    else {
        return Vec::new();
    };
    projection
        .ordered_nodes()
        .map(|node| DevTeamSequenceStep {
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

pub(crate) fn configured_dev_team_first_step_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Option<ConfiguredDevTeamTaskRoute> {
    let sequence = snapshot_sequence(activation_bundle);
    let task_value = serde_json::to_value(task).ok()?;
    let requested_class = crate::infer_task_class_from_task_payload(&task_value);
    let step = sequence
        .iter()
        .find(|step| step.requires_task && step.task_class == requested_class)
        .or_else(|| sequence.iter().find(|step| step.requires_task))?
        .clone();
    let flow_id = team_flow_bundle(activation_bundle)
        .and_then(|bundle| {
            crate::team_flow_authority_adapter::compile_team_flow_authority(bundle, None, None).ok()
        })
        .map(|projection| projection.snapshot.flow_ref);
    Some(ConfiguredDevTeamTaskRoute {
        flow_id,
        role_label: step.role_label.clone(),
        runtime_role: step.runtime_role.clone(),
        task_class: step.task_class.clone(),
        dispatch_target: step.role_label.clone(),
        sequence,
    })
}

pub(crate) fn dev_team_sequence(activation_bundle: &serde_json::Value) -> Vec<DevTeamSequenceStep> {
    snapshot_sequence(activation_bundle)
}

pub(crate) fn dev_team_sequence_for_flow_id(
    activation_bundle: &serde_json::Value,
    flow_id: &str,
) -> Vec<DevTeamSequenceStep> {
    let Some(bundle) = team_flow_bundle(activation_bundle) else {
        return Vec::new();
    };
    crate::team_flow_authority_adapter::compile_team_flow_authority(bundle, Some(flow_id), None)
        .map(|projection| {
            projection
                .ordered_nodes()
                .map(|node| DevTeamSequenceStep {
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
        })
        .unwrap_or_default()
}

pub(crate) fn dev_team_sequence_for_work_item(
    activation_bundle: &serde_json::Value,
    _work_item_type: &str,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence(activation_bundle)
}

pub(crate) fn dev_team_sequence_for_task(
    activation_bundle: &serde_json::Value,
    _task: &state_store::TaskRecord,
) -> Vec<DevTeamSequenceStep> {
    dev_team_sequence(activation_bundle)
}

/// Compatibility surface for the dispatch planner. Flow selection is owned by
/// the compiled TeamFlow authority; readiness JSON is deliberately not a
/// second parser or a fallback source.
pub(crate) fn selected_dev_team_flow_for_task<'a>(
    readiness: &'a serde_json::Value,
    task: &state_store::TaskRecord,
) -> Option<&'a serde_json::Value> {
    let bindings = readiness
        .get("work_item_flow_bindings")
        .and_then(serde_json::Value::as_object)?;
    let flow_ref = task_flow_lookup_keys(task)
        .into_iter()
        .find_map(|key| bindings.get(&key).and_then(serde_json::Value::as_str))?;
    let authority = readiness.get("team_flow_authority")?;
    let bundle = serde_json::json!({"team_flow_authority": authority});
    let projection = crate::team_flow_authority_adapter::compile_team_flow_authority(
        &bundle,
        Some(flow_ref),
        None,
    )
    .ok()?;
    readiness["flows"]
        .as_array()?
        .iter()
        .find(|flow| flow["flow_id"].as_str() == Some(projection.snapshot.flow_ref.as_str()))
}

pub(crate) fn dev_team_flow_is_explicitly_sequential(
    _readiness: &serde_json::Value,
    flow_id: Option<&str>,
) -> bool {
    flow_id.is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn task_flow_lookup_keys(task: &state_store::TaskRecord) -> Vec<String> {
    let mut keys = Vec::new();
    let task_value = serde_json::to_value(task).unwrap_or(serde_json::Value::Null);
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    let work_item_kind = state_store::task_work_item_kind(&task.issue_type);
    for value in [
        work_item_kind.canonical_issue_type.as_str(),
        work_item_kind
            .provider_issue_type
            .as_deref()
            .unwrap_or_default(),
        task.issue_type.as_str(),
        work_item_kind.default_flow_binding.as_str(),
        inferred_task_class.as_str(),
    ] {
        let value = value.trim().to_ascii_lowercase();
        if !value.is_empty() && !keys.contains(&value) {
            keys.push(value);
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_bundle() -> serde_json::Value {
        let config = serde_json::json!({
            "authority_selection": {"config_id":"cfg","team_profile_id":"profile","default_flow_id":"default_flow"},
            "roles": {"coder": {"runtime_role":"worker","task_class":"implementation","inclusion_rule":"always","packet_template_kind":"delivery_task_packet","closure_class":"implementation","stage":"execution","completion_blocker":"pending"}},
            "flows": {
                "default_flow": {"flow_id":"default_flow","steps": [{"role_id":"coder","included":true,"required":true,"proof_gates":{"required_outputs":["changed"]},"terminal":true}]},
                "bound_flow": {"flow_id":"bound_flow","steps": [{"role_id":"coder","included":true,"required":true,"proof_gates":{"required_outputs":["changed"]},"terminal":true}]}
            }
        });
        let hash = taskflow_authority::team_flow_transition::hash_json(&config);
        serde_json::json!({
            "team_flow_authority": {
                "authority_id":"team-flow-authority:test",
                "config":{"content_blake3":hash},
                "registries":{"content_blake3":"registry"},
                "selected_config":config
            }
        })
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
    fn sequence_uses_snapshot_flow_and_unknown_flow_fails_closed() {
        let bundle = authority_bundle();
        let default_flow = bundle["team_flow_authority"]["selected_config"]["authority_selection"]
            ["default_flow_id"]
            .as_str()
            .expect("authority fixture should select a default flow");
        assert_eq!(
            dev_team_sequence_for_flow_id(&bundle, default_flow).len(),
            1
        );
        let unknown_flow = format!("{default_flow}_unknown");
        assert!(dev_team_sequence_for_flow_id(&bundle, &unknown_flow).is_empty());
    }

    #[test]
    fn selected_flow_uses_task_binding_and_fails_closed_without_binding() {
        let bundle = authority_bundle();
        let authority = bundle["team_flow_authority"].clone();
        let selected_config = &authority["selected_config"];
        let default_flow = selected_config["authority_selection"]["default_flow_id"]
            .as_str()
            .expect("authority fixture should select a default flow");
        let bound_flow = selected_config["flows"]
            .as_object()
            .expect("authority fixture should define flows")
            .keys()
            .find(|flow_id| flow_id.as_str() != default_flow)
            .cloned()
            .expect("authority fixture should define a non-default flow");
        let work_item_type = selected_config["roles"]
            .as_object()
            .expect("authority fixture should define roles")
            .keys()
            .next()
            .cloned()
            .expect("authority fixture should define a role");
        let mut bindings = serde_json::Map::new();
        bindings.insert(work_item_type.clone(), serde_json::json!(bound_flow));
        let readiness = serde_json::json!({
            "team_flow_authority": authority,
            "work_item_flow_bindings": serde_json::Value::Object(bindings),
            "flows": [
                {"flow_id": default_flow},
                {"flow_id": bound_flow}
            ]
        });
        let task = task_with_issue_type(&work_item_type);
        let selected = selected_dev_team_flow_for_task(&readiness, &task)
            .expect("configured work item binding should select a flow");
        assert_eq!(selected["flow_id"], bound_flow);
        assert_ne!(selected["flow_id"], default_flow);

        let unknown_task = task_with_issue_type(&format!("{work_item_type}_unknown"));
        assert!(selected_dev_team_flow_for_task(&readiness, &unknown_task).is_none());
        let unbound_readiness = serde_json::json!({
            "team_flow_authority": readiness["team_flow_authority"].clone(),
            "work_item_flow_bindings": {},
            "flows": readiness["flows"].clone()
        });
        assert!(selected_dev_team_flow_for_task(&unbound_readiness, &task).is_none());
    }
}
