use std::collections::BTreeMap;

/// Returns the generic carrier tier used for capability matching.
///
/// `carrier_tier` is the canonical field.  `tier` remains the concrete/provider
/// tier for backwards-compatible rates and status projections.
pub(crate) fn canonical_carrier_tier(row: &serde_json::Value) -> Option<String> {
    row.get("carrier_tier")
        .and_then(serde_json::Value::as_str)
        .or_else(|| row.get("tier").and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn concrete_carrier_tier(row: &serde_json::Value) -> Option<String> {
    row.get("concrete_tier")
        .and_then(serde_json::Value::as_str)
        .or_else(|| row.get("tier").and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// Expands the canonical framework template into a deterministic system/tier/alias
/// relation matrix. Every cross-product row is retained; incompatibilities are
/// represented as diagnostics so capability gaps cannot disappear in projection.
pub(crate) fn master_template_dispatch_alias_matrix(
    template: &serde_yaml::Value,
    dispatch_alias_rows: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let tier_catalog = template
        .get("host_environment")
        .and_then(|value| value.get("carrier_tier_contract"))
        .and_then(|value| value.get("tier_catalog"))
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("tier_id").and_then(serde_yaml::Value::as_str))
        .map(str::trim)
        .filter(|tier| !tier.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let mut systems = template
        .get("host_environment")
        .and_then(|value| value.get("systems"))
        .and_then(serde_yaml::Value::as_mapping)
        .into_iter()
        .flatten()
        .filter_map(|(system_id, value)| {
            let system_id = system_id.as_str()?.trim();
            (!system_id.is_empty()).then_some((system_id.to_string(), value))
        })
        .collect::<Vec<_>>();
    systems.sort_by(|left, right| left.0.cmp(&right.0));

    let mut relations = Vec::with_capacity(
        systems
            .iter()
            .map(|_| tier_catalog.len() * dispatch_alias_rows.len())
            .sum(),
    );
    for (system_id, system) in systems {
        let admissible_tiers = system
            .get("admissible_carrier_tiers")
            .and_then(serde_yaml::Value::as_sequence)
            .into_iter()
            .flatten()
            .filter_map(serde_yaml::Value::as_str)
            .map(str::trim)
            .filter(|tier| !tier.is_empty())
            .collect::<std::collections::BTreeSet<_>>();
        let mut carriers = Vec::new();
        if let Some(mapping) = system
            .get("carriers")
            .and_then(serde_yaml::Value::as_mapping)
        {
            for (carrier_id, value) in mapping {
                let Some(carrier_id) = carrier_id.as_str().map(str::trim) else {
                    continue;
                };
                let Some(mut row) = serde_json::to_value(value).ok() else {
                    continue;
                };
                row["role_id"] = serde_json::Value::String(carrier_id.to_string());
                carriers.push(row);
            }
        }
        carriers.sort_by(|left, right| {
            left["role_id"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["role_id"].as_str().unwrap_or_default())
        });

        for tier in &tier_catalog {
            for (source_index, alias) in dispatch_alias_rows.iter().enumerate() {
                let alias_id = alias
                    .get("alias_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                let alias_tier = canonical_carrier_tier(alias);
                let runtime_roles = json_string_list(alias.get("runtime_roles"));
                let task_classes = json_string_list(alias.get("task_classes"));
                let mut compatible = carriers.iter().filter(|carrier| {
                    canonical_carrier_tier(carrier).as_deref() == alias_tier.as_deref()
                        && (runtime_roles.is_empty()
                            || runtime_roles.iter().any(|requested| {
                                json_string_list(carrier.get("runtime_roles"))
                                    .iter()
                                    .any(|actual| actual == requested)
                            }))
                        && (task_classes.is_empty()
                            || task_classes.iter().any(|requested| {
                                json_string_list(carrier.get("task_classes"))
                                    .iter()
                                    .any(|actual| actual == requested)
                            }))
                });
                let (status, diagnostic) = if alias_id.is_none() {
                    (
                        "diagnostic",
                        Some((
                            "invalid_alias_id",
                            "alias identity is missing or non-string",
                        )),
                    )
                } else if !admissible_tiers.contains(tier.as_str()) {
                    (
                        "diagnostic",
                        Some((
                            "tier_not_admissible",
                            "system does not admit this carrier tier",
                        )),
                    )
                } else if alias_tier.as_deref() != Some(tier.as_str()) {
                    (
                        "diagnostic",
                        Some((
                            "alias_tier_mismatch",
                            "alias requests a different carrier tier",
                        )),
                    )
                } else if compatible.next().is_none() {
                    (
                        "diagnostic",
                        Some((
                            "carrier_capability_mismatch",
                            "no compatible carrier capability is materialized",
                        )),
                    )
                } else {
                    ("materialized", None)
                };
                let diagnostic_value = diagnostic.map(|(code, message)| {
                    serde_json::json!({
                        "code": code,
                        "message": message,
                        "system_id": system_id,
                        "carrier_tier": tier,
                        "alias_id": alias_id,
                        "source_index": source_index,
                    })
                });
                relations.push(serde_json::json!({
                    "key": [system_id, tier, alias_id, source_index],
                    "system_id": system_id,
                    "carrier_tier": tier,
                    "alias_id": alias_id,
                    "source_index": source_index,
                    "status": status,
                    "admissible": status == "materialized",
                    "materialized": status == "materialized",
                    "diagnostic": diagnostic_value,
                }));
            }
        }
    }
    relations.sort_by(|left, right| left["key"].to_string().cmp(&right["key"].to_string()));
    relations
}

fn json_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn resolved_carrier_roles(
    config: &serde_yaml::Value,
    catalog_root: &std::path::Path,
) -> Vec<serde_json::Value> {
    let overlay_roles = super::project_activator_surface::overlay_host_cli_agent_catalog(config);
    if overlay_roles.is_empty() {
        super::project_activator_surface::read_host_cli_agent_catalog(catalog_root)
    } else {
        overlay_roles
    }
}

pub(crate) fn carrier_role_validation_errors(roles: &[serde_json::Value]) -> Vec<String> {
    let mut errors = roles
        .iter()
        .filter_map(|row| {
            let role_id = row["role_id"].as_str().unwrap_or("<unknown>");
            let mut missing = Vec::new();
            if row["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("runtime_roles");
            }
            if row["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("task_classes");
            }
            if missing.is_empty() {
                None
            } else {
                Some(format!(
                    "carrier role `{role_id}` is missing required runtime metadata: {}",
                    missing.join(", ")
                ))
            }
        })
        .collect::<Vec<_>>();
    errors.extend(
        duplicate_non_empty_carrier_role_ids(roles)
            .into_iter()
            .map(|role_id| {
                format!(
                    "duplicate carrier role id `{role_id}`: role_id must be globally unique"
                )
            }),
    );
    errors.sort();
    errors
}

pub(crate) fn duplicate_non_empty_carrier_role_ids(
    roles: &[serde_json::Value],
) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for role_id in roles.iter().filter_map(|row| {
        row["role_id"]
            .as_str()
            .map(str::trim)
            .filter(|role_id| !role_id.is_empty())
    }) {
        *counts.entry(role_id.to_string()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(role_id, count)| (count > 1).then_some(role_id))
        .collect()
}

pub(crate) fn materialized_dispatch_aliases(
    config: &serde_yaml::Value,
    dispatch_alias_rows: &[serde_json::Value],
    carrier_roles: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    if dispatch_alias_rows.is_empty() {
        super::project_activator_surface::overlay_host_cli_dispatch_alias_catalog(
            config,
            carrier_roles,
        )
    } else {
        super::project_activator_surface::materialize_host_cli_dispatch_alias_catalog(
            dispatch_alias_rows,
            carrier_roles,
        )
    }
}

pub(crate) fn carrier_dispatch_alias_validation_errors(
    dispatch_aliases: &[serde_json::Value],
) -> Vec<String> {
    dispatch_aliases
        .iter()
        .filter_map(|row| {
            let role_id = row["role_id"].as_str().unwrap_or("<unknown>");
            if row["unresolved"] == serde_json::Value::Bool(true) {
                let code = row["unresolved_diagnostic"]["code"]
                    .as_str()
                    .unwrap_or("carrier_alias_unresolved");
                return Some(format!(
                    "carrier dispatch alias `{role_id}` unresolved: {code}"
                ));
            }
            let mut missing = Vec::new();
            if row["template_role_id"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
            {
                missing.push("carrier_tier");
            }
            if row["default_runtime_role"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
            {
                missing.push("runtime_role");
            }
            if row["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("runtime_roles");
            }
            if row["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                missing.push("task_classes");
            }
            if row["developer_instructions"]
                .as_str()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                missing.push("developer_instructions");
            }
            if missing.is_empty() {
                None
            } else {
                Some(format!(
                    "carrier dispatch alias `{role_id}` is missing required runtime metadata: {}",
                    missing.join(", ")
                ))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        carrier_dispatch_alias_validation_errors, carrier_role_validation_errors,
        duplicate_non_empty_carrier_role_ids, master_template_dispatch_alias_matrix,
    };

    #[test]
    fn role_validation_errors_are_carrier_neutral() {
        let errors = carrier_role_validation_errors(&[serde_json::json!({
            "role_id": "junior"
        })]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("carrier role `junior`"));
        assert!(!errors[0].contains("codex"));
    }

    #[test]
    fn role_validation_errors_reject_duplicate_non_empty_ids_deterministically() {
        let roles = [
            serde_json::json!({
                "role_id": "middle",
                "runtime_roles": ["worker"],
                "task_classes": ["implementation"]
            }),
            serde_json::json!({
                "role_id": "middle",
                "runtime_roles": ["coach"],
                "task_classes": ["review"]
            }),
        ];

        assert_eq!(duplicate_non_empty_carrier_role_ids(&roles), vec!["middle"]);
        assert_eq!(
            carrier_role_validation_errors(&roles),
            vec!["duplicate carrier role id `middle`: role_id must be globally unique"]
        );
    }

    #[test]
    fn role_validation_errors_ignore_duplicate_blank_ids_per_existing_law() {
        let roles = [
            serde_json::json!({"role_id": ""}),
            serde_json::json!({"role_id": ""}),
        ];

        assert!(duplicate_non_empty_carrier_role_ids(&roles).is_empty());
        assert_eq!(carrier_role_validation_errors(&roles).len(), 2);
    }

    #[test]
    fn dispatch_alias_validation_errors_are_carrier_neutral() {
        let errors = carrier_dispatch_alias_validation_errors(&[serde_json::json!({
            "role_id": "development_implementer"
        })]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("carrier dispatch alias `development_implementer`"));
        assert!(!errors[0].contains("codex"));
    }

    fn project_fixture() -> (serde_yaml::Value, Vec<serde_json::Value>) {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(root.join("vida.config.yaml")).expect("project config"),
        )
        .expect("project config yaml");
        let agents =
            super::super::host_runtime_materialization::overlay_host_runtime_agent_catalog(&config);
        (config, agents)
    }

    #[test]
    fn canonical_carrier_tier_uses_explicit_value_and_legacy_fallback() {
        let (_, agents) = project_fixture();
        let source = agents.first().expect("project fixture carrier");
        let explicit = serde_json::json!({
            "carrier_tier": source["carrier_tier"],
            "tier": source["tier"]
        });
        assert_eq!(
            super::canonical_carrier_tier(&explicit),
            source["carrier_tier"].as_str().map(ToOwned::to_owned)
        );
        let legacy = serde_json::json!({"tier": source["tier"]});
        assert_eq!(
            super::canonical_carrier_tier(&legacy),
            source["tier"].as_str().map(ToOwned::to_owned)
        );
    }

    #[test]
    fn materialization_preserves_external_concrete_tier_and_provider_identity() {
        let (_, agents) = project_fixture();
        let source = agents.first().expect("project fixture carrier");
        let source_id = source["role_id"].as_str().expect("carrier id");
        let carrier_tier = super::canonical_carrier_tier(source).expect("carrier tier");
        let concrete_tier = format!(
            "{}-provider",
            super::concrete_carrier_tier(source).expect("concrete tier")
        );
        let provider = source["model_provider"].as_str().expect("provider");
        let external_provider = format!("{provider}-provider");
        let mut external = source.clone();
        external["role_id"] = serde_json::Value::String(format!("{source_id}-provider"));
        external["tier"] = serde_json::Value::String(concrete_tier.clone());
        external["concrete_tier"] = serde_json::Value::String(concrete_tier);
        external["carrier_tier"] = serde_json::Value::String(carrier_tier.clone());
        external["model_provider"] = serde_json::Value::String(external_provider.clone());
        let alias = serde_json::json!({
            "alias_id": format!("{source_id}-alias"),
            "carrier_tier": carrier_tier,
            "runtime_roles": source["runtime_roles"],
            "task_classes": source["task_classes"]
        });

        let rows = super::super::host_runtime_materialization::materialize_host_runtime_dispatch_alias_catalog(
            &[alias],
            &[external],
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["unresolved"], false);
        assert_eq!(rows[0]["carrier_tier"], source["carrier_tier"]);
        assert_eq!(
            rows[0]["tier"],
            format!("{}-provider", source["tier"].as_str().unwrap())
        );
        assert_eq!(rows[0]["carrier_provider"], external_provider);
        assert_eq!(rows[0]["provider_identity"], external_provider);
    }

    #[test]
    fn materialization_emits_deterministic_diagnostics_for_duplicate_and_missing_tiers() {
        let (_, agents) = project_fixture();
        let source = agents.first().expect("project fixture carrier");
        let source_id = source["role_id"].as_str().expect("carrier id");
        let carrier_tier = super::canonical_carrier_tier(source).expect("carrier tier");
        let duplicate = {
            let mut row = source.clone();
            row["role_id"] = serde_json::Value::String(format!("{source_id}-duplicate"));
            row
        };
        let alias = serde_json::json!({
            "alias_id": format!("{source_id}-duplicate-alias"),
            "carrier_tier": carrier_tier,
            "runtime_roles": source["runtime_roles"],
            "task_classes": source["task_classes"]
        });
        let rows = super::super::host_runtime_materialization::materialize_host_runtime_dispatch_alias_catalog(
            &[alias],
            &[source.clone(), duplicate],
        );
        assert_eq!(
            rows[0]["unresolved_diagnostic"]["code"],
            "ambiguous_compatible_carriers"
        );

        let missing = serde_json::json!({
            "alias_id": format!("{source_id}-missing-tier"),
            "runtime_roles": source["runtime_roles"],
            "task_classes": source["task_classes"]
        });
        let rows = super::super::host_runtime_materialization::materialize_host_runtime_dispatch_alias_catalog(
            &[missing],
            std::slice::from_ref(source),
        );
        assert_eq!(rows[0]["unresolved"], true);
        assert_eq!(
            rows[0]["unresolved_diagnostic"]["code"],
            "carrier_tier_missing"
        );
    }

    #[test]
    fn project_alias_matrix_never_silently_drops_declared_aliases() {
        let (config, agents) = project_fixture();
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = root.join("docs/process/agent-extensions/dispatch-aliases.yaml");
        let registry: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(path).expect("dispatch aliases registry"),
        )
        .expect("dispatch aliases yaml");
        let aliases = crate::registry_rows_by_key(&registry, "dispatch_aliases", "alias_id", &[]);
        let materialized = super::materialized_dispatch_aliases(&config, &aliases, &agents);
        let ids = materialized
            .iter()
            .filter_map(|row| row["role_id"].as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for alias in aliases {
            let alias_id = alias["alias_id"].as_str().expect("alias id");
            assert!(ids.contains(alias_id), "alias {alias_id} must be retained");
        }
    }

    #[test]
    fn master_template_matrix_covers_derived_system_tier_alias_cross_product() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let template: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(
                root.join("docs/framework/templates/vida.config.yaml.template"),
            )
            .expect("framework template"),
        )
        .expect("framework template yaml");
        let configured_path = template["agent_extensions"]["registries"]["dispatch_aliases"]
            .as_str()
            .expect("template dispatch alias path");
        let configured = root.join(configured_path);
        let registry_path = if configured.is_file() {
            configured
        } else {
            let suffix = std::path::Path::new(configured_path)
                .strip_prefix(std::path::Path::new(".vida/project"))
                .expect("template registry path should use project overlay root");
            root.join("docs/process").join(suffix)
        };
        let registry: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(registry_path).expect("dispatch aliases registry"),
        )
        .expect("dispatch aliases registry yaml");
        let aliases = crate::registry_rows_by_key(&registry, "dispatch_aliases", "alias_id", &[]);
        let tier_count = template["host_environment"]["carrier_tier_contract"]["tier_catalog"]
            .as_sequence()
            .map(Vec::len)
            .expect("template tier catalog");
        let system_count = template["host_environment"]["systems"]
            .as_mapping()
            .map(serde_yaml::Mapping::len)
            .expect("template systems");
        let matrix = master_template_dispatch_alias_matrix(&template, &aliases);
        assert_eq!(tier_count, 4);
        assert_eq!(system_count, 4);
        assert_eq!(aliases.len(), 9);
        assert_eq!(matrix.len(), tier_count * system_count * aliases.len());
        let keys = matrix
            .iter()
            .map(|row| row["key"].to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), matrix.len());
        assert!(matrix.iter().all(|row| {
            matches!(
                row["status"].as_str(),
                Some("materialized") | Some("diagnostic")
            ) && (row["materialized"].as_bool() == Some(true) || row["diagnostic"].is_object())
        }));
    }
}
