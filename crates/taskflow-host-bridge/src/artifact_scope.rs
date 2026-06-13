use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::HostBridgeError;
use crate::request::HostBridgeRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeImplementationArtifact {
    pub artifact_path: PathBuf,
    pub artifact_kind: String,
    pub changed_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementationArtifactScopeDecision {
    pub accepted: bool,
    pub blocker_codes: Vec<String>,
    pub out_of_scope_paths: Vec<PathBuf>,
}

pub fn attach_host_bridge_implementation_artifact(
    request: &HostBridgeRequest,
    artifact_path: impl Into<PathBuf>,
) -> Result<HostBridgeImplementationArtifact, HostBridgeError> {
    let artifact_path = artifact_path.into();
    let decision = validate_implementation_artifact_scope(
        std::slice::from_ref(&artifact_path),
        &request.owned_paths,
    );
    if !decision.accepted {
        return Err(HostBridgeError::ArtifactScope {
            path: decision
                .out_of_scope_paths
                .first()
                .cloned()
                .unwrap_or(artifact_path),
        });
    }

    Ok(HostBridgeImplementationArtifact {
        artifact_path,
        artifact_kind: "patch_proposal".to_string(),
        changed_files: Vec::new(),
    })
}

pub fn validate_implementation_artifact_scope(
    changed_files: &[PathBuf],
    owned_paths: &[PathBuf],
) -> ImplementationArtifactScopeDecision {
    if changed_files.is_empty() {
        return ImplementationArtifactScopeDecision {
            accepted: false,
            blocker_codes: vec!["implementation_artifact_has_no_changed_files".to_string()],
            out_of_scope_paths: Vec::new(),
        };
    }
    if owned_paths.is_empty() {
        return ImplementationArtifactScopeDecision {
            accepted: false,
            blocker_codes: vec!["implementation_artifact_has_no_owned_paths".to_string()],
            out_of_scope_paths: changed_files.to_vec(),
        };
    }

    let out_of_scope_paths = changed_files
        .iter()
        .filter(|path| !owned_paths.iter().any(|owned| path_is_within(path, owned)))
        .cloned()
        .collect::<Vec<_>>();

    if out_of_scope_paths.is_empty() {
        ImplementationArtifactScopeDecision {
            accepted: true,
            blocker_codes: Vec::new(),
            out_of_scope_paths,
        }
    } else {
        ImplementationArtifactScopeDecision {
            accepted: false,
            blocker_codes: vec!["implementation_artifact_out_of_scope".to_string()],
            out_of_scope_paths,
        }
    }
}

fn path_is_within(path: &Path, owned: &Path) -> bool {
    path == owned || path.starts_with(owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_scope_rejects_empty_changed_files() {
        let decision = validate_implementation_artifact_scope(
            &[],
            &[PathBuf::from("crates/taskflow-host-bridge")],
        );

        assert!(!decision.accepted);
        assert_eq!(
            decision.blocker_codes,
            vec!["implementation_artifact_has_no_changed_files"]
        );
    }

    #[test]
    fn artifact_scope_rejects_paths_outside_owned_paths() {
        let decision = validate_implementation_artifact_scope(
            &[PathBuf::from("crates/vida/src/agent_dispatch_surface.rs")],
            &[PathBuf::from("crates/taskflow-host-bridge")],
        );

        assert!(!decision.accepted);
        assert_eq!(
            decision.blocker_codes,
            vec!["implementation_artifact_out_of_scope"]
        );
    }
}
