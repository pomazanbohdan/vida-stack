use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::project_activator_surface::load_registry_projection;

use super::*;

fn escape_toml_basic_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rendered_host_runtime_agent_catalog(
    agent_catalog: &[serde_json::Value],
    _named_lane_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    agent_catalog.to_vec()
}

fn render_host_runtime_config_toml(
    runtime_label: &str,
    template_root: &Path,
    agent_catalog: &[serde_json::Value],
    named_lane_catalog: &[serde_json::Value],
) -> String {
    let template_config = read_simple_toml_sections(&template_root.join("config.toml"));
    let max_threads = template_config
        .get("agents")
        .and_then(|section| section.get("max_threads"))
        .cloned()
        .unwrap_or_else(|| "4".to_string());
    let max_depth = template_config
        .get("agents")
        .and_then(|section| section.get("max_depth"))
        .cloned()
        .unwrap_or_else(|| "2".to_string());
    let mut lines = vec![
        "[features]".to_string(),
        "multi_agent = true".to_string(),
        String::new(),
        "[agents]".to_string(),
        format!("max_threads = {max_threads}"),
        format!("max_depth = {max_depth}"),
        String::new(),
    ];
    for row in rendered_host_runtime_agent_catalog(agent_catalog, named_lane_catalog) {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        let description = row["description"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(escape_toml_basic_string)
            .unwrap_or_else(|| {
                escape_toml_basic_string(&format!(
                    "Rendered {runtime_label} executor lane for VIDA tier `{}`. Rate: {}.",
                    row["tier"].as_str().unwrap_or(role_id),
                    row["rate"].as_u64().unwrap_or(0)
                ))
            });
        lines.push(format!("[agents.{role_id}]"));
        lines.push(format!("description = \"{description}\""));
        lines.push(format!("config_file = \"agents/{role_id}.toml\""));
        lines.push(String::new());
    }
    format!("{}\n", lines.join("\n"))
}

fn set_toml_scalar_line(contents: &str, key: &str, rendered_value: &str) -> String {
    let replacement = format!("{key} = {rendered_value}");
    let mut lines = Vec::new();
    let mut replaced = false;
    for line in contents.lines() {
        if line.trim_start().starts_with(&format!("{key} =")) && !replaced {
            lines.push(replacement.clone());
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(replacement);
    }
    format!("{}\n", lines.join("\n"))
}

fn extract_toml_multiline_string(contents: &str, key: &str) -> Option<String> {
    let marker = format!("{key} = \"\"\"");
    let mut lines = contents.lines();
    while let Some(line) = lines.next() {
        if !line.trim_start().starts_with(&marker) {
            continue;
        }
        let mut body = Vec::new();
        for next_line in &mut lines {
            if next_line.trim() == "\"\"\"" {
                return Some(body.join("\n"));
            }
            body.push(next_line.to_string());
        }
        return Some(body.join("\n"));
    }
    None
}

fn set_toml_multiline_string(contents: &str, key: &str, body: &str) -> String {
    let marker = format!("{key} = \"\"\"");
    let mut lines = Vec::new();
    let mut replaced = false;
    let mut source = contents.lines();

    while let Some(line) = source.next() {
        if line.trim_start().starts_with(&marker) && !replaced {
            lines.push(marker.clone());
            lines.extend(body.lines().map(ToString::to_string));
            lines.push("\"\"\"".to_string());
            replaced = true;
            for next_line in &mut source {
                if next_line.trim() == "\"\"\"" {
                    break;
                }
            }
            continue;
        }
        lines.push(line.to_string());
    }

    if !replaced {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(marker);
        lines.extend(body.lines().map(ToString::to_string));
        lines.push("\"\"\"".to_string());
    }

    format!("{}\n", lines.join("\n"))
}

fn strip_toml_keys(contents: &str, keys: &[&str]) -> String {
    let mut lines = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if keys
            .iter()
            .any(|key| trimmed.starts_with(&format!("{key} =")))
        {
            continue;
        }
        lines.push(line.to_string());
    }
    format!("{}\n", lines.join("\n"))
}

fn compose_host_runtime_lane_developer_instructions(
    base_instructions: Option<&str>,
    lane_override: Option<&str>,
) -> Option<String> {
    match (
        base_instructions
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        lane_override
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ) {
        (Some(base), Some(overlay)) => Some(format!(
            "{base}\n\nLane activation overlay:\n{overlay}\n\nFollow both layers: keep the carrier-tier posture and boundaries, then apply the lane-specific mission as the active role for this packet."
        )),
        (Some(base), None) => Some(base.to_string()),
        (None, Some(overlay)) => Some(overlay.to_string()),
        (None, None) => None,
    }
}

fn render_host_runtime_agent_toml(
    _runtime_label: &str,
    row: &serde_json::Value,
    template_contents: Option<&str>,
) -> Option<String> {
    row["role_id"].as_str()?;
    let model = row["model"].as_str().unwrap_or("gpt-5.4");
    let reasoning_effort = row["model_reasoning_effort"].as_str().unwrap_or("medium");
    let sandbox_mode = row["sandbox_mode"].as_str().unwrap_or("workspace-write");
    let developer_instructions_override = row["developer_instructions"]
        .as_str()
        .filter(|value| !value.trim().is_empty());
    if let Some(template) = template_contents.filter(|value| !value.trim().is_empty()) {
        let patched = set_toml_scalar_line(template, "model", &format!("\"{model}\""));
        let patched = set_toml_scalar_line(
            &patched,
            "model_reasoning_effort",
            &format!("\"{reasoning_effort}\""),
        );
        let patched =
            set_toml_scalar_line(&patched, "sandbox_mode", &format!("\"{sandbox_mode}\""));
        let patched = if let Some(instructions) = compose_host_runtime_lane_developer_instructions(
            extract_toml_multiline_string(template, "developer_instructions").as_deref(),
            developer_instructions_override,
        ) {
            set_toml_multiline_string(&patched, "developer_instructions", &instructions)
        } else {
            patched
        };
        return Some(strip_toml_keys(
            &patched,
            &[
                "vida_tier",
                "vida_rate",
                "vida_reasoning_band",
                "vida_default_runtime_role",
                "vida_runtime_roles",
                "vida_task_classes",
            ],
        ));
    }

    if let Some(instructions) =
        compose_host_runtime_lane_developer_instructions(None, developer_instructions_override)
    {
        return Some(format!(
            "model = \"{model}\"\nmodel_reasoning_effort = \"{reasoning_effort}\"\nsandbox_mode = \"{sandbox_mode}\"\ndeveloper_instructions = \"\"\"\n{instructions}\n\"\"\"\n"
        ));
    }

    Some(format!(
        "model = \"{model}\"\nmodel_reasoning_effort = \"{reasoning_effort}\"\nsandbox_mode = \"{sandbox_mode}\"\n"
    ))
}

pub(crate) fn render_host_runtime_template_from_catalog(
    runtime_label: &str,
    _project_root: &Path,
    runtime_root: &Path,
    template_root: &Path,
    agent_catalog: &[serde_json::Value],
    named_lane_catalog: &[serde_json::Value],
) -> Result<(), String> {
    let agents_root = runtime_root.join("agents");
    fs::create_dir_all(&agents_root)
        .map_err(|error| format!("Failed to create {}: {error}", agents_root.display()))?;

    let template_agents = read_host_runtime_agent_catalog(template_root)
        .into_iter()
        .filter_map(|row| {
            Some((
                row["role_id"].as_str()?.to_string(),
                row["config_file"].as_str()?.to_string(),
            ))
        })
        .collect::<HashMap<_, _>>();

    for entry in fs::read_dir(&agents_root)
        .map_err(|error| format!("Failed to read {}: {error}", agents_root.display()))?
    {
        let path = entry
            .map_err(|error| format!("Failed to inspect {}: {error}", agents_root.display()))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            fs::remove_file(&path)
                .map_err(|error| format!("Failed to remove {}: {error}", path.display()))?;
        }
    }

    let config_body = render_host_runtime_config_toml(
        runtime_label,
        template_root,
        agent_catalog,
        named_lane_catalog,
    );
    fs::write(runtime_root.join("config.toml"), config_body).map_err(|error| {
        format!(
            "Failed to write {}: {error}",
            runtime_root.join("config.toml").display()
        )
    })?;

    for row in rendered_host_runtime_agent_catalog(agent_catalog, named_lane_catalog) {
        let Some(role_id) = row["role_id"].as_str() else {
            continue;
        };
        let template_role_id = row["template_role_id"].as_str().unwrap_or(role_id);
        let template_contents = template_agents
            .get(template_role_id)
            .and_then(|config_file| fs::read_to_string(template_root.join(config_file)).ok());
        let Some(body) =
            render_host_runtime_agent_toml(runtime_label, &row, template_contents.as_deref())
        else {
            continue;
        };
        fs::write(agents_root.join(format!("{role_id}.toml")), body).map_err(|error| {
            format!(
                "Failed to write {}: {error}",
                agents_root.join(format!("{role_id}.toml")).display()
            )
        })?;
    }

    Ok(())
}

pub(crate) fn read_host_runtime_agent_catalog(runtime_root: &Path) -> Vec<serde_json::Value> {
    let runtime_config = read_simple_toml_sections(&runtime_root.join("config.toml"));
    let mut roles = runtime_config
        .iter()
        .filter_map(|(section, values)| {
            let role_id = section.strip_prefix("agents.")?;
            if role_id.is_empty() || role_id == "development" {
                return None;
            }
            let config_file = values.get("config_file").cloned().unwrap_or_default();
            let role_config = if config_file.is_empty() {
                HashMap::new()
            } else {
                read_simple_toml_sections(&runtime_root.join(&config_file))
                    .remove("")
                    .unwrap_or_default()
            };
            let tier = role_config
                .get("vida_tier")
                .cloned()
                .unwrap_or_else(|| role_id.to_string());
            let runtime_roles =
                csv_string_list(role_config.get("vida_runtime_roles"));
            let task_classes =
                csv_string_list(role_config.get("vida_task_classes"));
            let rate = role_config
                .get("vida_rate")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
            let compatibility_row = serde_json::json!({
                "role_id": role_id,
                "model": role_config.get("model").cloned().unwrap_or_default(),
                "model_provider": role_config.get("model_provider").cloned().unwrap_or_default(),
                "model_reasoning_effort": role_config.get("model_reasoning_effort").cloned().unwrap_or_default(),
                "plan_mode_reasoning_effort": role_config.get("plan_mode_reasoning_effort").cloned().unwrap_or_default(),
                "sandbox_mode": role_config.get("sandbox_mode").cloned().unwrap_or_default(),
                "rate": rate,
                "runtime_roles": runtime_roles.clone(),
                "task_classes": task_classes.clone(),
            });
            let profile_projection =
                crate::model_profile_contract::normalize_profile_projection_from_json_compat(
                    role_id,
                    &compatibility_row,
                    Some(rate),
                    &runtime_roles,
                    &task_classes,
                );
            Some(serde_json::json!({
                "role_id": role_id,
                "description": values.get("description").cloned().unwrap_or_default(),
                "config_file": config_file,
                "model": profile_projection["model"].clone(),
                "model_provider": profile_projection["model_provider"].clone(),
                "model_reasoning_effort": profile_projection["model_reasoning_effort"].clone(),
                "plan_mode_reasoning_effort": profile_projection["plan_mode_reasoning_effort"].clone(),
                "sandbox_mode": profile_projection["sandbox_mode"].clone(),
                "default_model_profile": profile_projection["default_model_profile"].clone(),
                "model_profiles": profile_projection["model_profiles"].clone(),
                "tier": tier,
                "rate": rate,
                "normalized_cost_units": profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| profile_projection["default_model_profile"].as_str().and_then(|profile_id| profiles.get(profile_id)))
                    .and_then(|profile| profile["normalized_cost_units"].as_u64())
                    .unwrap_or(rate),
                "reasoning_band": role_config
                    .get("vida_reasoning_band")
                    .cloned()
                    .unwrap_or_else(|| role_config.get("model_reasoning_effort").cloned().unwrap_or_default()),
                "default_runtime_role": role_config.get("vida_default_runtime_role").cloned().unwrap_or_default(),
                "runtime_roles": runtime_roles,
                "task_classes": task_classes,
            }))
        })
        .collect::<Vec<_>>();
    roles.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
            .then_with(|| {
                left["role_id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["role_id"].as_str().unwrap_or_default())
            })
    });
    roles
}

pub(crate) fn overlay_host_runtime_agent_catalog(
    config: &serde_yaml::Value,
) -> Vec<serde_json::Value> {
    let Some(serde_yaml::Value::Mapping(agents)) =
        yaml_lookup(config, &["host_environment", "codex", "agents"])
    else {
        return Vec::new();
    };
    let mut rows = agents
        .iter()
        .filter_map(|(agent_id, value)| {
            let role_id = match agent_id {
                serde_yaml::Value::String(text) if !text.trim().is_empty() => text.trim(),
                _ => return None,
            };
            let runtime_roles = yaml_string_list(yaml_lookup(value, &["runtime_roles"]));
            let task_classes = yaml_string_list(yaml_lookup(value, &["task_classes"]));
            let rate = yaml_string(yaml_lookup(value, &["rate"]))
                .and_then(|raw| raw.parse::<u64>().ok())
                .unwrap_or(0);
            let profile_projection =
                crate::model_profile_contract::normalize_profile_projection_from_yaml(
                    role_id,
                    value,
                    Some(rate),
                    &runtime_roles,
                    &task_classes,
                );
            Some(serde_json::json!({
                "role_id": role_id,
                "description": yaml_string(yaml_lookup(value, &["description"])).unwrap_or_default(),
                "config_file": format!("agents/{role_id}.toml"),
                "model": profile_projection["model"].clone(),
                "model_provider": profile_projection["model_provider"].clone(),
                "model_reasoning_effort": profile_projection["model_reasoning_effort"].clone(),
                "plan_mode_reasoning_effort": profile_projection["plan_mode_reasoning_effort"].clone(),
                "sandbox_mode": profile_projection["sandbox_mode"].clone(),
                "default_model_profile": profile_projection["default_model_profile"].clone(),
                "model_profiles": profile_projection["model_profiles"].clone(),
                "tier": yaml_string(yaml_lookup(value, &["tier"])).unwrap_or_else(|| role_id.to_string()),
                "rate": rate,
                "normalized_cost_units": profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| profile_projection["default_model_profile"].as_str().and_then(|profile_id| profiles.get(profile_id)))
                    .and_then(|profile| profile["normalized_cost_units"].as_u64())
                    .unwrap_or(rate),
                "reasoning_band": yaml_string(yaml_lookup(value, &["reasoning_band"])).unwrap_or_default(),
                "default_runtime_role": yaml_string(yaml_lookup(value, &["default_runtime_role"])).unwrap_or_default(),
                "runtime_roles": runtime_roles,
                "task_classes": task_classes,
            }))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
            .then_with(|| {
                left["role_id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["role_id"].as_str().unwrap_or_default())
            })
    });
    rows
}

pub(crate) fn host_runtime_entry_carrier_catalog(
    entry: Option<&serde_yaml::Value>,
) -> Vec<serde_json::Value> {
    let Some(serde_yaml::Value::Mapping(carriers)) =
        entry.and_then(|value| yaml_lookup(value, &["carriers"]))
    else {
        return Vec::new();
    };
    let mut rows = carriers
        .iter()
        .filter_map(|(carrier_id, value)| {
            let role_id = match carrier_id {
                serde_yaml::Value::String(text) if !text.trim().is_empty() => text.trim(),
                _ => return None,
            };
            let runtime_roles = yaml_string_list(yaml_lookup(value, &["runtime_roles"]));
            let task_classes = yaml_string_list(yaml_lookup(value, &["task_classes"]));
            let rate = yaml_string(yaml_lookup(value, &["rate"]))
                .and_then(|raw| raw.parse::<u64>().ok())
                .unwrap_or(0);
            let profile_projection =
                crate::model_profile_contract::normalize_profile_projection_from_yaml(
                    role_id,
                    value,
                    Some(rate),
                    &runtime_roles,
                    &task_classes,
                );
            Some(serde_json::json!({
                "role_id": role_id,
                "description": yaml_string(yaml_lookup(value, &["description"])).unwrap_or_default(),
                "config_file": "",
                "model": profile_projection["model"].clone(),
                "model_provider": profile_projection["model_provider"].clone(),
                "model_reasoning_effort": profile_projection["model_reasoning_effort"].clone(),
                "plan_mode_reasoning_effort": profile_projection["plan_mode_reasoning_effort"].clone(),
                "sandbox_mode": profile_projection["sandbox_mode"].clone(),
                "default_model_profile": profile_projection["default_model_profile"].clone(),
                "model_profiles": profile_projection["model_profiles"].clone(),
                "tier": yaml_string(yaml_lookup(value, &["tier"])).unwrap_or_else(|| role_id.to_string()),
                "rate": rate,
                "normalized_cost_units": profile_projection["model_profiles"]
                    .as_object()
                    .and_then(|profiles| profile_projection["default_model_profile"].as_str().and_then(|profile_id| profiles.get(profile_id)))
                    .and_then(|profile| profile["normalized_cost_units"].as_u64())
                    .unwrap_or(rate),
                "reasoning_band": yaml_string(yaml_lookup(value, &["reasoning_band"])).unwrap_or_default(),
                "default_runtime_role": yaml_string(yaml_lookup(value, &["default_runtime_role"])).unwrap_or_default(),
                "runtime_roles": runtime_roles,
                "task_classes": task_classes,
            }))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
            .then_with(|| {
                left["role_id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["role_id"].as_str().unwrap_or_default())
            })
    });
    rows
}

pub(crate) fn materialize_host_runtime_dispatch_alias_catalog(
    configured_aliases: &[serde_json::Value],
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let carrier_rows = agent_catalog
        .iter()
        .filter_map(|row| Some((row["tier"].as_str()?.to_string(), row.clone())))
        .collect::<std::collections::HashMap<_, _>>();
    let mut rows = configured_aliases
        .iter()
        .filter_map(|value| {
            let lane_id = value["alias_id"].as_str()?.trim();
            let carrier_tier = value["carrier_tier"].as_str()?.trim();
            let mut row = carrier_rows.get(carrier_tier)?.clone();
            let runtime_role = json_string(value.get("runtime_role"))
                .or_else(|| json_string(value.get("default_runtime_role")))
                .unwrap_or_default();
            let runtime_roles = value["runtime_roles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            let task_classes = value["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            row["role_id"] = serde_json::Value::String(lane_id.to_string());
            row["description"] = value
                .get("description")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            row["config_file"] = serde_json::Value::String(format!("agents/{lane_id}.toml"));
            row["default_runtime_role"] = serde_json::Value::String(runtime_role.clone());
            row["runtime_roles"] =
                serde_json::json!(if runtime_roles.is_empty() && !runtime_role.is_empty() {
                    vec![runtime_role]
                } else {
                    runtime_roles
                });
            row["task_classes"] = serde_json::json!(task_classes);
            row["template_role_id"] = serde_json::Value::String(carrier_tier.to_string());
            row["carrier_tier"] = row["tier"].clone();
            row["developer_instructions"] = value
                .get("developer_instructions")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            Some(row)
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

pub(crate) fn overlay_host_runtime_dispatch_alias_catalog(
    config: &serde_yaml::Value,
    agent_catalog: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let Some(serde_yaml::Value::Mapping(configured_aliases)) =
        yaml_lookup(config, &["host_environment", "codex", "dispatch_aliases"])
    else {
        return Vec::new();
    };
    let carrier_rows = agent_catalog
        .iter()
        .filter_map(|row| Some((row["tier"].as_str()?.to_string(), row.clone())))
        .collect::<std::collections::HashMap<_, _>>();
    let mut rows = configured_aliases
        .iter()
        .filter_map(|(lane_id, value)| {
            let lane_id = match lane_id {
                serde_yaml::Value::String(text) if !text.trim().is_empty() => text.trim(),
                _ => return None,
            };
            let carrier_tier = yaml_string(yaml_lookup(value, &["carrier_tier"]))?;
            let mut row = carrier_rows.get(&carrier_tier)?.clone();
            let runtime_role = yaml_string(yaml_lookup(value, &["runtime_role"]))
                .or_else(|| yaml_string(yaml_lookup(value, &["default_runtime_role"])))
                .unwrap_or_default();
            let runtime_roles = {
                let rows = yaml_string_list(yaml_lookup(value, &["runtime_roles"]));
                if rows.is_empty() && !runtime_role.is_empty() {
                    vec![runtime_role.clone()]
                } else {
                    rows
                }
            };
            row["role_id"] = serde_json::Value::String(lane_id.to_string());
            row["description"] = serde_json::Value::String(
                yaml_string(yaml_lookup(value, &["description"])).unwrap_or_default(),
            );
            row["config_file"] = serde_json::Value::String(format!("agents/{lane_id}.toml"));
            row["default_runtime_role"] = serde_json::Value::String(runtime_role);
            row["runtime_roles"] = serde_json::json!(runtime_roles);
            row["task_classes"] =
                serde_json::json!(yaml_string_list(yaml_lookup(value, &["task_classes"])));
            row["template_role_id"] = serde_json::Value::String(carrier_tier);
            row["carrier_tier"] = row["tier"].clone();
            row["developer_instructions"] = serde_json::Value::String(
                yaml_string(yaml_lookup(value, &["developer_instructions"])).unwrap_or_default(),
            );
            Some(row)
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

pub(crate) fn host_runtime_dispatch_alias_catalog_for_root(
    config: &serde_yaml::Value,
    root: &Path,
    agent_catalog: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, String> {
    let require_registry_files = yaml_bool(
        yaml_lookup(
            config,
            &["agent_extensions", "validation", "require_registry_files"],
        ),
        false,
    );
    let configured_path = yaml_string(yaml_lookup(
        config,
        &["agent_extensions", "registries", "dispatch_aliases"],
    ));
    if let Some(path) = configured_path.as_deref() {
        let registry = load_registry_projection(
            root,
            Some(path),
            "dispatch_aliases",
            "alias_id",
            "dispatch_aliases",
            require_registry_files,
        )
        .map_err(|error| format!("failed to load dispatch aliases registry `{path}`: {error}"))?;
        let rows = registry_rows_by_key(&registry, "dispatch_aliases", "alias_id", &[]);
        if !rows.is_empty() {
            return Ok(materialize_host_runtime_dispatch_alias_catalog(
                &rows,
                agent_catalog,
            ));
        }
    }
    Ok(overlay_host_runtime_dispatch_alias_catalog(
        config,
        agent_catalog,
    ))
}
fn csv_string_list(value: Option<&String>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
