use taskflow_core::run_workflow::{
    StatusMappingDecision, status_mapping_corpus, transition_matrix, transition_matrix_mermaid,
};

pub const MODULE: &str = "run_workflow";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunWorkflowProofSummary {
    pub transition_rows: usize,
    pub mermaid_lines: usize,
    pub status_mapping_cases: usize,
    pub unknown_status_blockers: usize,
}

#[must_use]
pub fn run_workflow_proof_summary() -> RunWorkflowProofSummary {
    RunWorkflowProofSummary {
        transition_rows: transition_matrix().len(),
        mermaid_lines: transition_matrix_mermaid().lines().count(),
        status_mapping_cases: status_mapping_corpus().len(),
        unknown_status_blockers: status_mapping_corpus()
            .iter()
            .filter(|case| {
                matches!(
                    case.decision,
                    StatusMappingDecision::Blocked { blocker_code: _ }
                )
            })
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use super::run_workflow_proof_summary;

    #[test]
    fn proof_summary_covers_matrix_diagram_and_status_corpus() {
        let summary = run_workflow_proof_summary();

        assert!(summary.transition_rows >= 8);
        assert!(summary.mermaid_lines >= summary.transition_rows);
        assert!(summary.status_mapping_cases >= 8);
        assert!(summary.unknown_status_blockers >= 1);
    }
}
