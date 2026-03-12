use docflow_contracts::RegistryRow;
use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};
use docflow_markdown::split_footer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub artifact_path: ArtifactPath,
    pub verdict: ReadinessVerdict,
    pub code: String,
    pub message: String,
    pub checked_at: CheckedAt,
}

pub fn validate_markdown_footer(path: ArtifactPath, content: &str) -> Vec<ValidationIssue> {
    match split_footer(content) {
        Ok(artifact) => {
            if artifact.footer.is_some() {
                vec![]
            } else {
                vec![ValidationIssue {
                    artifact_path: path,
                    verdict: ReadinessVerdict::Blocking,
                    code: "missing_footer".into(),
                    message: "canonical markdown artifact is missing footer metadata".into(),
                    checked_at: CheckedAt::now_utc(),
                }]
            }
        }
        Err(error) => vec![ValidationIssue {
            artifact_path: path,
            verdict: ReadinessVerdict::Blocking,
            code: "invalid_footer".into(),
            message: error.to_string(),
            checked_at: CheckedAt::now_utc(),
        }],
    }
}

pub fn validate_registry_rows(rows: &[RegistryRow]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for row in rows {
        if row.artifact_type.trim().is_empty() {
            issues.push(ValidationIssue {
                artifact_path: row.artifact_path.clone(),
                verdict: ReadinessVerdict::Blocking,
                code: "missing_artifact_type".into(),
                message: "registry row is missing artifact_type".into(),
                checked_at: CheckedAt::now_utc(),
            });
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::{validate_markdown_footer, validate_registry_rows};
    use docflow_contracts::RegistryRow;
    use docflow_core::{ArtifactPath, ReadinessVerdict};

    #[test]
    fn markdown_footer_validation_blocks_missing_footer() {
        let issues =
            validate_markdown_footer(ArtifactPath("docs/process/test.md".into()), "# title\n");
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].verdict, ReadinessVerdict::Blocking));
        assert_eq!(issues[0].code, "missing_footer");
    }

    #[test]
    fn markdown_footer_validation_accepts_footered_artifact() {
        let issues = validate_markdown_footer(
            ArtifactPath("docs/process/test.md".into()),
            "# title\n\n-----\nartifact_path: docs/process/test\n",
        );
        assert!(issues.is_empty());
    }

    #[test]
    fn registry_validation_blocks_missing_artifact_type() {
        let issues = validate_registry_rows(&[RegistryRow {
            artifact_path: ArtifactPath("docs/process/test.md".into()),
            artifact_type: "".into(),
        }]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "missing_artifact_type");
    }
}
