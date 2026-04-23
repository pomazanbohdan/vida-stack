use std::path::Path;

use crate::host_runtime_materialization::host_runtime_dispatch_alias_catalog_for_root;
use crate::status_surface_external_cli::external_cli_preflight_summary;
use crate::status_surface_host_cli_summary::{
    host_cli_system_carrier_summary, host_cli_system_entry_summary,
};
use crate::status_surface_host_cli_system::{
    runtime_root_for_selected_system, runtime_surface_for_selected_system,
    selected_host_cli_system_entry,
};

pub(crate) fn build_host_agent_status_summary(project_root: &Path) -> Option<serde_json::Value> {
    let overlay = crate::project_activator_surface::read_yaml_file_checked(
        &project_root.join("vida.config.yaml"),
    )
    .ok()?;
    let (selected_cli_system, host_cli_entry) = selected_host_cli_system_entry(&overlay);
    let runtime_surface =
        runtime_surface_for_selected_system(&selected_cli_system, host_cli_entry.as_ref());
    let observability =
        crate::read_json_file_if_present(&crate::host_agent_observability_state_path(project_root))
            .unwrap_or_else(|| {
                crate::load_or_initialize_host_agent_observability_state(project_root)
            });
    let latest_events = observability["events"]
        .as_array()
        .map(|events| events.iter().rev().take(5).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let latest_event = latest_events
        .first()
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let recent_events_value = serde_json::Value::Array(latest_events);
    let budget_value = observability["budget"].clone();
    let runtime_root = runtime_root_for_selected_system(
        project_root,
        &selected_cli_system,
        host_cli_entry.as_ref(),
    );
    let external_cli_preflight =
        external_cli_preflight_summary(&overlay, &selected_cli_system, host_cli_entry.as_ref());
    let hybrid_external_cli_relevant =
        external_cli_preflight["hybrid_external_cli_relevant"].clone();
    let selected_execution_class = external_cli_preflight["selected_execution_class"].clone();
    let requires_external_cli = external_cli_preflight["requires_external_cli"].clone();
    let route_primary_external_backends =
        external_cli_preflight["route_primary_external_backends"].clone();
    let blocked_primary_backends = external_cli_preflight["blocked_primary_backends"].clone();
    let hybrid_external_cli_relevant_flag = hybrid_external_cli_relevant.as_bool().unwrap_or(false);
    let effective_execution_posture = match (
        selected_execution_class.as_str(),
        hybrid_external_cli_relevant_flag,
    ) {
        (Some("external"), _) => "external_only",
        (_, true) => "hybrid_external_cli",
        (Some("internal"), _) => "internal_only",
        _ => "unknown",
    };

    let mut payload = serde_json::Map::new();
    payload.insert(
        "host_cli_system".to_string(),
        serde_json::Value::String(selected_cli_system.clone()),
    );
    payload.insert(
        "runtime_surface".to_string(),
        serde_json::Value::String(runtime_surface),
    );
    payload.insert(
        "runtime_root".to_string(),
        serde_json::Value::String(runtime_root),
    );
    payload.insert(
        "external_cli_preflight".to_string(),
        external_cli_preflight.clone(),
    );
    payload.insert(
        "hybrid_external_cli_relevant".to_string(),
        hybrid_external_cli_relevant.clone(),
    );
    payload.insert(
        "effective_execution_posture".to_string(),
        external_cli_preflight["effective_execution_posture"].clone(),
    );
    payload.insert(
        "mixed_posture".to_string(),
        external_cli_preflight["mixed_posture"].clone(),
    );
    payload.insert(
        "mixed_posture_details".to_string(),
        serde_json::json!({
            "selected_cli_system": selected_cli_system.clone(),
            "selected_execution_class": selected_execution_class.clone(),
            "effective_execution_posture": effective_execution_posture,
            "requires_external_cli": requires_external_cli,
            "hybrid_external_cli_relevant": hybrid_external_cli_relevant_flag,
            "route_primary_external_backends": route_primary_external_backends,
            "blocked_primary_backends": blocked_primary_backends,
        }),
    );
    payload.insert("budget".to_string(), budget_value);
    payload.insert("recent_events".to_string(), recent_events_value);
    payload.insert(
        "latest_feedback_event".to_string(),
        latest_event["feedback_event"].clone(),
    );
    payload.insert(
        "latest_evaluation_baseline".to_string(),
        latest_event["evaluation_baseline"].clone(),
    );
    payload.insert(
        "latest_prompt_lifecycle_baseline".to_string(),
        latest_event["prompt_lifecycle_baseline"].clone(),
    );
    payload.insert(
        "latest_safety_baseline".to_string(),
        latest_event["safety_baseline"].clone(),
    );
    payload.insert("selection_policy".to_string(), serde_json::Value::Null);
    payload.insert(
        "model_selection".to_string(),
        serde_json::to_value(
            crate::yaml_lookup(&overlay, &["agent_system", "model_selection"])
                .cloned()
                .unwrap_or(serde_yaml::Value::Null),
        )
        .unwrap_or(serde_json::Value::Null),
    );
    payload.insert("agents".to_string(), serde_json::json!({}));
    payload.insert("subagent_backends".to_string(), serde_json::json!({}));
    payload.insert(
        "internal_dispatch_alias_count".to_string(),
        serde_json::Value::Null,
    );
    payload.insert(
        "internal_dispatch_alias_load_error".to_string(),
        serde_json::Value::Null,
    );
    payload.insert(
        "system_entry".to_string(),
        host_cli_system_entry_summary(host_cli_entry.as_ref(), &selected_cli_system),
    );

    let carrier_catalog =
        crate::project_activator_surface::resolved_host_cli_agent_catalog_for_root(
            project_root,
            &overlay,
        )
        .map(|(_, catalog)| catalog)
        .unwrap_or_default();
    let strategy =
        crate::read_json_file_if_present(&crate::worker_strategy_state_path(project_root))
            .unwrap_or(serde_json::Value::Null);
    let scorecards =
        crate::read_json_file_if_present(&crate::worker_scorecards_state_path(project_root))
            .unwrap_or(serde_json::Value::Null);

    let mut agents = serde_json::Map::new();
    for role in &carrier_catalog {
        let Some(role_id) = role["role_id"].as_str() else {
            continue;
        };
        let feedback_rows = scorecards["agents"][role_id]["feedback"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let last_feedback = feedback_rows
            .last()
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        agents.insert(
            role_id.to_string(),
            serde_json::json!({
                "tier": role["tier"],
                "rate": role["rate"],
                "normalized_cost_units": role["normalized_cost_units"],
                "reasoning_band": role["reasoning_band"],
                "model": role["model"],
                "model_provider": role["model_provider"],
                "model_reasoning_effort": role["model_reasoning_effort"],
                "plan_mode_reasoning_effort": role["plan_mode_reasoning_effort"],
                "sandbox_mode": role["sandbox_mode"],
                "default_model_profile": role["default_model_profile"],
                "current_model_ref": role["current_model_ref"],
                "current_reasoning_effort": role["current_reasoning_effort"],
                "model_profiles": role["model_profiles"],
                "default_runtime_role": role["default_runtime_role"],
                "runtime_roles": role["runtime_roles"],
                "task_classes": role["task_classes"],
                "feedback_count": feedback_rows.len(),
                "last_feedback_at": last_feedback["recorded_at"],
                "last_feedback_outcome": last_feedback["outcome"],
                "effective_score": strategy["agents"][role_id]["effective_score"],
                "lifecycle_state": strategy["agents"][role_id]["lifecycle_state"],
            }),
        );
    }
    if agents.is_empty() {
        agents = host_cli_system_carrier_summary(host_cli_entry.as_ref());
    }

    payload.insert(
        "selection_policy".to_string(),
        strategy["selection_policy"].clone(),
    );
    payload.insert(
        "agents".to_string(),
        serde_json::Value::Object(agents.clone()),
    );
    let mut subagent_backends = serde_json::Map::new();
    if let Some(entries) = crate::yaml_lookup(&overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
    {
        for (key, entry) in entries {
            let Some(backend_id) = key
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            if !crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false) {
                continue;
            }
            let fallback_rate =
                crate::yaml_string(crate::yaml_lookup(entry, &["budget_cost_units"]))
                    .and_then(|raw| raw.parse::<u64>().ok())
                    .or_else(|| {
                        crate::yaml_string(crate::yaml_lookup(entry, &["normalized_cost_units"]))
                            .and_then(|raw| raw.parse::<u64>().ok())
                    })
                    .or_else(|| {
                        crate::yaml_string(crate::yaml_lookup(entry, &["rate"]))
                            .and_then(|raw| raw.parse::<u64>().ok())
                    })
                    .unwrap_or(0);
            let fallback_runtime_roles =
                crate::yaml_string_list(crate::yaml_lookup(entry, &["runtime_roles"]));
            let fallback_task_classes =
                crate::yaml_string_list(crate::yaml_lookup(entry, &["task_classes"]));
            let projection = crate::model_profile_contract::normalize_profile_projection_from_yaml(
                backend_id,
                entry,
                Some(fallback_rate),
                &fallback_runtime_roles,
                &fallback_task_classes,
            );
            subagent_backends.insert(
                backend_id.to_string(),
                serde_json::json!({
                    "backend_class": crate::yaml_lookup(entry, &["subagent_backend_class"]).and_then(serde_yaml::Value::as_str).unwrap_or_default(),
                    "orchestration_tier": crate::yaml_lookup(entry, &["orchestration_tier"]).and_then(serde_yaml::Value::as_str).unwrap_or_default(),
                    "budget_cost_units": fallback_rate,
                    "write_scope": crate::yaml_lookup(entry, &["write_scope"]).and_then(serde_yaml::Value::as_str).unwrap_or_default(),
                    "default_model_profile": projection["default_model_profile"],
                    "current_model_ref": projection["current_model_ref"],
                    "current_reasoning_effort": projection["current_reasoning_effort"],
                    "model_profiles": projection["model_profiles"],
                }),
            );
        }
    }
    payload.insert(
        "subagent_backends".to_string(),
        serde_json::Value::Object(subagent_backends),
    );
    let overlay_dispatch_aliases_result =
        host_runtime_dispatch_alias_catalog_for_root(&overlay, project_root, &carrier_catalog);
    let internal_dispatch_alias_load_error = overlay_dispatch_aliases_result
        .as_ref()
        .err()
        .map(std::string::ToString::to_string);
    let overlay_dispatch_aliases = overlay_dispatch_aliases_result.unwrap_or_default();
    payload.insert(
        "internal_dispatch_alias_count".to_string(),
        serde_json::json!(overlay_dispatch_aliases.len()),
    );
    payload.insert(
        "internal_dispatch_alias_load_error".to_string(),
        internal_dispatch_alias_load_error
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    payload.insert(
        "stores".to_string(),
        serde_json::json!({
            "scorecards": if strategy.is_null() { serde_json::Value::Null } else { serde_json::Value::String(crate::WORKER_SCORECARDS_STATE.to_string()) },
            "strategy": if strategy.is_null() { serde_json::Value::Null } else { serde_json::Value::String(crate::WORKER_STRATEGY_STATE.to_string()) },
            "observability": crate::HOST_AGENT_OBSERVABILITY_STATE,
            "prompt_lifecycle": crate::PROMPT_LIFECYCLE_STATE,
        }),
    );
    Some(serde_json::Value::Object(payload))
}

#[cfg(test)]
mod tests {
    use super::build_host_agent_status_summary;
    use std::path::Path;

    #[test]
    fn build_host_agent_status_summary_exposes_hybrid_external_cli_relevance() {
        let summary =
            build_host_agent_status_summary(Path::new("/home/unnamed/project/vida-stack"))
                .expect("host agent summary should render");
        assert_eq!(summary["host_cli_system"], "codex");
        assert_eq!(summary["hybrid_external_cli_relevant"], true);
        assert_eq!(
            summary["external_cli_preflight"]["hybrid_external_cli_relevant"],
            true
        );
        assert_eq!(summary["effective_execution_posture"], "mixed");
        assert_eq!(summary["mixed_posture"], true);
        assert_eq!(summary["model_selection"]["enabled"], true);
        assert!(summary["agents"]["junior"]["default_model_profile"]
            .as_str()
            .is_some());
        assert_eq!(summary["agents"]["senior"]["model"], "gpt-5.3-codex-spark");
        assert_eq!(
            summary["subagent_backends"]["internal_subagents"]["default_model_profile"],
            "internal_fast"
        );
    }

    #[test]
    fn project_config_exposes_four_internal_and_three_external_agent_surfaces() {
        let project_root = Path::new("/home/unnamed/project/vida-stack");
        let overlay = crate::project_activator_surface::read_yaml_file_checked(
            &project_root.join("vida.config.yaml"),
        )
        .expect("project config should exist");

        let codex_carriers = crate::yaml_lookup(
            &overlay,
            &["host_environment", "systems", "codex", "carriers"],
        )
        .and_then(serde_yaml::Value::as_mapping)
        .expect("codex carriers should be configured");
        assert_eq!(codex_carriers.len(), 4);

        let enabled_external_systems =
            crate::yaml_lookup(&overlay, &["host_environment", "systems"])
                .and_then(serde_yaml::Value::as_mapping)
                .expect("host systems should be configured")
                .iter()
                .filter_map(|(key, entry)| {
                    let system_id = key.as_str()?;
                    let enabled = crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false);
                    let execution_class = crate::yaml_lookup(entry, &["execution_class"])
                        .and_then(serde_yaml::Value::as_str)
                        .unwrap_or_default();
                    if enabled && execution_class == "external" {
                        Some(system_id.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
        assert_eq!(enabled_external_systems, vec!["hermes", "opencode"]);

        let enabled_external_backends =
            crate::yaml_lookup(&overlay, &["agent_system", "subagents"])
                .and_then(serde_yaml::Value::as_mapping)
                .expect("subagents should be configured")
                .iter()
                .filter_map(|(key, entry)| {
                    let backend_id = key.as_str()?;
                    let enabled = crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false);
                    let backend_class = crate::yaml_lookup(entry, &["subagent_backend_class"])
                        .and_then(serde_yaml::Value::as_str)
                        .unwrap_or_default();
                    if enabled && backend_class == "external_cli" {
                        Some(backend_id.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
        assert_eq!(
            enabled_external_backends,
            vec!["hermes_cli", "opencode_cli", "kilo_cli", "vibe_cli"]
        );
    }
}
