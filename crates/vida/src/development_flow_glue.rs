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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_lane_label_normalizes_and_preserves_empty_input() {
        for (input, expected) in [
            (" review_lane ", "review lane"),
            ("qa-ready", "qa ready"),
            ("", ""),
            ("   ", "   "),
        ] {
            assert_eq!(display_lane_label(input), expected, "input={input:?}");
        }
    }

    #[test]
    fn execution_plan_flag_requires_boolean_true_value() {
        assert!(execution_plan_agent_only_development_required(
            &serde_json::json!({
                "autonomous_execution": {"agent_only_development": true}
            })
        ));
        assert!(!execution_plan_agent_only_development_required(
            &serde_json::json!({
                "autonomous_execution": {"agent_only_development": false}
            })
        ));
        assert!(!execution_plan_agent_only_development_required(
            &serde_json::json!({"autonomous_execution": {}})
        ));
        assert!(!execution_plan_agent_only_development_required(
            &serde_json::json!({
                "autonomous_execution": {"agent_only_development": "true"}
            })
        ));
    }
}
