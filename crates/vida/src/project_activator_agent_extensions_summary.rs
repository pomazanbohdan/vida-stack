use std::path::Path;

pub(crate) struct ProjectActivatorAgentExtensionsSummary {
    pub(crate) agent_extensions_enabled: bool,
    pub(crate) agent_extensions_ready: bool,
    pub(crate) agent_extension_validation_error: Option<String>,
    pub(crate) execution_carrier_model: serde_json::Value,
}

pub(crate) fn build_project_activator_agent_extensions_summary(
    project_root: &Path,
    project_overlay: Option<&serde_yaml::Value>,
) -> ProjectActivatorAgentExtensionsSummary {
    let agent_extensions_enabled = project_overlay
        .map(|config| {
            crate::yaml_bool(
                crate::yaml_lookup(config, &["agent_extensions", "enabled"]),
                false,
            )
        })
        .unwrap_or(false);
    let agent_extension_bundle = project_overlay
        .filter(|_| agent_extensions_enabled)
        .map(|config| crate::build_compiled_agent_extension_bundle_for_root(config, project_root));
    let agent_extensions_ready = agent_extension_bundle
        .as_ref()
        .map(|result| result.is_ok())
        .unwrap_or(true);
    let agent_extension_validation_error = agent_extension_bundle
        .as_ref()
        .and_then(|result| result.as_ref().err())
        .cloned();
    let compiled_carrier_runtime = agent_extension_bundle
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(crate::carrier_runtime_section);
    let execution_carrier_model = serde_json::json!({
        "agent_identity": compiled_carrier_runtime
            .and_then(|runtime| runtime["agent_model"]["agent_identity"].as_str().map(str::to_string))
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                project_overlay.and_then(|config| {
                    crate::yaml_string(crate::yaml_lookup(config, &["agent_system", "agent_identity"]))
                })
            })
            .unwrap_or_else(|| "execution_carrier".to_string()),
        "runtime_role_identity": compiled_carrier_runtime
            .and_then(|runtime| {
                runtime["agent_model"]["runtime_role_identity"]
                    .as_str()
                    .map(str::to_string)
            })
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                project_overlay.and_then(|config| {
                    crate::yaml_string(crate::yaml_lookup(config, &["agent_system", "runtime_role_identity"]))
                })
            })
            .unwrap_or_else(|| "activation_state".to_string()),
        "selection_rule": compiled_carrier_runtime
            .and_then(|runtime| runtime["agent_model"]["selection_rule"].as_str().map(str::to_string))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "capability_first_then_score_guard_then_cheapest_tier".to_string()),
        "carrier_catalog_owner": compiled_carrier_runtime
            .and_then(|runtime| {
                runtime["source_of_truth"]["carrier_catalog_owner"]
                    .as_str()
                    .map(str::to_string)
            })
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "vida.config.yaml -> configured host-system carrier surfaces".to_string()),
        "strategy_store": crate::WORKER_STRATEGY_STATE,
        "scorecards_store": crate::WORKER_SCORECARDS_STATE,
        "inspect_commands": {
            "snapshot": "vida taskflow consume agent-system --json",
            "carrier_catalog": "vida taskflow consume agent-system --json | jq '.snapshot.carriers'",
            "runtime_roles": "vida taskflow consume agent-system --json | jq '.snapshot.runtime_roles'",
            "scores": "vida taskflow consume agent-system --json | jq '.snapshot.worker_strategy.agents'",
            "selection_preview": "vida taskflow consume final \"<request>\" --json | jq '.payload.taskflow_handoff_plan.runtime_assignment'"
        }
    });

    ProjectActivatorAgentExtensionsSummary {
        agent_extensions_enabled,
        agent_extensions_ready,
        agent_extension_validation_error,
        execution_carrier_model,
    }
}
