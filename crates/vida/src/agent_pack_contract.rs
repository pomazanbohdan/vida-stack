use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(crate) const RUNTIME_PACKS_PROJECTION: &str = ".vida/project/agent-extensions/packs.yaml";

#[derive(Debug, Clone)]
pub(crate) struct PackRegistryProjection {
    pub(crate) source_path: Option<String>,
    pub(crate) packs: Vec<serde_json::Value>,
    pub(crate) blocker_codes: Vec<String>,
}

pub(crate) fn canonical_dev_team_role_id(role_id: &str) -> String {
    role_id.trim().to_string()
}

pub(crate) fn canonical_role_alias_source(role_id: &str) -> Option<&'static str> {
    let _ = role_id;
    None
}

#[derive(Debug, Clone, Default)]
struct ConfiguredPackRoleContract {
    runtime_role: Option<String>,
    task_class: Option<String>,
}

fn configured_dev_team_role_contracts(
    overlay: &serde_yaml::Value,
) -> BTreeMap<String, ConfiguredPackRoleContract> {
    let Some(roles) =
        crate::yaml_lookup(overlay, &["dev_team", "roles"]).and_then(serde_yaml::Value::as_mapping)
    else {
        return BTreeMap::new();
    };
    roles
        .iter()
        .filter_map(|(role_id, contract)| {
            let role_id = role_id.as_str().map(canonical_dev_team_role_id)?;
            if role_id.is_empty() {
                return None;
            }
            let runtime_role = crate::yaml_string(crate::yaml_lookup(contract, &["runtime_role"]))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let task_class_values =
                crate::yaml_string_list(crate::yaml_lookup(contract, &["task_classes"]));
            let task_class = if task_class_values.len() == 1 {
                task_class_values.into_iter().next()
            } else {
                crate::yaml_string(crate::yaml_lookup(contract, &["task_class"]))
            }
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
            Some((
                role_id,
                ConfiguredPackRoleContract {
                    runtime_role,
                    task_class,
                },
            ))
        })
        .collect()
}

pub(crate) fn default_worktree_policy_for_step(
    _role_id: &str,
    task_class: &str,
    receive_mode: &str,
) -> &'static str {
    if task_class == "quality_gate" && receive_mode == "batch" {
        return "isolated_per_lane";
    }
    if matches!(
        task_class,
        "implementation" | "delivery_task" | "execution_block" | "refactor" | "test_authoring"
    ) {
        "isolated_per_task"
    } else {
        "current"
    }
}

pub(crate) fn canonical_receive_mode(receive_mode: &str) -> String {
    match receive_mode.trim().to_ascii_lowercase().as_str() {
        "" | "single" | "task" => "task".to_string(),
        "batch" => "batch".to_string(),
        value => value.to_string(),
    }
}

pub(crate) fn supported_receive_mode(receive_mode: &str) -> bool {
    matches!(
        canonical_receive_mode(receive_mode).as_str(),
        "task" | "batch"
    )
}

pub(crate) fn load_pack_registry_for_root(root: &Path) -> Result<PackRegistryProjection, String> {
    let overlay = crate::config_value_utils::load_project_overlay_yaml_for_root(root)?;
    Ok(load_pack_registry_from_overlay(root, &overlay))
}

pub(crate) fn load_pack_registry_from_overlay(
    root: &Path,
    overlay: &serde_yaml::Value,
) -> PackRegistryProjection {
    let Some(configured_path) = crate::yaml_string(crate::yaml_lookup(
        overlay,
        &["agent_extensions", "registries", "packs"],
    )) else {
        return PackRegistryProjection {
            source_path: None,
            packs: Vec::new(),
            blocker_codes: Vec::new(),
        };
    };
    let source_path =
        crate::project_activator_surface::resolve_overlay_path(root, &configured_path);
    let mut blocker_codes = Vec::new();
    let registry = match crate::project_activator_surface::read_yaml_file_checked(&source_path) {
        Ok(value) => value,
        Err(error) => {
            return PackRegistryProjection {
                source_path: Some(configured_path),
                packs: Vec::new(),
                blocker_codes: vec![format!("pack_registry_unreadable:{error}")],
            };
        }
    };
    let Some(rows) =
        crate::yaml_lookup(&registry, &["packs"]).and_then(serde_yaml::Value::as_sequence)
    else {
        return PackRegistryProjection {
            source_path: Some(configured_path),
            packs: Vec::new(),
            blocker_codes: vec!["pack_registry_missing_packs".to_string()],
        };
    };
    let configured_role_contracts = configured_dev_team_role_contracts(overlay);
    let registry_refs = registry_refs(root, overlay);
    let mut seen_pack_ids = BTreeSet::new();
    let packs = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let pack = compile_pack_row(
                row,
                index,
                &configured_path,
                &configured_role_contracts,
                &registry_refs,
                &mut blocker_codes,
            )?;
            if let Some(pack_id) = pack["pack_id"].as_str() {
                if !seen_pack_ids.insert(pack_id.to_string()) {
                    blocker_codes.push(format!("duplicate_pack_id:{pack_id}"));
                }
            }
            Some(pack)
        })
        .collect::<Vec<_>>();

    PackRegistryProjection {
        source_path: Some(configured_path),
        packs,
        blocker_codes,
    }
}

pub(crate) fn normalized_pack_lookup_key(pack_id: &str) -> String {
    pack_id.trim().to_ascii_lowercase()
}

pub(crate) fn pack_id_matches(pack: &serde_json::Value, requested_pack_id: &str) -> bool {
    let requested = normalized_pack_lookup_key(requested_pack_id);
    if requested.is_empty() {
        return false;
    }
    pack["pack_id"]
        .as_str()
        .is_some_and(|pack_id| normalized_pack_lookup_key(pack_id) == requested)
}

pub(crate) fn pack_by_id<'a>(
    readiness: &'a serde_json::Value,
    pack_id: &str,
) -> Option<&'a serde_json::Value> {
    readiness["packs"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|pack| pack_id_matches(pack, pack_id))
}

pub(crate) fn pack_validation_blockers(pack: &serde_json::Value) -> Vec<String> {
    crate::json_string_list(pack.get("blocker_codes"))
}

pub(crate) fn pack_flow_id(pack: &serde_json::Value) -> Option<&str> {
    pack["flow_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn compile_pack_row(
    row: &serde_yaml::Value,
    index: usize,
    configured_path: &str,
    configured_role_contracts: &BTreeMap<String, ConfiguredPackRoleContract>,
    registry_refs: &RegistryRefs,
    registry_blockers: &mut Vec<String>,
) -> Option<serde_json::Value> {
    let pack_id = crate::yaml_string(crate::yaml_lookup(row, &["pack_id"]))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let Some(pack_id) = pack_id else {
        registry_blockers.push(format!("missing_pack_id:{index}"));
        return None;
    };
    let mut blocker_codes = Vec::new();
    let flow_id = crate::yaml_string(crate::yaml_lookup(row, &["flow_id"]))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if flow_id.is_none() {
        blocker_codes.push(format!("missing_pack_flow_id:{pack_id}"));
    } else if let Some(flow_id) = flow_id.as_deref() {
        if !registry_refs.flow_ids.contains(flow_id) {
            blocker_codes.push(format!("unknown_pack_flow_id:{pack_id}:{flow_id}"));
        }
    }
    let aliases = pack_aliases(row, &pack_id);
    if !aliases.is_empty() {
        blocker_codes.push(format!("pack_aliases_not_supported:{pack_id}"));
    }
    let enabled = crate::yaml_bool(crate::yaml_lookup(row, &["enabled"]), true);
    let terminal_proof_target =
        crate::yaml_string(crate::yaml_lookup(row, &["terminal_proof_target"])).or_else(|| {
            crate::yaml_string(crate::yaml_lookup(
                row,
                &["proof_gates", "terminal_proof_target"],
            ))
        });
    let ordered_steps = compile_pack_steps(
        &pack_id,
        row,
        terminal_proof_target.as_deref(),
        configured_role_contracts,
        registry_refs,
        &mut blocker_codes,
    );
    if ordered_steps.is_empty() {
        blocker_codes.push(format!("missing_pack_steps:{pack_id}"));
    }
    let terminal_proof_target = ordered_steps
        .last()
        .and_then(|step| step["proof_target"].as_str())
        .map(str::to_string)
        .or(terminal_proof_target);
    if terminal_proof_target
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        blocker_codes.push(format!("missing_terminal_proof_target:{pack_id}"));
    }

    let mut combined_blockers = blocker_codes.clone();
    registry_blockers.extend(blocker_codes);
    combined_blockers.sort();
    combined_blockers.dedup();
    Some(serde_json::json!({
        "pack_id": pack_id,
        "aliases": aliases,
        "flow_id": flow_id,
        "enabled": enabled,
        "default": crate::yaml_bool(crate::yaml_lookup(row, &["default"]), false),
        "description": crate::yaml_string(crate::yaml_lookup(row, &["description"])),
        "work_item_bindings": crate::yaml_string_list(crate::yaml_lookup(row, &["work_item_bindings"])),
        "ordered_steps": ordered_steps,
        "terminal_proof_target": terminal_proof_target,
        "external_refs": yaml_field_json(row, "external_refs"),
        "source_path": configured_path,
        "status": if combined_blockers.is_empty() && enabled { "ready" } else { "blocked" },
        "blocker_codes": combined_blockers,
    }))
}

fn pack_aliases(row: &serde_yaml::Value, pack_id: &str) -> Vec<String> {
    let mut aliases = crate::yaml_string_list(crate::yaml_lookup(row, &["aliases"]));
    let _ = pack_id;
    aliases.sort();
    aliases.dedup();
    aliases
}

fn compile_pack_steps(
    pack_id: &str,
    pack: &serde_yaml::Value,
    pack_terminal_proof_target: Option<&str>,
    configured_role_contracts: &BTreeMap<String, ConfiguredPackRoleContract>,
    registry_refs: &RegistryRefs,
    blocker_codes: &mut Vec<String>,
) -> Vec<serde_json::Value> {
    let Some(step_rows) =
        crate::yaml_lookup(pack, &["ordered_steps"]).and_then(serde_yaml::Value::as_sequence)
    else {
        return Vec::new();
    };
    let last_index = step_rows.len().saturating_sub(1);
    step_rows
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            let configured_role_id = crate::yaml_string(crate::yaml_lookup(step, &["role_id"]))
                .or_else(|| crate::yaml_string(crate::yaml_lookup(step, &["role"])))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let Some(configured_role_id) = configured_role_id else {
                blocker_codes.push(format!("missing_pack_step_role:{pack_id}:{index}"));
                return None;
            };
            if !is_canonical_role_id(&configured_role_id) {
                blocker_codes.push(format!(
                    "non_canonical_pack_step_role:{pack_id}:{configured_role_id}"
                ));
            }
            let canonical_role_id = canonical_dev_team_role_id(&configured_role_id);
            let configured_role_contract = configured_role_contracts.get(&canonical_role_id);
            if configured_role_contract.is_none() {
                blocker_codes.push(format!(
                    "unknown_pack_step_role:{pack_id}:{configured_role_id}"
                ));
            }
            let runtime_role = crate::yaml_string(crate::yaml_lookup(step, &["runtime_role"]))
                .or_else(|| {
                    configured_role_contract.and_then(|contract| contract.runtime_role.clone())
                })
                .unwrap_or_default();
            if runtime_role.is_empty() {
                blocker_codes.push(format!("missing_pack_step_runtime_role:{pack_id}:{index}"));
            }
            let task_class = crate::yaml_string(crate::yaml_lookup(step, &["task_class"]))
                .or_else(|| {
                    configured_role_contract.and_then(|contract| contract.task_class.clone())
                })
                .unwrap_or_default();
            if task_class.is_empty() {
                blocker_codes.push(format!("missing_pack_step_task_class:{pack_id}:{index}"));
            }
            let configured_receive_mode =
                crate::yaml_string(crate::yaml_lookup(step, &["receive_mode"]))
                    .map(|value| value.trim().to_ascii_lowercase())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "task".to_string());
            let receive_mode = canonical_receive_mode(&configured_receive_mode);
            if !supported_receive_mode(&configured_receive_mode) {
                blocker_codes.push(format!(
                    "unsupported_pack_step_receive_mode:{pack_id}:{index}:{configured_receive_mode}"
                ));
            }
            let worktree_policy =
                crate::yaml_string(crate::yaml_lookup(step, &["worktree_policy"]))
                    .map(|value| value.trim().to_ascii_lowercase())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| {
                        default_worktree_policy_for_step(
                            &canonical_role_id,
                            &task_class,
                            &receive_mode,
                        )
                        .to_string()
                    });
            if !matches!(
                worktree_policy.as_str(),
                "current" | "isolated_per_task" | "isolated_per_lane"
            ) {
                blocker_codes.push(format!(
                    "unsupported_pack_step_worktree_policy:{pack_id}:{index}:{worktree_policy}"
                ));
            }
            let proof_target = crate::yaml_string(crate::yaml_lookup(step, &["proof_target"]))
                .or_else(|| {
                    crate::yaml_string(crate::yaml_lookup(step, &["proof_target_template"]))
                })
                .or_else(|| {
                    if index == last_index {
                        pack_terminal_proof_target.map(str::to_string)
                    } else {
                        None
                    }
                });
            if index == last_index
                && proof_target
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
            {
                blocker_codes.push(format!(
                    "missing_pack_step_terminal_proof_target:{pack_id}:{index}"
                ));
            }
            let command_ref = crate::yaml_string(crate::yaml_lookup(step, &["command_ref"]))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            match command_ref.as_deref() {
                Some(command_ref) if !registry_refs.command_ids.contains(command_ref) => {
                    blocker_codes.push(format!(
                        "unknown_pack_step_command_ref:{pack_id}:{index}:{command_ref}"
                    ));
                }
                Some(_) => {}
                None => {
                    blocker_codes.push(format!("missing_pack_step_command_ref:{pack_id}:{index}"));
                }
            }
            let dispatch_alias = crate::yaml_string(crate::yaml_lookup(step, &["dispatch_alias"]))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            if let Some(dispatch_alias) = dispatch_alias.as_deref() {
                if !registry_refs.dispatch_alias_ids.contains(dispatch_alias) {
                    blocker_codes.push(format!(
                        "unknown_pack_step_dispatch_alias:{pack_id}:{index}:{dispatch_alias}"
                    ));
                }
            }
            let role_contract = role_contract_from_yaml(step);
            Some(serde_json::json!({
                "step_id": crate::yaml_string(crate::yaml_lookup(step, &["step_id"]))
                    .unwrap_or_else(|| format!("{pack_id}-{index}")),
                "order": index,
                "role_id": canonical_role_id.clone(),
                "configured_role_id": configured_role_id.clone(),
                "canonical_role_id": canonical_dev_team_role_id(&configured_role_id),
                "role_alias_source": canonical_role_alias_source(&configured_role_id),
                "runtime_role": runtime_role,
                "task_class": task_class,
                "receive_mode": receive_mode,
                "worktree_policy": worktree_policy,
                "proof_target": proof_target,
                "command_ref": command_ref,
                "requires_user_approval": crate::yaml_bool(
                    crate::yaml_lookup(step, &["requires_user_approval"]),
                    false,
                ),
                "approval_policy": yaml_field_json(step, "approval_policy"),
                "proof_gates": yaml_field_json(step, "proof_gates"),
                "adapter_projection": yaml_field_json(step, "adapter_projection"),
                "dispatch_alias": dispatch_alias,
                "external_refs": yaml_field_json(step, "external_refs"),
                "quality_profile": yaml_field_json(step, "quality_profile"),
                "batch_policy": yaml_field_json(step, "batch_policy"),
                "owned_paths": yaml_field_json(step, "owned_paths"),
                "conflict_domain": yaml_field_json(step, "conflict_domain"),
                "order_bucket": yaml_field_json(step, "order_bucket"),
                "parallel_group": yaml_field_json(step, "parallel_group"),
                "role_contract": role_contract,
            }))
        })
        .collect()
}

pub(crate) fn role_contract_from_yaml(value: &serde_yaml::Value) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for key in [
        "owns",
        "does_not_own",
        "allowed_mutations",
        "required_outputs",
        "handoff_target",
        "forbidden_actions",
    ] {
        let field = yaml_field_json(value, key);
        if !field.is_null() {
            object.insert(key.to_string(), field);
        }
    }
    if object.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(object)
    }
}

struct RegistryRefs {
    flow_ids: BTreeSet<String>,
    command_ids: BTreeSet<String>,
    dispatch_alias_ids: BTreeSet<String>,
}

fn registry_refs(root: &Path, overlay: &serde_yaml::Value) -> RegistryRefs {
    RegistryRefs {
        flow_ids: configured_flow_ids(root, overlay),
        command_ids: configured_command_ids(root, overlay),
        dispatch_alias_ids: configured_dispatch_alias_ids(root, overlay),
    }
}

fn configured_flow_ids(root: &Path, overlay: &serde_yaml::Value) -> BTreeSet<String> {
    let mut flow_ids = BTreeSet::new();
    if let Some(mapping) =
        crate::yaml_lookup(overlay, &["dev_team", "flows"]).and_then(serde_yaml::Value::as_mapping)
    {
        flow_ids.extend(
            mapping
                .keys()
                .filter_map(serde_yaml::Value::as_str)
                .map(str::to_string),
        );
    }
    if let Some(configured_path) = crate::yaml_string(crate::yaml_lookup(
        overlay,
        &["agent_extensions", "registries", "flows"],
    )) {
        let source_path =
            crate::project_activator_surface::resolve_overlay_path(root, &configured_path);
        if let Ok(registry) = crate::project_activator_surface::read_yaml_file_checked(&source_path)
        {
            if let Some(rows) = crate::yaml_lookup(&registry, &["flow_sets"])
                .and_then(serde_yaml::Value::as_sequence)
            {
                flow_ids.extend(rows.iter().filter_map(|row| {
                    crate::yaml_string(crate::yaml_lookup(row, &["flow_id"]))
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty())
                }));
            }
        }
    }
    flow_ids
}

fn configured_command_ids(root: &Path, overlay: &serde_yaml::Value) -> BTreeSet<String> {
    let Some(configured_path) = crate::yaml_string(crate::yaml_lookup(
        overlay,
        &["agent_extensions", "registries", "commands"],
    )) else {
        return BTreeSet::new();
    };
    let source_path =
        crate::project_activator_surface::resolve_overlay_path(root, &configured_path);
    let Ok(registry) = crate::project_activator_surface::read_yaml_file_checked(&source_path)
    else {
        return BTreeSet::new();
    };
    crate::yaml_lookup(&registry, &["commands"])
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            crate::yaml_string(crate::yaml_lookup(row, &["command_id"]))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .collect()
}

fn configured_dispatch_alias_ids(root: &Path, overlay: &serde_yaml::Value) -> BTreeSet<String> {
    let Some(configured_path) = crate::yaml_string(crate::yaml_lookup(
        overlay,
        &["agent_extensions", "registries", "dispatch_aliases"],
    )) else {
        return BTreeSet::new();
    };
    let source_path =
        crate::project_activator_surface::resolve_overlay_path(root, &configured_path);
    let Ok(registry) = crate::project_activator_surface::read_yaml_file_checked(&source_path)
    else {
        return BTreeSet::new();
    };
    crate::yaml_lookup(&registry, &["dispatch_aliases"])
        .and_then(serde_yaml::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            crate::yaml_string(crate::yaml_lookup(row, &["alias_id"]))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .collect()
}

fn is_canonical_role_id(role_id: &str) -> bool {
    let trimmed = role_id.trim();
    !trimmed.is_empty()
        && trimmed == role_id
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn yaml_field_json(value: &serde_yaml::Value, key: &str) -> serde_json::Value {
    crate::yaml_lookup(value, &[key])
        .and_then(|entry| serde_json::to_value(entry).ok())
        .unwrap_or(serde_json::Value::Null)
}

pub(crate) fn resolve_config_root_for_pack_surface() -> Result<PathBuf, String> {
    if let Ok(root) = crate::resolve_runtime_project_root() {
        return Ok(root);
    }
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("Failed to resolve current directory: {error}"))?;
    for candidate in current_dir.ancestors() {
        if candidate.join("vida.config.yaml").is_file() {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(format!(
        "Unable to resolve VIDA config root from {}. Run inside a tree with vida.config.yaml or set VIDA_ROOT explicitly.",
        current_dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;

    #[test]
    fn dev_team_role_ids_are_strict_and_do_not_alias() {
        assert_eq!(canonical_dev_team_role_id("specifier"), "specifier");
        assert_eq!(canonical_dev_team_role_id("coder"), "coder");
        assert_eq!(canonical_dev_team_role_id("analyst"), "analyst");
        assert_eq!(canonical_dev_team_role_id("developer"), "developer");
        assert_eq!(canonical_role_alias_source("developer"), None);
    }

    #[test]
    fn carrier_tier_names_do_not_become_roles_except_existing_architect_role() {
        assert_eq!(canonical_dev_team_role_id("junior"), "junior");
        assert_eq!(canonical_dev_team_role_id("middle"), "middle");
        assert_eq!(canonical_dev_team_role_id("senior"), "senior");
        assert_eq!(canonical_dev_team_role_id("architect"), "architect");
    }

    #[test]
    fn pack_validation_requires_terminal_proof_target() {
        let root = TempStateHarness::new().expect("temp state harness");
        std::fs::write(
            root.path().join("packs.yaml"),
            concat!(
                "version: 1\n",
                "packs:\n",
                "  - pack_id: bad_pack\n",
                "    flow_id: bad_pack_flow\n",
                "    ordered_steps:\n",
                "      - role_id: coder\n",
                "        task_class: implementation\n",
                "        command_ref: agent-init-worker\n",
            ),
        )
        .expect("pack registry should write");
        std::fs::write(
            root.path().join("flows.yaml"),
            "version: 1\nflow_sets:\n  - flow_id: bad_pack_flow\n",
        )
        .expect("flow registry should write");
        std::fs::write(
            root.path().join("commands.yaml"),
            "version: 1\ncommands:\n  - command_id: agent-init-worker\n",
        )
        .expect("command registry should write");
        let overlay: serde_yaml::Value =
            serde_yaml::from_str(
                "agent_extensions:\n  registries:\n    packs: packs.yaml\n    flows: flows.yaml\n    commands: commands.yaml\n",
            )
                .expect("overlay");

        let registry = load_pack_registry_from_overlay(root.path(), &overlay);

        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code.contains("missing_pack_step_terminal_proof_target:bad_pack")));
        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code == "unknown_pack_step_role:bad_pack:coder"));
        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code == "missing_pack_step_runtime_role:bad_pack:0"));
    }

    #[test]
    fn pack_steps_get_default_worktree_policy() {
        assert_eq!(
            default_worktree_policy_for_step("configured_writer", "implementation", "single"),
            "isolated_per_task"
        );
        assert_eq!(
            default_worktree_policy_for_step("configured_verifier", "verification", "task"),
            "current"
        );
        assert_eq!(
            default_worktree_policy_for_step("configured_gate", "quality_gate", "batch"),
            "isolated_per_lane"
        );
    }

    #[test]
    fn pack_lookup_rejects_implicit_hyphen_underscore_aliases() {
        let readiness = serde_json::json!({
            "packs": [
                {
                    "pack_id": "quick-two-pack",
                    "aliases": [],
                    "flow_id": "quick-two-pack-flow"
                }
            ]
        });

        assert!(pack_by_id(&readiness, "quick-two-pack").is_some());
        assert!(pack_by_id(&readiness, "quick_two_pack").is_none());
    }

    #[test]
    fn pack_validation_rejects_non_canonical_roles_and_missing_command_refs() {
        let root = TempStateHarness::new().expect("temp state harness");
        std::fs::write(
            root.path().join("packs.yaml"),
            concat!(
                "version: 1\n",
                "packs:\n",
                "  - pack_id: bad-pack\n",
                "    flow_id: missing-flow\n",
                "    aliases: [bad_pack]\n",
                "    ordered_steps:\n",
                "      - role_id: developer-role\n",
                "        task_class: implementation\n",
                "        proof_target: agent:bad-pack:developer-role\n",
            ),
        )
        .expect("pack registry should write");
        let overlay: serde_yaml::Value =
            serde_yaml::from_str("agent_extensions:\n  registries:\n    packs: packs.yaml\n")
                .expect("overlay");

        let registry = load_pack_registry_from_overlay(root.path(), &overlay);

        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code == "pack_aliases_not_supported:bad-pack"));
        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code == "unknown_pack_flow_id:bad-pack:missing-flow"));
        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code == "non_canonical_pack_step_role:bad-pack:developer-role"));
        assert!(registry
            .blocker_codes
            .iter()
            .any(|code| code == "missing_pack_step_command_ref:bad-pack:0"));
    }

    #[test]
    fn pack_validation_accepts_known_flow_command_and_dispatch_alias_refs() {
        let root = TempStateHarness::new().expect("temp state harness");
        std::fs::write(
            root.path().join("packs.yaml"),
            concat!(
                "version: 1\n",
                "packs:\n",
                "  - pack_id: good-pack\n",
                "    flow_id: good_flow\n",
                "    ordered_steps:\n",
                "      - role_id: coder\n",
                "        task_class: implementation\n",
                "        command_ref: agent-init-worker\n",
                "        dispatch_alias: development_refactorer\n",
                "        proof_target: agent:good-pack:coder\n",
            ),
        )
        .expect("pack registry should write");
        std::fs::write(
            root.path().join("flows.yaml"),
            "version: 1\nflow_sets:\n  - flow_id: good_flow\n",
        )
        .expect("flow registry should write");
        std::fs::write(
            root.path().join("commands.yaml"),
            "version: 1\ncommands:\n  - command_id: agent-init-worker\n",
        )
        .expect("command registry should write");
        std::fs::write(
            root.path().join("dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases:\n  - alias_id: development_refactorer\n",
        )
        .expect("dispatch alias registry should write");
        let overlay: serde_yaml::Value =
            serde_yaml::from_str(
                "dev_team:\n  roles:\n    coder:\n      runtime_role: worker\n      task_classes: [implementation]\nagent_extensions:\n  registries:\n    packs: packs.yaml\n    flows: flows.yaml\n    commands: commands.yaml\n    dispatch_aliases: dispatch-aliases.yaml\n",
            )
            .expect("overlay");

        let registry = load_pack_registry_from_overlay(root.path(), &overlay);

        assert!(
            registry.blocker_codes.is_empty(),
            "{:?}",
            registry.blocker_codes
        );
        assert_eq!(
            registry.packs[0]["ordered_steps"][0]["command_ref"],
            "agent-init-worker"
        );
        assert_eq!(
            registry.packs[0]["ordered_steps"][0]["dispatch_alias"],
            "development_refactorer"
        );
    }

    #[test]
    fn receive_mode_single_is_rendered_as_task() {
        assert_eq!(canonical_receive_mode("single"), "task");
        assert_eq!(canonical_receive_mode("task"), "task");
        assert_eq!(canonical_receive_mode("batch"), "batch");
    }

    #[test]
    fn project_pack_registry_resolves_configured_role_pack_flows() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");

        let registry =
            load_pack_registry_for_root(&root).expect("project pack registry should load");
        let overlay =
            crate::config_value_utils::load_project_overlay_yaml_for_root(&root).expect("overlay");
        let configured_flow_path = crate::yaml_string(crate::yaml_lookup(
            &overlay,
            &["agent_extensions", "registries", "flows"],
        ));
        assert_eq!(
            configured_flow_path.as_deref(),
            Some("docs/process/agent-extensions/flows.yaml")
        );
        let flow_registry_path = crate::project_activator_surface::resolve_overlay_path(
            &root,
            configured_flow_path.as_deref().expect("flow path"),
        );
        let flow_registry =
            crate::project_activator_surface::read_yaml_file_checked(&flow_registry_path)
                .expect("flow registry should read");
        assert!(
            crate::yaml_lookup(&flow_registry, &["flow_sets"])
                .and_then(serde_yaml::Value::as_sequence)
                .is_some_and(|rows| rows.iter().any(|row| {
                    crate::yaml_string(crate::yaml_lookup(row, &["flow_id"])).as_deref()
                        == Some("quick-two-pack-flow")
                })),
            "flow registry path: {}",
            flow_registry_path.display()
        );
        let flow_ids = configured_flow_ids(&root, &overlay);

        assert!(
            flow_ids.contains("quick-two-pack-flow"),
            "flow ids: {:?}",
            flow_ids
        );

        assert!(
            !registry
                .blocker_codes
                .iter()
                .any(|code| code.starts_with("unknown_pack_flow_id:")),
            "{:?}",
            registry.blocker_codes
        );
    }
}
