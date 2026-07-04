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
    fn completion_result_blocks_negative_summary_prose() {
        let state_root =
            std::env::temp_dir().join(format!("vida-lane-completion-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state_root);

        let result_path = write_runtime_lane_completion_result_with_summary(
            &state_root,
            "run-alpha-gate",
            "alpha_gate",
            "receipt-1",
            "packet.json",
            Some("alpha_gate decision=blocked; rework required; implementation evidence missing"),
        )
        .expect("completion result should write");
        let body: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&result_path).expect("completion result should be readable"),
        )
        .expect("completion result should decode");

        assert_eq!(body["status"], "blocked");
        assert_eq!(body["execution_state"], "blocked");
        assert_eq!(body["decision"], "rework_required");
        assert_eq!(body["verdict"], "rework_required");
        assert_eq!(body["completion_verdict"], "rework_required");
        assert_eq!(
            body["blocker_codes"],
            serde_json::json!(["host_bridge_completion_summary_blocked"])
        );
        assert_eq!(body["closure_ready"], false);
        assert_eq!(
            body["summary_classifier_source"],
            "typed_and_summary_blockers"
        );
        assert_eq!(body["host_bridge_completion_authority"]["accepted"], false);
        assert_eq!(
            body["summary"],
            "alpha_gate decision=blocked; rework required; implementation evidence missing"
        );
        assert_eq!(
            body["blocker_code"],
            "host_bridge_completion_summary_blocked"
        );
        assert_eq!(body["rework_target"], "alpha_gate");
        assert_eq!(body["allowed_next_node"], "alpha_gate");

        let _ = fs::remove_dir_all(&state_root);
    }

    #[test]
    fn completion_result_routes_negative_summary_prose_to_blocked_gate() {
        let cases = [
            (
                "alpha_gate",
                None,
                "pass",
                "executed",
                "closure",
                "__null__",
            ),
            (
                "alpha_gate",
                Some("alpha_gate decision=blocked; rework required"),
                "blocked",
                "blocked",
                "alpha_gate",
                "alpha_gate",
            ),
            ("beta_gate", None, "pass", "executed", "closure", "__null__"),
            (
                "beta_gate",
                Some("beta_gate decision=blocked; focused proof failed; completion failed"),
                "blocked",
                "blocked",
                "beta_gate",
                "beta_gate",
            ),
            (
                "gamma_gate",
                None,
                "pass",
                "executed",
                "closure",
                "__null__",
            ),
            (
                "gamma_gate",
                Some("gamma_gate decision=blocked; proof review needs rework; rework required"),
                "blocked",
                "blocked",
                "gamma_gate",
                "gamma_gate",
            ),
        ];

        for (
            target,
            summary,
            expected_status,
            expected_execution_state,
            allowed_next_node,
            expected_rework_target,
        ) in cases
        {
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

            assert_eq!(body["status"], expected_status, "{target}");
            assert_eq!(
                body["execution_state"], expected_execution_state,
                "{target}"
            );
            if expected_status == "pass" {
                assert_eq!(body["decision"], "approve", "{target}");
                assert_eq!(body["verdict"], "pass", "{target}");
                assert_eq!(body["completion_verdict"], "pass", "{target}");
                assert_eq!(body["blocker_codes"], serde_json::json!([]), "{target}");
                assert_eq!(body["rework_target"], serde_json::Value::Null, "{target}");
                assert_eq!(body["allowed_next_node"], allowed_next_node, "{target}");
            } else {
                assert_eq!(body["decision"], "rework_required", "{target}");
                assert_eq!(body["verdict"], "rework_required", "{target}");
                assert_eq!(body["completion_verdict"], "rework_required", "{target}");
                assert_eq!(
                    body["blocker_codes"],
                    serde_json::json!(["host_bridge_completion_summary_blocked"]),
                    "{target}"
                );
                assert_eq!(body["rework_target"], expected_rework_target, "{target}");
                assert_eq!(body["allowed_next_node"], allowed_next_node, "{target}");
            }
            assert_eq!(
                body["summary_classifier_source"], "typed_and_summary_blockers",
                "{target}"
            );

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
            "run-alpha-intake",
            "alpha_intake",
            "receipt-1",
            "packet.json",
            None,
            Some("beta_impl"),
        )
        .expect("completion result should write");
        let body: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&result_path).expect("completion result should be readable"),
        )
        .expect("completion result should decode");

        assert_eq!(body["status"], "pass");
        assert_eq!(body["allowed_next_node"], "beta_impl");

        let _ = fs::remove_dir_all(&state_root);
    }

    #[test]
    fn completion_result_ignores_advisory_summary_for_explicit_pass_contract() {
        let state_root = std::env::temp_dir().join(format!(
            "vida-lane-completion-explicit-pass-summary-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&state_root);

        let result_path =
            super::write_runtime_lane_completion_result_with_summary_next_and_blockers(
                &state_root,
                "activity-meeting-event-form-fields",
                "alpha_intake",
                "receipt-1",
                "packet.json",
                Some("verdict: blocker; rework required text is advisory for explicit pass"),
                false,
                Some("beta_design"),
                &[],
                None,
            )
            .expect("completion result should write");
        let body: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&result_path).expect("completion result should be readable"),
        )
        .expect("completion result should decode");

        assert_eq!(body["status"], "pass");
        assert_eq!(body["decision"], "approve");
        assert_eq!(body["verdict"], "pass");
        assert_eq!(body["blocker_codes"], serde_json::json!([]));
        assert_eq!(body["allowed_next_node"], "beta_design");
        assert_eq!(body["summary_classifier_source"], "typed_blockers_only");

        let _ = fs::remove_dir_all(&state_root);
    }

    #[test]
    fn completion_result_separates_typed_blockers_from_summary_derivation() {
        let state_root = std::env::temp_dir().join(format!(
            "vida-lane-completion-summary-derivation-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&state_root);

        let pass_path = super::write_runtime_lane_completion_result_with_summary_next_and_blockers(
            &state_root,
            "run-alpha-verifier-pass",
            "alpha_verifier",
            "receipt-pass",
            "packet.json",
            Some("verdict: blocked; rework required text is advisory-only"),
            false,
            Some("beta_impl"),
            &[],
            None,
        )
        .expect("advisory summary should not block explicit typed pass");
        let pass: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&pass_path).expect("pass result should be readable"),
        )
        .expect("pass result should decode");

        assert_eq!(pass["status"], "pass");
        assert_eq!(pass["blocker_codes"], serde_json::json!([]));
        assert_eq!(pass["allowed_next_node"], "beta_impl");
        assert_eq!(pass["summary_classifier_source"], "typed_blockers_only");
        assert_eq!(pass["host_bridge_completion_authority"]["accepted"], true);
        assert_eq!(
            pass["host_bridge_completion_authority"]["final_state"],
            "Passed"
        );
        assert_eq!(
            pass["host_bridge_completion_authority"]["effect_intents"],
            serde_json::json!(["CommitEvidence", "PlanNextStepPacket"])
        );

        let typed_blockers = vec!["host_bridge_completion_result_blocked".to_string()];
        let blocked_path =
            super::write_runtime_lane_completion_result_with_summary_next_and_blockers(
                &state_root,
                "run-alpha-verifier-blocked",
                "alpha_verifier",
                "receipt-blocked",
                "packet.json",
                Some("summary says pass, but typed blocker is authoritative"),
                false,
                Some("beta_impl"),
                &typed_blockers,
                Some("beta_impl"),
            )
            .expect("typed blocker should produce blocked completion");
        let blocked: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&blocked_path).expect("blocked result should be readable"),
        )
        .expect("blocked result should decode");

        assert_eq!(blocked["status"], "blocked");
        assert_eq!(blocked["blocker_codes"], serde_json::json!(typed_blockers));
        assert_eq!(blocked["allowed_next_node"], "beta_impl");
        assert_eq!(blocked["summary_classifier_source"], "typed_blockers_only");
        assert_eq!(
            blocked["host_bridge_completion_authority"]["accepted"],
            false
        );
        assert_eq!(
            blocked["host_bridge_completion_authority"]["final_state"],
            "Blocked"
        );
        assert_eq!(
            blocked["host_bridge_completion_authority"]["effect_intents"],
            serde_json::json!(["RecordBlocker"])
        );

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
        true,
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
    derive_summary_blockers: bool,
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
            summary: derive_summary_blockers
                .then_some(summary)
                .flatten()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            provenance_valid: true,
            receipt_bound: true,
            next_step_packet_requested: pass_allowed_next_node.is_some(),
        },
    );
    let blocker_codes = authority_decision.blocker_codes.clone();
    let verdict_fields = taskflow_host_bridge::host_bridge_result_verdict_fields_for_gate_and_next(
        completed_target,
        &blocker_codes,
        rework_target,
        allowed_next_node,
    );
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
        "summary_classifier_source": if derive_summary_blockers { "typed_and_summary_blockers" } else { "typed_blockers_only" },
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
