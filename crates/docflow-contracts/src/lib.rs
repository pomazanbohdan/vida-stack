use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRelationKind {
    Owns,
    DependsOn,
    Affects,
    Implements,
    Verifies,
    Documents,
    Supersedes,
    MigratesFrom,
}

impl ArtifactRelationKind {
    pub const ALL: [ArtifactRelationKind; 8] = [
        ArtifactRelationKind::Owns,
        ArtifactRelationKind::DependsOn,
        ArtifactRelationKind::Affects,
        ArtifactRelationKind::Implements,
        ArtifactRelationKind::Verifies,
        ArtifactRelationKind::Documents,
        ArtifactRelationKind::Supersedes,
        ArtifactRelationKind::MigratesFrom,
    ];

    pub fn footer_key(self) -> &'static str {
        match self {
            ArtifactRelationKind::Owns => "owns",
            ArtifactRelationKind::DependsOn => "depends_on",
            ArtifactRelationKind::Affects => "affects",
            ArtifactRelationKind::Implements => "implements",
            ArtifactRelationKind::Verifies => "verifies",
            ArtifactRelationKind::Documents => "documents",
            ArtifactRelationKind::Supersedes => "supersedes",
            ArtifactRelationKind::MigratesFrom => "migrates_from",
        }
    }

    pub fn from_footer_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|relation| relation.footer_key() == key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRelation {
    pub source_artifact: ArtifactPath,
    pub relation_kind: ArtifactRelationKind,
    pub target_artifact: ArtifactPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryRow {
    pub artifact_path: ArtifactPath,
    pub artifact_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRow {
    pub artifact_path: ArtifactPath,
    pub artifact_type: String,
    pub has_footer: bool,
    pub has_changelog: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessRow {
    pub artifact_path: ArtifactPath,
    pub verdict: ReadinessVerdict,
    pub checked_at: CheckedAt,
}

#[derive(Debug, Serialize)]
pub struct DocflowCloseoutVerdict {
    pub command: String,
    pub mode: String,
    pub task_id: Option<String>,
    pub root: String,
    pub profile: String,
    pub changed_doc_count: usize,
    pub changed_docs: Vec<String>,
    pub fastcheck_rows: usize,
    pub protocol_coverage_rows: usize,
    pub readiness_rows: usize,
    pub doctor_error_rows: usize,
    pub doctor_warning_rows: usize,
    pub task_close_allowed: bool,
    pub verdict: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
}

pub struct DocflowCloseoutVerdictInput<'a> {
    pub command: &'a str,
    pub mode: &'a str,
    pub task_id: Option<&'a str>,
    pub root: Option<&'a str>,
    pub profile: &'a str,
    pub changed_docs: Vec<String>,
    pub fastcheck_rows: usize,
    pub protocol_coverage_rows: usize,
    pub readiness_rows: usize,
    pub doctor_error_rows: usize,
    pub doctor_warning_rows: usize,
}

pub fn build_docflow_closeout_verdict(
    input: DocflowCloseoutVerdictInput<'_>,
) -> DocflowCloseoutVerdict {
    let mut blocker_codes = Vec::new();
    if input.changed_docs.is_empty() {
        blocker_codes.push(if input.mode == "task" {
            "missing_docflow_task_evidence".to_string()
        } else {
            "no_changed_docflow_docs".to_string()
        });
    }
    if input.fastcheck_rows > 0 || input.readiness_rows > 0 {
        blocker_codes.push("docflow_check_blocking".to_string());
    }
    if input.protocol_coverage_rows > 0 {
        blocker_codes.push("docflow_protocol_coverage_blocking".to_string());
    }
    if input.doctor_error_rows > 0 {
        blocker_codes.push("docflow_doctor_error".to_string());
    }

    let task_close_allowed = !input.changed_docs.is_empty() && blocker_codes.is_empty();
    let verdict = if task_close_allowed { "ok" } else { "blocking" }.to_string();
    let mut next_actions = Vec::new();
    if task_close_allowed {
        next_actions.push("Continue task closeout with the current DocFlow evidence.".to_string());
    } else if input.changed_docs.is_empty() && input.mode == "task" {
        next_actions.push(
            "Record DocFlow changelog evidence with the active task id before closing the task."
                .to_string(),
        );
        if let Some(task_id) = input.task_id {
            next_actions.push(format!(
                "Inspect task-bound DocFlow history with `docflow task-summary --task-id {task_id}`."
            ));
        }
    } else if input.changed_docs.is_empty() {
        next_actions.push(
            "Change or finalize at least one markdown DocFlow artifact before closeout."
                .to_string(),
        );
    } else {
        next_actions.push(
            "Run `docflow check` and clear blocking DocFlow validation or doctor rows before closing the task."
                .to_string(),
        );
    }

    DocflowCloseoutVerdict {
        command: input.command.to_string(),
        mode: input.mode.to_string(),
        task_id: input.task_id.map(ToString::to_string),
        root: input.root.unwrap_or_default().to_string(),
        profile: input.profile.to_string(),
        changed_doc_count: input.changed_docs.len(),
        changed_docs: input.changed_docs,
        fastcheck_rows: input.fastcheck_rows,
        protocol_coverage_rows: input.protocol_coverage_rows,
        readiness_rows: input.readiness_rows,
        doctor_error_rows: input.doctor_error_rows,
        doctor_warning_rows: input.doctor_warning_rows,
        task_close_allowed,
        verdict,
        blocker_codes,
        next_actions,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactRelation, ArtifactRelationKind, DocflowCloseoutVerdictInput, ReadinessRow,
        RegistryRow, ScanRow, build_docflow_closeout_verdict,
    };
    use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};

    #[test]
    fn artifact_relation_kind_maps_to_footer_keys() {
        assert_eq!(ArtifactRelationKind::Owns.footer_key(), "owns");
        assert_eq!(ArtifactRelationKind::DependsOn.footer_key(), "depends_on");
        assert_eq!(ArtifactRelationKind::Affects.footer_key(), "affects");
        assert_eq!(ArtifactRelationKind::Implements.footer_key(), "implements");
        assert_eq!(ArtifactRelationKind::Verifies.footer_key(), "verifies");
        assert_eq!(ArtifactRelationKind::Documents.footer_key(), "documents");
        assert_eq!(ArtifactRelationKind::Supersedes.footer_key(), "supersedes");
        assert_eq!(
            ArtifactRelationKind::MigratesFrom.footer_key(),
            "migrates_from"
        );
        assert!(matches!(
            ArtifactRelationKind::from_footer_key("verifies"),
            Some(ArtifactRelationKind::Verifies)
        ));
    }

    #[test]
    fn artifact_relation_keeps_source_kind_and_target() {
        let relation = ArtifactRelation {
            source_artifact: ArtifactPath("runtime/protocol".into()),
            relation_kind: ArtifactRelationKind::Documents,
            target_artifact: ArtifactPath("runtime/spec".into()),
        };

        assert_eq!(relation.relation_kind.footer_key(), "documents");
        assert_eq!(relation.source_artifact.0, "runtime/protocol");
        assert_eq!(relation.target_artifact.0, "runtime/spec");
    }

    #[test]
    fn readiness_row_carries_blocking_verdict() {
        let row = ReadinessRow {
            artifact_path: ArtifactPath("product/spec/foo".into()),
            verdict: ReadinessVerdict::Blocking,
            checked_at: CheckedAt::now_utc(),
        };
        assert!(matches!(row.verdict, ReadinessVerdict::Blocking));
    }

    #[test]
    fn registry_row_keeps_artifact_identity() {
        let row = RegistryRow {
            artifact_path: ArtifactPath("product/spec/foo".into()),
            artifact_type: "product_spec".into(),
        };
        assert_eq!(row.artifact_type, "product_spec");
    }

    #[test]
    fn scan_row_keeps_footer_and_changelog_state() {
        let row = ScanRow {
            artifact_path: ArtifactPath("docs/process/foo.md".into()),
            artifact_type: "process_doc".into(),
            has_footer: true,
            has_changelog: false,
        };
        assert!(row.has_footer);
        assert!(!row.has_changelog);
    }

    #[test]
    fn closeout_verdict_blocks_task_without_task_bound_evidence() {
        let verdict = build_docflow_closeout_verdict(DocflowCloseoutVerdictInput {
            command: "docflow closeout",
            mode: "task",
            task_id: Some("TASK-1"),
            root: Some("C:/repo"),
            profile: "",
            changed_docs: Vec::new(),
            fastcheck_rows: 0,
            protocol_coverage_rows: 0,
            readiness_rows: 0,
            doctor_error_rows: 0,
            doctor_warning_rows: 0,
        });

        assert!(!verdict.task_close_allowed);
        assert_eq!(verdict.blocker_codes, ["missing_docflow_task_evidence"]);
        assert!(
            verdict
                .next_actions
                .iter()
                .any(|action| { action.contains("docflow task-summary --task-id TASK-1") })
        );
    }

    #[test]
    fn closeout_verdict_allows_clean_changed_doc_evidence() {
        let verdict = build_docflow_closeout_verdict(DocflowCloseoutVerdictInput {
            command: "docflow closeout",
            mode: "changed",
            task_id: None,
            root: None,
            profile: "",
            changed_docs: vec!["docs/process/example.md".to_string()],
            fastcheck_rows: 0,
            protocol_coverage_rows: 0,
            readiness_rows: 0,
            doctor_error_rows: 0,
            doctor_warning_rows: 0,
        });

        assert!(verdict.task_close_allowed);
        assert!(verdict.blocker_codes.is_empty());
        assert_eq!(verdict.verdict, "ok");
    }
}
