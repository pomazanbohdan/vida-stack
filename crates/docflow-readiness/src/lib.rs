use docflow_contracts::ReadinessRow;
use docflow_core::{CheckedAt, ReadinessVerdict};
use docflow_validation::ValidationIssue;

pub fn issues_to_readiness_rows(issues: &[ValidationIssue]) -> Vec<ReadinessRow> {
    let mut rows: Vec<ReadinessRow> = issues
        .iter()
        .map(|issue| ReadinessRow {
            artifact_path: issue.artifact_path.clone(),
            verdict: issue.verdict,
            checked_at: CheckedAt::now_utc(),
        })
        .collect();
    rows.sort_by(|left, right| left.artifact_path.0.cmp(&right.artifact_path.0));
    rows
}

pub fn summarize_verdict(rows: &[ReadinessRow]) -> ReadinessVerdict {
    if rows
        .iter()
        .any(|row| matches!(row.verdict, ReadinessVerdict::Blocking))
    {
        ReadinessVerdict::Blocking
    } else if rows
        .iter()
        .any(|row| matches!(row.verdict, ReadinessVerdict::Warning))
    {
        ReadinessVerdict::Warning
    } else {
        ReadinessVerdict::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::{issues_to_readiness_rows, summarize_verdict};
    use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};
    use docflow_validation::ValidationIssue;

    #[test]
    fn readiness_rows_are_sorted_by_artifact_path() {
        let issues = vec![
            ValidationIssue {
                artifact_path: ArtifactPath("docs/product/spec/b.md".into()),
                verdict: ReadinessVerdict::Blocking,
                code: "missing_footer".into(),
                message: "missing".into(),
                checked_at: CheckedAt::now_utc(),
            },
            ValidationIssue {
                artifact_path: ArtifactPath("docs/process/a.md".into()),
                verdict: ReadinessVerdict::Warning,
                code: "warning".into(),
                message: "warn".into(),
                checked_at: CheckedAt::now_utc(),
            },
        ];

        let rows = issues_to_readiness_rows(&issues);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].artifact_path.0, "docs/process/a.md");
        assert_eq!(rows[1].artifact_path.0, "docs/product/spec/b.md");
    }

    #[test]
    fn blocking_verdict_dominates_summary() {
        let rows = vec![
            docflow_contracts::ReadinessRow {
                artifact_path: ArtifactPath("docs/process/a.md".into()),
                verdict: ReadinessVerdict::Warning,
                checked_at: CheckedAt::now_utc(),
            },
            docflow_contracts::ReadinessRow {
                artifact_path: ArtifactPath("docs/process/b.md".into()),
                verdict: ReadinessVerdict::Blocking,
                checked_at: CheckedAt::now_utc(),
            },
        ];
        assert!(matches!(
            summarize_verdict(&rows),
            ReadinessVerdict::Blocking
        ));
    }

    #[test]
    fn ok_summary_when_no_issues_exist() {
        let rows: Vec<docflow_contracts::ReadinessRow> = vec![];
        assert!(matches!(summarize_verdict(&rows), ReadinessVerdict::Ok));
    }
}
