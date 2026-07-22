use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::process::ExitCode;
use std::time::Duration;

const CONSUME_BUNDLE_CHECK_OPEN_LOCK_TIMEOUT: Duration = Duration::from_secs(15);
const CONSUME_BUNDLE_CHECK_PAYLOAD_TIMEOUT: Duration = Duration::from_secs(30);

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
            Some(run_consume_bundle_check(args, false).await)
        }
        [head, subcommand, mode, flag]
            if head == "consume"
                && subcommand == "bundle"
                && mode == "check"
                && flag == "--json" =>
        {
            Some(run_consume_bundle_check(args, true).await)
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

async fn run_consume_bundle_check(args: &[String], as_json: bool) -> ExitCode {
    let state_dir = super::taskflow_task_bridge::proxy_state_dir();
    let store = match fail_fast_with_timeout(
        "opening authoritative state store",
        CONSUME_BUNDLE_CHECK_OPEN_LOCK_TIMEOUT,
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
        CONSUME_BUNDLE_CHECK_PAYLOAD_TIMEOUT,
        super::build_taskflow_consume_bundle_payload(&store),
    )
    .await
    {
        Ok(payload) => {
            let check = super::taskflow_consume_bundle_check(&payload);
            let mut effective_blockers = check.blockers.clone();
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
            let team_flow_binding_diagnostics = current_team_flow_binding_diagnostics(
                &payload.vida_root,
                db_first_activation_truth["source_config_path"].as_str(),
            );
            merge_team_flow_binding_diagnostics(
                &mut effective_blockers,
                &team_flow_binding_diagnostics,
            );
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
            let cache_contract_blocked = effective_blockers.iter().any(|code| {
                code.starts_with("invalid_cache_key_input:")
                    || code.starts_with("invalid_invalidation_tuple_key:")
                    || code.starts_with("missing_cache_key_inputs")
                    || code.starts_with("missing_invalidation_tuple")
            });
            if !cache_contract_blocked {
                for blocker in seam_closure_admission_receipt_check["blocker_codes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                {
                    push_unique_string(&mut effective_blockers, blocker);
                }
            }
            let blocker_codes = normalize_consume_bundle_blocker_codes(&effective_blockers);
            let next_actions = consume_bundle_check_next_actions(&blocker_codes);
            let bundle_check_ok = blocker_codes.is_empty();
            let artifact_refs = serde_json::json!({
                "root_artifact_id": check.root_artifact_id,
                "bundle_artifact_name": payload.artifact_name,
                "surface": "vida taskflow consume bundle check",
                "team_flow_binding": team_flow_binding_diagnostics["context"].clone(),
            });
            let operator_output =
                match crate::release1_operator_output::build_release1_operator_output_payload(
                    "vida taskflow consume bundle check",
                    blocker_codes,
                    next_actions,
                    artifact_refs,
                    serde_json::json!({}),
                ) {
                    Ok(payload) => payload,
                    Err(error) => {
                        eprintln!("consume bundle check: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let snapshot_path = match super::write_runtime_consumption_snapshot(
                store.root(),
                "bundle-check",
                &serde_json::json!({
                    "surface": "vida taskflow consume bundle check",
                    "status": operator_output["status"].clone(),
                    "check": &check,
                    "seam_closure_admission_receipt_check": &seam_closure_admission_receipt_check,
                    "db_first_activation_truth": &db_first_activation_truth,
                    "effective_blockers": &effective_blockers,
                    "blocker_codes": operator_output["blocker_codes"].clone(),
                    "next_actions": operator_output["next_actions"].clone(),
                    "artifact_refs": operator_output["artifact_refs"].clone(),
                    "shared_fields": operator_output["shared_fields"].clone(),
                    "operator_contracts": operator_output["operator_contracts"].clone(),
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
                    serde_json::to_string_pretty(&consume_bundle_check_operator_envelope(
                        &check,
                        &seam_closure_admission_receipt_check,
                        &db_first_activation_truth,
                        &effective_blockers,
                        &operator_output,
                        &snapshot_path,
                    ))
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
                    if bundle_check_ok { "true" } else { "false" },
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
                    let actions = operator_output["next_actions"]
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
            if bundle_check_ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            if as_json {
                let raw_args = args
                    .iter()
                    .map(std::ffi::OsString::from)
                    .collect::<Vec<_>>();
                super::emit_pre_dispatch_error_with_surface(
                    &raw_args,
                    Some("vida taskflow consume bundle check"),
                    &error,
                );
            } else {
                eprintln!("{error}");
            }
            ExitCode::from(1)
        }
    }
}

fn consume_bundle_check_operator_envelope(
    check: &super::TaskflowConsumeBundleCheck,
    seam_closure_admission_receipt_check: &serde_json::Value,
    db_first_activation_truth: &serde_json::Value,
    effective_blockers: &[String],
    operator_output: &serde_json::Value,
    snapshot_path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida taskflow consume bundle check",
        "status": operator_output["status"].clone(),
        "check": check,
        "seam_closure_admission_receipt_check": seam_closure_admission_receipt_check,
        "db_first_activation_truth": db_first_activation_truth,
        "effective_blockers": effective_blockers,
        "blocker_codes": operator_output["blocker_codes"].clone(),
        "next_actions": operator_output["next_actions"].clone(),
        "artifact_refs": operator_output["artifact_refs"].clone(),
        "shared_fields": operator_output["shared_fields"].clone(),
        "operator_contracts": operator_output["operator_contracts"].clone(),
        "snapshot_path": snapshot_path,
    })
}

struct TeamFlowDiagnosticsSource {
    project_root: std::path::PathBuf,
    config_path: Option<std::path::PathBuf>,
    source: &'static str,
}

fn resolve_team_flow_diagnostics_source(
    fallback_root: &str,
    persisted_config_path: Option<&str>,
) -> TeamFlowDiagnosticsSource {
    let fallback_root = std::path::PathBuf::from(fallback_root);
    let Some(raw_path) = persisted_config_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return TeamFlowDiagnosticsSource {
            project_root: fallback_root,
            config_path: None,
            source: "cwd_fallback_no_persisted_config",
        };
    };
    let candidate = std::path::PathBuf::from(raw_path);
    let config_path = if candidate.is_absolute()
        || raw_path.starts_with('/')
        || raw_path.as_bytes().first() == Some(&92)
        || raw_path.as_bytes().get(1) == Some(&b':')
    {
        candidate
    } else {
        fallback_root.join(candidate)
    };
    let project_root = config_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| fallback_root.clone());
    TeamFlowDiagnosticsSource {
        project_root,
        config_path: Some(config_path),
        source: "persisted_launcher_snapshot",
    }
}

fn load_team_flow_diagnostics_overlay(
    source: &TeamFlowDiagnosticsSource,
) -> Result<serde_yaml::Value, String> {
    let Some(config_path) = source.config_path.as_ref() else {
        return crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(
            &source.project_root,
        );
    };
    let raw = std::fs::read_to_string(config_path)
        .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
    serde_yaml::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", config_path.display()))
}

fn audit_team_flow_binding_policies(compiled_bundle: &serde_json::Value) -> serde_json::Value {
    let bindings = compiled_bundle
        .get("team_flow_authority")
        .and_then(|authority| authority.get("resolved_all_flow_payload"))
        .and_then(|payload| payload.get("work_item_flow_bindings"))
        .and_then(serde_json::Value::as_object);
    let Some(bindings) = bindings else {
        return serde_json::json!({
            "status": "blocked",
            "blockers": ["team_flow_authority_binding_invalid"],
            "context": {"bindings": [], "validation_error": "persisted binding map missing"},
        });
    };
    let mut blockers = Vec::new();
    let mut binding_context = Vec::new();
    for (work_item, flow_id_value) in bindings {
        let Some(flow_id) = flow_id_value
            .as_str()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            push_unique_string(&mut blockers, "team_flow_authority_binding_invalid");
            binding_context.push(serde_json::json!({
                "work_item": work_item,
                "flow_id": flow_id_value,
                "status": "blocked",
                "validation_error": "binding target must be a non-empty string",
            }));
            continue;
        };
        match crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            compiled_bundle,
            Some(flow_id),
            None,
        ) {
            Ok(_) => binding_context.push(serde_json::json!({
                "work_item": work_item,
                "flow_id": flow_id,
                "status": "ready",
            })),
            Err(error) => {
                let blocker = if error.code == "team_flow_authority_flow_policy_disabled" {
                    "team_flow_authority_flow_policy_disabled"
                } else {
                    "team_flow_authority_binding_invalid"
                };
                push_unique_string(&mut blockers, blocker);
                binding_context.push(serde_json::json!({
                    "work_item": work_item,
                    "flow_id": flow_id,
                    "status": "blocked",
                    "blocker_code": blocker,
                    "validation_error": error.to_string(),
                }));
            }
        }
    }
    serde_json::json!({
        "status": if blockers.is_empty() { "ready" } else { "blocked" },
        "blockers": blockers,
        "context": {"bindings": binding_context},
    })
}

fn current_team_flow_binding_diagnostics(
    vida_root: &str,
    persisted_config_path: Option<&str>,
) -> serde_json::Value {
    let source = resolve_team_flow_diagnostics_source(vida_root, persisted_config_path);
    let source_config_path = source
        .config_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| {
            source
                .project_root
                .join("vida.config.yaml")
                .display()
                .to_string()
        });
    let overlay = match load_team_flow_diagnostics_overlay(&source) {
        Ok(overlay) => overlay,
        Err(error) => {
            return serde_json::json!({
                "status": "blocked",
                "blockers": ["team_flow_authority_unavailable"],
                "context": {
                    "source": source.source,
                    "config_path": source_config_path,
                    "validation_error": error,
                },
            });
        }
    };
    let overlay_json = serde_json::to_value(&overlay).unwrap_or(serde_json::Value::Null);
    let compiled_bundle =
        match super::build_compiled_agent_extension_bundle_for_root(&overlay, &source.project_root)
        {
            Ok(bundle) => bundle,
            Err(error) => {
                return serde_json::json!({
                    "status": "blocked",
                    "blockers": ["team_flow_authority_invalid"],
                    "context": team_flow_binding_context(
                        &overlay_json,
                        None,
                        Some(error.as_str()),
                        source.source,
                        &source_config_path,
                    ),
                });
            }
        };
    let readiness = build_dev_team_readiness(&source_config_path, &compiled_bundle);
    let binding_policy_audit = audit_team_flow_binding_policies(&compiled_bundle);
    let mut blockers = readiness["blockers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    merge_team_flow_binding_diagnostics(&mut blockers, &binding_policy_audit);
    let mut context = team_flow_binding_context(
        &overlay_json,
        Some(&compiled_bundle),
        None,
        source.source,
        &source_config_path,
    );
    context["binding_policy_audit"] = binding_policy_audit["context"].clone();
    let status = if blockers.is_empty() {
        readiness["status"].clone()
    } else {
        serde_json::Value::String("blocked".to_string())
    };
    serde_json::json!({
        "status": status,
        "blockers": blockers,
        "context": context,
    })
}

fn team_flow_binding_context(
    overlay: &serde_json::Value,
    compiled_bundle: Option<&serde_json::Value>,
    validation_error: Option<&str>,
    source: &str,
    config_path: &str,
) -> serde_json::Value {
    let raw_dev_team = overlay.get("dev_team");
    let raw_selected_flow = raw_dev_team
        .and_then(|value| value.get("authority_selection"))
        .and_then(|value| value.get("default_flow_id"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let raw_bindings = raw_dev_team
        .and_then(|value| value.get("work_item_flow_bindings"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let authority = compiled_bundle.and_then(|bundle| bundle.get("team_flow_authority"));
    let selected_flow = authority
        .and_then(|value| value.get("selected_config"))
        .and_then(|value| value.get("authority_selection"))
        .and_then(|value| value.get("default_flow_id"))
        .cloned()
        .filter(|value| !value.is_null())
        .unwrap_or(raw_selected_flow);
    let bound_work_item_flows = authority
        .and_then(|value| value.get("resolved_all_flow_payload"))
        .and_then(|value| value.get("work_item_flow_bindings"))
        .cloned()
        .unwrap_or(raw_bindings);
    let mut context = serde_json::json!({
        "source": source,
        "config_path": config_path,
        "selected_flow": selected_flow,
        "bound_work_item_flows": bound_work_item_flows,
        "authority_id": authority
            .and_then(|value| value.get("authority_id"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "config_authority_hash": authority
            .and_then(|value| value.get("config_authority_hash"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    });
    if let Some(error) = validation_error {
        context["validation_error"] = serde_json::Value::String(error.to_string());
    }
    context
}

fn merge_team_flow_binding_diagnostics(
    effective_blockers: &mut Vec<String>,
    diagnostics: &serde_json::Value,
) {
    for blocker in diagnostics["blockers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
    {
        push_unique_string(effective_blockers, blocker);
    }
}

async fn fail_fast_with_timeout<T, E, F>(
    label: &str,
    timeout: Duration,
    future: F,
) -> Result<T, String>
where
    F: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result.map_err(|error| error.to_string()),
        Err(_) => Err(format!(
            "consume bundle check failed fast: {label} timed out while waiting for authoritative datastore lock; another VIDA process still holds the authoritative datastore lock, so stop or wait for that process and retry the command"
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
            "Run `vida taskflow protocol-binding sync --json` to materialize protocol-bound retrieval-trust source/citation/freshness/ACL evidence; rerun `vida taskflow consume bundle check --json` only to verify the blocker set changed."
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
    crate::release1_operator_output::normalize_blocker_codes(
        blockers,
        crate::release_contract_adapters::canonical_blocker_codes,
        crate::release_contract_adapters::blocker_code(
            crate::release1_contracts::BlockerCode::Unsupported,
        ),
    )
}

fn consume_bundle_operator_contract_status(blockers: &[String]) -> &'static str {
    crate::release1_operator_output::operator_contract_status_for_blockers(
        &crate::release1_operator_output::RELEASE1_OPERATOR_CONTRACT_SPEC,
        blockers,
    )
}

fn bundle_check_operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    crate::release1_operator_output::operator_contracts_consistency_error(
        &crate::release1_operator_output::RELEASE1_OPERATOR_CONTRACT_SPEC,
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
    let docflow_receipt_backed = docflow_receipt_evidence["receipt_backed"]
        .as_bool()
        .unwrap_or(false);
    let protocol_binding_receipt_backed =
        !protocol_binding_receipt_id.is_empty() && binding_status == "bound" && protocol_rows > 0;
    let receipt_backed = docflow_receipt_backed && protocol_binding_receipt_backed;
    let total_receipts = docflow_receipt_evidence["total_receipts"]
        .as_u64()
        .unwrap_or_else(|| u64::from(docflow_receipt_backed))
        + u64::from(protocol_binding_receipt_backed);
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
        "notes": "TaskFlow->DocFlow seam closure admission requires DocFlow readiness/proof evidence and a runtime-bound TaskFlow protocol-binding receipt.",
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
    let team_flow_authority = activation_bundle
        .get("team_flow_authority")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let source_paths = dev_team_source_paths(config_path, None);
    let authority_availability =
        crate::team_flow_authority_adapter::team_flow_authority_availability(
            activation_bundle,
            None,
            None,
        );
    if !authority_availability.is_ready() {
        let status = match authority_availability.status {
            crate::team_flow_authority_adapter::TeamFlowAuthorityAvailabilityStatus::Unavailable =>
                "unavailable",
            crate::team_flow_authority_adapter::TeamFlowAuthorityAvailabilityStatus::Blocked =>
                "blocked",
            crate::team_flow_authority_adapter::TeamFlowAuthorityAvailabilityStatus::Ready =>
                "blocked",
        };
        let blocker = authority_availability
            .blocker
            .unwrap_or_else(|| "team_flow_authority_blocked".to_string());
        let blocker = if status == "unavailable" {
            format!("team_flow_authority_unavailable:{blocker}")
        } else {
            format!("team_flow_authority_invalid:{blocker}")
        };
        return serde_json::json!({
            "status": status,
            "configured": false,
            "enabled": serde_json::Value::Null,
            "default_flow_id": serde_json::Value::Null,
            "orchestrator_command_contract": serde_json::Value::Null,
            "work_item_flow_bindings": {},
            "roles": [],
            "sequence": [],
            "flows": [],
            "team_flow_authority": team_flow_authority,
            "team_flow_authority_status": status,
            "lane_work_context_contract": activation_bundle
                .get("lane_work_context_contract")
                .cloned()
                .unwrap_or_else(default_lane_work_context_contract),
            "zombie_d_gate": serde_json::Value::Null,
            "blockers": [blocker],
            "source_paths": source_paths,
        });
    }
    let team_flow_authority_status = "ready";
    let canonical_projection = authority_availability
        .projection
        .as_ref()
        .expect("ready authority must carry a projection");
    let mut blockers = Vec::new();
    let (roles, flows, work_item_flow_bindings) =
        persisted_dev_team_readiness_projection(&team_flow_authority, &mut blockers);
    let default_flow_id = Some(canonical_projection.snapshot.flow_ref.clone());
    let selected_flow_policy = flows
        .iter()
        .find(|flow| flow["flow_id"].as_str() == default_flow_id.as_deref())
        .and_then(|flow| flow.get("flow_policy"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let enabled = team_flow_authority["selected_config"]["team_flow_enabled"]
        .as_bool()
        .unwrap_or(false);
    if team_flow_authority["selected_config"]["team_flow_enabled"].is_null() {
        blockers.push("team_flow_authority_team_flow_enabled_missing".to_string());
    } else if !enabled {
        blockers.push("dev_team_disabled".to_string());
    }
    if selected_flow_policy.is_null() {
        blockers.push(format!(
            "team_flow_authority_selected_flow_policy_missing:{}",
            canonical_projection.snapshot.flow_ref
        ));
    } else if selected_flow_policy["enabled"].is_null() {
        blockers.push(format!(
            "team_flow_authority_flow_policy_field_missing:{}:enabled",
            canonical_projection.snapshot.flow_ref
        ));
    } else if selected_flow_policy["enabled"].as_bool() != Some(true) {
        blockers.push(format!(
            "team_flow_authority_selected_flow_disabled:{}",
            canonical_projection.snapshot.flow_ref
        ));
    }
    if roles.is_empty() {
        blockers.push("missing_dev_team_roles".to_string());
    }
    let sequence = flows
        .iter()
        .find(|flow| flow["flow_id"].as_str() == default_flow_id.as_deref())
        .and_then(|flow| flow["steps"].as_array())
        .cloned()
        .unwrap_or_default();
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
        "default_flow_id": default_flow_id,
        "orchestrator_command_contract": activation_bundle
            .get("orchestrator_command_contract")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "work_item_flow_bindings": work_item_flow_bindings,
        "roles": roles,
        "sequence": sequence,
        "flows": flows,
        "selected_flow_policy": selected_flow_policy,
        "team_flow_authority": team_flow_authority,
        "team_flow_authority_status": team_flow_authority_status,
        "lane_work_context_contract": activation_bundle
            .get("lane_work_context_contract")
            .cloned()
            .unwrap_or_else(default_lane_work_context_contract),
        "zombie_d_gate": activation_bundle
            .get("zombie_d_gate")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "blockers": blockers,
        "source_paths": source_paths,
    })
}

fn persisted_dev_team_readiness_projection(
    authority: &serde_json::Value,
    blockers: &mut Vec<String>,
) -> (
    Vec<serde_json::Value>,
    Vec<serde_json::Value>,
    serde_json::Value,
) {
    let flow_rows = authority["resolved_all_flow_payload"]["flows"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if flow_rows.is_empty() {
        blockers.push("team_flow_authority_persisted_flows_missing".to_string());
        return (Vec::new(), Vec::new(), serde_json::json!({}));
    }
    let work_item_flow_bindings =
        authority["resolved_all_flow_payload"]["work_item_flow_bindings"].clone();
    if !work_item_flow_bindings.is_object() {
        blockers.push("team_flow_authority_work_item_flow_bindings_missing".to_string());
    }

    let mut role_map = BTreeMap::new();
    let mut flows = Vec::with_capacity(flow_rows.len());
    for flow in flow_rows {
        let Some(flow_id) = flow["flow_id"].as_str().filter(|value| !value.is_empty()) else {
            blockers.push("team_flow_authority_flow_id_missing".to_string());
            continue;
        };
        let policy = flow["flow_policy"].clone();
        let lanes = flow["lanes"].as_array().cloned().unwrap_or_default();
        if lanes.is_empty() {
            blockers.push(format!("team_flow_authority_flow_lanes_missing:{flow_id}"));
        }
        let mut steps = Vec::with_capacity(lanes.len());
        let mut ordered_steps = Vec::with_capacity(lanes.len());
        for (index, lane) in lanes.iter().enumerate() {
            let node_id = lane["node_id"].as_str().unwrap_or_default().to_string();
            if node_id.is_empty() {
                blockers.push(format!(
                    "team_flow_authority_lane_id_missing:{flow_id}:{index}"
                ));
                continue;
            }
            steps.push(serde_json::Value::String(node_id.clone()));
            let mut ordered_step = lane.clone();
            ordered_step["step_id"] = serde_json::Value::String(node_id.clone());
            ordered_step["order"] = serde_json::json!(index);
            ordered_step["role_id"] = serde_json::Value::String(node_id.clone());
            ordered_steps.push(ordered_step);
            role_map
                .entry(node_id.clone())
                .or_insert_with(|| persisted_dev_team_role_projection(lane));
        }
        let mut flow_output = policy.clone();
        if !flow_output.is_object() {
            blockers.push(format!("team_flow_authority_flow_policy_missing:{flow_id}"));
            flow_output = serde_json::json!({});
        }
        flow_output["flow_id"] = serde_json::Value::String(flow_id.to_string());
        flow_output["flow_policy"] = policy;
        flow_output["steps"] = serde_json::Value::Array(steps.clone());
        flow_output["ordered_steps"] = serde_json::Value::Array(ordered_steps);
        flows.push(flow_output);
    }
    flows.sort_by(|left, right| {
        left["flow_id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["flow_id"].as_str().unwrap_or_default())
    });
    let roles = role_map.into_values().collect::<Vec<_>>();
    (roles, flows, work_item_flow_bindings)
}

fn persisted_dev_team_role_projection(lane: &serde_json::Value) -> serde_json::Value {
    let carrier_relation = lane["carrier_relation"].clone();
    let selected_model = lane["selected_model_profile"].clone();
    let required_outputs = lane["proof_gates"]["required_outputs"].clone();
    let next_role = lane["next_node"].clone();
    serde_json::json!({
        "role_id": lane["node_id"],
        "runtime_role": lane["runtime_role"],
        "task_classes": [lane["task_class"]],
        "packet_template_kind": lane["packet_template_kind"],
        "closure_class": lane["closure_class"],
        "stage": lane["stage"],
        "completion_blocker": lane["completion_blocker"],
        "inclusion_rule": lane["inclusion_rule"],
        "default_carrier": carrier_relation["selected_id"],
        "default_model": selected_model["model_ref"],
        "default_model_reasoning_effort": selected_model["reasoning_effort"],
        "cost_policy": serde_json::Value::Null,
        "handoff": {
            "next_role": next_role,
            "required_outputs": required_outputs,
            "fail_closed_on_missing_handoff": true
        },
        "selected_model": {
            "carrier_id": carrier_relation["selected_id"],
            "model_profile_id": selected_model["profile_id"],
            "model_profile_id_source_path": selected_model["selection_source"],
            "model_ref": selected_model["model_ref"],
            "model_ref_source_path": selected_model["selection_source"],
            "model_provider": selected_model["provider"],
            "model_reasoning_effort": selected_model["reasoning_effort"],
            "selected_rate": serde_json::Value::Null,
            "selected_rate_source_path": serde_json::Value::Null,
            "pricing": serde_json::Value::Null,
            "pricing_freshness": serde_json::Value::Null,
            "pricing_freshness_status": "persisted"
        },
        "selected_carrier": {
            "carrier_id": carrier_relation["selected_id"],
            "model": selected_model["model_ref"],
            "model_provider": selected_model["provider"],
            "reasoning_effort": selected_model["reasoning_effort"],
            "runtime_roles": [lane["runtime_role"]],
            "task_classes": [lane["task_class"]]
        }
    })
}

pub(crate) fn lane_role_profile_for_context(
    activation_bundle: &serde_json::Value,
    role_label: &str,
    runtime_role: &str,
    task_class: &str,
) -> serde_json::Value {
    let roles = activation_bundle["dev_team_readiness"]["roles"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let role_label = role_label.trim();
    let runtime_role = runtime_role.trim();
    let task_class = task_class.trim();
    let selected = roles
        .iter()
        .find(|role| role["role_id"].as_str() == Some(role_label))
        .or_else(|| {
            roles.iter().find(|role| {
                role["runtime_role"].as_str() == Some(runtime_role)
                    && role["task_classes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|value| value.as_str() == Some(task_class))
            })
        });
    let Some(role) = selected else {
        return serde_json::json!({
            "status": "unavailable",
            "profile_id": role_label,
            "runtime_role": runtime_role,
            "task_class": task_class,
            "field_set": default_lane_work_context_field_set(),
            "role_specific_fields": [],
            "omitted_sections": default_lane_work_context_omitted_sections(),
            "context_budget": {"max_tokens": 512},
            "source_refs": ["dev_team_readiness.roles"],
            "blocker_code": "lane_role_profile_missing"
        });
    };
    let role_id = role["role_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(role_label);
    let task_classes = role["task_classes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let required_outputs = role["handoff"]["required_outputs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let configured_fields = role["context"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let field_set = if configured_fields.is_empty() {
        default_lane_work_context_field_set()
    } else {
        configured_fields
    };
    let max_tokens = role["context"]["max_tokens"]
        .as_u64()
        .filter(|value| *value > 0)
        .unwrap_or(512);
    let mut source_refs = activation_bundle["dev_team_readiness"]["source_paths"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    source_refs.push("dev_team_readiness.roles".to_string());
    source_refs.sort();
    source_refs.dedup();
    let omitted_sections = activation_bundle
        .get("lane_work_context_contract")
        .and_then(|contract| contract.get("omitted_sections"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!(default_lane_work_context_omitted_sections()));
    serde_json::json!({
        "status": "configured",
        "profile_id": role_id,
        "runtime_role": role["runtime_role"],
        "task_classes": task_classes,
        "field_set": field_set,
        "role_specific_fields": required_outputs.clone(),
        "packet_template_kind": role["packet_template_kind"],
        "closure_class": role["closure_class"],
        "handoff": {"required_outputs": required_outputs},
        "omitted_sections": omitted_sections,
        "context_budget": {"max_tokens": max_tokens},
        "source_refs": source_refs
    })
}

fn default_lane_work_context_contract() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "lane-work-context.v1",
        "required_fields": default_lane_work_context_field_set(),
        "artifact_ref_fields": ["hash", "summary", "load_when"],
        "omitted_sections": default_lane_work_context_omitted_sections(),
        "source_of_truth": "taskflow_consume_bundle"
    })
}

fn default_lane_work_context_field_set() -> Vec<String> {
    vec![
        "task".to_string(),
        "role_profile".to_string(),
        "objective".to_string(),
        "owned_paths".to_string(),
        "read_paths".to_string(),
        "proof_targets".to_string(),
        "constraints".to_string(),
        "source_refs".to_string(),
        "result_schema".to_string(),
        "artifact_refs".to_string(),
        "context_budget".to_string(),
    ]
}

fn default_lane_work_context_omitted_sections() -> Vec<&'static str> {
    vec![
        "carrier_selection",
        "pricing",
        "unrelated_boot_diagnostics",
        "full_runtime_bundle",
        "conversation_transcript",
    ]
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
            "packet_template_kind": crate::yaml_string(
                crate::yaml_lookup(role_entry, &["packet_template_kind"])
            ),
            "closure_class": crate::yaml_string(
                crate::yaml_lookup(role_entry, &["closure_class"])
            ),
            "stage": crate::yaml_string(crate::yaml_lookup(role_entry, &["stage"])),
            "completion_blocker": crate::yaml_string(
                crate::yaml_lookup(role_entry, &["completion_blocker"])
            ),
            "inclusion_rule": crate::yaml_string(
                crate::yaml_lookup(role_entry, &["inclusion_rule"])
            ),
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

fn configured_lifecycle_hook_templates(overlay: &serde_yaml::Value) -> HashSet<String> {
    crate::yaml_string_list(crate::yaml_lookup(
        overlay,
        &["agent_extensions", "enabled_hook_templates"],
    ))
    .into_iter()
    .collect()
}

fn validate_lifecycle_hook_refs(
    surface: &str,
    hook_refs: &[String],
    configured_hook_templates: &HashSet<String>,
    blockers: &mut Vec<String>,
) {
    for hook_ref in hook_refs {
        if !configured_hook_templates.contains(hook_ref) {
            blockers.push(format!(
                "unknown_lifecycle_hook_template:{surface}:{hook_ref}"
            ));
        }
    }
}

fn validate_approval_policy(
    flow_id: &str,
    index: usize,
    step: &serde_yaml::Value,
    requires_user_approval: bool,
    blockers: &mut Vec<String>,
) {
    let Some(policy) = crate::yaml_lookup(step, &["approval_policy"]) else {
        if requires_user_approval {
            blockers.push(format!("missing_approval_policy:{flow_id}:{index}"));
        }
        return;
    };
    let Some(policy_map) = policy.as_mapping() else {
        blockers.push(format!("invalid_approval_policy:{flow_id}:{index}"));
        return;
    };
    if policy_map.is_empty() && requires_user_approval {
        blockers.push(format!("missing_approval_policy:{flow_id}:{index}"));
        return;
    }
    let mode = crate::yaml_string(crate::yaml_lookup(step, &["approval_policy", "mode"]));
    if requires_user_approval && mode.as_deref().unwrap_or_default().is_empty() {
        blockers.push(format!("missing_approval_policy_mode:{flow_id}:{index}"));
    }
}

fn dev_team_flows(
    dev_team: &serde_yaml::Value,
    roles: &[serde_json::Value],
    configured_hook_templates: &HashSet<String>,
    blockers: &mut Vec<String>,
) -> Vec<serde_json::Value> {
    let mut flows = Vec::new();
    let known_roles = roles
        .iter()
        .filter_map(|row| row["role_id"].as_str())
        .collect::<HashSet<_>>();
    let default_flow_id = crate::yaml_string(crate::yaml_lookup(dev_team, &["default_flow_id"]))
        .or_else(|| crate::yaml_string(crate::yaml_lookup(dev_team, &["default_flow"])));
    let Some(flow_map) =
        crate::yaml_lookup(dev_team, &["flows"]).and_then(serde_yaml::Value::as_mapping)
    else {
        return flows;
    };

    for (flow_id, flow_entry) in flow_map
        .iter()
        .filter_map(|(key, value)| key.as_str().map(|id| (id, value)))
    {
        let flow_hook_templates = crate::yaml_string_list(crate::yaml_lookup(
            flow_entry,
            &["lifecycle_hook_templates"],
        ));
        validate_lifecycle_hook_refs(
            &format!("flow:{flow_id}"),
            &flow_hook_templates,
            configured_hook_templates,
            blockers,
        );
        let ordered_steps = dev_team_flow_ordered_steps(
            flow_id,
            flow_entry,
            &known_roles,
            configured_hook_templates,
            blockers,
        );
        let steps = ordered_steps
            .iter()
            .filter_map(|step| step["role_id"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        if steps.is_empty() {
            blockers.push(format!("missing_flow_steps:{flow_id}"));
        }
        flows.push(serde_json::json!({
            "flow_id": flow_id,
            "enabled": crate::yaml_bool(crate::yaml_lookup(flow_entry, &["enabled"]), true),
            "default": default_flow_id.as_deref() == Some(flow_id)
                || crate::yaml_bool(crate::yaml_lookup(flow_entry, &["default"]), false),
            "description": crate::yaml_string(crate::yaml_lookup(flow_entry, &["description"])),
            "flow_class": crate::yaml_string(crate::yaml_lookup(flow_entry, &["flow_class"]))
                .unwrap_or_else(|| "development".to_string()),
            "work_item_bindings": crate::yaml_string_list(crate::yaml_lookup(flow_entry, &["work_item_bindings"])),
            "sequential": crate::yaml_bool(crate::yaml_lookup(flow_entry, &["sequential"]), false),
            "allow_parallel_handoffs": crate::yaml_bool(
                crate::yaml_lookup(flow_entry, &["allow_parallel_handoffs"]),
                false,
            ),
            "lifecycle_hook_templates": flow_hook_templates,
            "proof_gates": yaml_field_json(flow_entry, "proof_gates"),
            "resume_transitions": yaml_field_json(flow_entry, "resume_transitions"),
            "rework_transitions": yaml_field_json(flow_entry, "rework_transitions"),
            "adapter_projection": yaml_field_json(flow_entry, "adapter_projection"),
            "steps": steps,
            "ordered_steps": ordered_steps,
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

fn dev_team_work_item_flow_bindings(
    dev_team: &serde_yaml::Value,
    flows: &[serde_json::Value],
    blockers: &mut Vec<String>,
) -> serde_json::Value {
    let known_flows = flows
        .iter()
        .filter_map(|flow| flow["flow_id"].as_str())
        .collect::<HashSet<_>>();
    let mut bindings = serde_json::Map::new();
    if let Some(binding_map) = crate::yaml_lookup(dev_team, &["work_item_flow_bindings"])
        .and_then(serde_yaml::Value::as_mapping)
    {
        for (key, value) in binding_map {
            let Some(work_item_type) = key
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let flow_id = crate::yaml_string(Some(value)).unwrap_or_default();
            let flow_id = flow_id.trim().to_string();
            if flow_id.is_empty() {
                blockers.push(format!("missing_work_item_flow_binding:{work_item_type}"));
                continue;
            }
            if !known_flows.contains(flow_id.as_str()) {
                blockers.push(format!(
                    "unknown_work_item_flow_binding:{work_item_type}:{flow_id}"
                ));
            }
            bindings.insert(
                work_item_type.to_ascii_lowercase(),
                serde_json::Value::String(flow_id),
            );
        }
    }
    for flow in flows {
        let Some(flow_id) = flow["flow_id"].as_str() else {
            continue;
        };
        for work_item_type in crate::json_string_list(flow.get("work_item_bindings")) {
            bindings
                .entry(work_item_type.to_ascii_lowercase())
                .or_insert_with(|| serde_json::Value::String(flow_id.to_string()));
        }
    }
    serde_json::Value::Object(bindings)
}

fn yaml_field_json(value: &serde_yaml::Value, key: &str) -> serde_json::Value {
    crate::yaml_lookup(value, &[key])
        .and_then(|entry| serde_json::to_value(entry).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn dev_team_flow_ordered_steps(
    flow_id: &str,
    flow_entry: &serde_yaml::Value,
    known_roles: &HashSet<&str>,
    configured_hook_templates: &HashSet<String>,
    blockers: &mut Vec<String>,
) -> Vec<serde_json::Value> {
    let Some(steps_value) = crate::yaml_lookup(flow_entry, &["steps"]) else {
        return Vec::new();
    };
    let Some(step_rows) = steps_value.as_sequence() else {
        blockers.push(format!("invalid_flow_steps:{flow_id}"));
        return Vec::new();
    };
    step_rows
        .iter()
        .enumerate()
        .filter_map(|(index, step)| match step {
            serde_yaml::Value::String(role_id) => {
                let role_id = role_id.trim();
                if role_id.is_empty() {
                    blockers.push(format!("missing_flow_step_role:{flow_id}:{index}"));
                    return None;
                }
                if !known_roles.contains(role_id) {
                    blockers.push(format!("unknown_flow_step:{flow_id}:{role_id}"));
                }
                Some(serde_json::json!({
                    "step_id": format!("{flow_id}-{index}"),
                    "order": index,
                    "role_id": role_id,
                    "requires_user_approval": false,
                }))
            }
            serde_yaml::Value::Mapping(_) => {
                let role_id = crate::yaml_string(crate::yaml_lookup(step, &["role_id"]))
                    .or_else(|| crate::yaml_string(crate::yaml_lookup(step, &["role"])))
                    .unwrap_or_default();
                let role_id = role_id.trim().to_string();
                if role_id.is_empty() {
                    blockers.push(format!("missing_flow_step_role:{flow_id}:{index}"));
                    return None;
                }
                if !known_roles.contains(role_id.as_str()) {
                    blockers.push(format!("unknown_flow_step:{flow_id}:{role_id}"));
                }
                let lifecycle_hook_templates = crate::yaml_string_list(crate::yaml_lookup(
                    step,
                    &["lifecycle_hook_templates"],
                ));
                validate_lifecycle_hook_refs(
                    &format!("flow_step:{flow_id}:{index}"),
                    &lifecycle_hook_templates,
                    configured_hook_templates,
                    blockers,
                );
                let requires_user_approval =
                    crate::yaml_bool(crate::yaml_lookup(step, &["requires_user_approval"]), false);
                validate_approval_policy(flow_id, index, step, requires_user_approval, blockers);
                Some(serde_json::json!({
                    "step_id": crate::yaml_string(crate::yaml_lookup(step, &["step_id"]))
                        .unwrap_or_else(|| format!("{flow_id}-{index}")),
                    "order": index,
                    "role_id": role_id,
                    "runtime_role": crate::yaml_string(crate::yaml_lookup(step, &["runtime_role"])),
                    "task_class": crate::yaml_string(crate::yaml_lookup(step, &["task_class"])),
                    "packet_template_kind": crate::yaml_string(
                        crate::yaml_lookup(step, &["packet_template_kind"])
                    ),
                    "closure_class": crate::yaml_string(
                        crate::yaml_lookup(step, &["closure_class"])
                    ),
                    "stage": crate::yaml_string(crate::yaml_lookup(step, &["stage"])),
                    "completion_blocker": crate::yaml_string(
                        crate::yaml_lookup(step, &["completion_blocker"])
                    ),
                    "inclusion_rule": crate::yaml_string(
                        crate::yaml_lookup(step, &["inclusion_rule"])
                    ),
                    "command_template": yaml_field_json(step, "command_template"),
                    "command_ref": yaml_field_json(step, "command_ref"),
                    "lifecycle_hook_templates": lifecycle_hook_templates,
                    "carrier_constraints": yaml_field_json(step, "carrier_constraints"),
                    "model_profile_constraints": yaml_field_json(step, "model_profile_constraints"),
                    "proof_gates": yaml_field_json(step, "proof_gates"),
                    "resume_transitions": yaml_field_json(step, "resume_transitions"),
                    "rework_transitions": yaml_field_json(step, "rework_transitions"),
                    "adapter_projection": yaml_field_json(step, "adapter_projection"),
                    "approval_policy": yaml_field_json(step, "approval_policy"),
                    "requires_user_approval": requires_user_approval,
                }))
            }
            _ => {
                blockers.push(format!("invalid_flow_step:{flow_id}:{index}"));
                None
            }
        })
        .collect()
}

fn default_dev_team_sequence(
    flows: &[serde_json::Value],
    configured_default_flow_id: Option<&str>,
) -> Vec<String> {
    let selected_flow = configured_default_flow_id
        .and_then(|flow_id| {
            flows.iter().find(|flow| {
                flow["enabled"].as_bool().unwrap_or(false)
                    && flow["flow_id"].as_str() == Some(flow_id)
            })
        })
        .or_else(|| {
            flows.iter().find(|flow| {
                flow["enabled"].as_bool().unwrap_or(false)
                    && flow["default"].as_bool().unwrap_or(false)
            })
        })
        .or_else(|| {
            flows
                .iter()
                .find(|flow| flow["enabled"].as_bool().unwrap_or(false))
        });
    selected_flow
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
        audit_team_flow_binding_policies, build_dev_team_readiness,
        build_taskflow_agent_system_snapshot, bundle_check_operator_contracts_consistency_error,
        consume_bundle_check_next_actions, consume_bundle_check_operator_envelope,
        consume_bundle_operator_contract_status, current_team_flow_binding_diagnostics,
        db_first_activation_snapshot_validation_error, fail_fast_with_timeout,
        lane_role_profile_for_context, merge_team_flow_binding_diagnostics,
        normalize_agent_system_max_parallel_agents, normalize_consume_bundle_blocker_codes,
        persisted_dev_team_readiness_projection, push_unique_string,
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
                install_layout: None,
                active_executable_path: "/tmp/project/bin/vida".to_string(),
                active_executable_fingerprint: "fingerprint".to_string(),
                active_executable_fingerprint_algorithm:
                    crate::runtime_consumption_surface::LAUNCHER_BINARY_FINGERPRINT_ALGORITHM
                        .to_string(),
                active_executable_fingerprint_semantics:
                    crate::runtime_consumption_surface::LAUNCHER_BINARY_FINGERPRINT_SEMANTICS
                        .to_string(),
                active_executable_fingerprint_source:
                    crate::runtime_consumption_surface::LAUNCHER_BINARY_FINGERPRINT_SOURCE
                        .to_string(),
                installed_binaries: Vec::new(),
                path_resolution: crate::runtime_consumption_surface::LauncherPathResolution {
                    command: "vida".to_string(),
                    resolved_path: Some("/tmp/project/bin/vida".to_string()),
                    expected_runtime_bin_dir: None,
                    expected_runtime_bin_on_path: false,
                    active_executable_on_path: true,
                    status: "pass".to_string(),
                },
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
    fn lane_role_profiles_are_minimal_and_omit_carrier_runtime_details() {
        let activation_bundle = serde_json::json!({
            "lane_work_context_contract": {
                "omitted_sections": ["carrier_selection", "pricing", "boot_diagnostics"]
            },
            "dev_team_readiness": {
                "source_paths": ["vida.config.yaml"],
                "roles": [
                    {
                        "role_id": "writer_lane",
                        "runtime_role": "writer",
                        "task_classes": ["implementation"],
                        "packet_template_kind": "delivery_task_packet",
                        "handoff": {"required_outputs": ["changed_files"]}
                    },
                    {
                        "role_id": "review_lane",
                        "runtime_role": "reviewer",
                        "task_classes": ["verification"],
                        "packet_template_kind": "coach_review_packet",
                        "handoff": {"required_outputs": ["verification_evidence"]}
                    }
                ]
            }
        });
        let writer = lane_role_profile_for_context(
            &activation_bundle,
            "writer_lane",
            "writer",
            "implementation",
        );
        let reviewer = lane_role_profile_for_context(
            &activation_bundle,
            "review_lane",
            "reviewer",
            "verification",
        );
        assert_eq!(writer["status"], "configured");
        assert_eq!(
            writer["handoff"]["required_outputs"],
            serde_json::json!(["changed_files"])
        );
        assert_ne!(
            writer["role_specific_fields"],
            reviewer["role_specific_fields"]
        );
        let rendered = serde_json::to_string(&writer).expect("profile should serialize");
        assert!(!rendered.contains("selected_model"));
        assert!(!rendered.contains("selected_carrier"));
        assert!(!rendered.contains("normalized_cost"));
        assert_eq!(writer["omitted_sections"][1], "pricing");
        assert_eq!(writer["context_budget"]["max_tokens"], 512);
    }

    #[test]
    fn agent_system_snapshot_prefers_carrier_runtime_fields_without_legacy_aliases() {
        let activation_bundle = serde_json::json!({
            "team_flow_authority": authority_projection_fixture(),
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

        let snapshot = build_taskflow_agent_system_snapshot(
            "target/vida-tests/missing-carrier-runtime-only-config.yaml",
            &activation_bundle,
        );
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
    fn build_dev_team_readiness_projects_persisted_roles_and_sequence() {
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
      default_model: gpt-5.5
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
      default_model: gpt-5.5
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
      allow_parallel_handoffs: true
      fail_closed_on_missing_step_contract: true
      steps: [developer, coach]
"#,
        )
        .expect("config should write");
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"]
                        },
                        {
                            "role_id": "middle",
                            "model": "gpt-5.5",
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
        assert_eq!(readiness["blockers"], serde_json::json!([]));
        assert_eq!(readiness["configured"], true);
        assert_eq!(readiness["default_flow_id"], authority_fixture_flow_id());
        let default_flow = readiness["flows"]
            .as_array()
            .expect("persisted flows should project")
            .iter()
            .find(|flow| flow["flow_id"].as_str() == Some(authority_fixture_flow_id()))
            .expect("persisted default flow should project");
        let expected_sequence = default_flow["steps"].clone();
        assert_eq!(readiness["sequence"], expected_sequence);
        assert_eq!(
            default_flow["ordered_steps"]
                .as_array()
                .expect("ordered steps should project")
                .len(),
            default_flow["steps"]
                .as_array()
                .expect("steps should project")
                .len()
        );
        assert_eq!(
            readiness["source_paths"][0],
            config_path.to_string_lossy().to_string()
        );
        assert_eq!(readiness["source_paths"].as_array().map(Vec::len), Some(1));
    }

    fn authority_fixture_flow_id() -> &'static str {
        static FLOW_ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        FLOW_ID
            .get_or_init(|| {
                crate::team_flow_authority_projection::test_support::canonical_compiled_bundle()
                    ["team_flow_authority"]["selected_config"]["authority_selection"]
                    ["default_flow_id"]
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .map(ToOwned::to_owned)
                    .expect("canonical authority projection must provide default flow id")
            })
            .as_str()
    }

    fn authority_projection_fixture() -> serde_json::Value {
        crate::team_flow_authority_projection::test_support::canonical_compiled_bundle()
            ["team_flow_authority"]
            .clone()
    }

    fn readiness_config_fixture(path: &std::path::Path) {
        let flow_id = authority_fixture_flow_id();
        std::fs::write(
            path,
            format!(
                r#"
dev_team:
  enabled: true
  default_flow_id: {flow_id}
  work_item_flow_bindings:
    defect: {flow_id}
  roles:
    developer:
      runtime_role: worker
      task_classes: [implementation]
      default_carrier: junior
      handoff:
        next_role: terminal_closure
        required_outputs: [changed_files]
        fail_closed_on_missing_handoff: true
  flows:
    {flow_id}:
      enabled: true
      sequential: true
      steps: [developer]
"#,
                flow_id = flow_id,
            ),
        )
        .expect("readiness config should write");
    }

    fn readiness_activation_bundle(authority: Option<serde_json::Value>) -> serde_json::Value {
        let mut bundle = serde_json::json!({
            "agent_system": {
                "mode": "native",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 1,
                "pricing": {}
            },
            "autonomous_execution": {"enabled": true},
            "carrier_runtime": {
                "roles": [{
                    "role_id": "junior",
                    "model": "configured-model",
                    "model_provider": "configured-provider",
                    "model_reasoning_effort": "configured-effort",
                    "normalized_cost_units": 1,
                    "rate": 1,
                    "runtime_roles": ["worker"],
                    "task_classes": ["implementation"]
                }],
                "model_selection": {},
                "worker_strategy": {},
                "dispatch_aliases": {}
            }
        });
        if let Some(authority) = authority {
            bundle["team_flow_authority"] = authority;
        }
        bundle
    }

    fn assert_team_flow_readiness_fails_closed(readiness: &serde_json::Value) {
        assert_eq!(readiness["status"], "blocked");
        assert_eq!(readiness["configured"], false);
        assert_eq!(readiness["enabled"], serde_json::Value::Null);
        assert_eq!(readiness["default_flow_id"], serde_json::Value::Null);
        assert_eq!(
            readiness["orchestrator_command_contract"],
            serde_json::Value::Null
        );
        assert_eq!(readiness["work_item_flow_bindings"], serde_json::json!({}));
        assert_eq!(readiness["roles"], serde_json::json!([]));
        assert_eq!(readiness["sequence"], serde_json::json!([]));
        assert_eq!(readiness["flows"], serde_json::json!([]));
        assert_eq!(readiness["team_flow_authority_status"], "blocked");
        assert_eq!(readiness["zombie_d_gate"], serde_json::Value::Null);
        assert!(readiness["blockers"][0]
            .as_str()
            .is_some_and(|blocker| blocker.starts_with("team_flow_authority_invalid:")));
    }

    fn assert_team_flow_readiness_unavailable(readiness: &serde_json::Value) {
        assert_eq!(readiness["status"], "unavailable");
        assert_eq!(readiness["configured"], false);
        assert_eq!(readiness["team_flow_authority_status"], "unavailable");
        assert_eq!(readiness["roles"], serde_json::json!([]));
        assert_eq!(readiness["sequence"], serde_json::json!([]));
        assert_eq!(readiness["flows"], serde_json::json!([]));
        assert!(readiness["blockers"][0]
            .as_str()
            .is_some_and(|blocker| blocker.starts_with("team_flow_authority_unavailable:")));
    }

    #[test]
    fn readiness_snapshot_carries_authoritative_team_flow_and_preserves_binding() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        readiness_config_fixture(&config_path);
        let authority = authority_projection_fixture();
        let snapshot = build_taskflow_agent_system_snapshot(
            config_path.to_str().expect("config path should be valid"),
            &readiness_activation_bundle(Some(authority.clone())),
        );
        let readiness = &snapshot["dev_team_readiness"];
        assert_eq!(readiness["team_flow_authority"], authority);
        assert_eq!(readiness["team_flow_authority_status"], "ready");
        assert_eq!(
            readiness["work_item_flow_bindings"],
            authority["resolved_all_flow_payload"]["work_item_flow_bindings"]
        );
        assert_eq!(
            readiness["work_item_flow_bindings"]["defect"],
            authority_fixture_flow_id()
        );
        let persisted_flow = authority["resolved_all_flow_payload"]["flows"]
            .as_array()
            .expect("persisted flows should exist")
            .iter()
            .find(|flow| flow["flow_id"].as_str() == Some(authority_fixture_flow_id()))
            .expect("default persisted flow should exist");
        let expected_sequence_len = persisted_flow["lanes"]
            .as_array()
            .expect("default persisted flow lanes should exist")
            .len();
        assert_eq!(
            crate::dev_team_sequence_contract::dev_team_sequence_for_flow_id(
                readiness,
                authority_fixture_flow_id()
            )
            .len(),
            expected_sequence_len
        );
    }

    #[test]
    fn readiness_projects_persisted_authority_after_raw_config_mutation_and_delete() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(&config_path, "dev_team: {enabled: false}\n")
            .expect("diagnostic config should write");
        let authority = authority_projection_fixture();
        let activation_bundle = serde_json::json!({"team_flow_authority": authority});
        let persisted_enabled = activation_bundle["team_flow_authority"]["selected_config"]
            ["team_flow_enabled"]
            .as_bool()
            .expect("canonical authority must persist TeamFlow enablement");
        let baseline = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &activation_bundle,
        );
        assert_eq!(baseline["enabled"], persisted_enabled);
        assert_eq!(
            baseline["work_item_flow_bindings"],
            activation_bundle["team_flow_authority"]["resolved_all_flow_payload"]
                ["work_item_flow_bindings"]
        );
        assert_eq!(
            baseline["status"],
            if persisted_enabled {
                "ready"
            } else {
                "disabled"
            }
        );
        let baseline_status = baseline["status"].clone();
        let baseline_enabled = baseline["enabled"].clone();
        let baseline_roles = baseline["roles"].clone();
        let baseline_flows = baseline["flows"].clone();
        let baseline_bindings = baseline["work_item_flow_bindings"].clone();
        let baseline_sequence = baseline["sequence"].clone();
        std::fs::write(&config_path, "dev_team: [invalid\n").expect("mutated config should write");
        let mutated = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &activation_bundle,
        );
        assert_eq!(mutated["status"], baseline_status);
        assert_eq!(mutated["enabled"], baseline_enabled);
        assert_eq!(mutated["roles"], baseline_roles);
        assert_eq!(mutated["flows"], baseline_flows);
        assert_eq!(mutated["work_item_flow_bindings"], baseline_bindings);
        assert_eq!(mutated["sequence"], baseline_sequence);
        std::fs::remove_file(&config_path).expect("diagnostic config should delete");
        let deleted = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &activation_bundle,
        );
        assert_eq!(deleted["status"], baseline["status"]);
        assert_eq!(deleted["enabled"], baseline["enabled"]);
        assert_eq!(deleted["roles"], baseline["roles"]);
        assert_eq!(deleted["flows"], baseline["flows"]);
        assert_eq!(
            deleted["work_item_flow_bindings"],
            baseline["work_item_flow_bindings"]
        );
        assert_eq!(deleted["sequence"], baseline["sequence"]);
    }

    #[test]
    fn readiness_projection_keeps_overlapping_policy_bindings_display_only() {
        let authority = serde_json::json!({
            "resolved_all_flow_payload": {
                "work_item_flow_bindings": {"defect": "flow-b"},
                "flows": [
                    {
                        "flow_id": "flow-a",
                        "flow_policy": {
                            "enabled": true,
                            "work_item_bindings": ["defect"]
                        },
                        "lanes": [{"node_id": "lane-a"}]
                    },
                    {
                        "flow_id": "flow-b",
                        "flow_policy": {
                            "enabled": true,
                            "work_item_bindings": ["defect"]
                        },
                        "lanes": [{"node_id": "lane-b"}]
                    }
                ]
            }
        });
        let mut blockers = Vec::new();
        let (_, flows, bindings) =
            persisted_dev_team_readiness_projection(&authority, &mut blockers);
        assert_eq!(bindings, serde_json::json!({"defect": "flow-b"}));
        assert!(blockers
            .iter()
            .all(|blocker| !blocker.starts_with("work_item_flow_binding_conflict:")));
        assert!(flows.iter().all(|flow| {
            flow["flow_policy"]["work_item_bindings"] == serde_json::json!(["defect"])
        }));
    }

    #[test]
    fn readiness_non_default_flow_preserves_persisted_ordered_steps_alias() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        let authority = authority_projection_fixture();
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({"team_flow_authority": authority}),
        );
        let default_flow_id = readiness["default_flow_id"]
            .as_str()
            .expect("default flow id");
        let flow = readiness["flows"]
            .as_array()
            .expect("readiness flows")
            .iter()
            .find(|flow| {
                flow["flow_id"].as_str() != Some(default_flow_id)
                    && flow["flow_policy"]["enabled"].as_bool() == Some(true)
            })
            .expect("canonical authority should persist an enabled non-default flow");
        let flow_id = flow["flow_id"].as_str().expect("non-default flow id");
        let ordered_steps = flow["ordered_steps"]
            .as_array()
            .expect("ordered_steps alias");
        let steps = flow["steps"].as_array().expect("steps alias");
        assert_eq!(ordered_steps.len(), steps.len());
        assert!(ordered_steps
            .iter()
            .zip(steps)
            .all(|(step, role_id)| { step["role_id"].as_str() == role_id.as_str() }));
        let sequence = crate::dev_team_sequence_contract::dev_team_sequence_for_flow_id(
            &serde_json::json!({"team_flow_authority": readiness["team_flow_authority"]}),
            flow_id,
        );
        assert_eq!(
            sequence
                .iter()
                .map(|step| step.role_label.as_str())
                .collect::<Vec<_>>(),
            steps
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn readiness_snapshot_without_authority_fails_closed_for_team_flow_selection() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        readiness_config_fixture(&config_path);
        let snapshot = build_taskflow_agent_system_snapshot(
            config_path.to_str().expect("config path should be valid"),
            &readiness_activation_bundle(None),
        );
        let readiness = &snapshot["dev_team_readiness"];
        assert_team_flow_readiness_unavailable(readiness);
        assert_eq!(readiness["team_flow_authority"], serde_json::Value::Null);

        let mut invalid_bundle = readiness_activation_bundle(Some(authority_projection_fixture()));
        invalid_bundle["team_flow_authority"]["selected_config"] = serde_json::Value::Null;
        let invalid_snapshot = build_taskflow_agent_system_snapshot(
            config_path.to_str().expect("config path should be valid"),
            &invalid_bundle,
        );
        let invalid_readiness = &invalid_snapshot["dev_team_readiness"];
        assert_team_flow_readiness_fails_closed(invalid_readiness);
        assert!(invalid_readiness["team_flow_authority"].is_object());
    }

    #[test]
    fn development_flow_catalog_projects_persisted_default_and_step_schema() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(
            &config_path,
            r#"
agent_extensions:
  enabled_hook_templates:
    - command_timing_summary
dev_team:
  enabled: true
  default_flow_id: investigation_flow
  roles:
    analyst:
      runtime_role: business_analyst
      task_classes: [specification]
      default_carrier: middle
      handoff:
        next_role: developer
        required_outputs: [brief]
    developer:
      runtime_role: worker
      task_classes: [implementation]
      packet_template_kind: delivery_task_packet
      closure_class: implementation
      stage: execution
      completion_blocker: pending_implementation_evidence
      inclusion_rule: always
      default_carrier: junior
      handoff:
        next_role: tester
        required_outputs: [changed_files]
    tester:
      runtime_role: verifier
      task_classes: [verification]
      default_carrier: senior
      handoff:
        next_role: terminal_closure
        required_outputs: [verification_evidence]
  flows:
    quick_fix:
      enabled: true
      steps: [developer]
    investigation_flow:
      enabled: true
      flow_class: development
      work_item_bindings: [epic, defect]
      sequential: true
      allow_parallel_handoffs: true
      lifecycle_hook_templates: [command_timing_summary]
      proof_gates:
        required_commands: [cargo test -p vida development_flow_catalog]
      resume_transitions:
        blocked: analyst
      rework_transitions:
        coach_rework: developer
      adapter_projection:
        host_agent_bridge_contract: required
      steps:
        - step_id: analysis
          role_id: analyst
          runtime_role: business_analyst
          task_class: specification
          packet_template_kind: delivery_task_packet
          closure_class: law
          stage: design_gate
          completion_blocker: pending_specification_evidence
          inclusion_rule: when_design_gate
          command_ref: agent-init-business-analyst
          lifecycle_hook_templates: [command_timing_summary]
          carrier_constraints:
            allowed_carriers: [middle]
          model_profile_constraints:
            reasoning_floor: medium
          proof_gates:
            required_outputs: [brief]
          resume_transitions:
            approved: developer
          rework_transitions:
            rework: analyst
          adapter_projection:
            adapter_kind: codex_host_tools
          requires_user_approval: true
          approval_policy:
            mode: user_review_required
        - role_id: developer
"#,
        )
        .expect("config should write");

        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"]
                        },
                        {
                            "role_id": "middle",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "medium",
                            "normalized_cost_units": 4,
                            "rate": 4,
                            "runtime_roles": ["business_analyst"],
                            "task_classes": ["specification"]
                        },
                        {
                            "role_id": "senior",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "high",
                            "normalized_cost_units": 16,
                            "rate": 16,
                            "runtime_roles": ["verifier"],
                            "task_classes": ["verification"]
                        }
                    ]
                }
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["default_flow_id"], authority_fixture_flow_id());
        let flow = readiness["flows"]
            .as_array()
            .expect("flows should project")
            .iter()
            .find(|flow| flow["flow_id"].as_str() == Some(authority_fixture_flow_id()))
            .expect("persisted default flow should project");
        assert_eq!(flow["default"], true);
        assert_eq!(readiness["sequence"], flow["steps"]);
        assert_eq!(
            flow["work_item_bindings"],
            flow["flow_policy"]["work_item_bindings"]
        );
        assert_eq!(
            flow["lifecycle_hook_templates"],
            flow["flow_policy"]["lifecycle_hook_templates"]
        );
        assert_eq!(flow["proof_gates"], flow["flow_policy"]["proof_gates"]);
        assert_eq!(
            flow["adapter_projection"],
            flow["flow_policy"]["adapter_projection"]
        );
        assert_eq!(
            flow["ordered_steps"]
                .as_array()
                .expect("ordered steps should project")
                .iter()
                .map(|step| step["role_id"].clone())
                .collect::<Vec<_>>(),
            flow["steps"]
                .as_array()
                .expect("steps should project")
                .clone()
        );
    }

    #[test]
    fn dev_team_template_keeps_lane_sequence_sequential_while_enabling_parallel_handoffs() {
        let template_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/framework/templates/vida.config.yaml.template");
        let template = std::fs::read_to_string(&template_path)
            .expect("framework vida.config.yaml template should be readable");
        let config: serde_yaml::Value =
            serde_yaml::from_str(&template).expect("framework config template should parse");
        let flows = crate::yaml_lookup(&config, &["dev_team", "flows"])
            .and_then(serde_yaml::Value::as_mapping)
            .expect("dev_team flows should exist in the framework template");

        for flow_id in [
            "defect_repair_verified",
            "pr_repair_verified",
            "runtime_defect_remediation",
            "architecture_design",
            "service_tui_orchestration",
            "hook_enabled_internal_agent_development",
            "default_delivery",
        ] {
            let flow = flows
                .get(serde_yaml::Value::String(flow_id.to_string()))
                .unwrap_or_else(|| {
                    panic!("expected dev_team flow {flow_id} in framework template")
                });
            assert_eq!(
                crate::yaml_bool(crate::yaml_lookup(flow, &["sequential"]), false),
                true,
                "{flow_id} must keep per-task lane order sequential"
            );
            assert_eq!(
                crate::yaml_bool(
                    crate::yaml_lookup(flow, &["allow_parallel_handoffs"]),
                    false
                ),
                true,
                "{flow_id} must permit cross-task parallel handoffs when execution semantics admit disjoint work"
            );
        }
    }

    #[test]
    fn consumer_projects_persisted_hook_refs_without_source_schema_rejection() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(
            &config_path,
            r#"
agent_extensions:
  enabled_hook_templates:
    - approval_wait
dev_team:
  enabled: true
  default_flow_id: invalid_flow
  roles:
    developer:
      runtime_role: worker
      task_classes: [implementation]
      default_carrier: junior
      handoff:
        next_role: terminal_closure
        required_outputs: [changed_files]
  flows:
    invalid_flow:
      enabled: true
      unsupported_flow_knob: true
      lifecycle_hook_templates: [hardcoded_cli_hook]
      steps:
        - role_id: developer
          unsupported_step_knob: true
          lifecycle_hook_templates: [hardcoded_step_hook]
          requires_user_approval: true
          approval_policy:
            mode: auto_approve
"#,
        )
        .expect("config should write");

        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"]
                        }
                    ]
                }
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["blockers"], serde_json::json!([]));
        let persisted_flows = readiness["team_flow_authority"]["resolved_all_flow_payload"]
            ["flows"]
            .as_array()
            .expect("persisted flows should exist");
        for persisted_flow in persisted_flows {
            let flow_id = persisted_flow["flow_id"].as_str().expect("flow id");
            let projected = readiness["flows"]
                .as_array()
                .expect("projected flows should exist")
                .iter()
                .find(|flow| flow["flow_id"].as_str() == Some(flow_id))
                .expect("persisted flow should project");
            assert_eq!(
                projected["lifecycle_hook_templates"],
                persisted_flow["flow_policy"]["lifecycle_hook_templates"]
            );
        }
    }

    #[test]
    fn build_dev_team_readiness_projects_persisted_model_metadata() {
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
      allow_parallel_handoffs: true
      steps: [developer]
        "#,
        )
        .expect("config should write");
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"],
                            "model_profiles": {
                                "codex_gpt54_low_write": {
                                    "provider": "openai",
                                    "model_ref": "gpt-5.5",
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
        let role = readiness["roles"]
            .as_array()
            .expect("persisted roles should project")
            .first()
            .expect("persisted role should project");
        assert!(role["selected_model"]["model_profile_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        assert!(role["selected_model"]["model_ref"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(
            role["selected_model"]["selected_rate"],
            serde_json::Value::Null
        );
        assert_eq!(role["selected_model"]["pricing"], serde_json::Value::Null);
        assert_eq!(
            role["selected_model"]["pricing_freshness_status"],
            "persisted"
        );
    }

    #[test]
    fn build_dev_team_readiness_uses_carrier_profile_without_role_default_model() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(
            &config_path,
            r#"
dev_team:
  enabled: true
  roles:
    test_author:
      runtime_role: worker
      task_classes: [test_authoring, regression_test]
      default_carrier: middle
      cost_policy:
        budget_units: 4
      handoff:
        next_role: coach
        required_outputs: [failing_regression_test]
  flows:
    default_delivery:
      enabled: true
      sequential: true
      allow_parallel_handoffs: true
      steps: [test_author]
        "#,
        )
        .expect("config should write");

        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "middle",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "medium",
                            "normalized_cost_units": 4,
                            "rate": 4,
                            "runtime_roles": ["worker", "coach"],
                            "task_classes": ["test_authoring", "regression_test", "coach"],
                            "default_model_profile": "codex_gpt55_medium_write",
                            "model_profiles": {
                                "codex_gpt55_medium_write": {
                                    "provider": "openai",
                                    "model_ref": "gpt-5.5",
                                    "reasoning_effort": "medium",
                                    "normalized_cost_units": 4
                                }
                            }
                        }
                    ]
                }
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["blockers"], serde_json::json!([]));
        let role = readiness["roles"]
            .as_array()
            .expect("persisted roles should project")
            .first()
            .expect("persisted role should project");
        assert_eq!(role["default_model"], role["selected_model"]["model_ref"]);
        assert_eq!(
            role["default_model_reasoning_effort"],
            role["selected_model"]["model_reasoning_effort"]
        );
    }

    #[test]
    fn build_dev_team_readiness_uses_persisted_price_freshness_status() {
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
      allow_parallel_handoffs: true
      steps: [developer]
        "#,
        )
        .expect("config should write");
        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
                "carrier_runtime": {
                    "roles": [
                        {
                            "role_id": "junior",
                            "model": "gpt-5.5",
                            "model_provider": "openai",
                            "model_reasoning_effort": "low",
                            "normalized_cost_units": 1,
                            "rate": 1,
                            "runtime_roles": ["worker"],
                            "task_classes": ["implementation"],
                            "model_profiles": {
                                "codex_gpt54_low_write": {
                                    "provider": "openai",
                                    "model_ref": "gpt-5.5",
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
        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["blockers"], serde_json::json!([]));
        assert!(readiness["roles"]
            .as_array()
            .expect("persisted roles should be array")
            .iter()
            .all(|role| role["selected_model"]["pricing_freshness_status"] == "persisted"));
    }

    #[test]
    fn build_dev_team_readiness_ignores_missing_raw_config_when_authority_is_ready() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        std::fs::write(&config_path, "project:\n  id: demo\n").expect("config should write");
        std::fs::remove_file(&config_path).expect("diagnostic config should delete");

        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["configured"], true);
        assert_eq!(readiness["blockers"], serde_json::json!([]));
        assert_eq!(
            readiness["source_paths"][0],
            config_path.to_string_lossy().to_string()
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_dev_team_readiness_ignores_symlinked_raw_config_path() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        let target_path = harness.path().join("actual-config.yaml");
        std::fs::write(&target_path, "project:\n  id: demo\n").expect("target config should write");
        std::os::unix::fs::symlink(&target_path, &config_path)
            .expect("symlinked config should be created");

        let readiness = build_dev_team_readiness(
            config_path.to_str().expect("config path should be valid"),
            &serde_json::json!({
                "team_flow_authority": authority_projection_fixture(),
            }),
        );

        assert_eq!(readiness["status"], "ready");
        assert_eq!(readiness["configured"], true);
        assert_eq!(readiness["blockers"], serde_json::json!([]));
        assert_eq!(
            readiness["source_paths"][0],
            config_path.to_string_lossy().to_string()
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
                        "model": "gpt-5.5",
                        "model_provider": "openai",
                        "model_reasoning_effort": "medium",
                        "plan_mode_reasoning_effort": "high",
                        "sandbox_mode": "workspace-write",
                        "default_model_profile": "codex_gpt54_medium_write",
                        "model_profiles": {
                            "codex_gpt54_medium_write": {
                                "profile_id": "codex_gpt54_medium_write",
                                "model_ref": "gpt-5.5",
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
        assert!(actions.iter().any(|action| {
            action.contains("rerun `vida taskflow consume bundle check --json` only to verify")
        }));
        assert!(
            !actions.iter().any(|action| action
                .contains("then `vida taskflow consume bundle check --json` to materialize")),
            "bundle check must not recommend itself as the command that materializes missing retrieval trust evidence"
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
    fn consume_bundle_check_operator_envelope_projects_top_level_blocked_status() {
        let check = crate::TaskflowConsumeBundleCheck {
            ok: false,
            blockers: vec!["team_flow_authority_disabled".to_string()],
            root_artifact_id: "root-artifact".to_string(),
            artifact_count: 1,
            boot_classification: "blocked".to_string(),
            migration_state: "not_required".to_string(),
            activation_status: "blocked".to_string(),
        };
        let operator_output =
            crate::release1_operator_output::build_release1_operator_output_payload(
                "vida taskflow consume bundle check",
                vec!["team_flow_authority_disabled".to_string()],
                vec!["restore TeamFlow binding".to_string()],
                serde_json::json!({"root_artifact_id": "root-artifact"}),
                serde_json::json!({}),
            )
            .expect("operator output should build");
        let rendered = consume_bundle_check_operator_envelope(
            &check,
            &serde_json::json!({"status": "blocked"}),
            &serde_json::json!({"status": "blocked"}),
            &["team_flow_authority_disabled".to_string()],
            &operator_output,
            "bundle-check.json",
        );
        assert_eq!(rendered["surface"], "vida taskflow consume bundle check");
        assert_eq!(rendered["status"], "blocked");
        assert_eq!(rendered["status"], rendered["operator_contracts"]["status"]);
        assert_eq!(
            rendered["blocker_codes"],
            rendered["operator_contracts"]["blocker_codes"]
        );
    }

    #[test]
    fn team_flow_binding_diagnostics_merge_preserves_unrelated_blockers() {
        let cases = [
            (
                "disabled",
                serde_json::json!({
                    "blockers": ["dev_team_disabled"],
                    "context": {"selected_flow": "default-flow"},
                }),
                "dev_team_disabled",
            ),
            (
                "unknown",
                serde_json::json!({
                    "blockers": ["team_flow_authority_invalid"],
                    "context": {
                        "selected_flow": "default-flow",
                        "bound_work_item_flows": {"defect": "missing-flow"},
                    },
                }),
                "team_flow_authority_invalid",
            ),
        ];
        for (mutation, diagnostics, expected_blocker) in cases {
            let mut blockers = vec!["protocol_binding_not_runtime_ready".to_string()];
            merge_team_flow_binding_diagnostics(&mut blockers, &diagnostics);
            assert!(
                blockers
                    .iter()
                    .any(|blocker| blocker == "protocol_binding_not_runtime_ready"),
                "{mutation} mutation must preserve unrelated blockers"
            );
            assert!(
                blockers.iter().any(|blocker| blocker == expected_blocker),
                "{mutation} mutation must add TeamFlow blocker"
            );
            assert!(diagnostics["context"].is_object());
        }
    }

    #[test]
    fn persisted_team_flow_source_wins_over_cwd_for_disabled_and_unknown_mutations() {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime should initialize");
        let cwd_harness = TempStateHarness::new().expect("cwd harness should initialize");
        let persisted_harness =
            TempStateHarness::new().expect("persisted harness should initialize");
        for root in [cwd_harness.path(), persisted_harness.path()] {
            let _cwd = crate::test_cli_support::guard_current_dir(root);
            assert_eq!(
                runtime.block_on(crate::run(crate::test_cli_support::cli(&["init"]))),
                std::process::ExitCode::SUCCESS
            );
        }
        let persisted_config_path = persisted_harness.path().join("vida.config.yaml");
        let original_config = std::fs::read_to_string(&persisted_config_path)
            .expect("persisted config should be readable");
        let fallback_root = cwd_harness
            .path()
            .to_str()
            .expect("cwd root should be valid UTF-8");
        let persisted_path = persisted_config_path
            .to_str()
            .expect("persisted config path should be valid UTF-8");
        for mutation in ["disabled", "unknown"] {
            let mut config: serde_yaml::Value =
                serde_yaml::from_str(&original_config).expect("persisted config should parse");
            if mutation == "disabled" {
                config["dev_team"]["enabled"] = serde_yaml::Value::Bool(false);
            } else {
                config["dev_team"]["work_item_flow_bindings"]["defect"] =
                    serde_yaml::Value::String("missing-flow".to_string());
            }
            std::fs::write(
                &persisted_config_path,
                serde_yaml::to_string(&config).expect("mutated config should serialize"),
            )
            .expect("mutated persisted config should write");
            let diagnostics =
                current_team_flow_binding_diagnostics(fallback_root, Some(persisted_path));
            assert_eq!(
                diagnostics["context"]["source"],
                "persisted_launcher_snapshot"
            );
            assert_eq!(diagnostics["context"]["config_path"], persisted_path);
            assert!(diagnostics["context"]["selected_flow"].is_string());
            if mutation == "disabled" {
                assert!(diagnostics["blockers"]
                    .as_array()
                    .is_some_and(|blockers| blockers
                        .iter()
                        .any(|value| value == "dev_team_disabled")));
            } else {
                assert!(diagnostics["blockers"].as_array().is_some_and(|blockers| {
                    blockers
                        .iter()
                        .any(|value| value == "team_flow_authority_invalid")
                }));
                assert_eq!(
                    diagnostics["context"]["bound_work_item_flows"]["defect"],
                    "missing-flow"
                );
            }
        }
        let fallback_diagnostics = current_team_flow_binding_diagnostics(fallback_root, None);
        assert_eq!(
            fallback_diagnostics["context"]["source"],
            "cwd_fallback_no_persisted_config"
        );
    }

    fn refresh_binding_audit_fixture_hashes(bundle: &mut serde_json::Value) {
        let authority = &mut bundle["team_flow_authority"];
        for flow in authority["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .expect("persisted flows")
        {
            let flow_id = flow["flow_id"].as_str().expect("flow id").to_string();
            let flow_policy = flow["flow_policy"].clone();
            let lanes = flow["lanes"].as_array().expect("flow lanes");
            let identity_hash =
                taskflow_authority::team_flow_transition::hash_json(&serde_json::json!({
                    "flow_id": flow_id,
                    "flow_policy": flow_policy,
                    "lanes": lanes,
                }));
            flow["flow_identity"]["id"] =
                serde_json::json!(format!("team-flow-flow:{identity_hash}"));
        }
        let payload = authority["resolved_all_flow_payload"].clone();
        let payload_hash = taskflow_authority::team_flow_transition::hash_json(&payload);
        authority["resolved_all_flow_payload_blake3"] = serde_json::json!(payload_hash.clone());
        authority["authority_source"]["payload_blake3"] = serde_json::json!(payload_hash);
    }

    #[test]
    fn persisted_team_flow_binding_policy_audit_table() {
        let baseline =
            crate::team_flow_authority_projection::test_support::canonical_compiled_bundle();
        let flow_id = baseline["team_flow_authority"]["resolved_all_flow_payload"]
            ["work_item_flow_bindings"]
            .as_object()
            .and_then(|bindings| bindings.values().next())
            .and_then(serde_json::Value::as_str)
            .expect("canonical binding flow")
            .to_string();
        let enabled_binding_count = baseline["team_flow_authority"]["resolved_all_flow_payload"]
            ["work_item_flow_bindings"]
            .as_object()
            .map(serde_json::Map::len)
            .expect("canonical bindings");
        let mut disabled = baseline.clone();
        disabled["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"] =
            serde_json::json!({"defect": flow_id.clone()});
        if let Some(flow) = disabled["team_flow_authority"]["resolved_all_flow_payload"]["flows"]
            .as_array_mut()
            .and_then(|flows| {
                flows
                    .iter_mut()
                    .find(|flow| flow["flow_id"].as_str() == Some(flow_id.as_str()))
            })
        {
            flow["flow_policy"]["enabled"] = serde_json::Value::Bool(false);
        }
        refresh_binding_audit_fixture_hashes(&mut disabled);
        let mut repeated = disabled.clone();
        repeated["team_flow_authority"]["resolved_all_flow_payload"]["work_item_flow_bindings"]
            ["runtime_defect"] = serde_json::json!(flow_id.clone());
        refresh_binding_audit_fixture_hashes(&mut repeated);
        let cases = [
            ("enabled", baseline, 0usize, enabled_binding_count),
            ("disabled", disabled, 1usize, 1usize),
            ("repeated", repeated, 1usize, 2usize),
        ];
        for (case, bundle, blocker_count, binding_count) in cases {
            let audit = audit_team_flow_binding_policies(&bundle);
            assert_eq!(
                audit["blockers"].as_array().map(Vec::len),
                Some(blocker_count),
                "{case} binding blocker count"
            );
            assert_eq!(
                audit["context"]["bindings"].as_array().map(Vec::len),
                Some(binding_count),
                "{case} binding context count"
            );
            if case == "enabled" {
                assert_eq!(audit["status"], "ready");
            } else {
                assert_eq!(audit["status"], "blocked");
                assert_eq!(
                    audit["blockers"],
                    serde_json::json!(["team_flow_authority_flow_policy_disabled"])
                );
                assert!(audit["context"]["bindings"]
                    .as_array()
                    .is_some_and(|bindings| bindings.iter().all(|binding| {
                        binding["flow_id"] == flow_id && binding["status"] == "blocked"
                    })));
            }
        }
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
        assert_eq!(seam["receipt_backed"], false);
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
            fail_fast_with_timeout(
                "opening authoritative state store",
                std::time::Duration::from_millis(1),
                async { std::future::pending::<Result<(), String>>().await },
            )
            .await
        });

        let error = result.expect_err("pending future should time out");
        assert!(error.contains("consume bundle check failed fast"));
        assert!(error.contains("opening authoritative state store"));
        assert!(error.contains("authoritative datastore lock"));
    }
}
