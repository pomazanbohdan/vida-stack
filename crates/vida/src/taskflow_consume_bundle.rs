use std::collections::HashSet;
use std::process::ExitCode;

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
    match super::StateStore::open_existing(state_dir).await {
        Ok(store) => match super::build_taskflow_consume_bundle_payload(&store).await {
            Ok(payload) => {
                let check = super::taskflow_consume_bundle_check(&payload);
                let mut effective_blockers = check.blockers.clone();
                let seam_closure_admission_receipt_check =
                    taskflow_docflow_seam_receipt_backed_check(&payload);
                if !seam_closure_admission_receipt_check["receipt_backed"]
                    .as_bool()
                    .unwrap_or(false)
                {
                    effective_blockers.push("missing_protocol_binding_receipt".to_string());
                }
                let db_first_activation_truth =
                    match super::read_or_sync_launcher_activation_snapshot(&store).await {
                        Ok(snapshot) => {
                            if let Some(error) =
                                db_first_activation_snapshot_validation_error(&snapshot)
                            {
                                effective_blockers
                                    .push("missing_launcher_activation_snapshot".to_string());
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
                            effective_blockers.push("missing_launcher_activation_snapshot".to_string());
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
                let operator_contracts = serde_json::json!({
                    "contract_id": "release-1-operator-contracts",
                    "schema_version": "release-1-v1",
                    "status": if blocker_codes.is_empty() { "pass" } else { "blocked" },
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
        },
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}

fn consume_bundle_check_next_actions(blockers: &[String]) -> Vec<String> {
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
            crate::release1_contracts::BlockerCode::UnsupportedBlockerCode,
        )
        .into_iter()
        .collect();
    }
    canonical
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
    let total_receipts = payload.protocol_binding_registry["summary"]["total_receipts"]
        .as_u64()
        .unwrap_or(0);
    serde_json::json!({
        "status": if total_receipts > 0 { "pass" } else { "block" },
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
    let mut carriers = activation_bundle["codex_multi_agent"]["roles"]
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

    let selection_rule = activation_bundle["codex_multi_agent"]["worker_strategy"]
        ["selection_policy"]["rule"]
        .as_str()
        .unwrap_or("capability_first_then_score_guard_then_cheapest_tier");
    let max_parallel_agents =
        normalize_agent_system_max_parallel_agents(&activation_bundle["agent_system"]);

    serde_json::json!({
        "materialization_mode": "config_materialized_runtime_projection",
        "source_of_truth": {
            "carrier_catalog_owner": "vida.config.yaml -> host_environment.codex.agents",
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
            "selection_policy": activation_bundle["codex_multi_agent"]["worker_strategy"]["selection_policy"],
            "agents": activation_bundle["codex_multi_agent"]["worker_strategy"]["agents"],
            "store_path": activation_bundle["codex_multi_agent"]["worker_strategy"]["store_path"],
            "scorecards_path": activation_bundle["codex_multi_agent"]["worker_strategy"]["scorecards_path"],
        },
        "dispatch_aliases": activation_bundle["codex_multi_agent"]["dispatch_aliases"],
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
        build_taskflow_agent_system_snapshot, consume_bundle_check_next_actions,
        normalize_agent_system_max_parallel_agents, normalize_consume_bundle_blocker_codes,
    };

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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
            "codex_multi_agent": {
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
    fn consume_bundle_next_actions_fall_back_for_unsupported_blocker_code() {
        let actions =
            consume_bundle_check_next_actions(&["unsupported_blocker_code".to_string()]);
        assert_eq!(
            actions,
            vec!["Resolve consume-bundle-check blockers before closure packaging.".to_string()]
        );
    }
}
