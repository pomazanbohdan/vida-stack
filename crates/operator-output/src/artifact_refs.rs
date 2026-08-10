use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub kind: String,
    pub path: String,
}

impl ArtifactRef {
    pub fn new(kind: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_preserves_empty_and_non_empty_artifact_fields() {
        let empty = ArtifactRef::new("", "");
        assert_eq!(empty.kind, "");
        assert_eq!(empty.path, "");

        let populated = ArtifactRef::new(String::from("report"), String::from("docs/report.md"));
        assert_eq!(populated.kind, "report");
        assert_eq!(populated.path, "docs/report.md");
    }
}
