//! Attempt tracking command helpers.

pub const STAGE_ATTEMPT_SCHEMA_VERSION: &str = "stage-attempt-v1";

#[must_use]
pub fn normalize_artifact_refs(values: &[String]) -> Vec<String> {
    let mut refs = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty() && !refs.iter().any(|existing| existing == value) {
            refs.push(value.to_string());
        }
    }
    refs
}

pub fn validate_attempt_artifact_refs(values: &[String]) -> Result<Vec<String>, String> {
    let refs = normalize_artifact_refs(values);
    if refs.is_empty() {
        Err("attempt_artifact_refs_missing".to_string())
    } else {
        Ok(refs)
    }
}

pub fn validate_stage_attempt_artifact_identity(
    json: &serde_json::Value,
    attempt_id: &str,
    task_id: &str,
    stage_id: &str,
    artifact_ref: &str,
) -> Result<(), String> {
    for (field, expected) in [
        ("schema_version", STAGE_ATTEMPT_SCHEMA_VERSION),
        ("attempt_id", attempt_id),
        ("task_id", task_id),
        ("stage_id", stage_id),
    ] {
        let actual = json[field].as_str().unwrap_or("");
        if actual != expected {
            return Err(format!(
                "attempt artifact `{artifact_ref}` field `{field}` expected `{expected}`, got `{actual}`"
            ));
        }
    }
    let has_fact_array =
        json["observed_facts"].as_array().is_some() || json["facts"].as_array().is_some();
    if !has_fact_array {
        return Err(format!(
            "attempt artifact `{artifact_ref}` must include observed_facts or facts array"
        ));
    }
    for field in [
        "observed_facts",
        "facts",
        "hypotheses",
        "proof_results",
        "risks",
        "limitations",
        "conflicts",
    ] {
        if !json[field].is_null() && !json[field].is_array() {
            return Err(format!(
                "attempt artifact `{artifact_ref}` field `{field}` must be an array"
            ));
        }
    }
    Ok(())
}

pub fn validate_attempt_artifact_changed_files_scope(
    json: &serde_json::Value,
    artifact_ref: &str,
    owned_paths: &[String],
) -> Result<(), String> {
    let Some(changed_files) = json["changed_files"].as_array() else {
        return Ok(());
    };
    let normalized_owned_paths = owned_paths
        .iter()
        .filter_map(|path| normalize_attempt_artifact_repo_path(path))
        .collect::<Vec<_>>();
    if normalized_owned_paths.is_empty() && !changed_files.is_empty() {
        return Err(format!(
            "attempt artifact `{artifact_ref}` changed_files require task owned_paths"
        ));
    }
    for changed_file in changed_files {
        let Some(changed_file) = changed_file.as_str() else {
            return Err(format!(
                "attempt artifact `{artifact_ref}` changed_files entries must be strings"
            ));
        };
        let Some(changed_file) = normalize_attempt_artifact_repo_path(changed_file) else {
            return Err(format!(
                "attempt artifact `{artifact_ref}` changed_files entry `{changed_file}` must be a relative repository path without parent traversal"
            ));
        };
        if !normalized_owned_paths
            .iter()
            .any(|owned_path| attempt_artifact_path_is_owned(&changed_file, owned_path))
        {
            return Err(format!(
                "attempt artifact `{artifact_ref}` changed file `{changed_file}` is outside task owned_paths"
            ));
        }
    }
    Ok(())
}

#[must_use]
pub fn normalize_attempt_artifact_repo_path(path: &str) -> Option<String> {
    let normalized = path.trim().replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return None;
    }
    Some(normalized.to_string())
}

#[must_use]
pub fn attempt_artifact_path_is_owned(changed_file: &str, owned_path: &str) -> bool {
    changed_file == owned_path || changed_file.starts_with(&format!("{owned_path}/"))
}

pub fn merge_repeated_values(target: &mut Vec<String>, values: &[String]) {
    for value in normalize_artifact_refs(values) {
        push_unique(target, &value);
    }
}

pub fn append_json_string_array(json: &serde_json::Value, keys: &[&str], values: &mut Vec<String>) {
    for key in keys {
        for value in json[*key].as_array().into_iter().flatten() {
            if let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                push_unique(values, value);
            }
        }
    }
}

pub fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        append_json_string_array, normalize_artifact_refs, normalize_attempt_artifact_repo_path,
        push_unique, validate_attempt_artifact_changed_files_scope, validate_attempt_artifact_refs,
        validate_stage_attempt_artifact_identity,
    };

    #[test]
    fn attempt_artifact_refs_are_trimmed_deduped_and_required() {
        let refs = normalize_artifact_refs(&[
            " artifact-a.json ".to_string(),
            "".to_string(),
            "artifact-a.json".to_string(),
            "artifact-b.json".to_string(),
        ]);

        assert_eq!(refs, ["artifact-a.json", "artifact-b.json"]);
        assert_eq!(
            validate_attempt_artifact_refs(&["  ".to_string()]).unwrap_err(),
            "attempt_artifact_refs_missing"
        );
    }

    #[test]
    fn attempt_artifact_identity_requires_schema_and_fact_arrays() {
        let valid = json!({
            "schema_version": "stage-attempt-v1",
            "attempt_id": "attempt-a",
            "task_id": "task-a",
            "stage_id": "analysis",
            "observed_facts": ["fact"],
            "hypotheses": []
        });

        assert!(
            validate_stage_attempt_artifact_identity(
                &valid,
                "attempt-a",
                "task-a",
                "analysis",
                "artifact-a.json"
            )
            .is_ok()
        );

        let invalid = json!({
            "schema_version": "stage-attempt-v1",
            "attempt_id": "attempt-a",
            "task_id": "task-a",
            "stage_id": "analysis",
            "observed_facts": "fact"
        });

        assert!(
            validate_stage_attempt_artifact_identity(
                &invalid,
                "attempt-a",
                "task-a",
                "analysis",
                "artifact-a.json"
            )
            .unwrap_err()
            .contains("observed_facts")
        );
    }

    #[test]
    fn changed_files_must_stay_within_owned_paths() {
        let artifact = json!({
            "changed_files": [
                "crates/taskflow-core/src/task/attempts.rs",
                "crates/vida/src/task_surface.rs"
            ]
        });
        let owned_paths = vec![
            "crates/taskflow-core/src/task".to_string(),
            "crates/vida/src/task_surface.rs".to_string(),
        ];

        assert!(
            validate_attempt_artifact_changed_files_scope(
                &artifact,
                "artifact-a.json",
                &owned_paths
            )
            .is_ok()
        );

        let outside = json!({"changed_files": ["crates/vida/src/other.rs"]});
        assert!(
            validate_attempt_artifact_changed_files_scope(
                &outside,
                "artifact-a.json",
                &owned_paths
            )
            .unwrap_err()
            .contains("outside task owned_paths")
        );
    }

    #[test]
    fn repo_paths_reject_absolute_empty_and_traversal_segments() {
        assert_eq!(
            normalize_attempt_artifact_repo_path("./crates/vida/src/task_surface.rs").as_deref(),
            Some("crates/vida/src/task_surface.rs")
        );
        assert!(normalize_attempt_artifact_repo_path("/tmp/file").is_none());
        assert!(normalize_attempt_artifact_repo_path("crates/../vida").is_none());
        assert!(normalize_attempt_artifact_repo_path("crates//vida").is_none());
    }

    #[test]
    fn json_string_arrays_and_push_unique_dedupe_values() {
        let json = json!({
            "observed_facts": [" fact-a ", "", "fact-a"],
            "facts": ["fact-b"]
        });
        let mut values = Vec::new();

        append_json_string_array(&json, &["observed_facts", "facts"], &mut values);
        push_unique(&mut values, "fact-b");

        assert_eq!(values, ["fact-a", "fact-b"]);
    }
}
