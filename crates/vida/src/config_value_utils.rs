use serde::{Deserialize, Deserializer};
use std::{fs, path::Path};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(crate) struct ProjectOverlayConfig {
    pub(crate) agent_extensions: AgentExtensionsOverlayConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(crate) struct AgentExtensionsOverlayConfig {
    #[serde(deserialize_with = "deserialize_yaml_string_list")]
    pub(crate) enabled_project_roles: Vec<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_list")]
    pub(crate) enabled_project_profiles: Vec<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_list")]
    pub(crate) enabled_project_flows: Vec<String>,
    pub(crate) registries: AgentExtensionRegistryPathConfig,
    pub(crate) validation: AgentExtensionValidationFlagConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(crate) struct AgentExtensionRegistryPathConfig {
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) roles: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) skills: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) profiles: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) flows: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) packs: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) commands: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_string_option")]
    pub(crate) dispatch_aliases: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(crate) struct AgentExtensionValidationFlagConfig {
    #[serde(deserialize_with = "deserialize_yaml_bool")]
    pub(crate) require_registry_files: bool,
    #[serde(deserialize_with = "deserialize_yaml_bool")]
    pub(crate) require_profile_resolution: bool,
    #[serde(deserialize_with = "deserialize_yaml_bool")]
    pub(crate) require_flow_resolution: bool,
    #[serde(deserialize_with = "deserialize_yaml_bool")]
    pub(crate) require_framework_role_compatibility: bool,
    #[serde(deserialize_with = "deserialize_yaml_bool")]
    pub(crate) require_skill_role_compatibility: bool,
}

pub(crate) fn project_overlay_config(value: &serde_yaml::Value) -> ProjectOverlayConfig {
    let agent_extensions = AgentExtensionsOverlayConfig {
        enabled_project_roles: yaml_string_list(yaml_lookup(
            value,
            &["agent_extensions", "enabled_project_roles"],
        )),
        enabled_project_profiles: yaml_string_list(yaml_lookup(
            value,
            &["agent_extensions", "enabled_project_profiles"],
        )),
        enabled_project_flows: yaml_string_list(yaml_lookup(
            value,
            &["agent_extensions", "enabled_project_flows"],
        )),
        registries: AgentExtensionRegistryPathConfig {
            roles: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "roles"],
            ))
            .filter(|value| !value.trim().is_empty()),
            skills: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "skills"],
            ))
            .filter(|value| !value.trim().is_empty()),
            profiles: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "profiles"],
            ))
            .filter(|value| !value.trim().is_empty()),
            flows: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "flows"],
            ))
            .filter(|value| !value.trim().is_empty()),
            packs: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "packs"],
            ))
            .filter(|value| !value.trim().is_empty()),
            commands: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "commands"],
            ))
            .filter(|value| !value.trim().is_empty()),
            dispatch_aliases: yaml_string(yaml_lookup(
                value,
                &["agent_extensions", "registries", "dispatch_aliases"],
            ))
            .filter(|value| !value.trim().is_empty()),
        },
        validation: AgentExtensionValidationFlagConfig {
            require_registry_files: yaml_bool(
                yaml_lookup(
                    value,
                    &["agent_extensions", "validation", "require_registry_files"],
                ),
                false,
            ),
            require_profile_resolution: yaml_bool(
                yaml_lookup(
                    value,
                    &[
                        "agent_extensions",
                        "validation",
                        "require_profile_resolution",
                    ],
                ),
                false,
            ),
            require_flow_resolution: yaml_bool(
                yaml_lookup(
                    value,
                    &["agent_extensions", "validation", "require_flow_resolution"],
                ),
                false,
            ),
            require_framework_role_compatibility: yaml_bool(
                yaml_lookup(
                    value,
                    &[
                        "agent_extensions",
                        "validation",
                        "require_framework_role_compatibility",
                    ],
                ),
                false,
            ),
            require_skill_role_compatibility: yaml_bool(
                yaml_lookup(
                    value,
                    &[
                        "agent_extensions",
                        "validation",
                        "require_skill_role_compatibility",
                    ],
                ),
                false,
            ),
        },
    };

    ProjectOverlayConfig { agent_extensions }
}

fn deserialize_yaml_string_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_yaml::Value::deserialize(deserializer)?;
    Ok(yaml_string_list(Some(&value)))
}

fn deserialize_yaml_string_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_yaml::Value::deserialize(deserializer)?;
    Ok(yaml_string(Some(&value)).filter(|value| !value.trim().is_empty()))
}

fn deserialize_yaml_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_yaml::Value::deserialize(deserializer)?;
    Ok(yaml_bool(Some(&value), false))
}

pub(crate) fn load_project_overlay_yaml() -> Result<serde_yaml::Value, String> {
    load_project_overlay_yaml_for_root(&super::resolve_runtime_project_root()?)
}

pub(crate) fn load_project_overlay_yaml_for_root(root: &Path) -> Result<serde_yaml::Value, String> {
    let path = root.join("vida.config.yaml");
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) fn json_lookup<'a>(
    value: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

pub(crate) fn json_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|node| match node {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    })
}

pub(crate) fn json_bool(value: Option<&serde_json::Value>, default: bool) -> bool {
    match value {
        Some(serde_json::Value::Bool(flag)) => *flag,
        Some(serde_json::Value::String(text)) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" | "1" => true,
            "false" | "no" | "off" | "0" => false,
            _ => default,
        },
        _ => default,
    }
}

pub(crate) fn json_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Some(serde_json::Value::String(text)) => split_csv_like(text),
        _ => Vec::new(),
    }
}

pub(crate) fn csv_json_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Some(serde_json::Value::String(text)) => split_csv_like(text),
        _ => Vec::new(),
    }
}

pub(crate) fn json_trimmed_string_field(
    value: &serde_json::Value,
    key: &str,
) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn json_trimmed_string_field_any(
    value: &serde_json::Value,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| json_trimmed_string_field(value, key))
}

pub(crate) fn json_nonempty_string_array_field(
    value: &serde_json::Value,
    key: &str,
) -> bool {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row.as_str()
                        .map(str::trim)
                        .is_some_and(|value| !value.is_empty())
                })
        })
}

pub(crate) fn canonical_json_string_array_entries(
    value: &serde_json::Value,
) -> Option<Vec<String>> {
    let rows = value.as_array()?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = row.as_str()?;
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed != entry {
            return None;
        }
        entries.push(trimmed.to_string());
    }
    Some(entries)
}

pub(crate) fn yaml_lookup<'a>(
    value: &'a serde_yaml::Value,
    path: &[&str],
) -> Option<&'a serde_yaml::Value> {
    let mut current = value;
    for segment in path {
        match current {
            serde_yaml::Value::Mapping(map) => {
                current = map.get(serde_yaml::Value::String((*segment).to_string()))?;
            }
            _ => return None,
        }
    }
    Some(current)
}

pub(crate) fn yaml_string(value: Option<&serde_yaml::Value>) -> Option<String> {
    value.and_then(|node| match node {
        serde_yaml::Value::String(text) => Some(text.clone()),
        serde_yaml::Value::Number(number) => Some(number.to_string()),
        serde_yaml::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    })
}

pub(crate) fn yaml_bool(value: Option<&serde_yaml::Value>, default: bool) -> bool {
    value
        .and_then(|node| match node {
            serde_yaml::Value::Bool(flag) => Some(*flag),
            serde_yaml::Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => Some(true),
                "false" | "no" | "off" | "0" => Some(false),
                _ => None,
            },
            serde_yaml::Value::Number(number) => number.as_i64().map(|value| value != 0),
            _ => None,
        })
        .unwrap_or(default)
}

pub(crate) fn split_csv_like(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
        .collect()
}

pub(crate) fn yaml_string_list(value: Option<&serde_yaml::Value>) -> Vec<String> {
    match value {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| match row {
                serde_yaml::Value::String(text) => Some(text.trim().to_string()),
                _ => None,
            })
            .filter(|row| !row.is_empty())
            .collect(),
        Some(serde_yaml::Value::String(text)) => split_csv_like(text),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_json_string_array_entries, json_nonempty_string_array_field,
        json_trimmed_string_field, json_trimmed_string_field_any, project_overlay_config,
    };

    #[test]
    fn typed_overlay_config_preserves_legacy_agent_extension_shapes() {
        let raw = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
agent_extensions:
  enabled_project_roles: analyst, developer
  enabled_project_profiles:
    - default
    - deep
  enabled_project_flows: release
  registries:
    roles: 123
    skills: ".agents/skills.yaml"
    profiles: ".agents/profiles.yaml"
    flows: ".agents/flows.yaml"
    dispatch_aliases: ".agents/dispatch-aliases.yaml"
  validation:
    require_registry_files: "yes"
    require_profile_resolution: 1
    require_flow_resolution: false
    require_framework_role_compatibility: "on"
    require_skill_role_compatibility: "0"
"#,
        )
        .expect("overlay yaml should parse");

        let config = project_overlay_config(&raw).agent_extensions;

        assert_eq!(config.enabled_project_roles, vec!["analyst", "developer"]);
        assert_eq!(config.enabled_project_profiles, vec!["default", "deep"]);
        assert_eq!(config.enabled_project_flows, vec!["release"]);
        assert_eq!(config.registries.roles.as_deref(), Some("123"));
        assert_eq!(
            config.registries.skills.as_deref(),
            Some(".agents/skills.yaml")
        );
        assert!(config.validation.require_registry_files);
        assert!(config.validation.require_profile_resolution);
        assert!(!config.validation.require_flow_resolution);
        assert!(config.validation.require_framework_role_compatibility);
        assert!(!config.validation.require_skill_role_compatibility);
    }

    #[test]
    fn typed_overlay_config_preserves_valid_security_fields_when_sibling_key_is_malformed() {
        let raw = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
agent_extensions:
  enabled_project_profiles:
    - missing_profile
  registries:
    roles: ".agents/roles.yaml"
    profiles: ".agents/profiles.yaml"
  validation:
    require_registry_files: true
    require_profile_resolution: true
    require_flow_resolution: true
    require_framework_role_compatibility: true
    require_skill_role_compatibility: true
  ? [unexpected, sequence, key]
  : ignored
"#,
        )
        .expect("overlay yaml should parse");

        let config = project_overlay_config(&raw).agent_extensions;

        assert_eq!(config.enabled_project_profiles, vec!["missing_profile"]);
        assert_eq!(
            config.registries.roles.as_deref(),
            Some(".agents/roles.yaml")
        );
        assert_eq!(
            config.registries.profiles.as_deref(),
            Some(".agents/profiles.yaml")
        );
        assert!(config.validation.require_registry_files);
        assert!(config.validation.require_profile_resolution);
        assert!(config.validation.require_flow_resolution);
        assert!(config.validation.require_framework_role_compatibility);
        assert!(config.validation.require_skill_role_compatibility);
    }

    #[test]
    fn typed_overlay_config_defaults_invalid_shapes() {
        let raw = serde_yaml::from_str::<serde_yaml::Value>(
            r#"
agent_extensions:
  enabled_project_roles:
    nested: value
  registries:
    roles: []
  validation:
    require_registry_files: maybe
"#,
        )
        .expect("overlay yaml should parse");

        let config = project_overlay_config(&raw).agent_extensions;

        assert!(config.enabled_project_roles.is_empty());
        assert_eq!(config.registries.roles, None);
        assert!(!config.validation.require_registry_files);
    }

    #[test]
    fn json_string_field_helper_returns_trimmed_nonempty_string() {
        let value = serde_json::json!({ "route": " developer " });

        assert_eq!(
            json_trimmed_string_field(&value, "route").as_deref(),
            Some("developer")
        );
    }

    #[test]
    fn json_string_field_helper_rejects_missing_non_string_and_blank() {
        let value = serde_json::json!({
            "blank": "   ",
            "number": 7,
            "null": null
        });

        assert_eq!(json_trimmed_string_field(&value, "missing"), None);
        assert_eq!(json_trimmed_string_field(&value, "blank"), None);
        assert_eq!(json_trimmed_string_field(&value, "number"), None);
        assert_eq!(json_trimmed_string_field(&value, "null"), None);
    }

    #[test]
    fn json_string_field_any_helper_uses_first_nonempty_matching_key() {
        let value = serde_json::json!({
            "selected_backend_id": "   ",
            "selected_backend": " internal_subagents "
        });

        assert_eq!(
            json_trimmed_string_field_any(
                &value,
                &["selected_backend_id", "selected_backend"]
            )
            .as_deref(),
            Some("internal_subagents")
        );
    }

    #[test]
    fn json_nonempty_string_array_field_accepts_packet_nonempty_string_array() {
        let packet = serde_json::json!({
            "owned_paths": ["crates/vida/src/main.rs", " docs/process "]
        });

        assert!(json_nonempty_string_array_field(&packet, "owned_paths"));
    }

    #[test]
    fn json_nonempty_string_array_field_rejects_packet_nonempty_string_array_gaps() {
        for packet in [
            serde_json::json!({}),
            serde_json::json!({ "owned_paths": [] }),
            serde_json::json!({ "owned_paths": ["   "] }),
            serde_json::json!({ "owned_paths": ["ok", 7] }),
        ] {
            assert!(!json_nonempty_string_array_field(&packet, "owned_paths"));
        }
    }

    #[test]
    fn canonical_json_string_array_entries_rejects_blank_or_trimmed_entries() {
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["pending"])),
            Some(vec!["pending".to_string()])
        );
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!([" pending "])),
            None
        );
        assert_eq!(
            canonical_json_string_array_entries(&serde_json::json!(["   "])),
            None
        );
    }
}
