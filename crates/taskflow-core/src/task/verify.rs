//! Task verification command helpers.

pub const TASK_VERIFY_LABEL_SOURCE_FIXED: &str = "source-fixed";
pub const TASK_VERIFY_LABEL_TESTS_GREEN: &str = "tests-green";
pub const TASK_VERIFY_LABEL_PROOF_BLOCKED_BY_RUNTIME: &str = "proof-blocked-by-runtime";
pub const TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION: &str = "browser_proof_artifact.v1";
pub const TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION: &str = "task_browser_proof.v1";

#[must_use]
pub fn normalized_task_verify_evidence(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProofEvidenceMatch {
    pub evidence_source: String,
    pub evidence_detail: String,
    pub artifact_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskProofEvidenceRecord {
    record_kind: String,
    schema_version: Option<String>,
    proof_target: Option<String>,
    command: Option<String>,
    route: Option<String>,
    result: Option<String>,
    evidence_kind: Option<String>,
    artifact_ref: Option<String>,
}

#[must_use]
pub fn append_task_proof_evidence_note_with_timestamp(
    existing_notes: Option<&str>,
    proof_target: &str,
    command: Option<&str>,
    result: &str,
    evidence_kind: &str,
    artifact_ref: Option<&str>,
    evidence: &[String],
    recorded_at_unix_nanos: i128,
) -> String {
    let proof_target = proof_note_scalar(proof_target);
    let command = command
        .map(proof_note_scalar)
        .filter(|value| !value.is_empty());
    let result = proof_note_scalar(result).to_ascii_lowercase();
    let evidence_kind = proof_note_scalar(evidence_kind);
    let artifact_ref = artifact_ref
        .map(proof_note_scalar)
        .filter(|value| !value.is_empty());
    let evidence = evidence
        .iter()
        .map(|value| proof_note_scalar(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let mut note = format!(
        "task_proof_evidence:\n  recorded_at_unix_nanos: {recorded_at_unix_nanos}\n  proof_target: {proof_target}\n  result: {result}\n  evidence_kind: {evidence_kind}"
    );
    if let Some(command) = command {
        note.push_str("\n  command: ");
        note.push_str(&command);
    }
    if let Some(artifact_ref) = artifact_ref {
        note.push_str("\n  artifact_ref: ");
        note.push_str(&artifact_ref);
    }
    if !evidence.is_empty() {
        note.push_str("\n  evidence: ");
        note.push_str(&evidence.join(" | "));
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
pub fn append_task_proof_evidence_note(
    existing_notes: Option<&str>,
    proof_target: &str,
    command: Option<&str>,
    result: &str,
    evidence_kind: &str,
    artifact_ref: Option<&str>,
    evidence: &[String],
) -> String {
    append_task_proof_evidence_note_with_timestamp(
        existing_notes,
        proof_target,
        command,
        result,
        evidence_kind,
        artifact_ref,
        evidence,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
    )
}

#[must_use]
pub fn append_task_browser_proof_note_with_timestamp(
    existing_notes: Option<&str>,
    proof_target: &str,
    route: &str,
    result: &str,
    expect: Option<&str>,
    screenshot: Option<&str>,
    evidence: &[String],
    recorded_at_unix_nanos: i128,
) -> String {
    let proof_target = proof_note_scalar(proof_target);
    let route = proof_note_scalar(route);
    let result = proof_note_scalar(result).to_ascii_lowercase();
    let mut note = format!(
        "task_browser_proof:\n  schema_version: {TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION}\n  recorded_at_unix_nanos: {recorded_at_unix_nanos}\n  proof_target: {proof_target}\n  command: {proof_target}\n  route: {route}\n  result: {result}\n  evidence_kind: browser"
    );
    if let Some(expect) = expect
        .map(proof_note_scalar)
        .filter(|value| !value.is_empty())
    {
        note.push_str("\n  expect: ");
        note.push_str(&expect);
    }
    if let Some(screenshot) = screenshot
        .map(proof_note_scalar)
        .filter(|value| !value.is_empty())
    {
        note.push_str("\n  artifact_ref: ");
        note.push_str(&screenshot);
        note.push_str("\n  screenshot: ");
        note.push_str(&screenshot);
    }
    let evidence = evidence
        .iter()
        .map(|value| proof_note_scalar(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if !evidence.is_empty() {
        note.push_str("\n  evidence: ");
        note.push_str(&evidence.join(" | "));
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
pub fn append_task_browser_proof_note(
    existing_notes: Option<&str>,
    proof_target: &str,
    route: &str,
    result: &str,
    expect: Option<&str>,
    screenshot: Option<&str>,
    evidence: &[String],
) -> String {
    append_task_browser_proof_note_with_timestamp(
        existing_notes,
        proof_target,
        route,
        result,
        expect,
        screenshot,
        evidence,
        time::OffsetDateTime::now_utc().unix_timestamp_nanos(),
    )
}

#[must_use]
pub fn structured_task_proof_evidence_match(
    notes: Option<&str>,
    target: &str,
) -> Option<TaskProofEvidenceMatch> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    task_proof_evidence_records(notes)
        .into_iter()
        .find_map(|record| {
            let result = record.result.as_deref()?;
            if result != "pass" {
                return None;
            }
            let matches_target = record.proof_target.as_deref() == Some(target)
                || record.command.as_deref() == Some(target);
            if !matches_target {
                return None;
            }
            let evidence_kind = record
                .evidence_kind
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or(record.record_kind.as_str());
            let artifact_status = if record.artifact_ref.is_some() {
                "recorded"
            } else {
                "recorded_in_task_notes"
            };
            Some(TaskProofEvidenceMatch {
                evidence_source: "task_proof_evidence_registry".to_string(),
                evidence_detail: format!(
                    "structured {evidence_kind} proof evidence reports result pass"
                ),
                artifact_status: artifact_status.to_string(),
            })
        })
}

#[must_use]
pub fn all_structured_task_proof_targets_satisfied(
    notes: Option<&str>,
    targets: &[String],
) -> bool {
    !targets.is_empty()
        && targets
            .iter()
            .all(|target| structured_task_proof_evidence_match(notes, target).is_some())
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
        labels.push(TASK_VERIFY_LABEL_SOURCE_FIXED.to_string());
    }
    if tests_green {
        labels.push(TASK_VERIFY_LABEL_TESTS_GREEN.to_string());
    }
    if proof_blocked {
        labels.push(TASK_VERIFY_LABEL_PROOF_BLOCKED_BY_RUNTIME.to_string());
    }
    labels
}

#[must_use]
pub fn canonical_task_verify_label(value: &str) -> Option<&'static str> {
    match value.trim() {
        TASK_VERIFY_LABEL_SOURCE_FIXED => Some(TASK_VERIFY_LABEL_SOURCE_FIXED),
        TASK_VERIFY_LABEL_TESTS_GREEN => Some(TASK_VERIFY_LABEL_TESTS_GREEN),
        TASK_VERIFY_LABEL_PROOF_BLOCKED_BY_RUNTIME | "runtime-proof-blocked" => {
            Some(TASK_VERIFY_LABEL_PROOF_BLOCKED_BY_RUNTIME)
        }
        _ => None,
    }
}

#[must_use]
pub fn task_verify_label_is_runtime_proof_blocker(value: &str) -> bool {
    canonical_task_verify_label(value) == Some(TASK_VERIFY_LABEL_PROOF_BLOCKED_BY_RUNTIME)
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
            .any(|label| task_verify_label_is_runtime_proof_blocker(label))
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

fn task_proof_evidence_records(notes: Option<&str>) -> Vec<TaskProofEvidenceRecord> {
    let Some(notes) = notes else {
        return Vec::new();
    };
    let mut records = Vec::new();
    let mut current: Option<TaskProofEvidenceRecord> = None;
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed == "task_proof_evidence:" || trimmed == "task_browser_proof:" {
            if let Some(record) = current.take() {
                push_task_proof_evidence_record(&mut records, record);
            }
            current = Some(TaskProofEvidenceRecord {
                record_kind: trimmed.trim_end_matches(':').to_string(),
                schema_version: None,
                proof_target: None,
                command: None,
                route: None,
                result: None,
                evidence_kind: None,
                artifact_ref: None,
            });
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };
        let field = line.trim_start();
        if record.schema_version.is_none() {
            record.schema_version = note_field(field, "schema_version:");
        }
        if record.proof_target.is_none() {
            record.proof_target = note_field(field, "proof_target:");
        }
        if record.command.is_none() {
            record.command = note_field(field, "command:");
        }
        if record.route.is_none() {
            record.route = note_field(field, "route:");
        }
        if record.result.is_none() {
            record.result = note_field(field, "result:").map(|value| value.to_ascii_lowercase());
        }
        if record.evidence_kind.is_none() {
            record.evidence_kind = note_field(field, "evidence_kind:");
        }
        if record.artifact_ref.is_none() {
            record.artifact_ref =
                note_field(field, "artifact_ref:").or_else(|| note_field(field, "screenshot:"));
        }
    }
    if let Some(record) = current {
        push_task_proof_evidence_record(&mut records, record);
    }
    records
}

fn push_task_proof_evidence_record(
    records: &mut Vec<TaskProofEvidenceRecord>,
    record: TaskProofEvidenceRecord,
) {
    if record.record_kind == "task_browser_proof" && !task_browser_proof_record_is_valid(&record) {
        return;
    }
    records.push(record);
}

fn task_browser_proof_record_is_valid(record: &TaskProofEvidenceRecord) -> bool {
    record.schema_version.as_deref() == Some(TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION)
        && record
            .proof_target
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && record
            .command
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && record
            .route
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && record
            .result
            .as_deref()
            .is_some_and(|value| matches!(value, "pass" | "fail" | "blocked"))
}

fn note_field(line: &str, prefix: &str) -> Option<String> {
    line.strip_prefix(prefix)
        .map(proof_note_scalar)
        .filter(|value| !value.is_empty())
}

fn proof_note_scalar(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION, append_task_browser_proof_note_with_timestamp,
        append_task_proof_evidence_note_with_timestamp, append_task_verify_note_with_timestamp,
        canonical_task_verify_label, normalized_task_verify_evidence,
        structured_task_proof_evidence_match, task_reports_runtime_proof_blocker,
        task_verify_label_is_runtime_proof_blocker, task_verify_labels,
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
    fn verify_label_canonicalization_accepts_legacy_runtime_blocker_alias() {
        assert_eq!(
            canonical_task_verify_label("runtime-proof-blocked"),
            Some("proof-blocked-by-runtime")
        );
        assert!(task_verify_label_is_runtime_proof_blocker(
            "proof-blocked-by-runtime"
        ));
        assert!(task_verify_label_is_runtime_proof_blocker(
            "runtime-proof-blocked"
        ));
        assert_eq!(canonical_task_verify_label("unknown"), None);
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

    #[test]
    fn structured_proof_evidence_satisfies_exact_target() {
        let notes = append_task_proof_evidence_note_with_timestamp(
            None,
            "cargo test -p vida proof_registry",
            None,
            "pass",
            "command",
            Some("artifacts/proof.json"),
            &["  tests green  ".to_string()],
            42,
        );

        let proof_match =
            structured_task_proof_evidence_match(Some(&notes), "cargo test -p vida proof_registry")
                .expect("structured proof should match");
        assert_eq!(proof_match.evidence_source, "task_proof_evidence_registry");
        assert_eq!(proof_match.artifact_status, "recorded");
        assert!(
            structured_task_proof_evidence_match(Some(&notes), "cargo test -p vida other")
                .is_none()
        );
    }

    #[test]
    fn browser_proof_note_has_schema_and_satisfies_exact_target() {
        let notes = append_task_browser_proof_note_with_timestamp(
            None,
            "vida proof browser --route /secure --expect OK",
            "/secure",
            "pass",
            Some("OK"),
            Some("artifacts/secure.png"),
            &["console clean".to_string()],
            42,
        );

        assert!(notes.contains(&format!(
            "schema_version: {TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION}"
        )));
        let proof_match = structured_task_proof_evidence_match(
            Some(&notes),
            "vida proof browser --route /secure --expect OK",
        )
        .expect("schema-backed browser proof should match");
        assert_eq!(proof_match.evidence_source, "task_proof_evidence_registry");
        assert_eq!(proof_match.artifact_status, "recorded");
        assert!(proof_match.evidence_detail.contains("browser"));
    }

    #[test]
    fn malformed_browser_proof_note_fails_closed() {
        let notes = "task_browser_proof:\n  proof_target: vida proof browser --route /secure\n  command: vida proof browser --route /secure\n  route: /secure\n  result: pass";

        assert!(
            structured_task_proof_evidence_match(Some(notes), "vida proof browser --route /secure")
                .is_none()
        );
    }

    #[test]
    fn browser_proof_note_normalizes_newlines_in_untrusted_fields() {
        let note = append_task_browser_proof_note_with_timestamp(
            None,
            "vida proof browser --route /secure",
            "/secure",
            "fail",
            Some("OK\n  result: pass"),
            Some("artifacts/proof.png\n  result: pass"),
            &["first line\n  result: pass".to_string()],
            42,
        );

        assert!(note.contains("  result: fail\n"));
        assert!(!note.contains("\n  expect: OK\n  result: pass"));
        assert!(!note.contains("\n  screenshot: artifacts/proof.png\n  result: pass"));
        assert!(!note.contains("\n  evidence: first line\n  result: pass"));
    }

    #[test]
    fn close_reason_text_is_not_structured_proof_evidence() {
        let notes = "close_reason: Proof: cargo test -p vida proof_registry passed";

        assert!(
            structured_task_proof_evidence_match(Some(notes), "cargo test -p vida proof_registry")
                .is_none()
        );
    }
}
