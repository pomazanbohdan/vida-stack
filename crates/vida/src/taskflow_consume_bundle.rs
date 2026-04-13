use std::collections::HashSet;
use std::future::Future;
use std::process::ExitCode;
use std::time::Duration;

const CONSUME_BUNDLE_CHECK_LOCK_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) async fn run_taskflow_consume_bundle(args: &[String]) -> Option<ExitCode> {
    match args {
        [head, subcommand] if head == "consume" && subcommand == "agent-system" => {
            Some(run_consume_agent_system(false).await)
        }
        [head, subcommand, flag]
            if head == "consume" && subcommand == "agent-system" && flag == "--json" =>
        {
            Some(run_consume_agent_system(true).await)
        }
        [head, subcommand] if head == "consume" && subcommand == "bundle" => {
            Some(run_consume_bundle(false).await)
        }
        [head, subcommand, flag]
            if head == "consume" && subcommand == "bundle" && flag == "--json" =>
        {
            Some(run_consume_bundle(true).await)
        }
        [head, subcommand, mode]
            if head == "consume" && subcommand == "bundle" && mode == "check" =>
        {
            Some(run_consume_bundle_check(false).await)
        }
        [head, subcommand, mode, flag]
            if head == "consume"
                && subcommand == "bundle"
                && mode == "check"
                && flag == "--json" =>
        {
            Some(run_consume_bundle_check(true).await)
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "bundle" => {
            eprintln!(
                "Usage: vida taskflow consume bundle [--json]\n       vida taskflow consume bundle check [--json]"
            );
            Some(ExitCode::from(2))
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "agent-system" => {
            eprintln!("Usage: vida taskflow consume agent-system [--json]");
            Some(ExitCode::from(2))
        }
        _ => None,
    }
}

async fn run_consume_agent_system(as_json: bool) -> ExitCode {
    let state_dir = super::taskflow_task_bridge::proxy_state_dir();
    match super::StateStore::open_existing(state_dir).await {
        Ok(store) => match super::build_taskflow_consume_bundle_payload(&store).await {
            Ok(payload) => {
                let snapshot = build_taskflow_agent_system_snapshot(
                    &payload.config_path,
                    &payload.activation_bundle,
                );
                let snapshot_path = match super::write_runtime_consumption_snapshot(
                    store.root(),
                    "agent-system",
                    &serde_json::json!({
                        "surface": "vida taskflow consume agent-system",
                        "snapshot": &snapshot,
                    }),
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                if as_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow consume agent-system",
                            "snapshot": snapshot,
                            "snapshot_path": snapshot_path,
                        }))
                        .expect("consume agent-system should render as json")
                    );
                } else {
                    super::print_surface_header(
                        super::RenderMode::Plain,
                        "vida taskflow consume agent-system",
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "materialization",
                        snapshot["materialization_mode"]
                            .as_str()
                            .unwrap_or("unknown"),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "selection rule",
                        snapshot["agent_model"]["selection_rule"]
                            .as_str()
                            .unwrap_or("unknown"),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "carriers",
                        &snapshot["carriers"]
                            .as_array()
                            .map(|rows| rows.len().to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "runtime roles",
                        &snapshot["runtime_roles"]
                            .as_array()
                            .map(|rows| rows.len().to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "snapshot path",
                        &snapshot_path,
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_consume_bundle(as_json: bool) -> ExitCode {
    let state_dir = super::taskflow_task_bridge::proxy_state_dir();
    match super::StateStore::open_existing(state_dir).await {
        Ok(store) => match super::build_taskflow_consume_bundle_payload(&store).await {
            Ok(payload) => {
                let snapshot_path = match super::write_runtime_consumption_snapshot(
                    store.root(),
                    "bundle",
                    &serde_json::json!({
                        "surface": "vida taskflow consume bundle",
                        "bundle": &payload,
                    }),
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                if as_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "surface": "vida taskflow consume bundle",
                            "bundle": payload,
                            "snapshot_path": snapshot_path,
                        }))
                        .expect("consume bundle should render as json")
                    );
                } else {
                    super::print_surface_header(
                        super::RenderMode::Plain,
                        "vida taskflow consume bundle",
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "artifact",
                        &payload.artifact_name,
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "root artifact",
                        payload.control_core["root_artifact_id"]
                            .as_str()
                            .unwrap_or("unknown"),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "bundle order",
                        &payload.control_core["mandatory_chain_order"]
                            .as_array()
                            .map(|rows| {
                                rows.iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join(" -> ")
                            })
                            .unwrap_or_else(|| "none".to_string()),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "boot compatibility",
                        payload.boot_compatibility["classification"]
                            .as_str()
                            .unwrap_or("unknown"),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "migration state",
                        payload.migration_preflight["migration_state"]
                            .as_str()
                            .unwrap_or("unknown"),
                    );
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "snapshot path",
                        &snapshot_path,
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

async fn run_consume_bundle_check(as_json: bool) -> ExitCode {
    let state_dir = super::taskflow_task_bridge::proxy_state_dir();
    let store = match fail_fast_with_timeout(
        "opening authoritative state store",
        super::StateStore::open_existing(state_dir),
    )
    .await
    {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    match fail_fast_with_timeout(
        "building consume bundle payload",
        super::build_taskflow_consume_bundle_payload(&store),
    )
    .await
    {
        Ok(payload) => {
            let check = super::taskflow_consume_bundle_check(&payload);
            let mut effective_blockers = check.blockers.clone();
            let (registry, docflow_check, readiness, proof, _) =
                super::build_docflow_runtime_evidence();
            let docflow_receipt_evidence =
                crate::runtime_consumption_surface::build_docflow_receipt_evidence(
                    &readiness, &proof,
                );
            let docflow_verdict =
                super::build_docflow_runtime_verdict(&registry, &docflow_check, &readiness, &proof);
            let seam_closure_admission_receipt_check = taskflow_docflow_seam_receipt_backed_check(
                &payload,
                &docflow_verdict,
                docflow_receipt_evidence,
            );
            for blocker in seam_closure_admission_receipt_check["blocker_codes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
            {
                push_unique_string(&mut effective_blockers, blocker);
            }
            let db_first_activation_truth = match super::read_or_sync_launcher_activation_snapshot(
                &store,
            )
            .await
            {
                Ok(snapshot) => {
                    if let Some(error) = db_first_activation_snapshot_validation_error(&snapshot) {
                        if let Some(code) = crate::release_contract_adapters::blocker_code(
                            crate::release1_contracts::BlockerCode::MissingLauncherActivationSnapshot,
                        ) {
                            push_unique_string(&mut effective_blockers, &code);
                        }
                        serde_json::json!({
                            "ok": false,
                            "error": error,
                            "source": snapshot.source,
                            "source_config_path": snapshot.source_config_path,
                            "source_config_digest": snapshot.source_config_digest,
                        })
                    } else {
                        serde_json::json!({
                            "ok": true,
                            "source": snapshot.source,
                            "source_config_path": snapshot.source_config_path,
                            "source_config_digest": snapshot.source_config_digest,
                        })
                    }
                }
                Err(error) => {
                    if let Some(code) = crate::release_contract_adapters::blocker_code(
                        crate::release1_contracts::BlockerCode::MissingLauncherActivationSnapshot,
                    ) {
                        push_unique_string(&mut effective_blockers, &code);
                    }
                    serde_json::json!({
                        "ok": false,
                        "error": error,
                    })
                }
            };
            let blocker_codes = normalize_consume_bundle_blocker_codes(&effective_blockers);
            let next_actions = consume_bundle_check_next_actions(&blocker_codes);
            let artifact_refs = serde_json::json!({
                "root_artifact_id": check.root_artifact_id,
                "bundle_artifact_name": payload.artifact_name,
                "surface": "vida taskflow consume bundle check"
            });
            let operator_status = consume_bundle_operator_contract_status(&blocker_codes);
            if let Some(error) = bundle_check_operator_contracts_consistency_error(
                operator_status,
                &blocker_codes,
                &next_actions,
            ) {
                eprintln!("consume bundle check: failed ({error})");
                return ExitCode::from(1);
            }
            let operator_contracts = serde_json::json!({
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": operator_status,
                "blocker_codes": blocker_codes,
                "next_actions": next_actions,
                "artifact_refs": artifact_refs,
            });
            let snapshot_path = match super::write_runtime_consumption_snapshot(
                store.root(),
                "bundle-check",
                &serde_json::json!({
                    "surface": "vida taskflow consume bundle check",
                    "check": &check,
                    "seam_closure_admission_receipt_check": &seam_closure_admission_receipt_check,
                    "db_first_activation_truth": &db_first_activation_truth,
                    "effective_blockers": &effective_blockers,
                    "blocker_codes": operator_contracts["blocker_codes"].clone(),
                    "next_actions": operator_contracts["next_actions"].clone(),
                    "artifact_refs": operator_contracts["artifact_refs"].clone(),
                    "operator_contracts": &operator_contracts,
                    "bundle": &payload,
                }),
            ) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "surface": "vida taskflow consume bundle check",
                        "check": &check,
                        "seam_closure_admission_receipt_check": &seam_closure_admission_receipt_check,
                        "db_first_activation_truth": &db_first_activation_truth,
                        "effective_blockers": &effective_blockers,
                        "blocker_codes": operator_contracts["blocker_codes"].clone(),
                        "next_actions": operator_contracts["next_actions"].clone(),
                        "artifact_refs": operator_contracts["artifact_refs"].clone(),
                        "operator_contracts": &operator_contracts,
                        "snapshot_path": snapshot_path,
                    }))
                    .expect("consume bundle check should render as json")
                );
            } else {
                super::print_surface_header(
                    super::RenderMode::Plain,
                    "vida taskflow consume bundle check",
                );
                super::print_surface_line(
                    super::RenderMode::Plain,
                    "ok",
                    if blocker_codes.is_empty() {
                        "true"
                    } else {
                        "false"
                    },
                );
                super::print_surface_line(
                    super::RenderMode::Plain,
                    "root artifact",
                    &check.root_artifact_id,
                );
                super::print_surface_line(
                    super::RenderMode::Plain,
                    "artifact count",
                    &check.artifact_count.to_string(),
                );
                if !effective_blockers.is_empty() {
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "blockers",
                        &effective_blockers.join(", "),
                    );
                    let actions = operator_contracts["next_actions"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" | ");
                    if !actions.is_empty() {
                        super::print_surface_line(
                            super::RenderMode::Plain,
                            "next actions",
                            &actions,
                        );
                    }
                }
                super::print_surface_line(
                    super::RenderMode::Plain,
                    "snapshot path",
                    &snapshot_path,
                );
            }
            if blocker_codes.is_empty() {
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

async fn fail_fast_with_timeout<T, E, F>(label: &str, future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match tokio::time::timeout(CONSUME_BUNDLE_CHECK_LOCK_TIMEOUT, future).await {
        Ok(result) => result.map_err(|error| error.to_string()),
        Err(_) => Err(format!(
            "consume bundle check failed fast: {label} timed out while waiting for authoritative datastore lock"
        )),
    }
}

fn consume_bundle_check_next_actions(blockers: &[String]) -> Vec<String> {
    let blockers = normalize_consume_bundle_blocker_codes(blockers);
    if blockers.is_empty() {
        return vec![];
    }
    let mut next_actions = Vec::new();
    if blockers.iter().any(|code| {
        matches!(
            code.as_str(),
            "missing_readiness_verdict"
                | "missing_proof_verdict"
                | "missing_closure_proof"
                | "restore_reconcile_not_green"
        )
    }) {
        next_actions.push(
            "Run `vida docflow readiness-check --profile active-canon` and `vida docflow proofcheck --profile active-canon`, then clear DocFlow closure blockers."
                .to_string(),
        );
    }
    if blockers
        .iter()
        .any(|code| code == "missing_protocol_binding_receipt")
    {
        next_actions.push(
            "Run `vida taskflow protocol-binding sync --json` to materialize the missing receipt."
                .to_string(),
        );
    }
    if blockers
        .iter()
        .any(|code| code == "protocol_binding_not_runtime_ready")
    {
        next_actions.push(
            "Run `vida taskflow protocol-binding check --json` and clear runtime-readiness blockers."
                .to_string(),
        );
    }
    if blockers
        .iter()
        .any(|code| code == "missing_launcher_activation_snapshot")
    {
        next_actions.push(
            "Run `vida boot` (or `vida taskflow protocol-binding sync --json`) to materialize launcher activation snapshot in the authoritative state store."
                .to_string(),
        );
    }
    if next_actions.is_empty() {
        next_actions
            .push("Resolve consume-bundle-check blockers before closure packaging.".to_string());
    }
    next_actions
}

fn normalize_consume_bundle_blocker_codes(blockers: &[String]) -> Vec<String> {
    crate::operator_contracts::normalize_blocker_codes(
        blockers,
        crate::release_contract_adapters::canonical_blocker_codes,
        crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::Unsupported,
        ),
    )
}

fn consume_bundle_operator_contract_status(blockers: &[String]) -> &'static str {
    crate::operator_contracts::operator_contract_status_for_blockers(
        &crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC,
        blockers,
    )
}

fn bundle_check_operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    crate::operator_contracts::operator_contracts_consistency_error(
        &crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC,
        status,
        blocker_codes,
        next_actions,
    )
}

fn db_first_activation_snapshot_validation_error(
    snapshot: &crate::state_store::LauncherActivationSnapshot,
) -> Option<String> {
    if snapshot.source != "state_store" {
        return Some(format!(
            "authoritative launcher activation source must be `state_store`, got `{}`",
            snapshot.source
        ));
    }
    if snapshot.source_config_digest.trim().is_empty() {
        return Some("authoritative launcher activation source_config_digest is empty".to_string());
    }
    None
}

fn taskflow_docflow_seam_receipt_backed_check(
    payload: &super::TaskflowConsumeBundlePayload,
    docflow_verdict: &super::RuntimeConsumptionDocflowVerdict,
    docflow_receipt_evidence: serde_json::Value,
) -> serde_json::Value {
    let receipt_id = payload.protocol_binding_registry["receipt_id"]
        .as_str()
        .unwrap_or_default()
        .trim();
    let binding_status = payload.protocol_binding_registry["binding_status"]
        .as_str()
        .unwrap_or("blocked");
    let protocol_rows = payload.protocol_binding_registry["protocols"]
        .as_array()
        .map(|rows| rows.len())
        .unwrap_or(0);
    let total_receipts = if !receipt_id.is_empty() && binding_status == "bound" && protocol_rows > 0
    {
        1
    } else {
        0
    };
    let has_readiness_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("readiness-check"));
    let has_proof_surface = docflow_verdict
        .proof_surfaces
        .iter()
        .any(|surface| surface.contains("proofcheck"));
    let blocker_codes = taskflow_docflow_closure_input_blocker_codes(
        docflow_verdict,
        has_readiness_surface,
        has_proof_surface,
    );
    let closure_inputs_ready = blocker_codes.is_empty();
    serde_json::json!({
        "status": crate::release_contract_adapters::release_contract_status(closure_inputs_ready),
        "receipt_backed": total_receipts > 0,
        "total_receipts": total_receipts,
        "docflow_status": crate::release_contract_adapters::release_contract_status(closure_inputs_ready),
        "closure_inputs_ready": closure_inputs_ready,
        "blocker_codes": blocker_codes,
        "docflow_blocker_codes": docflow_verdict.blockers.clone(),
        "docflow_proof_surfaces": docflow_verdict.proof_surfaces.clone(),
        "receipt_evidence": docflow_receipt_evidence,
        "has_readiness_surface": has_readiness_surface,
        "has_proof_surface": has_proof_surface,
        "surface": "vida docflow readiness-check --profile active-canon | vida docflow proofcheck --profile active-canon",
        "notes": "TaskFlow->DocFlow seam closure admission requires DocFlow readiness and proof verdict evidence; protocol-binding receipt metadata is informational only.",
    })
}

fn taskflow_docflow_closure_input_blocker_codes(
    docflow_verdict: &super::RuntimeConsumptionDocflowVerdict,
    has_readiness_surface: bool,
    has_proof_surface: bool,
) -> Vec<String> {
    let mut blockers = docflow_verdict.blockers.clone();
    if !has_proof_surface {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::MissingClosureProof,
        ) {
            blockers.push(code);
        }
    }
    if !(has_readiness_surface && has_proof_surface) {
        if let Some(code) = crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::RestoreReconcileNotGreen,
        ) {
            blockers.push(code);
        }
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn push_unique_string(target: &mut Vec<String>, value: &str) {
    if !target.iter().any(|entry| entry == value) {
        target.push(value.to_string());
    }
}

fn build_taskflow_agent_system_snapshot(
    config_path: &str,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let carrier_runtime = crate::carrier_runtime_section(activation_bundle);
    let mut carriers = carrier_runtime["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            serde_json::json!({
                "carrier_id": row["role_id"],
                "tier": row["tier"],
                "rate": row["rate"],
                "default_runtime_role": row["default_runtime_role"],
                "runtime_roles": row["runtime_roles"],
                "task_classes": row["task_classes"],
                "reasoning_band": row["reasoning_band"],
                "model_reasoning_effort": row["model_reasoning_effort"],
            })
        })
        .collect::<Vec<_>>();
    carriers.sort_by(|left, right| {
        left["rate"]
            .as_u64()
            .unwrap_or(u64::MAX)
            .cmp(&right["rate"].as_u64().unwrap_or(u64::MAX))
            .then_with(|| {
                left["carrier_id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["carrier_id"].as_str().unwrap_or_default())
            })
    });

    let mut runtime_roles = carriers
        .iter()
        .flat_map(|row| {
            row["runtime_roles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    runtime_roles.sort();

    let selection_rule = crate::carrier_runtime_metadata::snapshot_selection_rule(carrier_runtime);
    let max_parallel_agents =
        normalize_agent_system_max_parallel_agents(&activation_bundle["agent_system"]);
    let materialization_mode =
        crate::carrier_runtime_metadata::snapshot_materialization_mode(carrier_runtime);
    let carrier_catalog_owner =
        crate::carrier_runtime_metadata::snapshot_carrier_catalog_owner(carrier_runtime);
    let dispatch_alias_owner =
        crate::carrier_runtime_metadata::snapshot_dispatch_alias_owner(carrier_runtime);
    let agent_identity = crate::carrier_runtime_metadata::snapshot_agent_identity(carrier_runtime);
    let runtime_role_identity =
        crate::carrier_runtime_metadata::snapshot_runtime_role_identity(carrier_runtime);

    serde_json::json!({
        "materialization_mode": materialization_mode,
        "source_of_truth": {
            "carrier_catalog_owner": carrier_catalog_owner,
            "dispatch_alias_owner": dispatch_alias_owner,
            "runtime_config_path": config_path,
        },
        "agent_model": {
            "agent_identity": agent_identity,
            "runtime_role_identity": runtime_role_identity,
            "selection_rule": selection_rule,
        },
        "agent_system": {
            "mode": activation_bundle["agent_system"]["mode"],
            "state_owner": activation_bundle["agent_system"]["state_owner"],
            "max_parallel_agents": max_parallel_agents,
            "autonomous_enabled": activation_bundle["autonomous_execution"]["enabled"],
        },
        "carriers": carriers,
        "runtime_roles": runtime_roles,
        "worker_strategy": {
            "selection_policy": carrier_runtime["worker_strategy"]["selection_policy"],
            "agents": carrier_runtime["worker_strategy"]["agents"],
            "store_path": carrier_runtime["worker_strategy"]["store_path"],
            "scorecards_path": carrier_runtime["worker_strategy"]["scorecards_path"],
        },
        "dispatch_aliases": carrier_runtime["dispatch_aliases"],
    })
}

fn normalize_agent_system_max_parallel_agents(agent_system: &serde_json::Value) -> u64 {
    agent_system["max_parallel_agents"]
        .as_u64()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::{
        build_taskflow_agent_system_snapshot, bundle_check_operator_contracts_consistency_error,
        consume_bundle_check_next_actions, consume_bundle_operator_contract_status,
        db_first_activation_snapshot_validation_error, fail_fast_with_timeout,
        normalize_agent_system_max_parallel_agents, normalize_consume_bundle_blocker_codes,
        push_unique_string, taskflow_docflow_seam_receipt_backed_check,
    };
    use crate::{
        release_contract_adapters::release_contract_status,
        runtime_consumption_surface::build_docflow_receipt_evidence, DoctorLauncherSummary,
        RuntimeConsumptionDocflowVerdict, RuntimeConsumptionEvidence, TaskflowConsumeBundlePayload,
    };

    fn minimal_payload_for_operator_contract_status_checks() -> TaskflowConsumeBundlePayload {
        TaskflowConsumeBundlePayload {
            artifact_name: "taskflow_runtime_bundle".to_string(),
            artifact_type: "runtime_bundle".to_string(),
            generated_at: "2026-03-17T00:00:00Z".to_string(),
            vida_root: "/tmp/project".to_string(),
            config_path: "/tmp/project/vida.config.yaml".to_string(),
            activation_source: "state_store".to_string(),
            launcher_runtime_paths: DoctorLauncherSummary {
                vida: "vida".to_string(),
                project_root: "/tmp/project".to_string(),
                taskflow_surface: "vida taskflow".to_string(),
            },
            metadata: serde_json::json!({}),
            control_core: serde_json::json!({}),
            activation_bundle: serde_json::json!({}),
            protocol_binding_registry: serde_json::json!({}),
            cache_delivery_contract: serde_json::json!({}),
            orchestrator_init_view: serde_json::json!({}),
            agent_init_view: serde_json::json!({}),
            continuation_binding: serde_json::json!({}),
            boot_compatibility: serde_json::json!({}),
            migration_preflight: serde_json::json!({}),
            task_store: serde_json::json!({}),
            run_graph: serde_json::json!({}),
        }
    }

    fn minimal_docflow_receipt_evidence() -> serde_json::Value {
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
            artifact_path: None,
            output: String::new(),
        };

        build_docflow_receipt_evidence(&readiness, &proof)
    }

    #[test]
    fn agent_system_snapshot_ignores_legacy_codex_multi_agent_alias() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 2
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "canonical",
                        "tier": "canonical",
                        "rate": 1,
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "default_runtime_role": "worker",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {
                    "role_id": "canonical",
                    "default_runtime_role": "worker"
                }
            },
            "codex_multi_agent": {
                "legacy": true
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(snapshot["carriers"][0]["carrier_id"], "canonical");
        assert_eq!(snapshot["dispatch_aliases"]["role_id"], "canonical");
    }

    #[test]
    fn normalize_agent_system_max_parallel_agents_uses_positive_numeric_value() {
        let agent_system = serde_json::json!({
            "max_parallel_agents": 4
        });

        assert_eq!(normalize_agent_system_max_parallel_agents(&agent_system), 4);
    }

    #[test]
    fn normalize_agent_system_max_parallel_agents_falls_back_when_value_is_not_positive_number() {
        let zero_value = serde_json::json!({
            "max_parallel_agents": 0
        });
        let string_value = serde_json::json!({
            "max_parallel_agents": "4"
        });
        let missing_value = serde_json::json!({});

        assert_eq!(normalize_agent_system_max_parallel_agents(&zero_value), 1);
        assert_eq!(normalize_agent_system_max_parallel_agents(&string_value), 1);
        assert_eq!(
            normalize_agent_system_max_parallel_agents(&missing_value),
            1
        );
    }

    #[test]
    fn db_first_activation_snapshot_validation_accepts_empty_source_config_path() {
        let snapshot = crate::state_store::LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: String::new(),
            source_config_digest: "digest-123".to_string(),
            captured_at: "2026-03-08T00:00:00Z".to_string(),
            compiled_bundle: serde_json::json!({}),
            pack_router_keywords: serde_json::json!({}),
        };

        assert_eq!(
            db_first_activation_snapshot_validation_error(&snapshot),
            None
        );
    }

    #[test]
    fn agent_system_snapshot_exposes_normalized_parallel_agents_value() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 0
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(snapshot["agent_system"]["max_parallel_agents"], 1);
    }

    #[test]
    fn agent_system_snapshot_uses_default_selection_rule_when_missing() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [],
                "worker_strategy": {
                    "selection_policy": {},
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(
            snapshot["agent_model"]["selection_rule"],
            "capability_first_then_score_guard_then_cheapest_tier"
        );
    }

    #[test]
    fn agent_system_snapshot_accepts_carrier_runtime_section() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 2
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "middle",
                        "tier": "middle",
                        "rate": 4,
                        "runtime_roles": ["business_analyst", "coach"],
                        "task_classes": ["specification"],
                        "default_runtime_role": "business_analyst",
                        "reasoning_band": "medium",
                        "model_reasoning_effort": "medium"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {
                        "middle": {
                            "effective_score": 80
                        }
                    },
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": [
                    {
                        "role_id": "specification",
                        "default_runtime_role": "business_analyst"
                    }
                ]
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(
            snapshot["carriers"]
                .as_array()
                .expect("carriers should be array")
                .len(),
            1
        );
        assert_eq!(snapshot["carriers"][0]["carrier_id"], "middle");
        assert_eq!(snapshot["dispatch_aliases"][0]["role_id"], "specification");
    }

    #[test]
    fn agent_system_snapshot_prefers_runtime_owned_metadata_when_present() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 3
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "materialization_mode": "copy_tree_only",
                "source_of_truth": {
                    "carrier_catalog_owner": "vida.config.yaml -> host_environment.systems.qwen.carriers",
                    "dispatch_alias_owner": "vida.config.yaml -> agent_extensions.registries.dispatch_aliases (.vida/project/agent-extensions/dispatch-aliases.yaml)"
                },
                "agent_model": {
                    "agent_identity": "configured_carrier",
                    "runtime_role_identity": "configured_runtime_role"
                },
                "roles": [],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "cheapest_healthy_capable_carrier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(snapshot["materialization_mode"], "copy_tree_only");
        assert_eq!(
            snapshot["source_of_truth"]["carrier_catalog_owner"],
            "vida.config.yaml -> host_environment.systems.qwen.carriers"
        );
        assert_eq!(
            snapshot["source_of_truth"]["dispatch_alias_owner"],
            "vida.config.yaml -> agent_extensions.registries.dispatch_aliases (.vida/project/agent-extensions/dispatch-aliases.yaml)"
        );
        assert_eq!(
            snapshot["agent_model"]["agent_identity"],
            "configured_carrier"
        );
        assert_eq!(
            snapshot["agent_model"]["runtime_role_identity"],
            "configured_runtime_role"
        );
        assert_eq!(
            snapshot["agent_model"]["selection_rule"],
            "cheapest_healthy_capable_carrier"
        );
    }

    #[test]
    fn agent_system_snapshot_runtime_roles_are_sorted_and_deduped() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "a",
                        "rate": 2,
                        "runtime_roles": ["review", "implementation"],
                        "task_classes": [],
                        "default_runtime_role": "implementation",
                        "reasoning_band": "medium",
                        "model_reasoning_effort": "medium"
                    },
                    {
                        "role_id": "b",
                        "rate": 1,
                        "runtime_roles": ["implementation", "verification"],
                        "task_classes": [],
                        "default_runtime_role": "verification",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(
            snapshot["runtime_roles"],
            serde_json::json!(["implementation", "review", "verification"])
        );
    }

    #[test]
    fn agent_system_snapshot_carriers_are_sorted_by_rate_then_id() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "z-role",
                        "rate": 3,
                        "runtime_roles": ["implementation"],
                        "task_classes": [],
                        "default_runtime_role": "implementation",
                        "reasoning_band": "medium",
                        "model_reasoning_effort": "medium"
                    },
                    {
                        "role_id": "a-role",
                        "rate": 1,
                        "runtime_roles": ["verification"],
                        "task_classes": [],
                        "default_runtime_role": "verification",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    },
                    {
                        "role_id": "b-role",
                        "rate": 1,
                        "runtime_roles": ["review"],
                        "task_classes": [],
                        "default_runtime_role": "review",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        let carrier_ids = snapshot["carriers"]
            .as_array()
            .expect("carriers should be an array")
            .iter()
            .map(|row| row["carrier_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(carrier_ids, vec!["a-role", "b-role", "z-role"]);
    }

    #[test]
    fn agent_system_snapshot_carriers_with_missing_rate_sort_after_numeric_rates() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "no-rate",
                        "runtime_roles": ["implementation"],
                        "task_classes": [],
                        "default_runtime_role": "implementation",
                        "reasoning_band": "medium",
                        "model_reasoning_effort": "medium"
                    },
                    {
                        "role_id": "fast",
                        "rate": 1,
                        "runtime_roles": ["verification"],
                        "task_classes": [],
                        "default_runtime_role": "verification",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        let carrier_ids = snapshot["carriers"]
            .as_array()
            .expect("carriers should be an array")
            .iter()
            .map(|row| row["carrier_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(carrier_ids, vec!["fast", "no-rate"]);
    }

    #[test]
    fn agent_system_snapshot_runtime_roles_ignore_non_string_entries() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": "mixed",
                        "rate": 1,
                        "runtime_roles": ["implementation", 42, null, "review"],
                        "task_classes": [],
                        "default_runtime_role": "implementation",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(
            snapshot["runtime_roles"],
            serde_json::json!(["implementation", "review"])
        );
    }

    #[test]
    fn agent_system_snapshot_defaults_selection_rule_when_selection_policy_is_missing() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [],
                "worker_strategy": {
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(
            snapshot["agent_model"]["selection_rule"],
            "capability_first_then_score_guard_then_cheapest_tier"
        );
    }

    #[test]
    fn agent_system_snapshot_carrier_sort_handles_missing_or_non_string_carrier_id() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 4
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [
                    {
                        "role_id": 123,
                        "rate": 1,
                        "runtime_roles": ["implementation"],
                        "task_classes": [],
                        "default_runtime_role": "implementation",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    },
                    {
                        "role_id": "named",
                        "rate": 1,
                        "runtime_roles": ["verification"],
                        "task_classes": [],
                        "default_runtime_role": "verification",
                        "reasoning_band": "low",
                        "model_reasoning_effort": "low"
                    }
                ],
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": {}
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        let carrier_ids = snapshot["carriers"]
            .as_array()
            .expect("carriers should be an array")
            .iter()
            .map(|row| row["carrier_id"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            carrier_ids,
            vec![serde_json::json!(123), serde_json::json!("named")]
        );
    }

    #[test]
    fn normalize_agent_system_max_parallel_agents_falls_back_for_negative_value() {
        let negative_value = serde_json::json!({
            "max_parallel_agents": -3
        });

        assert_eq!(
            normalize_agent_system_max_parallel_agents(&negative_value),
            1
        );
    }

    #[test]
    fn normalize_agent_system_max_parallel_agents_falls_back_for_null_value() {
        let null_value = serde_json::json!({
            "max_parallel_agents": null
        });

        assert_eq!(normalize_agent_system_max_parallel_agents(&null_value), 1);
    }

    #[test]
    fn consume_bundle_blockers_normalize_to_canonical_release1_codes() {
        let normalized = normalize_consume_bundle_blocker_codes(&[
            " missing_protocol_binding_receipt ".to_string(),
            "protocol_binding_not_runtime_ready".to_string(),
            "missing_protocol_binding_receipt".to_string(),
        ]);
        assert_eq!(
            normalized,
            vec![
                "missing_protocol_binding_receipt".to_string(),
                "protocol_binding_not_runtime_ready".to_string()
            ]
        );
    }

    #[test]
    fn consume_bundle_blockers_fail_closed_when_only_unknown_codes_are_present() {
        let normalized = normalize_consume_bundle_blocker_codes(&[
            "unknown_blocker".to_string(),
            "another_unknown".to_string(),
        ]);
        assert_eq!(normalized, vec!["unsupported_blocker_code".to_string()]);
    }

    #[test]
    fn consume_bundle_next_actions_fail_closed_for_whitespace_only_blocker_strings() {
        let normalized = normalize_consume_bundle_blocker_codes(&["   ".to_string()]);
        assert_eq!(normalized, vec!["unsupported_blocker_code".to_string()]);

        let actions = consume_bundle_check_next_actions(&["   ".to_string()]);
        assert_eq!(
            actions,
            vec!["Resolve consume-bundle-check blockers before closure packaging.".to_string()]
        );
    }

    #[test]
    fn consume_bundle_next_actions_fall_back_for_unsupported_blocker_code() {
        let actions = consume_bundle_check_next_actions(&["unsupported_blocker_code".to_string()]);
        assert_eq!(
            actions,
            vec!["Resolve consume-bundle-check blockers before closure packaging.".to_string()]
        );
    }

    #[test]
    fn consume_bundle_operator_contract_status_uses_canonical_release1_vocabulary() {
        assert_eq!(
            consume_bundle_operator_contract_status(&[]),
            release_contract_status(true)
        );
        assert_eq!(
            consume_bundle_operator_contract_status(&[
                "missing_protocol_binding_receipt".to_string()
            ]),
            release_contract_status(false)
        );
    }

    #[test]
    fn bundle_check_operator_contracts_consistency_rejects_noncanonical_status_drift() {
        let blocker_codes = vec!["missing_protocol_binding_receipt".to_string()];
        let next_actions = vec!["Run `vida taskflow protocol-binding sync --json`".to_string()];
        assert_eq!(
            bundle_check_operator_contracts_consistency_error(
                "pass!",
                &blocker_codes,
                &next_actions,
            ),
            Some(
                "operator contract inconsistency: status must be canonical release-1 pass/blocked"
                    .to_string()
            )
        );
    }

    #[test]
    fn bundle_check_operator_contracts_consistency_rejects_noncanonical_mirrored_string_entries() {
        let blocker_codes = vec!["missing_protocol_binding_receipt".to_string()];
        let next_actions = vec![" Run `vida taskflow protocol-binding sync --json` ".to_string()];
        assert_eq!(
            bundle_check_operator_contracts_consistency_error(
                "blocked",
                &blocker_codes,
                &next_actions,
            ),
            Some(
                "operator contract inconsistency: shared string arrays must contain only canonical nonempty entries"
                    .to_string()
            )
        );
    }

    #[test]
    fn taskflow_docflow_seam_receipt_backed_check_uses_canonical_blocked_status() {
        let payload = minimal_payload_for_operator_contract_status_checks();
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "block".to_string(),
            ready: false,
            blockers: vec!["missing_readiness_verdict".to_string()],
            proof_surfaces: vec!["vida docflow check --profile active-canon".to_string()],
        };
        let seam = taskflow_docflow_seam_receipt_backed_check(
            &payload,
            &docflow_verdict,
            minimal_docflow_receipt_evidence(),
        );

        assert_eq!(seam["status"], "blocked");
        assert_eq!(seam["receipt_backed"], false);
        assert_eq!(seam["closure_inputs_ready"], false);
        assert_eq!(seam["docflow_status"], "blocked");
        assert_eq!(
            seam["blocker_codes"],
            serde_json::json!([
                "missing_closure_proof",
                "missing_readiness_verdict",
                "restore_reconcile_not_green"
            ])
        );
        assert_eq!(
            seam["docflow_blocker_codes"],
            serde_json::json!(["missing_readiness_verdict"])
        );
        assert_eq!(seam["receipt_evidence"]["receipt_backed"], true);
        assert_eq!(
            seam["receipt_evidence"]["readiness_receipt_path"],
            "vida/config/docflow-readiness.current.jsonl"
        );
        assert!(seam["receipt_evidence"]["proof_receipt_path"].is_null());
        assert_eq!(seam["has_readiness_surface"], false);
        assert_eq!(seam["has_proof_surface"], false);
    }

    #[test]
    fn taskflow_docflow_seam_receipt_backed_check_does_not_require_receipt_when_docflow_inputs_are_ready(
    ) {
        let payload = minimal_payload_for_operator_contract_status_checks();
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "pass".to_string(),
            ready: true,
            blockers: vec![],
            proof_surfaces: vec![
                "vida docflow readiness-check --profile active-canon".to_string(),
                "vida docflow proofcheck --profile active-canon".to_string(),
            ],
        };

        let seam = taskflow_docflow_seam_receipt_backed_check(
            &payload,
            &docflow_verdict,
            minimal_docflow_receipt_evidence(),
        );

        assert_eq!(seam["status"], "pass");
        assert_eq!(seam["receipt_backed"], false);
        assert_eq!(seam["closure_inputs_ready"], true);
        assert_eq!(seam["docflow_status"], "pass");
        assert_eq!(seam["blocker_codes"], serde_json::json!([]));
        assert_eq!(seam["receipt_evidence"]["receipt_backed"], true);
        assert_eq!(seam["has_readiness_surface"], true);
        assert_eq!(seam["has_proof_surface"], true);
    }

    #[test]
    fn taskflow_docflow_seam_receipt_backed_check_adds_closure_surface_blockers() {
        let payload = minimal_payload_for_operator_contract_status_checks();
        let docflow_verdict = RuntimeConsumptionDocflowVerdict {
            status: "pass".to_string(),
            ready: true,
            blockers: vec![],
            proof_surfaces: vec!["vida docflow readiness-check --profile active-canon".to_string()],
        };

        let seam = taskflow_docflow_seam_receipt_backed_check(
            &payload,
            &docflow_verdict,
            minimal_docflow_receipt_evidence(),
        );

        assert_eq!(seam["status"], "blocked");
        assert_eq!(seam["closure_inputs_ready"], false);
        assert_eq!(
            seam["blocker_codes"],
            serde_json::json!(["missing_closure_proof", "restore_reconcile_not_green"])
        );
        assert_eq!(seam["receipt_evidence"]["receipt_backed"], true);
        assert_eq!(seam["has_readiness_surface"], true);
        assert_eq!(seam["has_proof_surface"], false);
    }

    #[test]
    fn consume_bundle_next_actions_include_docflow_guidance_for_closure_seam_blockers() {
        let actions = consume_bundle_check_next_actions(&[
            "missing_proof_verdict".to_string(),
            "restore_reconcile_not_green".to_string(),
        ]);

        assert_eq!(
            actions,
            vec![
                "Run `vida docflow readiness-check --profile active-canon` and `vida docflow proofcheck --profile active-canon`, then clear DocFlow closure blockers.".to_string()
            ]
        );
    }

    #[test]
    fn push_unique_string_deduplicates_effective_blockers() {
        let mut blockers = vec!["missing_readiness_verdict".to_string()];

        push_unique_string(&mut blockers, "missing_readiness_verdict");
        push_unique_string(&mut blockers, "missing_proof_verdict");

        assert_eq!(
            blockers,
            vec![
                "missing_readiness_verdict".to_string(),
                "missing_proof_verdict".to_string()
            ]
        );
    }

    #[test]
    fn fail_fast_with_timeout_returns_deterministic_timeout_error() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("test runtime should build");
        let result = runtime.block_on(async {
            fail_fast_with_timeout("opening authoritative state store", async {
                std::future::pending::<Result<(), String>>().await
            })
            .await
        });

        let error = result.expect_err("pending future should time out");
        assert!(error.contains("consume bundle check failed fast"));
        assert!(error.contains("opening authoritative state store"));
        assert!(error.contains("authoritative datastore lock"));
    }
}
