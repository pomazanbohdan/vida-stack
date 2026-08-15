use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::completion_authority::BLOCKER_ATTEMPT_SCOPE_GUARD;
use crate::errors::HostBridgeError;
use crate::request::HostBridgeRequest;
use runtime_path_policy::atomic_write::write_json_replace;
use runtime_path_policy::bounded_json::{TASK_ATTEMPT_ARTIFACT_LIMIT, read_json_value_file};
use runtime_path_policy::{
    ArtifactPathKind, PathPolicyError, StateRoot, existing_regular_file_under_root,
    new_output_path_under_root,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeImplementationArtifact {
    pub artifact_path: PathBuf,
    pub artifact_kind: String,
    pub changed_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeNormalizedImplementationArtifact {
    pub artifact: serde_json::Value,
    pub artifact_ref: String,
    pub source_artifact_ref: String,
    pub changed_files: Vec<String>,
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

pub fn normalized_record_component(value: &str, fallback: &str) -> String {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let normalized = normalized.trim_matches('-');
    if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized.to_string()
    }
}

pub fn host_bridge_record_component(value: &str) -> String {
    normalized_record_component(value, "host-bridge")
}

pub fn normalized_host_bridge_attempt_id(run_id: &str, value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "{}--implementation--host-bridge-artifact",
                host_bridge_record_component(run_id)
            )
        })
}

pub fn normalized_host_bridge_consolidation_receipt_id(
    attempt_id: &str,
    value: Option<&str>,
) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{attempt_id}--receipt"))
}

pub fn host_bridge_changed_files_from_artifact(
    artifact_json: Option<&serde_json::Value>,
    explicit_changed_files: &[String],
) -> Vec<String> {
    let mut changed_files = explicit_changed_files
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if changed_files.is_empty()
        && let Some(files) = artifact_json
            .and_then(|artifact| artifact.get("changed_files"))
            .and_then(serde_json::Value::as_array)
    {
        changed_files.extend(
            files
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        );
    }
    changed_files.sort();
    changed_files.dedup();
    changed_files
}

pub fn host_bridge_request_implementation_artifacts(
    request: &serde_json::Value,
) -> Vec<serde_json::Value> {
    request
        .get("implementation_artifacts")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn push_unique_host_bridge_implementation_artifact(
    artifacts: &mut Vec<serde_json::Value>,
    artifact: serde_json::Value,
) {
    let source_ref = artifact
        .get("source_artifact_ref")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    let attempt_id = artifact
        .get("attempt_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim);
    if let Some(existing) = artifacts.iter_mut().find(|existing| {
        existing
            .get("source_artifact_ref")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            == source_ref
            && existing
                .get("attempt_id")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                == attempt_id
    }) {
        if *existing != artifact {
            *existing = artifact;
        }
        return;
    }
    artifacts.push(artifact);
}

pub fn host_bridge_normalized_implementation_artifact_path(
    state_root: &Path,
    attempt_id: &str,
    index: usize,
    artifact_kind: &str,
) -> PathBuf {
    state_root
        .join("host-tool-bridge/implementation-artifacts")
        .join(format!(
            "{}-{}-{}.json",
            host_bridge_record_component(attempt_id),
            index,
            host_bridge_record_component(artifact_kind)
        ))
}

pub fn host_bridge_artifact_file(
    state_root: &Path,
    path: &Path,
) -> Result<Option<serde_json::Value>, String> {
    let state_root = StateRoot::open(state_root).map_err(|error| error.to_string())?;
    let artifact =
        existing_regular_file_under_root(&state_root, path, ArtifactPathKind::TaskAttemptArtifact)
            .map_err(|error| host_bridge_policy_error(error, "implementation artifact"))?;
    match read_json_value_file(&artifact, TASK_ATTEMPT_ARTIFACT_LIMIT) {
        Ok(value) => Ok(Some(value)),
        Err(PathPolicyError::Json { .. }) => Ok(None),
        Err(error) => Err(host_bridge_policy_error(error, "implementation artifact")),
    }
}

pub fn write_host_bridge_normalized_implementation_artifact(
    state_root: &Path,
    path: &Path,
    artifact: &serde_json::Value,
) -> Result<(), String> {
    let state_root = StateRoot::open(state_root).map_err(|error| error.to_string())?;
    let output_path = new_output_path_under_root(
        &state_root,
        path,
        ArtifactPathKind::TaskAttemptArtifact,
        true,
    )
    .map_err(|error| error.to_string())?;
    write_json_replace(&output_path, artifact).map_err(|error| error.to_string())
}

pub fn write_host_bridge_request(
    state_root: &Path,
    path: &Path,
    request: &serde_json::Value,
) -> Result<(), String> {
    let state_root = StateRoot::open(state_root).map_err(|error| error.to_string())?;
    let request_file =
        existing_regular_file_under_root(&state_root, path, ArtifactPathKind::HostBridgeRequest)
            .map_err(|error| host_bridge_policy_error(error, "host bridge request"))?;
    let output = new_output_path_under_root(
        &state_root,
        request_file.path(),
        ArtifactPathKind::HostBridgeRequest,
        true,
    )
    .map_err(|error| host_bridge_policy_error(error, "host bridge request"))?;
    write_json_replace(&output, request)
        .map_err(|error| host_bridge_policy_error(error, "host bridge request"))
}

fn host_bridge_policy_error(error: PathPolicyError, label: &str) -> String {
    match error {
        PathPolicyError::DotSegment { path, .. } => {
            format!("{label} path `{}` contains a dot segment", path.display())
        }
        PathPolicyError::Metadata { path, source, .. } => {
            format!("Failed to inspect {label} `{}`: {source}", path.display())
        }
        PathPolicyError::Symlink { path, .. } => {
            format!(
                "{label} `{}` is a symlink; refusing to follow it.",
                path.display()
            )
        }
        PathPolicyError::NotRegularFile { path, .. } => {
            format!("{label} `{}` is not a regular file.", path.display())
        }
        PathPolicyError::Canonicalize { path, source, .. } => {
            format!(
                "Failed to canonicalize {label} `{}`: {source}",
                path.display()
            )
        }
        PathPolicyError::OutsideStateRoot { path, root, .. } => format!(
            "{label} `{}` is outside VIDA state root `{}`.",
            path.display(),
            root.display()
        ),
        PathPolicyError::TooLarge {
            path, max_bytes, ..
        } => format!("{label} `{}` exceeds {max_bytes} bytes.", path.display()),
        PathPolicyError::Json { path, source, .. } => {
            format!(
                "Failed to encode {label} `{}` as JSON: {source}",
                path.display()
            )
        }
        PathPolicyError::Read { path, source, .. } => {
            format!("Failed to read {label} `{}`: {source}", path.display())
        }
        PathPolicyError::Write { path, source, .. } => {
            format!("Failed to write {label} `{}`: {source}", path.display())
        }
        other => other.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_host_bridge_normalized_implementation_artifact(
    artifact_kind: &str,
    attempt_id: &str,
    task_id: &str,
    task_freshness: &str,
    consolidation_receipt_id: &str,
    artifact_path: &Path,
    artifact_json: Option<&serde_json::Value>,
    changed_files: Vec<String>,
    state_root: &Path,
    index: usize,
) -> HostBridgeNormalizedImplementationArtifact {
    let artifact_ref = artifact_path.display().to_string();
    let artifact = serde_json::json!({
        "artifact_kind": artifact_kind,
        "schema_version": "host-bridge-implementation-artifact-v1",
        "attempt_id": attempt_id,
        "task_id": task_id,
        "stage_id": "implementation",
        "freshness": task_freshness,
        "consolidation_receipt_id": consolidation_receipt_id,
        "changed_files": changed_files,
        "source_artifact_ref": artifact_ref,
        "source_artifact_kind": artifact_json
            .and_then(|artifact| artifact.get("artifact_kind"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(artifact_kind),
        "receipt_backed": true
    });
    HostBridgeNormalizedImplementationArtifact {
        artifact,
        artifact_ref: host_bridge_normalized_implementation_artifact_path(
            state_root,
            attempt_id,
            index,
            artifact_kind,
        )
        .display()
        .to_string(),
        source_artifact_ref: artifact_ref,
        changed_files,
    }
}

pub fn validate_implementation_artifact_scope(
    changed_files: &[PathBuf],
    owned_paths: &[PathBuf],
) -> ImplementationArtifactScopeDecision {
    validate_implementation_artifact_scope_with_proof_paths(changed_files, owned_paths, &[])
}

pub fn validate_implementation_artifact_scope_with_proof_paths(
    changed_files: &[PathBuf],
    owned_paths: &[PathBuf],
    proof_artifact_paths: &[PathBuf],
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

    let safe_proof_artifact_paths = proof_artifact_paths
        .iter()
        .filter(|path| proof_artifact_path_is_safe(path))
        .collect::<Vec<_>>();
    let effective_scope = owned_paths
        .iter()
        .chain(safe_proof_artifact_paths.iter().copied())
        .collect::<Vec<_>>();
    let out_of_scope_paths = changed_files
        .iter()
        .filter(|path| {
            !effective_scope
                .iter()
                .any(|owned| path_is_within(path, owned))
        })
        .cloned()
        .collect::<Vec<_>>();

    if out_of_scope_paths.is_empty() {
        ImplementationArtifactScopeDecision {
            accepted: true,
            blocker_codes: Vec::new(),
            out_of_scope_paths,
        }
    } else {
        let blocker_code = if proof_artifact_paths.is_empty()
            && out_of_scope_paths
                .iter()
                .any(|path| path_looks_like_proof_or_test(path))
        {
            "missing_proof_artifact_scope"
        } else {
            "implementation_artifact_out_of_scope"
        };
        ImplementationArtifactScopeDecision {
            accepted: false,
            blocker_codes: vec![blocker_code.to_string()],
            out_of_scope_paths,
        }
    }
}

pub fn host_bridge_implementation_attempt_scope_decision(
    request: &serde_json::Value,
    artifacts: &serde_json::Value,
) -> ImplementationArtifactScopeDecision {
    let admission = crate::completion_authority::admit_host_bridge_implementation_attempt(
        request,
        Some(artifacts),
    );
    let owned_paths = request
        .get("implementation_isolation")
        .and_then(serde_json::Value::as_object)
        .map(|isolation| {
            isolation
                .get("owned_paths")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let changed_files = artifacts
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|artifact| {
            artifact
                .get("changed_files")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(PathBuf::from)
        })
        .collect::<Vec<_>>();
    let out_of_scope_paths = if admission
        .blocker_codes
        .iter()
        .any(|code| code == BLOCKER_ATTEMPT_SCOPE_GUARD)
    {
        changed_files
            .iter()
            .filter(|path| {
                !owned_paths
                    .iter()
                    .any(|owned| *path == owned || path.starts_with(owned))
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    ImplementationArtifactScopeDecision {
        accepted: admission.blocker_codes.is_empty(),
        blocker_codes: admission.blocker_codes,
        out_of_scope_paths,
    }
}

fn path_is_within(path: &Path, owned: &Path) -> bool {
    path == owned || path.starts_with(owned)
}

fn proof_artifact_path_is_safe(path: &Path) -> bool {
    let normalized = path.display().to_string().replace('\\', "/");
    if normalized.is_empty()
        || normalized == "."
        || normalized == ".."
        || normalized == "/"
        || normalized.starts_with('/')
        || normalized.starts_with(".vida/")
        || normalized == ".vida"
        || normalized.starts_with("~/")
        || normalized.starts_with("//")
        || normalized.ends_with('/')
        || !normalized.contains('/')
    {
        return false;
    }
    if normalized.len() >= 2
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        return false;
    }
    let components = normalized.split('/').collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| component.is_empty() || *component == "." || *component == "..")
    {
        return false;
    }
    proof_artifact_components_have_proof_context(&components)
}

fn proof_artifact_components_have_proof_context(components: &[&str]) -> bool {
    components.iter().any(|component| {
        matches!(
            *component,
            "test" | "tests" | "__tests__" | "spec" | "specs" | "proof" | "proofs"
        ) || component.ends_with("_test.rs")
            || component.ends_with("_test.dart")
            || component.ends_with(".test.ts")
            || component.ends_with(".test.tsx")
            || component.ends_with(".spec.ts")
            || component.ends_with(".spec.tsx")
    })
}

fn path_looks_like_proof_or_test(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            "test" | "tests" | "__tests__" | "spec" | "specs" | "proof" | "proofs"
        ) || value.ends_with("_test.rs")
            || value.ends_with("_test.dart")
            || value.ends_with(".test.ts")
            || value.ends_with(".test.tsx")
            || value.ends_with(".spec.ts")
            || value.ends_with(".spec.tsx")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_component_normalizes_non_path_safe_characters() {
        assert_eq!(
            host_bridge_record_component("run:1/impl review"),
            "run-1-impl-review"
        );
        assert_eq!(host_bridge_record_component("!!!"), "host-bridge");
        assert_eq!(normalized_record_component("!!!", "attempt"), "attempt");
    }

    #[test]
    fn changed_files_prefer_explicit_then_fallback_to_artifact_json() {
        let artifact = serde_json::json!({
            "changed_files": ["crates/vida/src/a.rs", "crates/vida/src/a.rs"]
        });

        assert_eq!(
            host_bridge_changed_files_from_artifact(Some(&artifact), &[]),
            vec!["crates/vida/src/a.rs"]
        );
        assert_eq!(
            host_bridge_changed_files_from_artifact(
                Some(&artifact),
                &["crates/vida/src/b.rs".to_string()]
            ),
            vec!["crates/vida/src/b.rs"]
        );
    }

    #[test]
    fn duplicate_artifacts_are_keyed_by_source_ref_and_attempt() {
        let mut artifacts = vec![serde_json::json!({
            "source_artifact_ref": "artifact.json",
            "attempt_id": "attempt-1"
        })];

        push_unique_host_bridge_implementation_artifact(
            &mut artifacts,
            serde_json::json!({
                "source_artifact_ref": "artifact.json",
                "attempt_id": "attempt-1"
            }),
        );
        push_unique_host_bridge_implementation_artifact(
            &mut artifacts,
            serde_json::json!({
                "source_artifact_ref": "artifact.json",
                "attempt_id": "attempt-2"
            }),
        );

        assert_eq!(artifacts.len(), 2);
    }

    #[test]
    fn same_key_artifact_refreshes_in_place_without_reordering_history() {
        let historical = serde_json::json!({
            "source_artifact_ref": "historical.json",
            "attempt_id": "attempt-0",
            "canonical_worktree_unchanged": true
        });
        let mut artifacts = vec![
            historical.clone(),
            serde_json::json!({
                "source_artifact_ref": "manifest.json",
                "attempt_id": "attempt-1",
                "receipt_backed": true
            }),
            serde_json::json!({
                "source_artifact_ref": "manifest.json",
                "attempt_id": "attempt-2",
                "receipt_backed": true
            }),
        ];
        let refreshed = serde_json::json!({
            "source_artifact_ref": "manifest.json",
            "attempt_id": "attempt-1",
            "receipt_backed": true,
            "canonical_worktree_unchanged": true,
            "canonical_worktree_touched": false,
            "line_ending_churn": false
        });

        push_unique_host_bridge_implementation_artifact(&mut artifacts, refreshed.clone());
        push_unique_host_bridge_implementation_artifact(&mut artifacts, refreshed.clone());

        assert_eq!(artifacts.len(), 3);
        assert_eq!(artifacts[0], historical);
        assert_eq!(artifacts[1], refreshed);
        assert_eq!(artifacts[2]["attempt_id"], "attempt-2");
    }

    #[test]
    fn normalized_artifact_payload_preserves_receipt_fields() {
        let artifact = build_host_bridge_normalized_implementation_artifact(
            "patch_proposal",
            "attempt-1",
            "task-1",
            "fresh-1",
            "receipt-1",
            Path::new("source.json"),
            Some(&serde_json::json!({ "artifact_kind": "draft_patch" })),
            vec!["crates/vida/src/a.rs".to_string()],
            Path::new(".vida/data/state"),
            3,
        );

        assert_eq!(artifact.source_artifact_ref, "source.json");
        assert_eq!(artifact.artifact["artifact_kind"], "patch_proposal");
        assert_eq!(artifact.artifact["attempt_id"], "attempt-1");
        assert_eq!(artifact.artifact["task_id"], "task-1");
        assert_eq!(artifact.artifact["freshness"], "fresh-1");
        assert_eq!(artifact.artifact["consolidation_receipt_id"], "receipt-1");
        assert_eq!(
            artifact.artifact["changed_files"],
            serde_json::json!(["crates/vida/src/a.rs"])
        );
        assert_eq!(artifact.changed_files, vec!["crates/vida/src/a.rs"]);
        assert_eq!(
            artifact.artifact["source_artifact_kind"],
            serde_json::json!("draft_patch")
        );
        assert_eq!(
            artifact.artifact_ref.replace('\\', "/"),
            ".vida/data/state/host-tool-bridge/implementation-artifacts/attempt-1-3-patch_proposal.json"
        );
    }

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
    fn artifact_scope_rejects_empty_owned_paths_and_reports_changed_files() {
        let changed_files = vec![PathBuf::from("crates/taskflow-host-bridge/src/lib.rs")];
        let decision = validate_implementation_artifact_scope(&changed_files, &[]);

        assert!(!decision.accepted);
        assert_eq!(
            decision.blocker_codes,
            vec!["implementation_artifact_has_no_owned_paths"]
        );
        assert_eq!(decision.out_of_scope_paths, changed_files);
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

    #[test]
    fn artifact_scope_rejects_unsafe_proof_artifact_paths() {
        let decision = validate_implementation_artifact_scope_with_proof_paths(
            &[PathBuf::from(
                "crates/vida/src/runtime_dispatch_execution.rs",
            )],
            &[PathBuf::from("crates/taskflow-host-bridge")],
            &[
                PathBuf::from("../.."),
                PathBuf::from("/"),
                PathBuf::from("/etc/passwd"),
                PathBuf::from(".vida/data/state"),
                PathBuf::from("C:/Windows"),
                PathBuf::from("crates/vida"),
            ],
        );

        assert!(!decision.accepted);
        assert_eq!(
            decision.blocker_codes,
            vec!["implementation_artifact_out_of_scope"]
        );
    }

    #[test]
    fn artifact_scope_accepts_proof_artifact_paths_without_role_names() {
        let decision = validate_implementation_artifact_scope_with_proof_paths(
            &[
                PathBuf::from("src/lib/features/list_view/domain/model.dart"),
                PathBuf::from("src/test/features/list_view/domain/model_test.dart"),
            ],
            &[PathBuf::from("src/lib/features/list_view")],
            &[PathBuf::from("src/test/features/list_view")],
        );

        assert!(decision.accepted);
        assert!(decision.blocker_codes.is_empty());
        assert!(decision.out_of_scope_paths.is_empty());
    }

    #[test]
    fn artifact_scope_reports_missing_proof_scope_for_test_paths() {
        let decision = validate_implementation_artifact_scope_with_proof_paths(
            &[PathBuf::from(
                "src/test/features/list_view/domain/model_test.dart",
            )],
            &[PathBuf::from("src/lib/features/list_view")],
            &[],
        );

        assert!(!decision.accepted);
        assert_eq!(decision.blocker_codes, vec!["missing_proof_artifact_scope"]);
        assert_eq!(
            decision.out_of_scope_paths,
            vec![PathBuf::from(
                "src/test/features/list_view/domain/model_test.dart"
            )]
        );
    }
}
