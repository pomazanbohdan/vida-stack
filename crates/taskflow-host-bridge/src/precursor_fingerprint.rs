use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION: &str =
    "host-bridge-precursor-fingerprint-v1";
pub const HOST_BRIDGE_PRECURSOR_FINGERPRINT_MISSING: &str =
    "host_bridge_precursor_fingerprint_missing";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgePrecursorFingerprintV1 {
    #[serde(default)]
    pub schema_version: String,
    pub run_id: String,
    pub dispatch_target: String,
    pub dispatch_status: String,
    #[serde(default)]
    pub lane_status: String,
    pub supersedes_receipt_id: Option<String>,
    pub exception_path_receipt_id: Option<String>,
    pub dispatch_kind: String,
    pub dispatch_surface: Option<String>,
    pub dispatch_command: Option<String>,
    pub dispatch_packet_path: Option<String>,
    pub dispatch_result_path: Option<String>,
    pub blocker_code: Option<String>,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_command: Option<String>,
    pub downstream_dispatch_note: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_blockers: Vec<String>,
    pub downstream_dispatch_packet_path: Option<String>,
    pub downstream_dispatch_status: Option<String>,
    pub downstream_dispatch_result_path: Option<String>,
    pub downstream_dispatch_trace_path: Option<String>,
    pub downstream_dispatch_executed_count: u32,
    pub downstream_dispatch_active_target: Option<String>,
    pub downstream_dispatch_last_target: Option<String>,
    pub activation_agent_type: Option<String>,
    pub activation_runtime_role: Option<String>,
    pub selected_backend: Option<String>,
    pub recorded_at: String,
}

impl HostBridgePrecursorFingerprintV1 {
    pub fn from_value(value: Option<&Value>) -> Result<Self, String> {
        value
            .map(Self::from_receipt_value)
            .unwrap_or_else(|| Err(HOST_BRIDGE_PRECURSOR_FINGERPRINT_MISSING.to_string()))
    }

    pub fn from_receipt_value(receipt: &Value) -> Result<Self, String> {
        let mut fingerprint: Self = serde_json::from_value(receipt.clone())
            .map_err(|error| format!("host_bridge_precursor_fingerprint_receipt_invalid:{error}"))?;
        fingerprint.schema_version =
            HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION.to_string();
        fingerprint.dispatch_status = fingerprint.dispatch_status.trim().to_string();
        fingerprint.lane_status = normalize_lane_status(&fingerprint.lane_status);
        fingerprint.dispatch_result_path =
            fingerprint.dispatch_result_path.map(normalize_result_path);
        Ok(fingerprint)
    }

    pub fn from_run_graph_dispatch_receipt(receipt: &Value) -> Result<Self, String> {
        Self::from_receipt_value(receipt)
    }

    #[must_use]
    pub fn canonical_value(&self) -> Value {
        serde_json::to_value(self).expect("host bridge precursor fingerprint serializes")
    }

    #[must_use]
    pub fn canonical_blake3_digest(&self) -> String {
        let canonical_bytes =
            serde_json::to_vec(self).expect("host bridge precursor fingerprint serializes");
        blake3::hash(&canonical_bytes).to_hex().to_string()
    }

    #[must_use]
    pub fn digest(&self) -> String {
        self.canonical_blake3_digest()
    }
}

#[must_use]
pub fn host_bridge_precursor_fingerprint(receipt: &Value) -> Result<String, String> {
    HostBridgePrecursorFingerprintV1::from_receipt_value(receipt)
        .map(|fingerprint| fingerprint.canonical_blake3_digest())
}

fn normalize_lane_status(value: &str) -> String {
    let trimmed = value.trim();
    taskflow_contracts::canonical_lane_status_str(trimmed)
        .unwrap_or(trimmed)
        .to_string()
}

fn normalize_result_path(value: String) -> String {
    let normalized = value.trim().replace('\\', "/");
    normalized
        .strip_prefix("//?/")
        .unwrap_or(&normalized)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn receipt() -> Value {
        json!({
            "run_id": "run-1",
            "dispatch_target": "developer",
            "dispatch_status": "executed",
            "lane_status": "lane_completed",
            "supersedes_receipt_id": null,
            "exception_path_receipt_id": null,
            "dispatch_kind": "implementation",
            "dispatch_surface": "agent-init",
            "dispatch_command": "vida agent-init",
            "dispatch_packet_path": "packets/request.json",
            "dispatch_result_path": "results/receipt.json",
            "blocker_code": null,
            "downstream_dispatch_target": null,
            "downstream_dispatch_command": null,
            "downstream_dispatch_note": null,
            "downstream_dispatch_ready": false,
            "downstream_dispatch_blockers": [],
            "downstream_dispatch_packet_path": null,
            "downstream_dispatch_status": null,
            "downstream_dispatch_result_path": null,
            "downstream_dispatch_trace_path": null,
            "downstream_dispatch_executed_count": 0,
            "downstream_dispatch_active_target": null,
            "downstream_dispatch_last_target": null,
            "activation_agent_type": null,
            "activation_runtime_role": null,
            "selected_backend": "internal_subagents",
            "recorded_at": "2026-07-28T00:00:00Z"
        })
    }

    #[test]
    fn immutable_receipt_field_mutation_changes_fingerprint() {
        let original = HostBridgePrecursorFingerprintV1::from_receipt_value(&receipt())
            .expect("receipt should fingerprint");
        let mut mutated = receipt();
        mutated["dispatch_kind"] = json!("analysis");

        let changed = HostBridgePrecursorFingerprintV1::from_receipt_value(&mutated)
            .expect("mutated receipt should fingerprint");
        assert_ne!(original.digest(), changed.digest());
    }

    #[test]
    fn only_declared_fields_normalize_before_fingerprint() {
        let mut normalized = receipt();
        normalized["dispatch_status"] = json!(" executed ");
        normalized["lane_status"] = json!(" lane_completed ");
        normalized["dispatch_result_path"] = json!(r"\\?\results\receipt.json");

        let canonical = HostBridgePrecursorFingerprintV1::from_receipt_value(&receipt())
            .expect("receipt should fingerprint");
        let normalized = HostBridgePrecursorFingerprintV1::from_receipt_value(&normalized)
            .expect("normalized receipt should fingerprint");

        assert_eq!(canonical, normalized);
    }

    #[test]
    fn exact_key_retains_request_id_and_compact_key_excludes_it() {
        let exact_a = crate::host_bridge_receipt_identity_key(
            "run-1",
            "developer",
            "packets/request.json",
            "request-a",
        );
        let exact_b = crate::host_bridge_receipt_identity_key(
            "run-1",
            "developer",
            "packets/request.json",
            "request-b",
        );
        let compact_a = crate::host_bridge_receipt_identity_compact_key(
            "run-1",
            "developer",
            "packets/request.json",
        );
        let compact_b = crate::host_bridge_receipt_identity_compact_key(
            "run-1",
            "developer",
            "packets/request.json",
        );

        assert_ne!(exact_a, exact_b);
        assert_eq!(compact_a, compact_b);
    }
}
