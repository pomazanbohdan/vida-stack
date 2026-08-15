use miette::Diagnostic;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error, Diagnostic)]
#[error("{payload_kind} {stage} validation failed at {path}")]
#[diagnostic(code(vida::external_payload::validation_failed), help("{help}"))]
pub struct ExternalPayloadDiagnostic {
    pub payload_kind: String,
    pub stage: String,
    pub path: String,
    pub blocker_code: String,
    pub message: String,
    pub schema_ref: Value,
    help: String,
}

impl ExternalPayloadDiagnostic {
    #[must_use]
    pub fn new(
        payload_kind: impl Into<String>,
        stage: impl Into<String>,
        path: impl Into<String>,
        blocker_code: impl Into<String>,
        message: impl Into<String>,
        schema_ref: Value,
    ) -> Self {
        let blocker_code = blocker_code.into();
        Self {
            payload_kind: payload_kind.into(),
            stage: stage.into(),
            path: path.into(),
            message: message.into(),
            help: validation_help(&blocker_code),
            blocker_code,
            schema_ref,
        }
    }

    #[must_use]
    pub fn primary_help(&self) -> &str {
        &self.help
    }
}

fn validation_help(blocker_code: &str) -> String {
    match blocker_code {
        "external_payload_schema_invalid" => {
            "repair the payload so it matches the published schema".to_string()
        }
        "external_payload_typed_decode_failed" => {
            "repair the payload type shape before domain validation".to_string()
        }
        "external_payload_json_parse_failed" => {
            "emit valid JSON before invoking the runtime boundary".to_string()
        }
        _ => "inspect the validation blocker code and schema reference".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OperatorContractDiagnostic;
    use crate::operator_contracts::{
        OperatorSurfaceVerdict, finalize_release1_operator_surface_verdict_with_status,
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

    #[test]
    fn external_payload_diagnostic_preserves_json_blocker_code() {
        let diagnostic = super::ExternalPayloadDiagnostic::new(
            "command_envelope",
            "schema",
            "$.request_id",
            "external_payload_schema_invalid",
            "request_id is required",
            json!({"schema_id": "vida.command_envelope", "version": 1}),
        );
        let json = serde_json::to_value(&diagnostic).expect("diagnostic should serialize");

        assert_eq!(diagnostic.blocker_code, "external_payload_schema_invalid");
        assert_eq!(
            json["blocker_code"],
            serde_json::json!("external_payload_schema_invalid")
        );
        assert_eq!(
            diagnostic.primary_help(),
            "repair the payload so it matches the published schema"
        );
        assert_eq!(
            diagnostic.schema_ref,
            json!({"schema_id": "vida.command_envelope", "version": 1})
        );
    }

    #[test]
    fn diagnostic_keeps_blocked_pass_status_with_nonempty_blockers() {
        let verdict = OperatorSurfaceVerdict {
            status: "pass".to_string(),
            blocker_codes: vec!["migration_required".to_string()],
            next_actions: vec!["run migration".to_string()],
            artifact_refs: json!({"source": "operator"}),
            shared_fields: json!({}),
            operator_contracts: json!({}),
        };
        let diagnostic = OperatorContractDiagnostic::from_verdict("vida status", &verdict)
            .expect("a pass verdict carrying blockers must remain diagnosable");

        assert_eq!(diagnostic.status, "pass");
        assert_eq!(diagnostic.blocker_codes, vec!["migration_required"]);
        assert_eq!(diagnostic.next_actions, vec!["run migration"]);
        assert_eq!(diagnostic.artifact_refs, json!({"source": "operator"}));
        assert_eq!(diagnostic.primary_help(), "run migration");
    }

    #[test]
    fn diagnostic_trims_surface_label() {
        let verdict = OperatorSurfaceVerdict {
            status: "blocked".to_string(),
            blocker_codes: vec!["migration_required".to_string()],
            next_actions: vec!["run migration".to_string()],
            artifact_refs: json!({}),
            shared_fields: json!({}),
            operator_contracts: json!({}),
        };

        let diagnostic = OperatorContractDiagnostic::from_verdict("  vida status  ", &verdict)
            .expect("blocked verdict should produce a diagnostic");
        assert_eq!(diagnostic.surface, "vida status");
    }

    #[test]
    fn external_payload_diagnostic_converts_and_reports_all_fields() {
        let cases = [
            (
                "external_payload_schema_invalid",
                "repair the payload so it matches the published schema",
            ),
            (
                "external_payload_typed_decode_failed",
                "repair the payload type shape before domain validation",
            ),
            (
                "external_payload_json_parse_failed",
                "emit valid JSON before invoking the runtime boundary",
            ),
            (
                "other_blocker",
                "inspect the validation blocker code and schema reference",
            ),
        ];
        for (blocker_code, help) in cases {
            let diagnostic = super::ExternalPayloadDiagnostic::new(
                String::from("command_envelope"),
                String::from("typed_decode"),
                String::from("$.request_id"),
                String::from(blocker_code),
                String::from("request_id is invalid"),
                json!({"schema_id": "vida.command_envelope", "version": 1}),
            );
            assert_eq!(diagnostic.payload_kind, "command_envelope");
            assert_eq!(diagnostic.stage, "typed_decode");
            assert_eq!(diagnostic.path, "$.request_id");
            assert_eq!(diagnostic.blocker_code, blocker_code);
            assert_eq!(diagnostic.message, "request_id is invalid");
            assert_eq!(diagnostic.primary_help(), help);
        }
    }
}
