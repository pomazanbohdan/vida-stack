use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) fn non_empty_yaml_string(config: &serde_yaml::Value, path: &[&str]) -> Option<String> {
    crate::yaml_string(crate::yaml_lookup(config, path)).filter(|value| !value.trim().is_empty())
}

pub(crate) fn read_simple_toml_sections(path: &Path) -> HashMap<String, HashMap<String, String>> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(document) = raw.parse::<toml_edit::DocumentMut>() else {
        return HashMap::new();
    };
    let mut sections = HashMap::<String, HashMap<String, String>>::new();
    collect_toml_sections("", document.as_table(), &mut sections);
    sections
}

fn collect_toml_sections(
    section: &str,
    table: &toml_edit::Table,
    sections: &mut HashMap<String, HashMap<String, String>>,
) {
    for (key, item) in table.iter() {
        if let Some(value) = item.as_value().and_then(toml_scalar_string) {
            sections
                .entry(section.to_string())
                .or_default()
                .insert(key.to_string(), value);
            continue;
        }
        if let Some(child) = item.as_table() {
            let child_section = if section.is_empty() {
                key.to_string()
            } else {
                format!("{section}.{key}")
            };
            sections.entry(child_section.clone()).or_default();
            collect_toml_sections(&child_section, child, sections);
        }
    }
}

fn toml_scalar_string(value: &toml_edit::Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }
    if let Some(value) = value.as_bool() {
        return Some(value.to_string());
    }
    if let Some(value) = value.as_integer() {
        return Some(value.to_string());
    }
    if let Some(value) = value.as_float() {
        return Some(value.to_string());
    }
    None
}

pub(crate) fn registry_rows_by_key(
    registry: &serde_yaml::Value,
    key: &str,
    id_field: &str,
    enabled_ids: &[String],
) -> Vec<serde_json::Value> {
    let enabled = enabled_ids.iter().cloned().collect::<HashSet<_>>();
    match crate::yaml_lookup(registry, &[key]) {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| {
                let row_id = crate::yaml_string(crate::yaml_lookup(row, &[id_field]))?;
                if !enabled.is_empty() && !enabled.contains(&row_id) {
                    return None;
                }
                serde_json::to_value(row).ok()
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn registry_all_ids_by_key(registry: &serde_yaml::Value, key: &str, id_field: &str) -> Vec<String> {
    match crate::yaml_lookup(registry, &[key]) {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| crate::yaml_string(crate::yaml_lookup(row, &[id_field])))
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn effective_enabled_registry_ids(
    config: &serde_yaml::Value,
    config_path: &[&str],
    registry: &serde_yaml::Value,
    registry_key: &str,
    id_field: &str,
) -> Vec<String> {
    if crate::yaml_lookup(config, config_path).is_some() {
        return crate::yaml_string_list(crate::yaml_lookup(config, config_path));
    }
    registry_all_ids_by_key(registry, registry_key, id_field)
}

pub(crate) fn registry_row_map_by_id(
    rows: &[serde_json::Value],
    id_field: &str,
) -> HashMap<String, serde_json::Value> {
    rows.iter()
        .filter_map(|row| Some((row[id_field].as_str()?.to_string(), row.clone())))
        .collect()
}

pub(crate) fn registry_ids_by_key(
    registry: &serde_yaml::Value,
    key: &str,
    id_field: &str,
) -> HashSet<String> {
    match crate::yaml_lookup(registry, &[key]) {
        Some(serde_yaml::Value::Sequence(rows)) => rows
            .iter()
            .filter_map(|row| crate::yaml_string(crate::yaml_lookup(row, &[id_field])))
            .collect(),
        _ => HashSet::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::temp_state::TempStateHarness;

    use super::read_simple_toml_sections;

    #[test]
    fn read_simple_toml_sections_uses_toml_edit_for_scalars_and_comments() {
        let temp = TempStateHarness::new().expect("temp root should initialize");
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
# preserved comment
root_key = "root value"

[features]
multi_agent = true

[agents]
max_threads = 4
max_depth = "2"
"#,
        )
        .expect("config should write");

        let sections = read_simple_toml_sections(&path);

        assert_eq!(sections[""]["root_key"], "root value");
        assert_eq!(sections["features"]["multi_agent"], "true");
        assert_eq!(sections["agents"]["max_threads"], "4");
        assert_eq!(sections["agents"]["max_depth"], "2");
    }

    #[test]
    fn read_simple_toml_sections_flattens_nested_tables() {
        let temp = TempStateHarness::new().expect("temp root should initialize");
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
[agents.worker]
model = "gpt"
"#,
        )
        .expect("config should write");

        let sections = read_simple_toml_sections(&path);

        assert_eq!(sections["agents.worker"]["model"], "gpt");
    }
}
