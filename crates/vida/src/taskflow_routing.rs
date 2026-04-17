use crate::runtime_contract_vocab::{
    DISPATCH_TARGET_COACH, DISPATCH_TARGET_EXECUTION_PREPARATION, DISPATCH_TARGET_IMPLEMENTER,
    DISPATCH_TARGET_SPECIFICATION, DISPATCH_TARGET_VERIFICATION, RUNTIME_ROLE_BUSINESS_ANALYST,
    RUNTIME_ROLE_COACH, RUNTIME_ROLE_PM, RUNTIME_ROLE_PROVER, RUNTIME_ROLE_SOLUTION_ARCHITECT,
    RUNTIME_ROLE_VERIFIER, RUNTIME_ROLE_WORKER,
};
use crate::{json_string, json_string_list};

fn legacy_dispatch_contract_lane<'a>(
    dispatch_contract: &'a serde_json::Value,
    dispatch_target: &str,
) -> Option<&'a serde_json::Value> {
    match dispatch_target {
        DISPATCH_TARGET_IMPLEMENTER => dispatch_contract.get("implementer_activation"),
        DISPATCH_TARGET_SPECIFICATION => dispatch_contract.get("specification_activation"),
        DISPATCH_TARGET_COACH => dispatch_contract.get("coach_activation"),
        DISPATCH_TARGET_VERIFICATION => dispatch_contract.get("verifier_activation"),
        "escalation" | DISPATCH_TARGET_EXECUTION_PREPARATION => {
            dispatch_contract.get("escalation_activation")
        }
        _ => None,
    }
}

fn legacy_dispatch_target_for_runtime_role(runtime_role: &str) -> Option<&'static str> {
    match runtime_role {
        RUNTIME_ROLE_BUSINESS_ANALYST | RUNTIME_ROLE_PM => Some(DISPATCH_TARGET_SPECIFICATION),
        RUNTIME_ROLE_WORKER => Some(DISPATCH_TARGET_IMPLEMENTER),
        RUNTIME_ROLE_COACH => Some(DISPATCH_TARGET_COACH),
        RUNTIME_ROLE_VERIFIER | RUNTIME_ROLE_PROVER => Some(DISPATCH_TARGET_VERIFICATION),
        RUNTIME_ROLE_SOLUTION_ARCHITECT => Some(DISPATCH_TARGET_EXECUTION_PREPARATION),
        _ => None,
    }
}

fn canonical_dispatch_target_name(dispatch_target: &str) -> String {
    legacy_dispatch_target_for_runtime_role(dispatch_target)
        .unwrap_or(dispatch_target)
        .to_string()
}

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
    legacy_dispatch_contract_lane(dispatch_contract, dispatch_target)
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
        .map(canonical_dispatch_target_name)
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

pub(crate) fn dispatch_target_for_runtime_role(
    execution_plan: &serde_json::Value,
    runtime_role: &str,
) -> Option<String> {
    let runtime_role = runtime_role.trim();
    if runtime_role.is_empty() {
        return None;
    }
    if let Some(lane_catalog) =
        execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"].as_object()
    {
        for (dispatch_target, lane) in lane_catalog {
            let activation = dispatch_contract_lane_activation(lane);
            let lane_runtime_role = json_string(activation.get("activation_runtime_role"))
                .or_else(|| json_string(lane.get("runtime_role")));
            if lane_runtime_role.as_deref() == Some(runtime_role) {
                return Some(dispatch_target.clone());
            }
        }
    }
    legacy_dispatch_target_for_runtime_role(runtime_role).map(str::to_string)
}

fn carrier_backend_from_assignment(assignment: &serde_json::Value) -> Option<String> {
    json_string(assignment.get("selected_tier"))
        .or_else(|| json_string(assignment.get("activation_agent_type")))
        .filter(|value| !value.is_empty())
}

fn route_backend_value(route: &serde_json::Value, key: &str) -> Option<String> {
    json_string(route.get(key))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            crate::json_string_list(route.get(key))
                .into_iter()
                .find(|value| !value.trim().is_empty())
        })
}

pub(crate) fn runtime_assignment_from_route<'a>(
    route: &'a serde_json::Value,
) -> &'a serde_json::Value {
    route
        .get("activation")
        .or_else(|| route.get("carrier_runtime_assignment"))
        .or_else(|| route.get("runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

#[allow(dead_code)]
pub(crate) fn runtime_assignment_source_from_route(route: &serde_json::Value) -> &'static str {
    if route.get("activation").is_some() {
        "activation"
    } else if route.get("carrier_runtime_assignment").is_some() {
        "carrier_runtime_assignment"
    } else if route.get("runtime_assignment").is_some() {
        "runtime_assignment"
    } else {
        "missing"
    }
}

pub(crate) fn runtime_assignment_from_execution_plan<'a>(
    execution_plan: &'a serde_json::Value,
) -> &'a serde_json::Value {
    execution_plan
        .get("runtime_assignment")
        .or_else(|| execution_plan.get("carrier_runtime_assignment"))
        .unwrap_or(&serde_json::Value::Null)
}

pub(crate) fn runtime_assignment_source_from_execution_plan(
    execution_plan: &serde_json::Value,
) -> &'static str {
    if execution_plan.get("runtime_assignment").is_some() {
        "runtime_assignment"
    } else if execution_plan.get("carrier_runtime_assignment").is_some() {
        "carrier_runtime_assignment"
    } else {
        "missing"
    }
}

#[allow(dead_code)]
fn carrier_backend_from_route(route: &serde_json::Value) -> Option<String> {
    json_string(route.get("preferred_agent_tier"))
        .or_else(|| json_string(route.get("preferred_agent_type")))
        .or_else(|| carrier_backend_from_assignment(runtime_assignment_from_route(route)))
        .filter(|value| !value.is_empty())
}

pub(crate) fn explicit_executor_backend_from_route(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "executor_backend")
}

pub(crate) fn fallback_executor_backend_from_route(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "fallback_executor_backend")
        .or_else(|| route_backend_value(route, "bridge_fallback_subagent"))
}

pub(crate) fn fanout_executor_backends_from_route(route: &serde_json::Value) -> Vec<String> {
    let values = json_string_list(route.get("fanout_executor_backends"));
    if !values.is_empty() {
        return values;
    }
    json_string_list(route.get("fanout_subagents"))
}

#[allow(dead_code)]
fn legacy_route_backend_hint(route: &serde_json::Value) -> Option<String> {
    route_backend_value(route, "carrier_backend_hint")
        .or_else(|| route_backend_value(route, "subagents"))
        .or_else(|| route_backend_value(route, "bridge_fallback_subagent"))
        .or_else(|| route_backend_value(route, "fanout_subagents"))
}

pub(crate) fn selected_backend_from_execution_plan_route(
    execution_plan: &serde_json::Value,
    route: &serde_json::Value,
) -> Option<String> {
    explicit_executor_backend_from_route(route)
        .or_else(|| route_backend_value(route, "fallback_executor_backend"))
        .or_else(|| route_backend_value(route, "fanout_executor_backends"))
        .or_else(|| carrier_backend_from_assignment(dispatch_contract_lane_activation(route)))
        .or_else(|| carrier_backend_from_assignment(runtime_assignment_from_route(route)))
        .or_else(|| {
            carrier_backend_from_assignment(runtime_assignment_from_execution_plan(execution_plan))
        })
        .or_else(|| legacy_route_backend_hint(route))
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        explicit_executor_backend_from_route, fallback_executor_backend_from_route,
        fanout_executor_backends_from_route, selected_backend_from_execution_plan_route,
    };

    #[test]
    fn explicit_executor_backend_wins_over_carrier_tier_and_legacy_hints() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "middle",
                "activation_agent_type": "middle",
            },
            "development_flow": {
                "implementation": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "internal_review",
                    "fanout_executor_backends": ["internal_fast", "internal_arch"],
                    "preferred_agent_tier": "junior",
                    "preferred_agent_type": "junior",
                    "carrier_backend_hint": "legacy_hint",
                    "subagents": "legacy_subagents",
                    "bridge_fallback_subagent": "legacy_bridge",
                    "fanout_subagents": "legacy_fanout",
                }
            },
            "default_route": {
                "subagents": "legacy_subagents"
            },
            "status": "execution_ready",
        });
        let route = &execution_plan["development_flow"]["implementation"];

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, route).as_deref(),
            Some("internal_subagents")
        );
    }

    #[test]
    fn explicit_executor_helpers_preserve_fallback_and_fanout_fields() {
        let route = serde_json::json!({
            "executor_backend": "internal_subagents",
            "fallback_executor_backend": "internal_review",
            "fanout_executor_backends": ["internal_fast", "internal_arch"]
        });

        assert_eq!(
            explicit_executor_backend_from_route(&route).as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            fallback_executor_backend_from_route(&route).as_deref(),
            Some("internal_review")
        );
        assert_eq!(
            fanout_executor_backends_from_route(&route),
            vec!["internal_fast".to_string(), "internal_arch".to_string()]
        );
    }

    #[test]
    fn explicit_executor_helpers_fall_back_to_legacy_hints() {
        let route = serde_json::json!({
            "carrier_backend_hint": "legacy_hint",
            "subagents": "legacy_subagents",
            "bridge_fallback_subagent": "legacy_bridge",
            "fanout_subagents": "legacy_fanout"
        });

        assert_eq!(explicit_executor_backend_from_route(&route), None);
        assert_eq!(
            fallback_executor_backend_from_route(&route).as_deref(),
            Some("legacy_bridge")
        );
        assert_eq!(
            fanout_executor_backends_from_route(&route),
            vec!["legacy_fanout".to_string()]
        );
        assert_eq!(
            selected_backend_from_execution_plan_route(&serde_json::json!({}), &route).as_deref(),
            Some("legacy_hint")
        );
    }

    #[test]
    fn selected_backend_prefers_route_activation_agent_type_when_executor_hints_missing() {
        let execution_plan = serde_json::json!({});
        let route = serde_json::json!({
            "activation": {
                "activation_agent_type": "middle",
                "activation_runtime_role": "business_analyst"
            }
        });

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, &route).as_deref(),
            Some("middle")
        );
    }

    #[test]
    fn selected_backend_preserves_explicit_priority_over_activation_hint() {
        let execution_plan = serde_json::json!({
            "runtime_assignment": {
                "selected_tier": "senior",
            }
        });
        let route = serde_json::json!({
            "activation": {
                "activation_agent_type": "middle",
            },
            "fallback_executor_backend": "qwen_cli"
        });

        assert_eq!(
            selected_backend_from_execution_plan_route(&execution_plan, &route).as_deref(),
            Some("qwen_cli")
        );
    }
}
