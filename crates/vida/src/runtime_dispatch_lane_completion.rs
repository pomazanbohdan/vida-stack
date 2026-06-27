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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        write_runtime_lane_completion_result_with_summary,
        write_runtime_lane_completion_result_with_summary_and_next,
    };

    #[test]
    fn completion_result_treats_summary_prose_as_advisory_only() {
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

        assert_eq!(body["status"], "pass");
        assert_eq!(body["execution_state"], "executed");
        assert_eq!(body["completion_verdict"], "pass");
        assert_eq!(body["blocker_codes"], serde_json::json!([]));
        assert_eq!(body["closure_ready"], true);
        assert_eq!(body["summary_classifier_source"], "advisory_only");
        assert_eq!(body["host_bridge_completion_authority"]["accepted"], true);
        assert_eq!(
            body["summary"],
            "coach decision=blocked; scheduledAt missing"
        );
        assert!(body.get("blocker_code").is_none());
        assert!(body.get("blocker_details").is_none());
        assert_eq!(body["rework_target"], serde_json::Value::Null);
        assert_eq!(body["allowed_next_node"], "closure");

        let _ = fs::remove_dir_all(&state_root);
    }

    #[test]
    fn completion_result_does_not_route_quality_gates_from_summary_prose() {
        let cases = [
            ("coach", None, "closure"),
            (
                "coach",
                Some("coach decision=blocked; implementation acceptance gap"),
                "closure",
            ),
            ("tester", None, "closure"),
            (
                "tester",
                Some("tester decision=blocked; focused proof failed"),
                "closure",
            ),
            ("reviewer", None, "closure"),
            (
                "reviewer",
                Some("reviewer decision=blocked; proof review needs tester rework"),
                "closure",
            ),
        ];

        for (target, summary, allowed_next_node) in cases {
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

            assert_eq!(body["status"], "pass", "{target}");
            assert_eq!(body["execution_state"], "executed", "{target}");
            assert_eq!(body["decision"], "approve", "{target}");
            assert_eq!(body["verdict"], "pass", "{target}");
            assert_eq!(body["completion_verdict"], "pass", "{target}");
            assert_eq!(body["blocker_codes"], serde_json::json!([]), "{target}");
            assert_eq!(body["rework_target"], serde_json::Value::Null, "{target}");
            assert_eq!(
                body["summary_classifier_source"], "advisory_only",
                "{target}"
            );
            assert_eq!(body["allowed_next_node"], allowed_next_node, "{target}");

            let _ = fs::remove_dir_all(&state_root);
        }
    }
    #[test]
    fn completion_result_preserves_explicit_pass_next_node() {
        let state_root = std::env::temp_dir().join(format!(
            "vida-lane-completion-explicit-next-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&state_root);

        let result_path = write_runtime_lane_completion_result_with_summary_and_next(
            &state_root,
            "run-analyst",
            "analyst",
            "receipt-1",
            "packet.json",
            None,
            Some("developer"),
        )
        .expect("completion result should write");
        let body: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&result_path).expect("completion result should be readable"),
        )
        .expect("completion result should decode");

        assert_eq!(body["status"], "pass");
        assert_eq!(body["allowed_next_node"], "developer");

        let _ = fs::remove_dir_all(&state_root);
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
    write_runtime_lane_completion_result_with_summary_and_next(
        state_root,
        run_id,
        completed_target,
        receipt_id,
        source_dispatch_packet_path,
        summary,
        None,
    )
}

pub(crate) fn write_runtime_lane_completion_result_with_summary_and_next(
    state_root: &Path,
    run_id: &str,
    completed_target: &str,
    receipt_id: &str,
    source_dispatch_packet_path: &str,
    summary: Option<&str>,
    allowed_next_node: Option<&str>,
) -> Result<String, String> {
    write_runtime_lane_completion_result_with_summary_next_and_blockers(
        state_root,
        run_id,
        completed_target,
        receipt_id,
        source_dispatch_packet_path,
        summary,
        allowed_next_node,
        &[],
        None,
    )
}

pub(crate) fn write_runtime_lane_completion_result_with_summary_next_and_blockers(
    state_root: &Path,
    run_id: &str,
    completed_target: &str,
    receipt_id: &str,
    source_dispatch_packet_path: &str,
    summary: Option<&str>,
    allowed_next_node: Option<&str>,
    blocker_codes: &[String],
    rework_target: Option<&str>,
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
    let blocker_codes = blocker_codes
        .iter()
        .map(|blocker| blocker.trim())
        .filter(|blocker| !blocker.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let pass_allowed_next_node = blocker_codes
        .is_empty()
        .then(|| {
            allowed_next_node
                .map(str::trim)
                .filter(|target| !target.is_empty())
        })
        .flatten();
    let authority_decision = taskflow_host_bridge::decide_host_bridge_completion_authority(
        taskflow_host_bridge::HostBridgeCompletionAuthorityInput {
            decision: "approve".to_string(),
            verdict: if blocker_codes.is_empty() {
                "pass"
            } else {
                "blocked"
            }
            .to_string(),
            blocker_codes: blocker_codes.clone(),
            summary: summary
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            provenance_valid: true,
            receipt_bound: true,
            next_step_packet_requested: pass_allowed_next_node.is_some(),
        },
    );
    let verdict_fields = if blocker_codes.is_empty() {
        taskflow_host_bridge::host_bridge_result_verdict_fields_for_gate(
            completed_target,
            &blocker_codes,
            pass_allowed_next_node,
        )
    } else {
        taskflow_host_bridge::host_bridge_result_verdict_fields_for_gate(
            completed_target,
            &blocker_codes,
            rework_target,
        )
    };
    let execution_state = if blocker_codes.is_empty() {
        "executed"
    } else {
        "blocked"
    };
    let status = if blocker_codes.is_empty() {
        "pass"
    } else {
        "blocked"
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
        "closure_ready": blocker_codes.is_empty(),
        "summary_classifier_source": "advisory_only",
        "host_bridge_completion_authority": {
            "accepted": authority_decision.accepted,
            "final_state": format!("{:?}", authority_decision.final_state),
            "events": authority_decision.events.iter().map(|event| format!("{event:?}")).collect::<Vec<_>>(),
            "effect_intents": authority_decision.effect_intents.iter().map(|intent| format!("{intent:?}")).collect::<Vec<_>>(),
            "next_step_packet_admitted": authority_decision.next_step_packet_admitted
        },
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
    if let Some(blocker_code) = blocker_codes.first() {
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
