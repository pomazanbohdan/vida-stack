use std::fs;
use std::path::Path;

use time::format_description::well_known::Rfc3339;

pub(crate) const CANONICAL_LAUNCHER_COMMAND: &str = "vida";
pub(crate) const DOCFLOW_READINESS_CURRENT_PATH: &str =
    "vida/config/docflow-readiness.current.jsonl";
pub(crate) const DOCFLOW_PROOF_CURRENT_PATH: &str = "vida/config/docflow-proof.current.jsonl";

#[derive(Debug, serde::Serialize, Clone, PartialEq, Eq)]
pub(crate) struct LauncherBinaryEvidence {
    pub(crate) path: String,
    pub(crate) fingerprint: String,
    pub(crate) active: bool,
}

#[derive(Debug, serde::Serialize, Clone, PartialEq, Eq)]
pub(crate) struct DoctorLauncherSummary {
    pub(crate) vida: String,
    pub(crate) project_root: String,
    pub(crate) taskflow_surface: String,
    pub(crate) active_executable_path: String,
    pub(crate) active_executable_fingerprint: String,
    pub(crate) installed_binaries: Vec<LauncherBinaryEvidence>,
    pub(crate) divergent_installed_binaries: bool,
    pub(crate) status: String,
    pub(crate) next_actions: Vec<String>,
}

pub(crate) fn doctor_launcher_summary_for_root(
    project_root: &Path,
) -> Result<DoctorLauncherSummary, String> {
    let active_executable_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve active vida executable: {error}"))?;
    let active_executable_fingerprint = launcher_binary_fingerprint(&active_executable_path)?;
    let installed_binaries = installed_launcher_binary_evidence(&active_executable_path)?;
    let divergent_installed_binaries = installed_binaries
        .iter()
        .map(|entry| entry.fingerprint.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1;
    let mut next_actions = Vec::new();
    if divergent_installed_binaries {
        next_actions.push(
            "Installed `vida` binaries diverge by content; refresh the intended system binary and verify `command -v vida` before collecting runtime proofs.".to_string(),
        );
    }
    Ok(DoctorLauncherSummary {
        vida: CANONICAL_LAUNCHER_COMMAND.to_string(),
        project_root: project_root.display().to_string(),
        taskflow_surface: "vida taskflow".to_string(),
        active_executable_path: active_executable_path.display().to_string(),
        active_executable_fingerprint,
        installed_binaries,
        divergent_installed_binaries,
        status: if divergent_installed_binaries {
            "warn".to_string()
        } else {
            "pass".to_string()
        },
        next_actions,
    })
}

fn launcher_binary_fingerprint(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "failed to read launcher binary `{}`: {error}",
            path.display()
        )
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn installed_launcher_binary_evidence(
    active_executable_path: &Path,
) -> Result<Vec<LauncherBinaryEvidence>, String> {
    let mut candidates = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        candidates.push(home.join(".local/bin/vida"));
        candidates.push(home.join(".cargo/bin/vida"));
    }
    candidates.push(active_executable_path.to_path_buf());

    let mut seen = std::collections::BTreeSet::new();
    let mut evidence = Vec::new();
    for candidate in candidates {
        if !candidate.is_file() {
            continue;
        }
        let canonical = candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.clone());
        if !seen.insert(canonical.clone()) {
            continue;
        }
        evidence.push(LauncherBinaryEvidence {
            fingerprint: launcher_binary_fingerprint(&canonical)?,
            active: canonical == active_executable_path,
            path: canonical.display().to_string(),
        });
    }
    Ok(evidence)
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowConsumeBundlePayload {
    pub(crate) artifact_name: String,
    pub(crate) artifact_type: String,
    pub(crate) generated_at: String,
    pub(crate) vida_root: String,
    pub(crate) config_path: String,
    pub(crate) activation_source: String,
    pub(crate) launcher_runtime_paths: DoctorLauncherSummary,
    pub(crate) metadata: serde_json::Value,
    pub(crate) control_core: serde_json::Value,
    pub(crate) activation_bundle: serde_json::Value,
    pub(crate) protocol_binding_registry: serde_json::Value,
    pub(crate) cache_delivery_contract: serde_json::Value,
    pub(crate) orchestrator_init_view: serde_json::Value,
    pub(crate) agent_init_view: serde_json::Value,
    pub(crate) continuation_binding: serde_json::Value,
    pub(crate) boot_compatibility: serde_json::Value,
    pub(crate) migration_preflight: serde_json::Value,
    pub(crate) task_store: serde_json::Value,
    pub(crate) run_graph: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowConsumeBundleCheck {
    pub(crate) ok: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) root_artifact_id: String,
    pub(crate) artifact_count: usize,
    pub(crate) boot_classification: String,
    pub(crate) migration_state: String,
    pub(crate) activation_status: String,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RuntimeConsumptionEvidence {
    pub(crate) surface: String,
    pub(crate) ok: bool,
    pub(crate) row_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artifact_path: Option<String>,
    pub(crate) output: String,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RuntimeConsumptionOverview {
    pub(crate) surface: String,
    pub(crate) ok: bool,
    pub(crate) registry_rows: usize,
    pub(crate) check_rows: usize,
    pub(crate) readiness_rows: usize,
    pub(crate) proof_blocking: bool,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RuntimeConsumptionDocflowActivation {
    pub(crate) activated: bool,
    pub(crate) runtime_family: String,
    pub(crate) owner_runtime: String,
    pub(crate) evidence: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct RuntimeConsumptionDocflowVerdict {
    pub(crate) status: String,
    pub(crate) ready: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) proof_surfaces: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RuntimeConsumptionClosureAdmission {
    pub(crate) status: String,
    pub(crate) admitted: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) proof_surfaces: Vec<String>,
}

pub(crate) fn canonical_closure_admission_artifact_json(
    generated_at: &str,
    closure_authority: &str,
    request_text: &str,
    closure_admission: &RuntimeConsumptionClosureAdmission,
) -> serde_json::Value {
    serde_json::to_value(
        crate::release1_contracts::CanonicalClosureAdmissionArtifact {
            closure_admission_record: crate::release1_contracts::CanonicalClosureAdmissionRecord {
                header: crate::release1_contracts::CanonicalArtifactHeader::new(
                    format!("closure-admission.{generated_at}"),
                    crate::release1_contracts::CanonicalArtifactType::ClosureAdmissionRecord,
                    generated_at.to_string(),
                    generated_at.to_string(),
                    closure_admission.status.clone(),
                    "taskflow_consume_final",
                    None,
                    Some(
                        crate::release1_contracts::WorkflowClass::DelegatedDevelopmentPacket
                            .as_str()
                            .to_string(),
                    ),
                ),
                release_scope: request_text.to_string(),
                supported_workflow_classes: vec![
                    crate::release1_contracts::WorkflowClass::DelegatedDevelopmentPacket
                        .as_str()
                        .to_string(),
                ],
                closure_decision: if closure_admission.admitted {
                    "admit".to_string()
                } else {
                    "block".to_string()
                },
                decision_at: generated_at.to_string(),
                decision_owner: closure_authority.to_string(),
                evidence_bundle_refs: closure_admission.proof_surfaces.clone(),
                open_risk_acceptance_ids: Vec::new(),
                blocked_by: closure_admission.blockers.clone(),
            },
        },
    )
    .expect("closure admission artifact should serialize")
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowDirectConsumptionPayload {
    pub(crate) artifact_name: String,
    pub(crate) artifact_type: String,
    pub(crate) generated_at: String,
    pub(crate) closure_authority: String,
    pub(crate) consume_final_mode: String,
    pub(crate) request_text: String,
    pub(crate) role_selection: crate::RuntimeConsumptionLaneSelection,
    pub(crate) runtime_bundle: TaskflowConsumeBundlePayload,
    pub(crate) bundle_check: TaskflowConsumeBundleCheck,
    pub(crate) docflow_activation: RuntimeConsumptionDocflowActivation,
    pub(crate) docflow_verdict: RuntimeConsumptionDocflowVerdict,
    pub(crate) closure_admission: RuntimeConsumptionClosureAdmission,
    pub(crate) closure_admission_artifact: serde_json::Value,
    pub(crate) taskflow_handoff_plan: serde_json::Value,
    pub(crate) run_graph_bootstrap: serde_json::Value,
    pub(crate) dispatch_receipt: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dispatch_packet_preview: Option<serde_json::Value>,
    pub(crate) direct_consumption_ready: bool,
}

fn count_nonempty_lines(output: &str) -> usize {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .count()
}

pub(crate) fn build_docflow_runtime_evidence() -> (
    RuntimeConsumptionEvidence,
    RuntimeConsumptionEvidence,
    RuntimeConsumptionEvidence,
    RuntimeConsumptionEvidence,
    RuntimeConsumptionOverview,
) {
    let registry_root = std::env::current_dir()
        .ok()
        .filter(|cwd| crate::looks_like_project_root(cwd))
        .or_else(|| crate::resolve_repo_root().ok())
        .expect("docflow registry evidence should resolve the repo root");
    let registry_root = registry_root.display().to_string();
    let registry_root_path = std::path::PathBuf::from(&registry_root);
    let registry_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "registry".to_string(),
            "--root".to_string(),
            registry_root.clone(),
        ],
    )
    .expect("docflow registry evidence should render");
    let check_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "check".to_string(),
            "--profile".to_string(),
            "active-canon".to_string(),
        ],
    )
    .expect("docflow check evidence should render");
    let readiness_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "readiness-check".to_string(),
            "--profile".to_string(),
            "active-canon".to_string(),
        ],
    )
    .expect("docflow readiness evidence should render");
    let proof_output = crate::taskflow_spec_bootstrap::run_docflow_cli_command(
        &registry_root_path,
        &[
            "proofcheck".to_string(),
            "--profile".to_string(),
            "active-canon".to_string(),
        ],
    )
    .expect("docflow proof evidence should render");
    let readiness_artifact_path =
        persist_docflow_current_receipt(&registry_root_path, "readiness-check", &readiness_output)
            .expect("docflow readiness receipt artifact should persist");
    let proof_artifact_path =
        persist_docflow_current_receipt(&registry_root_path, "proofcheck", &proof_output)
            .expect("docflow proof receipt artifact should persist");

    let registry_rows = count_nonempty_lines(&registry_output);
    let check_rows = count_nonempty_lines(&check_output);
    let readiness_rows = count_nonempty_lines(&readiness_output);
    let proof_ok = proof_output.contains("✅ OK: proofcheck");
    let proof_blocking = !proof_ok;

    let registry = RuntimeConsumptionEvidence {
        surface: format!("vida docflow registry --root {}", registry_root),
        ok: registry_rows > 0 && !registry_output.contains("\"artifact_type\":\"inventory_error\""),
        row_count: registry_rows,
        verdict: None,
        artifact_path: None,
        output: registry_output,
    };
    let check = RuntimeConsumptionEvidence {
        surface: "vida docflow check --profile active-canon".to_string(),
        ok: check_output.trim().is_empty(),
        row_count: check_rows,
        verdict: None,
        artifact_path: None,
        output: check_output,
    };
    let readiness = RuntimeConsumptionEvidence {
        surface: "vida docflow readiness-check --profile active-canon".to_string(),
        ok: readiness_output.trim().is_empty(),
        row_count: readiness_rows,
        verdict: Some(if readiness_output.trim().is_empty() {
            "ready".to_string()
        } else {
            "blocked".to_string()
        }),
        artifact_path: Some(readiness_artifact_path),
        output: readiness_output,
    };
    let proof = RuntimeConsumptionEvidence {
        surface: "vida docflow proofcheck --profile active-canon".to_string(),
        ok: proof_ok,
        row_count: count_nonempty_lines(&proof_output),
        verdict: Some(if proof_ok {
            "ready".to_string()
        } else {
            "blocked".to_string()
        }),
        artifact_path: Some(proof_artifact_path),
        output: proof_output,
    };
    let overview = RuntimeConsumptionOverview {
        surface: "vida taskflow direct runtime-consumption overview".to_string(),
        ok: registry.ok && check.ok && readiness.ok && proof.ok,
        registry_rows,
        check_rows,
        readiness_rows,
        proof_blocking,
    };

    (registry, check, readiness, proof, overview)
}

fn persist_docflow_current_receipt(
    project_root: &Path,
    check_kind: &str,
    output: &str,
) -> Result<String, String> {
    let relative_path = match check_kind {
        "readiness-check" => DOCFLOW_READINESS_CURRENT_PATH,
        "proofcheck" => DOCFLOW_PROOF_CURRENT_PATH,
        other => {
            return Err(format!(
                "unsupported docflow current receipt kind `{other}`"
            ));
        }
    };
    let verdict = if output.trim().is_empty() || output.contains("✅ OK:") {
        "ready"
    } else {
        "blocked"
    };
    let timestamp = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt = serde_json::json!({
        "receipt_id": format!("docflow-{check_kind}-{timestamp}"),
        "receipt_type": "docflow_current_receipt",
        "entity_type": "docflow_runtime_surface",
        "entity_id": check_kind,
        "machine": "docflow_runtime_evidence",
        "event": format!("{check_kind}_evaluated"),
        "actor": CANONICAL_LAUNCHER_COMMAND,
        "timestamp": timestamp,
        "config_artifact": relative_path,
        "config_revision": "current",
        "surface": format!("vida docflow {check_kind} --profile active-canon"),
        "verdict": verdict,
        "row_count": count_nonempty_lines(output),
        "proof_refs": [relative_path],
        "output_excerpt": output
            .lines()
            .take(20)
            .collect::<Vec<_>>()
            .join("\n"),
    });
    let path = project_root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string(&receipt).expect("docflow current receipt JSON should render")
        ),
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(relative_path.to_string())
}

pub(crate) fn build_docflow_receipt_evidence(
    readiness: &RuntimeConsumptionEvidence,
    proof: &RuntimeConsumptionEvidence,
) -> serde_json::Value {
    let readiness_surface = readiness.surface.clone();
    let readiness_verdict = readiness.verdict.clone();
    let readiness_artifact_path = readiness.artifact_path.clone();
    let readiness_receipt_path = readiness_artifact_path.clone();
    let proof_surface = proof.surface.clone();
    let proof_verdict = proof.verdict.clone();
    let proof_receipt_path = proof
        .artifact_path
        .clone()
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null);
    let total_receipts = usize::from(
        readiness_receipt_path
            .as_ref()
            .is_some_and(|path| !path.trim().is_empty()),
    ) + usize::from(
        proof_receipt_path
            .as_str()
            .is_some_and(|path| !path.trim().is_empty()),
    );
    let receipt_backed = total_receipts > 0;

    serde_json::json!({
        "receipt_backed": receipt_backed,
        "total_receipts": total_receipts,
        "readiness_surface": readiness_surface,
        "readiness_verdict": readiness_verdict,
        "readiness_artifact_path": readiness_artifact_path,
        "readiness_receipt_path": readiness_receipt_path,
        "proof_surface": proof_surface,
        "proof_verdict": proof_verdict,
        "proof_receipt_path": proof_receipt_path,
    })
}

pub(crate) fn blocking_lane_selection(
    request: &str,
    error: &str,
) -> crate::RuntimeConsumptionLaneSelection {
    crate::RuntimeConsumptionLaneSelection {
        ok: false,
        activation_source: "state_store".to_string(),
        selection_mode: "unresolved".to_string(),
        fallback_role: "orchestrator".to_string(),
        request: request.to_string(),
        selected_role: "orchestrator".to_string(),
        conversational_mode: None,
        single_task_only: false,
        tracked_flow_entry: None,
        allow_freeform_chat: false,
        confidence: "blocked".to_string(),
        matched_terms: Vec::new(),
        compiled_bundle: serde_json::Value::Null,
        execution_plan: serde_json::json!({
            "status": "blocked",
            "reason": error,
        }),
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_docflow_receipt_evidence, canonical_closure_admission_artifact_json,
        doctor_launcher_summary_for_root, RuntimeConsumptionClosureAdmission,
        RuntimeConsumptionEvidence, CANONICAL_LAUNCHER_COMMAND,
    };

    #[test]
    fn docflow_receipt_evidence_derives_readiness_receipt_path_from_artifact_path() {
        let readiness = RuntimeConsumptionEvidence {
            surface: "vida docflow readiness-check --profile active-canon".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
            output: String::new(),
        };
        let proof = RuntimeConsumptionEvidence {
            surface: "vida docflow proofcheck --profile active-canon".to_string(),
            ok: true,
            row_count: 1,
            verdict: Some("ready".to_string()),
            artifact_path: Some("vida/config/docflow-proof.current.jsonl".to_string()),
            output: String::new(),
        };

        let evidence = build_docflow_receipt_evidence(&readiness, &proof);

        assert_eq!(evidence["receipt_backed"], true);
        assert_eq!(evidence["total_receipts"], 2);
        assert_eq!(
            evidence["readiness_surface"],
            "vida docflow readiness-check --profile active-canon"
        );
        assert_eq!(evidence["readiness_verdict"], "ready");
        assert_eq!(
            evidence["readiness_artifact_path"],
            "vida/config/docflow-readiness.current.jsonl"
        );
        assert_eq!(
            evidence["readiness_receipt_path"],
            "vida/config/docflow-readiness.current.jsonl"
        );
        assert_eq!(
            evidence["proof_surface"],
            "vida docflow proofcheck --profile active-canon"
        );
        assert_eq!(evidence["proof_verdict"], "ready");
        assert_eq!(
            evidence["proof_receipt_path"],
            "vida/config/docflow-proof.current.jsonl"
        );
    }

    #[test]
    fn doctor_launcher_summary_captures_active_executable_evidence() {
        let project_root = std::path::Path::new("/tmp/vida-stack");
        let current_exe = std::env::current_exe()
            .expect("test executable path should resolve")
            .display()
            .to_string();

        let summary =
            doctor_launcher_summary_for_root(project_root).expect("launcher summary should build");

        assert_eq!(summary.vida, CANONICAL_LAUNCHER_COMMAND);
        assert_eq!(summary.project_root, "/tmp/vida-stack");
        assert_eq!(summary.taskflow_surface, "vida taskflow");
        assert_eq!(summary.active_executable_path, current_exe);
        assert!(!summary.active_executable_fingerprint.is_empty());
        assert!(summary
            .installed_binaries
            .iter()
            .any(|binary| binary.active && binary.path == summary.active_executable_path));
    }

    #[test]
    fn closure_admission_artifact_json_uses_canonical_release1_shape() {
        let closure_admission = RuntimeConsumptionClosureAdmission {
            status: "pass".to_string(),
            admitted: true,
            blockers: Vec::new(),
            proof_surfaces: vec![
                "vida docflow readiness-check --profile active-canon".to_string(),
                "vida docflow proofcheck --profile active-canon".to_string(),
            ],
        };

        let artifact = canonical_closure_admission_artifact_json(
            "2026-04-20T20:00:00Z",
            "taskflow",
            "schema hardening slice",
            &closure_admission,
        );

        assert_eq!(artifact["artifact_type"], "closure_admission_record");
        assert_eq!(artifact["owner_surface"], "taskflow_consume_final");
        assert_eq!(artifact["workflow_class"], "delegated_development_packet");
        assert_eq!(artifact["release_scope"], "schema hardening slice");
        assert_eq!(artifact["closure_decision"], "admit");
        assert_eq!(artifact["decision_owner"], "taskflow");
        assert_eq!(
            artifact["evidence_bundle_refs"][0],
            "vida docflow readiness-check --profile active-canon"
        );
        assert_eq!(
            artifact["evidence_bundle_refs"][1],
            "vida docflow proofcheck --profile active-canon"
        );
        assert_eq!(artifact["blocked_by"], serde_json::json!([]));
    }
}
