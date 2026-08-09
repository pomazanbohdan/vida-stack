use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taskflow_contracts::Release1ContractStatus;

use crate::request::HostBridgeRequest;

pub const HOST_BRIDGE_RECEIPT_IDENTITY_SCHEMA_VERSION: &str = "host-bridge-receipt-identity-v1";
pub const HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION: &str =
    "host-bridge-precursor-fingerprint-v1";
pub const BLOCKER_PRECURSOR_FINGERPRINT_MISSING: &str = "host_bridge_precursor_fingerprint_missing";
const HOST_BRIDGE_PRECURSOR_NORMALIZED_FIELDS: &[&str] =
    &["dispatch_status", "lane_status", "dispatch_result_path"];

pub const HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS: &[&str] = &[
    "run_id",
    "dispatch_target",
    "dispatch_status",
    "lane_status",
    "supersedes_receipt_id",
    "exception_path_receipt_id",
    "dispatch_kind",
    "dispatch_surface",
    "dispatch_command",
    "dispatch_packet_path",
    "dispatch_result_path",
    "blocker_code",
    "downstream_dispatch_target",
    "downstream_dispatch_command",
    "downstream_dispatch_note",
    "downstream_dispatch_ready",
    "downstream_dispatch_blockers",
    "downstream_dispatch_packet_path",
    "downstream_dispatch_status",
    "downstream_dispatch_result_path",
    "downstream_dispatch_trace_path",
    "downstream_dispatch_executed_count",
    "downstream_dispatch_active_target",
    "downstream_dispatch_last_target",
    "activation_agent_type",
    "activation_runtime_role",
    "selected_backend",
    "policy_bundle_ref",
    "recorded_at",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostBridgePrecursorFingerprintV1 {
    pub schema_version: String,
    pub request_id: String,
    pub receipt: Value,
}

impl HostBridgePrecursorFingerprintV1 {
    pub fn from_dispatch_receipt(request_id: &str, receipt: &Value) -> Result<Self, String> {
        if request_id.trim().is_empty() {
            return Err(BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string());
        }
        Ok(Self {
            schema_version: HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION.to_string(),
            request_id: request_id.to_string(),
            receipt: canonical_precursor_receipt(receipt)?,
        })
    }

    pub fn from_value(value: Option<&Value>) -> Result<Self, String> {
        let value = value.ok_or_else(|| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
        let parsed: Self = serde_json::from_value(value.clone())
            .map_err(|_| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
        if parsed.schema_version != HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION
            || parsed.request_id.trim().is_empty()
        {
            return Err(BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string());
        }
        Self::from_dispatch_receipt(&parsed.request_id, &parsed.receipt)
    }

    #[must_use]
    pub fn canonical_serialization(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("host bridge precursor fingerprint serializes")
    }

    #[must_use]
    pub fn fingerprint(&self) -> String {
        format!(
            "hbpf-v1-{}",
            blake3::hash(&self.canonical_serialization()).to_hex()
        )
    }

    #[must_use]
    pub fn exact_binding_key(&self) -> String {
        format!(
            "hbpf-exact-v1-{}",
            blake3::hash(&self.canonical_serialization()).to_hex()
        )
    }

    #[must_use]
    pub fn compact_binding_key(&self) -> String {
        let compact = serde_json::json!({
            "schema_version": self.schema_version,
            "receipt": self.receipt,
        });
        format!(
            "hbpf-compact-v1-{}",
            blake3::hash(
                &serde_json::to_vec(&compact)
                    .expect("compact host bridge precursor fingerprint serializes")
            )
            .to_hex()
        )
    }

    fn candidate_with_precursor_normalization(&self, receipt: &Value) -> Result<Self, String> {
        let mut candidate = Self::from_dispatch_receipt(&self.request_id, receipt)?;
        for field in HOST_BRIDGE_PRECURSOR_NORMALIZED_FIELDS {
            candidate.receipt[*field] = self.receipt[*field].clone();
        }
        Ok(candidate)
    }

    pub fn validate_candidate_receipt(&self, receipt: &Value) -> Result<(), String> {
        let candidate = self.candidate_with_precursor_normalization(receipt)?;
        if candidate.exact_binding_key() != self.exact_binding_key()
            || candidate.compact_binding_key() != self.compact_binding_key()
        {
            return Err("host_bridge_precursor_fingerprint_conflict".to_string());
        }
        Ok(())
    }
}

fn canonical_precursor_receipt(receipt: &Value) -> Result<Value, String> {
    let object = receipt
        .as_object()
        .filter(|object| object.len() == HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS.len())
        .ok_or_else(|| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
    let mut canonical = serde_json::Map::new();
    for field in HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS {
        let value = object
            .get(*field)
            .ok_or_else(|| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
        let value = match *field {
            "dispatch_status" | "lane_status" => Value::String(normalize_precursor_status(value)?),
            "dispatch_result_path" => normalize_precursor_result_path(value)?,
            _ => value.clone(),
        };
        canonical.insert((*field).to_string(), value);
    }
    Ok(Value::Object(canonical))
}

fn normalize_precursor_status(value: &Value) -> Result<String, String> {
    let value = value
        .as_str()
        .ok_or_else(|| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
    let mut normalized = String::new();
    let mut separator = false;
    for character in value.trim().chars() {
        if character == '-' || character.is_ascii_whitespace() {
            if !normalized.is_empty() {
                separator = true;
            }
        } else {
            if separator && !normalized.ends_with('_') {
                normalized.push('_');
            }
            normalized.push(character.to_ascii_lowercase());
            separator = false;
        }
    }
    Ok(normalized)
}

fn normalize_precursor_result_path(value: &Value) -> Result<Value, String> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::String(path) => {
            let path = path.trim();
            if path.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::String(path.replace('\\', "/")))
            }
        }
        _ => Err(BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeReceiptIdentityV1 {
    pub schema_version: String,
    pub request_id: String,
    pub run_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub packet_id: String,
    pub dispatch_target: String,
    pub packet_path: String,
    pub backend_id: String,
    pub carrier_id: String,
    pub adapter_kind: String,
    pub adapter_capability_id: String,
    pub invocation_mode: String,
    pub dispatch_transport: String,
    pub receipt_mode: String,
    pub adapter_contract_source: String,
    pub adapter_contract_snapshot: Value,
    pub adapter_contract_hash: String,
    pub adapter_operations: crate::HostBridgeAdapterOperations,
    pub request_path: String,
    pub result_path: String,
    pub receipt_path: String,
    #[serde(default)]
    pub precursor_fingerprint: Option<HostBridgePrecursorFingerprintV1>,
    pub recorded_at: String,
}

impl HostBridgeReceiptIdentityV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != HOST_BRIDGE_RECEIPT_IDENTITY_SCHEMA_VERSION {
            return Err("host_bridge_receipt_identity_unknown_schema".to_string());
        }
        for (field, value) in [
            ("request_id", &self.request_id),
            ("run_id", &self.run_id),
            ("task_id", &self.task_id),
            ("attempt_id", &self.attempt_id),
            ("packet_id", &self.packet_id),
            ("dispatch_target", &self.dispatch_target),
            ("packet_path", &self.packet_path),
            ("backend_id", &self.backend_id),
            ("carrier_id", &self.carrier_id),
            ("adapter_kind", &self.adapter_kind),
            ("adapter_capability_id", &self.adapter_capability_id),
            ("invocation_mode", &self.invocation_mode),
            ("dispatch_transport", &self.dispatch_transport),
            ("receipt_mode", &self.receipt_mode),
            ("adapter_contract_source", &self.adapter_contract_source),
            ("adapter_contract_hash", &self.adapter_contract_hash),
            ("request_path", &self.request_path),
            ("result_path", &self.result_path),
            ("receipt_path", &self.receipt_path),
            ("recorded_at", &self.recorded_at),
        ] {
            if value.trim().is_empty() {
                return Err(format!("host_bridge_receipt_identity_missing:{field}"));
            }
        }
        if self.adapter_contract_snapshot != self.adapter_operations.to_value() {
            return Err("host_bridge_receipt_identity_adapter_snapshot_mismatch".to_string());
        }
        let expected_hash = blake3::hash(
            &serde_json::to_vec(&self.adapter_operations.to_value())
                .map_err(|_| "host_bridge_receipt_identity_adapter_snapshot_invalid")?,
        )
        .to_hex()
        .to_string();
        if self.adapter_contract_hash != expected_hash {
            return Err("host_bridge_receipt_identity_adapter_hash_mismatch".to_string());
        }
        let precursor = self
            .precursor_fingerprint
            .as_ref()
            .ok_or_else(|| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
        let precursor_value = serde_json::to_value(precursor)
            .map_err(|_| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?;
        let canonical = HostBridgePrecursorFingerprintV1::from_value(Some(&precursor_value))?;
        if canonical != *precursor || precursor.request_id != self.request_id {
            return Err(BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string());
        }
        for (field, expected) in [
            ("run_id", self.run_id.as_str()),
            ("dispatch_target", self.dispatch_target.as_str()),
        ] {
            if precursor.receipt.get(field).and_then(Value::as_str) != Some(expected) {
                return Err(format!(
                    "host_bridge_receipt_identity_core_mismatch:{field}"
                ));
            }
        }
        if precursor
            .receipt
            .get("selected_backend")
            .and_then(Value::as_str)
            != Some(self.backend_id.as_str())
        {
            return Err("host_bridge_receipt_identity_core_mismatch:backend_id".to_string());
        }
        if let Some(packet_path) = precursor
            .receipt
            .get("dispatch_packet_path")
            .and_then(Value::as_str)
        {
            if !crate::host_bridge_packet_paths_equivalent(&self.packet_path, packet_path) {
                return Err("host_bridge_receipt_identity_core_mismatch:packet_path".to_string());
            }
        }
        Ok(())
    }

    pub fn identity_key(&self) -> Result<String, String> {
        self.validate()?;
        Ok(self
            .precursor_fingerprint
            .as_ref()
            .expect("validated identity has precursor fingerprint")
            .exact_binding_key())
    }

    pub fn compact_binding_key(&self) -> Result<String, String> {
        self.validate()?;
        Ok(self
            .precursor_fingerprint
            .as_ref()
            .expect("validated identity has precursor fingerprint")
            .compact_binding_key())
    }

    pub fn validate_precursor_receipt(&self, receipt: &Value) -> Result<(), String> {
        self.validate()?;
        self.precursor_fingerprint
            .as_ref()
            .expect("validated identity has precursor fingerprint")
            .validate_candidate_receipt(receipt)
    }

    #[must_use]
    pub fn as_value(&self) -> Value {
        serde_json::to_value(self).expect("host bridge receipt identity serializes")
    }

    pub fn from_request(
        request: &HostBridgeRequest,
        registry: &Value,
        contract_source: &str,
        recorded_at: String,
        precursor_fingerprint: HostBridgePrecursorFingerprintV1,
    ) -> Result<Self, String> {
        let configured = crate::HostBridgeAdapterOperations::from_registry_value(registry)
            .map_err(|error| format!("host_bridge_receipt_identity_registry_invalid:{error}"))?;
        let Some(request_operations) = request.adapter_operations.as_ref() else {
            return Err("host_bridge_receipt_identity_adapter_operations_missing".to_string());
        };
        if request_operations != &configured {
            return Err(
                "host_bridge_receipt_identity_registry_drift:adapter_operations".to_string(),
            );
        }
        if request.adapter_contract_source != contract_source {
            return Err(
                "host_bridge_receipt_identity_registry_drift:adapter_contract_source".to_string(),
            );
        }
        let identity = Self {
            schema_version: HOST_BRIDGE_RECEIPT_IDENTITY_SCHEMA_VERSION.to_string(),
            request_id: request.request_id.clone(),
            run_id: request.run_id.clone(),
            task_id: request.task_id.clone(),
            attempt_id: request.attempt_id.clone(),
            packet_id: request.packet_id.clone(),
            dispatch_target: request.dispatch_target.clone(),
            packet_path: request.packet_path.display().to_string(),
            backend_id: request.backend_id.clone(),
            carrier_id: request.carrier_id.clone(),
            adapter_kind: request.adapter_kind.clone(),
            adapter_capability_id: request.adapter_capability_id.clone(),
            invocation_mode: request.invocation_mode.clone(),
            dispatch_transport: request.dispatch_transport.clone(),
            receipt_mode: request.receipt_mode.clone(),
            adapter_contract_source: request.adapter_contract_source.clone(),
            adapter_contract_snapshot: request.adapter_contract_snapshot.clone(),
            adapter_contract_hash: request.adapter_contract_hash.clone(),
            adapter_operations: request_operations.clone(),
            request_path: request.request_path.display().to_string(),
            result_path: request.result_path.display().to_string(),
            receipt_path: request.receipt_path.display().to_string(),
            precursor_fingerprint: Some(precursor_fingerprint),
            recorded_at,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate_against_registry(
        &self,
        request: &HostBridgeRequest,
        registry: &Value,
        contract_source: &str,
    ) -> Result<(), String> {
        let current = Self::from_request(
            request,
            registry,
            contract_source,
            self.recorded_at.clone(),
            self.precursor_fingerprint
                .clone()
                .ok_or_else(|| BLOCKER_PRECURSOR_FINGERPRINT_MISSING.to_string())?,
        )?;
        if current.as_value() != self.as_value() {
            return Err("host_bridge_receipt_identity_registry_or_request_drift".to_string());
        }
        Ok(())
    }

    pub fn validate_paths(&self, state_root: &Path) -> Result<(), String> {
        let root = std::fs::canonicalize(state_root)
            .map_err(|error| format!("host_bridge_receipt_identity_state_root_invalid:{error}"))?;
        for (field, raw, must_exist) in [
            ("request_path", &self.request_path, true),
            ("packet_path", &self.packet_path, true),
            ("result_path", &self.result_path, false),
            ("receipt_path", &self.receipt_path, false),
        ] {
            let path = PathBuf::from(raw);
            if path
                .components()
                .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
            {
                return Err(format!("host_bridge_receipt_identity_path_invalid:{field}"));
            }
            let resolved = if path.is_absolute() {
                path
            } else {
                root.join(path)
            };
            if let Ok(metadata) = std::fs::symlink_metadata(&resolved) {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(format!("host_bridge_receipt_identity_path_invalid:{field}"));
                }
                let canonical = std::fs::canonicalize(&resolved).map_err(|error| {
                    format!("host_bridge_receipt_identity_path_invalid:{field}:{error}")
                })?;
                if !canonical.starts_with(&root) {
                    return Err(format!(
                        "host_bridge_receipt_identity_path_out_of_root:{field}"
                    ));
                }
            } else if must_exist {
                return Err(format!("host_bridge_receipt_identity_path_missing:{field}"));
            } else if let Some(parent) = resolved.parent() {
                let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
                    format!("host_bridge_receipt_identity_path_invalid:{field}:{error}")
                })?;
                if !canonical_parent.starts_with(&root) {
                    return Err(format!(
                        "host_bridge_receipt_identity_path_out_of_root:{field}"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchReceiptBindingInput {
    pub request: HostBridgeRequest,
    pub receipt: Option<Value>,
    pub allow_active_packet_target_override: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchReceiptBindingDecision {
    pub accepted: bool,
    pub blocker_codes: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeResultScaffoldInput {
    pub request: HostBridgeRequest,
    pub proof_outputs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub decision: Option<String>,
    pub verdict: Option<String>,
    pub blocker_codes: Vec<String>,
    pub rework_target: Option<String>,
    pub allowed_next_node: Option<String>,
    pub summary: Option<String>,
    pub host_agent_id: Option<String>,
    pub receipt_id: Option<String>,
}

fn request_identity_projection(request: &HostBridgeRequest) -> Value {
    serde_json::json!({
        "request_id": request.request_id,
        "run_id": request.run_id,
        "task_id": request.task_id,
        "attempt_id": request.attempt_id,
        "packet_id": request.packet_id,
        "dispatch_target": request.dispatch_target,
        "backend_id": request.backend_id,
        "carrier_id": request.carrier_id,
        "adapter_kind": request.adapter_kind,
        "adapter_capability_id": request.adapter_capability_id,
        "invocation_mode": request.invocation_mode,
        "dispatch_transport": request.dispatch_transport,
        "receipt_mode": request.receipt_mode,
        "adapter_contract_source": request.adapter_contract_source,
        "adapter_contract_snapshot": request.adapter_contract_snapshot,
        "adapter_contract_hash": request.adapter_contract_hash,
        "adapter_operations": request.adapter_operations.as_ref().map(|ops| ops.to_value()),
        "request_path": request.request_path,
        "packet_path": request.packet_path,
        "result_path": request.result_path,
        "receipt_path": request.receipt_path,
    })
}

fn receipt_identity_value(receipt: &Value, field: &str) -> Option<Value> {
    receipt
        .get(field)
        .cloned()
        .or_else(|| receipt.get("identity_binding")?.get(field).cloned())
}

fn receipt_identity_projection(receipt: &Value) -> Value {
    serde_json::json!({
        "request_id": receipt_identity_value(receipt, "request_id"),
        "run_id": receipt_identity_value(receipt, "run_id"),
        "task_id": receipt_identity_value(receipt, "task_id"),
        "attempt_id": receipt_identity_value(receipt, "attempt_id"),
        "packet_id": receipt_identity_value(receipt, "packet_id"),
        "dispatch_target": receipt_identity_value(receipt, "dispatch_target"),
        "backend_id": receipt_identity_value(receipt, "backend_id")
            .or_else(|| receipt_identity_value(receipt, "selected_backend")),
        "carrier_id": receipt_identity_value(receipt, "carrier_id"),
        "adapter_kind": receipt_identity_value(receipt, "adapter_kind"),
        "adapter_capability_id": receipt_identity_value(receipt, "adapter_capability_id"),
        "invocation_mode": receipt_identity_value(receipt, "invocation_mode"),
        "dispatch_transport": receipt_identity_value(receipt, "dispatch_transport"),
        "receipt_mode": receipt_identity_value(receipt, "receipt_mode"),
        "adapter_contract_source": receipt_identity_value(receipt, "adapter_contract_source"),
        "adapter_contract_snapshot": receipt_identity_value(receipt, "adapter_contract_snapshot"),
        "adapter_contract_hash": receipt_identity_value(receipt, "adapter_contract_hash"),
        "adapter_operations": receipt_identity_value(receipt, "adapter_operations"),
        "request_path": receipt_identity_value(receipt, "request_path"),
        "source_dispatch_packet_path": receipt_identity_value(receipt, "source_dispatch_packet_path")
            .or_else(|| receipt_identity_value(receipt, "packet_path")),
        "result_path": receipt_identity_value(receipt, "result_path"),
        "receipt_path": receipt_identity_value(receipt, "receipt_path"),
    })
}

pub fn validate_dispatch_receipt_binding(
    input: &DispatchReceiptBindingInput,
) -> DispatchReceiptBindingDecision {
    let Some(receipt) = input.receipt.as_ref() else {
        return rejected(vec!["missing_dispatch_receipt".to_string()]);
    };

    let mut blockers = Vec::new();
    if receipt.get("receipt_backed").is_some()
        && receipt.get("receipt_backed").and_then(Value::as_bool) != Some(true)
    {
        blockers.push("receipt_not_receipt_backed".to_string());
    }
    let active_dispatch_status = string_field(receipt, "dispatch_status").is_some_and(|status| {
        matches!(
            status,
            "routed" | "executing" | "bridge_request_pending" | "blocked"
        )
    });
    if string_field(receipt, "status").is_some_and(|status| status != "pass")
        && !active_dispatch_status
    {
        blockers.push("receipt_status_not_pass".to_string());
    }
    if string_field(receipt, "request_id") != Some(input.request.request_id.as_str()) {
        blockers.push("receipt_request_id_mismatch".to_string());
    }
    if string_field(receipt, "run_id") != Some(input.request.run_id.as_str()) {
        blockers.push("receipt_run_id_mismatch".to_string());
    }
    if string_field(receipt, "dispatch_target") != Some(input.request.dispatch_target.as_str())
        && !input.allow_active_packet_target_override
    {
        blockers.push("receipt_dispatch_target_mismatch".to_string());
    }
    let request_identity = request_identity_projection(&input.request);
    let mut receipt_identity = receipt_identity_projection(receipt);
    if input.allow_active_packet_target_override {
        if let Some(identity) = receipt_identity.as_object_mut() {
            identity.insert(
                "dispatch_target".to_string(),
                Value::String(input.request.dispatch_target.clone()),
            );
        }
    }
    blockers.extend(crate::host_bridge_dispatch_identity_blockers(
        &request_identity,
        &receipt_identity,
    ));
    blockers.sort();
    blockers.dedup();

    if blockers.is_empty() {
        DispatchReceiptBindingDecision {
            accepted: true,
            blocker_codes: blockers,
            reason: "dispatch receipt is bound to the host bridge request".to_string(),
        }
    } else {
        rejected(blockers)
    }
}

#[must_use]
pub fn build_host_bridge_result_scaffold(input: HostBridgeResultScaffoldInput) -> Value {
    let allowed_next_node = input.allowed_next_node.or_else(|| {
        input
            .request
            .raw
            .get("allowed_next_node")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    });
    let blocked = !input.blocker_codes.is_empty();
    let decision = input.decision.unwrap_or_else(|| {
        if blocked {
            "rework_required".to_string()
        } else {
            "approve".to_string()
        }
    });
    let verdict = input.verdict.unwrap_or_else(|| {
        if blocked {
            "blocked".to_string()
        } else {
            "pass".to_string()
        }
    });
    let status = if blocked {
        Release1ContractStatus::Blocked.as_str()
    } else {
        Release1ContractStatus::Pass.as_str()
    };
    let execution_state = if blocked { "blocked" } else { "executed" };
    let summary = input.summary.unwrap_or_else(|| {
        format!(
            "parent host adapter staged {verdict} result for {}",
            input.request.dispatch_target
        )
    });
    let attempt_id = Value::String(input.request.attempt_id.clone());
    let packet_id = Value::String(input.request.packet_id.clone());
    let selected_backend = input
        .request
        .raw
        .get("selected_backend")
        .cloned()
        .unwrap_or_else(|| Value::String(input.request.backend_id.clone()));

    serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "host_tool_bridge_result",
        "status": status,
        "execution_state": execution_state,
        "request_id": input.request.request_id,
        "run_id": input.request.run_id,
        "task_id": input.request.task_id,
        "attempt_id": attempt_id.clone(),
        "packet_id": packet_id.clone(),
        "dispatch_target": input.request.dispatch_target,
        "backend_id": input.request.backend_id,
        "selected_backend": selected_backend.clone(),
        "carrier_id": input.request.carrier_id,
        "adapter_kind": input.request.adapter_kind,
        "adapter_capability_id": input.request.adapter_capability_id,
        "invocation_mode": input.request.invocation_mode,
        "dispatch_transport": input.request.dispatch_transport,
        "receipt_mode": input.request.receipt_mode,
        "adapter_contract_source": input.request.adapter_contract_source,
        "adapter_contract_snapshot": input.request.adapter_contract_snapshot,
        "adapter_contract_hash": input.request.adapter_contract_hash,
        "adapter_operations": input.request.adapter_operations.as_ref().map(|ops| ops.to_value()),
        "request_path": input.request.request_path,
        "packet_path": input.request.packet_path,
        "result_path": input.request.result_path,
        "receipt_path": input.request.receipt_path,
        "decision": decision,
        "verdict": verdict,
        "blocker_codes": input.blocker_codes,
        "rework_target": input.rework_target,
        "allowed_next_node": allowed_next_node,
        "proof_outputs": input.proof_outputs,
        "artifact_refs": input.artifact_refs,
        "summary": summary,
        "carrier_id": input.request.carrier_id,
        "adapter_kind": input.request.adapter_kind,
        "adapter_capability_id": input.request.adapter_capability_id,
        "invocation_mode": input.request.invocation_mode,
        "dispatch_transport": input.request.dispatch_transport,
        "receipt_mode": input.request.receipt_mode,
        "adapter_contract_source": input.request.adapter_contract_source,
        "adapter_contract_snapshot": input.request.adapter_contract_snapshot,
        "adapter_contract_hash": input.request.adapter_contract_hash,
        "adapter_operations": input.request.adapter_operations.as_ref().map(|ops| ops.to_value()),
        "request_path": input.request.request_path,
        "result_path": input.request.result_path,
        "receipt_path": input.request.receipt_path,
        "execution_evidence": {
            "receipt_backed": true,
            "source": "vida_agent_host_bridge_scaffold",
            "host_agent_id": input.host_agent_id,
            "receipt_id": input.receipt_id,
            "request_id": input.request.request_id,
            "run_id": input.request.run_id,
            "task_id": input.request.task_id,
            "attempt_id": attempt_id.clone(),
            "packet_id": packet_id.clone(),
            "backend_id": input.request.backend_id,
            "selected_backend": selected_backend.clone()
        },
        "source_dispatch_packet_path": input.request.packet_path,
        "identity_binding": {
            "request_id": input.request.request_id,
            "run_id": input.request.run_id,
            "task_id": input.request.task_id,
            "attempt_id": attempt_id,
            "packet_id": packet_id,
            "dispatch_target": input.request.dispatch_target,
            "backend_id": input.request.backend_id,
            "selected_backend": selected_backend,
            "carrier_id": input.request.carrier_id,
            "adapter_kind": input.request.adapter_kind,
            "adapter_capability_id": input.request.adapter_capability_id,
            "invocation_mode": input.request.invocation_mode,
            "dispatch_transport": input.request.dispatch_transport,
            "receipt_mode": input.request.receipt_mode,
            "adapter_contract_source": input.request.adapter_contract_source,
            "adapter_contract_snapshot": input.request.adapter_contract_snapshot,
            "adapter_contract_hash": input.request.adapter_contract_hash,
            "adapter_operations": input.request.adapter_operations.as_ref().map(|ops| ops.to_value()),
            "request_path": input.request.request_path,
            "packet_path": input.request.packet_path,
            "result_path": input.request.result_path,
            "receipt_path": input.request.receipt_path
        }
    })
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn rejected(blocker_codes: Vec<String>) -> DispatchReceiptBindingDecision {
    DispatchReceiptBindingDecision {
        accepted: false,
        blocker_codes,
        reason: "dispatch receipt binding rejected fail-closed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::minimal_request;

    fn precursor_receipt() -> Value {
        serde_json::json!({
            "run_id": "run-1",
            "dispatch_target": "developer",
            "dispatch_status": "bridge_request_pending",
            "lane_status": "lane_running",
            "supersedes_receipt_id": "receipt-0",
            "exception_path_receipt_id": null,
            "dispatch_kind": "host_bridge",
            "dispatch_surface": "agent",
            "dispatch_command": "dispatch",
            "dispatch_packet_path": "runtime/packet.json",
            "dispatch_result_path": "runtime/result.json",
            "blocker_code": null,
            "downstream_dispatch_target": "tester",
            "downstream_dispatch_command": "test",
            "downstream_dispatch_note": "ready",
            "downstream_dispatch_ready": true,
            "downstream_dispatch_blockers": ["proof_pending"],
            "downstream_dispatch_packet_path": "runtime/downstream.json",
            "downstream_dispatch_status": "packet_ready",
            "downstream_dispatch_result_path": "runtime/downstream-result.json",
            "downstream_dispatch_trace_path": "runtime/trace.json",
            "downstream_dispatch_executed_count": 1,
            "downstream_dispatch_active_target": "tester",
            "downstream_dispatch_last_target": "developer",
            "activation_agent_type": "worker",
            "activation_runtime_role": "implementer",
            "selected_backend": "internal_subagents",
            "policy_bundle_ref": {
                "policy_id": "rhai.runtime.authority",
                "version": 1,
                "content_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "recorded_at": "2026-07-28T00:00:00Z"
        })
    }

    #[test]
    fn precursor_fingerprint_mutation_matrix_conflicts_for_every_receipt_field() {
        let receipt = precursor_receipt();
        let baseline =
            HostBridgePrecursorFingerprintV1::from_dispatch_receipt("request-1", &receipt)
                .expect("baseline fingerprint");
        let mutations = [
            ("run_id", serde_json::json!("run-2")),
            ("dispatch_target", serde_json::json!("tester")),
            ("dispatch_status", serde_json::json!("executed")),
            ("lane_status", serde_json::json!("lane_completed")),
            ("supersedes_receipt_id", serde_json::json!("receipt-x")),
            (
                "exception_path_receipt_id",
                serde_json::json!("exception-1"),
            ),
            ("dispatch_kind", serde_json::json!("direct")),
            ("dispatch_surface", serde_json::json!("consume")),
            ("dispatch_command", serde_json::json!("other")),
            (
                "dispatch_packet_path",
                serde_json::json!("runtime/other.json"),
            ),
            (
                "dispatch_result_path",
                serde_json::json!("runtime/other-result.json"),
            ),
            ("blocker_code", serde_json::json!("blocked")),
            ("downstream_dispatch_target", serde_json::json!("reviewer")),
            ("downstream_dispatch_command", serde_json::json!("review")),
            ("downstream_dispatch_note", serde_json::json!("changed")),
            ("downstream_dispatch_ready", serde_json::json!(false)),
            ("downstream_dispatch_blockers", serde_json::json!(["other"])),
            (
                "downstream_dispatch_packet_path",
                serde_json::json!("runtime/other-downstream.json"),
            ),
            ("downstream_dispatch_status", serde_json::json!("blocked")),
            (
                "downstream_dispatch_result_path",
                serde_json::json!("runtime/other-downstream-result.json"),
            ),
            (
                "downstream_dispatch_trace_path",
                serde_json::json!("runtime/other-trace.json"),
            ),
            ("downstream_dispatch_executed_count", serde_json::json!(2)),
            (
                "downstream_dispatch_active_target",
                serde_json::json!("reviewer"),
            ),
            (
                "downstream_dispatch_last_target",
                serde_json::json!("tester"),
            ),
            ("activation_agent_type", serde_json::json!("reviewer")),
            ("activation_runtime_role", serde_json::json!("verification")),
            ("selected_backend", serde_json::json!("external")),
            (
                "policy_bundle_ref",
                serde_json::json!({
                    "policy_id": "rhai.runtime.authority",
                    "version": 2,
                    "content_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                }),
            ),
            ("recorded_at", serde_json::json!("2026-07-28T00:00:01Z")),
        ];

        assert_eq!(mutations.len(), HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS.len());
        for (field, mutation) in mutations {
            let mut changed = receipt.clone();
            changed[field] = mutation;
            let changed =
                HostBridgePrecursorFingerprintV1::from_dispatch_receipt("request-1", &changed)
                    .expect("mutated fingerprint");
            assert_ne!(
                baseline.fingerprint(),
                changed.fingerprint(),
                "mutation of `{field}` must conflict"
            );
        }
    }

    #[test]
    fn precursor_fingerprint_normalizes_only_statuses_and_dispatch_result_path() {
        let receipt = precursor_receipt();
        let baseline =
            HostBridgePrecursorFingerprintV1::from_dispatch_receipt("request-1", &receipt)
                .expect("baseline fingerprint");
        let mut variant = receipt;
        variant["dispatch_status"] = serde_json::json!(" Bridge-Request Pending ");
        variant["lane_status"] = serde_json::json!(" Lane Running ");
        variant["dispatch_result_path"] = serde_json::json!(r" runtime\result.json ");
        let variant =
            HostBridgePrecursorFingerprintV1::from_dispatch_receipt("request-1", &variant)
                .expect("normalized variant");

        assert_eq!(baseline, variant);
        assert_eq!(baseline.fingerprint(), variant.fingerprint());
        assert_eq!(baseline.exact_binding_key(), variant.exact_binding_key());
        assert_eq!(
            baseline.compact_binding_key(),
            variant.compact_binding_key()
        );
    }

    #[test]
    fn precursor_fingerprint_compact_key_excludes_only_request_id() {
        let receipt = precursor_receipt();
        let first = HostBridgePrecursorFingerprintV1::from_dispatch_receipt("request-1", &receipt)
            .expect("first fingerprint");
        let duplicate =
            HostBridgePrecursorFingerprintV1::from_dispatch_receipt("request-2", &receipt)
                .expect("duplicate fingerprint");

        assert_ne!(first.fingerprint(), duplicate.fingerprint());
        assert_ne!(first.exact_binding_key(), duplicate.exact_binding_key());
        assert_eq!(first.compact_binding_key(), duplicate.compact_binding_key());
        assert_eq!(
            first.compact_binding_key(),
            first.compact_binding_key(),
            "compact key must be deterministic"
        );
    }

    #[test]
    fn precursor_fingerprint_missing_or_malformed_fails_closed() {
        assert_eq!(
            HostBridgePrecursorFingerprintV1::from_value(None).expect_err("missing must block"),
            BLOCKER_PRECURSOR_FINGERPRINT_MISSING
        );
        assert_eq!(
            HostBridgePrecursorFingerprintV1::from_value(Some(&serde_json::json!({
                "schema_version": HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION,
                "request_id": "request-1",
                "receipt": {"run_id": "run-1"}
            })))
            .expect_err("malformed must block"),
            BLOCKER_PRECURSOR_FINGERPRINT_MISSING
        );
    }

    #[test]
    fn receipt_binding_rejects_missing_receipt() {
        let decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
            request: minimal_request(),
            receipt: None,
            allow_active_packet_target_override: false,
        });

        assert!(!decision.accepted);
        assert_eq!(decision.blocker_codes, vec!["missing_dispatch_receipt"]);
    }

    #[test]
    fn receipt_binding_allows_active_packet_target_override() {
        let request = minimal_request();
        let mut receipt = build_host_bridge_result_scaffold(HostBridgeResultScaffoldInput {
            request: request.clone(),
            proof_outputs: Vec::new(),
            artifact_refs: Vec::new(),
            decision: None,
            verdict: None,
            blocker_codes: Vec::new(),
            rework_target: None,
            allowed_next_node: Some("next_node".to_string()),
            summary: None,
            host_agent_id: Some("host-agent".to_string()),
            receipt_id: Some("receipt-1".to_string()),
        });
        receipt["dispatch_status"] = Value::String("bridge_request_pending".to_string());
        receipt["dispatch_target"] = Value::String("coach".to_string());
        let decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
            request,
            receipt: Some(receipt),
            allow_active_packet_target_override: true,
        });

        assert!(decision.accepted);
    }

    #[test]
    fn result_scaffold_defaults_required_fields_and_binds_identity() {
        let mut request = minimal_request();
        request.dispatch_target = "analyst".to_string();
        request.raw = serde_json::json!({
            "allowed_next_node": "pass_to_designer",
            "attempt_id": "attempt-1",
            "packet_id": "packet-1",
            "backend_id": "internal_subagents",
            "selected_backend": "internal_subagents"
        });

        let result = build_host_bridge_result_scaffold(HostBridgeResultScaffoldInput {
            request,
            proof_outputs: vec![
                "changed_files".to_string(),
                "verification_notes".to_string(),
            ],
            artifact_refs: vec!["artifacts/focused-proof.txt".to_string()],
            decision: None,
            verdict: None,
            blocker_codes: Vec::new(),
            rework_target: None,
            allowed_next_node: None,
            summary: None,
            host_agent_id: Some("host-agent-1".to_string()),
            receipt_id: Some("receipt-1".to_string()),
        });

        assert_eq!(result["artifact_kind"], "host_tool_bridge_result");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["decision"], "approve");
        assert_eq!(result["verdict"], "pass");
        assert_eq!(result["blocker_codes"], serde_json::json!([]));
        assert_eq!(
            result["source_dispatch_packet_path"],
            "runtime-consumption/packet.json"
        );
        assert!(result.get("rework_target").is_some());
        assert_eq!(result["allowed_next_node"], "pass_to_designer");
        assert_eq!(result["attempt_id"], "attempt-1");
        assert_eq!(result["packet_id"], "packet-1");
        assert_eq!(result["selected_backend"], "internal_subagents");
        assert_eq!(
            result["proof_outputs"],
            serde_json::json!(["changed_files", "verification_notes"])
        );
        assert_eq!(
            result["artifact_refs"],
            serde_json::json!(["artifacts/focused-proof.txt"])
        );
        assert_eq!(result["identity_binding"]["request_id"], "req-1");
        assert_eq!(result["identity_binding"]["run_id"], "run-1");
        assert_eq!(result["identity_binding"]["attempt_id"], "attempt-1");
        assert_eq!(result["identity_binding"]["packet_id"], "packet-1");
        assert_eq!(result["identity_binding"]["dispatch_target"], "analyst");
        assert_eq!(result["execution_evidence"]["receipt_backed"], true);
        assert_eq!(result["execution_evidence"]["attempt_id"], "attempt-1");
        assert_eq!(result["execution_evidence"]["packet_id"], "packet-1");
    }

    fn valid_receipt_identity() -> HostBridgeReceiptIdentityV1 {
        let request = minimal_request();
        let mut precursor_receipt = precursor_receipt();
        precursor_receipt["dispatch_packet_path"] =
            Value::String(request.packet_path.display().to_string());
        let precursor = HostBridgePrecursorFingerprintV1::from_dispatch_receipt(
            &request.request_id,
            &precursor_receipt,
        )
        .expect("precursor fingerprint");
        HostBridgeReceiptIdentityV1::from_request(
            &request,
            &request.adapter_contract_snapshot,
            "request",
            "2026-07-18T00:00:00Z".to_string(),
            precursor,
        )
        .expect("minimal request should produce a valid receipt identity")
    }

    #[test]
    fn receipt_identity_v1_validates_and_rejects_tampering() {
        let identity = valid_receipt_identity();
        identity.validate().expect("identity should validate");
        assert_eq!(
            identity.identity_key().expect("identity key"),
            identity
                .precursor_fingerprint
                .as_ref()
                .expect("precursor fingerprint")
                .exact_binding_key()
        );

        let mut missing_attempt = identity.clone();
        missing_attempt.attempt_id.clear();
        assert_eq!(
            missing_attempt
                .validate()
                .expect_err("missing attempt must block"),
            "host_bridge_receipt_identity_missing:attempt_id"
        );

        let mut mutated_snapshot = identity.clone();
        mutated_snapshot.adapter_contract_snapshot["dispose_policy"] = serde_json::json!("forged");
        assert_eq!(
            mutated_snapshot
                .validate()
                .expect_err("snapshot mutation must block"),
            "host_bridge_receipt_identity_adapter_snapshot_mismatch"
        );

        let mut mutated_hash = identity.clone();
        mutated_hash.adapter_contract_hash = "forged-hash".to_string();
        assert_eq!(
            mutated_hash
                .validate()
                .expect_err("hash mutation must block"),
            "host_bridge_receipt_identity_adapter_hash_mismatch"
        );

        let mut mutated_operations = identity.clone();
        mutated_operations
            .adapter_operations
            .operations
            .insert("spawn".to_string(), "forged.spawn".to_string());
        assert_eq!(
            mutated_operations
                .validate()
                .expect_err("operations mutation must block"),
            "host_bridge_receipt_identity_adapter_snapshot_mismatch"
        );
    }

    #[test]
    fn receipt_identity_v1_rejects_registry_drift() {
        let request = minimal_request();
        let identity = valid_receipt_identity();
        let mut registry = request.adapter_contract_snapshot.clone();
        registry["operations"]["spawn"] = serde_json::json!("forged.spawn");
        let error = identity
            .validate_against_registry(&request, &registry, "request")
            .expect_err("registry drift must block identity reuse");
        assert!(error.contains("registry_drift") || error.contains("registry_or_request_drift"));
    }
}
