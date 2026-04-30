use std::path::PathBuf;

pub(crate) fn build_project_activator_host_environment(
    supported_host_cli_systems: Vec<String>,
    selected_host_cli_system: Option<String>,
    host_cli_execution_class: Option<String>,
    host_cli_selection_required: bool,
    host_cli_template_materialized: bool,
    host_cli_materialization_required: bool,
    host_cli_runtime_template_root: String,
    host_cli_template_source_root: Option<PathBuf>,
    default_host_agent_templates: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "supported_cli_systems": supported_host_cli_systems,
        "selected_cli_system": selected_host_cli_system,
        "selected_cli_execution_class": host_cli_execution_class,
        "selection_required": host_cli_selection_required,
        "template_materialized": host_cli_template_materialized,
        "materialization_required": host_cli_materialization_required,
        "runtime_template_root": host_cli_runtime_template_root,
        "template_source_root": host_cli_template_source_root
            .map(|path| path.to_string_lossy().replace('\\', "/")),
        "default_host_agent_templates": default_host_agent_templates,
        "configuration_protocols": [
            "runtime-instructions/work.host-cli-agent-setup-protocol"
        ],
    })
}

pub(crate) fn build_project_activator_activation_algorithm() -> serde_json::Value {
    serde_json::json!({
        "mode": "bounded_interview_then_materialize",
        "taskflow_admitted_while_pending": false,
        "non_canonical_taskflow_surfaces_forbidden_while_pending": [
            "vida taskflow",
            "external_taskflow_runtime"
        ],
        "docflow_first": true,
        "docflow_surface": "vida docflow",
        "allowed_activation_surfaces": [
            "vida project-activator",
            "vida docflow",
            "vida protocol view bootstrap/router",
            "vida protocol view runtime-instructions/work.host-cli-agent-setup-protocol"
        ],
        "activation_receipt_glob": ".vida/receipts/project-activation*.json"
    })
}
