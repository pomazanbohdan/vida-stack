use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use taskflow_core::role_step::{RoleStepDefinition, RoleStepFlowDefinition};
use thiserror::Error;

pub const MODULE: &str = "role_step";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledFlowDefinition {
    pub work_item_bindings: Vec<String>,
    pub flow: RoleStepFlowDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledFlowStep {
    pub position: usize,
    pub role_label: String,
    pub runtime_role: String,
    pub task_class: String,
    pub proof_gate: Option<String>,
    pub packet_template_kind: Option<String>,
    pub requires_user_approval: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RoleStepConfigError {
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
}

impl RoleStepConfigError {
    fn invalid(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            path: path.into(),
            message: message.into(),
        }
    }
}

pub fn compile_all_dev_team_flows(
    readiness: &serde_json::Value,
) -> Result<Vec<CompiledFlowDefinition>, RoleStepConfigError> {
    let roles = role_index(readiness)?;
    let flows = collection_entries(&readiness["flows"], "$.flows", "dev_team flows")?;
    let mut compiled = Vec::new();
    let mut seen = BTreeSet::new();
    for (path, flow_id_hint, flow) in &flows {
        let flow_id = required_string_or_hint(
            flow,
            "flow_id",
            format!("{path}.flow_id"),
            flow_id_hint.as_deref(),
        )?;
        if !seen.insert(flow_id.clone()) {
            return Err(RoleStepConfigError::invalid(
                format!("{path}.flow_id"),
                format!("duplicate flow id `{flow_id}`"),
            ));
        }
        compiled.push(compile_flow(flow, flow_id, &roles, path)?);
    }
    if compiled.is_empty() {
        return Err(RoleStepConfigError::invalid(
            "$.flows",
            "at least one dev_team flow is required",
        ));
    }
    Ok(compiled)
}

pub fn compile_dev_team_flow_for_work_item(
    readiness: &serde_json::Value,
    work_item_type: &str,
) -> Result<CompiledFlowDefinition, RoleStepConfigError> {
    let flows = compile_all_dev_team_flows(readiness)?;
    let lookup = normalize_key(work_item_type);
    if !lookup.is_empty() {
        if let Some(flow) = flows.iter().find(|flow| {
            flow.work_item_bindings
                .iter()
                .any(|binding| normalize_key(binding) == lookup)
        }) {
            return Ok(flow.clone());
        }
    }
    let default_id = readiness["default_flow_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(default_id) = default_id {
        if let Some(flow) = flows.iter().find(|flow| flow.flow.flow_id == default_id) {
            return Ok(flow.clone());
        }
        return Err(RoleStepConfigError::invalid(
            "$.default_flow_id",
            format!("default flow `{default_id}` is not defined"),
        ));
    }
    Ok(flows[0].clone())
}

fn compile_flow(
    flow: &serde_json::Value,
    flow_id: String,
    roles: &BTreeMap<String, serde_json::Value>,
    flow_path: &str,
) -> Result<CompiledFlowDefinition, RoleStepConfigError> {
    let steps_value = flow
        .get("ordered_steps")
        .or_else(|| flow.get("steps"))
        .unwrap_or(&serde_json::Value::Null);
    let steps = steps_value.as_array().ok_or_else(|| {
        RoleStepConfigError::invalid(format!("{flow_path}.steps"), "steps must be an array")
    })?;
    if steps.is_empty() {
        return Err(RoleStepConfigError::invalid(
            format!("{flow_path}.steps"),
            "steps must not be empty",
        ));
    }
    let compiled_steps = steps
        .iter()
        .enumerate()
        .map(|(step_index, step)| compile_step(step, roles, flow_path, step_index))
        .collect::<Result<Vec<_>, _>>()?;
    let work_item_bindings = work_item_bindings(&flow["work_item_bindings"]);
    let schema_hash = flow_schema_hash(&flow_id, &work_item_bindings, &compiled_steps);
    Ok(CompiledFlowDefinition {
        work_item_bindings,
        flow: RoleStepFlowDefinition {
            flow_id,
            schema_hash,
            steps: compiled_steps
                .into_iter()
                .map(CompiledFlowStep::into_role_step_definition)
                .collect(),
        },
    })
}

fn compile_step(
    step: &serde_json::Value,
    roles: &BTreeMap<String, serde_json::Value>,
    flow_path: &str,
    step_index: usize,
) -> Result<CompiledFlowStep, RoleStepConfigError> {
    let role_path = format!("{flow_path}.steps[{step_index}].role_id");
    let role_label = required_string(step, "role_id", &role_path)?;
    let role = roles.get(&role_label).ok_or_else(|| {
        RoleStepConfigError::invalid(role_path, format!("role `{role_label}` is not configured"))
    })?;
    let runtime_role =
        string_field(step, "runtime_role").or_else(|| string_field(role, "runtime_role"));
    let runtime_role = runtime_role.ok_or_else(|| {
        RoleStepConfigError::invalid(
            format!("{flow_path}.steps[{step_index}].runtime_role"),
            format!("role `{role_label}` has no runtime_role"),
        )
    })?;
    let task_class = string_field(step, "task_class")
        .or_else(|| first_string(&role["task_classes"]))
        .ok_or_else(|| {
            RoleStepConfigError::invalid(
                format!("{flow_path}.steps[{step_index}].task_class"),
                format!("role `{role_label}` has no task_class"),
            )
        })?;
    Ok(CompiledFlowStep {
        position: step_index,
        role_label,
        runtime_role,
        task_class,
        proof_gate: string_field(step, "proof_gate").or_else(|| string_field(role, "proof_gate")),
        packet_template_kind: string_field(step, "packet_template_kind")
            .or_else(|| string_field(role, "packet_template_kind")),
        requires_user_approval: step["requires_user_approval"].as_bool().unwrap_or(false),
    })
}

impl CompiledFlowStep {
    fn into_role_step_definition(self) -> RoleStepDefinition {
        RoleStepDefinition {
            role_id: self.role_label,
            runtime_role: self.runtime_role,
            task_class: self.task_class,
            lifecycle_stage: format!("step_{}", self.position),
            proof_gate: self.proof_gate,
            closes_workflow: false,
        }
    }
}

fn role_index(
    readiness: &serde_json::Value,
) -> Result<BTreeMap<String, serde_json::Value>, RoleStepConfigError> {
    let roles = collection_entries(&readiness["roles"], "$.roles", "dev_team roles")?;
    let mut index = BTreeMap::new();
    for (role_path, role_id_hint, role) in roles {
        let role_id = required_string_or_hint(
            role,
            "role_id",
            format!("{role_path}.role_id"),
            role_id_hint.as_deref(),
        )?;
        if index.insert(role_id.clone(), role.clone()).is_some() {
            return Err(RoleStepConfigError::invalid(
                format!("{role_path}.role_id"),
                format!("duplicate role id `{role_id}`"),
            ));
        }
    }
    Ok(index)
}

fn collection_entries<'a>(
    value: &'a serde_json::Value,
    path: &str,
    label: &str,
) -> Result<Vec<(String, Option<String>, &'a serde_json::Value)>, RoleStepConfigError> {
    match value {
        serde_json::Value::Array(values) => Ok(values
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("{path}[{index}]"), None, value))
            .collect()),
        serde_json::Value::Object(values) => Ok(values
            .iter()
            .map(|(key, value)| (format!("{path}.{key}"), Some(key.clone()), value))
            .collect()),
        _ => Err(RoleStepConfigError::invalid(
            path,
            format!("{label} must be an array or mapping"),
        )),
    }
}

fn required_string(
    value: &serde_json::Value,
    key: &str,
    path: impl Into<String>,
) -> Result<String, RoleStepConfigError> {
    string_field(value, key).ok_or_else(|| {
        RoleStepConfigError::invalid(path, format!("{key} must be a non-empty string"))
    })
}

fn required_string_or_hint(
    value: &serde_json::Value,
    key: &str,
    path: impl Into<String>,
    hint: Option<&str>,
) -> Result<String, RoleStepConfigError> {
    string_field(value, key)
        .or_else(|| hint.map(str::to_string))
        .ok_or_else(|| {
            RoleStepConfigError::invalid(path, format!("{key} must be a non-empty string"))
        })
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn first_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn work_item_bindings(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(value) => split_bindings(value).collect(),
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .flat_map(split_bindings)
            .collect(),
        _ => Vec::new(),
    }
}

fn split_bindings(value: &str) -> impl Iterator<Item = String> + '_ {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn flow_schema_hash(flow_id: &str, bindings: &[String], steps: &[CompiledFlowStep]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in flow_id
        .bytes()
        .chain(bindings.iter().flat_map(|value| value.bytes()))
        .chain(steps.iter().flat_map(|step| {
            step.role_label
                .bytes()
                .chain(step.runtime_role.bytes())
                .chain(step.task_class.bytes())
        }))
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("flow-fnv64-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    fn readiness_fixture() -> serde_json::Value {
        serde_json::json!({
            "default_flow_id": "task",
            "roles": [
                {"role_id": "analyst", "runtime_role": "implementation", "task_classes": ["analysis"]},
                {"role_id": "developer", "runtime_role": "implementation", "task_classes": ["implementation"], "packet_template_kind": "delivery_task_packet"},
                {"role_id": "tester", "runtime_role": "verification", "task_classes": ["verification"], "proof_gate": "test_report"},
                {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]},
                {"role_id": "closer", "runtime_role": "prover", "task_classes": ["closure"]},
                {"role_id": "specifier", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "coder", "runtime_role": "worker", "task_classes": ["implementation"], "packet_template_kind": "delivery_task_packet"},
                {"role_id": "refactorer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "flows": [
                {"flow_id": "task", "work_item_bindings": ["task"], "ordered_steps": [
                    {"role_id": "analyst"}, {"role_id": "developer"}, {"role_id": "tester"}, {"role_id": "closer"}
                ]},
                {"flow_id": "defect", "work_item_bindings": ["defect", "bug"], "ordered_steps": [
                    {"role_id": "analyst"}, {"role_id": "developer"}, {"role_id": "tester"}
                ]},
                {"flow_id": "runtime_defect_remediation", "work_item_bindings": ["runtime_defect"], "ordered_steps": [
                    {"role_id": "specifier"},
                    {"role_id": "coder", "requires_user_approval": true},
                    {"role_id": "refactorer"},
                    {"role_id": "architect"}
                ]},
                {"flow_id": "architecture", "work_item_bindings": ["architecture"], "ordered_steps": [
                    {"role_id": "architect"}, {"role_id": "developer"}, {"role_id": "tester"}, {"role_id": "closer"}
                ]}
            ]
        })
    }

    #[test]
    fn compiles_all_repository_flow_fixtures() {
        let flows = compile_all_dev_team_flows(&readiness_fixture()).expect("flows compile");

        assert_eq!(flows.len(), 4);
        for flow_id in [
            "task",
            "defect",
            "runtime_defect_remediation",
            "architecture",
        ] {
            let flow = flows
                .iter()
                .find(|flow| flow.flow.flow_id == flow_id)
                .unwrap();
            assert!(!flow.flow.schema_hash.is_empty());
            assert!(!flow.flow.steps.is_empty());
        }
    }

    #[test]
    fn compiles_configured_dev_team_flows_from_project_config() {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("vida.config.yaml");
        let config_text = fs::read_to_string(&config_path).expect("project config should exist");
        let config_yaml: serde_yaml::Value =
            serde_yaml::from_str(&config_text).expect("project config should parse");
        let config_json = serde_json::to_value(config_yaml).expect("yaml should convert to json");
        let readiness = config_json
            .get("dev_team")
            .expect("project config should define dev_team");

        let flows = compile_all_dev_team_flows(readiness).expect("configured flows compile");
        assert!(flows.len() >= 10);
        for flow_id in [
            "task_delivery_verified",
            "defect_repair_verified",
            "runtime_defect_remediation",
            "architecture_design",
        ] {
            let flow = flows
                .iter()
                .find(|flow| flow.flow.flow_id == flow_id)
                .unwrap_or_else(|| panic!("configured flow `{flow_id}` should compile"));
            assert!(!flow.flow.schema_hash.is_empty());
            assert!(!flow.flow.steps.is_empty());
        }
        assert_eq!(
            compile_dev_team_flow_for_work_item(readiness, "task")
                .unwrap()
                .flow
                .flow_id,
            "adaptive-task-flow"
        );
        assert_eq!(
            compile_dev_team_flow_for_work_item(readiness, "runtime-defect")
                .unwrap()
                .flow
                .flow_id,
            "adaptive-task-flow"
        );
    }

    #[test]
    fn selects_work_item_flow_or_default() {
        let readiness = readiness_fixture();

        assert_eq!(
            compile_dev_team_flow_for_work_item(&readiness, "runtime-defect")
                .unwrap()
                .flow
                .flow_id,
            "runtime_defect_remediation"
        );
        assert_eq!(
            compile_dev_team_flow_for_work_item(&readiness, "unknown")
                .unwrap()
                .flow
                .flow_id,
            "task"
        );
    }

    #[test]
    fn compiler_output_builds_core_task_role_step_state() {
        let compiled = compile_dev_team_flow_for_work_item(&readiness_fixture(), "task").unwrap();
        let state = taskflow_core::role_step::TaskRoleStepState::from_first_step(&compiled.flow)
            .expect("compiled flow should initialize core role-step state");

        assert_eq!(state.flow_id, "task");
        assert_eq!(state.role_id, "analyst");
        assert_eq!(state.task_role_step().role_id, "analyst");
    }

    #[test]
    fn fails_startup_with_actionable_paths_for_bad_flow_config() {
        let mut readiness = readiness_fixture();
        readiness["flows"][0]["ordered_steps"][0]["role_id"] = serde_json::json!("missing_role");

        let error = compile_all_dev_team_flows(&readiness).expect_err("missing role blocks");
        assert!(error.to_string().contains("$.flows[0].steps[0].role_id"));
        assert!(error.to_string().contains("missing_role"));
    }
}
