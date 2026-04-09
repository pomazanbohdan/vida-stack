pub(crate) fn display_lane_label(value: &str) -> String {
    let label = value.trim().replace('_', " ").replace('-', " ");
    if label.is_empty() {
        value.to_string()
    } else {
        label
    }
}

pub(crate) fn execution_plan_agent_only_development_required(
    execution_plan: &serde_json::Value,
) -> bool {
    execution_plan["autonomous_execution"]["agent_only_development"]
        .as_bool()
        .unwrap_or(false)
}
