use crate::json_string;

pub(crate) fn dispatch_contract_lane<'a>(
    execution_plan: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(lane) =
        execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"].get(dispatch_target)
    {
        return Some(lane);
    }
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    match dispatch_target {
        "implementer" => dispatch_contract.get("implementer_activation"),
        "specification" => dispatch_contract.get("specification_activation"),
        "coach" => dispatch_contract.get("coach_activation"),
        "verification" => dispatch_contract.get("verifier_activation"),
        "escalation" | "execution_preparation" => dispatch_contract.get("escalation_activation"),
        _ => None,
    }
}

pub(crate) fn dispatch_contract_lane_activation(lane: &serde_json::Value) -> &serde_json::Value {
    lane.get("activation").unwrap_or(lane)
}

pub(crate) fn dispatch_contract_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    let explicit = dispatch_contract["lane_sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    let mut legacy = Vec::new();
    if dispatch_contract.get("specification_activation").is_some() {
        legacy.push("specification".to_string());
    }
    if dispatch_contract.get("escalation_activation").is_some() {
        legacy.push("execution_preparation".to_string());
    }
    if dispatch_contract.get("implementer_activation").is_some() {
        legacy.push("implementer".to_string());
    }
    if dispatch_contract.get("coach_activation").is_some() {
        legacy.push("coach".to_string());
    }
    if dispatch_contract.get("verifier_activation").is_some() {
        legacy.push("verification".to_string());
    }
    legacy
}

pub(crate) fn dispatch_contract_execution_lane_sequence(
    dispatch_contract: &serde_json::Value,
) -> Vec<String> {
    let explicit = dispatch_contract["execution_lane_sequence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    dispatch_contract_lane_sequence(dispatch_contract)
        .into_iter()
        .filter(|target| target != "specification")
        .collect()
}

fn carrier_backend_from_assignment(assignment: &serde_json::Value) -> Option<String> {
    json_string(assignment.get("selected_tier"))
        .or_else(|| json_string(assignment.get("activation_agent_type")))
        .filter(|value| !value.is_empty())
}

fn runtime_assignment_from_route<'a>(route: &'a serde_json::Value) -> &'a serde_json::Value {
    route
        .get("activation")
        .or_else(|| route.get("runtime_assignment"))
        .or_else(|| route.get("codex_runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

fn runtime_assignment_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
) -> &'a serde_json::Value {
    execution_plan
        .get("runtime_assignment")
        .or_else(|| execution_plan.get("codex_runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

fn carrier_backend_from_route(route: &serde_json::Value) -> Option<String> {
    json_string(route.get("preferred_agent_tier"))
        .or_else(|| json_string(route.get("preferred_agent_type")))
        .or_else(|| carrier_backend_from_assignment(runtime_assignment_from_route(route)))
        .filter(|value| !value.is_empty())
}

pub(crate) fn selected_backend_from_execution_plan_route(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    carrier_backend_from_route(route)
        .or_else(|| {
            carrier_backend_from_assignment(runtime_assignment_from_execution_plan(execution_plan))
        })
        .or_else(|| json_string(route.get("subagents")))
        .filter(|value| !value.is_empty())
}
