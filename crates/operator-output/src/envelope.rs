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

#[cfg(test)]
mod tests {
    use super::OperatorEnvelope;
    use crate::artifact_refs::ArtifactRef;

    #[test]
    fn operator_envelope_new_preserves_empty_defaults_and_round_trip() {
        let mut envelope = OperatorEnvelope::new("vida status", "blocked");
        assert_eq!(envelope.surface, "vida status");
        assert_eq!(envelope.status, "blocked");
        assert!(envelope.blocker_codes.is_empty());
        assert!(envelope.next_actions.is_empty());
        assert!(envelope.artifact_refs.is_empty());

        envelope.blocker_codes.push("missing_receipt".to_string());
        envelope.next_actions.push("inspect evidence".to_string());
        envelope
            .artifact_refs
            .push(ArtifactRef::new("proof", ".vida/proof.json"));
        let encoded = serde_json::to_value(&envelope).expect("envelope serializes");
        let decoded: OperatorEnvelope =
            serde_json::from_value(encoded).expect("envelope deserializes");
        assert_eq!(decoded, envelope);
    }
}
