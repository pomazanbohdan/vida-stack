use super::*;

fn host_runtime_label(cli_system: &str) -> String {
    let trimmed = cli_system.trim();
    if trimmed.is_empty() {
        return "Host runtime".to_string();
    }
    trimmed
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    format!(
                        "{}{}",
                        first.to_uppercase(),
                        chars.as_str().to_ascii_lowercase()
                    )
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_runtime_surface_within_project(
    project_root: &Path,
    runtime_surface: &str,
) -> Result<PathBuf, String> {
    let surface = runtime_surface.trim();
    if surface.is_empty() {
        return Err("Host CLI runtime_root must not be empty".to_string());
    }
    let candidate = PathBuf::from(surface);
    if candidate.is_absolute() {
        return Err(format!(
            "Host CLI runtime_root must be relative to the project root: {surface}"
        ));
    }
    if candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "Host CLI runtime_root must not contain parent directory traversal: {surface}"
        ));
    }
    Ok(project_root.join(candidate))
}

pub(crate) fn render_host_cli_template_from_catalog(
    cli_system: &str,
    project_root: &Path,
    runtime_root: &Path,
    template_root: &Path,
    agent_catalog: &[serde_json::Value],
    named_lane_catalog: &[serde_json::Value],
) -> Result<(), String> {
    crate::host_runtime_materialization::render_host_runtime_template_from_catalog(
        &host_runtime_label(cli_system),
        project_root,
        runtime_root,
        template_root,
        agent_catalog,
        named_lane_catalog,
    )
}

pub(crate) fn read_host_cli_agent_catalog(runtime_root: &Path) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::read_host_runtime_agent_catalog(runtime_root)
}

pub(crate) fn overlay_host_cli_agent_catalog(config: &serde_yaml::Value) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::overlay_host_runtime_agent_catalog(config)
}

pub(crate) fn host_cli_entry_carrier_catalog(
    entry: Option<&serde_yaml::Value>,
) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(entry)
}

pub(crate) fn materialize_host_cli_dispatch_alias_catalog(
    configured_aliases: &[serde_json::Value],
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::materialize_host_runtime_dispatch_alias_catalog(
        configured_aliases,
        agent_catalog,
    )
}

pub(crate) fn overlay_host_cli_dispatch_alias_catalog(
    config: &serde_yaml::Value,
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    crate::host_runtime_materialization::overlay_host_runtime_dispatch_alias_catalog(
        config,
        agent_catalog,
    )
}

pub(crate) fn host_cli_dispatch_alias_catalog_for_root(
    config: &serde_yaml::Value,
    root: &Path,
    agent_catalog: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, String> {
    crate::host_runtime_materialization::host_runtime_dispatch_alias_catalog_for_root(
        config,
        root,
        agent_catalog,
    )
}

fn pi_projection_configured_path(
    registry_entry: &serde_yaml::Value,
    key: &str,
    default_path: &str,
) -> String {
    yaml_string(yaml_lookup(registry_entry, &["app", key]))
        .unwrap_or_else(|| default_path.to_string())
}

fn resolve_pi_projection_path(
    project_root: &Path,
    runtime_root: &Path,
    field_name: &str,
    configured_path: &str,
) -> Result<PathBuf, String> {
    super::validate_project_relative_path(configured_path, field_name)?;
    let resolved = project_root.join(configured_path);
    if !resolved.starts_with(runtime_root) {
        return Err(format!(
            "Invalid `{field_name}` path `{configured_path}`: Pi projection paths must stay under the configured runtime_root"
        ));
    }
    Ok(resolved)
}

fn safe_pi_projection_file_stem(carrier_id: &str) -> Result<String, String> {
    let trimmed = carrier_id.trim();
    if trimmed.is_empty() {
        return Err("Pi projection carrier id must not be empty".to_string());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(format!(
            "Pi projection carrier id `{carrier_id}` contains unsupported path characters"
        ));
    }
    Ok(trimmed.to_string())
}

fn pi_projection_string_list(value: &serde_json::Value, key: &str) -> String {
    value[key]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn pi_projection_profile_ids(model_profiles: &serde_json::Value) -> String {
    model_profiles
        .as_object()
        .map(|profiles| profiles.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| "configured_by_vida_runtime".to_string())
}

fn render_pi_agent_projection_markdown(
    carrier: &serde_json::Value,
    model_profiles: &serde_json::Value,
) -> String {
    let carrier_id = carrier["role_id"].as_str().unwrap_or("pi-carrier");
    let tier = carrier["tier"].as_str().unwrap_or(carrier_id);
    let model = carrier["model"].as_str().unwrap_or("");
    let provider = carrier["model_provider"].as_str().unwrap_or("");
    let reasoning = carrier["model_reasoning_effort"].as_str().unwrap_or("");
    let runtime_roles = pi_projection_string_list(carrier, "runtime_roles");
    let task_classes = pi_projection_string_list(carrier, "task_classes");
    let profile_ids = pi_projection_profile_ids(model_profiles);

    format!(
        "---\nname: {carrier_id}\ngenerated_by: VIDA\nsource_of_truth: vida.config.yaml\nprojection_only: true\ninternal_agent_projection: true\ncanonical_execution_surface: vida agent-init\n---\n\n# VIDA Pi Agent Projection: {carrier_id}\n\nThis file is generated by VIDA. Do not treat `.pi/**` files as authority; the source of truth is `vida.config.yaml` plus VIDA runtime state.\n\n## Carrier And Runtime Role Boundaries\n\n- carrier_id: `{carrier_id}`\n- tier: `{tier}`\n- provider: `{provider}`\n- default model hint: `{model}`\n- reasoning/thinking level hint: `{reasoning}`\n- runtime roles: `{runtime_roles}`\n- task classes: `{task_classes}`\n- admissible model profiles are selected by VIDA runtime, not this file: `{profile_ids}`\n\n## Internal-Agent Projection Contract\n\n1. This projection is for Pi host UI/subagent affordances only.\n2. Canonical delegated execution remains VIDA TaskFlow / `vida agent-init`.\n3. Runtime role, model profile, cost, readiness, and write posture come from VIDA config/runtime state.\n4. Do not launch child subagents from this generated agent; no child subagent recursion is allowed.\n5. Do not self-dispatch, self-approve, or claim closure authority.\n6. Do not perform write-capable Pi execution until VIDA supplies an active packet-owned write guard and touched-path validation.\n7. Raw Pi output is not a VIDA receipt; completion requires VIDA receipt-backed execution evidence.\n"
    )
}

fn render_pi_agent_init_chain_projection(cli_system: &str) -> String {
    format!(
        "---\nname: vida-agent-init\ngenerated_by: VIDA\nsource_of_truth: vida.config.yaml\nprojection_only: true\ncanonical_execution_surface: vida agent-init\n---\n\n# VIDA Pi Agent-Init Projection Chain\n\nThis projection is a host-side runbook for the `{cli_system}` Pi environment. It is not an autonomous execution chain and must not be treated as a VIDA dispatch receipt.\n\n1. Start from the active VIDA TaskFlow packet and run `vida agent-init` or the exact command selected by VIDA.\n2. Do not launch child subagents from this chain.\n3. Do not self-dispatch, self-approve, or close tasks from Pi-local state.\n4. Use Pi-local output only as host affordance evidence; VIDA completion still requires receipt-backed execution evidence.\n5. Write-capable execution is forbidden until VIDA supplies active packet-owned write scope and touched-path validation.\n"
    )
}

pub(crate) fn materialize_pi_agent_projection(
    project_root: &Path,
    cli_system: &str,
    registry_entry: &serde_yaml::Value,
) -> Result<PathBuf, String> {
    let runtime_surface = super::host_cli_system_runtime_surface(registry_entry, cli_system);
    if runtime_surface.trim().is_empty() {
        return Err(format!(
            "No runtime_root configured for host CLI `{cli_system}`"
        ));
    }
    super::validate_project_relative_path(&runtime_surface, "runtime_root")?;
    let runtime_root = resolve_runtime_surface_within_project(project_root, &runtime_surface)?;

    let settings_default = format!("{}/settings.json", runtime_surface.trim_end_matches('/'));
    let agents_default = format!("{}/agents", runtime_surface.trim_end_matches('/'));
    let chains_default = format!("{}/chains", runtime_surface.trim_end_matches('/'));
    let settings_path = resolve_pi_projection_path(
        project_root,
        &runtime_root,
        "app.settings_path",
        &pi_projection_configured_path(registry_entry, "settings_path", &settings_default),
    )?;
    let agents_dir = resolve_pi_projection_path(
        project_root,
        &runtime_root,
        "app.agents_dir",
        &pi_projection_configured_path(registry_entry, "agents_dir", &agents_default),
    )?;
    let chains_dir = resolve_pi_projection_path(
        project_root,
        &runtime_root,
        "app.chains_dir",
        &pi_projection_configured_path(registry_entry, "chains_dir", &chains_default),
    )?;

    let carrier_catalog = host_cli_entry_carrier_catalog(Some(registry_entry));
    if carrier_catalog.is_empty() {
        return Err(format!(
            "Pi projection materialization for `{cli_system}` requires at least one configured carrier"
        ));
    }
    let overlay = super::read_yaml_file_checked(&project_root.join("vida.config.yaml"))
        .unwrap_or(serde_yaml::Value::Null);
    let pi_cli = yaml_lookup(&overlay, &["agent_system", "subagents", "pi_cli"])
        .cloned()
        .unwrap_or(serde_yaml::Value::Null);
    let pi_model_profiles = serde_json::to_value(
        yaml_lookup(&pi_cli, &["model_profiles"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);

    std::fs::create_dir_all(&agents_dir)
        .map_err(|error| format!("Failed to create {}: {error}", agents_dir.display()))?;
    std::fs::create_dir_all(&chains_dir)
        .map_err(|error| format!("Failed to create {}: {error}", chains_dir.display()))?;
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }

    let settings = serde_json::json!({
        "generated_by": "VIDA",
        "generated_file": true,
        "selected_host_cli_system": cli_system,
        "materialization_mode": super::PI_AGENT_PROJECTION_RENDER_MODE,
        "source_of_truth": yaml_string(yaml_lookup(registry_entry, &["source_of_truth"]))
            .unwrap_or_else(|| "vida.config.yaml".to_string()),
        "projection_owner": yaml_string(yaml_lookup(registry_entry, &["app", "projection_owner"]))
            .unwrap_or_else(|| "vida".to_string()),
        "local_files_are_authority": false,
        "runtime_root": runtime_surface,
        "paths": {
            "settings_path": settings_path.strip_prefix(project_root).ok().map(|path| path.display().to_string()).unwrap_or_else(|| settings_path.display().to_string()),
            "agents_dir": agents_dir.strip_prefix(project_root).ok().map(|path| path.display().to_string()).unwrap_or_else(|| agents_dir.display().to_string()),
            "chains_dir": chains_dir.strip_prefix(project_root).ok().map(|path| path.display().to_string()).unwrap_or_else(|| chains_dir.display().to_string()),
        },
        "runtime_boundaries": {
            "projection_only_not_authority": true,
            "canonical_execution_surface": "vida agent-init",
            "raw_pi_dispatch_is_forbidden": true,
            "child_subagent_recursion_allowed": false,
            "self_dispatch_allowed": false,
            "closure_authority": false,
            "write_capable_execution_requires_vida_write_guard": true,
        },
        "internal_agent_projection": {
            "enabled": true,
            "projection_kind": "pi_internal_agent_projection",
            "generated_agent_format": "markdown",
            "generated_chain_format": "projection_runbook_markdown",
            "authority": "vida_config_and_runtime_state",
            "canonical_execution_surface": "vida agent-init",
            "completion_evidence": "vida_receipt_backed_execution_evidence",
            "child_subagent_recursion_allowed": false,
            "self_dispatch_allowed": false,
            "closure_authority": false,
        },
        "dispatch": serde_json::to_value(yaml_lookup(registry_entry, &["dispatch"]).cloned().unwrap_or(serde_yaml::Value::Null)).unwrap_or(serde_json::Value::Null),
        "model_profiles": pi_model_profiles,
        "carriers": carrier_catalog,
    });
    let settings_body = serde_json::to_string_pretty(&settings)
        .map_err(|error| format!("Failed to render Pi settings projection: {error}"))?;
    std::fs::write(&settings_path, format!("{settings_body}\n"))
        .map_err(|error| format!("Failed to write {}: {error}", settings_path.display()))?;

    for carrier in settings["carriers"].as_array().into_iter().flatten() {
        let carrier_id = carrier["role_id"].as_str().unwrap_or_default();
        let file_stem = safe_pi_projection_file_stem(carrier_id)?;
        let agent_path = agents_dir.join(format!("{file_stem}.md"));
        std::fs::write(
            &agent_path,
            render_pi_agent_projection_markdown(carrier, &settings["model_profiles"]),
        )
        .map_err(|error| format!("Failed to write {}: {error}", agent_path.display()))?;
    }
    let chain_path = chains_dir.join("vida-agent-init.chain.md");
    std::fs::write(
        &chain_path,
        render_pi_agent_init_chain_projection(cli_system),
    )
    .map_err(|error| format!("Failed to write {}: {error}", chain_path.display()))?;

    Ok(runtime_root)
}

pub(crate) fn materialize_host_cli_template_with_catalog_render(
    project_root: &Path,
    cli_system: &str,
    registry_entry: &serde_yaml::Value,
) -> Result<PathBuf, String> {
    let source = super::resolve_host_cli_template_source(cli_system, Some(registry_entry))?;
    let runtime_surface = super::host_cli_system_runtime_surface(registry_entry, cli_system);
    let runtime_root = resolve_runtime_surface_within_project(project_root, &runtime_surface)?;
    super::copy_tree_if_missing(&source, &runtime_root)?;
    let overlay = super::read_yaml_file_checked(&project_root.join("vida.config.yaml"))
        .unwrap_or(serde_yaml::Value::Null);
    let scoring_policy = serde_json::to_value(
        yaml_lookup(&overlay, &["agent_system", "scoring"])
            .cloned()
            .unwrap_or(serde_yaml::Value::Null),
    )
    .unwrap_or(serde_json::Value::Null);
    let rendered_catalog_root = runtime_root.clone();
    let carrier_roles = {
        let overlay_roles = overlay_host_cli_agent_catalog(&overlay);
        if overlay_roles.is_empty() {
            read_host_cli_agent_catalog(&rendered_catalog_root)
        } else {
            overlay_roles
        }
    };
    let carrier_dispatch_aliases =
        host_cli_dispatch_alias_catalog_for_root(&overlay, project_root, &carrier_roles)?;
    if !carrier_roles.is_empty() {
        render_host_cli_template_from_catalog(
            cli_system,
            project_root,
            &runtime_root,
            &source,
            &carrier_roles,
            &carrier_dispatch_aliases,
        )?;
    }
    super::refresh_worker_strategy(project_root, &carrier_roles, &scoring_policy);
    Ok(runtime_root)
}

pub(crate) fn resolve_host_cli_agent_catalog_for_rendered_root(
    project_root: &Path,
    overlay: &serde_yaml::Value,
    catalog_entry: Option<&serde_yaml::Value>,
    selected_host_cli_system: &str,
) -> Vec<serde_json::Value> {
    let carrier_catalog_root = project_root.join(super::host_cli_system_runtime_surface(
        catalog_entry.unwrap_or(&serde_yaml::Value::Null),
        selected_host_cli_system,
    ));
    let overlay_rows = overlay_host_cli_agent_catalog(overlay);
    if overlay_rows.is_empty() {
        read_host_cli_agent_catalog(carrier_catalog_root.as_path())
    } else {
        overlay_rows
    }
}
