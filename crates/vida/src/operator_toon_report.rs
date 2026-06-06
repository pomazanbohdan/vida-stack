pub(crate) struct OperatorToonField {
    pub(crate) key: String,
    pub(crate) value: serde_json::Value,
}

impl OperatorToonField {
    pub(crate) fn text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: serde_json::Value::String(value.into()),
        }
    }

    pub(crate) fn value(key: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

pub(crate) fn render(surface: &str, fields: Vec<OperatorToonField>) -> String {
    let mut payload = serde_json::Map::new();
    for field in fields {
        let key = taskflow_format_toon::sanitize_toon_scalar(&field.key);
        if key.trim().is_empty() {
            continue;
        }
        payload.insert(key, sanitize_value(field.value));
    }
    render_value(surface, serde_json::Value::Object(payload))
}

pub(crate) fn render_value(surface: &str, value: serde_json::Value) -> String {
    taskflow_format_toon::render_value_section(surface, &sanitize_value(value))
}

pub(crate) fn print(surface: &str, fields: Vec<OperatorToonField>) {
    println!("{}", render(surface, fields));
}

pub(crate) fn select_fields(value: serde_json::Value, fields: Option<&str>) -> serde_json::Value {
    let Some(fields) = fields else {
        return value;
    };
    let wanted = fields
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    if wanted.is_empty() {
        return value;
    }
    let Some(object) = value.as_object() else {
        return value;
    };
    let mut selected = serde_json::Map::new();
    for field in wanted {
        if let Some(value) = object.get(field) {
            selected.insert(field.to_string(), value.clone());
        }
    }
    serde_json::Value::Object(selected)
}

fn sanitize_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(value) => {
            serde_json::Value::String(taskflow_format_toon::sanitize_toon_scalar(&value))
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sanitize_value).collect())
        }
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    (
                        taskflow_format_toon::sanitize_toon_scalar(&key),
                        sanitize_value(value),
                    )
                })
                .collect(),
        ),
        other => other,
    }
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
    fn render_escapes_control_characters_in_keys_and_values() {
        let output = render(
            "vida status",
            vec![OperatorToonField::text("bad\nkey", "bad\x1bvalue")],
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("value"));
        assert!(!output.contains('\x1b'));
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
