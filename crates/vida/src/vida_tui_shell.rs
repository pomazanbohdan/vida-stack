use ratatui::{
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use serde_json::json;
use vida_contracts::{
    operation_spec, operations, VidaClientKind, VidaCommandEnvelope, VidaIdempotencyKey,
    VidaOperation, VidaProjectId, VidaProjectRef, VidaRequestId, VidaResponseStatus, VidaSessionId,
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};

use crate::vida_client::VidaClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VidaTuiShellSnapshot {
    pub(crate) service_status: String,
    pub(crate) active_session: String,
    pub(crate) project_count: usize,
    pub(crate) project_id: String,
    pub(crate) worktree_environment_id: String,
    pub(crate) wizard_step: String,
    pub(crate) wizard_apply_supported: bool,
    pub(crate) wizard_form_fields: Vec<VidaTuiFormFieldSnapshot>,
    pub(crate) wizard_validation_findings: usize,
    pub(crate) wizard_diff_change_count: usize,
    pub(crate) wizard_disabled_apply_reason: String,
    pub(crate) materialization_safe_updates: usize,
    pub(crate) materialization_manual_conflicts: usize,
    pub(crate) drift_report_only: usize,
    pub(crate) job_status: String,
    pub(crate) event_count: usize,
    pub(crate) receipt_count: usize,
    pub(crate) lifecycle_state: String,
    pub(crate) lifecycle_binary_fingerprint: String,
    pub(crate) orchestration_workspace_owner: String,
    pub(crate) orchestration_parallelism_source: String,
    pub(crate) orchestration_tui_projection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VidaTuiFormFieldSnapshot {
    pub(crate) field_id: String,
    pub(crate) label: String,
    pub(crate) required: bool,
    pub(crate) control: String,
}

impl VidaTuiShellSnapshot {
    pub(crate) fn from_client<C: VidaClient>(client: &C) -> Self {
        let status = client.execute(envelope(operations::SERVICE_STATUS));
        assert_eq!(status.status, VidaResponseStatus::Pass);
        let status = status.result.expect("service status result");

        let project_ref = VidaProjectRef::ProjectId {
            project_id: VidaProjectId("vida-stack".to_string()),
        };
        let project_registry = client.execute(envelope(operations::PROJECT_REGISTRY_LIST));
        assert_eq!(project_registry.status, VidaResponseStatus::Pass);
        let project_registry = project_registry.result.expect("project registry list");
        let project_count = project_registry["projects"]
            .as_array()
            .map_or(0, std::vec::Vec::len);

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
        let wizard_validate = client.execute(envelope_with_project_ref_and_payload(
            operations::WIZARD_SESSION_VALIDATE,
            project_ref.clone(),
            json!({ "inputs": {} }),
        ));
        assert_eq!(wizard_validate.status, VidaResponseStatus::Pass);
        let wizard_validate = wizard_validate.result.expect("wizard validation result");
        let wizard_diff = client.execute(envelope_with_project_ref_and_payload(
            operations::WIZARD_SESSION_DIFF,
            project_ref.clone(),
            json!({
                "current_revision": 2,
                "inputs": {
                    "project_root": "C:/project/vida-stack",
                    "host_system": "codex"
                }
            }),
        ));
        assert_eq!(wizard_diff.status, VidaResponseStatus::Pass);
        let wizard_diff = wizard_diff.result.expect("wizard diff result");

        let materialization = client.execute(envelope_with_project_ref(
            operations::MATERIALIZATION_UPDATE_PLAN,
            project_ref.clone(),
        ));
        assert_eq!(materialization.status, VidaResponseStatus::Pass);
        let materialization = materialization
            .result
            .expect("materialization update plan result");
        let actions = materialization["planned_actions"]
            .as_array()
            .expect("materialization actions");
        let drift = client.execute(envelope_with_project_ref(
            operations::MATERIALIZATION_DRIFT_CLASSIFY,
            project_ref.clone(),
        ));
        assert_eq!(drift.status, VidaResponseStatus::Pass);
        let drift = drift.result.expect("materialization drift result");
        let job = client.execute(envelope(operations::JOBS_GET));
        assert_eq!(job.status, VidaResponseStatus::Pass);
        let job = job.result.expect("job result");
        let events = client.execute(envelope(operations::EVENTS_SINCE));
        assert_eq!(events.status, VidaResponseStatus::Pass);
        let events = events.result.expect("events result");
        let receipts = client.execute(envelope_with_project_ref(
            operations::RECEIPTS_GET,
            project_ref.clone(),
        ));
        assert_eq!(receipts.status, VidaResponseStatus::Pass);
        let receipts = receipts.result.expect("receipts result");
        let lifecycle = client.execute(envelope(operations::SERVICE_LIFECYCLE_STATUS));
        assert_eq!(lifecycle.status, VidaResponseStatus::Pass);
        let lifecycle = lifecycle.result.expect("lifecycle status result");

        let orchestration = client.execute(envelope_with_project_ref(
            operations::ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET,
            project_ref,
        ));
        assert_eq!(orchestration.status, VidaResponseStatus::Pass);
        let orchestration = orchestration
            .result
            .expect("orchestration control-plane summary");

        Self {
            service_status: status["status"].as_str().unwrap_or("unknown").to_string(),
            active_session: status["session"]["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            project_count,
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
            wizard_form_fields: operation_form_fields(&wizard_schema),
            wizard_validation_findings: wizard_validate["validation"]["findings"]
                .as_array()
                .map_or(0, std::vec::Vec::len),
            wizard_diff_change_count: [
                "config_changes",
                "registry_changes",
                "materialization_changes",
                "service_changes",
                "runtime_impacts",
            ]
            .iter()
            .map(|key| {
                wizard_diff["diff_summary"][key]
                    .as_array()
                    .map_or(0, std::vec::Vec::len)
            })
            .sum(),
            wizard_disabled_apply_reason: wizard_diff["disabled_apply_reason"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            materialization_safe_updates: actions
                .iter()
                .filter(|action| action["mode"] == "safe_update")
                .count(),
            materialization_manual_conflicts: actions
                .iter()
                .filter(|action| action["mode"] == "manual_conflict")
                .count(),
            drift_report_only: drift["summary"]["report_only"].as_u64().unwrap_or(0) as usize,
            job_status: job["status"].as_str().unwrap_or("unknown").to_string(),
            event_count: events["events"].as_array().map_or(0, std::vec::Vec::len),
            receipt_count: receipts["receipts"]
                .as_array()
                .map_or(0, std::vec::Vec::len),
            lifecycle_state: lifecycle["lifecycle"]["state"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            lifecycle_binary_fingerprint: lifecycle["binary"]["fingerprint"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            orchestration_workspace_owner: orchestration["workspace_model"]["workspace_owner"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            orchestration_parallelism_source: orchestration["scheduling"]["parallelism_source"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            orchestration_tui_projection: orchestration["observability"]["tui_projection"]
                .as_bool()
                .unwrap_or(false),
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
            "Projects: count={} | active={} | Worktree: {}",
            snapshot.project_count, snapshot.project_id, snapshot.worktree_environment_id
        ))]),
        Line::from(vec![Span::raw(format!(
            "Wizard: step={} | validation_findings={} | diff_changes={} | apply_supported={}",
            snapshot.wizard_step,
            snapshot.wizard_validation_findings,
            snapshot.wizard_diff_change_count,
            snapshot.wizard_apply_supported
        ))]),
        Line::from(vec![Span::raw(format!(
            "Wizard form fields[{}]{{field_id,label,required,control}}: {}",
            snapshot.wizard_form_fields.len(),
            form_field_summary(&snapshot.wizard_form_fields)
        ))]),
        Line::from(vec![Span::raw(format!(
            "Disabled action: {}",
            snapshot.wizard_disabled_apply_reason
        ))]),
        Line::from(vec![Span::raw(format!(
            "Materialization: safe_updates={} | manual_conflicts={} | drift_report_only={}",
            snapshot.materialization_safe_updates,
            snapshot.materialization_manual_conflicts,
            snapshot.drift_report_only
        ))]),
        Line::from(vec![Span::raw(format!(
            "Jobs/Events/Receipts: job_status={} | events={} | receipts={}",
            snapshot.job_status, snapshot.event_count, snapshot.receipt_count
        ))]),
        Line::from(vec![Span::raw(format!(
            "Lifecycle: state={} | binary_fingerprint={}",
            snapshot.lifecycle_state, snapshot.lifecycle_binary_fingerprint
        ))]),
        Line::from(vec![Span::raw(format!(
            "Orchestration: workspace_owner={} | parallelism_source={} | tui_projection={}",
            snapshot.orchestration_workspace_owner,
            snapshot.orchestration_parallelism_source,
            snapshot.orchestration_tui_projection
        ))]),
    ];
    frame.render_widget(Paragraph::new(lines), frame.area());
}

fn operation_form_fields(schema: &serde_json::Value) -> Vec<VidaTuiFormFieldSnapshot> {
    schema["operation_input_schema"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|field| VidaTuiFormFieldSnapshot {
            field_id: field["field_id"].as_str().unwrap_or("").to_string(),
            label: field["label"].as_str().unwrap_or("").to_string(),
            required: field["required"].as_bool().unwrap_or(false),
            control: field["tui_control"].as_str().unwrap_or("").to_string(),
        })
        .collect()
}

fn form_field_summary(fields: &[VidaTuiFormFieldSnapshot]) -> String {
    fields
        .iter()
        .map(|field| {
            format!(
                "{}:{}:{}:{}",
                field.field_id, field.label, field.required, field.control
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("tui-fixture-session".to_string()),
        request_id: VidaRequestId(format!("tui-request-{operation}")),
        command_id: None,
        causation_id: None,
        expected_stream_version: None,
        consistency: None,
        deadline: None,
        client_kind: VidaClientKind::Tui,
        project_ref: None,
        claim_kind: operation_spec(operation).map(|spec| spec.required_claim),
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
