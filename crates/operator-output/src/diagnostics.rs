use miette::Diagnostic;
use serde_json::Value;
use thiserror::Error;

use crate::operator_contracts::OperatorSurfaceVerdict;

#[derive(Debug, Clone, PartialEq, Eq, Error, Diagnostic)]
#[error("{surface} returned {status}")]
#[diagnostic(code(vida::operator_output::blocked), help("{help}"))]
pub struct OperatorContractDiagnostic {
    pub surface: String,
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub artifact_refs: Value,
    help: String,
}

impl OperatorContractDiagnostic {
    #[must_use]
    pub fn from_verdict(surface: &str, verdict: &OperatorSurfaceVerdict) -> Option<Self> {
        if verdict.status == "pass" && verdict.blocker_codes.is_empty() {
            return None;
        }
        Some(Self {
            surface: surface.trim().to_string(),
            status: verdict.status.clone(),
            blocker_codes: verdict.blocker_codes.clone(),
            next_actions: verdict.next_actions.clone(),
            artifact_refs: verdict.artifact_refs.clone(),
            help: diagnostic_help(&verdict.next_actions),
        })
    }

    #[must_use]
    pub fn primary_help(&self) -> &str {
        &self.help
    }
}

fn diagnostic_help(next_actions: &[String]) -> String {
    next_actions
        .first()
        .cloned()
        .unwrap_or_else(|| "inspect the operator contract payload for remediation".to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OperatorContractDiagnostic;
    use crate::operator_contracts::{
        finalize_release1_operator_surface_verdict_with_status,
        shared_operator_output_contract_parity_error,
    };

    #[test]
    fn diagnostic_wraps_blocked_operator_contract_without_changing_json_fields() {
        let verdict = finalize_release1_operator_surface_verdict_with_status(
            "blocked",
            vec!["migration_required".to_string()],
            vec!["run migration proof".to_string()],
            json!({"surface": "migration"}),
        )
        .expect("blocked verdict should be valid");

        let diagnostic = OperatorContractDiagnostic::from_verdict("vida doctor", &verdict)
            .expect("blocked verdict should produce a diagnostic");

        assert_eq!(diagnostic.surface, "vida doctor");
        assert_eq!(diagnostic.status, "blocked");
        assert_eq!(diagnostic.blocker_codes, vec!["migration_required"]);
        assert_eq!(diagnostic.next_actions, vec!["run migration proof"]);
        assert_eq!(diagnostic.primary_help(), "run migration proof");
        assert_eq!(
            verdict.operator_contracts["blocker_codes"],
            json!(["migration_required"])
        );
        assert_eq!(
            shared_operator_output_contract_parity_error(&json!({
                "status": verdict.status,
                "blocker_codes": verdict.blocker_codes,
                "next_actions": verdict.next_actions,
                "shared_fields": verdict.shared_fields,
                "operator_contracts": verdict.operator_contracts,
            })),
            None
        );
    }

    #[test]
    fn diagnostic_is_absent_for_pass_verdicts() {
        let verdict = finalize_release1_operator_surface_verdict_with_status(
            "pass",
            vec![],
            vec![],
            json!({}),
        )
        .expect("pass verdict should be valid");

        assert!(OperatorContractDiagnostic::from_verdict("vida status", &verdict).is_none());
    }
}
