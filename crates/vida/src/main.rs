mod activation_status;
mod agent_dispatch_surface;
mod agent_extension_bundle_validation;
mod agent_extension_catalog_projection;
mod agent_extension_registry_projection;
mod agent_feedback_surface;
mod approval_surface;
mod bootstrap_value_utils;
mod carrier_runtime_catalog;
mod carrier_runtime_metadata;
mod carrier_runtime_projection;
mod carrier_runtime_strategy;
mod cli;
mod compiled_agent_extension_bundle;
mod config_value_utils;
mod consume_final_operator_surface;
mod continuation_binding_summary;
mod contract_profile_adapter;
mod contract_profile_registry;
mod development_flow_glue;
mod development_flow_orchestration;
mod development_request_analysis;
mod docflow_proxy;
mod docflow_runtime_verdict;
mod doctor_surface;
mod host_agent_state;
mod host_runtime_materialization;
mod host_runtime_registry;
mod init_surfaces;
mod lane_surface;
mod launcher_activation_snapshot;
mod launcher_task_commands;
mod memory_surface;
mod model_profile_contract;
mod operator_contracts;
mod project_activator_activation_summary;
mod project_activator_agent_extensions_summary;
mod project_activator_host_cli_summary;
mod project_activator_normal_work_defaults;
mod project_activator_runtime_surface;
mod project_activator_surface;
mod project_bootstrap_defaults;
mod project_root_paths;
mod protocol_surface;
mod registry_projection_utils;
mod release1_contracts;
mod release_contract_adapters;
mod release_surface;
mod root_command_router;
mod runtime_assignment_builder;
mod runtime_assignment_policy;
mod runtime_assignment_projection_utils;
mod runtime_consumption_state;
mod runtime_consumption_surface;
mod runtime_contract_vocab;
mod runtime_dispatch_bootstrap;
mod runtime_dispatch_downstream_packets;
mod runtime_dispatch_execution;
mod runtime_dispatch_packet_text;
mod runtime_dispatch_packets;
mod runtime_dispatch_state;
mod runtime_dispatch_status;
mod runtime_lane_summary;
mod shell_runtime_helpers;
mod state_store;
mod status_surface;
mod status_surface_external_cli;
mod status_surface_host_agents;
mod status_surface_host_cli_summary;
mod status_surface_host_cli_system;
mod status_surface_json_report;
mod status_surface_operator_contracts;
mod status_surface_signals;
mod status_surface_text_report;
mod status_surface_truth_inputs;
mod status_surface_write_guard;
mod surface_render;
mod task_cli_render;
mod task_surface;
mod taskflow_artifacts;
mod taskflow_consume;
mod taskflow_consume_bundle;
mod taskflow_consume_resume;
mod taskflow_continuation;
mod taskflow_layer4;
mod taskflow_packet;
mod taskflow_plan_graph;
mod taskflow_protocol_binding;
mod taskflow_proxy;
mod taskflow_routing;
mod taskflow_run_graph;
mod taskflow_runtime_bundle;
mod taskflow_spec_bootstrap;
mod taskflow_task_bridge;
mod temp_state;
#[cfg(test)]
mod test_cli_support;

use std::path::PathBuf;
use std::process::ExitCode;

use crate::contract_profile_adapter::{
    blocker_code as blocker_code_value, blocker_code_str, BlockerCode,
};
use agent_extension_bundle_validation::{
    extend_agent_extension_bundle_validation_errors, AgentExtensionBundleValidationInput,
};
use agent_extension_catalog_projection::build_agent_extension_catalog_projection;
use agent_extension_registry_projection::build_agent_extension_registry_projection;
pub(crate) use bootstrap_value_utils::{
    config_file_path, inferred_project_title, is_missing_or_placeholder, normalize_root_arg,
    slugify_project_id, trimmed_non_empty,
};
use carrier_runtime_projection::build_carrier_runtime_projection;
use clap::Parser;
pub(crate) use cli::*;
pub(crate) use compiled_agent_extension_bundle::build_compiled_agent_extension_bundle_for_root;
pub(crate) use config_value_utils::{
    csv_json_string_list, json_bool, json_lookup, json_string, json_string_list,
    load_project_overlay_yaml, split_csv_like, yaml_bool, yaml_lookup, yaml_string,
    yaml_string_list,
};
#[allow(unused_imports)]
pub(crate) use consume_final_operator_surface::{
    build_operator_contracts_envelope, emit_taskflow_consume_final_json,
};
#[allow(unused_imports)]
pub(crate) use development_flow_glue::{
    display_lane_label, execution_plan_agent_only_development_required,
};
pub(crate) use development_flow_orchestration::build_design_first_tracked_flow_bootstrap;
pub(crate) use development_request_analysis::{
    coach_review_terms, contains_keywords, feature_delivery_design_terms,
};
pub(crate) use docflow_runtime_verdict::{
    blocking_docflow_activation, build_docflow_runtime_verdict,
};
pub(crate) use host_agent_state::{
    append_host_agent_observability_event, host_agent_observability_state_path,
    load_or_initialize_host_agent_observability_state, load_or_initialize_worker_scorecards,
    read_json_file_if_present, refresh_worker_strategy, worker_scorecards_state_path,
    worker_strategy_state_path, HostAgentFeedbackInput, HOST_AGENT_OBSERVABILITY_STATE,
    PROMPT_LIFECYCLE_STATE, WORKER_SCORECARDS_STATE, WORKER_STRATEGY_STATE,
};
pub(crate) use init_surfaces::resolve_init_bootstrap_source_root;
pub(crate) use launcher_activation_snapshot::{
    ensure_launcher_bootstrap, read_or_sync_launcher_activation_snapshot,
    sync_launcher_activation_snapshot,
};
use launcher_task_commands::{
    build_task_close_command, build_task_create_command, build_task_ensure_command,
    build_task_show_command, infer_feature_request_slug, infer_feature_request_title, shell_quote,
};
pub(crate) use project_activator_surface::build_project_activator_view;
pub(crate) use project_activator_surface::merge_project_activation_into_init_view;
pub(crate) use project_activator_surface::ProjectActivationAnswers;
pub(crate) use project_bootstrap_defaults::*;
pub(crate) use project_root_paths::{
    ensure_dir, looks_like_project_root, resolve_repo_root, resolve_runtime_project_root,
    resolve_status_project_root,
};
pub(crate) use registry_projection_utils::{
    effective_enabled_registry_ids, non_empty_yaml_string, read_simple_toml_sections,
    registry_ids_by_key, registry_row_map_by_id, registry_rows_by_key,
};
use release1_contracts::{
    derive_lane_status, missing_downstream_lane_evidence_blocker, LaneStatus,
};
use root_command_router::run_root_command;
use runtime_assignment_builder::{
    build_runtime_assignment, build_runtime_assignment_from_dispatch_alias,
    build_runtime_assignment_from_resolved_constraints, resolve_dispatch_alias_id,
};
use runtime_assignment_policy::{
    infer_execution_runtime_role, infer_runtime_task_class, role_supports_task_class,
    runtime_role_for_task_class, task_complexity_multiplier,
};
pub(crate) use runtime_assignment_projection_utils::{
    carrier_runtime_section, infer_task_class_from_task_payload, json_u64,
    runtime_assignment_alias_fields, runtime_assignment_from_execution_plan,
};
#[allow(unused_imports)]
pub(crate) use runtime_consumption_state::{
    apply_runtime_consumption_final_dispatch_receipt_blocker,
    latest_admissible_retrieval_trust_signal,
    runtime_consumption_final_dispatch_receipt_blocker_code,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION,
};
pub(crate) use runtime_consumption_state::{
    latest_final_runtime_consumption_snapshot_path,
    latest_recorded_final_runtime_consumption_snapshot_path,
    latest_terminal_consume_continue_snapshot_run_id,
    runtime_consumption_snapshot_has_release_admission_evidence, runtime_consumption_summary,
    write_runtime_consumption_snapshot,
};
pub(crate) use runtime_consumption_surface::{
    blocking_lane_selection, build_docflow_runtime_evidence, doctor_launcher_summary_for_root,
    DoctorLauncherSummary, RuntimeConsumptionClosureAdmission, RuntimeConsumptionDocflowActivation,
    RuntimeConsumptionDocflowVerdict, RuntimeConsumptionEvidence, TaskflowConsumeBundleCheck,
    TaskflowConsumeBundlePayload, TaskflowDirectConsumptionPayload,
};
pub(crate) use runtime_dispatch_state::*;
pub(crate) use runtime_lane_summary::role_exists_in_lane_bundle;
pub(crate) use shell_runtime_helpers::{
    block_on_state_store, print_json_pretty, repo_runtime_root,
};
use state_store::{StateStore, StateStoreError};
pub(crate) use surface_render::{
    print_root_help, print_surface_header, print_surface_line, print_surface_ok,
};
use task_cli_render::{
    print_blocked_tasks, print_task_critical_path, print_task_dependencies,
    print_task_dependency_mutation, print_task_dependency_tree, print_task_export_summary,
    print_task_graph_issues, print_task_list, print_task_mutation, print_task_next_display_id,
    print_task_progress, print_task_ready, print_task_show,
};
use taskflow_layer4::print_taskflow_proxy_help;
use taskflow_proxy::run_taskflow_proxy;
pub(crate) use taskflow_routing::{
    dispatch_contract_execution_lane_sequence, dispatch_contract_lane,
    dispatch_contract_lane_activation, dispatch_contract_lane_sequence,
    dispatch_target_for_runtime_role, selected_backend_from_execution_plan_route,
};
use taskflow_runtime_bundle::{
    blocking_runtime_bundle, build_taskflow_consume_bundle_payload, taskflow_consume_bundle_check,
};
use taskflow_spec_bootstrap::{
    execute_taskflow_bootstrap_spec_with_store, execute_work_packet_create_with_store,
};
use time::format_description::well_known::Rfc3339;
#[tokio::main]
async fn main() -> ExitCode {
    run_root_command(Cli::parse()).await
}

#[cfg(test)]
pub(crate) async fn run(cli: Cli) -> ExitCode {
    run_root_command(cli).await
}

pub(crate) use development_flow_orchestration::{
    build_runtime_execution_plan_from_snapshot, build_runtime_lane_selection_with_store,
    RuntimeConsumptionLaneSelection,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::guard_current_dir;
    use std::fs;

    #[test]
    fn init_command_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        assert_eq!(
            runtime.block_on(run(Cli {
                command: Some(Command::Init(BootArgs {
                    state_dir: Some(harness.path().to_path_buf()),
                    render: RenderMode::Plain,
                    instruction_source_root: None,
                    framework_memory_source_root: None,
                    extra_args: Vec::new(),
                })),
            })),
            ExitCode::SUCCESS
        );
        assert!(harness.path().join("AGENTS.md").is_file());
        assert!(harness.path().join("AGENTS.sidecar.md").is_file());
        let agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("generated AGENTS should exist");
        assert!(
            agents.contains("VIDA Project Bootstrap Carrier"),
            "bare init should use the generated downstream bootstrap carrier"
        );
        assert!(
            !agents.contains("-v0"),
            "generated downstream bootstrap carrier should not leak legacy or historical runtime suffixes"
        );
        assert!(
            !harness.path().join(".codex").exists(),
            "host CLI templates should not materialize during bare `vida init`"
        );
        assert!(harness.path().join("vida.config.yaml").is_file());
        assert!(harness.path().join("README.md").is_file());
        assert!(harness.path().join(DEFAULT_PROJECT_ROOT_MAP).is_file());
        assert!(harness.path().join(DEFAULT_PROJECT_PRODUCT_INDEX).is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_PRODUCT_SPEC_README)
            .is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE)
            .is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_PROCESS_README)
            .is_file());
        assert!(harness
            .path()
            .join(DEFAULT_PROJECT_RESEARCH_README)
            .is_file());
        assert!(harness.path().join(".vida/config").is_dir());
        assert!(harness.path().join(".vida/db").is_dir());
        assert!(harness.path().join(".vida/cache").is_dir());
        assert!(harness.path().join(".vida/framework").is_dir());
        assert!(harness.path().join(".vida/project").is_dir());
        assert!(harness
            .path()
            .join(".vida/project/agent-extensions/README.md")
            .is_file());
        assert!(harness
            .path()
            .join(".vida/project/agent-extensions/roles.yaml")
            .is_file());
        assert!(harness
            .path()
            .join(".vida/project/agent-extensions/roles.sidecar.yaml")
            .is_file());
        assert!(harness.path().join(".vida/receipts").is_dir());
        assert!(harness.path().join(".vida/runtime").is_dir());
        assert!(harness.path().join(".vida/scratchpad").is_dir());
        assert!(!harness.path().join("vida").exists());
    }
}
