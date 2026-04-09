use crate::{
    dispatch_contract_execution_lane_sequence, dispatch_contract_lane,
    runtime_assignment_from_execution_plan, selected_backend_from_execution_plan_route,
    RuntimeConsumptionLaneSelection,
};

pub(crate) fn fallback_runtime_consumption_run_graph_status(
    role_selection: &RuntimeConsumptionLaneSelection,
    run_id: &str,
) -> crate::state_store::RunGraphStatus {
    let conversational_mode = role_selection.conversational_mode.as_deref();
    let route_target = match conversational_mode {
        Some("scope_discussion") => "spec-pack".to_string(),
        Some("pbi_discussion") => "work-pool-pack".to_string(),
        _ if role_selection.execution_plan["status"] == "design_first" => "spec-pack".to_string(),
        _ => dispatch_contract_execution_lane_sequence(
            &role_selection.execution_plan["development_flow"]["dispatch_contract"],
        )
        .first()
        .map(|value| value.as_str())
        .unwrap_or(role_selection.selected_role.as_str())
        .to_string(),
    };
    let selected_route = if conversational_mode.is_some() {
        &role_selection.execution_plan["default_route"]
    } else {
        dispatch_contract_lane(&role_selection.execution_plan, &route_target).unwrap_or(
            runtime_assignment_from_execution_plan(&role_selection.execution_plan),
        )
    };
    let route_backend =
        selected_backend_from_execution_plan_route(&role_selection.execution_plan, selected_route)
            .unwrap_or_else(|| "unknown".to_string());
    crate::state_store::RunGraphStatus {
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
        lane_id: format!("{}_lane", role_selection.selected_role.replace('-', "_")),
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
    }
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
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
}
