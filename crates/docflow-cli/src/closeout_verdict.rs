pub(crate) use docflow_contracts::{
    DocflowCloseoutVerdict, DocflowCloseoutVerdictInput, build_docflow_closeout_verdict,
};
use docflow_format_jsonl::encode_line;

pub(crate) fn render_docflow_closeout_verdict(
    command: &str,
    verdict: &DocflowCloseoutVerdict,
    format: &str,
    compact: bool,
) -> String {
    if format == "json" {
        let mut payload = serde_json::json!(verdict);
        payload["command"] = command.into();
        if compact {
            payload["changed_docs"] = serde_json::Value::Array(Vec::new());
        }
        return serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
            format!(
                "{{\"command\":\"{command}\",\"verdict\":\"blocking\",\"error\":\"encode_error:{error}\"}}"
            )
        });
    }
    if format == "jsonl" {
        return encode_line(verdict).unwrap_or_else(|error| {
            format!(
                "{{\"command\":\"{command}\",\"verdict\":\"blocking\",\"error\":\"encode_error:{error}\"}}"
            )
        });
    }
    let mut lines = vec![
        command.to_string(),
        format!("  mode: {}", verdict.mode),
        format!("  task_id: {}", verdict.task_id.as_deref().unwrap_or("")),
        format!("  changed_doc_count: {}", verdict.changed_doc_count),
        format!(
            "  protocol_coverage_rows: {}",
            verdict.protocol_coverage_rows
        ),
        format!("  task_close_allowed: {}", verdict.task_close_allowed),
        format!("  verdict: {}", verdict.verdict),
        format!("  blocker_codes: {}", verdict.blocker_codes.join(",")),
    ];
    if !compact {
        lines.push("  changed_docs:".to_string());
        for path in &verdict.changed_docs {
            lines.push(format!("    - {path}"));
        }
    }
    lines.push("  next_actions:".to_string());
    for action in &verdict.next_actions {
        lines.push(format!("    - {action}"));
    }
    lines.join("\n")
}

pub(crate) fn render_docflow_closeout_error(
    command: &str,
    format: &str,
    task_id: Option<&str>,
    error: String,
) -> String {
    let payload = serde_json::json!({
        "command": command,
        "task_id": task_id,
        "verdict": "blocking",
        "task_close_allowed": false,
        "blocker_codes": ["docflow_closeout_failed"],
        "error": error,
        "next_actions": ["Inspect DocFlow command inputs and retry the default command after the missing root, task, or changed-doc evidence is available."]
    });
    if format == "json" {
        serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!("{{\"command\":\"{command}\",\"verdict\":\"blocking\"}}"))
    } else {
        format!(
            "{command}\n  verdict: blocking\n  task_close_allowed: false\n  blocker_codes: docflow_closeout_failed\n  error: {}",
            payload["error"].as_str().unwrap_or("unknown")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_renderer_keeps_plain_and_json_contracts() {
        let verdict = build_docflow_closeout_verdict(DocflowCloseoutVerdictInput {
            command: "docflow closeout",
            mode: "changed",
            task_id: None,
            root: None,
            profile: "",
            changed_docs: vec!["docs/process/example.md".to_string()],
            fastcheck_rows: 0,
            protocol_coverage_rows: 1,
            readiness_rows: 0,
            doctor_error_rows: 0,
            doctor_warning_rows: 0,
        });

        let plain = render_docflow_closeout_verdict("closeout", &verdict, "toon", true);
        assert!(plain.contains("closeout\n  mode: changed"));
        assert!(plain.contains("  protocol_coverage_rows: 1"));
        assert!(plain.contains("  blocker_codes: docflow_protocol_coverage_blocking"));
        assert!(!plain.contains("docs/process/example.md"));

        let json = render_docflow_closeout_verdict("closeout", &verdict, "json", true);
        let payload: serde_json::Value =
            serde_json::from_str(&json).expect("closeout json should parse");
        assert_eq!(payload["command"], "closeout");
        assert_eq!(payload["changed_docs"].as_array().unwrap().len(), 0);
        assert_eq!(
            payload["blocker_codes"][0],
            "docflow_protocol_coverage_blocking"
        );
    }

    #[test]
    fn closeout_renderer_and_error_surface_preserve_noncompact_and_blocking_fields() {
        let verdict = build_docflow_closeout_verdict(DocflowCloseoutVerdictInput {
            command: "docflow closeout",
            mode: "changed",
            task_id: Some("task-42"),
            root: None,
            profile: "",
            changed_docs: vec!["docs/a.md".to_string(), "docs/b.md".to_string()],
            fastcheck_rows: 1,
            protocol_coverage_rows: 1,
            readiness_rows: 1,
            doctor_error_rows: 0,
            doctor_warning_rows: 0,
        });

        let plain = render_docflow_closeout_verdict("closeout", &verdict, "toon", false);
        assert!(plain.contains("  changed_docs:\n"));
        assert!(plain.contains("  next_actions:\n"));
        assert!(plain.contains("    - docs/a.md"));
        assert!(plain.contains("    - docs/b.md"));

        let jsonl = render_docflow_closeout_verdict("closeout", &verdict, "jsonl", false);
        let jsonl_payload: serde_json::Value =
            serde_json::from_str(jsonl.trim()).expect("closeout jsonl should parse");
        assert_eq!(jsonl_payload["task_id"], "task-42");
        assert_eq!(jsonl_payload["changed_docs"].as_array().unwrap().len(), 2);

        let error_json = render_docflow_closeout_error(
            "closeout",
            "json",
            Some("task-42"),
            "missing root".to_string(),
        );
        let error_payload: serde_json::Value =
            serde_json::from_str(&error_json).expect("closeout error json should parse");
        assert_eq!(error_payload["verdict"], "blocking");
        assert_eq!(error_payload["task_close_allowed"], false);
        assert_eq!(error_payload["blocker_codes"][0], "docflow_closeout_failed");
        assert_eq!(error_payload["task_id"], "task-42");
        assert_eq!(error_payload["command"], "closeout");
        assert_eq!(error_payload["error"], "missing root");
    }

    #[test]
    fn closeout_renderer_defaults_unknown_format_to_plain_and_expands_json_docs() {
        let verdict = build_docflow_closeout_verdict(DocflowCloseoutVerdictInput {
            command: "docflow closeout",
            mode: "changed",
            task_id: Some("task-7"),
            root: None,
            profile: "",
            changed_docs: vec!["docs/process/example.md".to_string()],
            fastcheck_rows: 0,
            protocol_coverage_rows: 0,
            readiness_rows: 0,
            doctor_error_rows: 0,
            doctor_warning_rows: 0,
        });

        let plain = render_docflow_closeout_verdict("closeout", &verdict, "yaml", false);
        assert!(plain.contains("  verdict: ok"));
        assert!(plain.contains("    - docs/process/example.md"));

        let json = render_docflow_closeout_verdict("closeout", &verdict, "json", false);
        let payload: serde_json::Value = serde_json::from_str(&json).expect("json should parse");
        assert_eq!(
            payload["changed_docs"],
            serde_json::json!(["docs/process/example.md"])
        );
    }
}
