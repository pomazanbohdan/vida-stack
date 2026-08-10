pub mod artifact_refs;
pub mod command_text;
pub mod diagnostics;
pub mod envelope;
pub mod next_actions;
pub mod operator_contracts;
pub mod toon_report;

#[cfg(test)]
mod tests {
    #[test]
    fn module_exports_artifact_and_envelope_contracts() {
        let artifact = crate::artifact_refs::ArtifactRef::new("spec", "docs/spec.md");
        assert_eq!(artifact.kind, "spec");
        assert_eq!(artifact.path, "docs/spec.md");

        let envelope = crate::envelope::OperatorEnvelope::new("taskflow", "pass");
        assert_eq!(envelope.surface, "taskflow");
        assert_eq!(envelope.status, "pass");
        assert!(envelope.blocker_codes.is_empty());
        assert!(envelope.next_actions.is_empty());
        assert!(envelope.artifact_refs.is_empty());
    }
}
