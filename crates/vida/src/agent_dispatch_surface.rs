use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use crate::dev_team_sequence_contract::{
    configured_dev_team_first_step_for_task, dev_team_sequence, dev_team_sequence_for_task,
    dev_team_sequence_for_work_item, selected_dev_team_flow_for_task, task_flow_lookup_keys,
    DevTeamSequenceStep,
};
use crate::launcher_activation_snapshot::capture_launcher_activation_snapshot_for_root;
use crate::runtime_proof_scope::{
    collect_test_like_paths_from_text, collect_test_like_paths_from_values,
    path_to_proof_scope_string, proof_intent_text, proof_scope_from_dispatch_packet_path,
    ProofArtifactScope,
};
use crate::{
    state_store, state_store::StateStore, AgentArgs, AgentCommand, AgentDispatchNextArgs,
    AgentHostBridgeArgs, AgentSelectArgs, AgentStatusArgs,
};
use operator_output::command_text::human_command;
use runtime_path_policy::{
    existing_regular_file_under_root, new_output_path_under_root, path_contains_dot_segment,
    ArtifactPathKind, PathPolicyError, StateRoot,
};
use taskflow_host_bridge::{
    build_host_bridge_adapter_payload, build_host_bridge_normalized_implementation_artifact,
    decide_host_bridge_completion_authority, host_bridge_artifact_file,
    host_bridge_artifact_has_retryable_completion_blocker, host_bridge_blocked_result_contract,
    host_bridge_blocked_result_contract_is_retryable, host_bridge_changed_files_from_artifact,
    host_bridge_completed_artifact_status_is_admissible,
    host_bridge_completed_result_has_preview_refresh_evidence,
    host_bridge_completed_result_status_is_admissible, host_bridge_completion_retryable_blocker,
    host_bridge_normalized_implementation_artifact_path, host_bridge_operator_fields,
    host_bridge_provenance_public_blocker_code, host_bridge_request_implementation_artifacts,
    host_bridge_request_owned_paths, host_bridge_request_proof_artifact_paths,
    host_bridge_request_requires_implementation_artifacts, host_bridge_request_string,
    normalize_host_bridge_provenance_for_completion, normalized_host_bridge_attempt_id,
    normalized_host_bridge_consolidation_receipt_id,
    push_unique_host_bridge_implementation_artifact,
    read_host_bridge_request as read_typed_host_bridge_request, validate_dispatch_receipt_binding,
    validate_host_bridge_request_provenance,
    validate_implementation_artifact_scope_with_proof_paths,
    write_host_bridge_normalized_implementation_artifact, write_host_bridge_request,
    DispatchReceiptBindingInput, HostBridgeAdapterPayloadInput, HostBridgeCompletionAuthorityInput,
    HostBridgeProvenanceInput, HostBridgeRequest, HostBridgeRequestPath,
};

const AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE: std::time::Duration =
    std::time::Duration::from_secs(300);
const HOST_BRIDGE_PROVENANCE_LOCK_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(250);

fn blocker_code_value(code: taskflow_contracts::BlockerCode) -> String {
    code.as_str().to_string()
}

fn release1_contract_status_value(ok: bool) -> &'static str {
    taskflow_contracts::release1_contract_status_str(ok)
}

fn release1_pass_status() -> &'static str {
    taskflow_contracts::Release1ContractStatus::Pass.as_str()
}

fn release1_blocked_status() -> &'static str {
    taskflow_contracts::Release1ContractStatus::Blocked.as_str()
}

fn agent_dispatch_assignment_blockers(selected_lanes: &[AgentDispatchLanePreview]) -> Vec<String> {
    selected_lanes
        .iter()
        .flat_map(|lane| selection_truth_guard_blockers(&lane.selection_truth))
        .collect()
}

fn agent_dispatch_contract_status(
    blocker_codes: &[String],
    assignment_blockers: &[String],
) -> &'static str {
    release1_contract_status_value(blocker_codes.is_empty() && assignment_blockers.is_empty())
}

fn agent_dispatch_status_from_blockers(blocker_codes: &[String]) -> &'static str {
    agent_dispatch_contract_status(blocker_codes, &[])
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLaneSelectionTruth {
    selected_carrier: String,
    selected_backend: String,
    selected_model_profile: String,
    selected_model_ref: String,
    selected_reasoning_effort: String,
    rate: u64,
    estimated_task_price_units: u64,
    budget_verdict: String,
    selected_over_budget: bool,
    selected_model_profile_readiness_status: String,
    pricing_freshness_status: String,
    selected_external_backend_readiness_status: String,
    selection_source_paths: serde_json::Value,
    pricing_readiness: serde_json::Value,
    runtime_role: String,
    task_class: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchLanePreview {
    lane_index: usize,
    task_id: String,
    title: String,
    role_label: String,
    runtime_role: String,
    task_class: String,
    dispatch_command: String,
    dispatch_command_kind: String,
    receipt_backed_execution_command: String,
    ready_parallel_safe: bool,
    selection_reason: String,
    selection_truth: AgentDispatchLaneSelectionTruth,
    requires_user_approval: bool,
    approval_gate: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchBlockedCandidate {
    task_id: String,
    title: String,
    ready_now: bool,
    ready_parallel_safe: bool,
    reasons: Vec<String>,
    parallel_blockers: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentDispatchNextPreview {
    status: String,
    mode: String,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    effective_max_parallel_agents: usize,
    lanes_selected: usize,
    selected_lanes: Vec<AgentDispatchLanePreview>,
    blocked_candidates: Vec<AgentDispatchBlockedCandidate>,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    execute_supported: bool,
    execution_attempted: bool,
    parallelization_planner: serde_json::Value,
    packet_materialization: serde_json::Value,
    carrier_selection_api: serde_json::Value,
    fanout_guard: serde_json::Value,
    flow_projection: serde_json::Value,
    source_surfaces: Vec<String>,
}

fn agent_dispatch_source_surfaces() -> Vec<String> {
    vec![
        "vida agent dispatch-next".to_string(),
        "StateStore::scheduling_projection_scoped".to_string(),
        "vida taskflow graph-summary".to_string(),
        "vida taskflow scheduler dispatch".to_string(),
        "vida agent select --runtime-role <role> --task-class <class>".to_string(),
        "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            .to_string(),
        "vida agent-init --role worker <task-id>".to_string(),
        "vida agent-init --role <runtime-role> <task-id>".to_string(),
    ]
}

fn read_host_bridge_request(
    path: &Path,
    state_root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let inferred_state_root = state_root
        .map(Path::to_path_buf)
        .or_else(|| infer_host_bridge_state_root_from_request_path(path));
    if let Some(state_root) = inferred_state_root {
        let canonical_path =
            canonical_state_artifact_path(&state_root, &path.display().to_string(), true)?;
        return read_typed_host_bridge_request(&HostBridgeRequestPath::new(
            state_root,
            canonical_path,
        ))
        .map(|request| request.raw)
        .map_err(|error| error.to_string());
    }
    read_canonical_host_bridge_json_artifact(path, "host bridge request")
}

fn canonical_host_bridge_request_path(
    path: &Path,
    state_root: Option<&Path>,
) -> Result<PathBuf, String> {
    let inferred_state_root = state_root
        .map(Path::to_path_buf)
        .or_else(|| infer_host_bridge_state_root_from_request_path(path));
    if let Some(state_root) = inferred_state_root {
        return canonical_state_artifact_path(&state_root, &path.display().to_string(), true);
    }
    Ok(path.to_path_buf())
}

fn host_bridge_task_or_request_owned_paths(
    task: &crate::state_store::TaskRecord,
    request: &serde_json::Value,
) -> Vec<PathBuf> {
    let task_owned_paths = task
        .planner_metadata
        .owned_paths
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if task_owned_paths.is_empty() {
        host_bridge_request_owned_paths(request)
    } else {
        task_owned_paths
    }
}

fn proof_artifact_scope_from_task_or_request(
    task: &crate::state_store::TaskRecord,
    request: &serde_json::Value,
) -> ProofArtifactScope {
    let mut scope = ProofArtifactScope {
        paths: host_bridge_request_proof_artifact_paths(request)
            .iter()
            .map(|path| path_to_proof_scope_string(path))
            .collect(),
        proof_intent_present: !host_bridge_request_proof_artifact_paths(request).is_empty(),
    };
    for target in &task.planner_metadata.proof_targets {
        scope.proof_intent_present |= proof_intent_text(target);
        collect_test_like_paths_from_text(&mut scope.paths, target);
    }
    scope.paths.sort();
    scope.paths.dedup();
    scope
}

fn proof_artifact_scope_from_task_request(
    store: &crate::state_store::StateStore,
    task: &crate::state_store::TaskRecord,
    request: &serde_json::Value,
) -> ProofArtifactScope {
    let mut scope = proof_artifact_scope_from_task_or_request(task, request);
    scope.merge(proof_artifact_scope_from_request_packet(
        store.root(),
        request,
    ));
    scope.paths.sort();
    scope.paths.dedup();
    scope
}

fn proof_artifact_scope_from_request_packet(
    state_root: &Path,
    request: &serde_json::Value,
) -> ProofArtifactScope {
    let Some(packet_path) = host_bridge_request_string(request, "packet_path") else {
        return ProofArtifactScope::default();
    };
    let packet_path = canonical_state_artifact_path(state_root, packet_path, true)
        .unwrap_or_else(|_| PathBuf::from(packet_path));
    proof_scope_from_dispatch_packet_path(&packet_path.display().to_string())
}

fn refresh_host_bridge_request_proof_artifact_paths(
    request: &mut serde_json::Value,
    proof_artifact_paths: &[PathBuf],
) {
    if proof_artifact_paths.is_empty() {
        return;
    }
    let mut proof_paths = host_bridge_request_proof_artifact_paths(request)
        .iter()
        .map(|path| path.display().to_string().replace('\\', "/"))
        .collect::<Vec<_>>();
    proof_paths.extend(
        proof_artifact_paths
            .iter()
            .map(|path| path.display().to_string().replace('\\', "/")),
    );
    proof_paths.sort();
    proof_paths.dedup();
    if proof_paths.is_empty() {
        return;
    }
    let proof_paths = proof_paths.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(object) = request.as_object_mut() {
        object.insert(
            "proof_artifact_paths".to_string(),
            serde_json::json!(proof_paths),
        );
        object.insert(
            "proof_artifact_scope".to_string(),
            serde_json::json!(proof_paths),
        );
        if let Some(implementation_isolation) = object
            .get_mut("implementation_isolation")
            .and_then(serde_json::Value::as_object_mut)
        {
            implementation_isolation.insert(
                "proof_artifact_paths".to_string(),
                serde_json::json!(proof_paths),
            );
            implementation_isolation.insert(
                "proof_artifact_scope".to_string(),
                serde_json::json!(proof_paths),
            );
        }
    }
}

const MAX_HOST_BRIDGE_ARTIFACT_BYTES: u64 = 1024 * 1024;

fn read_canonical_host_bridge_json_artifact(
    path: &Path,
    label: &str,
) -> Result<serde_json::Value, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("Failed to inspect {label} `{}`: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Host bridge artifact `{}` is a symlink; refusing to follow it.",
            path.display()
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "Host bridge artifact `{}` is not a regular file.",
            path.display()
        ));
    }
    if metadata.len() > MAX_HOST_BRIDGE_ARTIFACT_BYTES {
        return Err(format!(
            "Host bridge artifact `{}` is {} bytes, exceeding the {} byte intake cap.",
            path.display(),
            metadata.len(),
            MAX_HOST_BRIDGE_ARTIFACT_BYTES
        ));
    }
    let file = std::fs::File::open(path)
        .map_err(|error| format!("Failed to open {label} `{}`: {error}", path.display()))?;
    let mut raw = String::new();
    let mut limited = std::io::Read::take(file, MAX_HOST_BRIDGE_ARTIFACT_BYTES + 1);
    std::io::Read::read_to_string(&mut limited, &mut raw)
        .map_err(|error| format!("Failed to read {label} `{}`: {error}", path.display()))?;
    serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to decode {label} `{}` as JSON: {error}",
            path.display()
        )
    })
}

fn canonical_state_artifact_path(
    state_root: &Path,
    raw_path: &str,
    require_existing_file: bool,
) -> Result<std::path::PathBuf, String> {
    let path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(raw_path);
    if path_contains_dot_segment(&path) {
        return Err(format!(
            "Host bridge artifact path `{}` contains inadmissible dot-segment traversal.",
            path.display()
        ));
    }
    let canonical_state_root = std::fs::canonicalize(state_root).map_err(|error| {
        format!(
            "Failed to canonicalize VIDA state root `{}`: {error}",
            state_root.display()
        )
    })?;
    if require_existing_file {
        let state_root = StateRoot::open(state_root).map_err(|error| error.to_string())?;
        existing_regular_file_under_root(&state_root, &path, ArtifactPathKind::HostBridgeRequest)
            .map(|file| file.path().to_path_buf())
            .map_err(|error| match error {
                runtime_path_policy::PathPolicyError::OutsideStateRoot { path, root, .. } => {
                    format!(
                        "Host bridge artifact `{}` escapes VIDA state root `{}`.",
                        path.display(),
                        root.display()
                    )
                }
                other => other.to_string(),
            })
    } else {
        let parent = path.parent().ok_or_else(|| {
            format!(
                "Host bridge artifact path `{}` has no parent directory.",
                path.display()
            )
        })?;
        let canonical_parent = std::fs::canonicalize(parent).map_err(|error| {
            format!(
                "Failed to canonicalize host bridge artifact directory `{}`: {error}",
                parent.display()
            )
        })?;
        if !canonical_parent.starts_with(&canonical_state_root) {
            return Err(format!(
                "Host bridge artifact path `{}` escapes VIDA state root `{}`.",
                path.display(),
                canonical_state_root.display()
            ));
        }
        Ok(path)
    }
}

async fn host_bridge_request_provenance_blockers(
    request_path: &Path,
    request: &serde_json::Value,
    state_root: Option<&Path>,
    _retry_completion_override: bool,
) -> Vec<String> {
    let state_root = match state_root {
        Some(provided) => provided.to_path_buf(),
        None => infer_host_bridge_state_root_from_request_path(request_path)
            .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir),
    };
    host_bridge_request_provenance_blockers_for_state_root(
        &state_root,
        request_path,
        request,
        _retry_completion_override,
    )
    .await
}

fn infer_host_bridge_state_root_from_request_path(
    request_path: &Path,
) -> Option<std::path::PathBuf> {
    let request_path = std::fs::canonicalize(request_path).ok()?;
    for ancestor in request_path.ancestors() {
        let Some(state_name) = ancestor.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(data_dir) = ancestor.parent() else {
            continue;
        };
        let Some(data_name) = data_dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(vida_dir) = data_dir.parent() else {
            continue;
        };
        let Some(vida_name) = vida_dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if state_name == "state" && data_name == "data" && vida_name == ".vida" {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn host_bridge_observability_project_root(
    state_root: Option<&Path>,
    request_path: &Path,
) -> Option<PathBuf> {
    let inferred_state_root = state_root
        .map(|root| {
            if root.is_absolute() {
                root.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| crate::repo_runtime_root())
                    .join(root)
            }
        })
        .or_else(|| infer_host_bridge_state_root_from_request_path(request_path));
    match inferred_state_root {
        Some(state_root) => {
            crate::taskflow_task_bridge::infer_project_root_from_state_root(&state_root)
                .filter(|root| crate::looks_like_project_root(root))
        }
        None => crate::resolve_runtime_project_root().ok(),
    }
}

fn host_bridge_request_path_is_under_state_root(request_path: &Path, state_root: &Path) -> bool {
    let Ok(request_path) = std::fs::canonicalize(request_path) else {
        return false;
    };
    let Ok(state_root) = std::fs::canonicalize(state_root) else {
        return false;
    };
    request_path.starts_with(state_root)
}

async fn host_bridge_request_provenance_blockers_for_state_root(
    state_root: &Path,
    request_path: &Path,
    request: &serde_json::Value,
    retry_completion_override: bool,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if std::fs::canonicalize(&state_root).is_err() {
        blockers.push(blocker_code_value(
            taskflow_contracts::BlockerCode::HostBridgeStateRootMissing,
        ));
        return blockers;
    };
    let canonical_request_path =
        match canonical_state_artifact_path(&state_root, &request_path.display().to_string(), true)
        {
            Ok(path) => path,
            Err(_) => {
                blockers.push(blocker_code_value(
                    taskflow_contracts::BlockerCode::HostBridgeRequestUntrustedPath,
                ));
                return blockers;
            }
        };
    let declared_request_path = match host_bridge_request_string(request, "request_path") {
        Some(path) => path,
        None => {
            blockers.push(blocker_code_value(
                taskflow_contracts::BlockerCode::HostBridgeRequestPathMissing,
            ));
            return blockers;
        }
    };
    match canonical_state_artifact_path(&state_root, declared_request_path, true) {
        Ok(path) if path == canonical_request_path => {}
        _ => blockers.push(blocker_code_value(
            taskflow_contracts::BlockerCode::HostBridgeRequestPathMismatch,
        )),
    }
    let packet_path = host_bridge_request_string(request, "packet_path");
    let canonical_packet_path =
        packet_path.and_then(
            |path| match canonical_state_artifact_path(&state_root, path, true) {
                Ok(path) => Some(path),
                Err(_) => {
                    blockers.push(blocker_code_value(
                        taskflow_contracts::BlockerCode::HostBridgePacketPathUnbounded,
                    ));
                    None
                }
            },
        );
    for (field, code) in [
        (
            "result_path",
            taskflow_contracts::BlockerCode::HostBridgeResultPathUnbounded,
        ),
        (
            "receipt_path",
            taskflow_contracts::BlockerCode::HostBridgeReceiptPathUnbounded,
        ),
    ] {
        if let Some(path) = host_bridge_request_string(request, field) {
            if canonical_state_artifact_path(&state_root, path, false).is_err() {
                blockers.push(blocker_code_value(code));
            }
        }
    }
    let Some(run_id) = host_bridge_request_string(request, "run_id") else {
        return blockers;
    };
    let receipt_backed_retry_completion =
        host_bridge_request_has_retryable_dispatch_receipt_for_state_root(
            state_root,
            request,
            canonical_packet_path.as_deref(),
        )
        .await;
    if let Ok(typed_request) = HostBridgeRequest::from_value(request.clone()) {
        let retryable_completion = retry_completion_override
            || receipt_backed_retry_completion
            || retryable_host_bridge_completion_request_for_state_root(state_root, request);
        let decision = validate_host_bridge_request_provenance(&HostBridgeProvenanceInput {
            request: typed_request,
            expected_run_id: Some(run_id.to_string()),
            expected_task_id: None,
            expected_dispatch_target: host_bridge_request_string(request, "dispatch_target")
                .map(ToOwned::to_owned),
        });
        let decision =
            normalize_host_bridge_provenance_for_completion(&decision, retryable_completion);
        if !decision.accepted {
            for code in decision.blocker_codes {
                blockers.push(host_bridge_provenance_public_blocker_code(&code).to_string());
            }
            blockers.sort();
            blockers.dedup();
        }
    }
    let retryable_completion = retry_completion_override
        || host_bridge_request_string(request, "status") == Some("retryable_blocked")
        || receipt_backed_retry_completion
        || retryable_host_bridge_completion_request_for_state_root(state_root, request);
    if retryable_completion {
        blockers.retain(|code| {
            code != taskflow_contracts::BlockerCode::HostBridgeRequestNotPending.as_str()
        });
    }
    if host_bridge_packet_is_empty_object(canonical_packet_path.as_deref())
        && !completed_host_bridge_completion_request_for_state_root(&state_root, request)
    {
        blockers.push(blocker_code_value(
            taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing,
        ));
        return blockers;
    }
    let store = match StateStore::open_existing_read_only_with_timeout(
        state_root.to_path_buf(),
        HOST_BRIDGE_PROVENANCE_LOCK_TIMEOUT,
    )
    .await
    {
        Ok(store) => store,
        Err(_) => {
            if !retryable_completion {
                blockers.push(blocker_code_value(
                    taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing,
                ));
            }
            return blockers;
        }
    };
    append_host_bridge_dispatch_receipt_blockers(
        &mut blockers,
        &store,
        state_root,
        request,
        run_id,
        canonical_packet_path.as_deref(),
    )
    .await;
    blockers
}

async fn host_bridge_request_has_retryable_dispatch_receipt_for_state_root(
    state_root: &Path,
    request: &serde_json::Value,
    canonical_packet_path: Option<&Path>,
) -> bool {
    let Some(run_id) = host_bridge_request_string(request, "run_id") else {
        return false;
    };
    let Ok(store) = StateStore::open_existing_read_only_with_timeout(
        state_root.to_path_buf(),
        HOST_BRIDGE_PROVENANCE_LOCK_TIMEOUT,
    )
    .await
    else {
        return false;
    };
    let Ok(Some(receipt)) = store.run_graph_dispatch_receipt(run_id).await else {
        store.close().await;
        return false;
    };
    let retryable = retryable_host_bridge_completion_receipt_matches_request(
        state_root,
        request,
        &receipt,
        canonical_packet_path,
    );
    store.close().await;
    retryable
}

fn retryable_host_bridge_completion_receipt_matches_request(
    state_root: &Path,
    request: &serde_json::Value,
    receipt: &state_store::RunGraphDispatchReceipt,
    canonical_packet_path: Option<&Path>,
) -> bool {
    if !host_bridge_dispatch_receipt_has_retryable_completion_evidence(receipt)
        || host_bridge_dispatch_receipt_target_mismatch(request, receipt, false)
    {
        return false;
    }
    if host_bridge_request_string(request, "backend_id") != receipt.selected_backend.as_deref() {
        return false;
    }
    let Some(canonical_packet_path) = canonical_packet_path else {
        return false;
    };
    let Some(receipt_packet_path) = receipt.dispatch_packet_path.as_deref() else {
        return false;
    };
    canonical_state_artifact_path(state_root, receipt_packet_path, true)
        .ok()
        .as_deref()
        == Some(canonical_packet_path)
}

fn host_bridge_dispatch_receipt_has_retryable_completion_evidence(
    receipt: &state_store::RunGraphDispatchReceipt,
) -> bool {
    receipt.dispatch_status == release1_blocked_status()
        && (receipt
            .blocker_code
            .as_deref()
            .is_some_and(host_bridge_completion_retryable_blocker)
            || receipt
                .downstream_dispatch_blockers
                .iter()
                .any(|blocker| host_bridge_completion_retryable_blocker(blocker)))
}

async fn append_host_bridge_dispatch_receipt_blockers(
    blockers: &mut Vec<String>,
    store: &StateStore,
    state_root: &Path,
    request: &serde_json::Value,
    run_id: &str,
    canonical_packet_path: Option<&Path>,
) {
    let request_target = host_bridge_request_string(request, "dispatch_target");
    let receipt = match store.run_graph_dispatch_receipt(run_id).await {
        Ok(Some(receipt)) => receipt,
        Err(_) => {
            if !host_bridge_request_matches_reconciled_blocked_status(store, run_id, request_target)
                .await
                && !retryable_host_bridge_completion_request_for_state_root(state_root, request)
            {
                blockers.push(blocker_code_value(
                    taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing,
                ));
            }
            return;
        }
        Ok(None) => {
            if !host_bridge_request_matches_reconciled_blocked_status(store, run_id, request_target)
                .await
                && !retryable_host_bridge_completion_request_for_state_root(state_root, request)
            {
                blockers.push(blocker_code_value(
                    taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing,
                ));
            }
            return;
        }
    };
    let reconciled_blocked_status_matches =
        host_bridge_request_matches_reconciled_blocked_status(store, run_id, request_target).await
            || host_bridge_request_matches_reconciled_source_packet(
                state_root,
                &receipt,
                request_target,
            );
    if reconciled_blocked_status_matches && request_target != Some(receipt.dispatch_target.as_str())
    {
        return;
    }
    let retryable_blocked_receipt =
        host_bridge_dispatch_receipt_has_retryable_completion_evidence(&receipt);
    if !matches!(
        receipt.dispatch_status.as_str(),
        "routed" | "executing" | "bridge_request_pending"
    ) && !retryable_blocked_receipt
        && !retryable_host_bridge_completion_request_for_state_root(state_root, request)
    {
        blockers.push(blocker_code_value(
            taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptInactive,
        ));
    }
    if host_bridge_dispatch_receipt_target_mismatch(
        request,
        &receipt,
        reconciled_blocked_status_matches,
    ) || host_bridge_request_string(request, "backend_id") != receipt.selected_backend.as_deref()
        || canonical_packet_path
            .as_ref()
            .map(|path| path.display().to_string())
            != receipt.dispatch_packet_path.as_ref().and_then(|path| {
                canonical_state_artifact_path(&state_root, path, true)
                    .ok()
                    .map(|path| path.display().to_string())
            })
    {
        blockers.push(blocker_code_value(
            taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMismatch,
        ));
    }
}

fn host_bridge_dispatch_receipt_target_mismatch(
    request: &serde_json::Value,
    receipt: &state_store::RunGraphDispatchReceipt,
    allow_active_packet_target_override: bool,
) -> bool {
    let Ok(request) = HostBridgeRequest::from_value(request.clone()) else {
        return host_bridge_request_string(request, "dispatch_target")
            != Some(receipt.dispatch_target.as_str());
    };
    let decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
        request,
        receipt: Some(serde_json::json!({
            "receipt_backed": true,
            "dispatch_status": receipt.dispatch_status,
            "run_id": receipt.run_id,
            "dispatch_target": receipt.dispatch_target,
        })),
        allow_active_packet_target_override,
    });
    decision
        .blocker_codes
        .iter()
        .any(|code| code == "receipt_dispatch_target_mismatch")
}

async fn host_bridge_request_matches_reconciled_blocked_status(
    store: &StateStore,
    run_id: &str,
    request_target: Option<&str>,
) -> bool {
    let Some(request_target) = request_target else {
        return false;
    };
    let Ok(status) = store.run_graph_status(run_id).await else {
        return false;
    };
    (status.active_node.trim() == request_target
        || status.resume_target == format!("dispatch.{request_target}"))
        && (status.policy_gate == "host_tool_bridge_adapter_required"
            || status.lifecycle_stage == format!("{request_target}_blocked"))
}

fn host_bridge_request_matches_reconciled_source_packet(
    state_root: &Path,
    receipt: &state_store::RunGraphDispatchReceipt,
    request_target: Option<&str>,
) -> bool {
    let Some(request_target) = request_target
        .map(str::trim)
        .filter(|target| !target.is_empty())
    else {
        return false;
    };
    let Some(packet_path) = receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return false;
    };
    let Ok(packet_path) = canonical_state_artifact_path(state_root, packet_path, true) else {
        return false;
    };
    let Ok(packet) = read_canonical_host_bridge_json_artifact(&packet_path, "dispatch packet")
    else {
        return false;
    };
    if packet
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        != Some(receipt.run_id.trim())
    {
        return false;
    }
    let source_target_matches = packet
        .get("source_dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        == Some(request_target);
    let source_status = packet
        .get("source_dispatch_status")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let source_blocked = packet
        .get("source_blocker_code")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    source_target_matches
        && (source_blocked || matches!(source_status, "bridge_request_pending" | "blocked"))
}

fn host_bridge_packet_is_empty_object(canonical_packet_path: Option<&Path>) -> bool {
    let Some(packet_path) = canonical_packet_path else {
        return false;
    };
    read_canonical_host_bridge_json_artifact(packet_path, "host bridge packet")
        .ok()
        .and_then(|packet| packet.as_object().map(serde_json::Map::is_empty))
        .unwrap_or(false)
}

fn host_bridge_complete_can_defer_missing_dispatch_receipt(blockers: &[String]) -> bool {
    blockers.len() == 1
        && blockers[0] == taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing.as_str()
}

fn host_bridge_payload_should_show_completion_command(payload: &serde_json::Value) -> bool {
    payload["status"].as_str() == Some(release1_pass_status())
        || payload["blocker_codes"].as_array().is_some_and(|blockers| {
            blockers.len() == 1
                && blockers[0].as_str()
                    == Some(
                        taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing.as_str(),
                    )
        })
}

pub(crate) fn host_bridge_auto_invocation_scaffold_for_payload(
    payload: &serde_json::Value,
) -> serde_json::Value {
    let host_bridge = &payload["host_bridge"];
    let request_path = host_bridge["request_path"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["request_path"].as_str());
    let packet_path = host_bridge["packet_path"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["packet_path"].as_str());
    let result_path = host_bridge["result_path"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["result_path"].as_str());
    let receipt_path = host_bridge["receipt_path"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["receipt_path"].as_str());
    let adapter_kind = host_bridge["adapter_kind"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["adapter_kind"].as_str());
    let adapter_capability_id = host_bridge["adapter_capability_id"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["adapter_capability_id"].as_str());
    let invocation_mode = host_bridge["invocation_mode"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["invocation_mode"].as_str())
        .unwrap_or("parent_host_tool_api");
    let dispatch_transport = host_bridge["dispatch_transport"]
        .as_str()
        .or_else(|| payload["host_tool_bridge_request"]["dispatch_transport"].as_str());
    let blocker_codes = payload["blocker_codes"]
        .as_array()
        .map(|codes| {
            codes
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut blocker_codes = blocker_codes;
    if let Some(code) = payload["blocker_code"]
        .as_str()
        .map(str::trim)
        .filter(|code| !code.is_empty())
    {
        blocker_codes.push(code);
    }
    let request_gate_ready = dispatch_transport == Some("host_tool_bridge")
        && adapter_kind == Some("codex_host_tools")
        && adapter_capability_id == Some("codex.multi_agent_v1")
        && request_path.is_some()
        && packet_path.is_some()
        && result_path.is_some()
        && receipt_path.is_some();
    let only_adapter_required = blocker_codes.iter().all(|code| {
        *code == crate::release1_contracts::BlockerCode::HostToolBridgeAdapterRequired.as_str()
    });
    let safe_to_auto_invoke = request_gate_ready
        && (payload["status"].as_str() == Some(release1_pass_status()) || only_adapter_required);

    serde_json::json!({
        "schema_version": "host-bridge-auto-invocation-v1",
        "status": if safe_to_auto_invoke { "ready_to_invoke_parent_host_adapter" } else { "blocked_by_request_gate" },
        "safe_to_auto_invoke": safe_to_auto_invoke,
        "auto_invoke_supported": safe_to_auto_invoke,
        "adapter_kind": adapter_kind,
        "adapter_capability_id": adapter_capability_id,
        "invocation_mode": invocation_mode,
        "dispatch_transport": dispatch_transport,
        "request_path": request_path,
        "packet_path": packet_path,
        "result_path": result_path,
        "receipt_path": receipt_path,
        "tool_sequence": if safe_to_auto_invoke {
            serde_json::json!([
                "multi_agent_v1.spawn_agent",
                "multi_agent_v1.wait_agent",
                "multi_agent_v1.close_agent"
            ])
        } else {
            serde_json::json!([])
        },
        "result_contract": {
            "required_fields": [
                "decision",
                "verdict",
                "blocker_codes",
                "rework_target",
                "allowed_next_node"
            ],
            "blocked_blocker_codes": [
                taskflow_contracts::BlockerCode::HostAgentCapacityUnavailable.as_str(),
                taskflow_contracts::BlockerCode::HostToolCapabilityMissing.as_str(),
                "host_agent_execution_failed"
            ]
        },
        "binary_boundary": "vida.exe scaffolds and validates; the parent host session invokes native host tools and writes receipt-backed artifacts"
    })
}

pub(crate) fn attach_host_bridge_auto_invocation_scaffold(result: &mut serde_json::Value) -> bool {
    if result.get("host_bridge_auto_invocation").is_some() {
        return false;
    }
    let has_bridge_request = result.get("host_tool_bridge_request").is_some();
    let has_bridge_payload = result.get("host_bridge").is_some();
    if !has_bridge_request && !has_bridge_payload {
        return false;
    }
    let scaffold = host_bridge_auto_invocation_scaffold_for_payload(result);
    if let Some(object) = result.as_object_mut() {
        object.insert("host_bridge_auto_invocation".to_string(), scaffold);
        true
    } else {
        false
    }
}

fn retryable_host_bridge_completion_request_for_state_root(
    state_root: &Path,
    request: &serde_json::Value,
) -> bool {
    if completed_host_bridge_completion_request_for_state_root(state_root, request) {
        return true;
    }
    if !matches!(
        host_bridge_request_lifecycle_string(request, "status")
            .or_else(|| host_bridge_request_lifecycle_string(request, "request_status")),
        Some(status)
            if status == release1_blocked_status() || status == "retryable_blocked"
    ) || host_bridge_request_lifecycle_string(request, "dispatch_transport")
        != Some("host_tool_bridge")
    {
        return false;
    }
    if host_bridge_request_has_retryable_blocked_result_contract(request) {
        return true;
    }
    for field in ["receipt_path", "result_path"] {
        let Some(raw_path) = host_bridge_request_string(request, field) else {
            continue;
        };
        let Ok(path) = canonical_state_artifact_path(state_root, raw_path, true) else {
            continue;
        };
        let artifact_label = match field {
            "receipt_path" => "host bridge receipt",
            "result_path" => "host bridge result",
            _ => "host bridge artifact",
        };
        let Ok(artifact) = read_canonical_host_bridge_json_artifact(&path, artifact_label) else {
            continue;
        };
        if artifact.get("status").and_then(serde_json::Value::as_str)
            == Some(release1_blocked_status())
            && host_bridge_artifact_has_retryable_completion_blocker(&artifact)
        {
            return true;
        }
    }
    false
}

fn host_bridge_request_has_retryable_blocked_result_contract(request: &serde_json::Value) -> bool {
    host_bridge_blocked_result_contract(request)
        .is_some_and(host_bridge_blocked_result_contract_is_retryable)
}

fn host_bridge_request_lifecycle_string<'a>(
    request: &'a serde_json::Value,
    field: &str,
) -> Option<&'a str> {
    host_bridge_request_string(request, field)
        .or_else(|| {
            request
                .get("host_bridge")
                .and_then(|request| host_bridge_request_string(request, field))
        })
        .or_else(|| {
            request
                .get("host_tool_bridge_request")
                .and_then(|request| host_bridge_request_string(request, field))
        })
        .or_else(|| {
            request
                .get("request")
                .and_then(|request| host_bridge_request_string(request, field))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn completed_host_bridge_completion_request_for_state_root(
    state_root: &Path,
    request: &serde_json::Value,
) -> bool {
    let Some(status) = host_bridge_request_string(request, "status") else {
        return false;
    };
    if !host_bridge_completed_artifact_status_is_admissible(status) && status != "completed" {
        return false;
    }
    if host_bridge_request_string(request, "dispatch_transport") != Some("host_tool_bridge") {
        return false;
    }
    let Some(raw_result_path) = host_bridge_request_string(request, "result_path") else {
        return false;
    };
    let Ok(result_path) = canonical_state_artifact_path(state_root, raw_result_path, true) else {
        return false;
    };
    let Ok(result) = read_canonical_host_bridge_json_artifact(&result_path, "host bridge result")
    else {
        return false;
    };
    host_bridge_completed_result_has_preview_refresh_evidence(request, &result)
}

fn retryable_host_bridge_completion_request(
    request_path: &Path,
    request: &serde_json::Value,
    state_root: Option<&Path>,
) -> bool {
    state_root
        .map(Path::to_path_buf)
        .or_else(|| infer_host_bridge_state_root_from_request_path(request_path))
        .as_deref()
        .is_some_and(|state_root| {
            retryable_host_bridge_completion_request_for_state_root(state_root, request)
        })
}

fn host_bridge_adapter_payload(
    request_path: &Path,
    request: &serde_json::Value,
    provenance_blockers: Vec<String>,
    state_root: Option<&Path>,
    receipt_backed_retry_completion_evidence: bool,
) -> serde_json::Value {
    let retryable_completion_request = receipt_backed_retry_completion_evidence
        || retryable_host_bridge_completion_request(request_path, request, state_root);
    let effective_request = taskflow_host_bridge::effective_host_bridge_request(request);
    let typed_request = HostBridgeRequest::from_value(effective_request.clone()).ok();
    let completion_command = if let Some(request) = typed_request.as_ref() {
        let receipt_id = {
            let run_id = request.run_id.as_str();
            let dispatch_target = request.dispatch_target.as_str();
            format!("{run_id}-{dispatch_target}-host-bridge-receipt")
        };
        let submit_result_path = if provenance_blockers.len() == 1
            && provenance_blockers.first().is_some_and(|blocker| {
                blocker
                    == taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing.as_str()
            })
            && !request.result_path.as_os_str().is_empty()
        {
            request.result_path.display().to_string()
        } else {
            "<host-bridge-result-file>".to_string()
        };
        let retry_arg = if retryable_completion_request {
            " --retry-completion"
        } else {
            ""
        };
        let command = format!(
            "vida agent host-bridge --request {}{} --host-agent-id {} --submit-result {} --receipt-id {}",
            crate::shell_quote(&request_path.display().to_string()),
            retry_arg,
            crate::shell_quote("<host-agent-id>"),
            crate::shell_quote(&submit_result_path),
            crate::shell_quote(&receipt_id)
        );
        command
    } else {
        "repair host bridge request run_id before completion".to_string()
    };
    let artifact_attach_command = typed_request.as_ref().map(|_| {
        format!(
            "vida agent host-bridge --request {} --attach-artifact {} --changed-file {} --artifact-kind patch_proposal",
            crate::shell_quote(&request_path.display().to_string()),
            crate::shell_quote("<artifact-path>"),
            crate::shell_quote("<changed-file>")
        )
    });
    let mut payload = normalize_host_bridge_payload_operator_fields(
        build_host_bridge_adapter_payload(HostBridgeAdapterPayloadInput {
            request_path,
            request,
            provenance_blockers,
            retryable_completion_request,
            completion_command,
            artifact_attach_command,
        }),
    );
    attach_host_bridge_auto_invocation_scaffold(&mut payload);
    payload
}

fn normalize_host_bridge_payload_operator_fields(
    mut payload: serde_json::Value,
) -> serde_json::Value {
    if payload.get("next_actions").is_none() {
        let next_actions = payload["shared_fields"]["next_actions"].clone();
        if !next_actions.is_null() {
            payload["next_actions"] = next_actions;
        }
    }
    if payload.get("artifact_refs").is_none() {
        let artifact_refs = payload["shared_fields"]["artifact_refs"].clone();
        if !artifact_refs.is_null() {
            payload["artifact_refs"] = artifact_refs;
        }
    }
    payload
}

fn emit_host_bridge_payload(payload: &serde_json::Value, as_json: bool) -> ExitCode {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(payload)
                .expect("host bridge adapter payload should render")
        );
    } else {
        let mut fields = vec![operator_output::toon_report::OperatorToonField::text(
            "status",
            payload["status"].as_str().unwrap_or("unknown"),
        )];
        if payload["status"].as_str() == Some(release1_pass_status()) {
            if let Some(command) = payload["host_bridge"]["artifact_attach_command"].as_str() {
                fields.push(operator_output::toon_report::OperatorToonField::text(
                    "attach_artifact",
                    operator_output::command_text::human_command(command),
                ));
            }
        }
        if host_bridge_payload_should_show_completion_command(payload) {
            if let Some(command) = payload["host_bridge"]["completion_command"].as_str() {
                fields.push(operator_output::toon_report::OperatorToonField::text(
                    "completion",
                    operator_output::command_text::human_command(command),
                ));
            }
        }
        if let Some(blockers) = payload["blocker_codes"].as_array() {
            if !blockers.is_empty() {
                fields.push(operator_output::toon_report::OperatorToonField::value(
                    "blocker_codes",
                    serde_json::Value::Array(blockers.clone()),
                ));
            }
        }
        let next_actions = if payload["next_actions"].is_null() {
            &payload["shared_fields"]["next_actions"]
        } else {
            &payload["next_actions"]
        };
        if let Some(actions) = next_actions.as_array() {
            if !actions.is_empty() {
                fields.push(operator_output::toon_report::OperatorToonField::value(
                    "next_actions",
                    serde_json::Value::Array(actions.clone()),
                ));
            }
        }
        let artifact_refs = if payload["artifact_refs"].is_null() {
            &payload["shared_fields"]["artifact_refs"]
        } else {
            &payload["artifact_refs"]
        };
        if !artifact_refs.is_null() {
            fields.push(operator_output::toon_report::OperatorToonField::value(
                "artifact_refs",
                artifact_refs.clone(),
            ));
        }
        operator_output::toon_report::print("vida agent host-bridge", fields);
    }
    if payload["status"].as_str() == Some(release1_pass_status()) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn host_bridge_completion_lane_args(
    request_path: &Path,
    payload: &serde_json::Value,
    host_agent_id: &str,
    summary: Option<&str>,
    receipt_id_override: Option<&str>,
    state_dir: Option<&Path>,
    result_file: Option<&Path>,
    decision: Option<&str>,
    verdict: Option<&str>,
    allowed_next_node: Option<&str>,
    blocker_codes: Option<&str>,
    blocker_code: &[String],
    rework_target: Option<&str>,
    retry_completion: bool,
    as_json: bool,
) -> Result<Vec<String>, String> {
    let run_id = payload["host_bridge"]["run_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "host bridge request payload is missing run_id".to_string())?;
    let receipt_id = receipt_id_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| payload["host_bridge"]["receipt_id"].as_str())
        .ok_or_else(|| "host bridge request payload is missing receipt_id".to_string())?;
    let summary = summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("parent host adapter completed receipt-backed execution");
    let mut args = vec![
        "complete".to_string(),
        run_id.to_string(),
        "--receipt-id".to_string(),
        receipt_id.to_string(),
        "--host-bridge-request".to_string(),
        request_path.display().to_string(),
        "--host-agent-id".to_string(),
        host_agent_id.to_string(),
        "--host-bridge-summary".to_string(),
        summary.to_string(),
    ];
    if let Some(result_file) = result_file {
        args.push("--host-bridge-result-file".to_string());
        args.push(result_file.display().to_string());
    }
    if let Some(decision) = decision.map(str::trim).filter(|value| !value.is_empty()) {
        args.push("--decision".to_string());
        args.push(decision.to_string());
    }
    if let Some(verdict) = verdict.map(str::trim).filter(|value| !value.is_empty()) {
        args.push("--verdict".to_string());
        args.push(verdict.to_string());
    }
    if let Some(allowed_next_node) = allowed_next_node
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push("--allowed-next-node".to_string());
        args.push(allowed_next_node.to_string());
    }
    if let Some(blocker_codes) = blocker_codes
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push("--blocker-codes".to_string());
        args.push(blocker_codes.to_string());
    }
    for blocker_code in blocker_code
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push("--blocker-code".to_string());
        args.push(blocker_code.to_string());
    }
    if let Some(rework_target) = rework_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push("--rework-target".to_string());
        args.push(rework_target.to_string());
    }
    if let Some(state_dir) = state_dir {
        args.push("--state-dir".to_string());
        args.push(state_dir.display().to_string());
    }
    if retry_completion {
        args.push("--retry-host-bridge-completion".to_string());
    }
    if as_json {
        args.push("--json".to_string());
    }
    Ok(args)
}

fn host_bridge_result_string<'a>(result: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    result
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn host_bridge_result_blocker_codes(result: &serde_json::Value) -> Vec<String> {
    result
        .get("blocker_codes")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn host_bridge_command_blocker_codes(command: &AgentHostBridgeArgs) -> Vec<String> {
    let mut blocker_codes = command
        .blocker_codes
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            serde_json::from_str::<Vec<String>>(value).unwrap_or_else(|_| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|code| !code.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
        })
        .unwrap_or_default();
    blocker_codes.extend(
        command
            .blocker_code
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(ToOwned::to_owned),
    );
    blocker_codes.sort();
    blocker_codes.dedup();
    blocker_codes
}

fn host_bridge_result_uses_expected_red_pass_alias(result: &serde_json::Value) -> bool {
    matches!(
        host_bridge_result_string(result, "decision"),
        Some(decision) if decision.replace('-', "_").starts_with("pass_to_")
    ) && host_bridge_result_string(result, "verdict")
        .is_some_and(|verdict| verdict.replace('-', "_") == "test_contract_ready_with_expected_red")
}

fn host_bridge_result_allowed_next_matches_request(
    request: &serde_json::Value,
    result: &serde_json::Value,
) -> bool {
    let Some(result_allowed) = host_bridge_result_string(result, "allowed_next_node") else {
        return true;
    };
    request
        .get("allowed_next_node")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some_and(|expected| expected == result_allowed)
}

fn host_bridge_result_allowed_next_is_lawful(
    request: &serde_json::Value,
    result: &serde_json::Value,
) -> bool {
    if host_bridge_result_allowed_next_matches_request(request, result) {
        return true;
    }
    let Some(result_allowed) = host_bridge_result_string(result, "allowed_next_node") else {
        return true;
    };
    if host_bridge_result_allowed_next_matches_blocked_result_contract(request, result) {
        return true;
    }
    let Some(packet_path) = host_bridge_request_string(request, "packet_path") else {
        return false;
    };
    let packet_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(packet_path);
    let Some(packet) = crate::read_json_file_if_present(&packet_path) else {
        return false;
    };
    let execution_plan = packet
        .get("role_selection_full")
        .or_else(|| packet.get("role_selection"))
        .and_then(|selection| selection.get("execution_plan"));
    let Some(execution_plan) = execution_plan else {
        return false;
    };
    let completed_target = host_bridge_result_string(result, "dispatch_target")
        .or_else(|| host_bridge_request_string(request, "dispatch_target"))
        .unwrap_or_default();
    let previous_target = packet
        .get("downstream_dispatch_last_target")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            packet
                .get("source_dispatch_target")
                .and_then(serde_json::Value::as_str)
        });
    if crate::runtime_dispatch_state::lawful_explicit_downstream_dispatch_target_for_completed_target(
        execution_plan,
        completed_target,
        previous_target,
        result_allowed,
    )
    .is_some()
    {
        return true;
    }
    let Some(rework_target) = host_bridge_result_rework_target(result) else {
        return false;
    };
    if !host_bridge_result_is_rework_completion(result) {
        return false;
    }
    crate::runtime_dispatch_state::lawful_explicit_rework_dispatch_target_for_completed_target(
        execution_plan,
        completed_target,
        previous_target,
        result_allowed,
        rework_target,
    )
    .is_some()
}

fn host_bridge_result_allowed_next_matches_blocked_result_contract(
    request: &serde_json::Value,
    result: &serde_json::Value,
) -> bool {
    let Some(result_allowed) = host_bridge_result_string(result, "allowed_next_node")
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "next")
    else {
        return false;
    };
    let Some(contract) = host_bridge_blocked_result_contract(request) else {
        return false;
    };
    contract
        .get("allowed_next_node")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        == Some(result_allowed)
        && host_bridge_result_is_rework_completion(result)
        && host_bridge_blocked_result_contract_is_retryable(contract)
}

fn host_bridge_result_rework_target(result: &serde_json::Value) -> Option<&str> {
    host_bridge_result_string(result, "rework_target")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !matches!(*value, "none" | "null" | "closure"))
}

fn host_bridge_result_is_rework_completion(result: &serde_json::Value) -> bool {
    for field in ["completion_verdict", "decision", "verdict", "status"] {
        if let Some(value) = host_bridge_result_string(result, field)
            .map(|value| value.trim().to_ascii_lowercase().replace('-', "_"))
        {
            if matches!(
                value.as_str(),
                "rework" | "rework_required" | "blocked" | "fail" | "failed"
            ) {
                return true;
            }
        }
    }
    !host_bridge_result_blocker_codes(result).is_empty()
}

fn host_bridge_handle_state_from_result(
    result_file: Option<&Path>,
    command_blocker_codes: &[String],
) -> (String, Vec<String>) {
    let mut blocker_codes = command_blocker_codes.to_vec();
    let mut status = None;
    if let Some(result_file) = result_file {
        match read_canonical_host_bridge_json_artifact(result_file, "host bridge result") {
            Ok(result) => {
                status = result["status"]
                    .as_str()
                    .or_else(|| result["decision"].as_str())
                    .or_else(|| result["verdict"].as_str())
                    .map(str::to_string);
                if let Some(code) = result["blocker_code"].as_str() {
                    blocker_codes.push(code.to_string());
                }
                if let Some(codes) = result["blocker_codes"].as_array() {
                    blocker_codes.extend(
                        codes
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string),
                    );
                }
            }
            Err(_) => {
                blocker_codes.push("host_bridge_result_unreadable".to_string());
                status = Some("blocked".to_string());
            }
        }
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    if blocker_codes
        .iter()
        .any(|code| code == taskflow_contracts::BlockerCode::HostAgentCapacityUnavailable.as_str())
    {
        return ("capacity_unavailable".to_string(), blocker_codes);
    }
    if blocker_codes.is_empty()
        && match status.as_deref() {
            Some(status) => matches!(status, "pass" | "completed" | "done"),
            None => true,
        }
    {
        ("completed".to_string(), blocker_codes)
    } else {
        ("failed".to_string(), blocker_codes)
    }
}

fn emit_host_bridge_result_scaffold_blocked(
    request_path: &Path,
    result_path: &Path,
    as_json: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    error: String,
) -> ExitCode {
    let artifact_refs = serde_json::json!({
        "request_path": request_path.display().to_string(),
        "result_path": result_path.display().to_string()
    });
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        release1_blocked_status(),
        blocker_codes.clone(),
        next_actions.clone(),
        next_actions,
        artifact_refs.clone(),
    );
    let payload = serde_json::json!({
        "surface": "vida agent host-bridge",
        "mode": "result_scaffold",
        "status": release1_blocked_status(),
        "blocker_codes": blocker_codes,
        "next_actions": shared_fields["next_actions"].clone(),
        "artifact_refs": artifact_refs,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
        "error": error
    });
    emit_host_bridge_payload(&payload, as_json)
}

fn scaffold_host_bridge_result(
    command: &AgentHostBridgeArgs,
    request_path: &Path,
    request: &serde_json::Value,
    result_path: &Path,
) -> ExitCode {
    let effective_request = taskflow_host_bridge::effective_host_bridge_request(request);
    let typed_request = match HostBridgeRequest::from_value(effective_request) {
        Ok(request) => request,
        Err(error) => {
            return emit_host_bridge_result_scaffold_blocked(
                request_path,
                result_path,
                command.json,
                vec!["host_bridge_request_schema_invalid".to_string()],
                vec![format!(
                    "repair host bridge request before scaffolding result: {error}"
                )],
                error.to_string(),
            );
        }
    };
    let state_dir = command
        .state_dir
        .clone()
        .or_else(|| infer_host_bridge_state_root_from_request_path(request_path))
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let state_root = match StateRoot::open(&state_dir) {
        Ok(root) => root,
        Err(error) => {
            return emit_host_bridge_result_scaffold_blocked(
                request_path,
                result_path,
                command.json,
                vec![blocker_code_value(
                    taskflow_contracts::BlockerCode::HostBridgeStateRootMissing,
                )],
                vec![format!(
                    "open the TaskFlow state root before scaffolding result: {error}"
                )],
                error.to_string(),
            );
        }
    };
    let output_path = match new_output_path_under_root(
        &state_root,
        result_path,
        ArtifactPathKind::HostBridgeResult,
        true,
    ) {
        Ok(path) => path,
        Err(error) => {
            return emit_host_bridge_result_scaffold_blocked(
                request_path,
                result_path,
                command.json,
                vec!["host_bridge_result_untrusted_path".to_string()],
                vec!["write scaffolded host bridge results under the VIDA state root".to_string()],
                error.to_string(),
            );
        }
    };
    let result = taskflow_host_bridge::receipt_binding::build_host_bridge_result_scaffold(
        taskflow_host_bridge::receipt_binding::HostBridgeResultScaffoldInput {
            request: typed_request,
            decision: command.decision.clone(),
            verdict: command.verdict.clone(),
            blocker_codes: host_bridge_command_blocker_codes(command),
            rework_target: command.rework_target.clone(),
            allowed_next_node: command.allowed_next_node.clone(),
            summary: command.summary.clone(),
            host_agent_id: command.host_agent_id.clone(),
            receipt_id: command.receipt_id.clone(),
        },
    );
    let rendered = match serde_json::to_string_pretty(&result) {
        Ok(rendered) => rendered,
        Err(error) => {
            return emit_host_bridge_result_scaffold_blocked(
                request_path,
                output_path.path(),
                command.json,
                vec!["host_bridge_result_schema_invalid".to_string()],
                vec!["repair scaffold inputs before writing result JSON".to_string()],
                error.to_string(),
            );
        }
    };
    if let Err(error) = std::fs::write(output_path.path(), format!("{rendered}\n")) {
        return emit_host_bridge_result_scaffold_blocked(
            request_path,
            output_path.path(),
            command.json,
            vec!["host_bridge_result_write_failed".to_string()],
            vec!["repair the result artifact path and retry --scaffold-result".to_string()],
            error.to_string(),
        );
    }
    let validation =
        validate_host_bridge_result_dry_run(request_path, request, output_path.path(), &result);
    let blocker_codes = host_bridge_result_blocker_codes(&validation);
    let validate_command = format!(
        "vida agent host-bridge --request {} --validate-result {}{}",
        crate::shell_quote(&request_path.display().to_string()),
        crate::shell_quote(&output_path.path().display().to_string()),
        command
            .state_dir
            .as_ref()
            .map(|state_dir| format!(
                " --state-dir {}",
                crate::shell_quote(&state_dir.display().to_string())
            ))
            .unwrap_or_default()
    );
    let mut next_actions = vec![validate_command.clone()];
    if blocker_codes.is_empty() {
        next_actions.push(format!(
            "vida agent host-bridge --request {} --submit-result {} --host-agent-id <host-agent-id> --receipt-id <receipt-id>{}",
            crate::shell_quote(&request_path.display().to_string()),
            crate::shell_quote(&output_path.path().display().to_string()),
            command
                .state_dir
                .as_ref()
                .map(|state_dir| format!(" --state-dir {}", crate::shell_quote(&state_dir.display().to_string())))
                .unwrap_or_default()
        ));
    }
    let status = if blocker_codes.is_empty() {
        release1_pass_status()
    } else {
        release1_blocked_status()
    };
    let artifact_refs = serde_json::json!({
        "request_path": request_path.display().to_string(),
        "result_path": output_path.path().display().to_string(),
        "state_dir": state_dir.display().to_string()
    });
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        status,
        blocker_codes.clone(),
        next_actions.clone(),
        next_actions,
        artifact_refs.clone(),
    );
    let payload = serde_json::json!({
        "surface": "vida agent host-bridge",
        "mode": "result_scaffold",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": shared_fields["next_actions"].clone(),
        "artifact_refs": artifact_refs,
        "validation": validation,
        "result": result,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts
    });
    emit_host_bridge_payload(&payload, command.json)
}

fn validate_host_bridge_result_dry_run(
    request_path: &Path,
    request: &serde_json::Value,
    result_path: &Path,
    result: &serde_json::Value,
) -> serde_json::Value {
    let effective_request = taskflow_host_bridge::effective_host_bridge_request(request);
    let typed_request = match HostBridgeRequest::from_value(effective_request) {
        Ok(request) => request,
        Err(error) => {
            return host_bridge_result_validation_payload(
                release1_blocked_status(),
                vec!["host_bridge_request_schema_invalid".to_string()],
                vec![format!(
                    "repair host bridge request before validation: {error}"
                )],
                request_path,
                result_path,
                serde_json::Value::Null,
            );
        }
    };
    let mut blockers = Vec::new();
    for field in &typed_request.required_result_fields {
        if result.get(field).is_none() {
            blockers.push(format!("host_bridge_result_missing_{field}"));
        }
    }
    for (field, expected) in [
        ("request_id", typed_request.request_id.as_str()),
        ("run_id", typed_request.run_id.as_str()),
        ("dispatch_target", typed_request.dispatch_target.as_str()),
    ] {
        if host_bridge_result_string(result, field) != Some(expected) {
            blockers.push(format!("host_bridge_result_{field}_mismatch"));
        }
    }
    if host_bridge_result_string(result, "artifact_kind") != Some("host_tool_bridge_result") {
        blockers.push("host_bridge_result_artifact_kind_invalid".to_string());
    }
    let result_status = host_bridge_result_string(result, "status");
    if !result_status
        .is_some_and(taskflow_host_bridge::host_bridge_completed_result_status_is_admissible)
    {
        blockers.push("host_bridge_result_status_invalid".to_string());
    }
    let execution_state = host_bridge_result_string(result, "execution_state");
    if !execution_state.is_some_and(
        taskflow_host_bridge::host_bridge_completed_result_execution_state_is_admissible,
    ) {
        blockers.push("host_bridge_result_execution_state_invalid".to_string());
    }
    if host_bridge_result_string(result, "source_dispatch_packet_path").is_none() {
        blockers.push("host_bridge_result_source_dispatch_packet_path_missing".to_string());
    }
    if result
        .get("execution_evidence")
        .and_then(|evidence| evidence.get("receipt_backed"))
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        blockers.push("host_bridge_result_execution_evidence_not_receipt_backed".to_string());
    }
    let expected_red_pass_alias = host_bridge_result_uses_expected_red_pass_alias(result);
    let typed_blocker_codes = host_bridge_result_blocker_codes(result);
    if !host_bridge_result_allowed_next_is_lawful(request, result) {
        blockers.push("invalid_allowed_next_node_for_execution_plan".to_string());
    }
    if expected_red_pass_alias {
        if result_status != Some("pass") {
            blockers.push("host_bridge_completion_outcome_contradiction".to_string());
        }
        if execution_state != Some("executed") {
            blockers.push("host_bridge_completion_outcome_contradiction".to_string());
        }
        if !typed_blocker_codes.is_empty() {
            blockers.push("host_bridge_completion_outcome_contradiction".to_string());
        }
    }
    let authority_decision = if expected_red_pass_alias && blockers.is_empty() {
        "pass"
    } else {
        host_bridge_result_string(result, "decision").unwrap_or_default()
    };
    let authority_verdict = if expected_red_pass_alias && blockers.is_empty() {
        "pass"
    } else {
        host_bridge_result_string(result, "verdict").unwrap_or_default()
    };
    let decision = decide_host_bridge_completion_authority(HostBridgeCompletionAuthorityInput {
        decision: authority_decision.to_string(),
        verdict: authority_verdict.to_string(),
        blocker_codes: typed_blocker_codes,
        summary: host_bridge_result_string(result, "summary").map(ToOwned::to_owned),
        provenance_valid: true,
        receipt_bound: true,
        next_step_packet_requested: true,
    });
    if !decision.accepted
        && host_bridge_result_string(result, "verdict").is_some_and(|verdict| verdict == "pass")
    {
        blockers.extend(decision.blocker_codes.clone());
    }
    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        host_bridge_result_validation_payload(
            release1_pass_status(),
            Vec::new(),
            Vec::new(),
            request_path,
            result_path,
            serde_json::json!({
                "accepted_completion": decision.accepted,
                "final_state": format!("{:?}", decision.final_state),
                "authority_blocker_codes": decision.blocker_codes
            }),
        )
    } else {
        host_bridge_result_validation_payload(
            release1_blocked_status(),
            blockers,
            vec!["repair the host bridge result before submit-result".to_string()],
            request_path,
            result_path,
            serde_json::json!({
                "accepted_completion": decision.accepted,
                "final_state": format!("{:?}", decision.final_state),
                "authority_blocker_codes": decision.blocker_codes
            }),
        )
    }
}

fn host_bridge_result_validation_payload(
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    request_path: &Path,
    result_path: &Path,
    validation: serde_json::Value,
) -> serde_json::Value {
    let artifact_refs = serde_json::json!({
        "request_path": request_path.display().to_string(),
        "result_path": result_path.display().to_string()
    });
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        status,
        blocker_codes.clone(),
        next_actions.clone(),
        next_actions,
        artifact_refs.clone(),
    );
    serde_json::json!({
        "surface": "vida agent host-bridge",
        "mode": "result_validate",
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": shared_fields["next_actions"].clone(),
        "artifact_refs": artifact_refs,
        "validation": validation,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts
    })
}

async fn attach_host_bridge_implementation_artifacts(
    command: AgentHostBridgeArgs,
    mut request: serde_json::Value,
    payload: serde_json::Value,
) -> ExitCode {
    if command.complete {
        return emit_host_bridge_attach_blocked(
            &command.request,
            command.json,
            vec![
                taskflow_contracts::BlockerCode::HostBridgeCompletionArgsInvalid
                    .as_str()
                    .to_string(),
            ],
            vec![
                "run artifact attachment and lane completion as separate receipt-backed steps"
                    .to_string(),
            ],
            serde_json::json!({ "request_path": command.request.display().to_string() }),
        );
    }
    if payload["status"].as_str() != Some(release1_pass_status()) {
        return emit_host_bridge_payload(&payload, command.json);
    }
    let run_id = match payload["host_bridge"]["run_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value.to_string(),
        None => {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                vec![
                    taskflow_contracts::BlockerCode::HostBridgeRequestMissingFields
                        .as_str()
                        .to_string(),
                ],
                vec![
                "repair the host bridge request run_id before attaching implementation artifacts"
                    .to_string(),
            ],
                serde_json::json!({ "request_path": command.request.display().to_string() }),
            );
        }
    };
    let dispatch_target = payload["host_bridge"]["dispatch_target"]
        .as_str()
        .map(str::trim)
        .unwrap_or_default();
    let task_class = host_bridge_request_string(&request, "task_class");
    if !host_bridge_request_requires_implementation_artifacts(dispatch_target, task_class) {
        return emit_host_bridge_attach_blocked(
            &command.request,
            command.json,
            vec![blocker_code_value(
                taskflow_contracts::BlockerCode::ImplementationArtifactContractInvalid,
            )],
            vec![
                "attach implementation artifacts only to implementation host bridge requests"
                    .to_string(),
            ],
            serde_json::json!({
                "request_path": command.request.display().to_string(),
                "dispatch_target": dispatch_target,
                "task_class": task_class,
            }),
        );
    }
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let store = match StateStore::open_existing(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                vec![blocker_code_value(
                    taskflow_contracts::BlockerCode::HostBridgeStateRootMissing,
                )],
                vec![format!(
                    "open the TaskFlow state store before attaching artifacts: {error}"
                )],
                serde_json::json!({
                    "request_path": command.request.display().to_string(),
                    "state_dir": state_dir.display().to_string(),
                }),
            );
        }
    };
    let task = match store.show_task(&run_id).await {
        Ok(task) => task,
        Err(error) => {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                vec![blocker_code_value(
                    taskflow_contracts::BlockerCode::ImplementationArtifactAuthorityMissing,
                )],
                vec![format!(
                    "repair the TaskFlow task binding before attaching artifacts: {error}"
                )],
                serde_json::json!({
                    "request_path": command.request.display().to_string(),
                    "task_id": run_id,
                }),
            );
        }
    };
    let mut normalized_artifacts = host_bridge_request_implementation_artifacts(&request);
    let owned_paths = host_bridge_task_or_request_owned_paths(&task, &request);
    let mut proof_artifact_scope = proof_artifact_scope_from_task_request(&store, &task, &request);
    let mut proof_artifact_paths = proof_artifact_scope
        .paths
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    refresh_host_bridge_request_proof_artifact_paths(&mut request, &proof_artifact_paths);
    let attempt_id = normalized_host_bridge_attempt_id(&run_id, command.attempt_id.as_deref());
    let consolidation_receipt_id = normalized_host_bridge_consolidation_receipt_id(
        &attempt_id,
        command.consolidation_receipt_id.as_deref(),
    );
    let mut artifact_refs = Vec::new();
    let mut source_artifact_refs = Vec::new();
    for (index, artifact_path) in command.attach_artifacts.iter().enumerate() {
        let artifact_json = match host_bridge_artifact_file(store.root(), artifact_path) {
            Ok(json) => json,
            Err(error) => {
                return emit_host_bridge_attach_blocked(
                    &command.request,
                    command.json,
                    vec![blocker_code_value(
                        taskflow_contracts::BlockerCode::ImplementationArtifactContractInvalid,
                    )],
                    vec![error],
                    serde_json::json!({
                        "request_path": command.request.display().to_string(),
                        "artifact_path": artifact_path.display().to_string(),
                    }),
                );
            }
        };
        let changed_files =
            host_bridge_changed_files_from_artifact(artifact_json.as_ref(), &command.changed_files);
        if changed_files.is_empty() {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                vec![blocker_code_value(
                    taskflow_contracts::BlockerCode::ImplementationArtifactChangedFilesMissing,
                )],
                vec![
                    "provide --changed-file or attach a JSON artifact with changed_files before lane completion"
                        .to_string(),
                ],
                serde_json::json!({
                    "request_path": command.request.display().to_string(),
                    "artifact_path": artifact_path.display().to_string(),
                }),
            );
        }
        let changed_file_paths = changed_files.iter().map(PathBuf::from).collect::<Vec<_>>();
        if proof_artifact_scope.proof_intent_present {
            proof_artifact_scope
                .paths
                .extend(collect_test_like_paths_from_values(
                    changed_files.iter().map(String::as_str),
                ));
            proof_artifact_scope.paths.sort();
            proof_artifact_scope.paths.dedup();
            proof_artifact_paths = proof_artifact_scope
                .paths
                .iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            refresh_host_bridge_request_proof_artifact_paths(&mut request, &proof_artifact_paths);
        }
        let scope_decision = validate_implementation_artifact_scope_with_proof_paths(
            &changed_file_paths,
            &owned_paths,
            &proof_artifact_paths,
        );
        if !scope_decision.accepted {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                scope_decision.blocker_codes,
                vec![
                    "attach implementation artifacts only when changed files stay within the host bridge owned paths"
                        .to_string(),
                ],
                serde_json::json!({
                    "request_path": command.request.display().to_string(),
                    "artifact_path": artifact_path.display().to_string(),
                    "owned_paths": owned_paths,
                    "proof_artifact_paths": proof_artifact_paths,
                    "out_of_scope_paths": scope_decision.out_of_scope_paths,
                }),
            );
        }
        let normalized_artifact = build_host_bridge_normalized_implementation_artifact(
            &command.artifact_kind,
            &attempt_id,
            &run_id,
            &task.updated_at,
            &consolidation_receipt_id,
            artifact_path,
            artifact_json.as_ref(),
            changed_files,
            &state_dir,
            index,
        );
        source_artifact_refs.push(normalized_artifact.source_artifact_ref.clone());
        let normalized_artifact_path = PathBuf::from(&normalized_artifact.artifact_ref);
        if let Err(error) = write_host_bridge_normalized_implementation_artifact(
            &state_dir,
            &normalized_artifact_path,
            &normalized_artifact.artifact,
        ) {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                vec![blocker_code_value(
                    taskflow_contracts::BlockerCode::ImplementationArtifactContractInvalid,
                )],
                vec![error],
                serde_json::json!({
                    "request_path": command.request.display().to_string(),
                    "artifact_path": artifact_path.display().to_string(),
                }),
            );
        }
        artifact_refs.push(normalized_artifact_path.display().to_string());
        push_unique_host_bridge_implementation_artifact(
            &mut normalized_artifacts,
            normalized_artifact.artifact,
        );
    }
    let task_is_closed = crate::state_store::StateStore::task_status_is_closed_like(&task.status);
    if !task_is_closed {
        if let Err(error) = store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some(attempt_id.clone()),
                task_id: run_id.clone(),
                stage_id: "implementation".to_string(),
                backend: host_bridge_request_string(&request, "backend_id")
                    .unwrap_or("host_tool_bridge")
                    .to_string(),
                model_profile: host_bridge_request_string(&request, "carrier_id")
                    .unwrap_or("host_agent")
                    .to_string(),
                isolation: command.artifact_kind.clone(),
                freshness: Some(task.updated_at.clone()),
                status: "accepted".to_string(),
                artifact_refs: artifact_refs.clone(),
                consolidation_receipt_id: Some(consolidation_receipt_id.clone()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: Some("receipt_backed_host_bridge_artifact".to_string()),
            })
            .await
        {
            return emit_host_bridge_attach_blocked(
                &command.request,
                command.json,
                vec![blocker_code_value(
                    taskflow_contracts::BlockerCode::ImplementationArtifactAuthorityMissing,
                )],
                vec![format!(
                    "record TaskFlow implementation attempt authority before completion: {error}"
                )],
                serde_json::json!({
                    "request_path": command.request.display().to_string(),
                    "task_id": run_id,
                    "attempt_id": attempt_id,
                }),
            );
        }
    }
    if let Some(object) = request.as_object_mut() {
        object.insert(
            "implementation_artifacts".to_string(),
            serde_json::json!(normalized_artifacts),
        );
        object.insert(
            "implementation_artifact_refs".to_string(),
            serde_json::json!(artifact_refs),
        );
        object.insert(
            "implementation_source_artifact_refs".to_string(),
            serde_json::json!(source_artifact_refs),
        );
    }
    if let Err(error) = write_host_bridge_request(store.root(), &command.request, &request) {
        return emit_host_bridge_attach_blocked(
            &command.request,
            command.json,
            vec![blocker_code_value(
                taskflow_contracts::BlockerCode::HostBridgeRequestUnreadable,
            )],
            vec![error],
            serde_json::json!({ "request_path": command.request.display().to_string() }),
        );
    }
    let refreshed_payload = host_bridge_adapter_payload(
        &command.request,
        &request,
        host_bridge_request_provenance_blockers(
            &command.request,
            &request,
            command.state_dir.as_deref(),
            false,
        )
        .await,
        command.state_dir.as_deref(),
        false,
    );
    let completion_command = refreshed_payload["host_bridge"]["completion_command"]
        .as_str()
        .map(human_command)
        .unwrap_or_else(|| {
            "vida agent host-bridge --request <request-path> --host-agent-id <host-agent-id> --submit-result <result-path> --receipt-id <receipt-id>".to_string()
        });
    let artifact_refs_payload = serde_json::json!({
        "request_path": command.request.display().to_string(),
        "attached_artifacts": artifact_refs,
        "source_artifacts": source_artifact_refs,
        "attempt_id": attempt_id,
        "consolidation_receipt_id": consolidation_receipt_id,
    });
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        release1_pass_status(),
        Vec::new(),
        vec![completion_command.clone()],
        vec![completion_command],
        artifact_refs_payload.clone(),
    );
    let payload = serde_json::json!({
        "surface": "vida agent host-bridge attach-artifact",
        "status": release1_pass_status(),
        "blocker_codes": [],
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
        "artifact_refs": artifact_refs_payload,
        "host_bridge": refreshed_payload["host_bridge"].clone(),
        "implementation_artifact_authority": {
            "task_id": run_id,
            "stage_id": "implementation",
            "attempt_id": attempt_id,
            "freshness": task.updated_at,
            "consolidation_receipt_id": consolidation_receipt_id,
            "attempt_recorded": !task_is_closed,
            "authority_source": if task_is_closed {
                "host_bridge_request_embedded_artifact"
            } else {
                "taskflow_attempt_ledger"
            }
        }
    });
    emit_host_bridge_payload(&payload, command.json)
}

fn emit_host_bridge_attach_blocked(
    request_path: &Path,
    as_json: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
) -> ExitCode {
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        release1_blocked_status(),
        blocker_codes.clone(),
        next_actions.clone(),
        next_actions,
        artifact_refs.clone(),
    );
    let payload = serde_json::json!({
        "surface": "vida agent host-bridge attach-artifact",
        "status": release1_blocked_status(),
        "blocker_codes": blocker_codes,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
        "artifact_refs": artifact_refs,
        "host_bridge": {
            "request_path": request_path.display().to_string()
        }
    });
    emit_host_bridge_payload(&payload, as_json)
}

fn build_parallelization_planner(
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
) -> serde_json::Value {
    let ready_parallel_safe = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .count();
    let independent_failures = projection
        .blocked
        .iter()
        .filter(|candidate| !candidate.ready_now)
        .count();
    let triggers = [
        (
            "coverage_or_test_expansion",
            projection.ready.iter().any(|candidate| {
                let title = candidate.task.title.to_ascii_lowercase();
                let work_item_keys = task_flow_lookup_keys(&candidate.task).join(" ");
                let labels = candidate
                    .task
                    .labels
                    .iter()
                    .map(|label| label.to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join(" ");
                title.contains("test")
                    || title.contains("coverage")
                    || work_item_keys.contains("verification")
                    || labels.contains("verification")
                    || labels.contains("quality")
            }),
        ),
        (
            "three_or_more_independent_failures",
            independent_failures >= 3,
        ),
        (
            "parallel_safe_ready_candidates",
            ready_parallel_safe >= 2 && configured_max_parallel_agents > 1,
        ),
    ];
    let active_triggers = triggers
        .into_iter()
        .filter_map(|(trigger, active)| active.then(|| trigger.to_string()))
        .collect::<Vec<_>>();
    let packet_proposals = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .take(lanes_requested.min(configured_max_parallel_agents.max(1)))
        .map(|candidate| {
            serde_json::json!({
                "task_id": candidate.task.id,
                "title": candidate.task.title,
                "proposal_kind": "parallel_safe_dispatch_packet_preview",
                "materializes_packet": false,
                "next_surface": "vida agent-init",
                "reason": "candidate is ready and parallel-safe under TaskFlow scheduling projection"
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if packet_proposals.is_empty() { "no_packet_proposals" } else { "proposals_available" },
        "mode": "preview_only",
        "triggers": active_triggers,
        "ready_parallel_safe_count": ready_parallel_safe,
        "independent_failure_count": independent_failures,
        "packet_proposals": packet_proposals,
        "materializes_packets": false,
        "next_action": if ready_parallel_safe > 0 {
            "review selected lanes and launch with the shown `vida agent-init` command only after operator approval"
        } else {
            "add or unblock parallel-safe execution semantics before expecting planner proposals"
        }
    })
}

fn compact_diagnostics_omitted(diagnostic: &str) -> serde_json::Value {
    serde_json::json!({
        "status": "omitted",
        "view": "compact",
        "diagnostic": diagnostic,
        "full_output_flag": "--full",
    })
}

fn maybe_build_parallelization_planner(
    include_diagnostics: bool,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
) -> serde_json::Value {
    if include_diagnostics {
        build_parallelization_planner(projection, lanes_requested, configured_max_parallel_agents)
    } else {
        compact_diagnostics_omitted("parallelization_planner")
    }
}

fn maybe_build_carrier_selection_api_descriptor(
    include_diagnostics: bool,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    if include_diagnostics {
        build_carrier_selection_api_descriptor(activation_bundle)
    } else {
        compact_diagnostics_omitted("carrier_selection_api")
    }
}

fn maybe_build_fanout_guard_from_projection(
    include_diagnostics: bool,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    selected_lanes: &[AgentDispatchLanePreview],
    blocked_candidates: &[AgentDispatchBlockedCandidate],
    blocker_codes: &[String],
) -> serde_json::Value {
    if include_diagnostics {
        agent_dispatch_fanout_guard_from_projection(
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
        )
    } else {
        compact_diagnostics_omitted("fanout_guard")
    }
}

fn no_packet_materialization() -> serde_json::Value {
    serde_json::json!({
        "status": "not_requested",
        "requested": false,
        "materializes_packets": false,
        "artifacts": [],
    })
}

fn build_carrier_selection_api_descriptor(
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let dev_team_roles = activation_bundle["dev_team_readiness"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| {
            let api_id = role["role_id"].as_str()?.trim();
            let runtime_role = role["runtime_role"].as_str()?.trim();
            let task_class = role["task_classes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .find(|value| !value.is_empty())?;
            if api_id.is_empty() || runtime_role.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "api_id": api_id,
                "runtime_role": runtime_role,
                "task_class": task_class,
                "selection_surface": "vida agent select",
                "selection_materialized": false,
                "selection_reason": "dispatch_next_preview_exposes_selection_api_without_embedding_full_assignment",
                "command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class}"),
                "machine_command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class} --json")
            }))
        })
        .collect::<Vec<_>>();
    let first_class = if dev_team_roles.is_empty() {
        activation_bundle["carrier_runtime"]["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| {
                let api_id = role["role_id"].as_str()?.trim();
                let runtime_role = role["default_runtime_role"]
                    .as_str()
                    .or_else(|| {
                        role["runtime_roles"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .find(|value| !value.trim().is_empty())
                    })?
                    .trim();
                let task_class = role["task_classes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .find(|value| !value.trim().is_empty())?
                    .trim();
                if api_id.is_empty() || runtime_role.is_empty() || task_class.is_empty() {
                    return None;
                }
                Some(serde_json::json!({
                    "api_id": api_id,
                    "runtime_role": runtime_role,
                    "task_class": task_class,
                    "selection_surface": "vida agent select",
                    "selection_materialized": false,
                    "selection_reason": "dispatch_next_preview_exposes_selection_api_without_embedding_full_assignment",
                    "command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class}"),
                    "machine_command": format!("vida agent select --runtime-role {runtime_role} --task-class {task_class} --json")
                }))
            })
            .collect::<Vec<_>>()
    } else {
        dev_team_roles
    };
    serde_json::json!({
        "surface": "vida agent select",
        "mode": "config_driven_runtime_assignment",
        "status": release1_contract_status_value(!first_class.is_empty()),
        "blocker_codes": if first_class.is_empty() {
            vec!["carrier_selection_api_requires_configured_dev_team_roles"]
        } else {
            Vec::<&str>::new()
        },
        "first_class_carriers": first_class,
        "manual_host_tool_choice_required": false,
        "embedded_assignment_diagnostics": false,
        "diagnostics_note": "Run the listed `vida agent select` command for full carrier/model/cost assignment diagnostics.",
    })
}

fn non_dev_team_flow_projection() -> serde_json::Value {
    serde_json::json!({
        "status": "not_applicable",
        "reason": "dev_team_preview_not_enabled",
        "diagnostic_only": true,
    })
}

fn lifecycle_hook_event_stream(
    selected_flow: Option<&serde_json::Value>,
    sequence: &[DevTeamSequenceStep],
) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    if let Some(flow) = selected_flow {
        for hook in flow["lifecycle_hook_templates"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            events.push(serde_json::json!({
                "scope": "flow",
                "template_id": hook,
                "authority": "diagnostic_event_stream",
                "configured_from": "dev_team.flows.lifecycle_hook_templates",
            }));
        }
    }
    for (index, step) in sequence.iter().enumerate() {
        for hook in step
            .lifecycle_hook_templates
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            events.push(serde_json::json!({
                "scope": "step",
                "step_index": index,
                "role_label": step.role_label,
                "template_id": hook,
                "authority": "diagnostic_event_stream",
                "configured_from": "dev_team.flows.steps.lifecycle_hook_templates",
            }));
        }
    }
    events
}

fn build_dev_team_flow_projection(
    activation_bundle: &serde_json::Value,
    selected_flow_id: Option<&str>,
    sequence: &[DevTeamSequenceStep],
    selected_lanes: &[AgentDispatchLanePreview],
    blocker_codes: &[String],
) -> serde_json::Value {
    let readiness = &activation_bundle["dev_team_readiness"];
    let selected_flow = selected_flow_id.and_then(|flow_id| {
        readiness["flows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
    });
    let current_lane = selected_lanes.first();
    let current_step = current_lane
        .map(|lane| {
            serde_json::json!({
                "role_label": lane.role_label,
                "runtime_role": lane.runtime_role,
                "task_class": lane.task_class,
                "task_id": lane.task_id,
                "dispatch_command": lane.dispatch_command,
                "dispatch_command_kind": lane.dispatch_command_kind,
                "receipt_status": {
                    "receipt_backed": false,
                    "receipt_path": null,
                    "status": "preview_only"
                },
                "proof_state": {
                    "status": "pending_dispatch",
                    "diagnostic_only": true
                },
                "approval_gate": lane.approval_gate,
            })
        })
        .or_else(|| {
            sequence.first().map(|step| {
                serde_json::json!({
                    "role_label": step.role_label,
                    "runtime_role": step.runtime_role,
                    "task_class": step.task_class,
                    "task_id": null,
                    "dispatch_command": null,
                    "dispatch_command_kind": null,
                    "receipt_status": {
                        "receipt_backed": false,
                        "receipt_path": null,
                        "status": "not_selected"
                    },
                    "proof_state": {
                        "status": "not_started",
                        "diagnostic_only": true
                    },
                    "approval_gate": {
                        "required": step.requires_user_approval,
                        "status": if step.requires_user_approval {
                            "approval_required_after_step_completion"
                        } else {
                            "not_required"
                        },
                        "policy": step.approval_policy,
                    },
                })
            })
        })
        .unwrap_or(serde_json::Value::Null);
    let approval_waits = selected_lanes
        .iter()
        .filter(|lane| lane.requires_user_approval)
        .map(|lane| {
            serde_json::json!({
                "task_id": lane.task_id,
                "role_label": lane.role_label,
                "status": "approval_required_after_step_completion",
                "policy": lane.approval_gate["policy"],
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if blocker_codes.is_empty() {
            "ready"
        } else {
            release1_blocked_status()
        },
        "flow_id": selected_flow.and_then(|flow| flow["flow_id"].as_str()),
        "flow_class": selected_flow.and_then(|flow| flow["flow_class"].as_str()),
        "work_item_bindings": selected_flow
            .map(|flow| flow["work_item_bindings"].clone())
            .unwrap_or(serde_json::Value::Null),
        "adapter_projection": selected_flow
            .map(|flow| flow["adapter_projection"].clone())
            .unwrap_or(serde_json::Value::Null),
        "adapter_projection_source": "dev_team.flows.adapter_projection",
        "adapter_projection_is_data_only": true,
        "proof_gates": selected_flow
            .map(|flow| flow["proof_gates"].clone())
            .unwrap_or(serde_json::Value::Null),
        "current_step": current_step,
        "steps": sequence.iter().enumerate().map(|(index, step)| {
            serde_json::json!({
                "index": index,
                "role_label": step.role_label,
                "runtime_role": step.runtime_role,
                "task_class": step.task_class,
                "requires_user_approval": step.requires_user_approval,
                "approval_policy": step.approval_policy,
                "lifecycle_hook_templates": step.lifecycle_hook_templates,
                "resume_transitions": step.resume_transitions,
                "rework_transitions": step.rework_transitions,
            })
        }).collect::<Vec<_>>(),
        "approval_waits": approval_waits,
        "lifecycle_hook_event_stream": lifecycle_hook_event_stream(selected_flow, sequence),
        "receipt_status": {
            "receipt_backed": false,
            "receipt_path": null,
            "status": "preview_only"
        },
        "proof_state": {
            "status": "pending_dispatch",
            "diagnostic_only": true
        },
        "diagnostic_only": true,
    })
}

fn suppressed_current_task_flow_projection(blocker_codes: &[String]) -> serde_json::Value {
    serde_json::json!({
        "status": release1_blocked_status(),
        "reason": "current_task_not_ready_for_dev_team_dispatch",
        "flow_id": null,
        "steps": [],
        "current_step": null,
        "diagnostic_only": true,
        "blocker_codes": blocker_codes,
    })
}

fn single_in_progress_task_id_from_rows(rows: &[state_store::TaskRecord]) -> Option<&str> {
    let mut candidates = rows.iter().filter(|task| {
        task.status == "in_progress"
            && state_store::work_item_is_active_bounded_unit_candidate(&task.issue_type)
    });
    let candidate = candidates.next()?;
    candidates.next().is_none().then_some(candidate.id.as_str())
}

fn configured_max_parallel_agents_from_activation_bundle(
    activation_bundle: &serde_json::Value,
) -> usize {
    activation_bundle["agent_system"]["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1)
}

fn agent_init_command(
    task_id: &str,
    state_dir: Option<&std::path::Path>,
    runtime_role: &str,
) -> String {
    let runtime_role = if runtime_role.trim().is_empty() {
        "worker"
    } else {
        runtime_role
    };
    let mut command = format!(
        "vida agent-init --role {} {}",
        crate::shell_quote(runtime_role),
        crate::shell_quote(task_id)
    );
    if let Some(state_dir) = state_dir {
        command.push_str(" --state-dir ");
        command.push_str(&crate::shell_quote(&state_dir.display().to_string()));
    }
    command
}

fn receipt_backed_execution_command_hint(task_id: &str) -> String {
    format!(
        "vida taskflow run-graph dispatch-init {}, then vida agent-init --dispatch-packet <packet-path> --execute-dispatch",
        crate::shell_quote(task_id)
    )
}

fn required_string_field(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload[key]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn packet_string_field(packet: &serde_json::Value, field: &str) -> Option<String> {
    packet
        .get(field)
        .or_else(|| {
            packet
                .get("delivery_task_packet")
                .and_then(|value| value.get(field))
        })
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn packet_handoff_runtime_role(packet: &serde_json::Value) -> Option<String> {
    packet_string_field(packet, "handoff_runtime_role")
        .or_else(|| packet_string_field(packet, "activation_runtime_role"))
        .or_else(|| packet_string_field(packet, "runtime_role"))
}

fn packet_handoff_task_class(packet: &serde_json::Value) -> Option<String> {
    packet_string_field(packet, "handoff_task_class")
        .or_else(|| packet_string_field(packet, "task_class"))
        .or_else(|| packet_string_field(packet, "route_task_class"))
}

fn selection_truth_for_task(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    selection_truth_for_task_with_role_and_class(activation_bundle, task, "worker", None, None)
}

fn selection_truth_for_task_with_role_and_class(
    activation_bundle: &serde_json::Value,
    task: &state_store::TaskRecord,
    conversation_role: &str,
    runtime_role_override: Option<&str>,
    task_class_override: Option<&str>,
) -> Result<AgentDispatchLaneSelectionTruth, String> {
    let task_value = serde_json::to_value(task)
        .map_err(|error| format!("task_record_serialization_failed:{error}"))?;
    let inferred_task_class = crate::infer_task_class_from_task_payload(&task_value);
    let task_class = task_class_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or(inferred_task_class);
    let runtime_role = runtime_role_override
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| crate::runtime_role_for_task_class(&task_class).to_string());
    let assignment = crate::build_runtime_assignment_preview_from_resolved_constraints(
        activation_bundle,
        conversation_role,
        &task_class,
        &runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        let reason = required_string_field(&assignment, "reason")
            .unwrap_or_else(|| "runtime_assignment_disabled".to_string());
        return Err(reason);
    }

    let selected_carrier = required_string_field(&assignment, "selected_carrier_id")
        .ok_or_else(|| "selected_carrier_id_missing".to_string())?;
    let selected_backend = required_string_field(&assignment, "selected_backend_id")
        .ok_or_else(|| "selected_backend_id_missing".to_string())?;
    let selected_model_profile = required_string_field(&assignment, "selected_model_profile_id")
        .ok_or_else(|| "selected_model_profile_id_missing".to_string())?;
    let selected_model_ref = required_string_field(&assignment, "selected_model_ref")
        .ok_or_else(|| "selected_model_ref_missing".to_string())?;
    let selected_reasoning_effort = required_string_field(&assignment, "selected_reasoning_effort")
        .ok_or_else(|| "selected_reasoning_effort_missing".to_string())?;
    let budget_verdict = required_string_field(&assignment, "budget_verdict")
        .ok_or_else(|| "budget_verdict_missing".to_string())?;
    let selected_over_budget = assignment["selected_over_budget"]
        .as_bool()
        .unwrap_or(false);
    let selected_model_profile_readiness_status =
        required_string_field(&assignment, "selected_model_profile_readiness_status")
            .unwrap_or_else(|| "unknown".to_string());
    let pricing_freshness_status = assignment["pricing_readiness"]["pricing_freshness_status"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string();
    let selected_external_backend_readiness_status = assignment
        ["selected_external_backend_readiness"]["status"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("not_applicable")
        .to_string();
    let rate = assignment["rate"]
        .as_u64()
        .ok_or_else(|| "rate_missing".to_string())?;
    let estimated_task_price_units = assignment["estimated_task_price_units"]
        .as_u64()
        .ok_or_else(|| "estimated_task_price_units_missing".to_string())?;

    Ok(AgentDispatchLaneSelectionTruth {
        selected_carrier,
        selected_backend,
        selected_model_profile,
        selected_model_ref,
        selected_reasoning_effort,
        rate,
        estimated_task_price_units,
        budget_verdict,
        selected_over_budget,
        selected_model_profile_readiness_status,
        pricing_freshness_status,
        selected_external_backend_readiness_status,
        selection_source_paths: assignment["selection_source_paths"].clone(),
        pricing_readiness: assignment["pricing_readiness"].clone(),
        runtime_role,
        task_class,
    })
}

fn selection_truth_guard_blockers(truth: &AgentDispatchLaneSelectionTruth) -> Vec<String> {
    let mut blockers = Vec::new();
    if truth.selected_over_budget && truth.budget_verdict == "over_budget" {
        blockers.push(
            taskflow_contracts::BlockerCode::SelectedModelProfileOverBudget
                .as_str()
                .to_string(),
        );
    }
    if truth.selected_model_profile_readiness_status == release1_blocked_status() {
        blockers.push("selected_model_profile_not_ready".to_string());
    }
    let external_backend_readiness_status =
        truth.selected_external_backend_readiness_status.as_str();
    if external_backend_readiness_status == "external_backend_dispatch_blocked"
        || external_backend_readiness_status == release1_blocked_status()
    {
        blockers.push(
            taskflow_contracts::BlockerCode::SelectedExternalBackendNotReady
                .as_str()
                .to_string(),
        );
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn agent_dispatch_host_bridge_capacity_guard() -> serde_json::Value {
    serde_json::json!({
        "status": "parent_host_capacity_unobservable",
        "capacity_observable": false,
        "capacity_source": "parent_host_tool_runtime",
        "active_agents_count": serde_json::Value::Null,
        "thread_limit_reached": serde_json::Value::Null,
        "blocked_result_code": taskflow_contracts::BlockerCode::HostAgentCapacityUnavailable.as_str(),
        "next_actions": [
            "Attempt the parent host bridge only after dispatch admission is otherwise clean.",
            "If the parent host tool reports thread or capacity exhaustion, close stale host agents or write a blocked host bridge result with blocker_code host_agent_capacity_unavailable."
        ]
    })
}

fn agent_dispatch_fanout_guard_from_projection(
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    selected_lanes: &[AgentDispatchLanePreview],
    blocked_candidates: &[AgentDispatchBlockedCandidate],
    blocker_codes: &[String],
) -> serde_json::Value {
    let effective_max_parallel_agents = if lanes_requested == 0 {
        0
    } else {
        lanes_requested.min(configured_max_parallel_agents.max(1))
    };
    let ready_parallel_safe_count = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now && candidate.ready_parallel_safe)
        .count();
    let cap_limited_count = blocked_candidates
        .iter()
        .filter(|candidate| {
            candidate.reasons.iter().any(|reason| {
                reason == "effective_max_parallel_agents_cap_reached"
                    || reason == "max_parallel_agents_cap_reached"
            })
        })
        .count();
    let conflict_rejected_count = blocked_candidates
        .iter()
        .filter(|candidate| {
            candidate.reasons.iter().any(|reason| {
                reason.starts_with("conflict_domain_already_selected:")
                    || reason.starts_with("owned_path_already_selected:")
            })
        })
        .count();
    let unsafe_ready_count = blocked_candidates
        .iter()
        .filter(|candidate| candidate.ready_now && !candidate.ready_parallel_safe)
        .count();
    let assignment_blockers = agent_dispatch_assignment_blockers(selected_lanes);
    serde_json::json!({
        "status": agent_dispatch_contract_status(blocker_codes, &assignment_blockers),
        "configured_max_parallel_agents": configured_max_parallel_agents.max(1),
        "lanes_requested": lanes_requested,
        "effective_max_parallel_agents": effective_max_parallel_agents,
        "lanes_selected": selected_lanes.len(),
        "ready_parallel_safe_count": ready_parallel_safe_count,
        "cap_limited_rejected_count": cap_limited_count,
        "conflict_rejected_count": conflict_rejected_count,
        "unsafe_ready_rejected_count": unsafe_ready_count,
        "assignment_blocker_codes": assignment_blockers,
        "host_bridge_capacity": agent_dispatch_host_bridge_capacity_guard(),
        "blocker_codes": blocker_codes,
    })
}

fn agent_dispatch_fanout_guard_from_scheduler_plan(
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    selected_lanes: &[AgentDispatchLanePreview],
    blocked_candidates: &[AgentDispatchBlockedCandidate],
    blocker_codes: &[String],
) -> serde_json::Value {
    let mut guard = plan.fanout_guard.clone();
    if let Some(object) = guard.as_object_mut() {
        let assignment_blocker_codes = agent_dispatch_assignment_blockers(selected_lanes);
        object.insert(
            "status".to_string(),
            serde_json::json!(agent_dispatch_contract_status(
                blocker_codes,
                &assignment_blocker_codes
            )),
        );
        object.insert(
            "lanes_selected".to_string(),
            serde_json::json!(selected_lanes.len()),
        );
        object.insert(
            "assignment_blocker_codes".to_string(),
            serde_json::json!(assignment_blocker_codes),
        );
        object.insert(
            "agent_preview_blocker_codes".to_string(),
            serde_json::json!(blocker_codes),
        );
        object.insert(
            "agent_preview_rejected_count".to_string(),
            serde_json::json!(blocked_candidates.len()),
        );
        object.insert(
            "host_bridge_capacity".to_string(),
            agent_dispatch_host_bridge_capacity_guard(),
        );
    }
    guard
}

fn blocked_candidate(
    candidate: &state_store::TaskSchedulingCandidate,
    reasons: Vec<String>,
) -> AgentDispatchBlockedCandidate {
    AgentDispatchBlockedCandidate {
        task_id: candidate.task.id.clone(),
        title: candidate.task.title.clone(),
        ready_now: candidate.ready_now,
        ready_parallel_safe: candidate.ready_parallel_safe,
        reasons,
        parallel_blockers: candidate.parallel_blockers.clone(),
    }
}

fn materialization_owned_paths_for_lane_task(
    task: state_store::TaskRecord,
    lane: &AgentDispatchLanePreview,
) -> Vec<String> {
    if lane.task_class == crate::runtime_contract_vocab::TASK_CLASS_SPECIFICATION {
        Vec::new()
    } else {
        task.planner_metadata.owned_paths
    }
}

async fn preflight_agent_dispatch_next_packet_materialization(
    selected_lanes: &[AgentDispatchLanePreview],
    state_dir: &std::path::Path,
) -> Vec<serde_json::Value> {
    let Ok(store) = StateStore::open_existing_read_only(state_dir.to_path_buf()).await else {
        return selected_lanes
            .iter()
            .map(|lane| {
                serde_json::json!({
                    "task_id": lane.task_id,
                    "role_label": lane.role_label,
                    "blocker_code": "dispatch_packet_contract_invalid",
                    "blocker_codes": ["dispatch_packet_contract_invalid"],
                    "missing_fields": ["task_metadata_store"],
                    "next_actions": [
                        "Repair the TaskFlow state store binding before materializing dispatch packets."
                    ],
                    "error": format!(
                        "Runtime dispatch packet `{}` cannot validate required packet fields before materialization because TaskFlow state store `{}` could not be opened",
                        crate::development_flow_orchestration::packet_template_kind_for_dev_team_task_class(
                            lane.task_class.as_str()
                        ),
                        state_dir.display()
                    ),
                })
            })
            .collect();
    };
    let mut errors = Vec::new();
    for lane in selected_lanes {
        let task = store.show_task(&lane.task_id).await.ok();
        let mut missing_fields = Vec::new();
        if lane.runtime_role.trim().is_empty() {
            missing_fields.push("runtime_role");
        }
        if lane.task_class.trim().is_empty() {
            missing_fields.push("task_class");
        }
        if lane.selection_truth.selected_carrier.trim().is_empty() {
            missing_fields.push("carrier_id");
        }
        if crate::runtime_dispatch_packets::delivery_packet_task_class_requires_owned_paths(
            lane.task_class.as_str(),
        ) {
            let owned_paths = task
                .map(|task| materialization_owned_paths_for_lane_task(task, lane))
                .unwrap_or_default();
            if owned_paths.is_empty() {
                missing_fields.push("owned_paths");
            }
        }
        if !missing_fields.is_empty() {
            errors.push(serde_json::json!({
                "task_id": lane.task_id,
                "role_label": lane.role_label,
                "blocker_code": "dispatch_packet_contract_invalid",
                "blocker_codes": ["dispatch_packet_contract_invalid"],
                "missing_fields": missing_fields,
                "next_actions": [
                    "Add the missing TaskFlow planner_metadata.owned_paths or reshape the selected dev-team lane before materializing dispatch packets."
                ],
                "error": format!(
                    "Runtime dispatch packet `{}` is missing required packet fields before materialization: {}",
                    crate::development_flow_orchestration::packet_template_kind_for_dev_team_task_class(
                        lane.task_class.as_str()
                    ),
                    missing_fields.join(", ")
                ),
            }));
        }
    }
    store.close().await;
    errors
}

fn build_agent_dispatch_next_preview(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    dev_team: bool,
) -> AgentDispatchNextPreview {
    build_agent_dispatch_next_preview_with_diagnostics(
        activation_bundle,
        projection,
        lanes_requested,
        configured_max_parallel_agents,
        explicit_state_dir,
        dev_team,
        true,
    )
}

fn build_agent_dispatch_next_preview_with_diagnostics(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    dev_team: bool,
    include_diagnostics: bool,
) -> AgentDispatchNextPreview {
    if dev_team {
        build_agent_dispatch_next_preview_dev_team(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
            include_diagnostics,
        )
    } else {
        build_agent_dispatch_next_preview_standard(
            activation_bundle,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            explicit_state_dir,
            include_diagnostics,
        )
    }
}

fn build_agent_dispatch_next_preview_standard(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    include_diagnostics: bool,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);

    let Some(primary) = projection.ready.first() else {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(format!(
            "Inspect `{}` and resolve blockers before previewing agent dispatch.",
            human_command("vida task ready")
        ));
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        let flow_projection = if include_diagnostics {
            non_dev_team_flow_projection()
        } else {
            compact_diagnostics_omitted("flow_projection")
        };
        let fanout_guard = maybe_build_fanout_guard_from_projection(
            include_diagnostics,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            &selected_lanes,
            &blocked_candidates,
            &blocker_codes,
        );
        return AgentDispatchNextPreview {
            status: release1_blocked_status().to_string(),
            mode: "preview".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            parallelization_planner: maybe_build_parallelization_planner(
                include_diagnostics,
                projection,
                lanes_requested,
                configured_max_parallel_agents,
            ),
            packet_materialization: no_packet_materialization(),
            carrier_selection_api: maybe_build_carrier_selection_api_descriptor(
                include_diagnostics,
                activation_bundle,
            ),
            fanout_guard,
            flow_projection,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    };

    if effective_max_parallel_agents > 0 {
        match selection_truth_for_task(activation_bundle, &primary.task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: 1,
                task_id: primary.task.id.clone(),
                title: primary.task.title.clone(),
                role_label: "default".to_string(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &primary.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &primary.task.id,
                ),
                ready_parallel_safe: primary.ready_parallel_safe,
                selection_reason: "primary_ready_task".to_string(),
                selection_truth,
                requires_user_approval: false,
                approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
            }),
            Err(reason) => {
                blocker_codes.push(
                    taskflow_contracts::selected_lane_runtime_assignment_truth_missing(
                        &primary.task.id,
                        &reason,
                    ),
                );
            }
        }
    }

    let mut remaining = effective_max_parallel_agents.saturating_sub(selected_lanes.len());
    for candidate in projection.ready.iter().skip(1) {
        if candidate.ready_parallel_safe && remaining > 0 {
            match selection_truth_for_task(activation_bundle, &candidate.task) {
                Ok(selection_truth) => {
                    selected_lanes.push(AgentDispatchLanePreview {
                        lane_index: selected_lanes.len() + 1,
                        task_id: candidate.task.id.clone(),
                        title: candidate.task.title.clone(),
                        role_label: "parallel".to_string(),
                        runtime_role: selection_truth.runtime_role.clone(),
                        task_class: selection_truth.task_class.clone(),
                        dispatch_command: agent_init_command(
                            &candidate.task.id,
                            explicit_state_dir,
                            &selection_truth.runtime_role,
                        ),
                        dispatch_command_kind: "startup_activation_view_only".to_string(),
                        receipt_backed_execution_command: receipt_backed_execution_command_hint(
                            &candidate.task.id,
                        ),
                        ready_parallel_safe: candidate.ready_parallel_safe,
                        selection_reason: "parallel_safe_ready_task".to_string(),
                        selection_truth,
                        requires_user_approval: false,
                        approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
                    });
                    remaining -= 1;
                }
                Err(reason) => {
                    blocker_codes.push(
                        taskflow_contracts::selected_lane_runtime_assignment_truth_missing(
                            &candidate.task.id,
                            &reason,
                        ),
                    );
                }
            }
            continue;
        }

        let reasons = if candidate.ready_parallel_safe {
            vec!["effective_max_parallel_agents_cap_reached".to_string()]
        } else if candidate.parallel_blockers.is_empty() {
            vec!["parallel_safety_not_established".to_string()]
        } else {
            candidate.parallel_blockers.clone()
        };
        blocked_candidates.push(blocked_candidate(candidate, reasons));
    }

    for candidate in &projection.blocked {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["graph_blocked".to_string()],
        ));
    }

    let unsafe_ready_candidates = blocked_candidates
        .iter()
        .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe);
    if effective_max_parallel_agents > 1 && unsafe_ready_candidates && selected_lanes.is_empty() {
        blocker_codes.push("ambiguous_unsafe_parallel_candidates".to_string());
        next_actions.push(
            "Some ready candidates are not parallel-safe; reduce to `--lanes 1` or fix execution semantics/conflicts before multi-lane dispatch."
                .to_string(),
        );
    } else if effective_max_parallel_agents > 1 && unsafe_ready_candidates {
        next_actions.push(
            "Some ready candidates are not parallel-safe; they remain blocked candidates and are not selected for this preview."
                .to_string(),
        );
    }
    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| taskflow_contracts::is_selected_lane_runtime_assignment_truth_missing(code))
    {
        selected_lanes.clear();
        blocker_codes.push(
            taskflow_contracts::BlockerCode::SelectedLaneRuntimeAssignmentTruthRequired
                .as_str()
                .to_string(),
        );
        next_actions.push(
            "Selection truth is incomplete for at least one chosen lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    let assignment_guard_blockers = selected_lanes
        .iter()
        .flat_map(|lane| {
            selection_truth_guard_blockers(&lane.selection_truth)
                .into_iter()
                .map(move |blocker| {
                    taskflow_contracts::selected_lane_assignment_guard_blocked(
                        &lane.task_id,
                        &blocker,
                    )
                })
        })
        .collect::<Vec<_>>();
    if !assignment_guard_blockers.is_empty() {
        for blocker in assignment_guard_blockers {
            if !blocker_codes.iter().any(|code| code == &blocker) {
                blocker_codes.push(blocker);
            }
        }
        selected_lanes.clear();
        blocker_codes.push(
            taskflow_contracts::BlockerCode::SelectedLaneAssignmentGuardRequired
                .as_str()
                .to_string(),
        );
        next_actions.push(
            "Selection truth has budget, readiness, or backend blockers; fix assignment guard evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }

    let status = agent_dispatch_status_from_blockers(&blocker_codes);
    let fanout_guard = maybe_build_fanout_guard_from_projection(
        include_diagnostics,
        projection,
        lanes_requested,
        configured_max_parallel_agents,
        &selected_lanes,
        &blocked_candidates,
        &blocker_codes,
    );

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner: maybe_build_parallelization_planner(
            include_diagnostics,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
        ),
        packet_materialization: no_packet_materialization(),
        carrier_selection_api: maybe_build_carrier_selection_api_descriptor(
            include_diagnostics,
            activation_bundle,
        ),
        fanout_guard,
        flow_projection: if include_diagnostics {
            non_dev_team_flow_projection()
        } else {
            compact_diagnostics_omitted("flow_projection")
        },
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn build_agent_dispatch_next_preview_dev_team(
    activation_bundle: &serde_json::Value,
    projection: &state_store::TaskSchedulingProjection,
    lanes_requested: usize,
    configured_max_parallel_agents: usize,
    explicit_state_dir: Option<&std::path::Path>,
    include_diagnostics: bool,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut selected_lanes = Vec::new();
    let mut blocked_candidates = Vec::new();
    let current_task_id = projection.current_task_id.as_deref();
    let current_task_matches = current_task_id
        .map(|current_task_id| {
            projection
                .ready
                .iter()
                .filter(|candidate| candidate.task.id == current_task_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let current_task_missing_from_ready =
        current_task_id.is_some() && current_task_matches.is_empty();
    let current_task_blocked_candidate = current_task_id.and_then(|current_task_id| {
        projection
            .blocked
            .iter()
            .find(|candidate| candidate.task.id == current_task_id)
    });
    let all_ready_flow_ids = projection
        .ready
        .iter()
        .filter(|candidate| candidate.ready_now)
        .filter_map(|candidate| {
            selected_dev_team_flow_for_task(
                &activation_bundle["dev_team_readiness"],
                &candidate.task,
            )
            .and_then(|flow| flow["flow_id"].as_str())
            .map(str::to_string)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let has_unsafe_ready_candidates = projection
        .ready
        .iter()
        .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe);
    let scoped_current_task_dev_team = projection.current_task_id.is_some()
        && current_task_matches.len() == 1
        && (lanes_requested <= 1
            || projection.ready.len() == 1
            || all_ready_flow_ids.len() > 1
            || has_unsafe_ready_candidates);
    let selected_ready_candidates = if scoped_current_task_dev_team {
        current_task_matches
    } else if current_task_missing_from_ready {
        Vec::new()
    } else {
        projection.ready.iter().collect::<Vec<_>>()
    };
    let ready_flow_ids = selected_ready_candidates
        .iter()
        .filter(|candidate| candidate.ready_now)
        .filter_map(|candidate| {
            selected_dev_team_flow_for_task(
                &activation_bundle["dev_team_readiness"],
                &candidate.task,
            )
            .and_then(|flow| flow["flow_id"].as_str())
            .map(str::to_string)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let current_task_absent_from_scheduler =
        current_task_missing_from_ready && current_task_blocked_candidate.is_none();
    let sequence = if current_task_absent_from_scheduler {
        Vec::new()
    } else if current_task_missing_from_ready {
        current_task_blocked_candidate
            .map(|candidate| dev_team_sequence_for_task(activation_bundle, &candidate.task))
            .unwrap_or_else(|| dev_team_sequence(activation_bundle))
    } else if ready_flow_ids.len() == 1 {
        selected_ready_candidates
            .iter()
            .find(|candidate| candidate.ready_now)
            .map(|candidate| dev_team_sequence_for_task(activation_bundle, &candidate.task))
            .unwrap_or_else(|| dev_team_sequence(activation_bundle))
    } else {
        dev_team_sequence(activation_bundle)
    };
    let selected_flow_id = if current_task_missing_from_ready {
        current_task_blocked_candidate
            .and_then(|candidate| {
                selected_dev_team_flow_for_task(
                    &activation_bundle["dev_team_readiness"],
                    &candidate.task,
                )
            })
            .and_then(|flow| flow["flow_id"].as_str())
    } else if ready_flow_ids.len() == 1 {
        ready_flow_ids.iter().next().map(String::as_str)
    } else {
        activation_bundle["dev_team_readiness"]["default_flow_id"].as_str()
    };

    if let Some(current_task_id) = current_task_id.filter(|_| current_task_missing_from_ready) {
        blocker_codes.push(format!(
            "current_task_not_ready_for_dev_team_dispatch:task={current_task_id}"
        ));
        next_actions.push(format!(
            "Current task `{current_task_id}` is not ready for dev-team dispatch; resolve its blockers before dispatching unrelated ready work."
        ));
    }

    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    if sequence.is_empty() && !current_task_absent_from_scheduler {
        blocker_codes.push("configured_dev_team_sequence_required".to_string());
        next_actions.push(
            "Configure dev_team_readiness roles/sequence or dispatch_contract lanes before previewing dev-team dispatch."
                .to_string(),
        );
    }
    if projection.current_task_id.is_none() && ready_flow_ids.len() > 1 {
        blocker_codes.push("ambiguous_work_item_flow_selection".to_string());
        next_actions.push(
            "Ready task candidates map to multiple configured dev_team flows; narrow the task scope or dispatch one flow class at a time."
                .to_string(),
        );
    }

    let configured_max_parallel_agents = configured_max_parallel_agents.max(1);
    let effective_max_parallel_agents = lanes_requested.min(configured_max_parallel_agents);
    let preview_step_limit = lanes_requested;
    let steps_to_preview = sequence
        .iter()
        .cloned()
        .take(preview_step_limit)
        .collect::<Vec<_>>();
    if projection.ready.is_empty() {
        blocker_codes.push("no_ready_task_candidates".to_string());
        next_actions.push(format!(
            "Inspect `{}` and resolve blockers before previewing dev-team dispatch.",
            human_command("vida task ready")
        ));
        for candidate in &projection.blocked {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["graph_blocked".to_string()],
            ));
        }
        let flow_projection = if include_diagnostics && current_task_absent_from_scheduler {
            suppressed_current_task_flow_projection(&blocker_codes)
        } else if include_diagnostics {
            build_dev_team_flow_projection(
                activation_bundle,
                selected_flow_id,
                &sequence,
                &selected_lanes,
                &blocker_codes,
            )
        } else {
            compact_diagnostics_omitted("flow_projection")
        };
        let fanout_guard = maybe_build_fanout_guard_from_projection(
            include_diagnostics,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
            &selected_lanes,
            &blocked_candidates,
            &blocker_codes,
        );
        return AgentDispatchNextPreview {
            status: release1_blocked_status().to_string(),
            mode: "preview-dev-team".to_string(),
            lanes_requested,
            configured_max_parallel_agents,
            effective_max_parallel_agents,
            lanes_selected: 0,
            selected_lanes,
            blocked_candidates,
            blocker_codes,
            next_actions,
            execute_supported: false,
            execution_attempted: false,
            parallelization_planner: maybe_build_parallelization_planner(
                include_diagnostics,
                projection,
                lanes_requested,
                configured_max_parallel_agents,
            ),
            packet_materialization: no_packet_materialization(),
            carrier_selection_api: maybe_build_carrier_selection_api_descriptor(
                include_diagnostics,
                activation_bundle,
            ),
            fanout_guard,
            flow_projection,
            source_surfaces: agent_dispatch_source_surfaces(),
        };
    }

    let mut ready_index = 0;
    for (step_index, step) in steps_to_preview.into_iter().enumerate() {
        if !step.requires_task {
            next_actions.push(format!(
                "dev-team step [{}] {} is closure-oriented and does not emit a runtime launch command.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
            continue;
        }
        if step.requires_user_approval {
            next_actions.push(format!(
                "dev-team step [{}] {} will pause after receipt-backed completion for configured user approval before the next role starts.",
                step_index + 1,
                step.role_label.replace('_', "-")
            ));
        }
        let candidate = if scoped_current_task_dev_team {
            selected_ready_candidates.first().copied()
        } else {
            selected_ready_candidates.get(ready_index).copied()
        };
        if !scoped_current_task_dev_team {
            ready_index += usize::from(candidate.is_some());
        }
        let Some(candidate) = candidate else {
            blocker_codes.push(format!(
                "dev_team_step_missing_ready_task:position={}:{}",
                step_index + 1,
                step.role_label
            ));
            break;
        };
        if !candidate.ready_now {
            blocked_candidates.push(blocked_candidate(
                candidate,
                vec!["task_not_ready_for_dev_team_step".to_string()],
            ));
            continue;
        }
        if projection.current_task_id.is_none()
            && effective_max_parallel_agents > 1
            && !candidate.ready_parallel_safe
        {
            continue;
        }
        match selection_truth_for_task_with_role_and_class(
            activation_bundle,
            &candidate.task,
            &step.runtime_role,
            Some(&step.runtime_role),
            Some(&step.task_class),
        ) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: selected_lanes.len() + 1,
                task_id: candidate.task.id.clone(),
                title: candidate.task.title.clone(),
                role_label: step.role_label.clone(),
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &candidate.task.id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &candidate.task.id,
                ),
                ready_parallel_safe: candidate.ready_parallel_safe,
                selection_reason: format!("dev_team_step_{}:{}", step_index + 1, step.role_label),
                selection_truth,
                requires_user_approval: step.requires_user_approval,
                approval_gate: serde_json::json!({
                    "required": step.requires_user_approval,
                    "status": if step.requires_user_approval {
                        "approval_required_after_step_completion"
                    } else {
                        "not_required"
                    },
                    "policy": step.approval_policy,
                    "lifecycle_hook_templates": step.lifecycle_hook_templates,
                    "resume_transitions": step.resume_transitions,
                    "rework_transitions": step.rework_transitions,
                    "prompt_template_source": if step.requires_user_approval {
                        "dev_team.flows.steps.approval_policy"
                    } else {
                        "none"
                    },
                }),
            }),
            Err(reason) => {
                blocker_codes.push(
                    taskflow_contracts::selected_lane_runtime_assignment_truth_missing(
                        &candidate.task.id,
                        &reason,
                    ),
                );
            }
        }
    }

    let blocked_ready_parallel = projection
        .ready
        .iter()
        .filter(|candidate| {
            Some(candidate.task.id.as_str()) != projection.current_task_id.as_deref()
                && !candidate.ready_parallel_safe
        })
        .collect::<Vec<_>>();
    for candidate in blocked_ready_parallel {
        blocked_candidates.push(blocked_candidate(
            candidate,
            vec!["parallel_safety_not_established".to_string()],
        ));
    }
    for candidate in &projection.blocked {
        let mut reasons = vec!["graph_blocked".to_string()];
        if current_task_missing_from_ready && Some(candidate.task.id.as_str()) == current_task_id {
            reasons.push("current_task_not_ready_for_dev_team_dispatch".to_string());
        }
        blocked_candidates.push(blocked_candidate(candidate, reasons));
    }

    if selected_lanes.is_empty()
        && !blocker_codes
            .iter()
            .any(|code| code == "no_ready_task_candidates")
    {
        blocker_codes.push("no_dispatch_lanes_selected".to_string());
    }
    if blocker_codes
        .iter()
        .any(|code| taskflow_contracts::is_selected_lane_runtime_assignment_truth_missing(code))
    {
        selected_lanes.clear();
        blocker_codes.push(
            taskflow_contracts::BlockerCode::SelectedLaneRuntimeAssignmentTruthRequired
                .as_str()
                .to_string(),
        );
        next_actions.push(
            "Selection truth is incomplete for at least one configured dev-team step; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
        next_actions.push(
            "The shown `vida agent-init --role` command is startup activation view only; receipt-backed execution requires a dispatch packet and `--execute-dispatch`."
                .to_string(),
        );
    }

    let status = agent_dispatch_status_from_blockers(&blocker_codes);
    let flow_projection = if include_diagnostics && current_task_absent_from_scheduler {
        suppressed_current_task_flow_projection(&blocker_codes)
    } else if include_diagnostics {
        build_dev_team_flow_projection(
            activation_bundle,
            selected_flow_id,
            &sequence,
            &selected_lanes,
            &blocker_codes,
        )
    } else {
        compact_diagnostics_omitted("flow_projection")
    };
    let fanout_guard = maybe_build_fanout_guard_from_projection(
        include_diagnostics,
        projection,
        lanes_requested,
        configured_max_parallel_agents,
        &selected_lanes,
        &blocked_candidates,
        &blocker_codes,
    );

    AgentDispatchNextPreview {
        status: status.to_string(),
        mode: "preview-dev-team".to_string(),
        lanes_requested,
        configured_max_parallel_agents,
        effective_max_parallel_agents,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner: maybe_build_parallelization_planner(
            include_diagnostics,
            projection,
            lanes_requested,
            configured_max_parallel_agents,
        ),
        packet_materialization: no_packet_materialization(),
        carrier_selection_api: maybe_build_carrier_selection_api_descriptor(
            include_diagnostics,
            activation_bundle,
        ),
        fanout_guard,
        flow_projection,
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn scheduler_task_record<'a>(
    plan: &'a crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    task_id: &str,
) -> Option<&'a state_store::TaskRecord> {
    plan.scheduling
        .ready
        .iter()
        .chain(plan.scheduling.blocked.iter())
        .find(|candidate| candidate.task.id == task_id)
        .map(|candidate| &candidate.task)
}

fn scheduler_task_parallel_safety(
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    task_id: &str,
) -> bool {
    plan.scheduling
        .ready
        .iter()
        .chain(plan.scheduling.blocked.iter())
        .find(|candidate| candidate.task.id == task_id)
        .is_some_and(|candidate| candidate.ready_parallel_safe)
}

fn build_agent_dispatch_next_preview_from_scheduler_plan(
    activation_bundle: &serde_json::Value,
    plan: crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    lanes_requested: usize,
    explicit_state_dir: Option<&std::path::Path>,
) -> AgentDispatchNextPreview {
    build_agent_dispatch_next_preview_from_scheduler_plan_with_diagnostics(
        activation_bundle,
        plan,
        lanes_requested,
        explicit_state_dir,
        true,
    )
}

fn build_agent_dispatch_next_preview_from_scheduler_plan_with_diagnostics(
    activation_bundle: &serde_json::Value,
    plan: crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
    lanes_requested: usize,
    explicit_state_dir: Option<&std::path::Path>,
    include_diagnostics: bool,
) -> AgentDispatchNextPreview {
    let mut blocker_codes = plan.blocker_codes.clone();
    let mut next_actions = plan.next_actions.clone();
    let mut selected_lanes = Vec::new();
    if lanes_requested == 0 {
        blocker_codes.push("invalid_lanes_requested".to_string());
        next_actions.push("Pass `--lanes <n>` with n >= 1.".to_string());
    }
    let blocked_candidates = plan
        .rejected_candidates
        .iter()
        .map(|candidate| AgentDispatchBlockedCandidate {
            task_id: candidate.task_id.clone(),
            title: candidate.task.title.clone(),
            ready_now: candidate.ready_now,
            ready_parallel_safe: candidate.ready_now && candidate.parallel_blockers.is_empty(),
            reasons: candidate.reasons.clone(),
            parallel_blockers: candidate.parallel_blockers.clone(),
        })
        .collect::<Vec<_>>();

    for (index, reservation) in plan.reservations.iter().enumerate() {
        let Some(task) = scheduler_task_record(&plan, &reservation.task_id) else {
            blocker_codes.push(format!(
                "selected_lane_task_record_missing:task={}",
                reservation.task_id
            ));
            continue;
        };
        match selection_truth_for_task(activation_bundle, task) {
            Ok(selection_truth) => selected_lanes.push(AgentDispatchLanePreview {
                lane_index: index + 1,
                task_id: reservation.task_id.clone(),
                title: reservation.task.title.clone(),
                role_label: if reservation.launch_role == "primary" {
                    "default".to_string()
                } else {
                    reservation.launch_role.clone()
                },
                runtime_role: selection_truth.runtime_role.clone(),
                task_class: selection_truth.task_class.clone(),
                dispatch_command: agent_init_command(
                    &reservation.task_id,
                    explicit_state_dir,
                    &selection_truth.runtime_role,
                ),
                dispatch_command_kind: "startup_activation_view_only".to_string(),
                receipt_backed_execution_command: receipt_backed_execution_command_hint(
                    &reservation.task_id,
                ),
                ready_parallel_safe: scheduler_task_parallel_safety(&plan, &reservation.task_id),
                selection_reason: if reservation.launch_role == "primary" {
                    "scheduler_primary_ready_task".to_string()
                } else {
                    "scheduler_parallel_safe_ready_task".to_string()
                },
                selection_truth,
                requires_user_approval: false,
                approval_gate: serde_json::json!({"required": false, "status": "not_required"}),
            }),
            Err(reason) => {
                blocker_codes.push(
                    taskflow_contracts::selected_lane_runtime_assignment_truth_missing(
                        &reservation.task_id,
                        &reason,
                    ),
                );
            }
        }
    }

    if blocker_codes
        .iter()
        .any(|code| taskflow_contracts::is_selected_lane_runtime_assignment_truth_missing(code))
        || blocker_codes
            .iter()
            .any(|code| code.starts_with("selected_lane_task_record_missing:"))
    {
        selected_lanes.clear();
        blocker_codes.push(
            taskflow_contracts::BlockerCode::SelectedLaneRuntimeAssignmentTruthRequired
                .as_str()
                .to_string(),
        );
        next_actions.push(
            "Selection truth is incomplete for at least one scheduler-selected lane; fix runtime assignment evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    let assignment_guard_blockers = selected_lanes
        .iter()
        .flat_map(|lane| {
            selection_truth_guard_blockers(&lane.selection_truth)
                .into_iter()
                .map(move |blocker| {
                    taskflow_contracts::selected_lane_assignment_guard_blocked(
                        &lane.task_id,
                        &blocker,
                    )
                })
        })
        .collect::<Vec<_>>();
    if !assignment_guard_blockers.is_empty() {
        for blocker in assignment_guard_blockers {
            if !blocker_codes.iter().any(|code| code == &blocker) {
                blocker_codes.push(blocker);
            }
        }
        selected_lanes.clear();
        blocker_codes.push(
            taskflow_contracts::BlockerCode::SelectedLaneAssignmentGuardRequired
                .as_str()
                .to_string(),
        );
        next_actions.push(
            "Selection truth has budget, readiness, or backend blockers; fix assignment guard evidence before launching `vida agent-init`."
                .to_string(),
        );
    }
    if plan.max_parallel_agents > 1
        && blocked_candidates
            .iter()
            .any(|candidate| candidate.ready_now && !candidate.ready_parallel_safe)
    {
        next_actions.push(
            "Some ready candidates are not parallel-safe; they remain blocked candidates and are not selected for this preview."
                .to_string(),
        );
    }
    if !selected_lanes.is_empty() {
        next_actions.push(
            "Preview only: review the selected carrier/model/cost truth first; run the shown `vida agent-init` command only after operator review."
                .to_string(),
        );
    }
    if lanes_requested == 0 {
        selected_lanes.clear();
    }

    let status = agent_dispatch_status_from_blockers(&blocker_codes).to_string();
    let configured_parallel =
        usize::try_from(plan.configured_max_parallel_agents).unwrap_or(usize::MAX);
    let effective_parallel = if lanes_requested == 0 {
        0
    } else {
        usize::try_from(plan.max_parallel_agents).unwrap_or(usize::MAX)
    };
    let mut parallelization_planner = maybe_build_parallelization_planner(
        include_diagnostics,
        &plan.scheduling,
        lanes_requested,
        effective_parallel,
    );
    if include_diagnostics {
        apply_scheduler_plan_continuation_gate_to_parallelization_planner(
            &mut parallelization_planner,
            &plan,
        );
    }
    let fanout_guard = if include_diagnostics {
        agent_dispatch_fanout_guard_from_scheduler_plan(
            &plan,
            &selected_lanes,
            &blocked_candidates,
            &blocker_codes,
        )
    } else {
        compact_diagnostics_omitted("fanout_guard")
    };
    AgentDispatchNextPreview {
        status,
        mode: "preview".to_string(),
        lanes_requested,
        configured_max_parallel_agents: configured_parallel,
        effective_max_parallel_agents: effective_parallel,
        lanes_selected: selected_lanes.len(),
        selected_lanes,
        blocked_candidates,
        blocker_codes,
        next_actions,
        execute_supported: false,
        execution_attempted: false,
        parallelization_planner,
        packet_materialization: no_packet_materialization(),
        carrier_selection_api: maybe_build_carrier_selection_api_descriptor(
            include_diagnostics,
            activation_bundle,
        ),
        fanout_guard,
        flow_projection: if include_diagnostics {
            non_dev_team_flow_projection()
        } else {
            compact_diagnostics_omitted("flow_projection")
        },
        source_surfaces: agent_dispatch_source_surfaces(),
    }
}

fn apply_scheduler_plan_continuation_gate_to_parallelization_planner(
    planner: &mut serde_json::Value,
    plan: &crate::taskflow_proxy::TaskflowSchedulerDispatchPlan,
) {
    let blocked_by_continuation_gate = plan.selected_task_ids.is_empty()
        && plan.blocker_codes.iter().any(|code| {
            matches!(
                code.as_str(),
                "continuation_binding_ambiguous"
                    | "open_delegated_cycle"
                    | "latest_run_graph_status_blocked"
            )
        });
    if !blocked_by_continuation_gate {
        return;
    }

    let proposals = plan
        .selected_parallel_tasks
        .iter()
        .map(|task| {
            serde_json::json!({
                "task_id": task.id,
                "title": task.title,
                "proposal_kind": "parallel_safe_dispatch_packet_preview",
                "materializes_packet": false,
                "next_surface": "vida agent-init",
                "reason": "candidate remains visible as diagnostic-only evidence while continuation gate blocks execution"
            })
        })
        .collect::<Vec<_>>();
    if let Some(object) = planner.as_object_mut() {
        object.insert(
            "status".to_string(),
            serde_json::json!(if proposals.is_empty() {
                "no_packet_proposals"
            } else {
                "proposals_available"
            }),
        );
        object.insert("packet_proposals".to_string(), serde_json::json!(proposals));
        object.insert("materializes_packets".to_string(), serde_json::json!(false));
        object.insert("diagnostic_only".to_string(), serde_json::json!(true));
        object.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        object.insert(
            "continuation_gate_scope".to_string(),
            serde_json::json!("task_scoped"),
        );
        object.insert(
            "independent_parallel_available".to_string(),
            serde_json::json!(!plan.selected_parallel_tasks.is_empty()),
        );
    }
}

fn apply_continuation_dispatch_gate_to_preview(
    preview: &mut AgentDispatchNextPreview,
    gate: &crate::taskflow_proxy::TaskflowContinuationDispatchGate,
) {
    if gate.admissible {
        return;
    }

    let blocked_task_ids = gate
        .blocked_task_ids
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>();

    preview.status = release1_blocked_status().to_string();
    preview.selected_lanes.clear();
    preview.lanes_selected = 0;
    if let Some(blocker) = crate::release1_contracts::blocker_code_value(
        crate::release1_contracts::BlockerCode::LatestRunGraphStatusBlocked,
    ) {
        if !preview.blocker_codes.iter().any(|value| value == &blocker) {
            preview.blocker_codes.push(blocker);
        }
    }
    for blocker in &gate.blocker_codes {
        if !preview.blocker_codes.iter().any(|value| value == blocker) {
            preview.blocker_codes.push(blocker.clone());
        }
    }
    preview.next_actions.clear();
    for action in &gate.next_actions {
        if !preview.next_actions.iter().any(|value| value == action) {
            preview.next_actions.push(action.clone());
        }
    }
    if preview.next_actions.is_empty() {
        preview.next_actions.push(
            crate::status_surface_signals::continuation_binding_ambiguous_next_action().to_string(),
        );
    }
    fail_closed_flow_projection_for_continuation_gate(preview);
    if let Some(planner) = preview.parallelization_planner.as_object_mut() {
        let mut proposals_available = false;
        if !blocked_task_ids.is_empty() {
            if let Some(proposals) = planner
                .get_mut("packet_proposals")
                .and_then(serde_json::Value::as_array_mut)
            {
                proposals.retain(|proposal| {
                    proposal
                        .get("task_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .is_some_and(|task_id| !blocked_task_ids.contains(task_id))
                });
                proposals_available = !proposals.is_empty();
            }
            planner.insert(
                "continuation_gate_blocked_task_ids".to_string(),
                serde_json::json!(blocked_task_ids.iter().cloned().collect::<Vec<_>>()),
            );
        } else {
            proposals_available = planner
                .get("packet_proposals")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|proposals| !proposals.is_empty());
        }
        planner.insert(
            "status".to_string(),
            serde_json::json!(if proposals_available {
                "proposals_available"
            } else {
                "no_packet_proposals"
            }),
        );
        planner.insert("materializes_packets".to_string(), serde_json::json!(false));
        planner.insert("diagnostic_only".to_string(), serde_json::json!(true));
        planner.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        planner.insert(
            "continuation_gate_scope".to_string(),
            serde_json::json!(if blocked_task_ids.is_empty() {
                "global"
            } else {
                "task_scoped"
            }),
        );
        planner.insert(
            "independent_parallel_available".to_string(),
            serde_json::json!(proposals_available),
        );
    }
}

fn fail_closed_flow_projection_for_continuation_gate(preview: &mut AgentDispatchNextPreview) {
    let blocked_proof_state = serde_json::json!({
        "status": "blocked_by_continuation_gate",
        "diagnostic_only": true
    });
    if let Some(flow_projection) = preview.flow_projection.as_object_mut() {
        flow_projection.insert(
            "status".to_string(),
            serde_json::json!(release1_blocked_status()),
        );
        flow_projection.insert(
            "blocked_by_continuation_gate".to_string(),
            serde_json::json!(true),
        );
        flow_projection.insert(
            "blocker_codes".to_string(),
            serde_json::json!(preview.blocker_codes),
        );
        flow_projection.insert(
            "next_actions".to_string(),
            serde_json::json!(preview.next_actions),
        );
        flow_projection.insert("proof_state".to_string(), blocked_proof_state.clone());
        if let Some(current_step) = flow_projection
            .get_mut("current_step")
            .and_then(serde_json::Value::as_object_mut)
        {
            current_step.insert("dispatch_command".to_string(), serde_json::Value::Null);
            current_step.insert("dispatch_command_kind".to_string(), serde_json::Value::Null);
            current_step.insert("proof_state".to_string(), blocked_proof_state);
            current_step.insert(
                "blocked_by_continuation_gate".to_string(),
                serde_json::json!(true),
            );
        }
    }
}

fn dispatch_target_for_agent_dispatch_lane(lane: &AgentDispatchLanePreview) -> &str {
    lane.role_label.as_str()
}

fn validate_materialized_agent_dispatch_packet(
    lane: &AgentDispatchLanePreview,
    expected_dispatch_target: &str,
    dispatch_packet_path: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<serde_json::Value, String> {
    let packet_path = Path::new(dispatch_packet_path);
    if dispatch_packet_path.trim().is_empty() {
        return Err("dispatch packet path is empty".to_string());
    }
    let packet =
        read_canonical_host_bridge_json_artifact(packet_path, "materialized dispatch packet")?;
    if packet["run_id"].as_str() != Some(lane.task_id.as_str()) {
        return Err(format!(
            "materialized packet run_id mismatch: expected `{}`, got `{}`",
            lane.task_id,
            packet["run_id"].as_str().unwrap_or("<missing>")
        ));
    }
    if packet["dispatch_target"].as_str() != Some(expected_dispatch_target) {
        return Err(format!(
            "materialized packet dispatch_target mismatch: expected `{expected_dispatch_target}`, got `{}`",
            packet["dispatch_target"].as_str().unwrap_or("<missing>")
        ));
    }
    if receipt.run_id != lane.task_id {
        return Err(format!(
            "dispatch receipt run_id mismatch: expected `{}`, got `{}`",
            lane.task_id, receipt.run_id
        ));
    }
    if receipt.dispatch_target != expected_dispatch_target {
        return Err(format!(
            "dispatch receipt target mismatch: expected `{expected_dispatch_target}`, got `{}`",
            receipt.dispatch_target
        ));
    }
    if receipt.dispatch_status != "routed" {
        return Err(format!(
            "dispatch receipt is not routed: status `{}`",
            receipt.dispatch_status
        ));
    }
    Ok(packet)
}

fn apply_configured_lane_runtime_assignment(
    role_selection: &mut crate::RuntimeConsumptionLaneSelection,
    activation_bundle: &serde_json::Value,
    lane: &AgentDispatchLanePreview,
) -> Result<(), String> {
    let conversation_role = role_selection
        .fallback_role
        .trim()
        .is_empty()
        .then_some("orchestrator")
        .unwrap_or(role_selection.fallback_role.as_str());
    let assignment = crate::build_runtime_assignment_from_resolved_constraints(
        activation_bundle,
        conversation_role,
        &lane.task_class,
        &lane.runtime_role,
    );
    if !assignment["enabled"].as_bool().unwrap_or(false) {
        let reason = assignment["reason"]
            .as_str()
            .unwrap_or("runtime_assignment_disabled");
        return Err(format!(
            "configured lane `{}` assignment disabled for runtime_role `{}` task_class `{}`: {reason}",
            lane.role_label, lane.runtime_role, lane.task_class
        ));
    }
    let execution_plan = role_selection
        .execution_plan
        .as_object_mut()
        .ok_or_else(|| "configured lane execution_plan is not an object".to_string())?;
    execution_plan.extend(crate::runtime_assignment_alias_fields(&assignment));
    Ok(())
}

async fn materialize_configured_agent_dispatch_lane(
    lane: &AgentDispatchLanePreview,
    state_dir: &Path,
    activation_bundle: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let expected_dispatch_target = dispatch_target_for_agent_dispatch_lane(lane);
    let mut role_selection = crate::RuntimeConsumptionLaneSelection {
        ok: true,
        activation_source: "vida.config.yaml".to_string(),
        selection_mode: "configured_dev_team_dispatch_next".to_string(),
        fallback_role: "orchestrator".to_string(),
        request: lane.task_id.clone(),
        selected_role: lane.runtime_role.clone(),
        conversational_mode: None,
        single_task_only: true,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "explicit_configured_dev_team_lane".to_string(),
        matched_terms: vec![lane.role_label.clone(), lane.task_class.clone()],
        compiled_bundle: activation_bundle.clone(),
        execution_plan: serde_json::Value::Null,
        reason: format!(
            "materialize configured dev-team lane `{}` as `{expected_dispatch_target}`",
            lane.role_label
        ),
    };
    role_selection.execution_plan =
        crate::development_flow_orchestration::build_runtime_execution_plan_from_snapshot(
            activation_bundle,
            &role_selection,
        );
    apply_configured_lane_runtime_assignment(&mut role_selection, activation_bundle, lane)?;
    let run_graph_bootstrap = serde_json::json!({
        "status": "dispatch_init_ready",
        "handoff_ready": true,
        "run_id": lane.task_id,
        "latest_status": {
            "run_id": lane.task_id,
            "status": release1_pass_status(),
            "active_node": expected_dispatch_target,
            "next_node": expected_dispatch_target,
            "task_class": lane.task_class,
            "route_task_class": lane.task_class,
            "dispatch_ready": true,
            "dispatch_blockers": [],
        }
    });
    let taskflow_handoff_plan = crate::build_taskflow_handoff_plan(&role_selection);
    let mut dispatch_receipt = crate::taskflow_consume::build_runtime_consumption_dispatch_receipt(
        &role_selection,
        &run_graph_bootstrap,
    );
    crate::runtime_dispatch_state::sync_receipt_configured_activation_assignment(
        &role_selection,
        &mut dispatch_receipt,
    );
    dispatch_receipt.dispatch_command =
        crate::runtime_dispatch_command_for_target(&role_selection, expected_dispatch_target);
    let owned_paths_override =
        match StateStore::open_existing_read_only(state_dir.to_path_buf()).await {
            Ok(store) => {
                let owned_paths = store
                    .show_task(&lane.task_id)
                    .await
                    .ok()
                    .map(|task| materialization_owned_paths_for_lane_task(task, lane))
                    .unwrap_or_default();
                store.close().await;
                owned_paths
            }
            Err(_) => Vec::new(),
        };
    let ctx = crate::RuntimeDispatchPacketContext::new(
        state_dir,
        &role_selection,
        &dispatch_receipt,
        &taskflow_handoff_plan,
        &run_graph_bootstrap,
    )
    .with_owned_paths_override(owned_paths_override);
    let dispatch_packet_path = crate::write_runtime_dispatch_packet(&ctx)?;
    dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
    let store = StateStore::open_existing(state_dir.to_path_buf())
        .await
        .map_err(|error| {
            format!("Failed to open state store to record dev-team dispatch receipt: {error}")
        })?;
    store
        .record_run_graph_dispatch_receipt(&dispatch_receipt)
        .await
        .map_err(|error| format!("Failed to record dev-team dispatch receipt: {error}"))?;
    store
        .record_run_graph_dispatch_lane_receipt(&dispatch_receipt)
        .await
        .map_err(|error| format!("Failed to record dev-team dispatch lane receipt: {error}"))?;
    store.close().await;
    let packet = validate_materialized_agent_dispatch_packet(
        lane,
        expected_dispatch_target,
        &dispatch_packet_path,
        &dispatch_receipt,
    )?;
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            crate::runtime_dispatch_state::runtime_dispatch_packet_kind(
                &role_selection.execution_plan,
                expected_dispatch_target,
                &dispatch_receipt.dispatch_kind,
            )
        });
    let mut agent_init_execute_command = format!(
        "vida agent-init --dispatch-packet {} --execute-dispatch",
        crate::shell_quote(&dispatch_packet_path)
    );
    agent_init_execute_command.push_str(" --state-dir ");
    agent_init_execute_command.push_str(&crate::shell_quote(&state_dir.display().to_string()));
    Ok(serde_json::json!({
        "lane_index": lane.lane_index,
        "task_id": lane.task_id,
        "role_label": lane.role_label,
        "runtime_role": lane.runtime_role,
        "task_class": lane.task_class,
        "dispatch_packet_path": dispatch_packet_path,
        "dispatch_target": expected_dispatch_target,
        "packet_template_kind": packet_template_kind,
        "dispatch_receipt_id": dispatch_receipt.recorded_at,
        "dispatch_receipt": dispatch_receipt,
        "agent_init_execute_command": agent_init_execute_command,
        "machine_command": format!("{agent_init_execute_command} --json"),
        "receipt_backed": true,
        "status": "packet_ready",
    }))
}

async fn materialize_agent_dispatch_next_packets(
    mut preview: AgentDispatchNextPreview,
    state_dir: &std::path::Path,
    activation_bundle: &serde_json::Value,
) -> AgentDispatchNextPreview {
    if preview.status != release1_pass_status() {
        preview.packet_materialization = serde_json::json!({
            "status": release1_blocked_status(),
            "requested": true,
            "materializes_packets": false,
            "reason": "dispatch preview is blocked",
            "blocker_codes": preview.blocker_codes,
            "artifacts": [],
        });
        return preview;
    }
    if preview.selected_lanes.is_empty() {
        preview.status = release1_blocked_status().to_string();
        preview
            .blocker_codes
            .push("no_dispatch_lanes_selected".to_string());
        preview.packet_materialization = serde_json::json!({
            "status": release1_blocked_status(),
            "requested": true,
            "materializes_packets": false,
            "reason": "no selected lanes can be materialized",
            "artifacts": [],
        });
        return preview;
    }

    let materialization_lanes = agent_dispatch_materialization_lanes(&preview);
    let preflight_errors =
        preflight_agent_dispatch_next_packet_materialization(&materialization_lanes, state_dir)
            .await;
    if !preflight_errors.is_empty() {
        preview.status = release1_blocked_status().to_string();
        if !preview
            .blocker_codes
            .iter()
            .any(|value| value == "dispatch_packet_contract_invalid")
        {
            preview
                .blocker_codes
                .push("dispatch_packet_contract_invalid".to_string());
        }
        preview.next_actions.clear();
        preview.next_actions.push(
            "Add the missing TaskFlow planner_metadata.owned_paths or reshape the selected dev-team lane before materializing dispatch packets."
                .to_string(),
        );
        if let Some(planner) = preview.parallelization_planner.as_object_mut() {
            planner.insert(
                "mode".to_string(),
                serde_json::json!("blocked_packet_contract"),
            );
            planner.insert("materializes_packets".to_string(), serde_json::json!(false));
            planner.insert("packet_artifacts".to_string(), serde_json::json!([]));
        }
        if let Some(flow_projection) = preview.flow_projection.as_object_mut() {
            flow_projection.insert(
                "status".to_string(),
                serde_json::json!(release1_blocked_status()),
            );
            flow_projection.insert(
                "receipt_status".to_string(),
                serde_json::json!({
                    "receipt_backed": false,
                    "status": "blocked_packet_contract",
                }),
            );
            flow_projection.insert(
                "proof_state".to_string(),
                serde_json::json!({
                    "status": "blocked_packet_contract",
                    "diagnostic_only": false,
                }),
            );
            flow_projection.insert(
                "blocker_codes".to_string(),
                serde_json::json!(preview.blocker_codes),
            );
            flow_projection.insert(
                "next_actions".to_string(),
                serde_json::json!(preview.next_actions),
            );
        }
        preview.packet_materialization = serde_json::json!({
            "status": release1_blocked_status(),
            "requested": true,
            "materializes_packets": false,
            "errors": preflight_errors,
            "artifacts": [],
        });
        return preview;
    }

    let mut artifacts = Vec::new();
    let mut errors = Vec::new();
    for lane in &materialization_lanes {
        match materialize_configured_agent_dispatch_lane(lane, state_dir, activation_bundle).await {
            Ok(artifact) => artifacts.push(artifact),
            Err(error) => {
                let blocker = format!("packet_materialization_failed:task={}", lane.task_id);
                if !preview.blocker_codes.iter().any(|value| value == &blocker) {
                    preview.blocker_codes.push(blocker);
                }
                errors.push(serde_json::json!({
                    "task_id": lane.task_id,
                    "role_label": lane.role_label,
                    "blocker_code": "packet_materialization_failed",
                    "blocker_codes": ["packet_materialization_failed"],
                    "missing_fields": [],
                    "next_actions": [
                        "Inspect the packet materialization error and repair the selected lane before retrying dispatch packet materialization."
                    ],
                    "error": error,
                }));
            }
        }
    }

    if errors.is_empty() {
        preview.mode = if preview.mode == "preview-dev-team" {
            "materialized-dev-team".to_string()
        } else {
            "materialized".to_string()
        };
        preview.next_actions.retain(|action| {
            !action.contains("Preview only:") && !action.contains("startup activation view only")
        });
        if let Some(first) = artifacts
            .first()
            .and_then(|artifact| artifact["agent_init_execute_command"].as_str())
        {
            preview.next_actions.push(format!(
                "Run `{first}` to execute the first receipt-backed dispatch packet."
            ));
        }
        if let Some(planner) = preview.parallelization_planner.as_object_mut() {
            planner.insert(
                "mode".to_string(),
                serde_json::json!("materialized_packets"),
            );
            planner.insert("materializes_packets".to_string(), serde_json::json!(true));
            planner.insert(
                "packet_artifacts".to_string(),
                serde_json::json!(artifacts.clone()),
            );
        }
        if let Some(flow_projection) = preview.flow_projection.as_object_mut() {
            flow_projection.insert("diagnostic_only".to_string(), serde_json::json!(false));
            flow_projection.insert(
                "receipt_status".to_string(),
                serde_json::json!({
                    "receipt_backed": true,
                    "status": "packet_ready",
                    "artifacts": artifacts.clone(),
                }),
            );
            let current_step_value = flow_projection
                .entry("current_step".to_string())
                .or_insert_with(|| serde_json::json!({}));
            if let Some(current_step) = current_step_value.as_object_mut() {
                if let Some(first) = artifacts.first() {
                    current_step.insert(
                        "dispatch_command".to_string(),
                        first["agent_init_execute_command"].clone(),
                    );
                    current_step.insert(
                        "dispatch_command_kind".to_string(),
                        serde_json::json!("receipt_backed_dispatch_packet"),
                    );
                    current_step.insert(
                        "receipt_status".to_string(),
                        serde_json::json!({
                            "receipt_backed": true,
                            "receipt_path": first["dispatch_packet_path"],
                            "status": "packet_ready",
                        }),
                    );
                    current_step.insert(
                        "proof_state".to_string(),
                        serde_json::json!({
                            "status": "pending_receipt_backed_execution",
                            "diagnostic_only": false,
                        }),
                    );
                }
            }
        }
        preview.packet_materialization = serde_json::json!({
            "status": release1_pass_status(),
            "requested": true,
            "materializes_packets": true,
            "selected_lane_count": preview.selected_lanes.len(),
            "materialized_lane_count": artifacts.len(),
            "sequential_dev_team_first_packet_only": preview.mode == "materialized-dev-team",
            "artifacts": artifacts,
        });
    } else {
        preview.status = release1_blocked_status().to_string();
        preview.packet_materialization = serde_json::json!({
            "status": release1_blocked_status(),
            "requested": true,
            "materializes_packets": false,
            "errors": errors,
            "artifacts": artifacts,
        });
    }
    preview
}

fn agent_dispatch_materialization_lanes(
    preview: &AgentDispatchNextPreview,
) -> Vec<AgentDispatchLanePreview> {
    let lane_limit = if preview.mode == "preview-dev-team" {
        1
    } else {
        preview.selected_lanes.len()
    };
    preview
        .selected_lanes
        .iter()
        .take(lane_limit)
        .cloned()
        .collect()
}

fn agent_dispatch_next_projection_name(
    command: &AgentDispatchNextArgs,
    materialize_packets: bool,
) -> String {
    let materialization_mode = if materialize_packets {
        "-materialized"
    } else {
        ""
    };
    let output_mode = if command.full { "-full" } else { "-compact" };
    format!(
        "agent-dispatch-next-mode-{}{}{}-lanes-{}-scope-{}-current-{}-latest",
        if command.dev_team {
            "dev-team"
        } else {
            "scheduler"
        },
        materialization_mode,
        output_mode,
        command.lanes,
        crate::operator_projection_cache::sanitize_projection_component(
            command.scope.as_deref().unwrap_or("default"),
            "none",
            120,
        ),
        crate::operator_projection_cache::sanitize_projection_component(
            command.current_task_id.as_deref().unwrap_or("default"),
            "none",
            120,
        ),
    )
}

fn agent_dispatch_next_effective_materialize_packets(
    command: &AgentDispatchNextArgs,
    activation_bundle: &serde_json::Value,
) -> bool {
    command.materialize_packets
        || (command.dev_team
            && activation_bundle_default_args_include(activation_bundle, "--materialize-packets"))
}

fn activation_bundle_default_args_include(
    activation_bundle: &serde_json::Value,
    expected_arg: &str,
) -> bool {
    ["dev_team", "dev_team_readiness"]
        .into_iter()
        .any(|section| {
            activation_bundle
                .get(section)
                .and_then(|value| value.get("orchestrator_command_contract"))
                .and_then(|value| value.get("default_args"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|args| {
                    args.iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::trim)
                        .any(|arg| arg == expected_arg)
                })
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AgentDispatchNextCurrentTaskIds<'a> {
    preview_current_task_id: Option<&'a str>,
    scheduler_current_task_id: Option<&'a str>,
}

fn resolve_agent_dispatch_next_current_task_ids<'a>(
    requested_current_task_id: Option<&'a str>,
    explicit_bound_current_task_id: Option<&'a str>,
    taskflow_single_in_progress_task_id: Option<&'a str>,
) -> AgentDispatchNextCurrentTaskIds<'a> {
    AgentDispatchNextCurrentTaskIds {
        preview_current_task_id: requested_current_task_id
            .or(explicit_bound_current_task_id)
            .or(taskflow_single_in_progress_task_id),
        scheduler_current_task_id: requested_current_task_id
            .or(explicit_bound_current_task_id)
            .or(taskflow_single_in_progress_task_id),
    }
}

fn agent_dispatch_next_summary_task_id(summary: &serde_json::Value) -> Option<&str> {
    if summary.get("continuation_allowed") != Some(&serde_json::Value::Bool(true)) {
        return None;
    }
    let active_bounded_unit = summary.get("active_bounded_unit")?;
    if !matches!(
        active_bounded_unit
            .get("kind")
            .and_then(serde_json::Value::as_str),
        Some("task_graph_task" | "run_graph_task")
    ) {
        return None;
    }
    active_bounded_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
}

fn agent_dispatch_next_bound_current_task_id(
    binding: Option<&state_store::RunGraphContinuationBinding>,
    latest_status: Option<&state_store::RunGraphStatus>,
    latest_dispatch_receipt: Option<&state_store::RunGraphDispatchReceiptSummary>,
) -> Option<String> {
    if let Some(task_id) =
        crate::continuation_binding_summary::explicit_task_graph_continuation_task_id(binding)
    {
        return Some(task_id.to_string());
    }
    let summary = crate::continuation_binding_summary::build_continuation_binding_summary(
        binding,
        latest_status,
        None,
        latest_dispatch_receipt,
        None,
        false,
    );
    agent_dispatch_next_summary_task_id(&summary).map(str::to_string)
}

fn agent_dispatch_next_preserve_current_task_id(
    projection: &mut state_store::TaskSchedulingProjection,
    current_task_id: Option<&str>,
) {
    let Some(current_task_id) = current_task_id
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
    else {
        return;
    };
    projection.current_task_id = Some(current_task_id.to_string());
}

fn compact_agent_dispatch_packet_materialization(value: &serde_json::Value) -> serde_json::Value {
    let artifacts = value
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|artifact| {
                    serde_json::json!({
                        "task_id": artifact.get("task_id").cloned().unwrap_or(serde_json::Value::Null),
                        "role_label": artifact.get("role_label").cloned().unwrap_or(serde_json::Value::Null),
                        "dispatch_target": artifact.get("dispatch_target").cloned().unwrap_or(serde_json::Value::Null),
                        "packet_template_kind": artifact.get("packet_template_kind").cloned().unwrap_or(serde_json::Value::Null),
                        "dispatch_packet_path": artifact.get("dispatch_packet_path").cloned().unwrap_or(serde_json::Value::Null),
                        "agent_init_execute_command": artifact.get("agent_init_execute_command").cloned().unwrap_or(serde_json::Value::Null),
                        "receipt_backed": artifact.get("receipt_backed").cloned().unwrap_or(serde_json::Value::Null),
                        "status": artifact.get("status").cloned().unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let errors = value
        .get("errors")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|error| {
                    serde_json::json!({
                        "task_id": error.get("task_id").cloned().unwrap_or(serde_json::Value::Null),
                        "role_label": error.get("role_label").cloned().unwrap_or(serde_json::Value::Null),
                        "blocker_code": error.get("blocker_code").cloned().unwrap_or(serde_json::Value::Null),
                        "blocker_codes": error.get("blocker_codes").cloned().unwrap_or(serde_json::json!([])),
                        "missing_fields": error.get("missing_fields").cloned().unwrap_or(serde_json::json!([])),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({
        "status": value.get("status").cloned().unwrap_or(serde_json::Value::Null),
        "requested": value.get("requested").cloned().unwrap_or(serde_json::Value::Bool(false)),
        "materializes_packets": value.get("materializes_packets").cloned().unwrap_or(serde_json::Value::Bool(false)),
        "artifact_count": artifacts.len(),
        "artifacts": artifacts,
        "errors": errors,
        "reason": value.get("reason").cloned().unwrap_or(serde_json::Value::Null),
        "blocker_codes": value.get("blocker_codes").cloned().unwrap_or(serde_json::json!([])),
    })
}

fn agent_dispatch_next_compact_payload(preview: &AgentDispatchNextPreview) -> serde_json::Value {
    let selected_lanes = preview
        .selected_lanes
        .iter()
        .map(|lane| {
            serde_json::json!({
                "lane_index": lane.lane_index,
                "task_id": &lane.task_id,
                "title": &lane.title,
                "role_label": &lane.role_label,
                "runtime_role": &lane.runtime_role,
                "task_class": &lane.task_class,
                "dispatch_command": &lane.dispatch_command,
                "dispatch_command_kind": &lane.dispatch_command_kind,
                "receipt_backed_execution_command": &lane.receipt_backed_execution_command,
                "ready_parallel_safe": lane.ready_parallel_safe,
                "selection_reason": &lane.selection_reason,
                "requires_user_approval": lane.requires_user_approval,
                "selected_carrier": &lane.selection_truth.selected_carrier,
                "selected_backend": &lane.selection_truth.selected_backend,
                "selected_model_ref": &lane.selection_truth.selected_model_ref,
                "selected_reasoning_effort": &lane.selection_truth.selected_reasoning_effort,
                "rate": lane.selection_truth.rate,
                "estimated_task_price_units": lane.selection_truth.estimated_task_price_units,
                "budget_verdict": &lane.selection_truth.budget_verdict,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": &preview.status,
        "mode": &preview.mode,
        "lanes_requested": preview.lanes_requested,
        "configured_max_parallel_agents": preview.configured_max_parallel_agents,
        "effective_max_parallel_agents": preview.effective_max_parallel_agents,
        "lanes_selected": preview.lanes_selected,
        "selected_lanes": selected_lanes,
        "blocked_candidate_count": preview.blocked_candidates.len(),
        "blocker_codes": &preview.blocker_codes,
        "next_actions": &preview.next_actions,
        "execute_supported": preview.execute_supported,
        "execution_attempted": preview.execution_attempted,
        "flow_projection": {
            "current_step": preview.flow_projection
                .get("current_step")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            "receipt_status": preview.flow_projection
                .get("receipt_status")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        },
        "packet_materialization": compact_agent_dispatch_packet_materialization(&preview.packet_materialization),
        "source_surfaces": &preview.source_surfaces,
        "output_contract": {
            "view": "compact",
            "full_output_flag": "--full",
            "full_output_requires_json": true,
            "full_output_note": "Use --full --json to include blocked_candidates, parallelization_planner, carrier_selection_api, fanout_guard, and flow_projection diagnostics."
        },
    })
}

fn agent_dispatch_existing_packet_fast_path_payload(
    command: &AgentDispatchNextArgs,
    state_dir: &std::path::Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<serde_json::Value> {
    if command.full || !command.materialize_packets {
        return None;
    }
    let current_task_id = command.current_task_id.as_deref()?;
    if !crate::runtime_dispatch_receipt_helpers::dispatch_receipt_has_clean_routed_agent_handoff(
        receipt,
        Some(current_task_id),
    ) {
        return None;
    }
    let packet_path = receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if !std::path::Path::new(packet_path).is_file() {
        return None;
    }
    let packet = read_canonical_host_bridge_json_artifact(
        std::path::Path::new(packet_path),
        "existing dispatch packet",
    )
    .ok()?;
    let packet_template_kind = packet
        .get("packet_template_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "delivery_task_packet".to_string());
    let runtime_role = packet_handoff_runtime_role(&packet)
        .or_else(|| receipt.activation_runtime_role.clone())
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null);
    let task_class = packet_handoff_task_class(&packet)
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null);
    let mut execute_command =
        crate::continuation_binding_summary::routed_dispatch_command_from_parts(
            receipt.dispatch_command.as_deref(),
            Some(packet_path),
        )?;
    if !execute_command.contains(" --state-dir ") {
        execute_command.push_str(" --state-dir ");
        execute_command.push_str(&crate::shell_quote(&state_dir.display().to_string()));
    }
    Some(serde_json::json!({
        "status": release1_pass_status(),
        "mode": if command.dev_team { "materialized-dev-team" } else { "materialized" },
        "lanes_requested": command.lanes,
        "configured_max_parallel_agents": serde_json::Value::Null,
        "effective_max_parallel_agents": 1,
        "lanes_selected": 1,
        "selected_lanes": [
            {
                "lane_index": 1,
                "task_id": receipt.run_id,
                "title": serde_json::Value::Null,
                "role_label": receipt.dispatch_target,
                "runtime_role": runtime_role,
                "task_class": task_class,
                "dispatch_command": receipt.dispatch_command,
                "dispatch_command_kind": "receipt_backed_dispatch_packet",
                "receipt_backed_execution_command": execute_command,
                "ready_parallel_safe": true,
                "selection_reason": "existing_receipt_backed_dispatch_packet",
                "requires_user_approval": false,
                "selected_carrier": serde_json::Value::Null,
                "selected_backend": receipt.selected_backend,
                "selected_model_ref": serde_json::Value::Null,
                "selected_reasoning_effort": serde_json::Value::Null,
                "rate": serde_json::Value::Null,
                "estimated_task_price_units": serde_json::Value::Null,
                "budget_verdict": serde_json::Value::Null,
            }
        ],
        "blocked_candidate_count": 0,
        "blocker_codes": [],
        "next_actions": [
            format!("Run `{execute_command}` to execute the first receipt-backed dispatch packet.")
        ],
        "execute_supported": true,
        "execution_attempted": false,
        "flow_projection": {
            "diagnostic_only": false,
            "receipt_status": {
                "receipt_backed": true,
                "status": "packet_ready",
            },
            "current_step": {
                "dispatch_command": execute_command,
                "dispatch_command_kind": "receipt_backed_dispatch_packet",
                "receipt_status": {
                    "receipt_backed": true,
                    "receipt_path": packet_path,
                    "status": "packet_ready",
                },
                "proof_state": {
                    "status": "pending_receipt_backed_execution",
                    "diagnostic_only": false,
                },
            },
        },
        "packet_materialization": {
            "status": release1_pass_status(),
            "requested": true,
            "materializes_packets": true,
            "artifact_count": 1,
            "artifacts": [
                {
                    "task_id": receipt.run_id,
                    "role_label": receipt.dispatch_target,
                    "runtime_role": runtime_role,
                    "task_class": task_class,
                    "dispatch_target": receipt.dispatch_target,
                    "packet_template_kind": packet_template_kind,
                    "dispatch_packet_path": packet_path,
                    "agent_init_execute_command": execute_command,
                    "receipt_backed": true,
                    "status": "packet_ready",
                }
            ],
            "errors": [],
            "reason": "existing_receipt_backed_dispatch_packet",
            "blocker_codes": [],
        },
        "source_surfaces": [
            "vida agent dispatch-next",
            "StateStore::run_graph_dispatch_receipt",
            "runtime_dispatch_receipt_helpers::dispatch_receipt_has_clean_routed_agent_handoff",
        ],
        "output_contract": {
            "view": "compact",
            "full_output_flag": "--full",
            "full_output_requires_json": true,
            "full_output_note": "Use --full --json to recompute scheduler, planner, carrier, fanout, and flow diagnostics."
        },
    }))
}

async fn emit_agent_dispatch_existing_packet_fast_path(
    command: &AgentDispatchNextArgs,
    store: &StateStore,
    state_dir: &std::path::Path,
    projection_name: &str,
) -> Option<ExitCode> {
    let current_task_id = command.current_task_id.as_deref()?;
    let receipt = match store.run_graph_dispatch_receipt(current_task_id).await {
        Ok(Some(receipt)) => receipt,
        _ => return None,
    };
    let payload = agent_dispatch_existing_packet_fast_path_payload(command, state_dir, &receipt)?;
    if command.json {
        crate::print_json_pretty(&payload);
        crate::operator_projection_cache::write_json_projection(
            state_dir,
            projection_name,
            &payload,
        );
    } else {
        println!("agent dispatch-next: {}", release1_pass_status());
        println!("lanes selected: 1");
        println!("packet materialization: {}", release1_pass_status());
        if let Some(next) =
            payload["packet_materialization"]["artifacts"][0]["agent_init_execute_command"].as_str()
        {
            println!("next: {next}");
        }
    }
    Some(ExitCode::SUCCESS)
}

fn emit_agent_dispatch_next_preview(
    command: &AgentDispatchNextArgs,
    state_dir: &std::path::Path,
    projection_name: &str,
    preview: AgentDispatchNextPreview,
) -> ExitCode {
    if command.json {
        let payload = if command.full {
            serde_json::to_value(&preview).expect("agent dispatch-next preview should serialize")
        } else {
            agent_dispatch_next_compact_payload(&preview)
        };
        crate::print_json_pretty(&payload);
        crate::operator_projection_cache::write_json_projection(
            state_dir,
            projection_name,
            &payload,
        );
    } else {
        println!("agent dispatch-next: {}", preview.status);
        println!("lanes selected: {}", preview.lanes_selected);
        if preview.packet_materialization["requested"]
            .as_bool()
            .unwrap_or(false)
        {
            println!(
                "packet materialization: {}",
                preview.packet_materialization["status"]
                    .as_str()
                    .unwrap_or("unknown")
            );
        } else {
            println!(
                "preview only: review carrier/model/cost selection truth before launching any `vida agent-init` command"
            );
        }
        for lane in &preview.selected_lanes {
            println!(
                "lane {} [{}]: {} [{} / {} / rate={} / est_cost={}]",
                lane.lane_index,
                lane.role_label,
                lane.task_id,
                lane.selection_truth.selected_carrier,
                lane.selection_truth.selected_model_ref,
                lane.selection_truth.rate,
                lane.selection_truth.estimated_task_price_units
            );
        }
        if !preview.blocker_codes.is_empty() {
            println!("blockers: {}", preview.blocker_codes.join(", "));
        }
        if let Some(first_command) = preview
            .packet_materialization
            .get("artifacts")
            .and_then(serde_json::Value::as_array)
            .and_then(|artifacts| artifacts.first())
            .and_then(|artifact| artifact["agent_init_execute_command"].as_str())
        {
            println!("next: {first_command}");
        }
    }
    if preview.status == release1_pass_status() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(crate) async fn run_agent(args: AgentArgs) -> ExitCode {
    match args.command {
        AgentCommand::DispatchNext(command) => run_agent_dispatch_next(command).await,
        AgentCommand::Select(command) => run_agent_select(command).await,
        AgentCommand::HostBridge(command) => run_agent_host_bridge(command).await,
        AgentCommand::Status(command) => run_agent_status(command).await,
    }
}

async fn run_agent_status(command: AgentStatusArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let store = match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["project_activation_unknown".to_string()],
                vec![format!(
                    "open the authoritative state store before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };

    let latest_status = match store.latest_run_graph_status().await {
        Ok(status) => status,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["run_graph_status_unreadable".to_string()],
                vec![format!(
                    "repair run-graph status evidence before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };
    let latest_receipt = match store.latest_run_graph_dispatch_receipt_summary().await {
        Ok(receipt) => receipt,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["run_graph_dispatch_receipt_unreadable".to_string()],
                vec![format!(
                    "repair dispatch receipt evidence before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };
    let latest_recovery = match store.latest_run_graph_recovery_summary().await {
        Ok(recovery) => recovery,
        Err(error) => {
            let payload = agent_status_payload(
                vec!["run_graph_recovery_unreadable".to_string()],
                vec![format!(
                    "repair recovery evidence before reading agent status: {error}"
                )],
                serde_json::json!({
                    "surface": "vida agent status",
                    "state_dir": state_dir.display().to_string()
                }),
                serde_json::json!({
                    "view": if command.compact { "compact" } else { "compact" },
                    "active_agents_count": 0,
                    "active_lanes_count": 0,
                    "handoff_pending_count": 0,
                    "view_only_dispatch_count": 0,
                    "blocked_dispatch_count": 0,
                    "reclaimable_lanes": [],
                    "next_recovery_command": null,
                }),
            );
            print_agent_status_payload(&payload, command.json);
            return ExitCode::from(1);
        }
    };

    let current_run_id = latest_status
        .as_ref()
        .map(|status| status.run_id.clone())
        .or_else(|| {
            latest_receipt
                .as_ref()
                .map(|receipt| receipt.run_id.clone())
        });
    let latest_receipt = latest_receipt.filter(|receipt| {
        current_run_id
            .as_deref()
            .map(|run_id| run_id == receipt.run_id)
            .unwrap_or(true)
    });
    let latest_recovery = latest_recovery.filter(|summary| {
        current_run_id
            .as_deref()
            .map(|run_id| run_id == summary.run_id)
            .unwrap_or(true)
    });
    let tasks = store.all_tasks().await.unwrap_or_default();
    let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
    let closed_task_ids = tasks
        .iter()
        .filter(|task| crate::state_store::StateStore::task_status_is_closed_like(&task.status))
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let current_runtime_task_stale = latest_status.as_ref().and_then(|status| {
        agent_status_runtime_task_stale_code(
            &status.task_id,
            &task_ids,
            &closed_task_ids,
            crate::runtime_dispatch_receipt_helpers::recovery_summary_is_terminal_retired_runtime_run(
                latest_recovery.as_ref(),
            ),
        )
    });
    let current_runtime_task_missing =
        current_runtime_task_stale.is_some_and(|code| code == "next_action_target_missing");
    let current_runtime_task_closed = current_runtime_task_stale
        .is_some_and(|code| code == "closed_task_active_run_projection_mismatch");
    let latest_recovery_is_terminal_retired_runtime_run =
        crate::runtime_dispatch_receipt_helpers::recovery_summary_is_terminal_retired_runtime_run(
            latest_recovery.as_ref(),
        );
    let active_lanes_count = latest_status
        .as_ref()
        .filter(|status| {
            !matches!(
                status.lifecycle_stage.as_str(),
                "closure_complete" | "completed" | "lane_completed"
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let active_agents_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            matches!(
                receipt.dispatch_status.as_str(),
                "routed" | "pending" | "bridge_request_pending" | "blocked"
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let handoff_pending_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            receipt.downstream_dispatch_ready
                || matches!(
                    receipt.dispatch_status.as_str(),
                    "routed" | "bridge_request_pending"
                )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let view_only_dispatch_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            receipt
                .effective_execution_posture
                .get("activation_evidence_state")
                .and_then(|value| value.as_str())
                == Some("activation_view_only")
        })
        .map(|_| 1)
        .unwrap_or(0);
    let blocked_dispatch_count = latest_receipt
        .as_ref()
        .filter(|receipt| {
            receipt.dispatch_status == release1_blocked_status()
                || receipt
                    .blocker_code
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
        })
        .map(|_| 1)
        .unwrap_or(0);
    let reclaimable_lanes = latest_recovery
        .as_ref()
        .filter(|summary| {
            !latest_recovery_is_terminal_retired_runtime_run
                && !summary.delegation_gate.delegated_cycle_open
                && matches!(
                    summary.lifecycle_stage.as_str(),
                    "closure_complete" | "completed" | "lane_completed"
                )
        })
        .map(|summary| vec![summary.run_id.clone()])
        .unwrap_or_default();
    let next_recovery_command = latest_recovery.as_ref().and_then(|summary| {
        if summary.delegation_gate.delegated_cycle_open {
            Some(format!(
                "vida taskflow recovery status {}",
                crate::shell_quote(&summary.run_id)
            ))
        } else if active_lanes_count > 0 {
            current_run_id.as_ref().map(|run_id| {
                format!(
                    "vida taskflow recovery status {}",
                    crate::shell_quote(run_id)
                )
            })
        } else if !reclaimable_lanes.is_empty() {
            Some("vida taskflow settle".to_string())
        } else {
            None
        }
    });
    let mut blocker_codes = Vec::new();
    if blocked_dispatch_count > 0 {
        blocker_codes.push(
            latest_receipt
                .as_ref()
                .and_then(|receipt| receipt.blocker_code.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    taskflow_contracts::BlockerCode::BlockedDispatch
                        .as_str()
                        .to_string()
                }),
        );
    }
    if current_runtime_task_missing {
        blocker_codes.push("next_action_target_missing".to_string());
    }
    if current_runtime_task_closed {
        blocker_codes.push("closed_task_active_run_projection_mismatch".to_string());
    }
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else if current_runtime_task_missing || current_runtime_task_closed {
        latest_status
            .as_ref()
            .map(|status| {
                let mut actions = Vec::new();
                if current_runtime_task_closed {
                    actions.push(
                        crate::status_surface_signals::closed_task_active_run_projection_mismatch_next_action(),
                    );
                }
                actions.push(
                    crate::status_surface_signals::runtime_binding_task_missing_next_action(
                        Some(status.run_id.as_str()),
                        &status.task_id,
                    ),
                );
                actions
            })
            .unwrap_or_default()
    } else {
        next_recovery_command
            .as_ref()
            .map(|command| vec![format!("run `{command}`")])
            .unwrap_or_default()
    };
    let artifact_refs = serde_json::json!({
        "surface": "vida agent status",
        "latest_run_id": current_run_id
            .clone()
            .or_else(|| latest_recovery.as_ref().map(|summary| summary.run_id.clone())),
        "latest_dispatch_packet_path": latest_receipt
            .as_ref()
            .and_then(|receipt| receipt.dispatch_packet_path.clone()),
        "state_dir": state_dir.display().to_string(),
    });
    let extra_fields = serde_json::json!({
        "view": if command.compact { "compact" } else { "compact" },
        "active_agents_count": active_agents_count,
        "active_lanes_count": active_lanes_count,
        "handoff_pending_count": handoff_pending_count,
        "view_only_dispatch_count": view_only_dispatch_count,
        "blocked_dispatch_count": blocked_dispatch_count,
        "reclaimable_lanes": reclaimable_lanes,
        "next_recovery_command": next_recovery_command,
    });
    let payload = agent_status_payload(blocker_codes, next_actions, artifact_refs, extra_fields);
    let success = payload["status"].as_str() == Some(release1_pass_status());
    print_agent_status_payload(&payload, command.json);
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn agent_status_payload(
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
    extra_fields: serde_json::Value,
) -> serde_json::Value {
    crate::release1_operator_output::build_release1_operator_output_payload(
        "vida agent status",
        blocker_codes,
        next_actions,
        artifact_refs,
        extra_fields,
    )
    .expect("agent status operator payload should be valid")
}

fn agent_status_runtime_task_stale_code(
    task_id: &str,
    task_ids: &[String],
    closed_task_ids: &[String],
    terminal_retired_runtime_run: bool,
) -> Option<&'static str> {
    if terminal_retired_runtime_run {
        None
    } else if !task_ids.iter().any(|id| id == task_id) {
        Some("next_action_target_missing")
    } else if closed_task_ids.iter().any(|id| id == task_id) {
        Some("closed_task_active_run_projection_mismatch")
    } else {
        None
    }
}

fn print_agent_status_payload(payload: &serde_json::Value, json: bool) {
    if json {
        crate::print_json_pretty(payload);
        return;
    }
    operator_output::toon_report::print(
        "vida agent status",
        vec![
            operator_output::toon_report::OperatorToonField::value(
                "status",
                payload["status"].clone(),
            ),
            operator_output::toon_report::OperatorToonField::value(
                "active_agents_count",
                payload["active_agents_count"].clone(),
            ),
            operator_output::toon_report::OperatorToonField::value(
                "active_lanes_count",
                payload["active_lanes_count"].clone(),
            ),
            operator_output::toon_report::OperatorToonField::value(
                "handoff_pending_count",
                payload["handoff_pending_count"].clone(),
            ),
            operator_output::toon_report::OperatorToonField::value(
                "blocked_dispatch_count",
                payload["blocked_dispatch_count"].clone(),
            ),
            operator_output::toon_report::OperatorToonField::value(
                "reclaimable_lanes",
                payload["reclaimable_lanes"].clone(),
            ),
            operator_output::toon_report::OperatorToonField::value(
                "next_recovery_command",
                payload["next_recovery_command"].clone(),
            ),
        ],
    );
}

async fn run_agent_host_bridge(mut command: AgentHostBridgeArgs) -> ExitCode {
    if command.submit_result.is_some() {
        command.complete = true;
        command.result_file = command.submit_result.clone();
    }
    if command.complete
        && command
            .host_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        let blocker_codes = vec![taskflow_contracts::BlockerCode::HostAgentIdMissing
            .as_str()
            .to_string()];
        let next_actions = vec![
            "provide --host-agent-id from the parent host adapter before completing the lane"
                .to_string(),
        ];
        let artifact_refs = serde_json::json!({
            "request_path": command.request.display().to_string()
        });
        let (shared_fields, operator_contracts) = host_bridge_operator_fields(
            release1_blocked_status(),
            blocker_codes.clone(),
            next_actions.clone(),
            next_actions,
            artifact_refs,
        );
        let payload = serde_json::json!({
            "surface": "vida agent host-bridge",
            "status": release1_blocked_status(),
            "blocker_codes": blocker_codes,
            "shared_fields": shared_fields,
            "operator_contracts": operator_contracts
        });
        return emit_host_bridge_payload(&payload, command.json);
    }
    match read_host_bridge_request(&command.request, command.state_dir.as_deref()) {
        Ok(request) => {
            let operator_request_path = command.request.clone();
            if let Ok(canonical_request_path) =
                canonical_host_bridge_request_path(&command.request, command.state_dir.as_deref())
            {
                command.request = canonical_request_path;
            }
            let mut provenance_blockers = host_bridge_request_provenance_blockers(
                &command.request,
                &request,
                command.state_dir.as_deref(),
                command.retry_completion,
            )
            .await;
            if !command.attach_artifacts.is_empty() {
                provenance_blockers.retain(|code| {
                    code != taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing
                        .as_str()
                });
            }
            if command.complete
                && host_bridge_complete_can_defer_missing_dispatch_receipt(&provenance_blockers)
            {
                provenance_blockers.clear();
            }
            let retry_state_root = command
                .state_dir
                .clone()
                .or_else(|| infer_host_bridge_state_root_from_request_path(&command.request))
                .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
            let retry_packet_path = host_bridge_request_string(&request, "packet_path")
                .and_then(|path| canonical_state_artifact_path(&retry_state_root, path, true).ok());
            let receipt_backed_retry_completion_evidence =
                host_bridge_request_has_retryable_dispatch_receipt_for_state_root(
                    &retry_state_root,
                    &request,
                    retry_packet_path.as_deref(),
                )
                .await;
            let payload = host_bridge_adapter_payload(
                &operator_request_path,
                &request,
                provenance_blockers,
                command.state_dir.as_deref(),
                receipt_backed_retry_completion_evidence,
            );
            if !command.attach_artifacts.is_empty() {
                return attach_host_bridge_implementation_artifacts(command, request, payload)
                    .await;
            }
            if let Some(result_path) = command.scaffold_result.as_ref() {
                return scaffold_host_bridge_result(
                    &command,
                    &command.request,
                    &request,
                    result_path,
                );
            }
            if let Some(result_path) = command.validate_result.as_ref() {
                let result = match read_canonical_host_bridge_json_artifact(result_path, "result") {
                    Ok(result) => result,
                    Err(error) => {
                        let blocked = host_bridge_result_validation_payload(
                            release1_blocked_status(),
                            vec!["host_bridge_result_unreadable".to_string()],
                            vec!["provide a readable host bridge result JSON artifact".to_string()],
                            &command.request,
                            result_path,
                            serde_json::json!({ "error": error }),
                        );
                        return emit_host_bridge_payload(&blocked, command.json);
                    }
                };
                let validation = validate_host_bridge_result_dry_run(
                    &command.request,
                    &request,
                    result_path,
                    &result,
                );
                return emit_host_bridge_payload(&validation, command.json);
            }
            if command.complete {
                if payload["status"].as_str() != Some(release1_pass_status()) {
                    return emit_host_bridge_payload(&payload, command.json);
                }
                let Some(host_agent_id) = command
                    .host_agent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    let blocker_codes = vec![taskflow_contracts::BlockerCode::HostAgentIdMissing
                        .as_str()
                        .to_string()];
                    let next_actions = vec![
                        "provide --host-agent-id from the parent host adapter before completing the lane"
                            .to_string(),
                    ];
                    let artifact_refs = payload
                        .get("operator_contracts")
                        .and_then(|contracts| contracts.get("artifact_refs"))
                        .cloned()
                        .unwrap_or_else(|| {
                            serde_json::json!({
                                "request_path": command.request.display().to_string()
                            })
                        });
                    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
                        release1_blocked_status(),
                        blocker_codes.clone(),
                        next_actions.clone(),
                        next_actions,
                        artifact_refs,
                    );
                    let mut blocked = payload.clone();
                    if let Some(object) = blocked.as_object_mut() {
                        object.insert(
                            "status".to_string(),
                            serde_json::json!(release1_blocked_status()),
                        );
                        object.insert(
                            "blocker_codes".to_string(),
                            serde_json::json!(blocker_codes),
                        );
                        object.insert("shared_fields".to_string(), shared_fields);
                        object.insert("operator_contracts".to_string(), operator_contracts);
                    }
                    return emit_host_bridge_payload(&blocked, command.json);
                };
                let lane_args = match host_bridge_completion_lane_args(
                    &command.request,
                    &payload,
                    host_agent_id,
                    command.summary.as_deref(),
                    command.receipt_id.as_deref(),
                    command.state_dir.as_deref(),
                    command.result_file.as_deref(),
                    command.decision.as_deref(),
                    command.verdict.as_deref(),
                    command.allowed_next_node.as_deref(),
                    command.blocker_codes.as_deref(),
                    &command.blocker_code,
                    command.rework_target.as_deref(),
                    command.retry_completion,
                    command.json,
                ) {
                    Ok(args) => args,
                    Err(error) => {
                        let blocker_codes = vec![
                            taskflow_contracts::BlockerCode::HostBridgeCompletionArgsInvalid
                                .as_str()
                                .to_string(),
                        ];
                        let artifact_refs = serde_json::json!({
                            "request_path": command.request.display().to_string()
                        });
                        let (shared_fields, operator_contracts) = host_bridge_operator_fields(
                            release1_blocked_status(),
                            blocker_codes.clone(),
                            vec![error.clone()],
                            vec!["repair the host bridge request before completion".to_string()],
                            artifact_refs,
                        );
                        let blocked = serde_json::json!({
                            "surface": "vida agent host-bridge",
                            "status": release1_blocked_status(),
                            "blocker_codes": blocker_codes,
                            "shared_fields": shared_fields,
                            "operator_contracts": operator_contracts
                        });
                        return emit_host_bridge_payload(&blocked, command.json);
                    }
                };
                let command_blocker_codes = host_bridge_command_blocker_codes(&command);
                let (handle_state, handle_blocker_codes) = host_bridge_handle_state_from_result(
                    command.result_file.as_deref(),
                    &command_blocker_codes,
                );
                let run_id = host_bridge_request_string(&request, "run_id");
                let dispatch_target = host_bridge_request_string(&request, "dispatch_target");
                let result_path = command
                    .result_file
                    .as_ref()
                    .map(|path| path.display().to_string());
                let request_path = command.request.display().to_string();
                if let Some(project_root) = host_bridge_observability_project_root(
                    command.state_dir.as_deref(),
                    &command.request,
                ) {
                    let _ = crate::record_host_agent_handle_state(
                        &project_root,
                        &crate::HostAgentHandleStateInput {
                            host_agent_id,
                            state: &handle_state,
                            run_id,
                            dispatch_target,
                            request_path: Some(&request_path),
                            result_path: result_path.as_deref(),
                            receipt_id: command.receipt_id.as_deref(),
                            blocker_codes: handle_blocker_codes,
                        },
                    );
                }
                return crate::lane_surface::run_lane(crate::ProxyArgs { args: lane_args }).await;
            }
            emit_host_bridge_payload(&payload, command.json)
        }
        Err(error) => {
            let path_safety_error = error.contains("dot-segment")
                || error.contains("escapes VIDA state root")
                || error.contains("symlink");
            let blocker_codes = vec![if path_safety_error {
                blocker_code_value(taskflow_contracts::BlockerCode::HostBridgeRequestUntrustedPath)
            } else {
                blocker_code_value(taskflow_contracts::BlockerCode::HostBridgeRequestUnreadable)
            }];
            let next_actions = vec![if path_safety_error {
                "provide a host_tool_bridge_request JSON artifact under the VIDA state root"
                    .to_string()
            } else {
                "provide a readable host_tool_bridge_request JSON artifact".to_string()
            }];
            let artifact_refs = serde_json::json!({
                "request_path": command.request.display().to_string()
            });
            let (shared_fields, operator_contracts) = host_bridge_operator_fields(
                release1_blocked_status(),
                blocker_codes.clone(),
                next_actions.clone(),
                next_actions,
                artifact_refs,
            );
            let payload = serde_json::json!({
                "surface": "vida agent host-bridge",
                "status": release1_blocked_status(),
                "blocker_codes": blocker_codes,
                "shared_fields": shared_fields,
                "operator_contracts": operator_contracts,
                "error": error
            });
            emit_host_bridge_payload(&payload, command.json)
        }
    }
}

async fn run_agent_select(command: AgentSelectArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => {
            let activation_bundle = match crate::build_taskflow_consume_bundle_payload(&store).await
            {
                Ok(payload) => payload.activation_bundle,
                Err(error) => {
                    eprintln!("Failed to load activation bundle for carrier selection: {error}");
                    return ExitCode::from(1);
                }
            };
            let selection = crate::build_runtime_assignment_from_resolved_constraints(
                &activation_bundle,
                &command.conversation_role,
                &command.task_class,
                &command.runtime_role,
            );
            let status =
                release1_contract_status_value(selection["enabled"].as_bool().unwrap_or(false));
            let payload = serde_json::json!({
                "surface": "vida agent select",
                "status": status,
                "mode": "config_driven_runtime_assignment",
                "runtime_role": command.runtime_role,
                "task_class": command.task_class,
                "conversation_role": command.conversation_role,
                "selection": selection,
                "manual_host_tool_choice_required": false,
                "source_surfaces": [
                    "vida.config.yaml",
                    "build_runtime_assignment_from_resolved_constraints",
                    "carrier_runtime.roles"
                ],
            });
            if command.json {
                crate::print_json_pretty(&payload);
            } else {
                println!(
                    "agent select: {}",
                    payload["status"].as_str().unwrap_or("unknown")
                );
                if let Some(carrier) = payload["selection"]["selected_carrier_id"].as_str() {
                    println!("selected carrier: {carrier}");
                }
                if let Some(profile) = payload["selection"]["selected_model_profile_id"].as_str() {
                    println!("selected model profile: {profile}");
                }
            }
            if status == release1_pass_status() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_agent_dispatch_next(command: AgentDispatchNextArgs) -> ExitCode {
    let state_dir = command
        .state_dir
        .clone()
        .unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir);
    let explicit_state_dir = command.state_dir.as_deref();
    let projection_name =
        agent_dispatch_next_projection_name(&command, command.materialize_packets);
    let cache_read_allowed = command.current_task_id.is_some()
        && !command.materialize_packets
        && !command.dev_team
        && !command.full;
    if command.json && cache_read_allowed {
        if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
            &state_dir,
            &projection_name,
        ) {
            println!("{cached}");
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_launcher_stale_state_fresh_recent_json_projection(
                &state_dir,
                &projection_name,
                AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE,
            )
        {
            println!("{cached}");
            return ExitCode::SUCCESS;
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_state_stale_recent_json_projection(
                &state_dir,
                &projection_name,
                AGENT_DISPATCH_NEXT_RECENT_PROJECTION_MAX_AGE,
            )
        {
            if let Some(overlay) =
                crate::operator_projection_cache::read_runtime_continuation_binding_overlay(
                    &state_dir,
                )
            {
                if let Some(rendered) =
                    crate::operator_projection_cache::apply_runtime_continuation_binding_overlay_to_payload(
                        &state_dir,
                        &cached,
                        &overlay,
                    )
                {
                    println!("{rendered}");
                    return ExitCode::SUCCESS;
                }
            }
        }
    }
    match StateStore::open_existing_read_only(state_dir.clone()).await {
        Ok(store) => {
            if let Some(exit_code) = emit_agent_dispatch_existing_packet_fast_path(
                &command,
                &store,
                &state_dir,
                &projection_name,
            )
            .await
            {
                return exit_code;
            }
            let mut activation_bundle =
                match crate::read_or_sync_launcher_activation_snapshot(&store).await {
                    Ok(snapshot) => snapshot.compiled_bundle,
                    Err(error) => {
                        eprintln!(
                            "Failed to load activation bundle for agent dispatch preview: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
            let explicit_binding = if command.current_task_id.is_none() {
                match store
                    .latest_explicit_run_graph_continuation_binding_for_current_session()
                    .await
                {
                    Ok(Some(binding)) => Some(binding),
                    Ok(None) => {
                        match store.latest_explicit_run_graph_continuation_binding().await {
                            Ok(binding) => binding,
                            Err(error) => {
                                eprintln!(
                                    "Failed to read latest explicit continuation binding: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to read latest explicit continuation binding: {error}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                None
            };
            let latest_run_graph_status = if command.current_task_id.is_none() {
                let current_session_status =
                    match store.latest_run_graph_status_for_current_session().await {
                        Ok(status) => status,
                        Err(error) => {
                            eprintln!(
                                "Failed to read latest current-session run-graph status: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    };
                if current_session_status.is_some() {
                    current_session_status
                } else {
                    match store.latest_run_graph_status().await {
                        Ok(Some(status)) if status.status == "blocked" => Some(status),
                        Ok(_) => None,
                        Err(error) => {
                            eprintln!("Failed to read latest global run-graph status: {error}");
                            return ExitCode::from(1);
                        }
                    }
                }
            } else {
                None
            };
            let latest_run_graph_dispatch_receipt = if command.current_task_id.is_none() {
                let current_session_receipt = match store
                    .latest_run_graph_dispatch_receipt_summary_for_current_session()
                    .await
                {
                    Ok(status) => status,
                    Err(error) => {
                        eprintln!(
                            "Failed to read latest current-session run-graph dispatch receipt: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                if current_session_receipt.is_some() {
                    current_session_receipt
                } else if let Some(status) = latest_run_graph_status.as_ref() {
                    match store
                        .run_graph_dispatch_receipt_summary_for_status(status)
                        .await
                    {
                        Ok(receipt) => receipt,
                        Err(error) => {
                            eprintln!(
                                "Failed to read run-graph dispatch receipt for active status: {error}"
                            );
                            return ExitCode::from(1);
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let explicit_bound_current_task_id = agent_dispatch_next_bound_current_task_id(
                explicit_binding.as_ref(),
                latest_run_graph_status.as_ref(),
                latest_run_graph_dispatch_receipt.as_ref(),
            );
            let taskflow_single_in_progress_task_id =
                if command.current_task_id.is_none() && explicit_bound_current_task_id.is_none() {
                    StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root())
                        .ok()
                        .and_then(|rows| {
                            single_in_progress_task_id_from_rows(&rows).map(str::to_string)
                        })
                } else {
                    None
                };
            let resolved_current_task_ids = resolve_agent_dispatch_next_current_task_ids(
                command.current_task_id.as_deref(),
                explicit_bound_current_task_id.as_deref(),
                taskflow_single_in_progress_task_id.as_deref(),
            );
            let preview = if command.dev_team {
                let configured_max_parallel_agents =
                    configured_max_parallel_agents_from_activation_bundle(&activation_bundle);
                let mut projection =
                    match StateStore::read_fresh_tasks_from_jsonl_snapshot(store.root()) {
                        Ok(rows) => {
                            let critical_path_ids = match StateStore::critical_path_from_rows(&rows)
                            {
                                Ok(path) => path
                                    .nodes
                                    .into_iter()
                                    .map(|node| node.id)
                                    .collect::<std::collections::BTreeSet<_>>(),
                                Err(_) => std::collections::BTreeSet::new(),
                            };
                            match StateStore::scheduling_projection_scoped_from_rows(
                                &rows,
                                command.scope.as_deref(),
                                resolved_current_task_ids.preview_current_task_id,
                                &critical_path_ids,
                            ) {
                                Ok(projection) => projection,
                                Err(error) => {
                                    eprintln!("Failed to compute agent dispatch preview: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        Err(_) => match store
                            .scheduling_projection_scoped(
                                command.scope.as_deref(),
                                resolved_current_task_ids.preview_current_task_id,
                            )
                            .await
                        {
                            Ok(projection) => projection,
                            Err(error) => {
                                eprintln!("Failed to compute agent dispatch preview: {error}");
                                return ExitCode::from(1);
                            }
                        },
                    };
                agent_dispatch_next_preserve_current_task_id(
                    &mut projection,
                    resolved_current_task_ids.preview_current_task_id,
                );
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
                let continuation_gate =
                    match crate::taskflow_proxy::build_taskflow_continuation_dispatch_gate_from_store(
                        &store,
                        &state_dir,
                        resolved_current_task_ids
                            .preview_current_task_id
                            .or(command.scope.as_deref()),
                    )
                    .await
                    {
                        Ok(gate) => gate,
                        Err(error) => {
                            eprintln!("Failed to compute agent continuation gate: {error}");
                            return ExitCode::from(1);
                        }
                    };
                drop(store);
                let mut preview = build_agent_dispatch_next_preview_with_diagnostics(
                    &activation_bundle,
                    &projection,
                    command.lanes,
                    configured_max_parallel_agents,
                    explicit_state_dir,
                    true,
                    command.full,
                );
                if let Some(gate) = continuation_gate {
                    apply_continuation_dispatch_gate_to_preview(&mut preview, &gate);
                }
                preview
            } else {
                let requested_parallel_limit = u64::try_from(command.lanes).ok();
                let plan =
                    match crate::taskflow_proxy::build_taskflow_scheduler_dispatch_plan_from_store(
                        &store,
                        &state_dir,
                        command.scope.as_deref(),
                        resolved_current_task_ids.scheduler_current_task_id,
                        requested_parallel_limit,
                        true,
                        false,
                    )
                    .await
                    {
                        Ok(plan) => plan,
                        Err(error) => {
                            eprintln!("Failed to compute agent dispatch preview: {error}");
                            return ExitCode::from(1);
                        }
                    };
                drop(store);
                build_agent_dispatch_next_preview_from_scheduler_plan_with_diagnostics(
                    &activation_bundle,
                    plan,
                    command.lanes,
                    explicit_state_dir,
                    command.full,
                )
            };
            let effective_materialize_packets =
                agent_dispatch_next_effective_materialize_packets(&command, &activation_bundle);
            let projection_name =
                agent_dispatch_next_projection_name(&command, effective_materialize_packets);
            let preview = if effective_materialize_packets {
                materialize_agent_dispatch_next_packets(preview, &state_dir, &activation_bundle)
                    .await
            } else {
                preview
            };
            emit_agent_dispatch_next_preview(&command, &state_dir, &projection_name, preview)
        }
        Err(error) => {
            if command.dev_team {
                let Some(current_task_id) = command.current_task_id.as_deref() else {
                    eprintln!("Failed to open authoritative state store: {error}");
                    return ExitCode::from(1);
                };
                let project_root =
                    crate::taskflow_task_bridge::infer_project_root_from_state_root(&state_dir)
                        .or_else(|| crate::resolve_runtime_project_root().ok());
                let Some(project_root) = project_root else {
                    eprintln!(
                        "Failed to resolve activation project root for state dir {}",
                        state_dir.display()
                    );
                    return ExitCode::from(1);
                };
                let mut activation_bundle = match capture_launcher_activation_snapshot_for_root(
                    &project_root,
                ) {
                    Ok(snapshot) => snapshot.compiled_bundle,
                    Err(snapshot_error) => {
                        eprintln!(
                            "Failed to load activation bundle for agent dispatch preview: {snapshot_error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                let rows = match StateStore::read_fresh_tasks_from_jsonl_snapshot(&state_dir) {
                    Ok(rows) => rows,
                    Err(fresh_error) => {
                        let snapshot_path =
                            StateStore::canonical_task_snapshot_path_for_state_root(&state_dir);
                        match StateStore::read_tasks_from_jsonl_snapshot(&snapshot_path) {
                            Ok(rows) => rows,
                            Err(snapshot_error) => {
                                eprintln!("Failed to open authoritative state store: {error}");
                                eprintln!(
                                    "Failed to read canonical task snapshot after authoritative open failure: {snapshot_error}; fresh snapshot error: {fresh_error}"
                                );
                                return ExitCode::from(1);
                            }
                        }
                    }
                };
                let critical_path_ids = match StateStore::critical_path_from_rows(&rows) {
                    Ok(path) => path
                        .nodes
                        .into_iter()
                        .map(|node| node.id)
                        .collect::<std::collections::BTreeSet<_>>(),
                    Err(_) => std::collections::BTreeSet::new(),
                };
                let projection = match StateStore::scheduling_projection_scoped_from_rows(
                    &rows,
                    command.scope.as_deref(),
                    Some(current_task_id),
                    &critical_path_ids,
                ) {
                    Ok(projection) => projection,
                    Err(projection_error) => {
                        eprintln!("Failed to compute agent dispatch preview: {projection_error}");
                        return ExitCode::from(1);
                    }
                };
                let readiness = crate::taskflow_consume_bundle::build_dev_team_readiness(
                    "vida.config.yaml",
                    &activation_bundle,
                );
                if let Some(object) = activation_bundle.as_object_mut() {
                    object.insert("dev_team_readiness".to_string(), readiness);
                }
                let effective_materialize_packets =
                    agent_dispatch_next_effective_materialize_packets(&command, &activation_bundle);
                let configured_max_parallel_agents =
                    configured_max_parallel_agents_from_activation_bundle(&activation_bundle);
                let mut preview = build_agent_dispatch_next_preview_with_diagnostics(
                    &activation_bundle,
                    &projection,
                    command.lanes,
                    configured_max_parallel_agents,
                    explicit_state_dir,
                    true,
                    command.full,
                );
                preview.source_surfaces.push(
                    "StateStore::read_fresh_tasks_from_jsonl_snapshot(authoritative-open-fallback)"
                        .to_string(),
                );
                let projection_name =
                    agent_dispatch_next_projection_name(&command, effective_materialize_packets);
                let preview = if effective_materialize_packets {
                    materialize_agent_dispatch_next_packets(preview, &state_dir, &activation_bundle)
                        .await
                } else {
                    preview
                };
                emit_agent_dispatch_next_preview(&command, &state_dir, &projection_name, preview)
            } else {
                eprintln!("Failed to open authoritative state store: {error}");
                ExitCode::from(1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        agent_dispatch_contract_status, agent_dispatch_existing_packet_fast_path_payload,
        agent_dispatch_materialization_lanes, agent_dispatch_next_bound_current_task_id,
        agent_dispatch_next_compact_payload, agent_dispatch_next_effective_materialize_packets,
        agent_dispatch_next_preserve_current_task_id, agent_dispatch_next_projection_name,
        agent_dispatch_status_from_blockers, agent_status_runtime_task_stale_code,
        apply_configured_lane_runtime_assignment, apply_continuation_dispatch_gate_to_preview,
        build_agent_dispatch_next_preview, canonical_host_bridge_request_path,
        completed_host_bridge_completion_request_for_state_root,
        configured_dev_team_first_step_for_task, dev_team_sequence, dev_team_sequence_for_task,
        dev_team_sequence_for_work_item, dispatch_target_for_agent_dispatch_lane,
        host_bridge_adapter_payload, host_bridge_changed_files_from_artifact,
        host_bridge_completion_lane_args, host_bridge_normalized_implementation_artifact_path,
        host_bridge_observability_project_root,
        host_bridge_request_has_retryable_dispatch_receipt_for_state_root,
        host_bridge_request_provenance_blockers,
        host_bridge_request_provenance_blockers_for_state_root,
        infer_host_bridge_state_root_from_request_path, materialize_configured_agent_dispatch_lane,
        read_canonical_host_bridge_json_artifact, read_host_bridge_request, release1_pass_status,
        resolve_agent_dispatch_next_current_task_ids, run_agent_host_bridge,
        single_in_progress_task_id_from_rows, state_store,
        validate_materialized_agent_dispatch_packet, write_host_bridge_request,
        AgentDispatchLanePreview, AgentDispatchLaneSelectionTruth, AgentDispatchNextPreview,
        MAX_HOST_BRIDGE_ARTIFACT_BYTES,
    };
    use crate::state_store::{
        CreateTaskRequest, LauncherActivationSnapshot, RunGraphDispatchReceipt,
        TaskExecutionSemantics, TaskRecord, TaskSchedulingCandidate, TaskSchedulingProjection,
    };
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir, EnvVarGuard};
    use crate::{AgentDispatchNextArgs, AgentHostBridgeArgs};
    use std::process::ExitCode;

    #[test]
    fn agent_dispatch_contract_status_is_table_driven_by_preview_and_assignment_blockers() {
        let cases = [
            (Vec::<String>::new(), Vec::<String>::new(), "pass"),
            (
                vec!["no_ready_task_candidates".to_string()],
                Vec::<String>::new(),
                "blocked",
            ),
            (
                Vec::<String>::new(),
                vec!["selected_model_profile_not_ready".to_string()],
                "blocked",
            ),
        ];

        for (preview_blockers, assignment_blockers, expected) in cases {
            assert_eq!(
                agent_dispatch_contract_status(&preview_blockers, &assignment_blockers),
                expected
            );
            if assignment_blockers.is_empty() {
                assert_eq!(
                    agent_dispatch_status_from_blockers(&preview_blockers),
                    expected
                );
            }
        }
    }

    #[test]
    fn host_bridge_handle_state_marks_capacity_unavailable_from_result() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let result_path = harness.path().join("result.json");
        std::fs::write(
            &result_path,
            serde_json::json!({
                "status": "blocked",
                "blocker_codes": ["host_agent_capacity_unavailable"]
            })
            .to_string(),
        )
        .expect("result should write");

        let (state, blockers) =
            super::host_bridge_handle_state_from_result(Some(&result_path), &[]);

        assert_eq!(state, "capacity_unavailable");
        assert_eq!(blockers, vec!["host_agent_capacity_unavailable"]);
    }

    #[test]
    fn host_bridge_handle_state_marks_pass_result_completed() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let result_path = harness.path().join("result.json");
        std::fs::write(
            &result_path,
            serde_json::json!({
                "status": "pass",
                "blocker_codes": []
            })
            .to_string(),
        )
        .expect("result should write");

        let (state, blockers) =
            super::host_bridge_handle_state_from_result(Some(&result_path), &[]);

        assert_eq!(state, "completed");
        assert!(blockers.is_empty());
    }

    #[test]
    fn host_bridge_handle_state_rejects_oversized_result_artifact() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let result_path = harness.path().join("oversized-result.json");
        std::fs::write(
            &result_path,
            vec![b' '; MAX_HOST_BRIDGE_ARTIFACT_BYTES as usize + 1],
        )
        .expect("oversized result should write");

        let (state, blockers) =
            super::host_bridge_handle_state_from_result(Some(&result_path), &[]);

        assert_eq!(state, "failed");
        assert_eq!(blockers, vec!["host_bridge_result_unreadable"]);
    }

    #[cfg(unix)]
    #[test]
    fn host_bridge_handle_state_rejects_symlink_result_artifact() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let target_path = harness.path().join("target-result.json");
        let result_path = harness.path().join("linked-result.json");
        std::fs::write(
            &target_path,
            serde_json::json!({
                "status": "pass",
                "blocker_codes": []
            })
            .to_string(),
        )
        .expect("target result should write");
        std::os::unix::fs::symlink(&target_path, &result_path)
            .expect("result symlink should be created");

        let (state, blockers) =
            super::host_bridge_handle_state_from_result(Some(&result_path), &[]);

        assert_eq!(state, "failed");
        assert_eq!(blockers, vec!["host_bridge_result_unreadable"]);
    }

    #[test]
    fn completed_host_bridge_completion_request_rejects_forged_result_without_provenance() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path();
        let result_path = state_root.join("host-tool-bridge/results/forged.json");
        std::fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("result parent should be created");
        std::fs::write(
            &result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "status": "pass",
                "execution_state": "executed",
                "allowed_next_node": "developer"
            }))
            .expect("result should serialize"),
        )
        .expect("forged result should write");

        let request = serde_json::json!({
            "status": "completed",
            "dispatch_transport": "host_tool_bridge",
            "result_path": result_path.display().to_string(),
            "request_id": "req-forged",
            "run_id": "run-forged",
            "dispatch_target": "implementer"
        });

        assert!(
            !completed_host_bridge_completion_request_for_state_root(state_root, &request),
            "completed preview refresh evidence must require artifact kind, receipt-backed evidence, source packet path, and mandatory identity fields"
        );
    }

    #[test]
    fn agent_status_blocks_closed_runtime_task_target() {
        assert_eq!(
            agent_status_runtime_task_stale_code(
                "closed-task",
                &["closed-task".to_string()],
                &["closed-task".to_string()],
                false,
            ),
            Some("closed_task_active_run_projection_mismatch")
        );
        assert_eq!(
            agent_status_runtime_task_stale_code(
                "missing-task",
                &["other-task".to_string()],
                &[],
                false
            ),
            Some("next_action_target_missing")
        );
        assert_eq!(
            agent_status_runtime_task_stale_code(
                "closed-task",
                &["closed-task".to_string()],
                &[],
                false
            ),
            None
        );
        assert_eq!(
            agent_status_runtime_task_stale_code(
                "closed-task",
                &[],
                &["closed-task".to_string()],
                true
            ),
            None
        );
    }

    fn coach_dispatch_lane_preview(role_label: &str, task_id: &str) -> AgentDispatchLanePreview {
        AgentDispatchLanePreview {
            lane_index: 1,
            task_id: task_id.to_string(),
            title: format!("{role_label} task"),
            role_label: role_label.to_string(),
            runtime_role: "coach".to_string(),
            task_class: "coach".to_string(),
            dispatch_command: format!("vida agent-init --role coach {task_id}"),
            dispatch_command_kind: "startup_activation_view_only".to_string(),
            receipt_backed_execution_command: format!(
                "vida agent-init --dispatch-packet {task_id}.json --execute-dispatch"
            ),
            ready_parallel_safe: true,
            selection_reason: format!("configured_dev_team_lane:{role_label}"),
            selection_truth: AgentDispatchLaneSelectionTruth {
                selected_carrier: "coach-seat".to_string(),
                selected_backend: "internal_subagents".to_string(),
                selected_model_profile: "coach-profile".to_string(),
                selected_model_ref: "gpt-5.5-coach".to_string(),
                selected_reasoning_effort: "low".to_string(),
                rate: 3,
                estimated_task_price_units: 3,
                budget_verdict: "admissible".to_string(),
                selected_over_budget: false,
                selected_model_profile_readiness_status: "ready".to_string(),
                pricing_freshness_status: "fresh".to_string(),
                selected_external_backend_readiness_status: "ready".to_string(),
                selection_source_paths: serde_json::json!(["dev_team_readiness.roles"]),
                pricing_readiness: serde_json::json!({"status": "ready"}),
                runtime_role: "coach".to_string(),
                task_class: "coach".to_string(),
            },
            requires_user_approval: false,
            approval_gate: serde_json::json!({
                "required": false,
                "status": "not_required"
            }),
        }
    }

    fn analyst_dispatch_lane_preview(task_id: &str) -> AgentDispatchLanePreview {
        let mut lane = coach_dispatch_lane_preview("analyst", task_id);
        lane.runtime_role = "business_analyst".to_string();
        lane.task_class = "specification".to_string();
        lane.dispatch_command = format!("vida agent-init --role business_analyst {task_id}");
        lane.selection_truth.runtime_role = "business_analyst".to_string();
        lane.selection_truth.task_class = "specification".to_string();
        lane.selection_truth.selected_carrier = "middle".to_string();
        lane
    }

    fn analyst_assignment_bundle() -> serde_json::Value {
        serde_json::json!({
            "agent_system": {
                "mode": "local",
                "state_owner": "taskflow",
                "max_parallel_agents": 4,
                "routing": {
                    "default": {
                        "executor_backend": "internal_subagents",
                        "max_budget_units": 32
                    }
                }
            },
            "carrier_runtime": {
                "roles": [{
                    "role_id": "middle",
                    "tier": "middle",
                    "rate": 4,
                    "normalized_cost_units": 4,
                    "default_runtime_role": "business_analyst",
                    "runtime_roles": ["business_analyst"],
                    "task_classes": ["specification"],
                    "reasoning_band": "medium",
                    "default_model_profile": "codex_gpt55_medium_write",
                    "model_profiles": {
                        "codex_gpt55_medium_write": {
                            "profile_id": "codex_gpt55_medium_write",
                            "model_ref": "gpt-5.5",
                            "provider": "openai-codex",
                            "reasoning_effort": "medium",
                            "normalized_cost_units": 4,
                            "speed_tier": "fast",
                            "quality_tier": "medium",
                            "write_scope": "workspace-write",
                            "runtime_roles": ["business_analyst"],
                            "task_classes": ["specification"],
                            "readiness": { "required": false, "ready": true }
                        }
                    }
                }],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier",
                        "demotion_score": 45
                    },
                    "agents": {
                        "middle": {
                            "effective_score": 91,
                            "lifecycle_state": "promoted"
                        }
                    }
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "free_profiles_allowed": false,
                    "quality_floor_by_runtime_role": {
                        "business_analyst": "medium"
                    },
                    "reasoning_floor_by_task_class": {
                        "specification": "medium",
                        "implementation": "low"
                    }
                }
            }
        })
    }

    #[test]
    fn configured_dev_team_materialization_targets_keep_distinct_coach_gate_labels() {
        let coach_test_gate = coach_dispatch_lane_preview("coach_test_gate", "routing-proof-a");
        let coach_implementation_gate =
            coach_dispatch_lane_preview("coach_implementation_gate", "routing-proof-b");

        assert_eq!(
            dispatch_target_for_agent_dispatch_lane(&coach_test_gate),
            "coach_test_gate"
        );
        assert_eq!(
            dispatch_target_for_agent_dispatch_lane(&coach_implementation_gate),
            "coach_implementation_gate"
        );
        assert_ne!(
            dispatch_target_for_agent_dispatch_lane(&coach_test_gate),
            dispatch_target_for_agent_dispatch_lane(&coach_implementation_gate)
        );
        assert_eq!(coach_test_gate.task_class, "coach");
        assert_eq!(coach_implementation_gate.task_class, "coach");
    }

    #[test]
    fn configured_dev_team_materialization_preserves_lane_runtime_assignment() {
        let activation_bundle = analyst_assignment_bundle();
        let lane = analyst_dispatch_lane_preview("ldr-020");
        let mut role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "vida.config.yaml".to_string(),
            selection_mode: "configured_dev_team_dispatch_next".to_string(),
            fallback_role: "orchestrator".to_string(),
            request:
                "Implement RunWorkflow aggregate and hierarchical Statig machine. Architecture law applies."
                    .to_string(),
            selected_role: lane.runtime_role.clone(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "explicit_configured_dev_team_lane".to_string(),
            matched_terms: vec![lane.role_label.clone(), lane.task_class.clone()],
            compiled_bundle: activation_bundle.clone(),
            execution_plan: serde_json::Value::Null,
            reason: "materialize configured dev-team lane `analyst` as `analyst`".to_string(),
        };

        role_selection.execution_plan =
            crate::development_flow_orchestration::build_runtime_execution_plan_from_snapshot(
                &activation_bundle,
                &role_selection,
            );
        apply_configured_lane_runtime_assignment(&mut role_selection, &activation_bundle, &lane)
            .expect("configured lane assignment should resolve from lane task class");

        let assignment = &role_selection.execution_plan["runtime_assignment"];
        assert_eq!(assignment["enabled"], true);
        assert_eq!(assignment["task_class"], "specification");
        assert_eq!(assignment["runtime_role"], "business_analyst");
        assert_eq!(assignment["selected_backend_id"], "internal_subagents");
        assert_eq!(
            role_selection.execution_plan["carrier_runtime_assignment"]["selected_backend_id"],
            "internal_subagents"
        );
        assert_eq!(
            role_selection.execution_plan["runtime_assignment"],
            role_selection.execution_plan["carrier_runtime_assignment"]
        );
    }

    fn sample_agent_dispatch_next_preview() -> AgentDispatchNextPreview {
        AgentDispatchNextPreview {
            status: release1_pass_status().to_string(),
            mode: "materialized-dev-team".to_string(),
            lanes_requested: 4,
            configured_max_parallel_agents: 4,
            effective_max_parallel_agents: 4,
            lanes_selected: 1,
            selected_lanes: vec![coach_dispatch_lane_preview(
                "coach_implementation_gate",
                "task-a",
            )],
            blocked_candidates: vec![super::AgentDispatchBlockedCandidate {
                task_id: "blocked-a".to_string(),
                title: "blocked task".to_string(),
                ready_now: false,
                ready_parallel_safe: false,
                reasons: vec!["dependency_open".to_string()],
                parallel_blockers: vec!["conflict_domain_busy".to_string()],
            }],
            blocker_codes: Vec::new(),
            next_actions: vec![
                "Run `vida agent-init --dispatch-packet task-a.json --execute-dispatch`."
                    .to_string(),
            ],
            execute_supported: true,
            execution_attempted: false,
            parallelization_planner: serde_json::json!({
                "large_diagnostic": "planner",
                "packet_artifacts": [
                    {"dispatch_packet_path": "task-a.json", "agent_init_execute_command": "vida agent-init --dispatch-packet task-a.json --execute-dispatch"}
                ]
            }),
            packet_materialization: serde_json::json!({
                "status": release1_pass_status(),
                "requested": true,
                "materializes_packets": true,
                "artifacts": [
                    {
                        "task_id": "task-a",
                        "role_label": "coach_implementation_gate",
                        "dispatch_target": "coach_implementation_gate",
                        "dispatch_packet_path": "task-a.json",
                        "agent_init_execute_command": "vida agent-init --dispatch-packet task-a.json --execute-dispatch",
                        "extra_large_diagnostic": {"omit": true}
                    }
                ]
            }),
            carrier_selection_api: serde_json::json!({"large_diagnostic": "carrier"}),
            fanout_guard: serde_json::json!({"large_diagnostic": "fanout"}),
            flow_projection: serde_json::json!({
                "large_diagnostic": "flow",
                "current_step": {
                    "dispatch_command_kind": "receipt_backed_dispatch_packet",
                    "dispatch_command": "vida agent-init --dispatch-packet task-a.json --execute-dispatch"
                },
                "receipt_status": {
                    "status": "packet_ready"
                }
            }),
            source_surfaces: vec!["vida agent dispatch-next".to_string()],
        }
    }

    #[test]
    fn dev_team_materialization_lanes_are_limited_to_first_sequential_packet() {
        let mut preview = sample_agent_dispatch_next_preview();
        preview.mode = "preview-dev-team".to_string();
        preview.selected_lanes = vec![
            analyst_dispatch_lane_preview("task-a"),
            coach_dispatch_lane_preview("developer", "task-a"),
            coach_dispatch_lane_preview("coach_validator", "task-a"),
        ];
        preview.lanes_selected = preview.selected_lanes.len();

        let lanes = agent_dispatch_materialization_lanes(&preview);

        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].role_label, "analyst");
    }

    #[test]
    fn non_dev_team_materialization_keeps_all_selected_lanes() {
        let mut preview = sample_agent_dispatch_next_preview();
        preview.mode = "preview".to_string();
        preview.selected_lanes = vec![
            coach_dispatch_lane_preview("worker_a", "task-a"),
            coach_dispatch_lane_preview("worker_b", "task-b"),
        ];
        preview.lanes_selected = preview.selected_lanes.len();

        let lanes = agent_dispatch_materialization_lanes(&preview);

        assert_eq!(lanes.len(), 2);
        assert_eq!(lanes[0].task_id, "task-a");
        assert_eq!(lanes[1].task_id, "task-b");
    }

    #[test]
    fn agent_dispatch_next_compact_payload_omits_heavy_diagnostics_but_keeps_execute_command() {
        let payload = agent_dispatch_next_compact_payload(&sample_agent_dispatch_next_preview());

        assert_eq!(payload["output_contract"]["view"], "compact");
        assert_eq!(payload["output_contract"]["full_output_flag"], "--full");
        assert_eq!(payload["blocked_candidate_count"], 1);
        assert!(payload.get("blocked_candidates").is_none());
        assert!(payload.get("parallelization_planner").is_none());
        assert!(payload.get("carrier_selection_api").is_none());
        assert!(payload.get("fanout_guard").is_none());
        assert_eq!(
            payload["flow_projection"]["current_step"]["dispatch_command_kind"],
            "receipt_backed_dispatch_packet"
        );
        assert_eq!(
            payload["flow_projection"]["receipt_status"]["status"],
            "packet_ready"
        );
        assert!(payload["flow_projection"].get("large_diagnostic").is_none());
        assert_eq!(
            payload["packet_materialization"]["artifacts"][0]["agent_init_execute_command"],
            "vida agent-init --dispatch-packet task-a.json --execute-dispatch"
        );
        assert!(payload["packet_materialization"]["artifacts"][0]
            .get("extra_large_diagnostic")
            .is_none());
    }

    #[test]
    fn agent_dispatch_next_full_payload_preserves_diagnostics() {
        let payload = serde_json::to_value(sample_agent_dispatch_next_preview())
            .expect("preview should serialize");

        assert!(payload.get("blocked_candidates").is_some());
        assert!(payload.get("parallelization_planner").is_some());
        assert!(payload.get("carrier_selection_api").is_some());
        assert!(payload.get("fanout_guard").is_some());
        assert!(payload.get("flow_projection").is_some());
    }

    #[test]
    fn agent_dispatch_existing_packet_fast_path_payload_reuses_validated_receipt() {
        let temp = std::env::temp_dir().join(format!(
            "vida-dispatch-fast-path-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("create fast path temp dir");
        let packet_path = temp.join("dispatch-packet.json");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_template_kind": "delivery_task_packet",
                "handoff_runtime_role": "worker",
                "handoff_task_class": "test_authoring",
                "delivery_task_packet": {
                    "handoff_runtime_role": "worker",
                    "handoff_task_class": "test_authoring"
                }
            })
            .to_string(),
        )
        .expect("write dispatch packet");
        let command = AgentDispatchNextArgs {
            lanes: 4,
            scope: None,
            current_task_id: Some("run-fast".to_string()),
            state_dir: None,
            json: true,
            full: false,
            dev_team: true,
            materialize_packets: true,
        };
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-fast".to_string(),
            dispatch_target: "autotester".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_open".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(packet_path.display().to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: Some("autotester".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: None,
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("vibe_cli".to_string()),
            recorded_at: "2026-06-25T00:00:00Z".to_string(),
        };

        let payload = agent_dispatch_existing_packet_fast_path_payload(&command, &temp, &receipt)
            .expect("valid receipt and packet should fast-path");

        assert_eq!(payload["status"], release1_pass_status());
        assert_eq!(payload["output_contract"]["view"], "compact");
        assert!(payload.get("parallelization_planner").is_none());
        assert_eq!(payload["selected_lanes"][0]["role_label"], "autotester");
        assert_eq!(payload["selected_lanes"][0]["runtime_role"], "worker");
        assert_eq!(payload["selected_lanes"][0]["task_class"], "test_authoring");
        assert_eq!(
            payload["packet_materialization"]["artifacts"][0]["dispatch_packet_path"],
            packet_path.display().to_string()
        );
        assert_eq!(
            payload["packet_materialization"]["artifacts"][0]["runtime_role"],
            "worker"
        );
        assert_eq!(
            payload["packet_materialization"]["artifacts"][0]["task_class"],
            "test_authoring"
        );
        assert!(
            payload["packet_materialization"]["artifacts"][0]["agent_init_execute_command"]
                .as_str()
                .expect("execute command should render")
                .contains("--state-dir")
        );

        let mut full_command = command.clone();
        full_command.full = true;
        assert!(
            agent_dispatch_existing_packet_fast_path_payload(&full_command, &temp, &receipt)
                .is_none()
        );

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn configured_dev_team_packet_validation_uses_configured_coach_gate_targets() {
        let temp =
            std::env::temp_dir().join(format!("vida-dispatch-target-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).expect("create temp dir");
        let coach_test_gate = coach_dispatch_lane_preview("coach_test_gate", "routing-proof-a");
        let coach_implementation_gate =
            coach_dispatch_lane_preview("coach_implementation_gate", "routing-proof-b");

        for lane in [&coach_test_gate, &coach_implementation_gate] {
            let dispatch_target = dispatch_target_for_agent_dispatch_lane(lane);
            let packet_path = temp.join(format!("{}-packet.json", lane.task_id));
            std::fs::write(
                &packet_path,
                serde_json::json!({
                    "run_id": lane.task_id,
                    "dispatch_target": dispatch_target,
                    "packet_template_kind": "coach_review_packet"
                })
                .to_string(),
            )
            .expect("write packet");
            let receipt = RunGraphDispatchReceipt {
                run_id: lane.task_id.clone(),
                dispatch_target: dispatch_target.to_string(),
                dispatch_status: "routed".to_string(),
                lane_status: "active".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "configured_dev_team".to_string(),
                dispatch_surface: None,
                dispatch_command: None,
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: None,
                activation_runtime_role: Some("coach".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: format!("{}-receipt", lane.task_id),
            };

            let packet = validate_materialized_agent_dispatch_packet(
                lane,
                dispatch_target,
                &packet_path.display().to_string(),
                &receipt,
            )
            .expect("configured gate target validates");
            assert_eq!(packet["dispatch_target"].as_str(), Some(dispatch_target));
            assert_eq!(
                packet["packet_template_kind"].as_str(),
                Some("coach_review_packet")
            );
            assert!(validate_materialized_agent_dispatch_packet(
                lane,
                "coach",
                &packet_path.display().to_string(),
                &receipt,
            )
            .expect_err("legacy coach collapse must fail")
            .contains("expected `coach`"));
        }
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[tokio::test]
    async fn configured_dev_team_materialization_fails_when_dispatch_receipt_cannot_be_recorded() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-dispatch-missing-store-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create temp state root without state store");
        std::fs::write(
            root.join("datastore-payload-marker"),
            "existing state payload",
        )
        .expect("seed existing datastore payload marker");
        std::fs::create_dir(root.join(".vida-authoritative-open.guard"))
            .expect("block authoritative store guard file open");
        let lane = coach_dispatch_lane_preview("coach_test_gate", "receipt-recording-proof");

        let error = materialize_configured_agent_dispatch_lane(
            &lane,
            &root,
            &activation_bundle_with_worker_selection_truth(),
        )
        .await
        .expect_err("materialization must fail before claiming receipt-backed packet");

        assert!(
            error.contains("Failed to open state store to record dev-team dispatch receipt"),
            "unexpected materialization error: {error}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_adapter_payload_renders_parent_host_tool_contract() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            Vec::new(),
            None,
            false,
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"].as_array().unwrap().len(), 0);
        assert_eq!(payload["shared_fields"]["status"], payload["status"]);
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            payload["operator_contracts"]["next_actions"]
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"],
            payload["operator_contracts"]["artifact_refs"]
        );
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "host-agent-bridge-adapter-v1"
        );
        let completion_command = payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command should render");
        assert!(completion_command.starts_with("vida agent host-bridge --request request.json "));
        assert!(completion_command.contains("--submit-result '<host-bridge-result-file>'"));
        assert!(!completion_command.contains("--submit-result result.json"));
        assert!(completion_command.contains("--receipt-id run-1-implementer-host-bridge-receipt"));
        assert!(!completion_command.contains("--json"));
        let calls = payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("host tool calls should render");
        assert_eq!(calls[0]["tool"], "multi_agent_v1.spawn_agent");
        assert_eq!(calls[1]["tool"], "multi_agent_v1.wait_agent");
        assert_eq!(calls[2]["tool"], "multi_agent_v1.close_agent");
        assert_eq!(
            payload["host_bridge"]["blocked_result_contract"]["allowed_blocker_codes"][0],
            "host_agent_capacity_unavailable"
        );
        assert!(
            matches!(
                payload["host_bridge"]["adapter_capacity"]["status"].as_str(),
                Some("ready_to_attempt") | Some("parent_host_capacity_unobservable")
            ),
            "payload={payload}"
        );
        assert!(
            payload["host_bridge"]["adapter_capacity"]["capacity_observable"].is_boolean(),
            "payload={payload}"
        );
        assert!(
            matches!(
                payload["host_bridge"]["adapter_capacity"]["capacity_source"].as_str(),
                Some("host_agent_handle_registry") | Some("parent_host_tool_runtime")
            ),
            "payload={payload}"
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["blocked_result_code"],
            "host_agent_capacity_unavailable"
        );
    }

    #[test]
    fn host_bridge_adapter_completion_command_omits_untrusted_packet_next_node() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-packet-next-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let packet_path =
            state_root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("create packet parent");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("create request parent");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": "run-analyst",
                "dispatch_target": "analyst",
                "downstream_dispatch_target": "designer",
                "downstream_dispatch_active_target": "analyst"
            })
            .to_string(),
        )
        .expect("write packet");
        let canonical_result_path = state_root.join("host-tool-bridge/results/result.json");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-analyst",
            "run_id": "run-analyst",
            "task_id": "run-analyst",
            "dispatch_target": "source_lane",
            "allowed_next_node": "closure",
            "packet_path": packet_path.display().to_string(),
            "runtime_role": "business_analyst",
            "task_class": "specification",
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path.display().to_string(),
            "result_path": canonical_result_path.display().to_string(),
            "receipt_path": state_root.join("host-tool-bridge/receipts/receipt.json").display().to_string()
        });
        std::fs::write(&request_path, request.to_string()).expect("write request");

        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            Vec::new(),
            Some(&state_root),
            false,
        );

        assert_eq!(payload["status"], "pass");
        let completion_command = payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command");
        assert!(completion_command.starts_with("vida agent host-bridge --request "));
        assert!(completion_command.contains("--receipt-id run-analyst-analyst-host-bridge-receipt"));
        assert!(completion_command.contains("--host-agent-id '<host-agent-id>'"));
        assert!(completion_command.contains("--submit-result"));
        assert!(completion_command.contains("--submit-result '<host-bridge-result-file>'"));
        assert!(
            !completion_command.contains(&canonical_result_path.display().to_string()),
            "completion command must not use canonical result_path as submit-result input: {completion_command}"
        );
        assert!(!completion_command.contains("--host-bridge-summary"));
        assert!(
            !completion_command.contains("--allowed-next-node"),
            "completion command must not promote packet/request routing into CLI override: {completion_command}"
        );
        assert!(!completion_command.contains("--blocker-codes"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_adapter_payload_normalizes_legacy_internal_subagents_adapter_contract() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "unconfigured_host_agent_adapter",
            "adapter_capability_id": "unconfigured_host_agent_capability",
            "invocation_mode": "configured_host_capability_required",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            Vec::new(),
            None,
            false,
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["host_bridge_request_missing_fields"])
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capability_id"],
            "codex.multi_agent_v1"
        );
        assert_eq!(
            payload["host_bridge"]["adapter_contract_source"],
            "legacy_internal_subagents_default"
        );
        assert_eq!(
            payload["host_bridge"]["missing_fields"],
            serde_json::json!(["adapter_kind", "adapter_capability_id", "invocation_mode"])
        );
        assert_eq!(
            payload["host_bridge"]["host_tool_calls"],
            serde_json::json!([])
        );
    }

    #[test]
    fn host_bridge_adapter_payload_missing_paths_blocks() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "request.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            Vec::new(),
            None,
            false,
        );

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker codes should render")
            .iter()
            .any(|code| code == "host_bridge_request_missing_fields"));
        assert_eq!(
            payload["host_bridge"]["host_tool_calls"]
                .as_array()
                .expect("host tool calls should render")
                .len(),
            0
        );
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_wrong_transport_and_capability() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "codex_cli_exec",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "missing.capability",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            Vec::new(),
            None,
            false,
        );

        assert_eq!(payload["status"], "blocked");
        let blockers = payload["blocker_codes"]
            .as_array()
            .expect("blockers should render")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        assert!(blockers.contains(&"host_bridge_request_wrong_transport"));
        assert!(blockers.contains(&"host_tool_capability_missing"));
        assert_eq!(
            payload["host_bridge"]["host_tool_calls"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_untrusted_provenance() {
        let request = serde_json::json!({
            "status": "pending",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "/tmp/attacker-packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("/tmp/forged-request.json"),
            &request,
            vec!["host_bridge_request_untrusted_path".to_string()],
            None,
            false,
        );

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_untrusted_path"));
        assert!(payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("calls")
            .is_empty());
    }

    fn host_bridge_submit_result_role_selection(
        dev_task_id: &str,
    ) -> crate::RuntimeConsumptionLaneSelection {
        crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": dev_task_id,
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["alpha_spec", "beta_design", "gamma_proof", "delta_impl"],
                        "lane_catalog": {
                            "gamma_proof": {
                                "dispatch_target": "gamma_proof",
                                "stage": "verification",
                                "task_class": "verification",
                                "closure_class": "verification",
                                "completion_blocker": "pending_autotest_evidence",
                                "packet_template_kind": "verifier_proof_packet",
                                "activation_agent_type": "middle",
                                "activation_runtime_role": "worker",
                                "runtime_assignment": {
                                    "selected_backend_id": "internal_subagents",
                                    "selected_carrier_id": "middle"
                                }
                            },
                            "delta_impl": {
                                "dispatch_target": "delta_impl",
                                "stage": "execution",
                                "task_class": "implementation",
                                "closure_class": "implementation",
                                "completion_blocker": "pending_implementation_evidence",
                                "packet_template_kind": "delivery_task_packet",
                                "activation_agent_type": "junior",
                                "activation_runtime_role": "worker",
                                "runtime_assignment": {
                                    "selected_backend_id": "internal_subagents",
                                    "selected_carrier_id": "junior"
                                }
                            }
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        }
    }

    fn host_bridge_validate_request() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-validate-1",
            "run_id": "run-validate-1",
            "task_id": "task-validate-1",
            "dispatch_target": "delta_impl",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "host-tool-bridge/requests/request.json",
            "result_path": "host-tool-bridge/results/result.json",
            "receipt_path": "host-tool-bridge/receipts/receipt.json",
            "allowed_next_node": "epsilon_gate"
        })
    }

    fn host_bridge_validate_result(allowed_next_node: &str) -> serde_json::Value {
        serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "schema_version": 1,
            "status": "pass",
            "execution_state": "executed",
            "request_id": "req-validate-1",
            "run_id": "run-validate-1",
            "dispatch_target": "delta_impl",
            "decision": "pass",
            "verdict": "pass",
            "blocker_codes": [],
            "rework_target": serde_json::Value::Null,
            "allowed_next_node": allowed_next_node,
            "execution_evidence": {
                "receipt_backed": true
            },
            "source_dispatch_packet_path": "packet.json"
        })
    }

    #[test]
    fn host_bridge_result_validate_passes_valid_result_without_mutation() {
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &host_bridge_validate_request(),
            std::path::Path::new("result.json"),
            &host_bridge_validate_result("epsilon_gate"),
        );

        assert_eq!(payload["mode"], "result_validate");
        assert_eq!(payload["status"], super::release1_pass_status());
        assert_eq!(payload["validation"]["accepted_completion"], true);
        assert!(payload["blocker_codes"].as_array().unwrap().is_empty());
    }

    #[test]
    fn host_bridge_result_validate_accepts_scaffolded_materialized_result() {
        let request = host_bridge_validate_request();
        let typed_request = taskflow_host_bridge::HostBridgeRequest::from_value(request.clone())
            .expect("request should type");
        let result = taskflow_host_bridge::receipt_binding::build_host_bridge_result_scaffold(
            taskflow_host_bridge::receipt_binding::HostBridgeResultScaffoldInput {
                request: typed_request,
                decision: None,
                verdict: None,
                blocker_codes: Vec::new(),
                rework_target: None,
                allowed_next_node: None,
                summary: Some("parent host adapter completed scaffold proof".to_string()),
                host_agent_id: Some("host-agent-1".to_string()),
                receipt_id: Some("receipt-1".to_string()),
            },
        );
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_pass_status());
        assert_eq!(result["status"], super::release1_pass_status());
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["source_dispatch_packet_path"], "packet.json");
        assert!(payload["blocker_codes"].as_array().unwrap().is_empty());
    }

    #[test]
    fn host_bridge_result_validate_reports_schema_field_failure() {
        let mut result = host_bridge_validate_result("epsilon_gate");
        result.as_object_mut().unwrap().remove("verdict");
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &host_bridge_validate_request(),
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_blocked_status());
        assert!(payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "host_bridge_result_missing_verdict"));
    }

    #[test]
    fn host_bridge_result_validate_reports_materialized_contract_failure() {
        let mut result = host_bridge_validate_result("epsilon_gate");
        let object = result.as_object_mut().unwrap();
        object.remove("status");
        object.remove("execution_state");
        object.remove("source_dispatch_packet_path");
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &host_bridge_validate_request(),
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_blocked_status());
        let blocker_codes = payload["blocker_codes"].as_array().unwrap();
        for expected in [
            "host_bridge_result_status_invalid",
            "host_bridge_result_execution_state_invalid",
            "host_bridge_result_source_dispatch_packet_path_missing",
        ] {
            assert!(
                blocker_codes.iter().any(|code| code == expected),
                "missing {expected}: {payload}"
            );
        }
    }

    #[test]
    fn host_bridge_result_validate_reports_illegal_next_node() {
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &host_bridge_validate_request(),
            std::path::Path::new("result.json"),
            &host_bridge_validate_result("omega_verify"),
        );

        assert_eq!(payload["status"], super::release1_blocked_status());
        assert!(payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "invalid_allowed_next_node_for_execution_plan"));
    }

    fn host_bridge_rework_backedge_request_and_result(
        result_status: &str,
    ) -> (std::path::PathBuf, serde_json::Value, serde_json::Value) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-rework-backedge-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let packet_path = root.join("packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": [
                                    "repair",
                                    "quality_gate",
                                    "verification_lane"
                                ],
                                "lane_catalog": {
                                    "repair": {
                                        "dispatch_target": "repair",
                                        "task_class": "implementation"
                                    },
                                    "quality_gate": {
                                        "dispatch_target": "quality_gate",
                                        "task_class": "quality_gate"
                                    },
                                    "verification_lane": {
                                        "dispatch_target": "verification_lane",
                                        "task_class": "verification"
                                    },
                                    "repair_rework": {
                                        "dispatch_target": "repair_rework",
                                        "task_class": "implementation"
                                    }
                                }
                            }
                        }
                    }
                }
            }))
            .expect("packet should serialize"),
        )
        .expect("packet should write");
        let mut request = host_bridge_validate_request();
        request["dispatch_target"] = serde_json::json!("quality_gate");
        request["packet_path"] = serde_json::json!(packet_path.display().to_string());
        request["allowed_next_node"] = serde_json::json!("verification_lane");
        let mut result = host_bridge_validate_result("repair_rework");
        result["status"] = serde_json::json!(result_status);
        result["execution_state"] = serde_json::json!(if result_status == "pass" {
            "executed"
        } else {
            "blocked"
        });
        result["dispatch_target"] = serde_json::json!("quality_gate");
        result["decision"] = serde_json::json!(if result_status == "pass" {
            "pass"
        } else {
            "rework_required"
        });
        result["verdict"] = serde_json::json!(if result_status == "pass" {
            "pass"
        } else {
            "rework_required"
        });
        result["completion_verdict"] = serde_json::json!(if result_status == "pass" {
            "pass"
        } else {
            "rework_required"
        });
        result["blocker_codes"] = serde_json::json!(if result_status == "pass" {
            Vec::<String>::new()
        } else {
            vec!["quality_gate_rework_required".to_string()]
        });
        result["rework_target"] = serde_json::json!("repair");
        (root, request, result)
    }

    #[test]
    fn host_bridge_result_validate_accepts_rework_backedge_to_configured_rework_lane() {
        let (root, request, result) = host_bridge_rework_backedge_request_and_result("blocked");
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_pass_status());
        assert_eq!(payload["validation"]["final_state"], "Blocked");
        assert!(
            !payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
            "{payload}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn host_bridge_result_validate_accepts_contract_declared_rework_route() {
        let (root, mut request, result) = host_bridge_rework_backedge_request_and_result("blocked");
        request["blocked_result_contract"] = serde_json::json!({
            "decision": "rework_required",
            "verdict": "rework_required",
            "allowed_next_node": "repair_rework"
        });
        let packet_path = request["packet_path"]
            .as_str()
            .map(std::path::PathBuf::from)
            .expect("packet path");
        let mut packet: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&packet_path).expect("packet should read"))
                .expect("packet should parse");
        packet["role_selection_full"]["execution_plan"]["development_flow"]["dispatch_contract"]
            ["lane_catalog"]
            .as_object_mut()
            .expect("lane catalog should be object")
            .remove("repair_rework");
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&packet).expect("packet should serialize"),
        )
        .expect("packet should write");

        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_pass_status());
        assert!(
            !payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
            "{payload}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn host_bridge_result_validate_accepts_nested_contract_declared_synthetic_rework_route() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-synthetic-rework-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let packet_path = root.join("packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": [
                                    "alpha_impl",
                                    "beta_gate",
                                    "gamma_verify"
                                ],
                                "lane_catalog": {
                                    "alpha_impl": {
                                        "dispatch_target": "alpha_impl",
                                        "task_class": "implementation"
                                    },
                                    "beta_gate": {
                                        "dispatch_target": "beta_gate",
                                        "task_class": "quality_gate"
                                    },
                                    "gamma_verify": {
                                        "dispatch_target": "gamma_verify",
                                        "task_class": "verification"
                                    }
                                }
                            }
                        }
                    }
                }
            }))
            .expect("packet should serialize"),
        )
        .expect("packet should write");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-synthetic-rework",
            "run_id": "run-synthetic-rework",
            "task_id": "task-synthetic-rework",
            "dispatch_target": "beta_gate",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "host-tool-bridge/requests/request.json",
            "result_path": "host-tool-bridge/results/result.json",
            "receipt_path": "host-tool-bridge/receipts/receipt.json",
            "allowed_next_node": "gamma_verify",
            "host_bridge": {
                "blocked_result_contract": {
                    "allowed_next_node": "alpha_rework"
                }
            }
        });
        let result = serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "schema_version": 1,
            "status": "blocked",
            "execution_state": "blocked",
            "request_id": "req-synthetic-rework",
            "run_id": "run-synthetic-rework",
            "dispatch_target": "beta_gate",
            "decision": "rework_required",
            "verdict": "rework_required",
            "completion_verdict": "rework_required",
            "blocker_codes": ["quality_gate_rework_required"],
            "rework_target": "alpha_impl",
            "allowed_next_node": "alpha_rework",
            "execution_evidence": {
                "receipt_backed": true
            },
            "source_dispatch_packet_path": packet_path.display().to_string()
        });

        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_pass_status());
        assert!(
            !payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
            "{payload}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn host_bridge_result_validate_accepts_config_derived_synthetic_rework_route() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-config-rework-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temp root should be created");
        let packet_path = root.join("packet.json");
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "role_selection_full": {
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "execution_lane_sequence": [
                                    "alpha_impl",
                                    "beta_gate",
                                    "gamma_verify"
                                ],
                                "lane_catalog": {
                                    "alpha_impl": {
                                        "dispatch_target": "alpha_impl",
                                        "task_class": "implementation"
                                    },
                                    "beta_gate": {
                                        "dispatch_target": "beta_gate",
                                        "task_class": "quality_gate"
                                    },
                                    "gamma_verify": {
                                        "dispatch_target": "gamma_verify",
                                        "task_class": "verification"
                                    }
                                }
                            }
                        }
                    }
                }
            }))
            .expect("packet should serialize"),
        )
        .expect("packet should write");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-config-rework",
            "run_id": "run-config-rework",
            "task_id": "task-config-rework",
            "dispatch_target": "beta_gate",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": "host-tool-bridge/requests/request.json",
            "result_path": "host-tool-bridge/results/result.json",
            "receipt_path": "host-tool-bridge/receipts/receipt.json",
            "allowed_next_node": "gamma_verify"
        });
        let result = serde_json::json!({
            "artifact_kind": "host_tool_bridge_result",
            "schema_version": 1,
            "status": "blocked",
            "execution_state": "blocked",
            "request_id": "req-config-rework",
            "run_id": "run-config-rework",
            "dispatch_target": "beta_gate",
            "decision": "rework_required",
            "verdict": "rework_required",
            "completion_verdict": "rework_required",
            "blocker_codes": ["quality_gate_rework_required"],
            "rework_target": "alpha_impl",
            "allowed_next_node": "alpha_impl_rework",
            "execution_evidence": {
                "receipt_backed": true
            },
            "source_dispatch_packet_path": packet_path.display().to_string()
        });

        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_pass_status());
        assert!(!payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "invalid_allowed_next_node_for_execution_plan"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn host_bridge_result_validate_rejects_synthetic_rework_route_mismatch() {
        let (root, mut request, mut result) =
            host_bridge_rework_backedge_request_and_result("blocked");
        request["blocked_result_contract"] = serde_json::json!({
            "allowed_next_node": "configured_rework_lane"
        });
        result["allowed_next_node"] = serde_json::json!("different_rework_lane");

        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_blocked_status());
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
            "{payload}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn host_bridge_result_validate_rejects_rework_backedge_without_rework_result() {
        let (root, request, result) = host_bridge_rework_backedge_request_and_result("pass");
        let payload = super::validate_host_bridge_result_dry_run(
            std::path::Path::new("request.json"),
            &request,
            std::path::Path::new("result.json"),
            &result,
        );

        assert_eq!(payload["status"], super::release1_blocked_status());
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "invalid_allowed_next_node_for_execution_plan"),
            "{payload}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    fn host_bridge_submit_result_direct_developer_role_selection(
        dev_task_id: &str,
    ) -> crate::RuntimeConsumptionLaneSelection {
        let mut selection = host_bridge_submit_result_role_selection(dev_task_id);
        selection.execution_plan["development_flow"]["dispatch_contract"]
            ["execution_lane_sequence"] =
            serde_json::json!(["developer", "coach_implementation_gate", "tester"]);
        selection.execution_plan["development_flow"]["dispatch_contract"]["lane_catalog"] = serde_json::json!({
            "analyst": {
                "dispatch_target": "analyst",
                "stage": "specification",
                "task_class": "specification",
                "closure_class": "specification",
                "completion_blocker": "pending_analysis_evidence",
                "packet_template_kind": "delivery_task_packet",
                "activation_agent_type": "middle",
                "activation_runtime_role": "business_analyst"
            },
            "developer": {
                "dispatch_target": "developer",
                "stage": "execution",
                "task_class": "implementation",
                "closure_class": "implementation",
                "completion_blocker": "pending_implementation_evidence",
                "packet_template_kind": "delivery_task_packet",
                "activation_agent_type": "junior",
                "activation_runtime_role": "worker",
                "runtime_assignment": {
                    "selected_backend_id": "internal_subagents",
                    "selected_carrier_id": "junior"
                }
            },
            "coach_implementation_gate": {
                "dispatch_target": "coach_implementation_gate",
                "stage": "coach",
                "task_class": "coach",
                "completion_blocker": "pending_review_clean_evidence",
                "packet_template_kind": "coach_review_packet",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach"
            },
            "tester": {
                "dispatch_target": "tester",
                "stage": "verification",
                "task_class": "verification",
                "completion_blocker": "pending_verification_evidence",
                "packet_template_kind": "verifier_proof_packet",
                "activation_agent_type": "senior",
                "activation_runtime_role": "verifier"
            }
        });
        selection
    }

    #[test]
    fn host_bridge_provenance_blocks_request_outside_state_root() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-forged-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("state root");
        let request_path = root.join("forged-request.json");
        std::fs::write(&request_path, b"{}").expect("request file");
        let request = serde_json::json!({
            "request_path": request_path.display().to_string(),
            "run_id": "run-1",
            "packet_path": "/tmp/attacker-packet.json"
        });
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        let blockers = runtime.block_on(host_bridge_request_provenance_blockers_for_state_root(
            &state_root,
            &request_path,
            &request,
            false,
        ));

        assert!(blockers.contains(&"host_bridge_request_untrusted_path".to_string()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_request_read_rejects_state_root_escape_and_oversized_files() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-read-path-safety-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_dir = state_root.join("host-tool-bridge/requests");
        std::fs::create_dir_all(&request_dir).expect("request dir");

        let dot_segment_path = request_dir.join("../requests/request.json");
        std::fs::write(request_dir.join("request.json"), b"{}").expect("request");
        let dot_segment_error = read_host_bridge_request(&dot_segment_path, Some(&state_root))
            .expect_err("dot-segment request should be rejected");
        assert!(dot_segment_error.contains("dot-segment"));

        let outside_path = root.join("outside/request.json");
        std::fs::create_dir_all(outside_path.parent().expect("outside parent"))
            .expect("outside parent");
        std::fs::write(&outside_path, b"{}").expect("outside request");
        let outside_error = read_host_bridge_request(&outside_path, Some(&state_root))
            .expect_err("outside request should be rejected");
        assert!(outside_error.contains("escapes VIDA state root"));

        let oversized_request_path = request_dir.join("oversized-request.json");
        std::fs::write(
            &oversized_request_path,
            vec![b'{'; (MAX_HOST_BRIDGE_ARTIFACT_BYTES as usize) + 1],
        )
        .expect("oversized request");
        let oversized_error = read_host_bridge_request(&oversized_request_path, Some(&state_root))
            .expect_err("oversized request should be rejected");
        assert!(oversized_error.contains("exceeding"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_request_relative_path_canonicalizes_before_state_write() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-relative-request-write-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_rel = std::path::PathBuf::from("host-tool-bridge/requests/request.json");
        let request_path = state_root.join(&request_rel);
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should create");
        std::fs::write(&request_path, b"{\"status\":\"pending\"}").expect("request should write");

        let canonical =
            canonical_host_bridge_request_path(&request_rel, Some(&state_root)).unwrap();
        assert_eq!(canonical, request_path.canonicalize().unwrap());
        write_host_bridge_request(
            &state_root,
            &canonical,
            &serde_json::json!({ "status": "updated" }),
        )
        .expect("canonical request path should write under state root");

        let rewritten: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&request_path).unwrap()).unwrap();
        assert_eq!(rewritten["status"], "updated");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_packet_reader_rejects_non_regular_files_and_oversized_packets() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-packet-read-safety-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("state root");

        let request_dir =
            state_root.join("runtime-consumption/downstream-dispatch-packets/request-dir");
        std::fs::create_dir_all(&request_dir).expect("request dir");
        let request_error =
            read_canonical_host_bridge_json_artifact(&request_dir, "host bridge request")
                .expect_err("request directory should be rejected");
        assert!(request_error.contains("not a regular file"));

        let packet_path =
            state_root.join("runtime-consumption/downstream-dispatch-packets/packet.json");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("packet parent");
        std::fs::write(
            &packet_path,
            vec![b'x'; (MAX_HOST_BRIDGE_ARTIFACT_BYTES as usize) + 1],
        )
        .expect("oversized packet");
        let packet_error =
            read_canonical_host_bridge_json_artifact(&packet_path, "host bridge packet")
                .expect_err("oversized packet should be rejected");
        assert!(packet_error.contains("exceeding"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_request_untrusted_path_explicit_state_dir_is_authoritative() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-authority-{}-{nanos}",
            std::process::id()
        ));
        let trusted_state_root = root.join("trusted/.vida/data/state");
        let attacker_state_root = root.join("attacker/.vida/data/state");
        let request_path = attacker_state_root.join("host-tool-bridge/requests/request.json");
        let packet_path =
            trusted_state_root.join("runtime-consumption/downstream-dispatch-packets/packet.json");
        let result_path = trusted_state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = trusted_state_root.join("host-tool-bridge/receipts/receipt.json");
        std::fs::create_dir_all(&trusted_state_root).expect("trusted state root");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("packet parent");
        std::fs::create_dir_all(result_path.parent().expect("result parent"))
            .expect("result parent");
        std::fs::create_dir_all(receipt_path.parent().expect("receipt parent"))
            .expect("receipt parent");
        std::fs::write(&packet_path, b"{}").expect("packet");
        std::fs::write(&result_path, b"{}").expect("result");
        std::fs::write(&receipt_path, b"{}").expect("receipt");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-hb-001",
            "run_id": "run-hb-001",
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        let _cwd = guard_current_dir(&root);
        let _env = EnvVarGuard::unset("VIDA_ROOT");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let exit = runtime.block_on(run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: Vec::new(),
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(trusted_state_root.clone()),
        }));

        assert_eq!(exit, ExitCode::from(1));
        let persisted_request: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&request_path).expect("read request"))
                .expect("request should remain json");
        assert_eq!(persisted_request, request);
        let persisted_result = std::fs::read_to_string(&result_path).expect("read result artifact");
        assert_eq!(persisted_result, "{}");
        let persisted_receipt =
            std::fs::read_to_string(&receipt_path).expect("read receipt artifact");
        assert_eq!(persisted_receipt, "{}");

        let blockers = runtime.block_on(host_bridge_request_provenance_blockers(
            &request_path,
            &request,
            Some(&trusted_state_root),
            false,
        ));
        assert!(blockers
            .iter()
            .any(|code| code == "host_bridge_request_untrusted_path"));
        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            blockers,
            Some(&trusted_state_root),
            false,
        );
        assert_eq!(payload["surface"], "vida agent host-bridge");
        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker codes")
            .iter()
            .any(|code| code == "host_bridge_request_untrusted_path"));
        assert!(payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("host tool calls")
            .is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_missing_receipt_blocks_pending_request() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-missing-receipt-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        std::fs::create_dir_all(&state_root).expect("state root");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let run_id = "run-hb-002";
        runtime.block_on(async {
            let store = state_store::StateStore::open(state_root.clone())
                .await
                .expect("open store");
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id: run_id,
                    title: "Host bridge missing receipt",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 1,
                    parent_id: None,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: ".",
                })
                .await
                .expect("create task");
            store
                .refresh_task_snapshot()
                .await
                .expect("refresh snapshot");
        });

        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let packet_path =
            state_root.join("runtime-consumption/downstream-dispatch-packets/packet.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &packet_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "source_dispatch_target": "implementer",
                "source_dispatch_status": "bridge_request_pending",
                "downstream_dispatch_active_target": "implementer",
                "downstream_dispatch_status": "blocked"
            }))
            .expect("packet should serialize"),
        )
        .expect("packet");
        std::fs::write(&result_path, b"{}").expect("result");
        std::fs::write(&receipt_path, b"{}").expect("receipt");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-hb-002",
            "run_id": run_id,
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");

        let _cwd = guard_current_dir(&root);
        let _env = EnvVarGuard::unset("VIDA_ROOT");
        let exit = runtime.block_on(run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: Vec::new(),
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(state_root.clone()),
        }));

        assert_eq!(exit, ExitCode::from(1));
        let blockers = runtime.block_on(host_bridge_request_provenance_blockers(
            &request_path,
            &request,
            Some(&state_root),
            false,
        ));
        assert!(blockers
            .iter()
            .any(|code| code == "host_bridge_dispatch_receipt_missing"));
        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            blockers,
            Some(&state_root),
            false,
        );
        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker codes")
            .iter()
            .any(|code| code == "host_bridge_dispatch_receipt_missing"));
        assert!(payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("host tool calls")
            .is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_submit_result_uses_operator_staged_result_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::current_dir()
            .expect("current dir")
            .join("target/tmp")
            .join(format!(
                "vida-host-bridge-complete-missing-preflight-{}-{nanos}",
                std::process::id()
            ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create host bridge state root");
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-complete-missing-preflight";
        let session_id = format!("host-bridge-submit-result-session-{nanos}");
        let _session_env = EnvVarGuard::set("VIDA_SESSION_ID", &session_id);
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge complete missing preflight receipt",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        store
            .acquire_orchestrator_claim(crate::state_store::AcquireOrchestratorClaimRequest {
                claim_id: "host-bridge-submit-result-owner".to_string(),
                state_root_id: root.display().to_string(),
                worktree_environment_id: std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .display()
                    .to_string(),
                orchestrator_session_id: session_id,
                process_id: Some(std::process::id()),
                task_id: Some(run_id.to_string()),
                run_id: Some(run_id.to_string()),
                lane_id: Some("analyst".to_string()),
                claim_kind: "write".to_string(),
                conflict_domain: Some(format!("run:{run_id}")),
                owned_paths: vec!["crates/vida/src/agent_dispatch_surface.rs".to_string()],
                read_only_paths: Vec::new(),
                lease_mode: crate::state_store::LeaseMode::Exclusive,
                lease_seconds: 60,
            })
            .await
            .expect("claim run for generated local session");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "specification",
        );
        status.task_id = run_id.to_string();
        status.active_node = "developer".to_string();
        status.next_node = Some("developer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "developer_blocked".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.developer".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let request_path = root.join("host-tool-bridge/requests/request.json");
        let receipt_packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let canonical_result_path = root.join("host-tool-bridge/results/result.json");
        let staged_result_path = root.join("host-tool-bridge/staged-results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        for path in [
            &request_path,
            &receipt_packet_path,
            &canonical_result_path,
            &staged_result_path,
            &receipt_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &receipt_packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "analyst",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "packet_id": "run-host-bridge-complete-missing-preflight::analyst::delivery",
                    "task_id": run_id,
                    "backlog_id": run_id,
                    "goal": "Complete host bridge wrapper regression",
                    "scope_in": ["host bridge wrapper completion"],
                    "read_only_paths": ["crates/vida/src/agent_dispatch_surface.rs"],
                    "definition_of_done": ["host bridge wrapper completion succeeds"],
                    "verification_command": "vida agent host-bridge --submit-result",
                    "proof_target": "executed dispatch receipt",
                    "stop_rules": ["stop after lane completion"],
                    "blocking_question": "Does wrapper completion reuse lane complete?"
                },
                "request_text": "complete host bridge wrapper regression",
                "activation_runtime_role": "worker",
                "selected_backend": "internal_subagents"
            }))
            .expect("packet should serialize"),
        )
        .expect("write packet");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-complete-missing-preflight",
            "run_id": run_id,
            "task_id": run_id,
            "dispatch_target": "source_lane",
            "task_class": "specification",
            "packet_path": receipt_packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": canonical_result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "allowed_next_node": "developer"
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        assert!(
            !canonical_result_path.exists(),
            "canonical request result path is absent to prove --submit-result honors the staged result file"
        );
        std::fs::write(
            &staged_result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "blocked",
                "execution_state": "blocked",
                "request_id": "req-complete-missing-preflight",
                "run_id": run_id,
                "dispatch_target": "analyst",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": ["host_bridge_completion_result_blocked"],
                "rework_target": "repair",
                "allowed_next_node": "developer",
                "execution_evidence": {
                    "receipt_backed": true
                },
                "source_dispatch_packet_path": receipt_packet_path.display().to_string()
            }))
            .expect("result should serialize"),
        )
        .expect("write submitted result");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "analyst".to_string(),
                dispatch_status: super::release1_blocked_status().to_string(),
                lane_status: crate::LaneStatus::LaneRunning.as_str().to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: Some(receipt_packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: Some("lane_completion_blocked_by_summary".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-23T00:00:00Z".to_string(),
            })
            .await
            .expect("record lane completion receipt");
        drop(store);

        assert!(
            super::host_bridge_complete_can_defer_missing_dispatch_receipt(&[
                "host_bridge_dispatch_receipt_missing".to_string()
            ])
        );

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: Vec::new(),
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: Some("agent-1".to_string()),
            summary: Some("parent host completed analyst bridge".to_string()),
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: Some(staged_result_path.clone()),
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: Some("host-bridge-wrapper-complete-1".to_string()),
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::from(2));
        let store = state_store::StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt")
            .expect("receipt should exist");
        assert_eq!(after.dispatch_status, super::release1_blocked_status());
        assert_eq!(after.lane_status, crate::LaneStatus::LaneBlocked.as_str());
        assert_eq!(
            after.blocker_code.as_deref(),
            Some("host_bridge_completion_result_blocked")
        );
        let dispatch_result_path = after
            .dispatch_result_path
            .as_ref()
            .expect("dispatch result path should be persisted");
        assert!(
            std::path::Path::new(dispatch_result_path).exists(),
            "dispatch result path should exist: {dispatch_result_path}"
        );
        assert!(after.downstream_dispatch_trace_path.is_some());
        let result: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dispatch_result_path).expect("canonical result readable"),
        )
        .expect("canonical result should decode");
        let submitted_result_path = std::fs::canonicalize(&staged_result_path)
            .expect("staged result should canonicalize")
            .display()
            .to_string();
        assert_eq!(
            result["submitted_result_path"],
            submitted_result_path.as_str()
        );
        assert_eq!(
            result["submitted_result_source"],
            "operator_supplied_result_file"
        );
        let receipt: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&receipt_path).expect("receipt readable"),
        )
        .expect("receipt should decode");
        assert_eq!(
            receipt["submitted_result_path"],
            submitted_result_path.as_str()
        );
        assert_eq!(
            receipt["submitted_result_source"],
            "operator_supplied_result_file"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_submit_result_pass_reconciles_autotester_lane_projection() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::current_dir()
            .expect("current dir")
            .join("target/tmp")
            .join(format!(
                "vida-host-bridge-submit-autotester-pass-{}-{nanos}",
                std::process::id()
            ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create host bridge state root");
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "activity-meeting-event-form-fields";
        let request_id = "activity-autotester-request";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Autotester submit-result pass reconciles stale projection",
                display_id: None,
                description: "",
                issue_type: "defect",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec![
                        "test/activity_meeting_event_form_fields_test.dart".to_string()
                    ],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status =
            crate::taskflow_run_graph::default_run_graph_status(run_id, "design", "design");
        status.task_id = run_id.to_string();
        status.active_node = "designer".to_string();
        status.next_node = Some("autotester".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "designer_blocked".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "awaiting_autotester".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.autotester_lane".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist stale run graph status");

        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/activity-autotester.json");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/activity-autotester-activation.json");
        let request_path = root.join("host-tool-bridge/requests/activity-autotester-request.json");
        let canonical_result_path =
            root.join("host-tool-bridge/results/activity-autotester-result.json");
        let staged_result_path =
            root.join("host-tool-bridge/staged-results/activity-autotester-result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/activity-autotester-receipt.json");
        for path in [
            &packet_path,
            &activation_result_path,
            &request_path,
            &canonical_result_path,
            &staged_result_path,
            &receipt_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "run_id": run_id,
                "source_dispatch_target": "autotester",
                "source_dispatch_status": "bridge_request_pending",
                "source_blocker_code": "host_tool_bridge_adapter_required",
                "dispatch_target": "autotester",
                "activation_runtime_role": "worker",
                "packet_template_kind": "verifier_proof_packet",
                "owned_paths": ["test/activity_meeting_event_form_fields_test.dart"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "verifier_proof_packet": {
                    "proof_goal": "Verify autotester pass can hand off to developer.",
                    "goal": "Complete the autotester lane evidence.",
                    "scope_in": ["dispatch_target:autotester"],
                    "handoff_task_class": "verification",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["test/activity_meeting_event_form_fields_test.dart"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["autotester pass advances to developer"],
                    "verification_command": "vida agent host-bridge --submit-result",
                    "proof_target": "autotester submit-result pass materializes developer packet",
                    "stop_rules": ["stop if packet contract is invalid"],
                    "blocking_question": "none"
                },
                "role_selection_full": host_bridge_submit_result_role_selection(run_id),
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "downstream_dispatch_active_target": "autotester",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_autotest_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            }))
            .expect("packet should serialize"),
        )
        .expect("write packet");
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": request_id,
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "autotester",
                "task_class": "verification",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "middle",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "receipt_mode": "host_bridge_receipt",
                "request_path": request_path.display().to_string(),
                "result_path": canonical_result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string()
            }))
            .expect("request should serialize"),
        )
        .expect("write request");
        std::fs::write(
            &staged_result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "pass",
                "execution_state": "executed",
                "request_id": request_id,
                "run_id": run_id,
                "dispatch_target": "autotester",
                "decision": "pass_to_developer",
                "verdict": "test_contract_ready_with_expected_red",
                "blocker_codes": [],
                "rework_target": null,
                "allowed_next_node": "developer",
                "execution_evidence": {
                    "receipt_backed": true,
                    "backend_id": "internal_subagents"
                },
                "source_dispatch_packet_path": packet_path.display().to_string()
            }))
            .expect("result should serialize"),
        )
        .expect("write submitted result");
        std::fs::write(
            &activation_result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "packet_path": packet_path.display().to_string(),
                    "result_path": canonical_result_path.display().to_string(),
                    "receipt_path": receipt_path.display().to_string()
                }
            }))
            .expect("activation result should serialize"),
        )
        .expect("write activation result");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "autotester".to_string(),
                dispatch_status: "bridge_request_pending".to_string(),
                lane_status: crate::LaneStatus::LaneRunning.as_str().to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: Some(activation_result_path.display().to_string()),
                blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
                downstream_dispatch_target: Some("developer".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some(
                    "after autotester evidence is recorded, activate developer".to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_autotest_evidence".to_string()],
                downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("autotester".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-30T00:00:00Z".to_string(),
            })
            .await
            .expect("record lane receipt");
        drop(store);

        let validation_exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: Vec::new(),
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: Some(staged_result_path.clone()),
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(validation_exit, ExitCode::SUCCESS);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: Vec::new(),
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: Some("agent-autotester-1".to_string()),
            summary: Some("parent host completed autotester bridge".to_string()),
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: Some(staged_result_path.clone()),
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: Some("completion-autotester-pass-1".to_string()),
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let store = state_store::StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt")
            .expect("receipt should exist");
        assert_eq!(after.dispatch_status, "executed");
        assert_eq!(after.lane_status, crate::LaneStatus::LaneCompleted.as_str());
        assert_eq!(after.blocker_code, None);
        assert_eq!(
            after.downstream_dispatch_target.as_deref(),
            Some("developer")
        );
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());

        let advanced_status = store
            .run_graph_status(run_id)
            .await
            .expect("read advanced run graph status");
        assert_eq!(advanced_status.active_node, "autotester");
        assert_eq!(advanced_status.next_node.as_deref(), Some("developer"));
        assert_eq!(advanced_status.status, "ready");
        assert_eq!(advanced_status.lifecycle_stage, "autotester_complete");
        assert_eq!(advanced_status.lane_id, "autotester_lane");
        assert_eq!(advanced_status.resume_target, "dispatch.developer_lane");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_submit_result_analyst_pass_accepts_developer_over_self_target_marker() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::current_dir()
            .expect("current dir")
            .join("target/tmp")
            .join(format!(
                "vida-host-bridge-submit-analyst-pass-{}-{nanos}",
                std::process::id()
            ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create host bridge state root");
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "ldr-074j-analyst-submit";
        let request_id = "ldr-074j-analyst-request";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Analyst submit-result pass advances to developer",
                display_id: None,
                description: "",
                issue_type: "defect",
                status: "in_progress",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida-runtime-local/src/jobs.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "specification",
        );
        status.task_id = run_id.to_string();
        status.active_node = "analyst".to_string();
        status.next_node = Some("developer".to_string());
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analyst_blocked".to_string();
        status.policy_gate = "host_tool_bridge_adapter_required".to_string();
        status.handoff_state = "none".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "none".to_string();
        status.resume_target = "dispatch.analyst".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path = root.join("runtime-consumption/dispatch-packets/ldr-074j-analyst.json");
        let activation_result_path =
            root.join("runtime-consumption/dispatch-results/ldr-074j-analyst-activation.json");
        let request_path = root.join("host-tool-bridge/requests/ldr-074j-analyst-request.json");
        let canonical_result_path =
            root.join("host-tool-bridge/results/ldr-074j-analyst-result.json");
        let staged_result_path =
            root.join("host-tool-bridge/staged-results/ldr-074j-analyst-result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/ldr-074j-analyst-receipt.json");
        for path in [
            &packet_path,
            &activation_result_path,
            &request_path,
            &canonical_result_path,
            &staged_result_path,
            &receipt_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "run_id": run_id,
                "source_dispatch_target": "analyst",
                "source_dispatch_status": "bridge_request_pending",
                "source_blocker_code": "host_tool_bridge_adapter_required",
                "dispatch_target": "analyst",
                "activation_runtime_role": "business_analyst",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": [],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "delivery_task_packet": {
                    "packet_id": "ldr-074j-analyst-submit::analyst::delivery",
                    "task_id": run_id,
                    "backlog_id": run_id,
                    "goal": "Complete analyst handoff evidence.",
                    "scope_in": ["dispatch_target:analyst"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["analyst pass advances to developer"],
                    "verification_command": "vida agent host-bridge --submit-result",
                    "proof_target": "analyst submit-result pass materializes developer packet",
                    "stop_rules": ["stop if packet contract is invalid"],
                    "blocking_question": "none"
                },
                "role_selection_full": host_bridge_submit_result_direct_developer_role_selection(run_id),
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "downstream_dispatch_target": "analyst",
                "downstream_dispatch_active_target": "analyst",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_analysis_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            }))
            .expect("packet should serialize"),
        )
        .expect("write packet");
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": request_id,
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "analyst",
                "task_class": "specification",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "carrier_id": "middle",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "receipt_mode": "host_bridge_receipt",
                "request_path": request_path.display().to_string(),
                "result_path": canonical_result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string()
            }))
            .expect("request should serialize"),
        )
        .expect("write request");
        std::fs::write(
            &staged_result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "schema_version": 1,
                "status": "pass",
                "execution_state": "executed",
                "request_id": request_id,
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "analyst",
                "decision": "pass",
                "verdict": "pass",
                "blocker_codes": [],
                "allowed_next_node": "developer",
                "execution_evidence": {
                    "receipt_backed": true,
                    "backend_id": "internal_subagents"
                },
                "source_dispatch_packet_path": packet_path.display().to_string()
            }))
            .expect("result should serialize"),
        )
        .expect("write submitted result");
        std::fs::write(
            &activation_result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "blocked",
                "execution_state": "bridge_request_pending",
                "host_tool_bridge_request": {
                    "request_path": request_path.display().to_string(),
                    "packet_path": packet_path.display().to_string(),
                    "result_path": canonical_result_path.display().to_string(),
                    "receipt_path": receipt_path.display().to_string()
                }
            }))
            .expect("activation result should serialize"),
        )
        .expect("write activation result");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "analyst".to_string(),
                dispatch_status: "bridge_request_pending".to_string(),
                lane_status: crate::LaneStatus::LaneRunning.as_str().to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: Some(activation_result_path.display().to_string()),
                blocker_code: Some("host_tool_bridge_adapter_required".to_string()),
                downstream_dispatch_target: Some("analyst".to_string()),
                downstream_dispatch_command: Some("vida agent-init".to_string()),
                downstream_dispatch_note: Some(
                    "stale self-target marker from bridge activation".to_string(),
                ),
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_analysis_evidence".to_string()],
                downstream_dispatch_packet_path: Some(packet_path.display().to_string()),
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("analyst".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("middle".to_string()),
                activation_runtime_role: Some("business_analyst".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-30T00:00:00Z".to_string(),
            })
            .await
            .expect("record lane receipt");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: Vec::new(),
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: Some("agent-analyst-1".to_string()),
            summary: Some("parent host completed analyst bridge".to_string()),
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: Some(staged_result_path.clone()),
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: Some("completion-analyst-pass-1".to_string()),
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let store = state_store::StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt")
            .expect("receipt should exist");
        assert_eq!(after.dispatch_status, "executed");
        assert_eq!(after.lane_status, crate::LaneStatus::LaneCompleted.as_str());
        assert_eq!(after.blocker_code, None);
        assert_eq!(
            after.downstream_dispatch_target.as_deref(),
            Some("developer")
        );
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());
        assert!(receipt_path.exists());

        let advanced_status = store
            .run_graph_status(run_id)
            .await
            .expect("read advanced run graph status");
        assert_eq!(advanced_status.active_node, "analyst");
        assert_eq!(advanced_status.next_node.as_deref(), Some("developer"));
        assert_eq!(advanced_status.status, "ready");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_state_root_infers_from_project_state_request_path() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-state-root-infer-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should exist");
        std::fs::write(&request_path, b"{}").expect("request should write");

        let inferred = infer_host_bridge_state_root_from_request_path(&request_path)
            .expect("state root should infer from project state request path");

        assert_eq!(
            inferred,
            std::fs::canonicalize(&state_root).expect("state root should canonicalize")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_observability_project_root_accepts_project_state_root() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let project_root = harness.path();
        std::fs::write(project_root.join("vida.config.yaml"), "project: test\n")
            .expect("project marker should write");
        std::fs::write(project_root.join("AGENTS.md"), "test project\n")
            .expect("agents marker should write");
        std::fs::create_dir_all(project_root.join(".vida/config"))
            .expect("config marker should initialize");
        std::fs::create_dir_all(project_root.join(".vida/db"))
            .expect("db marker should initialize");
        std::fs::create_dir_all(project_root.join(".vida/project"))
            .expect("project marker should initialize");
        let state_root = project_root.join(crate::state_store::default_state_dir());
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should initialize");
        std::fs::write(&request_path, b"{}").expect("request should write");

        let resolved = host_bridge_observability_project_root(None, &request_path)
            .expect("project state root should resolve to project root");

        assert_eq!(
            resolved,
            std::fs::canonicalize(project_root).expect("project root should canonicalize")
        );
        let _ = std::fs::remove_dir_all(project_root);
    }

    #[test]
    fn host_bridge_observability_project_root_skips_package_local_state_root() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let package_root = harness.path().join("crates/vida");
        let state_root = package_root.join(crate::state_store::default_state_dir());
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should initialize");
        std::fs::write(&request_path, b"{}").expect("request should write");

        let resolved = host_bridge_observability_project_root(None, &request_path);

        assert_eq!(resolved, None);
        assert!(!package_root
            .join(crate::HOST_AGENT_OBSERVABILITY_STATE)
            .exists());
        let _ = std::fs::remove_dir_all(harness.path());
    }

    #[test]
    fn host_bridge_provenance_accepts_pending_bridge_receipt() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path();
        let request_path = state_root.join("host_bridge/request.json");
        let packet_path = state_root.join("packets/run-pending.json");
        let result_path = state_root.join("host_bridge/result.json");
        let receipt_path = state_root.join("host_bridge/receipt.json");
        std::fs::create_dir_all(request_path.parent().expect("request parent"))
            .expect("request parent should be created");
        std::fs::create_dir_all(packet_path.parent().expect("packet parent"))
            .expect("packet parent should be created");
        std::fs::write(&request_path, b"{}").expect("request file should be written");
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "run_id": "run-pending",
                "dispatch_target": "implementer"
            }))
            .expect("packet should serialize"),
        )
        .expect("packet file should be written");

        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-pending",
            "run_id": "run-pending",
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let blockers = runtime.block_on(async {
            let store = crate::StateStore::open(state_root.to_path_buf())
                .await
                .expect("state store should open");
            store
                .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                    run_id: "run-pending".to_string(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "bridge_request_pending".to_string(),
                    lane_status: "lane_running".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "implementation".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: Some(packet_path.display().to_string()),
                    dispatch_result_path: None,
                    blocker_code: None,
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("internal_subagents".to_string()),
                    activation_runtime_role: Some("worker".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-04T00:00:00Z".to_string(),
                })
                .await
                .expect("pending host bridge receipt should record");
            let canonical_packet_path =
                std::fs::canonicalize(&packet_path).expect("packet path should canonicalize");
            let mut blockers = Vec::new();
            super::append_host_bridge_dispatch_receipt_blockers(
                &mut blockers,
                &store,
                state_root,
                &request,
                "run-pending",
                Some(canonical_packet_path.as_path()),
            )
            .await;
            store.close().await;
            blockers
        });

        assert!(!blockers.contains(&"host_bridge_dispatch_receipt_inactive".to_string()));
        assert_eq!(blockers, Vec::<String>::new());
    }

    #[test]
    fn host_bridge_provenance_allows_completed_pass_preview_refresh() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path();
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let packet_path =
            state_root.join("runtime-consumption/dispatch-packets/run-completed.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &packet_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "run_id": "run-completed",
                "dispatch_target": "analyst"
            }))
            .expect("packet should serialize"),
        )
        .expect("packet file should be written");
        std::fs::write(
            &result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "status": "pass",
                "execution_state": "executed",
                "request_id": "req-completed",
                "run_id": "run-completed",
                "dispatch_target": "analyst",
                "decision": "approve",
                "verdict": "pass",
                "blocker_codes": [],
                "allowed_next_node": "developer",
                "source_dispatch_packet_path": packet_path.display().to_string(),
                "execution_evidence": {
                    "receipt_backed": true
                }
            }))
            .expect("result should serialize"),
        )
        .expect("result file should be written");
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "status": "pass",
                "request_id": "req-completed",
                "run_id": "run-completed",
                "dispatch_target": "analyst",
                "allowed_next_node": "developer",
                "result_path": result_path.display().to_string()
            }))
            .expect("receipt should serialize"),
        )
        .expect("receipt file should be written");

        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pass",
            "request_id": "req-completed",
            "run_id": "run-completed",
            "dispatch_target": "source_lane",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let blockers = runtime.block_on(async {
            let store = crate::StateStore::open(state_root.to_path_buf())
                .await
                .expect("state store should open");
            store
                .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                    run_id: "run-completed".to_string(),
                    dispatch_target: "analyst".to_string(),
                    dispatch_status: "executed".to_string(),
                    lane_status: "lane_completed".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: Some(packet_path.display().to_string()),
                    dispatch_result_path: Some(result_path.display().to_string()),
                    blocker_code: None,
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: Some(result_path.display().to_string()),
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("internal_subagents".to_string()),
                    activation_runtime_role: Some("worker".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-06-23T00:00:00Z".to_string(),
                })
                .await
                .expect("completed host bridge receipt should record");
            store.close().await;
            let blockers = super::host_bridge_request_provenance_blockers_for_state_root(
                state_root,
                &request_path,
                &request,
                false,
            )
            .await;
            blockers
        });

        assert_eq!(blockers, Vec::<String>::new());
        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            Vec::new(),
            Some(&state_root),
            false,
        );
        assert_eq!(payload["status"], "pass");
        assert!(!payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_not_pending"));
    }

    #[test]
    fn host_bridge_adapter_payload_allows_retryable_blocked_completion_request() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-retryable-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let packet_path =
            state_root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &packet_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(&packet_path, b"{}").expect("packet file should be written");
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "status": "blocked",
                "request_id": "req-retry",
                "run_id": "run-retry",
                "dispatch_target": "implementer",
                "blocker_code": "implementation_artifacts_missing",
                "blocker_codes": ["implementation_artifacts_missing"],
            }))
            .expect("receipt should serialize"),
        )
        .expect("receipt file should be written");
        std::fs::write(
            &result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "status": "blocked",
                "request_id": "req-retry",
                "run_id": "run-retry",
                "dispatch_target": "implementer",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": ["implementation_artifacts_missing"],
                "rework_target": "repair",
                "allowed_next_node": "repair_rework"
            }))
            .expect("result should serialize"),
        )
        .expect("result file should be written");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "blocked",
            "request_id": "req-retry",
            "run_id": "run-retry",
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");

        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            Vec::new(),
            Some(&state_root),
            false,
        );

        assert_eq!(payload["status"], "pass");
        assert!(!payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_not_pending"));
        assert_eq!(payload["host_bridge"]["request_status"], "blocked");
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .starts_with("vida agent host-bridge --request "));
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .contains("--submit-result"));
        assert!(!payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .contains("--decision"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_retry_completion_flag_does_not_replace_retry_evidence() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-retry-flag-without-evidence-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let packet_path =
            state_root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &packet_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(&packet_path, b"{}").expect("packet file should be written");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "blocked",
            "request_id": "req-retry-flag",
            "run_id": "run-retry-flag",
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        let blockers = runtime.block_on(host_bridge_request_provenance_blockers_for_state_root(
            &state_root,
            &request_path,
            &request,
            true,
        ));

        assert!(
            blockers.contains(&"host_bridge_dispatch_receipt_missing".to_string()),
            "retry intent without blocked receipt/result evidence must keep the public evidence blocker: {blockers:?}"
        );
        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            blockers,
            Some(&state_root),
            false,
        );
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_dispatch_receipt_missing"));
        assert!(!payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .contains("--retry-completion"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_receipt_backed_retry_evidence_allows_retry_guidance() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let state_root = harness.path();
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let packet_path =
            state_root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &packet_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(
            &packet_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "run_id": "run-retry-receipt",
                "dispatch_target": "implementer"
            }))
            .expect("packet should serialize"),
        )
        .expect("packet file should be written");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "blocked",
            "request_id": "req-retry-receipt",
            "run_id": "run-retry-receipt",
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");

        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let (blockers, retry_evidence) = runtime.block_on(async {
            let store = crate::StateStore::open(state_root.to_path_buf())
                .await
                .expect("state store should open");
            store
                .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                    run_id: "run-retry-receipt".to_string(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "blocked".to_string(),
                    lane_status: "lane_blocked".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "implementation".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                    dispatch_packet_path: Some(packet_path.display().to_string()),
                    dispatch_result_path: None,
                    blocker_code: Some("implementation_artifacts_missing".to_string()),
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: vec![
                        "implementation_artifacts_missing".to_string()
                    ],
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("internal_subagents".to_string()),
                    activation_runtime_role: Some("worker".to_string()),
                    selected_backend: Some("internal_subagents".to_string()),
                    recorded_at: "2026-07-01T00:00:00Z".to_string(),
                })
                .await
                .expect("retryable blocked receipt should record");
            store.close().await;
            let canonical_packet_path =
                std::fs::canonicalize(&packet_path).expect("packet path should canonicalize");
            let retry_evidence = host_bridge_request_has_retryable_dispatch_receipt_for_state_root(
                state_root,
                &request,
                Some(canonical_packet_path.as_path()),
            )
            .await;
            let blockers = host_bridge_request_provenance_blockers_for_state_root(
                state_root,
                &request_path,
                &request,
                true,
            )
            .await;
            (blockers, retry_evidence)
        });

        assert!(retry_evidence);
        assert!(!blockers.contains(&"host_bridge_request_not_pending".to_string()));
        assert_eq!(blockers, Vec::<String>::new());
        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            blockers,
            Some(&state_root),
            retry_evidence,
        );
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .contains("--retry-completion"));
    }

    #[test]
    fn host_bridge_blocked_missing_receipt_default_surface_keeps_completion_command() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-missing",
            "run_id": "run-missing",
            "dispatch_target": "source_lane",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            vec![
                taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing
                    .as_str()
                    .to_string(),
            ],
            None,
            false,
        );

        assert_eq!(payload["status"], super::release1_blocked_status());
        assert!(payload["next_actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()));
        assert_eq!(payload["artifact_refs"]["request_path"], "request.json");
        assert!(super::host_bridge_payload_should_show_completion_command(
            &payload
        ));
        let command = payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command");
        assert!(command.starts_with("vida agent host-bridge --request request.json "));
        assert!(command.contains("--submit-result result.json"));
        assert!(!command.contains("--json"));

        let mixed_blocker_payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            vec![
                taskflow_contracts::BlockerCode::HostBridgeDispatchReceiptMissing
                    .as_str()
                    .to_string(),
                "host_bridge_result_path_unbounded".to_string(),
            ],
            None,
            false,
        );
        assert!(!super::host_bridge_payload_should_show_completion_command(
            &mixed_blocker_payload
        ));

        let other_blocker_payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            vec!["host_bridge_result_path_unbounded".to_string()],
            None,
            false,
        );
        assert!(!super::host_bridge_payload_should_show_completion_command(
            &other_blocker_payload
        ));
    }

    #[test]
    fn host_bridge_adapter_payload_rejects_non_retryable_blocked_request() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-nonretryable-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "status": "blocked",
                "blocker_codes": ["host_agent_contract_violation"]
            }))
            .expect("receipt should serialize"),
        )
        .expect("receipt file should be written");
        std::fs::write(
            &result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "status": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": ["host_agent_contract_violation"],
                "rework_target": "repair",
                "allowed_next_node": "repair_rework"
            }))
            .expect("result should serialize"),
        )
        .expect("result file should be written");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "blocked",
            "request_id": "req-terminal",
            "run_id": "run-terminal",
            "dispatch_target": "implementer",
            "packet_path": state_root.join("packets/run.json").display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");

        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            Vec::new(),
            Some(&state_root),
            false,
        );

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_not_pending"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_adapter_payload_allows_blocked_result_contract_rework_retry() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-contract-retry-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "status": "blocked",
                "blocker_codes": ["host_agent_contract_violation"]
            }))
            .expect("receipt should serialize"),
        )
        .expect("receipt file should be written");
        std::fs::write(
            &result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "status": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": ["host_agent_contract_violation"],
                "allowed_next_node": "repair_rework"
            }))
            .expect("result should serialize"),
        )
        .expect("result file should be written");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "blocked",
            "request_id": "req-contract-retry",
            "run_id": "run-contract-retry",
            "dispatch_target": "quality_gate",
            "packet_path": state_root.join("packets/run.json").display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "blocked_result_contract": {
                "decision": "rework_required",
                "verdict": "rework_required",
                "allowed_next_node": "repair_rework"
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");

        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            Vec::new(),
            Some(&state_root),
            false,
        );

        assert_eq!(payload["status"], "pass");
        assert!(!payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_not_pending"));
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .contains("--retry-completion"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_adapter_payload_allows_nested_contract_only_synthetic_rework_retry() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-nested-contract-retry-{}-{nanos}",
            std::process::id()
        ));
        let state_root = root.join(".vida/data/state");
        let request_path = state_root.join("host-tool-bridge/requests/request.json");
        let result_path = state_root.join("host-tool-bridge/results/result.json");
        let receipt_path = state_root.join("host-tool-bridge/receipts/receipt.json");
        for path in [&request_path, &result_path, &receipt_path] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact parent should be created");
        }
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_receipt",
                "status": "blocked",
                "blocker_codes": ["synthetic_gate_rework_required"]
            }))
            .expect("receipt should serialize"),
        )
        .expect("receipt file should be written");
        std::fs::write(
            &result_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "artifact_kind": "host_tool_bridge_result",
                "status": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "blocker_codes": ["synthetic_gate_rework_required"],
                "allowed_next_node": "alpha_rework"
            }))
            .expect("result should serialize"),
        )
        .expect("result file should be written");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "blocked",
            "request_id": "req-nested-contract-retry",
            "run_id": "run-nested-contract-retry",
            "dispatch_target": "beta_gate",
            "packet_path": state_root.join("packets/run.json").display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "host_bridge": {
                "blocked_result_contract": {
                    "allowed_next_node": "alpha_rework"
                }
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("request file should be written");

        let payload = host_bridge_adapter_payload(
            &request_path,
            &request,
            Vec::new(),
            Some(&state_root),
            false,
        );

        assert_eq!(payload["status"], "pass");
        assert!(!payload["blocker_codes"]
            .as_array()
            .expect("blockers")
            .iter()
            .any(|code| code == "host_bridge_request_not_pending"));
        assert!(payload["host_bridge"]["completion_command"]
            .as_str()
            .expect("completion command")
            .contains("--retry-completion"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_bridge_completion_lane_args_routes_through_lane_complete() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            Vec::new(),
            None,
            false,
        );
        let args = host_bridge_completion_lane_args(
            std::path::Path::new("request.json"),
            &payload,
            "agent-1",
            Some("completed"),
            Some("receipt-1"),
            Some(std::path::Path::new("state-dir")),
            None,
            None,
            None,
            None,
            None,
            &[],
            None,
            false,
            false,
        )
        .expect("completion lane args should render");

        assert_eq!(
            args,
            vec![
                "complete",
                "run-1",
                "--receipt-id",
                "receipt-1",
                "--host-bridge-request",
                "request.json",
                "--host-agent-id",
                "agent-1",
                "--host-bridge-summary",
                "completed",
                "--state-dir",
                "state-dir",
            ]
        );

        let json_args = host_bridge_completion_lane_args(
            std::path::Path::new("request.json"),
            &payload,
            "agent-1",
            Some("completed"),
            Some("receipt-1"),
            Some(std::path::Path::new("state-dir")),
            None,
            None,
            None,
            None,
            None,
            &[],
            None,
            false,
            true,
        )
        .expect("json completion lane args should render");

        assert!(json_args.iter().any(|arg| arg == "--json"));
    }

    #[test]
    fn host_bridge_adapter_payload_advertises_attach_for_implementation_task_class() {
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-attach",
            "run_id": "run-attach",
            "dispatch_target": "alpha_impl",
            "task_class": "implementation",
            "packet_path": "packet.json",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });

        let payload = host_bridge_adapter_payload(
            std::path::Path::new("request.json"),
            &request,
            Vec::new(),
            None,
            false,
        );

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["host_bridge"]["artifact_attach_required"], true);
        let attach = payload["host_bridge"]["artifact_attach_command"]
            .as_str()
            .expect("attach command should be present");
        assert!(attach.contains("vida agent host-bridge"));
        assert!(attach.contains("--attach-artifact"));
        assert!(attach.contains("--changed-file"));
        assert!(payload["shared_fields"]["next_actions"]
            .as_array()
            .expect("next actions")
            .iter()
            .all(|action| !action.as_str().unwrap_or_default().contains("--json")));
    }

    #[test]
    fn host_bridge_changed_files_support_json_and_explicit_raw_diff_paths() {
        let from_json = host_bridge_changed_files_from_artifact(
            Some(&serde_json::json!({
                "changed_files": [
                    "crates/vida/src/agent_dispatch_surface.rs",
                    "crates/vida/src/agent_dispatch_surface.rs",
                    ""
                ]
            })),
            &[],
        );
        assert_eq!(
            from_json,
            vec!["crates/vida/src/agent_dispatch_surface.rs".to_string()]
        );

        let from_explicit = host_bridge_changed_files_from_artifact(
            None,
            &[
                "crates/vida/src/lane_surface.rs".to_string(),
                " ".to_string(),
                "crates/vida/src/lane_surface.rs".to_string(),
            ],
        );
        assert_eq!(
            from_explicit,
            vec!["crates/vida/src/lane_surface.rs".to_string()]
        );
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_records_attempt_authority_and_updates_request() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-attach-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-attach";
        let task = store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/agent_dispatch_surface.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/patch-proposal.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["crates/vida/src/agent_dispatch_surface.rs"]
            })
            .to_string(),
        )
        .expect("write artifact");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-attach",
            "run_id": run_id,
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "implementation_isolation": {
                "owned_paths": ["crates/vida/src/agent_dispatch_surface.rs"]
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        let task_updated_at = task.updated_at.clone();
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path.clone()],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("attach-attempt-1".to_string()),
            consolidation_receipt_id: Some("attach-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request should remain json");
        let artifacts = updated["implementation_artifacts"]
            .as_array()
            .expect("implementation artifacts should be an array");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0]["artifact_kind"], "patch_proposal");
        assert_eq!(artifacts[0]["attempt_id"], "attach-attempt-1");
        assert_eq!(artifacts[0]["task_id"], run_id);
        assert_eq!(artifacts[0]["freshness"], task_updated_at);
        assert_eq!(artifacts[0]["consolidation_receipt_id"], "attach-receipt-1");
        assert_eq!(artifacts[0]["receipt_backed"], true);
        assert_eq!(
            artifacts[0]["source_artifact_ref"],
            artifact_path.display().to_string()
        );
        let normalized_artifact_refs = updated["implementation_artifact_refs"]
            .as_array()
            .expect("normalized artifact refs should be recorded");
        assert_eq!(normalized_artifact_refs.len(), 1);
        let normalized_artifact_ref = normalized_artifact_refs[0]
            .as_str()
            .expect("normalized artifact ref should be a string");
        assert!(normalized_artifact_ref.contains("host-tool-bridge"));
        assert!(std::path::Path::new(normalized_artifact_ref).exists());
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].attempt_id, "attach-attempt-1");
        assert_eq!(attempts[0].status, "accepted");
        assert_eq!(
            attempts[0].consolidation_receipt_id.as_deref(),
            Some("attach-receipt-1")
        );
        assert_eq!(
            attempts[0].artifact_refs,
            vec![normalized_artifact_ref.to_string()]
        );
        let taskflow_artifacts =
            crate::runtime_dispatch_packets::taskflow_attempt_implementation_artifacts(
                &attempts,
                &task_updated_at,
                root.as_path(),
            )
            .expect("collect implementation artifacts");
        assert_eq!(taskflow_artifacts.artifacts.len(), 1);
        assert_eq!(
            taskflow_artifacts.artifacts[0]["artifact_kind"],
            "patch_proposal"
        );
        let scope_validation =
            crate::runtime_dispatch_packets::implementation_artifact_scope_validation(
                &["crates/vida/src/agent_dispatch_surface.rs".to_string()],
                &[],
                &serde_json::Value::Array(taskflow_artifacts.artifacts),
                crate::runtime_dispatch_packets::ImplementationArtifactAuthority {
                    task_id: run_id,
                    task_updated_at: &task_updated_at,
                },
            );
        assert_eq!(scope_validation["status"], "pass");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn agent_host_bridge_attach_uses_task_owned_paths_after_request_scope_stales() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-developer-attach-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-developer-attach";
        let task = store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge developer attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/taskflow-host-bridge/src".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/developer-patch-proposal.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "decision": "pass",
                "verdict": "implemented",
                "allowed_next_node": "coach",
                "changed_files": ["crates/taskflow-host-bridge/src/completion.rs"]
            })
            .to_string(),
        )
        .expect("write artifact");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-developer-attach",
            "run_id": run_id,
            "task_id": run_id,
            "dispatch_target": "developer",
            "task_class": "implementation",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "implementation_isolation": {
                "owned_paths": ["crates/vida/src"]
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        let task_updated_at = task.updated_at.clone();
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path.clone()],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("developer-attach-attempt-1".to_string()),
            consolidation_receipt_id: Some("developer-attach-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request should remain json");
        assert_eq!(updated["dispatch_target"], "developer");
        assert_eq!(updated["task_class"], "implementation");
        let artifacts = updated["implementation_artifacts"]
            .as_array()
            .expect("implementation artifacts should be an array");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0]["attempt_id"], "developer-attach-attempt-1");
        assert_eq!(artifacts[0]["task_id"], run_id);
        assert_eq!(artifacts[0]["freshness"], task_updated_at);
        assert_eq!(
            artifacts[0]["consolidation_receipt_id"],
            "developer-attach-receipt-1"
        );
        assert_eq!(artifacts[0]["receipt_backed"], true);
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].attempt_id, "developer-attach-attempt-1");
        assert_eq!(attempts[0].status, "accepted");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_accepts_explicit_proof_artifact_scope() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-proof-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-proof-scope";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge proof scope attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["src/lib/features/list_view".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/proof-patch.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": [
                    "src/lib/features/list_view/domain/model.dart",
                    "src/test/features/list_view/domain/model_test.dart"
                ]
            })
            .to_string(),
        )
        .expect("write artifact");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-proof-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "writer",
                "task_class": "implementation",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "request_path": request_path.display().to_string(),
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string(),
                "proof_artifact_scope": ["src/test/features/list_view"],
                "implementation_isolation": {
                    "owned_paths": ["src/lib/features/list_view"]
                }
            })
            .to_string(),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("proof-scope-attempt-1".to_string()),
            consolidation_receipt_id: Some("proof-scope-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request json");
        assert_eq!(
            updated["implementation_artifacts"][0]["attempt_id"],
            "proof-scope-attempt-1"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_reports_missing_proof_scope_for_test_paths() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-missing-proof-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-missing-proof-scope";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge missing proof scope",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["src/lib/features/list_view".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/proof-patch.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["src/test/features/list_view/domain/model_test.dart"]
            })
            .to_string(),
        )
        .expect("write artifact");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-missing-proof-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "writer",
                "task_class": "implementation",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "request_path": request_path.display().to_string(),
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string(),
                "implementation_isolation": {
                    "owned_paths": ["src/lib/features/list_view"]
                }
            })
            .to_string(),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("missing-proof-scope-attempt-1".to_string()),
            consolidation_receipt_id: Some("missing-proof-scope-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::from(1));
        let updated: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&request_path).expect("read request"))
                .expect("request json");
        assert!(updated.get("implementation_artifacts").is_none());
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read attempts");
        assert!(attempts.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_accepts_packet_proof_scope_for_stale_request() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-packet-proof-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-packet-proof-scope";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge packet proof scope attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["src/lib/features/list_view".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let implementation_artifact_path = root.join("attempt-artifacts/developer-patch.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &implementation_artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "delivery_task_packet": {
                    "proof_artifact_paths": [
                        "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                        "src/test/features/list_view/data/record_chatter_repository_test.dart",
                        "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                    ],
                    "verification_commands": [
                        "flutter test src/test/features/list_view/domain/models/record_chatter_models_test.dart src/test/features/list_view/data/record_chatter_repository_test.dart src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                    ]
                }
            })
            .to_string(),
        )
        .expect("write packet");
        std::fs::write(
            &implementation_artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": [
                    "src/lib/features/list_view/domain/models/record_chatter.dart",
                    "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                    "src/test/features/list_view/data/record_chatter_repository_test.dart",
                    "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ]
            })
            .to_string(),
        )
        .expect("write implementation artifact");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-packet-proof-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "writer",
                "task_class": "implementation",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "request_path": request_path.display().to_string(),
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string(),
                "implementation_isolation": {
                    "owned_paths": ["src/lib/features/list_view"]
                }
            })
            .to_string(),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![implementation_artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("packet-proof-scope-attempt-1".to_string()),
            consolidation_receipt_id: Some("packet-proof-scope-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request json");
        assert_eq!(
            updated["implementation_artifacts"][0]["attempt_id"],
            "packet-proof-scope-attempt-1"
        );
        assert_eq!(
            updated["proof_artifact_paths"],
            serde_json::json!([
                "src/test/features/list_view/data/record_chatter_repository_test.dart",
                "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
            ])
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_derives_proof_scope_from_changed_tests_when_proof_intent_is_prose_only(
    ) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-prose-proof-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-prose-proof-scope";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge prose proof scope attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["src/lib/features/list_view".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let implementation_artifact_path = root.join("attempt-artifacts/developer-patch.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &implementation_artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "delivery_task_packet": {
                    "proof_targets": [
                        "RecordActivityType tests detect meeting from category or label",
                        "Repository tests prove meeting schedule sends partner_ids/calendar.event fields",
                        "Widget tests cover compact mini meeting form and wider full meeting form"
                    ]
                }
            })
            .to_string(),
        )
        .expect("write packet");
        std::fs::write(
            &implementation_artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": [
                    "src/lib/features/list_view/domain/models/record_chatter.dart",
                    "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                    "src/test/features/list_view/data/record_chatter_repository_test.dart",
                    "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ]
            })
            .to_string(),
        )
        .expect("write implementation artifact");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-prose-proof-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "writer",
                "task_class": "implementation",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "request_path": request_path.display().to_string(),
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string(),
                "implementation_isolation": {
                    "owned_paths": ["src/lib/features/list_view"]
                }
            })
            .to_string(),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![implementation_artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("prose-proof-scope-attempt-1".to_string()),
            consolidation_receipt_id: Some("prose-proof-scope-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request json");
        assert_eq!(
            updated["proof_artifact_paths"],
            serde_json::json!([
                "src/test/features/list_view/data/record_chatter_repository_test.dart",
                "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
            ])
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_merges_changed_tests_into_partial_proof_scope() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-partial-proof-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-partial-proof-scope";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge partial proof scope attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["src/lib/features/list_view".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let implementation_artifact_path = root.join("attempt-artifacts/developer-patch.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &implementation_artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "delivery_task_packet": {
                    "proof_targets": [
                        "RecordActivityType tests detect meeting from category or label",
                        "Repository tests prove meeting schedule sends partner_ids/calendar.event fields",
                        "Widget tests cover compact mini meeting form and wider full meeting form"
                    ]
                }
            })
            .to_string(),
        )
        .expect("write packet");
        std::fs::write(
            &implementation_artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": [
                    "src/lib/features/list_view/domain/models/record_chatter.dart",
                    "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                    "src/test/features/list_view/data/record_chatter_repository_test.dart",
                    "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ]
            })
            .to_string(),
        )
        .expect("write implementation artifact");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-partial-proof-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "writer",
                "task_class": "implementation",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "request_path": request_path.display().to_string(),
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string(),
                "proof_artifact_paths": [
                    "src/test/features/list_view/domain/models/record_chatter_models_test.dart"
                ],
                "implementation_isolation": {
                    "owned_paths": ["src/lib/features/list_view"],
                    "proof_artifact_paths": [
                        "src/test/features/list_view/domain/models/record_chatter_models_test.dart"
                    ]
                }
            })
            .to_string(),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![implementation_artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("partial-proof-scope-attempt-1".to_string()),
            consolidation_receipt_id: Some("partial-proof-scope-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request json");
        assert_eq!(
            updated["proof_artifact_paths"],
            serde_json::json!([
                "src/test/features/list_view/data/record_chatter_repository_test.dart",
                "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
            ])
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_rejects_attempt_derived_proof_scope() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-attempt-proof-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-attempt-proof-scope";
        let task = store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attempt proof scope attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["src/lib/features/list_view".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let upstream_proof_artifact_path =
            root.join("attempt-artifacts/upstream-proof-result.json");
        let implementation_artifact_path = root.join("attempt-artifacts/developer-patch.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &upstream_proof_artifact_path,
            &implementation_artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &upstream_proof_artifact_path,
            serde_json::json!({
                "status": "pass",
                "execution_state": "executed",
                "proof_targets": [
                    "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                    "src/test/features/list_view/data/record_chatter_repository_test.dart",
                    "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ],
                "verification_commands": [
                    "flutter test src/test/features/list_view/domain/models/record_chatter_models_test.dart src/test/features/list_view/data/record_chatter_repository_test.dart src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ]
            })
            .to_string(),
        )
        .expect("write upstream proof artifact");
        store
            .record_task_attempt(crate::state_store::RecordTaskAttemptRequest {
                attempt_id: Some("proof-attempt-1".to_string()),
                task_id: run_id.to_string(),
                stage_id: "proof".to_string(),
                backend: "host_tool_bridge".to_string(),
                model_profile: "proof-agent".to_string(),
                isolation: "proof_contract".to_string(),
                freshness: Some(task.updated_at.clone()),
                status: "accepted".to_string(),
                artifact_refs: vec![upstream_proof_artifact_path.display().to_string()],
                consolidation_receipt_id: Some("proof-receipt-1".to_string()),
                selected_model_profile_readiness_status: None,
                budget_posture: None,
                cap_posture: None,
                write_scope_classification: None,
            })
            .await
            .expect("record upstream proof attempt");
        std::fs::write(
            &implementation_artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": [
                    "src/lib/features/list_view/domain/models/record_chatter.dart",
                    "src/test/features/list_view/domain/models/record_chatter_models_test.dart",
                    "src/test/features/list_view/data/record_chatter_repository_test.dart",
                    "src/test/features/list_view/presentation/stac/widgets/record_detail_view_test.dart"
                ]
            })
            .to_string(),
        )
        .expect("write implementation artifact");
        std::fs::write(
            &request_path,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-attempt-proof-scope",
                "run_id": run_id,
                "task_id": run_id,
                "dispatch_target": "writer",
                "task_class": "implementation",
                "packet_path": packet_path.display().to_string(),
                "backend_id": "internal_subagents",
                "dispatch_transport": "host_tool_bridge",
                "request_path": request_path.display().to_string(),
                "result_path": result_path.display().to_string(),
                "receipt_path": receipt_path.display().to_string(),
                "implementation_isolation": {
                    "owned_paths": ["src/lib/features/list_view"]
                }
            })
            .to_string(),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![implementation_artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("implementation-attempt-1".to_string()),
            consolidation_receipt_id: Some("implementation-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_ne!(exit, ExitCode::SUCCESS);
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert!(
            attempts.is_empty(),
            "attempt-derived proof paths must not authorize implementation artifacts"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_accepts_closed_task_receipt_backed_request_authority() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-closed-attach-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-closed-attach";
        let task = store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge closed task attach",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/taskflow-host-bridge/src".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/developer-patch-proposal.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["crates/taskflow-host-bridge/src/completion.rs"]
            })
            .to_string(),
        )
        .expect("write artifact");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-closed-developer-attach",
            "run_id": run_id,
            "task_id": run_id,
            "dispatch_target": "developer",
            "task_class": "implementation",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "implementation_isolation": {
                "owned_paths": ["crates/taskflow-host-bridge/src"]
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        let task_updated_at = task.updated_at.clone();
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("closed-attach-attempt-1".to_string()),
            consolidation_receipt_id: Some("closed-attach-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::SUCCESS);
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request should remain json");
        let artifacts = updated["implementation_artifacts"]
            .as_array()
            .expect("implementation artifacts should be an array");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0]["attempt_id"], "closed-attach-attempt-1");
        assert_eq!(artifacts[0]["task_id"], run_id);
        assert_eq!(artifacts[0]["freshness"], task_updated_at);
        assert_eq!(
            artifacts[0]["consolidation_receipt_id"],
            "closed-attach-receipt-1"
        );
        assert_eq!(artifacts[0]["receipt_backed"], true);
        assert_eq!(
            updated["implementation_artifact_authority"],
            serde_json::Value::Null
        );
        let refs = updated["implementation_artifact_refs"]
            .as_array()
            .expect("artifact refs should be recorded");
        assert_eq!(refs.len(), 1);
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert!(
            attempts.is_empty(),
            "closed task attach must not create a new task attempt"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn host_bridge_attach_artifact_blocks_symlinked_normalized_artifact_directory() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-attach-symlink-parent-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-attach-symlink-parent";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attach symlink parent",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/vida/src/agent_dispatch_surface.rs".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/patch-proposal.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["crates/vida/src/agent_dispatch_surface.rs"]
            })
            .to_string(),
        )
        .expect("write artifact");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-attach-symlink-parent",
            "run_id": run_id,
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "implementation_isolation": {
                "owned_paths": ["crates/vida/src/agent_dispatch_surface.rs"]
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        store
            .record_run_graph_dispatch_receipt(&RunGraphDispatchReceipt {
                run_id: run_id.to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "bridge_request_pending".to_string(),
                lane_status: "lane_running".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "implementation".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: Some(packet_path.display().to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("internal_subagents".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-06-11T00:00:00Z".to_string(),
            })
            .await
            .expect("seed host bridge dispatch receipt");

        let normalized_artifact_path = host_bridge_normalized_implementation_artifact_path(
            &root,
            "attach-attempt-1",
            0,
            "patch_proposal",
        );
        let normalized_artifact_name = normalized_artifact_path
            .file_name()
            .expect("normalized artifact name")
            .to_owned();
        let outside_dir = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-outside-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&outside_dir).expect("create outside dir");
        let outside_artifact_path = outside_dir.join(&normalized_artifact_name);
        std::fs::write(&outside_artifact_path, "outside-sentinel").expect("seed outside artifact");
        std::os::unix::fs::symlink(
            &outside_dir,
            normalized_artifact_path
                .parent()
                .expect("normalized artifact parent"),
        )
        .expect("create symlinked normalized artifact directory");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path.clone()],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("attach-attempt-1".to_string()),
            consolidation_receipt_id: Some("attach-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::from(1));
        assert_eq!(
            std::fs::read_to_string(&outside_artifact_path).expect("read outside artifact"),
            "outside-sentinel"
        );
        let updated: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&request_path).expect("read updated request"),
        )
        .expect("request should remain json");
        assert!(updated.get("implementation_artifacts").is_none());
        assert!(updated.get("implementation_artifact_refs").is_none());
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert!(attempts.is_empty());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_blocks_without_changed_files() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-attach-missing-files-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-attach-missing-files";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attach missing files",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/patch.diff");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(&artifact_path, "diff --git a/file b/file").expect("write diff artifact");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-missing-files",
            "run_id": run_id,
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string()
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: None,
            consolidation_receipt_id: None,
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::from(1));
        let unchanged: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&request_path).expect("read request"))
                .expect("request should remain json");
        assert!(unchanged.get("implementation_artifacts").is_none());
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert!(attempts.is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn host_bridge_attach_artifact_blocks_changed_files_outside_owned_paths() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-agent-host-bridge-attach-out-of-scope-{}-{nanos}",
            std::process::id()
        ));
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("open store");
        let run_id = "run-host-bridge-attach-out-of-scope";
        store
            .create_task_with_fixture_parent(CreateTaskRequest {
                task_id: run_id,
                title: "Host bridge attach out of scope",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata {
                    owned_paths: vec!["crates/taskflow-host-bridge".to_string()],
                    ..Default::default()
                },
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create task");
        let request_path = root.join("host-tool-bridge/requests/request.json");
        let packet_path = root.join("runtime-consumption/downstream-dispatch-packets/run.json");
        let result_path = root.join("host-tool-bridge/results/result.json");
        let receipt_path = root.join("host-tool-bridge/receipts/receipt.json");
        let artifact_path = root.join("attempt-artifacts/patch-proposal.json");
        for path in [
            &request_path,
            &packet_path,
            &result_path,
            &receipt_path,
            &artifact_path,
        ] {
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("create artifact parent");
        }
        std::fs::write(&packet_path, "{}").expect("write packet");
        std::fs::write(
            &artifact_path,
            serde_json::json!({
                "artifact_kind": "patch_proposal",
                "changed_files": ["crates/vida/src/agent_dispatch_surface.rs"]
            })
            .to_string(),
        )
        .expect("write artifact");
        let request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-attach-out-of-scope",
            "run_id": run_id,
            "dispatch_target": "implementer",
            "packet_path": packet_path.display().to_string(),
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "request_path": request_path.display().to_string(),
            "result_path": result_path.display().to_string(),
            "receipt_path": receipt_path.display().to_string(),
            "implementation_isolation": {
                "owned_paths": ["crates/taskflow-host-bridge"]
            }
        });
        std::fs::write(
            &request_path,
            serde_json::to_vec_pretty(&request).expect("request should serialize"),
        )
        .expect("write request");
        drop(store);

        let exit = run_agent_host_bridge(AgentHostBridgeArgs {
            request: request_path.clone(),
            attach_artifacts: vec![artifact_path],
            artifact_kind: "patch_proposal".to_string(),
            changed_files: Vec::new(),
            attempt_id: Some("attach-attempt-1".to_string()),
            consolidation_receipt_id: Some("attach-receipt-1".to_string()),
            complete: false,
            host_agent_id: None,
            summary: None,
            decision: None,
            verdict: None,
            allowed_next_node: None,
            blocker_codes: None,
            blocker_code: Vec::new(),
            rework_target: None,
            submit_result: None,
            validate_result: None,
            scaffold_result: None,
            retry_completion: false,
            result_file: None,
            receipt_id: None,
            json: true,
            state_dir: Some(root.clone()),
        })
        .await;

        assert_eq!(exit, ExitCode::from(1));
        let unchanged: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&request_path).expect("read request"))
                .expect("request should remain json");
        assert!(unchanged.get("implementation_artifacts").is_none());
        assert!(unchanged.get("implementation_artifact_refs").is_none());
        let store = state_store::StateStore::open(root.clone())
            .await
            .expect("reopen store");
        let attempts = store
            .task_stage_attempts(run_id, "implementation")
            .await
            .expect("read implementation attempts");
        assert!(attempts.is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn agent_dispatch_next_scheduler_keeps_explicit_binding_implicit() {
        let resolved =
            resolve_agent_dispatch_next_current_task_ids(None, Some("explicit-bound"), None);

        assert_eq!(resolved.preview_current_task_id, Some("explicit-bound"));
        assert_eq!(resolved.scheduler_current_task_id, Some("explicit-bound"));
    }

    #[test]
    fn agent_dispatch_next_scheduler_preserves_operator_requested_current_task() {
        let resolved = resolve_agent_dispatch_next_current_task_ids(
            Some("requested"),
            Some("explicit-bound"),
            Some("single-in-progress"),
        );

        assert_eq!(resolved.preview_current_task_id, Some("requested"));
        assert_eq!(resolved.scheduler_current_task_id, Some("requested"));
    }

    #[test]
    fn agent_dispatch_next_scheduler_preserves_single_in_progress_fallback() {
        let resolved =
            resolve_agent_dispatch_next_current_task_ids(None, None, Some("single-in-progress"));

        assert_eq!(resolved.preview_current_task_id, Some("single-in-progress"));
        assert_eq!(
            resolved.scheduler_current_task_id,
            Some("single-in-progress")
        );
    }

    #[test]
    fn agent_dispatch_next_current_task_uses_explicit_run_graph_binding() {
        let binding = state_store::RunGraphContinuationBinding {
            run_id: "run-active".to_string(),
            task_id: "task-active".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "run_id": "run-active",
                "task_id": "task-active",
                "active_node": "coach_implementation_gate"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "active run graph".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
            request_text: None,
            recorded_at: "2026-06-25T00:00:00Z".to_string(),
        };

        assert_eq!(
            agent_dispatch_next_bound_current_task_id(Some(&binding), None, None),
            Some("task-active".to_string())
        );
    }

    #[test]
    fn agent_dispatch_next_current_task_uses_active_exception_takeover_status() {
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-active",
            "runtime_defect",
            "runtime_defect",
        );
        status.task_id = "task-active".to_string();
        status.active_node = "coach_implementation_gate".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "coach_implementation_gate_blocked".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        let dispatch =
            state_store::RunGraphDispatchReceiptSummary::from_receipt(RunGraphDispatchReceipt {
                run_id: "run-active".to_string(),
                dispatch_target: "coach_implementation_gate".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_exception_takeover".to_string(),
                supersedes_receipt_id: Some("developer-receipt".to_string()),
                exception_path_receipt_id: Some("exception-takeover-receipt".to_string()),
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida lane exception-takeover".to_string()),
                dispatch_command: Some("vida lane exception-takeover run-active".to_string()),
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: Some("configured_backend_dispatch_failed".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("vibe_cli".to_string()),
                activation_runtime_role: Some("coach".to_string()),
                selected_backend: Some("vibe_cli".to_string()),
                recorded_at: "2026-06-25T00:00:00Z".to_string(),
            });

        assert_eq!(
            agent_dispatch_next_bound_current_task_id(None, Some(&status), Some(&dispatch)),
            Some("task-active".to_string())
        );
    }

    #[test]
    fn agent_dispatch_next_preserves_bound_current_task_when_projection_drops_it() {
        let mut projection = TaskSchedulingProjection {
            current_task_id: Some("unrelated-ready".to_string()),
            ready: vec![candidate_with_type(
                "unrelated-ready",
                "Unrelated ready",
                true,
                true,
                "task",
            )],
            blocked: vec![candidate_with_type(
                "runtime-defect-active",
                "Active runtime defect",
                false,
                false,
                "runtime_defect",
            )],
            parallel_candidates_after_current: Vec::new(),
        };

        agent_dispatch_next_preserve_current_task_id(
            &mut projection,
            Some("runtime-defect-active"),
        );

        assert_eq!(
            projection.current_task_id.as_deref(),
            Some("runtime-defect-active")
        );
    }

    trait StateStoreFixtureTaskExt {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        >;
    }

    impl StateStoreFixtureTaskExt for crate::StateStore {
        fn create_task_with_fixture_parent<'a>(
            &'a self,
            request: crate::state_store::CreateTaskRequest<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::state_store::TaskRecord,
                            crate::state_store::StateStoreError,
                        >,
                    > + 'a,
            >,
        > {
            Box::pin(async move {
                let crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id,
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                } = request;
                let generated_parent_id = (issue_type != "epic" && parent_id.is_none())
                    .then(|| format!("{task_id}-fixture-parent"));
                if let Some(parent_task_id) = generated_parent_id.as_deref() {
                    let parent_labels: Vec<String> = Vec::new();
                    let parent_status = if matches!(status.trim(), "closed" | "completed") {
                        "closed"
                    } else {
                        "open"
                    };
                    self.create_task(crate::state_store::CreateTaskRequest {
                        task_id: parent_task_id,
                        title: "Fixture parent epic",
                        display_id: None,
                        description: "Test-only parent epic for strict task hierarchy fixtures",
                        issue_type: "epic",
                        status: parent_status,
                        priority,
                        parent_id: None,
                        labels: &parent_labels,
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by,
                        source_repo,
                    })
                    .await?;
                }
                self.create_task(crate::state_store::CreateTaskRequest {
                    task_id,
                    title,
                    display_id,
                    description,
                    issue_type,
                    status,
                    priority,
                    parent_id: parent_id.or(generated_parent_id.as_deref()),
                    labels,
                    execution_semantics,
                    planner_metadata,
                    created_by,
                    source_repo,
                })
                .await
            })
        }
    }

    fn task_with_labels(id: &str, title: &str, labels: &[&str]) -> TaskRecord {
        task_with_labels_and_type(id, title, labels, "task")
    }

    fn task_with_labels_and_type(
        id: &str,
        title: &str,
        labels: &[&str],
        issue_type: &str,
    ) -> TaskRecord {
        TaskRecord {
            id: id.to_string(),
            display_id: None,
            title: title.to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 2,
            issue_type: issue_type.to_string(),
            created_at: "2026-04-24T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-04-24T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: labels.iter().map(|label| label.to_string()).collect(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: state_store::TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn candidate(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
    ) -> TaskSchedulingCandidate {
        candidate_with_labels(
            id,
            title,
            ready_now,
            ready_parallel_safe,
            parallel_blockers,
            &[],
        )
    }

    fn candidate_with_labels(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        parallel_blockers: Vec<String>,
        labels: &[&str],
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels(id, title, labels),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers,
        }
    }

    fn candidate_with_type(
        id: &str,
        title: &str,
        ready_now: bool,
        ready_parallel_safe: bool,
        issue_type: &str,
    ) -> TaskSchedulingCandidate {
        TaskSchedulingCandidate {
            task: task_with_labels_and_type(id, title, &[], issue_type),
            ready_now,
            ready_parallel_safe,
            blocked_by: Vec::new(),
            active_critical_path: false,
            parallel_blockers: Vec::new(),
        }
    }

    #[test]
    fn single_in_progress_task_id_from_rows_selects_only_bounded_active_task() {
        let mut active = task_with_labels_and_type("task-active", "Active task", &[], "task");
        active.status = "in_progress".to_string();
        let mut epic = task_with_labels_and_type("epic-active", "Active epic", &[], "epic");
        epic.status = "in_progress".to_string();
        let mut todo = task_with_labels_and_type("todo-active", "Active todo", &[], "todo");
        todo.status = "in_progress".to_string();

        assert_eq!(
            single_in_progress_task_id_from_rows(&[epic, todo, active]),
            Some("task-active")
        );
    }

    #[test]
    fn single_in_progress_task_id_from_rows_fails_closed_for_multiple_active_tasks() {
        let mut first = task_with_labels_and_type("task-first", "First task", &[], "task");
        first.status = "in_progress".to_string();
        let mut second = task_with_labels_and_type("task-second", "Second task", &[], "task");
        second.status = "in_progress".to_string();

        assert_eq!(single_in_progress_task_id_from_rows(&[first, second]), None);
    }

    #[test]
    fn single_in_progress_task_id_from_rows_fails_closed_without_active_task() {
        assert_eq!(
            single_in_progress_task_id_from_rows(&[task_with_labels_and_type(
                "task-open",
                "Open task",
                &[],
                "task",
            )]),
            None
        );
    }

    fn activation_bundle_with_worker_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "junior",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation", "verification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "write_scope": "scoped_only",
                        "model_profiles": {
                            "gpt-5.5-low": {
                                "profile_id": "gpt-5.5-low",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation", "verification"],
                                "normalized_cost_units": 1
                            }
                        }
                    },
                    {
                        "role_id": "coach-seat",
                        "tier": "middle",
                        "default_runtime_role": "coach",
                        "runtime_roles": ["coach"],
                        "task_classes": ["coach"],
                        "normalized_cost_units": 3,
                        "quality_tier": "medium",
                        "write_scope": "read_only",
                        "model_profiles": {
                            "coach-profile": {
                                "profile_id": "coach-profile",
                                "model_ref": "gpt-5.5-coach",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "read-only",
                                "write_scope": "read_only",
                                "runtime_roles": ["coach"],
                                "task_classes": ["coach"],
                                "normalized_cost_units": 3
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "junior": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational"
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_dev_team_selection_truth() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "analyst-seat",
                        "tier": "senior",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "normalized_cost_units": 1,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["specification"],
                        "model_profiles": {
                            "analyst": {
                                "profile_id": "analyst-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["business_analyst"],
                                "task_classes": ["specification"],
                                "normalized_cost_units": 1
                            }
                        }
                    },
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "normalized_cost_units": 1,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    },
                    {
                        "role_id": "coach-seat",
                        "tier": "middle",
                        "default_runtime_role": "coach",
                        "runtime_roles": ["coach"],
                        "task_classes": ["coach"],
                        "normalized_cost_units": 3,
                        "quality_tier": "medium",
                        "reasoning_band": "medium",
                        "task_classes_for_runtime": ["coach"],
                        "model_profiles": {
                            "coach": {
                                "profile_id": "coach-profile",
                                "model_ref": "gpt-5.5-coach",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["coach"],
                                "task_classes": ["coach"],
                                "normalized_cost_units": 3
                            }
                        }
                    },
                    {
                        "role_id": "verifier-seat",
                        "tier": "middle",
                        "default_runtime_role": "verifier",
                        "runtime_roles": ["verifier", "prover"],
                        "task_classes": ["verification"],
                        "normalized_cost_units": 4,
                        "quality_tier": "high",
                        "reasoning_band": "high",
                        "task_classes_for_runtime": ["verification"],
                        "model_profiles": {
                            "prover": {
                                "profile_id": "verifier-profile",
                                "model_ref": "gpt-5.3",
                                "provider": "openai",
                                "reasoning_effort": "high",
                                "quality_tier": "high",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["verifier", "prover"],
                                "task_classes": ["verification"],
                                "normalized_cost_units": 4
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "analyst-seat": {
                            "effective_score": 70,
                            "lifecycle_state": "active"
                        },
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        },
                        "coach-seat": {
                            "effective_score": 74,
                            "lifecycle_state": "active"
                        },
                        "verifier-seat": {
                            "effective_score": 76,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_role_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_model_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "",
                                "model_ref": "",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "normalized_cost_units": 2
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_price_data_blocked() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": false
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn activation_bundle_with_missing_price_data() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "developer-seat",
                        "tier": "junior",
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "model_profiles": {
                            "developer": {
                                "profile_id": "developer-profile",
                                "model_ref": "gpt-5.5",
                                "provider": "openai",
                                "reasoning_effort": "low",
                                "quality_tier": "medium",
                                "speed_tier": "fast",
                                "sandbox_mode": "workspace-write",
                                "write_scope": "scoped_only",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"]
                            }
                        }
                    }
                ],
                "dispatch_aliases": [],
                "worker_strategy": {
                    "selection_policy": {
                        "demotion_score": 45
                    },
                    "agents": {
                        "developer-seat": {
                            "effective_score": 72,
                            "lifecycle_state": "active"
                        }
                    },
                    "store_path": ".vida/state/worker-strategy.json",
                    "scorecards_path": ".vida/state/worker-scorecards.json"
                },
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles",
                    "budget_policy": "informational",
                    "free_profiles_allowed": true
                }
            },
            "agent_system": {
                "max_parallel_agents": 4
            }
        })
    }

    fn assertion_message_contains_actionable_blocker(blocker_codes: &[String], task_id: &str) {
        let expected_prefix =
            format!("selected_lane_runtime_assignment_truth_missing:task={task_id}:");
        assert!(blocker_codes
            .iter()
            .any(|code| code.starts_with(&expected_prefix)));
    }

    #[test]
    fn agent_dispatch_next_preview_selects_parallel_safe_lanes_with_commands() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            2,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            false,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 2);
        assert_eq!(preview.configured_max_parallel_agents, 4);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert_eq!(preview.selected_lanes[0].task_class, "implementation");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "junior"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert!(preview.selected_lanes[0]
            .selection_truth
            .selection_source_paths["selected_rate"]
            .as_str()
            .is_some_and(
                |path| path.starts_with("carrier_runtime.roles[junior].model_profiles.")
                    && path.ends_with(".normalized_cost_units")
            ));
        assert_eq!(
            preview.selected_lanes[0].selection_truth.pricing_readiness["pricing_freshness_status"],
            "missing"
        );
        assert!(preview.selected_lanes[1]
            .dispatch_command
            .contains("--state-dir /tmp/vida-state"));
        assert_eq!(
            preview.parallelization_planner["status"],
            "proposals_available"
        );
        assert_eq!(preview.fanout_guard["status"], "pass");
        assert_eq!(preview.fanout_guard["lanes_selected"], 2);
        assert_eq!(preview.fanout_guard["ready_parallel_safe_count"], 2);
        assert_eq!(
            preview.fanout_guard["host_bridge_capacity"]["blocked_result_code"],
            "host_agent_capacity_unavailable"
        );
        assert_eq!(
            preview.parallelization_planner["materializes_packets"],
            false
        );
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| proposals.len() == 2));
        assert_eq!(
            preview.carrier_selection_api["surface"],
            "vida agent select"
        );
        assert_eq!(preview.carrier_selection_api["status"], "pass");
        assert!(preview.carrier_selection_api["first_class_carriers"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["api_id"] == "junior")));
    }

    #[test]
    fn agent_dispatch_next_preview_blocks_no_ready_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: None,
            ready: Vec::new(),
            blocked: vec![candidate(
                "task-blocked",
                "Blocked",
                false,
                false,
                vec!["graph_blocked".to_string()],
            )],
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert_eq!(preview.blocker_codes, vec!["no_ready_task_candidates"]);
        assert_eq!(preview.blocked_candidates[0].task_id, "task-blocked");
    }

    #[test]
    fn agent_dispatch_next_preview_selects_primary_and_reports_unsafe_parallel_candidates() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate(
                    "task-b",
                    "Task B",
                    true,
                    false,
                    vec!["execution_mode_not_parallel_safe".to_string()],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(
            preview.selected_lanes[0].dispatch_command_kind,
            "startup_activation_view_only"
        );
        assert!(preview.selected_lanes[0]
            .receipt_backed_execution_command
            .contains("--execute-dispatch"));
        assert!(preview.blocker_codes.is_empty());
        assert_eq!(preview.blocked_candidates[0].task_id, "task-b");
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("remain blocked candidates and are not selected")));
    }

    #[test]
    fn agent_dispatch_next_preview_clamps_requested_lanes_to_configured_max() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, false, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
                candidate("task-c", "Task C", true, true, Vec::new()),
                candidate("task-d", "Task D", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            4,
            2,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview");
        assert_eq!(preview.lanes_requested, 4);
        assert_eq!(preview.configured_max_parallel_agents, 2);
        assert_eq!(preview.effective_max_parallel_agents, 2);
        assert_eq!(preview.lanes_selected, 2);
        assert!(!preview.execute_supported);
        assert!(!preview.execution_attempted);
        assert_eq!(preview.selected_lanes[0].task_id, "task-a");
        assert_eq!(preview.selected_lanes[1].task_id, "task-b");
        assert!(preview.blocked_candidates.iter().any(
            |candidate| candidate.reasons == vec!["effective_max_parallel_agents_cap_reached"]
        ));
        assert_eq!(preview.fanout_guard["effective_max_parallel_agents"], 2);
        assert_eq!(preview.fanout_guard["cap_limited_rejected_count"], 2);
    }

    #[test]
    fn agent_dispatch_next_preview_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({}),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
    }

    #[test]
    fn dev_team_sequence_uses_configured_flow_ordered_step_overrides() {
        let sequence = dev_team_sequence(&serde_json::json!({
            "dev_team_readiness": {
                "default_flow_id": "debug_flow",
                "roles": [
                    {
                        "role_id": "analyst",
                        "runtime_role": "business_analyst",
                        "task_classes": ["specification"]
                    },
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "task_classes": ["implementation"]
                    }
                ],
                "sequence": ["developer"],
                "flows": [
                    {
                        "flow_id": "debug_flow",
                        "enabled": true,
                        "default": true,
                        "ordered_steps": [
                            {
                                "role_id": "analyst",
                                "runtime_role": "solution_architect",
                                "task_class": "architecture"
                            },
                            {
                                "role_id": "developer"
                            }
                        ]
                    }
                ]
            }
        }));

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].role_label, "analyst");
        assert_eq!(sequence[0].runtime_role, "solution_architect");
        assert_eq!(sequence[0].task_class, "architecture");
        assert_eq!(sequence[1].role_label, "developer");
        assert_eq!(sequence[1].runtime_role, "worker");
        assert_eq!(sequence[1].task_class, "implementation");
    }

    #[test]
    fn development_flow_binding_selects_sequence_by_work_item_type() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                        {"role_id": "test_author", "runtime_role": "worker", "task_classes": ["test_authoring"]},
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [
                                {"role_id": "developer"}
                            ]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [
                                {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                                {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                            ]
                        }
                    ]
                }
            }),
            "defect",
        );

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence[0].role_label, "analyst");
        assert_eq!(sequence[0].task_class, "specification");
        assert_eq!(sequence[1].role_label, "tester");
        assert_eq!(sequence[1].task_class, "verification");
    }

    #[test]
    fn configured_dev_team_route_selects_current_task_class_slice_for_generic_task() {
        let mut task = task_with_labels(
            "implementation-task",
            "Implement design-backed configured feature",
            &[],
        );
        task.planner_metadata.owned_paths = vec!["src/lib.rs".to_string()];
        let route = configured_dev_team_first_step_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery"
                    },
                    "roles": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                        {"role_id": "test_author", "runtime_role": "worker", "task_classes": ["test_authoring"]},
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "work_item_bindings": ["task"],
                            "ordered_steps": [
                                {"role_id": "analyst"},
                                {"role_id": "test_author"},
                                {"role_id": "developer"},
                                {"role_id": "tester"}
                            ]
                        }
                    ]
                }
            }),
            &task,
        )
        .expect("configured generic implementation task should resolve a route");

        assert_eq!(route.role_label, "test_author");
        assert_eq!(route.dispatch_target, "test_author");
        assert_eq!(route.runtime_role, "worker");
        assert_eq!(route.task_class, "test_authoring");
    }

    #[test]
    fn development_flow_binding_selects_sequence_by_canonical_work_item_alias() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "tester");
        assert_eq!(sequence[0].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_prefers_explicit_alias_binding_over_canonical_binding() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair",
                        "bug": "bug_triage"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                        {"role_id": "triager", "runtime_role": "business_analyst", "task_classes": ["triage"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        },
                        {
                            "flow_id": "bug_triage",
                            "enabled": true,
                            "work_item_bindings": ["bug"],
                            "ordered_steps": [{"role_id": "triager"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "triager");
        assert_eq!(sequence[0].task_class, "triage");
    }

    #[test]
    fn development_flow_fallback_prefers_explicit_alias_binding_over_canonical_binding() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]},
                        {"role_id": "triager", "runtime_role": "business_analyst", "task_classes": ["triage"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        },
                        {
                            "flow_id": "bug_triage",
                            "enabled": true,
                            "work_item_bindings": ["bug"],
                            "ordered_steps": [{"role_id": "triager"}]
                        }
                    ]
                }
            }),
            "bug",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "triager");
        assert_eq!(sequence[0].task_class, "triage");
    }

    #[test]
    fn task_sequence_skips_default_on_inferred_key_miss_before_canonical_work_item() {
        let task = task_with_labels_and_type(
            "defect-review",
            "Verify defect remediation",
            &["verification"],
            "defect",
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "tester");
        assert_eq!(sequence[0].runtime_role, "verifier");
        assert_eq!(sequence[0].task_class, "verification");
    }

    #[test]
    fn development_flow_binding_prefers_task_class_for_generic_task_kind() {
        let task = task_with_labels(
            "architecture-task",
            "Architecture migration task",
            &["architecture"],
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery",
                        "architecture": "architecture_design"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "architecture_design",
                            "enabled": true,
                            "work_item_bindings": ["architecture"],
                            "ordered_steps": [{"role_id": "architect", "runtime_role": "solution_architect", "task_class": "architecture"}]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "architect");
        assert_eq!(sequence[0].runtime_role, "solution_architect");
        assert_eq!(sequence[0].task_class, "architecture");
    }

    #[test]
    fn development_flow_binding_prefers_explicit_runtime_defect_kind_over_inferred_architecture() {
        let task = task_with_labels_and_type(
            "runtime-defect-architect",
            "Repair architect dispatch admissibility",
            &["architecture"],
            "runtime_defect",
        );
        let sequence = dev_team_sequence_for_task(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "runtime_defect": "runtime_defect_remediation",
                        "architecture": "architecture_design",
                        "task": "default_delivery"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "specifier", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                        {"role_id": "coder", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "refactorer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "architecture_design",
                            "enabled": true,
                            "work_item_bindings": ["architecture"],
                            "ordered_steps": [{"role_id": "architect", "runtime_role": "solution_architect", "task_class": "architecture"}]
                        },
                        {
                            "flow_id": "runtime_defect_remediation",
                            "enabled": true,
                            "work_item_bindings": ["runtime_defect"],
                            "ordered_steps": [
                                {"role_id": "specifier", "runtime_role": "business_analyst", "task_class": "specification"},
                                {"role_id": "coder", "runtime_role": "worker", "task_class": "implementation"},
                                {"role_id": "refactorer", "runtime_role": "worker", "task_class": "implementation"},
                                {"role_id": "architect", "runtime_role": "solution_architect", "task_class": "architecture"}
                            ]
                        }
                    ]
                }
            }),
            &task,
        );

        assert_eq!(sequence.len(), 4);
        assert_eq!(sequence[0].role_label, "specifier");
        assert_eq!(sequence[1].role_label, "coder");
        assert_eq!(sequence[2].role_label, "refactorer");
        assert_eq!(sequence[3].role_label, "architect");
    }

    #[test]
    fn development_flow_binding_selects_sequence_from_scalar_comma_bindings() {
        let sequence = dev_team_sequence_for_work_item(
            &serde_json::json!({
                "dev_team_readiness": {
                    "default_flow_id": "minimal",
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "coach", "runtime_role": "coach", "task_classes": ["coach"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "minimal",
                            "enabled": true,
                            "default": true,
                            "work_item_bindings": "task",
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "reviewed",
                            "enabled": true,
                            "work_item_bindings": "epic,task",
                            "ordered_steps": [{"role_id": "coach"}]
                        }
                    ]
                }
            }),
            "epic",
        );

        assert_eq!(sequence.len(), 1);
        assert_eq!(sequence[0].role_label, "coach");
        assert_eq!(sequence[0].task_class, "coach");
    }

    #[test]
    fn development_flow_binding_blocks_mixed_ready_flow_classes_without_current_task() {
        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({
                "agent_system": {"max_parallel_agents": 2},
                "dev_team_readiness": {
                    "default_flow_id": "default_delivery",
                    "work_item_flow_bindings": {
                        "task": "default_delivery",
                        "defect": "defect_repair"
                    },
                    "roles": [
                        {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                        {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
                    ],
                    "sequence": ["developer"],
                    "flows": [
                        {
                            "flow_id": "default_delivery",
                            "enabled": true,
                            "default": true,
                            "ordered_steps": [{"role_id": "developer"}]
                        },
                        {
                            "flow_id": "defect_repair",
                            "enabled": true,
                            "work_item_bindings": ["defect"],
                            "ordered_steps": [{"role_id": "tester"}]
                        }
                    ]
                }
            }),
            &TaskSchedulingProjection {
                current_task_id: None,
                ready: vec![
                    candidate_with_type("task-a", "Task A", true, true, "task"),
                    candidate_with_type("defect-a", "Defect A", true, true, "defect"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"ambiguous_work_item_flow_selection".to_string()));
    }

    #[test]
    fn development_flow_binding_uses_current_task_before_mixed_ready_flow_classes() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery",
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [{"role_id": "developer"}]
                },
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [{"role_id": "tester"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![
                    candidate_with_type("task-a", "Task A", true, true, "task"),
                    candidate_with_type("defect-a", "Defect A", true, true, "defect"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[0].role_label, "tester");
        assert!(!preview
            .blocker_codes
            .contains(&"ambiguous_work_item_flow_selection".to_string()));
    }

    #[test]
    fn development_flow_binding_orders_current_task_first_with_same_ready_flow_class() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [{"role_id": "developer"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("task-active".to_string()),
                ready: vec![
                    candidate_with_type("task-other", "Other task", true, true, "task"),
                    candidate_with_type("task-active", "Active task", true, true, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "task-active");
        assert_eq!(preview.selected_lanes[0].role_label, "developer");
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_honors_current_task_for_same_flow_ready_candidates() {
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &TaskSchedulingProjection {
                current_task_id: Some("zzz-bound".to_string()),
                ready: vec![
                    candidate_with_type("aaa-other", "Other specification", true, true, "task"),
                    candidate_with_type("zzz-bound", "Bound specification", true, true, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "zzz-bound");
        assert!(preview.selected_lanes[0]
            .dispatch_command
            .contains("vida agent-init --role business_analyst zzz-bound"));
        assert!(!preview.selected_lanes[0]
            .dispatch_command
            .contains("--json"));
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_fails_closed_when_current_task_is_not_ready() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "runtime_defect": "runtime_defect_remediation",
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "specifier", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "coder", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "refactorer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "architect", "runtime_role": "solution_architect", "task_classes": ["architecture"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["task"],
                    "ordered_steps": [{"role_id": "developer"}]
                },
                {
                    "flow_id": "runtime_defect_remediation",
                    "enabled": true,
                    "work_item_bindings": ["runtime_defect"],
                    "ordered_steps": [
                        {"role_id": "specifier", "runtime_role": "business_analyst", "task_class": "specification"},
                        {"role_id": "coder", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "refactorer", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "architect", "runtime_role": "solution_architect", "task_class": "architecture"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("runtime-defect-active".to_string()),
                ready: vec![candidate_with_type(
                    "unrelated-ready",
                    "Unrelated ready",
                    true,
                    true,
                    "task",
                )],
                blocked: vec![candidate_with_type(
                    "runtime-defect-active",
                    "Active runtime defect",
                    false,
                    false,
                    "runtime_defect",
                )],
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert!(preview.blocker_codes.iter().any(|code| {
            code == "current_task_not_ready_for_dev_team_dispatch:task=runtime-defect-active"
        }));
        assert!(preview.blocked_candidates.iter().any(|candidate| {
            candidate.task_id == "runtime-defect-active"
                && candidate
                    .reasons
                    .contains(&"current_task_not_ready_for_dev_team_dispatch".to_string())
        }));
        assert_eq!(
            preview.flow_projection["flow_id"],
            "runtime_defect_remediation"
        );
        assert_eq!(preview.flow_projection["status"], "blocked");
        let roles = preview.flow_projection["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|step| step["role_label"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(roles, vec!["specifier", "coder", "refactorer", "architect"]);
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_suppresses_flow_projection_when_current_task_is_absent()
    {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["task"],
                    "ordered_steps": [{"role_id": "developer"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("closed-runtime-defect".to_string()),
                ready: vec![candidate_with_type(
                    "unrelated-ready",
                    "Unrelated ready",
                    true,
                    true,
                    "task",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            4,
            4,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.blocker_codes.iter().any(|code| {
            code == "current_task_not_ready_for_dev_team_dispatch:task=closed-runtime-defect"
        }));
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert!(preview.flow_projection["flow_id"].is_null());
        assert!(preview.flow_projection["current_step"].is_null());
        assert!(preview.flow_projection["steps"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_suppresses_flow_projection_when_current_task_is_absent_and_no_ready_candidates(
    ) {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["task"],
                    "ordered_steps": [{"role_id": "developer"}]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("closed-runtime-defect".to_string()),
                ready: Vec::new(),
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            4,
            4,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.blocker_codes.iter().any(|code| {
            code == "current_task_not_ready_for_dev_team_dispatch:task=closed-runtime-defect"
        }));
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert!(preview.flow_projection["flow_id"].is_null());
        assert!(preview.flow_projection["current_step"].is_null());
        assert!(preview.flow_projection["steps"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn development_flow_binding_reuses_current_task_for_ordered_role_steps() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "defect_repair",
            "work_item_flow_bindings": {
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "flows": [
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![candidate_with_type(
                    "defect-a", "Defect A", true, false, "defect",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 2);
        assert_eq!(preview.selected_lanes[0].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[0].role_label, "analyst");
        assert_eq!(preview.selected_lanes[0].task_class, "specification");
        assert_eq!(preview.selected_lanes[1].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[1].role_label, "tester");
        assert_eq!(preview.selected_lanes[1].task_class, "verification");
        assert!(preview.blocker_codes.is_empty());
    }

    #[test]
    fn dev_team_validation_step_uses_coach_assignment_alias_truth() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "defect_repair",
            "work_item_flow_bindings": {
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "coach_validator", "runtime_role": "coach", "task_classes": ["coach"]}
            ],
            "flows": [
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [
                        {"role_id": "coach_validator", "runtime_role": "coach", "task_class": "coach"}
                    ]
                }
            ]
        });

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![candidate_with_type(
                    "defect-a", "Defect A", true, false, "defect",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].runtime_role, "coach");
        assert_eq!(preview.selected_lanes[0].task_class, "coach");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "coach-seat"
        );
        assert!(preview.blocker_codes.is_empty());
    }

    #[test]
    fn development_flow_binding_scopes_all_ordered_steps_to_current_task() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [
                        {"role_id": "developer", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("task-active".to_string()),
                ready: vec![
                    candidate_with_type("task-active", "Active task", true, false, "task"),
                    candidate_with_type("task-other", "Other task", true, false, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 2);
        assert!(preview
            .selected_lanes
            .iter()
            .all(|lane| lane.task_id == "task-active"));
        assert!(!preview
            .selected_lanes
            .iter()
            .any(|lane| lane.task_id == "task-other"));
    }

    #[test]
    fn development_flow_binding_skips_unsafe_parallel_ready_candidates_without_current_task() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "default_delivery",
            "work_item_flow_bindings": {
                "task": "default_delivery"
            },
            "roles": [
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "sequence": ["developer"],
            "flows": [
                {
                    "flow_id": "default_delivery",
                    "enabled": true,
                    "default": true,
                    "ordered_steps": [
                        {"role_id": "developer", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });
        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: None,
                ready: vec![
                    candidate_with_type("task-safe", "Safe task", true, true, "task"),
                    candidate_with_type("task-unsafe", "Unsafe task", true, false, "task"),
                ],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            2,
            2,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_selected, 1);
        assert_eq!(preview.selected_lanes[0].task_id, "task-safe");
        assert!(!preview
            .selected_lanes
            .iter()
            .any(|lane| lane.task_id == "task-unsafe"));
        assert!(preview.blocked_candidates.iter().any(|candidate| {
            candidate.task_id == "task-unsafe"
                && candidate
                    .reasons
                    .contains(&"parallel_safety_not_established".to_string())
        }));
    }

    #[test]
    fn flow_projection_projects_user_approval_step_gate_and_rework_policy() {
        let preview = build_agent_dispatch_next_preview(
            &serde_json::json!({
                "agent_system": {"max_parallel_agents": 1},
                "carrier_runtime": {
                    "roles": [{
                        "role_id": "middle",
                        "tier": "middle",
                        "default_runtime_role": "business_analyst",
                        "runtime_roles": ["business_analyst"],
                        "task_classes": ["specification"],
                        "rate": 4,
                        "model": "gpt-5.5",
                        "model_provider": "openai",
                        "model_reasoning_effort": "medium",
                        "normalized_cost_units": 4,
                        "readiness": {"status": "ready"},
                        "lifecycle": {"state": "ready"}
                    }]
                },
                "dev_team_readiness": {
                    "default_flow_id": "approval_flow",
                    "roles": [{
                        "role_id": "analyst",
                        "runtime_role": "business_analyst",
                        "task_classes": ["specification"]
                    }],
                    "flows": [{
                        "flow_id": "approval_flow",
                        "enabled": true,
                        "default": true,
                        "ordered_steps": [{
                            "role_id": "analyst",
                            "runtime_role": "business_analyst",
                            "task_class": "specification",
                            "requires_user_approval": true,
                            "approval_policy": {
                                "mode": "user_review_required",
                                "prompt_template": "review_document_before_next_role"
                            },
                            "lifecycle_hook_templates": ["approval_wait", "approval_complete"],
                            "resume_transitions": {"approved": "developer"},
                            "rework_transitions": {"rework": "analyst"}
                        }]
                    }]
                }
            }),
            &TaskSchedulingProjection {
                current_task_id: Some("task-approval".to_string()),
                ready: vec![candidate_with_type(
                    "task-approval",
                    "Approval task",
                    true,
                    true,
                    "task",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.selected_lanes.len(), 1);
        let lane = &preview.selected_lanes[0];
        assert!(lane.requires_user_approval);
        assert_eq!(
            lane.approval_gate["status"],
            "approval_required_after_step_completion"
        );
        assert_eq!(
            lane.approval_gate["policy"]["prompt_template"],
            "review_document_before_next_role"
        );
        assert_eq!(
            lane.approval_gate["rework_transitions"]["rework"],
            "analyst"
        );
        assert!(preview
            .next_actions
            .iter()
            .any(|action| action.contains("will pause after receipt-backed completion")));
        assert_eq!(preview.flow_projection["flow_id"], "approval_flow");
        assert_eq!(
            preview.flow_projection["current_step"]["role_label"],
            "analyst"
        );
        assert_eq!(
            preview.flow_projection["current_step"]["receipt_status"]["status"],
            "preview_only"
        );
        assert_eq!(
            preview.flow_projection["approval_waits"][0]["policy"]["prompt_template"],
            "review_document_before_next_role"
        );
        assert_eq!(
            preview.flow_projection["lifecycle_hook_event_stream"][0]["template_id"],
            "approval_wait"
        );
        assert_eq!(
            preview.flow_projection["adapter_projection_source"],
            "dev_team.flows.adapter_projection"
        );
        assert_eq!(
            preview.flow_projection["adapter_projection_is_data_only"],
            true
        );
    }

    #[test]
    fn agent_dispatch_next_preview_renders_configured_dev_team_sequence() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate_with_labels(
                    "task-analyst",
                    "Specification task",
                    true,
                    true,
                    Vec::new(),
                    &["documentation"],
                ),
                candidate_with_labels(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                    &[],
                ),
                candidate_with_labels(
                    "task-coach",
                    "Coach review task",
                    true,
                    true,
                    Vec::new(),
                    &["coach"],
                ),
                candidate_with_labels(
                    "task-tester",
                    "Tester verification",
                    true,
                    true,
                    Vec::new(),
                    &["tester"],
                ),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            4,
            4,
            Some(std::path::Path::new("/tmp/vida-state")),
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert_eq!(preview.selected_lanes[0].role_label, "analyst-seat");
        assert_eq!(preview.selected_lanes[1].role_label, "developer-seat");
        assert_eq!(preview.selected_lanes[2].role_label, "coach-seat");
        assert_eq!(preview.selected_lanes[3].role_label, "verifier-seat");
        assert_eq!(preview.selected_lanes[0].task_id, "task-analyst");
        assert_eq!(preview.selected_lanes[1].task_id, "task-developer");
        assert_eq!(preview.selected_lanes[2].task_id, "task-coach");
        assert_eq!(preview.selected_lanes[3].task_id, "task-tester");
        assert_eq!(preview.selected_lanes[0].runtime_role, "business_analyst");
        assert_eq!(preview.selected_lanes[1].runtime_role, "worker");
        assert_eq!(preview.selected_lanes[2].runtime_role, "coach");
        assert_eq!(preview.selected_lanes[3].runtime_role, "verifier");
        assert_eq!(
            preview.selected_lanes[0].selection_truth.task_class,
            "specification"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.task_class,
            "implementation"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.task_class,
            "coach"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.task_class,
            "verification"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_carrier,
            "analyst-seat"
        );
        assert_eq!(
            preview.selected_lanes[0]
                .selection_truth
                .selected_model_profile,
            "analyst-profile"
        );
        assert_eq!(
            preview.selected_lanes[0].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[0].selection_truth.rate, 1);
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_carrier,
            "developer-seat"
        );
        assert_eq!(
            preview.selected_lanes[1]
                .selection_truth
                .selected_model_profile,
            "developer-profile"
        );
        assert_eq!(
            preview.selected_lanes[1].selection_truth.selected_model_ref,
            "gpt-5.5"
        );
        assert_eq!(preview.selected_lanes[1].selection_truth.rate, 2);
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_carrier,
            "coach-seat"
        );
        assert_eq!(
            preview.selected_lanes[2]
                .selection_truth
                .selected_model_profile,
            "coach-profile"
        );
        assert_eq!(
            preview.selected_lanes[2].selection_truth.selected_model_ref,
            "gpt-5.5-coach"
        );
        assert_eq!(preview.selected_lanes[2].selection_truth.rate, 3);
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_carrier,
            "verifier-seat"
        );
        assert_eq!(
            preview.selected_lanes[3]
                .selection_truth
                .selected_model_profile,
            "verifier-profile"
        );
        assert_eq!(
            preview.selected_lanes[3].selection_truth.selected_model_ref,
            "gpt-5.3"
        );
        assert_eq!(preview.selected_lanes[3].selection_truth.rate, 4);
        assert!(preview.selected_lanes[0].dispatch_command.contains(
            "vida agent-init --role business_analyst task-analyst --state-dir /tmp/vida-state"
        ));
        assert!(!preview.selected_lanes[0]
            .dispatch_command
            .contains("--json"));
    }

    #[test]
    fn dev_team_explicit_lanes_can_expose_tester_beyond_parallel_cap() {
        let mut activation_bundle = activation_bundle_with_dev_team_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "defect_repair",
            "work_item_flow_bindings": {
                "defect": "defect_repair"
            },
            "roles": [
                {"role_id": "analyst", "runtime_role": "business_analyst", "task_classes": ["specification"]},
                {"role_id": "autotester", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "developer", "runtime_role": "worker", "task_classes": ["implementation"]},
                {"role_id": "coach_validator", "runtime_role": "coach", "task_classes": ["coach"]},
                {"role_id": "tester", "runtime_role": "verifier", "task_classes": ["verification"]}
            ],
            "flows": [
                {
                    "flow_id": "defect_repair",
                    "enabled": true,
                    "default": true,
                    "work_item_bindings": ["defect"],
                    "ordered_steps": [
                        {"role_id": "analyst", "runtime_role": "business_analyst", "task_class": "specification"},
                        {"role_id": "autotester", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "developer", "runtime_role": "worker", "task_class": "implementation"},
                        {"role_id": "coach_validator", "runtime_role": "coach", "task_class": "coach"},
                        {"role_id": "tester", "runtime_role": "verifier", "task_class": "verification"}
                    ]
                }
            ]
        });

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle,
            &TaskSchedulingProjection {
                current_task_id: Some("defect-a".to_string()),
                ready: vec![candidate_with_type(
                    "defect-a", "Defect A", true, false, "defect",
                )],
                blocked: Vec::new(),
                parallel_candidates_after_current: Vec::new(),
            },
            5,
            4,
            None,
            true,
        );

        assert_eq!(preview.status, "pass", "{preview:#?}");
        assert_eq!(preview.lanes_requested, 5);
        assert_eq!(preview.configured_max_parallel_agents, 4);
        assert_eq!(preview.effective_max_parallel_agents, 4);
        assert_eq!(preview.lanes_selected, 5);
        let role_labels = preview
            .selected_lanes
            .iter()
            .map(|lane| lane.role_label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            role_labels,
            vec![
                "analyst",
                "autotester",
                "developer",
                "coach_validator",
                "tester"
            ]
        );
        assert_eq!(preview.selected_lanes[4].task_id, "defect-a");
        assert_eq!(preview.selected_lanes[4].task_class, "verification");
        assert_eq!(preview.fanout_guard["effective_max_parallel_agents"], 4);
        assert_eq!(preview.fanout_guard["lanes_selected"], 5);
    }

    #[test]
    fn agent_dispatch_next_materialization_uses_config_default_or_explicit_cli_flag() {
        let activation_bundle = serde_json::json!({
            "dev_team": {
                "orchestrator_command_contract": {
                    "default_args": ["--dev-team", "--materialize-packets"]
                }
            },
            "dev_team_readiness": {
                "orchestrator_command_contract": {
                    "default_args": ["--dev-team", "--materialize-packets"]
                }
            }
        });
        let preview_command = AgentDispatchNextArgs {
            lanes: 4,
            scope: None,
            current_task_id: None,
            state_dir: None,
            json: true,
            full: false,
            dev_team: true,
            materialize_packets: false,
        };
        assert!(
            agent_dispatch_next_effective_materialize_packets(&preview_command, &activation_bundle),
            "dev-team config default_args should materialize configured dispatch packets"
        );

        let mut materialize_command = preview_command.clone();
        materialize_command.materialize_packets = true;
        assert!(agent_dispatch_next_effective_materialize_packets(
            &materialize_command,
            &activation_bundle
        ));
    }

    #[test]
    fn agent_dispatch_next_projection_cache_names_are_separate_for_compact_and_full_json() {
        let mut command = AgentDispatchNextArgs {
            lanes: 4,
            scope: None,
            current_task_id: Some("task-a".to_string()),
            state_dir: None,
            json: true,
            full: false,
            dev_team: false,
            materialize_packets: false,
        };
        let compact_name = agent_dispatch_next_projection_name(&command, false);
        command.full = true;
        let full_name = agent_dispatch_next_projection_name(&command, false);

        assert!(compact_name.contains("-compact-"));
        assert!(full_name.contains("-full-"));
        assert_ne!(compact_name, full_name);
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_uses_only_configured_registry_roles() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-analyst".to_string()),
            ready: vec![
                candidate("task-analyst", "Specification task", true, true, Vec::new()),
                candidate(
                    "task-developer",
                    "Implementation task",
                    true,
                    true,
                    Vec::new(),
                ),
                candidate("task-coach", "Coach review task", true, true, Vec::new()),
                candidate("task-tester", "Tester verification", true, true, Vec::new()),
                candidate("task-unused", "Unused final task", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_dev_team_selection_truth(),
            &projection,
            5,
            5,
            None,
            true,
        );

        assert_eq!(preview.status, "pass");
        assert_eq!(preview.mode, "preview-dev-team");
        assert_eq!(preview.lanes_selected, 4);
        assert!(!preview
            .next_actions
            .iter()
            .any(|action| action.contains("closure-oriented")));
    }

    #[test]
    fn agent_dispatch_next_preview_dev_team_fails_closed_when_selection_truth_is_missing() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, false, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            true,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
        assert!(preview
            .blocker_codes
            .contains(&"selected_lane_runtime_assignment_truth_required".to_string()));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_role_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_role_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_carrier_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_model_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_model_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_model_profile_id_missing")));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_price_policy() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_price_data_blocked(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview.blocker_codes.iter().any(|code| {
            code.starts_with("selected_lane_runtime_assignment_truth_missing:task=task-a:")
        }));
    }

    #[test]
    fn agent_dispatch_next_preview_actionable_blocker_for_missing_rate_data() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_missing_price_data(),
            &projection,
            1,
            1,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assertion_message_contains_actionable_blocker(&preview.blocker_codes, "task-a");
        assert!(preview
            .blocker_codes
            .iter()
            .any(|code| code.ends_with(":selected_rate_missing")));
        assert!(preview.blocked_candidates.is_empty());
    }

    #[test]
    fn agent_dispatch_next_preview_exposes_dispatch_flow_discovery_surfaces() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "pass");
        assert!(preview.source_surfaces.iter().any(|surface| {
            surface == "vida taskflow graph-summary"
                || surface == "vida taskflow scheduler dispatch"
        }));
        assert!(
            preview.source_surfaces.iter().any(
                |surface| surface
                    == "build_taskflow_consume_bundle_payload.activation_bundle.agent_system.max_parallel_agents"
            )
        );
        assert!(preview
            .source_surfaces
            .iter()
            .any(|surface| surface == "vida agent-init --role worker <task-id>"));
    }

    #[test]
    fn agent_dispatch_next_preview_uses_default_ready_command_in_human_next_action() {
        let projection = TaskSchedulingProjection {
            current_task_id: None,
            ready: Vec::new(),
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            1,
            4,
            None,
            false,
        );

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"no_ready_task_candidates".to_string()));
        assert!(preview.next_actions.iter().any(|action| {
            action.contains("Inspect `vida task ready`") && !action.contains("ready --json")
        }));
    }

    #[test]
    fn dev_team_dispatch_preview_uses_default_ready_command_in_human_next_action() {
        let mut activation_bundle = activation_bundle_with_worker_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "implementation_flow",
            "roles": [{
                "role_id": "worker",
                "runtime_role": "worker",
                "task_classes": ["implementation"]
            }],
            "flows": [{
                "flow_id": "implementation_flow",
                "enabled": true,
                "default": true,
                "ordered_steps": [{
                    "role_id": "worker",
                    "runtime_role": "worker",
                    "task_class": "implementation"
                }]
            }]
        });
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: Vec::new(),
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };

        let preview =
            build_agent_dispatch_next_preview(&activation_bundle, &projection, 1, 4, None, true);

        assert_eq!(preview.status, "blocked");
        assert!(preview
            .blocker_codes
            .contains(&"no_ready_task_candidates".to_string()));
        assert!(preview.next_actions.iter().any(|action| {
            action.contains("Inspect `vida task ready`") && !action.contains("ready --json")
        }));
    }

    #[test]
    fn agent_dispatch_next_preview_terminal_gate_blocks_execution_but_preserves_diagnostic_proposals(
    ) {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![
                candidate("task-a", "Task A", true, true, Vec::new()),
                candidate("task-b", "Task B", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            2,
            4,
            None,
            false,
        );
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.lanes_selected, 2);
        assert!(preview.parallelization_planner["packet_proposals"]
            .as_array()
            .is_some_and(|proposals| proposals.len() == 2));

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "terminal_continue_snapshot_without_next_bounded_unit"
                    .to_string(),
                blocker_codes: vec![
                    "terminal_continue_snapshot_without_next_bounded_unit".to_string(),
                    "continuation_binding_ambiguous".to_string(),
                ],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
                blocked_task_ids: vec!["task-a".to_string()],
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert!(preview
            .blocker_codes
            .contains(&"terminal_continue_snapshot_without_next_bounded_unit".to_string()));
        assert!(preview
            .blocker_codes
            .contains(&"continuation_binding_ambiguous".to_string()));
        assert!(preview
            .next_actions
            .contains(&"bind an explicit next bounded unit".to_string()));
        assert_eq!(
            preview.parallelization_planner["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.parallelization_planner["continuation_gate_scope"],
            "task_scoped"
        );
        assert_eq!(
            preview.parallelization_planner["independent_parallel_available"],
            true
        );
        let proposals = preview.parallelization_planner["packet_proposals"]
            .as_array()
            .expect("diagnostic proposals should remain visible");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0]["task_id"], "task-b");
        assert_eq!(proposals[0]["materializes_packet"], false);
    }

    #[test]
    fn continuation_gate_preserves_disjoint_parallel_packet_proposals() {
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-current".to_string()),
            ready: vec![
                candidate("task-current", "Current task", true, true, Vec::new()),
                candidate("task-parallel-a", "Parallel A", true, true, Vec::new()),
                candidate("task-parallel-b", "Parallel B", true, true, Vec::new()),
            ],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview = build_agent_dispatch_next_preview(
            &activation_bundle_with_worker_selection_truth(),
            &projection,
            3,
            4,
            None,
            false,
        );
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.lanes_selected, 3);

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "continuation_binding_ambiguous".to_string(),
                blocker_codes: vec!["continuation_binding_ambiguous".to_string()],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
                blocked_task_ids: vec!["task-current".to_string()],
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert_eq!(
            preview.flow_projection["current_step"]["dispatch_command"],
            serde_json::Value::Null
        );
        assert_eq!(
            preview.parallelization_planner["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.parallelization_planner["materializes_packets"],
            false
        );
        assert_eq!(preview.parallelization_planner["diagnostic_only"], true);
        let proposals = preview.parallelization_planner["packet_proposals"]
            .as_array()
            .expect("packet proposals should remain diagnostic");
        let proposal_task_ids = proposals
            .iter()
            .map(|proposal| proposal["task_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(
            proposal_task_ids,
            vec!["task-parallel-a", "task-parallel-b"]
        );
    }

    #[test]
    fn continuation_gate_blocks_flow_projection_dispatch_state() {
        let mut activation_bundle = activation_bundle_with_worker_selection_truth();
        activation_bundle["dev_team_readiness"] = serde_json::json!({
            "default_flow_id": "implementation_flow",
            "roles": [{
                "role_id": "worker",
                "runtime_role": "worker",
                "task_classes": ["implementation"]
            }],
            "flows": [{
                "flow_id": "implementation_flow",
                "enabled": true,
                "default": true,
                "ordered_steps": [{
                    "role_id": "worker",
                    "runtime_role": "worker",
                    "task_class": "implementation"
                }]
            }]
        });
        let projection = TaskSchedulingProjection {
            current_task_id: Some("task-a".to_string()),
            ready: vec![candidate("task-a", "Task A", true, true, Vec::new())],
            blocked: Vec::new(),
            parallel_candidates_after_current: Vec::new(),
        };
        let mut preview =
            build_agent_dispatch_next_preview(&activation_bundle, &projection, 1, 4, None, true);
        assert_eq!(preview.status, "pass");
        assert_eq!(preview.flow_projection["status"], "ready");
        assert_eq!(
            preview.flow_projection["current_step"]["proof_state"]["status"],
            "pending_dispatch"
        );
        assert!(preview.flow_projection["current_step"]["dispatch_command"].is_string());

        apply_continuation_dispatch_gate_to_preview(
            &mut preview,
            &crate::taskflow_proxy::TaskflowContinuationDispatchGate {
                admissible: false,
                admissibility_gate: "continuation_binding_ambiguous".to_string(),
                blocker_codes: vec!["continuation_binding_ambiguous".to_string()],
                next_actions: vec!["bind an explicit next bounded unit".to_string()],
                blocked_task_ids: Vec::new(),
            },
        );

        assert_eq!(preview.status, "blocked");
        assert_eq!(preview.lanes_selected, 0);
        assert!(preview.selected_lanes.is_empty());
        assert_eq!(preview.flow_projection["status"], "blocked");
        assert_eq!(
            preview.flow_projection["proof_state"]["status"],
            "blocked_by_continuation_gate"
        );
        assert_eq!(
            preview.flow_projection["blocked_by_continuation_gate"],
            true
        );
        assert_eq!(
            preview.flow_projection["blocker_codes"],
            serde_json::json!([
                "latest_run_graph_status_blocked",
                "continuation_binding_ambiguous"
            ])
        );
        assert_eq!(
            preview.flow_projection["next_actions"],
            serde_json::json!(["bind an explicit next bounded unit"])
        );
        assert!(preview.flow_projection["current_step"]["dispatch_command"].is_null());
        assert!(preview.flow_projection["current_step"]["dispatch_command_kind"].is_null());
        assert_eq!(
            preview.flow_projection["current_step"]["proof_state"]["status"],
            "blocked_by_continuation_gate"
        );
        assert_eq!(
            preview.flow_projection["current_step"]["blocked_by_continuation_gate"],
            true
        );
    }

    #[test]
    fn agent_dispatch_next_command_uses_configured_runtime_selection_truth() {
        std::thread::Builder::new()
            .name("agent_dispatch_next_command_selection_truth".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(agent_dispatch_next_command_uses_configured_runtime_selection_truth_inner)
            .expect("test thread should spawn")
            .join()
            .expect("test thread should complete");
    }

    fn agent_dispatch_next_command_uses_configured_runtime_selection_truth_inner() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        runtime.block_on(async {
            let store = crate::StateStore::open(harness.path().to_path_buf())
                .await
                .expect("state store should open");
            let project_root = crate::repo_runtime_root();
            let config_path = crate::config_file_path_for_root(&project_root);
            let source_config_digest =
                crate::launcher_activation_snapshot::config_file_digest(&config_path)
                    .expect("config digest should compute");
            let mut compiled_bundle = activation_bundle_with_worker_selection_truth();
            compiled_bundle["role_selection"] = serde_json::json!({
                "fallback_role": "worker",
                "mode": "native"
            });
            store
                .write_launcher_activation_snapshot(&LauncherActivationSnapshot {
                    source: "state_store".to_string(),
                    source_config_path: config_path.display().to_string(),
                    source_config_digest,
                    captured_at: "2026-03-08T00:00:00Z".to_string(),
                    compiled_bundle,
                    pack_router_keywords: serde_json::json!({}),
                })
                .await
                .expect("launcher activation snapshot should seed");
            store
                .create_task_with_fixture_parent(CreateTaskRequest {
                    task_id: "task-ready",
                    title: "Ready task",
                    display_id: None,
                    description: "",
                    issue_type: "task",
                    status: "open",
                    priority: 2,
                    parent_id: None,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: state_store::TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: ".",
                })
                .await
                .expect("task should create");
            store
                .refresh_task_snapshot()
                .await
                .expect("snapshot should refresh");
        });

        let _cwd = guard_current_dir(&crate::repo_runtime_root());
        let _vida_root = EnvVarGuard::unset("VIDA_ROOT");
        let code = runtime.block_on(crate::run(cli(&[
            "agent",
            "dispatch-next",
            "--lanes",
            "1",
            "--state-dir",
            harness.path().to_str().expect("state dir should be utf8"),
            "--json",
        ])));

        assert_eq!(code, ExitCode::SUCCESS);
    }
}
