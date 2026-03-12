use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};
use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::{ReadinessRow, RegistryRow, ScanRow};
    use docflow_core::{ArtifactPath, CheckedAt, ReadinessVerdict};

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
}
