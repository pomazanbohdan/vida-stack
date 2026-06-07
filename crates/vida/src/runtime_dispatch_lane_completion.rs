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
    let normalized = summary?.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let classifier_text = completion_summary_classifier_text(&normalized);
    let has_explicit_blocker_verdict = [
        "verdict: blocker",
        "verdict=blocker",
        "verdict: blocked",
        "verdict=blocked",
        "verdict: rework_required",
        "verdict=rework_required",
        "completion_verdict: blocker",
        "completion_verdict=blocker",
        "completion_verdict: blocked",
        "completion_verdict=blocked",
        "completion_verdict: rework_required",
        "completion_verdict=rework_required",
        "status: blocked",
        "status=blocked",
        "blocker: true",
        "blocked: true",
    ]
    .iter()
    .any(|needle| normalized.contains(needle));
    let has_negative_completion_semantics = [
        "not closure-ready",
        "not closure ready",
        "not approve",
        "not approved",
        "not closure_ready",
        "rework",
        "review_findings",
        "changed_scope",
        "implementation evidence absent",
        "implementation evidence missing",
        "product implementation evidence absent",
        "product implementation evidence missing",
        "not ready for closure",
        "closure not ready",
    ]
    .iter()
    .any(|needle| classifier_text.contains(needle));

    let has_explicit_blocker = has_explicit_blocker_verdict || has_negative_completion_semantics;
    if !has_explicit_blocker {
        return None;
    }

    let only_positive_blocker_context = [
        "no blocker",
        "no blockers",
        "without blockers",
        "blockers: []",
        "blocker_codes: []",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
        && ![
            "not closure-ready",
            "not closure ready",
            "not approve",
            "not approved",
            "rework",
            "review_findings",
            "changed_scope",
            "implementation evidence absent",
            "implementation evidence missing",
        ]
        .iter()
        .any(|needle| normalized.contains(needle));

    if only_positive_blocker_context {
        return None;
    }

    Some(
        match completed_target.trim() {
            "verification" => "verification_rework_required",
            "coach" => "coach_rework_required",
            "closure" => "closure_evidence_blocked",
            _ => "lane_completion_blocked_by_summary",
        }
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::runtime_lane_completion_summary_blocker_code;

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
}

fn completion_summary_classifier_text(normalized_summary: &str) -> String {
    [
        "blocker_codes",
        "blocker code",
        "blocker codes",
        "blocker_code",
        "blockers field",
        "blockers array",
    ]
    .iter()
    .fold(normalized_summary.to_string(), |text, field_name| {
        text.replace(field_name, " ")
    })
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
        body["blockers"] = serde_json::json!([body["blocker_code"].clone()]);
        body["closure_ready"] = serde_json::json!(false);
        body["completion_verdict"] = serde_json::json!("rework_required");
    }
    let encoded = serde_json::to_string_pretty(&body)
        .map_err(|error| format!("Failed to encode lane completion result: {error}"))?;
    std::fs::write(&result_path, encoded)
        .map_err(|error| format!("Failed to write lane completion result: {error}"))?;
    Ok(result_path.display().to_string())
}
