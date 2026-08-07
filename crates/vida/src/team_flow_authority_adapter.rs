//! Persisted TeamFlow authority adapter.
//!
//! Source normalization and projection construction live in
//! [`crate::team_flow_authority_projection`].  This module is deliberately
//! source-free: it validates the immutable persisted projection and resolves
//! a requested flow/node for executable consumers.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::Deref,
};

use serde_json::{Map, Value};
use taskflow_authority::team_flow_transition::{
    admit_transition, TeamFlowNode, TeamFlowReceipt, TeamFlowSnapshot, TransitionVerdict,
};

#[derive(Debug, Clone)]
pub(crate) struct TeamFlowNodeProjection {
    pub(crate) node: TeamFlowNode,
    pub(crate) lane_id: String,
    pub(crate) dispatch_target: String,
    pub(crate) dispatch_alias: String,
    pub(crate) dispatch_alias_blocker: Option<String>,
    pub(crate) carrier_id: String,
    pub(crate) carrier_tier: String,
    pub(crate) carrier_relation: Value,
    pub(crate) executor_backend_relation: Value,
    pub(crate) executor_backend_class: String,
    pub(crate) backend_relation: Value,
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
    pub(crate) selected_model_profile: Value,
    pub(crate) component_registry: Value,
    pub(crate) activation: Value,
    pub(crate) assignment: Value,
    pub(crate) authority_identities: Value,
    pub(crate) execution_identity: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamFlowAuthorityProjection {
    pub(crate) snapshot: TeamFlowSnapshot,
    /// Configured immutable entry node for fresh flow selection.
    pub(crate) entry_node_id: String,
    pub(crate) authority_id: String,
    pub(crate) authority_content_hash: String,
    pub(crate) config_authority_hash: String,
    pub(crate) registry_authority_hash: String,
    pub(crate) registry_identities: Value,
    pub(crate) nodes: Vec<TeamFlowNodeProjection>,
}

impl TeamFlowAuthorityProjection {
    pub(crate) fn node(&self, node_id: &str) -> Option<&TeamFlowNodeProjection> {
        self.nodes.iter().find(|node| node.node.node_id == node_id)
    }

    pub(crate) fn ordered_node_ids(&self) -> &[String] {
        self.snapshot.ordered_configured_nodes()
    }

    pub(crate) fn ordered_nodes(&self) -> impl Iterator<Item = TeamFlowNodeProjection> {
        self.snapshot
            .ordered_configured_nodes()
            .iter()
            .filter_map(|id| self.node(id).cloned())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamFlowResolutionBlocker {
    pub(crate) code: String,
    pub(crate) requested: String,
    pub(crate) candidates: Vec<String>,
}

impl TeamFlowResolutionBlocker {
    fn new(code: impl Into<String>, requested: impl Into<String>, candidates: Vec<String>) -> Self {
        Self {
            code: code.into(),
            requested: requested.into(),
            candidates,
        }
    }
}

impl fmt::Display for TeamFlowResolutionBlocker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.candidates.is_empty() {
            write!(formatter, "{}: {}", self.code, self.requested)
        } else {
            write!(
                formatter,
                "{}: {} (candidates: {})",
                self.code,
                self.requested,
                self.candidates.join(", ")
            )
        }
    }
}

impl std::error::Error for TeamFlowResolutionBlocker {}

pub(crate) const TEAM_FLOW_AUTHORITY_UNAVAILABLE_BLOCKER: &str =
    "team_flow_authority_selection_missing";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TeamFlowAuthorityAvailabilityStatus {
    Unavailable,
    Blocked,
    Disabled,
    Ready,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamFlowAuthorityAvailability {
    pub(crate) status: TeamFlowAuthorityAvailabilityStatus,
    pub(crate) blocker: Option<String>,
    pub(crate) projection: Option<TeamFlowAuthorityProjection>,
}

impl TeamFlowAuthorityAvailability {
    pub(crate) fn is_ready(&self) -> bool {
        self.status == TeamFlowAuthorityAvailabilityStatus::Ready
            && self.projection.is_some()
            && self.blocker.is_none()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TeamFlowExecutionAuthority {
    projection: TeamFlowAuthorityProjection,
}

impl Deref for TeamFlowExecutionAuthority {
    type Target = TeamFlowAuthorityProjection;

    fn deref(&self) -> &Self::Target {
        &self.projection
    }
}

impl TeamFlowExecutionAuthority {
    pub(crate) fn require(
        compiled_bundle: &Value,
        flow_ref: Option<&str>,
        profile: Option<&str>,
    ) -> Result<Self, TeamFlowResolutionBlocker> {
        if team_flow_is_disabled(compiled_bundle) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_disabled",
                flow_ref.unwrap_or("team_flow"),
                Vec::new(),
            ));
        }
        compile_persisted(compiled_bundle, flow_ref, profile).map(|projection| Self { projection })
    }

    pub(crate) fn projection(&self) -> &TeamFlowAuthorityProjection {
        &self.projection
    }

    pub(crate) fn resolve_target(
        &self,
        execution_plan: Option<&Value>,
        requested: &str,
    ) -> Result<TeamFlowNodeResolution, TeamFlowResolutionBlocker> {
        validate_execution_plan(&self.projection, execution_plan)?;
        resolve_node(&self.projection, requested, LookupKind::Target)
    }

    pub(crate) fn resolve_runtime_role(
        &self,
        execution_plan: Option<&Value>,
        runtime_role: &str,
    ) -> Result<TeamFlowNodeResolution, TeamFlowResolutionBlocker> {
        validate_execution_plan(&self.projection, execution_plan)?;
        resolve_node(&self.projection, runtime_role, LookupKind::RuntimeRole)
    }

    pub(crate) fn admit_next(
        &self,
        current: &str,
        receipt: &TeamFlowReceipt,
        requested: &str,
    ) -> Result<TransitionVerdict, TeamFlowResolutionBlocker> {
        self.admit(current, receipt, requested)
    }

    pub(crate) fn admit_rework(
        &self,
        current: &str,
        receipt: &TeamFlowReceipt,
        requested: &str,
    ) -> Result<TransitionVerdict, TeamFlowResolutionBlocker> {
        let node = resolve_node(&self.projection, current, LookupKind::Target)?;
        if !node
            .rework_targets
            .iter()
            .any(|target| target == requested.trim())
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_rework_target_not_configured",
                requested,
                node.rework_targets,
            ));
        }
        self.admit(current, receipt, requested)
    }

    pub(crate) fn admit_terminal(
        &self,
        current: &str,
        receipt: &TeamFlowReceipt,
    ) -> Result<TransitionVerdict, TeamFlowResolutionBlocker> {
        let node = resolve_node(&self.projection, current, LookupKind::Target)?;
        if !node.terminal {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_terminal_closure_not_declared",
                current,
                vec![node.node_id],
            ));
        }
        self.admit(current, receipt, "")
    }

    fn admit(
        &self,
        current: &str,
        receipt: &TeamFlowReceipt,
        requested: &str,
    ) -> Result<TransitionVerdict, TeamFlowResolutionBlocker> {
        let node = resolve_node(&self.projection, current, LookupKind::Target)?;
        let verdict = admit_transition(
            &self.projection.snapshot,
            &node.node_id,
            Some(receipt),
            requested,
        );
        if verdict.allowed {
            Ok(verdict)
        } else {
            Err(TeamFlowResolutionBlocker::new(
                verdict
                    .blocker
                    .clone()
                    .unwrap_or_else(|| "team_flow_transition_blocked".to_string()),
                requested,
                verdict.expected.into_iter().collect(),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamFlowNodeResolution {
    pub(crate) node_id: String,
    pub(crate) lane_id: String,
    pub(crate) dispatch_target: String,
    pub(crate) dispatch_alias: String,
    pub(crate) runtime_role: String,
    pub(crate) task_class: String,
    pub(crate) inclusion_rule: String,
    pub(crate) included: bool,
    pub(crate) required: bool,
    pub(crate) evidence_requirements: Vec<String>,
    pub(crate) packet_template_kind: String,
    pub(crate) closure_class: String,
    pub(crate) stage: String,
    pub(crate) completion_blocker: String,
    pub(crate) proof_gates: Value,
    pub(crate) approval_policy: Value,
    pub(crate) lifecycle_hook_templates: Value,
    pub(crate) resume_transitions: Value,
    pub(crate) rework_transitions: Value,
    pub(crate) command_surface: Option<String>,
    pub(crate) command_ref: Option<String>,
    pub(crate) command_mapping: Option<Value>,
    pub(crate) next_node: Option<String>,
    pub(crate) terminal: bool,
    pub(crate) requires_user_approval: bool,
    pub(crate) rework_targets: Vec<String>,
    pub(crate) carrier_id: String,
    pub(crate) carrier_tier: String,
    pub(crate) carrier_relation: Value,
    pub(crate) executor_backend_relation: Value,
    pub(crate) executor_backend_class: String,
    pub(crate) backend_relation: Value,
    pub(crate) profile_authority: Value,
    pub(crate) selected_model_profile: Value,
    pub(crate) component_registry: Value,
    pub(crate) activation: Value,
    pub(crate) assignment: Value,
    pub(crate) authority_identities: Value,
    pub(crate) execution_identity: Value,
    pub(crate) authority_id: String,
    pub(crate) authority_content_hash: String,
    pub(crate) config_authority_hash: String,
    pub(crate) registry_authority_hash: String,
    pub(crate) ordered_nodes: Vec<String>,
    pub(crate) source: String,
}

fn nonempty(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn required_string(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<String, TeamFlowResolutionBlocker> {
    nonempty(value.get(key)).ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_missing",
            format!("{path}.{key}"),
            Vec::new(),
        )
    })
}

fn required_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a Map<String, Value>, TeamFlowResolutionBlocker> {
    value.as_object().ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_object_invalid",
            path,
            Vec::new(),
        )
    })
}

fn required_array<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a [Value], TeamFlowResolutionBlocker> {
    value.as_array().map(Vec::as_slice).ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_array_invalid",
            path,
            Vec::new(),
        )
    })
}

fn required_member<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a Value, TeamFlowResolutionBlocker> {
    object.get(key).ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_missing",
            format!("{path}.{key}"),
            Vec::new(),
        )
    })
}

fn exact_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    path: &str,
) -> Result<(), TeamFlowResolutionBlocker> {
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual != expected {
        let mut differences = actual
            .difference(&expected)
            .map(|key| format!("unexpected:{key}"))
            .collect::<Vec<_>>();
        differences.extend(
            expected
                .difference(&actual)
                .map(|key| format!("missing:{key}")),
        );
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_schema_invalid",
            path,
            differences,
        ));
    }
    Ok(())
}

fn nonempty_string_array(
    value: &Value,
    path: &str,
    require_nonempty: bool,
) -> Result<Vec<String>, TeamFlowResolutionBlocker> {
    let values = required_array(value, path)?;
    if require_nonempty && values.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_invalid",
            path,
            Vec::new(),
        ));
    }
    let mut result = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    for value in values {
        let Some(value) = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                path,
                Vec::new(),
            ));
        };
        if !seen.insert(value.to_string()) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_schema_invalid",
                path,
                vec![format!("duplicate:{value}")],
            ));
        }
        result.push(value.to_string());
    }
    Ok(result)
}

fn strict_relation(
    value: &Value,
    path: &str,
    expected_keys: &[&str],
    expected_kind: &str,
) -> Result<Map<String, Value>, TeamFlowResolutionBlocker> {
    let object = required_object(value, path)?;
    exact_keys(object, expected_keys, path)?;
    let kind = required_string(value, "relation_kind", path)?;
    if kind != expected_kind {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_relation_invalid",
            format!("{path}.relation_kind"),
            vec![expected_kind.to_string()],
        ));
    }
    for key in expected_keys
        .iter()
        .copied()
        .filter(|key| *key != "relation_kind")
    {
        required_string(value, key, path)?;
    }
    Ok(object.clone())
}

fn strict_identity(
    value: &Value,
    path: &str,
) -> Result<Map<String, Value>, TeamFlowResolutionBlocker> {
    let object = required_object(value, path)?;
    exact_keys(object, &["id", "content_blake3"], path)?;
    required_string(value, "id", path)?;
    required_string(value, "content_blake3", path)?;
    Ok(object.clone())
}

fn nullable_bool(value: &Value, key: &str, path: &str) -> Result<(), TeamFlowResolutionBlocker> {
    if !value
        .get(key)
        .is_some_and(|value| value.is_null() || value.is_boolean())
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_invalid",
            format!("{path}.{key}"),
            Vec::new(),
        ));
    }
    Ok(())
}

fn nullable_string(value: &Value, key: &str, path: &str) -> Result<(), TeamFlowResolutionBlocker> {
    if !value
        .get(key)
        .is_some_and(|value| value.is_null() || nonempty(Some(value)).is_some())
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_invalid",
            format!("{path}.{key}"),
            Vec::new(),
        ));
    }
    Ok(())
}

fn nullable_string_array(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<(), TeamFlowResolutionBlocker> {
    let Some(member) = value.get(key) else {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_missing",
            format!("{path}.{key}"),
            Vec::new(),
        ));
    };
    if member.is_null() {
        return Ok(());
    }
    nonempty_string_array(member, &format!("{path}.{key}"), false).map(|_| ())
}

fn nullable_object(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<Option<Map<String, Value>>, TeamFlowResolutionBlocker> {
    let Some(member) = value.get(key) else {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_missing",
            format!("{path}.{key}"),
            Vec::new(),
        ));
    };
    if member.is_null() {
        return Ok(None);
    }
    Ok(Some(
        required_object(member, &format!("{path}.{key}"))?.clone(),
    ))
}

fn strict_flow_policy(value: &Value, path: &str) -> Result<Value, TeamFlowResolutionBlocker> {
    const FIELDS: [&str; 12] = [
        "enabled",
        "default",
        "flow_class",
        "description",
        "work_item_bindings",
        "sequential",
        "allow_parallel_handoffs",
        "lifecycle_hook_templates",
        "proof_gates",
        "resume_transitions",
        "rework_transitions",
        "adapter_projection",
    ];
    let object = required_object(value, path)?;
    exact_keys(object, &FIELDS, path)?;
    let _enabled = required_member(object, "enabled", path)?
        .as_bool()
        .ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                format!("{path}.enabled"),
                vec!["boolean".to_string()],
            )
        })?;
    for key in [
        "enabled",
        "default",
        "sequential",
        "allow_parallel_handoffs",
    ] {
        nullable_bool(value, key, path)?;
    }
    for key in ["flow_class", "description"] {
        nullable_string(value, key, path)?;
    }
    for key in ["work_item_bindings", "lifecycle_hook_templates"] {
        nullable_string_array(value, key, path)?;
    }
    if let Some(proof_gates) = nullable_object(value, "proof_gates", path)? {
        exact_keys(
            &proof_gates,
            &["required_outputs"],
            &format!("{path}.proof_gates"),
        )?;
        nonempty_string_array(
            proof_gates
                .get("required_outputs")
                .expect("exact proof-gates keys include required_outputs"),
            &format!("{path}.proof_gates.required_outputs"),
            false,
        )?;
    }
    for key in ["resume_transitions", "rework_transitions"] {
        if let Some(map) = nullable_object(value, key, path)? {
            for (entry, target) in map {
                if entry.trim().is_empty()
                    || target
                        .as_str()
                        .map(str::trim)
                        .filter(|target| !target.is_empty())
                        .is_none()
                {
                    return Err(TeamFlowResolutionBlocker::new(
                        "team_flow_authority_persisted_field_invalid",
                        format!("{path}.{key}"),
                        Vec::new(),
                    ));
                }
            }
        }
    }
    nullable_object(value, "adapter_projection", path)?;
    Ok(value.clone())
}

fn strict_profile_authority(value: &Value, path: &str) -> Result<Value, TeamFlowResolutionBlocker> {
    let object = required_object(value, path)?;
    exact_keys(
        object,
        &["team_role_id", "runtime_role", "task_class", "source_path"],
        path,
    )?;
    for key in ["team_role_id", "runtime_role", "task_class", "source_path"] {
        required_string(value, key, path)?;
    }
    Ok(value.clone())
}

fn strict_model_profile(value: &Value, path: &str) -> Result<Value, TeamFlowResolutionBlocker> {
    let object = required_object(value, path)?;
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "profile_id" | "selection_source" | "provider" | "model_ref" | "reasoning_effort"
        ) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_schema_invalid",
                path,
                vec![format!("unexpected:{key}")],
            ));
        }
    }
    required_string(value, "profile_id", path)?;
    required_string(value, "provider", path)?;
    required_string(value, "model_ref", path)?;
    required_string(value, "reasoning_effort", path)?;
    required_string(value, "selection_source", path)?;
    Ok(value.clone())
}

fn strict_assignment(value: &Value, path: &str) -> Result<Value, TeamFlowResolutionBlocker> {
    let object = required_object(value, path)?;
    if object.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_invalid",
            path,
            Vec::new(),
        ));
    }
    if [
        "executor_backend",
        "backend_id",
        "backend_class",
        "required_backend_class",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_runtime_assignment_backend_field_forbidden",
            path,
            Vec::new(),
        ));
    }
    Ok(value.clone())
}

fn validate_snapshot_schema(snapshot: &TeamFlowSnapshot) -> Result<(), TeamFlowResolutionBlocker> {
    if snapshot.nodes.is_empty() || snapshot.ordered_nodes.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_empty",
            snapshot.flow_ref.clone(),
            Vec::new(),
        ));
    }
    if snapshot.ordered_nodes.len() != snapshot.nodes.len() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_sequence_invalid",
            snapshot.flow_ref.clone(),
            Vec::new(),
        ));
    }
    let mut ids = BTreeSet::new();
    for node in &snapshot.nodes {
        if node.node_id.trim().is_empty()
            || node.runtime_role.trim().is_empty()
            || node.task_class.trim().is_empty()
            || node.inclusion_rule.trim().is_empty()
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_snapshot_node_identity_invalid",
                snapshot.flow_ref.clone(),
                Vec::new(),
            ));
        }
        if !ids.insert(node.node_id.clone()) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_snapshot_node_duplicate",
                node.node_id.clone(),
                Vec::new(),
            ));
        }
        if node.required && node.evidence_requirements.is_empty() {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_snapshot_evidence_missing",
                node.node_id.clone(),
                Vec::new(),
            ));
        }
        match (node.terminal, node.next_node.as_deref()) {
            (true, Some(_)) | (false, None) => {
                return Err(TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_snapshot_terminal_edge_invalid",
                    node.node_id.clone(),
                    Vec::new(),
                ));
            }
            _ => {}
        }
    }
    if snapshot
        .ordered_nodes
        .iter()
        .zip(snapshot.nodes.iter())
        .any(|(ordered, node)| ordered != &node.node_id)
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_lane_order_mismatch",
            snapshot.flow_ref.clone(),
            Vec::new(),
        ));
    }
    for node in &snapshot.nodes {
        if let Some(next) = &node.next_node {
            if !ids.contains(next) {
                return Err(TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_snapshot_edge_invalid",
                    format!("{}.next_node", node.node_id),
                    vec![next.clone()],
                ));
            }
        }
        for target in &node.rework_targets {
            if !ids.contains(target) {
                return Err(TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_snapshot_edge_invalid",
                    format!("{}.rework", node.node_id),
                    vec![target.clone()],
                ));
            }
        }
    }
    for node in &snapshot.nodes {
        let mut seen = BTreeSet::new();
        let mut current = Some(node.node_id.as_str());
        while let Some(node_id) = current {
            if !seen.insert(node_id) {
                return Err(TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_snapshot_transition_cycle",
                    node.node_id.clone(),
                    Vec::new(),
                ));
            }
            current = snapshot
                .node(node_id)
                .and_then(|next| next.next_node.as_deref());
        }
    }
    Ok(())
}

fn validate_execution_plan(
    projection: &TeamFlowAuthorityProjection,
    execution_plan: Option<&Value>,
) -> Result<(), TeamFlowResolutionBlocker> {
    let Some(plan) = execution_plan else {
        return Ok(());
    };
    let contract = plan
        .get("development_flow")
        .and_then(|flow| flow.get("dispatch_contract"))
        .or_else(|| plan.get("dispatch_contract"));
    let Some(contract) = contract else {
        return Ok(());
    };
    for (key, expected, code) in [
        (
            "team_flow_authority_id",
            projection.authority_id.as_str(),
            "team_flow_authority_plan_identity_mismatch",
        ),
        (
            "team_flow_config_hash",
            projection.config_authority_hash.as_str(),
            "team_flow_authority_plan_config_hash_mismatch",
        ),
        (
            "team_flow_registry_hash",
            projection.registry_authority_hash.as_str(),
            "team_flow_authority_plan_registry_hash_mismatch",
        ),
    ] {
        let actual = required_string(contract, key, "execution_plan.dispatch_contract")?;
        if actual != expected {
            return Err(TeamFlowResolutionBlocker::new(
                code,
                actual,
                vec![expected.to_string()],
            ));
        }
    }
    validate_selected_node_identity(projection, plan, contract)?;
    Ok(())
}

fn validate_selected_node_identity(
    projection: &TeamFlowAuthorityProjection,
    plan: &Value,
    contract: &Value,
) -> Result<(), TeamFlowResolutionBlocker> {
    let compatibility = plan
        .get("team_flow_authority_selected_node_id_compatibility")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let fields = [
        (
            "execution_plan.team_flow_authority_selected_node_id",
            plan.get("team_flow_authority_selected_node_id"),
        ),
        (
            "execution_plan.dispatch_contract.selected_node_id",
            contract.get("selected_node_id"),
        ),
        (
            "execution_plan.dispatch_contract.team_flow_authority_selected_node_id",
            contract.get("team_flow_authority_selected_node_id"),
        ),
    ];
    let mut values = Vec::new();
    let mut missing = Vec::new();
    for (path, value) in fields {
        match value {
            Some(value) => {
                let selected_node_id = value
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        TeamFlowResolutionBlocker::new(
                            "team_flow_authority_selected_node_id_invalid",
                            path,
                            Vec::new(),
                        )
                    })?;
                values.push((path, selected_node_id));
            }
            None => missing.push(path),
        }
    }
    if values.is_empty() {
        if compatibility
            == Some(
                crate::runtime_dispatch_state::TEAM_FLOW_SELECTED_NODE_LEGACY_INITIAL_COMPATIBILITY,
            )
        {
            return Ok(());
        }
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_selected_node_id_missing",
            missing.join(","),
            Vec::new(),
        ));
    }
    if !missing.is_empty()
        && compatibility
            != Some(
                crate::runtime_dispatch_state::TEAM_FLOW_SELECTED_NODE_LEGACY_INITIAL_COMPATIBILITY,
            )
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_selected_node_id_missing",
            missing.join(","),
            values
                .iter()
                .map(|(_, value)| (*value).to_string())
                .collect(),
        ));
    }
    let selected_node_id = values[0].1;
    if values.iter().any(|(_, value)| *value != selected_node_id) {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_selected_node_id_conflict",
            "execution_plan.selected_node_id",
            values
                .iter()
                .map(|(path, value)| format!("{path}={value}"))
                .collect(),
        ));
    }
    let node = projection.node(selected_node_id).ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_selected_node_id_unknown",
            selected_node_id,
            projection.ordered_node_ids().to_vec(),
        )
    })?;
    if !node.node.included {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_selected_node_id_excluded",
            selected_node_id,
            vec![selected_node_id.to_string()],
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum LookupKind {
    Target,
    RuntimeRole,
}

fn resolve_node(
    projection: &TeamFlowAuthorityProjection,
    requested: &str,
    kind: LookupKind,
) -> Result<TeamFlowNodeResolution, TeamFlowResolutionBlocker> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_node_resolution_missing",
            requested,
            Vec::new(),
        ));
    }
    let mut matches = Vec::new();
    let exact_node_id = projection
        .nodes
        .iter()
        .find(|node| node.node.node_id == requested)
        .map(|node| node.node.node_id.as_str());
    if let Some(exact_node_id) = exact_node_id {
        let alias_collisions = projection
            .nodes
            .iter()
            .filter(|node| node.node.node_id != exact_node_id && node.dispatch_alias == requested)
            .map(|node| node.node.node_id.clone())
            .collect::<Vec<_>>();
        if !alias_collisions.is_empty() {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_node_resolution_identity_collision",
                requested,
                alias_collisions,
            ));
        }
    }
    for node in &projection.nodes {
        let matched = match kind {
            LookupKind::RuntimeRole => node.node.runtime_role == requested,
            LookupKind::Target => {
                exact_node_id == Some(node.node.node_id.as_str())
                    || (exact_node_id.is_none()
                        && (node.dispatch_target == requested || node.dispatch_alias == requested))
            }
        };
        if matched {
            matches.push(node);
        }
    }
    let mut unique = Vec::new();
    for node in matches {
        if !unique
            .iter()
            .any(|candidate: &&TeamFlowNodeProjection| candidate.node.node_id == node.node.node_id)
        {
            unique.push(node);
        }
    }
    if unique.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_node_resolution_missing",
            requested,
            Vec::new(),
        ));
    }
    if unique.len() != 1 {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_node_resolution_ambiguous",
            requested,
            unique
                .iter()
                .map(|node| node.node.node_id.clone())
                .collect(),
        ));
    }
    let node = unique[0];
    if !node.node.included {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_node_excluded",
            requested,
            vec![node.node.node_id.clone()],
        ));
    }
    let command_surface = node
        .command_mapping
        .as_ref()
        .and_then(|mapping| nonempty(mapping.get("surface")));
    Ok(TeamFlowNodeResolution {
        node_id: node.node.node_id.clone(),
        lane_id: node.lane_id.clone(),
        dispatch_target: node.dispatch_target.clone(),
        dispatch_alias: node.dispatch_alias.clone(),
        runtime_role: node.node.runtime_role.clone(),
        task_class: node.node.task_class.clone(),
        inclusion_rule: node.node.inclusion_rule.clone(),
        included: node.node.included,
        required: node.node.required,
        evidence_requirements: node.node.evidence_requirements.clone(),
        packet_template_kind: node.packet_template_kind.clone(),
        closure_class: node.closure_class.clone(),
        stage: node.stage.clone(),
        completion_blocker: node.completion_blocker.clone(),
        proof_gates: node.proof_gates.clone(),
        approval_policy: node.approval_policy.clone(),
        lifecycle_hook_templates: node.lifecycle_hook_templates.clone(),
        resume_transitions: node.resume_transitions.clone(),
        rework_transitions: node.rework_transitions.clone(),
        command_surface,
        command_ref: node.node.command_ref.clone(),
        command_mapping: node.command_mapping.clone(),
        next_node: node.node.next_node.clone(),
        terminal: node.node.terminal,
        requires_user_approval: node.node.requires_user_approval,
        rework_targets: node.node.rework_targets.clone(),
        carrier_id: node.carrier_id.clone(),
        carrier_tier: node.carrier_tier.clone(),
        carrier_relation: node.carrier_relation.clone(),
        executor_backend_relation: node.executor_backend_relation.clone(),
        executor_backend_class: node.executor_backend_class.clone(),
        backend_relation: node.backend_relation.clone(),
        profile_authority: node.profile_authority.clone(),
        selected_model_profile: node.selected_model_profile.clone(),
        component_registry: node.component_registry.clone(),
        activation: node.activation.clone(),
        assignment: node.assignment.clone(),
        authority_identities: node.authority_identities.clone(),
        execution_identity: node.execution_identity.clone(),
        authority_id: projection.authority_id.clone(),
        authority_content_hash: projection.authority_content_hash.clone(),
        config_authority_hash: projection.config_authority_hash.clone(),
        registry_authority_hash: projection.registry_authority_hash.clone(),
        ordered_nodes: projection.ordered_node_ids().to_vec(),
        source: "persisted_projection".to_string(),
    })
}

fn parse_persisted_lane(
    row: &Value,
    path: &str,
    registry_identities: &Value,
) -> Result<TeamFlowNodeProjection, TeamFlowResolutionBlocker> {
    const LANE_FIELDS: [&str; 34] = [
        "node_id",
        "lane_id",
        "dispatch_target",
        "dispatch_alias",
        "runtime_role",
        "task_class",
        "packet_template_kind",
        "closure_class",
        "stage",
        "completion_blocker",
        "inclusion_rule",
        "included",
        "required",
        "next_node",
        "evidence_requirements",
        "proof_gates",
        "command_ref",
        "command_mapping",
        "rework",
        "terminal",
        "profile_authority",
        "selected_model_profile",
        "requires_user_approval",
        "approval_policy",
        "lifecycle_hook_templates",
        "resume_transitions",
        "policy_diagnostics",
        "activation",
        "runtime_assignment",
        "carrier_runtime_assignment",
        "carrier_relation",
        "executor_backend_relation",
        "authority_identities",
        "execution_identity",
    ];
    let object = required_object(row, path)?;
    exact_keys(object, &LANE_FIELDS, path)?;
    let text_field = |key: &str| required_string(row, key, path);
    let lane_id = text_field("lane_id")?;
    let dispatch_target = text_field("dispatch_target")?;
    let dispatch_alias = text_field("dispatch_alias")?;
    let runtime_role = text_field("runtime_role")?;
    let task_class = text_field("task_class")?;
    let inclusion_rule = text_field("inclusion_rule")?;
    let included = required_member(object, "included", path)?
        .as_bool()
        .ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                format!("{path}.included"),
                Vec::new(),
            )
        })?;
    let required = required_member(object, "required", path)?
        .as_bool()
        .ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                format!("{path}.required"),
                Vec::new(),
            )
        })?;
    let next_node = match required_member(object, "next_node", path)? {
        Value::Null => None,
        value => Some(
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    TeamFlowResolutionBlocker::new(
                        "team_flow_authority_persisted_field_invalid",
                        format!("{path}.next_node"),
                        Vec::new(),
                    )
                })?,
        ),
    };
    let rework = required_member(object, "rework", path)?;
    let rework_object = required_object(rework, &format!("{path}.rework"))?;
    exact_keys(rework_object, &["targets"], &format!("{path}.rework"))?;
    let rework_targets = nonempty_string_array(
        required_member(rework_object, "targets", &format!("{path}.rework"))?,
        &format!("{path}.rework.targets"),
        false,
    )?;
    let evidence_requirements = nonempty_string_array(
        required_member(object, "evidence_requirements", path)?,
        &format!("{path}.evidence_requirements"),
        required,
    )?;
    let proof_gates = required_member(object, "proof_gates", path)?;
    let proof_gates_object = required_object(proof_gates, &format!("{path}.proof_gates"))?;
    exact_keys(
        proof_gates_object,
        &["required_outputs"],
        &format!("{path}.proof_gates"),
    )?;
    let proof_outputs = nonempty_string_array(
        required_member(
            proof_gates_object,
            "required_outputs",
            &format!("{path}.proof_gates"),
        )?,
        &format!("{path}.proof_gates.required_outputs"),
        required,
    )?;
    if evidence_requirements != proof_outputs {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_evidence_requirements_conflict",
            format!("{path}.evidence_requirements"),
            proof_outputs.clone(),
        ));
    }
    let mut proof_gates_value = proof_gates.clone();
    proof_gates_value["required_outputs"] = serde_json::json!(proof_outputs);
    let command_ref = match required_member(object, "command_ref", path)? {
        Value::Null => None,
        value => Some(
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    TeamFlowResolutionBlocker::new(
                        "team_flow_authority_persisted_field_invalid",
                        format!("{path}.command_ref"),
                        Vec::new(),
                    )
                })?,
        ),
    };
    let command_mapping = match required_member(object, "command_mapping", path)? {
        Value::Null => None,
        value => {
            let mapping = required_object(value, &format!("{path}.command_mapping"))?;
            for key in mapping.keys() {
                if !matches!(
                    key.as_str(),
                    "command_id" | "surface" | "args" | "description"
                ) {
                    return Err(TeamFlowResolutionBlocker::new(
                        "team_flow_authority_persisted_schema_invalid",
                        format!("{path}.command_mapping"),
                        vec![format!("unexpected:{key}")],
                    ));
                }
            }
            required_string(value, "command_id", &format!("{path}.command_mapping"))?;
            required_string(value, "surface", &format!("{path}.command_mapping"))?;
            if command_ref.is_none() {
                return Err(TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_command_mapping_conflict",
                    format!("{path}.command_mapping"),
                    Vec::new(),
                ));
            }
            Some(value.clone())
        }
    };
    if command_ref.is_some() && command_mapping.is_none() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_command_mapping_missing",
            path,
            Vec::new(),
        ));
    }
    let terminal = required_member(object, "terminal", path)?
        .as_bool()
        .ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                format!("{path}.terminal"),
                Vec::new(),
            )
        })?;
    if (terminal && next_node.is_some()) || (!terminal && next_node.is_none()) {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_terminal_edge_invalid",
            format!("{path}.terminal"),
            Vec::new(),
        ));
    }
    let requires_user_approval = required_member(object, "requires_user_approval", path)?
        .as_bool()
        .ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                format!("{path}.requires_user_approval"),
                Vec::new(),
            )
        })?;
    let approval_policy = required_member(object, "approval_policy", path)?;
    let approval_object = required_object(approval_policy, &format!("{path}.approval_policy"))?;
    for key in approval_object.keys() {
        if !matches!(
            key.as_str(),
            "mode" | "prompt_template" | "allowed_decisions"
        ) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_schema_invalid",
                format!("{path}.approval_policy"),
                vec![format!("unexpected:{key}")],
            ));
        }
    }
    let approval_mode = if approval_object.contains_key("mode") {
        Some(required_string(
            approval_policy,
            "mode",
            &format!("{path}.approval_policy"),
        )?)
    } else {
        None
    };
    if approval_mode.is_none() && !approval_object.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_missing",
            format!("{path}.approval_policy.mode"),
            Vec::new(),
        ));
    }
    if let Some(mode) = approval_mode.as_deref() {
        if !matches!(
            mode,
            "user_review_required" | "optional_user_review" | "not_required"
        ) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_approval_mode_unsupported",
                format!("{path}.approval_policy.mode"),
                vec![
                    "user_review_required".to_string(),
                    "optional_user_review".to_string(),
                    "not_required".to_string(),
                ],
            ));
        }
    }
    let approval_decisions = approval_object
        .get("allowed_decisions")
        .map(|value| {
            nonempty_string_array(
                value,
                &format!("{path}.approval_policy.allowed_decisions"),
                true,
            )
        })
        .transpose()?;
    if approval_mode.is_some() && approval_decisions.is_none() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_missing",
            format!("{path}.approval_policy.allowed_decisions"),
            Vec::new(),
        ));
    }
    if let Some(decisions) = approval_decisions.as_ref() {
        if decisions
            .iter()
            .any(|decision| !matches!(decision.as_str(), "approved" | "rework"))
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_approval_decision_unsupported",
                format!("{path}.approval_policy.allowed_decisions"),
                vec!["approved".to_string(), "rework".to_string()],
            ));
        }
        if decisions.iter().any(|decision| decision == "rework") && rework_targets.is_empty() {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_rework_approval_contract_invalid",
                format!("{path}.approval_policy.allowed_decisions"),
                vec!["rework target".to_string()],
            ));
        }
    }
    if requires_user_approval {
        if approval_mode.as_deref() != Some("user_review_required") {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_approval_contract_invalid",
                format!("{path}.approval_policy.mode"),
                vec!["user_review_required".to_string()],
            ));
        }
        required_string(approval_policy, "mode", &format!("{path}.approval_policy"))?;
        required_string(
            approval_policy,
            "prompt_template",
            &format!("{path}.approval_policy"),
        )?;
        let decisions = approval_decisions.clone().ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_missing",
                format!("{path}.approval_policy.allowed_decisions"),
                Vec::new(),
            )
        })?;
        if !decisions.iter().any(|decision| decision == "approved") {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_approval_contract_invalid",
                format!("{path}.approval_policy.allowed_decisions"),
                vec!["approved".to_string()],
            ));
        }
    } else {
        match approval_mode.as_deref() {
            Some("user_review_required") => {
                return Err(TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_approval_contract_invalid",
                    format!("{path}.requires_user_approval"),
                    vec!["true".to_string()],
                ));
            }
            Some("optional_user_review") => {}
            Some("not_required") => {
                if approval_object.contains_key("prompt_template") {
                    return Err(TeamFlowResolutionBlocker::new(
                        "team_flow_authority_persisted_approval_contract_invalid",
                        format!("{path}.approval_policy.prompt_template"),
                        Vec::new(),
                    ));
                }
            }
            Some(_) | None => {}
        }
    }
    let lifecycle_hook_templates = required_member(object, "lifecycle_hook_templates", path)?;
    let lifecycle = nonempty_string_array(
        lifecycle_hook_templates,
        &format!("{path}.lifecycle_hook_templates"),
        false,
    )?;
    let resume_transitions = required_member(object, "resume_transitions", path)?;
    let resume_object = required_object(resume_transitions, &format!("{path}.resume_transitions"))?;
    for (key, value) in resume_object {
        if key.trim().is_empty()
            || value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                format!("{path}.resume_transitions"),
                Vec::new(),
            ));
        }
    }
    if approval_mode.as_deref() == Some("not_required") && resume_object.contains_key("approved") {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_approval_contract_invalid",
            format!("{path}.resume_transitions.approved"),
            Vec::new(),
        ));
    }
    if requires_user_approval && !resume_object.contains_key("approved") {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_approval_resume_missing",
            format!("{path}.resume_transitions.approved"),
            Vec::new(),
        ));
    }
    let policy_diagnostics = required_member(object, "policy_diagnostics", path)?;
    let diagnostics = required_object(policy_diagnostics, &format!("{path}.policy_diagnostics"))?;
    exact_keys(
        diagnostics,
        &["source", "fallback_used", "fallback_fields"],
        &format!("{path}.policy_diagnostics"),
    )?;
    if required_string(
        policy_diagnostics,
        "source",
        &format!("{path}.policy_diagnostics"),
    )? != "team_flow_authority.selected_config"
        || diagnostics.get("fallback_used").and_then(Value::as_bool) != Some(false)
        || !required_member(
            diagnostics,
            "fallback_fields",
            &format!("{path}.policy_diagnostics"),
        )?
        .as_array()
        .is_some_and(Vec::is_empty)
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_policy_diagnostics_invalid",
            format!("{path}.policy_diagnostics"),
            Vec::new(),
        ));
    }
    let profile_authority = strict_profile_authority(
        required_member(object, "profile_authority", path)?,
        &format!("{path}.profile_authority"),
    )?;
    if profile_authority
        .get("team_role_id")
        .and_then(Value::as_str)
        != object.get("node_id").and_then(Value::as_str)
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_role_index_identity_mismatch",
            format!("{path}.profile_authority.team_role_id"),
            vec![object
                .get("node_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()],
        ));
    }
    let selected_model_profile = strict_model_profile(
        required_member(object, "selected_model_profile", path)?,
        &format!("{path}.selected_model_profile"),
    )?;
    let activation = strict_assignment(
        required_member(object, "activation", path)?,
        &format!("{path}.activation"),
    )?;
    let assignment = strict_assignment(
        required_member(object, "runtime_assignment", path)?,
        &format!("{path}.runtime_assignment"),
    )?;
    let carrier_runtime_assignment = strict_assignment(
        required_member(object, "carrier_runtime_assignment", path)?,
        &format!("{path}.carrier_runtime_assignment"),
    )?;
    if assignment != carrier_runtime_assignment {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_assignment_mismatch",
            path,
            Vec::new(),
        ));
    }
    let carrier_relation = required_member(object, "carrier_relation", path)?;
    let carrier_relation = strict_relation(
        carrier_relation,
        &format!("{path}.carrier_relation"),
        &["relation_kind", "source_path", "selected_id"],
        "carrier_catalog",
    )?;
    let executor_backend_relation = required_member(object, "executor_backend_relation", path)?;
    let executor_backend_relation = strict_relation(
        executor_backend_relation,
        &format!("{path}.executor_backend_relation"),
        &[
            "relation_kind",
            "source_path",
            "selected_id",
            "backend_class",
            "required_backend_class",
        ],
        "executor_backend",
    )?;
    let executor_backend_class = required_string(
        &Value::Object(executor_backend_relation.clone()),
        "backend_class",
        &format!("{path}.executor_backend_relation"),
    )?;
    let authority_identities = required_member(object, "authority_identities", path)?;
    let authority_identities_array = required_array(
        authority_identities,
        &format!("{path}.authority_identities"),
    )?;
    if authority_identities_array.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_invalid",
            format!("{path}.authority_identities"),
            Vec::new(),
        ));
    }
    for (index, identity) in authority_identities_array.iter().enumerate() {
        let identity_object =
            required_object(identity, &format!("{path}.authority_identities[{index}]"))?;
        exact_keys(
            identity_object,
            &["kind", "id", "source_path"],
            &format!("{path}.authority_identities[{index}]"),
        )?;
        required_string(
            identity,
            "kind",
            &format!("{path}.authority_identities[{index}]"),
        )?;
        required_string(
            identity,
            "id",
            &format!("{path}.authority_identities[{index}]"),
        )?;
        required_string(
            identity,
            "source_path",
            &format!("{path}.authority_identities[{index}]"),
        )?;
    }
    let execution_identity = required_member(object, "execution_identity", path)?;
    let execution_object =
        required_object(execution_identity, &format!("{path}.execution_identity"))?;
    exact_keys(
        execution_object,
        &["id", "source_fields"],
        &format!("{path}.execution_identity"),
    )?;
    required_string(
        execution_identity,
        "id",
        &format!("{path}.execution_identity"),
    )?;
    nonempty_string_array(
        required_member(
            execution_object,
            "source_fields",
            &format!("{path}.execution_identity"),
        )?,
        &format!("{path}.execution_identity.source_fields"),
        true,
    )?;
    let node = TeamFlowNode {
        node_id: text_field("node_id")?,
        runtime_role,
        task_class,
        inclusion_rule,
        included,
        required,
        next_node,
        rework_targets,
        evidence_requirements,
        command_ref,
        command_mapping_hash: command_mapping
            .as_ref()
            .map(taskflow_authority::team_flow_transition::hash_json),
        requires_user_approval,
        terminal,
    };
    Ok(TeamFlowNodeProjection {
        node,
        lane_id,
        dispatch_target,
        dispatch_alias,
        dispatch_alias_blocker: None,
        carrier_id: carrier_relation
            .get("selected_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_relation_invalid",
                    format!("{path}.carrier_relation.selected_id"),
                    Vec::new(),
                )
            })?,
        carrier_tier: assignment
            .get("carrier_tier")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_field_missing",
                    format!("{path}.runtime_assignment.carrier_tier"),
                    Vec::new(),
                )
            })?,
        carrier_relation: Value::Object(carrier_relation),
        executor_backend_relation: Value::Object(executor_backend_relation.clone()),
        executor_backend_class,
        backend_relation: Value::Object(executor_backend_relation),
        packet_template_kind: text_field("packet_template_kind")?,
        closure_class: text_field("closure_class")?,
        stage: text_field("stage")?,
        completion_blocker: text_field("completion_blocker")?,
        proof_gates: proof_gates_value,
        command_mapping,
        approval_policy: approval_policy.clone(),
        lifecycle_hook_templates: Value::Array(lifecycle.into_iter().map(Value::String).collect()),
        resume_transitions: resume_transitions.clone(),
        rework_transitions: rework.clone(),
        profile_authority,
        selected_model_profile,
        component_registry: registry_identities.clone(),
        activation,
        assignment,
        authority_identities: authority_identities.clone(),
        execution_identity: execution_identity.clone(),
    })
}

/// Resolve a persisted transition edge by its configured node identity only.
///
/// Dispatch targets and aliases belong to the explicit Fresh/external lookup
/// boundary. Persisted and replayed snapshots must never canonicalize those
/// values into executable node ids, because doing so would silently rewrite
/// durable state.
fn resolve_persisted_node_identity(
    projections: &[TeamFlowNodeProjection],
    requested: &str,
    path: &str,
) -> Result<String, TeamFlowResolutionBlocker> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_node_resolution_missing",
            path,
            Vec::new(),
        ));
    }
    let matches = projections
        .iter()
        .filter(|projection| projection.node.node_id == requested)
        .map(|projection| projection.node.node_id.clone())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [node_id] => Ok(node_id.clone()),
        [] => Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_edge_invalid",
            path,
            vec![requested.to_string()],
        )),
        _ => Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_identity_ambiguous",
            path,
            matches,
        )),
    }
}

fn persisted_snapshot(
    flow_id: &str,
    entry_node_id: &str,
    lanes: &[Value],
    config_id: &str,
    profile: &str,
    config_hash: &str,
    registry_hash: &str,
    registry_identities: &Value,
) -> Result<TeamFlowSnapshot, TeamFlowResolutionBlocker> {
    let mut projections = Vec::with_capacity(lanes.len());
    for (index, lane) in lanes.iter().enumerate() {
        projections.push(parse_persisted_lane(
            lane,
            &format!("resolved_all_flow_payload.flows[{flow_id}].lanes[{index}]"),
            registry_identities,
        )?);
    }
    let normalized_edges = projections
        .iter()
        .map(|projection| {
            let next_node = projection
                .node
                .next_node
                .as_deref()
                .map(|target| {
                    resolve_persisted_node_identity(
                        &projections,
                        target,
                        &format!("{}.next_node", projection.node.node_id),
                    )
                })
                .transpose()?;
            let rework_targets = projection
                .node
                .rework_targets
                .iter()
                .map(|target| {
                    resolve_persisted_node_identity(
                        &projections,
                        target,
                        &format!("{}.rework", projection.node.node_id),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, TeamFlowResolutionBlocker>((next_node, rework_targets))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (projection, (next_node, rework_targets)) in projections.iter_mut().zip(normalized_edges) {
        projection.node.next_node = next_node;
        projection.node.rework_targets = rework_targets;
    }
    let nodes = projections
        .into_iter()
        .map(|projection| projection.node)
        .collect::<Vec<_>>();
    let entry_node_id = entry_node_id.trim();
    if entry_node_id.is_empty() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_entry_node_missing",
            format!("resolved_all_flow_payload.flows[{flow_id}].entry_node_id"),
            Vec::new(),
        ));
    }
    let Some(entry_node) = nodes.iter().find(|node| node.node_id == entry_node_id) else {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_entry_node_unknown",
            entry_node_id,
            nodes.iter().map(|node| node.node_id.clone()).collect(),
        ));
    };
    if !entry_node.included {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_entry_node_excluded",
            entry_node_id,
            vec![entry_node_id.to_string()],
        ));
    }
    let ordered_nodes = nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let mut snapshot = TeamFlowSnapshot {
        config_id: config_id.to_string(),
        profile: profile.to_string(),
        flow_ref: flow_id.to_string(),
        config_hash: config_hash.to_string(),
        registry_hash: registry_hash.to_string(),
        snapshot_ref: String::new(),
        entry_node_id: entry_node_id.to_string(),
        ordered_nodes,
        nodes,
    };
    validate_snapshot_schema(&snapshot)?;
    snapshot.snapshot_ref = hash_persisted_snapshot(&snapshot);
    if !snapshot.has_valid_identity() {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_identity_mismatch",
            flow_id,
            Vec::new(),
        ));
    }
    Ok(snapshot)
}

fn hash_persisted_snapshot(snapshot: &TeamFlowSnapshot) -> String {
    let mut copy = snapshot.clone();
    copy.snapshot_ref.clear();
    taskflow_authority::team_flow_transition::hash_json(
        &serde_json::to_value(copy).expect("snapshot serializes"),
    )
}

fn deterministic_flow_identity_id(flow_id: &str, flow_policy: &Value, lanes: &[Value]) -> String {
    format!(
        "team-flow-flow:{}",
        taskflow_authority::team_flow_transition::hash_json(&serde_json::json!({
            "flow_id": flow_id,
            "flow_policy": flow_policy,
            "lanes": lanes,
        }))
    )
}

fn compile_persisted(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> Result<TeamFlowAuthorityProjection, TeamFlowResolutionBlocker> {
    const REGISTRY_NAMES: [&str; 7] = [
        "roles",
        "skills",
        "profiles",
        "flows",
        "packs",
        "commands",
        "dispatch_aliases",
    ];
    const SELECTION_FIELDS: [&str; 15] = [
        "schema_version",
        "config_id",
        "team_profile_id",
        "default_flow_id",
        "projection_mode",
        "registry_identity_algorithm",
        "terminal_source",
        "edge_source",
        "command_resolution_mode",
        "approval_enforcement_mode",
        "alias_conflict_policy",
        "node_field_source_mode",
        "dispatch_alias_resolution_mode",
        "carrier_relation_mode",
        "profile_model_resolution_mode",
    ];
    let authority = compiled_bundle.get("team_flow_authority").ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_missing",
            "team_flow_authority",
            Vec::new(),
        )
    })?;
    let authority_object = required_object(authority, "team_flow_authority")?;
    if authority.get("status").and_then(Value::as_str) == Some("unavailable") {
        return Err(TeamFlowResolutionBlocker::new(
            TEAM_FLOW_AUTHORITY_UNAVAILABLE_BLOCKER,
            "team_flow_authority",
            Vec::new(),
        ));
    }
    for key in [
        "dev_team",
        "flows",
        "roles",
        "resolved_projection",
        "snapshot_parser_config",
        "raw_registries",
        "carrier_runtime",
        "project_node",
    ] {
        if authority_object.contains_key(key) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_legacy_source_rejected",
                format!("team_flow_authority.{key}"),
                Vec::new(),
            ));
        }
    }
    if required_string(authority, "schema_version", "team_flow_authority")?
        != "team-flow-authority.v1"
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_schema_invalid",
            "team_flow_authority.schema_version",
            vec!["team-flow-authority.v1".to_string()],
        ));
    }
    let authority_id = required_string(authority, "authority_id", "team_flow_authority")?;
    let authority_content_hash =
        required_string(authority, "content_blake3", "team_flow_authority")?;
    let config = required_member(authority_object, "config", "team_flow_authority")?;
    strict_identity(config, "team_flow_authority.config")?;
    let config_object = required_object(config, "team_flow_authority.config")?;
    let config_hash = required_string(config, "content_blake3", "team_flow_authority.config")?;
    let selected_config =
        required_member(authority_object, "selected_config", "team_flow_authority")?;
    let selected_config_object =
        required_object(selected_config, "team_flow_authority.selected_config")?;
    for key in [
        "dev_team",
        "flows",
        "roles",
        "resolved_projection",
        "source",
        "snapshot_parser_config",
        "raw_registries",
        "carrier_runtime",
        "project_node",
    ] {
        if selected_config_object.contains_key(key) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_legacy_source_rejected",
                format!("team_flow_authority.selected_config.{key}"),
                Vec::new(),
            ));
        }
    }
    exact_keys(
        selected_config_object,
        &[
            "schema_version",
            "config_id",
            "profile",
            "team_flow_enabled",
            "authority_selection",
            "registry_hash",
        ],
        "team_flow_authority.selected_config",
    )?;
    if required_string(
        selected_config,
        "schema_version",
        "team_flow_authority.selected_config",
    )? != "team-flow-authority.v1"
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_schema_invalid",
            "team_flow_authority.selected_config.schema_version",
            vec!["team-flow-authority.v1".to_string()],
        ));
    }
    let config_id = required_string(
        selected_config,
        "config_id",
        "team_flow_authority.selected_config",
    )?;
    let selected_profile = required_string(
        selected_config,
        "profile",
        "team_flow_authority.selected_config",
    )?;
    let team_flow_enabled = required_member(
        selected_config_object,
        "team_flow_enabled",
        "team_flow_authority.selected_config",
    )?
    .as_bool()
    .ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_field_invalid",
            "team_flow_authority.selected_config.team_flow_enabled",
            vec!["boolean".to_string()],
        )
    })?;
    if !team_flow_enabled {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_disabled",
            "team_flow_authority.selected_config.team_flow_enabled",
            vec!["true".to_string()],
        ));
    }
    let source_of_truth =
        required_member(authority_object, "source_of_truth", "team_flow_authority")?;
    let source_of_truth_object =
        required_object(source_of_truth, "team_flow_authority.source_of_truth")?;
    exact_keys(
        source_of_truth_object,
        &["options", "selection", "schema"],
        "team_flow_authority.source_of_truth",
    )?;
    for (key, expected) in [
        ("options", "dev_team.authority_catalog"),
        ("selection", "dev_team.authority_selection"),
        (
            "schema",
            "vida/config/schemas/team-flow-authority.schema.json",
        ),
    ] {
        if required_string(source_of_truth, key, "team_flow_authority.source_of_truth")? != expected
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_source_of_truth_invalid",
                format!("team_flow_authority.source_of_truth.{key}"),
                vec![expected.to_string()],
            ));
        }
    }
    let selection = required_member(
        selected_config_object,
        "authority_selection",
        "team_flow_authority.selected_config",
    )?;
    let selection_object = required_object(
        selection,
        "team_flow_authority.selected_config.authority_selection",
    )?;
    exact_keys(
        selection_object,
        &SELECTION_FIELDS,
        "team_flow_authority.selected_config.authority_selection",
    )?;
    for (key, expected) in [
        ("schema_version", "team-flow-authority.v1"),
        ("projection_mode", "typed_fail_closed"),
        ("registry_identity_algorithm", "canonical_json_blake3_v1"),
        ("terminal_source", "config_only"),
        ("edge_source", "explicit_config_only"),
        ("command_resolution_mode", "registry_ref_only"),
        ("approval_enforcement_mode", "required"),
        ("alias_conflict_policy", "reject"),
        ("node_field_source_mode", "typed_exact_one"),
        ("dispatch_alias_resolution_mode", "registry_ref_exactly_one"),
        ("carrier_relation_mode", "distinct_from_executor_backend"),
        ("profile_model_resolution_mode", "registry_identity_ref"),
    ] {
        if required_string(
            selection,
            key,
            "team_flow_authority.selected_config.authority_selection",
        )? != expected
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_selection_metadata_invalid",
                format!("team_flow_authority.selected_config.authority_selection.{key}"),
                vec![expected.to_string()],
            ));
        }
    }
    let selection_config_id = required_string(
        selection,
        "config_id",
        "team_flow_authority.selected_config.authority_selection",
    )?;
    if selection_config_id != config_id {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_config_identity_mismatch",
            config_id.clone(),
            vec![selection_config_id],
        ));
    }
    let default_flow_id = required_string(
        selection,
        "default_flow_id",
        "team_flow_authority.selected_config.authority_selection",
    )?;
    let selection_profile = required_string(
        selection,
        "team_profile_id",
        "team_flow_authority.selected_config.authority_selection",
    )?;
    if selection_profile != selected_profile {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_profile_conflict",
            selected_profile.clone(),
            vec![selection_profile],
        ));
    }
    let registry_hash = required_string(
        selected_config,
        "registry_hash",
        "team_flow_authority.selected_config",
    )?;
    let registry_identities =
        required_member(authority_object, "registries", "team_flow_authority")?;
    let registry_object = required_object(registry_identities, "team_flow_authority.registries")?;
    exact_keys(
        registry_object,
        &REGISTRY_NAMES,
        "team_flow_authority.registries",
    )?;
    for name in REGISTRY_NAMES {
        strict_identity(
            required_member(registry_object, name, "team_flow_authority.registries")?,
            &format!("team_flow_authority.registries.{name}"),
        )?;
    }
    let authority_seed = serde_json::json!({
        "config": config_object.clone(),
        "registries": registry_object.clone(),
        "selected_config": selected_config_object.clone()
    });
    let expected_authority_hash =
        taskflow_authority::team_flow_transition::hash_json(&authority_seed);
    if authority_content_hash != expected_authority_hash {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_content_hash_mismatch",
            "team_flow_authority.content_blake3",
            vec![expected_authority_hash.clone()],
        ));
    }
    let expected_authority_id = format!("team-flow-authority:{expected_authority_hash}");
    if authority_id != expected_authority_id {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_identity_mismatch",
            "team_flow_authority.authority_id",
            vec![expected_authority_id],
        ));
    }
    let payload = required_member(
        authority_object,
        "resolved_all_flow_payload",
        "team_flow_authority",
    )?;
    let payload_object = required_object(payload, "team_flow_authority.resolved_all_flow_payload")?;
    exact_keys(
        payload_object,
        &[
            "schema_version",
            "flow_count",
            "lane_count",
            "work_item_flow_bindings",
            "flows",
        ],
        "team_flow_authority.resolved_all_flow_payload",
    )?;
    if required_string(
        payload,
        "schema_version",
        "team_flow_authority.resolved_all_flow_payload",
    )? != "team-flow-authority.v1"
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_schema_invalid",
            "team_flow_authority.resolved_all_flow_payload.schema_version",
            vec!["team-flow-authority.v1".to_string()],
        ));
    }
    let persisted_work_item_flow_bindings = required_object(
        required_member(
            payload_object,
            "work_item_flow_bindings",
            "team_flow_authority.resolved_all_flow_payload",
        )?,
        "team_flow_authority.resolved_all_flow_payload.work_item_flow_bindings",
    )?;
    let mut work_item_flow_bindings = BTreeMap::new();
    for (work_item, target) in persisted_work_item_flow_bindings {
        let canonical_work_item = work_item.trim().to_ascii_lowercase();
        if canonical_work_item.is_empty() {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_field_invalid",
                "team_flow_authority.resolved_all_flow_payload.work_item_flow_bindings",
                Vec::new(),
            ));
        }
        if canonical_work_item != *work_item {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_work_item_flow_binding_key_noncanonical",
                format!(
                    "team_flow_authority.resolved_all_flow_payload.work_item_flow_bindings.{work_item}"
                ),
                vec![canonical_work_item],
            ));
        }
        let target = target
            .as_str()
            .map(str::trim)
            .filter(|target| !target.is_empty())
            .ok_or_else(|| {
                TeamFlowResolutionBlocker::new(
                    "team_flow_authority_persisted_field_invalid",
                    format!(
                        "team_flow_authority.resolved_all_flow_payload.work_item_flow_bindings.{work_item}"
                    ),
                    Vec::new(),
                )
            })?;
        if work_item_flow_bindings
            .insert(work_item.clone(), target.to_string())
            .is_some()
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_work_item_flow_binding_key_collision",
                format!(
                    "team_flow_authority.resolved_all_flow_payload.work_item_flow_bindings.{work_item}"
                ),
                Vec::new(),
            ));
        }
    }
    let payload_hash = required_string(
        authority,
        "resolved_all_flow_payload_blake3",
        "team_flow_authority",
    )?;
    let expected_payload_hash = taskflow_authority::team_flow_transition::hash_json(payload);
    if payload_hash != expected_payload_hash {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_payload_hash_mismatch",
            "team_flow_authority.resolved_all_flow_payload",
            vec![expected_payload_hash],
        ));
    }
    let authority_source =
        required_member(authority_object, "authority_source", "team_flow_authority")?;
    let authority_source_object =
        required_object(authority_source, "team_flow_authority.authority_source")?;
    exact_keys(
        authority_source_object,
        &["kind", "payload_path", "payload_blake3", "identity_phase"],
        "team_flow_authority.authority_source",
    )?;
    for (key, expected) in [
        ("kind", "resolved_all_flow_payload"),
        (
            "payload_path",
            "team_flow_authority.resolved_all_flow_payload",
        ),
        ("identity_phase", "phase_2_persisted_payload"),
    ] {
        if required_string(
            authority_source,
            key,
            "team_flow_authority.authority_source",
        )? != expected
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_source_invalid",
                format!("team_flow_authority.authority_source.{key}"),
                vec![expected.to_string()],
            ));
        }
    }
    if required_string(
        authority_source,
        "payload_blake3",
        "team_flow_authority.authority_source",
    )? != payload_hash
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_source_hash_mismatch",
            "team_flow_authority.authority_source.payload_blake3",
            vec![payload_hash.clone()],
        ));
    }
    let flow_rows = required_array(
        required_member(
            payload_object,
            "flows",
            "team_flow_authority.resolved_all_flow_payload",
        )?,
        "team_flow_authority.resolved_all_flow_payload.flows",
    )?;
    let flow_count = required_member(
        payload_object,
        "flow_count",
        "team_flow_authority.resolved_all_flow_payload",
    )?
    .as_u64()
    .ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_payload_field_invalid",
            "team_flow_authority.resolved_all_flow_payload.flow_count",
            Vec::new(),
        )
    })? as usize;
    let lane_count = required_member(
        payload_object,
        "lane_count",
        "team_flow_authority.resolved_all_flow_payload",
    )?
    .as_u64()
    .ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_payload_field_invalid",
            "team_flow_authority.resolved_all_flow_payload.lane_count",
            Vec::new(),
        )
    })? as usize;
    if flow_count == 0 || flow_rows.is_empty() || flow_rows.len() != flow_count {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_flow_count_mismatch",
            "team_flow_authority.resolved_all_flow_payload.flow_count",
            vec![flow_rows.len().to_string()],
        ));
    }
    if lane_count == 0 {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_lane_count_mismatch",
            "team_flow_authority.resolved_all_flow_payload.lane_count",
            Vec::new(),
        ));
    }
    let selected_flow = match flow_ref.map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        Some(_) | None => default_flow_id.clone(),
    };
    let mut flow_ids = BTreeSet::new();
    let mut selected_snapshot = None;
    let mut selected_nodes = None;
    let mut total_lanes = 0usize;
    for (index, flow) in flow_rows.iter().enumerate() {
        let path = format!("team_flow_authority.resolved_all_flow_payload.flows[{index}]");
        let flow_object = required_object(flow, &path)?;
        exact_keys(
            flow_object,
            &[
                "flow_id",
                "flow_identity",
                "flow_policy",
                "entry_node_id",
                "lanes",
            ],
            &path,
        )?;
        let flow_id = required_string(flow, "flow_id", &path)?;
        let entry_node_id = required_string(flow, "entry_node_id", &path)?;
        if !flow_ids.insert(flow_id.clone()) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_flow_identity_ambiguous",
                flow_id,
                Vec::new(),
            ));
        }
        let flow_identity = required_member(flow_object, "flow_identity", &path)?;
        let flow_identity_object =
            required_object(flow_identity, &format!("{path}.flow_identity"))?;
        exact_keys(
            flow_identity_object,
            &["kind", "id", "source_path"],
            &format!("{path}.flow_identity"),
        )?;
        if required_string(flow_identity, "kind", &format!("{path}.flow_identity"))? != "flow"
            || required_string(
                flow_identity,
                "source_path",
                &format!("{path}.flow_identity"),
            )? != format!("dev_team.flows.{flow_id}")
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_flow_identity_mismatch",
                flow_id,
                Vec::new(),
            ));
        }
        required_string(flow_identity, "id", &format!("{path}.flow_identity"))?;
        let flow_policy = strict_flow_policy(
            required_member(flow_object, "flow_policy", &path)?,
            &format!("{path}.flow_policy"),
        )?;
        if flow_id == selected_flow && flow_policy["enabled"].as_bool() == Some(false) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_flow_policy_disabled",
                format!("{path}.flow_policy.enabled"),
                vec!["true".to_string()],
            ));
        }
        let lanes = required_array(
            required_member(flow_object, "lanes", &path)?,
            &format!("{path}.lanes"),
        )?;
        if lanes.is_empty() {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_persisted_lane_count_mismatch",
                flow_id,
                Vec::new(),
            ));
        }
        let expected_flow_identity = deterministic_flow_identity_id(&flow_id, &flow_policy, lanes);
        if required_string(flow_identity, "id", &format!("{path}.flow_identity"))?
            != expected_flow_identity
        {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_flow_identity_mismatch",
                format!("{path}.flow_identity.id"),
                vec![expected_flow_identity],
            ));
        }
        total_lanes += lanes.len();
        let snapshot = persisted_snapshot(
            &flow_id,
            &entry_node_id,
            lanes,
            &config_id,
            &selected_profile,
            &config_hash,
            &registry_hash,
            registry_identities,
        )?;
        let mut nodes = Vec::with_capacity(lanes.len());
        for (lane_index, lane) in lanes.iter().enumerate() {
            nodes.push(parse_persisted_lane(
                lane,
                &format!("{path}.lanes[{lane_index}]"),
                registry_identities,
            )?);
        }
        if flow_id == selected_flow {
            selected_snapshot = Some(snapshot);
            selected_nodes = Some(nodes);
        }
    }
    for (work_item, target) in work_item_flow_bindings {
        if !flow_ids.contains(&target) {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_work_item_flow_binding_target_missing",
                format!(
                    "team_flow_authority.resolved_all_flow_payload.work_item_flow_bindings.{work_item}"
                ),
                flow_ids.iter().cloned().collect(),
            ));
        }
    }
    if total_lanes != lane_count {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_lane_count_mismatch",
            "team_flow_authority.resolved_all_flow_payload.lane_count",
            vec![total_lanes.to_string()],
        ));
    }
    let snapshot = selected_snapshot.ok_or_else(|| {
        TeamFlowResolutionBlocker::new(
            "team_flow_authority_unknown_flow",
            selected_flow.clone(),
            flow_ids.iter().cloned().collect(),
        )
    })?;
    if snapshot.flow_ref != selected_flow
        || snapshot.config_hash != config_hash
        || !snapshot.has_valid_identity()
    {
        return Err(TeamFlowResolutionBlocker::new(
            "team_flow_authority_persisted_snapshot_identity_mismatch",
            selected_flow,
            Vec::new(),
        ));
    }
    if let Some(requested_profile) = profile.map(str::trim).filter(|value| !value.is_empty()) {
        if requested_profile != selected_profile {
            return Err(TeamFlowResolutionBlocker::new(
                "team_flow_authority_profile_conflict",
                requested_profile,
                vec![selected_profile],
            ));
        }
    }
    let entry_node_id = snapshot.entry_node_id.clone();
    Ok(TeamFlowAuthorityProjection {
        snapshot,
        entry_node_id,
        authority_id,
        authority_content_hash,
        config_authority_hash: config_hash,
        registry_authority_hash: registry_hash,
        registry_identities: registry_identities.clone(),
        nodes: selected_nodes.ok_or_else(|| {
            TeamFlowResolutionBlocker::new(
                "team_flow_authority_selected_flow_lanes_missing",
                selected_flow,
                Vec::new(),
            )
        })?,
    })
}

pub(crate) fn team_flow_authority_availability(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> TeamFlowAuthorityAvailability {
    let Some(authority) = compiled_bundle.get("team_flow_authority") else {
        return TeamFlowAuthorityAvailability {
            status: TeamFlowAuthorityAvailabilityStatus::Unavailable,
            blocker: Some(TEAM_FLOW_AUTHORITY_UNAVAILABLE_BLOCKER.to_string()),
            projection: None,
        };
    };
    if authority.get("status").and_then(Value::as_str) == Some("unavailable") {
        return TeamFlowAuthorityAvailability {
            status: TeamFlowAuthorityAvailabilityStatus::Unavailable,
            blocker: Some(TEAM_FLOW_AUTHORITY_UNAVAILABLE_BLOCKER.to_string()),
            projection: None,
        };
    }
    if authority.get("status").and_then(Value::as_str) == Some("disabled") {
        return TeamFlowAuthorityAvailability {
            status: TeamFlowAuthorityAvailabilityStatus::Disabled,
            blocker: None,
            projection: None,
        };
    }
    match compile_persisted(compiled_bundle, flow_ref, profile) {
        Ok(projection) => TeamFlowAuthorityAvailability {
            status: TeamFlowAuthorityAvailabilityStatus::Ready,
            blocker: None,
            projection: Some(projection),
        },
        Err(error) => TeamFlowAuthorityAvailability {
            status: TeamFlowAuthorityAvailabilityStatus::Blocked,
            blocker: Some(error.code),
            projection: None,
        },
    }
}

pub(crate) fn require_team_flow_authority(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> Result<TeamFlowAuthorityProjection, String> {
    if team_flow_is_disabled(compiled_bundle) {
        return Err("team_flow_disabled".to_string());
    }
    compile_persisted(compiled_bundle, flow_ref, profile).map_err(|error| error.to_string())
}

pub(crate) fn require_team_flow_execution_authority(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> Result<TeamFlowExecutionAuthority, TeamFlowResolutionBlocker> {
    TeamFlowExecutionAuthority::require(compiled_bundle, flow_ref, profile)
}

fn team_flow_is_disabled(compiled_bundle: &Value) -> bool {
    compiled_bundle
        .get("team_flow_authority")
        .and_then(|authority| authority.get("status"))
        .and_then(Value::as_str)
        == Some("disabled")
}

pub(crate) fn resolve_team_flow_node(
    projection: &TeamFlowAuthorityProjection,
    execution_plan: Option<&Value>,
    requested: &str,
) -> Result<TeamFlowNodeResolution, TeamFlowResolutionBlocker> {
    validate_execution_plan(projection, execution_plan)?;
    resolve_node(projection, requested, LookupKind::Target)
}

pub(crate) fn resolve_team_flow_runtime_role(
    projection: &TeamFlowAuthorityProjection,
    execution_plan: Option<&Value>,
    runtime_role: &str,
) -> Result<TeamFlowNodeResolution, TeamFlowResolutionBlocker> {
    validate_execution_plan(projection, execution_plan)?;
    resolve_node(projection, runtime_role, LookupKind::RuntimeRole)
}

pub(crate) fn compile_team_flow_authority(
    compiled_bundle: &Value,
    flow_ref: Option<&str>,
    profile: Option<&str>,
) -> Result<TeamFlowAuthorityProjection, String> {
    require_team_flow_authority(compiled_bundle, flow_ref, profile)
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::{compile_persisted, resolve_node};
    pub(crate) use crate::team_flow_authority_projection::test_support::canonical_compiled_bundle;
    use serde_json::{json, Map, Value};

    #[derive(Debug, Clone)]
    pub(crate) struct ScenarioSpec {
        pub(crate) compiled_bundle: Value,
        pub(crate) dev_task_id: String,
        pub(crate) lane_catalog_override: Option<Map<String, Value>>,
        pub(crate) lane_sequence_override: Option<Vec<String>>,
    }

    pub(crate) fn canonical_scenario_spec(dev_task_id: &str) -> ScenarioSpec {
        let compiled_bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&compiled_bundle, None, None)
            .expect("canonical persisted bundle must compile");
        let mut lane_catalog = Map::new();
        let mut lane_sequence = Vec::new();
        for node in projection.ordered_nodes().filter(|node| node.node.included) {
            let resolution =
                resolve_node(&projection, &node.node.node_id, super::LookupKind::Target)
                    .expect("canonical persisted lane must resolve");
            lane_sequence.push(resolution.node_id.clone());
            lane_catalog.insert(
                resolution.node_id.clone(),
                json!({
                    "node_id": resolution.node_id,
                    "lane_id": resolution.lane_id,
                    "dispatch_target": resolution.dispatch_target,
                    "dispatch_alias": resolution.dispatch_alias,
                    "task_class": resolution.task_class,
                    "runtime_role": resolution.runtime_role,
                    "evidence_requirements": resolution.evidence_requirements,
                    "stage": resolution.stage,
                    "inclusion_rule": node.node.inclusion_rule,
                    "included": node.node.included,
                    "required": node.node.required,
                    "next_node": resolution.next_node,
                    "terminal": resolution.terminal,
                    "rework": {"targets": resolution.rework_targets},
                    "closure_class": resolution.closure_class,
                    "completion_blocker": resolution.completion_blocker,
                    "packet_template_kind": resolution.packet_template_kind,
                    "command_ref": resolution.command_ref,
                    "command_surface": resolution.command_surface,
                    "command_mapping": resolution.command_mapping,
                    "requires_user_approval": resolution.requires_user_approval,
                    "proof_gates": resolution.proof_gates,
                    "approval_policy": resolution.approval_policy,
                    "lifecycle_hook_templates": resolution.lifecycle_hook_templates,
                    "resume_transitions": resolution.resume_transitions,
                    "rework_transitions": resolution.rework_transitions,
                    "profile_authority": resolution.profile_authority,
                    "selected_model_profile": resolution.selected_model_profile,
                    "carrier_id": resolution.carrier_id,
                    "carrier_tier": resolution.carrier_tier,
                    "authority_identities": resolution.authority_identities,
                    "execution_identity": resolution.execution_identity,
                    "carrier_relation": resolution.carrier_relation,
                    "executor_backend_relation": resolution.executor_backend_relation,
                    "executor_backend_class": resolution.executor_backend_class,
                    "backend_relation": resolution.backend_relation,
                    "component_registry": resolution.component_registry,
                    "activation": resolution.activation,
                    "runtime_assignment": resolution.assignment,
                    "carrier_runtime_assignment": resolution.assignment,
                    "policy_diagnostics": {
                        "source": resolution.source,
                        "fallback_used": false,
                        "fallback_fields": []
                    }
                }),
            );
        }
        ScenarioSpec {
            compiled_bundle,
            dev_task_id: dev_task_id.to_string(),
            lane_catalog_override: Some(lane_catalog),
            lane_sequence_override: Some(lane_sequence),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compile_persisted, deterministic_flow_identity_id, require_team_flow_authority,
        resolve_team_flow_node, team_flow_authority_availability,
        test_support::canonical_compiled_bundle, TeamFlowAuthorityAvailabilityStatus,
        TeamFlowAuthorityProjection,
    };
    use serde_json::json;

    fn refresh_payload_hashes(bundle: &mut serde_json::Value) {
        let authority = bundle
            .get_mut("team_flow_authority")
            .expect("canonical authority");
        let flows = authority["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows");
        for flow in flows {
            let flow_id = flow["flow_id"]
                .as_str()
                .expect("persisted flow id")
                .to_string();
            let flow_policy = flow["flow_policy"].clone();
            let lanes = flow["lanes"].as_array().expect("persisted flow lanes");
            let identity_id = deterministic_flow_identity_id(&flow_id, &flow_policy, lanes);
            flow["flow_identity"]["id"] = json!(identity_id);
        }
        refresh_outer_hashes(authority);
    }

    fn refresh_outer_hashes(authority: &mut serde_json::Value) {
        let payload = authority
            .get("resolved_all_flow_payload")
            .cloned()
            .expect("persisted payload");
        let payload_hash = taskflow_authority::team_flow_transition::hash_json(&payload);
        authority["resolved_all_flow_payload_blake3"] = json!(payload_hash.clone());
        authority["authority_source"]["payload_blake3"] = json!(payload_hash);
        let authority_seed = json!({
            "config": authority["config"].clone(),
            "registries": authority["registries"].clone(),
            "selected_config": authority["selected_config"].clone(),
        });
        let authority_hash = taskflow_authority::team_flow_transition::hash_json(&authority_seed);
        authority["content_blake3"] = json!(authority_hash.clone());
        authority["authority_id"] = json!(format!("team-flow-authority:{authority_hash}"));
    }

    #[test]
    fn disabled_team_flow_is_non_blocking_for_availability() {
        let bundle = json!({
            "team_flow_authority": {
                "status": "disabled",
                "enabled": false,
                "reason": "dev_team_disabled"
            }
        });
        let availability = team_flow_authority_availability(&bundle, None, None);
        assert_eq!(
            availability.status,
            TeamFlowAuthorityAvailabilityStatus::Disabled
        );
        assert!(availability.blocker.is_none());
        assert!(availability.projection.is_none());
        assert!(matches!(
            require_team_flow_authority(&bundle, None, None),
            Err(error) if error == "team_flow_disabled"
        ));
    }

    fn persisted_lane_mut<'a>(
        bundle: &'a mut serde_json::Value,
        node_id: &str,
    ) -> &'a mut serde_json::Value {
        let flows = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows");
        for flow in flows {
            let lanes = flow["lanes"].as_array_mut().expect("persisted lanes");
            if let Some(index) = lanes
                .iter()
                .position(|lane| lane["node_id"].as_str() == Some(node_id))
            {
                return &mut lanes[index];
            }
        }
        panic!("persisted lane must exist for node {node_id}");
    }

    fn selected_node_execution_plan(
        projection: &TeamFlowAuthorityProjection,
        top_level_node_id: Option<&str>,
        contract_node_id: Option<&str>,
        contract_authority_node_id: Option<&str>,
        compatibility: Option<&str>,
    ) -> serde_json::Value {
        let mut contract = json!({
            "team_flow_authority_id": projection.authority_id.clone(),
            "team_flow_config_hash": projection.config_authority_hash.clone(),
            "team_flow_registry_hash": projection.registry_authority_hash.clone(),
        });
        if let Some(node_id) = contract_node_id {
            contract["selected_node_id"] = json!(node_id);
        }
        if let Some(node_id) = contract_authority_node_id {
            contract["team_flow_authority_selected_node_id"] = json!(node_id);
        }
        let mut plan = json!({
            "development_flow": {
                "dispatch_contract": contract,
            }
        });
        if let Some(node_id) = top_level_node_id {
            plan["team_flow_authority_selected_node_id"] = json!(node_id);
        }
        if let Some(marker) = compatibility {
            plan["team_flow_authority_selected_node_id_compatibility"] = json!(marker);
        }
        plan
    }

    fn canonical_selected_nodes(
        projection: &TeamFlowAuthorityProjection,
    ) -> (String, String, String) {
        let nodes = projection
            .ordered_nodes()
            .filter(|node| node.node.included)
            .collect::<Vec<_>>();
        let first = nodes
            .first()
            .expect("canonical flow must expose an included node")
            .node
            .node_id
            .clone();
        let alternate = nodes
            .iter()
            .find(|node| node.node.node_id != first)
            .expect("canonical flow must expose a second included node")
            .node
            .node_id
            .clone();
        let excluded = projection
            .nodes
            .iter()
            .find(|node| !node.node.included)
            .expect("canonical authority must expose an excluded node")
            .node
            .node_id
            .clone();
        (first, alternate, excluded)
    }

    #[test]
    fn exact_node_id_resolution_wins_over_duplicate_dispatch_alias() {
        let bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let (alias, exact_node_id) = projection
            .nodes
            .iter()
            .filter(|node| node.node.included)
            .find_map(|node| {
                projection
                    .nodes
                    .iter()
                    .find(|other| {
                        other.node.included
                            && other.node.node_id != node.node.node_id
                            && other.dispatch_alias == node.dispatch_alias
                    })
                    .map(|_| (node.dispatch_alias.clone(), node.node.node_id.clone()))
            })
            .expect("canonical authority must expose a duplicate dispatch alias");

        let exact = resolve_team_flow_node(&projection, None, &exact_node_id)
            .expect("exact node id must resolve despite duplicate alias");
        assert_eq!(exact.node_id, exact_node_id);

        let error = resolve_team_flow_node(&projection, None, &alias)
            .expect_err("duplicate alias fallback must remain ambiguous");
        assert_eq!(error.code, "team_flow_node_resolution_ambiguous");
    }

    #[test]
    fn persisted_edges_require_exact_configured_node_ids() {
        let mut bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let source = projection
            .nodes
            .iter()
            .find(|node| node.node.next_node.is_some())
            .expect("canonical authority must expose a nonterminal node");
        let target_id = source
            .node
            .next_node
            .as_deref()
            .expect("nonterminal node must have next node")
            .to_string();
        let target_alias = projection
            .node(&target_id)
            .expect("next node must exist")
            .dispatch_alias
            .clone();
        assert_eq!(
            projection
                .snapshot
                .node(&source.node.node_id)
                .and_then(|node| node.next_node.as_deref()),
            Some(target_id.as_str())
        );
        let source_id = source.node.node_id.clone();
        let lane = persisted_lane_mut(&mut bundle, &source_id);
        lane["next_node"] = json!(target_alias);
        refresh_payload_hashes(&mut bundle);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("persisted aliases must not be canonicalized into node ids");
        assert_eq!(
            error.code,
            "team_flow_authority_persisted_snapshot_edge_invalid"
        );
        assert!(error.requested.ends_with(".next_node"));

        let mut unknown = canonical_compiled_bundle();
        let projection = compile_persisted(&unknown, None, None)
            .expect("canonical persisted authority must compile");
        let source_id = projection
            .nodes
            .iter()
            .find(|node| node.node.next_node.is_some())
            .expect("canonical authority must expose a nonterminal node")
            .node
            .node_id
            .clone();
        let lane = persisted_lane_mut(&mut unknown, &source_id);
        lane["next_node"] = json!("unknown-persisted-node");
        refresh_payload_hashes(&mut unknown);
        let error = compile_persisted(&unknown, None, None)
            .expect_err("unknown persisted identity must fail closed");
        assert_eq!(
            error.code,
            "team_flow_authority_persisted_snapshot_edge_invalid"
        );
    }

    #[test]
    fn persisted_rework_targets_require_exact_configured_node_ids() {
        let mut bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let source = projection
            .nodes
            .iter()
            .find(|node| !node.node.rework_targets.is_empty())
            .expect("canonical authority must expose a rework target");
        let target_id = source.node.rework_targets[0].clone();
        let target_alias = projection
            .node(&target_id)
            .expect("rework target must exist")
            .dispatch_alias
            .clone();
        assert!(projection
            .snapshot
            .node(&source.node.node_id)
            .expect("source node must exist")
            .rework_targets
            .iter()
            .any(|target| target == &target_id));
        let source_id = source.node.node_id.clone();
        let lane = persisted_lane_mut(&mut bundle, &source_id);
        lane["rework"]["targets"] = json!([target_alias]);
        refresh_payload_hashes(&mut bundle);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("persisted rework aliases must fail closed");
        assert_eq!(
            error.code,
            "team_flow_authority_persisted_snapshot_edge_invalid"
        );
        assert!(error.requested.ends_with(".rework"));
    }

    #[test]
    fn fresh_alias_boundary_rejects_node_id_collision_but_accepts_unique_alias() {
        let mut bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let (first, alternate, _) = canonical_selected_nodes(&projection);
        let lane = persisted_lane_mut(&mut bundle, &alternate);
        lane["dispatch_alias"] = json!(first);
        refresh_payload_hashes(&mut bundle);
        let collision_projection = compile_persisted(&bundle, None, None)
            .expect("alias collision is a Fresh lookup concern, not persisted edge normalization");
        let error = resolve_team_flow_node(&collision_projection, None, &first)
            .expect_err("Fresh alias/node-id collision must fail closed");
        assert_eq!(error.code, "team_flow_node_resolution_identity_collision");
        assert!(error.candidates.contains(&alternate));

        let mut fresh = canonical_compiled_bundle();
        let projection = compile_persisted(&fresh, None, None)
            .expect("canonical persisted authority must compile");
        let fresh_node = projection
            .nodes
            .first()
            .expect("canonical authority must expose a node")
            .node
            .node_id
            .clone();
        let fresh_alias = "fresh-external-alias";
        let lane = persisted_lane_mut(&mut fresh, &fresh_node);
        lane["dispatch_alias"] = json!(fresh_alias);
        refresh_payload_hashes(&mut fresh);
        let projection = compile_persisted(&fresh, None, None)
            .expect("unique Fresh alias must remain externally resolvable");
        let resolution = resolve_team_flow_node(&projection, None, fresh_alias)
            .expect("Fresh alias boundary must resolve unique aliases");
        assert_eq!(resolution.node_id, fresh_node);
    }

    #[test]
    fn execution_plan_selected_node_rejects_alternate_valid_node_tamper() {
        let bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let (first, alternate, _) = canonical_selected_nodes(&projection);
        let plan = selected_node_execution_plan(
            &projection,
            Some(&alternate),
            Some(&first),
            Some(&first),
            None,
        );
        let error = resolve_team_flow_node(&projection, Some(&plan), &first)
            .expect_err("alternate valid selected node tamper must fail closed");
        assert_eq!(error.code, "team_flow_authority_selected_node_id_conflict");
    }

    #[test]
    fn execution_plan_selected_node_rejects_conflicting_identity_fields() {
        let bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let (first, alternate, _) = canonical_selected_nodes(&projection);
        let plan = selected_node_execution_plan(
            &projection,
            Some(&first),
            Some(&alternate),
            Some(&first),
            None,
        );
        let error = resolve_team_flow_node(&projection, Some(&plan), &first)
            .expect_err("conflicting selected node fields must fail closed");
        assert_eq!(error.code, "team_flow_authority_selected_node_id_conflict");
    }

    #[test]
    fn execution_plan_selected_node_rejects_missing_unknown_and_excluded_identity() {
        let bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let (first, _, excluded) = canonical_selected_nodes(&projection);

        let missing = selected_node_execution_plan(&projection, None, None, None, None);
        let error = resolve_team_flow_node(&projection, Some(&missing), &first)
            .expect_err("missing selected node identity must fail closed");
        assert_eq!(error.code, "team_flow_authority_selected_node_id_missing");

        let unknown = selected_node_execution_plan(
            &projection,
            Some("unknown-selected-node"),
            Some("unknown-selected-node"),
            Some("unknown-selected-node"),
            None,
        );
        let error = resolve_team_flow_node(&projection, Some(&unknown), &first)
            .expect_err("unknown selected node identity must fail closed");
        assert_eq!(error.code, "team_flow_authority_selected_node_id_unknown");

        let excluded_plan = selected_node_execution_plan(
            &projection,
            Some(&excluded),
            Some(&excluded),
            Some(&excluded),
            None,
        );
        let error = resolve_team_flow_node(&projection, Some(&excluded_plan), &first)
            .expect_err("excluded selected node identity must fail closed");
        assert_eq!(error.code, "team_flow_authority_selected_node_id_excluded");
    }

    #[test]
    fn execution_plan_selected_node_allows_explicit_legacy_initial_compatibility_only() {
        let bundle = canonical_compiled_bundle();
        let projection = compile_persisted(&bundle, None, None)
            .expect("canonical persisted authority must compile");
        let (first, _, _) = canonical_selected_nodes(&projection);
        let plan = selected_node_execution_plan(
            &projection,
            None,
            None,
            None,
            Some(
                crate::runtime_dispatch_state::TEAM_FLOW_SELECTED_NODE_LEGACY_INITIAL_COMPATIBILITY,
            ),
        );
        resolve_team_flow_node(&projection, Some(&plan), &first)
            .expect("explicit legacy initial marker permits omitted node identity");
    }

    #[test]
    fn optional_proof_gates_allow_empty_derived_evidence() {
        let mut bundle = canonical_compiled_bundle();
        let lane =
            &mut bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["lanes"][0];
        lane["required"] = json!(false);
        lane["proof_gates"]["required_outputs"] = json!([]);
        lane["evidence_requirements"] = json!([]);
        refresh_payload_hashes(&mut bundle);
        compile_persisted(&bundle, None, None).expect("optional empty proof must remain valid");
    }

    #[test]
    fn evidence_requirements_tamper_is_rejected_after_hash_refresh() {
        let mut bundle = canonical_compiled_bundle();
        let lane =
            &mut bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["lanes"][0];
        lane["evidence_requirements"] = json!(["opaque-tampered-evidence"]);
        refresh_payload_hashes(&mut bundle);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("evidence requirements must derive exactly from proof gates");
        assert_eq!(
            error.code,
            "team_flow_authority_persisted_evidence_requirements_conflict"
        );
    }

    #[test]
    fn selected_model_profile_shape_requires_configured_provider_fields() {
        let mut bundle = canonical_compiled_bundle();
        let lane =
            &mut bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["lanes"][0];
        lane["selected_model_profile"]
            .as_object_mut()
            .expect("model profile")
            .remove("provider");
        refresh_payload_hashes(&mut bundle);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("model profile provider must remain persisted");
        assert_eq!(error.code, "team_flow_authority_persisted_field_missing");
    }

    #[test]
    fn flow_identity_tamper_is_rejected_after_payload_hash_refresh() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["flow_identity"]
            ["id"] = json!("team-flow-flow:tampered");
        refresh_outer_hashes(&mut bundle["team_flow_authority"]);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("flow identity must be deterministic from persisted flow payload");
        assert_eq!(error.code, "team_flow_authority_flow_identity_mismatch");
    }

    #[test]
    fn selected_config_requires_team_flow_enabled() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["selected_config"]
            .as_object_mut()
            .expect("selected config")
            .remove("team_flow_enabled");
        refresh_outer_hashes(&mut bundle["team_flow_authority"]);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("persisted global TeamFlow policy must be explicit");
        assert_eq!(error.code, "team_flow_authority_persisted_schema_invalid");
    }

    #[test]
    fn selected_config_rejects_invalid_team_flow_enabled() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["selected_config"]["team_flow_enabled"] = json!("enabled");
        refresh_outer_hashes(&mut bundle["team_flow_authority"]);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("persisted global TeamFlow policy must be a boolean");
        assert_eq!(error.code, "team_flow_authority_persisted_field_invalid");
    }

    #[test]
    fn disabled_nondefault_flow_persists_but_cannot_be_selected() {
        let mut bundle = canonical_compiled_bundle();
        let default_flow = bundle["team_flow_authority"]["selected_config"]["authority_selection"]
            ["default_flow_id"]
            .as_str()
            .expect("default flow id")
            .to_string();
        let flows = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows");
        let disabled_flow = flows
            .iter_mut()
            .find(|flow| flow["flow_id"].as_str() != Some(default_flow.as_str()))
            .expect("canonical bundle must contain a non-default flow");
        let disabled_flow_id = disabled_flow["flow_id"]
            .as_str()
            .expect("disabled flow id")
            .to_string();
        disabled_flow["flow_policy"]["enabled"] = json!(false);
        refresh_payload_hashes(&mut bundle);

        compile_persisted(&bundle, None, None)
            .expect("disabled non-default flow must remain reloadable");
        let error = compile_persisted(&bundle, Some(&disabled_flow_id), None)
            .expect_err("disabled flow selection must fail closed");
        assert_eq!(error.code, "team_flow_authority_flow_policy_disabled");
    }

    #[test]
    fn persisted_work_item_flow_bindings_reject_extra_non_string_and_unknown_target() {
        let mut extra = canonical_compiled_bundle();
        extra["team_flow_authority"]["resolved_all_flow_payload"]["unexpected"] = json!(true);
        refresh_payload_hashes(&mut extra);
        assert_eq!(
            compile_persisted(&extra, None, None)
                .expect_err("payload extras must fail")
                .code,
            "team_flow_authority_persisted_schema_invalid"
        );

        let mut non_string = canonical_compiled_bundle();
        non_string["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            ["defect"] = json!(7);
        refresh_payload_hashes(&mut non_string);
        assert_eq!(
            compile_persisted(&non_string, None, None)
                .expect_err("binding values must be strings")
                .code,
            "team_flow_authority_persisted_field_invalid"
        );

        let mut unknown = canonical_compiled_bundle();
        unknown["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            ["defect"] = json!("missing-flow");
        refresh_payload_hashes(&mut unknown);
        assert_eq!(
            compile_persisted(&unknown, None, None)
                .expect_err("binding targets must exist")
                .code,
            "team_flow_authority_work_item_flow_binding_target_missing"
        );
    }

    #[test]
    fn persisted_work_item_flow_binding_keys_must_be_canonical_lowercase() {
        let mut mixed_case = canonical_compiled_bundle();
        let binding = mixed_case["team_flow_authority"]["resolved_all_flow_payload"]
            ["work_item_flow_bindings"]
            .as_object_mut()
            .expect("persisted bindings")
            .remove("defect")
            .expect("canonical defect binding");
        mixed_case["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            ["Defect"] = binding;
        refresh_payload_hashes(&mut mixed_case);
        let error = compile_persisted(&mixed_case, None, None)
            .expect_err("mixed-case persisted key must fail closed");
        assert_eq!(
            error.code,
            "team_flow_authority_persisted_work_item_flow_binding_key_noncanonical"
        );
    }

    #[test]
    fn nested_proof_gate_extras_are_rejected_after_payload_hash_refresh() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["lanes"][0]
            ["proof_gates"]["unexpected"] = json!(true);
        refresh_payload_hashes(&mut bundle);
        let error =
            compile_persisted(&bundle, None, None).expect_err("proof gate extras must fail closed");
        assert_eq!(error.code, "team_flow_authority_persisted_schema_invalid");
    }

    #[test]
    fn nested_rework_extras_are_rejected_after_payload_hash_refresh() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["lanes"][0]
            ["rework"]["unexpected"] = json!("tampered");
        refresh_payload_hashes(&mut bundle);
        let error =
            compile_persisted(&bundle, None, None).expect_err("rework extras must fail closed");
        assert_eq!(error.code, "team_flow_authority_persisted_schema_invalid");
    }

    #[test]
    fn flow_policy_extras_are_rejected_after_payload_hash_refresh() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["flow_policy"]
            ["unexpected"] = json!(true);
        refresh_payload_hashes(&mut bundle);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("flow policy extras must fail closed");
        assert_eq!(error.code, "team_flow_authority_persisted_schema_invalid");
    }

    #[test]
    fn persisted_role_index_mismatch_is_rejected_after_payload_hash_refresh() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]["lanes"][0]
            ["profile_authority"]["team_role_id"] = json!("tampered-role");
        refresh_payload_hashes(&mut bundle);
        let error = compile_persisted(&bundle, None, None)
            .expect_err("profile authority role key must match node identity");
        assert_eq!(
            error.code,
            "team_flow_authority_role_index_identity_mismatch"
        );
    }

    #[test]
    fn materialized_authority_reload_is_idempotent() {
        let bundle = canonical_compiled_bundle();
        let authority = bundle["team_flow_authority"].clone();
        let reloaded = serde_json::json!({"team_flow_authority": authority});
        assert_eq!(
            reloaded["team_flow_authority"]["selected_config"]["team_flow_enabled"],
            true
        );
        assert!(reloaded["team_flow_authority"]["resolved_all_flow_payload"]
            ["work_item_flow_bindings"]
            .as_object()
            .is_some_and(|bindings| !bindings.is_empty()));
        let first =
            compile_persisted(&reloaded, None, None).expect("materialized authority should reload");
        let second = compile_persisted(&reloaded, None, None)
            .expect("materialized authority should reload repeatedly");
        assert_eq!(first.snapshot, second.snapshot);
        assert_eq!(first.authority_id, second.authority_id);
        assert_eq!(first.authority_content_hash, second.authority_content_hash);
        assert_eq!(first.nodes.len(), second.nodes.len());
    }

    #[test]
    fn require_team_flow_authority_retains_persisted_blocker_path() {
        let mut bundle = canonical_compiled_bundle();
        bundle["team_flow_authority"]
            .as_object_mut()
            .expect("canonical materialized authority must be an object")
            .remove("authority_id")
            .expect("canonical materialized authority must persist authority_id");

        let error = super::require_team_flow_authority(&bundle, None, None)
            .expect_err("missing persisted authority field must fail closed");

        assert!(
            error.starts_with("team_flow_authority_persisted_field_missing: "),
            "blocker must retain its stable code prefix: {error}"
        );
        assert!(
            error.contains("team_flow_authority.authority_id"),
            "blocker must retain its requested JSON path: {error}"
        );
    }

    #[test]
    fn entry_node_projection_is_typed_and_rejects_unknown_or_excluded_entries() {
        let canonical = canonical_compiled_bundle();
        let projection =
            compile_persisted(&canonical, None, None).expect("canonical authority must compile");
        assert_eq!(projection.entry_node_id, projection.snapshot.entry_node_id);

        let alternate = projection
            .ordered_nodes()
            .find(|node| node.node.included && node.node.node_id != projection.entry_node_id)
            .expect("canonical flow must expose an alternate included entry")
            .node
            .node_id
            .clone();
        let mut alternate_bundle = canonical.clone();
        alternate_bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows")
            .iter_mut()
            .find(|flow| flow["flow_id"] == projection.snapshot.flow_ref)
            .expect("selected flow")
            .as_object_mut()
            .expect("flow object")
            .insert("entry_node_id".to_string(), json!(alternate));
        refresh_payload_hashes(&mut alternate_bundle);
        let alternate_projection = compile_persisted(&alternate_bundle, None, None)
            .expect("alternate configured entry must compile");
        assert_eq!(alternate_projection.entry_node_id, alternate);

        let mut unknown_bundle = canonical.clone();
        unknown_bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows")
            .iter_mut()
            .find(|flow| flow["flow_id"] == projection.snapshot.flow_ref)
            .expect("selected flow")
            .as_object_mut()
            .expect("flow object")
            .insert("entry_node_id".to_string(), json!("unknown-entry-node"));
        refresh_payload_hashes(&mut unknown_bundle);
        assert_eq!(
            compile_persisted(&unknown_bundle, None, None)
                .expect_err("unknown entry must fail closed")
                .code,
            "team_flow_authority_entry_node_unknown"
        );

        let excluded = projection
            .nodes
            .iter()
            .find(|node| !node.node.included)
            .expect("canonical flow must expose an excluded node")
            .node
            .node_id
            .clone();
        let mut excluded_bundle = canonical;
        excluded_bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows")
            .iter_mut()
            .find(|flow| flow["flow_id"] == projection.snapshot.flow_ref)
            .expect("selected flow")
            .as_object_mut()
            .expect("flow object")
            .insert("entry_node_id".to_string(), json!(excluded));
        refresh_payload_hashes(&mut excluded_bundle);
        assert_eq!(
            compile_persisted(&excluded_bundle, None, None)
                .expect_err("excluded entry must fail closed")
                .code,
            "team_flow_authority_entry_node_excluded"
        );
    }

    #[test]
    fn zombie_d_all_materialized_configured_flows_compile_by_explicit_flow_id() {
        let bundle = canonical_compiled_bundle();
        let flow_ids = bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("canonical materialized authority must persist flows")
            .iter()
            .map(|flow| {
                flow.get("flow_id")
                    .and_then(serde_json::Value::as_str)
                    .filter(|flow_id| !flow_id.trim().is_empty())
                    .expect("every materialized flow must persist a non-empty flow_id")
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert!(
            !flow_ids.is_empty(),
            "canonical materialized authority must persist at least one flow"
        );

        for flow_id in flow_ids {
            compile_persisted(&bundle, Some(&flow_id), None).unwrap_or_else(|error| {
                panic!(
                    "materialized flow `{flow_id}` must compile by explicit flow_id: code={}; requested={}; candidates={:?}",
                    error.code, error.requested, error.candidates
                )
            });
        }
    }
}
