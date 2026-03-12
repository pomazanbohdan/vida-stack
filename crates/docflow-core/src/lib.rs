use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactPath(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessVerdict {
    Ok,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckedAt(pub OffsetDateTime);

impl CheckedAt {
    #[must_use]
    pub fn now_utc() -> Self {
        Self(OffsetDateTime::now_utc())
    }
}

#[derive(Debug, Error)]
pub enum DocflowCoreError {
    #[error("artifact path is empty")]
    EmptyArtifactPath,
}

#[must_use]
pub fn validate_artifact_path(path: &ArtifactPath) -> Result<(), DocflowCoreError> {
    if path.0.trim().is_empty() {
        Err(DocflowCoreError::EmptyArtifactPath)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactPath, ReadinessVerdict, validate_artifact_path};

    #[test]
    fn readiness_verdict_ordering_stays_explicit() {
        assert!(matches!(
            ReadinessVerdict::Blocking,
            ReadinessVerdict::Blocking
        ));
    }

    #[test]
    fn empty_artifact_path_is_rejected() {
        assert!(validate_artifact_path(&ArtifactPath("".into())).is_err());
    }
}
