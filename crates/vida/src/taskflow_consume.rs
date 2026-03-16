use std::process::ExitCode;
use time::format_description::well_known::Rfc3339;

pub(crate) async fn run_taskflow_consume(args: &[String]) -> ExitCode {
    if let Some(exit) = super::taskflow_consume_bundle::run_taskflow_consume_bundle(args).await {
        return exit;
    }

    match args {
        [head] if head == "consume" => {
            super::print_taskflow_proxy_help(Some("consume"));
            ExitCode::SUCCESS
        }
        [head, flag] if head == "consume" && matches!(flag.as_str(), "--help" | "-h") => {
            super::print_taskflow_proxy_help(Some("consume"));
            ExitCode::SUCCESS
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "continue" => {
            let (
                as_json,
                requested_run_id,
                requested_dispatch_packet_path,
                requested_downstream_packet_path,
            ) = match super::taskflow_consume_resume::parse_taskflow_consume_continue_args(args) {
                Ok(parsed) => parsed,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            return super::taskflow_consume_resume::run_taskflow_consume_resume_command(
                super::taskflow_task_bridge::proxy_state_dir(),
                as_json,
                requested_run_id,
                requested_dispatch_packet_path,
                requested_downstream_packet_path,
                "vida taskflow consume continue",
                true,
            )
            .await;
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "advance" => {
            let (as_json, requested_run_id, max_rounds) =
                match super::taskflow_consume_resume::parse_taskflow_consume_advance_args(args) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            return super::taskflow_consume_resume::run_taskflow_consume_advance_command(
                super::taskflow_task_bridge::proxy_state_dir(),
                as_json,
                requested_run_id,
                max_rounds,
            )
            .await;
        }
        [head, subcommand, request @ ..] if head == "consume" && subcommand == "final" => {
            let as_json = request.iter().any(|arg| arg == "--json");
            let request_text = request
                .iter()
                .filter(|arg| arg.as_str() != "--json")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if request_text.is_empty() {
                eprintln!("Usage: vida taskflow consume final <request_text> [--json]");
                return ExitCode::from(2);
            }

            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match super::StateStore::open_existing(state_dir).await {
                Ok(store) => match super::build_taskflow_consume_bundle_payload(&store).await {
                    Ok(runtime_bundle) => {
                        let bundle_check = super::taskflow_consume_bundle_check(&runtime_bundle);
                        let (registry, check, readiness, proof, overview) =
                            super::build_docflow_runtime_evidence();
                        let docflow_verdict = super::build_docflow_runtime_verdict(
                            &registry, &check, &readiness, &proof,
                        );
                        let role_selection = match super::build_runtime_lane_selection_with_store(
                            &store,
                            &request_text,
                        )
                        .await
                        {
                            Ok(selection) => selection,
                            Err(error) => {
                                if as_json {
                                    let payload = super::TaskflowDirectConsumptionPayload {
                                        artifact_name: "taskflow_direct_runtime_consumption"
                                            .to_string(),
                                        artifact_type: "runtime_consumption".to_string(),
                                        generated_at: time::OffsetDateTime::now_utc()
                                            .format(&super::Rfc3339)
                                            .expect("rfc3339 timestamp should render"),
                                        closure_authority: "taskflow".to_string(),
                                        role_selection: super::blocking_lane_selection(
                                            &request_text,
                                            &error,
                                        ),
                                        request_text: request_text.clone(),
                                        direct_consumption_ready: false,
                                        runtime_bundle,
                                        bundle_check,
                                        docflow_activation:
                                            super::RuntimeConsumptionDocflowActivation {
                                                activated: true,
                                                runtime_family: "docflow".to_string(),
                                                owner_runtime: "taskflow".to_string(),
                                                evidence: serde_json::json!({
                                                    "overview": overview,
                                                    "registry": registry,
                                                    "check": check,
                                                    "readiness": readiness,
                                                    "proof": proof,
                                                }),
                                            },
                                        docflow_verdict,
                                        closure_admission:
                                            super::RuntimeConsumptionClosureAdmission {
                                                status: "block".to_string(),
                                                admitted: false,
                                                blockers: vec![
                                                    "unresolved_lane_selection".to_string()
                                                ],
                                                proof_surfaces: vec![
                                                    "vida taskflow consume bundle check"
                                                        .to_string(),
                                                ],
                                            },
                                        taskflow_handoff_plan: serde_json::json!({
                                            "status": "blocked",
                                            "handoff_ready": false,
                                            "reason": "unresolved_lane_selection",
                                        }),
                                        run_graph_bootstrap: serde_json::json!({
                                            "status": "blocked",
                                            "handoff_ready": false,
                                            "reason": "unresolved_lane_selection",
                                        }),
                                        dispatch_receipt: serde_json::json!({
                                            "status": "blocked",
                                            "reason": "unresolved_lane_selection",
                                        }),
                                    };
                                    if let Err(snapshot_error) =
                                        super::emit_taskflow_consume_final_json(&store, &payload)
                                    {
                                        eprintln!("{snapshot_error}");
                                    }
                                    return ExitCode::from(1);
                                }
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        };
                        let closure_admission = super::build_runtime_closure_admission(
                            &bundle_check,
                            &docflow_verdict,
                            &role_selection,
                        );
                        let taskflow_handoff_plan =
                            super::build_taskflow_handoff_plan(&role_selection);
                        let run_graph_bootstrap =
                            super::build_runtime_consumption_run_graph_bootstrap(
                                &store,
                                &role_selection,
                            )
                            .await;
                        let mut dispatch_receipt =
                            build_runtime_consumption_dispatch_receipt(
                                &role_selection,
                                &run_graph_bootstrap,
                            );
                        dispatch_receipt.dispatch_command =
                            super::runtime_dispatch_command_for_target(
                                &role_selection,
                                &dispatch_receipt.dispatch_target,
                            );
                        if let Err(error) = super::refresh_downstream_dispatch_preview(
                            store.root(),
                            &role_selection,
                            &run_graph_bootstrap,
                            &mut dispatch_receipt,
                        ) {
                            eprintln!(
                                "Failed to write downstream runtime dispatch packet: {error}"
                            );
                            return ExitCode::from(1);
                        }
                        let dispatch_packet_path = match super::write_runtime_dispatch_packet(
                            store.root(),
                            &role_selection,
                            &dispatch_receipt,
                            &taskflow_handoff_plan,
                            &run_graph_bootstrap,
                        ) {
                            Ok(path) => path,
                            Err(error) => {
                                eprintln!("Failed to write runtime dispatch packet: {error}");
                                return ExitCode::from(1);
                            }
                        };
                        dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path);
                        let allow_taskflow_pack_execution = dispatch_receipt.dispatch_kind
                            != "taskflow_pack"
                            || super::taskflow_task_bridge::infer_project_root_from_state_root(store.root()).is_some();
                        if dispatch_receipt.dispatch_status == "routed"
                            && allow_taskflow_pack_execution
                        {
                            if let Err(error) = super::execute_and_record_dispatch_receipt(
                                store.root(),
                                &store,
                                &role_selection,
                                &run_graph_bootstrap,
                                &mut dispatch_receipt,
                            )
                            .await
                            {
                                eprintln!("Failed to execute runtime dispatch handoff: {error}");
                                return ExitCode::from(1);
                            }
                        }
                        if let Err(error) = super::execute_downstream_dispatch_chain(
                            store.root(),
                            &store,
                            &role_selection,
                            &run_graph_bootstrap,
                            &mut dispatch_receipt,
                        )
                        .await
                        {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                        let dispatch_receipt_json = serde_json::to_value(&dispatch_receipt)
                            .unwrap_or(serde_json::Value::Null);
                        if let Err(error) = store
                            .record_run_graph_dispatch_receipt(&dispatch_receipt)
                            .await
                        {
                            eprintln!("Failed to record run-graph dispatch receipt: {error}");
                            return ExitCode::from(1);
                        }
                        let direct_consumption_ready = bundle_check.ok
                            && docflow_verdict.ready
                            && !closure_admission
                                .blockers
                                .iter()
                                .any(|row| row == "pending_design_packet");
                        let payload = super::TaskflowDirectConsumptionPayload {
                            artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                            artifact_type: "runtime_consumption".to_string(),
                            generated_at: time::OffsetDateTime::now_utc()
                                .format(&super::Rfc3339)
                                .expect("rfc3339 timestamp should render"),
                            closure_authority: "taskflow".to_string(),
                            role_selection,
                            request_text,
                            direct_consumption_ready,
                            runtime_bundle,
                            bundle_check,
                            docflow_activation: super::RuntimeConsumptionDocflowActivation {
                                activated: true,
                                runtime_family: "docflow".to_string(),
                                owner_runtime: "taskflow".to_string(),
                                evidence: serde_json::json!({
                                    "overview": overview,
                                    "registry": registry,
                                    "check": check,
                                    "readiness": readiness,
                                    "proof": proof,
                                }),
                            },
                            docflow_verdict,
                            closure_admission,
                            taskflow_handoff_plan,
                            run_graph_bootstrap,
                            dispatch_receipt: dispatch_receipt_json,
                        };
                        if as_json {
                            if let Err(error) =
                                super::emit_taskflow_consume_final_json(&store, &payload)
                            {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        } else {
                            let snapshot = serde_json::json!({
                                "surface": "vida taskflow consume final",
                                "payload": &payload,
                            });
                            let snapshot_path = match super::write_runtime_consumption_snapshot(
                                store.root(),
                                "final",
                                &snapshot,
                            ) {
                                Ok(path) => path,
                                Err(error) => {
                                    eprintln!("{error}");
                                    return ExitCode::from(1);
                                }
                            };
                            super::print_surface_header(
                                super::RenderMode::Plain,
                                "vida taskflow consume final",
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "request",
                                &payload.request_text,
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "bundle ready",
                                if payload.bundle_check.ok {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "docflow ready",
                                if payload.docflow_verdict.ready {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "closure admitted",
                                if payload.closure_admission.admitted {
                                    "true"
                                } else {
                                    "false"
                                },
                            );
                            if let Some(mode) = payload.role_selection.execution_plan
                                ["orchestration_contract"]["mode"]
                                .as_str()
                            {
                                super::print_surface_line(
                                    super::RenderMode::Plain,
                                    "execution mode",
                                    mode,
                                );
                            }
                            if let Some(message) = payload.role_selection.execution_plan
                                ["orchestration_contract"]["initial_response"]["operator_message"]
                                .as_str()
                            {
                                super::print_surface_line(
                                    super::RenderMode::Plain,
                                    "first step",
                                    message,
                                );
                            }
                            let replanning = payload.role_selection.execution_plan
                                ["orchestration_contract"]["replanning"]["checkpoints"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(serde_json::Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ");
                            if !replanning.is_empty() {
                                super::print_surface_line(
                                    super::RenderMode::Plain,
                                    "replan checkpoints",
                                    &replanning,
                                );
                            }
                            if payload.role_selection.execution_plan["status"] == "design_first" {
                                if let Some(feature_slug) = payload.role_selection.execution_plan
                                    ["tracked_flow_bootstrap"]["feature_slug"]
                                    .as_str()
                                {
                                    super::print_surface_line(
                                        super::RenderMode::Plain,
                                        "tracked flow",
                                        &format!("spec-first bootstrap for `{feature_slug}`"),
                                    );
                                }
                                if let Some(command) = payload.role_selection.execution_plan
                                    ["tracked_flow_bootstrap"]["bootstrap_command"]
                                    .as_str()
                                {
                                    super::print_surface_line(
                                        super::RenderMode::Plain,
                                        "next tracked command",
                                        command,
                                    );
                                }
                                let required_lanes = payload.role_selection.execution_plan
                                    ["orchestration_contract"]["delegation_policy"]
                                    ["required_lanes"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(super::display_lane_label)
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                if !required_lanes.is_empty() {
                                    super::print_surface_line(
                                        super::RenderMode::Plain,
                                        "delegated lanes",
                                        &required_lanes,
                                    );
                                }
                            } else if let Some(agent_type) = payload.taskflow_handoff_plan
                                ["activation_chain"]["implementer"]["activation_agent_type"]
                                .as_str()
                            {
                                super::print_surface_line(
                                    super::RenderMode::Plain,
                                    "implementer carrier",
                                    agent_type,
                                );
                            }
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "snapshot path",
                                &snapshot_path,
                            );
                        }

                        if payload.closure_admission.admitted {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::from(1)
                        }
                    }
                    Err(error) => {
                        if as_json {
                            let runtime_bundle = super::blocking_runtime_bundle(&error);
                            let bundle_check =
                                super::taskflow_consume_bundle_check(&runtime_bundle);
                            let docflow_verdict = super::RuntimeConsumptionDocflowVerdict {
                                status: "block".to_string(),
                                ready: false,
                                blockers: vec![
                                    "missing_docflow_activation".to_string(),
                                    "missing_readiness_verdict".to_string(),
                                    "missing_proof_verdict".to_string(),
                                ],
                                proof_surfaces: vec![],
                            };
                            let role_selection =
                                super::blocking_lane_selection(&request_text, &error);
                            let closure_admission = super::build_runtime_closure_admission(
                                &bundle_check,
                                &docflow_verdict,
                                &role_selection,
                            );
                            let payload = super::TaskflowDirectConsumptionPayload {
                                artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                                artifact_type: "runtime_consumption".to_string(),
                                generated_at: time::OffsetDateTime::now_utc()
                                    .format(&super::Rfc3339)
                                    .expect("rfc3339 timestamp should render"),
                                closure_authority: "taskflow".to_string(),
                                request_text,
                                role_selection,
                                runtime_bundle,
                                bundle_check,
                                docflow_activation: super::blocking_docflow_activation(&error),
                                docflow_verdict,
                                closure_admission,
                                taskflow_handoff_plan: serde_json::json!({
                                    "status": "blocked",
                                    "handoff_ready": false,
                                    "reason": "docflow_activation_failed",
                                }),
                                run_graph_bootstrap: serde_json::json!({
                                    "status": "blocked",
                                    "handoff_ready": false,
                                    "reason": "docflow_activation_failed",
                                }),
                                dispatch_receipt: serde_json::json!({
                                    "status": "blocked",
                                    "reason": "docflow_activation_failed",
                                }),
                                direct_consumption_ready: false,
                            };
                            if let Err(snapshot_error) =
                                super::emit_taskflow_consume_final_json(&store, &payload)
                            {
                                eprintln!("{snapshot_error}");
                                return ExitCode::from(1);
                            }
                            return ExitCode::from(1);
                        }
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
        [head, subcommand, ..] if head == "consume" && subcommand == "final" => {
            eprintln!("Usage: vida taskflow consume final <request_text> [--json]");
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

fn build_runtime_consumption_dispatch_receipt(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
) -> crate::state_store::RunGraphDispatchReceipt {
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let run_id = super::json_string(run_graph_bootstrap.get("run_id"))
        .unwrap_or_else(|| super::runtime_consumption_run_id(role_selection));
    let latest_status = run_graph_bootstrap
        .get("latest_status")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let dispatch_target = super::json_string(latest_status.get("next_node"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| role_selection.selected_role.clone());
    let (dispatch_kind, dispatch_surface, activation_agent_type, activation_runtime_role) =
        super::downstream_activation_fields(role_selection, &dispatch_target);
    let activation_agent_type = activation_agent_type.or_else(|| {
        if role_selection.conversational_mode.is_some() {
            role_selection.execution_plan["default_route"]["activation_agent_type"]
                .as_str()
                .map(str::to_string)
        } else {
            super::dispatch_contract_lane(&role_selection.execution_plan, &dispatch_target)
                .and_then(|route| route.get("activation_agent_type"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    role_selection.execution_plan["codex_runtime_assignment"]["activation_agent_type"]
                        .as_str()
                        .map(str::to_string)
                })
        }
    });
    let activation_runtime_role = activation_runtime_role.or_else(|| {
        if role_selection.conversational_mode.is_some() {
            role_selection.execution_plan["default_route"]["activation_runtime_role"]
                .as_str()
                .map(str::to_string)
        } else {
            super::dispatch_contract_lane(&role_selection.execution_plan, &dispatch_target)
                .and_then(|route| route.get("activation_runtime_role"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    role_selection.execution_plan["codex_runtime_assignment"]["activation_runtime_role"]
                        .as_str()
                        .map(str::to_string)
                })
        }
    });
    let selected_backend = activation_agent_type
        .clone()
        .or_else(|| {
            if role_selection.conversational_mode.is_some() {
                role_selection.execution_plan["default_route"]["selected_agent_id"]
                    .as_str()
                    .map(str::to_string)
            } else {
                super::dispatch_contract_lane(&role_selection.execution_plan, &dispatch_target)
                    .and_then(|route| route.get("selected_agent_id"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        role_selection.execution_plan["codex_runtime_assignment"]["selected_agent_id"]
                            .as_str()
                            .map(str::to_string)
                    })
            }
        })
        .filter(|value| !value.is_empty());
    let dispatch_command = super::json_string(latest_status.get("dispatch_command"));
    let dispatch_blockers = super::json_string_list(latest_status.get("dispatch_blockers"));
    let dispatch_ready = super::json_bool(
        latest_status.get("dispatch_ready"),
        super::json_bool(run_graph_bootstrap.get("handoff_ready"), false),
    );
    crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: if dispatch_ready {
            "routed".to_string()
        } else {
            "blocked".to_string()
        },
        lane_status: super::LaneStatus::LaneRunning.as_str().to_string(),
        supersedes_receipt_id: None,
        exception_path_receipt_id: None,
        dispatch_kind,
        dispatch_surface,
        dispatch_command: dispatch_command.clone(),
        dispatch_packet_path: run_graph_bootstrap
            .get("dispatch_packet_path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        dispatch_result_path: None,
        blocker_code: if !dispatch_blockers.is_empty() {
            Some(dispatch_blockers[0].clone())
        } else {
            None
        },
        downstream_dispatch_target: Some(dispatch_target),
        downstream_dispatch_command: dispatch_command.clone(),
        downstream_dispatch_note: None,
        downstream_dispatch_ready: dispatch_ready,
        downstream_dispatch_blockers: dispatch_blockers,
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: 0,
        downstream_dispatch_active_target: None,
        downstream_dispatch_last_target: None,
        activation_agent_type,
        activation_runtime_role,
        selected_backend,
        recorded_at,
    }
}
