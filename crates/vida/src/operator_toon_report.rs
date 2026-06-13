pub(crate) use operator_output::toon_report::OperatorToonField;

pub(crate) fn render(surface: &str, fields: Vec<OperatorToonField>) -> String {
    operator_output::toon_report::render(surface, fields)
}

pub(crate) fn render_value(surface: &str, value: serde_json::Value) -> String {
    operator_output::toon_report::render_value(surface, value)
}

pub(crate) fn print(surface: &str, fields: Vec<OperatorToonField>) {
    operator_output::toon_report::print(surface, fields);
}

pub(crate) fn select_fields(value: serde_json::Value, fields: Option<&str>) -> serde_json::Value {
    operator_output::toon_report::select_fields(value, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_outputs_compact_toon_section_with_fields() {
        let output = render(
            "vida status",
            vec![
                OperatorToonField::text("status", "pass"),
                OperatorToonField::text("backend", "state-store"),
            ],
        );

        assert!(output.starts_with("vida status\n"));
        assert!(output.contains("status: pass"));
        assert!(output.contains("backend: \"state-store\""));
    }

    #[test]
    fn render_outputs_value_arrays_with_toon_headers() {
        let output = render(
            "vida agent host-bridge",
            vec![OperatorToonField::value(
                "blocker_codes",
                serde_json::json!(["host_tool_capability_missing"]),
            )],
        );

        assert!(output.starts_with("vida agent host-bridge\n"));
        assert!(output.contains("blocker_codes[1]:"));
        assert!(output.contains("host_tool_capability_missing"));
    }

    #[test]
    fn select_fields_returns_requested_top_level_fields_in_order() {
        let value = serde_json::json!({
            "status": "pass",
            "open_count": 3,
            "ready_count": 2,
        });
        let selected = select_fields(value, Some("ready_count,status,missing"));
        let object = selected
            .as_object()
            .expect("selected fields should remain an object");
        assert_eq!(
            object.keys().cloned().collect::<Vec<_>>(),
            vec!["ready_count".to_string(), "status".to_string()]
        );
        assert_eq!(selected["ready_count"], 2);
        assert_eq!(selected["status"], "pass");
    }
}
