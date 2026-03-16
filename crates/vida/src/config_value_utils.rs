use std::fs;

pub(crate) fn load_project_overlay_yaml() -> Result<serde_yaml::Value, String> {
    let path = super::resolve_runtime_project_root()?.join("vida.config.yaml");
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
