//! Runtime adapter for the typed TeamFlow transition authority.
//!
//! This module is the only VIDA-side boundary that compiles the selected
//! TeamFlow config into a `TeamFlowSnapshot`. Consumers receive projections
//! from that snapshot and never parse flow aliases or synthesize a sequence.

use serde_json::{Map, Value};
use taskflow_authority::team_flow_transition::{
    TeamFlowNode, TeamFlowSnapshot, TeamFlowSnapshotInput,
};

#[derive(Debug, Clone)]
pub(crate) struct TeamFlowNodeProjection {
    pub(crate) node: TeamFlowNode,
    pub(crate) dispatch_alias: String,
    pub(crate) packet_template_kind: String,
    pub(crate) closure_class: String,
    pub(crate) stage: String,
    pub(crate) completion_blocker: String,
    pub(crate) proof_gates: Value,
    pub(crate) command_mapping: Option<Value>,
    pub(crate) approval_policy: Value,
    pub(crate) lifecycle_hook_templates: Value,
    pub(crate) resume_transitions: Value,
    pub(crate) rework_transitions: Value,
    pub(crate) profile_authority: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamFlowAuthorityProjection {
    pub(crate) snapshot: TeamFlowSnapshot,
    pub(crate) authority_id: String,
    pub(crate) config_authority_hash: String,
    pub(crate) registry_authority_hash: String,
    pub(crate) nodes: Vec<TeamFlowNodeProjection>,
}

impl TeamFlowAuthorityProjection {
    pub(crate) fn node(&self, node_id: &str) -> Option<&TeamFlowNodeProjection> {
        self.nodes.iter().find(|node| node.node.node_id == node_id)
    }

    pub(crate) fn ordered_nodes(&self) -> impl Iterator<Item = &TeamFlowNodeProjection> {
        self.snapshot
            .ordered_configured_nodes()
            .iter()
            .filter_map(|node_id| self.node(node_id))
    }
}

#[derive(Debug, Clone)]
struct AuthorityInputs {
    config_id: String,
    profile: String,
    flow_ref: String,
    registry_hash: String,
    config_hash: String,
    authority_id: String,
}

fn nonempty(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn authority_string(authority: &Value, key: &str) -> Option<String> {
    nonempty(authority.get(key))
        .or_else(|| nonempty(authority.get(key).and_then(|value| value.get("id"))))
}

fn authority_hash(authority: &Value, key: &str) -> Option<String> {
    let value = authority.get(key)?;
    nonempty(value.get("content_blake3"))
        .or_else(|| nonempty(authority.get(key).and_then(|value| value.get("hash"))))
        .or_else(|| nonempty(Some(value)))
        .or_else(|| Some(taskflow_authority::team_flow_transition::hash_json(value)))
}

fn authority_inputs(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> Result<AuthorityInputs, String> {
    let authority = compiled_bundle
        .get("team_flow_authority")
        .ok_or_else(|| "team_flow_authority_missing".to_string())?;
    let selected_config = authority
        .get("selected_config")
        .filter(|value| value.is_object())
        .ok_or_else(|| "team_flow_authority_selected_config_missing".to_string())?;
    let selection = selected_config
        .get("authority_selection")
        .filter(|value| value.is_object())
        .ok_or_else(|| "team_flow_authority_selection_missing".to_string())?;
    let config_id = nonempty(selection.get("config_id"))
        .or_else(|| nonempty(selected_config.get("config_id")))
        .ok_or_else(|| "team_flow_authority_config_id_missing".to_string())?;
    let profile = profile
        .and_then(|value| (!value.trim().is_empty()).then(|| value.trim().to_string()))
        .or_else(|| nonempty(selection.get("team_profile_id")))
        .or_else(|| nonempty(selection.get("profile")))
        .ok_or_else(|| "team_flow_authority_profile_missing".to_string())?;
    let flow_ref = flow_ref
        .and_then(|value| (!value.trim().is_empty()).then(|| value.trim().to_string()))
        .or_else(|| nonempty(selection.get("default_flow_id")))
        .or_else(|| nonempty(selected_config.get("default_flow_id")))
        .ok_or_else(|| "team_flow_authority_flow_ref_missing".to_string())?;
    let config_hash = authority_hash(authority, "config")
        .ok_or_else(|| "team_flow_authority_config_hash_missing".to_string())?;
    let registry_hash = authority_hash(authority, "registries")
        .ok_or_else(|| "team_flow_authority_registry_hash_missing".to_string())?;
    let authority_id = authority_string(authority, "authority_id")
        .ok_or_else(|| "team_flow_authority_id_missing".to_string())?;
    Ok(AuthorityInputs {
        config_id,
        profile,
        flow_ref,
        registry_hash,
        config_hash,
        authority_id,
    })
}

fn selected_flow<'a>(config: &'a Value, flow_ref: &str) -> Result<&'a Value, String> {
    let flows = config
        .get("flows")
        .and_then(Value::as_object)
        .ok_or_else(|| "team_flow_authority_flows_missing".to_string())?;
    flows
        .get(flow_ref)
        .ok_or_else(|| format!("team_flow_authority_unknown_flow:{flow_ref}"))
}

fn selected_steps(flow: &Value) -> Result<&[Value], String> {
    match (flow.get("ordered_steps"), flow.get("steps")) {
        (Some(_), Some(_)) => Err("team_flow_authority_conflicting_steps_aliases".to_string()),
        (Some(value), None) | (None, Some(value)) => value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| "team_flow_authority_invalid_steps_type".to_string()),
        (None, None) => Err("team_flow_authority_missing_steps".to_string()),
    }
}

fn step_node_id(step: &Value) -> Result<String, String> {
    let mut values = Vec::new();
    for key in ["node_id", "role_id", "step_id"] {
        if let Some(value) = nonempty(step.get(key)) {
            values.push(value);
        }
    }
    match values.as_slice() {
        [value] => Ok(value.clone()),
        [] => Err("team_flow_authority_missing_node_id".to_string()),
        _ => Err("team_flow_authority_conflicting_node_aliases".to_string()),
    }
}

fn role_for<'a>(config: &'a Value, node_id: &str) -> Option<&'a Value> {
    config.get("roles").and_then(Value::as_object)?.get(node_id)
}

fn merged_string(step: &Value, role: &Value, key: &str, node_id: &str) -> Result<String, String> {
    let step_value = nonempty(step.get(key));
    let role_value = nonempty(role.get(key));
    match (step_value, role_value) {
        (Some(_), Some(_)) => Err(format!(
            "team_flow_authority_conflicting_sources:{node_id}:{key}"
        )),
        (Some(value), None) | (None, Some(value)) => Ok(value),
        (None, None) => Err(format!("team_flow_authority_missing_field:{node_id}:{key}")),
    }
}

fn command_mapping(compiled_bundle: &Value, node: &TeamFlowNode) -> Option<Value> {
    let catalog = compiled_bundle.get("command_catalog")?.as_object()?;
    let command_ref = node.command_ref.as_ref()?;
    catalog.get(command_ref).cloned()
}

fn project_node(
    compiled_bundle: &Value,
    config: &Value,
    step: &Value,
    node: &TeamFlowNode,
) -> Result<TeamFlowNodeProjection, String> {
    let role = role_for(config, &node.node_id).unwrap_or(&Value::Null);
    let proof_gates = step
        .get("proof_gates")
        .cloned()
        .ok_or_else(|| format!("team_flow_authority_missing_proof_gates:{}", node.node_id))?;
    if !proof_gates.is_object() {
        return Err(format!(
            "team_flow_authority_invalid_proof_gates:{}",
            node.node_id
        ));
    }
    Ok(TeamFlowNodeProjection {
        node: node.clone(),
        dispatch_alias: nonempty(step.get("dispatch_alias")).unwrap_or_default(),
        packet_template_kind: merged_string(step, role, "packet_template_kind", &node.node_id)?,
        closure_class: merged_string(step, role, "closure_class", &node.node_id)?,
        stage: merged_string(step, role, "stage", &node.node_id)?,
        completion_blocker: merged_string(step, role, "completion_blocker", &node.node_id)?,
        proof_gates,
        command_mapping: command_mapping(compiled_bundle, node),
        approval_policy: step.get("approval_policy").cloned().unwrap_or(Value::Null),
        lifecycle_hook_templates: step
            .get("lifecycle_hook_templates")
            .cloned()
            .unwrap_or(Value::Null),
        resume_transitions: step
            .get("resume_transitions")
            .cloned()
            .unwrap_or(Value::Null),
        rework_transitions: step
            .get("rework_transitions")
            .cloned()
            .unwrap_or(Value::Null),
        profile_authority: serde_json::json!({
            "team_role_id": node.node_id,
            "runtime_role": node.runtime_role,
            "task_class": node.task_class,
            "source_path": format!("vida.config.yaml#dev_team.roles.{}", node.node_id),
        }),
    })
}

pub(crate) fn compile_team_flow_authority(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> Result<TeamFlowAuthorityProjection, String> {
    let inputs = authority_inputs(compiled_bundle, flow_ref, profile)?;
    let authority = compiled_bundle
        .get("team_flow_authority")
        .ok_or_else(|| "team_flow_authority_missing".to_string())?;
    let selected_config = authority
        .get("selected_config")
        .cloned()
        .ok_or_else(|| "team_flow_authority_selected_config_missing".to_string())?;
    let mut parser_config = selected_config.clone();
    let parser_object = parser_config
        .as_object_mut()
        .ok_or_else(|| "team_flow_authority_selected_config_invalid".to_string())?;
    parser_object.insert(
        "config_id".to_string(),
        Value::String(inputs.config_id.clone()),
    );
    parser_object.insert("profile".to_string(), Value::String(inputs.profile.clone()));
    parser_object.insert(
        "registry_hash".to_string(),
        Value::String(inputs.registry_hash.clone()),
    );
    let snapshot = TeamFlowSnapshot::from_config(
        &parser_config,
        TeamFlowSnapshotInput {
            config_id: &inputs.config_id,
            profile: &inputs.profile,
            flow_ref: &inputs.flow_ref,
            registry_hash: &inputs.registry_hash,
        },
    )
    .map_err(|error| format!("team_flow_authority_snapshot_compile:{error}"))?;
    let flow = selected_flow(&selected_config, &inputs.flow_ref)?;
    let steps = selected_steps(flow)?;
    let mut step_by_node = Map::new();
    for step in steps {
        let node_id = step_node_id(step)?;
        if step_by_node.insert(node_id.clone(), step.clone()).is_some() {
            return Err(format!("team_flow_authority_duplicate_node:{node_id}"));
        }
    }
    let mut nodes = Vec::with_capacity(snapshot.nodes.len());
    for node in &snapshot.nodes {
        let step = step_by_node
            .get(&node.node_id)
            .ok_or_else(|| format!("team_flow_authority_snapshot_node_missing:{}", node.node_id))?;
        nodes.push(project_node(compiled_bundle, &selected_config, step, node)?);
    }
    if taskflow_authority::team_flow_transition::hash_json(&selected_config) != inputs.config_hash {
        return Err("team_flow_authority_config_hash_mismatch".to_string());
    }
    Ok(TeamFlowAuthorityProjection {
        snapshot,
        authority_id: inputs.authority_id,
        config_authority_hash: inputs.config_hash,
        registry_authority_hash: inputs.registry_hash,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle() -> Value {
        let config = serde_json::json!({
            "authority_selection": {"config_id":"cfg","team_profile_id":"profile","default_flow_id":"flow"},
            "roles": {"coder": {"runtime_role":"worker","task_class":"implementation","inclusion_rule":"always","packet_template_kind":"delivery_task_packet","closure_class":"implementation","stage":"execution","completion_blocker":"pending"}},
            "flows": {"flow": {"flow_id":"flow","steps": [{"role_id":"coder","included":true,"required":true,"proof_gates":{"required_outputs":["changed"]},"terminal":true,"command_ref":"cmd"}]}}
        });
        let config_hash = taskflow_authority::team_flow_transition::hash_json(&config);
        serde_json::json!({"team_flow_authority":{"authority_id":"team-flow-authority:test","config":{"content_blake3":config_hash},"registries":{"content_blake3":"registry"},"selected_config":config},"command_catalog":{"cmd":{"surface":"vida agent-init-worker"}}})
    }

    #[test]
    fn compiles_snapshot_and_preserves_typed_fields() {
        let projection = compile_team_flow_authority(&bundle(), None, None).expect("projection");
        assert_eq!(projection.snapshot.ordered_nodes, vec!["coder"]);
        assert!(projection.node("coder").expect("node").node.terminal);
        assert_eq!(
            projection.node("coder").expect("node").proof_gates["required_outputs"][0],
            "changed"
        );
    }

    #[test]
    fn fails_closed_on_config_hash_drift() {
        let mut bundle = bundle();
        bundle["team_flow_authority"]["config"]["content_blake3"] =
            Value::String("drift".to_string());
        let error = compile_team_flow_authority(&bundle, None, None).expect_err("drift must block");
        assert_eq!(error, "team_flow_authority_config_hash_mismatch");
    }
}
