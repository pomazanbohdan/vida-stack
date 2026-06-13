use serde::{Deserialize, Serialize};

use crate::artifact_refs::ArtifactRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorEnvelope {
    pub surface: String,
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub artifact_refs: Vec<ArtifactRef>,
}

impl OperatorEnvelope {
    pub fn new(surface: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            surface: surface.into(),
            status: status.into(),
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
            artifact_refs: Vec::new(),
        }
    }
}
