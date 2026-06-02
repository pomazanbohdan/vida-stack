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

#[cfg(test)]
mod tests {
    use super::{ArtifactRelation, ArtifactRelationKind, ReadinessRow, RegistryRow, ScanRow};
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
}
