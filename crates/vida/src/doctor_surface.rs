use std::process::ExitCode;

pub(crate) async fn run_doctor(args: super::DoctorArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(super::state_store::default_state_dir);
    let render = args.render;
    let as_json = args.json;

    match super::StateStore::open_existing(state_dir).await {
        Ok(store) => {
            let storage_metadata = match store.storage_metadata_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("storage metadata: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let storage_metadata_display = format!(
                "{} state-v{} instruction-v{}",
                storage_metadata.backend,
                storage_metadata.state_schema_version,
                storage_metadata.instruction_schema_version
            );
            let state_spine = match store.state_spine_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("authoritative state spine: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let task_store = match store.task_store_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task store: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let run_graph = match store.run_graph_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("run graph: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let launcher_runtime_paths = match super::resolve_repo_root()
                .and_then(|project_root| super::doctor_launcher_summary_for_root(&project_root))
            {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("launcher/runtime paths: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let dependency_graph = match store.validate_task_graph().await {
                Ok(issues) if issues.is_empty() => issues,
                Ok(issues) => {
                    let first = issues.first().expect("issues is not empty");
                    eprintln!(
                        "dependency graph: failed ({} issue(s), first={} on {})",
                        issues.len(),
                        first.issue_type,
                        first.issue_id
                    );
                    return ExitCode::from(1);
                }
                Err(error) => {
                    eprintln!("dependency graph: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let boot_compatibility = match store.evaluate_boot_compatibility().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("boot compatibility: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_preflight = match store.evaluate_migration_preflight().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("migration preflight: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let migration_receipts = match store.migration_receipt_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("migration receipts: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_task_reconciliation = match store.latest_task_reconciliation_summary().await
            {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task reconciliation: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let task_reconciliation_rollup = match store.task_reconciliation_rollup().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("task reconciliation rollup: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let snapshot_bridge = match store.taskflow_snapshot_bridge_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("taskflow snapshot bridge: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let runtime_consumption = match super::runtime_consumption_summary(store.root()) {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("runtime consumption: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let protocol_binding = match store.protocol_binding_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("protocol binding: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_status = match store.latest_run_graph_status().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph status: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_recovery = match store.latest_run_graph_recovery_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph recovery: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_checkpoint =
                match store.latest_run_graph_checkpoint_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph checkpoint: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let latest_run_graph_gate = match store.latest_run_graph_gate_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("latest run graph gate: failed ({error})");
                    return ExitCode::from(1);
                }
            };
            let latest_run_graph_dispatch_receipt =
                match store.latest_run_graph_dispatch_receipt_summary().await {
                    Ok(summary) => summary,
                    Err(error) => {
                        eprintln!("latest run graph dispatch receipt: failed ({error})");
                        return ExitCode::from(1);
                    }
                };
            let effective_instruction_bundle = match store.active_instruction_root().await {
                Ok(root_artifact_id) => match store
                    .inspect_effective_instruction_bundle(&root_artifact_id)
                    .await
                {
                    Ok(bundle) => bundle,
                    Err(error) => {
                        eprintln!("effective instruction bundle: failed ({error})");
                        return ExitCode::from(1);
                    }
                },
                Err(error) => {
                    eprintln!("active instruction root: failed ({error})");
                    return ExitCode::from(1);
                }
            };

            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "surface": "vida doctor",
                        "storage_metadata": {
                            "engine": storage_metadata.engine,
                            "backend": storage_metadata.backend,
                            "namespace": storage_metadata.namespace,
                            "database": storage_metadata.database,
                            "state_schema_version": storage_metadata.state_schema_version,
                            "instruction_schema_version": storage_metadata.instruction_schema_version,
                        },
                        "state_spine": {
                            "state_schema_version": state_spine.state_schema_version,
                            "entity_surface_count": state_spine.entity_surface_count,
                            "authoritative_mutation_root": state_spine.authoritative_mutation_root,
                        },
                        "task_store": {
                            "total_count": task_store.total_count,
                            "open_count": task_store.open_count,
                            "in_progress_count": task_store.in_progress_count,
                            "closed_count": task_store.closed_count,
                            "epic_count": task_store.epic_count,
                            "ready_count": task_store.ready_count,
                        },
                        "run_graph": {
                            "execution_plan_count": run_graph.execution_plan_count,
                            "routed_run_count": run_graph.routed_run_count,
                            "governance_count": run_graph.governance_count,
                            "resumability_count": run_graph.resumability_count,
                            "reconciliation_count": run_graph.reconciliation_count,
                        },
                        "launcher_runtime_paths": launcher_runtime_paths,
                        "dependency_graph": {
                            "issue_count": dependency_graph.len(),
                        },
                        "boot_compatibility": {
                            "classification": boot_compatibility.classification,
                            "reasons": boot_compatibility.reasons,
                            "next_step": boot_compatibility.next_step,
                        },
                        "migration_preflight": {
                            "compatibility_classification": migration_preflight.compatibility_classification,
                            "migration_state": migration_preflight.migration_state,
                            "blockers": migration_preflight.blockers,
                            "source_version_tuple": migration_preflight.source_version_tuple,
                            "next_step": migration_preflight.next_step,
                        },
                        "migration_receipts": {
                            "compatibility_receipts": migration_receipts.compatibility_receipts,
                            "application_receipts": migration_receipts.application_receipts,
                            "verification_receipts": migration_receipts.verification_receipts,
                            "cutover_readiness_receipts": migration_receipts.cutover_readiness_receipts,
                            "rollback_notes": migration_receipts.rollback_notes,
                        },
                        "latest_task_reconciliation": latest_task_reconciliation,
                        "task_reconciliation_rollup": task_reconciliation_rollup,
                        "taskflow_snapshot_bridge": snapshot_bridge,
                        "runtime_consumption": runtime_consumption,
                        "protocol_binding": protocol_binding,
                        "latest_run_graph_status": latest_run_graph_status,
                        "latest_run_graph_delegation_gate": latest_run_graph_status.as_ref().map(|status| status.delegation_gate()),
                        "latest_run_graph_recovery": latest_run_graph_recovery,
                        "latest_run_graph_checkpoint": latest_run_graph_checkpoint,
                        "latest_run_graph_gate": latest_run_graph_gate,
                        "latest_run_graph_dispatch_receipt": latest_run_graph_dispatch_receipt,
                        "effective_instruction_bundle": {
                            "root_artifact_id": effective_instruction_bundle.root_artifact_id,
                            "mandatory_chain_order": effective_instruction_bundle.mandatory_chain_order,
                            "source_version_tuple": effective_instruction_bundle.source_version_tuple,
                            "receipt_id": effective_instruction_bundle.receipt_id,
                            "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
                        },
                        "storage_metadata_display": storage_metadata_display,
                    }))
                    .expect("doctor summary should render as json")
                );
                return ExitCode::SUCCESS;
            }

            super::print_surface_header(render, "vida doctor");
            super::print_surface_ok(render, "storage metadata", &storage_metadata_display);
            super::print_surface_ok(
                render,
                "authoritative state spine",
                &format!(
                    "state-v{}, {} entity surfaces, mutation root {}",
                    state_spine.state_schema_version,
                    state_spine.entity_surface_count,
                    state_spine.authoritative_mutation_root
                ),
            );
            super::print_surface_ok(render, "task store", &task_store.as_display());
            super::print_surface_ok(render, "run graph", &run_graph.as_display());
            super::print_surface_ok(
                render,
                "launcher/runtime paths",
                &format!(
                    "vida={}, project_root={}, taskflow_surface={}",
                    launcher_runtime_paths.vida,
                    launcher_runtime_paths.project_root,
                    launcher_runtime_paths.taskflow_surface
                ),
            );
            super::print_surface_ok(render, "dependency graph", "0 issues");
            super::print_surface_ok(
                render,
                "boot compatibility",
                &format!(
                    "{} ({})",
                    boot_compatibility.classification, boot_compatibility.next_step
                ),
            );
            super::print_surface_ok(
                render,
                "migration preflight",
                &format!(
                    "{} / {} ({})",
                    migration_preflight.compatibility_classification,
                    migration_preflight.migration_state,
                    migration_preflight.next_step
                ),
            );
            super::print_surface_ok(
                render,
                "migration receipts",
                &migration_receipts.as_display(),
            );
            match latest_task_reconciliation {
                Some(receipt) => {
                    super::print_surface_ok(render, "task reconciliation", &receipt.as_display());
                }
                None => {
                    super::print_surface_ok(render, "task reconciliation", "none");
                }
            }
            super::print_surface_ok(
                render,
                "task reconciliation rollup",
                &task_reconciliation_rollup.as_display(),
            );
            super::print_surface_ok(
                render,
                "taskflow snapshot bridge",
                &snapshot_bridge.as_display(),
            );
            super::print_surface_ok(
                render,
                "runtime consumption",
                &runtime_consumption.as_display(),
            );
            super::print_surface_ok(render, "protocol binding", &protocol_binding.as_display());
            match latest_run_graph_status {
                Some(status) => {
                    super::print_surface_ok(
                        render,
                        "latest run graph status",
                        &status.as_display(),
                    );
                    super::print_surface_ok(
                        render,
                        "latest run graph delegation gate",
                        &status.delegation_gate().as_display(),
                    );
                }
                None => {
                    super::print_surface_ok(render, "latest run graph status", "none");
                }
            }
            match latest_run_graph_recovery {
                Some(summary) => {
                    super::print_surface_ok(
                        render,
                        "latest run graph recovery",
                        &summary.as_display(),
                    );
                }
                None => {
                    super::print_surface_ok(render, "latest run graph recovery", "none");
                }
            }
            match latest_run_graph_checkpoint {
                Some(summary) => {
                    super::print_surface_ok(
                        render,
                        "latest run graph checkpoint",
                        &summary.as_display(),
                    );
                }
                None => {
                    super::print_surface_ok(render, "latest run graph checkpoint", "none");
                }
            }
            match latest_run_graph_gate {
                Some(summary) => {
                    super::print_surface_ok(render, "latest run graph gate", &summary.as_display());
                }
                None => {
                    super::print_surface_ok(render, "latest run graph gate", "none");
                }
            }
            super::print_surface_ok(
                render,
                "effective instruction bundle",
                &effective_instruction_bundle
                    .mandatory_chain_order
                    .join(" -> "),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
    }
}
