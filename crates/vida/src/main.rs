#![recursion_limit = "256"]
#![allow(dead_code, unused_imports)]

mod activation_status;
mod agent_dispatch_surface;
mod agent_extension_bundle_validation;
mod agent_extension_catalog_projection;
mod agent_extension_registry_projection;
mod agent_feedback_surface;
mod agent_pack_contract;
mod approval_surface;
mod bootstrap_value_utils;
mod carrier_runtime_catalog;
mod carrier_runtime_metadata;
mod carrier_runtime_projection;
mod carrier_runtime_strategy;
mod cli;
mod command_lifecycle_hooks;
mod command_pipeline;
mod compat;
mod compiled_agent_extension_bundle;
mod config_value_utils;
mod consume_final_operator_surface;
mod continuation_binding_summary;
mod contract_profile_adapter;
mod contract_profile_registry;
mod cutover_gate;
mod dev_team_sequence_contract;
mod development_flow_glue;
mod development_flow_orchestration;
mod development_request_analysis;
mod diagnostics_surface;
mod docflow_proxy;
mod docflow_runtime_verdict;
mod docs_surface;
mod doctor_surface;
mod exception_takeover_metadata;
mod external_provider_health;
mod hook_template_registry_projection;
mod host_agent_state;
mod host_runtime_materialization;
mod host_runtime_registry;
mod init;
mod init_surfaces;
mod lane_surface;
mod launcher_activation_snapshot;
mod launcher_task_commands;
mod memory_surface;
mod model_profile_contract;
mod operator_projection_cache;
mod operator_session_projection;
mod orchestrator_session_surface;
mod pack_surface;
mod project_activator_activation_summary;
mod project_activator_agent_extensions_summary;
mod project_activator_host_cli_summary;
mod project_activator_normal_work_defaults;
mod project_activator_runtime_surface;
mod project_activator_surface;
mod project_bootstrap_defaults;
mod project_root_paths;
mod proof_surface;
mod protocol_surface;
mod quality_surface;
mod registry_projection_utils;
mod release1_contracts;
mod release1_operator_output;
mod release_contract_adapters;
mod release_surface;
mod requirement_surface;
mod root_command_router;
mod root_state_binding;
mod runtime_assignment_builder;
mod runtime_assignment_policy;
mod runtime_assignment_projection_utils;
mod runtime_consumption_state;
mod runtime_consumption_surface;
mod runtime_contract_vocab;
mod runtime_dispatch_bootstrap;
mod runtime_dispatch_downstream_packets;
mod runtime_dispatch_execution;
mod runtime_dispatch_lane_completion;
mod runtime_dispatch_packet_text;
mod runtime_dispatch_packets;
mod runtime_dispatch_receipt_helpers;
mod runtime_dispatch_result_evidence;
mod runtime_dispatch_state;
pub(crate) use runtime_dispatch_state::{
    INTERNAL_CODEX_CARRIER_UNAVAILABLE, INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT,
    ModelProfileCatalog, RuntimeAgentLaneDispatch, RuntimeDispatchPacketContext,
    RuntimeDispatchTargetResolution, active_downstream_dispatch_target,
    admissible_selected_backend_for_dispatch_target, agent_init_command_for_packet_path,
    agent_init_execute_command_for_packet_path, apply_dispatch_execution_timeout_to_receipt,
    apply_dispatch_handoff_timeout_to_receipt_for_state_root,
    apply_existing_executed_dispatch_result_to_receipt,
    apply_first_handoff_execution_to_run_graph_status,
    apply_internal_activation_timeout_to_receipt, apply_internal_activation_view_only_to_receipt,
    apply_owned_paths, apply_owned_paths_if_missing, backend_is_admissible_for_dispatch_target,
    backend_is_admissible_or_runtime_selected_carrier_for_dispatch_target,
    backend_policy_dispatch_target_for_resolution, build_downstream_dispatch_receipt,
    build_runtime_closure_admission, build_taskflow_handoff_plan,
    canonical_selected_backend_for_receipt, clear_runtime_consumption_fallback_owned_paths,
    configured_dispatch_backend_class, configured_external_activation_parts,
    configured_external_activation_stdin_payload, configured_external_backend_dispatch_blocker,
    configured_external_backend_entry, configured_external_backend_entry_any,
    current_project_model_profile_catalog_for_root, derive_downstream_dispatch_preview,
    dispatch_activation_evidence_summary, dispatch_execution_route_summary,
    dispatch_execution_started_stale_after_seconds,
    dispatch_handoff_timeout_seconds_for_state_root,
    dispatch_handoff_uses_internal_host_for_state_root,
    dispatch_receipt_has_closure_execution_evidence, dispatch_receipt_has_execution_evidence,
    dispatch_result_stale_after_seconds, dispatch_surface_truth_from_packet_path,
    dispatch_target_runtime_assignment, downstream_activation_fields,
    downstream_dispatch_ready_blocker_parity_error, downstream_selected_backend,
    effective_execution_posture_summary, execute_and_record_dispatch_receipt,
    execute_downstream_dispatch_chain, execute_runtime_dispatch_handoff,
    execution_plan_route_for_dispatch_target, external_backend_profile_projection,
    fallback_backend_for_blocked_primary_dispatch_receipt,
    first_runtime_dispatch_target_after_dev_pack, implementation_owned_paths_for_dispatch_context,
    implementation_owned_paths_for_role_selection, internal_host_activation_view_only_blocker_code,
    internal_host_activation_view_only_requires_terminal_blocker,
    internal_host_dispatch_requires_prelaunch_blocker, internal_host_runtime_window_seconds,
    maybe_bridge_closed_implementer_task_into_latest_receipt,
    maybe_bridge_closed_implementer_task_into_receipt,
    maybe_bridge_closed_implementer_task_into_receipt_with_context,
    maybe_bridge_closed_specification_task_into_latest_receipt,
    maybe_bridge_closed_specification_task_into_receipt,
    maybe_reconcile_blocked_verification_timeout_with_receipt_evidence,
    maybe_reconcile_blocked_verification_timeout_with_receipt_evidence_with_admission,
    model_profile_catalog_from_overlay, normalize_activation_view_only_receipt_truth,
    normalize_persisted_runtime_path, normalize_stale_in_flight_dispatch_receipt,
    planner_metadata_owned_paths_from_role_selection, planner_metadata_owned_paths_from_task,
    policy_dispatch_target_for_admissibility, preferred_selected_backend_for_receipt,
    preferred_selected_model_profile_for_dispatch_target, preview_downstream_dispatch_receipt,
    receipt_waiting_on_specification_evidence,
    reconcile_executed_dispatch_result_state_best_effort, record_dispatch_execution_started,
    refresh_downstream_dispatch_preview, refresh_downstream_dispatch_preview_with_owned_paths,
    render_command_display, resolve_runtime_dispatch_target, resolved_tracked_design_doc_path,
    resolved_tracked_flow_bootstrap_for_scope, route_assignment_catalog_drift_payload,
    route_selected_model_profile_for_backend, runtime_agent_lane_dispatch_for_root,
    runtime_consumption_run_id, runtime_dispatch_command_for_target,
    runtime_dispatch_execution_started_result, runtime_dispatch_packet_has_concrete_owned_paths,
    runtime_dispatch_packet_kind, runtime_dispatch_packet_preview,
    runtime_dispatch_project_root_from_state_root,
    runtime_host_execution_contract_allows_automatic_dispatch_execution,
    runtime_host_execution_contract_for_root, runtime_packet_handoff_task_class,
    runtime_packet_handoff_task_class_for_plan, selected_external_backend_for_system,
    selected_host_cli_system_for_runtime_dispatch, selected_profile_requires_owned_path_guard,
    spec_first_dev_handoff_gate_from_taskflow,
    stale_in_flight_dispatch_timeout_seconds_for_receipt,
    sync_receipt_configured_activation_assignment, sync_receipt_dispatch_handoff_surface,
    try_bridge_bounded_implementer_completion_to_downstream_receipt,
    try_bridge_bounded_specification_completion_to_downstream_receipt,
    validate_runtime_dispatch_packet_contract, write_runtime_dispatch_packet,
    write_runtime_dispatch_result,
};
mod runtime_dispatch_status;
mod runtime_lane_summary;
mod runtime_proof_scope;
mod runtime_web_surface;
mod semantic_route_cache;
mod semantic_routing_features;
mod service_client_cli;
mod session_surface;
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
mod taskflow_closeout;
mod taskflow_consume;
mod taskflow_consume_bundle;
mod taskflow_consume_resume;
mod taskflow_consume_resume_output;
mod taskflow_consume_resume_projection;
mod taskflow_consume_resume_receipt;
mod taskflow_continuation;
mod taskflow_layer4;
mod taskflow_operator_diagnostics;
mod taskflow_packet;
mod taskflow_plan_graph;
mod taskflow_pricing;
mod taskflow_protocol_binding;
mod taskflow_proxy;
mod taskflow_receipt_pack;
mod taskflow_routing;
mod taskflow_run_graph;
mod taskflow_run_graph_task_authority;
mod taskflow_runtime_bundle;
mod taskflow_spec_bootstrap;
mod taskflow_task_bridge;
mod team_flow_authority_adapter;
mod team_flow_state_machine;
mod temp_state;
#[cfg(test)]
mod test_cli_support;
mod vida_client;
#[cfg(test)]
mod vida_client_fixture;
mod vida_client_inprocess;
mod vida_transport_tarpc;
mod vida_tui_shell;
mod zombie_d_gate;

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::contract_profile_adapter::{
    BlockerCode, blocker_code as blocker_code_value, blocker_code_str,
};
use agent_extension_bundle_validation::{
    AgentExtensionBundleValidationInput, extend_agent_extension_bundle_validation_errors,
};
use agent_extension_catalog_projection::build_agent_extension_catalog_projection;
use agent_extension_registry_projection::build_agent_extension_registry_projection;
pub(crate) use bootstrap_value_utils::{
    config_file_path, config_file_path_for_root, inferred_project_title, is_missing_or_placeholder,
    normalize_root_arg, slugify_project_id, trimmed_non_empty,
};
use carrier_runtime_projection::build_carrier_runtime_projection;
use clap::{Parser, error::ErrorKind};
pub(crate) use cli::*;
pub(crate) use compiled_agent_extension_bundle::build_compiled_agent_extension_bundle_for_root;
pub(crate) use config_value_utils::{
    canonical_json_string_array_entries, csv_json_string_list, json_bool, json_lookup,
    json_nonempty_string_array_field, json_string, json_string_list, json_trimmed_string_field,
    json_trimmed_string_field_any, load_project_overlay_yaml, project_overlay_config,
    split_csv_like, yaml_bool, yaml_lookup, yaml_string, yaml_string_list,
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
    docflow_runtime_verdict_next_actions, runtime_blocker_codes_for_docflow_closeout,
};
use hook_template_registry_projection::build_hook_template_registry_projection;
pub(crate) use host_agent_state::{
    HOST_AGENT_OBSERVABILITY_STATE, HostAgentFeedbackInput, HostAgentHandleStateInput,
    PROMPT_LIFECYCLE_STATE, WORKER_SCORECARDS_STATE, WORKER_STRATEGY_STATE,
    append_host_agent_observability_event, host_agent_observability_state_path,
    load_or_initialize_host_agent_observability_state, load_or_initialize_worker_scorecards,
    read_json_file_if_present, record_host_agent_handle_state, refresh_worker_strategy,
    worker_scorecards_state_path, worker_strategy_state_path,
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
pub(crate) use project_activator_surface::ProjectActivationAnswers;
pub(crate) use project_activator_surface::build_project_activator_view;
pub(crate) use project_activator_surface::merge_project_activation_into_init_view;
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
    LaneStatus, derive_lane_status, missing_downstream_lane_evidence_blocker,
};
use root_command_router::run_root_command;
use runtime_assignment_builder::{
    build_runtime_assignment, build_runtime_assignment_from_dispatch_alias,
    build_runtime_assignment_from_resolved_constraints,
    build_runtime_assignment_preview_from_dispatch_alias,
    build_runtime_assignment_preview_from_resolved_constraints, resolve_dispatch_alias_id,
};
use runtime_assignment_policy::{
    declared_task_class_supports_requested, infer_execution_runtime_role, infer_runtime_task_class,
    role_supports_task_class, runtime_role_for_task_class, task_complexity_multiplier,
};
pub(crate) use runtime_assignment_projection_utils::{
    apply_run_graph_runtime_assignment_to_selection, carrier_runtime_section,
    infer_task_class_from_task_payload, json_u64, runtime_assignment_alias_fields,
    runtime_assignment_from_execution_plan,
};
#[allow(unused_imports)]
pub(crate) use runtime_consumption_state::{
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER,
    RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION,
    apply_runtime_consumption_final_dispatch_receipt_blocker,
    latest_admissible_retrieval_trust_signal,
    runtime_consumption_final_dispatch_receipt_blocker_code,
};
pub(crate) use runtime_consumption_state::{
    RuntimeReflexLoopEvidenceRefs, RuntimeReflexLoopRecord, RuntimeReflexLoopStage,
    RuntimeReflexLoopSummary, latest_final_runtime_consumption_dispatch_receipt_summary,
    latest_final_runtime_consumption_snapshot_path,
    latest_recorded_final_runtime_consumption_snapshot_path, latest_runtime_reflex_loop_record,
    latest_terminal_consume_continue_snapshot_run_id,
    runtime_consumption_snapshot_has_release_admission_evidence, runtime_consumption_summary,
    runtime_reflex_loop_record, runtime_reflex_loop_summary, write_runtime_consumption_snapshot,
};
pub(crate) use runtime_consumption_surface::{
    DoctorLauncherSummary, RuntimeConsumptionClosureAdmission, RuntimeConsumptionDocflowActivation,
    RuntimeConsumptionDocflowVerdict, RuntimeConsumptionEvidence, TaskflowConsumeBundleCheck,
    TaskflowConsumeBundlePayload, TaskflowDirectConsumptionPayload, blocking_lane_selection,
    build_docflow_runtime_evidence, doctor_launcher_summary_for_root,
};
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
    dispatch_contract_allowed_next_lane_sequence, dispatch_contract_execution_lane_sequence,
    dispatch_contract_lane, dispatch_contract_lane_activation, dispatch_contract_lane_sequence,
    dispatch_target_for_runtime_role, selected_backend_from_execution_plan_route,
};
use taskflow_runtime_bundle::{
    blocking_runtime_bundle, build_taskflow_consume_bundle_payload, taskflow_consume_bundle_check,
};
use taskflow_spec_bootstrap::{
    execute_taskflow_bootstrap_spec_with_store, execute_work_packet_create_with_store,
};
use time::format_description::well_known::Rfc3339;

const CLI_RUNTIME_THREAD_STACK_BYTES: usize = 32 * 1024 * 1024;

fn main() -> ExitCode {
    bootstrap_windows_host_environment();
    let args = normalized_cli_args();
    match std::thread::Builder::new()
        .name("vida-cli-runtime".to_string())
        .stack_size(CLI_RUNTIME_THREAD_STACK_BYTES)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio runtime should initialize");
            let _runtime_state_dir_parse_guard =
                match root_command_router::prepare_runtime_state_dir_for_parse(&args) {
                    Ok(guard) => guard,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let cli = match parse_cli_or_emit_error(args) {
                Ok(cli) => cli,
                Err(exit_code) => return exit_code,
            };
            runtime.block_on(run_root_command(cli))
        }) {
        Ok(handle) => match handle.join() {
            Ok(code) => code,
            Err(_) => {
                eprintln!("vida CLI runtime thread panicked");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to start vida CLI runtime thread: {error}");
            ExitCode::from(1)
        }
    }
}

fn parse_cli_or_emit_error(args: Vec<OsString>) -> Result<Cli, ExitCode> {
    match Cli::try_parse_from(args.clone()) {
        Ok(cli) => Ok(cli),
        Err(error) => {
            let exit_code = clap_error_exit_code(&error);
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return Err(exit_code);
            }
            if cli_args_request_json(&args) {
                print_json_pretty(&cli_parse_error_payload(&args, &error));
                return Err(exit_code);
            }
            let _ = error.print();
            Err(exit_code)
        }
    }
}

fn clap_error_exit_code(error: &clap::Error) -> ExitCode {
    u8::try_from(error.exit_code())
        .map(ExitCode::from)
        .unwrap_or_else(|_| ExitCode::from(1))
}

fn cli_args_request_json(args: &[OsString]) -> bool {
    args.iter()
        .filter_map(|arg| arg.to_str())
        .any(|arg| arg == "--json")
}

fn cli_parse_error_surface(args: &[OsString]) -> String {
    let command_tokens = args
        .iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .filter(|arg| !arg.starts_with('-'))
        .collect::<Vec<_>>();
    match command_tokens.as_slice() {
        [] => "vida".to_string(),
        ["agent", subcommand, ..] => format!("vida agent {subcommand}"),
        ["lane", subcommand, ..] => format!("vida lane {subcommand}"),
        ["task", subcommand, ..] => format!("vida task {subcommand}"),
        ["taskflow", "run-graph", subcommand, ..] => {
            format!("vida taskflow run-graph {subcommand}")
        }
        ["taskflow", subcommand, ..] => format!("vida taskflow {subcommand}"),
        [command, ..] => format!("vida {command}"),
    }
}

fn cli_parse_error_payload(args: &[OsString], error: &clap::Error) -> serde_json::Value {
    let surface = cli_parse_error_surface(args);
    let help_command = format!("{surface} --help");
    serde_json::json!({
        "surface": surface,
        "status": "blocked",
        "blocker_codes": ["cli_parse_error"],
        "error_kind": format!("{:?}", error.kind()),
        "error": error.to_string().trim(),
        "next_actions": [
            format!("Run `{help_command}` for the required arguments."),
            "Retry the command with the missing required arguments and `--json`."
        ]
    })
}

fn bootstrap_windows_host_environment() {
    #[cfg(windows)]
    bootstrap_windows_host_environment_impl();
}

#[cfg(windows)]
fn bootstrap_windows_host_environment_impl() {
    let system_drive_env = env::var_os("SystemDrive");
    let system_root_env = env::var_os("SystemRoot");
    let windir_env = env::var_os("windir");
    let current_exe = env::current_exe().ok();
    let current_dir = env::current_dir().ok();
    let system_drive = windows_system_drive_from_candidates(&[
        system_drive_env.as_deref(),
        system_root_env.as_deref(),
        windir_env.as_deref(),
        current_exe.as_ref().map(|path| path.as_os_str()),
        current_dir.as_ref().map(|path| path.as_os_str()),
    ]);

    if env::var_os("SystemDrive")
        .filter(|value| !value.is_empty())
        .is_none()
    {
        env::set_var("SystemDrive", &system_drive);
    }
    if env::var_os("ProgramData")
        .filter(|value| !value.is_empty())
        .is_none()
    {
        env::set_var("ProgramData", windows_program_data_path(&system_drive));
    }
}

#[cfg(windows)]
fn windows_system_drive_from_candidates(candidates: &[Option<&std::ffi::OsStr>]) -> String {
    candidates
        .iter()
        .filter_map(|candidate| candidate.and_then(windows_drive_from_path))
        .next()
        .unwrap_or_else(|| "C:".to_string())
}

#[cfg(windows)]
fn windows_program_data_path(system_drive: &str) -> String {
    format!("{}\\ProgramData", system_drive.trim_end_matches('\\'))
}

#[cfg(windows)]
fn windows_drive_from_path(value: &std::ffi::OsStr) -> Option<String> {
    use std::path::{Component, Prefix};

    match std::path::Path::new(value).components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some(format!("{}:", char::from(letter).to_ascii_uppercase()))
            }
            _ => None,
        },
        _ => None,
    }
}

fn normalized_cli_args() -> Vec<OsString> {
    env::args_os().map(normalize_cli_arg).collect()
}

fn normalize_cli_arg(arg: OsString) -> OsString {
    match arg.to_str() {
        Some("--HELP" | "--Help" | "/HELP" | "/Help" | "/help" | "/?") => OsString::from("--help"),
        Some("-H") => OsString::from("-h"),
        _ => arg,
    }
}

#[cfg(test)]
pub(crate) async fn run(cli: Cli) -> ExitCode {
    run_root_command(cli).await
}

pub(crate) use development_flow_orchestration::{
    RuntimeConsumptionLaneSelection, build_runtime_execution_plan_from_snapshot,
    build_runtime_lane_selection_with_store,
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
        assert!(
            harness
                .path()
                .join(DEFAULT_PROJECT_PRODUCT_SPEC_INDEX)
                .is_file()
        );
        assert!(
            harness
                .path()
                .join(DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE)
                .is_file()
        );
        assert!(harness.path().join(DEFAULT_PROJECT_PROCESS_INDEX).is_file());
        assert!(
            harness
                .path()
                .join(DEFAULT_PROJECT_RESEARCH_INDEX)
                .is_file()
        );
        assert!(harness.path().join(".vida/config").is_dir());
        assert!(harness.path().join(".vida/db").is_dir());
        assert!(harness.path().join(".vida/cache").is_dir());
        assert!(harness.path().join(".vida/framework").is_dir());
        assert!(harness.path().join(".vida/project").is_dir());
        assert!(
            harness
                .path()
                .join(".vida/project/agent-extensions/index.md")
                .is_file()
        );
        assert!(
            harness
                .path()
                .join(".vida/project/agent-extensions/roles.yaml")
                .is_file()
        );
        assert!(
            harness
                .path()
                .join(".vida/project/agent-extensions/roles.sidecar.yaml")
                .is_file()
        );
        assert!(harness.path().join(".vida/receipts").is_dir());
        assert!(harness.path().join(".vida/runtime").is_dir());
        assert!(harness.path().join(".vida/scratchpad").is_dir());
        assert!(
            harness
                .path()
                .join("vida/config/instructions/bundles/framework-source")
                .is_dir()
        );
        assert!(
            harness
                .path()
                .join("vida/config/instructions/bundles/framework-memory-source")
                .is_dir()
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_host_environment_bootstrap_derives_program_data_from_system_drive() {
        assert_eq!(
            windows_system_drive_from_candidates(&[
                None,
                Some(std::ffi::OsStr::new("D:\\Windows"))
            ]),
            "D:"
        );
        assert_eq!(windows_program_data_path("D:"), "D:\\ProgramData");
    }

    #[test]
    fn uppercase_help_flag_is_normalized_for_windows_operator_habit() {
        let parsed = Cli::try_parse_from(["vida", "--help"])
            .expect_err("canonical help should render clap display error");
        let argv = [OsString::from("vida"), OsString::from("--HELP")]
            .into_iter()
            .map(normalize_cli_arg);
        let normalized =
            Cli::try_parse_from(argv).expect_err("uppercase help should render clap display error");

        assert_eq!(parsed.kind(), normalized.kind());
        assert!(normalized.to_string().contains("Usage: vida"));
    }
}
