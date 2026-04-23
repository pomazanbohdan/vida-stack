use std::{collections::HashMap, path::Path};

use crate::{read_simple_toml_sections, registry_rows_by_key};

pub(crate) struct CarrierRuntimeProjection {
    pub(crate) carrier_runtime: serde_json::Value,
    pub(crate) validation_errors: Vec<String>,
}

fn selected_runtime_root(
    root: &Path,
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
) -> std::path::PathBuf {
    if let Some(system) = selected_host_cli_system {
        if let Some(entry) = host_cli_system_registry.get(system) {
            return crate::project_activator_surface::host_cli_system_runtime_root(
                entry, system, root,
            );
        }
    }

    let mut enabled_entries = host_cli_system_registry
        .iter()
        .filter(|(_, entry)| crate::project_activator_surface::host_cli_system_enabled(entry))
        .collect::<Vec<_>>();
    enabled_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    if let Some((system, entry)) = enabled_entries.first() {
        return crate::project_activator_surface::host_cli_system_runtime_root(entry, system, root);
    }

    let mut entries = host_cli_system_registry.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    if let Some((system, entry)) = entries.first() {
        return crate::project_activator_surface::host_cli_system_runtime_root(entry, system, root);
    }

    root.join(".host-runtime")
}

fn subagent_runtime_candidate_rows(config: &serde_yaml::Value) -> Vec<serde_json::Value> {
    let Some(entries) = crate::yaml_lookup(config, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
    else {
        return Vec::new();
    };

    let mut rows = entries
        .iter()
        .filter_map(|(key, entry)| {
            let backend_id = key.as_str()?.trim();
            if backend_id.is_empty()
                || !crate::yaml_bool(crate::yaml_lookup(entry, &["enabled"]), false)
            {
                return None;
            }
            let backend_class = crate::yaml_string(crate::yaml_lookup(
                entry,
                &["subagent_backend_class"],
            ))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                if backend_id == "internal_subagents" {
                    "internal".to_string()
                } else {
                    "external_cli".to_string()
                }
            });

            let fallback_rate = crate::yaml_string(
                crate::yaml_lookup(entry, &["budget_cost_units"]),
            )
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
            let profile_projection =
                crate::model_profile_contract::normalize_profile_projection_from_yaml(
                    backend_id,
                    entry,
                    Some(fallback_rate),
                    &fallback_runtime_roles,
                    &fallback_task_classes,
                );

            let model_profiles = profile_projection["model_profiles"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let runtime_roles = if fallback_runtime_roles.is_empty() {
                let mut roles = model_profiles
                    .values()
                    .flat_map(|profile| {
                        profile["runtime_roles"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                roles.sort();
                roles.dedup();
                roles
            } else {
                fallback_runtime_roles
            };
            let task_classes = if fallback_task_classes.is_empty() {
                let mut task_classes = model_profiles
                    .values()
                    .flat_map(|profile| {
                        profile["task_classes"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                task_classes.sort();
                task_classes.dedup();
                task_classes
            } else {
                fallback_task_classes
            };
            let default_runtime_role = crate::yaml_string(crate::yaml_lookup(
                entry,
                &["default_runtime_role"],
            ))
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| {
                        profile_projection["default_model_profile"]
                            .as_str()
                            .and_then(|profile_id| profiles.get(profile_id))
                    })
                    .and_then(|profile| {
                        profile["runtime_roles"]
                            .as_array()
                            .and_then(|roles| roles.first())
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                    })
            })
            .or_else(|| runtime_roles.first().cloned())
            .unwrap_or_default();
            let reasoning_band = crate::yaml_string(crate::yaml_lookup(entry, &["reasoning_band"]))
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    profile_projection["current_reasoning_effort"]
                        .as_str()
                        .map(str::to_string)
                })
                .unwrap_or_default();

            Some(serde_json::json!({
                "role_id": backend_id,
                "description": crate::yaml_string(crate::yaml_lookup(entry, &["description"]))
                    .unwrap_or_else(|| format!("{} backend `{backend_id}`", backend_class)),
                "config_file": "",
                "model": profile_projection["model"].clone(),
                "model_provider": profile_projection["model_provider"].clone(),
                "model_reasoning_effort": profile_projection["model_reasoning_effort"].clone(),
                "plan_mode_reasoning_effort": profile_projection["plan_mode_reasoning_effort"].clone(),
                "sandbox_mode": profile_projection["sandbox_mode"].clone(),
                "default_model_profile": profile_projection["default_model_profile"].clone(),
                "model_profiles": profile_projection["model_profiles"].clone(),
                "tier": crate::yaml_string(crate::yaml_lookup(entry, &["orchestration_tier"]))
                    .unwrap_or_else(|| backend_id.to_string()),
                "rate": fallback_rate,
                "normalized_cost_units": profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| {
                        profile_projection["default_model_profile"]
                            .as_str()
                            .and_then(|profile_id| profiles.get(profile_id))
                    })
                    .and_then(|profile| profile["normalized_cost_units"].as_u64())
                    .unwrap_or(fallback_rate),
                "reasoning_band": reasoning_band,
                "default_runtime_role": default_runtime_role,
                "runtime_roles": runtime_roles,
                "task_classes": task_classes,
                "backend_class": backend_class,
                "carrier_kind": format!("{backend_class}_backend"),
                "write_scope": crate::yaml_string(crate::yaml_lookup(entry, &["write_scope"])).unwrap_or_default(),
                "speed_tier": profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| {
                        profile_projection["default_model_profile"]
                            .as_str()
                            .and_then(|profile_id| profiles.get(profile_id))
                    })
                    .and_then(|profile| profile["speed_tier"].as_str())
                    .unwrap_or_default(),
                "quality_tier": profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| {
                        profile_projection["default_model_profile"]
                            .as_str()
                            .and_then(|profile_id| profiles.get(profile_id))
                    })
                    .and_then(|profile| profile["quality_tier"].as_str())
                    .unwrap_or_default(),
            }))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["role_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["role_id"].as_str().unwrap_or_default())
    });
    rows
}

pub(crate) fn build_carrier_runtime_projection(
    config: &serde_yaml::Value,
    root: &Path,
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
    dispatch_aliases_registry: &serde_yaml::Value,
    dispatch_aliases_path: Option<&str>,
) -> CarrierRuntimeProjection {
    let runtime_root =
        selected_runtime_root(root, selected_host_cli_system, host_cli_system_registry);
    let runtime_config = read_simple_toml_sections(&runtime_root.join("config.toml"));
    let mut carrier_roles =
        crate::carrier_runtime_catalog::resolved_carrier_roles(config, &runtime_root);
    carrier_roles.extend(subagent_runtime_candidate_rows(config));
    let dispatch_alias_rows = registry_rows_by_key(
        dispatch_aliases_registry,
        "dispatch_aliases",
        "alias_id",
        &[],
    );
    let carrier_dispatch_aliases = crate::carrier_runtime_catalog::materialized_dispatch_aliases(
        config,
        &dispatch_alias_rows,
        &carrier_roles,
    );
    let scoring_policy = serde_json::to_value(
        crate::yaml_lookup(config, &["agent_system", "scoring"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let worker_strategy = crate::carrier_runtime_strategy::resolved_worker_strategy(
        root,
        &carrier_roles,
        &scoring_policy,
    );
    let pricing_policy = crate::carrier_runtime_strategy::resolved_pricing_policy(
        config,
        &carrier_roles,
        &worker_strategy,
    );
    let model_selection = serde_json::to_value(
        crate::yaml_lookup(config, &["agent_system", "model_selection"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);

    let mut validation_errors =
        crate::carrier_runtime_catalog::carrier_role_validation_errors(&carrier_roles);
    validation_errors.extend(
        crate::carrier_runtime_catalog::carrier_dispatch_alias_validation_errors(
            &carrier_dispatch_aliases,
        ),
    );

    CarrierRuntimeProjection {
        carrier_runtime: serde_json::json!({
            "enabled": runtime_config
                .get("features")
                .and_then(|section| section.get("multi_agent"))
                .map(|value| value == "true")
                .unwrap_or(false),
            "max_threads": runtime_config
                .get("agents")
                .and_then(|section| section.get("max_threads"))
                .cloned()
                .unwrap_or_default(),
            "max_depth": runtime_config
                .get("agents")
                .and_then(|section| section.get("max_depth"))
                .cloned()
                .unwrap_or_default(),
            "materialization_mode": crate::carrier_runtime_metadata::carrier_runtime_materialization_mode(
                selected_host_cli_system,
                host_cli_system_registry,
            ),
            "roles": carrier_roles,
            "dispatch_aliases": carrier_dispatch_aliases,
            "source_of_truth": crate::carrier_runtime_metadata::carrier_runtime_source_of_truth(
                selected_host_cli_system,
                dispatch_alias_rows.is_empty(),
                dispatch_aliases_path,
            ),
            "agent_model": crate::carrier_runtime_metadata::carrier_runtime_agent_model(
                config,
                &worker_strategy,
            ),
            "worker_strategy": worker_strategy,
            "pricing_policy": pricing_policy,
            "model_selection": model_selection,
        }),
        validation_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::selected_runtime_root;
    use super::subagent_runtime_candidate_rows;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn selected_runtime_root_prefers_explicit_system_from_registry() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    codex:
      enabled: true
      runtime_root: .codex
    hermes:
      enabled: true
      runtime_root: .hermes
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_with_fallback(Some(&config));

        let root = selected_runtime_root(Path::new("/tmp/project"), Some("hermes"), &registry);

        assert_eq!(root, Path::new("/tmp/project/.hermes"));
    }

    #[test]
    fn selected_runtime_root_uses_first_enabled_system_without_hardcoded_internal_default() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    hermes:
      enabled: true
      runtime_root: .hermes
    opencode:
      enabled: false
      runtime_root: .opencode
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_with_fallback(Some(&config));

        let root = selected_runtime_root(Path::new("/tmp/project"), None, &registry);

        assert_eq!(root, Path::new("/tmp/project/.hermes"));
    }

    #[test]
    fn selected_runtime_root_has_stable_neutral_fallback_without_registry() {
        let root = selected_runtime_root(Path::new("/tmp/project"), None, &HashMap::new());

        assert_eq!(root, Path::new("/tmp/project/.host-runtime"));
    }

    #[test]
    fn subagent_runtime_candidate_rows_preserve_profile_only_backend_projection() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
      default_model_profile: opencode_minimax_free_review
      budget_cost_units: 0
      model_profiles:
        opencode_minimax_free_review:
          provider: opencode
          model_ref: opencode/minimax-m2.5-free
          reasoning_effort: provider_default
          normalized_cost_units: 0
          speed_tier: fast
          quality_tier: medium
          write_scope: none
          runtime_roles:
            - coach
            - verifier
          task_classes:
            - review
            - verification
          readiness:
            mode: external_cli_profile
            required: true
"#,
        )
        .expect("config should parse");

        let rows = subagent_runtime_candidate_rows(&config);

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["role_id"], "opencode_cli");
        assert_eq!(row["default_model_profile"], "opencode_minimax_free_review");
        assert_eq!(row["model"], "opencode/minimax-m2.5-free");
        assert_eq!(row["model_provider"], "opencode");
        assert_eq!(row["model_reasoning_effort"], "provider_default");
        assert_eq!(row["normalized_cost_units"], 0);
        assert_eq!(row["default_runtime_role"], "coach");
        assert_eq!(row["runtime_roles"], json!(["coach", "verifier"]));
        assert_eq!(row["task_classes"], json!(["review", "verification"]));
        assert_eq!(row["speed_tier"], "fast");
        assert_eq!(row["quality_tier"], "medium");
        assert_eq!(
            row["model_profiles"]["opencode_minimax_free_review"]["normalized_cost_units"],
            0
        );
    }

    #[test]
    fn subagent_runtime_candidate_rows_include_internal_model_profiles() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      budget_cost_units: 10
      runtime_roles:
        - worker
        - verifier
      task_classes:
        - implementation
        - verification
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: internal_fast
          reasoning_effort: low
          normalized_cost_units: 6
          speed_tier: fast
          quality_tier: medium_high
          write_scope: orchestrator_native
          runtime_roles:
            - worker
          task_classes:
            - implementation
        internal_review:
          provider: internal
          model_ref: internal_review
          reasoning_effort: high
          normalized_cost_units: 8
          speed_tier: medium
          quality_tier: high
          write_scope: read_only
          runtime_roles:
            - verifier
          task_classes:
            - verification
"#,
        )
        .expect("config should parse");

        let rows = subagent_runtime_candidate_rows(&config);

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["role_id"], "internal_subagents");
        assert_eq!(row["backend_class"], "internal");
        assert_eq!(row["carrier_kind"], "internal_backend");
        assert_eq!(row["default_model_profile"], "internal_fast");
        assert_eq!(row["model"], "internal_fast");
        assert_eq!(
            row["model_profiles"]["internal_fast"]["model_ref"],
            "internal_fast"
        );
        assert_eq!(
            row["model_profiles"]["internal_review"]["model_ref"],
            "internal_review"
        );
    }
}
