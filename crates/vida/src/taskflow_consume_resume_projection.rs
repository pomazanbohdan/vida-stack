pub(crate) fn toon_text(
    surface_name: &str,
    status: &str,
    source_run_id: &str,
    source_dispatch_packet_path: &str,
    snapshot_path: &str,
    projection_reason: Option<&str>,
    next_action: Option<&str>,
) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert("status".to_string(), status.into());
    payload.insert("source_run".to_string(), source_run_id.into());
    payload.insert(
        "source_packet".to_string(),
        source_dispatch_packet_path.into(),
    );
    if let Some(projection_reason) = projection_reason {
        payload.insert("projection".to_string(), projection_reason.into());
    }
    if let Some(next_action) = next_action {
        payload.insert(
            "next_action".to_string(),
            crate::taskflow_consume_resume_output::command_text(next_action).into(),
        );
    }
    payload.insert("snapshot_path".to_string(), snapshot_path.into());
    taskflow_format_toon::render_value_section(surface_name, &serde_json::Value::Object(payload))
}

pub(crate) fn build_operator_projection_payload(
    surface_name: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
    extra_fields: serde_json::Value,
    parity_context: &str,
) -> Result<serde_json::Value, String> {
    let payload = crate::release1_operator_output::build_release1_operator_output_payload(
        surface_name,
        blocker_codes,
        next_actions,
        artifact_refs,
        extra_fields,
    )?;
    validate_operator_projection_payload(&payload, parity_context)?;
    Ok(payload)
}

pub(crate) fn validate_operator_projection_payload(
    payload: &serde_json::Value,
    parity_context: &str,
) -> Result<(), String> {
    if let Some(error) =
        crate::release1_operator_output::shared_operator_output_contract_parity_error(payload)
    {
        return Err(format!(
            "Failed to preserve {parity_context} operator-contract parity: {error}"
        ));
    }
    Ok(())
}

pub(crate) fn write_operator_projection_payload(
    path: &str,
    payload: &serde_json::Value,
    encode_context: &str,
    write_context: &str,
    parity_context: &str,
) -> Result<(), String> {
    validate_operator_projection_payload(payload, parity_context)?;
    std::fs::write(
        path,
        serde_json::to_string_pretty(payload)
            .map_err(|error| format!("Failed to encode {encode_context}: {error}"))?,
    )
    .map_err(|error| format!("Failed to write {write_context}: {error}"))
}

pub(crate) fn build_output_from_projection_payload(
    surface_name: &str,
    projection_payload: &serde_json::Value,
    extra_fields: serde_json::Value,
    parity_context: &str,
) -> Result<serde_json::Value, String> {
    build_operator_projection_payload(
        surface_name,
        string_array_field(projection_payload, "blocker_codes"),
        string_array_field(projection_payload, "next_actions"),
        projection_payload["artifact_refs"].clone(),
        extra_fields,
        parity_context,
    )
}

fn string_array_field(payload: &serde_json::Value, key: &str) -> Vec<String> {
    payload[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_projection_payload_mirrors_release1_contract_fields() {
        let payload = build_operator_projection_payload(
            "vida taskflow consume continue",
            vec!["dispatch_packet_contract_invalid".to_string()],
            vec!["vida taskflow packet repair --run-id run-1 --json".to_string()],
            serde_json::json!({
                "runtime_consumption_latest_snapshot_path": "snapshot.json",
                "latest_run_graph_dispatch_receipt_id": "run-1"
            }),
            serde_json::json!({
                "source_run_id": "run-1",
                "source_dispatch_packet_path": "packet.json"
            }),
            "test consume-resume projection",
        )
        .expect("projection payload should build");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], payload["status"]);
        assert_eq!(payload["operator_contracts"]["status"], payload["status"]);
        assert_eq!(
            payload["shared_fields"]["artifact_refs"],
            payload["artifact_refs"]
        );
        assert_eq!(
            payload["operator_contracts"]["artifact_refs"],
            payload["artifact_refs"]
        );
        assert!(
            payload["next_actions"][0]
                .as_str()
                .expect("next action should be text")
                .contains("vida taskflow packet repair")
        );
    }

    #[test]
    fn toon_text_projects_default_human_command_without_json_bias() {
        let output = toon_text(
            "vida taskflow consume continue",
            "blocked",
            "run-1",
            "packet.json",
            "snapshot.json",
            Some("blocked projection"),
            Some("vida taskflow consume continue --json"),
        );

        assert!(output.starts_with("vida taskflow consume continue"));
        assert!(output.contains("next_action: vida taskflow consume continue"));
        assert!(!output.contains("--json"));
    }

    #[test]
    fn output_from_projection_payload_preserves_operator_mirrors() {
        let projection = build_operator_projection_payload(
            "vida taskflow consume continue",
            Vec::new(),
            Vec::new(),
            serde_json::json!({"latest_run_graph_dispatch_receipt_id": "run-1"}),
            serde_json::json!({"snapshot_path": "snapshot.json"}),
            "test projection",
        )
        .expect("projection should build");

        let output = build_output_from_projection_payload(
            "vida taskflow consume continue",
            &projection,
            serde_json::json!({"source_run_id": "run-1"}),
            "test output",
        )
        .expect("output should build");

        assert_eq!(output["status"], "pass");
        assert_eq!(output["shared_fields"]["status"], output["status"]);
        assert_eq!(output["operator_contracts"]["status"], output["status"]);
        assert_eq!(output["source_run_id"], "run-1");
    }
}
