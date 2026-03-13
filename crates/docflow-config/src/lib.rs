use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSurfaceConfig {
    pub output_format: String,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocflowConfig {
    pub runtime_family: String,
    pub operator: OperatorSurfaceConfig,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DocflowConfigError {
    #[error("docflow config json decode failed: {0}")]
    Decode(String),
    #[error("runtime_family must not be empty")]
    EmptyRuntimeFamily,
    #[error("operator.output_format must not be empty")]
    EmptyOutputFormat,
    #[error("operator.profile must not be empty")]
    EmptyProfile,
    #[error("profile `{0}` not found in policy")]
    ProfileNotFound(String),
    #[error("policy parse failed: {0}")]
    PolicyParse(String),
    #[error("policy path is missing: {0}")]
    MissingPolicyPath(String),
}

impl DocflowConfig {
    pub fn validate(&self) -> Result<(), DocflowConfigError> {
        if self.runtime_family.trim().is_empty() {
            return Err(DocflowConfigError::EmptyRuntimeFamily);
        }
        if self.operator.output_format.trim().is_empty() {
            return Err(DocflowConfigError::EmptyOutputFormat);
        }
        if self.operator.profile.trim().is_empty() {
            return Err(DocflowConfigError::EmptyProfile);
        }
        Ok(())
    }
}

pub fn load_from_json_str(input: &str) -> Result<DocflowConfig, DocflowConfigError> {
    let config: DocflowConfig =
        serde_json::from_str(input).map_err(|err| DocflowConfigError::Decode(err.to_string()))?;
    config.validate()?;
    Ok(config)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyProfile {
    pub roots: Vec<String>,
    pub scan_ignored_globs: Vec<String>,
}

pub fn load_policy_profile(
    policy_path: &Path,
    profile: &str,
) -> Result<PolicyProfile, DocflowConfigError> {
    if !policy_path.exists() {
        return Err(DocflowConfigError::MissingPolicyPath(
            policy_path.display().to_string(),
        ));
    }

    let text = std::fs::read_to_string(policy_path)
        .map_err(|err| DocflowConfigError::PolicyParse(err.to_string()))?;
    let parsed = parse_policy_profiles(&text)?;
    let roots = parsed
        .profiles
        .get(profile)
        .cloned()
        .ok_or_else(|| DocflowConfigError::ProfileNotFound(profile.to_string()))?;
    Ok(PolicyProfile {
        roots,
        scan_ignored_globs: parsed.scan_ignored_globs,
    })
}

#[derive(Debug, Default)]
struct ParsedPolicy {
    profiles: BTreeMap<String, Vec<String>>,
    scan_ignored_globs: Vec<String>,
}

fn parse_policy_profiles(input: &str) -> Result<ParsedPolicy, DocflowConfigError> {
    let mut parsed = ParsedPolicy::default();
    let mut in_profiles = false;
    let mut current_profile: Option<String> = None;
    let mut in_scan_ignored = false;
    let mut pending_scan_ignored_scope = false;

    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !line.starts_with(' ') && trimmed == "profiles:" {
            in_profiles = true;
            in_scan_ignored = false;
            current_profile = None;
            continue;
        }
        if !line.starts_with(' ') && trimmed == "scan_ignored:" {
            in_scan_ignored = true;
            in_profiles = false;
            current_profile = None;
            continue;
        }
        if !line.starts_with(' ') {
            in_profiles = false;
            in_scan_ignored = false;
            current_profile = None;
        }

        if in_scan_ignored {
            let indent = line.len() - line.trim_start().len();
            if indent == 2 && trimmed.starts_with("- scope:") {
                pending_scan_ignored_scope = trimmed.ends_with("relative_path");
                continue;
            }
            if indent == 4 && trimmed.starts_with("glob:") && pending_scan_ignored_scope {
                if let Some(value) = trimmed.split_once(':').map(|(_, value)| value.trim()) {
                    parsed
                        .scan_ignored_globs
                        .push(value.trim_matches('"').to_string());
                }
                continue;
            }
        }

        if in_profiles {
            let indent = line.len() - line.trim_start().len();
            if indent == 2 && trimmed.ends_with(':') {
                let profile = trimmed.trim_end_matches(':').to_string();
                parsed.profiles.entry(profile.clone()).or_default();
                current_profile = Some(profile);
                continue;
            }
            if indent == 4 && trimmed.starts_with("- ") {
                if let Some(profile) = &current_profile {
                    parsed
                        .profiles
                        .entry(profile.clone())
                        .or_default()
                        .push(trimmed.trim_start_matches("- ").to_string());
                } else {
                    return Err(DocflowConfigError::PolicyParse(
                        "profile entry encountered before profile header".into(),
                    ));
                }
            }
        }
    }

    Ok(parsed)
}

pub fn resolve_profile_roots(
    root: Option<&Path>,
    policy_path: &Path,
    profile: &str,
) -> Result<Vec<PathBuf>, DocflowConfigError> {
    let loaded = load_policy_profile(policy_path, profile)?;
    let root = root.unwrap_or_else(|| policy_path.parent().unwrap_or_else(|| Path::new(".")));
    Ok(loaded
        .roots
        .into_iter()
        .map(|item| root.join(item))
        .collect())
}

pub fn resolve_scan_ignored_globs(policy_path: &Path) -> Result<Vec<String>, DocflowConfigError> {
    Ok(load_policy_profile(policy_path, "active-canon")
        .map(|profile| profile.scan_ignored_globs)
        .or_else(|_| {
            let text = std::fs::read_to_string(policy_path)
                .map_err(|err| DocflowConfigError::PolicyParse(err.to_string()))?;
            Ok(parse_policy_profiles(&text)?.scan_ignored_globs)
        })?)
}

#[cfg(test)]
mod tests {
    use super::{
        DocflowConfigError, load_from_json_str, load_policy_profile, resolve_profile_roots,
    };
    use std::fs;

    #[test]
    fn loads_valid_docflow_config() {
        let config = load_from_json_str(
            r#"{
                "runtime_family": "docflow",
                "operator": {
                    "output_format": "toon",
                    "profile": "active-canon"
                }
            }"#,
        )
        .expect("config should load");

        assert_eq!(config.runtime_family, "docflow");
        assert_eq!(config.operator.output_format, "toon");
    }

    #[test]
    fn rejects_empty_output_format() {
        let error = load_from_json_str(
            r#"{
                "runtime_family": "docflow",
                "operator": {
                    "output_format": "",
                    "profile": "active-canon"
                }
            }"#,
        )
        .expect_err("config should fail");

        assert_eq!(error, DocflowConfigError::EmptyOutputFormat);
    }

    #[test]
    fn loads_profile_roots_and_scan_ignored_from_policy_subset() {
        let temp = std::env::temp_dir().join("docflow-config-policy-test.yaml");
        fs::write(
            &temp,
            r#"schema_version: 1
scan_ignored:
  - scope: relative_path
    glob: "target/**"
profiles:
  active-canon:
    - docs/process
    - docs/product
"#,
        )
        .expect("policy should be written");

        let profile = load_policy_profile(&temp, "active-canon").expect("profile should load");
        assert_eq!(profile.roots, vec!["docs/process", "docs/product"]);
        assert_eq!(profile.scan_ignored_globs, vec!["target/**"]);

        let roots =
            resolve_profile_roots(Some(std::path::Path::new("/repo")), &temp, "active-canon")
                .expect("profile roots should resolve");
        assert_eq!(roots[0], std::path::PathBuf::from("/repo/docs/process"));

        fs::remove_file(temp).expect("temp policy should be removed");
    }
}
