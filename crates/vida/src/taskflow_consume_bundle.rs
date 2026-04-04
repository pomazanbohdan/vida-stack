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
            let seam_closure_admission_receipt_check =
                taskflow_docflow_seam_receipt_backed_check(&payload);
            if !seam_closure_admission_receipt_check["receipt_backed"]
                .as_bool()
                .unwrap_or(false)
            {
                effective_blockers.push(
                    crate::release1_contracts::blocker_code_str(
                        crate::release1_contracts::BlockerCode::MissingProtocolBindingReceipt,
                    )
                    .to_string(),
                );
            }
            let db_first_activation_truth = match super::read_or_sync_launcher_activation_snapshot(
                &store,
            )
            .await
            {
                Ok(snapshot) => {
                    if let Some(error) = db_first_activation_snapshot_validation_error(&snapshot) {
                        if let Some(code) = crate::release1_contracts::blocker_code_value(
                                crate::release1_contracts::BlockerCode::MissingLauncherActivationSnapshot,
                            ) {
                                effective_blockers.push(code);
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
                    if let Some(code) = crate::release1_contracts::blocker_code_value(
                        crate::release1_contracts::BlockerCode::MissingLauncherActivationSnapshot,
                    ) {
                        effective_blockers.push(code);
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
    let canonical =
        crate::release1_contracts::canonical_blocker_code_list(blockers.iter().map(String::as_str));
    if canonical.is_empty() && !blockers.is_empty() {
        return crate::release1_contracts::blocker_code_value(
            crate::release1_contracts::BlockerCode::Unsupported,
        )
        .into_iter()
        .collect();
    }
    canonical
}

fn consume_bundle_operator_contract_status(blockers: &[String]) -> &'static str {
    crate::release1_contracts::release1_contract_status_str(blockers.is_empty())
}

fn bundle_check_operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    let Some(canonical_status) =
        crate::release1_contracts::canonical_release1_contract_status_str(status)
    else {
        return Some(
            "operator contract inconsistency: status must be canonical release-1 pass/blocked"
                .to_string(),
        );
    };
    let string_is_canonical_nonempty = |value: &String| {
        let trimmed = value.trim();
        !trimmed.is_empty() && trimmed == value
    };

    if !blocker_codes.iter().all(string_is_canonical_nonempty)
        || !next_actions.iter().all(string_is_canonical_nonempty)
    {
        return Some(
            "operator contract inconsistency: shared string arrays must contain only canonical nonempty entries"
                .to_string(),
        );
    }

    match canonical_status {
        "pass" if !blocker_codes.is_empty() => Some(
            "operator contract inconsistency: status=pass must not include blocker_codes"
                .to_string(),
        ),
        "pass" if !next_actions.is_empty() => Some(
            "operator contract inconsistency: status=pass must not include next_actions"
                .to_string(),
        ),
        "pass" => None,
        "blocked" if blocker_codes.is_empty() => Some(
            "operator contract inconsistency: status=blocked requires blocker_codes".to_string(),
        ),
        "blocked" if next_actions.is_empty() => Some(
            "operator contract inconsistency: status=blocked requires next_actions".to_string(),
        ),
        "blocked" => None,
        _ => unreachable!("canonical release-1 contract status should be pass or blocked"),
    }
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
    if snapshot.source_config_path.trim().is_empty() {
        return Some("authoritative launcher activation source_config_path is empty".to_string());
    }
    if snapshot.source_config_digest.trim().is_empty() {
        return Some("authoritative launcher activation source_config_digest is empty".to_string());
    }
    None
}

fn taskflow_docflow_seam_receipt_backed_check(
    payload: &super::TaskflowConsumeBundlePayload,
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
    serde_json::json!({
        "status": crate::release1_contracts::release1_contract_status_str(total_receipts > 0),
        "receipt_backed": total_receipts > 0,
        "total_receipts": total_receipts,
        "surface": "vida taskflow protocol-binding status --json",
        "notes": "TaskFlow->DocFlow seam closure admission requires receipt-backed protocol-binding evidence.",
    })
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

    let selection_rule = carrier_runtime["worker_strategy"]["selection_policy"]["rule"]
        .as_str()
        .unwrap_or("capability_first_then_score_guard_then_cheapest_tier");
    let max_parallel_agents =
        normalize_agent_system_max_parallel_agents(&activation_bundle["agent_system"]);

    serde_json::json!({
        "materialization_mode": "config_materialized_runtime_projection",
        "source_of_truth": {
            "carrier_catalog_owner": "vida.config.yaml -> configured host-system carrier surfaces",
            "dispatch_alias_owner": "vida.config.yaml -> agent_extensions.registries.dispatch_aliases",
            "runtime_config_path": config_path,
        },
        "agent_model": {
            "agent_identity": "execution_carrier",
            "runtime_role_identity": "activation_state",
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
        fail_fast_with_timeout, normalize_agent_system_max_parallel_agents,
        normalize_consume_bundle_blocker_codes, taskflow_docflow_seam_receipt_backed_check,
    };
    use crate::{
        release1_contracts::release1_contract_status_str, DoctorLauncherSummary,
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
            },
            metadata: serde_json::json!({}),
            control_core: serde_json::json!({}),
            activation_bundle: serde_json::json!({}),
            protocol_binding_registry: serde_json::json!({}),
            cache_delivery_contract: serde_json::json!({}),
            orchestrator_init_view: serde_json::json!({}),
            agent_init_view: serde_json::json!({}),
            boot_compatibility: serde_json::json!({}),
            migration_preflight: serde_json::json!({}),
            task_store: serde_json::json!({}),
            run_graph: serde_json::json!({}),
        }
    }

    #[test]
    fn agent_system_snapshot_accepts_legacy_codex_multi_agent_alias() {
        let activation_bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 2
            },
            "autonomous_execution": {
                "enabled": true
            },
            "codex_multi_agent": {
                "roles": [
                    {
                        "role_id": "legacy",
                        "tier": "legacy",
                        "rate": 4,
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "default_runtime_role": "worker",
                        "reasoning_band": "medium",
                        "model_reasoning_effort": "medium"
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
        assert_eq!(snapshot["carriers"][0]["carrier_id"], "legacy");
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
            release1_contract_status_str(true)
        );
        assert_eq!(
            consume_bundle_operator_contract_status(&[
                "missing_protocol_binding_receipt".to_string()
            ]),
            release1_contract_status_str(false)
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
        let seam = taskflow_docflow_seam_receipt_backed_check(&payload);

        assert_eq!(seam["status"], "blocked");
        assert_eq!(seam["receipt_backed"], false);
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
