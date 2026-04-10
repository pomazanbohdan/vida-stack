use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub(crate) struct DoctorLauncherSummary {
    pub(crate) vida: String,
    pub(crate) project_root: String,
    pub(crate) taskflow_surface: String,
}

pub(crate) fn doctor_launcher_summary_for_root(
    project_root: &Path,
) -> Result<DoctorLauncherSummary, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    Ok(DoctorLauncherSummary {
        vida: current_exe.display().to_string(),
        project_root: project_root.display().to_string(),
        taskflow_surface: "vida taskflow".to_string(),
    })
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

#[derive(Debug, serde::Serialize)]
pub(crate) struct RuntimeConsumptionClosureAdmission {
    pub(crate) status: String,
    pub(crate) admitted: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) proof_surfaces: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct TaskflowDirectConsumptionPayload {
    pub(crate) artifact_name: String,
    pub(crate) artifact_type: String,
    pub(crate) generated_at: String,
    pub(crate) closure_authority: String,
    pub(crate) request_text: String,
    pub(crate) role_selection: crate::RuntimeConsumptionLaneSelection,
    pub(crate) runtime_bundle: TaskflowConsumeBundlePayload,
    pub(crate) bundle_check: TaskflowConsumeBundleCheck,
    pub(crate) docflow_activation: RuntimeConsumptionDocflowActivation,
    pub(crate) docflow_verdict: RuntimeConsumptionDocflowVerdict,
    pub(crate) closure_admission: RuntimeConsumptionClosureAdmission,
    pub(crate) taskflow_handoff_plan: serde_json::Value,
    pub(crate) run_graph_bootstrap: serde_json::Value,
    pub(crate) dispatch_receipt: serde_json::Value,
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
        artifact_path: Some("vida/config/docflow-readiness.current.jsonl".to_string()),
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
        artifact_path: None,
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
