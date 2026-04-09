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
    pub(crate) host_agents: Option<&'a serde_json::Value>,
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
