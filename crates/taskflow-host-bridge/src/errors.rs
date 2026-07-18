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
