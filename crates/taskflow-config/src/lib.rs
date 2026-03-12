use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBundleConfig {
    pub state_adapter: String,
    pub format_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskflowConfig {
    pub runtime_family: String,
    pub bundle: RuntimeBundleConfig,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TaskflowConfigError {
    #[error("taskflow config json decode failed: {0}")]
    Decode(String),
    #[error("runtime_family must not be empty")]
    EmptyRuntimeFamily,
    #[error("bundle.state_adapter must not be empty")]
    EmptyStateAdapter,
    #[error("bundle.format_profile must not be empty")]
    EmptyFormatProfile,
}

impl TaskflowConfig {
    pub fn validate(&self) -> Result<(), TaskflowConfigError> {
        if self.runtime_family.trim().is_empty() {
            return Err(TaskflowConfigError::EmptyRuntimeFamily);
        }
        if self.bundle.state_adapter.trim().is_empty() {
            return Err(TaskflowConfigError::EmptyStateAdapter);
        }
        if self.bundle.format_profile.trim().is_empty() {
            return Err(TaskflowConfigError::EmptyFormatProfile);
        }
        Ok(())
    }
}

pub fn load_from_json_str(input: &str) -> Result<TaskflowConfig, TaskflowConfigError> {
    let config: TaskflowConfig =
        serde_json::from_str(input).map_err(|err| TaskflowConfigError::Decode(err.to_string()))?;
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::{TaskflowConfigError, load_from_json_str};

    #[test]
    fn loads_valid_taskflow_config() {
        let config = load_from_json_str(
            r#"{
                "runtime_family": "taskflow",
                "bundle": {
                    "state_adapter": "surreal",
                    "format_profile": "canonical"
                }
            }"#,
        )
        .expect("config should load");

        assert_eq!(config.runtime_family, "taskflow");
        assert_eq!(config.bundle.state_adapter, "surreal");
    }

    #[test]
    fn rejects_empty_runtime_family() {
        let error = load_from_json_str(
            r#"{
                "runtime_family": "",
                "bundle": {
                    "state_adapter": "surreal",
                    "format_profile": "canonical"
                }
            }"#,
        )
        .expect_err("config should fail");

        assert_eq!(error, TaskflowConfigError::EmptyRuntimeFamily);
    }
}
