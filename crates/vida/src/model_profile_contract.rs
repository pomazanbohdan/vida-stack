use serde_json::Map;

fn sanitize_profile_token(raw: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}

fn synthetic_profile_id(owner_id: &str, model_ref: &str, reasoning_effort: &str) -> String {
    let owner = sanitize_profile_token(owner_id);
    let model = sanitize_profile_token(model_ref);
    let reasoning = sanitize_profile_token(reasoning_effort);
    let parts = [owner, model, reasoning]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "default_profile".to_string()
    } else {
        parts.join("_")
    }
}

fn yaml_u64(value: Option<&serde_yaml::Value>) -> Option<u64> {
    value.and_then(|node| match node {
        serde_yaml::Value::Number(number) => number.as_u64(),
        serde_yaml::Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    })
}

fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|node| match node {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    })
}

fn default_provider_from_model_ref(model_ref: &str) -> Option<String> {
    let trimmed = model_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((provider, _)) = trimmed.split_once('/') {
        let provider = provider.trim();
        if !provider.is_empty() {
            return Some(provider.to_string());
        }
    }
    None
}

fn profile_with_defaults(
    profile_id: &str,
    model_ref: Option<String>,
    provider: Option<String>,
    reasoning_effort: Option<String>,
    plan_mode_reasoning_effort: Option<String>,
    sandbox_mode: Option<String>,
    normalized_cost_units: Option<u64>,
    speed_tier: Option<String>,
    quality_tier: Option<String>,
    write_scope: Option<String>,
    runtime_roles: Vec<String>,
    task_classes: Vec<String>,
    readiness: Option<serde_json::Value>,
    reasoning_control: Option<serde_json::Value>,
) -> serde_json::Value {
    let model_ref = model_ref.unwrap_or_default();
    let provider = provider.or_else(|| default_provider_from_model_ref(&model_ref));
    serde_json::json!({
        "profile_id": profile_id,
        "model_ref": model_ref,
        "provider": provider,
        "reasoning_effort": reasoning_effort.unwrap_or_default(),
        "plan_mode_reasoning_effort": plan_mode_reasoning_effort.unwrap_or_default(),
        "sandbox_mode": sandbox_mode.unwrap_or_default(),
        "normalized_cost_units": normalized_cost_units.unwrap_or(0),
        "speed_tier": speed_tier.unwrap_or_default(),
        "quality_tier": quality_tier.unwrap_or_default(),
        "write_scope": write_scope.unwrap_or_default(),
        "runtime_roles": runtime_roles,
        "task_classes": task_classes,
        "readiness": readiness.unwrap_or(serde_json::Value::Null),
        "reasoning_control": reasoning_control.unwrap_or(serde_json::Value::Null),
    })
}

fn projection_from_profiles(
    default_model_profile: Option<String>,
    mut profiles: Vec<serde_json::Value>,
) -> serde_json::Value {
    profiles.sort_by(|left, right| {
        left["profile_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["profile_id"].as_str().unwrap_or_default())
    });
    let mut profile_map = Map::new();
    for profile in &profiles {
        if let Some(profile_id) = profile["profile_id"].as_str() {
            profile_map.insert(profile_id.to_string(), profile.clone());
        }
    }
    let default_profile = default_model_profile
        .as_deref()
        .and_then(|profile_id| profile_map.get(profile_id))
        .cloned()
        .or_else(|| profiles.first().cloned())
        .unwrap_or(serde_json::Value::Null);
    let default_profile_id = default_profile["profile_id"]
        .as_str()
        .map(str::to_string)
        .or(default_model_profile);
    serde_json::json!({
        "default_model_profile": default_profile_id,
        "model_profiles": serde_json::Value::Object(profile_map),
        "model": default_profile["model_ref"].clone(),
        "model_provider": default_profile["provider"].clone(),
        "model_reasoning_effort": default_profile["reasoning_effort"].clone(),
        "plan_mode_reasoning_effort": default_profile["plan_mode_reasoning_effort"].clone(),
        "sandbox_mode": default_profile["sandbox_mode"].clone(),
        "current_model_ref": default_profile["model_ref"].clone(),
        "current_reasoning_effort": default_profile["reasoning_effort"].clone(),
        "current_model_profile": default_profile["profile_id"].clone(),
    })
}

pub(crate) fn normalize_profile_projection_from_yaml(
    owner_id: &str,
    owner: &serde_yaml::Value,
    fallback_rate: Option<u64>,
    fallback_runtime_roles: &[String],
    fallback_task_classes: &[String],
) -> serde_json::Value {
    let default_model_profile =
        crate::yaml_string(crate::yaml_lookup(owner, &["default_model_profile"]))
            .filter(|value| !value.trim().is_empty());
    let fallback_model_ref = crate::yaml_string(crate::yaml_lookup(owner, &["model"]))
        .or_else(|| crate::yaml_string(crate::yaml_lookup(owner, &["model_ref"])));
    let fallback_provider = crate::yaml_string(crate::yaml_lookup(owner, &["provider"]))
        .or_else(|| crate::yaml_string(crate::yaml_lookup(owner, &["model_provider"])));
    let fallback_reasoning_effort =
        crate::yaml_string(crate::yaml_lookup(owner, &["model_reasoning_effort"]))
            .or_else(|| crate::yaml_string(crate::yaml_lookup(owner, &["reasoning_effort"])));
    let fallback_plan_mode_reasoning_effort =
        crate::yaml_string(crate::yaml_lookup(owner, &["plan_mode_reasoning_effort"]));
    let fallback_sandbox_mode = crate::yaml_string(crate::yaml_lookup(owner, &["sandbox_mode"]));
    let fallback_normalized_cost_units =
        yaml_u64(crate::yaml_lookup(owner, &["normalized_cost_units"]))
            .or_else(|| yaml_u64(crate::yaml_lookup(owner, &["budget_cost_units"])))
            .or(fallback_rate)
            .or_else(|| yaml_u64(crate::yaml_lookup(owner, &["rate"])));
    let fallback_speed_tier = crate::yaml_string(crate::yaml_lookup(owner, &["speed_tier"]));
    let fallback_quality_tier = crate::yaml_string(crate::yaml_lookup(owner, &["quality_tier"]));
    let fallback_write_scope = crate::yaml_string(crate::yaml_lookup(owner, &["write_scope"]));
    let mut profiles = Vec::new();

    if let Some(entries) =
        crate::yaml_lookup(owner, &["model_profiles"]).and_then(serde_yaml::Value::as_mapping)
    {
        for (profile_id, profile_value) in entries {
            let Some(profile_id) = profile_id
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let runtime_roles = {
                let rows =
                    crate::yaml_string_list(crate::yaml_lookup(profile_value, &["runtime_roles"]));
                if rows.is_empty() {
                    fallback_runtime_roles.to_vec()
                } else {
                    rows
                }
            };
            let task_classes = {
                let rows =
                    crate::yaml_string_list(crate::yaml_lookup(profile_value, &["task_classes"]));
                if rows.is_empty() {
                    fallback_task_classes.to_vec()
                } else {
                    rows
                }
            };
            profiles.push(profile_with_defaults(
                profile_id,
                crate::yaml_string(crate::yaml_lookup(profile_value, &["model_ref"]))
                    .or_else(|| crate::yaml_string(crate::yaml_lookup(profile_value, &["model"])))
                    .or_else(|| fallback_model_ref.clone()),
                crate::yaml_string(crate::yaml_lookup(profile_value, &["provider"]))
                    .or_else(|| {
                        crate::yaml_string(crate::yaml_lookup(profile_value, &["model_provider"]))
                    })
                    .or_else(|| fallback_provider.clone()),
                crate::yaml_string(crate::yaml_lookup(profile_value, &["reasoning_effort"]))
                    .or_else(|| {
                        crate::yaml_string(crate::yaml_lookup(
                            profile_value,
                            &["model_reasoning_effort"],
                        ))
                    })
                    .or_else(|| fallback_reasoning_effort.clone()),
                crate::yaml_string(crate::yaml_lookup(
                    profile_value,
                    &["plan_mode_reasoning_effort"],
                ))
                .or_else(|| fallback_plan_mode_reasoning_effort.clone()),
                crate::yaml_string(crate::yaml_lookup(profile_value, &["sandbox_mode"]))
                    .or_else(|| fallback_sandbox_mode.clone()),
                yaml_u64(crate::yaml_lookup(
                    profile_value,
                    &["normalized_cost_units"],
                ))
                .or_else(|| yaml_u64(crate::yaml_lookup(profile_value, &["budget_cost_units"])))
                .or_else(|| yaml_u64(crate::yaml_lookup(profile_value, &["rate"])))
                .or(fallback_normalized_cost_units),
                crate::yaml_string(crate::yaml_lookup(profile_value, &["speed_tier"]))
                    .or_else(|| fallback_speed_tier.clone()),
                crate::yaml_string(crate::yaml_lookup(profile_value, &["quality_tier"]))
                    .or_else(|| fallback_quality_tier.clone()),
                crate::yaml_string(crate::yaml_lookup(profile_value, &["write_scope"]))
                    .or_else(|| fallback_write_scope.clone()),
                runtime_roles,
                task_classes,
                crate::yaml_lookup(profile_value, &["readiness"]).map(|value| {
                    serde_json::to_value(value.clone()).unwrap_or(serde_json::Value::Null)
                }),
                crate::yaml_lookup(profile_value, &["reasoning_control"]).map(|value| {
                    serde_json::to_value(value.clone()).unwrap_or(serde_json::Value::Null)
                }),
            ));
        }
    }

    if profiles.is_empty() && fallback_model_ref.is_some() {
        let model_ref = fallback_model_ref.clone().unwrap_or_default();
        let reasoning_effort = fallback_reasoning_effort.clone().unwrap_or_default();
        profiles.push(profile_with_defaults(
            &synthetic_profile_id(owner_id, &model_ref, &reasoning_effort),
            Some(model_ref),
            fallback_provider,
            fallback_reasoning_effort,
            fallback_plan_mode_reasoning_effort,
            fallback_sandbox_mode,
            fallback_normalized_cost_units,
            fallback_speed_tier,
            fallback_quality_tier,
            fallback_write_scope,
            fallback_runtime_roles.to_vec(),
            fallback_task_classes.to_vec(),
            crate::yaml_lookup(owner, &["readiness"]).map(|value| {
                serde_json::to_value(value.clone()).unwrap_or(serde_json::Value::Null)
            }),
            crate::yaml_lookup(owner, &["reasoning_control"]).map(|value| {
                serde_json::to_value(value.clone()).unwrap_or(serde_json::Value::Null)
            }),
        ));
    }

    projection_from_profiles(default_model_profile, profiles)
}

pub(crate) fn normalize_profile_projection_from_json_compat(
    owner_id: &str,
    row: &serde_json::Value,
    fallback_rate: Option<u64>,
    fallback_runtime_roles: &[String],
    fallback_task_classes: &[String],
) -> serde_json::Value {
    let profiles = model_profiles_from_json_row(row)
        .into_iter()
        .map(|mut profile| {
            if profile["normalized_cost_units"].as_u64().unwrap_or(0) == 0 {
                if let Some(cost) = fallback_rate {
                    profile["normalized_cost_units"] = serde_json::json!(cost);
                }
            }
            if profile["runtime_roles"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                profile["runtime_roles"] = serde_json::json!(fallback_runtime_roles);
            }
            if profile["task_classes"]
                .as_array()
                .map(|rows| rows.is_empty())
                .unwrap_or(true)
            {
                profile["task_classes"] = serde_json::json!(fallback_task_classes);
            }
            profile
        })
        .collect::<Vec<_>>();
    projection_from_profiles(
        row["default_model_profile"]
            .as_str()
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                row["current_model_profile"]
                    .as_str()
                    .map(str::to_string)
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                let model_ref = row["model"].as_str().unwrap_or_default();
                let reasoning_effort = row["model_reasoning_effort"].as_str().unwrap_or_default();
                if model_ref.is_empty() {
                    None
                } else {
                    Some(synthetic_profile_id(owner_id, model_ref, reasoning_effort))
                }
            }),
        profiles,
    )
}

pub(crate) fn model_profiles_from_json_row(row: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut profiles = row["model_profiles"]
        .as_object()
        .map(|entries| {
            entries
                .iter()
                .map(|(profile_id, profile)| {
                    let mut value = profile.clone();
                    if value.get("profile_id").is_none() {
                        value["profile_id"] = serde_json::json!(profile_id);
                    }
                    value
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if profiles.is_empty() {
        let model_ref = row["model"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(model_ref) = model_ref {
            let reasoning_effort = row["model_reasoning_effort"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_string();
            profiles.push(profile_with_defaults(
                &synthetic_profile_id(
                    row["role_id"].as_str().unwrap_or_default(),
                    model_ref,
                    &reasoning_effort,
                ),
                Some(model_ref.to_string()),
                row["model_provider"].as_str().map(str::to_string),
                Some(reasoning_effort),
                row["plan_mode_reasoning_effort"]
                    .as_str()
                    .map(str::to_string),
                row["sandbox_mode"].as_str().map(str::to_string),
                json_u64(row.get("normalized_cost_units")).or_else(|| json_u64(row.get("rate"))),
                row["speed_tier"].as_str().map(str::to_string),
                row["quality_tier"].as_str().map(str::to_string),
                row["write_scope"].as_str().map(str::to_string),
                row["runtime_roles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect(),
                row["task_classes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect(),
                row.get("readiness").cloned(),
                row.get("reasoning_control").cloned(),
            ));
        }
    }

    profiles.sort_by(|left, right| {
        left["profile_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["profile_id"].as_str().unwrap_or_default())
    });
    profiles
}

pub(crate) fn selected_model_profile_from_json_row(
    row: &serde_json::Value,
    preferred_profile_id: Option<&str>,
) -> Option<serde_json::Value> {
    let profiles = model_profiles_from_json_row(row);
    let preferred_profile_id = preferred_profile_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let default_profile_id = row["default_model_profile"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            row["current_model_profile"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        });
    preferred_profile_id
        .and_then(|profile_id| {
            profiles
                .iter()
                .find(|profile| profile["profile_id"].as_str() == Some(profile_id))
                .cloned()
        })
        .or_else(|| {
            default_profile_id.and_then(|profile_id| {
                profiles
                    .iter()
                    .find(|profile| profile["profile_id"].as_str() == Some(profile_id))
                    .cloned()
            })
        })
        .or_else(|| profiles.first().cloned())
}

pub(crate) fn apply_selected_model_profile_to_row(
    row: &serde_json::Value,
    preferred_profile_id: Option<&str>,
) -> serde_json::Value {
    let mut patched = row.clone();
    let Some(selected_profile) = selected_model_profile_from_json_row(row, preferred_profile_id)
    else {
        return patched;
    };
    let object = patched
        .as_object_mut()
        .expect("carrier/backend row should serialize to an object");
    object.insert(
        "selected_model_profile_id".to_string(),
        selected_profile["profile_id"].clone(),
    );
    object.insert("model".to_string(), selected_profile["model_ref"].clone());
    object.insert(
        "model_provider".to_string(),
        selected_profile["provider"].clone(),
    );
    object.insert(
        "model_reasoning_effort".to_string(),
        selected_profile["reasoning_effort"].clone(),
    );
    object.insert(
        "plan_mode_reasoning_effort".to_string(),
        selected_profile["plan_mode_reasoning_effort"].clone(),
    );
    object.insert(
        "sandbox_mode".to_string(),
        selected_profile["sandbox_mode"].clone(),
    );
    object.insert(
        "normalized_cost_units".to_string(),
        selected_profile["normalized_cost_units"].clone(),
    );
    object.insert(
        "speed_tier".to_string(),
        selected_profile["speed_tier"].clone(),
    );
    object.insert(
        "quality_tier".to_string(),
        selected_profile["quality_tier"].clone(),
    );
    object.insert(
        "write_scope".to_string(),
        selected_profile["write_scope"].clone(),
    );
    patched
}

#[cfg(test)]
mod tests {
    #[test]
    fn yaml_legacy_projection_synthesizes_default_profile() {
        let owner: serde_yaml::Value = serde_yaml::from_str(
            r#"
model: gpt-5.4
model_reasoning_effort: low
sandbox_mode: workspace-write
rate: 1
runtime_roles: [worker]
task_classes: [implementation]
"#,
        )
        .expect("yaml should parse");

        let projection = super::normalize_profile_projection_from_yaml(
            "junior",
            &owner,
            Some(1),
            &["worker".to_string()],
            &["implementation".to_string()],
        );

        assert_eq!(projection["model"], "gpt-5.4");
        assert_eq!(projection["model_provider"], serde_json::Value::Null);
        assert_eq!(projection["model_reasoning_effort"], "low");
        assert!(projection["default_model_profile"].as_str().is_some());
        assert_eq!(
            projection["model_profiles"]
                .as_object()
                .expect("profiles map")
                .len(),
            1
        );
    }

    #[test]
    fn yaml_legacy_projection_uses_provider_only_when_configured() {
        let owner: serde_yaml::Value = serde_yaml::from_str(
            r#"
model: configured-model
provider: configured-provider
model_reasoning_effort: low
"#,
        )
        .expect("yaml should parse");

        let projection =
            super::normalize_profile_projection_from_yaml("junior", &owner, None, &[], &[]);

        assert_eq!(projection["model"], "configured-model");
        assert_eq!(projection["model_provider"], "configured-provider");
    }

    #[test]
    fn apply_selected_model_profile_to_row_prefers_explicit_profile() {
        let row = serde_json::json!({
            "role_id": "architect",
            "default_model_profile": "codex_spark_high_arch",
            "model_profiles": {
                "codex_spark_high_arch": {
                    "profile_id": "codex_spark_high_arch",
                    "model_ref": "gpt-5.3-codex-spark",
                    "provider": "openai",
                    "reasoning_effort": "high",
                    "sandbox_mode": "read-only",
                    "normalized_cost_units": 32,
                    "runtime_roles": ["solution_architect"],
                    "task_classes": ["architecture"]
                },
                "codex_spark_xhigh_arch": {
                    "profile_id": "codex_spark_xhigh_arch",
                    "model_ref": "gpt-5.3-codex-spark",
                    "provider": "openai",
                    "reasoning_effort": "xhigh",
                    "sandbox_mode": "read-only",
                    "normalized_cost_units": 48,
                    "runtime_roles": ["solution_architect"],
                    "task_classes": ["hard_escalation"]
                }
            }
        });

        let patched =
            super::apply_selected_model_profile_to_row(&row, Some("codex_spark_xhigh_arch"));

        assert_eq!(
            patched["selected_model_profile_id"],
            "codex_spark_xhigh_arch"
        );
        assert_eq!(patched["model_reasoning_effort"], "xhigh");
        assert_eq!(patched["normalized_cost_units"], 48);
    }
}
