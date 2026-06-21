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
    serde_yaml::from_value(value.clone()).unwrap_or_default()
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
    use super::project_overlay_config;

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
}
