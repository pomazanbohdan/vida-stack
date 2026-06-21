use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) const RUNTIME_PACKS_PROJECTION: &str = ".vida/project/agent-extensions/packs.yaml";

#[derive(Debug, Clone)]
pub(crate) struct PackRegistryProjection {
    pub(crate) source_path: Option<String>,
    pub(crate) packs: Vec<serde_json::Value>,
    pub(crate) blocker_codes: Vec<String>,
}

pub(crate) fn canonical_dev_team_role_id(role_id: &str) -> String {
    let trimmed = role_id.trim();
    let normalized = trimmed.to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "analyst" | "business_analyst" => "specifier".to_string(),
        "developer" | "worker" | "implementer" => "coder".to_string(),
        "coach_test_gate" => "reviewer_test_gate".to_string(),
        "coach_implementation_gate" => "reviewer_implementation_gate".to_string(),
        "coach_validator" => "reviewer_validator".to_string(),
        "reviewer" | "adversarial_reviewer" => "adversarial_reviewer".to_string(),
        "duplication_reviewer" => "cleaner_review_gate".to_string(),
        "solution_architect" => "architect".to_string(),
        "hardener" | "hardender" => "hardender".to_string(),
        "tester" | "verifier" | "qa" | "qa_tester" => "qa_tester".to_string(),
        "autotester" => "test_author".to_string(),
        _ => trimmed.to_string(),
    }
}

pub(crate) fn canonical_role_alias_source(role_id: &str) -> Option<&'static str> {
    let trimmed = role_id.trim();
    if trimmed.is_empty() || canonical_dev_team_role_id(trimmed) == trimmed {
        None
    } else {
        Some("agent_pack_role_alias")
    }
}

pub(crate) fn known_pack_role(role_id: &str) -> bool {
    matches!(
        canonical_dev_team_role_id(role_id).as_str(),
        "specifier"
            | "coder"
            | "reviewer_test_gate"
            | "reviewer_implementation_gate"
            | "reviewer_validator"
            | "cleaner_review_gate"
            | "cleaner"
            | "refactorer"
            | "architect"
            | "hardender"
            | "qa_tester"
            | "adversarial_reviewer"
            | "prover"
            | "release_closure"
            | "test_author"
    )
}

pub(crate) fn default_runtime_role_for_canonical_role(role_id: &str) -> Option<&'static str> {
    match canonical_dev_team_role_id(role_id).as_str() {
        "specifier" => Some("business_analyst"),
        "coder" | "cleaner" | "refactorer" | "test_author" => Some("worker"),
        "reviewer_test_gate" | "reviewer_implementation_gate" | "reviewer_validator" => {
            Some("coach")
        }
        "adversarial_reviewer" => Some("coach"),
        "cleaner_review_gate" | "qa_tester" | "hardender" => Some("verifier"),
        "architect" => Some("solution_architect"),
        "prover" | "release_closure" => Some("prover"),
        _ => None,
    }
}

pub(crate) fn default_task_class_for_canonical_role(role_id: &str) -> Option<&'static str> {
    match canonical_dev_team_role_id(role_id).as_str() {
        "specifier" => Some("specification"),
        "coder" | "cleaner" => Some("implementation"),
        "refactorer" => Some("refactor"),
        "architect" => Some("architecture"),
        "hardender" | "cleaner_review_gate" => Some("quality_gate"),
        "reviewer_test_gate" | "reviewer_implementation_gate" => Some("review"),
        "adversarial_reviewer" => Some("review"),
        "reviewer_validator" => Some("validation"),
        "qa_tester" => Some("verification"),
        "prover" | "release_closure" => Some("release_readiness"),
        "test_author" => Some("test_authoring"),
        _ => None,
    }
}

pub(crate) fn default_worktree_policy_for_step(
    role_id: &str,
    task_class: &str,
    receive_mode: &str,
) -> &'static str {
    let role_id = canonical_dev_team_role_id(role_id);
    if role_id == "hardender" && receive_mode == "batch" {
        return "isolated_per_lane";
    }
    match role_id.as_str() {
        "coder" | "cleaner" | "refactorer" | "test_author" => "isolated_per_task",
        "hardender" if matches!(task_class, "implementation" | "refactor") => "isolated_per_task",
        "specifier"
        | "architect"
        | "reviewer_test_gate"
        | "reviewer_implementation_gate"
        | "reviewer_validator"
        | "adversarial_reviewer"
        | "cleaner_review_gate"
        | "qa_tester"
        | "prover"
        | "release_closure"
        | "hardender" => "current",
        _ => "current",
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
    let configured_roles = configured_dev_team_canonical_roles(overlay);
    let mut seen_pack_ids = BTreeSet::new();
    let packs = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let pack = compile_pack_row(
                row,
                index,
                &configured_path,
                &configured_roles,
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
    pack_id.trim().to_ascii_lowercase().replace('_', "-")
}

pub(crate) fn pack_id_matches(pack: &serde_json::Value, requested_pack_id: &str) -> bool {
    let requested = normalized_pack_lookup_key(requested_pack_id);
    if requested.is_empty() {
        return false;
    }
    pack["pack_id"]
        .as_str()
        .is_some_and(|pack_id| normalized_pack_lookup_key(pack_id) == requested)
        || pack["aliases"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .any(|alias| normalized_pack_lookup_key(alias) == requested)
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
    configured_roles: &BTreeSet<String>,
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
    }
    let aliases = pack_aliases(row, &pack_id);
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
        configured_roles,
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
    if pack_id.contains('-') {
        aliases.push(pack_id.replace('-', "_"));
    }
    if pack_id.contains('_') {
        aliases.push(pack_id.replace('_', "-"));
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn compile_pack_steps(
    pack_id: &str,
    pack: &serde_yaml::Value,
    pack_terminal_proof_target: Option<&str>,
    configured_roles: &BTreeSet<String>,
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
            let canonical_role_id = canonical_dev_team_role_id(&configured_role_id);
            if !known_pack_role(&canonical_role_id)
                && !configured_roles.contains(&canonical_role_id)
            {
                blocker_codes.push(format!(
                    "unknown_pack_step_role:{pack_id}:{configured_role_id}"
                ));
            }
            let runtime_role = crate::yaml_string(crate::yaml_lookup(step, &["runtime_role"]))
                .or_else(|| {
                    default_runtime_role_for_canonical_role(&canonical_role_id).map(str::to_string)
                })
                .unwrap_or_default();
            if runtime_role.is_empty() {
                blocker_codes.push(format!("missing_pack_step_runtime_role:{pack_id}:{index}"));
            }
            let task_class = crate::yaml_string(crate::yaml_lookup(step, &["task_class"]))
                .or_else(|| {
                    default_task_class_for_canonical_role(&canonical_role_id).map(str::to_string)
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
                "requires_user_approval": crate::yaml_bool(
                    crate::yaml_lookup(step, &["requires_user_approval"]),
                    false,
                ),
                "approval_policy": yaml_field_json(step, "approval_policy"),
                "proof_gates": yaml_field_json(step, "proof_gates"),
                "adapter_projection": yaml_field_json(step, "adapter_projection"),
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

fn configured_dev_team_canonical_roles(overlay: &serde_yaml::Value) -> BTreeSet<String> {
    crate::yaml_lookup(overlay, &["dev_team", "roles"])
        .and_then(serde_yaml::Value::as_mapping)
        .into_iter()
        .flat_map(|mapping| mapping.keys())
        .filter_map(serde_yaml::Value::as_str)
        .map(canonical_dev_team_role_id)
        .collect()
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
    fn role_aliases_normalize_to_new_canonical_ids() {
        assert_eq!(canonical_dev_team_role_id("analyst"), "specifier");
        assert_eq!(canonical_dev_team_role_id("developer"), "coder");
        assert_eq!(
            canonical_dev_team_role_id("coach_implementation_gate"),
            "reviewer_implementation_gate"
        );
        assert_eq!(canonical_dev_team_role_id("tester"), "qa_tester");
        assert_eq!(canonical_dev_team_role_id("qa"), "qa_tester");
        assert_eq!(canonical_dev_team_role_id("hardener"), "hardender");
        assert_eq!(
            canonical_dev_team_role_id("reviewer"),
            "adversarial_reviewer"
        );
        assert_eq!(canonical_dev_team_role_id("prover"), "prover");
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
            .any(|code| code.contains("missing_pack_step_terminal_proof_target:bad_pack")));
    }

    #[test]
    fn pack_steps_get_default_worktree_policy() {
        assert_eq!(
            default_worktree_policy_for_step("coder", "implementation", "single"),
            "isolated_per_task"
        );
        assert_eq!(
            default_worktree_policy_for_step("qa_tester", "verification", "task"),
            "current"
        );
        assert_eq!(
            default_worktree_policy_for_step("hardender", "quality_gate", "batch"),
            "isolated_per_lane"
        );
    }

    #[test]
    fn pack_lookup_accepts_hyphen_and_underscore_aliases() {
        let readiness = serde_json::json!({
            "packs": [
                {
                    "pack_id": "quick-two-pack",
                    "aliases": ["quick_two_pack"],
                    "flow_id": "quick_two_pack_flow"
                }
            ]
        });

        assert!(pack_by_id(&readiness, "quick-two-pack").is_some());
        assert!(pack_by_id(&readiness, "quick_two_pack").is_some());
    }

    #[test]
    fn receive_mode_single_is_rendered_as_task() {
        assert_eq!(canonical_receive_mode("single"), "task");
        assert_eq!(canonical_receive_mode("task"), "task");
        assert_eq!(canonical_receive_mode("batch"), "batch");
    }
}
