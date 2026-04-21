use std::process::ExitCode;

use serde::Serialize;

use crate::contract_profile_adapter::{
    blocker_code_str, canonical_approval_status_str, canonical_gate_level_str,
    render_operator_contract_envelope, BlockerCode,
};
use crate::taskflow_task_bridge::proxy_state_dir;
use crate::{state_store::StateStore, ProxyArgs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApprovalCommand<'a> {
    ShowLatest { as_json: bool },
    ShowRun { run_id: &'a str, as_json: bool },
}

#[derive(Serialize)]
struct ApprovalEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    run_id: String,
    task_id: String,
    approval_scope: String,
    approval_status: String,
    gate_level: String,
    decision_reason: String,
    expiry_state: String,
    approval_evidence_refs: serde_json::Value,
    principal_delegation: crate::state_store::RunGraphPrincipalDelegationProjection,
    memory_governance: crate::state_store::RunGraphMemoryGovernanceProjection,
}

#[derive(Serialize)]
struct BlockedApprovalEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    reason: String,
}

fn parse_approval_args<'a>(args: &'a [String]) -> Result<ApprovalCommand<'a>, String> {
    match args {
        [] => Err("missing approval subcommand".to_string()),
        [flag] if matches!(flag.as_str(), "-h" | "--help") => {
            Err("help requested for root approval surface".to_string())
        }
        [head, rest @ ..] if head == "show" => {
            let mut as_json = false;
            let mut latest = false;
            let mut run_id = None;
            for arg in rest {
                match arg.as_str() {
                    "--json" => as_json = true,
                    "--latest" => latest = true,
                    value if !value.starts_with('-') && run_id.is_none() => run_id = Some(value),
                    _ => return Err("invalid approval show arguments".to_string()),
                }
            }
            if latest {
                if run_id.is_some() {
                    return Err(
                        "approval show --latest cannot be combined with a run id".to_string()
                    );
                }
                return Ok(ApprovalCommand::ShowLatest { as_json });
            }
            let Some(run_id) = run_id else {
                return Err("approval show requires either <run-id> or --latest".to_string());
            };
            Ok(ApprovalCommand::ShowRun { run_id, as_json })
        }
        _ => Err("invalid approval subcommand".to_string()),
    }
}

fn emit_blocked_approval_envelope(as_json: bool, reason: String) -> ExitCode {
    let next_actions = vec![
        "Use `vida approval show --latest --json` or `vida approval show <run-id> --json` once approval evidence exists."
            .to_string(),
    ];
    let operator_contracts = render_operator_contract_envelope(
        "blocked",
        vec!["unsupported_blocker_code".to_string()],
        next_actions.clone(),
        serde_json::json!([]),
    );
    let status = if operator_contracts["status"].as_str() == Some("blocked") {
        "blocked"
    } else {
        "pass"
    };
    let envelope = BlockedApprovalEnvelope {
        surface: "vida approval",
        status,
        trace_id: operator_contracts["trace_id"]
            .as_str()
            .map(ToOwned::to_owned),
        workflow_class: operator_contracts["workflow_class"]
            .as_str()
            .map(ToOwned::to_owned),
        risk_tier: operator_contracts["risk_tier"]
            .as_str()
            .map(ToOwned::to_owned),
        artifact_refs: operator_contracts["artifact_refs"].clone(),
        next_actions,
        blocker_codes: operator_contracts["blocker_codes"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        reason,
    };

    if crate::surface_render::print_surface_json(
        &envelope,
        as_json,
        "blocked approval surface should serialize",
    ) {
        return ExitCode::from(2);
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "blocker_codes",
        &envelope.blocker_codes.join(", "),
    );
    crate::print_surface_line(crate::RenderMode::Plain, "reason", &envelope.reason);
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    ExitCode::from(2)
}

fn approval_scope_from_status(status: &crate::state_store::RunGraphStatus) -> String {
    format!(
        "task_id={} route_task_class={} active_node={} lifecycle_stage={}",
        status.task_id, status.route_task_class, status.active_node, status.lifecycle_stage
    )
}

fn derive_approval_status(
    status: &crate::state_store::RunGraphStatus,
    approval_receipt: Option<&crate::state_store::RunGraphApprovalDelegationReceipt>,
) -> &'static str {
    match approval_receipt.map(|receipt| receipt.transition_kind.as_str()) {
        Some("approval_wait") => {
            canonical_approval_status_str("waiting_for_approval").unwrap_or("waiting_for_approval")
        }
        Some("approval_complete") => {
            canonical_approval_status_str("approved").unwrap_or("approved")
        }
        _ if status.policy_gate
            == canonical_approval_status_str("approval_required")
                .unwrap_or("approval_required") =>
        {
            canonical_approval_status_str("approval_required").unwrap_or("approval_required")
        }
        _ if status.status == "completed" && status.policy_gate == "not_required" => {
            canonical_approval_status_str("approved").unwrap_or("approved")
        }
        _ => {
            canonical_approval_status_str("waiting_for_approval").unwrap_or("waiting_for_approval")
        }
    }
}

fn derive_gate_level(
    approval_status: &str,
    status: &crate::state_store::RunGraphStatus,
) -> &'static str {
    match approval_status {
        "approved" => canonical_gate_level_str("observe").unwrap_or("observe"),
        "denied" | "expired" => canonical_gate_level_str("warn").unwrap_or("warn"),
        _ if status.policy_gate == "not_required" => {
            canonical_gate_level_str("observe").unwrap_or("observe")
        }
        _ => canonical_gate_level_str("block").unwrap_or("block"),
    }
}

fn derive_decision_reason(
    status: &crate::state_store::RunGraphStatus,
    approval_receipt: Option<&crate::state_store::RunGraphApprovalDelegationReceipt>,
) -> String {
    match approval_receipt.map(|receipt| receipt.transition_kind.as_str()) {
        Some("approval_wait") => {
            "route-bound implementation is waiting for explicit approval".to_string()
        }
        Some("approval_complete") => {
            "route-bound implementation approval has already been recorded".to_string()
        }
        _ if status.policy_gate
            == canonical_approval_status_str("approval_required")
                .unwrap_or("approval_required") =>
        {
            "approval is required before the run can complete".to_string()
        }
        _ if status.policy_gate == "not_required" => {
            "approval is not required for this run".to_string()
        }
        _ => "approval state is derived from the latest run-graph status".to_string(),
    }
}

fn derive_expiry_state(
    approval_status: &str,
    status: &crate::state_store::RunGraphStatus,
) -> &'static str {
    match approval_status {
        "approved" if status.policy_gate == "not_required" => "not_applicable",
        "approved" => "not_applicable",
        "denied" | "expired" => "expired",
        _ => "not_tracked",
    }
}

fn build_approval_evidence_refs(
    status: &crate::state_store::RunGraphStatus,
    dispatch_summary: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    approval_receipt: Option<&crate::state_store::RunGraphApprovalDelegationReceipt>,
) -> serde_json::Value {
    serde_json::json!({
        "run_id": status.run_id,
        "task_id": status.task_id,
        "dispatch_run_id": dispatch_summary.map(|summary| summary.run_id.clone()),
        "dispatch_packet_path": dispatch_summary
            .and_then(|summary| summary.dispatch_packet_path.clone()),
        "dispatch_result_path": dispatch_summary
            .and_then(|summary| summary.dispatch_result_path.clone()),
        "dispatch_status": dispatch_summary
            .map(|summary| summary.dispatch_status.clone()),
        "dispatch_target": dispatch_summary
            .map(|summary| summary.dispatch_target.clone()),
        "approval_receipt_id": approval_receipt.map(|receipt| receipt.receipt_id.clone()),
        "approval_transition_kind": approval_receipt
            .map(|receipt| receipt.transition_kind.clone()),
        "approval_recorded_at": approval_receipt.map(|receipt| receipt.recorded_at.clone()),
    })
}

fn build_artifact_refs(
    status: &crate::state_store::RunGraphStatus,
    dispatch_summary: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    approval_receipt: Option<&crate::state_store::RunGraphApprovalDelegationReceipt>,
) -> serde_json::Value {
    serde_json::json!({
        "run_id": status.run_id,
        "task_id": status.task_id,
        "route_task_class": status.route_task_class,
        "approval_delegation_receipt_id": approval_receipt.map(|receipt| receipt.receipt_id.clone()),
        "approval_transition_kind": approval_receipt.map(|receipt| receipt.transition_kind.clone()),
        "dispatch_run_id": dispatch_summary.map(|summary| summary.run_id.clone()),
        "dispatch_packet_path": dispatch_summary
            .and_then(|summary| summary.dispatch_packet_path.clone()),
        "dispatch_result_path": dispatch_summary
            .and_then(|summary| summary.dispatch_result_path.clone()),
    })
}

fn build_approval_envelope(
    status: crate::state_store::RunGraphStatus,
    dispatch_summary: Option<crate::state_store::RunGraphDispatchReceiptSummary>,
    approval_receipt: Option<crate::state_store::RunGraphApprovalDelegationReceipt>,
) -> ApprovalEnvelope {
    let approval_status = derive_approval_status(&status, approval_receipt.as_ref());
    let principal_delegation = status
        .principal_delegation_projection(dispatch_summary.as_ref(), approval_receipt.as_ref());
    let memory_governance = status.memory_governance_projection(approval_receipt.as_ref());
    let mut blocker_codes = Vec::new();
    blocker_codes.extend(principal_delegation.blocker_codes.iter().cloned());
    blocker_codes.extend(memory_governance.blocker_codes.iter().cloned());
    let surface_blocked = !blocker_codes.is_empty();
    blocker_codes = crate::contract_profile_adapter::canonical_blocker_codes(&blocker_codes);
    let mut next_actions = match approval_status {
        "waiting_for_approval" | "approval_required" => vec![
            "Use `vida taskflow run-graph update <task-id> implementation review_ensemble approved implementation` for the active run once the approval decision is ready.".to_string(),
        ],
        "approved" => vec![
            "Use `vida taskflow consume continue --json` to continue the active run after approval.".to_string(),
        ],
        _ => vec!["Use `vida approval show <run-id> --json` to inspect a specific run.".to_string()],
    };
    if principal_delegation
        .blocker_codes
        .iter()
        .any(|code| code == blocker_code_str(BlockerCode::DelegationChainBroken))
    {
        next_actions.push(
            "Refresh the bounded delegation projection so the approval surface carries explicit delegator/delegatee and audit linkage for the active run."
                .to_string(),
        );
    }
    if memory_governance.governance_required && memory_governance.enforcement_state == "blocked" {
        next_actions.push(
            "Record consent, TTL, and approval linkage before continuing memory-governed approval or delegation work."
                .to_string(),
        );
    }
    let artifact_refs = build_artifact_refs(
        &status,
        dispatch_summary.as_ref(),
        approval_receipt.as_ref(),
    );
    let operator_contracts = render_operator_contract_envelope(
        if surface_blocked { "blocked" } else { "pass" },
        blocker_codes.clone(),
        next_actions.clone(),
        artifact_refs,
    );
    let surface_status = if operator_contracts["status"].as_str() == Some("blocked") {
        "blocked"
    } else {
        "pass"
    };
    ApprovalEnvelope {
        surface: "vida approval",
        status: surface_status,
        trace_id: operator_contracts["trace_id"]
            .as_str()
            .map(ToOwned::to_owned),
        workflow_class: operator_contracts["workflow_class"]
            .as_str()
            .map(ToOwned::to_owned),
        risk_tier: operator_contracts["risk_tier"]
            .as_str()
            .map(ToOwned::to_owned),
        artifact_refs: operator_contracts["artifact_refs"].clone(),
        next_actions,
        blocker_codes: operator_contracts["blocker_codes"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        run_id: status.run_id.clone(),
        task_id: status.task_id.clone(),
        approval_scope: approval_scope_from_status(&status),
        approval_status: approval_status.to_string(),
        gate_level: derive_gate_level(approval_status, &status).to_string(),
        decision_reason: derive_decision_reason(&status, approval_receipt.as_ref()),
        expiry_state: derive_expiry_state(approval_status, &status).to_string(),
        approval_evidence_refs: build_approval_evidence_refs(
            &status,
            dispatch_summary.as_ref(),
            approval_receipt.as_ref(),
        ),
        principal_delegation,
        memory_governance,
    }
}

fn emit_approval_envelope(envelope: &ApprovalEnvelope, as_json: bool) -> ExitCode {
    if crate::surface_render::print_surface_json(
        envelope,
        as_json,
        "approval surface should serialize",
    ) {
        return ExitCode::SUCCESS;
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(crate::RenderMode::Plain, "run_id", &envelope.run_id);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "approval_status",
        &envelope.approval_status,
    );
    crate::print_surface_line(crate::RenderMode::Plain, "gate_level", &envelope.gate_level);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "approval_scope",
        &envelope.approval_scope,
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "decision_reason",
        &envelope.decision_reason,
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "expiry_state",
        &envelope.expiry_state,
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "principal_delegation",
        &envelope.principal_delegation.as_display(),
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "memory_governance",
        &envelope.memory_governance.as_display(),
    );
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    ExitCode::SUCCESS
}

pub(crate) async fn run_approval(args: ProxyArgs) -> ExitCode {
    let as_json = args.args.iter().any(|arg| arg == "--json");
    if args.args.is_empty() || args.args.iter().all(|arg| arg.starts_with('-')) {
        return emit_blocked_approval_envelope(
            as_json,
            "vida approval requires a bounded subcommand; the root surface blocks missing or invalid approval requests instead of inferring one."
                .to_string(),
        );
    }

    let command = match parse_approval_args(&args.args) {
        Ok(command) => command,
        Err(reason) => {
            return emit_blocked_approval_envelope(
                as_json,
                format!(
                    "vida approval rejected the request: {reason}. The root surface only accepts bounded approval inspection subcommands."
                ),
            );
        }
    };

    let state_dir = proxy_state_dir();
    let store = match StateStore::open_existing(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    match command {
        ApprovalCommand::ShowLatest { as_json } => {
            let Some(status) = (match store.latest_run_graph_status().await {
                Ok(status) => status,
                Err(error) => {
                    eprintln!("Failed to read latest approval status: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("No approval evidence found.");
                return ExitCode::from(2);
            };
            let dispatch_summary = store
                .latest_run_graph_dispatch_receipt_summary()
                .await
                .ok()
                .flatten();
            let approval_receipt = store
                .run_graph_approval_delegation_receipt(&status.run_id)
                .await
                .ok()
                .flatten();
            let envelope = build_approval_envelope(status, dispatch_summary, approval_receipt);
            emit_approval_envelope(&envelope, as_json)
        }
        ApprovalCommand::ShowRun { run_id, as_json } => {
            let status = match store.run_graph_status(run_id).await {
                Ok(status) => status,
                Err(error) => {
                    eprintln!("Failed to read approval status `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            };
            let dispatch_summary = store
                .run_graph_dispatch_receipt(run_id)
                .await
                .ok()
                .flatten()
                .map(crate::state_store::RunGraphDispatchReceiptSummary::from_receipt);
            let approval_receipt = store
                .run_graph_approval_delegation_receipt(run_id)
                .await
                .ok()
                .flatten();
            let envelope = build_approval_envelope(status, dispatch_summary, approval_receipt);
            emit_approval_envelope(&envelope, as_json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn approval_surface_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct ProxyStateDirOverrideGuard;

    impl ProxyStateDirOverrideGuard {
        fn install(path: std::path::PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for ProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
    }

    fn sample_run_graph_status() -> crate::state_store::RunGraphStatus {
        crate::state_store::RunGraphStatus {
            run_id: "run-approval-test".to_string(),
            task_id: "approval-test".to_string(),
            task_class: "implementation".to_string(),
            active_node: "verification".to_string(),
            next_node: Some("approval".to_string()),
            status: "awaiting_approval".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "internal_subagents".to_string(),
            lane_id: "dev_pack_direct".to_string(),
            lifecycle_stage: "approval_wait".to_string(),
            policy_gate: crate::release1_contracts::ApprovalStatus::ApprovalRequired
                .as_str()
                .to_string(),
            handoff_state: "awaiting_approval".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "dispatch.approval".to_string(),
            recovery_ready: true,
        }
    }

    #[test]
    fn parse_approval_show_latest_supports_json() {
        let args = vec![
            "show".to_string(),
            "--latest".to_string(),
            "--json".to_string(),
        ];
        let command = parse_approval_args(&args).expect("approval show latest should parse");
        assert!(matches!(
            command,
            ApprovalCommand::ShowLatest { as_json: true }
        ));
    }

    #[tokio::test]
    async fn approval_surface_renders_latest_approval_wait_with_canonical_envelope() {
        let _guard = approval_surface_test_lock()
            .lock()
            .expect("approval surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-approval-surface-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let status = sample_run_graph_status();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist approval wait run graph status");

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("latest status should load")
            .expect("latest status should exist");
        let dispatch_summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("latest dispatch summary should load");
        let approval_receipt = store
            .run_graph_approval_delegation_receipt(&latest_status.run_id)
            .await
            .expect("approval receipt should load");
        let envelope = build_approval_envelope(latest_status, dispatch_summary, approval_receipt);

        assert_eq!(envelope.surface, "vida approval");
        assert_eq!(envelope.status, "pass");
        assert_eq!(envelope.approval_status, "waiting_for_approval");
        assert_eq!(envelope.gate_level, "block");
        assert!(envelope.approval_scope.contains("task_id=approval-test"));
        assert!(envelope
            .decision_reason
            .contains("waiting for explicit approval"));
        assert_eq!(envelope.expiry_state, "not_tracked");
        assert_eq!(envelope.trace_id, None);
        assert_eq!(envelope.workflow_class, None);
        assert_eq!(envelope.risk_tier, None);
        assert_eq!(envelope.blocker_codes.len(), 0);
        assert_eq!(
            envelope.approval_evidence_refs["run_id"],
            "run-approval-test"
        );
        assert!(envelope.artifact_refs["dispatch_packet_path"].is_null());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn approval_surface_renders_specific_completed_run_as_approved() {
        let _guard = approval_surface_test_lock()
            .lock()
            .expect("approval surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-approval-surface-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let mut awaiting_approval = sample_run_graph_status();
        store
            .record_run_graph_status(&awaiting_approval)
            .await
            .expect("persist approval wait run graph status");

        awaiting_approval.status = "completed".to_string();
        awaiting_approval.next_node = None;
        awaiting_approval.lifecycle_stage = "implementation_complete".to_string();
        awaiting_approval.policy_gate = "not_required".to_string();
        awaiting_approval.handoff_state = "none".to_string();
        awaiting_approval.resume_target = "none".to_string();
        store
            .record_run_graph_status(&awaiting_approval)
            .await
            .expect("persist approval complete run graph status");

        let status = store
            .run_graph_status("run-approval-test")
            .await
            .expect("specific status should load");
        let dispatch_summary = store
            .run_graph_dispatch_receipt("run-approval-test")
            .await
            .expect("dispatch receipt should load")
            .map(crate::state_store::RunGraphDispatchReceiptSummary::from_receipt);
        let approval_receipt = store
            .run_graph_approval_delegation_receipt("run-approval-test")
            .await
            .expect("approval receipt should load");
        let envelope = build_approval_envelope(status, dispatch_summary, approval_receipt);

        assert_eq!(envelope.approval_status, "approved");
        assert_eq!(envelope.gate_level, "observe");
        assert_eq!(envelope.expiry_state, "not_applicable");
        assert!(envelope
            .next_actions
            .first()
            .expect("next action should exist")
            .contains("consume continue"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
