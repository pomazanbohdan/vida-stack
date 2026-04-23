use std::process::ExitCode;

use crate::release1_contracts::CompatibilityClass;

pub(crate) struct StatusTextReportInputs<'a> {
    pub(crate) render: crate::RenderMode,
    pub(crate) backend_summary: &'a str,
    pub(crate) state_dir: &'a std::path::Path,
    pub(crate) state_spine: &'a crate::state_store::StateSpineSummary,
    pub(crate) effective_bundle_receipt:
        Option<&'a crate::state_store::EffectiveBundleReceiptSummary>,
    pub(crate) boot_compatibility: Option<&'a crate::state_store::BootCompatibilitySummary>,
    pub(crate) migration_state: Option<&'a crate::state_store::MigrationPreflightSummary>,
    pub(crate) migration_receipts: &'a crate::state_store::MigrationReceiptSummary,
    pub(crate) latest_task_reconciliation:
        Option<&'a crate::state_store::TaskReconciliationSummary>,
    pub(crate) task_reconciliation_rollup: &'a crate::state_store::TaskReconciliationRollup,
    pub(crate) snapshot_bridge: &'a crate::state_store::TaskflowSnapshotBridgeSummary,
    pub(crate) runtime_consumption: &'a crate::runtime_consumption_state::RuntimeConsumptionSummary,
    pub(crate) protocol_binding: &'a crate::state_store::ProtocolBindingSummary,
    pub(crate) activation_truth:
        Option<&'a crate::project_activator_surface::ProjectActivationStatusTruth>,
    pub(crate) project_activation_status: Option<&'a str>,
    pub(crate) project_activation_pending: bool,
    pub(crate) latest_run_graph_status: Option<&'a crate::state_store::RunGraphStatus>,
    pub(crate) latest_run_graph_recovery: Option<&'a crate::state_store::RunGraphRecoverySummary>,
    pub(crate) latest_run_graph_checkpoint:
        Option<&'a crate::state_store::RunGraphCheckpointSummary>,
    pub(crate) latest_run_graph_gate: Option<&'a crate::state_store::RunGraphGateSummary>,
    pub(crate) latest_run_graph_snapshot_inconsistent: bool,
    pub(crate) latest_run_graph_dispatch_receipt_signal_ambiguous: bool,
    pub(crate) latest_run_graph_dispatch_receipt_summary_inconsistent: bool,
    pub(crate) latest_run_graph_dispatch_receipt_checkpoint_leakage: bool,
    pub(crate) continuation_binding: &'a serde_json::Value,
    pub(crate) host_agents: Option<&'a serde_json::Value>,
    pub(crate) latest_run_graph_dispatch_receipt:
        Option<&'a crate::state_store::RunGraphDispatchReceiptSummary>,
    pub(crate) latest_run_graph_mixed_posture: Option<&'a serde_json::Value>,
    pub(crate) latest_run_graph_activation_vs_execution_evidence: Option<&'a serde_json::Value>,
}

fn format_run_graph_mixed_posture(mixed_posture: &serde_json::Value) -> String {
    let fanout = mixed_posture["fanout_backends"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{} backend={} fallback={} fanout={}",
        mixed_posture["effective_posture_kind"]
            .as_str()
            .unwrap_or("unknown"),
        mixed_posture["selected_backend"].as_str().unwrap_or("none"),
        mixed_posture["fallback_backend"].as_str().unwrap_or("none"),
        fanout
    )
}

fn format_run_graph_activation_vs_execution_evidence(evidence: &serde_json::Value) -> String {
    format!(
        "{} activation_kind={}",
        evidence["evidence_state"].as_str().unwrap_or("unknown"),
        evidence["activation_kind"].as_str().unwrap_or("unknown")
    )
}

fn format_dispatch_blockers(blocker_codes: &[String]) -> String {
    if blocker_codes.is_empty() {
        "none".to_string()
    } else {
        blocker_codes.join(", ")
    }
}

fn format_run_graph_dispatch_compact_summary(
    summary: &crate::taskflow_run_graph::RunGraphDispatchCompactSummary,
) -> (String, String, Option<String>) {
    let route_truth = format!(
        "source={} parity={} receipt_present={} binding_present={} evidence={} activation_kind={} receipt_backed={} stale={}",
        summary.route_truth.projection_source,
        summary.route_truth.projection_vs_receipt_parity,
        summary.route_truth.dispatch_receipt_present,
        summary.route_truth.continuation_binding_present,
        summary.route_truth.evidence_state,
        summary.route_truth.activation_kind,
        summary.route_truth.receipt_backed_execution_evidence,
        summary.stale_state_suspected,
    );
    let downstream_preview = format!(
        "target={} status={} lane_status={} backend={} agent={} runtime_role={} next_target={} next_status={} next_ready={} next_count={} next_active={} next_last={} blockers={}",
        summary.downstream_dispatch_preview.dispatch_target,
        summary.downstream_dispatch_preview.dispatch_status,
        summary.downstream_dispatch_preview.lane_status,
        summary.downstream_dispatch_preview.selected_backend,
        summary.downstream_dispatch_preview.activation_agent_type,
        summary.downstream_dispatch_preview.activation_runtime_role,
        summary
            .downstream_dispatch_preview
            .downstream_dispatch_target,
        summary
            .downstream_dispatch_preview
            .downstream_dispatch_status,
        summary
            .downstream_dispatch_preview
            .downstream_dispatch_ready,
        summary
            .downstream_dispatch_preview
            .downstream_dispatch_executed_count,
        summary
            .downstream_dispatch_preview
            .downstream_dispatch_active_target,
        summary
            .downstream_dispatch_preview
            .downstream_dispatch_last_target,
        format_dispatch_blockers(&summary.blocker_codes),
    );
    let next_action = summary.recommended_command.as_ref().map(|command| {
        format!(
            "{} ({})",
            command,
            summary.recommended_surface.as_deref().unwrap_or("unknown")
        )
    });
    (route_truth, downstream_preview, next_action)
}

fn emit_dispatch_diagnosis_lines(
    render: crate::RenderMode,
    summary: &crate::taskflow_run_graph::RunGraphDispatchCompactSummary,
) {
    let (route_truth, downstream_preview, next_action) =
        format_run_graph_dispatch_compact_summary(summary);
    crate::surface_render::print_surface_line(render, "latest dispatch route truth", &route_truth);
    crate::surface_render::print_surface_line(
        render,
        "latest downstream dispatch preview",
        &downstream_preview,
    );
    crate::surface_render::print_surface_line(
        render,
        "latest dispatch projection reason",
        &summary.route_truth.projection_reason,
    );
    if let Some(next_action) = next_action {
        crate::surface_render::print_surface_line(
            render,
            "latest dispatch next action",
            &next_action,
        );
    }
}

pub(crate) fn emit_status_text_report(inputs: StatusTextReportInputs<'_>) -> ExitCode {
    crate::surface_render::print_surface_header(inputs.render, "vida status");
    crate::surface_render::print_surface_line(inputs.render, "backend", inputs.backend_summary);
    crate::surface_render::print_surface_line(
        inputs.render,
        "state dir",
        &inputs.state_dir.display().to_string(),
    );
    crate::surface_render::print_surface_line(
        inputs.render,
        "state spine",
        &format!(
            "initialized (state-v{}, {} entity surfaces, mutation root {})",
            inputs.state_spine.state_schema_version,
            inputs.state_spine.entity_surface_count,
            inputs.state_spine.authoritative_mutation_root
        ),
    );
    match inputs.effective_bundle_receipt {
        Some(receipt) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest effective bundle receipt",
                &receipt.receipt_id,
            );
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest effective bundle root",
                &receipt.root_artifact_id,
            );
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest effective bundle artifact count",
                &receipt.artifact_count.to_string(),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest effective bundle receipt",
                "none",
            );
        }
    }
    match inputs.boot_compatibility {
        Some(compatibility) => {
            let compatibility_classification =
                crate::release1_contracts::canonical_compatibility_class_str(
                    &compatibility.classification,
                )
                .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str());
            crate::surface_render::print_surface_line(
                inputs.render,
                "boot compatibility",
                &format!(
                    "{} ({})",
                    compatibility_classification, compatibility.next_step
                ),
            );
        }
        None => {
            crate::surface_render::print_surface_line(inputs.render, "boot compatibility", "none");
        }
    }
    match inputs.migration_state {
        Some(migration) => {
            let compatibility_classification =
                crate::release1_contracts::canonical_compatibility_class_str(
                    &migration.compatibility_classification,
                )
                .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str());
            crate::surface_render::print_surface_line(
                inputs.render,
                "migration state",
                &format!(
                    "{} / {} ({})",
                    compatibility_classification, migration.migration_state, migration.next_step
                ),
            );
        }
        None => {
            crate::surface_render::print_surface_line(inputs.render, "migration state", "none");
        }
    }
    crate::surface_render::print_surface_line(
        inputs.render,
        "migration receipts",
        &inputs.migration_receipts.as_display(),
    );
    match inputs.latest_task_reconciliation {
        Some(receipt) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest task reconciliation",
                &receipt.as_display(),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest task reconciliation",
                "none",
            );
        }
    }
    crate::surface_render::print_surface_line(
        inputs.render,
        "task reconciliation rollup",
        &inputs.task_reconciliation_rollup.as_display(),
    );
    crate::surface_render::print_surface_line(
        inputs.render,
        "taskflow snapshot bridge",
        &inputs.snapshot_bridge.as_display(),
    );
    crate::surface_render::print_surface_line(
        inputs.render,
        "runtime consumption",
        &inputs.runtime_consumption.as_display(),
    );
    crate::surface_render::print_surface_line(
        inputs.render,
        "protocol binding",
        &inputs.protocol_binding.as_display(),
    );
    if inputs.activation_truth.is_some() {
        crate::surface_render::print_surface_line(
            inputs.render,
            "project activation",
            &format!(
                "{} (activation_pending={})",
                inputs.project_activation_status.unwrap_or("pending"),
                inputs.project_activation_pending
            ),
        );
    } else {
        crate::surface_render::print_surface_line(
            inputs.render,
            "project activation",
            "unknown (fail-closed: activation_pending=true)",
        );
    }
    match inputs.latest_run_graph_status {
        Some(status) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph status",
                &status.as_display(),
            );
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph delegation gate",
                &status.delegation_gate().as_display(),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph status",
                "none",
            );
        }
    }
    match inputs.latest_run_graph_recovery {
        Some(summary) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph recovery",
                &summary.as_display(),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph recovery",
                "none",
            );
        }
    }
    match inputs.latest_run_graph_checkpoint {
        Some(summary) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph checkpoint",
                &summary.as_display(),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph checkpoint",
                "none",
            );
        }
    }
    if let Some(mixed_posture) = inputs.latest_run_graph_mixed_posture {
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph mixed posture",
            &format_run_graph_mixed_posture(mixed_posture),
        );
    }
    if let Some(evidence) = inputs.latest_run_graph_activation_vs_execution_evidence {
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph activation vs execution evidence",
            &format_run_graph_activation_vs_execution_evidence(evidence),
        );
    }
    match inputs.latest_run_graph_gate {
        Some(summary) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph gate",
                &summary.as_display(),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph gate",
                "none",
            );
        }
    }
    let compact_summary = crate::taskflow_run_graph::build_run_graph_dispatch_compact_summary(
        inputs.latest_run_graph_status,
        inputs.latest_run_graph_recovery,
        inputs.latest_run_graph_dispatch_receipt,
        Some(inputs.continuation_binding),
        inputs.latest_run_graph_activation_vs_execution_evidence,
    );
    match inputs.latest_run_graph_dispatch_receipt {
        Some(summary) => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph dispatch receipt",
                &format!(
                    "run={} target={} status={} lane_status={} backend={} evidence={}",
                    summary.run_id,
                    summary.dispatch_target,
                    summary.dispatch_status,
                    summary.lane_status,
                    summary.selected_backend.as_deref().unwrap_or("none"),
                    summary.activation_evidence["activation_kind"]
                        .as_str()
                        .unwrap_or("unknown"),
                ),
            );
        }
        None => {
            crate::surface_render::print_surface_line(
                inputs.render,
                "latest run graph dispatch receipt",
                "none",
            );
        }
    }
    if let Some(compact_summary) = compact_summary.as_ref() {
        emit_dispatch_diagnosis_lines(inputs.render, compact_summary);
    }
    crate::surface_render::print_surface_line(
        inputs.render,
        "continuation binding",
        &format!(
            "status={} primary_path={} posture={}",
            inputs.continuation_binding["status"]
                .as_str()
                .unwrap_or("unknown"),
            inputs.continuation_binding["primary_path"]
                .as_str()
                .unwrap_or("unknown"),
            inputs.continuation_binding["sequential_vs_parallel_posture"]
                .as_str()
                .unwrap_or("unknown"),
        ),
    );
    if let Some(reason) = inputs.continuation_binding["ambiguity_reason"].as_str() {
        if !reason.trim().is_empty() {
            crate::surface_render::print_surface_line(
                inputs.render,
                "continuation binding ambiguity",
                reason,
            );
        }
    }
    if let Some(step) = inputs.continuation_binding["next_actions"]
        .as_array()
        .and_then(|steps| steps.first())
        .and_then(serde_json::Value::as_str)
    {
        crate::surface_render::print_surface_line(
            inputs.render,
            "continuation binding next action",
            step,
        );
    }
    if inputs.latest_run_graph_snapshot_inconsistent {
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph next action",
            crate::status_surface_signals::run_graph_latest_snapshot_inconsistent_next_action(),
        );
    }
    if inputs.latest_run_graph_dispatch_receipt_signal_ambiguous {
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph dispatch receipt next action",
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_signal_ambiguous_next_action(),
        );
    }
    if inputs.latest_run_graph_dispatch_receipt_summary_inconsistent {
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph dispatch receipt blocker",
            "run_graph_latest_dispatch_receipt_summary_inconsistent",
        );
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph dispatch receipt summary next action",
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_summary_inconsistent_next_action(),
        );
    }
    if inputs.latest_run_graph_dispatch_receipt_checkpoint_leakage {
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph dispatch receipt blocker",
            "run_graph_latest_dispatch_receipt_checkpoint_leakage",
        );
        crate::surface_render::print_surface_line(
            inputs.render,
            "latest run graph dispatch receipt checkpoint leakage next action",
            crate::status_surface_signals::run_graph_latest_dispatch_receipt_checkpoint_leakage_next_action(),
        );
    }
    if let Some(host_agents) = inputs.host_agents {
        crate::surface_render::print_surface_line(
            inputs.render,
            "host agents",
            host_agents["host_cli_system"].as_str().unwrap_or("unknown"),
        );
        crate::surface_render::print_surface_line(
            inputs.render,
            "host agent budget units",
            &host_agents["budget"]["total_estimated_units"]
                .as_u64()
                .unwrap_or_default()
                .to_string(),
        );
        crate::surface_render::print_surface_line(
            inputs.render,
            "host agent events",
            &host_agents["budget"]["event_count"]
                .as_u64()
                .unwrap_or_default()
                .to_string(),
        );
        crate::surface_render::print_surface_line(
            inputs.render,
            "host agent posture",
            host_agents["mixed_posture_details"]["effective_execution_posture"]
                .as_str()
                .unwrap_or("unknown"),
        );
        crate::surface_render::print_surface_line(
            inputs.render,
            "root session write guard",
            host_agents["root_session_write_guard"]["status"]
                .as_str()
                .unwrap_or("missing"),
        );
        if host_agents["external_cli_preflight"]["status"]
            .as_str()
            .is_some_and(|value| value == "blocked")
        {
            crate::surface_render::print_surface_line(
                inputs.render,
                "external cli preflight",
                host_agents["external_cli_preflight"]["blocker_code"]
                    .as_str()
                    .unwrap_or("blocked"),
            );
            if let Some(next_actions) =
                host_agents["external_cli_preflight"]["next_actions"].as_array()
            {
                for action in next_actions {
                    if let Some(text) = action.as_str() {
                        crate::surface_render::print_surface_line(
                            inputs.render,
                            "external cli next action",
                            text,
                        );
                    }
                }
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        format_run_graph_activation_vs_execution_evidence,
        format_run_graph_dispatch_compact_summary, format_run_graph_mixed_posture,
    };

    #[test]
    fn mixed_posture_display_includes_selected_backend_and_fanout() {
        let mixed_posture = serde_json::json!({
            "effective_posture_kind": "hybrid_external_cli",
            "selected_backend": "opencode_cli",
            "fallback_backend": "internal_subagents",
            "fanout_backends": ["opencode_cli", "hermes_cli", "kilo_cli"],
        });

        assert_eq!(
            format_run_graph_mixed_posture(&mixed_posture),
            "hybrid_external_cli backend=opencode_cli fallback=internal_subagents fanout=opencode_cli, hermes_cli, kilo_cli"
        );
    }

    #[test]
    fn activation_vs_execution_evidence_display_includes_state_and_kind() {
        let evidence = serde_json::json!({
            "evidence_state": "consistent",
            "activation_kind": "dispatch",
        });

        assert_eq!(
            format_run_graph_activation_vs_execution_evidence(&evidence),
            "consistent activation_kind=dispatch"
        );
    }

    #[test]
    fn compact_dispatch_summary_display_includes_route_truth_and_preview() {
        let summary = crate::taskflow_run_graph::RunGraphDispatchCompactSummary {
            route_truth: crate::taskflow_run_graph::RunGraphDispatchRouteTruthSummary {
                projection_source: "reconciled_run_graph_status".to_string(),
                projection_reason: "status reconciled from receipt".to_string(),
                projection_vs_receipt_parity: "aligned".to_string(),
                dispatch_receipt_present: true,
                continuation_binding_present: true,
                evidence_state: "activation_view_only".to_string(),
                activation_kind: "activation_view".to_string(),
                receipt_backed_execution_evidence: false,
            },
            downstream_dispatch_preview:
                crate::taskflow_run_graph::RunGraphDownstreamDispatchPreviewSummary {
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "executed".to_string(),
                    lane_status: "lane_completed".to_string(),
                    selected_backend: "opencode_cli".to_string(),
                    activation_agent_type: "junior".to_string(),
                    activation_runtime_role: "worker".to_string(),
                    downstream_dispatch_target: "verifier".to_string(),
                    downstream_dispatch_status: "blocked".to_string(),
                    downstream_dispatch_ready: true,
                    downstream_dispatch_executed_count: 1,
                    downstream_dispatch_active_target: "verifier".to_string(),
                    downstream_dispatch_last_target: "verifier".to_string(),
                },
            blocker_codes: vec!["open_delegated_cycle".to_string()],
            stale_state_suspected: false,
            recommended_command: Some(
                "vida taskflow consume continue --run-id run-1 --json".to_string(),
            ),
            recommended_surface: Some("vida taskflow consume continue".to_string()),
        };

        let (route_truth, downstream_preview, next_action) =
            format_run_graph_dispatch_compact_summary(&summary);

        assert!(route_truth.contains("source=reconciled_run_graph_status"));
        assert!(route_truth.contains("receipt_present=true"));
        assert!(route_truth.contains("binding_present=true"));
        assert!(route_truth.contains("evidence=activation_view_only"));
        assert!(route_truth.contains("stale=false"));
        assert!(downstream_preview.contains("next_target=verifier"));
        assert!(downstream_preview.contains("blockers=open_delegated_cycle"));
        assert_eq!(
            next_action.as_deref(),
            Some(
                "vida taskflow consume continue --run-id run-1 --json (vida taskflow consume continue)"
            )
        );
    }

    #[test]
    fn compact_dispatch_summary_display_handles_status_only_preview() {
        let summary = crate::taskflow_run_graph::RunGraphDispatchCompactSummary {
            route_truth: crate::taskflow_run_graph::RunGraphDispatchRouteTruthSummary {
                projection_source: "persisted_run_graph_status".to_string(),
                projection_reason: "run-graph status reflects authoritative persisted state"
                    .to_string(),
                projection_vs_receipt_parity: "no_receipt".to_string(),
                dispatch_receipt_present: false,
                continuation_binding_present: true,
                evidence_state: "activation_view_only".to_string(),
                activation_kind: "activation_view".to_string(),
                receipt_backed_execution_evidence: false,
            },
            downstream_dispatch_preview:
                crate::taskflow_run_graph::RunGraphDownstreamDispatchPreviewSummary {
                    dispatch_target: "business_analyst".to_string(),
                    dispatch_status: "ready".to_string(),
                    lane_status: "analysis_active".to_string(),
                    selected_backend: "opencode_cli".to_string(),
                    activation_agent_type: "none".to_string(),
                    activation_runtime_role: "none".to_string(),
                    downstream_dispatch_target: "implementer".to_string(),
                    downstream_dispatch_status: "resume_ready".to_string(),
                    downstream_dispatch_ready: true,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: "business_analyst".to_string(),
                    downstream_dispatch_last_target: "implementer".to_string(),
                },
            blocker_codes: Vec::new(),
            stale_state_suspected: false,
            recommended_command: Some(
                "vida taskflow consume continue --run-id run-2 --json".to_string(),
            ),
            recommended_surface: Some("vida taskflow consume continue".to_string()),
        };

        let (route_truth, downstream_preview, next_action) =
            format_run_graph_dispatch_compact_summary(&summary);

        assert!(route_truth.contains("source=persisted_run_graph_status"));
        assert!(route_truth.contains("parity=no_receipt"));
        assert!(route_truth.contains("receipt_present=false"));
        assert!(route_truth.contains("binding_present=true"));
        assert!(route_truth.contains("stale=false"));
        assert!(downstream_preview.contains("target=business_analyst"));
        assert!(downstream_preview.contains("next_target=implementer"));
        assert!(downstream_preview.contains("next_status=resume_ready"));
        assert!(downstream_preview.contains("next_ready=true"));
        assert!(downstream_preview.contains("blockers=none"));
        assert_eq!(
            next_action.as_deref(),
            Some(
                "vida taskflow consume continue --run-id run-2 --json (vida taskflow consume continue)"
            )
        );
    }
}
