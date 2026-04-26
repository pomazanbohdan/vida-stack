use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{json, Value};

const REQUIRED_EXECUTION_PREPARATION_ARTIFACTS: &[&str] = &[
    "architecture_preparation_report",
    "developer_handoff_packet",
    "change_boundary",
    "dependency_impact_summary",
    "spec_alignment_summary",
];
const EXECUTION_ARTIFACT_REGISTRY_CONTRACT_ID: &str =
    "execution_preparation_artifact_registry_contract";
const EXECUTION_ARTIFACT_REGISTRY_SCHEMA_VERSION: &str = "foundation-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactRegistryContractViolation {
    pub(crate) code: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone)]
struct ArtifactSource {
    kind: &'static str,
    path: String,
    pointer: &'static str,
}

#[derive(Debug, Clone)]
struct ArtifactSnapshot {
    source: ArtifactSource,
    artifacts_payload: Value,
    operator_contracts: Value,
    shared_fields: Value,
    artifact_refs: Value,
}

pub(crate) async fn run_taskflow_artifacts(args: &[String]) -> ExitCode {
    match parse_artifact_command(args) {
        ArtifactCommand::Help => {
            print_artifacts_help();
            ExitCode::SUCCESS
        }
        ArtifactCommand::List { json } => render_artifacts_list(json),
        ArtifactCommand::Show { artifact_id, json } => render_artifact_show(&artifact_id, json),
        ArtifactCommand::Invalid { reason } => {
            eprintln!("{reason}");
            print_artifacts_help();
            ExitCode::from(2)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArtifactCommand {
    Help,
    List { json: bool },
    Show { artifact_id: String, json: bool },
    Invalid { reason: String },
}

fn parse_artifact_command(args: &[String]) -> ArtifactCommand {
    let Some(head) = args.first().map(String::as_str) else {
        return ArtifactCommand::Invalid {
            reason: "missing taskflow artifact command".to_string(),
        };
    };
    if !matches!(head, "artifact" | "artifacts") {
        return ArtifactCommand::Invalid {
            reason: format!("unsupported taskflow artifact command head `{head}`"),
        };
    }

    match args.get(1).map(String::as_str) {
        None | Some("--help" | "-h") => ArtifactCommand::Help,
        Some("list") => {
            let json = args.iter().skip(2).any(|arg| arg == "--json");
            let unexpected = args
                .iter()
                .skip(2)
                .find(|arg| arg.as_str() != "--json")
                .cloned();
            if let Some(arg) = unexpected {
                return ArtifactCommand::Invalid {
                    reason: format!("unsupported `vida taskflow artifacts list` argument `{arg}`"),
                };
            }
            ArtifactCommand::List { json }
        }
        Some("show") => {
            let Some(artifact_id) = args.get(2).cloned() else {
                return ArtifactCommand::Invalid {
                    reason: "missing artifact id for `vida taskflow artifacts show`".to_string(),
                };
            };
            let json = args.iter().skip(3).any(|arg| arg == "--json");
            let unexpected = args
                .iter()
                .skip(3)
                .find(|arg| arg.as_str() != "--json")
                .cloned();
            if let Some(arg) = unexpected {
                return ArtifactCommand::Invalid {
                    reason: format!("unsupported `vida taskflow artifacts show` argument `{arg}`"),
                };
            }
            ArtifactCommand::Show { artifact_id, json }
        }
        Some(other) => ArtifactCommand::Invalid {
            reason: format!("unsupported `vida taskflow artifacts` subcommand `{other}`"),
        },
    }
}

fn render_artifacts_list(json_output: bool) -> ExitCode {
    let state_root = crate::taskflow_task_bridge::proxy_state_dir();
    let project_root = crate::taskflow_task_bridge::infer_project_root_from_state_root(&state_root)
        .or_else(|| crate::resolve_runtime_project_root().ok());
    let payload = match load_execution_artifact_snapshot(&state_root) {
        Ok(snapshot) => build_artifact_list_payload(&snapshot, project_root.as_deref()),
        Err(error) => blocked_payload("vida taskflow artifacts list", &state_root, error),
    };

    if json_output {
        crate::print_json_pretty(&payload);
    } else {
        print_artifact_list_plain(&payload);
    }
    ExitCode::SUCCESS
}

fn render_artifact_show(artifact_id: &str, json_output: bool) -> ExitCode {
    let state_root = crate::taskflow_task_bridge::proxy_state_dir();
    let project_root = crate::taskflow_task_bridge::infer_project_root_from_state_root(&state_root)
        .or_else(|| crate::resolve_runtime_project_root().ok());
    let payload = match load_execution_artifact_snapshot(&state_root) {
        Ok(snapshot) => {
            let list = build_artifact_list_payload(&snapshot, project_root.as_deref());
            build_artifact_show_payload(&list, artifact_id)
        }
        Err(error) => blocked_payload("vida taskflow artifacts show", &state_root, error),
    };

    if json_output {
        crate::print_json_pretty(&payload);
    } else {
        print_artifact_show_plain(&payload);
    }
    if payload["status"].as_str() == Some("blocked") {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn load_execution_artifact_snapshot(state_root: &Path) -> Result<ArtifactSnapshot, String> {
    let path =
        crate::runtime_consumption_state::latest_recorded_final_runtime_consumption_snapshot_path(
            state_root,
        )?
        .ok_or_else(|| "no recorded final runtime-consumption snapshot found".to_string())?;
    let body = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read runtime-consumption snapshot: {error}"))?;
    let snapshot = serde_json::from_str::<Value>(&body)
        .map_err(|error| format!("failed to parse runtime-consumption snapshot: {error}"))?;

    let Some((pointer, artifacts_payload)) = find_execution_preparation_artifacts(&snapshot) else {
        return Err(format!(
            "runtime-consumption snapshot `{path}` does not contain execution_preparation_artifacts"
        ));
    };

    Ok(ArtifactSnapshot {
        source: ArtifactSource {
            kind: "runtime_consumption_final_snapshot",
            path,
            pointer,
        },
        artifacts_payload: artifacts_payload.clone(),
        operator_contracts: snapshot
            .get("operator_contracts")
            .cloned()
            .unwrap_or(Value::Null),
        shared_fields: snapshot
            .get("shared_fields")
            .cloned()
            .unwrap_or(Value::Null),
        artifact_refs: snapshot
            .get("artifact_refs")
            .cloned()
            .unwrap_or(Value::Null),
    })
}

fn find_execution_preparation_artifacts(snapshot: &Value) -> Option<(&'static str, &Value)> {
    const POINTERS: &[&str] = &[
        "/execution_preparation_artifacts",
        "/taskflow_handoff_plan/execution_preparation_artifacts",
        "/run_graph_bootstrap/execution_preparation_artifacts",
        "/dispatch_packet/execution_preparation_artifacts",
        "/payload/execution_preparation_artifacts",
        "/payload/taskflow_handoff_plan/execution_preparation_artifacts",
        "/payload/run_graph_bootstrap/execution_preparation_artifacts",
        "/payload/dispatch_packet_preview/execution_preparation_artifacts",
        "/payload/dispatch_packet/execution_preparation_artifacts",
    ];
    POINTERS
        .iter()
        .find_map(|pointer| snapshot.pointer(pointer).map(|value| (*pointer, value)))
}

fn build_artifact_list_payload(snapshot: &ArtifactSnapshot, project_root: Option<&Path>) -> Value {
    let required_artifacts = required_artifacts(&snapshot.artifacts_payload);
    let artifacts: Vec<Value> = required_artifacts
        .iter()
        .map(|artifact_id| {
            normalize_artifact(
                artifact_id,
                snapshot.artifacts_payload.get(artifact_id),
                project_root,
            )
        })
        .collect();
    let missing_artifacts: Vec<Value> = artifacts
        .iter()
        .filter(|artifact| artifact["missing"].as_bool() == Some(true))
        .map(|artifact| artifact["artifact_id"].clone())
        .collect();
    let materialized_artifacts: Vec<Value> = artifacts
        .iter()
        .filter(|artifact| artifact["materialized"].as_bool() == Some(true))
        .map(|artifact| artifact["artifact_id"].clone())
        .collect();
    let artifact_registry_contract = build_execution_artifact_registry_contract(&artifacts);
    let artifact_registry_validation =
        execution_artifact_registry_validation_payload(&artifact_registry_contract);

    let payload = json!({
        "surface": "vida taskflow artifacts list",
        "status": "pass",
        "artifact_scope": "execution_preparation",
        "source": {
            "kind": snapshot.source.kind,
            "path": snapshot.source.path,
            "json_pointer": snapshot.source.pointer,
        },
        "required_artifacts": required_artifacts,
        "artifacts": artifacts,
        "artifact_registry_contract": artifact_registry_contract,
        "artifact_registry_validation": artifact_registry_validation,
        "missing_artifacts": missing_artifacts,
        "materialized_artifacts": materialized_artifacts,
        "execution_preparation_evidence": snapshot.artifacts_payload
            .get("execution_preparation_evidence")
            .cloned()
            .unwrap_or_else(|| json!({
                "ready": false,
                "status": "missing_execution_preparation_evidence_truth",
            })),
        "handoff_ready": snapshot.artifacts_payload
            .get("handoff_ready")
            .cloned()
            .unwrap_or(Value::Null),
        "source_operator_contracts": snapshot.operator_contracts,
        "source_shared_fields": snapshot.shared_fields,
        "source_artifact_refs": snapshot.artifact_refs,
        "artifact_refs": {
            "runtime_consumption_snapshot_path": snapshot.source.path,
            "execution_preparation_artifacts_pointer": snapshot.source.pointer,
        },
    });
    with_artifact_operator_contract(
        payload,
        None,
        "inspect execution-preparation artifacts with `vida taskflow artifacts list --json`",
    )
}

fn build_artifact_show_payload(list_payload: &Value, artifact_id: &str) -> Value {
    let artifact = list_payload["artifacts"]
        .as_array()
        .and_then(|artifacts| {
            artifacts
                .iter()
                .find(|artifact| artifact["artifact_id"].as_str() == Some(artifact_id))
        })
        .cloned();

    match artifact {
        Some(artifact) => with_artifact_operator_contract(
            json!({
            "surface": "vida taskflow artifacts show",
            "status": "pass",
            "artifact_scope": list_payload["artifact_scope"].clone(),
            "source": list_payload["source"].clone(),
            "artifact": artifact,
            "artifact_registry_contract": list_payload["artifact_registry_contract"].clone(),
            "artifact_registry_validation": list_payload["artifact_registry_validation"].clone(),
            "execution_preparation_evidence": list_payload["execution_preparation_evidence"].clone(),
            "artifact_refs": list_payload["artifact_refs"].clone(),
            }),
            None,
            "inspect execution-preparation artifacts with `vida taskflow artifacts list --json`",
        ),
        None => with_artifact_operator_contract(
            json!({
            "surface": "vida taskflow artifacts show",
            "status": "blocked",
            "blocker_codes": [
                crate::release1_contracts::BlockerCode::MissingExecutionPreparationArtifactQueryTarget.as_str()
            ],
            "artifact_scope": list_payload["artifact_scope"].clone(),
            "requested_artifact_id": artifact_id,
            "required_artifacts": list_payload["required_artifacts"].clone(),
            "source": list_payload["source"].clone(),
            "artifact_refs": list_payload["artifact_refs"].clone(),
            "next_actions": [
                "Run `vida taskflow artifacts list --json` to inspect queryable execution-preparation artifacts."
            ],
            }),
            Some(
                crate::release1_contracts::BlockerCode::MissingExecutionPreparationArtifactQueryTarget,
            ),
            "run `vida taskflow artifacts list --json` to inspect queryable execution-preparation artifacts",
        ),
    }
}

fn string_array_from_payload(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn with_artifact_operator_contract(
    mut payload: Value,
    fallback_blocker: Option<crate::release1_contracts::BlockerCode>,
    fallback_next_action: &str,
) -> Value {
    let mut blocker_codes = string_array_from_payload(&payload["blocker_codes"]);
    if blocker_codes.is_empty() {
        if let Some(blocker_code) = payload["blocker_code"].as_str() {
            blocker_codes.push(blocker_code.to_string());
        }
    }
    if blocker_codes.is_empty() && payload["status"].as_str() == Some("blocked") {
        if let Some(blocker) = fallback_blocker {
            blocker_codes.push(blocker.as_str().to_string());
        }
    }

    let mut next_actions = string_array_from_payload(&payload["next_actions"]);
    if !blocker_codes.is_empty() && next_actions.is_empty() {
        next_actions.push(fallback_next_action.to_string());
    }
    let artifact_refs = payload
        .get("artifact_refs")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));

    let finalized = crate::operator_contracts::finalize_release1_operator_truth(
        blocker_codes,
        next_actions,
        artifact_refs,
    )
    .unwrap_or_else(|_| {
        crate::operator_contracts::finalize_release1_operator_truth(
            vec![crate::release1_contracts::BlockerCode::Unsupported
                .as_str()
                .to_string()],
            vec!["inspect execution-preparation artifact operator contract output".to_string()],
            json!({}),
        )
        .expect("unsupported blocker fallback should be a valid release-1 contract")
    });

    let Some(object) = payload.as_object_mut() else {
        return payload;
    };
    object.insert(
        "status".to_string(),
        Value::String(finalized.status.to_string()),
    );
    object.insert(
        "blocker_codes".to_string(),
        serde_json::to_value(finalized.blocker_codes)
            .expect("artifact blocker codes should serialize"),
    );
    object.insert(
        "next_actions".to_string(),
        serde_json::to_value(finalized.next_actions)
            .expect("artifact next actions should serialize"),
    );
    object.insert("artifact_refs".to_string(), finalized.artifact_refs);
    object.insert("shared_fields".to_string(), finalized.shared_fields);
    object.insert(
        "operator_contracts".to_string(),
        finalized.operator_contracts,
    );
    object.remove("blocker_code");
    payload
}

fn required_artifacts(artifacts_payload: &Value) -> Vec<String> {
    let mut required: Vec<String> = artifacts_payload
        .get("required_artifacts")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| {
            REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
                .iter()
                .map(|artifact| (*artifact).to_string())
                .collect()
        });
    if required.is_empty() {
        required = REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
            .iter()
            .map(|artifact| (*artifact).to_string())
            .collect();
    }
    required.sort();
    required.dedup();
    required
}

fn normalize_artifact(
    artifact_id: &str,
    artifact: Option<&Value>,
    project_root: Option<&Path>,
) -> Value {
    let ready = artifact
        .and_then(|artifact| artifact.get("ready"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = artifact
        .and_then(|artifact| artifact.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("missing_execution_preparation_artifact_truth");
    let path = artifact
        .and_then(|artifact| artifact.get("path"))
        .cloned()
        .unwrap_or(Value::Null);
    let materialized = path
        .as_str()
        .map(|artifact_path| artifact_path_materialized(artifact_path, project_root))
        .unwrap_or(false);
    let required_now = status != "not_required";
    let missing = required_now && !ready;
    let owner_id = execution_artifact_owner_id(artifact_id).unwrap_or("unknown_artifact_owner");

    json!({
        "artifact_id": artifact_id,
        "owner_id": owner_id,
        "required": true,
        "required_now": required_now,
        "ready": ready,
        "status": status,
        "path": path,
        "materialized": materialized,
        "missing": missing,
        "queryable": true,
        "source_field_present": artifact.is_some(),
    })
}

fn execution_artifact_owner_id(artifact_id: &str) -> Option<&'static str> {
    match artifact_id {
        "architecture_preparation_report" => Some("architecture_preparation"),
        "developer_handoff_packet" => Some("developer_handoff"),
        "change_boundary" => Some("change_boundary"),
        "dependency_impact_summary" => Some("dependency_impact"),
        "spec_alignment_summary" => Some("spec_alignment"),
        _ => None,
    }
}

fn build_execution_artifact_registry_contract(artifacts: &[Value]) -> Value {
    let entries: Vec<Value> = artifacts
        .iter()
        .map(|artifact| {
            json!({
                "artifact_id": artifact["artifact_id"].clone(),
                "owner_id": artifact["owner_id"].clone(),
                "status": artifact["status"].clone(),
                "path": artifact["path"].clone(),
                "ready": artifact["ready"].clone(),
                "required_now": artifact["required_now"].clone(),
                "materialized": artifact["materialized"].clone(),
                "missing": artifact["missing"].clone(),
            })
        })
        .collect();
    json!({
        "contract_id": EXECUTION_ARTIFACT_REGISTRY_CONTRACT_ID,
        "schema_version": EXECUTION_ARTIFACT_REGISTRY_SCHEMA_VERSION,
        "scope": "execution_preparation",
        "status": "foundation",
        "persistence": "derived_runtime_consumption_snapshot",
        "entries": entries,
    })
}

pub(crate) fn validate_execution_artifact_registry_contract(
    contract: &Value,
) -> Result<(), ArtifactRegistryContractViolation> {
    if contract["contract_id"].as_str() != Some(EXECUTION_ARTIFACT_REGISTRY_CONTRACT_ID) {
        return Err(registry_violation(
            "invalid_contract_id",
            "execution-preparation artifact registry contract id is missing or invalid",
        ));
    }
    if contract["schema_version"].as_str() != Some(EXECUTION_ARTIFACT_REGISTRY_SCHEMA_VERSION) {
        return Err(registry_violation(
            "invalid_schema_version",
            "execution-preparation artifact registry schema version is missing or invalid",
        ));
    }
    if contract["scope"].as_str() != Some("execution_preparation") {
        return Err(registry_violation(
            "invalid_scope",
            "execution-preparation artifact registry scope is missing or invalid",
        ));
    }
    let entries = contract["entries"].as_array().ok_or_else(|| {
        registry_violation(
            "missing_entries",
            "execution-preparation artifact registry entries must be an array",
        )
    })?;
    if entries.is_empty() {
        return Err(registry_violation(
            "empty_entries",
            "execution-preparation artifact registry entries must not be empty",
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        validate_execution_artifact_registry_entry(entry, &mut seen)?;
    }
    let missing_canonical = REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
        .iter()
        .find(|artifact_id| !seen.contains(**artifact_id));
    if let Some(artifact_id) = missing_canonical {
        return Err(registry_violation(
            "missing_canonical_artifact",
            &format!("artifact registry contract is missing canonical artifact `{artifact_id}`"),
        ));
    }
    Ok(())
}

fn validate_execution_artifact_registry_entry(
    entry: &Value,
    seen: &mut std::collections::BTreeSet<String>,
) -> Result<(), ArtifactRegistryContractViolation> {
    let artifact_id = entry["artifact_id"].as_str().ok_or_else(|| {
        registry_violation(
            "missing_artifact_id",
            "artifact registry entries must include artifact_id",
        )
    })?;
    if !seen.insert(artifact_id.to_string()) {
        return Err(registry_violation(
            "duplicate_artifact_id",
            &format!("artifact registry entry `{artifact_id}` is duplicated"),
        ));
    }
    if !REQUIRED_EXECUTION_PREPARATION_ARTIFACTS.contains(&artifact_id) {
        return Err(registry_violation(
            "unknown_artifact_id",
            &format!("artifact registry entry `{artifact_id}` is not canonical"),
        ));
    }

    let expected_owner = execution_artifact_owner_id(artifact_id).ok_or_else(|| {
        registry_violation(
            "unknown_artifact_owner",
            &format!("artifact registry entry `{artifact_id}` has no owner mapping"),
        )
    })?;
    let owner_id = entry["owner_id"].as_str().ok_or_else(|| {
        registry_violation(
            "missing_owner_id",
            &format!("artifact registry entry `{artifact_id}` must include owner_id"),
        )
    })?;
    if owner_id != expected_owner {
        return Err(registry_violation(
            "owner_id_mismatch",
            &format!(
                "artifact registry entry `{artifact_id}` owner `{owner_id}` does not match `{expected_owner}`"
            ),
        ));
    }

    let status = entry["status"].as_str().ok_or_else(|| {
        registry_violation(
            "missing_status",
            &format!("artifact registry entry `{artifact_id}` must include status"),
        )
    })?;
    let ready = entry["ready"].as_bool().ok_or_else(|| {
        registry_violation(
            "missing_ready",
            &format!("artifact registry entry `{artifact_id}` must include ready"),
        )
    })?;
    let required_now = entry["required_now"].as_bool().ok_or_else(|| {
        registry_violation(
            "missing_required_now",
            &format!("artifact registry entry `{artifact_id}` must include required_now"),
        )
    })?;
    let materialized = entry["materialized"].as_bool().ok_or_else(|| {
        registry_violation(
            "missing_materialized",
            &format!("artifact registry entry `{artifact_id}` must include materialized"),
        )
    })?;
    let missing = entry["missing"].as_bool().ok_or_else(|| {
        registry_violation(
            "missing_missing_flag",
            &format!("artifact registry entry `{artifact_id}` must include missing"),
        )
    })?;
    let path = entry.get("path").unwrap_or(&Value::Null);
    if !path.is_null()
        && path
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return Err(registry_violation(
            "invalid_path",
            &format!(
                "artifact registry entry `{artifact_id}` path must be null or a non-empty string"
            ),
        ));
    }
    if ready
        && path
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return Err(registry_violation(
            "ready_artifact_missing_path",
            &format!("artifact registry entry `{artifact_id}` is ready without a path"),
        ));
    }
    if materialized
        && path
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return Err(registry_violation(
            "materialized_artifact_missing_path",
            &format!("artifact registry entry `{artifact_id}` is materialized without a path"),
        ));
    }
    if status == "not_required" && required_now {
        return Err(registry_violation(
            "not_required_artifact_marked_required_now",
            &format!("artifact registry entry `{artifact_id}` has contradictory required_now"),
        ));
    }
    if status != "not_required" && required_now && !ready && !missing {
        return Err(registry_violation(
            "required_pending_artifact_not_marked_missing",
            &format!(
                "artifact registry entry `{artifact_id}` is required and pending but not missing"
            ),
        ));
    }
    Ok(())
}

fn registry_violation(code: &str, detail: &str) -> ArtifactRegistryContractViolation {
    ArtifactRegistryContractViolation {
        code: code.to_string(),
        detail: detail.to_string(),
    }
}

fn execution_artifact_registry_validation_payload(contract: &Value) -> Value {
    match validate_execution_artifact_registry_contract(contract) {
        Ok(()) => json!({
            "status": "pass",
            "blocker_code": Value::Null,
            "detail": "execution-preparation artifact registry contract is valid",
        }),
        Err(error) => json!({
            "status": "blocked",
            "blocker_code": error.code,
            "detail": error.detail,
        }),
    }
}

fn artifact_path_materialized(artifact_path: &str, project_root: Option<&Path>) -> bool {
    if artifact_path.trim().is_empty() {
        return false;
    }
    let path = PathBuf::from(artifact_path);
    if path.is_absolute() {
        return path.exists();
    }
    if let Some(project_root) = project_root {
        return project_root.join(&path).exists();
    }
    path.exists()
}

fn blocked_payload(surface: &str, state_root: &Path, reason: String) -> Value {
    with_artifact_operator_contract(
        json!({
        "surface": surface,
        "status": "blocked",
        "blocker_codes": [
            crate::release1_contracts::BlockerCode::ExecutionPreparationArtifactsUnavailable.as_str()
        ],
        "artifact_scope": "execution_preparation",
        "state_root": state_root.display().to_string(),
        "reason": reason,
        "required_artifacts": REQUIRED_EXECUTION_PREPARATION_ARTIFACTS,
        "missing_artifacts": REQUIRED_EXECUTION_PREPARATION_ARTIFACTS,
        "materialized_artifacts": [],
        "artifact_refs": {
            "state_root": state_root.display().to_string(),
            "surface": surface,
        },
        "next_actions": [
            "Materialize or refresh a final runtime-consumption snapshot that includes execution_preparation_artifacts."
        ],
        }),
        Some(crate::release1_contracts::BlockerCode::ExecutionPreparationArtifactsUnavailable),
        "materialize or refresh a final runtime-consumption snapshot that includes execution_preparation_artifacts",
    )
}

fn print_artifact_list_plain(payload: &Value) {
    crate::print_surface_header(crate::RenderMode::Plain, "vida taskflow artifacts list");
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "status",
        payload["status"].as_str().unwrap_or("unknown"),
    );
    if let Some(path) = payload["source"]["path"].as_str() {
        crate::print_surface_line(crate::RenderMode::Plain, "source", path);
    }
    if let Some(reason) = payload["reason"].as_str() {
        crate::print_surface_line(crate::RenderMode::Plain, "reason", reason);
    }
    if let Some(artifacts) = payload["artifacts"].as_array() {
        for artifact in artifacts {
            let artifact_id = artifact["artifact_id"].as_str().unwrap_or("unknown");
            let status = artifact["status"].as_str().unwrap_or("unknown");
            let ready = artifact["ready"].as_bool().unwrap_or(false);
            let materialized = artifact["materialized"].as_bool().unwrap_or(false);
            crate::print_surface_line(
                crate::RenderMode::Plain,
                artifact_id,
                &format!("ready={ready} materialized={materialized} status={status}"),
            );
        }
    }
}

fn print_artifact_show_plain(payload: &Value) {
    crate::print_surface_header(crate::RenderMode::Plain, "vida taskflow artifacts show");
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "status",
        payload["status"].as_str().unwrap_or("unknown"),
    );
    if let Some(reason) = payload["reason"].as_str() {
        crate::print_surface_line(crate::RenderMode::Plain, "reason", reason);
    }
    if let Some(artifact) = payload.get("artifact") {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "artifact",
            artifact["artifact_id"].as_str().unwrap_or("unknown"),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "ready",
            &artifact["ready"].as_bool().unwrap_or(false).to_string(),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "materialized",
            &artifact["materialized"]
                .as_bool()
                .unwrap_or(false)
                .to_string(),
        );
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "status",
            artifact["status"].as_str().unwrap_or("unknown"),
        );
        if let Some(path) = artifact["path"].as_str() {
            crate::print_surface_line(crate::RenderMode::Plain, "path", path);
        }
    }
}

fn print_artifacts_help() {
    println!("VIDA TaskFlow execution-preparation artifacts");
    println!();
    println!("Usage:");
    println!("  vida taskflow artifacts list [--json]");
    println!("  vida taskflow artifacts show <artifact-id> [--json]");
    println!();
    println!("Purpose:");
    println!(
        "  Query missing/materialized execution-preparation artifact truth from the latest final runtime-consumption snapshot."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn execution_artifact_payload_lists_missing_and_materialized_truth() {
        let temp_root = std::env::temp_dir().join(format!(
            "vida-artifact-surface-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("create temp root");
        let artifact_path = temp_root.join("docs/artifacts/developer-handoff.json");
        fs::create_dir_all(
            artifact_path
                .parent()
                .expect("artifact parent should exist"),
        )
        .expect("create artifact parent");
        fs::write(&artifact_path, "{}\n").expect("write artifact");

        let snapshot = ArtifactSnapshot {
            source: ArtifactSource {
                kind: "test",
                path: "final-test.json".to_string(),
                pointer: "/execution_preparation_artifacts",
            },
            artifacts_payload: json!({
                "required_artifacts": [
                    "architecture_preparation_report",
                    "change_boundary",
                    "dependency_impact_summary",
                    "developer_handoff_packet",
                    "spec_alignment_summary"
                ],
                "architecture_preparation_report": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "change_boundary": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "dependency_impact_summary": {
                    "ready": false,
                    "status": "not_required",
                    "path": null
                },
                "developer_handoff_packet": {
                    "ready": true,
                    "status": "ready",
                    "path": artifact_path.to_string_lossy()
                },
                "spec_alignment_summary": {
                    "ready": false,
                    "status": "pending_spec_alignment_summary",
                    "path": null
                },
                "execution_preparation_evidence": {
                    "ready": false,
                    "status": "pending_execution_preparation_evidence"
                }
            }),
            operator_contracts: Value::Null,
            shared_fields: Value::Null,
            artifact_refs: Value::Null,
        };

        let payload = build_artifact_list_payload(&snapshot, Some(&temp_root));
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(payload["shared_fields"]["status"], payload["status"]);
        assert_eq!(
            payload["shared_fields"]["artifact_refs"],
            payload["artifact_refs"]
        );
        assert_eq!(payload["artifact_registry_validation"]["status"], "pass");
        assert_eq!(
            payload["missing_artifacts"],
            json!(["spec_alignment_summary"])
        );
        assert_eq!(
            payload["materialized_artifacts"],
            json!(["developer_handoff_packet"])
        );

        let show = build_artifact_show_payload(&payload, "developer_handoff_packet");
        assert_eq!(show["status"], "pass");
        assert_eq!(show["operator_contracts"]["status"], "pass");
        assert_eq!(show["shared_fields"]["status"], show["status"]);
        assert_eq!(show["artifact"]["materialized"], true);
        assert_eq!(
            show["artifact_registry_contract"]["contract_id"],
            EXECUTION_ARTIFACT_REGISTRY_CONTRACT_ID
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn execution_artifact_registry_contract_links_owner_id_path_and_status() {
        let artifacts = REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
            .iter()
            .map(|artifact_id| {
                normalize_artifact(
                    artifact_id,
                    Some(&json!({
                        "ready": false,
                        "status": format!("pending_{artifact_id}"),
                        "path": null,
                    })),
                    None,
                )
            })
            .collect::<Vec<_>>();

        let contract = build_execution_artifact_registry_contract(&artifacts);

        validate_execution_artifact_registry_contract(&contract)
            .expect("canonical registry contract should validate");
        assert_eq!(
            contract["contract_id"],
            EXECUTION_ARTIFACT_REGISTRY_CONTRACT_ID
        );
        let entries = contract["entries"]
            .as_array()
            .expect("registry entries should render");
        assert!(entries.iter().any(|entry| {
            entry["artifact_id"] == "developer_handoff_packet"
                && entry["owner_id"] == "developer_handoff"
                && entry["status"] == "pending_developer_handoff_packet"
                && entry["path"].is_null()
                && entry["missing"] == true
        }));
    }

    #[test]
    fn execution_artifact_registry_contract_fails_closed_on_owner_mismatch() {
        let artifacts = REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
            .iter()
            .map(|artifact_id| {
                normalize_artifact(
                    artifact_id,
                    Some(&json!({
                        "ready": false,
                        "status": format!("pending_{artifact_id}"),
                        "path": null,
                    })),
                    None,
                )
            })
            .collect::<Vec<_>>();
        let mut contract = build_execution_artifact_registry_contract(&artifacts);
        contract["entries"][0]["owner_id"] = json!("wrong_owner");

        let error = validate_execution_artifact_registry_contract(&contract)
            .expect_err("owner mismatch should fail closed");

        assert_eq!(error.code, "owner_id_mismatch");
    }

    #[test]
    fn execution_artifact_registry_contract_fails_closed_on_ready_without_path() {
        let artifacts = REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
            .iter()
            .map(|artifact_id| {
                let ready = *artifact_id == "developer_handoff_packet";
                let status = if ready {
                    "ready".to_string()
                } else {
                    format!("pending_{artifact_id}")
                };
                normalize_artifact(
                    artifact_id,
                    Some(&json!({
                        "ready": ready,
                        "status": status,
                        "path": null,
                    })),
                    None,
                )
            })
            .collect::<Vec<_>>();
        let contract = build_execution_artifact_registry_contract(&artifacts);

        let error = validate_execution_artifact_registry_contract(&contract)
            .expect_err("ready artifact without path should fail closed");

        assert_eq!(error.code, "ready_artifact_missing_path");
    }

    #[test]
    fn execution_artifact_registry_contract_fails_closed_when_canonical_id_missing() {
        let artifacts = REQUIRED_EXECUTION_PREPARATION_ARTIFACTS
            .iter()
            .filter(|artifact_id| **artifact_id != "spec_alignment_summary")
            .map(|artifact_id| {
                normalize_artifact(
                    artifact_id,
                    Some(&json!({
                        "ready": false,
                        "status": format!("pending_{artifact_id}"),
                        "path": null,
                    })),
                    None,
                )
            })
            .collect::<Vec<_>>();
        let contract = build_execution_artifact_registry_contract(&artifacts);

        let error = validate_execution_artifact_registry_contract(&contract)
            .expect_err("missing canonical artifact should fail closed");

        assert_eq!(error.code, "missing_canonical_artifact");
    }

    #[test]
    fn execution_artifact_show_blocks_unknown_target_with_required_truth() {
        let snapshot = ArtifactSnapshot {
            source: ArtifactSource {
                kind: "test",
                path: "final-test.json".to_string(),
                pointer: "/execution_preparation_artifacts",
            },
            artifacts_payload: json!({
                "required_artifacts": ["change_boundary"],
                "change_boundary": {
                    "ready": false,
                    "status": "pending_change_boundary",
                    "path": null
                }
            }),
            operator_contracts: Value::Null,
            shared_fields: Value::Null,
            artifact_refs: Value::Null,
        };

        let payload = build_artifact_list_payload(&snapshot, None);
        let show = build_artifact_show_payload(&payload, "unknown_artifact");
        assert_eq!(show["status"], "blocked");
        assert_eq!(
            show["blocker_codes"],
            json!(["missing_execution_preparation_artifact_query_target"])
        );
        assert_eq!(
            show["operator_contracts"]["blocker_codes"],
            show["blocker_codes"]
        );
        assert_eq!(
            show["shared_fields"]["next_actions"],
            show["operator_contracts"]["next_actions"]
        );
        assert_eq!(show["required_artifacts"], json!(["change_boundary"]));
    }

    #[test]
    fn execution_artifact_blocked_payload_uses_release1_operator_contract() {
        let state_root = Path::new("/tmp/vida-artifact-state");
        let payload = blocked_payload(
            "vida taskflow artifacts list",
            state_root,
            "no recorded final runtime-consumption snapshot found".to_string(),
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            json!(["execution_preparation_artifacts_unavailable"])
        );
        assert!(payload.get("blocker_code").is_none());
        assert_eq!(payload["operator_contracts"]["status"], payload["status"]);
        assert_eq!(
            payload["shared_fields"]["blocker_codes"],
            payload["blocker_codes"]
        );
        assert_eq!(
            payload["operator_contracts"]["artifact_refs"],
            payload["artifact_refs"]
        );
    }
}
