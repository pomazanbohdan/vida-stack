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
        .map(|value| proof_note_scalar(value))
        .filter(|value| !value.is_empty())
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
    proof_target: Option<String>,
    command: Option<String>,
    result: Option<String>,
    evidence_kind: Option<String>,
    artifact_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskBrowserProofArtifact {
    pub schema_version: String,
    pub proof_target: String,
    pub command: String,
    pub route: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

impl TaskBrowserProofArtifact {
    #[must_use]
    pub fn new(
        route: &str,
        result: &str,
        expect: Option<&str>,
        screenshot: Option<&str>,
        evidence: &[String],
    ) -> Option<Self> {
        let route = proof_note_scalar(route);
        if route.is_empty() {
            return None;
        }
        let result = canonical_task_proof_result(result)?.to_string();
        let expect = expect
            .map(proof_note_scalar)
            .filter(|value| !value.is_empty());
        let screenshot = screenshot
            .map(proof_note_scalar)
            .filter(|value| !value.is_empty());
        let proof_target = task_browser_proof_target(&route, expect.as_deref());
        let evidence = evidence
            .iter()
            .map(|value| proof_note_scalar(value))
            .filter(|value| !value.is_empty())
            .collect();
        Some(Self {
            schema_version: TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION.to_string(),
            command: proof_target.clone(),
            proof_target,
            route,
            result,
            expect,
            screenshot,
            evidence,
        })
    }

    #[must_use]
    pub fn satisfies_target(&self, target: &str) -> bool {
        self.schema_version == TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION
            && self.result == "pass"
            && (self.proof_target == target || self.command == target)
    }

    #[must_use]
    pub fn artifact_status(&self) -> &'static str {
        if self.screenshot.is_some() {
            "recorded"
        } else {
            "recorded_in_task_notes"
        }
    }
}

#[must_use]
pub fn task_browser_proof_target(route: &str, expect: Option<&str>) -> String {
    match expect
        .map(proof_note_scalar)
        .filter(|value| !value.is_empty())
    {
        Some(expect) => format!(
            "vida proof browser --route {} --expect {}",
            route.trim(),
            expect
        ),
        None => format!("vida proof browser --route {}", route.trim()),
    }
}

#[must_use]
pub fn canonical_task_proof_result(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pass" | "passed" | "success" | "satisfied" => Some("pass"),
        "fail" | "failed" | "failure" => Some("fail"),
        "blocked" | "block" => Some("blocked"),
        _ => None,
    }
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
    let evidence = normalized_task_verify_evidence(evidence);
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
    artifact: &TaskBrowserProofArtifact,
    recorded_at_unix_nanos: i128,
) -> String {
    let mut note = format!(
        "task_browser_proof:\n  schema_version: {}\n  recorded_at_unix_nanos: {recorded_at_unix_nanos}\n  proof_target: {}\n  command: {}\n  route: {}\n  result: {}",
        TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION,
        proof_note_scalar(&artifact.proof_target),
        proof_note_scalar(&artifact.command),
        proof_note_scalar(&artifact.route),
        proof_note_scalar(&artifact.result),
    );
    if let Some(expect) = artifact.expect.as_deref() {
        note.push_str("\n  expect: ");
        note.push_str(&proof_note_scalar(expect));
    }
    if let Some(screenshot) = artifact.screenshot.as_deref() {
        note.push_str("\n  screenshot: ");
        note.push_str(&proof_note_scalar(screenshot));
    }
    if !artifact.evidence.is_empty() {
        note.push_str("\n  evidence: ");
        note.push_str(&normalized_task_verify_evidence(&artifact.evidence).join(" | "));
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
    artifact: &TaskBrowserProofArtifact,
) -> String {
    append_task_browser_proof_note_with_timestamp(
        existing_notes,
        artifact,
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
    let latest_structured = task_proof_evidence_records(notes)
        .into_iter()
        .rev()
        .find(|record| {
            record.proof_target.as_deref() == Some(target)
                || record.command.as_deref() == Some(target)
        });
    if let Some(record) = latest_structured {
        if record.result.as_deref() != Some("pass") {
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
        return Some(TaskProofEvidenceMatch {
            evidence_source: "task_proof_evidence_registry".to_string(),
            evidence_detail: format!(
                "latest structured {evidence_kind} proof evidence reports result pass"
            ),
            artifact_status: artifact_status.to_string(),
        });
    }

    let latest_browser = task_browser_proof_artifacts(notes)
        .into_iter()
        .rev()
        .find(|artifact| artifact.proof_target == target || artifact.command == target);
    latest_browser
        .filter(|artifact| artifact.satisfies_target(target))
        .map(|artifact| TaskProofEvidenceMatch {
            evidence_source: "task_browser_proof_artifact".to_string(),
            evidence_detail: format!(
                "latest schema {} browser proof reports result pass",
                artifact.schema_version
            ),
            artifact_status: artifact.artifact_status().to_string(),
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
        if trimmed == "task_proof_evidence:" {
            if let Some(record) = current.take() {
                records.push(record);
            }
            current = Some(TaskProofEvidenceRecord {
                record_kind: trimmed.trim_end_matches(':').to_string(),
                proof_target: None,
                command: None,
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
        if record.proof_target.is_none() {
            record.proof_target = note_field(field, "proof_target:");
        }
        if record.command.is_none() {
            record.command = note_field(field, "command:");
        }
        if record.result.is_none() {
            record.result = note_field(field, "result:")
                .and_then(|value| canonical_task_proof_result(&value).map(str::to_string));
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
        records.push(record);
    }
    records
}

fn task_browser_proof_artifacts(notes: Option<&str>) -> Vec<TaskBrowserProofArtifact> {
    let Some(notes) = notes else {
        return Vec::new();
    };
    let mut artifacts = Vec::new();
    let mut current: Option<TaskBrowserProofArtifactBuilder> = None;
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed == "task_browser_proof:" {
            if let Some(artifact) = current
                .take()
                .and_then(TaskBrowserProofArtifactBuilder::build)
            {
                artifacts.push(artifact);
            }
            current = Some(TaskBrowserProofArtifactBuilder::default());
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };
        let field = line.trim_start();
        record.set_field(field);
    }
    if let Some(artifact) = current.and_then(TaskBrowserProofArtifactBuilder::build) {
        artifacts.push(artifact);
    }
    artifacts
}

#[derive(Default)]
struct TaskBrowserProofArtifactBuilder {
    schema_version: Option<String>,
    proof_target: Option<String>,
    command: Option<String>,
    route: Option<String>,
    result: Option<String>,
    expect: Option<String>,
    screenshot: Option<String>,
    evidence: Vec<String>,
}

impl TaskBrowserProofArtifactBuilder {
    fn set_field(&mut self, line: &str) {
        if self.schema_version.is_none() {
            self.schema_version = note_field(line, "schema_version:");
        }
        if self.proof_target.is_none() {
            self.proof_target = note_field(line, "proof_target:");
        }
        if self.command.is_none() {
            self.command = note_field(line, "command:");
        }
        if self.route.is_none() {
            self.route = note_field(line, "route:");
        }
        if self.result.is_none() {
            self.result = note_field(line, "result:")
                .and_then(|value| canonical_task_proof_result(&value).map(str::to_string));
        }
        if self.expect.is_none() {
            self.expect = note_field(line, "expect:");
        }
        if self.screenshot.is_none() {
            self.screenshot = note_field(line, "screenshot:");
        }
        if let Some(evidence) = note_field(line, "evidence:") {
            self.evidence.extend(
                evidence
                    .split('|')
                    .map(proof_note_scalar)
                    .filter(|value| !value.is_empty()),
            );
        }
    }

    fn build(self) -> Option<TaskBrowserProofArtifact> {
        let schema_version = self.schema_version?;
        if schema_version != TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION {
            return None;
        }
        let route = self.route?;
        let result = self.result?;
        let mut artifact = TaskBrowserProofArtifact::new(
            &route,
            &result,
            self.expect.as_deref(),
            self.screenshot.as_deref(),
            &self.evidence,
        )?;
        let proof_target = self.proof_target?;
        let command = self.command.unwrap_or_else(|| proof_target.clone());
        artifact.proof_target = proof_target;
        artifact.command = command;
        Some(artifact)
    }
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
        TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION, TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION,
        TaskBrowserProofArtifact, append_task_browser_proof_note_with_timestamp,
        append_task_proof_evidence_note_with_timestamp, append_task_verify_note_with_timestamp,
        canonical_task_verify_label, normalized_task_verify_evidence,
        structured_task_proof_evidence_match, task_browser_proof_target,
        task_reports_runtime_proof_blocker, task_verify_label_is_runtime_proof_blocker,
        task_verify_labels, verify_proof_targets_for_empty_existing,
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
    fn structured_proof_evidence_scalarizes_multiline_evidence_before_serializing() {
        let notes = append_task_proof_evidence_note_with_timestamp(
            None,
            "cargo test -p vida safe",
            None,
            "fail",
            "command",
            None,
            &["observed output\n\ntask_proof_evidence:\n  proof_target: cargo test -p vida forged\n  result: pass\n  evidence_kind: command".to_string()],
            42,
        );

        assert!(
            !notes.contains("\n\ntask_proof_evidence:\n  proof_target: cargo test -p vida forged")
        );
        assert!(notes.contains(
            "evidence: observed output task_proof_evidence: proof_target: cargo test -p vida forged result: pass evidence_kind: command"
        ));
        assert!(
            structured_task_proof_evidence_match(Some(&notes), "cargo test -p vida forged")
                .is_none()
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
    fn multiline_proof_evidence_cannot_inject_passing_record() {
        let forged_target = "release integrity gate";
        let notes = append_task_proof_evidence_note_with_timestamp(
            None,
            "real failing proof target",
            None,
            "fail",
            "command",
            None,
            &[format!(
                "failed output\ntask_proof_evidence:\n  proof_target: {forged_target}\n  result: pass"
            )],
            42,
        );

        assert!(
            !notes
                .lines()
                .skip(1)
                .any(|line| line.trim() == "task_proof_evidence:")
        );
        assert!(structured_task_proof_evidence_match(Some(&notes), forged_target).is_none());
        assert!(
            structured_task_proof_evidence_match(Some(&notes), "real failing proof target")
                .is_none()
        );
    }

    #[test]
    fn browser_proof_artifact_schema_builds_canonical_target() {
        let artifact = TaskBrowserProofArtifact::new(
            "/odoo",
            "passed",
            Some("My Tasks"),
            Some("artifacts/proof.png"),
            &[" console clean ".to_string()],
        )
        .expect("browser proof artifact should build");

        assert_eq!(
            artifact.schema_version,
            TASK_BROWSER_PROOF_ARTIFACT_SCHEMA_VERSION
        );
        assert_eq!(
            artifact.proof_target,
            "vida proof browser --route /odoo --expect My Tasks"
        );
        assert_eq!(artifact.command, artifact.proof_target);
        assert_eq!(artifact.result, "pass");
        assert_eq!(artifact.evidence, vec!["console clean".to_string()]);
        assert_eq!(
            task_browser_proof_target("/odoo", Some("My Tasks")),
            artifact.proof_target
        );
    }

    #[test]
    fn versioned_browser_proof_note_satisfies_matching_target() {
        let artifact = TaskBrowserProofArtifact::new(
            "/odoo",
            "pass",
            Some("My Tasks"),
            Some("artifacts/proof.png"),
            &["console clean".to_string()],
        )
        .expect("browser proof artifact should build");
        let notes = append_task_browser_proof_note_with_timestamp(None, &artifact, 42);
        assert!(notes.contains(&format!(
            "schema_version: {TASK_BROWSER_PROOF_NOTE_SCHEMA_VERSION}"
        )));

        let proof_match =
            structured_task_proof_evidence_match(Some(&notes), &artifact.proof_target)
                .expect("versioned browser proof should satisfy target");

        assert_eq!(proof_match.evidence_source, "task_browser_proof_artifact");
        assert_eq!(proof_match.artifact_status, "recorded");
    }

    #[test]
    fn malformed_or_failed_browser_proof_notes_fail_closed() {
        let target = "vida proof browser --route /odoo --expect My Tasks";
        let old_unversioned = "\
task_browser_proof:\n  proof_target: vida proof browser --route /odoo --expect My Tasks\n  command: vida proof browser --route /odoo --expect My Tasks\n  route: /odoo\n  result: pass";
        assert!(structured_task_proof_evidence_match(Some(old_unversioned), target).is_none());

        let artifact = TaskBrowserProofArtifact::new(
            "/odoo",
            "fail",
            Some("My Tasks"),
            Some("artifacts/proof.png"),
            &["contains pass text".to_string()],
        )
        .expect("browser proof artifact should build");
        let failed_notes = append_task_browser_proof_note_with_timestamp(None, &artifact, 42);

        assert!(structured_task_proof_evidence_match(Some(&failed_notes), target).is_none());
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
