use std::path::Path;

use time::format_description::well_known::Rfc3339;

fn safe_dispatch_result_run_id(run_id: &str) -> String {
    let trimmed = run_id.trim();
    if trimmed.is_empty() {
        return "run".to_string();
    }
    let sanitized: String = trimmed
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect();
    if sanitized.chars().any(|ch| ch != '_') {
        sanitized
    } else {
        "run".to_string()
    }
}

pub(crate) fn write_runtime_lane_completion_result(
    state_root: &Path,
    run_id: &str,
    completed_target: &str,
    receipt_id: &str,
    source_dispatch_packet_path: &str,
) -> Result<String, String> {
    write_runtime_lane_completion_result_with_summary(
        state_root,
        run_id,
        completed_target,
        receipt_id,
        source_dispatch_packet_path,
        None,
    )
}

pub(crate) fn runtime_lane_completion_summary_blocker_code(
    completed_target: &str,
    summary: Option<&str>,
) -> Option<String> {
    taskflow_host_bridge::host_bridge_lane_completion_summary_blocker_code(
        completed_target,
        summary,
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        runtime_lane_completion_summary_blocker_code,
        write_runtime_lane_completion_result_with_summary,
    };

    #[test]
    fn summary_classifier_ignores_positive_receipt_blocker_context() {
        let summary = "verifier proof passed focused host-bridge tests and confirmed pending receipt was the only closure blocker";

        assert_eq!(
            runtime_lane_completion_summary_blocker_code("verification", Some(summary)),
            None
        );
    }

    #[test]
    fn summary_classifier_keeps_explicit_blocker_verdicts() {
        let summary = "verdict: blocker; rework required; product implementation evidence missing";

        assert_eq!(
            runtime_lane_completion_summary_blocker_code("verification", Some(summary)),
            Some("verification_rework_required".to_string())
        );
    }

    #[test]
    fn summary_classifier_keeps_strong_negative_completion_evidence() {
        let summary = "implementation evidence missing";

        assert_eq!(
            runtime_lane_completion_summary_blocker_code("coach", Some(summary)),
            Some("coach_rework_required".to_string())
        );
    }

    #[test]
    fn summary_classifier_keeps_blocked_coach_decision() {
        let summary = "coach decision=blocked; scheduledAt missing for non-all-day meeting";

        assert_eq!(
            runtime_lane_completion_summary_blocker_code("coach", Some(summary)),
            Some("coach_rework_required".to_string())
        );
    }

    #[test]
    fn summary_classifier_canonicalizes_coach_lane_target() {
        let summary = "coach decision=blocked; scheduledAt missing for non-all-day meeting";

        assert_eq!(
            runtime_lane_completion_summary_blocker_code("coach_lane", Some(summary)),
            Some("coach_rework_required".to_string())
        );
    }

    #[test]
    fn summary_classifier_preserves_coach_decision_when_target_is_stale() {
        let summary = "coach decision=blocked; scheduledAt missing for non-all-day meeting";

        assert_eq!(
            runtime_lane_completion_summary_blocker_code("implementer", Some(summary)),
            Some("coach_rework_required".to_string())
        );
    }

    #[test]
    fn completion_result_blocks_on_blocked_coach_decision() {
        let state_root =
            std::env::temp_dir().join(format!("vida-lane-completion-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state_root);

        let result_path = write_runtime_lane_completion_result_with_summary(
            &state_root,
            "run-coach",
            "coach",
            "receipt-1",
            "packet.json",
            Some("coach decision=blocked; scheduledAt missing"),
        )
        .expect("completion result should write");
        let body: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&result_path).expect("completion result should be readable"),
        )
        .expect("completion result should decode");

        assert_eq!(body["status"], "blocked");
        assert_eq!(body["execution_state"], "blocked");
        assert_eq!(body["completion_verdict"], "rework_required");
        assert_eq!(body["blocker_code"], "coach_rework_required");
        assert_eq!(body["closure_ready"], false);
        assert_eq!(
            body["summary"],
            "coach decision=blocked; scheduledAt missing"
        );
        assert_eq!(
            body["blocker_details"][0]["message"],
            "coach decision=blocked; scheduledAt missing"
        );
        assert_eq!(body["rework_target"], "developer");
        assert_eq!(body["allowed_next_node"], "developer_rework");

        let _ = fs::remove_dir_all(&state_root);
    }

    #[test]
    fn completion_result_uses_quality_gate_transition_matrix() {
        let cases = [
            ("coach", None, None, None, "next"),
            (
                "coach",
                Some("coach decision=blocked; implementation acceptance gap"),
                Some("coach_rework_required"),
                Some("developer"),
                "developer_rework",
            ),
            ("tester", None, None, None, "next"),
            (
                "tester",
                Some("tester decision=blocked; focused proof failed"),
                Some("verification_rework_required"),
                Some("developer"),
                "developer_rework",
            ),
            ("reviewer", None, None, None, "next"),
            (
                "reviewer",
                Some("reviewer decision=blocked; proof review needs tester rework"),
                Some("review_rework_required"),
                Some("tester"),
                "tester",
            ),
        ];

        for (target, summary, blocker_code, rework_target, allowed_next_node) in cases {
            let state_root = std::env::temp_dir().join(format!(
                "vida-lane-completion-{}-{target}-{}",
                std::process::id(),
                summary.is_some()
            ));
            let _ = fs::remove_dir_all(&state_root);

            let result_path = write_runtime_lane_completion_result_with_summary(
                &state_root,
                &format!("run-{target}-{}", summary.is_some()),
                target,
                "receipt-1",
                "packet.json",
                summary,
            )
            .expect("completion result should write");
            let body: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(&result_path).expect("completion result should be readable"),
            )
            .expect("completion result should decode");

            if let Some(blocker_code) = blocker_code {
                assert_eq!(body["status"], "blocked", "{target}");
                assert_eq!(body["execution_state"], "blocked", "{target}");
                assert_eq!(body["decision"], "rework_required", "{target}");
                assert_eq!(body["verdict"], "rework_required", "{target}");
                assert_eq!(body["completion_verdict"], "rework_required", "{target}");
                assert_eq!(body["blocker_code"], blocker_code, "{target}");
                assert_eq!(
                    body["blocker_codes"],
                    serde_json::json!([blocker_code]),
                    "{target}"
                );
                assert_eq!(
                    body["rework_target"],
                    rework_target.expect("rework target"),
                    "{target}"
                );
            } else {
                assert_eq!(body["status"], "pass", "{target}");
                assert_eq!(body["execution_state"], "executed", "{target}");
                assert_eq!(body["decision"], "approve", "{target}");
                assert_eq!(body["verdict"], "pass", "{target}");
                assert_eq!(body["completion_verdict"], "pass", "{target}");
                assert_eq!(body["blocker_codes"], serde_json::json!([]), "{target}");
                assert_eq!(body["rework_target"], serde_json::Value::Null, "{target}");
            }
            assert_eq!(body["allowed_next_node"], allowed_next_node, "{target}");

            let _ = fs::remove_dir_all(&state_root);
        }
    }
}

pub(crate) fn write_runtime_lane_completion_result_with_summary(
    state_root: &Path,
    run_id: &str,
    completed_target: &str,
    receipt_id: &str,
    source_dispatch_packet_path: &str,
    summary: Option<&str>,
) -> Result<String, String> {
    let result_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-results");
    std::fs::create_dir_all(&result_dir)
        .map_err(|error| format!("Failed to create dispatch-results directory: {error}"))?;
    let safe_run_id = safe_dispatch_result_run_id(run_id);
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let result_path = result_dir.join(format!("{safe_run_id}-{ts}.json"));
    let blocker_code = runtime_lane_completion_summary_blocker_code(completed_target, summary);
    let blocker_codes = blocker_code.iter().cloned().collect::<Vec<_>>();
    let verdict_fields = taskflow_host_bridge::host_bridge_result_verdict_fields_for_gate(
        completed_target,
        &blocker_codes,
        None,
    );
    let execution_state = if blocker_code.is_some() {
        "blocked"
    } else {
        "executed"
    };
    let status = if blocker_code.is_some() {
        "blocked"
    } else {
        "pass"
    };
    let mut body = serde_json::json!({
        "artifact_kind": "runtime_lane_completion_result",
        "status": status,
        "execution_state": execution_state,
        "decision": verdict_fields.decision,
        "verdict": verdict_fields.verdict,
        "blocker_codes": verdict_fields.blocker_codes,
        "rework_target": verdict_fields.rework_target,
        "allowed_next_node": verdict_fields.allowed_next_node,
        "completion_verdict": verdict_fields.verdict,
        "closure_ready": blocker_code.is_none(),
        "run_id": run_id,
        "completed_target": completed_target,
        "completion_receipt_id": receipt_id,
        "source_dispatch_packet_path": source_dispatch_packet_path,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
    });
    if let Some(summary) = summary.map(str::trim).filter(|value| !value.is_empty()) {
        body["summary"] = serde_json::json!(summary);
    }
    if let Some(blocker_code) = blocker_code {
        body["blocker_code"] = serde_json::json!(blocker_code);
        body["blockers"] = body["blocker_codes"].clone();
        if let Some(summary) = summary.map(str::trim).filter(|value| !value.is_empty()) {
            body["blocker_details"] = serde_json::json!([{
                "code": body["blocker_code"].clone(),
                "message": summary,
                "completed_target": completed_target
            }]);
        }
    }
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode lane completion result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write lane completion result: {error}"))?;
    Ok(result_path.display().to_string())
}
