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

    json!({
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
        "operator_contracts": snapshot.operator_contracts,
        "shared_fields": snapshot.shared_fields,
        "artifact_refs": snapshot.artifact_refs,
    })
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
        Some(artifact) => json!({
            "surface": "vida taskflow artifacts show",
            "status": "pass",
            "artifact_scope": list_payload["artifact_scope"].clone(),
            "source": list_payload["source"].clone(),
            "artifact": artifact,
            "execution_preparation_evidence": list_payload["execution_preparation_evidence"].clone(),
            "operator_contracts": list_payload["operator_contracts"].clone(),
            "shared_fields": list_payload["shared_fields"].clone(),
            "artifact_refs": list_payload["artifact_refs"].clone(),
        }),
        None => json!({
            "surface": "vida taskflow artifacts show",
            "status": "blocked",
            "blocker_code": "missing_execution_preparation_artifact_query_target",
            "artifact_scope": list_payload["artifact_scope"].clone(),
            "requested_artifact_id": artifact_id,
            "required_artifacts": list_payload["required_artifacts"].clone(),
            "source": list_payload["source"].clone(),
            "next_actions": [
                "Run `vida taskflow artifacts list --json` to inspect queryable execution-preparation artifacts."
            ],
        }),
    }
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

    json!({
        "artifact_id": artifact_id,
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
    json!({
        "surface": surface,
        "status": "blocked",
        "blocker_code": "execution_preparation_artifacts_unavailable",
        "artifact_scope": "execution_preparation",
        "state_root": state_root.display().to_string(),
        "reason": reason,
        "required_artifacts": REQUIRED_EXECUTION_PREPARATION_ARTIFACTS,
        "missing_artifacts": REQUIRED_EXECUTION_PREPARATION_ARTIFACTS,
        "materialized_artifacts": [],
        "next_actions": [
            "Materialize or refresh a final runtime-consumption snapshot that includes execution_preparation_artifacts."
        ],
    })
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
                    "developer_handoff_packet",
                    "spec_alignment_summary"
                ],
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
        assert_eq!(show["artifact"]["materialized"], true);

        let _ = fs::remove_dir_all(temp_root);
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
            show["blocker_code"],
            "missing_execution_preparation_artifact_query_target"
        );
        assert_eq!(show["required_artifacts"], json!(["change_boundary"]));
    }
}
