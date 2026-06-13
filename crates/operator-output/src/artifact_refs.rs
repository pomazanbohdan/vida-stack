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
