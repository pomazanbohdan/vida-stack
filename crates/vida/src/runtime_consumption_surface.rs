use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

pub(crate) const CANONICAL_LAUNCHER_COMMAND: &str = "vida";
pub(crate) const DOCFLOW_READINESS_CURRENT_PATH: &str =
    "vida/config/docflow-readiness.current.jsonl";
pub(crate) const DOCFLOW_PROOF_CURRENT_PATH: &str = "vida/config/docflow-proof.current.jsonl";
const MAX_LAUNCHER_BINARY_BYTES: u64 = 256 * 1024 * 1024;

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
    pub(crate) install_layout: Option<crate::release_surface::ReleaseInstallLayout>,
    pub(crate) active_executable_path: String,
    pub(crate) active_executable_fingerprint: String,
    pub(crate) installed_binaries: Vec<LauncherBinaryEvidence>,
    pub(crate) path_resolution: LauncherPathResolution,
    pub(crate) divergent_installed_binaries: bool,
    pub(crate) status: String,
    pub(crate) next_actions: Vec<String>,
}

#[derive(Debug, serde::Serialize, Clone, PartialEq, Eq)]
pub(crate) struct LauncherPathResolution {
    pub(crate) command: String,
    pub(crate) resolved_path: Option<String>,
    pub(crate) expected_runtime_bin_dir: Option<String>,
    pub(crate) expected_runtime_bin_on_path: bool,
    pub(crate) active_executable_on_path: bool,
    pub(crate) status: String,
}

pub(crate) fn doctor_launcher_summary_for_root(
    project_root: &Path,
) -> Result<DoctorLauncherSummary, String> {
    let active_executable_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve active vida executable: {error}"))?;
    let active_executable_fingerprint = launcher_binary_fingerprint(&active_executable_path)?;
    let install_layout = crate::release_surface::release_install_layout(None);
    let installed_binaries = installed_launcher_binary_evidence(&active_executable_path)?;
    let path_resolution =
        launcher_path_resolution(&active_executable_path, install_layout.as_ref());
    let divergent_installed_binaries =
        installed_binary_divergence(&installed_binaries, install_layout.as_ref());
    let mut next_actions = Vec::new();
    if divergent_installed_binaries {
        next_actions.push(
            "Installed `vida` binaries diverge by content; refresh the intended system binary and verify the shell resolves the expected runtime binary before collecting runtime proofs.".to_string(),
        );
    }
    if path_resolution.status == "warn" {
        next_actions.push(launcher_path_resolution_next_action(
            install_layout.as_ref(),
        ));
    }
    let status = if divergent_installed_binaries || path_resolution.status == "warn" {
        "warn"
    } else {
        "pass"
    };
    Ok(DoctorLauncherSummary {
        vida: CANONICAL_LAUNCHER_COMMAND.to_string(),
        project_root: project_root.display().to_string(),
        taskflow_surface: "vida taskflow".to_string(),
        install_layout,
        active_executable_path: active_executable_path.display().to_string(),
        active_executable_fingerprint,
        installed_binaries,
        path_resolution,
        divergent_installed_binaries,
        status: status.to_string(),
        next_actions,
    })
}

fn launcher_binary_fingerprint(path: &Path) -> Result<String, String> {
    if let Some(fingerprint) = cached_launcher_binary_fingerprint(path) {
        return Ok(fingerprint);
    }
    let file = std::fs::File::open(path).map_err(|error| {
        format!(
            "failed to open launcher binary `{}`: {error}",
            path.display()
        )
    })?;
    let size = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    if size > MAX_LAUNCHER_BINARY_BYTES {
        return Ok(launcher_binary_fingerprint_skipped(size));
    }
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut total_bytes = 0_u64;
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader.read(&mut chunk).map_err(|error| {
            format!(
                "failed to read launcher binary `{}`: {error}",
                path.display()
            )
        })?;
        if read == 0 {
            break;
        }
        total_bytes = total_bytes.saturating_add(read as u64);
        if total_bytes > MAX_LAUNCHER_BINARY_BYTES {
            return Ok(launcher_binary_fingerprint_skipped(total_bytes));
        }
        hasher.update(&chunk[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn cached_launcher_binary_fingerprint(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified_unix_ms = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    let cache_path = launcher_binary_fingerprint_metadata_path(path);
    let payload: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(cache_path).ok()?).ok()?;
    if payload["schema_version"].as_str()? != "vida-binary-fingerprint-v1" {
        return None;
    }
    if payload["len"].as_u64()? != metadata.len() {
        return None;
    }
    if payload["modified_unix_ms"].as_u64()? as u128 != modified_unix_ms {
        return None;
    }
    payload["fingerprint"].as_str().map(ToOwned::to_owned)
}

fn launcher_binary_fingerprint_metadata_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.fingerprint.json",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("binary")
    ))
}

fn launcher_binary_fingerprint_skipped(size: u64) -> String {
    format!("fingerprint-skipped:size-exceeds-limit:{size}:{MAX_LAUNCHER_BINARY_BYTES}")
}

fn installed_launcher_binary_evidence(
    active_executable_path: &Path,
) -> Result<Vec<LauncherBinaryEvidence>, String> {
    let mut candidates = Vec::new();
    if let Some(root) = crate::release_surface::release_install_root(None) {
        push_launcher_bin_candidates(&mut candidates, &root.join("current").join("bin"));
    }
    candidates.push(active_executable_path.to_path_buf());

    let active_canonical = active_executable_path
        .canonicalize()
        .unwrap_or_else(|_| active_executable_path.to_path_buf());
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
        let active = canonical == active_canonical;
        let fingerprint = match launcher_binary_fingerprint(&canonical) {
            Ok(fingerprint) => fingerprint,
            Err(error) if !active => {
                eprintln!(
                    "Warning: skipping launcher candidate `{}`: {error}",
                    canonical.display()
                );
                continue;
            }
            Err(error) => return Err(error),
        };
        evidence.push(LauncherBinaryEvidence {
            fingerprint,
            active,
            path: canonical.display().to_string(),
        });
    }
    Ok(evidence)
}

fn installed_binary_divergence(
    binaries: &[LauncherBinaryEvidence],
    install_layout: Option<&crate::release_surface::ReleaseInstallLayout>,
) -> bool {
    let Some(install_layout) = install_layout else {
        return false;
    };
    let install_root = Path::new(&install_layout.install_root);
    binaries
        .iter()
        .filter(|entry| path_is_under(Path::new(&entry.path), install_root))
        .map(|entry| entry.fingerprint.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1
}

fn launcher_path_resolution(
    active_executable_path: &Path,
    install_layout: Option<&crate::release_surface::ReleaseInstallLayout>,
) -> LauncherPathResolution {
    launcher_path_resolution_with_path_env(
        active_executable_path,
        install_layout,
        std::env::var_os("PATH"),
    )
}

fn launcher_path_resolution_with_path_env(
    active_executable_path: &Path,
    install_layout: Option<&crate::release_surface::ReleaseInstallLayout>,
    path_env: Option<std::ffi::OsString>,
) -> LauncherPathResolution {
    let resolved = resolve_command_from_path_env(CANONICAL_LAUNCHER_COMMAND, path_env.clone());
    let expected_runtime_bin_dir = install_layout.map(|layout| layout.runtime_bin_dir.clone());
    let expected_runtime_bin_on_path = expected_runtime_bin_dir
        .as_ref()
        .is_some_and(|dir| path_env_contains_dir(Path::new(dir), path_env));
    let active_executable_on_path = resolved
        .as_ref()
        .is_some_and(|path| same_path(path, active_executable_path));
    let installed_active = install_layout.as_ref().is_some_and(|layout| {
        path_is_under(active_executable_path, Path::new(&layout.current_root))
    });
    let status = if installed_active && !active_executable_on_path {
        "warn"
    } else {
        "pass"
    };
    LauncherPathResolution {
        command: CANONICAL_LAUNCHER_COMMAND.to_string(),
        resolved_path: resolved.map(|path| path.display().to_string()),
        expected_runtime_bin_dir,
        expected_runtime_bin_on_path,
        active_executable_on_path,
        status: status.to_string(),
    }
}

fn resolve_command_from_path_env(
    command: &str,
    path_env: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    let path_env = path_env?;
    let names = launcher_command_file_names(command);
    for dir in std::env::split_paths(&path_env) {
        for name in &names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate.canonicalize().unwrap_or(candidate));
            }
        }
    }
    None
}

fn path_env_contains_dir(dir: &Path, path_env: Option<std::ffi::OsString>) -> bool {
    let Some(path_env) = path_env else {
        return false;
    };
    std::env::split_paths(&path_env).any(|entry| same_path(&entry, dir))
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn path_is_under(path: &Path, root: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.starts_with(root)
}

fn launcher_path_resolution_next_action(
    install_layout: Option<&crate::release_surface::ReleaseInstallLayout>,
) -> String {
    if cfg!(windows) {
        if let Some(layout) = install_layout {
            return format!(
                "The active installed `vida` was launched directly, but this shell does not resolve it from PATH; run `. \"{}\"` in PowerShell or restart the host shell after installer PATH updates.",
                layout.env_file
            );
        }
        return "The active installed `vida` was launched directly, but this shell does not resolve it from PATH; source the installer env file or restart the host shell.".to_string();
    }
    if let Some(layout) = install_layout {
        return format!(
            "The active installed `vida` was launched directly, but this shell does not resolve it from PATH; run `source \"{}\"` or reload the shell profile installed by the Unix installer.",
            layout.env_file
        );
    }
    "The active installed `vida` was launched directly, but this shell does not resolve it from PATH; source the installer env file or reload the shell profile.".to_string()
}

fn push_launcher_bin_candidates(candidates: &mut Vec<PathBuf>, bin_root: &Path) {
    for file_name in launcher_vida_file_names() {
        candidates.push(bin_root.join(file_name));
    }
}

fn launcher_vida_file_names() -> Vec<String> {
    launcher_command_file_names(CANONICAL_LAUNCHER_COMMAND)
}

fn launcher_command_file_names(command: &str) -> Vec<String> {
    let canonical = format!("{command}{}", std::env::consts::EXE_SUFFIX);
    if canonical == command {
        vec![canonical]
    } else {
        vec![canonical, command.to_string()]
    }
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
pub(crate) struct RuntimeConsumptionClosureAdmissionEvidence {
    pub(crate) requirement: String,
    pub(crate) status: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RuntimeConsumptionClosureAdmission {
    pub(crate) status: String,
    pub(crate) admitted: bool,
    pub(crate) blockers: Vec<String>,
    pub(crate) proof_surfaces: Vec<String>,
    pub(crate) evidence_table: Vec<RuntimeConsumptionClosureAdmissionEvidence>,
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
                evidence_table: closure_admission
                    .evidence_table
                    .iter()
                    .map(|row| serde_json::to_value(row).expect("evidence row should serialize"))
                    .collect(),
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
    pub(crate) requested_owned_paths: Vec<String>,
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
        CANONICAL_LAUNCHER_COMMAND, LauncherBinaryEvidence, RuntimeConsumptionClosureAdmission,
        RuntimeConsumptionEvidence, build_docflow_receipt_evidence,
        canonical_closure_admission_artifact_json, doctor_launcher_summary_for_root,
        launcher_binary_fingerprint_skipped,
    };
    use std::path::PathBuf;

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
        assert!(summary.installed_binaries.iter().any(|binary| {
            binary.active
                && PathBuf::from(&binary.path)
                    == PathBuf::from(&summary.active_executable_path)
                        .canonicalize()
                        .expect("active executable path should canonicalize")
        }));
    }

    #[test]
    fn oversized_launcher_fingerprint_degrades_to_bounded_marker() {
        let marker = launcher_binary_fingerprint_skipped(super::MAX_LAUNCHER_BINARY_BYTES + 1);

        assert_eq!(
            marker,
            "fingerprint-skipped:size-exceeds-limit:268435457:268435456"
        );
    }

    #[test]
    fn launcher_path_helpers_resolve_platform_command_without_user_hardcode() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-launcher-path-resolution-{}-{}",
            std::process::id(),
            nanos
        ));
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).expect("bin dir should write");
        let binary = bin.join(crate::release_surface::vida_binary_file_name());
        std::fs::write(&binary, b"fake vida").expect("fake vida should write");

        let path_env = std::env::join_paths([bin.clone()]).expect("path env should join");

        let resolved = super::resolve_command_from_path_env("vida", Some(path_env.clone()))
            .expect("vida should resolve from synthetic PATH");
        assert_eq!(
            resolved,
            binary
                .canonicalize()
                .expect("fake vida should canonicalize")
        );
        assert!(super::path_env_contains_dir(&bin, Some(path_env)));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn installed_binary_divergence_ignores_non_installed_active_executable() {
        let install_root = std::env::temp_dir().join("vida-launcher-install-root");
        let layout = crate::release_surface::ReleaseInstallLayout {
            env_file: install_root.join("env.ps1").display().to_string(),
            install_root: install_root.display().to_string(),
            current_root: install_root.join("current").display().to_string(),
            runtime_bin_dir: install_root
                .join("current")
                .join("bin")
                .display()
                .to_string(),
            platform: std::env::consts::OS.to_string(),
        };
        let installed = LauncherBinaryEvidence {
            path: install_root
                .join("releases")
                .join("v0.9.7")
                .join("bin")
                .join(crate::release_surface::vida_binary_file_name())
                .display()
                .to_string(),
            fingerprint: "installed-release".to_string(),
            active: false,
        };
        let debug_active = LauncherBinaryEvidence {
            path: std::env::temp_dir()
                .join("vida-stack")
                .join("target")
                .join("debug")
                .join(crate::release_surface::vida_binary_file_name())
                .display()
                .to_string(),
            fingerprint: "debug-build".to_string(),
            active: true,
        };

        assert!(!super::installed_binary_divergence(
            &[installed.clone(), debug_active],
            Some(&layout)
        ));

        let stale_installed = LauncherBinaryEvidence {
            path: install_root
                .join("releases")
                .join("v0.9.6")
                .join("bin")
                .join(crate::release_surface::vida_binary_file_name())
                .display()
                .to_string(),
            fingerprint: "stale-release".to_string(),
            active: false,
        };
        assert!(super::installed_binary_divergence(
            &[installed, stale_installed],
            Some(&layout)
        ));
    }

    #[test]
    fn launcher_path_resolution_passes_when_path_resolves_installed_and_active_is_debug() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let install_root = std::env::temp_dir().join(format!(
            "vida-launcher-installed-path-{}-{}",
            std::process::id(),
            nanos
        ));
        let current_bin = install_root.join("current").join("bin");
        std::fs::create_dir_all(&current_bin).expect("current bin dir should write");
        let installed_binary = current_bin.join(crate::release_surface::vida_binary_file_name());
        std::fs::write(&installed_binary, b"installed vida")
            .expect("installed binary should write");
        let layout = crate::release_surface::ReleaseInstallLayout {
            env_file: install_root.join("env.ps1").display().to_string(),
            install_root: install_root.display().to_string(),
            current_root: install_root.join("current").display().to_string(),
            runtime_bin_dir: current_bin.display().to_string(),
            platform: std::env::consts::OS.to_string(),
        };
        let active_debug = install_root
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("vida-stack")
            .join("target")
            .join("debug")
            .join(crate::release_surface::vida_binary_file_name());
        let path_env = std::env::join_paths([current_bin.clone()]).expect("path env should join");

        let resolution = super::launcher_path_resolution_with_path_env(
            &active_debug,
            Some(&layout),
            Some(path_env),
        );

        assert_eq!(resolution.status, "pass");
        assert!(resolution.expected_runtime_bin_on_path);
        assert!(!resolution.active_executable_on_path);
        assert_eq!(
            PathBuf::from(resolution.resolved_path.expect("vida should resolve")),
            installed_binary
                .canonicalize()
                .expect("installed binary should canonicalize")
        );

        let _ = std::fs::remove_dir_all(install_root);
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
            evidence_table: vec![super::RuntimeConsumptionClosureAdmissionEvidence {
                requirement: "docflow_readiness".to_string(),
                status: "pass".to_string(),
                evidence_refs: vec![
                    "vida docflow readiness-check --profile active-canon".to_string(),
                ],
                blockers: Vec::new(),
            }],
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
            artifact["evidence_table"][0]["requirement"],
            "docflow_readiness"
        );
        assert_eq!(
            artifact["evidence_bundle_refs"][1],
            "vida docflow proofcheck --profile active-canon"
        );
        assert_eq!(artifact["blocked_by"], serde_json::json!([]));
    }
}
