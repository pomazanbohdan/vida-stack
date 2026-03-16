use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::state_store::{ProtocolBindingState, ProtocolBindingSummary, StateStore};

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
    fn trusted(source: &str) -> bool {
        matches!(
            source,
            "state_store" | super::TASKFLOW_PROTOCOL_BINDING_AUTHORITY
        )
    }
}

pub(crate) async fn protocol_binding_compiled_payload_import_evidence(
    store: &StateStore,
) -> ProtocolBindingCompiledPayloadImportEvidence {
    let mut blockers = Vec::new();

    let activation_snapshot = match super::read_or_sync_launcher_activation_snapshot(store).await {
        Ok(snapshot) => Some(snapshot),
        Err(error) => {
            blockers.push(format!("launcher_activation_snapshot_unavailable:{error}"));
            None
        }
    };
    let effective_bundle_receipt = match store.latest_effective_bundle_receipt_summary().await {
        Ok(receipt) => receipt,
        Err(error) => {
            blockers.push(format!("effective_bundle_receipt_unavailable:{error}"));
            None
        }
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

    if source.is_empty() {
        blockers.push("missing_launcher_activation_snapshot".to_string());
    } else if !ProtocolBindingCompiledPayloadImportEvidence::trusted(&source) {
        blockers.push(format!("untrusted_compiled_payload_source:{source}"));
    }
    if let Some(receipt) = effective_bundle_receipt.as_ref() {
        if receipt.receipt_id.trim().is_empty() {
            blockers.push("missing_effective_bundle_receipt_id".to_string());
        }
        if receipt.root_artifact_id.trim().is_empty() {
            blockers.push("missing_effective_bundle_root_artifact_id".to_string());
        }
        if receipt.artifact_count == 0 {
            blockers.push("empty_effective_bundle_artifact_count".to_string());
        }
    } else {
        blockers.push("missing_effective_bundle_receipt".to_string());
    }

    ProtocolBindingCompiledPayloadImportEvidence {
        imported: activation_snapshot.is_some() && effective_bundle_receipt.is_some(),
        trusted: blockers.is_empty(),
        source,
        source_config_path,
        source_config_digest,
        captured_at,
        effective_bundle_receipt_id: effective_bundle_receipt
            .as_ref()
            .map(|receipt| receipt.receipt_id.clone())
            .unwrap_or_default(),
        effective_bundle_root_artifact_id: effective_bundle_receipt
            .as_ref()
            .map(|receipt| receipt.root_artifact_id.clone())
            .unwrap_or_default(),
        effective_bundle_artifact_count: effective_bundle_receipt
            .as_ref()
            .map(|receipt| receipt.artifact_count)
            .unwrap_or_default(),
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
            blockers.push(format!("missing_source_path:{}", seed.source_path));
        }
        if !protocol_index.contains(&format!("`{}`", seed.protocol_id)) {
            blockers.push(format!(
                "missing_protocol_index_binding:{}",
                seed.protocol_id
            ));
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
    evidence.imported
        && evidence.trusted
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
                    let ok = protocol_binding_check_ok(&summary, &rows, &evidence);
                    super::print_surface_header(
                        super::RenderMode::Plain,
                        "vida taskflow protocol-binding check",
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "ok",
                        if ok { "true" } else { "false" },
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
                    if ok {
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
        [head, subcommand, flag]
            if head == "protocol-binding" && subcommand == "check" && flag == "--json" =>
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
                    let ok = protocol_binding_check_ok(&summary, &rows, &evidence);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow protocol-binding check",
                            "ok": ok,
                            "compiled_payload_import_evidence": evidence,
                            "summary": summary,
                            "bindings": rows,
                        }))
                        .expect("protocol-binding check should render as json")
                    );
                    if ok {
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
