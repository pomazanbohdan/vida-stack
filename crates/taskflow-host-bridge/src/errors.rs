use std::path::PathBuf;

use runtime_path_policy::PathPolicyError;
use thiserror::Error;

use crate::adapter_contract::HostBridgeAdapterContractError;

#[derive(Debug, Error)]
pub enum HostBridgeError {
    #[error(transparent)]
    PathPolicy(#[from] PathPolicyError),

    #[error("host bridge request `{path}` could not be read: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("host bridge request `{path}` contains invalid json: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "host bridge request `{path}` is exceeding maximum size {max_bytes} bytes and exceeds the allowed limit"
    )]
    Oversized { path: PathBuf, max_bytes: u64 },

    #[error("host bridge request is missing required field `{field}`")]
    MissingRequiredField { field: &'static str },

    #[error("host bridge request has invalid required identity field `{field}`")]
    InvalidRequiredField { field: &'static str },

    #[error("host bridge request adapter contract is invalid: {0}")]
    AdapterContract(#[source] HostBridgeAdapterContractError),

    #[error("implementation artifact `{path}` is outside the declared host bridge scope")]
    ArtifactScope { path: PathBuf },
}

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use super::HostBridgeError;
    use crate::adapter_contract::HostBridgeAdapterContractError;

    #[test]
    fn host_bridge_error_display_preserves_variant_context() {
        let cases = [
            (
                HostBridgeError::MissingRequiredField { field: "run_id" },
                "host bridge request is missing required field `run_id`",
            ),
            (
                HostBridgeError::InvalidRequiredField {
                    field: "request_id",
                },
                "host bridge request has invalid required identity field `request_id`",
            ),
            (
                HostBridgeError::Oversized {
                    path: PathBuf::from("request.json"),
                    max_bytes: 4096,
                },
                "host bridge request `request.json` is exceeding maximum size 4096 bytes and exceeds the allowed limit",
            ),
            (
                HostBridgeError::ArtifactScope {
                    path: PathBuf::from("outside/result.json"),
                },
                "implementation artifact `outside/result.json` is outside the declared host bridge scope",
            ),
            (
                HostBridgeError::AdapterContract(HostBridgeAdapterContractError::MissingField(
                    "operations",
                )),
                "host bridge request adapter contract is invalid: host bridge adapter registry missing `operations`",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn host_bridge_error_nested_failures_expose_source_chain() {
        let read = HostBridgeError::Read {
            path: PathBuf::from("request.json"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(read.source().is_some());

        let json_source = serde_json::from_str::<serde_json::Value>("{")
            .expect_err("malformed JSON should produce a source error");
        let json = HostBridgeError::Json {
            path: PathBuf::from("request.json"),
            source: json_source,
        };
        assert!(json.source().is_some());

        let adapter =
            HostBridgeError::AdapterContract(HostBridgeAdapterContractError::MissingDisposePolicy);
        assert!(adapter.source().is_some());
    }
}
