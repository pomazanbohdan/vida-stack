use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Config-resolved lifecycle operations for a parent-host adapter.
///
/// The VIDA binary may render and validate this contract, but it never invokes
/// these operations. The parent host owns spawn/wait/dispose execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeAdapterOperations {
    pub adapter_kind: String,
    pub adapter_capability_id: String,
    pub invocation_mode: String,
    pub dispatch_transport: String,
    pub receipt_mode: String,
    pub operations: BTreeMap<String, String>,
    pub dispose_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostBridgeAdapterContractError {
    MissingField(&'static str),
    InvalidField(&'static str),
    MissingOperation(&'static str),
    MissingDisposePolicy,
}

impl std::fmt::Display for HostBridgeAdapterContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => {
                write!(f, "host bridge adapter registry missing `{field}`")
            }
            Self::InvalidField(field) => {
                write!(f, "host bridge adapter registry has invalid `{field}`")
            }
            Self::MissingOperation(operation) => {
                write!(
                    f,
                    "host bridge adapter registry missing `{operation}` operation"
                )
            }
            Self::MissingDisposePolicy => {
                f.write_str("host bridge adapter registry must declare dispose or dispose_policy")
            }
        }
    }
}

impl std::error::Error for HostBridgeAdapterContractError {}

impl HostBridgeAdapterOperations {
    /// Resolve an adapter contract from a config/registry row. Legacy operation
    /// aliases are accepted only at this boundary; emitted JSON always uses the
    /// canonical `operations` map.
    pub fn from_registry_value(value: &Value) -> Result<Self, HostBridgeAdapterContractError> {
        let source = value
            .get("host_tool_bridge")
            .or_else(|| value.get("adapter_registry"))
            .unwrap_or(value);
        let required = |field: &'static str| {
            source
                .get(field)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .ok_or(HostBridgeAdapterContractError::MissingField(field))
        };
        let adapter_kind = required("adapter_kind")?;
        let adapter_capability_id = required("adapter_capability_id")?;
        let invocation_mode = required("invocation_mode")?;
        let dispatch_transport = required("dispatch_transport")?;
        let receipt_mode = required("receipt_mode")?;

        let mut operations = BTreeMap::new();
        if let Some(map) = source.get("operations").and_then(Value::as_object) {
            for (key, value) in map {
                if let Some(operation) = value.as_str().map(str::trim).filter(|v| !v.is_empty()) {
                    operations.insert(canonical_operation_name(key), operation.to_string());
                }
            }
        }
        for (canonical, aliases) in [
            ("spawn", &["spawn", "spawn_tool"] as &[&str]),
            ("wait", &["wait", "wait_tool"] as &[&str]),
            (
                "dispose",
                &["dispose", "dispose_tool", "close_tool"] as &[&str],
            ),
        ] {
            if operations.contains_key(canonical) {
                continue;
            }
            for alias in aliases {
                if let Some(operation) = source
                    .get(alias)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    operations.insert(canonical.to_string(), operation.to_string());
                    break;
                }
            }
        }
        for required_operation in ["spawn", "wait"] {
            if !operations.contains_key(required_operation) {
                return Err(HostBridgeAdapterContractError::MissingOperation(
                    required_operation,
                ));
            }
        }
        let dispose_policy = source
            .get("dispose_policy")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| operations.get("dispose").map(|_| "configured".to_string()))
            .ok_or(HostBridgeAdapterContractError::MissingDisposePolicy)?;
        if dispose_policy != "configured" && dispose_policy != "unavailable" {
            return Err(HostBridgeAdapterContractError::InvalidField(
                "dispose_policy",
            ));
        }
        if dispose_policy == "configured" && !operations.contains_key("dispose") {
            return Err(HostBridgeAdapterContractError::MissingOperation("dispose"));
        }

        Ok(Self {
            adapter_kind,
            adapter_capability_id,
            invocation_mode,
            dispatch_transport,
            receipt_mode,
            operations,
            dispose_policy,
        })
    }

    pub fn from_request_value(value: &Value) -> Result<Self, HostBridgeAdapterContractError> {
        value
            .get("adapter_operations")
            .or_else(|| value.get("adapter_params"))
            .ok_or(HostBridgeAdapterContractError::MissingField(
                "adapter_operations",
            ))
            .and_then(Self::from_registry_value)
            .and_then(|mut contract| {
                for field in [
                    ("adapter_kind", &contract.adapter_kind),
                    ("adapter_capability_id", &contract.adapter_capability_id),
                    ("invocation_mode", &contract.invocation_mode),
                    ("dispatch_transport", &contract.dispatch_transport),
                ] {
                    if let Some(expected) = value.get(field.0).and_then(Value::as_str) {
                        if expected.trim() != field.1 {
                            return Err(HostBridgeAdapterContractError::InvalidField(field.0));
                        }
                    }
                }
                contract.receipt_mode = value
                    .get("receipt_mode")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .unwrap_or(&contract.receipt_mode)
                    .to_string();
                Ok(contract)
            })
    }

    #[must_use]
    pub fn operation_sequence(&self) -> Vec<String> {
        let mut sequence = vec![
            self.operations["spawn"].clone(),
            self.operations["wait"].clone(),
        ];
        if self.dispose_policy == "configured" {
            if let Some(dispose) = self.operations.get("dispose") {
                sequence.push(dispose.clone());
            }
        }
        sequence
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "adapter_kind": self.adapter_kind,
            "adapter_capability_id": self.adapter_capability_id,
            "invocation_mode": self.invocation_mode,
            "dispatch_transport": self.dispatch_transport,
            "receipt_mode": self.receipt_mode,
            "operations": self.operations,
            "dispose_policy": self.dispose_policy,
        })
    }
}

fn canonical_operation_name(key: &str) -> String {
    match key.trim() {
        "spawn_tool" => "spawn",
        "wait_tool" => "wait",
        "close_tool" | "dispose_tool" => "dispose",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Value {
        serde_json::json!({
            "adapter_kind": "configured_adapter",
            "adapter_capability_id": "configured_capability",
            "invocation_mode": "configured_parent_mode",
            "dispatch_transport": "configured_transport",
            "receipt_mode": "configured_receipt",
            "operations": {
                "spawn": "host.spawn",
                "wait": "host.wait",
                "dispose": "host.dispose"
            },
            "dispose_policy": "configured"
        })
    }

    #[test]
    fn resolves_one_configured_adapter_without_defaults() {
        let contract = HostBridgeAdapterOperations::from_registry_value(&registry()).unwrap();
        assert_eq!(
            contract.operation_sequence(),
            ["host.spawn", "host.wait", "host.dispose"]
        );
        assert_eq!(contract.to_value()["operations"]["spawn"], "host.spawn");
    }

    #[test]
    fn rejects_empty_registry_and_missing_dispose_mapping() {
        assert!(matches!(
            HostBridgeAdapterOperations::from_registry_value(&Value::Null),
            Err(HostBridgeAdapterContractError::MissingField("adapter_kind"))
        ));
        let mut value = registry();
        value["operations"]
            .as_object_mut()
            .unwrap()
            .remove("dispose");
        value["dispose_policy"] = Value::Null;
        assert!(matches!(
            HostBridgeAdapterOperations::from_registry_value(&value),
            Err(HostBridgeAdapterContractError::MissingDisposePolicy)
        ));
    }

    #[test]
    fn translates_legacy_operation_aliases_only_on_registry_read() {
        let value = serde_json::json!({
            "adapter_kind": "legacy_adapter",
            "adapter_capability_id": "legacy_capability",
            "invocation_mode": "legacy_parent",
            "dispatch_transport": "legacy_transport",
            "receipt_mode": "legacy_receipt",
            "spawn_tool": "legacy.spawn",
            "wait_tool": "legacy.wait",
            "close_tool": "legacy.close"
        });
        let contract = HostBridgeAdapterOperations::from_registry_value(&value).unwrap();
        assert_eq!(
            contract.to_value()["operations"],
            serde_json::json!({
                "dispose": "legacy.close",
                "spawn": "legacy.spawn",
                "wait": "legacy.wait"
            })
        );
    }
}
