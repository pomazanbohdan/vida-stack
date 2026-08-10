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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        build_project_activator_activation_algorithm, build_project_activator_host_environment,
    };

    #[test]
    fn project_activator_host_environment_preserves_selection_and_normalizes_source_path() {
        let environment = build_project_activator_host_environment(
            vec!["codex".to_string(), "claude".to_string()],
            Some("codex".to_string()),
            Some("internal".to_string()),
            true,
            false,
            true,
            "runtime/codex".to_string(),
            Some(PathBuf::from(r"templates\codex")),
            vec!["default".to_string()],
        );

        assert_eq!(
            environment["supported_cli_systems"],
            serde_json::json!(["codex", "claude"])
        );
        assert_eq!(environment["selected_cli_system"], "codex");
        assert_eq!(environment["selected_cli_execution_class"], "internal");
        assert!(environment["selection_required"].as_bool().unwrap());
        assert!(!environment["template_materialized"].as_bool().unwrap());
        assert!(environment["materialization_required"].as_bool().unwrap());
        assert_eq!(environment["runtime_template_root"], "runtime/codex");
        assert_eq!(environment["template_source_root"], "templates/codex");
        assert_eq!(
            environment["default_host_agent_templates"],
            serde_json::json!(["default"])
        );
        assert_eq!(
            environment["configuration_protocols"],
            serde_json::json!(["runtime-instructions/work.host-cli-agent-setup-protocol"])
        );

        let no_source = build_project_activator_host_environment(
            Vec::new(),
            None,
            None,
            false,
            false,
            false,
            String::new(),
            None,
            Vec::new(),
        );
        assert!(no_source["template_source_root"].is_null());
    }

    #[test]
    fn project_activator_activation_algorithm_remains_fail_closed_while_pending() {
        let algorithm = build_project_activator_activation_algorithm();

        assert_eq!(algorithm["mode"], "bounded_interview_then_materialize");
        assert!(
            !algorithm["taskflow_admitted_while_pending"]
                .as_bool()
                .unwrap()
        );
        assert!(algorithm["docflow_first"].as_bool().unwrap());
        assert_eq!(algorithm["docflow_surface"], "vida docflow");
        assert_eq!(
            algorithm["activation_receipt_glob"],
            ".vida/receipts/project-activation*.json"
        );
        assert_eq!(
            algorithm["non_canonical_taskflow_surfaces_forbidden_while_pending"],
            serde_json::json!(["vida taskflow", "external_taskflow_runtime"])
        );
        assert_eq!(
            algorithm["allowed_activation_surfaces"],
            serde_json::json!([
                "vida project-activator",
                "vida docflow",
                "vida protocol view bootstrap/router",
                "vida protocol view runtime-instructions/work.host-cli-agent-setup-protocol"
            ])
        );
    }
}
