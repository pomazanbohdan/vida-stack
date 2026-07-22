use std::{collections::HashMap, path::Path};

use crate::{read_simple_toml_sections, registry_rows_by_key};

pub(crate) struct CarrierRuntimeProjection {
    pub(crate) carrier_runtime: serde_json::Value,
    pub(crate) validation_errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SelectedHostCliSystemError {
    MissingSelection,
    UnknownSelection { system: String },
    DisabledSelection { system: String },
}

impl SelectedHostCliSystemError {
    fn blocker_code(&self) -> &'static str {
        taskflow_contracts::BlockerCode::HostToolCapabilityMissing.as_str()
    }
}

impl std::fmt::Display for SelectedHostCliSystemError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSelection => {
                write!(formatter, "selected host CLI system is missing")
            }
            Self::UnknownSelection { system } => {
                write!(
                    formatter,
                    "selected host CLI system `{system}` is not registered"
                )
            }
            Self::DisabledSelection { system } => {
                write!(formatter, "selected host CLI system `{system}` is disabled")
            }
        }
    }
}

impl std::error::Error for SelectedHostCliSystemError {}

fn selected_runtime_root(
    root: &Path,
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
) -> Result<std::path::PathBuf, SelectedHostCliSystemError> {
    let Some(system) = selected_host_cli_system
        .map(str::trim)
        .filter(|system| !system.is_empty())
    else {
        return Err(SelectedHostCliSystemError::MissingSelection);
    };
    let system = system.to_ascii_lowercase();
    let Some(entry) = host_cli_system_registry.get(&system) else {
        return Err(SelectedHostCliSystemError::UnknownSelection { system });
    };
    if !crate::project_activator_surface::host_cli_system_enabled(entry) {
        return Err(SelectedHostCliSystemError::DisabledSelection { system });
    }
    Ok(crate::project_activator_surface::host_cli_system_runtime_root(entry, &system, root))
}

pub(crate) fn build_carrier_runtime_projection(
    config: &serde_yaml::Value,
    root: &Path,
    selected_host_cli_system: Option<&str>,
    host_cli_system_registry: &HashMap<String, serde_yaml::Value>,
    dispatch_aliases_registry: &serde_yaml::Value,
    dispatch_aliases_path: Option<&str>,
) -> CarrierRuntimeProjection {
    let (runtime_config, carrier_roles, selected_host_system_error) =
        match selected_runtime_root(root, selected_host_cli_system, host_cli_system_registry) {
            Ok(runtime_root) => (
                read_simple_toml_sections(&runtime_root.join("config.toml")),
                crate::carrier_runtime_catalog::resolved_carrier_roles(config, &runtime_root),
                None,
            ),
            Err(error) => (HashMap::new(), Vec::new(), Some(error)),
        };
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
    let selected_host_system_id = selected_host_cli_system
        .map(str::trim)
        .filter(|system| !system.is_empty())
        .map(str::to_ascii_lowercase);
    let stage_attempt_policies = serde_json::to_value(
        crate::yaml_lookup(config, &["agent_system", "stage_attempt_policies"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let executor_backend_relation = selected_host_system_id
        .as_deref()
        .and_then(|system_id| host_cli_system_registry.get(system_id))
        .and_then(|system| crate::yaml_lookup(system, &["executor_backend_relation"]))
        .cloned()
        .and_then(|relation| serde_json::to_value(relation).ok())
        .unwrap_or(serde_json::Value::Null);

    let mut validation_errors =
        crate::carrier_runtime_catalog::carrier_role_validation_errors(&carrier_roles);
    validation_errors.extend(
        crate::carrier_runtime_catalog::carrier_dispatch_alias_validation_errors(
            &carrier_dispatch_aliases,
        ),
    );
    if let Some(error) = &selected_host_system_error {
        validation_errors.push(format!("{}: {error}", error.blocker_code()));
    }

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
                selected_host_system_id.as_deref(),
                host_cli_system_registry,
            ),
            "roles": carrier_roles,
            "dispatch_aliases": carrier_dispatch_aliases,
            "source_of_truth": crate::carrier_runtime_metadata::carrier_runtime_source_of_truth(
                selected_host_system_id.as_deref(),
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
            "stage_attempt_policies": stage_attempt_policies,
            "selected_host_system_id": selected_host_system_id,
            "selected_host_system_error": selected_host_system_error
                .as_ref()
                .map(ToString::to_string),
            "executor_backend_relation": executor_backend_relation,
        }),
        validation_errors,
    }
}

fn carrier_policy_string(assignment: &serde_json::Value, field: &str) -> Option<String> {
    assignment
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn policy_list_contains(policy: &serde_json::Value, field: &str, value: &str) -> bool {
    policy
        .get(field)
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .any(|candidate| candidate.as_str() == Some(value))
        })
        .unwrap_or(false)
}

fn policy_list_is_restrictive(policy: &serde_json::Value, field: &str) -> bool {
    policy
        .get(field)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|values| !values.is_empty())
}

fn carrier_policy_mismatch_code() -> &'static str {
    taskflow_contracts::BlockerCode::ActiveCarrierPolicyMismatch.as_str()
}

fn carrier_policy_reselection_code() -> &'static str {
    taskflow_contracts::BlockerCode::CarrierPolicyReselectionRequired.as_str()
}

pub(crate) fn carrier_policy_revalidation(
    current_bundle: &serde_json::Value,
    assignment: &serde_json::Value,
) -> serde_json::Value {
    let selected_carrier = carrier_policy_string(assignment, "selected_carrier_id");
    let selected_carrier_tier = carrier_policy_string(assignment, "selected_carrier_tier");
    let selected_concrete_tier = carrier_policy_string(assignment, "selected_concrete_tier")
        .or_else(|| carrier_policy_string(assignment, "selected_tier"));
    let selected_provider = carrier_policy_string(assignment, "selected_carrier_provider")
        .or_else(|| carrier_policy_string(assignment, "selected_model_provider"));
    let selected_backend = carrier_policy_string(assignment, "selected_backend_id");
    let selected_profile = carrier_policy_string(assignment, "selected_model_profile_id");
    let selected_model = carrier_policy_string(assignment, "selected_model_ref");
    let selected_reasoning = carrier_policy_string(assignment, "selected_reasoning_effort");
    let selected_runtime_role = carrier_policy_string(assignment, "selected_runtime_role")
        .or_else(|| carrier_policy_string(assignment, "runtime_role"));
    let selected_task_class = carrier_policy_string(assignment, "task_class")
        .or_else(|| carrier_policy_string(assignment, "route_task_class"));

    let mut blockers = Vec::new();
    let mut reasons = Vec::new();
    let mut mismatches = Vec::new();
    let roles = current_bundle["carrier_runtime"]["roles"].as_array();
    let Some(roles) = roles else {
        blockers.push(carrier_policy_reselection_code().to_string());
        reasons.push("current carrier policy snapshot is unavailable".to_string());
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "mismatches": mismatches,
        });
    };

    let duplicate_role_ids =
        crate::carrier_runtime_catalog::duplicate_non_empty_carrier_role_ids(roles);
    if !duplicate_role_ids.is_empty() {
        let validation_errors =
            crate::carrier_runtime_catalog::carrier_role_validation_errors(roles);
        blockers.push(carrier_policy_mismatch_code().to_string());
        blockers.push(carrier_policy_reselection_code().to_string());
        blockers.sort();
        blockers.dedup();
        reasons.push("current carrier policy contains duplicate role ids".to_string());
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "validation_errors": validation_errors,
            "mismatches": [{
                "field": "carrier_role_id",
                "selected": selected_carrier,
                "current": duplicate_role_ids,
                "reason": "carrier role_id must be globally unique before assignment or dispatch"
            }]
        });
    }

    let Some(selected_carrier) = selected_carrier.as_deref() else {
        blockers.push(carrier_policy_reselection_code().to_string());
        reasons.push("selected carrier is missing from the assignment".to_string());
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "mismatches": mismatches,
        });
    };
    let Some(carrier) = roles
        .iter()
        .find(|role| role["role_id"].as_str() == Some(selected_carrier))
    else {
        mismatches.push(serde_json::json!({
            "field": "selected_carrier_id",
            "selected": selected_carrier,
            "current": serde_json::Value::Null,
            "reason": "selected carrier is not admissible in the current carrier policy",
        }));
        reasons
            .push("selected carrier is not admissible in the current carrier policy".to_string());
        blockers.push(carrier_policy_mismatch_code().to_string());
        blockers.push(carrier_policy_reselection_code().to_string());
        blockers.sort();
        blockers.dedup();
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "mismatches": mismatches,
        });
    };

    let Some(selected_profile) = selected_profile.as_deref() else {
        blockers.push(carrier_policy_reselection_code().to_string());
        reasons.push("selected model profile is missing from the assignment".to_string());
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "carrier": carrier,
            "mismatches": mismatches,
        });
    };
    let profiles = carrier["model_profiles"].as_object();
    let Some(profiles) = profiles else {
        blockers.push(carrier_policy_reselection_code().to_string());
        reasons.push("current carrier has no model profile policy".to_string());
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "carrier": carrier,
            "mismatches": mismatches,
        });
    };
    let Some(profile) = profiles.get(selected_profile) else {
        mismatches.push(serde_json::json!({
            "field": "selected_model_profile_id",
            "selected": selected_profile,
            "current": profiles.keys().collect::<Vec<_>>(),
            "reason": "selected model profile is not admissible in the current carrier policy",
        }));
        reasons.push(
            "selected model profile is not admissible in the current carrier policy".to_string(),
        );
        blockers.push(carrier_policy_mismatch_code().to_string());
        blockers.push(carrier_policy_reselection_code().to_string());
        blockers.sort();
        blockers.dedup();
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": blockers,
            "reason": reasons[0],
            "reselection_required": true,
            "selected": assignment,
            "carrier": carrier,
            "mismatches": mismatches,
        });
    };

    let compare_required_scalar = |field: &str,
                                   selected: Option<&str>,
                                   current: Option<&str>,
                                   mismatch_reason: &str,
                                   missing_reason: &str,
                                   mismatches: &mut Vec<serde_json::Value>,
                                   reasons: &mut Vec<String>| {
        match selected {
            Some(selected) if current == Some(selected) => {}
            Some(selected) => {
                mismatches.push(serde_json::json!({
                    "field": field,
                    "selected": selected,
                    "current": current,
                    "reason": mismatch_reason,
                }));
                reasons.push(mismatch_reason.to_string());
            }
            None => {
                mismatches.push(serde_json::json!({
                    "field": field,
                    "selected": serde_json::Value::Null,
                    "current": current,
                    "reason": missing_reason,
                }));
                reasons.push(missing_reason.to_string());
            }
        }
    };
    compare_required_scalar(
        "selected_model_ref",
        selected_model.as_deref(),
        profile["model_ref"].as_str(),
        "selected model reference differs from current profile",
        "selected model reference is missing from the assignment",
        &mut mismatches,
        &mut reasons,
    );
    compare_required_scalar(
        "selected_reasoning_effort",
        selected_reasoning.as_deref(),
        profile["reasoning_effort"].as_str(),
        "selected reasoning effort differs from current profile",
        "selected reasoning effort is missing from the assignment",
        &mut mismatches,
        &mut reasons,
    );
    if selected_carrier_tier.is_some() {
        compare_required_scalar(
            "selected_carrier_tier",
            selected_carrier_tier.as_deref(),
            crate::carrier_runtime_catalog::canonical_carrier_tier(carrier).as_deref(),
            "selected carrier tier differs from current carrier policy",
            "selected carrier tier is missing from the assignment",
            &mut mismatches,
            &mut reasons,
        );
    }
    if selected_concrete_tier.is_some() {
        compare_required_scalar(
            "selected_concrete_tier",
            selected_concrete_tier.as_deref(),
            crate::carrier_runtime_catalog::concrete_carrier_tier(carrier).as_deref(),
            "selected concrete tier differs from current carrier policy",
            "selected concrete tier is missing from the assignment",
            &mut mismatches,
            &mut reasons,
        );
    }
    if selected_provider.is_some() {
        compare_required_scalar(
            "selected_carrier_provider",
            selected_provider.as_deref(),
            profile["provider"]
                .as_str()
                .or_else(|| carrier["model_provider"].as_str()),
            "selected carrier provider differs from current carrier policy",
            "selected carrier provider is missing from the assignment",
            &mut mismatches,
            &mut reasons,
        );
    }
    if policy_list_is_restrictive(profile, "runtime_roles") {
        match selected_runtime_role.as_deref() {
            Some(selected_runtime_role)
                if policy_list_contains(profile, "runtime_roles", selected_runtime_role) => {}
            Some(selected_runtime_role) => {
                mismatches.push(serde_json::json!({
                    "field": "selected_runtime_role",
                    "selected": selected_runtime_role,
                    "current": profile["runtime_roles"],
                    "reason": "selected runtime role is not admitted by current profile",
                }));
                reasons
                    .push("selected runtime role is not admitted by current profile".to_string());
            }
            None => {
                mismatches.push(serde_json::json!({
                    "field": "selected_runtime_role",
                    "selected": serde_json::Value::Null,
                    "current": profile["runtime_roles"],
                    "reason": "selected runtime role is missing from the assignment",
                }));
                reasons.push("selected runtime role is missing from the assignment".to_string());
            }
        }
    }
    if policy_list_is_restrictive(profile, "task_classes") {
        match selected_task_class.as_deref() {
            Some(selected_task_class)
                if policy_list_contains(profile, "task_classes", selected_task_class) => {}
            Some(selected_task_class) => {
                mismatches.push(serde_json::json!({
                    "field": "task_class",
                    "selected": selected_task_class,
                    "current": profile["task_classes"],
                    "reason": "selected task class is not admitted by current profile",
                }));
                reasons.push("selected task class is not admitted by current profile".to_string());
            }
            None => {
                mismatches.push(serde_json::json!({
                    "field": "task_class",
                    "selected": serde_json::Value::Null,
                    "current": profile["task_classes"],
                    "reason": "selected task class is missing from the assignment",
                }));
                reasons.push("selected task class is missing from the assignment".to_string());
            }
        }
    }
    if let Some(backends) = current_bundle["agent_system"]["subagents"].as_object() {
        match selected_backend.as_deref() {
            Some(selected_backend) if backends.contains_key(selected_backend) => {}
            Some(selected_backend) => {
                mismatches.push(serde_json::json!({
                    "field": "selected_backend_id",
                    "selected": selected_backend,
                    "current": backends.keys().collect::<Vec<_>>(),
                    "reason": "selected backend is not present in current agent-system policy",
                }));
                reasons.push(
                    "selected backend is not present in current agent-system policy".to_string(),
                );
            }
            None => {
                mismatches.push(serde_json::json!({
                    "field": "selected_backend_id",
                    "selected": serde_json::Value::Null,
                    "current": backends.keys().collect::<Vec<_>>(),
                    "reason": "selected backend is missing from the assignment",
                }));
                reasons.push("selected backend is missing from the assignment".to_string());
            }
        }
    }

    if !mismatches.is_empty() {
        blockers.push(carrier_policy_mismatch_code().to_string());
        blockers.push(carrier_policy_reselection_code().to_string());
    }
    blockers.sort();
    blockers.dedup();
    reasons.sort();
    reasons.dedup();
    serde_json::json!({
        "status": if blockers.is_empty() { "pass" } else { "blocked" },
        "blocker_codes": blockers,
        "reason": reasons.first().cloned().unwrap_or_else(|| "selected carrier policy is current".to_string()),
        "reselection_required": !mismatches.is_empty(),
        "selected": assignment,
        "carrier": carrier,
        "profile": profile,
        "mismatches": mismatches,
    })
}

pub(crate) fn carrier_policy_revalidation_blockers(
    current_bundle: &serde_json::Value,
    assignment: &serde_json::Value,
) -> Vec<String> {
    carrier_policy_revalidation(current_bundle, assignment)["blocker_codes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}

pub(crate) fn carrier_policy_assignment_for_dispatch(
    execution_plan: &serde_json::Value,
    dispatch_target: &str,
) -> serde_json::Value {
    let (assignment, _) = crate::runtime_dispatch_state::dispatch_target_runtime_assignment(
        execution_plan,
        dispatch_target,
    );
    if assignment.is_object() {
        return assignment;
    }
    for candidate in [
        execution_plan.get("runtime_assignment"),
        execution_plan.get("carrier_runtime_assignment"),
    ] {
        if let Some(candidate) = candidate.filter(|value| value.is_object()) {
            return candidate.clone();
        }
    }
    serde_json::Value::Null
}

pub(crate) fn carrier_policy_assignment_has_policy_identity(
    assignment: &serde_json::Value,
) -> bool {
    assignment
        .get("selected_model_profile_id")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn carrier_policy_revalidation_for_project_root(
    project_root: &Path,
    assignment: &serde_json::Value,
) -> serde_json::Value {
    let Ok(config) =
        crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root)
    else {
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": [carrier_policy_reselection_code()],
            "reason": "current project carrier policy could not be loaded",
            "reselection_required": true,
            "selected": assignment,
        });
    };
    let selected_host_cli_system = crate::yaml_string(crate::yaml_lookup(
        &config,
        &["host_environment", "cli_system"],
    ));
    let host_cli_system_registry =
        crate::project_activator_surface::host_cli_system_registry_from_config(Some(&config));
    let runtime_root = match selected_runtime_root(
        project_root,
        selected_host_cli_system.as_deref(),
        &host_cli_system_registry,
    ) {
        Ok(runtime_root) => runtime_root,
        Err(error) => {
            return serde_json::json!({
                "status": "blocked",
                "blocker_codes": [error.blocker_code()],
                "reason": error.to_string(),
                "reselection_required": true,
                "selected": assignment,
            });
        }
    };
    let roles = crate::carrier_runtime_catalog::resolved_carrier_roles(&config, &runtime_root);
    let current_bundle = serde_json::json!({
        "agent_system": serde_json::to_value(
            crate::yaml_lookup(&config, &["agent_system"])
                .cloned()
                .unwrap_or(serde_yaml::Value::Null),
        )
        .unwrap_or(serde_json::Value::Null),
        "carrier_runtime": {"roles": roles},
    });
    carrier_policy_revalidation(&current_bundle, assignment)
}

#[cfg(test)]
mod tests {
    use super::build_carrier_runtime_projection;
    use super::selected_runtime_root;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;

    fn test_bundle() -> serde_json::Value {
        json!({
            "agent_system": {"subagents": {"backend-a": {}}},
            "carrier_runtime": {
                "roles": [{
                    "role_id": "carrier-a",
                    "model_profiles": {
                        "profile-a": {
                            "model_ref": "model-a",
                            "reasoning_effort": "effort-a",
                            "runtime_roles": ["runtime-a"],
                            "task_classes": ["class-a"]
                        }
                    }
                }]
            }
        })
    }

    fn test_assignment() -> serde_json::Value {
        json!({
            "selected_carrier_id": "carrier-a",
            "selected_backend_id": "backend-a",
            "selected_model_profile_id": "profile-a",
            "selected_model_ref": "model-a",
            "selected_reasoning_effort": "effort-a",
            "selected_runtime_role": "runtime-a",
            "task_class": "class-a"
        })
    }

    fn duplicate_role_bundle() -> serde_json::Value {
        json!({
            "agent_system": {"subagents": {"middle": {}}},
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "middle",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "host-profile": {
                                "model_ref": "host-model",
                                "reasoning_effort": "host-effort",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    },
                    {
                        "role_id": "middle",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "subagent-profile": {
                                "model_ref": "subagent-model",
                                "reasoning_effort": "subagent-effort",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ]
            }
        })
    }

    fn duplicate_role_assignment() -> serde_json::Value {
        json!({
            "selected_carrier_id": "middle",
            "selected_backend_id": "middle",
            "selected_model_profile_id": "subagent-profile",
            "selected_model_ref": "subagent-model",
            "selected_reasoning_effort": "subagent-effort",
            "selected_runtime_role": "worker",
            "task_class": "implementation"
        })
    }

    #[test]
    fn carrier_policy_revalidation_accepts_current_selection() {
        let result = super::carrier_policy_revalidation(&test_bundle(), &test_assignment());
        assert_eq!(result["status"], "pass");
        assert_eq!(result["blocker_codes"], json!([]));
    }

    #[test]
    fn carrier_policy_revalidation_blocks_duplicate_role_ids_before_first_match() {
        let result = super::carrier_policy_revalidation(
            &duplicate_role_bundle(),
            &duplicate_role_assignment(),
        );
        assert_eq!(result["status"], "blocked");
        assert_eq!(
            result["validation_errors"],
            json!(["duplicate carrier role id `middle`: role_id must be globally unique"])
        );
        assert_eq!(result["mismatches"][0]["current"], json!(["middle"]));
        assert_eq!(result["mismatches"][0]["field"], "carrier_role_id");

        let mut reversed = duplicate_role_bundle();
        reversed["carrier_runtime"]["roles"]
            .as_array_mut()
            .expect("roles")
            .reverse();
        let reversed_result =
            super::carrier_policy_revalidation(&reversed, &duplicate_role_assignment());
        assert_eq!(
            reversed_result["validation_errors"],
            result["validation_errors"]
        );
        assert_eq!(reversed_result["mismatches"], result["mismatches"]);
    }

    #[test]
    fn carrier_policy_revalidation_selects_unique_role_id_without_profile_fallback() {
        let mut bundle = test_bundle();
        bundle["carrier_runtime"]["roles"] = json!([
            {
                "role_id": "host-middle",
                "model_profiles": {
                    "shared-profile": {
                        "model_ref": "host-model",
                        "reasoning_effort": "host-effort",
                        "runtime_roles": ["runtime-a"],
                        "task_classes": ["class-a"]
                    }
                }
            },
            {
                "role_id": "subagent-middle",
                "model_profiles": {
                    "shared-profile": {
                        "model_ref": "subagent-model",
                        "reasoning_effort": "subagent-effort",
                        "runtime_roles": ["runtime-a"],
                        "task_classes": ["class-a"]
                    }
                }
            }
        ]);
        let assignment = json!({
            "selected_carrier_id": "subagent-middle",
            "selected_backend_id": "backend-a",
            "selected_model_profile_id": "shared-profile",
            "selected_model_ref": "subagent-model",
            "selected_reasoning_effort": "subagent-effort",
            "selected_runtime_role": "runtime-a",
            "task_class": "class-a"
        });

        let result = super::carrier_policy_revalidation(&bundle, &assignment);

        assert_eq!(result["status"], "pass");
        assert_eq!(result["carrier"]["role_id"], "subagent-middle");
    }

    #[test]
    fn carrier_policy_revalidation_blocks_stale_profile_and_reasoning() {
        let mut assignment = test_assignment();
        assignment["selected_model_profile_id"] = json!("profile-stale");
        assignment["selected_reasoning_effort"] = json!("effort-stale");
        let result = super::carrier_policy_revalidation(&test_bundle(), &assignment);
        assert_eq!(result["status"], "blocked");
        assert!(result["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "active_carrier_policy_mismatch"));
        assert_eq!(result["reselection_required"], json!(true));
    }

    #[test]
    fn carrier_policy_revalidation_blocks_stripped_policy_identity_fields() {
        for field in [
            "selected_backend_id",
            "selected_model_ref",
            "selected_reasoning_effort",
            "selected_runtime_role",
            "task_class",
        ] {
            let mut assignment = test_assignment();
            assignment
                .as_object_mut()
                .expect("assignment should be an object")
                .remove(field);
            let result = super::carrier_policy_revalidation(&test_bundle(), &assignment);
            assert_eq!(result["status"], "blocked", "field={field}");
            assert!(
                result["blocker_codes"]
                    .as_array()
                    .expect("blockers")
                    .iter()
                    .any(|code| {
                        code == taskflow_contracts::BlockerCode::ActiveCarrierPolicyMismatch
                            .as_str()
                    }),
                "field={field} result={result:#}"
            );
            assert!(
                result["mismatches"]
                    .as_array()
                    .expect("mismatches")
                    .iter()
                    .any(|mismatch| mismatch["field"] == field),
                "field={field} result={result:#}"
            );
        }
    }

    #[test]
    fn carrier_policy_assignment_for_dispatch_uses_nested_runtime_assignment() {
        let assignment = test_assignment();
        let execution_plan = json!({"runtime_assignment": assignment});
        let resolved = super::carrier_policy_assignment_for_dispatch(&execution_plan, "worker");
        assert_eq!(resolved["selected_model_profile_id"], "profile-a");
        assert_eq!(resolved["selected_reasoning_effort"], "effort-a");
    }

    #[test]
    fn carrier_policy_revalidation_blocks_stale_carrier_backend_and_scope() {
        for (field, value) in [
            ("selected_carrier_id", "carrier-stale"),
            ("selected_backend_id", "backend-stale"),
            ("selected_runtime_role", "runtime-stale"),
            ("task_class", "class-stale"),
        ] {
            let mut assignment = test_assignment();
            assignment[field] = json!(value);
            let result = super::carrier_policy_revalidation(&test_bundle(), &assignment);
            assert_eq!(result["status"], "blocked", "field={field}");
            assert!(
                result["blocker_codes"]
                    .as_array()
                    .expect("blockers")
                    .iter()
                    .any(|code| code
                        == taskflow_contracts::BlockerCode::ActiveCarrierPolicyMismatch.as_str()),
                "field={field} result={result:#}"
            );
        }
    }

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
            crate::project_activator_surface::host_cli_system_registry_from_config(Some(&config));

        let root = selected_runtime_root(Path::new("/tmp/project"), Some("hermes"), &registry)
            .expect("explicit enabled system should resolve");

        assert_eq!(root, Path::new("/tmp/project/.hermes"));
    }

    #[test]
    fn selected_runtime_root_fails_closed_when_selection_is_missing_even_with_enabled_system() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    hermes:
      enabled: true
      runtime_root: .hermes
    acme:
      enabled: true
      runtime_root: .acme
    opencode:
      enabled: false
      runtime_root: .opencode
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_from_config(Some(&config));

        let error = selected_runtime_root(Path::new("/tmp/project"), None, &registry)
            .expect_err("missing selection must not choose the first enabled system");

        assert_eq!(error, super::SelectedHostCliSystemError::MissingSelection);
        assert_eq!(
            error.blocker_code(),
            taskflow_contracts::BlockerCode::HostToolCapabilityMissing.as_str()
        );
    }

    #[test]
    fn selected_runtime_root_fails_closed_without_registry() {
        let error = selected_runtime_root(Path::new("/tmp/project"), None, &HashMap::new())
            .expect_err("missing selection must fail closed without a registry");

        assert_eq!(error, super::SelectedHostCliSystemError::MissingSelection);
    }

    #[test]
    fn selected_runtime_root_fails_closed_for_disabled_selected_system() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    hermes:
      enabled: false
      runtime_root: .hermes
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_from_config(Some(&config));

        let error = selected_runtime_root(Path::new("/tmp/project"), Some("hermes"), &registry)
            .expect_err("disabled selected system must fail closed");

        assert_eq!(
            error,
            super::SelectedHostCliSystemError::DisabledSelection {
                system: "hermes".to_string()
            }
        );
    }

    #[test]
    fn selected_runtime_root_fails_closed_for_unknown_selected_system() {
        let error =
            selected_runtime_root(Path::new("/tmp/project"), Some("missing"), &HashMap::new())
                .expect_err("unknown selected system must fail closed");

        assert_eq!(
            error,
            super::SelectedHostCliSystemError::UnknownSelection {
                system: "missing".to_string()
            }
        );
    }

    #[test]
    fn build_projection_reports_typed_host_selection_blocker_without_registry_fallback() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
host_environment:
  systems:
    acme:
      enabled: true
      runtime_root: .acme
"#,
        )
        .expect("config should parse");
        let registry =
            crate::project_activator_surface::host_cli_system_registry_from_config(Some(&config));
        let projection = build_carrier_runtime_projection(
            &config,
            Path::new("/tmp/project"),
            None,
            &registry,
            &serde_yaml::Value::Null,
            None,
        );

        assert!(projection
            .validation_errors
            .iter()
            .any(|error| error.starts_with("host_tool_capability_missing:")));
        assert_eq!(
            projection.carrier_runtime["selected_host_system_id"],
            serde_json::Value::Null
        );
        assert_eq!(
            projection.carrier_runtime["selected_host_system_error"],
            "selected host CLI system is missing"
        );
        assert_eq!(projection.carrier_runtime["roles"], serde_json::json!([]));
    }

    #[test]
    fn configured_backends_never_enter_host_carrier_ranking() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut template: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(
                root.join("docs/framework/templates/vida.config.yaml.template"),
            )
            .expect("framework template"),
        )
        .expect("framework template yaml");
        let project_config: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(root.join("vida.config.yaml")).expect("project config"),
        )
        .expect("project config yaml");
        template["agent_extensions"]["registries"] =
            project_config["agent_extensions"]["registries"].clone();
        let enabled_system_ids = template["host_environment"]["systems"]
            .as_mapping()
            .expect("template host systems")
            .iter()
            .filter_map(|(system_id, system)| {
                (system["enabled"].as_bool() == Some(true))
                    .then(|| system_id.as_str().map(str::to_string))
                    .flatten()
            })
            .collect::<Vec<_>>();
        let backend_ids = template["agent_system"]["subagents"]
            .as_mapping()
            .expect("configured execution backends")
            .keys()
            .filter_map(serde_yaml::Value::as_str)
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>();

        for system_id in enabled_system_ids {
            template["host_environment"]["cli_system"] =
                serde_yaml::Value::String(system_id.clone());
            let configured_carrier_ids = template["host_environment"]["systems"][&system_id]
                ["carriers"]
                .as_mapping()
                .expect("configured host carriers")
                .keys()
                .filter_map(serde_yaml::Value::as_str)
                .map(str::to_string)
                .collect::<std::collections::BTreeSet<_>>();
            let compiled = crate::compiled_agent_extension_bundle::build_compiled_agent_extension_bundle_for_root(
                &template,
                &root,
            )
            .unwrap_or_else(|error| panic!("system {system_id} bundle must compile: {error}"));
            let ranked_carrier_ids = compiled["carrier_runtime"]["roles"]
                .as_array()
                .expect("ranked carrier roles")
                .iter()
                .filter_map(|role| role["role_id"].as_str())
                .map(str::to_string)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(ranked_carrier_ids, configured_carrier_ids);
            assert!(ranked_carrier_ids.is_disjoint(&backend_ids));

            for alias in compiled["carrier_runtime"]["dispatch_aliases"]
                .as_array()
                .expect("compiled dispatch aliases")
                .iter()
                .filter(|alias| {
                    alias["enabled"] != serde_json::Value::Bool(false)
                        && alias["unresolved"] != serde_json::Value::Bool(true)
                })
            {
                let alias_id = alias["role_id"].as_str().expect("dispatch alias id");
                let task_class = alias["task_classes"]
                    .as_array()
                    .and_then(|classes| classes.first())
                    .and_then(serde_json::Value::as_str)
                    .expect("dispatch alias task class");
                let assignment =
                    crate::runtime_assignment_builder::build_runtime_assignment_from_dispatch_alias(
                        &compiled, alias_id, task_class,
                    );
                let selected_carrier = assignment["selected_carrier_id"]
                    .as_str()
                    .unwrap_or_else(|| panic!("missing carrier: {assignment}"));
                let selected_backend = assignment["selected_backend_id"]
                    .as_str()
                    .unwrap_or_else(|| panic!("missing backend: {assignment}"));
                assert!(configured_carrier_ids.contains(selected_carrier));
                assert!(backend_ids.contains(selected_backend));
                let revalidation = super::carrier_policy_revalidation(&compiled, &assignment);
                assert_eq!(
                    revalidation["status"], "pass",
                    "system {system_id} alias {alias_id} must revalidate: {revalidation}"
                );
            }
        }
    }

    #[test]
    fn carrier_runtime_projection_exposes_stage_attempt_policies() {
        let config = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
agent_system:
  stage_attempt_policies:
    analysis:
      fanout:
        mode: parallel
        max_attempts: 2
      attempts:
        - attempt_id: analysis-vibe
          carrier_id: vibe_cli
          model_profile_id: vibe_review
          runtime_role: business_analyst
          task_class: analysis
          isolation: external_readonly_complete
      consolidator:
        attempt_id: analysis-consolidator
        carrier_id: internal_subagents
        model_profile_id: codex_medium
        runtime_role: business_analyst
        task_class: analysis
        isolation: root_validate_only
"#,
        )
        .expect("config should parse");

        let projection = build_carrier_runtime_projection(
            &config,
            Path::new("/tmp/project"),
            None,
            &HashMap::new(),
            &serde_yaml::Value::Null,
            None,
        );

        assert_eq!(
            projection.carrier_runtime["stage_attempt_policies"]["analysis"]["fanout"]
                ["max_attempts"],
            2
        );
        assert_eq!(
            projection.carrier_runtime["stage_attempt_policies"]["analysis"]["attempts"][0]
                ["carrier_id"],
            "vibe_cli"
        );
        assert_eq!(
            projection.carrier_runtime["stage_attempt_policies"]["analysis"]["consolidator"]
                ["model_profile_id"],
            "codex_medium"
        );
    }
}
