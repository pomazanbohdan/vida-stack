fn nonempty_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn binding_from_row(row: &serde_json::Value) -> Option<(String, String)> {
    let runtime_role = nonempty_string(
        row.get("runtime_role")
            .or_else(|| row.get("execution_runtime_role")),
    )?;
    let task_class = nonempty_string(
        row.get("task_class")
            .or_else(|| row.get("route_task_class")),
    )?;
    Some((runtime_role, task_class))
}

fn binding_row_active(row: &serde_json::Value) -> bool {
    row["enabled"] != serde_json::Value::Bool(false)
        && row["disabled"] != serde_json::Value::Bool(true)
        && row["unresolved"] != serde_json::Value::Bool(true)
        && !matches!(
            row["lifecycle_state"].as_str().map(str::trim),
            Some("disabled") | Some("retired") | Some("inactive")
        )
}

fn collect_binding_rows(
    rows: impl IntoIterator<Item = serde_json::Value>,
    selected_role: &str,
    task_class_hint: Option<&str>,
    bindings: &mut Vec<(String, String)>,
) {
    for row in rows {
        if !binding_row_active(&row) {
            continue;
        }
        let role_matches = selected_role.trim().is_empty()
            || row.get("role_id").and_then(serde_json::Value::as_str) == Some(selected_role)
            || row.get("node_id").and_then(serde_json::Value::as_str) == Some(selected_role)
            || row.get("lane_id").and_then(serde_json::Value::as_str) == Some(selected_role);
        if !role_matches {
            continue;
        }
        if let Some(binding) = binding_from_row(&row) {
            if task_class_hint
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none_or(|hint| binding.1 == hint)
            {
                bindings.push(binding);
            }
        }
        if let Some(task_classes) = row
            .get("task_classes")
            .and_then(serde_json::Value::as_array)
        {
            if let Some(runtime_role) = nonempty_string(
                row.get("runtime_role")
                    .or_else(|| row.get("execution_runtime_role")),
            ) {
                for task_class in task_classes.iter().filter_map(|value| {
                    value
                        .as_str()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                }) {
                    if task_class_hint
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none_or(|hint| task_class == hint)
                    {
                        bindings.push((runtime_role.clone(), task_class.to_string()));
                    }
                }
            }
        }
    }
}

fn configured_role_task_binding(
    compiled_bundle: &serde_json::Value,
    execution_plan: &serde_json::Value,
    selected_role: &str,
    task_class_hint: Option<&str>,
) -> Option<(String, String)> {
    if selected_role.trim().is_empty() {
        return None;
    }
    let mut bindings = Vec::new();
    for assignment in [
        execution_plan.get("runtime_assignment"),
        execution_plan.get("carrier_runtime_assignment"),
    ]
    .into_iter()
    .flatten()
    {
        if !binding_row_active(assignment) {
            continue;
        }
        if let Some(assignment_role) = ["role_id", "node_id", "lane_id"]
            .into_iter()
            .find_map(|field| assignment[field].as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if assignment_role != selected_role {
                continue;
            }
        }
        if let Some(binding) = binding_from_row(assignment) {
            if task_class_hint
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none_or(|hint| binding.1 == hint)
            {
                bindings.push(binding);
            }
        }
    }

    for source in [
        &compiled_bundle["authority_selection"]["roles"],
        &compiled_bundle["dev_team"]["roles"],
        &compiled_bundle["dev_team_readiness"]["roles"],
        &compiled_bundle["carrier_runtime"]["roles"],
    ] {
        match source {
            serde_json::Value::Object(rows) => {
                if let Some(row) = rows.get(selected_role) {
                    let mut row = row.clone();
                    if row.get("role_id").is_none() {
                        row["role_id"] = serde_json::Value::String(selected_role.to_string());
                    }
                    collect_binding_rows([row], selected_role, task_class_hint, &mut bindings);
                }
            }
            serde_json::Value::Array(rows) => collect_binding_rows(
                rows.iter().cloned(),
                selected_role,
                task_class_hint,
                &mut bindings,
            ),
            _ => {}
        }
    }

    let flow_id = execution_plan
        .get("team_flow_authority_selected_flow_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let node_id = execution_plan
        .get("team_flow_authority_selected_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(selected_role);
    for flow in compiled_bundle["dev_team_readiness"]["flows"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if flow_id
            .is_some_and(|id| flow.get("flow_id").and_then(serde_json::Value::as_str) != Some(id))
        {
            continue;
        }
        collect_binding_rows(
            flow["ordered_steps"]
                .as_array()
                .into_iter()
                .flatten()
                .cloned(),
            node_id,
            task_class_hint,
            &mut bindings,
        );
    }
    bindings.sort();
    bindings.dedup();
    (bindings.len() == 1).then(|| bindings.remove(0))
}

pub(crate) fn infer_runtime_task_class(
    selection: &super::RuntimeConsumptionLaneSelection,
    requires_design_gate: bool,
) -> String {
    let _ = requires_design_gate;
    configured_role_task_binding(
        &selection.compiled_bundle,
        &selection.execution_plan,
        &selection.selected_role,
        None,
    )
    .map(|(_, task_class)| task_class)
    .unwrap_or_default()
}

pub(crate) fn infer_execution_runtime_role(
    selection: &super::RuntimeConsumptionLaneSelection,
    task_class: &str,
    requires_design_gate: bool,
) -> String {
    let _ = requires_design_gate;
    configured_role_task_binding(
        &selection.compiled_bundle,
        &selection.execution_plan,
        &selection.selected_role,
        Some(task_class),
    )
    .map(|(runtime_role, _)| runtime_role)
    .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DispatchContractLane<'a> {
    pub(crate) task_class: Option<&'a str>,
}

impl<'a> DispatchContractLane<'a> {
    pub(crate) fn from_value(value: &'a serde_json::Value) -> Self {
        Self {
            task_class: value["task_class"].as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MinimumWriteScope {
    ReadOnly,
    WorkspaceWrite,
    GuardRequired,
}

impl MinimumWriteScope {
    pub(crate) fn from_write_scope(write_scope: &str) -> Option<Self> {
        let normalized = write_scope.trim().to_ascii_lowercase();
        if normalized.is_empty()
            || matches!(
                normalized.as_str(),
                "none" | "read-only" | "read_only" | "readonly" | "readorreview"
            )
            || normalized.starts_with("read_or_")
        {
            return Some(Self::ReadOnly);
        }
        if matches!(
            normalized.as_str(),
            "guard_required"
                | "guard-required"
                | "guard_required_owned_paths"
                | "guard-required-owned-paths"
                | "guard_required_packet_owned_paths"
                | "guard-required-packet-owned-paths"
                | "live_guard"
        ) {
            return Some(Self::GuardRequired);
        }
        matches!(
            normalized.as_str(),
            "workspace"
                | "scoped_only"
                | "workspace-write"
                | "workspace_write"
                | "service_executor"
                | "orchestrator_native"
                | "architecture_safe"
        )
        .then_some(Self::WorkspaceWrite)
    }

    pub(crate) fn admits(self, candidate: Self) -> bool {
        match self {
            Self::ReadOnly => candidate == Self::ReadOnly,
            Self::WorkspaceWrite | Self::GuardRequired => candidate >= self,
        }
    }
}

pub(crate) fn minimum_write_scope_for_task_class(task_class: &str) -> Option<MinimumWriteScope> {
    match task_class.trim() {
        "implementation"
        | "implementation_medium"
        | "delivery_task"
        | "test_authoring"
        | "regression_test" => Some(MinimumWriteScope::WorkspaceWrite),
        "execution_block" => Some(MinimumWriteScope::GuardRequired),
        crate::runtime_contract_vocab::TASK_CLASS_VERIFICATION
        | crate::runtime_contract_vocab::TASK_CLASS_ARCHITECTURE
        | crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION
        | crate::runtime_contract_vocab::TASK_CLASS_COACH
        | crate::runtime_contract_vocab::DISPATCH_TARGET_ANALYSIS
        | "review" => Some(MinimumWriteScope::ReadOnly),
        _ => None,
    }
}

pub(crate) fn stricter_write_scope_override(
    minimum: MinimumWriteScope,
    override_scope: &str,
) -> Option<MinimumWriteScope> {
    let override_scope = MinimumWriteScope::from_write_scope(override_scope)?;
    minimum.admits(override_scope).then_some(override_scope)
}

pub(crate) fn effective_minimum_write_scope(
    compiled_bundle: &serde_json::Value,
    task_class: &str,
) -> Option<MinimumWriteScope> {
    let minimum = minimum_write_scope_for_task_class(task_class)?;
    let Some(capability_registry) = compiled_bundle
        .get("policy_runtime")
        .and_then(|value| value.get("capability_registry"))
    else {
        return Some(minimum);
    };
    let Some(project_overrides) = capability_registry.get("project_overrides") else {
        return Some(minimum);
    };
    let Some(project_overrides) = project_overrides.as_object() else {
        return None;
    };
    match project_overrides.get(task_class) {
        None => Some(minimum),
        Some(override_scope) => stricter_write_scope_override(minimum, override_scope.as_str()?),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BackendAdmissibilityKey {
    Implementation,
    Verification,
    Architecture,
    Specification,
    Coach,
    Analysis,
    Review,
    Conservative(String),
}

impl BackendAdmissibilityKey {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Implementation => crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION,
            Self::Verification => crate::runtime_contract_vocab::TASK_CLASS_VERIFICATION,
            Self::Architecture => crate::runtime_contract_vocab::TASK_CLASS_ARCHITECTURE,
            Self::Specification => crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION,
            Self::Coach => crate::runtime_contract_vocab::TASK_CLASS_COACH,
            Self::Analysis => crate::runtime_contract_vocab::DISPATCH_TARGET_ANALYSIS,
            Self::Review => "review",
            Self::Conservative(value) => value.as_str(),
        }
    }

    pub(crate) fn into_string(self) -> String {
        self.as_str().to_string()
    }
}

pub(crate) fn backend_admissibility_key_for_task_class(
    task_class: &str,
) -> Option<BackendAdmissibilityKey> {
    match task_class.trim() {
        crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION => {
            Some(BackendAdmissibilityKey::Implementation)
        }
        crate::runtime_contract_vocab::TASK_CLASS_VERIFICATION => {
            Some(BackendAdmissibilityKey::Verification)
        }
        crate::runtime_contract_vocab::TASK_CLASS_ARCHITECTURE => {
            Some(BackendAdmissibilityKey::Architecture)
        }
        crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION => {
            Some(BackendAdmissibilityKey::Specification)
        }
        crate::runtime_contract_vocab::TASK_CLASS_COACH => Some(BackendAdmissibilityKey::Coach),
        crate::runtime_contract_vocab::DISPATCH_TARGET_ANALYSIS => {
            Some(BackendAdmissibilityKey::Analysis)
        }
        "review" => Some(BackendAdmissibilityKey::Review),
        _ => None,
    }
}

pub(crate) fn declared_task_class_supports_requested(
    declared_task_class: &str,
    requested_task_class: &str,
) -> bool {
    let declared_task_class = declared_task_class.trim();
    let requested_task_class = requested_task_class.trim();
    if declared_task_class.is_empty() || requested_task_class.is_empty() {
        return false;
    }
    if declared_task_class == requested_task_class {
        return true;
    }

    let Some(declared_key) = backend_admissibility_key_for_task_class(declared_task_class) else {
        return false;
    };
    let Some(requested_key) = backend_admissibility_key_for_task_class(requested_task_class) else {
        return false;
    };

    declared_task_class == declared_key.as_str() && declared_key == requested_key
}

pub(crate) fn backend_admissibility_key_for_dispatch_target(
    dispatch_target: &str,
    dispatch_contract_lane: Option<&DispatchContractLane<'_>>,
) -> BackendAdmissibilityKey {
    let canonical_target = canonical_dispatch_target_name(dispatch_target.trim());

    if let Some(task_class_key) = dispatch_contract_lane
        .and_then(|lane| lane.task_class)
        .and_then(backend_admissibility_key_for_task_class)
    {
        return task_class_key;
    }

    match canonical_target.as_str() {
        crate::runtime_contract_vocab::TASK_CLASS_IMPLEMENTATION => {
            BackendAdmissibilityKey::Implementation
        }
        crate::runtime_contract_vocab::TASK_CLASS_VERIFICATION => {
            BackendAdmissibilityKey::Verification
        }
        crate::runtime_contract_vocab::TASK_CLASS_ARCHITECTURE => {
            BackendAdmissibilityKey::Architecture
        }
        crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION => {
            BackendAdmissibilityKey::Specification
        }
        crate::runtime_contract_vocab::TASK_CLASS_COACH => BackendAdmissibilityKey::Coach,
        crate::runtime_contract_vocab::DISPATCH_TARGET_ANALYSIS => {
            BackendAdmissibilityKey::Analysis
        }
        "review" => BackendAdmissibilityKey::Review,
        other => BackendAdmissibilityKey::Conservative(other.to_string()),
    }
}

pub(crate) fn backend_admissibility_requires_strict_dispatch_target(dispatch_target: &str) -> bool {
    !dispatch_target.trim().is_empty()
}

pub(crate) fn backend_metadata_supports_architecture(entry: &serde_json::Value) -> bool {
    ["capability_band", "specialties"].iter().any(|field| {
        crate::json_string_list(entry.get(*field))
            .into_iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .any(|value| {
                matches!(
                    value.as_str(),
                    "architecture_safe" | "architecture" | "planning" | "long_context"
                )
            })
    })
}

pub(crate) fn backend_is_admissible_for_dispatch_target(
    execution_plan: &serde_json::Value,
    backend_id: &str,
    dispatch_target: &str,
    dispatch_contract_lane: Option<&DispatchContractLane<'_>>,
) -> bool {
    let canonical_target =
        backend_admissibility_key_for_dispatch_target(dispatch_target, dispatch_contract_lane)
            .into_string();
    let strict_required = backend_admissibility_requires_strict_dispatch_target(dispatch_target);
    let Some(matrix) = execution_plan["backend_admissibility_matrix"].as_array() else {
        return !strict_required;
    };
    let Some(row) = matrix
        .iter()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
    else {
        return !strict_required;
    };
    let Some(lane_admissibility) = row["lane_admissibility"].as_object() else {
        return !strict_required;
    };
    if let Some(explicit) = lane_admissibility
        .get(canonical_target.as_str())
        .and_then(serde_json::Value::as_bool)
    {
        return explicit;
    }

    let derived_architecture_capability =
        canonical_target == "architecture" && backend_metadata_supports_architecture(row);

    derived_architecture_capability || !strict_required
}

pub(crate) fn canonical_dispatch_target_alias(value: &str) -> Option<&'static str> {
    use crate::runtime_contract_vocab::{
        DISPATCH_TARGET_ANALYSIS, DISPATCH_TARGET_CLOSURE, DISPATCH_TARGET_COACH,
        DISPATCH_TARGET_EXECUTION_PREPARATION, DISPATCH_TARGET_IMPLEMENTER,
        DISPATCH_TARGET_SPECIFICATION, DISPATCH_TARGET_VERIFICATION,
    };

    match value.trim() {
        DISPATCH_TARGET_ANALYSIS => Some(DISPATCH_TARGET_ANALYSIS),
        DISPATCH_TARGET_COACH => Some(DISPATCH_TARGET_COACH),
        DISPATCH_TARGET_CLOSURE => Some(DISPATCH_TARGET_CLOSURE),
        DISPATCH_TARGET_EXECUTION_PREPARATION => Some(DISPATCH_TARGET_EXECUTION_PREPARATION),
        DISPATCH_TARGET_IMPLEMENTER => Some(DISPATCH_TARGET_IMPLEMENTER),
        DISPATCH_TARGET_SPECIFICATION => Some(DISPATCH_TARGET_SPECIFICATION),
        DISPATCH_TARGET_VERIFICATION => Some(DISPATCH_TARGET_VERIFICATION),
        _ => None,
    }
}

pub(crate) fn canonical_dispatch_target_name(value: &str) -> String {
    canonical_dispatch_target_alias(value)
        .unwrap_or_else(|| value.trim())
        .to_string()
}

#[derive(Debug, Clone)]
pub(crate) struct AgentInitResolvedRole {
    pub(crate) selected_role: String,
    pub(crate) mapping_source: Option<&'static str>,
}

pub(crate) fn agent_init_explicit_role_selection(
    resolved_role: &AgentInitResolvedRole,
    requested_role: &str,
    request_text: String,
) -> serde_json::Value {
    let role_mapping = resolved_role.mapping_source.map(|source| {
        serde_json::json!({
            "requested_role": requested_role,
            "selected_role": resolved_role.selected_role,
            "source": source,
        })
    });
    serde_json::json!({
        "mode": "explicit_role",
        "selected_role": resolved_role.selected_role,
        "requested_role": requested_role,
        "dispatch_target": requested_role,
        "role_mapping": role_mapping,
        "request_text": request_text,
    })
}

pub(crate) fn canonical_sorted_nonempty_strings(
    values: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn sorted_unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    canonical_sorted_nonempty_strings(values)
}

fn dev_team_role_runtime_role(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    requested_role: &str,
) -> Option<String> {
    if let Some(runtime_role) = compiled_bundle["dev_team"]["roles"]
        .as_object()
        .and_then(|roles| {
            roles
                .get(requested_role)
                .and_then(|role| role["runtime_role"].as_str())
        })
        .map(ToOwned::to_owned)
    {
        return Some(runtime_role);
    }
    dev_team_readiness["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|role| role["role_id"].as_str() == Some(requested_role))
        .and_then(|role| role["runtime_role"].as_str())
        .map(ToOwned::to_owned)
}

fn dev_team_flow_step_runtime_role(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    requested_role: &str,
) -> Option<String> {
    if let Some(runtime_role) = compiled_bundle["dev_team"]["flows"]
        .as_object()
        .into_iter()
        .flat_map(|flows| flows.values())
        .flat_map(|flow| flow["steps"].as_array().into_iter().flatten())
        .find_map(|step| {
            if step["role_id"].as_str() == Some(requested_role) {
                step["runtime_role"].as_str().map(ToOwned::to_owned)
            } else {
                None
            }
        })
    {
        return Some(runtime_role);
    }
    dev_team_readiness["flows"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|flow| flow["ordered_steps"].as_array().into_iter().flatten())
        .find_map(|step| {
            if step["role_id"].as_str() == Some(requested_role) {
                step["runtime_role"].as_str().map(ToOwned::to_owned)
            } else {
                None
            }
        })
}

fn dev_team_role_ids(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
) -> Vec<String> {
    let mut role_ids = Vec::new();
    role_ids.extend(
        compiled_bundle["dev_team"]["roles"]
            .as_object()
            .into_iter()
            .flat_map(|roles| roles.keys())
            .cloned(),
    );
    role_ids.extend(
        dev_team_readiness["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    role_ids.extend(
        compiled_bundle["dev_team"]["flows"]
            .as_object()
            .into_iter()
            .flat_map(|flows| flows.values())
            .flat_map(|flow| flow["steps"].as_array().into_iter().flatten())
            .filter_map(|step| step["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    role_ids.extend(
        dev_team_readiness["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|flow| flow["ordered_steps"].as_array().into_iter().flatten())
            .filter_map(|step| step["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    sorted_unique_strings(role_ids)
}

pub(crate) fn agent_init_selected_role_allowed(selected_role: &str) -> bool {
    !selected_role.trim().is_empty()
}

pub(crate) fn resolve_agent_init_explicit_role(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    requested_role: &str,
) -> Option<AgentInitResolvedRole> {
    if requested_role.trim().is_empty() {
        return None;
    }
    if agent_init_selected_role_allowed(requested_role)
        && crate::role_exists_in_lane_bundle(compiled_bundle, requested_role)
    {
        return Some(AgentInitResolvedRole {
            selected_role: requested_role.to_string(),
            mapping_source: None,
        });
    }
    if let Some(runtime_role) =
        dev_team_role_runtime_role(compiled_bundle, dev_team_readiness, requested_role)
    {
        if agent_init_selected_role_allowed(&runtime_role)
            && crate::role_exists_in_lane_bundle(compiled_bundle, &runtime_role)
        {
            return Some(AgentInitResolvedRole {
                selected_role: runtime_role,
                mapping_source: Some("dev_team.roles.runtime_role"),
            });
        }
    }
    if let Some(runtime_role) =
        dev_team_flow_step_runtime_role(compiled_bundle, dev_team_readiness, requested_role)
    {
        if agent_init_selected_role_allowed(&runtime_role)
            && crate::role_exists_in_lane_bundle(compiled_bundle, &runtime_role)
        {
            return Some(AgentInitResolvedRole {
                selected_role: runtime_role,
                mapping_source: Some("dev_team.flows.steps.runtime_role"),
            });
        }
    }
    None
}

pub(crate) fn agent_init_role_candidates(
    compiled_bundle: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
) -> Vec<String> {
    let mut candidates = Vec::new();
    candidates.extend(
        compiled_bundle["enabled_framework_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
    );
    candidates.extend(
        compiled_bundle["project_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    for row in crate::carrier_runtime_section(compiled_bundle)["roles"]
        .as_array()
        .into_iter()
        .flatten()
    {
        candidates.extend(
            ["role_id", "runtime_role", "default_runtime_role"]
                .into_iter()
                .filter_map(|field| row[field].as_str())
                .map(ToOwned::to_owned),
        );
        candidates.extend(
            row["runtime_roles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        );
        for profile in row["model_profiles"]
            .as_object()
            .into_iter()
            .flat_map(|profiles| profiles.values())
        {
            candidates.extend(
                profile["runtime_roles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
            );
        }
    }
    candidates.extend(dev_team_role_ids(compiled_bundle, dev_team_readiness));
    sorted_unique_strings(candidates)
}

pub(crate) fn task_complexity_multiplier(_task_class: &str) -> u64 {
    1
}

pub(crate) fn role_supports_task_class(role: &serde_json::Value, task_class: &str) -> bool {
    let task_classes = role["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    task_classes.is_empty()
        || task_classes
            .iter()
            .any(|declared| declared_task_class_supports_requested(declared, task_class))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_contract_vocab::RUNTIME_ROLE_WORKER;

    fn selection(selected_role: &str, request: &str) -> crate::RuntimeConsumptionLaneSelection {
        crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test".to_string(),
            fallback_role: RUNTIME_ROLE_WORKER.to_string(),
            request: request.to_string(),
            selected_role: selected_role.to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: Vec::new(),
            compiled_bundle:
                crate::team_flow_authority_adapter::test_support::canonical_compiled_bundle(),
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        }
    }

    fn selection_with_binding(
        selected_role: &str,
        runtime_role: &str,
        task_class: &str,
    ) -> crate::RuntimeConsumptionLaneSelection {
        let mut selection = selection(selected_role, "configured binding");
        selection.compiled_bundle["authority_selection"]["roles"][selected_role] = serde_json::json!({
            "runtime_role": runtime_role,
            "task_class": task_class,
        });
        selection
    }

    #[test]
    fn configured_role_task_binding_is_order_invariant_and_missing_is_blocked() {
        let mut configured = selection("configured-role", "configured binding");
        configured.compiled_bundle["authority_selection"]["roles"] = serde_json::json!({});
        configured.compiled_bundle["dev_team_readiness"]["roles"] = serde_json::json!([]);
        configured.compiled_bundle["dev_team_readiness"]["flows"] = serde_json::json!([
            {"flow_id": "selected", "ordered_steps": [
                {"node_id": "configured-role", "runtime_role": "configured-runtime", "task_class": "configured-task"}
            ]},
            {"flow_id": "unrelated", "ordered_steps": []}
        ]);
        configured.execution_plan = serde_json::json!({
            "team_flow_authority_selected_flow_id": "selected",
            "team_flow_authority_selected_node_id": "configured-role"
        });
        let mut ordered = configured.clone();
        ordered.compiled_bundle["dev_team_readiness"]["flows"] = serde_json::json!([
            {"flow_id": "unrelated", "ordered_steps": []},
            {"flow_id": "selected", "ordered_steps": [
                {"node_id": "configured-role", "runtime_role": "configured-runtime", "task_class": "configured-task"}
            ]}
        ]);
        assert_eq!(
            infer_runtime_task_class(&configured, false),
            "configured-task"
        );
        assert_eq!(infer_runtime_task_class(&ordered, false), "configured-task");
        configured.compiled_bundle["dev_team_readiness"]["flows"] = serde_json::json!([]);
        configured.execution_plan = serde_json::json!({});
        assert_eq!(infer_runtime_task_class(&configured, false), "");
    }

    #[test]
    fn configured_role_task_binding_fails_closed_for_disabled_and_ambiguous_rows() {
        let mut disabled =
            selection_with_binding("configured-role", "configured-runtime", "configured-task");
        disabled.compiled_bundle["authority_selection"]["roles"]["configured-role"]["enabled"] =
            serde_json::json!(false);
        let mut ambiguous = selection("configured-role", "ambiguous");
        ambiguous.compiled_bundle["dev_team_readiness"]["roles"] = serde_json::json!([
            {"role_id": "configured-role", "runtime_role": "runtime-a", "task_class": "task-a"},
            {"role_id": "configured-role", "runtime_role": "runtime-b", "task_class": "task-b"}
        ]);
        assert_eq!(infer_runtime_task_class(&disabled, false), "");
        assert_eq!(infer_runtime_task_class(&ambiguous, false), "");
    }

    #[test]
    fn canonical_sorted_nonempty_strings_trims_dedups_and_preserves_case() {
        let values = canonical_sorted_nonempty_strings(vec![
            " worker ".to_string(),
            "".to_string(),
            "Analyst".to_string(),
            "worker".to_string(),
            "worker".to_string(),
        ]);

        assert_eq!(values, vec!["Analyst".to_string(), "worker".to_string()]);
    }

    #[test]
    fn dispatch_contract_task_class_wins_over_human_target_label() {
        let lane = DispatchContractLane {
            task_class: Some("implementation"),
        };

        assert_eq!(
            backend_admissibility_key_for_dispatch_target("tester", Some(&lane)).as_str(),
            "implementation"
        );
    }

    #[test]
    fn tester_target_uses_verification_without_contract_lane() {
        assert_eq!(
            backend_admissibility_key_for_dispatch_target("tester", None).as_str(),
            "tester"
        );
    }

    #[test]
    fn architecture_alias_uses_architecture_strictness() {
        assert_eq!(
            backend_admissibility_key_for_dispatch_target("solution_architect", None).as_str(),
            "solution_architect"
        );
    }

    #[test]
    fn architecture_capability_requires_explicit_backend_metadata() {
        assert!(backend_metadata_supports_architecture(&serde_json::json!({
            "capability_band": ["architecture_safe"]
        })));
        assert!(backend_metadata_supports_architecture(&serde_json::json!({
            "specialties": ["architecture"]
        })));
        assert!(backend_metadata_supports_architecture(&serde_json::json!({
            "capability_band": "architecture_safe,review_safe"
        })));
        assert!(!backend_metadata_supports_architecture(
            &serde_json::json!({
                "subagent_backend_class": "internal",
                "specialties": ["implementation"]
            })
        ));
    }

    #[test]
    fn architecture_explicit_false_overrides_capability_metadata() {
        let plan = serde_json::json!({
            "backend_admissibility_matrix": [{
                "backend_id": "internal_subagents",
                "capability_band": ["architecture_safe"],
                "lane_admissibility": {"architecture": false}
            }]
        });

        assert!(!backend_is_admissible_for_dispatch_target(
            &plan,
            "internal_subagents",
            "architecture",
            None
        ));
    }

    #[test]
    fn internal_backend_without_architecture_metadata_is_denied() {
        let plan = serde_json::json!({
            "backend_admissibility_matrix": [{
                "backend_id": "internal_subagents",
                "backend_class": "internal",
                "lane_admissibility": {}
            }]
        });

        assert!(!backend_is_admissible_for_dispatch_target(
            &plan,
            "internal_subagents",
            "architecture",
            None
        ));
    }

    #[test]
    fn unknown_dispatch_target_is_conservative() {
        assert_eq!(
            backend_admissibility_key_for_dispatch_target("custom-lane", None).as_str(),
            "custom-lane"
        );
    }

    #[test]
    fn dispatch_target_aliases_normalize_in_routing_policy() {
        assert_eq!(
            canonical_dispatch_target_name("business_analyst"),
            "business_analyst"
        );
        assert_eq!(canonical_dispatch_target_name("prover"), "prover");
        assert_eq!(canonical_dispatch_target_name("escalation"), "escalation");
        assert_eq!(
            canonical_dispatch_target_name("release/closure"),
            "release/closure"
        );
        assert_eq!(canonical_dispatch_target_name("custom-lane"), "custom-lane");
    }

    #[test]
    fn generic_task_class_declaration_requires_exact_configured_relation() {
        assert!(declared_task_class_supports_requested("coach", "coach"));
        assert!(declared_task_class_supports_requested(
            "verification",
            "verification"
        ));
        assert!(!declared_task_class_supports_requested(
            "coach",
            "validation"
        ));
        assert!(!declared_task_class_supports_requested(
            "verification",
            "release_readiness"
        ));
    }

    #[test]
    fn specific_task_class_alias_does_not_claim_generic_request() {
        assert!(!declared_task_class_supports_requested(
            "validation",
            "coach"
        ));
        assert!(!declared_task_class_supports_requested("custom", "coach"));
    }

    #[test]
    fn hard_capability_floor_maps_implementation_test_authoring_and_analysis() {
        assert_eq!(
            minimum_write_scope_for_task_class("implementation"),
            Some(MinimumWriteScope::WorkspaceWrite)
        );
        assert_eq!(
            minimum_write_scope_for_task_class("test_authoring"),
            Some(MinimumWriteScope::WorkspaceWrite)
        );
        assert_eq!(
            minimum_write_scope_for_task_class(
                crate::runtime_contract_vocab::DISPATCH_TARGET_ANALYSIS
            ),
            Some(MinimumWriteScope::ReadOnly)
        );
        assert_eq!(
            minimum_write_scope_for_task_class("execution_block"),
            Some(MinimumWriteScope::GuardRequired)
        );
    }

    #[test]
    fn hard_capability_floor_distinguishes_readonly_workspace_and_guard() {
        assert_eq!(
            MinimumWriteScope::from_write_scope("readonly"),
            Some(MinimumWriteScope::ReadOnly)
        );
        assert_eq!(
            MinimumWriteScope::from_write_scope("workspace-write"),
            Some(MinimumWriteScope::WorkspaceWrite)
        );
        assert_eq!(
            MinimumWriteScope::from_write_scope("guard_required"),
            Some(MinimumWriteScope::GuardRequired)
        );
    }

    #[test]
    fn readonly_scope_rejects_write_capable_candidates() {
        assert!(MinimumWriteScope::ReadOnly.admits(MinimumWriteScope::ReadOnly));
        assert!(!MinimumWriteScope::ReadOnly.admits(MinimumWriteScope::WorkspaceWrite));
        assert!(!MinimumWriteScope::ReadOnly.admits(MinimumWriteScope::GuardRequired));
        assert!(MinimumWriteScope::WorkspaceWrite.admits(MinimumWriteScope::GuardRequired));
    }

    #[test]
    fn project_override_must_be_equal_or_stricter_than_framework_floor() {
        assert_eq!(
            stricter_write_scope_override(MinimumWriteScope::ReadOnly, "workspace-write"),
            None
        );
        assert_eq!(
            stricter_write_scope_override(MinimumWriteScope::WorkspaceWrite, "readonly"),
            None
        );
        assert_eq!(
            stricter_write_scope_override(MinimumWriteScope::WorkspaceWrite, "workspace-write"),
            Some(MinimumWriteScope::WorkspaceWrite)
        );
        assert_eq!(
            stricter_write_scope_override(MinimumWriteScope::WorkspaceWrite, "guard_required"),
            Some(MinimumWriteScope::GuardRequired)
        );
    }

    #[test]
    fn effective_floor_uses_framework_when_override_is_absent() {
        assert_eq!(
            effective_minimum_write_scope(&serde_json::json!({}), "implementation"),
            Some(MinimumWriteScope::WorkspaceWrite)
        );
    }

    #[test]
    fn effective_floor_accepts_equal_and_stricter_overrides() {
        assert_eq!(
            effective_minimum_write_scope(
                &serde_json::json!({"policy_runtime":{"capability_registry":{"project_overrides":{"implementation":"workspace-write"}}}}),
                "implementation"
            ),
            Some(MinimumWriteScope::WorkspaceWrite)
        );
        assert_eq!(
            effective_minimum_write_scope(
                &serde_json::json!({"policy_runtime":{"capability_registry":{"project_overrides":{"implementation":"guard_required"}}}}),
                "implementation"
            ),
            Some(MinimumWriteScope::GuardRequired)
        );
    }

    #[test]
    fn effective_floor_fails_closed_for_weaker_or_unknown_overrides() {
        assert_eq!(
            effective_minimum_write_scope(
                &serde_json::json!({"policy_runtime":{"capability_registry":{"project_overrides":{"implementation":"readonly"}}}}),
                "implementation"
            ),
            None
        );
        assert_eq!(
            effective_minimum_write_scope(
                &serde_json::json!({"policy_runtime":{"capability_registry":{"project_overrides":{"implementation":"unknown"}}}}),
                "implementation"
            ),
            None
        );
    }
}
