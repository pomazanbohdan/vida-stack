//! Shared TeamFlow authority materializer.
//!
//! This module is the sole source-to-projection boundary.  The adapter only
//! validates and selects the immutable projection persisted here; it must not
//! parse raw `dev_team` or registry data.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde_json::{Map, Value};
use taskflow_authority::team_flow_transition::{TeamFlowSnapshot, TeamFlowSnapshotInput};

pub(crate) const REGISTRY_NAMES: [&str; 7] = [
    "roles",
    "skills",
    "profiles",
    "flows",
    "packs",
    "commands",
    "dispatch_aliases",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamFlowAuthorityMaterializationBlocker {
    pub(crate) code: String,
    pub(crate) path: String,
}

impl fmt::Display for TeamFlowAuthorityMaterializationBlocker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.path)
    }
}

impl std::error::Error for TeamFlowAuthorityMaterializationBlocker {}

impl TeamFlowAuthorityMaterializationBlocker {
    fn new(code: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
        }
    }
}

/// Raw source values captured after the regular registry/carrier projections.
/// All fields are owned so the persisted result can be reloaded without the
/// source files or configuration tree remaining in memory.
#[derive(Debug, Clone)]
pub(crate) struct SourceInputs {
    pub(crate) dev_team: Value,
    pub(crate) registries: Value,
    pub(crate) carrier_runtime: Value,
    pub(crate) agent_system: Value,
    pub(crate) catalog: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedTeamFlowAuthority {
    pub(crate) authority: Value,
    pub(crate) authority_id: String,
    pub(crate) authority_identity_hash: String,
    pub(crate) resolved_content_hash: String,
    pub(crate) config_hash: String,
    pub(crate) registry_hash: String,
}

#[derive(Debug, Clone)]
struct CarrierResolution {
    carrier_relation: Value,
    carrier_id: String,
    carrier_tier: String,
    dispatch_target: String,
    selected_model_profile: Value,
    model_profile_id: String,
}

impl ResolvedTeamFlowAuthority {
    pub(crate) fn into_value(self) -> Value {
        self.authority
    }
}

fn hash_json(value: &Value) -> String {
    taskflow_authority::team_flow_transition::hash_json(value)
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| (key.clone(), canonical_json(value)))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect(),
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        value => value.clone(),
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn required_text(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<String, TeamFlowAuthorityMaterializationBlocker> {
    text(value.get(key)).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_required_field_missing",
            format!("{path}.{key}"),
        )
    })
}

fn required_merged_text(
    step: &Map<String, Value>,
    role: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<String, TeamFlowAuthorityMaterializationBlocker> {
    let value = merged_value(step, role, key, path)?.ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_non_derivable_field_missing",
            format!("{path}.{key}"),
        )
    })?;
    text(Some(&value)).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_non_derivable_field_invalid",
            format!("{path}.{key}"),
        )
    })
}

fn object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a Map<String, Value>, TeamFlowAuthorityMaterializationBlocker> {
    value.as_object().ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new("team_flow_authority_object_required", path)
    })
}

fn array<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a [Value], TeamFlowAuthorityMaterializationBlocker> {
    value.as_array().map(Vec::as_slice).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new("team_flow_authority_array_required", path)
    })
}

fn identity(prefix: &str, value: &Value) -> Value {
    let hash = hash_json(value);
    serde_json::json!({"id": format!("{prefix}:{hash}"), "content_blake3": hash})
}

fn source_value<'a>(
    object: &'a Map<String, Value>,
    keys: &[&str],
    path: &str,
) -> Result<Option<&'a Value>, TeamFlowAuthorityMaterializationBlocker> {
    let values = keys
        .iter()
        .filter_map(|key| object.get(*key).map(|value| (*key, value)))
        .collect::<Vec<_>>();
    if values.len() > 1 {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_conflicting_source_aliases",
            format!("{path}.{}", keys.join("/")),
        ));
    }
    Ok(values.into_iter().next().map(|(_, value)| value))
}

fn merged_value(
    step: &Map<String, Value>,
    role: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<Value>, TeamFlowAuthorityMaterializationBlocker> {
    match (step.get(key), role.get(key)) {
        (Some(left), Some(right)) if left != right => {
            Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_conflicting_node_field",
                format!("{path}.{key}"),
            ))
        }
        (Some(value), _) | (_, Some(value)) => Ok(Some(value.clone())),
        (None, None) => Ok(None),
    }
}

fn source_catalog_field_set(
    catalog: &Value,
    key: &str,
) -> Result<BTreeSet<String>, TeamFlowAuthorityMaterializationBlocker> {
    let authority_catalog = catalog.get("authority_catalog").ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_source_catalog_missing",
            "catalog.authority_catalog",
        )
    })?;
    let authority_catalog = object(authority_catalog, "catalog.authority_catalog")?;
    let fields = authority_catalog.get(key).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_source_catalog_missing",
            format!("catalog.authority_catalog.{key}"),
        )
    })?;
    let fields = array(fields, &format!("catalog.authority_catalog.{key}"))?;
    let mut result = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        let field = text(Some(field)).ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_source_catalog_invalid",
                format!("catalog.authority_catalog.{key}[{index}]"),
            )
        })?;
        if !result.insert(field) {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_source_catalog_duplicate",
                format!("catalog.authority_catalog.{key}[{index}]"),
            ));
        }
    }
    if result.is_empty() {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_source_catalog_empty",
            format!("catalog.authority_catalog.{key}"),
        ));
    }
    Ok(result)
}

fn validate_source_dev_team(
    dev_team: &Map<String, Value>,
    catalog: &Value,
) -> Result<(), TeamFlowAuthorityMaterializationBlocker> {
    let flow_fields = source_catalog_field_set(catalog, "source_flow_fields")?;
    let step_fields = source_catalog_field_set(catalog, "source_flow_step_fields")?;
    let approval_modes = source_catalog_field_set(catalog, "approval_policy_modes")?;
    let flows = object(
        dev_team.get("flows").unwrap_or(&Value::Null),
        "dev_team.flows",
    )?;
    for (flow_id, flow) in flows {
        let flow = object(flow, &format!("dev_team.flows.{flow_id}"))?;
        for key in flow.keys() {
            if !flow_fields.contains(key) {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_source_flow_field_unsupported",
                    format!("dev_team.flows.{flow_id}.{key}"),
                ));
            }
        }
        let entry_values = ["entry_node_id", "initial_node_id"]
            .iter()
            .filter_map(|key| {
                flow.get(*key)
                    .and_then(|value| text(Some(value)))
                    .map(|value| (*key, value))
            })
            .collect::<Vec<_>>();
        if entry_values.is_empty() {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_entry_node_missing",
                format!("dev_team.flows.{flow_id}.entry_node_id"),
            ));
        }
        if entry_values
            .iter()
            .map(|(_, value)| value)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
        {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_entry_node_ambiguous",
                format!("dev_team.flows.{flow_id}.entry_node_id/initial_node_id"),
            ));
        }
        let steps = flow
            .get("steps")
            .or_else(|| flow.get("ordered_steps"))
            .unwrap_or(&Value::Null);
        let Some(steps) = steps.as_array() else {
            continue;
        };
        for (index, step) in steps.iter().enumerate() {
            let Some(step) = step.as_object() else {
                continue;
            };
            for key in step.keys() {
                if !step_fields.contains(key) {
                    return Err(TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_source_flow_step_field_unsupported",
                        format!("dev_team.flows.{flow_id}.steps[{index}].{key}"),
                    ));
                }
            }
            let Some(policy) = step.get("approval_policy") else {
                continue;
            };
            let Some(policy) = policy.as_object() else {
                continue;
            };
            let Some(mode) = policy.get("mode") else {
                if !policy.is_empty() {
                    return Err(TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_source_approval_policy_mode_missing",
                        format!("dev_team.flows.{flow_id}.steps[{index}].approval_policy.mode"),
                    ));
                }
                continue;
            };
            let mode = text(Some(mode)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_source_approval_policy_mode_invalid",
                    format!("dev_team.flows.{flow_id}.steps[{index}].approval_policy.mode"),
                )
            })?;
            if !approval_modes.contains(&mode) {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_source_approval_policy_mode_unsupported",
                    format!("dev_team.flows.{flow_id}.steps[{index}].approval_policy.mode:{mode}"),
                ));
            }
            let approval_decisions =
                source_catalog_field_set(catalog, "approval_policy_allowed_decisions")?;
            let decisions = policy.get("allowed_decisions").ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_source_approval_policy_decisions_missing",
                    format!(
                        "dev_team.flows.{flow_id}.steps[{index}].approval_policy.allowed_decisions"
                    ),
                )
            })?;
            let decisions = array(
                decisions,
                &format!(
                    "dev_team.flows.{flow_id}.steps[{index}].approval_policy.allowed_decisions"
                ),
            )?;
            let mut seen = BTreeSet::new();
            for (decision_index, decision) in decisions.iter().enumerate() {
                let decision = text(Some(decision)).ok_or_else(|| {
                    TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_source_approval_policy_decision_invalid",
                        format!(
                            "dev_team.flows.{flow_id}.steps[{index}].approval_policy.allowed_decisions[{decision_index}]"
                        ),
                    )
                })?;
                if !approval_decisions.contains(&decision) || !seen.insert(decision.clone()) {
                    return Err(TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_source_approval_policy_decision_unsupported",
                        format!(
                            "dev_team.flows.{flow_id}.steps[{index}].approval_policy.allowed_decisions[{decision_index}]:{decision}"
                        ),
                    ));
                }
            }
        }
    }
    let _ = source_catalog_field_set(catalog, "approval_policy_allowed_decisions")?;
    Ok(())
}

fn required_bool_or(
    step: &Map<String, Value>,
    role: &Map<String, Value>,
    key: &str,
    fallback: Option<bool>,
    path: &str,
) -> Result<bool, TeamFlowAuthorityMaterializationBlocker> {
    match (step.get(key), role.get(key)) {
        (Some(left), Some(right)) if left != right => {
            Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_conflicting_node_field",
                format!("{path}.{key}"),
            ))
        }
        (Some(value), _) | (_, Some(value)) => value.as_bool().ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_boolean_required",
                format!("{path}.{key}"),
            )
        }),
        (None, None) => fallback.ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_non_derivable_boolean_missing",
                format!("{path}.{key}"),
            )
        }),
    }
}

fn derive_evidence_requirements(
    step: &Map<String, Value>,
    proof_outputs: &[Value],
    path: &str,
) -> Result<Value, TeamFlowAuthorityMaterializationBlocker> {
    let mut derived = Vec::with_capacity(proof_outputs.len());
    let mut seen = BTreeSet::new();
    for (index, value) in proof_outputs.iter().enumerate() {
        let value = text(Some(value)).ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_evidence_requirement_invalid",
                format!("{path}.proof_gates.required_outputs[{index}]"),
            )
        })?;
        if !seen.insert(value.clone()) {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_evidence_requirement_conflict",
                format!("{path}.proof_gates.required_outputs[{index}]"),
            ));
        }
        derived.push(Value::String(value));
    }

    if let Some(explicit) = step.get("evidence_requirements") {
        let explicit = array(explicit, &format!("{path}.evidence_requirements"))?;
        let explicit = explicit
            .iter()
            .enumerate()
            .map(|(index, value)| {
                text(Some(value)).ok_or_else(|| {
                    TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_evidence_requirement_invalid",
                        format!("{path}.evidence_requirements[{index}]"),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let derived_text = derived
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if explicit != derived_text {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_evidence_requirements_conflict",
                format!("{path}.evidence_requirements"),
            ));
        }
    }

    Ok(Value::Array(derived))
}

fn normalize_task_class(
    step: &Map<String, Value>,
    role: &Map<String, Value>,
    path: &str,
) -> Result<String, TeamFlowAuthorityMaterializationBlocker> {
    if let Some(value) = merged_value(step, role, "task_class", path)? {
        return text(Some(&value)).ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_task_class_invalid",
                format!("{path}.task_class"),
            )
        });
    }
    let classes = merged_value(step, role, "task_classes", path)?.ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_task_class_missing",
            format!("{path}.task_class"),
        )
    })?;
    let classes = array(&classes, &format!("{path}.task_classes"))?;
    let values = classes
        .iter()
        .enumerate()
        .map(|(index, value)| {
            text(Some(value)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_task_class_invalid",
                    format!("{path}.task_classes[{index}]"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    match values.as_slice() {
        [value] => Ok(value.clone()),
        _ => Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_task_class_ambiguous",
            format!("{path}.task_classes"),
        )),
    }
}

fn registry_rows<'a>(
    registry: &'a Value,
    collection: &str,
    path: &str,
) -> Result<&'a [Value], TeamFlowAuthorityMaterializationBlocker> {
    let object = object(registry, path)?;
    let rows = object.get(collection).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_registry_collection_missing",
            format!("{path}.{collection}"),
        )
    })?;
    array(rows, &format!("{path}.{collection}"))
}

fn normalize_carrier_registry(
    carrier_runtime: &Value,
    collection: &str,
    path: &str,
) -> Result<Value, TeamFlowAuthorityMaterializationBlocker> {
    let source = carrier_runtime.get(collection).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_registry_collection_missing",
            format!("{path}.{collection}"),
        )
    })?;
    let rows = source.as_array().ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_carrier_registry_array_required",
            format!("{path}.{collection}"),
        )
    })?;
    let mut normalized = Map::new();
    normalized.insert(collection.to_string(), Value::Array(rows.to_vec()));
    Ok(Value::Object(normalized))
}

fn required_concrete_text(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<String, TeamFlowAuthorityMaterializationBlocker> {
    let value = required_text(value, key, path)?;
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "pending" | "placeholder" | "unresolved" | "unknown"
    ) {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_placeholder_value",
            format!("{path}.{key}"),
        ));
    }
    Ok(value)
}

fn required_identity_id(
    value: &Value,
    path: &str,
) -> Result<String, TeamFlowAuthorityMaterializationBlocker> {
    let identity = object(value, path)?;
    let id = text(identity.get("id")).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_identity_invalid",
            format!("{path}.id"),
        )
    })?;
    let content_blake3 = text(identity.get("content_blake3")).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_identity_invalid",
            format!("{path}.content_blake3"),
        )
    })?;
    if content_blake3.len() != 64 {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_identity_invalid",
            format!("{path}.content_blake3"),
        ));
    }
    Ok(id)
}

fn project_authority_identities(
    config_identity: &Value,
    registry_identities: &Map<String, Value>,
) -> Result<Vec<Value>, TeamFlowAuthorityMaterializationBlocker> {
    let config_id = required_identity_id(config_identity, "team_flow_authority.config")?;
    let mut authority_identities = Vec::with_capacity(REGISTRY_NAMES.len() + 1);
    authority_identities.push(serde_json::json!({
        "kind": "config",
        "id": config_id,
        "source_path": "team_flow_authority.config"
    }));
    for name in REGISTRY_NAMES {
        let path = format!("team_flow_authority.registries.{name}");
        let identity = registry_identities.get(name).ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_identity_missing",
                path.clone(),
            )
        })?;
        let id = required_identity_id(identity, &path)?;
        authority_identities.push(serde_json::json!({
            "kind": format!("registry:{name}"),
            "id": id,
            "source_path": path
        }));
    }
    Ok(authority_identities)
}

fn registry_row<'a>(
    registry: &'a Value,
    collection: &str,
    id_key: &str,
    id: &str,
    path: &str,
) -> Result<&'a Value, TeamFlowAuthorityMaterializationBlocker> {
    let rows = registry_rows(registry, collection, path)?;
    let matches = rows
        .iter()
        .filter(|row| text(row.get(id_key)).as_deref() == Some(id))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [row] => Ok(row),
        [] => Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_registry_reference_missing",
            format!("{path}.{collection}.{id}"),
        )),
        _ => Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_registry_reference_ambiguous",
            format!("{path}.{collection}.{id}"),
        )),
    }
}

fn normalize_command(
    step: &Map<String, Value>,
    commands: &Value,
    node_path: &str,
) -> Result<(Option<String>, Option<Value>), TeamFlowAuthorityMaterializationBlocker> {
    let direct = source_value(step, &["command_ref"], node_path)?
        .map(|value| {
            text(Some(value)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_command_reference_invalid",
                    format!("{node_path}.command_ref"),
                )
            })
        })
        .transpose()?;
    let mapping = step.get("command_mapping");
    if let Some(mapping) = mapping {
        let mapping = object(mapping, &format!("{node_path}.command_mapping"))?;
        let id = source_value(
            mapping,
            &["command_id", "ref"],
            &format!("{node_path}.command_mapping"),
        )?
        .map(|value| {
            text(Some(value)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_command_reference_invalid",
                    format!("{node_path}.command_mapping.command_id"),
                )
            })
        })
        .transpose()?;
        if direct.is_some() && id.as_deref() != direct.as_deref() {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_command_mapping_conflict",
                format!("{node_path}.command_mapping"),
            ));
        }
        let id = id.ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_command_reference_missing",
                format!("{node_path}.command_mapping.command_id"),
            )
        })?;
        let catalog = registry_row(commands, "commands", "command_id", &id, "registries")?;
        let surface = text(mapping.get("surface"))
            .or_else(|| text(catalog.get("surface")))
            .ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_command_surface_missing",
                    format!("{node_path}.command_mapping.surface"),
                )
            })?;
        let mut resolved = mapping.clone();
        resolved.insert("command_id".to_string(), Value::String(id.clone()));
        resolved.insert("surface".to_string(), Value::String(surface));
        return Ok((Some(id), Some(Value::Object(resolved))));
    }
    let Some(id) = direct else {
        return Ok((None, None));
    };
    let catalog = registry_row(commands, "commands", "command_id", &id, "registries")?;
    let surface = text(catalog.get("surface")).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_command_surface_missing",
            format!("registries.commands.{id}.surface"),
        )
    })?;
    Ok((
        Some(id.clone()),
        Some(serde_json::json!({"command_id": id, "surface": surface})),
    ))
}

fn resolve_carrier(
    carrier_runtime: &Value,
    alias_id: &str,
    runtime_role: &str,
    task_class: &str,
    path: &str,
) -> Result<CarrierResolution, TeamFlowAuthorityMaterializationBlocker> {
    let alias_registry =
        normalize_carrier_registry(carrier_runtime, "dispatch_aliases", "carrier_runtime")?;
    let alias = registry_row(
        &alias_registry,
        "dispatch_aliases",
        "alias_id",
        alias_id,
        "carrier_runtime",
    )?;
    let alias_path = format!("carrier_runtime.dispatch_aliases.{alias_id}");
    if alias.get("enabled").and_then(Value::as_bool) == Some(false)
        || alias.get("unselectable").and_then(Value::as_bool) == Some(true)
        || alias.get("unresolved").and_then(Value::as_bool) == Some(true)
    {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_dispatch_alias_blocked",
            alias_path.clone(),
        ));
    }
    let template_role_id = required_concrete_text(alias, "template_role_id", &alias_path)?;
    let dispatch_target = required_concrete_text(alias, "role_id", &alias_path)?;
    let carrier_tier = required_concrete_text(alias, "carrier_tier", &alias_path)?;
    let runtime_roles = array(
        alias.get("runtime_roles").unwrap_or(&Value::Null),
        &format!("{alias_path}.runtime_roles"),
    )?;
    if !runtime_roles
        .iter()
        .any(|value| value.as_str() == Some(runtime_role))
    {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_dispatch_alias_runtime_role_mismatch",
            format!("{path}.runtime_role"),
        ));
    }
    let task_classes = array(
        alias.get("task_classes").unwrap_or(&Value::Null),
        &format!("{alias_path}.task_classes"),
    )?;
    if !task_classes
        .iter()
        .any(|value| value.as_str() == Some(task_class))
    {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_dispatch_alias_task_class_mismatch",
            format!("{path}.task_class"),
        ));
    }
    let carrier_registry = normalize_carrier_registry(carrier_runtime, "roles", "carrier_runtime")?;
    let carrier = registry_row(
        &carrier_registry,
        "roles",
        "role_id",
        &template_role_id,
        "carrier_runtime",
    )?;
    let carrier_path = format!("carrier_runtime.roles.{template_role_id}");
    let carrier_tier_actual = required_concrete_text(carrier, "carrier_tier", &carrier_path)?;
    if carrier_tier_actual != carrier_tier {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_carrier_tier_mismatch",
            format!("carrier_runtime.roles.{template_role_id}"),
        ));
    }
    let profile_id = required_concrete_text(carrier, "default_model_profile", &carrier_path)?;
    let profile_source = if let Some(alias_profile) = text(alias.get("model_profile_id")) {
        if alias_profile != profile_id {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_model_profile_conflict",
                alias_path.clone(),
            ));
        }
        format!("{alias_path}.model_profile_id")
    } else {
        format!("{carrier_path}.default_model_profile")
    };
    let profiles = object(
        carrier.get("model_profiles").unwrap_or(&Value::Null),
        &format!("{carrier_path}.model_profiles"),
    )?;
    let profile = profiles.get(&profile_id).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_model_profile_missing",
            format!("{carrier_path}.model_profiles.{profile_id}"),
        )
    })?;
    let profile = object(
        profile,
        &format!("{carrier_path}.model_profiles.{profile_id}"),
    )?;
    let profile_value = Value::Object(profile.clone());
    let provider = required_concrete_text(
        &profile_value,
        "provider",
        &format!("{carrier_path}.model_profiles.{profile_id}"),
    )?;
    let model_ref = required_concrete_text(
        &profile_value,
        "model_ref",
        &format!("{carrier_path}.model_profiles.{profile_id}"),
    )?;
    let reasoning_effort = required_concrete_text(
        &profile_value,
        "reasoning_effort",
        &format!("{carrier_path}.model_profiles.{profile_id}"),
    )?;
    Ok(CarrierResolution {
        carrier_relation: serde_json::json!({
            "relation_kind": "carrier_catalog",
            "source_path": "carrier_runtime.roles",
            "selected_id": template_role_id.clone()
        }),
        carrier_id: template_role_id,
        carrier_tier,
        dispatch_target,
        selected_model_profile: serde_json::json!({
            "profile_id": profile_id.clone(),
            "provider": provider,
            "model_ref": model_ref,
            "reasoning_effort": reasoning_effort,
            "selection_source": profile_source
        }),
        model_profile_id: profile_id,
    })
}

fn normalize_step(
    raw: &Value,
    roles: &Map<String, Value>,
    commands: &Value,
    flow: &Map<String, Value>,
    index: usize,
    carrier_runtime: &Value,
    backend_relation: &Value,
) -> Result<Value, TeamFlowAuthorityMaterializationBlocker> {
    let raw = object(raw, &format!("flow.steps[{index}]"))?;
    let node_id = ["node_id", "role_id", "step_id"]
        .iter()
        .filter_map(|key| text(raw.get(*key)))
        .collect::<Vec<_>>();
    let node_id = match node_id.as_slice() {
        [id] => id.clone(),
        [] => {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_node_identity_missing",
                format!("flow.steps[{index}]"),
            ));
        }
        _ => {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_node_identity_conflict",
                format!("flow.steps[{index}]"),
            ));
        }
    };
    let role = roles.get(&node_id).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_role_reference_missing",
            format!("dev_team.roles.{node_id}"),
        )
    })?;
    let role = object(role, &format!("dev_team.roles.{node_id}"))?;
    if let Some(role_id) = text(role.get("role_id")) {
        if role_id != node_id {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_role_index_identity_mismatch",
                format!("dev_team.roles.{node_id}.role_id"),
            ));
        }
    }
    let path = format!("flow.steps[{index}].{node_id}");
    let runtime_role = merged_value(raw, role, "runtime_role", &path)?
        .and_then(|value| text(Some(&value)))
        .ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_runtime_role_missing",
                format!("{path}.runtime_role"),
            )
        })?;
    let task_class = normalize_task_class(raw, role, &path)?;
    let inclusion_rule = merged_value(raw, role, "inclusion_rule", &path)?
        .and_then(|value| text(Some(&value)))
        .ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_inclusion_rule_missing",
                format!("{path}.inclusion_rule"),
            )
        })?;
    let included = required_bool_or(
        raw,
        role,
        "included",
        Some(inclusion_rule == "always"),
        &path,
    )?;
    let required = required_bool_or(raw, role, "required", None, &path)?;
    let next_source = source_value(raw, &["next_node", "next"], &path)?;
    let terminal_source = source_value(raw, &["terminal", "terminal_closure"], &path)?;
    let parse_next = |value: &Value| {
        if value.is_null() {
            Ok(Value::Null)
        } else {
            text(Some(value)).map(Value::String).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_edge_invalid",
                    format!("{path}.next_node"),
                )
            })
        }
    };
    let parse_terminal = |value: &Value| {
        value.as_bool().ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_terminal_invalid",
                format!("{path}.terminal"),
            )
        })
    };
    let (next_node, terminal) = match (next_source, terminal_source) {
        (None, None) => {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_non_derivable_field_missing",
                format!("{path}.next_node/terminal"),
            ));
        }
        (None, Some(terminal_source)) => {
            let terminal = parse_terminal(terminal_source)?;
            if !terminal {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_terminal_edge_conflict",
                    format!("{path}.terminal"),
                ));
            }
            (Value::Null, true)
        }
        (Some(next_source), None) => {
            let next_node = parse_next(next_source)?;
            if next_node.is_null() {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_terminal_edge_conflict",
                    format!("{path}.next_node"),
                ));
            }
            (next_node, false)
        }
        (Some(next_source), Some(terminal_source)) => {
            let next_node = parse_next(next_source)?;
            let terminal = parse_terminal(terminal_source)?;
            if terminal != next_node.is_null() {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_terminal_edge_conflict",
                    format!("{path}.terminal"),
                ));
            }
            (next_node, terminal)
        }
    };
    let approval_policy = raw
        .get("approval_policy")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let approval_policy = object(&approval_policy, &format!("{path}.approval_policy"))?.clone();
    let explicit_approval = raw.get("requires_user_approval");
    let policy_approval = approval_policy.get("mode").map(|mode| {
        mode.as_str()
            .map(|mode| mode == "user_review_required")
            .ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_flag_invalid",
                    format!("{path}.approval_policy.mode"),
                )
            })
    });
    let policy_approval = match policy_approval {
        Some(value) => Some(value?),
        None => None,
    };
    let requires_user_approval = match (explicit_approval, policy_approval) {
        (None, None) => false,
        (None, Some(value)) => value,
        (Some(value), None) => value.as_bool().ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_approval_flag_invalid",
                format!("{path}.requires_user_approval"),
            )
        })?,
        (Some(value), Some(policy_value)) => {
            let explicit_value = value.as_bool().ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_flag_invalid",
                    format!("{path}.requires_user_approval"),
                )
            })?;
            if explicit_value != policy_value {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_conflict",
                    format!("{path}.requires_user_approval"),
                ));
            }
            explicit_value
        }
    };
    let approval_mode = text(approval_policy.get("mode"));
    if approval_mode.is_none() && !approval_policy.is_empty() {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_approval_mode_missing",
            format!("{path}.approval_policy.mode"),
        ));
    }
    let approval_prompt = approval_policy
        .get("prompt_template")
        .map(|value| {
            text(Some(value)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_prompt_invalid",
                    format!("{path}.approval_policy.prompt_template"),
                )
            })
        })
        .transpose()?;
    let approval_decisions = approval_policy.get("allowed_decisions");
    let approval_decisions = approval_decisions
        .map(|value| {
            let decisions = array(value, &format!("{path}.approval_policy.allowed_decisions"))?;
            if decisions.is_empty() {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_decisions_missing",
                    format!("{path}.approval_policy.allowed_decisions"),
                ));
            }
            Ok(decisions)
        })
        .transpose()?;
    if approval_mode.is_some() && approval_decisions.is_none() {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_approval_decisions_missing",
            format!("{path}.approval_policy.allowed_decisions"),
        ));
    }
    match approval_mode.as_deref() {
        Some("user_review_required") => {
            if !requires_user_approval || approval_prompt.is_none() {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_contract_invalid",
                    format!("{path}.approval_policy.mode"),
                ));
            }
            if !approval_decisions.is_some_and(|decisions| {
                decisions
                    .iter()
                    .any(|decision| decision.as_str() == Some("approved"))
            }) {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_contract_invalid",
                    format!("{path}.approval_policy.allowed_decisions"),
                ));
            }
        }
        Some("optional_user_review") if requires_user_approval => {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_approval_contract_invalid",
                format!("{path}.approval_policy.mode"),
            ));
        }
        Some("not_required") => {
            if requires_user_approval || approval_prompt.is_some() {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_approval_contract_invalid",
                    format!("{path}.approval_policy.mode"),
                ));
            }
        }
        Some(_) | None => {}
    }
    let mut resume_transitions = raw
        .get("resume_transitions")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let resume_object = object(&resume_transitions, &format!("{path}.resume_transitions"))?;
    if approval_mode.as_deref() == Some("not_required") && resume_object.contains_key("approved") {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_approval_contract_invalid",
            format!("{path}.resume_transitions.approved"),
        ));
    }
    if requires_user_approval && !resume_object.contains_key("approved") {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_approval_resume_missing",
            format!("{path}.resume_transitions.approved"),
        ));
    }
    let rework_source = source_value(raw, &["rework_transitions", "rework"], &path)?;
    let rework_transitions = rework_source
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let rework_object = object(&rework_transitions, &format!("{path}.rework_transitions"))?;
    let mut targets = Vec::with_capacity(rework_object.len());
    for (key, value) in rework_object {
        if key.trim().is_empty() {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_rework_target_invalid",
                format!("{path}.rework_transitions"),
            ));
        }
        let target = text(Some(value)).ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_rework_target_invalid",
                format!("{path}.rework_transitions.{key}"),
            )
        })?;
        targets.push(target);
    }
    let proof_gates = raw.get("proof_gates").cloned().ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_proof_gates_missing",
            format!("{path}.proof_gates"),
        )
    })?;
    let proof_gates_object = object(&proof_gates, &format!("{path}.proof_gates"))?;
    let proof_outputs = array(
        proof_gates_object
            .get("required_outputs")
            .unwrap_or(&Value::Null),
        &format!("{path}.proof_gates.required_outputs"),
    )?;
    if required && proof_outputs.is_empty() {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_proof_gates_empty",
            format!("{path}.proof_gates.required_outputs"),
        ));
    }
    let evidence_requirements = derive_evidence_requirements(raw, proof_outputs, &path)?;
    let lifecycle = raw
        .get("lifecycle_hook_templates")
        .or_else(|| flow.get("lifecycle_hook_templates"))
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let lifecycle = array(&lifecycle, &format!("{path}.lifecycle_hook_templates"))?;
    let (command_ref, command_mapping) = normalize_command(raw, commands, &path)?;
    let dispatch_alias = text(raw.get("dispatch_alias")).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_dispatch_alias_missing",
            format!("{path}.dispatch_alias"),
        )
    })?;
    let packet_template_kind = required_merged_text(raw, role, "packet_template_kind", &path)?;
    let closure_class = required_merged_text(raw, role, "closure_class", &path)?;
    let stage = required_merged_text(raw, role, "stage", &path)?;
    let completion_blocker = required_merged_text(raw, role, "completion_blocker", &path)?;
    let carrier = resolve_carrier(
        carrier_runtime,
        &dispatch_alias,
        &runtime_role,
        &task_class,
        &path,
    )?;
    let lane_id = node_id.clone();
    for (source_name, source) in [("step", raw), ("role", role)] {
        if let Some(value) = source.get("lane_id") {
            let value = text(Some(value)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_lane_id_invalid",
                    format!("{path}.{source_name}.lane_id"),
                )
            })?;
            if value != lane_id {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_lane_id_conflict",
                    format!("{path}.{source_name}.lane_id"),
                ));
            }
        }
        if let Some(value) = source.get("dispatch_target") {
            let value = text(Some(value)).ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_dispatch_target_invalid",
                    format!("{path}.{source_name}.dispatch_target"),
                )
            })?;
            if value != carrier.dispatch_target {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_dispatch_target_conflict",
                    format!("{path}.{source_name}.dispatch_target"),
                ));
            }
        }
    }
    let dispatch_target = carrier.dispatch_target.clone();
    let runtime_assignment = serde_json::json!({
        "source": "team_flow_authority",
        "dispatch_target": dispatch_target,
        "dispatch_alias": dispatch_alias.clone(),
        "carrier_id": carrier.carrier_id.clone(),
        "carrier_tier": carrier.carrier_tier.clone(),
        "model_profile_id": carrier.model_profile_id.clone()
    });
    let mut normalized = raw.clone();
    normalized.insert("node_id".to_string(), Value::String(node_id.clone()));
    normalized.remove("role_id");
    normalized.remove("step_id");
    normalized.insert("lane_id".to_string(), Value::String(lane_id));
    normalized.insert(
        "dispatch_target".to_string(),
        Value::String(dispatch_target),
    );
    normalized.insert("dispatch_alias".to_string(), Value::String(dispatch_alias));
    normalized.insert(
        "runtime_role".to_string(),
        Value::String(runtime_role.clone()),
    );
    normalized.insert("task_class".to_string(), Value::String(task_class.clone()));
    normalized.insert("inclusion_rule".to_string(), Value::String(inclusion_rule));
    normalized.insert("included".to_string(), Value::Bool(included));
    normalized.insert("required".to_string(), Value::Bool(required));
    normalized.insert("next_node".to_string(), next_node);
    normalized.insert("terminal".to_string(), Value::Bool(terminal));
    normalized.insert(
        "requires_user_approval".to_string(),
        Value::Bool(requires_user_approval),
    );
    normalized.insert(
        "approval_policy".to_string(),
        Value::Object(approval_policy),
    );
    normalized.insert("resume_transitions".to_string(), resume_transitions.take());
    normalized.insert(
        "rework".to_string(),
        serde_json::json!({"targets": targets}),
    );
    normalized.remove("rework_transitions");
    normalized.insert("proof_gates".to_string(), proof_gates);
    normalized.insert("evidence_requirements".to_string(), evidence_requirements);
    normalized.insert(
        "lifecycle_hook_templates".to_string(),
        Value::Array(lifecycle.to_vec()),
    );
    normalized.insert(
        "command_ref".to_string(),
        command_ref
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    normalized.insert(
        "command_mapping".to_string(),
        command_mapping.clone().unwrap_or(Value::Null),
    );
    normalized.insert(
        "packet_template_kind".to_string(),
        Value::String(packet_template_kind.clone()),
    );
    normalized.insert(
        "closure_class".to_string(),
        Value::String(closure_class.clone()),
    );
    normalized.insert("stage".to_string(), Value::String(stage.clone()));
    normalized.insert(
        "completion_blocker".to_string(),
        Value::String(completion_blocker.clone()),
    );
    normalized.insert("policy_diagnostics".to_string(), serde_json::json!({"source":"team_flow_authority.selected_config","fallback_used":false,"fallback_fields":[]}));
    normalized.insert(
        "activation".to_string(),
        serde_json::json!({"source":"team_flow_authority"}),
    );
    normalized.insert("runtime_assignment".to_string(), runtime_assignment.clone());
    normalized.insert("carrier_runtime_assignment".to_string(), runtime_assignment);
    normalized.insert("profile_authority".to_string(), serde_json::json!({"team_role_id":node_id,"runtime_role":runtime_role,"task_class":task_class,"source_path":format!("dev_team.roles.{}", normalized.get("node_id").and_then(Value::as_str).unwrap_or_default())}));
    normalized.insert(
        "selected_model_profile".to_string(),
        carrier.selected_model_profile,
    );
    normalized.insert("carrier_relation".to_string(), carrier.carrier_relation);
    normalized.insert(
        "executor_backend_relation".to_string(),
        backend_relation.clone(),
    );
    for key in [
        "runtime_role",
        "task_class",
        "inclusion_rule",
        "included",
        "required",
        "requires_user_approval",
    ] {
        if role.contains_key(key) {
            normalized.remove(key);
        }
    }
    if role.contains_key("task_classes") {
        normalized.remove("task_classes");
    }
    if ["terminal", "terminal_closure", "closes_workflow"]
        .iter()
        .any(|key| role.contains_key(*key))
    {
        normalized.remove("terminal");
    }
    Ok(Value::Object(normalized.clone()))
}

fn normalize_flow(
    flow_id: &str,
    raw: &Value,
    roles: &Map<String, Value>,
    commands: &Value,
    carrier_runtime: &Value,
    backend_relation: &Value,
) -> Result<Value, TeamFlowAuthorityMaterializationBlocker> {
    let raw = object(raw, &format!("dev_team.flows.{flow_id}"))?;
    if let Some(existing) = text(raw.get("flow_id")).or_else(|| text(raw.get("id"))) {
        if existing != flow_id {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_flow_identity_conflict",
                format!("dev_team.flows.{flow_id}.flow_id"),
            ));
        }
    }
    let steps = source_value(
        raw,
        &["steps", "ordered_steps"],
        &format!("dev_team.flows.{flow_id}"),
    )?
    .ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_flow_steps_missing",
            format!("dev_team.flows.{flow_id}.steps"),
        )
    })?;
    let steps = array(steps, &format!("dev_team.flows.{flow_id}.steps"))?;
    if steps.is_empty() {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_flow_steps_empty",
            format!("dev_team.flows.{flow_id}.steps"),
        ));
    }
    let mut normalized_steps = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        normalized_steps.push(normalize_step(
            step,
            roles,
            commands,
            raw,
            index,
            carrier_runtime,
            backend_relation,
        )?);
    }
    let mut node_ids = Vec::with_capacity(normalized_steps.len());
    let mut seen_node_ids = BTreeSet::new();
    for (index, step) in normalized_steps.iter().enumerate() {
        let node_id = step
            .get("node_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|node_id| !node_id.is_empty())
            .ok_or_else(|| {
                TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_node_identity_missing",
                    format!("dev_team.flows.{flow_id}.steps[{index}].node_id"),
                )
            })?;
        if !seen_node_ids.insert(node_id.to_string()) {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_node_identity_duplicate",
                format!("dev_team.flows.{flow_id}.steps[{index}].node_id"),
            ));
        }
        node_ids.push(node_id.to_string());
    }
    let sequence =
        |values: &[String]| Value::Array(values.iter().cloned().map(Value::String).collect());
    let included_nodes = normalized_steps
        .iter()
        .zip(node_ids.iter())
        .filter(|(step, _)| step.get("included").and_then(Value::as_bool) == Some(true))
        .map(|(_, node_id)| node_id.clone())
        .collect::<Vec<_>>();
    let required_nodes = normalized_steps
        .iter()
        .zip(node_ids.iter())
        .filter(|(step, _)| step.get("required").and_then(Value::as_bool) == Some(true))
        .map(|(_, node_id)| node_id.clone())
        .collect::<Vec<_>>();
    let mut result = raw.clone();
    result.remove("ordered_steps");
    let entry_values = ["entry_node_id", "initial_node_id"]
        .iter()
        .filter_map(|key| {
            raw.get(*key)
                .and_then(|value| text(Some(value)))
                .map(|value| (*key, value))
        })
        .collect::<Vec<_>>();
    let entry_node_id = match entry_values.as_slice() {
        [] => {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_entry_node_missing",
                format!("dev_team.flows.{flow_id}.entry_node_id"),
            ));
        }
        values
            if values
                .iter()
                .map(|(_, value)| value)
                .collect::<BTreeSet<_>>()
                .len()
                > 1 =>
        {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_entry_node_ambiguous",
                format!("dev_team.flows.{flow_id}.entry_node_id/initial_node_id"),
            ));
        }
        [(_, value)] => value.clone(),
        values => values[0].1.clone(),
    };
    result.remove("initial_node_id");
    result.insert("flow_id".to_string(), Value::String(flow_id.to_string()));
    result.insert("entry_node_id".to_string(), Value::String(entry_node_id));
    result.insert("steps".to_string(), Value::Array(normalized_steps));
    result.insert("lane_sequence".to_string(), sequence(&node_ids));
    result.insert("execution_lane_sequence".to_string(), sequence(&node_ids));
    result.insert("included_nodes".to_string(), sequence(&included_nodes));
    result.insert("required_nodes".to_string(), sequence(&required_nodes));
    Ok(Value::Object(result.clone()))
}

fn flow_policy(flow_id: &str, flow: &Map<String, Value>, default_flow_id: &str) -> Value {
    let mut policy = Map::new();
    for key in [
        "enabled",
        "flow_class",
        "description",
        "work_item_bindings",
        "sequential",
        "allow_parallel_handoffs",
        "lifecycle_hook_templates",
        "proof_gates",
        "resume_transitions",
        "rework_transitions",
        "adapter_projection",
    ] {
        policy.insert(
            key.to_string(),
            flow.get(key).cloned().unwrap_or(Value::Null),
        );
    }
    policy.insert(
        "default".to_string(),
        flow.get("default")
            .cloned()
            .unwrap_or_else(|| Value::Bool(flow_id == default_flow_id)),
    );
    Value::Object(policy)
}

fn resolve_work_item_flow_bindings(
    dev_team: &Map<String, Value>,
    flows: &Map<String, Value>,
) -> Result<Map<String, Value>, TeamFlowAuthorityMaterializationBlocker> {
    let canonical_key = |raw: &str, path: String| {
        let key = raw.trim().to_ascii_lowercase();
        if key.is_empty() {
            Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_work_item_flow_binding_invalid",
                path,
            ))
        } else {
            Ok(key)
        }
    };
    if let Some(explicit) = dev_team.get("work_item_flow_bindings") {
        let explicit = explicit.as_object().ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_work_item_flow_bindings_invalid",
                "dev_team.work_item_flow_bindings",
            )
        })?;
        let mut resolved = Map::new();
        for (work_item, target) in explicit {
            let canonical_work_item = canonical_key(
                work_item,
                format!("dev_team.work_item_flow_bindings.{work_item}"),
            )?;
            let target = target
                .as_str()
                .map(str::trim)
                .filter(|target| !target.is_empty())
                .ok_or_else(|| {
                    TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_work_item_flow_binding_invalid",
                        format!("dev_team.work_item_flow_bindings.{work_item}"),
                    )
                })?;
            if !flows.contains_key(target) {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_work_item_flow_binding_target_missing",
                    format!("dev_team.work_item_flow_bindings.{work_item}"),
                ));
            }
            if resolved
                .insert(canonical_work_item, Value::String(target.to_string()))
                .is_some()
            {
                return Err(TeamFlowAuthorityMaterializationBlocker::new(
                    "team_flow_authority_work_item_flow_binding_key_collision",
                    format!("dev_team.work_item_flow_bindings.{work_item}"),
                ));
            }
        }
        return Ok(resolved);
    }

    let mut candidates = BTreeMap::<String, BTreeSet<String>>::new();
    let mut raw_binding_keys = BTreeMap::<String, String>::new();
    for (flow_id, flow) in flows {
        let flow = object(flow, &format!("dev_team.flows.{flow_id}"))?;
        let Some(bindings) = flow.get("work_item_bindings") else {
            continue;
        };
        if bindings.is_null() {
            continue;
        }
        let bindings = bindings.as_array().ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_work_item_binding_invalid",
                format!("dev_team.flows.{flow_id}.work_item_bindings"),
            )
        })?;
        for (index, binding) in bindings.iter().enumerate() {
            let raw_binding = binding
                .as_str()
                .map(str::trim)
                .filter(|binding| !binding.is_empty())
                .ok_or_else(|| {
                    TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_work_item_binding_invalid",
                        format!("dev_team.flows.{flow_id}.work_item_bindings[{index}]"),
                    )
                })?;
            let binding = canonical_key(
                raw_binding,
                format!("dev_team.flows.{flow_id}.work_item_bindings[{index}]"),
            )?;
            if let Some(previous) =
                raw_binding_keys.insert(binding.clone(), raw_binding.to_string())
            {
                if previous != raw_binding {
                    return Err(TeamFlowAuthorityMaterializationBlocker::new(
                        "team_flow_authority_work_item_flow_binding_key_collision",
                        format!("dev_team.flows.{flow_id}.work_item_bindings[{index}]"),
                    ));
                }
            }
            candidates
                .entry(binding)
                .or_default()
                .insert(flow_id.clone());
        }
    }
    let mut resolved = Map::new();
    for (work_item, targets) in candidates {
        if targets.len() != 1 {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_work_item_flow_binding_ambiguous",
                format!("dev_team.work_item_flow_bindings.{work_item}"),
            ));
        }
        resolved.insert(
            work_item,
            Value::String(targets.into_iter().next().expect("one unique target")),
        );
    }
    Ok(resolved)
}

fn relation_backend(
    carrier_runtime: &Value,
    agent_system: &Value,
) -> Result<Value, TeamFlowAuthorityMaterializationBlocker> {
    let relation = carrier_runtime
        .get("executor_backend_relation")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_executor_backend_relation_missing",
                "carrier_runtime.executor_backend_relation",
            )
        })?;
    if let Some(relation_kind) = text(relation.get("relation_kind")) {
        if relation_kind != "executor_backend" {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_executor_backend_relation_conflict",
                "carrier_runtime.executor_backend_relation.relation_kind",
            ));
        }
    }
    if let Some(source_path) = text(relation.get("source_path")) {
        if source_path != "carrier_runtime.executor_backend_relation" {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_executor_backend_relation_conflict",
                "carrier_runtime.executor_backend_relation.source_path",
            ));
        }
    }
    let backend_id = required_concrete_text(
        &Value::Object(relation.clone()),
        "backend_id",
        "carrier_runtime.executor_backend_relation",
    )?;
    if let Some(selected_id) = text(relation.get("selected_id")) {
        if selected_id != backend_id {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_executor_backend_relation_conflict",
                "carrier_runtime.executor_backend_relation.selected_id",
            ));
        }
    }
    let required_class = required_concrete_text(
        &Value::Object(relation.clone()),
        "required_backend_class",
        "carrier_runtime.executor_backend_relation",
    )?;
    let subagents = object(
        agent_system.get("subagents").unwrap_or(&Value::Null),
        "agent_system.subagents",
    )?;
    let backend = subagents.get(&backend_id).ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_non_derivable_backend_missing",
            format!("agent_system.subagents.{backend_id}"),
        )
    })?;
    if backend.get("enabled").and_then(Value::as_bool) != Some(true) {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_executor_backend_disabled",
            format!("agent_system.subagents.{backend_id}"),
        ));
    }
    let backend_class = required_concrete_text(
        backend,
        "subagent_backend_class",
        &format!("agent_system.subagents.{backend_id}"),
    )?;
    if backend_class != required_class {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_executor_backend_class_mismatch",
            format!("agent_system.subagents.{backend_id}"),
        ));
    }
    if let Some(source_backend_class) = text(relation.get("backend_class")) {
        if source_backend_class != backend_class {
            return Err(TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_executor_backend_relation_conflict",
                "carrier_runtime.executor_backend_relation.backend_class",
            ));
        }
    }
    Ok(serde_json::json!({
        "relation_kind":"executor_backend",
        "source_path":"carrier_runtime.executor_backend_relation",
        "selected_id":backend_id,
        "backend_class":backend_class,
        "required_backend_class":required_class
    }))
}

fn project_node(
    step: &Value,
    snapshot_node: &taskflow_authority::team_flow_transition::TeamFlowNode,
    carrier_runtime: &Value,
    backend_relation: &Value,
    registry_identities: &Map<String, Value>,
    config_identity: &Value,
    authority_id: &str,
    flow_id: &str,
) -> Result<Value, TeamFlowAuthorityMaterializationBlocker> {
    let step = object(
        step,
        &format!("flows.{flow_id}.steps.{}", snapshot_node.node_id),
    )?;
    let alias_id = required_text(
        &Value::Object(step.clone()),
        "dispatch_alias",
        &format!("flows.{flow_id}.steps.{}", snapshot_node.node_id),
    )?;
    let carrier = resolve_carrier(
        carrier_runtime,
        &alias_id,
        &snapshot_node.runtime_role,
        &snapshot_node.task_class,
        &format!("flows.{flow_id}.steps.{}", snapshot_node.node_id),
    )?;
    let command_mapping = step
        .get("command_mapping")
        .cloned()
        .filter(|value| !value.as_object().is_some_and(Map::is_empty));
    let lane_id = required_text(
        &Value::Object(step.clone()),
        "lane_id",
        &format!("flows.{flow_id}.steps.{}", snapshot_node.node_id),
    )?;
    let dispatch_target = required_text(
        &Value::Object(step.clone()),
        "dispatch_target",
        &format!("flows.{flow_id}.steps.{}", snapshot_node.node_id),
    )?;
    let execution_seed = serde_json::json!({"authority_id":authority_id,"flow_ref":flow_id,"node_id":snapshot_node.node_id,"lane_id":lane_id,"dispatch_alias":alias_id});
    let execution_hash = hash_json(&execution_seed);
    let authority_identities = project_authority_identities(config_identity, registry_identities)?;
    Ok(serde_json::json!({
        "node_id": snapshot_node.node_id,
        "lane_id": lane_id.clone(),
        "dispatch_target": dispatch_target.clone(),
        "dispatch_alias": alias_id.clone(),
        "runtime_role": snapshot_node.runtime_role,
        "task_class": snapshot_node.task_class,
        "packet_template_kind": step["packet_template_kind"],
        "closure_class": step["closure_class"],
        "stage": step["stage"],
        "completion_blocker": step["completion_blocker"],
        "inclusion_rule": snapshot_node.inclusion_rule,
        "included": snapshot_node.included,
        "required": snapshot_node.required,
        "next_node": snapshot_node.next_node,
        "evidence_requirements": snapshot_node.evidence_requirements,
        "command_ref": snapshot_node.command_ref,
        "command_mapping": command_mapping,
        "terminal": snapshot_node.terminal,
        "requires_user_approval": snapshot_node.requires_user_approval,
        "carrier_relation": carrier.carrier_relation.clone(),
        "executor_backend_relation": backend_relation.clone(),
        "proof_gates": step["proof_gates"],
        "approval_policy": step["approval_policy"],
        "lifecycle_hook_templates": step["lifecycle_hook_templates"],
        "resume_transitions": step["resume_transitions"],
        "rework": serde_json::json!({"targets": snapshot_node.rework_targets}),
        "profile_authority": step["profile_authority"],
        "selected_model_profile": carrier.selected_model_profile.clone(),
        "policy_diagnostics": serde_json::json!({"source":"team_flow_authority.selected_config","fallback_used":false,"fallback_fields":[]}),
        "activation": serde_json::json!({"source":"team_flow_authority","runtime_role":snapshot_node.runtime_role,"task_class":snapshot_node.task_class}),
        "runtime_assignment": serde_json::json!({"source":"team_flow_authority","dispatch_target":dispatch_target,"dispatch_alias":alias_id,"carrier_id":carrier.carrier_id,"carrier_tier":carrier.carrier_tier,"model_profile_id":carrier.model_profile_id}),
        "carrier_runtime_assignment": serde_json::json!({"source":"team_flow_authority","dispatch_target":dispatch_target,"dispatch_alias":alias_id,"carrier_id":carrier.carrier_id,"carrier_tier":carrier.carrier_tier,"model_profile_id":carrier.model_profile_id}),
        "authority_identities": authority_identities,
        "execution_identity": {"id": format!("team-flow-execution:{execution_hash}"),"source_fields":["authority_id","flow_ref","node_id","lane_id","dispatch_alias"]}
    }))
}

pub(crate) fn materialize_team_flow_authority(
    inputs: SourceInputs,
) -> Result<ResolvedTeamFlowAuthority, TeamFlowAuthorityMaterializationBlocker> {
    let dev_team = object(&inputs.dev_team, "dev_team")?;
    let team_flow_enabled = match dev_team.get("enabled") {
        None => true,
        Some(value) => value.as_bool().ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_enabled_invalid",
                "dev_team.enabled",
            )
        })?,
    };
    validate_source_dev_team(dev_team, &inputs.catalog)?;
    let selection = object(
        dev_team.get("authority_selection").unwrap_or(&Value::Null),
        "dev_team.authority_selection",
    )?;
    let config_id = required_text(
        &Value::Object(selection.clone()),
        "config_id",
        "dev_team.authority_selection",
    )?;
    let profile = required_text(
        &Value::Object(selection.clone()),
        "team_profile_id",
        "dev_team.authority_selection",
    )?;
    let default_flow_id = required_text(
        &Value::Object(selection.clone()),
        "default_flow_id",
        "dev_team.authority_selection",
    )?;
    let roles = object(
        dev_team.get("roles").unwrap_or(&Value::Null),
        "dev_team.roles",
    )?;
    let commands = inputs.registries.get("commands").ok_or_else(|| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_commands_registry_missing",
            "registries.commands",
        )
    })?;
    let backend_relation = relation_backend(&inputs.carrier_runtime, &inputs.agent_system)?;
    let raw_flows = object(
        dev_team.get("flows").unwrap_or(&Value::Null),
        "dev_team.flows",
    )?;
    let mut normalized_flows = Map::new();
    for (flow_id, flow) in raw_flows {
        normalized_flows.insert(
            flow_id.clone(),
            normalize_flow(
                flow_id,
                flow,
                roles,
                commands,
                &inputs.carrier_runtime,
                &backend_relation,
            )?,
        );
    }
    if !normalized_flows.contains_key(&default_flow_id) {
        return Err(TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_default_flow_missing",
            format!("dev_team.flows.{default_flow_id}"),
        ));
    }
    let work_item_flow_bindings = resolve_work_item_flow_bindings(dev_team, &normalized_flows)?;
    let authority_catalog = inputs
        .catalog
        .get("authority_catalog")
        .cloned()
        .ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_source_catalog_missing",
                "catalog.authority_catalog",
            )
        })?;
    let mut resolved_config = inputs.dev_team.clone();
    let resolved = resolved_config.as_object_mut().expect("dev_team object");
    resolved.insert(
        "schema_version".to_string(),
        Value::String("team-flow-authority.v1".to_string()),
    );
    resolved.insert("config_id".to_string(), Value::String(config_id.clone()));
    resolved.insert("profile".to_string(), Value::String(profile.clone()));
    resolved.insert("enabled".to_string(), Value::Bool(team_flow_enabled));
    resolved.insert(
        "work_item_flow_bindings".to_string(),
        Value::Object(work_item_flow_bindings.clone()),
    );
    resolved.insert("flows".to_string(), Value::Object(normalized_flows.clone()));
    resolved.insert("authority_catalog".to_string(), authority_catalog);
    resolved.insert(
        "command_catalog".to_string(),
        commands
            .get("commands")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    let registry_hash = hash_json(&canonical_json(&inputs.registries));
    resolved.insert(
        "registry_hash".to_string(),
        Value::String(registry_hash.clone()),
    );
    let config_hash = hash_json(&canonical_json(&resolved_config));
    let config_identity = identity("dev-team-config", &resolved_config);
    let mut registry_identities = Map::new();
    for name in REGISTRY_NAMES {
        let registry = inputs.registries.get(name).ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_registry_missing",
                format!("registries.{name}"),
            )
        })?;
        registry_identities.insert(
            name.to_string(),
            identity(&format!("agent-extension-registry/{name}"), registry),
        );
    }
    let registry_identities_value = Value::Object(registry_identities.clone());
    let selected_config = serde_json::json!({
        "schema_version": "team-flow-authority.v1",
        "config_id": config_id.clone(),
        "profile": profile.clone(),
        "team_flow_enabled": team_flow_enabled,
        "authority_selection": selection,
        "registry_hash": registry_hash.clone()
    });
    let base_seed = serde_json::json!({
        "config":config_identity,
        "registries":registry_identities_value,
        "selected_config":selected_config
    });
    let authority_identity_hash = hash_json(&base_seed);
    let authority_id = format!("team-flow-authority:{authority_identity_hash}");
    let mut resolved_flows = Vec::with_capacity(normalized_flows.len());
    let mut total_nodes = 0usize;
    for (flow_id, flow) in &normalized_flows {
        let flow_object = object(flow, &format!("flows.{flow_id}"))?;
        let steps = array(
            flow_object.get("steps").unwrap_or(&Value::Null),
            &format!("flows.{flow_id}.steps"),
        )?;
        let snapshot = TeamFlowSnapshot::from_config(
            &resolved_config,
            TeamFlowSnapshotInput {
                config_id: &config_id,
                profile: &profile,
                flow_ref: flow_id,
                registry_hash: &registry_hash,
            },
        )
        .map_err(|error| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_snapshot_compile_failed",
                format!("flows.{flow_id}: {error}"),
            )
        })?;
        let mut lanes = Vec::with_capacity(snapshot.nodes.len());
        for (index, node) in snapshot.nodes.iter().enumerate() {
            lanes.push(project_node(
                &steps[index],
                node,
                &inputs.carrier_runtime,
                &backend_relation,
                &registry_identities,
                &config_identity,
                &authority_id,
                flow_id,
            )?);
        }
        total_nodes += lanes.len();
        let flow_policy = flow_policy(flow_id, flow_object, &default_flow_id);
        let entry_node_id = required_text(flow, "entry_node_id", &format!("flows.{flow_id}"))?;
        let flow_identity_payload = serde_json::json!({
            "flow_id": flow_id,
            "flow_policy": flow_policy.clone(),
            "lanes": lanes.clone(),
        });
        let flow_identity = serde_json::json!({
            "kind":"flow",
            "id":format!("team-flow-flow:{}", hash_json(&flow_identity_payload)),
            "source_path":format!("dev_team.flows.{flow_id}")
        });
        resolved_flows.push(serde_json::json!({
            "flow_id":flow_id,
            "flow_identity":flow_identity,
            "flow_policy":flow_policy,
            "entry_node_id":entry_node_id,
            "lanes":lanes
        }));
    }
    resolved_flows.sort_by(|left, right| left["flow_id"].as_str().cmp(&right["flow_id"].as_str()));
    let resolved_all_flow_payload = serde_json::json!({
        "schema_version":"team-flow-authority.v1",
        "flow_count":resolved_flows.len(),
        "lane_count":total_nodes,
        "work_item_flow_bindings":work_item_flow_bindings,
        "flows":resolved_flows
    });
    let resolved_content_hash = hash_json(&canonical_json(&resolved_all_flow_payload));
    let default_lanes = resolved_all_flow_payload["flows"]
        .as_array()
        .and_then(|flows| {
            flows
                .iter()
                .find(|flow| flow["flow_id"].as_str() == Some(default_flow_id.as_str()))
        })
        .and_then(|flow| flow.get("lanes"))
        .cloned()
        .ok_or_else(|| {
            TeamFlowAuthorityMaterializationBlocker::new(
                "team_flow_authority_default_flow_lanes_missing",
                format!("resolved_all_flow_payload.flows.{default_flow_id}.lanes"),
            )
        })?;
    let authority = serde_json::json!({
        "schema_version":"team-flow-authority.v1",
        "authority_id":authority_id,
        "content_blake3":authority_identity_hash,
        "config":config_identity,
        "registries":registry_identities_value,
        "selected_config":selected_config,
        "lanes":default_lanes,
        "resolved_all_flow_payload":resolved_all_flow_payload,
        "resolved_all_flow_payload_blake3":resolved_content_hash,
        "authority_source":{
            "kind":"resolved_all_flow_payload",
            "payload_path":"team_flow_authority.resolved_all_flow_payload",
            "payload_blake3":resolved_content_hash,
            "identity_phase":"phase_2_persisted_payload"
        },
        "source_of_truth":{"options":"dev_team.authority_catalog","selection":"dev_team.authority_selection","schema":"vida/config/schemas/team-flow-authority.schema.json"}
    });
    crate::team_flow_authority_adapter::compile_team_flow_authority(
        &serde_json::json!({"team_flow_authority": authority.clone()}),
        None,
        None,
    )
    .map_err(|error| {
        TeamFlowAuthorityMaterializationBlocker::new(
            "team_flow_authority_persisted_self_validation_failed",
            error,
        )
    })?;
    Ok(ResolvedTeamFlowAuthority {
        authority,
        authority_id,
        authority_identity_hash,
        resolved_content_hash,
        config_hash,
        registry_hash,
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    pub(crate) fn canonical_compiled_bundle() -> Value {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let config = crate::project_activator_surface::read_yaml_file_checked(
            &repository_root.join("vida.config.yaml"),
        )
        .expect("repository master config should load");
        crate::compiled_agent_extension_bundle::build_compiled_agent_extension_bundle_for_root(
            &config,
            repository_root,
        )
        .expect("repository master config should compile the canonical bundle")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        derive_evidence_requirements, normalize_command, resolve_work_item_flow_bindings,
        source_value, test_support::canonical_compiled_bundle, validate_source_dev_team,
    };
    use serde_json::{json, Map, Value};

    #[test]
    fn evidence_requirement_derivation_matrix_is_typed_and_conflict_safe() {
        let cases = [
            (
                "derive",
                json!({}),
                vec![json!("changed_files"), json!("verification_notes")],
                None,
            ),
            (
                "explicit_equal",
                json!({"evidence_requirements":["changed_files","verification_notes"]}),
                vec![json!("changed_files"), json!("verification_notes")],
                None,
            ),
            (
                "explicit_conflict",
                json!({"evidence_requirements":["other"]}),
                vec![json!("changed_files")],
                Some("team_flow_authority_evidence_requirements_conflict"),
            ),
            (
                "explicit_invalid",
                json!({"evidence_requirements":[{"id":"changed_files"}]}),
                vec![json!("changed_files")],
                Some("team_flow_authority_evidence_requirement_invalid"),
            ),
        ];

        for (name, step, proof_outputs, expected_code) in cases {
            let step = step.as_object().expect("matrix step object");
            let result = derive_evidence_requirements(step, &proof_outputs, "flow.steps[0]");
            match expected_code {
                None => assert_eq!(
                    result.expect(name).as_array().expect("typed evidence"),
                    &proof_outputs
                ),
                Some(code) => assert_eq!(result.expect_err(name).code, code),
            }
        }

        let missing = derive_evidence_requirements(&Map::new(), &[], "flow.steps[0]")
            .expect("optional empty proof source remains typed");
        assert_eq!(missing, Value::Array(Vec::new()));
    }

    #[test]
    fn canonical_bundle_replay_preserves_hash_and_persisted_policy_source() {
        let first = canonical_compiled_bundle();
        let replay = canonical_compiled_bundle();
        assert_eq!(
            first["team_flow_authority"]["authority_id"],
            replay["team_flow_authority"]["authority_id"]
        );
        assert_eq!(
            first["team_flow_authority"]["resolved_all_flow_payload_blake3"],
            replay["team_flow_authority"]["resolved_all_flow_payload_blake3"]
        );

        let flows = first["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("persisted flow payload");
        assert!(!flows.is_empty(), "master config must provide a flow");
        for flow in flows {
            for lane in flow["lanes"].as_array().expect("persisted lanes") {
                assert_eq!(
                    lane["evidence_requirements"],
                    lane["proof_gates"]["required_outputs"]
                );
                assert_eq!(
                    lane["policy_diagnostics"]["source"],
                    "team_flow_authority.selected_config"
                );
            }
        }
    }

    #[test]
    fn aliases_reject_duplicates_and_malformed_command_ids() {
        let commands = json!({
            "commands": [{"command_id": "opaque-command", "surface": "opaque-surface"}]
        });
        let duplicate_mapping = json!({
            "command_mapping": {"command_id": "opaque-command", "ref": "opaque-command"}
        });
        let duplicate_mapping = duplicate_mapping.as_object().expect("mapping object");
        let error = normalize_command(duplicate_mapping, &commands, "flow.steps[0]")
            .expect_err("duplicate command aliases must fail closed");
        assert_eq!(error.code, "team_flow_authority_conflicting_source_aliases");

        let malformed_mapping = json!({"command_mapping": {"command_id": 7}});
        let malformed_mapping = malformed_mapping.as_object().expect("mapping object");
        let error = normalize_command(malformed_mapping, &commands, "flow.steps[0]")
            .expect_err("malformed command alias must fail closed");
        assert_eq!(error.code, "team_flow_authority_command_reference_invalid");

        let duplicate_rework = json!({
            "rework_transitions": {"opaque": "opaque-target"},
            "rework": {"opaque": "opaque-target"}
        });
        let duplicate_rework = duplicate_rework.as_object().expect("rework object");
        let error = source_value(
            duplicate_rework,
            &["rework_transitions", "rework"],
            "flow.steps[0]",
        )
        .expect_err("duplicate rework aliases must fail closed");
        assert_eq!(error.code, "team_flow_authority_conflicting_source_aliases");
    }

    #[test]
    fn source_dsl_catalog_rejects_unknown_fields_and_approval_modes() {
        let catalog = json!({
            "authority_catalog": {
                "source_flow_fields": ["steps", "entry_node_id"],
                "source_flow_step_fields": [
                    "role_id", "required", "next_node", "dispatch_alias", "terminal",
                    "approval_policy"
                ],
                "approval_policy_modes": ["user_review_required"]
            }
        });
        let unknown_flow = json!({"flows": {"flow": {"entry_node_id": "role", "steps": [], "unknown_flow": true}}});
        let error = validate_source_dev_team(
            unknown_flow.as_object().expect("source flow object"),
            &catalog,
        )
        .expect_err("unknown source flow fields must fail closed");
        assert_eq!(
            error.code,
            "team_flow_authority_source_flow_field_unsupported"
        );

        let unknown_step = json!({
            "flows": {"flow": {"entry_node_id": "role", "steps": [{"role_id": "role", "unknown_step": true}]}}
        });
        let error = validate_source_dev_team(
            unknown_step.as_object().expect("source flow object"),
            &catalog,
        )
        .expect_err("unknown source flow-step fields must fail closed");
        assert_eq!(
            error.code,
            "team_flow_authority_source_flow_step_field_unsupported"
        );

        let unsupported_mode = json!({
            "flows": {
                "flow": {
                    "entry_node_id": "role",
                    "steps": [{
                        "role_id": "role",
                        "approval_policy": {"mode": "unsupported"}
                    }]
                }
            }
        });
        let error = validate_source_dev_team(
            unsupported_mode.as_object().expect("source flow object"),
            &catalog,
        )
        .expect_err("unsupported approval modes must fail closed");
        assert_eq!(
            error.code,
            "team_flow_authority_source_approval_policy_mode_unsupported"
        );
    }

    #[test]
    fn work_item_flow_binding_keys_are_canonicalized_and_collisions_fail_closed() {
        let flows = serde_json::json!({"delivery": {}});
        let flows = flows.as_object().expect("flow map");
        let mixed_case = serde_json::json!({
            "work_item_flow_bindings": {"  DeFeCt  ": "delivery"}
        });
        let normalized = resolve_work_item_flow_bindings(
            mixed_case.as_object().expect("dev-team object"),
            flows,
        )
        .expect("mixed-case binding should normalize");
        assert_eq!(
            normalized.get("defect").and_then(Value::as_str),
            Some("delivery")
        );

        let empty = serde_json::json!({"work_item_flow_bindings": {"  ": "delivery"}});
        assert_eq!(
            resolve_work_item_flow_bindings(empty.as_object().expect("dev-team object"), flows)
                .expect_err("empty binding key must fail")
                .code,
            "team_flow_authority_work_item_flow_binding_invalid"
        );

        let collision = serde_json::json!({
            "work_item_flow_bindings": {
                "Defect": "delivery",
                "defect": "delivery"
            }
        });
        assert_eq!(
            resolve_work_item_flow_bindings(
                collision.as_object().expect("dev-team object"),
                flows,
            )
            .expect_err("case-colliding binding keys must fail")
            .code,
            "team_flow_authority_work_item_flow_binding_key_collision"
        );
    }
}
