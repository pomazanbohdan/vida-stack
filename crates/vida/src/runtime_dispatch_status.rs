use crate::{
    runtime_assignment_from_execution_plan, selected_backend_from_execution_plan_route,
    RuntimeConsumptionLaneSelection,
};

pub(crate) fn fallback_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let conversational_mode = role_selection.conversational_mode.as_deref();
    let (route_target, mut route_blocker, route_lane_id) = match conversational_mode {
        Some("scope_discussion") => ("spec-pack".to_string(), None, None),
        Some("pbi_discussion") => ("work-pool-pack".to_string(), None, None),
        _ if role_selection.execution_plan["status"] == "design_first" => {
            ("spec-pack".to_string(), None, None)
        }
        _ => match crate::runtime_dispatch_state::typed_lane_node_sequence(role_selection, true)
            .and_then(|sequence| {
                sequence
                    .into_iter()
                    .next()
                    .ok_or_else(|| "dispatch_contract_lane_catalog_incomplete".to_string())
            }) {
            Ok(node) => (node.node_id, None, Some(node.lane_id)),
            Err(blocker_code) => ("blocked".to_string(), Some(blocker_code), None),
        },
    };
    let typed_route = if conversational_mode.is_none() && route_blocker.is_none() {
        match crate::runtime_dispatch_state::require_team_flow_authority_for_selection(
            role_selection,
        )
        .map_err(|blocker| blocker.code)
        .and_then(|authority| {
            authority
                .resolve_target(Some(&role_selection.execution_plan), &route_target)
                .map_err(|blocker| blocker.code)
        }) {
            Ok(node) => Some(node),
            Err(blocker) => {
                route_blocker = Some(blocker);
                None
            }
        }
    } else {
        None
    };
    let selected_route = if conversational_mode.is_some() {
        &role_selection.execution_plan["default_route"]
    } else {
        typed_route
            .as_ref()
            .map(|node| &node.assignment)
            .unwrap_or_else(|| {
                runtime_assignment_from_execution_plan(&role_selection.execution_plan)
            })
    };
    let route_backend = if conversational_mode.is_some() {
        selected_runtime_assignment_carrier(&role_selection.execution_plan)
    } else {
        None
    }
    .or_else(|| {
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, selected_route)
    })
    .unwrap_or_else(|| "unknown".to_string());
    let mut status = crate::state_store::RunGraphStatus {
        run_id: run_id.to_string(),
        task_id: run_id.to_string(),
        task_class: conversational_mode.unwrap_or("implementation").to_string(),
        active_node: if conversational_mode.is_some() {
            role_selection.selected_role.clone()
        } else {
            "planning".to_string()
        },
        next_node: Some(route_target.clone()),
        status: "ready".to_string(),
        route_task_class: if conversational_mode.is_some() {
            route_target.clone()
        } else {
            "implementation".to_string()
        },
        selected_backend: route_backend,
        lane_id: route_lane_id
            .unwrap_or_else(|| format!("{}_lane", role_selection.selected_role.replace('-', "_"))),
        lifecycle_stage: if conversational_mode.is_some() {
            "conversation_active".to_string()
        } else {
            "implementation_dispatch_ready".to_string()
        },
        policy_gate: "not_required".to_string(),
        handoff_state: format!("awaiting_{route_target}"),
        context_state: "sealed".to_string(),
        checkpoint_kind: if conversational_mode.is_some() {
            "conversation_cursor".to_string()
        } else {
            "execution_cursor".to_string()
        },
        resume_target: format!("dispatch.{route_target}"),
        recovery_ready: true,
    };
    if let Some(blocker_code) = route_blocker {
        status.status = "blocked".to_string();
        status.next_node = None;
        status.lifecycle_stage = blocker_code.clone();
        status.policy_gate = blocker_code;
        status.handoff_state = "none".to_string();
        status.context_state = "open".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "diagnose.team_flow_authority".to_string();
        status.recovery_ready = false;
    }
    status
}

fn selected_runtime_assignment_carrier(execution_plan: &serde_json::Value) -> Option<String> {
    let assignment = runtime_assignment_from_execution_plan(execution_plan);
    [
        "selected_tier",
        "activation_agent_type",
        "selected_carrier_id",
        "selected_backend",
    ]
    .iter()
    .find_map(|field| {
        assignment
            .get(*field)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub(crate) fn blocking_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let mut status = fallback_runtime_consumption_run_graph_status(role_selection, run_id);
    status.status = "blocked".to_string();
    status.next_node = None;
    status.lifecycle_stage = "runtime_consumption_blocked".to_string();
    status.handoff_state = "none".to_string();
    status.context_state = "open".to_string();
    status.checkpoint_kind = "none".to_string();
    status.recovery_ready = false;
    status
}

#[cfg(test)]
mod tests {
    use super::{
        blocking_runtime_consumption_run_graph_status,
        fallback_runtime_consumption_run_graph_status,
    };
    use crate::RuntimeConsumptionLaneSelection;

    #[test]
    fn fallback_run_graph_status_uses_carrier_tier_for_conversation_routes() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "research and specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({
                "status": "design_first",
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                },
                "default_route": {
                    "subagents": "internal_subagents"
                },
                "development_flow": {
                    "implementation": {
                        "preferred_agent_tier": "junior",
                        "subagents": "internal_subagents"
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let status = fallback_runtime_consumption_run_graph_status(&role_selection, "run-test");
        assert_eq!(status.selected_backend, "middle");
    }

    #[test]
    fn blocking_run_graph_status_preserves_dispatch_resume_target() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implementation".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({
                "status": "ready",
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                },
                "development_flow": {}
            }),
            reason: "test".to_string(),
        };

        let status = blocking_runtime_consumption_run_graph_status(&role_selection, "run-test");

        assert_eq!(status.status, "blocked");
        assert_eq!(status.resume_target, "dispatch.coach");
        assert!(!status.recovery_ready);
    }

    #[test]
    fn fallback_status_uses_selected_non_default_flow_node() {
        let mut role_selection =
            crate::runtime_dispatch_state::repository_team_flow_test_selection();
        let (flow_id, _) =
            crate::runtime_dispatch_state::select_repository_non_default_flow(&mut role_selection)
                .expect("repository should expose an enabled non-default flow");
        let authority = crate::runtime_dispatch_state::require_team_flow_authority_for_selection(
            &role_selection,
        )
        .expect("selected flow authority should compile");
        let expected_node = authority
            .ordered_nodes()
            .find(|projection| {
                projection.node.included && projection.node.inclusion_rule != "design_gate"
            })
            .expect("selected flow should expose an executable node");
        let status = fallback_runtime_consumption_run_graph_status(&role_selection, "run-test");
        assert_eq!(
            crate::runtime_dispatch_state::selected_flow_ref(&role_selection),
            Some(flow_id.as_str())
        );
        assert_eq!(
            status.next_node.as_deref(),
            Some(expected_node.node.node_id.as_str())
        );
        assert_eq!(status.status, "ready");
    }
}
