pub(crate) fn command_text(command: &str) -> String {
    operator_output::command_text::human_command(command)
}

fn action_text(action: &str) -> String {
    command_text(action)
}

pub(crate) fn action_entries(actions: &serde_json::Value) -> Vec<String> {
    actions
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(action_text)
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn output_payload(payload: &serde_json::Value) -> serde_json::Value {
    let mut output = serde_json::Map::new();
    for key in [
        "status",
        "source_run_id",
        "run_id",
        "source_dispatch_packet_path",
        "snapshot_path",
    ] {
        if let Some(value) = payload.get(key).filter(|value| !value.is_null()) {
            output.insert(key.to_string(), value.clone());
        }
    }
    let blocker_codes = payload
        .get("blocker_codes")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    output.insert("blocker_codes".to_string(), blocker_codes);
    output.insert(
        "next_actions".to_string(),
        serde_json::json!(action_entries(&payload["next_actions"])),
    );
    if let Some(next_action) = payload["projection_truth"]["next_lawful_operator_action"]
        .as_str()
        .map(command_text)
        .filter(|value| !value.is_empty())
    {
        output.insert(
            "next_lawful_operator_action".to_string(),
            next_action.into(),
        );
    }
    if let Some(artifact_refs) = payload
        .get("artifact_refs")
        .filter(|value| !value.is_null())
    {
        output.insert("artifact_refs".to_string(), artifact_refs.clone());
    }
    serde_json::Value::Object(output)
}

pub(crate) fn print_toon(surface_name: &str, payload: &serde_json::Value) {
    println!(
        "{}",
        taskflow_format_toon::render_value_section(surface_name, &output_payload(payload),)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_entries_keep_default_human_commands_without_json_bias() {
        let actions = serde_json::json!([
            "vida lane retire run-1",
            "vida taskflow consume continue",
            "vida task show task-1 --json-output"
        ]);

        assert_eq!(
            action_entries(&actions),
            vec![
                "vida lane retire run-1".to_string(),
                "vida taskflow consume continue".to_string(),
                "vida task show task-1 --json-output".to_string()
            ]
        );
    }

    #[test]
    fn output_payload_projects_default_operator_fields_once() {
        let payload = serde_json::json!({
            "status": "blocked",
            "source_run_id": "run-1",
            "source_dispatch_packet_path": ".vida/packets/run-1.json",
            "snapshot_path": null,
            "blocker_codes": ["stale_missing_task_run_graph"],
            "next_actions": ["vida lane retire run-1"],
            "projection_truth": {
                "next_lawful_operator_action": "vida taskflow consume continue"
            },
            "artifact_refs": {
                "run_id": "run-1"
            },
            "ignored_broad_field": {
                "value": true
            }
        });

        let output = output_payload(&payload);

        assert_eq!(output["status"], "blocked");
        assert_eq!(output["source_run_id"], "run-1");
        assert_eq!(output["snapshot_path"], serde_json::Value::Null);
        assert_eq!(
            output["blocker_codes"],
            serde_json::json!(["stale_missing_task_run_graph"])
        );
        assert_eq!(
            output["next_actions"],
            serde_json::json!(["vida lane retire run-1"])
        );
        assert_eq!(
            output["next_lawful_operator_action"],
            "vida taskflow consume continue"
        );
        assert_eq!(output["artifact_refs"]["run_id"], "run-1");
        assert!(output.get("ignored_broad_field").is_none());
    }
}
