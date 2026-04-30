use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::process::ExitCode;
use std::time::Duration;

const CONSUME_BUNDLE_CHECK_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

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
        .any(|code| code.starts_with("missing_retrieval_trust_evidence_field:"))
    {
        next_actions.push(
            "Run `vida taskflow protocol-binding sync --json`, then `vida taskflow consume bundle check --json` to materialize retrieval-trust source/citation/freshness/ACL evidence."
                .to_string(),
        );
    }
    if blockers
        .iter()
        .any(|code| code.starts_with("invalid_cache_key_input:"))
    {
        next_actions.push(
            "Refresh the startup bundle projection with `vida orchestrator-init --json`, then rerun `vida taskflow consume bundle check --json`."
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
    let protocol_binding_receipt_id = payload.protocol_binding_registry["receipt_id"]
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
    let receipt_backed = docflow_receipt_evidence["receipt_backed"]
        .as_bool()
        .unwrap_or(false);
    let total_receipts = docflow_receipt_evidence["total_receipts"]
        .as_u64()
        .unwrap_or_else(|| u64::from(receipt_backed));
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
        "receipt_backed": receipt_backed,
        "total_receipts": total_receipts,
        "docflow_status": crate::release_contract_adapters::release_contract_status(closure_inputs_ready),
        "closure_inputs_ready": closure_inputs_ready,
        "blocker_codes": blocker_codes,
        "docflow_blocker_codes": docflow_verdict.blockers.clone(),
        "docflow_proof_surfaces": docflow_verdict.proof_surfaces.clone(),
        "receipt_evidence": docflow_receipt_evidence,
        "protocol_binding_receipt_id": protocol_binding_receipt_id,
        "protocol_binding_binding_status": binding_status,
        "protocol_binding_protocol_rows": protocol_rows,
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
                "normalized_cost_units": row["normalized_cost_units"],
                "default_runtime_role": row["default_runtime_role"],
                "runtime_roles": row["runtime_roles"],
                "task_classes": row["task_classes"],
                "reasoning_band": row["reasoning_band"],
                "model": row["model"],
                "model_provider": row["model_provider"],
                "model_reasoning_effort": row["model_reasoning_effort"],
                "plan_mode_reasoning_effort": row["plan_mode_reasoning_effort"],
                "sandbox_mode": row["sandbox_mode"],
                "default_model_profile": row["default_model_profile"],
                "model_profiles": row["model_profiles"],
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

    let selection_rule = carrier_runtime["model_selection"]["selection_rule"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            crate::carrier_runtime_metadata::snapshot_selection_rule(carrier_runtime)
        });
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
    let dev_team_readiness = build_dev_team_readiness(config_path, activation_bundle);

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
            "model_selection": carrier_runtime["model_selection"],
            "model_selection_enabled": carrier_runtime["model_selection"]["enabled"]
                .as_bool()
                .unwrap_or(false),
        },
        "agent_system": {
            "mode": activation_bundle["agent_system"]["mode"],
            "state_owner": activation_bundle["agent_system"]["state_owner"],
            "max_parallel_agents": max_parallel_agents,
            "autonomous_enabled": activation_bundle["autonomous_execution"]["enabled"],
        },
        "carriers": carriers,
        "runtime_roles": runtime_roles,
        "dev_team_readiness": dev_team_readiness,
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

pub(crate) fn build_dev_team_readiness(
    config_path: &str,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let source_paths = dev_team_source_paths(config_path, None);
    let config_text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) => {
            return serde_json::json!({
                "status": "config_unreadable",
                "configured": false,
                "enabled": serde_json::Value::Null,
                "roles": [],
                "sequence": [],
                "flows": [],
                "blockers": [format!("dev_team_config_unreadable: {error}")],
                "source_paths": source_paths,
            });
        }
    };
    let overlay: serde_yaml::Value = match serde_yaml::from_str(&config_text) {
        Ok(value) => value,
        Err(error) => {
            return serde_json::json!({
                "status": "config_invalid",
                "configured": false,
                "enabled": serde_json::Value::Null,
                "roles": [],
                "sequence": [],
                "flows": [],
                "blockers": [format!("dev_team_config_invalid: {error}")],
                "source_paths": source_paths,
            });
        }
    };
    let Some(dev_team) = crate::yaml_lookup(&overlay, &["dev_team"]) else {
        return serde_json::json!({
            "status": "missing_config",
            "configured": false,
            "enabled": serde_json::Value::Null,
            "roles": [],
            "sequence": [],
            "flows": [],
            "blockers": ["missing_dev_team_config"],
            "source_paths": source_paths,
        });
    };

    let enabled = crate::yaml_bool(crate::yaml_lookup(dev_team, &["enabled"]), true);
    let contract_doc = crate::yaml_string(crate::yaml_lookup(dev_team, &["contract_doc"]));
    let source_paths = dev_team_source_paths(config_path, contract_doc.as_deref());
    let carrier_catalog = carrier_catalog_by_id(activation_bundle);
    let pricing_catalog = &activation_bundle["agent_system"]["pricing"];

    let mut blockers = Vec::new();
    let roles = dev_team_roles(dev_team, &carrier_catalog, pricing_catalog, &mut blockers);
    let flows = dev_team_flows(dev_team, &roles, &mut blockers);
    let sequence = default_dev_team_sequence(&flows);

    if !enabled {
        blockers.push("dev_team_disabled".to_string());
    }
    if roles.is_empty() {
        blockers.push("missing_dev_team_roles".to_string());
    }
    if sequence.is_empty() {
        blockers.push("missing_dev_team_sequence".to_string());
    }

    let status = if !enabled {
        "disabled"
    } else if blockers.is_empty() {
        "ready"
    } else {
        "blocked"
    };

    serde_json::json!({
        "status": status,
        "configured": true,
        "enabled": enabled,
        "roles": roles,
        "sequence": sequence,
        "flows": flows,
        "blockers": blockers,
        "source_paths": source_paths,
    })
}

fn dev_team_source_paths(config_path: &str, contract_doc: Option<&str>) -> Vec<String> {
    let mut source_paths = vec![config_path.to_string()];
    if let Some(contract_doc) = contract_doc
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        source_paths.push(contract_doc.to_string());
    }
    source_paths
}

fn carrier_catalog_by_id(
    activation_bundle: &serde_json::Value,
) -> BTreeMap<String, serde_json::Value> {
    crate::carrier_runtime_section(activation_bundle)["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            row["role_id"]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(|carrier_id| (carrier_id.to_string(), row.clone()))
        })
        .collect()
}

fn dev_team_roles(
    dev_team: &serde_yaml::Value,
    carrier_catalog: &BTreeMap<String, serde_json::Value>,
    pricing_catalog: &serde_json::Value,
    blockers: &mut Vec<String>,
) -> Vec<serde_json::Value> {
    let mut roles = Vec::new();
    let Some(role_map) =
        crate::yaml_lookup(dev_team, &["roles"]).and_then(serde_yaml::Value::as_mapping)
    else {
        return roles;
    };

    for (role_id, role_entry) in role_map
        .iter()
        .filter_map(|(key, value)| key.as_str().map(|id| (id, value)))
    {
        let runtime_role = crate::yaml_string(crate::yaml_lookup(role_entry, &["runtime_role"]));
        if runtime_role.as_deref().unwrap_or_default().is_empty() {
            blockers.push(format!("missing_runtime_role:{role_id}"));
        }
        let task_classes =
            crate::yaml_string_list(crate::yaml_lookup(role_entry, &["task_classes"]));
        if task_classes.is_empty() {
            blockers.push(format!("missing_task_classes:{role_id}"));
        }
        let default_carrier =
            crate::yaml_string(crate::yaml_lookup(role_entry, &["default_carrier"]));
        if default_carrier.as_deref().unwrap_or_default().is_empty() {
            blockers.push(format!("missing_default_carrier:{role_id}"));
        }
        let default_model = crate::yaml_string(crate::yaml_lookup(role_entry, &["default_model"]));
        if default_model.as_deref().unwrap_or_default().is_empty() {
            blockers.push(format!("missing_default_model:{role_id}"));
        }
        let handoff_next_role =
            crate::yaml_string(crate::yaml_lookup(role_entry, &["handoff", "next_role"]));
        let required_outputs = crate::yaml_string_list(crate::yaml_lookup(
            role_entry,
            &["handoff", "required_outputs"],
        ));
        if handoff_next_role.as_deref().unwrap_or_default().is_empty() {
            blockers.push(format!("missing_handoff_next_role:{role_id}"));
        }
        if required_outputs.is_empty() {
            blockers.push(format!("missing_handoff_outputs:{role_id}"));
        }
        let carrier = default_carrier
            .as_deref()
            .and_then(|carrier_id| carrier_catalog.get(carrier_id));
        if default_carrier.is_some() && carrier.is_none() {
            blockers.push(format!("unknown_default_carrier:{role_id}"));
        }
        let selected_model = dev_team_selected_model_projection(
            role_id,
            default_model.as_deref(),
            carrier,
            pricing_catalog,
            blockers,
        );

        roles.push(serde_json::json!({
            "role_id": role_id,
            "runtime_role": runtime_role,
            "task_classes": task_classes,
            "default_carrier": default_carrier,
            "default_model": default_model,
            "default_model_reasoning_effort": crate::yaml_string(
                crate::yaml_lookup(role_entry, &["default_model_reasoning_effort"])
            ),
            "cost_policy": {
                "budget_units": crate::yaml_lookup(role_entry, &["cost_policy", "budget_units"])
                    .and_then(serde_yaml::Value::as_u64),
                "fallback_carrier": crate::yaml_string(
                    crate::yaml_lookup(role_entry, &["cost_policy", "fallback_carrier"])
                ),
                "selection_rule": crate::yaml_string(
                    crate::yaml_lookup(role_entry, &["cost_policy", "selection_rule"])
                ),
            },
            "handoff": {
                "next_role": handoff_next_role,
                "required_outputs": required_outputs,
                "fail_closed_on_missing_handoff": crate::yaml_bool(
                    crate::yaml_lookup(role_entry, &["handoff", "fail_closed_on_missing_handoff"]),
                    false,
                ),
            },
            "selected_model": selected_model,
            "selected_carrier": carrier.cloned().map(|carrier| {
                serde_json::json!({
                    "carrier_id": carrier["role_id"],
                    "model": carrier["model"],
                    "model_provider": carrier["model_provider"],
                    "reasoning_effort": carrier["model_reasoning_effort"],
                    "normalized_cost_units": carrier["normalized_cost_units"],
                    "rate": carrier["rate"],
                    "runtime_roles": carrier["runtime_roles"],
                    "task_classes": carrier["task_classes"],
                })
            }).unwrap_or(serde_json::Value::Null),
        }));
    }

    roles.sort_by(|left, right| {
        left["role_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["role_id"].as_str().unwrap_or_default())
    });
    roles
}

fn dev_team_selected_model_projection(
    role_id: &str,
    default_model: Option<&str>,
    carrier: Option<&serde_json::Value>,
    pricing_catalog: &serde_json::Value,
    blockers: &mut Vec<String>,
) -> serde_json::Value {
    let Some(carrier) = carrier else {
        return serde_json::Value::Null;
    };
    let carrier_id = carrier
        .get("role_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let default_model = default_model
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut model_profile_id = serde_json::Value::Null;
    let mut model_profile_id_source_path = serde_json::Value::Null;
    let model_ref: serde_json::Value;
    let mut model_ref_source_path = serde_json::Value::Null;
    let model_provider: serde_json::Value;
    let model_reasoning_effort: serde_json::Value;
    let mut selected_rate = serde_json::Value::Null;
    let mut selected_rate_source_path = serde_json::Value::Null;

    let mut selected_profile = None::<(String, &serde_json::Value)>;
    if let Some(profiles) = carrier["model_profiles"].as_object() {
        if let Some(default_model) = default_model {
            if let Some(profile) = profiles.get(default_model) {
                selected_profile = Some((default_model.to_string(), profile));
            } else {
                for (profile_id, profile_row) in profiles {
                    if profile_row
                        .get("model_ref")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|value| value == default_model)
                    {
                        selected_profile = Some((profile_id.to_string(), profile_row));
                        break;
                    }
                }
            }
        }
        if selected_profile.is_none() {
            if let Some(default_model_profile) = carrier["default_model_profile"]
                .as_str()
                .filter(|value| !value.is_empty())
            {
                if let Some(profile_row) = profiles.get(default_model_profile) {
                    selected_profile = Some((default_model_profile.to_string(), profile_row));
                }
            }
        }
        if selected_profile.is_none() {
            if profiles.len() == 1 {
                if let Some((profile_id, profile_row)) = profiles.iter().next() {
                    selected_profile = Some((profile_id.to_string(), profile_row));
                }
            } else if default_model.is_some() {
                blockers.push(format!("selected_model_profile_missing:{role_id}"));
            }
        }

        if let Some((profile_id, profile_row)) = selected_profile.as_ref() {
            model_profile_id = serde_json::Value::String(profile_id.clone());
            model_profile_id_source_path = serde_json::Value::String(format!(
                "carrier_runtime.roles[{carrier_id}].model_profiles.{profile_id}.profile_id"
            ));
            model_ref = profile_row
                .get("model_ref")
                .cloned()
                .or_else(|| carrier.get("model").cloned())
                .unwrap_or(serde_json::Value::Null);
            if model_ref.is_null() {
                blockers.push(format!("selected_model_ref_missing:{role_id}"));
            } else {
                if profile_row.get("model_ref").is_some() {
                    model_ref_source_path = serde_json::Value::String(format!(
                        "carrier_runtime.roles[{carrier_id}].model_profiles.{profile_id}.model_ref"
                    ));
                } else {
                    model_ref_source_path = serde_json::Value::String(format!(
                        "carrier_runtime.roles[{carrier_id}].model"
                    ));
                }
            }
            model_provider = profile_row
                .get("provider")
                .cloned()
                .or_else(|| carrier.get("model_provider").cloned())
                .unwrap_or(serde_json::Value::Null);
            model_reasoning_effort = profile_row
                .get("reasoning_effort")
                .cloned()
                .or_else(|| carrier.get("model_reasoning_effort").cloned())
                .unwrap_or(serde_json::Value::Null);
            if let Some(profile_rate) = profile_row.get("normalized_cost_units") {
                selected_rate = profile_rate.clone();
                selected_rate_source_path = serde_json::Value::String(format!(
                    "carrier_runtime.roles[{carrier_id}].model_profiles.{profile_id}.normalized_cost_units"
                ));
            } else if let Some(profile_rate) = profile_row.get("rate") {
                selected_rate = profile_rate.clone();
                selected_rate_source_path = serde_json::Value::String(format!(
                    "carrier_runtime.roles[{carrier_id}].model_profiles.{profile_id}.rate"
                ));
            } else if let Some(carrier_rate) = carrier.get("normalized_cost_units") {
                selected_rate = carrier_rate.clone();
                selected_rate_source_path = serde_json::Value::String(format!(
                    "carrier_runtime.roles[{carrier_id}].normalized_cost_units"
                ));
            } else if let Some(carrier_rate) = carrier.get("rate") {
                selected_rate = carrier_rate.clone();
                selected_rate_source_path =
                    serde_json::Value::String(format!("carrier_runtime.roles[{carrier_id}].rate"));
            }
            if selected_rate.is_null() {
                blockers.push(format!("selected_rate_missing:{role_id}"));
            }
        } else {
            model_ref = carrier
                .get("model")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if model_ref.is_null() {
                blockers.push(format!("selected_model_ref_missing:{role_id}"));
            } else {
                model_ref_source_path =
                    serde_json::Value::String(format!("carrier_runtime.roles[{carrier_id}].model"));
            }
            model_provider = carrier
                .get("model_provider")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            model_reasoning_effort = carrier
                .get("model_reasoning_effort")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            selected_rate = carrier
                .get("normalized_cost_units")
                .or_else(|| carrier.get("rate"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if selected_rate.is_null() {
                blockers.push(format!("selected_rate_missing:{role_id}"));
            } else {
                if carrier.get("normalized_cost_units").is_some() {
                    selected_rate_source_path = serde_json::Value::String(format!(
                        "carrier_runtime.roles[{carrier_id}].normalized_cost_units"
                    ));
                } else {
                    selected_rate_source_path = serde_json::Value::String(format!(
                        "carrier_runtime.roles[{carrier_id}].rate"
                    ));
                }
            }
        }
    } else {
        model_ref = carrier
            .get("model")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if model_ref.is_null() {
            blockers.push(format!("selected_model_ref_missing:{role_id}"));
        } else {
            model_ref_source_path =
                serde_json::Value::String(format!("carrier_runtime.roles[{carrier_id}].model"));
        }
        model_provider = carrier
            .get("model_provider")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        model_reasoning_effort = carrier
            .get("model_reasoning_effort")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        selected_rate = carrier
            .get("normalized_cost_units")
            .or_else(|| carrier.get("rate"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if selected_rate.is_null() {
            blockers.push(format!("selected_rate_missing:{role_id}"));
        } else {
            if carrier.get("normalized_cost_units").is_some() {
                selected_rate_source_path = serde_json::Value::String(format!(
                    "carrier_runtime.roles[{carrier_id}].normalized_cost_units"
                ));
            } else {
                selected_rate_source_path =
                    serde_json::Value::String(format!("carrier_runtime.roles[{carrier_id}].rate"));
            }
        }
    }

    let pricing = pricing_metadata_for_profile(
        selected_profile
            .as_ref()
            .map(|(profile_id, _)| carrier["model_profiles"][profile_id].as_object())
            .and_then(|value| value),
        model_provider.as_str().and_then(|provider| {
            pricing_catalog
                .get("providers")
                .and_then(|providers| providers.get(provider))
        }),
        pricing_catalog,
    )
    .unwrap_or(serde_json::Value::Null);
    let pricing_freshness = pricing
        .get("freshness")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let pricing_freshness_status = if pricing_freshness.is_null() {
        serde_json::json!("missing")
    } else if pricing_freshness
        .get("required")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && pricing_freshness.get("max_age_days").is_none()
    {
        blockers.push(format!("model_price_freshness_policy_incomplete:{role_id}"));
        serde_json::json!("missing")
    } else if pricing_freshness
        .get("required")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && pricing_freshness
            .get("max_age_days")
            .and_then(serde_json::Value::as_u64)
            == Some(0)
    {
        blockers.push(format!("model_price_freshness_stale:{role_id}"));
        serde_json::json!("stale")
    } else {
        serde_json::json!("ready")
    };

    serde_json::json!({
        "carrier_id": carrier.get("role_id").cloned().unwrap_or(serde_json::Value::Null),
        "model_profile_id": model_profile_id,
        "model_profile_id_source_path": model_profile_id_source_path,
        "model_ref": model_ref,
        "model_ref_source_path": model_ref_source_path,
        "model_provider": model_provider,
        "model_reasoning_effort": model_reasoning_effort,
        "selected_rate": selected_rate,
        "selected_rate_source_path": selected_rate_source_path,
        "pricing": pricing,
        "pricing_freshness": pricing_freshness,
        "pricing_freshness_status": pricing_freshness_status,
    })
}

fn pricing_metadata_for_profile(
    profile_row: Option<&serde_json::Map<String, serde_json::Value>>,
    provider_pricing: Option<&serde_json::Value>,
    pricing_catalog: &serde_json::Value,
) -> Option<serde_json::Value> {
    if let Some(profile_row) = profile_row {
        if let Some(pricing) = profile_row.get("pricing") {
            return Some(pricing.clone());
        }
    }
    if let Some(pricing) = provider_pricing {
        return Some(pricing.clone());
    }
    pricing_catalog
        .get("model_profile_defaults")
        .and_then(|defaults| defaults.get("pricing"))
        .cloned()
}

fn dev_team_flows(
    dev_team: &serde_yaml::Value,
    roles: &[serde_json::Value],
    blockers: &mut Vec<String>,
) -> Vec<serde_json::Value> {
    let mut flows = Vec::new();
    let known_roles = roles
        .iter()
        .filter_map(|row| row["role_id"].as_str())
        .collect::<HashSet<_>>();
    let Some(flow_map) =
        crate::yaml_lookup(dev_team, &["flows"]).and_then(serde_yaml::Value::as_mapping)
    else {
        return flows;
    };

    for (flow_id, flow_entry) in flow_map
        .iter()
        .filter_map(|(key, value)| key.as_str().map(|id| (id, value)))
    {
        let steps = crate::yaml_string_list(crate::yaml_lookup(flow_entry, &["steps"]));
        if steps.is_empty() {
            blockers.push(format!("missing_flow_steps:{flow_id}"));
        }
        for step in &steps {
            if !known_roles.contains(step.as_str()) {
                blockers.push(format!("unknown_flow_step:{flow_id}:{step}"));
            }
        }
        flows.push(serde_json::json!({
            "flow_id": flow_id,
            "enabled": crate::yaml_bool(crate::yaml_lookup(flow_entry, &["enabled"]), true),
            "description": crate::yaml_string(crate::yaml_lookup(flow_entry, &["description"])),
            "sequential": crate::yaml_bool(crate::yaml_lookup(flow_entry, &["sequential"]), false),
            "allow_parallel_handoffs": crate::yaml_bool(
                crate::yaml_lookup(flow_entry, &["allow_parallel_handoffs"]),
                false,
            ),
            "steps": steps,
        }));
    }

    flows.sort_by(|left, right| {
        left["flow_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["flow_id"].as_str().unwrap_or_default())
    });
    flows
}

fn default_dev_team_sequence(flows: &[serde_json::Value]) -> Vec<String> {
    flows
        .iter()
        .find(|flow| {
            flow["enabled"].as_bool().unwrap_or(false)
                && flow["flow_id"].as_str() == Some("default_delivery")
        })
        .or_else(|| {
            flows
                .iter()
                .find(|flow| flow["enabled"].as_bool().unwrap_or(false))
        })
        .and_then(|flow| flow["steps"].as_array())
        .map(|steps| {
            steps
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        build_dev_team_readiness, build_taskflow_agent_system_snapshot,
        bundle_check_operator_contracts_consistency_error, consume_bundle_check_next_actions,
        consume_bundle_operator_contract_status, db_first_activation_snapshot_validation_error,
        fail_fast_with_timeout, normalize_agent_system_max_parallel_agents,
        normalize_consume_bundle_blocker_codes, push_unique_string,
        taskflow_docflow_seam_receipt_backed_check,
    };
    use crate::{
        release_contract_adapters::release_contract_status,
        runtime_consumption_surface::build_docflow_receipt_evidence, temp_state::TempStateHarness,
        DoctorLauncherSummary, RuntimeConsumptionDocflowVerdict, RuntimeConsumptionEvidence,
        TaskflowConsumeBundlePayload,
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
                active_executable_path: "/tmp/project/bin/vida".to_string(),
                active_executable_fingerprint: "fingerprint".to_string(),
                installed_binaries: Vec::new(),
                divergent_installed_binaries: false,
                status: "pass".to_string(),
                next_actions: Vec::new(),
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
            artifact_path: Some(
                crate::runtime_consumption_surface::DOCFLOW_PROOF_CURRENT_PATH.to_string(),
            ),
            output: String::new(),
        };

        build_docflow_receipt_evidence(&readiness, &proof)
    }

    #[test]
    fn agent_system_snapshot_prefers_carrier_runtime_fields_without_legacy_aliases() {
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
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);
        assert_eq!(snapshot["carriers"][0]["carrier_id"], "canonical");
        assert_eq!(snapshot["dispatch_aliases"]["role_id"], "canonical");
        assert_eq!(
            snapshot["dev_team_readiness"]["status"],
            "config_unreadable"
        );
        assert!(snapshot.get("codex_multi_agent").is_none());
    }

    #[test]
    fn agent_system_snapshot_ignores_legacy_multi_agent_alias_field() {
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
        assert!(snapshot.get("codex_multi_agent").is_none());
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
    fn build_dev_team_readiness_reports_configured_roles_and_sequence() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(
            &config_path,
            r#"
dev_team:
  enabled: true
  contract_doc: docs/process/team-development-and-orchestration-protocol.md
  roles:
    developer:
      runtime_role: worker
      task_classes: [implementation, delivery_task]
      default_carrier: junior
      default_model: gpt-5.4
      default_model_reasoning_effort: low
      cost_policy:
        budget_units: 1
        fallback_carrier: middle
        selection_rule: cheapest_eligible_write_carrier
      handoff:
        next_role: coach
        required_outputs: [changed_files]
        fail_closed_on_missing_handoff: true
    coach:
      runtime_role: coach
      task_classes: [review]
      default_carrier: middle
      default_model: gpt-5.4
      default_model_reasoning_effort: medium
      cost_policy:
        budget_units: 4
        selection_rule: cheapest_eligible_review_carrier
      handoff:
        next_role: developer
        required_outputs: [review_notes]
        fail_closed_on_missing_handoff: true
  flows:
    default_delivery:
      enabled: true
      sequential: true
      allow_parallel_handoffs: false
      steps: [developer, coach]
"#,
        )
        .expect("config should write");
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.4",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"]
                        },
                        {
                            "role_id": "middle",
                            "model": "gpt-5.4",
                            "model_provider": "openai",
                            "model_reasoning_effort": "medium",
                            "normalized_cost_units": 4,
                            "rate": 4,
                            "runtime_roles": ["coach"],
                            "task_classes": ["review"]
                        }
                    ]
                }
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["configured"], true);
        assert_eq!(
            readiness["sequence"],
            serde_json::json!(["developer", "coach"])
        );
        assert_eq!(readiness["roles"][1]["role_id"], "developer");
        assert_eq!(
            readiness["roles"][1]["selected_carrier"]["carrier_id"],
            "junior"
        );
        assert_eq!(
            readiness["source_paths"][1],
            "docs/process/team-development-and-orchestration-protocol.md"
        );
    }

    #[test]
    fn build_dev_team_readiness_reports_selected_model_pricing_metadata() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(
            &config_path,
            r#"
dev_team:
  enabled: true
  contract_doc: docs/process/team-development-and-orchestration-protocol.md
  roles:
    developer:
      runtime_role: worker
      task_classes: [implementation, delivery_task]
      default_carrier: junior
      default_model: codex_gpt54_low_write
      default_model_reasoning_effort: low
      cost_policy:
        budget_units: 1
        fallback_carrier: middle
        selection_rule: cheapest_eligible_write_carrier
      handoff:
        next_role: coach
        required_outputs: [changed_files]
        fail_closed_on_missing_handoff: true
  flows:
    default_delivery:
      enabled: true
      sequential: true
      allow_parallel_handoffs: false
      steps: [developer]
        "#,
        )
        .expect("config should write");
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.4",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"],
                            "model_profiles": {
                                "codex_gpt54_low_write": {
                                    "provider": "openai",
                                    "model_ref": "gpt-5.4",
                                    "reasoning_effort": "low",
                                    "normalized_cost_units": 1,
                                    "pricing": {
                                        "price_source_kind": "provider_catalog",
                                        "source_paths": [
                                            "docs/process/codex-agent-configuration-guide.md"
                                        ],
                                        "freshness": {
                                            "required": false,
                                            "max_age_days": 30,
                                            "stale_price_policy": "diagnostic_only_use_normalized_cost_units",
                                            "missing_price_policy": "diagnostic_only_use_normalized_cost_units",
                                            "diagnostic_only": true,
                                            "enforced": false
                                        }
                                    }
                                }
                            }
                        }
                    ]
                },
                "agent_system": {
                    "pricing": {
                        "model_profile_defaults": {
                            "pricing": {
                                "price_source_kind": "provider_catalog",
                                "source_paths": [
                                    "docs/process/codex-agent-configuration-guide.md"
                                ],
                                "freshness": {
                                    "required": true,
                                    "max_age_days": 14,
                                    "stale_price_policy": "diagnostic_only_use_normalized_cost_units",
                                    "missing_price_policy": "diagnostic_only_use_normalized_cost_units",
                                    "diagnostic_only": true,
                                    "enforced": false
                                }
                            }
                        }
                    }
                }
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(
            readiness["roles"][0]["selected_model"]["model_profile_id"],
            "codex_gpt54_low_write"
        );
        assert_eq!(readiness["roles"][0]["selected_model"]["selected_rate"], 1);
        assert_eq!(
            readiness["roles"][0]["selected_model"]["selected_rate_source_path"],
            "carrier_runtime.roles[junior].model_profiles.codex_gpt54_low_write.normalized_cost_units"
        );
        assert_eq!(
            readiness["roles"][0]["selected_model"]["pricing"]["price_source_kind"],
            "provider_catalog"
        );
        assert_eq!(
            readiness["roles"][0]["selected_model"]["pricing_freshness"]["required"],
            false
        );
        assert_eq!(
            readiness["roles"][0]["selected_model"]["pricing_freshness_status"],
            "ready"
        );
    }

    #[test]
    fn build_dev_team_readiness_blocks_price_freshness_incomplete() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(
            &config_path,
            r#"
dev_team:
  enabled: true
  roles:
    developer:
      runtime_role: worker
      task_classes: [implementation, delivery_task]
      default_carrier: junior
      default_model: codex_gpt54_low_write
      cost_policy:
        budget_units: 1
      handoff:
        next_role: coach
        required_outputs: [changed_files]
  flows:
    default_delivery:
      enabled: true
      sequential: true
      allow_parallel_handoffs: false
      steps: [developer]
        "#,
        )
        .expect("config should write");
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.4",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"],
                            "model_profiles": {
                                "codex_gpt54_low_write": {
                                    "provider": "openai",
                                    "model_ref": "gpt-5.4",
                                    "reasoning_effort": "low",
                                    "normalized_cost_units": 1,
                                    "pricing": {
                                        "price_source_kind": "provider_catalog",
                                        "source_paths": ["docs/process/codex-agent-configuration-guide.md"],
                                        "freshness": {
                                            "required": true,
                                            "stale_price_policy": "diagnostic_only_use_normalized_cost_units",
                                            "missing_price_policy": "diagnostic_only_use_normalized_cost_units",
                                            "diagnostic_only": true,
                                            "enforced": false
                                        }
                                    }
                                }
                            }
                        }
                    ]
                }
            }),
        );
        assert_eq!(readiness["status"], "blocked");
        assert!(readiness["blockers"]
            .as_array()
            .expect("readiness blockers should be array")
            .iter()
            .any(|entry| entry == "model_price_freshness_policy_incomplete:developer"));
        assert_eq!(
            readiness["roles"][0]["selected_model"]["pricing_freshness_status"],
            "missing"
        );
    }

    #[test]
    fn build_dev_team_readiness_reports_missing_config_truthfully() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(&config_path, "project:\n  id: demo\n").expect("config should write");

        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({}),
        );

        assert_eq!(readiness["status"], "missing_config");
        assert_eq!(readiness["configured"], false);
        assert_eq!(
            readiness["blockers"],
            serde_json::json!(["missing_dev_team_config"])
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
                        "normalized_cost_units": 4,
                        "runtime_roles": ["business_analyst", "coach"],
                        "task_classes": ["specification"],
                        "default_runtime_role": "business_analyst",
                        "reasoning_band": "medium",
                        "model": "gpt-5.4",
                        "model_provider": "openai",
                        "model_reasoning_effort": "medium",
                        "plan_mode_reasoning_effort": "high",
                        "sandbox_mode": "workspace-write",
                        "default_model_profile": "codex_gpt54_medium_write",
                        "model_profiles": {
                            "codex_gpt54_medium_write": {
                                "profile_id": "codex_gpt54_medium_write",
                                "model_ref": "gpt-5.4",
                                "provider": "openai",
                                "reasoning_effort": "medium",
                                "plan_mode_reasoning_effort": "high",
                                "sandbox_mode": "workspace-write",
                                "normalized_cost_units": 4,
                                "speed_tier": "fast",
                                "quality_tier": "medium",
                                "write_scope": "workspace-write",
                                "runtime_roles": ["business_analyst", "coach"],
                                "task_classes": ["specification"]
                            }
                        }
                    }
                ],
                "model_selection": {
                    "enabled": true,
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality"
                },
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
        assert_eq!(
            snapshot["carriers"][0]["default_model_profile"],
            "codex_gpt54_medium_write"
        );
        assert_eq!(snapshot["agent_model"]["model_selection_enabled"], true);
        assert_eq!(
            snapshot["agent_model"]["selection_rule"],
            "role_task_then_readiness_then_score_then_cost_quality"
        );
        assert_eq!(snapshot["dispatch_aliases"][0]["role_id"], "specification");
    }

    #[test]
    fn agent_system_snapshot_model_selection_enabled_follows_enabled_flag() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 1
            },
            "autonomous_execution": {
                "enabled": true
            },
            "carrier_runtime": {
                "roles": [],
                "model_selection": {
                    "enabled": false,
                    "selection_rule": "role_task_then_readiness_then_score_then_cost_quality"
                },
                "worker_strategy": {
                    "selection_policy": {
                        "rule": "capability_first_then_score_guard_then_cheapest_tier"
                    },
                    "agents": {},
                    "store_path": ".vida/data/state/agents.json",
                    "scorecards_path": ".vida/data/state/agent-scorecards.json"
                },
                "dispatch_aliases": []
            }
        });

        let snapshot = build_taskflow_agent_system_snapshot("vida.config.yaml", &activation_bundle);

        assert_eq!(snapshot["agent_model"]["model_selection_enabled"], false);
        assert!(snapshot["agent_model"]["model_selection"].is_object());
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
                    "carrier_catalog_owner": "vida.config.yaml -> host_environment.systems.codex.carriers",
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
            "vida.config.yaml -> host_environment.systems.codex.carriers"
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
    fn consume_bundle_next_actions_include_retrieval_trust_remediation() {
        let actions = consume_bundle_check_next_actions(&[
            "missing_retrieval_trust_evidence_field:source".to_string(),
        ]);
        assert!(actions
            .iter()
            .any(|action| action.contains("vida taskflow protocol-binding sync --json")));
        assert!(actions
            .iter()
            .any(|action| action.contains("vida taskflow consume bundle check --json")));
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
        assert_eq!(seam["receipt_backed"], true);
        assert_eq!(seam["total_receipts"], 2);
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
        assert_eq!(seam["protocol_binding_receipt_id"], "");
        assert_eq!(seam["protocol_binding_binding_status"], "blocked");
        assert_eq!(seam["protocol_binding_protocol_rows"], 0);
        assert_eq!(
            seam["receipt_evidence"]["readiness_receipt_path"],
            "vida/config/docflow-readiness.current.jsonl"
        );
        assert_eq!(
            seam["receipt_evidence"]["proof_receipt_path"],
            "vida/config/docflow-proof.current.jsonl"
        );
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
        assert_eq!(seam["receipt_backed"], true);
        assert_eq!(seam["total_receipts"], 2);
        assert_eq!(seam["closure_inputs_ready"], true);
        assert_eq!(seam["docflow_status"], "pass");
        assert_eq!(seam["blocker_codes"], serde_json::json!([]));
        assert_eq!(seam["receipt_evidence"]["receipt_backed"], true);
        assert_eq!(seam["protocol_binding_receipt_id"], "");
        assert_eq!(seam["protocol_binding_binding_status"], "blocked");
        assert_eq!(seam["protocol_binding_protocol_rows"], 0);
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
