use ratatui::{
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use serde_json::json;
use vida_contracts::{
    operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaIdempotencyKey,
    VidaOperation, VidaProjectId, VidaProjectRef, VidaRequestId, VidaResponseStatus, VidaSessionId,
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};

use crate::vida_client::VidaClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VidaTuiShellSnapshot {
    pub(crate) service_status: String,
    pub(crate) active_session: String,
    pub(crate) project_id: String,
    pub(crate) worktree_environment_id: String,
    pub(crate) wizard_step: String,
    pub(crate) wizard_apply_supported: bool,
    pub(crate) materialization_safe_updates: usize,
    pub(crate) materialization_manual_conflicts: usize,
}

impl VidaTuiShellSnapshot {
    pub(crate) fn from_client<C: VidaClient>(client: &C) -> Self {
        let status = client.execute(envelope(operations::SERVICE_STATUS));
        assert_eq!(status.status, VidaResponseStatus::Pass);
        let status = status.result.expect("service status result");

        let project_ref = VidaProjectRef::ProjectId {
            project_id: VidaProjectId("vida-stack".to_string()),
        };
        let project_status = client.execute(envelope_with_project_ref(
            operations::PROJECT_STATUS,
            project_ref.clone(),
        ));
        assert_eq!(project_status.status, VidaResponseStatus::Pass);
        let project_status = project_status.result.expect("project status result");

        let wizard_schema = client.execute(envelope_with_project_ref_and_payload(
            operations::WIZARD_SCHEMA_GET,
            project_ref.clone(),
            json!({ "wizard_kind": "project_init" }),
        ));
        assert_eq!(wizard_schema.status, VidaResponseStatus::Pass);
        let wizard_schema = wizard_schema.result.expect("wizard schema result");

        let materialization = client.execute(envelope_with_project_ref(
            operations::MATERIALIZATION_UPDATE_PLAN,
            project_ref,
        ));
        assert_eq!(materialization.status, VidaResponseStatus::Pass);
        let materialization = materialization
            .result
            .expect("materialization update plan result");
        let actions = materialization["planned_actions"]
            .as_array()
            .expect("materialization actions");

        Self {
            service_status: status["status"].as_str().unwrap_or("unknown").to_string(),
            active_session: status["session"]["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            project_id: project_status["project_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            worktree_environment_id: project_status["worktree_environment_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            wizard_step: wizard_schema["current_step"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            wizard_apply_supported: wizard_schema["apply_supported"].as_bool().unwrap_or(false),
            materialization_safe_updates: actions
                .iter()
                .filter(|action| action["mode"] == "safe_update")
                .count(),
            materialization_manual_conflicts: actions
                .iter()
                .filter(|action| action["mode"] == "manual_conflict")
                .count(),
        }
    }
}

pub(crate) fn render_app_shell(frame: &mut Frame<'_>, snapshot: &VidaTuiShellSnapshot) {
    let lines = vec![
        Line::from(vec![Span::raw("VIDA Operator Console")]),
        Line::from(vec![Span::raw(format!(
            "Service: {} | Session: {}",
            snapshot.service_status, snapshot.active_session
        ))]),
        Line::from(vec![Span::raw(format!(
            "Project: {} | Worktree: {}",
            snapshot.project_id, snapshot.worktree_environment_id
        ))]),
        Line::from(vec![Span::raw(format!(
            "Wizard: step={} | apply_supported={}",
            snapshot.wizard_step, snapshot.wizard_apply_supported
        ))]),
        Line::from(vec![Span::raw(format!(
            "Materialization: safe_updates={} | manual_conflicts={}",
            snapshot.materialization_safe_updates, snapshot.materialization_manual_conflicts
        ))]),
    ];
    frame.render_widget(Paragraph::new(lines), frame.area());
}

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("tui-fixture-session".to_string()),
        request_id: VidaRequestId(format!("tui-request-{operation}")),
        client_kind: VidaClientKind::Tui,
        project_ref: None,
        claim_kind: Some(VidaClaimKind::Observe),
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("tui-idem-{operation}"))),
        apply_token: None,
    }
}

fn envelope_with_project_ref(operation: &str, project_ref: VidaProjectRef) -> VidaCommandEnvelope {
    let mut envelope = envelope(operation);
    envelope.project_ref = Some(project_ref);
    envelope
}

fn envelope_with_project_ref_and_payload(
    operation: &str,
    project_ref: VidaProjectRef,
    payload: serde_json::Value,
) -> VidaCommandEnvelope {
    let mut envelope = envelope_with_project_ref(operation, project_ref);
    envelope.payload = payload;
    envelope
}
