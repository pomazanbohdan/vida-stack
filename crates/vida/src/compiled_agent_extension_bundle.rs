use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn canonical_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(values) => BTreeMap::from_iter(
            values
                .iter()
                .map(|(key, value)| (key.clone(), canonical_json(value))),
        )
        .into_iter()
        .collect(),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(canonical_json).collect())
        }
        value => value.clone(),
    }
}

fn canonical_registry_json(
    registry: &serde_yaml::Value,
    collection_key: &str,
    id_key: &str,
) -> serde_json::Value {
    let mut value =
        canonical_json(&serde_json::to_value(registry).unwrap_or(serde_json::Value::Null));
    if let Some(rows) = value
        .get_mut(collection_key)
        .and_then(serde_json::Value::as_array_mut)
    {
        rows.sort_by(|left, right| {
            left.get(id_key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .cmp(
                    right
                        .get(id_key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default(),
                )
        });
    }
    value
}

fn deterministic_content_id(namespace: &str, value: &serde_json::Value) -> serde_json::Value {
    let canonical = canonical_json(value);
    let encoded = serde_json::to_vec(&canonical).unwrap_or_default();
    let digest = blake3::hash(&encoded).to_hex().to_string();
    serde_json::json!({
        "id": format!("{namespace}:{digest}"),
        "content_blake3": digest,
    })
}

fn registry_catalog(
    registry: &serde_yaml::Value,
    collection_key: &str,
    id_key: &str,
) -> serde_json::Map<String, serde_json::Value> {
    crate::registry_rows_by_key(registry, collection_key, id_key, &[])
        .into_iter()
        .filter_map(|row| {
            let id = row
                .get(id_key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)?;
            Some((id, row))
        })
        .collect()
}

fn active_policy_pin_from_store(root: &Path) -> serde_json::Value {
    let store_path = root
        .join(crate::state_store::default_state_dir())
        .join("policy-store.json");
    let Ok(raw) = fs::read_to_string(store_path) else {
        return serde_json::Value::Null;
    };
    let Ok(snapshot) =
        serde_json::from_str::<crate::state_store::policy::PolicyLifecycleStoreSnapshot>(&raw)
    else {
        return serde_json::Value::Null;
    };
    let Some(active_pointer) = snapshot.active_pointer.as_deref() else {
        return serde_json::Value::Null;
    };
    let Some(bundle) = snapshot
        .bundles
        .iter()
        .find(|bundle| bundle.bundle_id == active_pointer)
    else {
        return serde_json::Value::Null;
    };
    if !matches!(
        bundle.lifecycle,
        crate::state_store::policy::PolicyLifecycle::Active
    ) {
        return serde_json::Value::Null;
    }
    serde_json::json!({
        "policy_id": bundle.policy_id,
        "version": bundle.version,
        "content_digest": bundle.content_digest,
    })
}

pub(crate) fn build_compiled_agent_extension_bundle_for_root(
    config: &serde_yaml::Value,
    root: &Path,
) -> Result<serde_json::Value, String> {
    let registry_projection = crate::build_agent_extension_registry_projection(config, root);
    let mut validation_errors = registry_projection.validation_errors;
    let roles_registry = registry_projection.roles_registry;
    let skills_registry = registry_projection.skills_registry;
    let profiles_registry = registry_projection.profiles_registry;
    let flows_registry = registry_projection.flows_registry;
    let packs_registry = registry_projection.packs_registry;
    let commands_registry = registry_projection.commands_registry;
    let dispatch_aliases_registry = registry_projection.dispatch_aliases_registry;
    let enabled_project_roles = registry_projection.enabled_project_roles;
    let enabled_project_skills = registry_projection.enabled_project_skills;
    let enabled_project_profiles = registry_projection.enabled_project_profiles;
    let enabled_project_flows = registry_projection.enabled_project_flows;
    let selected_host_cli_system = registry_projection.selected_host_cli_system;
    let host_cli_system_registry = registry_projection.host_cli_system_registry;
    let dispatch_aliases_path = registry_projection.dispatch_aliases_path;
    let require_profile_resolution = registry_projection.validation.require_profile_resolution;
    let require_flow_resolution = registry_projection.validation.require_flow_resolution;
    let hook_template_projection = crate::build_hook_template_registry_projection(
        config,
        root,
        registry_projection.validation.require_registry_files,
    );
    validation_errors.extend(hook_template_projection.validation_errors);
    let hook_templates = crate::registry_rows_by_key(
        &hook_template_projection.hook_templates_registry,
        "hook_templates",
        "template_id",
        &hook_template_projection.enabled_hook_templates,
    );
    let require_framework_role_compatibility = registry_projection
        .validation
        .require_framework_role_compatibility;
    let require_skill_role_compatibility = registry_projection
        .validation
        .require_skill_role_compatibility;
    let carrier_runtime_projection = crate::build_carrier_runtime_projection(
        config,
        root,
        selected_host_cli_system.as_deref(),
        &host_cli_system_registry,
        &dispatch_aliases_registry,
        dispatch_aliases_path.as_deref(),
    );
    validation_errors.extend(carrier_runtime_projection.validation_errors);
    let catalog_projection = crate::build_agent_extension_catalog_projection(
        &roles_registry,
        &skills_registry,
        &profiles_registry,
        &flows_registry,
        &enabled_project_roles,
        &enabled_project_skills,
        &enabled_project_profiles,
        &enabled_project_flows,
    );
    let project_roles = catalog_projection.project_roles;
    let project_skills = catalog_projection.project_skills;
    let project_profiles = catalog_projection.project_profiles;
    let project_flows = catalog_projection.project_flows;
    let project_role_map = catalog_projection.project_role_map;
    let project_skill_map = catalog_projection.project_skill_map;
    let project_profile_map = catalog_projection.project_profile_map;
    let project_flow_map = catalog_projection.project_flow_map;
    let all_project_flow_map = catalog_projection.all_project_flow_map;
    let enabled_framework_roles = crate::yaml_string_list(crate::yaml_lookup(
        config,
        &["agent_extensions", "enabled_framework_roles"],
    ));
    let pack_catalog = registry_catalog(&packs_registry, "packs", "pack_id");
    let command_catalog = registry_catalog(&commands_registry, "commands", "command_id");
    let canonical_registries = serde_json::json!({
        "roles": canonical_registry_json(&roles_registry, "roles", "role_id"),
        "skills": canonical_registry_json(&skills_registry, "skills", "skill_id"),
        "profiles": canonical_registry_json(&profiles_registry, "profiles", "profile_id"),
        "flows": canonical_registry_json(&flows_registry, "flow_sets", "flow_id"),
        "packs": canonical_registry_json(&packs_registry, "packs", "pack_id"),
        "commands": canonical_registry_json(&commands_registry, "commands", "command_id"),
        "dispatch_aliases": canonical_registry_json(
            &dispatch_aliases_registry,
            "dispatch_aliases",
            "alias_id",
        ),
    });
    let dev_team_value = crate::yaml_lookup(config, &["dev_team"])
        .cloned()
        .unwrap_or(serde_yaml::Value::Null);
    let dev_team_json = serde_json::to_value(dev_team_value).unwrap_or(serde_json::Value::Null);
    let taskflow_dispatch_enabled = crate::taskflow_runtime::taskflow_dispatch_enabled(config);
    let agent_system_value = serde_json::to_value(
        crate::yaml_lookup(config, &["agent_system"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let team_flow_enabled = dev_team_json
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let team_flow_authority = if !team_flow_enabled {
        serde_json::json!({
            "status": "disabled",
            "enabled": false,
            "reason": "dev_team_disabled",
            "selected_config": {"team_flow_enabled": false},
        })
    } else {
        crate::team_flow_authority_projection::materialize_team_flow_authority(
            crate::team_flow_authority_projection::SourceInputs {
                dev_team: dev_team_json,
                registries: canonical_registries,
                carrier_runtime: carrier_runtime_projection.carrier_runtime.clone(),
                agent_system: agent_system_value.clone(),
                catalog: serde_json::json!({
                    "project_roles": &project_roles,
                    "project_skills": &project_skills,
                    "project_profiles": &project_profiles,
                    "project_flows": &project_flows,
                    "project_role_catalog": &project_role_map,
                    "project_skill_catalog": &project_skill_map,
                    "project_profile_catalog": &project_profile_map,
                    "authority_catalog": serde_json::to_value(
                        crate::yaml_lookup(config, &["dev_team", "authority_catalog"])
                            .cloned()
                            .unwrap_or(serde_yaml::Value::Null),
                    )
                    .unwrap_or(serde_json::Value::Null),
                }),
            },
        )
        .map_err(|blocker| {
            format!(
                "team_flow_authority_materialization_blocked:{}:{}",
                blocker.code, blocker.path
            )
        })?
        .authority
    };
    let active_policy_pin = active_policy_pin_from_store(root);

    let bundle = serde_json::json!({
        "ok": true,
        "enabled": crate::yaml_bool(crate::yaml_lookup(config, &["agent_extensions", "enabled"]), false),
        "map_doc": crate::yaml_string(crate::yaml_lookup(config, &["agent_extensions", "map_doc"])).unwrap_or_default(),
        "enabled_framework_roles": enabled_framework_roles,
        "enabled_standard_flow_sets": crate::yaml_string_list(crate::yaml_lookup(config, &["agent_extensions", "enabled_standard_flow_sets"])),
        "enabled_shared_skills": crate::yaml_string_list(crate::yaml_lookup(config, &["agent_extensions", "enabled_shared_skills"])),
        "default_flow_set": crate::yaml_string(crate::yaml_lookup(config, &["agent_extensions", "default_flow_set"])).unwrap_or_default(),
        "runtime_projection_root": crate::project_activator_surface::runtime_agent_extensions_root(root).display().to_string(),
        "project_roles": project_roles,
        "project_skills": project_skills,
        "project_profiles": project_profiles,
        "project_flows": project_flows,
        "project_role_catalog": project_role_map,
        "project_profile_catalog": project_profile_map,
        "project_flow_catalog": project_flow_map,
        "all_project_flow_catalog": all_project_flow_map,
        "pack_catalog": pack_catalog,
        "command_catalog": command_catalog,
        "team_flow_authority": team_flow_authority,
        "policy_runtime": {
            "active": active_policy_pin,
        },
        "taskflow": {
            "management_runtime": "always_on",
            "management_status": crate::taskflow_runtime::management_status_projection(),
            "dispatch_status": crate::taskflow_runtime::dispatch_status_projection(
                &root.join(crate::state_store::default_state_dir()),
            ),
            "dispatch": {
                "enabled": taskflow_dispatch_enabled,
                "runtime": "task_dispatch",
            },
        },
        "hook_templates": hook_templates,
        "hook_template_registry": {
            "configured_path": hook_template_projection.hook_templates_path,
            "enabled_template_ids": hook_template_projection.enabled_hook_templates,
            "source_of_truth": hook_template_projection.hook_templates_path
                .as_ref()
                .map(|path| format!("vida.config.yaml -> agent_extensions.registries.hook_templates ({path})"))
                .unwrap_or_else(|| "vida.config.yaml -> agent_extensions.registries.hook_templates".to_string()),
        },
        "lane_work_context_contract": {
            "schema_version": "lane-work-context.v1",
            "required_fields": [
                "task",
                "role_profile",
                "objective",
                "owned_paths",
                "read_paths",
                "proof_targets",
                "constraints",
                "source_refs",
                "result_schema",
                "artifact_refs",
                "context_budget"
            ],
            "artifact_ref_fields": ["hash", "summary", "load_when"],
            "omitted_sections": [
                "carrier_selection",
                "pricing",
                "unrelated_boot_diagnostics",
                "full_runtime_bundle",
                "conversation_transcript"
            ],
            "source_of_truth": "compiled_agent_extension_bundle"
        },
        "agent_system": agent_system_value,
        "autonomous_execution": serde_json::to_value(crate::yaml_lookup(config, &["autonomous_execution"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "carrier_runtime": carrier_runtime_projection.carrier_runtime,
        "role_selection": serde_json::to_value(crate::yaml_lookup(config, &["agent_extensions", "role_selection"]).cloned().unwrap_or(serde_yaml::Value::Null))
            .unwrap_or(serde_json::Value::Null),
    });

    let role_ids = crate::registry_ids_by_key(&roles_registry, "roles", "role_id");
    let skill_ids = crate::registry_ids_by_key(&skills_registry, "skills", "skill_id");
    let profile_ids = crate::registry_ids_by_key(&profiles_registry, "profiles", "profile_id");
    let flow_ids = crate::registry_ids_by_key(&flows_registry, "flow_sets", "flow_id");

    crate::extend_agent_extension_bundle_validation_errors(
        &mut validation_errors,
        crate::AgentExtensionBundleValidationInput {
            require_profile_resolution,
            require_flow_resolution,
            require_framework_role_compatibility,
            require_skill_role_compatibility,
            enabled_framework_roles: &enabled_framework_roles,
            project_roles: &project_roles,
            project_skills: &project_skills,
            project_profiles: &project_profiles,
            project_flows: &project_flows,
            project_role_map: &project_role_map,
            project_skill_map: &project_skill_map,
            enabled_project_roles: &enabled_project_roles,
            enabled_project_skills: &enabled_project_skills,
            enabled_project_profiles: &enabled_project_profiles,
            enabled_project_flows: &enabled_project_flows,
            role_ids: &role_ids,
            skill_ids: &skill_ids,
            profile_ids: &profile_ids,
            flow_ids: &flow_ids,
        },
    );

    if !validation_errors.is_empty() {
        return Err(format!(
            "Agent extension bundle validation failed: {}",
            validation_errors.join("; ")
        ));
    }

    if team_flow_enabled {
        crate::team_flow_authority_adapter::compile_team_flow_authority(&bundle, None, None)
            .map_err(|error| {
                format!("team_flow_authority_persisted_self_validation_failed:{error}")
            })?;
    }

    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::{
        active_policy_pin_from_store, build_compiled_agent_extension_bundle_for_root,
        canonical_registry_json, deterministic_content_id,
    };
    use crate::project_activator_surface::read_yaml_file_checked;
    use crate::run;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir};
    use std::collections::BTreeSet;
    use std::fs;
    use std::process::ExitCode;

    #[test]
    fn active_policy_pin_projection_is_store_backed_and_fail_closed() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        assert_eq!(active_policy_pin_from_store(root), serde_json::Value::Null);

        let state_dir = root.join(crate::state_store::default_state_dir());
        fs::create_dir_all(&state_dir).expect("state dir should exist");
        fs::write(
            state_dir.join("policy-store.json"),
            serde_json::json!({
                "schema_version": 1,
                "bundles": [{
                    "bundle_id": "rhai.runtime.authority@1",
                    "policy_id": "rhai.runtime.authority",
                    "version": 1,
                    "engine_abi": "rhai-policy-engine-v1",
                    "source": "ctx.value",
                    "content_digest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                    "lifecycle": "active"
                }],
                "test_receipts": [],
                "evaluation_receipts": [],
                "shadow_diffs": [],
                "active_pointer": "rhai.runtime.authority@1",
                "last_known_good": null,
                "modes": [],
                "run_pins": [],
                "shadow_receipts": []
            })
            .to_string(),
        )
        .expect("policy store should exist");

        assert_eq!(
            active_policy_pin_from_store(root),
            serde_json::json!({
                "policy_id": "rhai.runtime.authority",
                "version": 1,
                "content_digest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            })
        );
    }

    #[test]
    fn compiled_agent_extension_bundle_merges_sidecar_overrides() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: .vida/project/agent-extensions/roles.yaml\n",
                "    skills: .vida/project/agent-extensions/skills.yaml\n",
                "    profiles: .vida/project/agent-extensions/profiles.yaml\n",
                "    flows: .vida/project/agent-extensions/flows.yaml\n",
                "    dispatch_aliases: .vida/project/agent-extensions/dispatch-aliases.yaml\n",
                "  enabled_framework_roles:\n",
                "    - orchestrator\n",
                "    - worker\n",
                "  enabled_standard_flow_sets:\n",
                "    - minimal\n",
                "  enabled_project_roles:\n",
                "    - party_chat_facilitator\n",
                "  enabled_project_skills: []\n",
                "  enabled_project_profiles: []\n",
                "  enabled_project_flows: []\n",
                "  enabled_shared_skills: []\n",
                "  default_flow_set: minimal\n",
                "  validation:\n",
                "    require_registry_files: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: party_chat_facilitator\n",
                "    base_role: business_analyst\n",
                "    description: base\n",
            ),
        )
        .expect("base roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: party_chat_facilitator\n",
                "    base_role: business_analyst\n",
                "    description: overridden\n",
            ),
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect("bundle should compile");
        assert_eq!(bundle["project_roles"][0]["description"], "overridden");
        assert_eq!(
            bundle["lane_work_context_contract"]["schema_version"],
            "lane-work-context.v1"
        );
        assert!(bundle["lane_work_context_contract"]["omitted_sections"]
            .as_array()
            .is_some_and(|sections| sections.iter().any(|value| value == "pricing")));
    }

    #[test]
    fn compiled_agent_extension_bundle_uses_registry_rows_when_enabled_lists_are_omitted() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: .vida/project/agent-extensions/roles.yaml\n",
                "    skills: .vida/project/agent-extensions/skills.yaml\n",
                "    profiles: .vida/project/agent-extensions/profiles.yaml\n",
                "    flows: .vida/project/agent-extensions/flows.yaml\n",
                "    dispatch_aliases: .vida/project/agent-extensions/dispatch-aliases.yaml\n",
                "  enabled_framework_roles:\n",
                "    - orchestrator\n",
                "    - business_analyst\n",
                "    - coach\n",
                "    - verifier\n",
                "  validation:\n",
                "    require_registry_files: true\n",
                "    require_framework_role_compatibility: true\n",
                "    require_profile_resolution: true\n",
                "    require_flow_resolution: true\n",
                "    require_skill_role_compatibility: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: role_a\n",
                "    base_role: business_analyst\n",
                "    description: role a\n",
            ),
        )
        .expect("roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            concat!(
                "version: 1\n",
                "skills:\n",
                "  - skill_id: skill_a\n",
                "    description: skill a\n",
                "    compatible_base_roles: business_analyst\n",
            ),
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            concat!(
                "version: 1\n",
                "profiles:\n",
                "  - profile_id: profile_a\n",
                "    role_ref: role_a\n",
                "    skill_refs: skill_a\n",
            ),
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            concat!(
                "version: 1\n",
                "flow_sets:\n",
                "  - flow_id: flow_a\n",
                "    role_chain: role_a\n",
            ),
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases sidecar should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect("bundle should compile from registries");
        assert_eq!(bundle["project_roles"][0]["role_id"], "role_a");
        assert_eq!(bundle["project_profiles"][0]["profile_id"], "profile_a");
        assert_eq!(bundle["project_flows"][0]["flow_id"], "flow_a");
    }

    #[test]
    fn compiled_agent_extension_bundle_fails_closed_on_invalid_profile_skill_and_flow_links() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join(".vida/project/agent-extensions"))
            .expect("runtime agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: .vida/project/agent-extensions/roles.yaml\n",
                "    skills: .vida/project/agent-extensions/skills.yaml\n",
                "    profiles: .vida/project/agent-extensions/profiles.yaml\n",
                "    flows: .vida/project/agent-extensions/flows.yaml\n",
                "  enabled_framework_roles:\n",
                "    - business_analyst\n",
                "    - verifier\n",
                "  validation:\n",
                "    require_registry_files: true\n",
                "    require_framework_role_compatibility: true\n",
                "    require_profile_resolution: true\n",
                "    require_flow_resolution: true\n",
                "    require_skill_role_compatibility: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.yaml"),
            concat!(
                "version: 1\n",
                "roles:\n",
                "  - role_id: role_a\n",
                "    base_role: business_analyst\n",
                "    description: role a\n",
            ),
        )
        .expect("roles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.yaml"),
            concat!(
                "version: 1\n",
                "skills:\n",
                "  - skill_id: skill_a\n",
                "    description: skill a\n",
                "    compatible_base_roles: verifier\n",
            ),
        )
        .expect("skills registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.yaml"),
            concat!(
                "version: 1\n",
                "profiles:\n",
                "  - profile_id: profile_a\n",
                "    role_ref: role_a\n",
                "    skill_refs: skill_a,missing_skill\n",
            ),
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.yaml"),
            concat!(
                "version: 1\n",
                "flow_sets:\n",
                "  - flow_id: flow_a\n",
                "    role_chain: role_a,missing_role\n",
            ),
        )
        .expect("flows registry should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/roles.sidecar.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/skills.sidecar.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/profiles.sidecar.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles sidecar should exist");
        fs::write(
            root.join(".vida/project/agent-extensions/flows.sidecar.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows sidecar should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let error = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect_err("bundle should fail closed");
        assert!(error.contains("missing_skill"));
        assert!(error.contains("missing_role"));
        assert!(error.contains("incompatible skill `skill_a`"));
    }

    #[test]
    fn compiled_bundle_exposes_canonical_carrier_runtime_without_legacy_aliases() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let carrier_runtime = bundle["carrier_runtime"].clone();
        assert!(carrier_runtime.is_object());
        assert!(bundle.get("codex_multi_agent").is_none());
        assert!(carrier_runtime["dispatch_aliases"].is_array());
    }

    #[test]
    fn dispatch_aliases_require_canonical_overlay_key() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let config_path = harness.path().join("vida.config.yaml");
        let config_body =
            fs::read_to_string(&config_path).expect("config should be readable after init");
        let updated = config_body.replace("dispatch_aliases:", "named_lanes:");
        fs::write(&config_path, updated).expect("config should be rewritten");

        let config =
            read_yaml_file_checked(&harness.path().join("vida.config.yaml")).expect("config");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, harness.path())
            .expect("bundle should compile");
        let carrier_runtime = bundle["carrier_runtime"].clone();
        let dispatch_aliases = carrier_runtime["dispatch_aliases"]
            .as_array()
            .expect("dispatch aliases should still be an array");

        assert!(dispatch_aliases.is_empty());
    }

    #[test]
    fn compiled_bundle_persists_resolved_team_flow_authority_without_raw_flow_reload() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let config = read_yaml_file_checked(&repository_root.join("vida.config.yaml"))
            .expect("repository config should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&config, repository_root)
            .expect("repository bundle should compile");
        let authority = &bundle["team_flow_authority"];
        let config_json = serde_json::to_value(&config).expect("config should convert to JSON");
        let selection = config_json["dev_team"]["authority_selection"]
            .as_object()
            .expect("authority selection should be an object");
        let expected_config_id = selection["config_id"]
            .as_str()
            .expect("authority selection config id should be concrete");
        let expected_default_flow_id = selection["default_flow_id"]
            .as_str()
            .expect("authority selection default flow id should be concrete");
        let expected_flow_ids = config_json["dev_team"]["flows"]
            .as_object()
            .expect("configured flows should be an object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(authority["schema_version"], "team-flow-authority.v1");
        assert!(authority["authority_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("team-flow-authority:")));
        assert!(authority["config"]["content_blake3"].as_str().is_some());
        assert_eq!(
            authority["selected_config"]["config_id"],
            expected_config_id
        );
        assert_eq!(
            authority["selected_config"]["team_flow_enabled"],
            config_json["dev_team"]["enabled"]
        );
        assert_eq!(
            authority["selected_config"]["authority_selection"]["default_flow_id"],
            expected_default_flow_id
        );
        assert!(authority["selected_config"].get("flows").is_none());
        assert!(authority["selected_config"]
            .get("default_flow_id")
            .is_none());

        let payload = &authority["resolved_all_flow_payload"];
        assert_eq!(
            payload["work_item_flow_bindings"],
            config_json["dev_team"]["work_item_flow_bindings"]
        );
        let defect_eligible_flow_count = config_json["dev_team"]["flows"]
            .as_object()
            .expect("configured flows")
            .values()
            .filter(|flow| {
                flow["work_item_bindings"]
                    .as_array()
                    .is_some_and(|bindings| bindings.iter().any(|binding| binding == "defect"))
            })
            .count();
        assert!(defect_eligible_flow_count > 1);
        let flows = payload["flows"]
            .as_array()
            .expect("resolved flow payload should contain flows");
        let payload_flow_ids = flows
            .iter()
            .map(|flow| {
                flow["flow_id"]
                    .as_str()
                    .expect("resolved flow id should be concrete")
                    .to_string()
            })
            .collect::<BTreeSet<_>>();
        let payload_lane_count = flows
            .iter()
            .map(|flow| {
                flow["lanes"]
                    .as_array()
                    .expect("resolved flow lanes should be an array")
                    .len()
            })
            .sum::<usize>();
        assert_eq!(payload_flow_ids, expected_flow_ids);
        assert_eq!(payload["flow_count"].as_u64(), Some(flows.len() as u64));
        assert_eq!(
            payload["lane_count"].as_u64(),
            Some(payload_lane_count as u64)
        );
        let default_flow = flows
            .iter()
            .find(|flow| flow["flow_id"].as_str() == Some(expected_default_flow_id))
            .expect("configured default flow should be materialized");
        assert_eq!(authority["lanes"], default_flow["lanes"]);
        let flow_policy_keys = [
            "enabled",
            "default",
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
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        for flow in flows {
            let policy = flow["flow_policy"]
                .as_object()
                .expect("persisted flow policy should be an object");
            assert_eq!(
                policy.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                flow_policy_keys
            );
        }

        let payload_hash = deterministic_content_id("resolved-all-flow-payload", payload);
        assert_eq!(
            authority["resolved_all_flow_payload_blake3"],
            payload_hash["content_blake3"]
        );
        assert_eq!(
            authority["authority_source"]["payload_blake3"],
            payload_hash["content_blake3"]
        );

        let config_identity_id = authority["config"]["id"]
            .as_str()
            .filter(|id| !id.trim().is_empty())
            .expect("authority config identity id should be concrete");
        let registry_identities = authority["registries"]
            .as_object()
            .expect("authority registry identities should be an object");
        assert_eq!(
            registry_identities.len(),
            crate::team_flow_authority_projection::REGISTRY_NAMES.len()
        );
        for flow in flows {
            for lane in flow["lanes"]
                .as_array()
                .expect("resolved flow lanes should be an array")
            {
                let identities = lane["authority_identities"]
                    .as_array()
                    .expect("persisted lane authority identities should be an array");
                assert_eq!(
                    identities.len(),
                    crate::team_flow_authority_projection::REGISTRY_NAMES.len() + 1
                );
                let config_identity = identities[0]
                    .as_object()
                    .expect("config authority identity should be an object");
                assert_eq!(config_identity.len(), 3);
                assert_eq!(config_identity["kind"], "config");
                assert_eq!(config_identity["id"], config_identity_id);
                assert_eq!(config_identity["source_path"], "team_flow_authority.config");
                for (index, name) in crate::team_flow_authority_projection::REGISTRY_NAMES
                    .iter()
                    .enumerate()
                {
                    let identity = identities[index + 1]
                        .as_object()
                        .expect("registry authority identity should be an object");
                    let header_identity = registry_identities
                        .get(*name)
                        .and_then(serde_json::Value::as_object)
                        .expect("authority header registry identity should be an object");
                    let source_path = format!("team_flow_authority.registries.{name}");
                    assert_eq!(identity.len(), 3);
                    assert_eq!(identity["kind"], format!("registry:{name}"));
                    assert_eq!(identity["id"], header_identity["id"]);
                    assert_eq!(identity["source_path"], source_path);
                    assert!(identity["id"]
                        .as_str()
                        .is_some_and(|id| !id.trim().is_empty()));
                    assert!(identity["source_path"]
                        .as_str()
                        .is_some_and(|path| !path.trim().is_empty()));
                }
            }
        }
        assert_eq!(
            authority["authority_source"],
            serde_json::json!({
                "kind": "resolved_all_flow_payload",
                "payload_path": "team_flow_authority.resolved_all_flow_payload",
                "payload_blake3": authority["resolved_all_flow_payload_blake3"],
                "identity_phase": "phase_2_persisted_payload",
            })
        );
        assert!(authority.get("dev_team").is_none());
    }

    #[test]
    fn compiled_bundle_derives_only_unique_work_item_flow_bindings() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let config = read_yaml_file_checked(&repository_root.join("vida.config.yaml"))
            .expect("repository config should parse");
        let key = |name: &str| serde_yaml::Value::String(name.to_string());

        let mut unique = config.clone();
        let unique_dev_team = unique
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("dev_team should be a mapping");
        unique_dev_team.remove(key("work_item_flow_bindings"));
        let flows = unique_dev_team
            .get_mut(key("flows"))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("flows should be a mapping");
        let mut expected = serde_json::Map::new();
        for (index, (flow_id, flow)) in flows.iter_mut().enumerate() {
            let flow_id = flow_id.as_str().expect("flow id").to_string();
            let binding = format!("unique-{index}");
            flow.as_mapping_mut()
                .expect("flow should be a mapping")
                .insert(
                    key("work_item_bindings"),
                    serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(binding.clone())]),
                );
            expected.insert(binding, serde_json::Value::String(flow_id));
        }
        let unique_bundle =
            build_compiled_agent_extension_bundle_for_root(&unique, repository_root)
                .expect("unique eligibility should derive exact bindings");
        assert_eq!(
            unique_bundle["team_flow_authority"]["resolved_all_flow_payload"]
                ["work_item_flow_bindings"],
            serde_json::Value::Object(expected)
        );

        let mut ambiguous = config.clone();
        ambiguous
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("dev_team should be a mapping")
            .remove(key("work_item_flow_bindings"));
        let error = build_compiled_agent_extension_bundle_for_root(&ambiguous, repository_root)
            .expect_err("overlapping eligibility without explicit binding must fail");
        assert!(error.contains("team_flow_authority_work_item_flow_binding_ambiguous"));

        let mut unknown = config;
        unknown
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .and_then(|dev_team| dev_team.get_mut(key("work_item_flow_bindings")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("explicit bindings should be a mapping")
            .insert(
                key("defect"),
                serde_yaml::Value::String("missing-flow".to_string()),
            );
        let error = build_compiled_agent_extension_bundle_for_root(&unknown, repository_root)
            .expect_err("unknown explicit target must fail");
        assert!(error.contains("team_flow_authority_work_item_flow_binding_target_missing"));
    }

    #[test]
    fn compiled_bundle_normalizes_global_team_flow_enabled_policy() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let config = read_yaml_file_checked(&repository_root.join("vida.config.yaml"))
            .expect("repository config should parse");
        let key = |name: &str| serde_yaml::Value::String(name.to_string());

        let explicit_bundle =
            build_compiled_agent_extension_bundle_for_root(&config, repository_root)
                .expect("explicitly enabled bundle should compile");

        let mut missing = config.clone();
        missing
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("dev_team should be a mapping")
            .remove(key("enabled"));
        let defaulted_bundle =
            build_compiled_agent_extension_bundle_for_root(&missing, repository_root)
                .expect("missing TeamFlow policy should use the schema default");
        assert_eq!(
            defaulted_bundle["team_flow_authority"]["selected_config"]["team_flow_enabled"],
            true
        );
        assert_eq!(
            defaulted_bundle["team_flow_authority"]["authority_id"],
            explicit_bundle["team_flow_authority"]["authority_id"]
        );

        let mut disabled = config.clone();
        disabled
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("dev_team should be a mapping")
            .insert(key("enabled"), serde_yaml::Value::Bool(false));
        let disabled_bundle =
            build_compiled_agent_extension_bundle_for_root(&disabled, repository_root)
                .expect("disabled TeamFlow policy should remain materializable");
        assert_eq!(
            disabled_bundle["team_flow_authority"]["selected_config"]["team_flow_enabled"],
            false
        );
        assert_ne!(
            disabled_bundle["team_flow_authority"]["authority_id"],
            explicit_bundle["team_flow_authority"]["authority_id"]
        );

        let mut disabled_without_catalog = config.clone();
        disabled_without_catalog
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .map(|dev_team| {
                *dev_team = serde_yaml::from_str("enabled: false\n")
                    .expect("minimal disabled TeamFlow config")
            })
            .expect("dev_team should be present");
        let disabled_without_catalog_bundle = build_compiled_agent_extension_bundle_for_root(
            &disabled_without_catalog,
            repository_root,
        )
        .expect("disabled TeamFlow should not require the authority catalog");
        assert_eq!(
            disabled_without_catalog_bundle["team_flow_authority"]["status"],
            "disabled"
        );
        assert_eq!(
            disabled_without_catalog_bundle["team_flow_authority"]["reason"],
            "dev_team_disabled"
        );

        let mut invalid = config;
        invalid
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("dev_team should be a mapping")
            .insert(
                key("enabled"),
                serde_yaml::Value::String("enabled".to_string()),
            );
        let error = build_compiled_agent_extension_bundle_for_root(&invalid, repository_root)
            .expect_err("invalid TeamFlow policy must fail closed");
        assert!(error.contains("team_flow_authority_enabled_invalid"));
        assert!(error.contains("dev_team.enabled"));
    }

    #[test]
    fn compiled_bundle_propagates_typed_team_flow_materialization_blocker() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let mut config = read_yaml_file_checked(&repository_root.join("vida.config.yaml"))
            .expect("repository config should parse");
        let key = |name: &str| serde_yaml::Value::String(name.to_string());
        let dev_team = config
            .as_mapping_mut()
            .and_then(|root| root.get_mut(key("dev_team")))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("dev_team should be a mapping");
        let authority_selection = dev_team
            .get_mut(key("authority_selection"))
            .and_then(serde_yaml::Value::as_mapping_mut)
            .expect("authority selection should be a mapping");
        authority_selection.remove(key("config_id"));

        let error = build_compiled_agent_extension_bundle_for_root(&config, repository_root)
            .expect_err("bundle should fail closed when authority selection is incomplete");
        assert!(error.contains("team_flow_authority_materialization_blocked"));
        assert!(error.contains("team_flow_authority_required_field_missing"));
        assert!(error.contains("dev_team.authority_selection.config_id"));
    }

    #[test]
    fn compiled_agent_extension_bundle_projects_config_selected_hook_templates() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join("docs/process/agent-extensions"))
            .expect("agent extensions dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    roles: docs/process/agent-extensions/roles.yaml\n",
                "    skills: docs/process/agent-extensions/skills.yaml\n",
                "    profiles: docs/process/agent-extensions/profiles.yaml\n",
                "    flows: docs/process/agent-extensions/flows.yaml\n",
                "    dispatch_aliases: docs/process/agent-extensions/dispatch-aliases.yaml\n",
                "    hook_templates: docs/process/agent-extensions/hook-templates.yaml\n",
                "  enabled_framework_roles:\n",
                "    - orchestrator\n",
                "  enabled_hook_templates:\n",
                "    - operation_registry_fixture_closed\n",
                "  validation:\n",
                "    require_registry_files: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join("docs/process/agent-extensions/roles.yaml"),
            "version: 1\nroles: []\n",
        )
        .expect("roles registry should exist");
        fs::write(
            root.join("docs/process/agent-extensions/skills.yaml"),
            "version: 1\nskills: []\n",
        )
        .expect("skills registry should exist");
        fs::write(
            root.join("docs/process/agent-extensions/profiles.yaml"),
            "version: 1\nprofiles: []\n",
        )
        .expect("profiles registry should exist");
        fs::write(
            root.join("docs/process/agent-extensions/flows.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows registry should exist");
        fs::write(
            root.join("docs/process/agent-extensions/dispatch-aliases.yaml"),
            "version: 1\ndispatch_aliases: []\n",
        )
        .expect("dispatch aliases registry should exist");
        fs::write(
            root.join("docs/process/agent-extensions/hook-templates.yaml"),
            concat!(
                "version: 1\n",
                "hook_templates:\n",
                "  - template_id: operation_registry_fixture_closed\n",
                "    hook: operation_registry_golden_fixture_closure\n",
                "    template_path: docs/product/spec/operation-registry-golden-fixture-closure.md\n",
                "  - template_id: unrelated_template\n",
                "    hook: unrelated\n",
                "    template_path: docs/product/spec/unrelated.md\n",
            ),
        )
        .expect("hook templates registry should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect("bundle should compile");

        assert_eq!(bundle["hook_templates"].as_array().unwrap().len(), 1);
        assert_eq!(
            bundle["hook_templates"][0]["template_id"],
            "operation_registry_fixture_closed"
        );
        assert_eq!(
            bundle["hook_template_registry"]["configured_path"],
            "docs/process/agent-extensions/hook-templates.yaml"
        );
        assert_eq!(
            bundle["hook_template_registry"]["enabled_template_ids"],
            serde_json::json!(["operation_registry_fixture_closed"])
        );
    }

    #[test]
    fn lifecycle_hook_contract_projects_diagnostic_command_timing_template() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join("docs/product/spec")).expect("product spec dir should exist");
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    hook_templates: docs/product/spec/hook-templates.yaml\n",
                "  enabled_hook_templates:\n",
                "    - command_timing_summary\n",
                "  validation:\n",
                "    require_registry_files: true\n",
            ),
        )
        .expect("overlay should exist");
        fs::write(
            root.join("docs/product/spec/hook-templates.yaml"),
            concat!(
                "version: 1\n",
                "hook_templates:\n",
                "  - template_id: command_timing_summary\n",
                "    hook_class: command_lifecycle\n",
                "    diagnostic_only: true\n",
                "    fail_closed_on_hook_error: false\n",
                "    phases: [pre_execution, execution, post_execution]\n",
            ),
        )
        .expect("hook templates registry should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let bundle = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect("bundle should compile");

        assert_eq!(bundle["hook_templates"].as_array().unwrap().len(), 1);
        assert_eq!(
            bundle["hook_templates"][0]["template_id"],
            "command_timing_summary"
        );
        assert_eq!(
            bundle["hook_templates"][0]["hook_class"],
            "command_lifecycle"
        );
        assert_eq!(bundle["hook_templates"][0]["diagnostic_only"], true);
        assert_eq!(
            bundle["hook_templates"][0]["fail_closed_on_hook_error"],
            false
        );
    }

    #[test]
    fn compiled_agent_extension_bundle_fails_closed_when_enabled_hook_template_registry_missing() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::write(
            root.join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  enabled: true\n",
                "  registries:\n",
                "    hook_templates: docs/process/agent-extensions/missing-hook-templates.yaml\n",
                "  enabled_hook_templates:\n",
                "    - operation_registry_fixture_closed\n",
                "  validation:\n",
                "    require_registry_files: true\n",
            ),
        )
        .expect("overlay should exist");

        let overlay =
            read_yaml_file_checked(&root.join("vida.config.yaml")).expect("overlay should parse");
        let error = build_compiled_agent_extension_bundle_for_root(&overlay, root)
            .expect_err("bundle should fail closed");

        assert!(error.contains("missing-hook-templates.yaml"));
    }

    #[test]
    fn pack_and_command_registry_ids_are_stable_across_row_and_mapping_order() {
        let packs_left: serde_yaml::Value = serde_yaml::from_str(
            "version: 1\npacks:\n  - pack_id: second\n    steps: [b]\n  - pack_id: first\n    steps: [a]\n",
        )
        .expect("packs registry should parse");
        let packs_right: serde_yaml::Value = serde_yaml::from_str(
            "packs:\n  - steps: [a]\n    pack_id: first\n  - steps: [b]\n    pack_id: second\nversion: 1\n",
        )
        .expect("reordered packs registry should parse");
        let commands_left: serde_yaml::Value = serde_yaml::from_str(
            "version: 1\ncommands:\n  - command_id: beta\n    surface: vida agent-init\n  - command_id: alpha\n    surface: vida taskflow\n",
        )
        .expect("commands registry should parse");
        let commands_right: serde_yaml::Value = serde_yaml::from_str(
            "commands:\n  - surface: vida taskflow\n    command_id: alpha\n  - surface: vida agent-init\n    command_id: beta\nversion: 1\n",
        )
        .expect("reordered commands registry should parse");

        for (namespace, left, right) in [
            ("packs", packs_left, packs_right),
            ("commands", commands_left, commands_right),
        ] {
            let id_key = if namespace == "packs" {
                "pack_id"
            } else {
                "command_id"
            };
            let left = deterministic_content_id(
                namespace,
                &canonical_registry_json(&left, namespace, id_key),
            );
            let right = deterministic_content_id(
                namespace,
                &canonical_registry_json(&right, namespace, id_key),
            );
            assert_eq!(left, right, "{namespace} identity must be canonical");
        }
    }

    fn repository_root() -> &'static std::path::Path {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root")
    }

    fn read_yaml_as_json(relative_path: &str) -> serde_json::Value {
        let yaml: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(repository_root().join(relative_path))
                .unwrap_or_else(|_| panic!("{relative_path} should be readable")),
        )
        .unwrap_or_else(|_| panic!("{relative_path} should parse"));
        serde_json::to_value(yaml).expect("YAML should convert to JSON")
    }

    fn string_set(value: &serde_json::Value, context: &str) -> BTreeSet<String> {
        value
            .as_array()
            .unwrap_or_else(|| panic!("{context} must be an array"))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("{context} entries must be strings"))
                    .to_string()
            })
            .collect()
    }

    fn object_key_set(value: &serde_json::Value, context: &str) -> BTreeSet<String> {
        value
            .as_object()
            .unwrap_or_else(|| panic!("{context} must be an object"))
            .keys()
            .cloned()
            .collect()
    }

    fn assert_subset(actual: &BTreeSet<String>, declared: &BTreeSet<String>, context: &str) {
        let missing = actual.difference(declared).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "{context} missing declarations: {missing:?}"
        );
    }

    fn collect_object_keys_named(
        value: &serde_json::Value,
        key: &str,
        collected: &mut BTreeSet<String>,
    ) {
        match value {
            serde_json::Value::Object(values) => {
                if let Some(entries) = values.get(key).and_then(serde_json::Value::as_object) {
                    collected.extend(entries.keys().cloned());
                }
                for child in values.values() {
                    collect_object_keys_named(child, key, collected);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    collect_object_keys_named(child, key, collected);
                }
            }
            _ => {}
        }
    }

    fn collect_string_values_named(
        value: &serde_json::Value,
        keys: &[&str],
        collected: &mut BTreeSet<String>,
    ) {
        match value {
            serde_json::Value::Object(values) => {
                for (key, child) in values {
                    if keys.contains(&key.as_str()) {
                        if let Some(value) = child.as_str() {
                            collected.insert(value.to_string());
                        }
                    }
                    collect_string_values_named(child, keys, collected);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    collect_string_values_named(child, keys, collected);
                }
            }
            _ => {}
        }
    }

    fn registry_ids(relative_path: &str, collection_key: &str, id_key: &str) -> BTreeSet<String> {
        let registry: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(repository_root().join(relative_path))
                .unwrap_or_else(|_| panic!("{relative_path} should be readable")),
        )
        .unwrap_or_else(|_| panic!("{relative_path} should parse"));
        crate::registry_ids_by_key(&registry, collection_key, id_key)
            .into_iter()
            .collect()
    }

    #[test]
    fn project_team_flow_authority_selection_is_declared_by_template_and_schema() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let template: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(
                repository_root.join("docs/framework/templates/vida.config.yaml.template"),
            )
            .expect("master config template should be readable"),
        )
        .expect("master config template should parse");
        let install_template: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(repository_root.join("install/assets/vida.config.yaml.template"))
                .expect("install config template should be readable"),
        )
        .expect("install config template should parse");
        let project: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(repository_root.join("vida.config.yaml"))
                .expect("project config should be readable"),
        )
        .expect("project config should parse");
        let schema: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(
                repository_root.join("vida/config/schemas/team-flow-authority.schema.json"),
            )
            .expect("TeamFlow schema should be readable"),
        )
        .expect("TeamFlow schema should parse");

        let selection = crate::yaml_lookup(&project, &["dev_team", "authority_selection"])
            .and_then(serde_yaml::Value::as_mapping)
            .expect("project authority selection is required");
        let catalog = crate::yaml_lookup(&template, &["dev_team", "authority_catalog"])
            .and_then(serde_yaml::Value::as_mapping)
            .expect("master authority catalog is required");
        let option_pairs = [
            ("projection_mode", "projection_modes"),
            (
                "registry_identity_algorithm",
                "registry_identity_algorithms",
            ),
            ("terminal_source", "terminal_sources"),
            ("edge_source", "edge_sources"),
            ("command_resolution_mode", "command_resolution_modes"),
            ("approval_enforcement_mode", "approval_enforcement_modes"),
            ("alias_conflict_policy", "alias_conflict_policies"),
            ("node_field_source_mode", "node_field_source_modes"),
            (
                "dispatch_alias_resolution_mode",
                "dispatch_alias_resolution_modes",
            ),
            ("carrier_relation_mode", "carrier_relation_modes"),
            (
                "profile_model_resolution_mode",
                "profile_model_resolution_modes",
            ),
        ];
        for (selection_key, catalog_key) in option_pairs {
            let selected = selection
                .get(&serde_yaml::Value::String(selection_key.to_string()))
                .and_then(serde_yaml::Value::as_str)
                .expect("authority selection values must be typed strings");
            let declared = catalog
                .get(&serde_yaml::Value::String(catalog_key.to_string()))
                .and_then(serde_yaml::Value::as_sequence)
                .expect("authority catalog option lists are required");
            let install_catalog = crate::yaml_lookup(
                &install_template,
                &["dev_team", "authority_catalog", catalog_key],
            )
            .and_then(serde_yaml::Value::as_sequence)
            .expect("install authority catalog option lists are required");
            assert_eq!(
                declared, install_catalog,
                "docs and install authority catalogs must match for {catalog_key}"
            );
            assert!(
                declared
                    .iter()
                    .any(|value| value.as_str() == Some(selected)),
                "{selection_key}={selected} must be declared by {catalog_key}"
            );
            let schema_values = schema["$defs"]["authoritySelection"]["properties"][selection_key]
                ["enum"]
                .as_array()
                .expect("selection schema must enumerate each option");
            assert!(
                schema_values.iter().any(|value| value == selected),
                "{selection_key}={selected} must be accepted by the schema"
            );
            for schema_value in schema_values {
                let schema_value = schema_value
                    .as_str()
                    .expect("selection schema option values must be strings");
                assert!(
                    declared
                        .iter()
                        .any(|value| value.as_str() == Some(schema_value)),
                    "{selection_key} schema value {schema_value} must be declared by {catalog_key}"
                );
                assert!(
                    install_catalog
                        .iter()
                        .any(|value| value.as_str() == Some(schema_value)),
                    "{selection_key} schema value {schema_value} must be declared by install {catalog_key}"
                );
            }
            for declared_value in declared {
                let declared_value = declared_value
                    .as_str()
                    .expect("authority catalog option values must be strings");
                assert!(
                    schema_values.iter().any(|value| value == declared_value),
                    "{catalog_key} value {declared_value} must be accepted by the schema"
                );
            }
        }

        let required_lane_fields = catalog
            .get(&serde_yaml::Value::String(
                "required_lane_fields".to_string(),
            ))
            .and_then(serde_yaml::Value::as_sequence)
            .expect("master template must enumerate typed lane fields");
        let schema_required = schema["$defs"]["lane"]["required"]
            .as_array()
            .expect("lane schema must declare required fields");
        for field in required_lane_fields {
            let field = field.as_str().expect("lane field names must be strings");
            assert!(
                schema_required.iter().any(|value| value == field),
                "schema must require catalog lane field {field}"
            );
        }

        for required_option in [
            "when_proof_required",
            "when_review_triggered",
            "when_architecture_triggered",
            "when_rework_required",
        ] {
            assert!(catalog
                .get(&serde_yaml::Value::String("inclusion_rules".to_string()))
                .and_then(serde_yaml::Value::as_sequence)
                .is_some_and(|values| values
                    .iter()
                    .any(|value| value.as_str() == Some(required_option))));
        }
        assert!(schema["$defs"]["lane"]["allOf"]
            .as_array()
            .is_some_and(|conditions| conditions.len() >= 3));
    }

    #[test]
    fn project_authority_references_are_derived_from_master_catalogs() {
        let project = read_yaml_as_json("vida.config.yaml");
        let template = read_yaml_as_json("docs/framework/templates/vida.config.yaml.template");
        let schema: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(
                repository_root().join("vida/config/schemas/team-flow-authority.schema.json"),
            )
            .expect("TeamFlow schema should be readable"),
        )
        .expect("TeamFlow schema should parse");
        let catalog = &template["dev_team"]["authority_catalog"];
        let declared = |key: &str| string_set(&catalog[key], &format!("authority_catalog.{key}"));

        let project_roles = object_key_set(&project["dev_team"]["roles"], "project roles");
        let template_roles = object_key_set(&template["dev_team"]["roles"], "template roles");
        let declared_roles = declared("declared_team_role_ids");
        assert_eq!(declared_roles, template_roles);
        assert_subset(&project_roles, &template_roles, "project roles");
        let optional_roles = declared("optional_team_role_ids");
        assert_eq!(
            optional_roles,
            template_roles
                .difference(&project_roles)
                .cloned()
                .collect::<BTreeSet<_>>()
        );

        let project_flows = object_key_set(&project["dev_team"]["flows"], "project flows");
        let template_flows = object_key_set(&template["dev_team"]["flows"], "template flows");
        let declared_flows = declared("declared_team_flow_ids");
        assert_eq!(declared_flows, template_flows);
        assert_subset(&project_flows, &template_flows, "project flows");
        let optional_flows = catalog
            .get("optional_team_flow_ids")
            .map(|value| string_set(value, "optional team flows"))
            .unwrap_or_default();
        assert_eq!(
            optional_flows,
            template_flows
                .difference(&project_flows)
                .cloned()
                .collect::<BTreeSet<_>>()
        );

        let registry_contracts = [
            (
                "declared_agent_profile_ids",
                "docs/process/agent-extensions/profiles.yaml",
                "profiles",
                "profile_id",
            ),
            (
                "declared_agent_flow_ids",
                "docs/process/agent-extensions/flows.yaml",
                "flow_sets",
                "flow_id",
            ),
            (
                "declared_pack_ids",
                "docs/process/agent-extensions/packs.yaml",
                "packs",
                "pack_id",
            ),
            (
                "declared_command_ids",
                "docs/process/agent-extensions/commands.yaml",
                "commands",
                "command_id",
            ),
        ];
        for (catalog_key, path, collection_key, id_key) in registry_contracts {
            assert_eq!(
                declared(catalog_key),
                registry_ids(path, collection_key, id_key),
                "{catalog_key} must equal its canonical registry"
            );
        }
        for (enabled_key, declared_key) in [
            ("enabled_project_profiles", "declared_agent_profile_ids"),
            ("enabled_project_flows", "declared_agent_flow_ids"),
        ] {
            let enabled = string_set(
                &project["agent_extensions"][enabled_key],
                &format!("agent_extensions.{enabled_key}"),
            );
            assert_subset(&enabled, &declared(declared_key), enabled_key);
        }

        let mut project_model_profiles = BTreeSet::new();
        let mut template_model_profiles = BTreeSet::new();
        collect_object_keys_named(&project, "model_profiles", &mut project_model_profiles);
        collect_object_keys_named(&template, "model_profiles", &mut template_model_profiles);
        let declared_model_profiles = declared("declared_model_profile_ids");
        assert_eq!(declared_model_profiles, template_model_profiles);
        assert_subset(
            &project_model_profiles,
            &template_model_profiles,
            "project model profiles",
        );
        let optional_model_profiles = declared("optional_model_profile_ids");
        assert_eq!(
            optional_model_profiles,
            template_model_profiles
                .difference(&project_model_profiles)
                .cloned()
                .collect::<BTreeSet<_>>()
        );
        for (config, context) in [(&project, "project"), (&template, "template")] {
            let mut referenced_profiles = BTreeSet::new();
            collect_string_values_named(
                config,
                &["default_model_profile", "model_profile_id"],
                &mut referenced_profiles,
            );
            assert_subset(
                &referenced_profiles,
                &declared_model_profiles,
                &format!("{context} model profile refs"),
            );
        }

        let selection_schema = &schema["$defs"]["authoritySelection"];
        let schema_required = string_set(&selection_schema["required"], "selection required");
        let schema_properties = object_key_set(&selection_schema["properties"], "selection props");
        let mut selected_config_ids = BTreeSet::new();
        let mut selected_team_profile_ids = BTreeSet::new();
        for (config, context) in [(&project, "project"), (&template, "template")] {
            let selection = config["dev_team"]["authority_selection"]
                .as_object()
                .unwrap_or_else(|| panic!("{context} authority selection must be an object"));
            let selection_keys = selection.keys().cloned().collect::<BTreeSet<_>>();
            assert_eq!(selection_keys, schema_required);
            assert_eq!(selection_keys, schema_properties);

            let config_id = selection["config_id"]
                .as_str()
                .expect("selected config id must be a string");
            assert_eq!(config_id, config["project"]["id"]);
            selected_config_ids.insert(config_id.to_string());
            selected_team_profile_ids.insert(
                selection["team_profile_id"]
                    .as_str()
                    .expect("selected team profile id must be a string")
                    .to_string(),
            );
            let default_flow_id = selection["default_flow_id"]
                .as_str()
                .expect("selected default flow id must be a string");
            assert_eq!(default_flow_id, config["dev_team"]["default_flow_id"]);
            assert!(declared_flows.contains(default_flow_id));

            let binding_flow_ids = config["dev_team"]["work_item_flow_bindings"]
                .as_object()
                .expect("work-item bindings must be an object")
                .values()
                .map(|value| {
                    value
                        .as_str()
                        .expect("work-item binding values must be strings")
                        .to_string()
                })
                .collect::<BTreeSet<_>>();
            assert_subset(
                &binding_flow_ids,
                &declared_flows,
                &format!("{context} work-item flow bindings"),
            );
        }
        assert_eq!(selected_config_ids, declared("declared_config_ids"));
        assert_eq!(
            selected_team_profile_ids,
            declared("declared_team_profile_ids")
        );

        let approval_states = declared("approval_states");
        let mut capabilities = BTreeSet::new();
        for row in catalog["capability_admissibility"]
            .as_array()
            .expect("capability admissibility must be an array")
        {
            let capability = row["capability"]
                .as_str()
                .expect("capability id must be a string");
            assert!(capabilities.insert(capability.to_string()));
            assert!(!string_set(&row["admissible_effects"], "admissible effects").is_empty());
            assert!(row["command_required"].is_boolean());
            assert!(row["evidence_required"].is_boolean());
            assert_subset(
                &string_set(&row["approval_states"], "admissible approval states"),
                &approval_states,
                "capability approval states",
            );
        }
    }

    fn assert_strict_team_flow_config(
        config: &serde_json::Value,
        command_ids: &std::collections::BTreeSet<String>,
    ) {
        let roles = config["dev_team"]["roles"]
            .as_object()
            .expect("dev_team roles must be an object");
        let flows = config["dev_team"]["flows"]
            .as_object()
            .expect("dev_team flows must be an object");
        for (flow_id, flow) in flows {
            let step_sources = [flow.get("steps"), flow.get("ordered_steps")]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            assert_eq!(
                step_sources.len(),
                1,
                "flow {flow_id} must declare exactly one step-list alias"
            );
            let steps = step_sources[0]
                .as_array()
                .expect("flow steps must be an array");
            assert!(!steps.is_empty(), "flow {flow_id} must not be empty");
            let node_ids = steps
                .iter()
                .map(|step| {
                    let ids = ["node_id", "role_id", "step_id"]
                        .into_iter()
                        .filter_map(|key| step.get(key).and_then(serde_json::Value::as_str))
                        .collect::<Vec<_>>();
                    assert_eq!(
                        ids.len(),
                        1,
                        "flow {flow_id} nodes require exactly one id alias"
                    );
                    ids[0].to_string()
                })
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                node_ids.len(),
                steps.len(),
                "flow {flow_id} node ids must be unique"
            );
            let terminal_count = steps
                .iter()
                .filter(|step| {
                    ["terminal", "terminal_closure", "closes_workflow"]
                        .into_iter()
                        .any(|key| step.get(key).and_then(serde_json::Value::as_bool) == Some(true))
                })
                .count();
            assert_eq!(terminal_count, 1, "flow {flow_id} needs one terminal");
            for step in steps {
                let node_id = ["node_id", "role_id", "step_id"]
                    .into_iter()
                    .find_map(|key| step.get(key).and_then(serde_json::Value::as_str))
                    .unwrap();
                let role = roles
                    .get(node_id)
                    .unwrap_or_else(|| panic!("flow {flow_id} references unknown role {node_id}"));
                for field in ["runtime_role", "task_class", "inclusion_rule"] {
                    let source_count = usize::from(step.get(field).is_some())
                        + usize::from(role.get(field).is_some());
                    assert_eq!(
                        source_count, 1,
                        "flow {flow_id} node {node_id} field {field} needs one authority source"
                    );
                }
                let inclusion_rule = step
                    .get("inclusion_rule")
                    .or_else(|| role.get("inclusion_rule"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap();
                if !matches!(inclusion_rule, "always" | "never") {
                    assert!(
                        step.get("included")
                            .and_then(serde_json::Value::as_bool)
                            .is_some(),
                        "conditional node {flow_id}/{node_id} needs explicit included"
                    );
                }
                assert!(
                    step.get("required")
                        .and_then(serde_json::Value::as_bool)
                        .is_some(),
                    "node {flow_id}/{node_id} needs typed required"
                );
                assert!(
                    step["proof_gates"]["required_outputs"]
                        .as_array()
                        .is_some_and(|outputs| !outputs.is_empty()),
                    "node {flow_id}/{node_id} needs proof evidence"
                );
                let evidence_source_count = usize::from(
                    step["proof_gates"]["required_outputs"]
                        .as_array()
                        .is_some_and(|outputs| !outputs.is_empty()),
                ) + usize::from(
                    step.get("evidence_requirements")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|requirements| !requirements.is_empty()),
                );
                assert_eq!(
                    evidence_source_count, 1,
                    "node {flow_id}/{node_id} needs exactly one evidence source"
                );
                if let Some(command_ref) =
                    step.get("command_ref").and_then(serde_json::Value::as_str)
                {
                    assert!(
                        command_ids.contains(command_ref),
                        "node {flow_id}/{node_id} references unknown command {command_ref}"
                    );
                }
                let terminal_aliases = ["terminal", "terminal_closure", "closes_workflow"]
                    .into_iter()
                    .filter_map(|key| step.get(key).and_then(serde_json::Value::as_bool))
                    .collect::<Vec<_>>();
                assert!(
                    terminal_aliases.len() <= 1,
                    "node {flow_id}/{node_id} has terminal aliases"
                );
                let terminal = terminal_aliases.first().copied().unwrap_or(false);
                let next_node = step.get("next_node").and_then(serde_json::Value::as_str);
                if terminal {
                    assert!(
                        next_node.is_none(),
                        "terminal node {flow_id}/{node_id} has next_node"
                    );
                } else {
                    let next_node = next_node.unwrap_or_else(|| {
                        panic!("nonterminal node {flow_id}/{node_id} needs next_node")
                    });
                    assert!(
                        node_ids.contains(next_node),
                        "node {flow_id}/{node_id} has unknown next_node"
                    );
                }
                if let Some(rework) = step.get("rework_transitions") {
                    for target in rework
                        .as_object()
                        .expect("rework transitions must be a mapping")
                        .values()
                    {
                        let target = target.as_str().expect("rework target must be a string");
                        assert!(
                            node_ids.contains(target),
                            "node {flow_id}/{node_id} has unknown rework target"
                        );
                    }
                }
                if step
                    .get("requires_user_approval")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                {
                    let policy = step["approval_policy"]
                        .as_object()
                        .expect("approval policy must be an object");
                    assert!(policy
                        .get("mode")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|mode| !mode.is_empty()));
                    assert!(
                        policy
                            .get("allowed_decisions")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|values| values.iter().any(|value| value == "approved")),
                        "approval node {flow_id}/{node_id} must accept approved"
                    );
                    let approved_target = step["resume_transitions"]["approved"]
                        .as_str()
                        .expect("approval node needs an approved resume target");
                    assert_eq!(
                        Some(approved_target),
                        next_node,
                        "approval node {flow_id}/{node_id} must resume through its explicit edge"
                    );
                    assert!(
                        node_ids.contains(approved_target),
                        "approval node {flow_id}/{node_id} has unknown resume target"
                    );
                }
            }

            let mut current = ["node_id", "role_id", "step_id"]
                .into_iter()
                .find_map(|key| steps[0].get(key).and_then(serde_json::Value::as_str))
                .expect("first node needs an id");
            let mut visited = BTreeSet::new();
            loop {
                assert!(
                    visited.insert(current.to_string()),
                    "flow {flow_id} contains a forward-edge cycle at {current}"
                );
                let step = steps
                    .iter()
                    .find(|step| {
                        ["node_id", "role_id", "step_id"]
                            .into_iter()
                            .find_map(|key| step.get(key).and_then(serde_json::Value::as_str))
                            == Some(current)
                    })
                    .expect("reachable node must exist");
                let terminal = ["terminal", "terminal_closure", "closes_workflow"]
                    .into_iter()
                    .any(|key| step.get(key).and_then(serde_json::Value::as_bool) == Some(true));
                if terminal {
                    break;
                }
                current = step["next_node"]
                    .as_str()
                    .expect("nonterminal forward edge is required");
            }
            assert_eq!(
                visited.len(),
                steps.len(),
                "flow {flow_id} must reach every node before terminal closure"
            );
        }
    }

    #[test]
    fn project_and_master_team_flow_configs_are_strict_authority_inputs() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let commands: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(
                repository_root.join("docs/process/agent-extensions/commands.yaml"),
            )
            .expect("commands registry should be readable"),
        )
        .expect("commands registry should parse");
        let command_ids = crate::registry_ids_by_key(&commands, "commands", "command_id")
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        for relative_path in [
            "vida.config.yaml",
            "docs/framework/templates/vida.config.yaml.template",
        ] {
            let yaml: serde_yaml::Value = serde_yaml::from_str(
                &fs::read_to_string(repository_root.join(relative_path))
                    .unwrap_or_else(|_| panic!("{relative_path} should be readable")),
            )
            .unwrap_or_else(|_| panic!("{relative_path} should parse"));
            let json = serde_json::to_value(yaml).expect("YAML should convert to JSON");
            assert_strict_team_flow_config(&json, &command_ids);
        }
    }

    #[test]
    fn source_dsl_authority_catalogs_are_equal_and_schema_backed() {
        let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should be nested under the repository root");
        let config_paths = [
            "vida.config.yaml",
            "docs/framework/templates/vida.config.yaml.template",
            "install/assets/vida.config.yaml.template",
        ];
        let catalogs = config_paths
            .iter()
            .map(|relative_path| {
                let yaml: serde_yaml::Value = serde_yaml::from_str(
                    &fs::read_to_string(repository_root.join(relative_path))
                        .unwrap_or_else(|_| panic!("{relative_path} should be readable")),
                )
                .unwrap_or_else(|_| panic!("{relative_path} should parse"));
                serde_json::to_value(yaml).expect("YAML should convert to JSON")["dev_team"]
                    ["authority_catalog"]
                    .clone()
            })
            .collect::<Vec<_>>();
        for field in [
            "source_flow_fields",
            "source_flow_step_fields",
            "approval_policy_modes",
            "approval_policy_allowed_decisions",
        ] {
            for catalog in catalogs.iter().skip(1) {
                assert_eq!(catalog[field], catalogs[0][field]);
            }
        }
        let schema: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(
                repository_root.join("vida/config/schemas/team-flow-authority.schema.json"),
            )
            .expect("TeamFlow schema should be readable"),
        )
        .expect("TeamFlow schema should parse");
        for field in [
            "source_flow_fields",
            "source_flow_step_fields",
            "approval_policy_modes",
            "approval_policy_allowed_decisions",
        ] {
            let allowed = schema["$defs"]["sourceAuthorityCatalog"]["properties"][field]["items"]
                ["enum"]
                .as_array()
                .expect("source catalog schema enum");
            for value in catalogs[0][field].as_array().expect("source catalog array") {
                assert!(allowed.iter().any(|candidate| candidate == value));
            }
        }
    }
}
