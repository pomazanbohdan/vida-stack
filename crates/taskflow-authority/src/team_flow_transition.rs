//! Typed, config-bound TeamFlow transition authority.
//!
//! This module owns transition admission for one immutable flow snapshot. It
//! does not select agents, models, runtime roles, commands, or profiles. Those
//! values must be supplied by the configured flow/registry view and are bound
//! into the snapshot identity before a receipt can authorize a transition.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MODULE: &str = "team_flow_transition";

pub const BLOCKER_MISSING_SNAPSHOT_CONFIG_MAPPING: &str =
    "team_flow_snapshot_config_mapping_missing";
pub const BLOCKER_CONFIG_HASH_MISMATCH: &str = "team_flow_config_hash_mismatch";
pub const BLOCKER_REGISTRY_HASH_MISMATCH: &str = "team_flow_registry_hash_mismatch";
pub const BLOCKER_PROFILE_DRIFT: &str = "team_flow_profile_drift";
pub const BLOCKER_SNAPSHOT_HASH_MISMATCH: &str = "team_flow_snapshot_hash_mismatch";
pub const BLOCKER_SNAPSHOT_INTEGRITY_INVALID: &str = "team_flow_snapshot_integrity_invalid";
pub const BLOCKER_CURRENT_NODE_MISSING: &str = "team_flow_current_node_missing";
pub const BLOCKER_CURRENT_NODE_UNKNOWN: &str = "team_flow_current_node_unknown";
pub const BLOCKER_CURRENT_NODE_NOT_INCLUDED: &str = "team_flow_current_node_not_included";
pub const BLOCKER_RECEIPT_REQUIRED: &str = "team_flow_receipt_required";
pub const BLOCKER_RECEIPT_NODE_MISMATCH: &str = "team_flow_receipt_node_mismatch";
pub const BLOCKER_RECEIPT_NOT_COMPLETED: &str = "team_flow_receipt_not_completed";
pub const BLOCKER_USER_APPROVAL_REQUIRED: &str = "team_flow_user_approval_required";
pub const BLOCKER_REQUIRED_EVIDENCE_MISSING: &str = "team_flow_required_evidence_missing";
pub const BLOCKER_INVALID_REQUESTED_NODE: &str = "team_flow_invalid_requested_node";
pub const BLOCKER_TERMINAL_TRANSITION_NOT_CONFIGURED: &str =
    "team_flow_terminal_transition_not_configured";
pub const BLOCKER_REWORK_TARGET_NOT_CONFIGURED: &str = "team_flow_rework_target_not_configured";

/// A semantic evidence item. Receipts store normalized evidence ids in
/// `Vec<String>`; this type is available to callers that need a typed proof
/// value at the authority boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFlowEvidence {
    pub evidence_id: String,
    pub kind: String,
    pub value: String,
}

impl TeamFlowEvidence {
    pub fn new(evidence_id: &str, kind: &str, value: &str) -> Option<Self> {
        let evidence_id = nonempty(evidence_id)?;
        let kind = nonempty(kind)?;
        let value = nonempty(value)?;
        Some(Self {
            evidence_id,
            kind,
            value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFlowNode {
    pub node_id: String,
    pub runtime_role: String,
    pub task_class: String,
    pub inclusion_rule: String,
    pub included: bool,
    pub required: bool,
    pub next_node: Option<String>,
    pub rework_targets: Vec<String>,
    pub evidence_requirements: Vec<String>,
    pub command_ref: Option<String>,
    pub command_mapping_hash: Option<String>,
    pub requires_user_approval: bool,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFlowSnapshot {
    pub config_id: String,
    pub profile: String,
    pub flow_ref: String,
    pub config_hash: String,
    pub registry_hash: String,
    pub snapshot_ref: String,
    pub entry_node_id: String,
    pub ordered_nodes: Vec<String>,
    pub nodes: Vec<TeamFlowNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamFlowSnapshotInput<'a> {
    pub config_id: &'a str,
    pub profile: &'a str,
    pub flow_ref: &'a str,
    pub registry_hash: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFlowReceipt {
    pub receipt_id: String,
    pub node_id: String,
    pub status: String,
    pub evidence: Vec<String>,
    pub config_hash: String,
    pub snapshot_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptOutcome {
    Completed,
    Approved,
    Rework,
}

/// Canonicalize the supported receipt status protocol. Unknown or untyped
/// values are rejected rather than treated as successful completion.
#[must_use]
pub fn normalize_receipt_outcome(status: &str) -> Option<ReceiptOutcome> {
    match status.trim().to_ascii_lowercase().as_str() {
        "pass" | "completed" => Some(ReceiptOutcome::Completed),
        "approve" | "approved" => Some(ReceiptOutcome::Approved),
        "rework" => Some(ReceiptOutcome::Rework),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionVerdict {
    pub allowed: bool,
    pub blocker: Option<String>,
    pub blocker_codes: Vec<String>,
    pub expected: Option<String>,
    pub current: String,
    pub requested: String,
    pub next_node: Option<String>,
    pub rework: bool,
    pub required_remaining: Vec<String>,
    pub evidence_requirements: Vec<String>,
    pub config_hash: String,
    pub snapshot_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TeamFlowSnapshotError {
    #[error("team-flow snapshot field `{0}` is missing or empty")]
    MissingField(&'static str),
    #[error("team-flow snapshot config identity mismatch: expected `{expected}`, got `{actual}`")]
    ConfigIdentityMismatch { expected: String, actual: String },
    #[error("team-flow snapshot profile mismatch: expected `{expected}`, got `{actual}`")]
    ProfileMismatch { expected: String, actual: String },
    #[error("team-flow snapshot flow identity mismatch: expected `{expected}`, got `{actual}`")]
    FlowIdentityMismatch { expected: String, actual: String },
    #[error("team-flow snapshot flow has no configured steps")]
    EmptyFlow,
    #[error("team-flow snapshot node `{0}` is duplicated")]
    DuplicateNode(String),
    #[error("team-flow snapshot node at index {0} is not an object")]
    InvalidNode(usize),
    #[error("team-flow snapshot node `{node}` field `{field}` is missing or empty")]
    MissingNodeField { node: String, field: &'static str },
    #[error("team-flow snapshot node `{node}` field `{field}` has an invalid type")]
    InvalidNodeFieldType { node: String, field: &'static str },
    #[error("team-flow snapshot aliases for `{field}` conflict: {values:?}")]
    ConflictingAliases {
        field: &'static str,
        values: Vec<String>,
    },
    #[error(
        "team-flow snapshot node `{node}` task class `{task_class}` is not declared in role capabilities"
    )]
    TaskClassNotInRoleCapabilities { node: String, task_class: String },
    #[error(
        "team-flow snapshot node `{node}` command ref `{command_ref}` does not resolve in the command catalog"
    )]
    UnresolvedCommandReference { node: String, command_ref: String },
    #[error("team-flow snapshot sequence field `{field}` is missing, malformed, or inconsistent")]
    InvalidSequence { field: &'static str },
    #[error(
        "team-flow snapshot node `{node}` approval contract field `{field}` is missing or inconsistent"
    )]
    InvalidApprovalContract { node: String, field: &'static str },
    #[error("team-flow snapshot node `{node}` has malformed evidence field `{field}`")]
    MalformedEvidence { node: String, field: &'static str },
    #[error("team-flow snapshot node `{node}` points to unknown node `{target}`")]
    InvalidEdge { node: String, target: String },
    #[error("team-flow snapshot node `{0}` has a transition cycle")]
    TransitionCycle(String),
    #[error("terminal team-flow node `{0}` cannot point to another node")]
    InvalidTerminalEdge(String),
}

impl TeamFlowSnapshot {
    /// Compile one immutable snapshot from a configured flow and registry.
    /// No role, flow, command, or terminal identifier is invented here.
    pub fn from_config(
        config: &Value,
        input: TeamFlowSnapshotInput<'_>,
    ) -> Result<Self, TeamFlowSnapshotError> {
        let config_id = required_metadata(input.config_id, "config_id")?;
        let profile = required_metadata(input.profile, "profile")?;
        let flow_ref = required_metadata(input.flow_ref, "flow_ref")?;
        let registry_hash = required_metadata(input.registry_hash, "registry_hash")?;
        let flow = select_flow(config, &flow_ref)?;
        let flow_id = strict_string_aliases(flow, &["flow_id", "id"], "flow_id", "flow")?
            .ok_or(TeamFlowSnapshotError::MissingField("flow_id"))?;
        if flow_id != flow_ref {
            return Err(TeamFlowSnapshotError::FlowIdentityMismatch {
                expected: flow_ref,
                actual: flow_id,
            });
        }
        if let Some(selected_flow) = strict_string_aliases(
            config,
            &["selected_flow_set", "selected_flow_id"],
            "selected_flow_set",
            "config",
        )? {
            if selected_flow != flow_ref {
                return Err(TeamFlowSnapshotError::FlowIdentityMismatch {
                    expected: flow_ref.clone(),
                    actual: selected_flow,
                });
            }
        }
        if let Some(config_value) =
            strict_string_aliases(config, &["config_id", "id"], "config_id", "config")?
        {
            if config_value != config_id {
                return Err(TeamFlowSnapshotError::ConfigIdentityMismatch {
                    expected: config_id,
                    actual: config_value,
                });
            }
        }
        if let Some(profile_value) =
            strict_string_aliases(config, &["profile", "profile_id"], "profile", "config")?
        {
            if profile_value != profile {
                return Err(TeamFlowSnapshotError::ProfileMismatch {
                    expected: profile,
                    actual: profile_value,
                });
            }
        }
        if let Some(registry_value) =
            strict_string_aliases(config, &["registry_hash"], "registry_hash", "config")?
        {
            if registry_value != registry_hash {
                return Err(TeamFlowSnapshotError::ConfigIdentityMismatch {
                    expected: registry_hash,
                    actual: registry_value,
                });
            }
        }
        let steps = configured_steps(flow)?;
        if steps.is_empty() {
            return Err(TeamFlowSnapshotError::EmptyFlow);
        }
        let entry_node_id =
            strict_string_aliases(flow, &["entry_node_id"], "entry_node_id", "flow")?
                .ok_or(TeamFlowSnapshotError::MissingField("entry_node_id"))?;

        let roles_value = strict_source(
            config,
            &["roles", "role_registry", "registry"],
            "roles",
            "registry",
        )?
        .ok_or(TeamFlowSnapshotError::MissingField("roles"))?;
        let roles = role_index(Some(roles_value))?;
        let mut seen = BTreeSet::new();
        let mut nodes = Vec::with_capacity(steps.len());

        for (index, raw_step) in steps.iter().enumerate() {
            let step = match raw_step {
                Value::String(role_id) if !role_id.trim().is_empty() => {
                    serde_json::json!({ "role_id": role_id.trim() })
                }
                Value::Object(_) => raw_step.clone(),
                _ => return Err(TeamFlowSnapshotError::InvalidNode(index)),
            };
            let node_id = strict_string_aliases(
                &step,
                &["node_id", "role_id", "step_id"],
                "node_id",
                &index.to_string(),
            )?
            .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                node: index.to_string(),
                field: "node_id",
            })?;
            if !seen.insert(node_id.clone()) {
                return Err(TeamFlowSnapshotError::DuplicateNode(node_id));
            }
            let role = roles.get(&node_id).cloned().ok_or_else(|| {
                TeamFlowSnapshotError::MissingNodeField {
                    node: node_id.clone(),
                    field: "role",
                }
            })?;
            let runtime_role =
                merged_string(&step, &role, "runtime_role", &node_id)?.ok_or_else(|| {
                    TeamFlowSnapshotError::MissingNodeField {
                        node: node_id.clone(),
                        field: "runtime_role",
                    }
                })?;
            let task_class = resolve_task_class(&step, &role, &node_id)?;
            let inclusion_rule = merged_string(&step, &role, "inclusion_rule", &node_id)?
                .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                    node: node_id.clone(),
                    field: "inclusion_rule",
                })?;
            let included = strict_bool_aliases(
                &step,
                &role,
                &["included", "lane_template_included"],
                &node_id,
            )?
            .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                node: node_id.clone(),
                field: "included",
            })?;
            let required =
                strict_bool_aliases(&step, &role, &["required"], &node_id)?.ok_or_else(|| {
                    TeamFlowSnapshotError::MissingNodeField {
                        node: node_id.clone(),
                        field: "required",
                    }
                })?;

            let command = strict_source(
                &step,
                &["command_template", "command_mapping"],
                "command_template/command_mapping",
                &node_id,
            )?;
            let command_mapping = match command {
                Some(value) if value.is_null() => None,
                Some(value) => Some(strict_object_value(value, "command_mapping", &node_id)?),
                None => None,
            };
            let command_ref = resolve_command_ref(&step, command_mapping, &node_id)?;
            validate_command_reference(
                config,
                &step,
                command_mapping,
                command_ref.as_deref(),
                &node_id,
            )?;
            let command_mapping_hash = command_mapping.map(hash_json);
            let rework_targets = parse_targets(
                strict_source(
                    &step,
                    &["rework", "rework_transitions"],
                    "rework/rework_transitions",
                    &node_id,
                )?,
                &node_id,
            )?;
            let evidence_requirements = parse_evidence_requirements(
                &step,
                required,
                is_projection_config(config, &step),
                &node_id,
            )?;
            let next_node = strict_nullable_string_aliases(
                &step,
                &["next_node", "next"],
                "next_node",
                &node_id,
            )?
            .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                node: node_id.clone(),
                field: "next_node",
            })?;
            let terminal = strict_bool_aliases(
                &step,
                &role,
                &["terminal", "terminal_closure", "closes_workflow"],
                &node_id,
            )?
            .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                node: node_id.clone(),
                field: "terminal",
            })?;
            let requires_user_approval =
                strict_bool_aliases(&step, &role, &["requires_user_approval"], &node_id)?
                    .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                        node: node_id.clone(),
                        field: "requires_user_approval",
                    })?;
            validate_inclusion_rule(&inclusion_rule, &node_id)?;
            validate_terminal_edge(&node_id, terminal, next_node.as_deref())?;
            validate_approval_contract(
                config,
                &step,
                &node_id,
                requires_user_approval,
                next_node.as_deref(),
                &rework_targets,
            )?;
            validate_projection_metadata(config, &step, required, &node_id)?;

            nodes.push(TeamFlowNode {
                node_id,
                runtime_role,
                task_class,
                inclusion_rule,
                included,
                required,
                next_node,
                rework_targets,
                evidence_requirements,
                command_ref,
                command_mapping_hash,
                requires_user_approval,
                terminal,
            });
        }

        let ordered_nodes = nodes
            .iter()
            .map(|node| node.node_id.clone())
            .collect::<Vec<_>>();
        validate_execution_sequences(config, flow, &nodes)?;
        let mut snapshot = Self {
            config_id,
            profile,
            flow_ref: flow_id,
            config_hash: hash_json(config),
            registry_hash,
            snapshot_ref: String::new(),
            entry_node_id,
            ordered_nodes,
            nodes,
        };
        validate_edges(&snapshot.nodes)?;
        let Some(entry_node) = snapshot.node(&snapshot.entry_node_id) else {
            return Err(TeamFlowSnapshotError::InvalidEdge {
                node: "entry_node_id".to_string(),
                target: snapshot.entry_node_id.clone(),
            });
        };
        if !entry_node.included {
            return Err(TeamFlowSnapshotError::InvalidEdge {
                node: "entry_node_id".to_string(),
                target: snapshot.entry_node_id.clone(),
            });
        }
        snapshot.snapshot_ref = hash_snapshot(&snapshot);
        Ok(snapshot)
    }

    #[must_use]
    pub fn ordered_configured_nodes(&self) -> &[String] {
        &self.ordered_nodes
    }

    #[must_use]
    pub fn node(&self, node_id: &str) -> Option<&TeamFlowNode> {
        self.nodes
            .iter()
            .find(|node| node.node_id == node_id.trim())
    }

    #[must_use]
    pub fn has_valid_identity(&self) -> bool {
        !self.config_id.trim().is_empty()
            && !self.profile.trim().is_empty()
            && !self.flow_ref.trim().is_empty()
            && !self.config_hash.trim().is_empty()
            && !self.registry_hash.trim().is_empty()
            && !self.snapshot_ref.trim().is_empty()
            && hash_snapshot(self) == self.snapshot_ref
    }
}

/// Pure transition admission over one immutable snapshot and one receipt.
#[must_use]
pub fn admit_transition(
    snapshot: &TeamFlowSnapshot,
    current: &str,
    receipt: Option<&TeamFlowReceipt>,
    requested: &str,
) -> TransitionVerdict {
    let current = current.trim().to_string();
    let requested = requested.trim().to_string();
    let base = |blocker: &str, expected: Option<String>| TransitionVerdict {
        allowed: false,
        blocker: Some(blocker.to_string()),
        blocker_codes: vec![blocker.to_string()],
        expected,
        current: current.clone(),
        requested: requested.clone(),
        next_node: None,
        rework: false,
        required_remaining: required_remaining(snapshot, &current),
        evidence_requirements: Vec::new(),
        config_hash: snapshot.config_hash.clone(),
        snapshot_ref: snapshot.snapshot_ref.clone(),
    };

    if snapshot.config_id.trim().is_empty()
        || snapshot.profile.trim().is_empty()
        || snapshot.flow_ref.trim().is_empty()
        || snapshot.config_hash.trim().is_empty()
        || snapshot.registry_hash.trim().is_empty()
        || snapshot.snapshot_ref.trim().is_empty()
    {
        return base(BLOCKER_MISSING_SNAPSHOT_CONFIG_MAPPING, None);
    }
    if hash_snapshot(snapshot) != snapshot.snapshot_ref {
        return base(BLOCKER_SNAPSHOT_INTEGRITY_INVALID, None);
    }
    if current.is_empty() {
        return base(BLOCKER_CURRENT_NODE_MISSING, None);
    }
    let Some(current_index) = snapshot
        .ordered_nodes
        .iter()
        .position(|node| node == &current)
    else {
        return base(BLOCKER_CURRENT_NODE_UNKNOWN, None);
    };
    let Some(node) = snapshot.nodes.get(current_index) else {
        return base(BLOCKER_CURRENT_NODE_UNKNOWN, None);
    };
    if !node.included {
        return base(BLOCKER_CURRENT_NODE_NOT_INCLUDED, node.next_node.clone());
    }
    let Some(receipt) = receipt else {
        return base(BLOCKER_RECEIPT_REQUIRED, node.next_node.clone());
    };
    if receipt.config_hash != snapshot.config_hash {
        return base(BLOCKER_CONFIG_HASH_MISMATCH, node.next_node.clone());
    }
    if receipt.snapshot_ref != snapshot.snapshot_ref {
        return base(BLOCKER_SNAPSHOT_HASH_MISMATCH, node.next_node.clone());
    }
    if receipt.node_id.trim() != current {
        return base(BLOCKER_RECEIPT_NODE_MISMATCH, node.next_node.clone());
    }
    let Some(outcome) = normalize_receipt_outcome(&receipt.status) else {
        return base(BLOCKER_RECEIPT_NOT_COMPLETED, node.next_node.clone());
    };
    let missing_evidence = missing_evidence(&node.evidence_requirements, &receipt.evidence);
    if !missing_evidence.is_empty() {
        let mut verdict = base(BLOCKER_REQUIRED_EVIDENCE_MISSING, node.next_node.clone());
        verdict.evidence_requirements = missing_evidence;
        return verdict;
    }

    if outcome == ReceiptOutcome::Rework {
        if node
            .rework_targets
            .iter()
            .any(|target| target == &requested)
        {
            return allowed_verdict(snapshot, &current, &requested, node, true);
        }
        return base(BLOCKER_REWORK_TARGET_NOT_CONFIGURED, node.next_node.clone());
    }

    if node.requires_user_approval && outcome != ReceiptOutcome::Approved {
        return base(BLOCKER_USER_APPROVAL_REQUIRED, node.next_node.clone());
    }

    if node.terminal {
        if requested.is_empty() {
            let mut verdict = allowed_verdict(snapshot, &current, &requested, node, false);
            verdict.next_node = nonempty(&requested);
            verdict.expected = nonempty(&requested);
            return verdict;
        }
        return base(BLOCKER_TERMINAL_TRANSITION_NOT_CONFIGURED, None);
    }

    let expected = next_included_required(snapshot, current_index);
    if requested.is_empty() {
        return base(BLOCKER_INVALID_REQUESTED_NODE, expected);
    }
    if expected.as_deref() != Some(requested.as_str()) {
        return base(BLOCKER_INVALID_REQUESTED_NODE, expected);
    }
    allowed_verdict(snapshot, &current, &requested, node, false)
}

fn allowed_verdict(
    snapshot: &TeamFlowSnapshot,
    current: &str,
    requested: &str,
    node: &TeamFlowNode,
    rework: bool,
) -> TransitionVerdict {
    TransitionVerdict {
        allowed: true,
        blocker: None,
        blocker_codes: Vec::new(),
        expected: nonempty(requested),
        current: current.to_string(),
        requested: requested.to_string(),
        next_node: nonempty(requested),
        rework,
        required_remaining: required_remaining(snapshot, &node.node_id),
        evidence_requirements: node.evidence_requirements.clone(),
        config_hash: snapshot.config_hash.clone(),
        snapshot_ref: snapshot.snapshot_ref.clone(),
    }
}

#[must_use]
pub fn required_evidence_satisfied(requirements: &[String], evidence: &[String]) -> Vec<String> {
    missing_evidence(requirements, evidence)
}

fn missing_evidence(requirements: &[String], evidence: &[String]) -> Vec<String> {
    let mut actual = BTreeSet::new();
    for value in evidence {
        if let Some(value) = nonempty(value) {
            actual.insert(value);
        }
    }
    let mut missing = Vec::new();
    for required in requirements {
        if let Some(required) = nonempty(required) {
            if !actual.contains(&required) {
                missing.push(required);
            }
        }
    }
    missing
}

fn required_remaining(snapshot: &TeamFlowSnapshot, current: &str) -> Vec<String> {
    let Some(node) = snapshot.node(current) else {
        return Vec::new();
    };
    let mut remaining = Vec::new();
    let mut cursor = node.next_node.clone();
    let mut seen = BTreeSet::new();
    while let Some(node_id) = cursor {
        if !seen.insert(node_id.clone()) {
            break;
        }
        let Some(next) = snapshot.node(&node_id) else {
            break;
        };
        if next.included && next.required {
            remaining.push(next.node_id.clone());
        }
        cursor = next.next_node.clone();
    }
    remaining
}

fn next_included_required(snapshot: &TeamFlowSnapshot, current_index: usize) -> Option<String> {
    let mut index = current_index;
    let mut seen = BTreeSet::new();
    loop {
        let node = snapshot.nodes.get(index)?;
        let next = node.next_node.as_deref()?;
        if !seen.insert(next.to_string()) {
            return None;
        }
        let next_index = snapshot
            .ordered_nodes
            .iter()
            .position(|candidate| candidate == next)?;
        let next_node = snapshot.nodes.get(next_index)?;
        if next_node.included && next_node.required {
            return Some(next_node.node_id.clone());
        }
        index = next_index;
    }
}

fn select_flow<'a>(config: &'a Value, flow_ref: &str) -> Result<&'a Value, TeamFlowSnapshotError> {
    let mut sources = Vec::new();
    for key in ["flow", "flows", "flow_catalog", "project_flow_catalog"] {
        if let Some(value) = config.get(key) {
            sources.push((key, value));
        }
    }
    if config.get("ordered_steps").is_some() || config.get("steps").is_some() {
        sources.push(("root_steps", config));
    }
    if sources.len() > 1 {
        return Err(TeamFlowSnapshotError::ConflictingAliases {
            field: "flow_source",
            values: sources.iter().map(|(key, _)| (*key).to_string()).collect(),
        });
    }
    let Some((source, value)) = sources.into_iter().next() else {
        return Err(TeamFlowSnapshotError::MissingField("flow"));
    };
    match source {
        "flow" => strict_object_value(value, "flow", "flow"),
        "flows" | "flow_catalog" | "project_flow_catalog" => {
            let catalog = strict_map_value(value, source, "flow")?;
            let Some(flow) = catalog.get(flow_ref) else {
                return Err(TeamFlowSnapshotError::MissingField("flow_ref"));
            };
            strict_object_value(flow, "flow", flow_ref)
        }
        "root_steps" => Ok(value),
        _ => unreachable!("flow source set is closed"),
    }
}

fn configured_steps(flow: &Value) -> Result<&[Value], TeamFlowSnapshotError> {
    match (flow.get("ordered_steps"), flow.get("steps")) {
        (Some(_), Some(_)) => Err(TeamFlowSnapshotError::ConflictingAliases {
            field: "ordered_steps/steps",
            values: vec!["ordered_steps".to_string(), "steps".to_string()],
        }),
        (Some(value), None) => Ok(strict_array_value(value, "ordered_steps", "flow")?
            .as_array()
            .expect("strict_array_value returned an array")),
        (None, Some(value)) => Ok(strict_array_value(value, "steps", "flow")?
            .as_array()
            .expect("strict_array_value returned an array")),
        (None, None) => Err(TeamFlowSnapshotError::EmptyFlow),
    }
}

fn validate_edges(nodes: &[TeamFlowNode]) -> Result<(), TeamFlowSnapshotError> {
    let ids = nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();
    for node in nodes {
        if node.terminal && node.next_node.is_some() {
            return Err(TeamFlowSnapshotError::InvalidTerminalEdge(
                node.node_id.clone(),
            ));
        }
        if !node.terminal && node.next_node.is_none() {
            return Err(TeamFlowSnapshotError::MissingNodeField {
                node: node.node_id.clone(),
                field: "next_node",
            });
        }
        if let Some(target) = node.next_node.as_deref() {
            if !ids.contains(target) {
                return Err(TeamFlowSnapshotError::InvalidEdge {
                    node: node.node_id.clone(),
                    target: target.to_string(),
                });
            }
        }
        for target in &node.rework_targets {
            if !ids.contains(target.as_str()) {
                return Err(TeamFlowSnapshotError::InvalidEdge {
                    node: node.node_id.clone(),
                    target: target.clone(),
                });
            }
        }
        let mut seen = BTreeSet::new();
        let mut cursor = Some(node.node_id.as_str());
        while let Some(current) = cursor {
            if !seen.insert(current) {
                return Err(TeamFlowSnapshotError::TransitionCycle(node.node_id.clone()));
            }
            cursor = nodes
                .iter()
                .find(|candidate| candidate.node_id == current)
                .and_then(|candidate| candidate.next_node.as_deref());
        }
    }
    Ok(())
}

fn role_index(value: Option<&Value>) -> Result<BTreeMap<String, Value>, TeamFlowSnapshotError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    match value {
        Value::Array(values) => {
            let mut roles = BTreeMap::new();
            for role in values {
                let role = strict_object_value(role, "role", "registry")?;
                let Some(id) = strict_string_aliases(role, &["role_id"], "role_id", "registry")?
                else {
                    return Err(TeamFlowSnapshotError::MissingNodeField {
                        node: "registry".to_string(),
                        field: "role_id",
                    });
                };
                if roles.insert(id.clone(), role.clone()).is_some() {
                    return Err(TeamFlowSnapshotError::DuplicateNode(id));
                }
            }
            Ok(roles)
        }
        Value::Object(values) => {
            let mut roles = BTreeMap::new();
            for (id, role) in values {
                if nonempty(id).is_none() {
                    return Err(TeamFlowSnapshotError::MissingNodeField {
                        node: "registry".to_string(),
                        field: "role_id",
                    });
                }
                let role = strict_object_value(role, "role", id)?;
                roles.insert(id.clone(), role.clone());
            }
            Ok(roles)
        }
        _ => Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: "registry".to_string(),
            field: "roles",
        }),
    }
}

fn parse_targets(value: Option<&Value>, node: &str) -> Result<Vec<String>, TeamFlowSnapshotError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = match value {
        Value::Array(values) => values
            .iter()
            .map(|value| value.as_str().and_then(nonempty))
            .collect::<Option<Vec<_>>>()
            .ok_or(TeamFlowSnapshotError::MalformedEvidence {
                node: node.to_string(),
                field: "rework_transitions",
            })?,
        Value::Object(values) => {
            if let Some(targets) = values.get("targets") {
                return parse_targets(Some(targets), node);
            }
            values
                .values()
                .map(|value| value.as_str().and_then(nonempty))
                .collect::<Option<Vec<_>>>()
                .ok_or(TeamFlowSnapshotError::MalformedEvidence {
                    node: node.to_string(),
                    field: "rework_transitions",
                })?
        }
        Value::String(value) => {
            vec![
                nonempty(value).ok_or(TeamFlowSnapshotError::MalformedEvidence {
                    node: node.to_string(),
                    field: "rework_transitions",
                })?,
            ]
        }
        _ => {
            return Err(TeamFlowSnapshotError::MalformedEvidence {
                node: node.to_string(),
                field: "rework_transitions",
            });
        }
    };
    let mut seen = BTreeSet::new();
    for value in &values {
        if !seen.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "rework/rework_transitions",
                values: vec![value.clone()],
            });
        }
    }
    Ok(values)
}

fn parse_evidence_requirements(
    step: &Value,
    required: bool,
    projection: bool,
    node: &str,
) -> Result<Vec<String>, TeamFlowSnapshotError> {
    let flat = strict_source(
        step,
        &[
            "evidence_requirements",
            "required_evidence",
            "required_outputs",
            "proof_outputs",
        ],
        "evidence_requirements",
        node,
    )?;
    let proof_gates = strict_source(step, &["proof_gates"], "proof_gates", node)?;
    let nested = match proof_gates {
        Some(value) => {
            let proof_gates = strict_object_value(value, "proof_gates", node)?;
            strict_source(
                proof_gates,
                &["required_outputs", "required_evidence"],
                "proof_gates.required_outputs/required_evidence",
                node,
            )?
        }
        None => None,
    };
    let selected = match (flat, nested) {
        (Some(flat), Some(nested)) if projection => {
            let flat_values = parse_evidence_value(flat, node)?;
            let nested_values = parse_evidence_value(nested, node)?;
            if flat_values != nested_values {
                return Err(TeamFlowSnapshotError::ConflictingAliases {
                    field: "evidence_requirements",
                    values: vec!["flat".to_string(), "proof_gates".to_string()],
                });
            }
            ensure_unique_evidence(&flat_values, node)?;
            if required && flat_values.is_empty() {
                return Err(TeamFlowSnapshotError::MissingNodeField {
                    node: node.to_string(),
                    field: "evidence_requirements",
                });
            }
            return Ok(flat_values);
        }
        (Some(_), Some(_)) => {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "evidence_requirements",
                values: vec!["flat".to_string(), "proof_gates".to_string()],
            });
        }
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => {
            return Err(TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field: "proof_gates/evidence_requirements",
            });
        }
    };
    let evidence = parse_evidence_value(selected, node)?;
    ensure_unique_evidence(&evidence, node)?;
    if required && evidence.is_empty() {
        return Err(TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field: "evidence_requirements",
        });
    }
    Ok(evidence)
}

fn ensure_unique_evidence(evidence: &[String], node: &str) -> Result<(), TeamFlowSnapshotError> {
    let mut seen = BTreeSet::new();
    for value in evidence {
        if !seen.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "evidence_requirements",
                values: vec![value.clone()],
            });
        }
    }
    let _ = node;
    Ok(())
}

fn parse_evidence_value(value: &Value, node: &str) -> Result<Vec<String>, TeamFlowSnapshotError> {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|item| parse_evidence_item(item, node))
            .collect(),
        Value::String(value) => Ok(vec![nonempty(value).ok_or(
            TeamFlowSnapshotError::MalformedEvidence {
                node: node.to_string(),
                field: "evidence_requirements",
            },
        )?]),
        Value::Object(_) => Ok(vec![parse_evidence_item(value, node)?]),
        _ => Err(TeamFlowSnapshotError::MalformedEvidence {
            node: node.to_string(),
            field: "evidence_requirements",
        }),
    }
}

fn parse_evidence_item(value: &Value, node: &str) -> Result<String, TeamFlowSnapshotError> {
    if let Some(value) = value.as_str() {
        return nonempty(value).ok_or(TeamFlowSnapshotError::MalformedEvidence {
            node: node.to_string(),
            field: "evidence_requirements",
        });
    }
    let Some(_object) = value.as_object() else {
        return Err(TeamFlowSnapshotError::MalformedEvidence {
            node: node.to_string(),
            field: "evidence_requirements",
        });
    };
    let id = strict_string_aliases(value, &["evidence_id", "id", "name"], "evidence_id", node)?;
    let kind = strict_string_aliases(value, &["kind", "type"], "kind", node)?;
    let value = strict_string_aliases(value, &["value", "proof", "output"], "value", node)?;
    match (id, kind, value) {
        (Some(id), Some(_), Some(_)) => Ok(id),
        _ => Err(TeamFlowSnapshotError::MalformedEvidence {
            node: node.to_string(),
            field: "evidence_requirements",
        }),
    }
}

fn strict_source<'a>(
    value: &'a Value,
    keys: &[&'static str],
    field: &'static str,
    node: &str,
) -> Result<Option<&'a Value>, TeamFlowSnapshotError> {
    let _ = node;
    let mut sources = Vec::new();
    for key in keys {
        if let Some(raw) = value.get(key) {
            sources.push((*key, raw));
        }
    }
    if sources.len() > 1 {
        return Err(TeamFlowSnapshotError::ConflictingAliases {
            field,
            values: sources.iter().map(|(key, _)| (*key).to_string()).collect(),
        });
    }
    Ok(sources.into_iter().next().map(|(_, value)| value))
}

fn strict_string_aliases(
    value: &Value,
    keys: &[&'static str],
    field: &'static str,
    node: &str,
) -> Result<Option<String>, TeamFlowSnapshotError> {
    let Some(value) = strict_source(value, keys, field, node)? else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field,
        });
    };
    nonempty(value)
        .map(Some)
        .ok_or(TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field,
        })
}

fn strict_nullable_string_aliases(
    value: &Value,
    keys: &[&'static str],
    field: &'static str,
    node: &str,
) -> Result<Option<Option<String>>, TeamFlowSnapshotError> {
    let Some(value) = strict_source(value, keys, field, node)? else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let Some(value) = value.as_str() else {
        return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field,
        });
    };
    nonempty(value)
        .map(Some)
        .map(Some)
        .ok_or(TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field,
        })
}

fn strict_object_value<'a>(
    value: &'a Value,
    field: &'static str,
    node: &str,
) -> Result<&'a Value, TeamFlowSnapshotError> {
    if value.is_object() {
        Ok(value)
    } else {
        Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field,
        })
    }
}

fn strict_map_value<'a>(
    value: &'a Value,
    field: &'static str,
    node: &str,
) -> Result<&'a Value, TeamFlowSnapshotError> {
    strict_object_value(value, field, node)
}

fn strict_array_value<'a>(
    value: &'a Value,
    field: &'static str,
    node: &str,
) -> Result<&'a Value, TeamFlowSnapshotError> {
    if value.is_array() {
        Ok(value)
    } else {
        Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field,
        })
    }
}

fn strict_string_values(
    value: &Value,
    field: &'static str,
    node: &str,
) -> Result<Vec<String>, TeamFlowSnapshotError> {
    let values = strict_array_value(value, field, node)?
        .as_array()
        .expect("strict_array_value returned an array");
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let Some(value) = value.as_str().and_then(nonempty) else {
            return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                node: node.to_string(),
                field,
            });
        };
        if !seen.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field,
                values: vec![value],
            });
        }
        result.push(value);
    }
    Ok(result)
}

fn merged_string(
    step: &Value,
    role: &Value,
    field: &'static str,
    node: &str,
) -> Result<Option<String>, TeamFlowSnapshotError> {
    let step_value = strict_string_aliases(step, &[field], field, node)?;
    let role_value = strict_string_aliases(role, &[field], field, node)?;
    match (step_value, role_value) {
        (Some(step_value), Some(role_value)) => Err(TeamFlowSnapshotError::ConflictingAliases {
            field,
            values: vec![step_value, role_value],
        }),
        (Some(value), None) | (None, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

fn resolve_task_class(
    step: &Value,
    role: &Value,
    node: &str,
) -> Result<String, TeamFlowSnapshotError> {
    let direct = merged_string(step, role, "task_class", node)?;
    let step_classes = strict_source(step, &["task_classes"], "task_classes", node)?;
    let role_classes = strict_source(role, &["task_classes"], "task_classes", node)?;
    if role.get("task_class").is_some() && role_classes.is_some() {
        return Err(TeamFlowSnapshotError::ConflictingAliases {
            field: "task_class/task_classes",
            values: vec![
                "role.task_class".to_string(),
                "role.task_classes".to_string(),
            ],
        });
    }
    if let Some(value) = direct {
        if step_classes.is_some() {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "task_class/task_classes",
                values: vec!["task_class".to_string(), "task_classes".to_string()],
            });
        }
        if let Some(role_classes) = role_classes {
            let classes = strict_string_values(role_classes, "task_classes", node)?;
            let compatible = classes.iter().any(|class| class == &value);
            if !compatible {
                return Err(TeamFlowSnapshotError::TaskClassNotInRoleCapabilities {
                    node: node.to_string(),
                    task_class: value,
                });
            }
        }
        return Ok(value);
    }
    let classes = match (step_classes, role_classes) {
        (Some(_), Some(_)) => {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "task_classes",
                values: vec![
                    "step.task_classes".to_string(),
                    "registry.task_classes".to_string(),
                ],
            });
        }
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => {
            return Err(TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field: "task_class",
            });
        }
    };
    let classes = strict_string_values(classes, "task_classes", node)?;
    if classes.is_empty() {
        return Err(TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field: "task_class",
        });
    }
    match classes.as_slice() {
        [value] => Ok(value.clone()),
        [] => Err(TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field: "task_class",
        }),
        _ => Err(TeamFlowSnapshotError::ConflictingAliases {
            field: "task_classes",
            values: classes,
        }),
    }
}

fn resolve_command_ref(
    step: &Value,
    command: Option<&Value>,
    node: &str,
) -> Result<Option<String>, TeamFlowSnapshotError> {
    let direct = strict_nullable_string_aliases(
        step,
        &["command_ref", "command_mapping_ref"],
        "command_ref",
        node,
    )?
    .flatten();
    let nested = match command {
        Some(value) => {
            let command_id = strict_string_aliases(value, &["command_id"], "command_ref", node)?;
            let reference = strict_string_aliases(value, &["ref"], "command_ref", node)?;
            let _surface = strict_string_aliases(value, &["surface"], "command_ref", node)?;
            if command_id.is_some() && reference.is_some() {
                return Err(TeamFlowSnapshotError::ConflictingAliases {
                    field: "command_ref",
                    values: vec!["command_id".to_string(), "ref".to_string()],
                });
            }
            if command_id.is_none() && reference.is_some() && value.get("surface").is_some() {
                return Err(TeamFlowSnapshotError::ConflictingAliases {
                    field: "command_ref",
                    values: vec!["surface".to_string(), "ref".to_string()],
                });
            }
            command_id.or(reference)
        }
        None => None,
    };
    match (direct, nested) {
        (Some(direct), Some(nested)) if direct == nested => Ok(Some(direct)),
        (Some(_), Some(_)) => Err(TeamFlowSnapshotError::ConflictingAliases {
            field: "command_ref",
            values: vec!["direct".to_string(), "command_mapping".to_string()],
        }),
        (Some(value), None) | (None, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

fn validate_command_reference(
    config: &Value,
    step: &Value,
    command_mapping: Option<&Value>,
    command_ref: Option<&str>,
    node: &str,
) -> Result<(), TeamFlowSnapshotError> {
    let Some(command_ref) = command_ref else {
        if step
            .get("command_ref")
            .is_some_and(|value| !value.is_null())
            || step
                .get("command_mapping_ref")
                .is_some_and(|value| !value.is_null())
        {
            return Err(TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field: "command_ref",
            });
        }
        return Ok(());
    };
    if let Some(mapping) = command_mapping {
        let command_id = strict_string_aliases(mapping, &["command_id"], "command_id", node)?
            .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field: "command_mapping.command_id",
            })?;
        if command_id != command_ref {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "command_ref",
                values: vec![command_ref.to_string(), command_id],
            });
        }
        strict_string_aliases(mapping, &["surface"], "surface", node)?.ok_or_else(|| {
            TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field: "command_mapping.surface",
            }
        })?;
    }
    let catalog = match strict_source(
        config,
        &["command_catalog", "commands"],
        "command_catalog",
        node,
    )? {
        Some(catalog) => catalog,
        None => config
            .get("registries")
            .and_then(|registries| registries.get("commands"))
            .ok_or_else(|| TeamFlowSnapshotError::UnresolvedCommandReference {
                node: node.to_string(),
                command_ref: command_ref.to_string(),
            })?,
    };
    let entry = strict_command_catalog_entry(catalog, command_ref, node)?;
    let entry_id =
        strict_string_aliases(entry, &["command_id"], "command_id", node)?.ok_or_else(|| {
            TeamFlowSnapshotError::UnresolvedCommandReference {
                node: node.to_string(),
                command_ref: command_ref.to_string(),
            }
        })?;
    if entry_id != command_ref {
        return Err(TeamFlowSnapshotError::UnresolvedCommandReference {
            node: node.to_string(),
            command_ref: command_ref.to_string(),
        });
    }
    strict_string_aliases(entry, &["surface"], "surface", node)?.ok_or_else(|| {
        TeamFlowSnapshotError::UnresolvedCommandReference {
            node: node.to_string(),
            command_ref: command_ref.to_string(),
        }
    })?;
    Ok(())
}

fn strict_command_catalog_entry<'a>(
    catalog: &'a Value,
    command_ref: &str,
    node: &str,
) -> Result<&'a Value, TeamFlowSnapshotError> {
    let mut found = None;
    let mut inspect = |entry: &'a Value, key: Option<&str>| -> Result<(), TeamFlowSnapshotError> {
        let entry = strict_object_value(entry, "command_catalog", node)?;
        let entry_id = strict_string_aliases(entry, &["command_id"], "command_id", node)?
            .ok_or_else(|| TeamFlowSnapshotError::UnresolvedCommandReference {
                node: node.to_string(),
                command_ref: command_ref.to_string(),
            })?;
        if let Some(key) = key {
            if key != entry_id {
                return Err(TeamFlowSnapshotError::ConflictingAliases {
                    field: "command_catalog.command_id",
                    values: vec![key.to_string(), entry_id],
                });
            }
        }
        strict_string_aliases(entry, &["surface"], "surface", node)?.ok_or_else(|| {
            TeamFlowSnapshotError::UnresolvedCommandReference {
                node: node.to_string(),
                command_ref: command_ref.to_string(),
            }
        })?;
        if entry_id == command_ref {
            if found.is_some() {
                return Err(TeamFlowSnapshotError::ConflictingAliases {
                    field: "command_catalog.command_id",
                    values: vec![command_ref.to_string()],
                });
            }
            found = Some(entry);
        }
        Ok(())
    };
    match catalog {
        Value::Object(entries) => {
            for (key, entry) in entries {
                inspect(entry, Some(key))?;
            }
        }
        Value::Array(entries) => {
            for entry in entries {
                inspect(entry, None)?;
            }
        }
        _ => {
            return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                node: node.to_string(),
                field: "command_catalog",
            });
        }
    }
    found.ok_or_else(|| TeamFlowSnapshotError::UnresolvedCommandReference {
        node: node.to_string(),
        command_ref: command_ref.to_string(),
    })
}

fn validate_inclusion_rule(inclusion_rule: &str, node: &str) -> Result<(), TeamFlowSnapshotError> {
    if inclusion_rule
        .parse::<crate::team_flow_inclusion::InclusionRule>()
        .is_ok()
    {
        Ok(())
    } else {
        Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "inclusion_rule",
        })
    }
}

fn validate_terminal_edge(
    node: &str,
    terminal: bool,
    next_node: Option<&str>,
) -> Result<(), TeamFlowSnapshotError> {
    if terminal && next_node.is_some() {
        return Err(TeamFlowSnapshotError::InvalidTerminalEdge(node.to_string()));
    }
    if !terminal && next_node.is_none() {
        return Err(TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field: "next_node",
        });
    }
    Ok(())
}

fn approval_policy_catalog(
    config: &Value,
    field: &'static str,
    node: &str,
) -> Result<BTreeSet<String>, TeamFlowSnapshotError> {
    let values = config
        .get("authority_catalog")
        .and_then(Value::as_object)
        .or_else(|| {
            config
                .get("dev_team")
                .and_then(Value::as_object)
                .and_then(|dev_team| dev_team.get("authority_catalog"))
                .and_then(Value::as_object)
        })
        .and_then(|catalog| catalog.get(field))
        .and_then(Value::as_array)
        .ok_or_else(|| TeamFlowSnapshotError::InvalidApprovalContract {
            node: node.to_string(),
            field,
        })?;
    let mut result = BTreeSet::new();
    for value in values {
        let Some(value) = value.as_str().filter(|value| !value.trim().is_empty()) else {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field,
            });
        };
        if !result.insert(value.to_string()) {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field,
            });
        }
    }
    if result.is_empty() {
        return Err(TeamFlowSnapshotError::InvalidApprovalContract {
            node: node.to_string(),
            field,
        });
    }
    Ok(result)
}

fn validate_approval_contract(
    config: &Value,
    step: &Value,
    node: &str,
    requires_user_approval: bool,
    next_node: Option<&str>,
    rework_targets: &[String],
) -> Result<(), TeamFlowSnapshotError> {
    let Some(policy) = step.get("approval_policy") else {
        if requires_user_approval {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "approval_policy",
            });
        }
        return Ok(());
    };
    let policy = strict_object_value(policy, "approval_policy", node)?;
    let mode = strict_string_aliases(policy, &["mode"], "approval_policy.mode", node)?;
    if mode.is_none() && !policy.as_object().is_some_and(|object| object.is_empty()) {
        return Err(TeamFlowSnapshotError::InvalidApprovalContract {
            node: node.to_string(),
            field: "approval_policy.mode",
        });
    }
    let mode_catalog = if mode.is_some() || requires_user_approval {
        Some(approval_policy_catalog(
            config,
            "approval_policy_modes",
            node,
        )?)
    } else {
        None
    };
    if let Some(mode) = mode.as_deref() {
        if !mode_catalog
            .as_ref()
            .is_some_and(|catalog| catalog.contains(mode))
        {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "approval_policy.mode",
            });
        }
    }
    let prompt = strict_string_aliases(
        policy,
        &["prompt_template"],
        "approval_policy.prompt_template",
        node,
    )?;
    let decisions = policy.get("allowed_decisions");
    let decision_catalog = if mode.is_some() || requires_user_approval || decisions.is_some() {
        Some(approval_policy_catalog(
            config,
            "approval_policy_allowed_decisions",
            node,
        )?)
    } else {
        None
    };
    if mode.is_some() && decisions.is_none() {
        return Err(TeamFlowSnapshotError::InvalidApprovalContract {
            node: node.to_string(),
            field: "approval_policy.allowed_decisions",
        });
    }
    if let Some(decisions) = decisions {
        let decisions =
            decisions
                .as_array()
                .ok_or_else(|| TeamFlowSnapshotError::InvalidApprovalContract {
                    node: node.to_string(),
                    field: "approval_policy.allowed_decisions",
                })?;
        if mode.is_some() && decisions.is_empty() {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "approval_policy.allowed_decisions",
            });
        }
        let mut seen = BTreeSet::new();
        for value in decisions {
            let Some(value) = value.as_str() else {
                return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                    node: node.to_string(),
                    field: "approval_policy.allowed_decisions",
                });
            };
            if !seen.insert(value) {
                return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                    node: node.to_string(),
                    field: "approval_policy.allowed_decisions",
                });
            }
            if !decision_catalog
                .as_ref()
                .is_some_and(|catalog| catalog.contains(value))
            {
                return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                    node: node.to_string(),
                    field: "approval_policy.allowed_decisions",
                });
            }
            if value == "rework" && rework_targets.is_empty() {
                return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                    node: node.to_string(),
                    field: "approval_policy.allowed_decisions",
                });
            }
        }
    }
    if requires_user_approval {
        if mode.as_deref() != Some("user_review_required") || prompt.is_none() {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "approval_policy.mode/prompt_template",
            });
        }
        let decisions = decisions.and_then(Value::as_array).ok_or_else(|| {
            TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "approval_policy.allowed_decisions",
            }
        })?;
        if !decisions.iter().all(|value| {
            value.as_str().is_some_and(|value| {
                decision_catalog
                    .as_ref()
                    .is_some_and(|catalog| catalog.contains(value))
            })
        }) || !decisions
            .iter()
            .any(|value| value.as_str() == Some("approved"))
        {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "approval_policy.allowed_decisions",
            });
        }
        let transitions = step
            .get("resume_transitions")
            .and_then(Value::as_object)
            .ok_or_else(|| TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "resume_transitions.approved",
            })?;
        let approved = transitions
            .get("approved")
            .and_then(Value::as_str)
            .and_then(nonempty)
            .ok_or_else(|| TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "resume_transitions.approved",
            })?;
        if Some(approved.as_str()) != next_node {
            return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                node: node.to_string(),
                field: "resume_transitions.approved",
            });
        }
    } else {
        match mode.as_deref() {
            Some("user_review_required") => {
                return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                    node: node.to_string(),
                    field: "approval_policy.mode",
                });
            }
            Some("not_required") => {
                if requires_user_approval || prompt.is_some() {
                    return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                        node: node.to_string(),
                        field: "approval_policy.mode/prompt_template",
                    });
                }
                if step
                    .get("resume_transitions")
                    .and_then(Value::as_object)
                    .is_some_and(|transitions| transitions.contains_key("approved"))
                {
                    return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                        node: node.to_string(),
                        field: "resume_transitions.approved",
                    });
                }
            }
            Some("optional_user_review") => {
                if requires_user_approval {
                    return Err(TeamFlowSnapshotError::InvalidApprovalContract {
                        node: node.to_string(),
                        field: "approval_policy.mode",
                    });
                }
            }
            Some(_) | None => {}
        }
    }
    Ok(())
}

fn is_projection_config(config: &Value, step: &Value) -> bool {
    config
        .get("schema_version")
        .and_then(Value::as_str)
        .is_some_and(|version| version == "team-flow-authority.v1")
        || step.get("lane_id").is_some()
}

fn validate_projection_metadata(
    config: &Value,
    step: &Value,
    required: bool,
    node: &str,
) -> Result<(), TeamFlowSnapshotError> {
    if !is_projection_config(config, step) {
        return Ok(());
    }
    for field in [
        "lane_id",
        "dispatch_target",
        "dispatch_alias",
        "packet_template_kind",
        "closure_class",
        "stage",
        "completion_blocker",
        "evidence_requirements",
        "proof_gates",
        "command_ref",
        "command_mapping",
        "lifecycle_hook_templates",
        "resume_transitions",
        "policy_diagnostics",
        "activation",
        "runtime_assignment",
        "carrier_runtime_assignment",
        "profile_authority",
        "selected_model_profile",
        "approval_policy",
        "rework",
    ] {
        if !step.get(field).is_some() {
            return Err(TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field,
            });
        }
    }
    let proof_gates = step
        .get("proof_gates")
        .and_then(Value::as_object)
        .ok_or_else(|| TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "proof_gates",
        })?;
    let proof_outputs = proof_gates
        .get("required_outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field: "proof_gates.required_outputs",
        })?;
    let mut proof_ids = BTreeSet::new();
    for value in proof_outputs {
        let Some(value) = value.as_str().and_then(nonempty) else {
            return Err(TeamFlowSnapshotError::MalformedEvidence {
                node: node.to_string(),
                field: "proof_gates.required_outputs",
            });
        };
        if !proof_ids.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "proof_gates.required_outputs",
                values: vec![value.to_string()],
            });
        }
    }
    if required && proof_outputs.is_empty() {
        return Err(TeamFlowSnapshotError::MalformedEvidence {
            node: node.to_string(),
            field: "proof_gates.required_outputs",
        });
    }
    let rework = step
        .get("rework")
        .and_then(Value::as_object)
        .ok_or_else(|| TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "rework",
        })?;
    let targets = rework
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| TeamFlowSnapshotError::MissingNodeField {
            node: node.to_string(),
            field: "rework.targets",
        })?;
    let mut target_ids = BTreeSet::new();
    for value in targets {
        let Some(value) = value.as_str().and_then(nonempty) else {
            return Err(TeamFlowSnapshotError::MalformedEvidence {
                node: node.to_string(),
                field: "rework.targets",
            });
        };
        if !target_ids.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "rework.targets",
                values: vec![value.to_string()],
            });
        }
    }
    for field in [
        "proof_gates",
        "approval_policy",
        "resume_transitions",
        "policy_diagnostics",
        "activation",
        "runtime_assignment",
        "carrier_runtime_assignment",
        "profile_authority",
        "selected_model_profile",
        "rework",
    ] {
        if !step.get(field).is_some_and(Value::is_object) {
            return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                node: node.to_string(),
                field,
            });
        }
    }
    for field in [
        "activation",
        "runtime_assignment",
        "carrier_runtime_assignment",
    ] {
        if step
            .get(field)
            .and_then(Value::as_object)
            .is_some_and(|object| object.is_empty())
        {
            return Err(TeamFlowSnapshotError::MissingNodeField {
                node: node.to_string(),
                field,
            });
        }
    }
    let profile_fields = ["team_role_id", "runtime_role", "task_class", "source_path"];
    let selected_profile_fields = ["profile_id", "selection_source"];
    for (field, required_fields) in [
        ("profile_authority", profile_fields.as_slice()),
        ("selected_model_profile", selected_profile_fields.as_slice()),
    ] {
        let object = step.get(field).and_then(Value::as_object).ok_or_else(|| {
            TeamFlowSnapshotError::InvalidNodeFieldType {
                node: node.to_string(),
                field,
            }
        })?;
        for required_field in required_fields {
            if object
                .get(*required_field)
                .and_then(Value::as_str)
                .and_then(nonempty)
                .is_none()
            {
                return Err(TeamFlowSnapshotError::MissingNodeField {
                    node: node.to_string(),
                    field: required_field,
                });
            }
        }
    }
    let hooks = step
        .get("lifecycle_hook_templates")
        .and_then(Value::as_array)
        .ok_or_else(|| TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "lifecycle_hook_templates",
        })?;
    let mut hook_ids = BTreeSet::new();
    for value in hooks {
        let Some(value) = value.as_str().and_then(nonempty) else {
            return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                node: node.to_string(),
                field: "lifecycle_hook_templates",
            });
        };
        if !hook_ids.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "lifecycle_hook_templates",
                values: vec![value.to_string()],
            });
        }
    }
    if hooks.len() != hook_ids.len() {
        return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "lifecycle_hook_templates",
        });
    }
    let resume = step
        .get("resume_transitions")
        .and_then(Value::as_object)
        .ok_or_else(|| TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "resume_transitions",
        })?;
    if resume
        .values()
        .any(|value| value.as_str().and_then(nonempty).is_none())
    {
        return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "resume_transitions",
        });
    }
    let diagnostics = step
        .get("policy_diagnostics")
        .and_then(Value::as_object)
        .ok_or_else(|| TeamFlowSnapshotError::InvalidNodeFieldType {
            node: node.to_string(),
            field: "policy_diagnostics",
        })?;
    if diagnostics.get("source").and_then(Value::as_str)
        != Some("team_flow_authority.selected_config")
        || diagnostics.get("fallback_used") != Some(&Value::Bool(false))
        || diagnostics
            .get("fallback_fields")
            .and_then(Value::as_array)
            .map_or(true, |fields| !fields.is_empty())
    {
        return Err(TeamFlowSnapshotError::InvalidApprovalContract {
            node: node.to_string(),
            field: "policy_diagnostics",
        });
    }
    if step.get("requires_user_approval").and_then(Value::as_bool) == Some(true)
        && !resume.contains_key("approved")
    {
        return Err(TeamFlowSnapshotError::InvalidApprovalContract {
            node: node.to_string(),
            field: "resume_transitions.approved",
        });
    }
    Ok(())
}

fn validate_execution_sequences(
    config: &Value,
    flow: &Value,
    nodes: &[TeamFlowNode],
) -> Result<(), TeamFlowSnapshotError> {
    let ordered = nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    for source in [config, flow] {
        for field in ["execution_lane_sequence", "lane_sequence"] {
            let Some(value) = source.get(field) else {
                continue;
            };
            let values = strict_sequence_values(value, field)?;
            if values != ordered {
                return Err(TeamFlowSnapshotError::InvalidSequence { field });
            }
        }
        for (field, expected) in [
            (
                "included_nodes",
                nodes
                    .iter()
                    .filter(|node| node.included)
                    .map(|node| node.node_id.clone())
                    .collect::<Vec<_>>(),
            ),
            (
                "required_nodes",
                nodes
                    .iter()
                    .filter(|node| node.required)
                    .map(|node| node.node_id.clone())
                    .collect::<Vec<_>>(),
            ),
        ] {
            let Some(value) = source.get(field) else {
                continue;
            };
            let values = strict_sequence_values(value, field)?;
            if values != expected {
                return Err(TeamFlowSnapshotError::InvalidSequence { field });
            }
        }
    }
    Ok(())
}

fn strict_sequence_values(
    value: &Value,
    field: &'static str,
) -> Result<Vec<String>, TeamFlowSnapshotError> {
    let values = value
        .as_array()
        .ok_or(TeamFlowSnapshotError::InvalidSequence { field })?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let Some(value) = value.as_str().and_then(nonempty) else {
            return Err(TeamFlowSnapshotError::InvalidSequence { field });
        };
        if !seen.insert(value.clone()) {
            return Err(TeamFlowSnapshotError::InvalidSequence { field });
        }
        result.push(value);
    }
    Ok(result)
}

fn strict_bool_aliases(
    step: &Value,
    role: &Value,
    keys: &[&'static str],
    node: &str,
) -> Result<Option<bool>, TeamFlowSnapshotError> {
    let field = keys[0];
    let mut values = Vec::new();
    for (source, value) in [("step", step), ("registry", role)] {
        for key in keys {
            let Some(raw) = value.get(key) else {
                continue;
            };
            let Some(raw) = raw.as_bool() else {
                return Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                    node: node.to_string(),
                    field,
                });
            };
            values.push((format!("{source}.{key}"), raw));
        }
    }
    match values.as_slice() {
        [] => Ok(None),
        [(source, value)] => {
            let _ = source;
            Ok(Some(*value))
        }
        _ => Err(TeamFlowSnapshotError::ConflictingAliases {
            field,
            values: values
                .into_iter()
                .map(|(source, value)| format!("{source}={value}"))
                .collect(),
        }),
    }
}

fn required_metadata(value: &str, field: &'static str) -> Result<String, TeamFlowSnapshotError> {
    nonempty(value).ok_or(TeamFlowSnapshotError::MissingField(field))
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn hash_snapshot(snapshot: &TeamFlowSnapshot) -> String {
    let mut copy = snapshot.clone();
    copy.snapshot_ref.clear();
    hash_json(&serde_json::to_value(copy).expect("TeamFlowSnapshot is serializable"))
}

#[must_use]
pub fn hash_json(value: &Value) -> String {
    blake3::hash(canonical_json(value).as_bytes())
        .to_hex()
        .to_string()
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).expect("JSON string is serializable"),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            format!(
                "{{{}}}",
                entries
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("JSON object key is serializable"),
                        canonical_json(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::json!({
            "flow_id": "flow-a",
            "entry_node_id": "node-a",
            "roles": {
                "node-a": {"runtime_role": "runtime-a", "task_classes": ["class-a"]},
                "node-b": {"runtime_role": "runtime-b", "task_classes": ["class-b"]},
                "node-c": {"runtime_role": "runtime-c", "task_classes": ["class-c"]}
            },
            "ordered_steps": [
                {"role_id": "node-a", "inclusion_rule": "always", "included": true, "required": true, "required_outputs": ["proof-a"], "rework_transitions": {"rework": "node-a"}, "next_node": "node-b", "terminal": false, "requires_user_approval": false},
                {"role_id": "node-b", "inclusion_rule": "optional", "included": false, "required": false, "evidence_requirements": [], "next_node": "node-c", "terminal": false, "requires_user_approval": false},
                {"role_id": "node-c", "inclusion_rule": "always", "included": true, "required": true, "evidence_requirements": ["proof-a"], "next_node": null, "terminal": true, "requires_user_approval": false}
            ]
        })
    }

    fn snapshot() -> TeamFlowSnapshot {
        TeamFlowSnapshot::from_config(
            &fixture(),
            TeamFlowSnapshotInput {
                config_id: "cfg-a",
                profile: "profile-a",
                flow_ref: "flow-a",
                registry_hash: "registry-a",
            },
        )
        .expect("fixture snapshot should compile")
    }

    fn receipt(snapshot: &TeamFlowSnapshot, node_id: &str, status: &str) -> TeamFlowReceipt {
        TeamFlowReceipt {
            receipt_id: "receipt-a".to_string(),
            node_id: node_id.to_string(),
            status: status.to_string(),
            evidence: vec!["proof-a".to_string()],
            config_hash: snapshot.config_hash.clone(),
            snapshot_ref: snapshot.snapshot_ref.clone(),
        }
    }

    fn compile(config: &Value) -> Result<TeamFlowSnapshot, TeamFlowSnapshotError> {
        TeamFlowSnapshot::from_config(
            config,
            TeamFlowSnapshotInput {
                config_id: "cfg-a",
                profile: "profile-a",
                flow_ref: "flow-a",
                registry_hash: "registry-a",
            },
        )
    }

    fn projection_fixture() -> Value {
        let mut config = fixture();
        config["schema_version"] = serde_json::json!("team-flow-authority.v1");
        config["config_id"] = serde_json::json!("cfg-a");
        config["profile"] = serde_json::json!("profile-a");
        config["registry_hash"] = serde_json::json!("registry-a");
        config["selected_flow_set"] = serde_json::json!("flow-a");

        let steps = config["ordered_steps"].as_array().expect("steps");
        let node_ids = steps
            .iter()
            .map(|step| {
                step.get("role_id")
                    .and_then(Value::as_str)
                    .expect("role_id")
                    .to_string()
            })
            .collect::<Vec<_>>();
        let included_nodes = steps
            .iter()
            .zip(node_ids.iter())
            .filter_map(|(step, node)| {
                step.get("included")
                    .and_then(Value::as_bool)
                    .filter(|included| *included)
                    .map(|_| node.clone())
            })
            .collect::<Vec<_>>();
        let required_nodes = steps
            .iter()
            .zip(node_ids.iter())
            .filter_map(|(step, node)| {
                step.get("required")
                    .and_then(Value::as_bool)
                    .filter(|required| *required)
                    .map(|_| node.clone())
            })
            .collect::<Vec<_>>();
        config["execution_lane_sequence"] = serde_json::json!(node_ids.clone());
        config["lane_sequence"] = serde_json::json!(node_ids.clone());
        config["included_nodes"] = serde_json::json!(included_nodes);
        config["required_nodes"] = serde_json::json!(required_nodes);

        let roles = config["roles"].clone();
        let profile = config["profile"].clone();
        for (index, raw_step) in config["ordered_steps"]
            .as_array_mut()
            .expect("steps")
            .iter_mut()
            .enumerate()
        {
            let node = node_ids[index].clone();
            let runtime_role = roles[&node]["runtime_role"].clone();
            let task_class = roles[&node]["task_classes"][0].clone();
            let evidence = format!("proof-{node}");
            let step = raw_step.as_object_mut().expect("step");
            step.remove("required_outputs");
            step.remove("rework_transitions");
            step.insert("lane_id".to_string(), serde_json::json!(node));
            step.insert("dispatch_target".to_string(), serde_json::json!(node));
            step.insert("dispatch_alias".to_string(), serde_json::json!(node));
            step.insert(
                "packet_template_kind".to_string(),
                serde_json::json!("packet"),
            );
            step.insert("closure_class".to_string(), serde_json::json!("proof"));
            step.insert("stage".to_string(), serde_json::json!("execution"));
            step.insert(
                "completion_blocker".to_string(),
                serde_json::json!("pending"),
            );
            step.insert(
                "evidence_requirements".to_string(),
                serde_json::json!([evidence.clone()]),
            );
            step.insert(
                "proof_gates".to_string(),
                serde_json::json!({"required_outputs": [evidence]}),
            );
            step.insert("command_ref".to_string(), Value::Null);
            step.insert("command_mapping".to_string(), Value::Null);
            step.insert(
                "lifecycle_hook_templates".to_string(),
                serde_json::json!(["timing"]),
            );
            step.insert("resume_transitions".to_string(), serde_json::json!({}));
            step.insert(
                "policy_diagnostics".to_string(),
                serde_json::json!({
                    "source": "team_flow_authority.selected_config",
                    "fallback_used": false,
                    "fallback_fields": []
                }),
            );
            step.insert(
                "activation".to_string(),
                serde_json::json!({"source": "configured"}),
            );
            step.insert(
                "runtime_assignment".to_string(),
                serde_json::json!({"source": "configured"}),
            );
            step.insert(
                "carrier_runtime_assignment".to_string(),
                serde_json::json!({"source": "configured"}),
            );
            step.insert(
                "profile_authority".to_string(),
                serde_json::json!({
                    "team_role_id": node,
                    "runtime_role": runtime_role,
                    "task_class": task_class,
                    "source_path": "config"
                }),
            );
            step.insert(
                "selected_model_profile".to_string(),
                serde_json::json!({
                    "profile_id": profile.clone(),
                    "selection_source": "configured"
                }),
            );
            step.insert("approval_policy".to_string(), serde_json::json!({}));
            step.insert(
                "rework".to_string(),
                serde_json::json!({"targets": [node_ids[index].clone()]}),
            );
        }
        config
    }

    #[test]
    fn snapshot_binds_identity_and_deterministic_hashes() {
        let first = snapshot();
        let second = snapshot();
        assert_eq!(first, second);
        assert!(first.has_valid_identity());
        assert_eq!(first.ordered_nodes, vec!["node-a", "node-b", "node-c"]);
    }

    #[test]
    fn receipt_aliases_normalize_to_completion() {
        for status in ["pass", "completed"] {
            assert_eq!(
                normalize_receipt_outcome(status),
                Some(ReceiptOutcome::Completed)
            );
        }
        for status in ["approve", "approved"] {
            assert_eq!(
                normalize_receipt_outcome(status),
                Some(ReceiptOutcome::Approved)
            );
        }
        assert_eq!(
            normalize_receipt_outcome("rework"),
            Some(ReceiptOutcome::Rework)
        );
        assert_eq!(normalize_receipt_outcome("unknown"), None);
    }

    #[test]
    fn conditional_exclusion_skips_optional_node() {
        let snapshot = snapshot();
        let verdict = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "pass")),
            "node-c",
        );
        assert!(verdict.allowed);
        assert_eq!(verdict.next_node.as_deref(), Some("node-c"));
    }

    #[test]
    fn explicit_non_sequential_edge_is_authoritative() {
        let mut config = fixture();
        config["ordered_steps"][0]["next_node"] = serde_json::json!("node-c");
        let snapshot = TeamFlowSnapshot::from_config(
            &config,
            TeamFlowSnapshotInput {
                config_id: "cfg-a",
                profile: "profile-a",
                flow_ref: "flow-a",
                registry_hash: "registry-a",
            },
        )
        .expect("explicit edge should compile");
        let verdict = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "pass")),
            "node-c",
        );
        assert!(verdict.allowed);
        assert_eq!(verdict.next_node.as_deref(), Some("node-c"));
    }

    #[test]
    fn terminal_completion_allows_empty_requested_node_when_proof_is_present() {
        let snapshot = snapshot();
        let verdict = admit_transition(
            &snapshot,
            "node-c",
            Some(&receipt(&snapshot, "node-c", "completed")),
            "",
        );
        assert!(verdict.allowed);
        assert_eq!(verdict.next_node, None);

        let invented = admit_transition(
            &snapshot,
            "node-c",
            Some(&receipt(&snapshot, "node-c", "completed")),
            "invented-terminal-id",
        );
        assert_eq!(
            invented.blocker.as_deref(),
            Some(BLOCKER_TERMINAL_TRANSITION_NOT_CONFIGURED)
        );
    }

    #[test]
    fn approval_requirement_distinguishes_pass_from_approve() {
        let mut config = fixture();
        config["ordered_steps"][0]["requires_user_approval"] = serde_json::json!(true);
        config["ordered_steps"][0]["next_node"] = serde_json::json!("node-c");
        config["ordered_steps"][0]["approval_policy"] = serde_json::json!({
            "mode": "user_review_required",
            "prompt_template": "approve",
            "allowed_decisions": ["approved", "rework"]
        });
        config["dev_team"]["authority_catalog"]["approval_policy_allowed_decisions"] =
            serde_json::json!(["approved", "rework"]);
        config["dev_team"]["authority_catalog"]["approval_policy_modes"] = serde_json::json!([
            "user_review_required",
            "optional_user_review",
            "not_required"
        ]);
        config["ordered_steps"][0]["resume_transitions"] =
            serde_json::json!({"approved": "node-c"});
        let snapshot = TeamFlowSnapshot::from_config(
            &config,
            TeamFlowSnapshotInput {
                config_id: "cfg-a",
                profile: "profile-a",
                flow_ref: "flow-a",
                registry_hash: "registry-a",
            },
        )
        .expect("approval fixture should compile");
        let pass = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "pass")),
            "node-c",
        );
        assert_eq!(
            pass.blocker.as_deref(),
            Some(BLOCKER_USER_APPROVAL_REQUIRED)
        );
        let approve = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "approve")),
            "node-c",
        );
        assert!(approve.allowed);
    }

    #[test]
    fn approval_mode_contract_matrix_is_exhaustive() {
        let mut missing_mode = fixture();
        missing_mode["ordered_steps"][0]["approval_policy"] =
            serde_json::json!({"allowed_decisions": ["approved"]});
        missing_mode["dev_team"]["authority_catalog"] = serde_json::json!({
            "approval_policy_modes": [
                "user_review_required",
                "optional_user_review",
                "not_required"
            ],
            "approval_policy_allowed_decisions": ["approved", "rework"]
        });
        assert!(compile(&missing_mode).is_err());

        for mode in [
            "user_review_required",
            "optional_user_review",
            "not_required",
        ] {
            for requires_user_approval in [false, true] {
                for prompt in [false, true] {
                    for decisions in [false, true] {
                        for approved_resume in [false, true] {
                            let mut config = fixture();
                            let step = &mut config["ordered_steps"][0];
                            step["requires_user_approval"] =
                                serde_json::json!(requires_user_approval);
                            let mut policy = serde_json::json!({"mode": mode});
                            if prompt {
                                policy["prompt_template"] = serde_json::json!("review");
                            }
                            if decisions {
                                policy["allowed_decisions"] = serde_json::json!(["approved"]);
                            }
                            step["approval_policy"] = policy;
                            if approved_resume {
                                step["resume_transitions"] =
                                    serde_json::json!({"approved": "node-b"});
                            }
                            config["dev_team"]["authority_catalog"] = serde_json::json!({
                                "approval_policy_modes": [
                                    "user_review_required",
                                    "optional_user_review",
                                    "not_required"
                                ],
                                "approval_policy_allowed_decisions": ["approved", "rework"]
                            });
                            let valid = match mode {
                                "user_review_required" => {
                                    requires_user_approval && prompt && decisions && approved_resume
                                }
                                "optional_user_review" => !requires_user_approval && decisions,
                                "not_required" => {
                                    !requires_user_approval
                                        && decisions
                                        && !prompt
                                        && !approved_resume
                                }
                                _ => false,
                            };
                            assert_eq!(
                                compile(&config).is_ok(),
                                valid,
                                "mode={mode} requires={requires_user_approval} prompt={prompt} decisions={decisions} approved_resume={approved_resume}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn missing_evidence_and_artifact_path_only_fail_closed() {
        let snapshot = snapshot();
        let mut missing = receipt(&snapshot, "node-a", "approved");
        missing.evidence.clear();
        let verdict = admit_transition(&snapshot, "node-a", Some(&missing), "node-c");
        assert_eq!(
            verdict.blocker.as_deref(),
            Some(BLOCKER_REQUIRED_EVIDENCE_MISSING)
        );

        let mut path_only = missing;
        path_only.evidence = vec!["C:/tmp/proof-a.json".to_string()];
        let verdict = admit_transition(&snapshot, "node-a", Some(&path_only), "node-c");
        assert_eq!(
            verdict.blocker.as_deref(),
            Some(BLOCKER_REQUIRED_EVIDENCE_MISSING)
        );
    }

    #[test]
    fn rework_requires_explicit_configured_back_edge() {
        let snapshot = snapshot();
        let verdict = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "rework")),
            "node-a",
        );
        assert!(verdict.allowed);
        assert!(verdict.rework);

        let blocked = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "rework")),
            "node-c",
        );
        assert_eq!(
            blocked.blocker.as_deref(),
            Some(BLOCKER_REWORK_TARGET_NOT_CONFIGURED)
        );
    }

    #[test]
    fn profile_or_snapshot_mutation_fails_closed() {
        let mut snapshot = snapshot();
        snapshot.profile = "changed".to_string();
        assert!(!snapshot.has_valid_identity());
        let verdict = admit_transition(
            &snapshot,
            "node-a",
            Some(&receipt(&snapshot, "node-a", "pass")),
            "node-c",
        );
        assert_eq!(
            verdict.blocker.as_deref(),
            Some(BLOCKER_SNAPSHOT_INTEGRITY_INVALID)
        );
    }

    #[test]
    fn alias_groups_reject_conflicts_and_malformed_fallbacks() {
        let mut duplicate_sequences = fixture();
        duplicate_sequences["steps"] = duplicate_sequences["ordered_steps"].clone();
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &duplicate_sequences,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::ConflictingAliases { .. })
        ));
        let mut malformed_sequences = fixture();
        malformed_sequences["ordered_steps"] = serde_json::json!(42);
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &malformed_sequences,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { .. })
        ));

        let mut runtime_role_conflict = fixture();
        runtime_role_conflict["ordered_steps"][0]["runtime_role"] =
            serde_json::json!("runtime-override");
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &runtime_role_conflict,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::ConflictingAliases { .. })
        ));
        let mut runtime_role_malformed = fixture();
        runtime_role_malformed["ordered_steps"][0]["runtime_role"] = serde_json::json!(42);
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &runtime_role_malformed,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { .. })
        ));

        let mut inclusion_conflict = fixture();
        inclusion_conflict["ordered_steps"][0]["included"] = serde_json::json!(true);
        inclusion_conflict["ordered_steps"][0]["lane_template_included"] = serde_json::json!(true);
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &inclusion_conflict,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::ConflictingAliases { .. })
        ));
        let mut inclusion_malformed = fixture();
        inclusion_malformed["ordered_steps"][0]["included"] = serde_json::json!("yes");
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &inclusion_malformed,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { .. })
        ));

        let mut edge_conflict = fixture();
        edge_conflict["ordered_steps"][0]["next_node"] = serde_json::json!("node-c");
        edge_conflict["ordered_steps"][0]["next"] = serde_json::json!("node-c");
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &edge_conflict,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::ConflictingAliases { .. })
        ));
        let mut edge_malformed = fixture();
        edge_malformed["ordered_steps"][0]["next_node"] = serde_json::json!(42);
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &edge_malformed,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { .. })
        ));
    }

    #[test]
    fn malformed_flow_and_typed_evidence_fail_closed() {
        let malformed = serde_json::json!({"roles": {}, "ordered_steps": [42]});
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &malformed,
                TeamFlowSnapshotInput {
                    config_id: "cfg",
                    profile: "p",
                    flow_ref: "flow-a",
                    registry_hash: "r"
                }
            ),
            Err(TeamFlowSnapshotError::InvalidNode(_))
                | Err(TeamFlowSnapshotError::MissingNodeField { .. })
                | Err(TeamFlowSnapshotError::MissingField("flow_id"))
        ));

        let mut conflicting_node_alias = fixture();
        conflicting_node_alias["ordered_steps"][0]["node_id"] = serde_json::json!("other");
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &conflicting_node_alias,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::ConflictingAliases { .. })
        ));

        let mut conflicting_config_alias = fixture();
        conflicting_config_alias["config_id"] = serde_json::json!("cfg-a");
        conflicting_config_alias["id"] = serde_json::json!("cfg-b");
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &conflicting_config_alias,
                TeamFlowSnapshotInput {
                    config_id: "cfg-a",
                    profile: "profile-a",
                    flow_ref: "flow-a",
                    registry_hash: "registry-a"
                }
            ),
            Err(TeamFlowSnapshotError::ConflictingAliases { .. })
        ));

        let mut config = fixture();
        config["ordered_steps"][0]["required_outputs"] = serde_json::json!([{"id": "proof-a"}]);
        assert!(matches!(
            TeamFlowSnapshot::from_config(
                &config,
                TeamFlowSnapshotInput {
                    config_id: "cfg",
                    profile: "p",
                    flow_ref: "flow-a",
                    registry_hash: "r"
                }
            ),
            Err(TeamFlowSnapshotError::MalformedEvidence { .. })
        ));
    }

    #[test]
    fn strict_source_matrix_rejects_duplicate_conflict_and_malformed_sources() {
        let mut flow_sources = fixture();
        flow_sources["flows"] = serde_json::json!({"flow-a": fixture()});
        assert!(matches!(
            compile(&flow_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "flow_source",
                ..
            })
        ));
        let malformed_flow = serde_json::json!({"flow": 42});
        assert!(matches!(
            compile(&malformed_flow),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { field: "flow", .. })
        ));
        let malformed_catalog = serde_json::json!({"flows": 42});
        assert!(matches!(
            compile(&malformed_catalog),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { field: "flows", .. })
        ));
        for (left, right) in [
            ("flow", "flows"),
            ("flow", "flow_catalog"),
            ("flow", "project_flow_catalog"),
            ("flows", "flow_catalog"),
            ("flows", "project_flow_catalog"),
            ("flow_catalog", "project_flow_catalog"),
        ] {
            let mut pair = serde_json::json!({});
            pair[left] = if left == "flow" {
                fixture()
            } else {
                serde_json::json!({"flow-a": fixture()})
            };
            pair[right] = if right == "flow" {
                fixture()
            } else {
                serde_json::json!({"flow-a": fixture()})
            };
            assert!(
                matches!(
                    compile(&pair),
                    Err(TeamFlowSnapshotError::ConflictingAliases {
                        field: "flow_source",
                        ..
                    })
                ),
                "pair {left}/{right}"
            );
        }

        let mut role_sources = fixture();
        role_sources["role_registry"] = role_sources["roles"].clone();
        assert!(matches!(
            compile(&role_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases { field: "roles", .. })
        ));
        for (left, right) in [
            ("roles", "role_registry"),
            ("roles", "registry"),
            ("role_registry", "registry"),
        ] {
            let mut pair = fixture();
            pair[right] = pair["roles"].clone();
            if left != "roles" {
                pair[left] = pair["roles"].clone();
            }
            assert!(
                matches!(
                    compile(&pair),
                    Err(TeamFlowSnapshotError::ConflictingAliases { field: "roles", .. })
                ),
                "pair {left}/{right}"
            );
        }
        let mut malformed_roles = fixture();
        malformed_roles["roles"] = serde_json::json!(["not-an-object"]);
        assert!(matches!(
            compile(&malformed_roles),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType { field: "role", .. })
        ));

        let mut command_sources = fixture();
        command_sources["ordered_steps"][0]["command_template"] = serde_json::json!({"ref": "one"});
        command_sources["ordered_steps"][0]["command_mapping"] = serde_json::json!({"ref": "one"});
        assert!(matches!(
            compile(&command_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "command_template/command_mapping",
                ..
            })
        ));
        let mut malformed_command = fixture();
        malformed_command["ordered_steps"][0]["command_template"] = serde_json::json!("template");
        assert!(matches!(
            compile(&malformed_command),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "command_mapping",
                ..
            })
        ));
        let mut command_ref_sources = fixture();
        command_ref_sources["ordered_steps"][0]["command_ref"] = serde_json::json!("one");
        command_ref_sources["ordered_steps"][0]["command_mapping_ref"] = serde_json::json!("one");
        assert!(matches!(
            compile(&command_ref_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "command_ref",
                ..
            })
        ));
        let mut nested_command_ref_sources = fixture();
        nested_command_ref_sources["ordered_steps"][0]["command_mapping"] =
            serde_json::json!({"surface": "one", "ref": "one"});
        assert!(matches!(
            compile(&nested_command_ref_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "command_ref",
                ..
            })
        ));

        for keys in [
            ["evidence_requirements", "required_evidence"],
            ["evidence_requirements", "required_outputs"],
            ["evidence_requirements", "proof_outputs"],
        ] {
            let mut evidence_sources = fixture();
            evidence_sources["ordered_steps"][0][keys[0]] = serde_json::json!(["proof-a"]);
            evidence_sources["ordered_steps"][0][keys[1]] = serde_json::json!(["proof-a"]);
            assert!(matches!(
                compile(&evidence_sources),
                Err(TeamFlowSnapshotError::ConflictingAliases {
                    field: "evidence_requirements",
                    ..
                })
            ));
        }
        let mut proof_gate_sources = fixture();
        proof_gate_sources["ordered_steps"][0]["proof_gates"] =
            serde_json::json!({"required_outputs": ["proof-a"], "required_evidence": ["proof-a"]});
        assert!(matches!(
            compile(&proof_gate_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "proof_gates.required_outputs/required_evidence",
                ..
            })
        ));
        let mut flat_and_nested_evidence = fixture();
        flat_and_nested_evidence["ordered_steps"][0]["proof_gates"] =
            serde_json::json!({"required_outputs": ["proof-a"]});
        assert!(matches!(
            compile(&flat_and_nested_evidence),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "evidence_requirements",
                ..
            })
        ));
        let mut malformed_evidence_source = fixture();
        malformed_evidence_source["ordered_steps"][0]["proof_gates"] = serde_json::json!(42);
        assert!(matches!(
            compile(&malformed_evidence_source),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "proof_gates",
                ..
            })
        ));
        let mut evidence_id_sources = fixture();
        evidence_id_sources["ordered_steps"][0]["required_outputs"] = serde_json::json!([{"id": "proof-a", "evidence_id": "proof-a", "kind": "test", "value": "ok"}]);
        assert!(matches!(
            compile(&evidence_id_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "evidence_id",
                ..
            })
        ));
        let mut evidence_kind_sources = fixture();
        evidence_kind_sources["ordered_steps"][0]["required_outputs"] =
            serde_json::json!([{"id": "proof-a", "kind": "test", "type": "test", "value": "ok"}]);
        assert!(matches!(
            compile(&evidence_kind_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases { field: "kind", .. })
        ));
        let mut evidence_value_sources = fixture();
        evidence_value_sources["ordered_steps"][0]["required_outputs"] =
            serde_json::json!([{"id": "proof-a", "kind": "test", "value": "ok", "proof": "ok"}]);
        assert!(matches!(
            compile(&evidence_value_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases { field: "value", .. })
        ));

        let mut task_class_sources = fixture();
        task_class_sources["ordered_steps"][0]["task_class"] = serde_json::json!("class-a");
        assert_eq!(
            compile(&task_class_sources)
                .expect("node task class should use role capability")
                .nodes[0]
                .task_class,
            "class-a"
        );
        let mut incompatible_task_class = task_class_sources.clone();
        incompatible_task_class["ordered_steps"][0]["task_class"] = serde_json::json!("class-z");
        assert!(matches!(
            compile(&incompatible_task_class),
            Err(TeamFlowSnapshotError::TaskClassNotInRoleCapabilities { .. })
        ));
        let mut two_node_task_class_sources = fixture();
        two_node_task_class_sources["ordered_steps"][0]["task_class"] =
            serde_json::json!("class-a");
        two_node_task_class_sources["ordered_steps"][0]["task_classes"] =
            serde_json::json!(["class-a"]);
        assert!(matches!(
            compile(&two_node_task_class_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "task_class/task_classes",
                ..
            })
        ));
        let mut malformed_task_classes = fixture();
        malformed_task_classes["roles"]["node-a"]["task_classes"] = serde_json::json!([42]);
        assert!(matches!(
            compile(&malformed_task_classes),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "task_classes",
                ..
            })
        ));

        let mut flow_id_sources = fixture();
        flow_id_sources["id"] = serde_json::json!("flow-a");
        assert!(matches!(
            compile(&flow_id_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "flow_id",
                ..
            })
        ));
        let mut malformed_flow_id = fixture();
        malformed_flow_id["flow_id"] = serde_json::json!(42);
        assert!(matches!(
            compile(&malformed_flow_id),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "flow_id",
                ..
            })
        ));
        let mut profile_sources = fixture();
        profile_sources["profile"] = serde_json::json!("profile-a");
        profile_sources["profile_id"] = serde_json::json!("profile-a");
        assert!(matches!(
            compile(&profile_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "profile",
                ..
            })
        ));
        let mut malformed_profile = fixture();
        malformed_profile["profile"] = serde_json::json!(42);
        assert!(matches!(
            compile(&malformed_profile),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "profile",
                ..
            })
        ));
        let mut malformed_registry_hash = fixture();
        malformed_registry_hash["registry_hash"] = serde_json::json!(42);
        assert!(matches!(
            compile(&malformed_registry_hash),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "registry_hash",
                ..
            })
        ));

        let mut terminal_sources = fixture();
        terminal_sources["ordered_steps"][2]["terminal_closure"] = serde_json::json!(true);
        assert!(matches!(
            compile(&terminal_sources),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "terminal",
                ..
            })
        ));
        let mut malformed_terminal = fixture();
        malformed_terminal["ordered_steps"][2]["terminal"] = serde_json::json!("yes");
        assert!(matches!(
            compile(&malformed_terminal),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "terminal",
                ..
            })
        ));
        let mut malformed_approval = fixture();
        malformed_approval["ordered_steps"][0]["requires_user_approval"] = serde_json::json!("yes");
        assert!(matches!(
            compile(&malformed_approval),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "requires_user_approval",
                ..
            })
        ));

        let mut malformed_rework = fixture();
        malformed_rework["ordered_steps"][0]["rework_transitions"] =
            serde_json::json!({"rework": 42});
        assert!(matches!(
            compile(&malformed_rework),
            Err(TeamFlowSnapshotError::MalformedEvidence {
                field: "rework_transitions",
                ..
            })
        ));
        let mut malformed_nested_command = fixture();
        malformed_nested_command["ordered_steps"][0]["command_mapping"] =
            serde_json::json!({"surface": 42});
        assert!(matches!(
            compile(&malformed_nested_command),
            Err(TeamFlowSnapshotError::InvalidNodeFieldType {
                field: "command_ref",
                ..
            })
        ));
    }

    #[test]
    fn typed_authority_rejects_missing_flags_sequences_and_selected_flow_drift() {
        let mut missing_included = fixture();
        missing_included["ordered_steps"][0]
            .as_object_mut()
            .expect("step")
            .remove("included");
        assert!(matches!(
            compile(&missing_included),
            Err(TeamFlowSnapshotError::MissingNodeField {
                field: "included",
                ..
            })
        ));

        let mut invalid_sequence = fixture();
        invalid_sequence["execution_lane_sequence"] = serde_json::json!(["node-a", "node-c"]);
        assert!(matches!(
            compile(&invalid_sequence),
            Err(TeamFlowSnapshotError::InvalidSequence {
                field: "execution_lane_sequence"
            })
        ));

        let mut selected_flow_drift = fixture();
        selected_flow_drift["selected_flow_set"] = serde_json::json!("other-flow");
        assert!(matches!(
            compile(&selected_flow_drift),
            Err(TeamFlowSnapshotError::FlowIdentityMismatch { .. })
        ));
    }

    #[test]
    fn command_ref_requires_registry_resolution_and_consistent_mapping() {
        let mut config = fixture();
        config["ordered_steps"][0]["command_ref"] = serde_json::json!("command-a");
        config["ordered_steps"][0]["command_mapping"] =
            serde_json::json!({"command_id": "command-a", "surface": "test.surface"});
        assert!(matches!(
            compile(&config),
            Err(TeamFlowSnapshotError::UnresolvedCommandReference {
                command_ref,
                ..
            }) if command_ref == "command-a"
        ));
        config["command_catalog"] = serde_json::json!({
            "command-a": {"command_id": "command-a", "surface": "test.surface"}
        });
        assert!(compile(&config).is_ok());

        config["ordered_steps"][0]["command_mapping"]["command_id"] =
            serde_json::json!("command-b");
        assert!(matches!(
            compile(&config),
            Err(TeamFlowSnapshotError::ConflictingAliases {
                field: "command_ref",
                ..
            })
        ));
    }

    #[test]
    fn canonical_projection_requires_lifecycle_resume_and_policy_diagnostics() {
        let mut config = fixture();
        config["schema_version"] = serde_json::json!("team-flow-authority.v1");
        config["config_id"] = serde_json::json!("cfg-a");
        config["profile"] = serde_json::json!("profile-a");
        config["registry_hash"] = serde_json::json!("registry-a");
        config["flow_ref"] = serde_json::json!("flow-a");
        config["selected_flow_set"] = serde_json::json!("flow-a");
        config["execution_lane_sequence"] = serde_json::json!(["node-a", "node-b", "node-c"]);
        config["lane_sequence"] = serde_json::json!(["node-a", "node-b", "node-c"]);
        config["included_nodes"] = serde_json::json!(["node-a", "node-c"]);
        config["required_nodes"] = serde_json::json!(["node-a", "node-c"]);
        for (index, raw_step) in config["ordered_steps"]
            .as_array_mut()
            .expect("steps")
            .iter_mut()
            .enumerate()
        {
            let step = raw_step.as_object_mut().expect("step");
            let node = format!("node-{}", (b'a' + index as u8) as char);
            let evidence = if index == 1 { "proof-b" } else { "proof-a" };
            step.remove("required_outputs");
            step.remove("rework_transitions");
            step.insert("lane_id".to_string(), serde_json::json!(node));
            step.insert("dispatch_target".to_string(), serde_json::json!(node));
            step.insert("dispatch_alias".to_string(), serde_json::json!(node));
            step.insert(
                "packet_template_kind".to_string(),
                serde_json::json!("packet"),
            );
            step.insert("closure_class".to_string(), serde_json::json!("proof"));
            step.insert("stage".to_string(), serde_json::json!("execution"));
            step.insert(
                "completion_blocker".to_string(),
                serde_json::json!("pending"),
            );
            step.insert(
                "evidence_requirements".to_string(),
                serde_json::json!([evidence]),
            );
            step.insert(
                "proof_gates".to_string(),
                serde_json::json!({"required_outputs": [evidence]}),
            );
            step.insert("command_ref".to_string(), Value::Null);
            step.insert("command_mapping".to_string(), Value::Null);
            step.insert(
                "lifecycle_hook_templates".to_string(),
                serde_json::json!(["timing"]),
            );
            step.insert("resume_transitions".to_string(), serde_json::json!({}));
            step.insert(
                "policy_diagnostics".to_string(),
                serde_json::json!({
                    "source": "team_flow_authority.selected_config",
                    "fallback_used": false,
                    "fallback_fields": []
                }),
            );
            step.insert(
                "activation".to_string(),
                serde_json::json!({"source": "configured"}),
            );
            step.insert(
                "runtime_assignment".to_string(),
                serde_json::json!({"source": "configured"}),
            );
            step.insert(
                "carrier_runtime_assignment".to_string(),
                serde_json::json!({"source": "configured"}),
            );
            step.insert(
                "profile_authority".to_string(),
                serde_json::json!({
                    "team_role_id": node,
                    "runtime_role": "runtime",
                    "task_class": "class",
                    "source_path": "config"
                }),
            );
            step.insert(
                "selected_model_profile".to_string(),
                serde_json::json!({
                    "profile_id": "profile",
                    "selection_source": "configured"
                }),
            );
            step.insert(
                "carrier_relation".to_string(),
                serde_json::json!({
                    "relation_kind": "carrier_catalog",
                    "source_path": "config.carriers",
                    "selected_id": node
                }),
            );
            step.insert(
                "executor_backend_relation".to_string(),
                serde_json::json!({
                    "relation_kind": "executor_backend",
                    "source_path": "config.backends",
                    "selected_id": "backend"
                }),
            );
            step.insert(
                "authority_identities".to_string(),
                serde_json::json!([{
                    "kind": "team_flow_node",
                    "id": node,
                    "source_path": "config.ordered_steps"
                }]),
            );
            step.insert(
                "execution_identity".to_string(),
                serde_json::json!({
                    "id": node,
                    "source_fields": ["flow_id", "lane_id", "dispatch_alias"]
                }),
            );
            step.insert("approval_policy".to_string(), serde_json::json!({}));
            step.insert(
                "rework".to_string(),
                serde_json::json!({"targets": vec![node.clone()]}),
            );
        }
        let result = compile(&config);
        assert!(result.is_ok(), "{result:?}");

        config["ordered_steps"][0]
            .as_object_mut()
            .expect("step")
            .remove("policy_diagnostics");
        assert!(matches!(
            compile(&config),
            Err(TeamFlowSnapshotError::MissingNodeField {
                field: "policy_diagnostics",
                ..
            })
        ));
    }

    #[test]
    fn projection_proof_outputs_matrix_enforces_requiredness_and_transition_acceptance() {
        let template = projection_fixture();
        let cases = [
            ("required_empty", true, Vec::<&str>::new(), false),
            ("required_nonempty", true, vec!["proof-b"], true),
            ("optional_empty", false, Vec::<&str>::new(), true),
            ("optional_nonempty", false, vec!["proof-b"], true),
        ];

        for (label, required, outputs, accepted) in cases {
            let mut config = template.clone();
            let output_values = outputs
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>();
            config["ordered_steps"][1]["required"] = serde_json::json!(required);
            config["ordered_steps"][1]["evidence_requirements"] =
                serde_json::json!(output_values.clone());
            config["ordered_steps"][1]["proof_gates"]["required_outputs"] =
                serde_json::json!(output_values.clone());
            let required_nodes = config["ordered_steps"]
                .as_array()
                .expect("steps")
                .iter()
                .filter(|step| step["required"].as_bool() == Some(true))
                .map(|step| step["role_id"].as_str().expect("role_id").to_string())
                .collect::<Vec<_>>();
            config["required_nodes"] = serde_json::json!(required_nodes);

            let result = compile(&config);
            if !accepted {
                assert!(
                    matches!(
                        &result,
                        Err(TeamFlowSnapshotError::MissingNodeField {
                            field: "evidence_requirements",
                            ..
                        }) | Err(TeamFlowSnapshotError::MalformedEvidence {
                            field: "proof_gates.required_outputs",
                            ..
                        })
                    ),
                    "{label}: expected required proof output rejection, got {result:?}"
                );
                continue;
            }

            let snapshot = result.expect("projection proof matrix case should compile");
            assert_eq!(snapshot.nodes[1].required, required, "{label}");
            assert_eq!(
                snapshot.nodes[1].evidence_requirements, output_values,
                "{label}"
            );
            let roundtrip = serde_json::from_str::<TeamFlowSnapshot>(
                &serde_json::to_string(&snapshot).expect("snapshot should serialize"),
            )
            .expect("snapshot should roundtrip");
            assert_eq!(roundtrip, snapshot, "{label}");

            let verdict = admit_transition(
                &snapshot,
                "node-a",
                Some(&receipt(&snapshot, "node-a", "pass")),
                "node-c",
            );
            assert!(verdict.allowed, "{label}: {verdict:?}");
            assert_eq!(verdict.next_node.as_deref(), Some("node-c"), "{label}");
        }
    }

    #[test]
    fn projection_rework_targets_accept_empty_and_reject_invalid_shapes() {
        let template = projection_fixture();
        let steps = template["ordered_steps"].as_array().expect("steps");
        let node_ids = steps
            .iter()
            .map(|step| step["role_id"].as_str().expect("role_id").to_string())
            .collect::<Vec<_>>();
        let terminal_index = steps
            .iter()
            .position(|step| step["terminal"].as_bool() == Some(true))
            .expect("terminal node");
        let nonterminal_index = steps
            .iter()
            .position(|step| step["terminal"].as_bool() == Some(false))
            .expect("nonterminal node");
        let valid_target = serde_json::json!([node_ids[terminal_index].clone()]);
        let duplicate_target = serde_json::json!([
            node_ids[nonterminal_index].clone(),
            node_ids[nonterminal_index].clone()
        ]);
        let cases = vec![
            (
                "terminal_empty_targets",
                terminal_index,
                serde_json::json!([]),
                "accept",
            ),
            (
                "nonterminal_empty_targets",
                nonterminal_index,
                serde_json::json!([]),
                "accept",
            ),
            (
                "nonempty_valid_targets",
                nonterminal_index,
                valid_target,
                "accept",
            ),
            (
                "malformed_target_type",
                nonterminal_index,
                serde_json::json!([42]),
                "malformed",
            ),
            (
                "duplicate_targets",
                nonterminal_index,
                duplicate_target,
                "duplicate",
            ),
        ];

        for (label, index, targets, expected) in cases {
            let mut config = template.clone();
            config["ordered_steps"][index]["rework"]["targets"] = targets.clone();
            match (expected, compile(&config)) {
                ("accept", Ok(snapshot)) => {
                    let expected_targets = targets
                        .as_array()
                        .expect("targets")
                        .iter()
                        .map(|target| target.as_str().expect("target string").to_string())
                        .collect::<Vec<_>>();
                    assert_eq!(
                        snapshot.nodes[index].rework_targets, expected_targets,
                        "{label}"
                    );
                }
                ("malformed", Err(error)) => {
                    assert!(
                        matches!(error, TeamFlowSnapshotError::MalformedEvidence { .. }),
                        "{label}: {error:?}"
                    );
                }
                ("duplicate", Err(error)) => {
                    assert!(
                        matches!(error, TeamFlowSnapshotError::ConflictingAliases { .. }),
                        "{label}: {error:?}"
                    );
                }
                (expected, result) => panic!("{label}: expected {expected}, got {result:?}"),
            }
        }
    }

    #[test]
    fn source_selection_contains_no_first_win_fallback_branches() {
        let source = include_str!("team_flow_transition.rs");
        let forbidden = [
            [".", "or_else", "("].concat(),
            ["unwrap_", "or_else", "(|| flow_ref"].concat(),
            ["find(", "|value| value.is_object())"].concat(),
            ["filter_map(", "Value::as_str)"].concat(),
        ];
        for forbidden in forbidden {
            assert!(
                !source.contains(forbidden.as_str()),
                "first-win source fallback remains: {forbidden}"
            );
        }
    }
}
