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
                let snapshot_path = match super::write_runtime_consumption_snapshot(
                    store.root(),
                    "bundle-check",
                    &serde_json::json!({
                        "surface": "vida taskflow consume bundle check",
                        "check": &check,
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
                            "check": check,
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
                        if check.ok { "true" } else { "false" },
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
                    if !check.blockers.is_empty() {
                        super::print_surface_line(
                            super::RenderMode::Plain,
                            "blockers",
                            &check.blockers.join(", "),
                        );
                    }
                    super::print_surface_line(
                        super::RenderMode::Plain,
                        "snapshot path",
                        &snapshot_path,
                    );
                }
                if check.ok {
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
            "max_parallel_agents": activation_bundle["agent_system"]["max_parallel_agents"],
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
