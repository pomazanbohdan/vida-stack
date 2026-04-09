use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::state_store::{ProtocolBindingState, ProtocolBindingSummary, StateStore};
use crate::contract_profile_adapter::{
    blocker_code, canonical_blocker_code_list, evaluate_policy_gate_protocol_binding,
    release_contract_status, BlockerCode,
};
use crate::operator_contracts::shared_operator_output_contract_parity_error;

#[derive(Clone, serde::Serialize)]
struct ProtocolBindingDecisionGateStatus {
    policy_gate: String,
    ready: bool,
    blocker_code: Option<String>,
}

struct TaskflowProtocolBindingSeed {
    protocol_id: &'static str,
    source_path: &'static str,
    activation_class: &'static str,
    runtime_owner: &'static str,
    enforcement_type: &'static str,
    proof_surface: &'static str,
}

fn taskflow_protocol_binding_seeds() -> &'static [TaskflowProtocolBindingSeed] {
    &[
        TaskflowProtocolBindingSeed {
            protocol_id: "instruction-contracts/bridge.instruction-activation-protocol",
            source_path:
                "vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md",
            activation_class: "always_on",
            runtime_owner: "vida::taskflow::protocol_binding::activation_bridge",
            enforcement_type: "activation-resolution",
            proof_surface: "vida docflow activation-check --profile active-canon",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/work.taskflow-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.taskflow-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::taskflow::protocol_binding::taskflow_surface",
            enforcement_type: "execution-discipline",
            proof_surface: "vida taskflow consume bundle check --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/runtime.task-state-telemetry-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::state_store::task_state_telemetry",
            enforcement_type: "state-telemetry",
            proof_surface: "vida status --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/work.execution-health-check-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md",
            activation_class: "triggered_domain",
            runtime_owner: "vida::doctor::execution_health",
            enforcement_type: "health-gate",
            proof_surface: "vida taskflow doctor --json",
        },
        TaskflowProtocolBindingSeed {
            protocol_id: "runtime-instructions/work.task-state-reconciliation-protocol",
            source_path:
                "vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md",
            activation_class: "closure_reflection",
            runtime_owner: "vida::state_store::task_reconciliation",
            enforcement_type: "state-reconciliation",
            proof_surface: "vida status --json",
        },
    ]
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct ProtocolBindingCompiledPayloadImportEvidence {
    pub(crate) imported: bool,
    pub(crate) trusted: bool,
    pub(crate) source: String,
    pub(crate) source_config_path: String,
    pub(crate) source_config_digest: String,
    pub(crate) captured_at: String,
    pub(crate) effective_bundle_receipt_id: String,
    pub(crate) effective_bundle_root_artifact_id: String,
    pub(crate) effective_bundle_artifact_count: usize,
    pub(crate) compiled_payload_summary: serde_json::Value,
    pub(crate) blockers: Vec<String>,
}

impl ProtocolBindingCompiledPayloadImportEvidence {
    fn trusted(
        source: &str,
        source_config_digest: &str,
        effective_bundle_receipt_id: &str,
    ) -> bool {
        source == "state_store"
            && !source_config_digest.trim().is_empty()
            && !effective_bundle_receipt_id.trim().is_empty()
    }
}

fn protocol_binding_compiled_payload_import_ready(
    evidence: &ProtocolBindingCompiledPayloadImportEvidence,
) -> bool {
    evidence.imported
        && evidence.trusted
        && evidence.source == "state_store"
        && !evidence.source_config_digest.trim().is_empty()
        && !evidence.effective_bundle_receipt_id.trim().is_empty()
}

fn has_non_empty_string_field(payload: &serde_json::Value, path: &[&str]) -> bool {
    let mut current = payload;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return false;
        };
        current = next;
    }
    current
        .as_str()
        .map(str::trim)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

pub(crate) async fn protocol_binding_compiled_payload_import_evidence(
    store: &StateStore,
) -> ProtocolBindingCompiledPayloadImportEvidence {
    let mut blockers = Vec::new();

    let activation_snapshot = match super::read_or_sync_launcher_activation_snapshot(store).await {
        Ok(snapshot) => Some(snapshot),
        Err(_) => None,
    };
    let effective_bundle_receipt = match store.latest_effective_bundle_receipt_summary().await {
        Ok(receipt) => receipt,
        Err(_) => None,
    };

    let (source, source_config_path, source_config_digest, captured_at, compiled_payload_summary) =
        if let Some(snapshot) = activation_snapshot.as_ref() {
            (
                snapshot.source.clone(),
                snapshot.source_config_path.clone(),
                snapshot.source_config_digest.clone(),
                snapshot.captured_at.clone(),
                serde_json::json!({
                    "selection_mode": snapshot.compiled_bundle["role_selection"]["mode"],
                    "fallback_role": snapshot.compiled_bundle["role_selection"]["fallback_role"],
                    "agent_system_mode": snapshot.compiled_bundle["agent_system"]["mode"],
                    "agent_system_state_owner": snapshot.compiled_bundle["agent_system"]["state_owner"],
                }),
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                serde_json::json!({}),
            )
        };
    let effective_bundle_receipt_id = effective_bundle_receipt
        .as_ref()
        .map(|receipt| receipt.receipt_id.clone())
        .unwrap_or_default();
    let effective_bundle_root_artifact_id = effective_bundle_receipt
        .as_ref()
        .map(|receipt| receipt.root_artifact_id.clone())
        .unwrap_or_default();
    let effective_bundle_artifact_count = effective_bundle_receipt
        .as_ref()
        .map(|receipt| receipt.artifact_count)
        .unwrap_or_default();

    if source.is_empty() {
        if let Some(code) = blocker_code(BlockerCode::MissingLauncherActivationSnapshot) {
            blockers.push(code);
        }
    } else if source != "state_store" {
        if let Some(code) = blocker_code(BlockerCode::SourceUnregistered) {
            blockers.push(code);
        }
    }
    if let Some(snapshot) = activation_snapshot.as_ref() {
        if !has_non_empty_string_field(&snapshot.compiled_bundle, &["role_selection", "mode"]) {
            if let Some(code) = blocker_code(BlockerCode::InvalidCompiledBundleRoleSelectionMode) {
                blockers.push(code);
            }
        }
        if !has_non_empty_string_field(&snapshot.compiled_bundle, &["agent_system", "mode"]) {
            if let Some(code) = blocker_code(BlockerCode::InvalidCompiledBundleAgentSystemMode) {
                blockers.push(code);
            }
        }
        if !has_non_empty_string_field(&snapshot.compiled_bundle, &["agent_system", "state_owner"])
        {
            if let Some(code) =
                blocker_code(BlockerCode::InvalidCompiledBundleAgentSystemStateOwner)
            {
                blockers.push(code);
            }
        }
    }
    if let Some(receipt) = effective_bundle_receipt.as_ref() {
        if receipt.receipt_id.trim().is_empty() {
            if let Some(code) = blocker_code(BlockerCode::MissingEffectiveBundleReceiptId) {
                blockers.push(code);
            }
        }
        if receipt.root_artifact_id.trim().is_empty() {
            if let Some(code) = blocker_code(BlockerCode::MissingEffectiveBundleRootArtifactId) {
                blockers.push(code);
            }
        }
        if receipt.artifact_count == 0 {
            if let Some(code) = blocker_code(BlockerCode::EmptyEffectiveBundleArtifactCount) {
                blockers.push(code);
            }
        }
    } else {
        if let Some(code) = blocker_code(BlockerCode::MissingEffectiveBundleReceipt) {
            blockers.push(code);
        }
    }

    // `source_config_path` is retained as provenance only; trust comes from the
    // authoritative source, source digest, and effective receipt identity.
    let trusted = ProtocolBindingCompiledPayloadImportEvidence::trusted(
        &source,
        &source_config_digest,
        &effective_bundle_receipt_id,
    ) && blockers.is_empty();

    ProtocolBindingCompiledPayloadImportEvidence {
        imported: activation_snapshot.is_some() && effective_bundle_receipt.is_some(),
        trusted,
        source,
        source_config_path,
        source_config_digest,
        captured_at,
        effective_bundle_receipt_id,
        effective_bundle_root_artifact_id,
        effective_bundle_artifact_count,
        compiled_payload_summary,
        blockers,
    }
}

fn resolve_protocol_binding_source_root() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Ok(root) = super::resolve_repo_root() {
        candidates.push(root);
    }
    if let Some(installed_root) = super::init_surfaces::resolve_installed_runtime_root() {
        candidates.push(installed_root.join("current"));
        candidates.push(installed_root);
    }
    let repo_root = super::repo_runtime_root();
    if !candidates.iter().any(|root| root == &repo_root) {
        candidates.push(repo_root);
    }

    super::init_surfaces::first_existing_path(
        &candidates
            .into_iter()
            .map(|root| root.join("vida/config/instructions/system-maps/protocol.index.md"))
            .collect::<Vec<_>>(),
    )
    .and_then(|path| {
        path.parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    })
    .ok_or_else(|| {
        "Unable to resolve protocol-binding source root with vida/config/instructions/system-maps/protocol.index.md"
            .to_string()
    })
}

fn build_taskflow_protocol_binding_rows(
    evidence: &ProtocolBindingCompiledPayloadImportEvidence,
) -> Result<Vec<ProtocolBindingState>, String> {
    let repo_root = resolve_protocol_binding_source_root()?;
    let protocol_index_path =
        repo_root.join("vida/config/instructions/system-maps/protocol.index.md");
    let protocol_index = fs::read_to_string(&protocol_index_path).map_err(|error| {
        format!(
            "Failed to read protocol index {}: {error}",
            protocol_index_path.display()
        )
    })?;

    let mut rows = Vec::new();
    for seed in taskflow_protocol_binding_seeds() {
        let source = repo_root.join(seed.source_path);
        let mut blockers = Vec::new();
        if !source.exists() {
            if let Some(code) = blocker_code(BlockerCode::SchemaContractMissing) {
                blockers.push(code);
            }
        }
        if !protocol_index.contains(&format!("`{}`", seed.protocol_id)) {
            if let Some(code) = blocker_code(BlockerCode::SchemaContractMissing) {
                blockers.push(code);
            }
        }
        blockers.extend(evidence.blockers.iter().cloned());

        rows.push(ProtocolBindingState {
            protocol_id: seed.protocol_id.to_string(),
            source_path: seed.source_path.to_string(),
            activation_class: seed.activation_class.to_string(),
            runtime_owner: seed.runtime_owner.to_string(),
            enforcement_type: seed.enforcement_type.to_string(),
            proof_surface: seed.proof_surface.to_string(),
            primary_state_authority: super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY.to_string(),
            binding_status: if blockers.is_empty() {
                "fully-runtime-bound".to_string()
            } else {
                "unbound".to_string()
            },
            active: true,
            blockers,
            scenario: super::TASKFLOW_PROTOCOL_BINDING_SCENARIO.to_string(),
            synced_at: String::new(),
        });
    }
    Ok(rows)
}

fn protocol_binding_check_ok(
    summary: &ProtocolBindingSummary,
    rows: &[ProtocolBindingState],
    evidence: &ProtocolBindingCompiledPayloadImportEvidence,
) -> bool {
    protocol_binding_decision_gate_status(summary, evidence)
        .blocker_code
        .is_none()
        && protocol_binding_compiled_payload_import_ready(evidence)
        && summary.total_receipts > 0
        && summary.total_bindings == taskflow_protocol_binding_seeds().len()
        && summary.unbound_count == 0
        && summary.blocking_issue_count == 0
        && summary.script_bound_count == 0
        && summary.fully_runtime_bound_count == taskflow_protocol_binding_seeds().len()
        && rows.len() == taskflow_protocol_binding_seeds().len()
        && rows
            .iter()
            .all(|row| row.binding_status == "fully-runtime-bound" && row.blockers.is_empty())
}

fn protocol_binding_decision_gate_status(
    summary: &ProtocolBindingSummary,
    evidence: &ProtocolBindingCompiledPayloadImportEvidence,
) -> ProtocolBindingDecisionGateStatus {
    let policy_gate = "retrieval_evidence";
    let receipt_hint = if summary.total_receipts > 0 {
        Some("protocol_binding_summary_receipt")
    } else {
        Some(evidence.effective_bundle_receipt_id.as_str())
    };
    let runtime_ready = protocol_binding_compiled_payload_import_ready(evidence)
        && summary.total_receipts > 0
        && summary.unbound_count == 0
        && summary.blocking_issue_count == 0
        && summary.fully_runtime_bound_count == taskflow_protocol_binding_seeds().len();
    let blocker = evaluate_policy_gate_protocol_binding(policy_gate, receipt_hint, runtime_ready);
    let canonical_blocker_code = blocker.as_ref().map(|code: &String| {
        canonical_blocker_code_list([code.as_str()])
            .into_iter()
            .next()
            .unwrap_or_else(|| code.clone())
    });
    ProtocolBindingDecisionGateStatus {
        policy_gate: policy_gate.to_string(),
        ready: blocker.is_none(),
        blocker_code: canonical_blocker_code,
    }
}

fn build_protocol_binding_operator_contract_fields(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
    })
}

fn ensure_protocol_binding_operator_contract_parity(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Result<(serde_json::Value, serde_json::Value), String> {
    let operator_contracts =
        build_protocol_binding_operator_contract_fields(status, blocker_codes, next_actions);
    let shared_fields = operator_contracts.clone();
    let parity_payload = serde_json::json!({
        "status": status,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "shared_fields": shared_fields.clone(),
        "operator_contracts": operator_contracts.clone(),
    });
    match shared_operator_output_contract_parity_error(&parity_payload) {
        None => Ok((operator_contracts, shared_fields)),
        Some(error) => Err(error.to_string()),
    }
}

struct ProtocolBindingOperatorContractPayload {
    status: String,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    operator_contracts: serde_json::Value,
    shared_fields: serde_json::Value,
}

fn protocol_binding_operator_contract_payload(
    decision_gate: &ProtocolBindingDecisionGateStatus,
    ok: bool,
) -> Result<ProtocolBindingOperatorContractPayload, String> {
    let status = release_contract_status(ok);
    let blocker_codes = decision_gate
        .blocker_code
        .as_ref()
        .cloned()
        .map(|code| vec![code])
        .unwrap_or_default();
    let next_actions = if blocker_codes.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Run `vida taskflow protocol-binding check --json` to resolve {} gate blockers.",
            decision_gate.policy_gate
        )]
    };
    let (operator_contracts, shared_fields) =
        ensure_protocol_binding_operator_contract_parity(status, &blocker_codes, &next_actions)?;
    Ok(ProtocolBindingOperatorContractPayload {
        status: status.to_string(),
        blocker_codes,
        next_actions,
        operator_contracts,
        shared_fields,
    })
}

struct ProtocolBindingCheckContext {
    evidence: ProtocolBindingCompiledPayloadImportEvidence,
    summary: ProtocolBindingSummary,
    rows: Vec<ProtocolBindingState>,
    decision_gate: ProtocolBindingDecisionGateStatus,
    ok: bool,
    payload: ProtocolBindingOperatorContractPayload,
}

async fn build_protocol_binding_check_context(
    state_dir: &Path,
) -> Result<ProtocolBindingCheckContext, String> {
    let store = super::StateStore::open_existing(state_dir.to_path_buf())
        .await
        .map_err(|error| format!("Failed to open authoritative state store: {error}"))?;
    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
    let summary = store
        .protocol_binding_summary()
        .await
        .map_err(|error| format!("Failed to read protocol-binding summary: {error}"))?;
    let rows = store
        .latest_protocol_binding_rows()
        .await
        .map_err(|error| format!("Failed to read protocol-binding rows: {error}"))?;
    let decision_gate = protocol_binding_decision_gate_status(&summary, &evidence);
    let ok = protocol_binding_check_ok(&summary, &rows, &evidence);
    let payload = protocol_binding_operator_contract_payload(&decision_gate, ok)
        .map_err(|error| format!("protocol-binding contract parity guard: {error}"))?;
    Ok(ProtocolBindingCheckContext {
        evidence,
        summary,
        rows,
        decision_gate,
        ok,
        payload,
    })
}

fn render_protocol_binding_check_plain(context: &ProtocolBindingCheckContext) {
    super::print_surface_header(
        super::RenderMode::Plain,
        "vida taskflow protocol-binding check",
    );
    super::print_surface_line(super::RenderMode::Plain, "status", &context.payload.status);
    super::print_surface_line(
        super::RenderMode::Plain,
        "summary",
        &context.summary.as_display(),
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "compiled payload import",
        if context.evidence.trusted {
            "trusted"
        } else {
            "blocked"
        },
    );
    super::print_surface_line(
        super::RenderMode::Plain,
        "decision gate",
        &format!(
            "{} ({})",
            context.decision_gate.policy_gate,
            context
                .decision_gate
                .blocker_code
                .as_deref()
                .unwrap_or("ready")
        ),
    );
    let blocker_codes_list = serde_json::to_string(&context.payload.blocker_codes)
        .expect("protocol-binding blocker_codes should render");
    super::print_surface_line(
        super::RenderMode::Plain,
        "blocker_codes",
        &blocker_codes_list,
    );
    let next_actions_list = serde_json::to_string(&context.payload.next_actions)
        .expect("protocol-binding next_actions should render");
    super::print_surface_line(super::RenderMode::Plain, "next_actions", &next_actions_list);
    let shared_fields_list = serde_json::to_string(&context.payload.shared_fields)
        .expect("protocol-binding shared_fields should render");
    super::print_surface_line(
        super::RenderMode::Plain,
        "shared_fields",
        &shared_fields_list,
    );
    let operator_contracts_list = serde_json::to_string(&context.payload.operator_contracts)
        .expect("protocol-binding operator_contracts should render");
    super::print_surface_line(
        super::RenderMode::Plain,
        "operator_contracts",
        &operator_contracts_list,
    );
}

fn render_protocol_binding_check_json(context: &ProtocolBindingCheckContext) {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "surface": "vida taskflow protocol-binding check",
            "status": context.payload.status,
            "decision_gate": context.decision_gate,
            "blocker_codes": context.payload.blocker_codes,
            "next_actions": context.payload.next_actions,
            "compiled_payload_import_evidence": context.evidence,
            "summary": context.summary,
            "bindings": context.rows,
            "shared_fields": context.payload.shared_fields.clone(),
            "operator_contracts": context.payload.operator_contracts.clone(),
        }))
        .expect("protocol-binding check should render as json")
    );
}

async fn run_protocol_binding_check(state_dir: &Path, json: bool) -> ExitCode {
    match build_protocol_binding_check_context(state_dir).await {
        Ok(context) => {
            if json {
                render_protocol_binding_check_json(&context);
            } else {
                render_protocol_binding_check_plain(&context);
            }
            if context.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_taskflow_protocol_binding(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "protocol-binding" => {
            super::print_taskflow_proxy_help(Some("protocol-binding"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "protocol-binding" && matches!(flag.as_str(), "--help" | "-h") => {
            super::print_taskflow_proxy_help(Some("protocol-binding"));
            ExitCode::SUCCESS
        }
        [head, subcommand] if head == "protocol-binding" && subcommand == "sync" => {
            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match super::StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let rows = match build_taskflow_protocol_binding_rows(&evidence) {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    match store
                        .record_protocol_binding_snapshot(
                            super::TASKFLOW_PROTOCOL_BINDING_SCENARIO,
                            super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
                            &rows,
                        )
                        .await
                    {
                        Ok(receipt) => {
                            super::print_surface_header(
                                super::RenderMode::Plain,
                                "vida taskflow protocol-binding sync",
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "receipt",
                                &receipt.receipt_id,
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "scenario",
                                &receipt.scenario,
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "authority",
                                &receipt.primary_state_authority,
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "bindings",
                                &receipt.total_bindings.to_string(),
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "blocking issues",
                                &receipt.blocking_issue_count.to_string(),
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "compiled payload import",
                                if evidence.trusted {
                                    "trusted"
                                } else {
                                    "blocked"
                                },
                            );
                            if receipt.unbound_count == 0
                                && receipt.blocking_issue_count == 0
                                && evidence.trusted
                            {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to record protocol-binding state: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "sync" && flag == "--json" =>
        {
            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match super::StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let rows = match build_taskflow_protocol_binding_rows(&evidence) {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                    match store
                        .record_protocol_binding_snapshot(
                            super::TASKFLOW_PROTOCOL_BINDING_SCENARIO,
                            super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
                            &rows,
                        )
                        .await
                    {
                        Ok(receipt) => {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "surface": "vida taskflow protocol-binding sync",
                                    "compiled_payload_import_evidence": evidence,
                                    "receipt": receipt,
                                    "bindings": rows,
                                }))
                                .expect("protocol-binding sync should render as json")
                            );
                            if rows.iter().all(|row| row.blockers.is_empty()) && evidence.trusted {
                                ExitCode::SUCCESS
                            } else {
                                ExitCode::from(1)
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to record protocol-binding state: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "protocol-binding" && subcommand == "status" => {
            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match super::StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let summary = match store.protocol_binding_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match store.latest_protocol_binding_rows().await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    super::print_surface_header(
                        super::RenderMode::Plain,
                        "vida taskflow protocol-binding status",
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "summary",
                        &summary.as_display(),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "compiled payload import",
                        if evidence.trusted {
                            "trusted"
                        } else {
                            "blocked"
                        },
                    );
                    for row in rows {
                        super::print_surface_line(
                            super::RenderMode::Plain,
                            "binding",
                            &row.as_display(),
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "status" && flag == "--json" =>
        {
            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match super::StateStore::open_existing(state_dir).await {
                Ok(store) => {
                    let evidence = protocol_binding_compiled_payload_import_evidence(&store).await;
                    let summary = match store.protocol_binding_summary().await {
                        Ok(summary) => summary,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding summary: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let rows = match store.latest_protocol_binding_rows().await {
                        Ok(rows) => rows,
                        Err(error) => {
                            eprintln!("Failed to read protocol-binding rows: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow protocol-binding status",
                            "compiled_payload_import_evidence": evidence,
                            "summary": summary,
                            "bindings": rows,
                        }))
                        .expect("protocol-binding status should render as json")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand] if head == "protocol-binding" && subcommand == "check" => {
            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            run_protocol_binding_check(&state_dir, false).await
        }
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "check" && flag == "--json" =>
        {
            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            run_protocol_binding_check(&state_dir, true).await
        }
        [head, subcommand, ..] if head == "protocol-binding" && subcommand == "sync" => {
            eprintln!("Usage: vida taskflow protocol-binding sync [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "protocol-binding" && subcommand == "status" => {
            eprintln!("Usage: vida taskflow protocol-binding status [--json]");
            ExitCode::from(2)
        }
        [head, subcommand, ..] if head == "protocol-binding" && subcommand == "check" => {
            eprintln!("Usage: vida taskflow protocol-binding check [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

pub(crate) async fn sync_taskflow_protocol_binding_snapshot(
    store: &StateStore,
) -> Result<(), String> {
    let evidence = protocol_binding_compiled_payload_import_evidence(store).await;
    let rows = build_taskflow_protocol_binding_rows(&evidence)?;
    store
        .record_protocol_binding_snapshot(
            super::TASKFLOW_PROTOCOL_BINDING_SCENARIO,
            super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
            &rows,
        )
        .await
        .map_err(|error| format!("Failed to record protocol-binding snapshot: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{protocol_binding_check_ok, ProtocolBindingCompiledPayloadImportEvidence};
    use crate::contract_profile_adapter::release_contract_status;
    use crate::state_store::{ProtocolBindingState, ProtocolBindingSummary};

    fn sample_evidence(
        imported: bool,
        trusted: bool,
    ) -> ProtocolBindingCompiledPayloadImportEvidence {
        ProtocolBindingCompiledPayloadImportEvidence {
            imported,
            trusted,
            source: "state_store".to_string(),
            source_config_path: String::new(),
            source_config_digest: "digest-1".to_string(),
            captured_at: "2026-03-17T00:00:00Z".to_string(),
            effective_bundle_receipt_id: "receipt-1".to_string(),
            effective_bundle_root_artifact_id: "root-1".to_string(),
            effective_bundle_artifact_count: 1,
            compiled_payload_summary: serde_json::json!({}),
            blockers: vec![],
        }
    }

    fn sample_summary() -> ProtocolBindingSummary {
        let seed_count = super::taskflow_protocol_binding_seeds().len();
        ProtocolBindingSummary {
            total_receipts: 1,
            total_bindings: seed_count,
            active_bindings: seed_count,
            script_bound_count: 0,
            rust_bound_count: 0,
            fully_runtime_bound_count: seed_count,
            unbound_count: 0,
            blocking_issue_count: 0,
            latest_receipt_id: Some("receipt-1".to_string()),
            latest_scenario: Some(super::super::TASKFLOW_PROTOCOL_BINDING_SCENARIO.to_string()),
            latest_recorded_at: Some("2026-03-17T00:00:00Z".to_string()),
            primary_state_authority: Some(
                super::super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY.to_string(),
            ),
        }
    }

    fn sample_rows() -> Vec<ProtocolBindingState> {
        super::taskflow_protocol_binding_seeds()
            .iter()
            .map(|seed| ProtocolBindingState {
                protocol_id: seed.protocol_id.to_string(),
                source_path: seed.source_path.to_string(),
                activation_class: seed.activation_class.to_string(),
                runtime_owner: seed.runtime_owner.to_string(),
                enforcement_type: seed.enforcement_type.to_string(),
                proof_surface: seed.proof_surface.to_string(),
                primary_state_authority: super::super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY
                    .to_string(),
                binding_status: "fully-runtime-bound".to_string(),
                active: true,
                blockers: vec![],
                scenario: super::super::TASKFLOW_PROTOCOL_BINDING_SCENARIO.to_string(),
                synced_at: "2026-03-17T00:00:00Z".to_string(),
            })
            .collect()
    }

    #[test]
    fn protocol_binding_check_ok_accepts_ready_release1_payload() {
        let summary = sample_summary();
        let rows = sample_rows();
        let evidence = sample_evidence(true, true);

        assert!(protocol_binding_check_ok(&summary, &rows, &evidence));
        assert_eq!(
            release_contract_status(protocol_binding_check_ok(&summary, &rows, &evidence)),
            "pass"
        );
    }

    #[test]
    fn protocol_binding_check_ok_blocks_untrusted_evidence() {
        let summary = sample_summary();
        let rows = sample_rows();
        let evidence = sample_evidence(true, false);

        assert!(!protocol_binding_check_ok(&summary, &rows, &evidence));
        assert_eq!(
            release_contract_status(protocol_binding_check_ok(&summary, &rows, &evidence)),
            "blocked"
        );
    }

    #[test]
    fn protocol_binding_check_ok_blocks_missing_source_config_digest() {
        let summary = sample_summary();
        let rows = sample_rows();
        let mut evidence = sample_evidence(true, true);
        evidence.source_config_digest = String::new();

        assert!(!protocol_binding_check_ok(&summary, &rows, &evidence));
        assert_eq!(
            release_contract_status(protocol_binding_check_ok(&summary, &rows, &evidence)),
            "blocked"
        );
    }
}
