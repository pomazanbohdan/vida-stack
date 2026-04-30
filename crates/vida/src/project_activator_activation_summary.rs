use std::path::Path;

use crate::project_activator_surface::{host_cli_display_name, inferred_project_id_candidate};

pub(crate) struct ProjectActivatorActivationSummary {
    pub(crate) activation_pending: bool,
    pub(crate) execution_posture_ambiguous: bool,
    pub(crate) sidecar_or_project_docs_too_thin: bool,
    pub(crate) required_inputs: Vec<serde_json::Value>,
    pub(crate) one_shot_example: String,
    pub(crate) next_steps: Vec<String>,
}

pub(crate) struct ProjectActivatorActivationInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) bootstrap_missing: bool,
    pub(crate) sidecar_missing: bool,
    pub(crate) sidecar_has_placeholders: bool,
    pub(crate) docs_missing: bool,
    pub(crate) config_has_placeholders: bool,
    pub(crate) current_project_id: Option<&'a str>,
    pub(crate) current_user_communication_language: Option<&'a str>,
    pub(crate) current_reasoning_language: Option<&'a str>,
    pub(crate) current_documentation_language: Option<&'a str>,
    pub(crate) current_todo_protocol_language: Option<&'a str>,
    pub(crate) host_cli_selection_required: bool,
    pub(crate) host_cli_materialization_required: bool,
    pub(crate) host_cli_suggested_system: &'a str,
    pub(crate) host_cli_supported_list: &'a str,
    pub(crate) supported_host_cli_systems: &'a [String],
    pub(crate) selected_host_cli_system: Option<&'a str>,
    pub(crate) agent_extensions_enabled: bool,
    pub(crate) runtime_agent_extensions_missing: bool,
    pub(crate) agent_extensions_ready: bool,
    pub(crate) agent_extension_validation_error: Option<&'a str>,
}

pub(crate) fn build_project_activator_activation_summary(
    input: ProjectActivatorActivationInputs<'_>,
) -> ProjectActivatorActivationSummary {
    let sidecar_or_project_docs_too_thin =
        input.sidecar_missing || input.sidecar_has_placeholders || input.docs_missing;
    let execution_posture_ambiguous = input.bootstrap_missing
        || input.sidecar_missing
        || input.config_has_placeholders
        || input.host_cli_selection_required
        || input.host_cli_materialization_required
        || input.sidecar_has_placeholders
        || input.docs_missing
        || !input.agent_extensions_ready;
    let activation_pending = input.bootstrap_missing
        || input.sidecar_missing
        || input.config_has_placeholders
        || input.host_cli_selection_required
        || input.host_cli_materialization_required
        || input.sidecar_has_placeholders
        || input.docs_missing
        || (input.agent_extensions_enabled
            && (input.runtime_agent_extensions_missing || !input.agent_extensions_ready));

    let project_id_missing =
        crate::is_missing_or_placeholder(input.current_project_id, crate::PROJECT_ID_PLACEHOLDER);
    let user_communication_language_missing = crate::is_missing_or_placeholder(
        input.current_user_communication_language,
        crate::USER_COMMUNICATION_PLACEHOLDER,
    );
    let reasoning_language_missing = crate::is_missing_or_placeholder(
        input.current_reasoning_language,
        crate::REASONING_LANGUAGE_PLACEHOLDER,
    );
    let documentation_language_missing = crate::is_missing_or_placeholder(
        input.current_documentation_language,
        crate::DOCUMENTATION_LANGUAGE_PLACEHOLDER,
    );
    let todo_protocol_language_missing = crate::is_missing_or_placeholder(
        input.current_todo_protocol_language,
        crate::TODO_PROTOCOL_LANGUAGE_PLACEHOLDER,
    );

    let inferred_project_id = inferred_project_id_candidate(input.project_root);
    let mut required_inputs = Vec::new();
    if project_id_missing {
        required_inputs.push(serde_json::json!({
            "id": "project_id",
            "prompt": "What project id should VIDA record for this repository?",
            "flag": "--project-id",
            "suggested_value": inferred_project_id,
            "required": true
        }));
    }
    if user_communication_language_missing
        || reasoning_language_missing
        || documentation_language_missing
        || todo_protocol_language_missing
    {
        required_inputs.push(serde_json::json!({
            "id": "language",
            "prompt": "Which language should VIDA use by default for user communication, reasoning, documentation, and todo protocol?",
            "flag": "--language",
            "suggested_value": input.current_user_communication_language
                .filter(|value| !crate::is_missing_or_placeholder(Some(value), crate::USER_COMMUNICATION_PLACEHOLDER))
                .unwrap_or("english"),
            "required": true,
            "covers": [
                "language_policy.user_communication",
                "language_policy.reasoning",
                "language_policy.documentation",
                "language_policy.todo_protocol"
            ]
        }));
    }
    if input.host_cli_selection_required {
        required_inputs.push(serde_json::json!({
            "id": "host_cli_system",
            "prompt": "Which supported host CLI system should VIDA activate for agents in this project?",
            "flag": "--host-cli-system",
            "suggested_value": input.host_cli_suggested_system,
            "supported_values": input.supported_host_cli_systems,
            "required": true
        }));
    }

    let mut one_shot_example_parts = vec!["vida project-activator".to_string()];
    if project_id_missing {
        one_shot_example_parts.push("--project-id <project-id>".to_string());
    }
    if user_communication_language_missing
        || reasoning_language_missing
        || documentation_language_missing
        || todo_protocol_language_missing
    {
        one_shot_example_parts.push("--language <language>".to_string());
    }
    if input.host_cli_selection_required {
        one_shot_example_parts.push(format!(
            "--host-cli-system {}",
            input.host_cli_suggested_system
        ));
    }
    one_shot_example_parts.push("--json".to_string());
    let one_shot_example = one_shot_example_parts.join(" ");

    let mut next_steps = Vec::new();
    if input.bootstrap_missing || input.sidecar_missing {
        next_steps.push(
            "run `vida init` in the project root to materialize bootstrap carriers".to_string(),
        );
    }
    if input.config_has_placeholders {
        next_steps.push(
            "run `vida project-activator --repair --json` or the bounded one-shot activation command to record project identity, language policy, docs roots, and host CLI setup before normal work"
                .to_string(),
        );
    }
    if input.host_cli_selection_required {
        next_steps.push(format!(
            "choose the host CLI system from the supported host CLI list ({}) and run the one-shot `vida project-activator` activation command; project activation is not complete until the host agent template is selected",
            input.host_cli_supported_list
        ));
    } else if input.host_cli_materialization_required {
        if let Some(selected_system) = input.selected_host_cli_system {
            let display_name = host_cli_display_name(selected_system);
            next_steps.push(format!(
                "materialize the selected host CLI template with `vida project-activator --repair --host-cli-system {selected_system}`, then close and restart {display_name} so agent configuration becomes visible to the runtime environment",
            ));
        } else {
            next_steps.push(
                "materialize the selected host CLI template with `vida project-activator --host-cli-system <host>` and restart the host CLI so the activated agent template becomes visible to the runtime environment"
                    .to_string(),
            );
        }
    }
    if input.sidecar_has_placeholders {
        next_steps.push(
            "replace placeholder project instruction/docs pointers in `AGENTS.sidecar.md`, or run `vida project-activator --repair --json` when safe defaults are acceptable"
                .to_string(),
        );
    }
    if input.docs_missing {
        next_steps.push(
            "materialize the minimum project-doc roots with `vida project-activator --repair --json` or record an explicit activation override"
                .to_string(),
        );
    }
    if input.agent_extensions_enabled && input.runtime_agent_extensions_missing {
        next_steps.push(
            "repair `.vida/project/agent-extensions/**` with `vida init` so runtime-owned role/skill/profile/flow projections and sidecars exist".to_string(),
        );
    }
    if let Some(error) = input.agent_extension_validation_error {
        next_steps.push(format!(
            "resolve agent-extension validation drift under `.vida/project/agent-extensions/**`: {error}"
        ));
    }
    if next_steps.is_empty() {
        next_steps.push(
            "activation looks ready enough for normal orchestrator and worker initialization"
                .to_string(),
        );
    }

    ProjectActivatorActivationSummary {
        activation_pending,
        execution_posture_ambiguous,
        sidecar_or_project_docs_too_thin,
        required_inputs,
        one_shot_example,
        next_steps,
    }
}
