//! Task verification command helpers.

#[must_use]
pub fn normalized_task_verify_evidence(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[must_use]
pub fn append_task_verify_note_with_timestamp(
    existing_notes: Option<&str>,
    source_fixed: bool,
    tests_green: bool,
    proof_blocked: bool,
    proof_blocker: Option<&str>,
    evidence: &[String],
    recorded_at_unix_nanos: i128,
) -> String {
    let mut note = format!(
        "task_partial_verification:\n  recorded_at_unix_nanos: {recorded_at_unix_nanos}\n  source_fixed: {source_fixed}\n  tests_green: {tests_green}\n  proof_blocked: {proof_blocked}",
    );
    if let Some(proof_blocker) = proof_blocker
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        note.push_str("\n  proof_blocker: ");
        note.push_str(proof_blocker);
    }
    if !evidence.is_empty() {
        note.push_str("\n  evidence:");
        for item in evidence {
            note.push_str("\n    - ");
            note.push_str(item);
        }
    }

    match existing_notes
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(existing) => format!("{existing}\n\n{note}"),
        None => note,
    }
}

#[must_use]
pub fn append_task_verify_note(
    existing_notes: Option<&str>,
    source_fixed: bool,
    tests_green: bool,
    proof_blocked: bool,
    proof_blocker: Option<&str>,
    evidence: &[String],
) -> String {
    append_task_verify_note_with_timestamp(
        existing_notes,
        source_fixed,
        tests_green,
        proof_blocked,
        proof_blocker,
        evidence,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
    )
}

#[must_use]
pub fn task_verify_labels(
    source_fixed: bool,
    tests_green: bool,
    proof_blocked: bool,
) -> Vec<String> {
    let mut labels = Vec::new();
    if source_fixed {
        labels.push("source-fixed".to_string());
    }
    if tests_green {
        labels.push("tests-green".to_string());
    }
    if proof_blocked {
        labels.push("proof-blocked-by-runtime".to_string());
    }
    labels
}

#[must_use]
pub fn task_reports_runtime_proof_blocker(labels: &[String], close_reason: Option<&str>) -> bool {
    close_reason
        .map(str::to_ascii_lowercase)
        .is_some_and(|reason| {
            reason.contains("proof blocked by runtime")
                || reason.contains("runtime proof blocker")
                || reason.contains("runtime blocker")
        })
        || labels
            .iter()
            .any(|label| label == "proof-blocked-by-runtime" || label == "runtime-proof-blocked")
}

#[must_use]
pub fn verify_proof_targets_for_empty_existing(
    existing_proof_targets: &[String],
    proof_blocked: bool,
    proof_blocker: Option<&str>,
    evidence: &[String],
) -> Option<Vec<String>> {
    if !proof_blocked || !existing_proof_targets.is_empty() {
        return None;
    }
    let mut proof_targets = Vec::new();
    if evidence.is_empty() {
        if let Some(proof_blocker) = proof_blocker
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            proof_targets.push(proof_blocker.to_string());
        }
    } else {
        proof_targets.extend(evidence.iter().cloned());
    }

    if proof_targets.is_empty() {
        None
    } else {
        Some(proof_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        append_task_verify_note_with_timestamp, normalized_task_verify_evidence,
        task_reports_runtime_proof_blocker, task_verify_labels,
        verify_proof_targets_for_empty_existing,
    };

    #[test]
    fn verify_evidence_trims_and_omits_empty_entries() {
        let evidence = normalized_task_verify_evidence(&[
            " cargo test ".to_string(),
            " ".to_string(),
            "vida task validate-graph".to_string(),
        ]);

        assert_eq!(
            evidence,
            vec![
                "cargo test".to_string(),
                "vida task validate-graph".to_string()
            ]
        );
    }

    #[test]
    fn verify_note_preserves_existing_notes_and_evidence() {
        let note = append_task_verify_note_with_timestamp(
            Some("existing"),
            true,
            false,
            true,
            Some(" runtime proof "),
            &["cargo test".to_string()],
            99,
        );

        assert_eq!(
            note,
            "existing\n\n\
task_partial_verification:\n  recorded_at_unix_nanos: 99\n  source_fixed: true\n  tests_green: false\n  proof_blocked: true\n  proof_blocker: runtime proof\n  evidence:\n    - cargo test"
        );
    }

    #[test]
    fn verify_labels_and_runtime_blocker_detection_match_cli_contract() {
        assert_eq!(
            task_verify_labels(true, true, true),
            vec![
                "source-fixed".to_string(),
                "tests-green".to_string(),
                "proof-blocked-by-runtime".to_string()
            ]
        );
        assert!(task_reports_runtime_proof_blocker(
            &["runtime-proof-blocked".to_string()],
            None,
        ));
        assert!(task_reports_runtime_proof_blocker(
            &[],
            Some("Runtime proof blocker: browser unavailable"),
        ));
    }

    #[test]
    fn proof_targets_only_fill_when_runtime_proof_blocked_and_empty() {
        assert_eq!(
            verify_proof_targets_for_empty_existing(&[], true, Some("browser proof"), &[]),
            Some(vec!["browser proof".to_string()])
        );
        assert_eq!(
            verify_proof_targets_for_empty_existing(&[], true, None, &["cargo test".to_string()]),
            Some(vec!["cargo test".to_string()])
        );
        assert_eq!(
            verify_proof_targets_for_empty_existing(
                &["existing".to_string()],
                true,
                Some("browser proof"),
                &[]
            ),
            None
        );
        assert_eq!(
            verify_proof_targets_for_empty_existing(&[], false, Some("browser proof"), &[]),
            None
        );
    }
}
