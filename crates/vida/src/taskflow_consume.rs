use std::process::ExitCode;
use time::format_description::well_known::Rfc3339;

use crate::display_lane_label;
use crate::BlockerCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConsumeFinalMode {
    Execute,
    Preview,
    ValidateOnly,
}

impl ConsumeFinalMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Execute => "execute",
            Self::Preview => "preview",
            Self::ValidateOnly => "validate_only",
        }
    }

    fn is_read_only(self) -> bool {
        !matches!(self, Self::Execute)
    }
}

fn parse_taskflow_consume_final_args(
    request: &[String],
) -> Result<(bool, ConsumeFinalMode, String), String> {
    let mut as_json = false;
    let mut mode = ConsumeFinalMode::Execute;
    let mut request_parts = Vec::new();
    for arg in request {
        match arg.as_str() {
            "--json" => as_json = true,
            "--preview" => mode = ConsumeFinalMode::Preview,
            "--validate-only" => mode = ConsumeFinalMode::ValidateOnly,
            "--help" | "-h" => {
                return Err(
                    "Usage: vida taskflow consume final <request_text> [--preview | --validate-only] [--json]"
                        .to_string(),
                )
            }
            _ => request_parts.push(arg.clone()),
        }
    }
    let request_text = request_parts.join(" ").trim().to_string();
    if request_text.is_empty() {
        return Err(
            "Usage: vida taskflow consume final <request_text> [--preview | --validate-only] [--json]"
                .to_string(),
        );
    }
    Ok((as_json, mode, request_text))
}

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
            super::taskflow_consume_resume::run_taskflow_consume_advance_command(
                super::taskflow_task_bridge::proxy_state_dir(),
                as_json,
                requested_run_id,
                max_rounds,
            )
            .await
        }
        [head, subcommand, request @ ..] if head == "consume" && subcommand == "final" => {
            let (as_json, consume_final_mode, request_text) =
                match parse_taskflow_consume_final_args(request) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            if request_text.is_empty() {
                eprintln!(
                    "Usage: vida taskflow consume final <request_text> [--preview | --validate-only] [--json]"
                );
                return ExitCode::from(2);
            }

            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match super::StateStore::open_existing(state_dir).await {
                Ok(store) => match super::build_taskflow_consume_bundle_payload(&store).await {
                    Ok(runtime_bundle) => {
                        let bundle_check = super::taskflow_consume_bundle_check(&runtime_bundle);
                        let (registry, check, readiness, proof, overview) =
                            super::build_docflow_runtime_evidence();
                        let docflow_receipt_evidence =
                            crate::runtime_consumption_surface::build_docflow_receipt_evidence(
                                &readiness, &proof,
                            );
                        let mut docflow_verdict = super::build_docflow_runtime_verdict(
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
                                    let dispatch_receipt = blocked_dispatch_receipt(
                                        "unresolved_lane_selection",
                                        &bundle_check,
                                        &runtime_bundle,
                                    );
                                    let mut closure_admission =
                                        super::RuntimeConsumptionClosureAdmission {
                                            status: "blocked".to_string(),
                                            admitted: false,
                                            blockers: vec!["unresolved_lane_selection".to_string()],
                                            proof_surfaces: vec![
                                                "vida taskflow consume bundle check".to_string(),
                                            ],
                                        };
                                    normalize_runtime_consumption_statuses(
                                        &mut docflow_verdict,
                                        &mut closure_admission,
                                    );
                                    let generated_at = time::OffsetDateTime::now_utc()
                                        .format(&super::Rfc3339)
                                        .expect("rfc3339 timestamp should render");
                                    let payload = super::TaskflowDirectConsumptionPayload {
                                        artifact_name: "taskflow_direct_runtime_consumption"
                                            .to_string(),
                                        artifact_type: "runtime_consumption".to_string(),
                                        generated_at: generated_at.clone(),
                                        closure_authority: "taskflow".to_string(),
                                        consume_final_mode: consume_final_mode.as_str().to_string(),
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
                                                    "receipt_evidence": docflow_receipt_evidence.clone(),
                                                }),
                                            },
                                        docflow_verdict,
                                        closure_admission: closure_admission.clone(),
                                        closure_admission_artifact:
                                            crate::runtime_consumption_surface::canonical_closure_admission_artifact_json(
                                                &generated_at,
                                                "taskflow",
                                                &request_text,
                                                &closure_admission,
                                            ),
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
                                        dispatch_receipt,
                                        dispatch_packet_preview: None,
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
                        let taskflow_handoff_plan =
                            super::build_taskflow_handoff_plan(&role_selection);
                        let run_graph_bootstrap =
                            crate::runtime_dispatch_bootstrap::build_runtime_consumption_run_graph_bootstrap(
                                &store,
                                &role_selection,
                            )
                            .await;
                        let mut closure_admission = super::build_runtime_closure_admission(
                            &bundle_check,
                            &docflow_verdict,
                            &role_selection,
                        );
                        normalize_runtime_consumption_statuses(
                            &mut docflow_verdict,
                            &mut closure_admission,
                        );
                        let execution_preparation_gate = build_execution_preparation_evidence_gate(
                            &role_selection,
                            &taskflow_handoff_plan,
                            &run_graph_bootstrap,
                        );
                        let retrieval_policy_gate =
                            build_retrieval_policy_decision_gate(&bundle_check);
                        let approval_delegation_gate = build_approval_delegation_evidence_gate(
                            &store,
                            &role_selection,
                            &run_graph_bootstrap,
                        )
                        .await;
                        if let Some(blocker_code) = execution_preparation_gate.blocker_code() {
                            if !closure_admission
                                .blockers
                                .iter()
                                .any(|value| value == blocker_code)
                            {
                                closure_admission.blockers.push(blocker_code.to_string());
                                closure_admission.blockers.sort();
                                closure_admission.blockers.dedup();
                            }
                            closure_admission.status = "blocked".to_string();
                            closure_admission.admitted = false;
                        }
                        if let Some(blocker_code) = retrieval_policy_gate.blocker_code() {
                            if !closure_admission
                                .blockers
                                .iter()
                                .any(|value| value == blocker_code)
                            {
                                closure_admission.blockers.push(blocker_code.to_string());
                                closure_admission.blockers.sort();
                                closure_admission.blockers.dedup();
                            }
                            closure_admission.status = "blocked".to_string();
                            closure_admission.admitted = false;
                        }
                        if let Some(blocker_code) = approval_delegation_gate.blocker_code() {
                            if !closure_admission
                                .blockers
                                .iter()
                                .any(|value| value == blocker_code)
                            {
                                closure_admission.blockers.push(blocker_code.to_string());
                                closure_admission.blockers.sort();
                                closure_admission.blockers.dedup();
                            }
                            closure_admission.status = "blocked".to_string();
                            closure_admission.admitted = false;
                        }
                        let mut dispatch_receipt = build_runtime_consumption_dispatch_receipt(
                            &role_selection,
                            &run_graph_bootstrap,
                        );
                        if let Some(blocker_code) = execution_preparation_gate.blocker_code() {
                            dispatch_receipt.dispatch_status = "blocked".to_string();
                            dispatch_receipt.blocker_code = Some(blocker_code.to_string());
                            dispatch_receipt.downstream_dispatch_ready = false;
                            if !dispatch_receipt
                                .downstream_dispatch_blockers
                                .iter()
                                .any(|value| value == blocker_code)
                            {
                                dispatch_receipt
                                    .downstream_dispatch_blockers
                                    .insert(0, blocker_code.to_string());
                            }
                        }
                        if let Some(blocker_code) = retrieval_policy_gate.blocker_code() {
                            dispatch_receipt.dispatch_status = "blocked".to_string();
                            dispatch_receipt.blocker_code = Some(blocker_code.to_string());
                            dispatch_receipt.downstream_dispatch_ready = false;
                            if !dispatch_receipt
                                .downstream_dispatch_blockers
                                .iter()
                                .any(|value| value == blocker_code)
                            {
                                dispatch_receipt
                                    .downstream_dispatch_blockers
                                    .insert(0, blocker_code.to_string());
                            }
                        }
                        if let Some(blocker_code) = approval_delegation_gate.blocker_code() {
                            dispatch_receipt.dispatch_status = "blocked".to_string();
                            dispatch_receipt.blocker_code = Some(blocker_code.to_string());
                            dispatch_receipt.downstream_dispatch_ready = false;
                            if !dispatch_receipt
                                .downstream_dispatch_blockers
                                .iter()
                                .any(|value| value == blocker_code)
                            {
                                dispatch_receipt
                                    .downstream_dispatch_blockers
                                    .insert(0, blocker_code.to_string());
                            }
                        }
                        dispatch_receipt.dispatch_command =
                            super::runtime_dispatch_command_for_target(
                                &role_selection,
                                &dispatch_receipt.dispatch_target,
                            );
                        let downstream_preview_result = if consume_final_mode.is_read_only() {
                            super::preview_downstream_dispatch_receipt(
                                &store,
                                &role_selection,
                                &mut dispatch_receipt,
                            )
                            .await
                        } else {
                            super::refresh_downstream_dispatch_preview(
                                &store,
                                &role_selection,
                                &run_graph_bootstrap,
                                &mut dispatch_receipt,
                            )
                            .await
                        };
                        if let Err(error) = downstream_preview_result {
                            eprintln!(
                                "Failed to write downstream runtime dispatch packet: {error}"
                            );
                            return ExitCode::from(1);
                        }
                        let ctx = crate::RuntimeDispatchPacketContext::new(
                            store.root(),
                            &role_selection,
                            &dispatch_receipt,
                            &taskflow_handoff_plan,
                            &run_graph_bootstrap,
                        );
                        let dispatch_packet_preview =
                            match super::runtime_dispatch_packet_preview(&ctx) {
                                Ok(preview) => Some(preview),
                                Err(error) => {
                                    eprintln!(
                                        "Failed to build runtime dispatch packet preview: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            };
                        let pending_design_packet =
                            super::blocker_code_str(super::BlockerCode::PendingDesignPacket);
                        let pending_execution_preparation_evidence = super::blocker_code_str(
                            super::BlockerCode::PendingExecutionPreparationEvidence,
                        );
                        let direct_consumption_ready = bundle_check.ok
                            && docflow_verdict.ready
                            && !closure_admission.blockers.iter().any(|row| {
                                row == pending_design_packet
                                    || row == pending_execution_preparation_evidence
                            })
                            && dispatch_packet_preview
                                .as_ref()
                                .and_then(|preview| preview.get("status"))
                                .and_then(serde_json::Value::as_str)
                                != Some("blocked");
                        if !consume_final_mode.is_read_only() {
                            let dispatch_packet_path =
                                match super::write_runtime_dispatch_packet(&ctx) {
                                    Ok(path) => path,
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to write runtime dispatch packet: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                };
                            dispatch_receipt.dispatch_packet_path = Some(dispatch_packet_path);
                            if !direct_consumption_ready {
                                let blocker_code = dispatch_packet_preview
                                    .as_ref()
                                    .and_then(|preview| preview.get("status"))
                                    .and_then(serde_json::Value::as_str)
                                    .filter(|status| *status == "blocked")
                                    .map(|_| {
                                        super::blocker_code_str(
                                            super::BlockerCode::MissingExecutionPreparationContract,
                                        )
                                        .to_string()
                                    })
                                    .or_else(|| closure_admission.blockers.first().cloned())
                                    .or_else(|| docflow_verdict.blockers.first().cloned())
                                    .or_else(|| bundle_check.blockers.first().cloned())
                                    .unwrap_or_else(|| {
                                        super::blocker_code_str(
                                            super::BlockerCode::PendingExecutionPreparationEvidence,
                                        )
                                        .to_string()
                                    });
                                dispatch_receipt.dispatch_status = "blocked".to_string();
                                dispatch_receipt.lane_status =
                                    super::LaneStatus::LaneBlocked.as_str().to_string();
                                dispatch_receipt.blocker_code = Some(blocker_code);
                            }
                        }
                        if let Some(project_root) =
                            super::taskflow_task_bridge::infer_project_root_from_state_root(
                                store.root(),
                            )
                        {
                            if let Some(fallback_backend) =
                                super::fallback_backend_for_blocked_primary_dispatch_receipt(
                                    &project_root,
                                    &role_selection,
                                    &dispatch_receipt,
                                )
                            {
                                dispatch_receipt.selected_backend = Some(fallback_backend);
                            }
                        }
                        let allow_taskflow_pack_execution = dispatch_receipt.dispatch_kind
                            != "taskflow_pack"
                            || super::taskflow_task_bridge::infer_project_root_from_state_root(
                                store.root(),
                            )
                            .is_some();
                        let state_root = store.root().to_path_buf();
                        drop(store);
                        if !consume_final_mode.is_read_only()
                            && direct_consumption_ready
                            && dispatch_receipt.dispatch_status == "routed"
                            && allow_taskflow_pack_execution
                        {
                            if let Err(error) = super::execute_and_record_dispatch_receipt(
                                &state_root,
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
                        let store = match super::StateStore::open_existing(state_root.clone()).await
                        {
                            Ok(store) => store,
                            Err(error) => {
                                eprintln!(
                                    "Failed to reopen authoritative state store after runtime dispatch: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        let state_root = store.root().to_path_buf();
                        drop(store);
                        if !consume_final_mode.is_read_only() && direct_consumption_ready {
                            if let Err(error) = super::execute_downstream_dispatch_chain(
                                &state_root,
                                &role_selection,
                                &run_graph_bootstrap,
                                &mut dispatch_receipt,
                            )
                            .await
                            {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        }
                        let store = match super::StateStore::open_existing(state_root.clone()).await
                        {
                            Ok(store) => store,
                            Err(error) => {
                                eprintln!(
                                    "Failed to reopen authoritative state store before receipt persistence: {error}"
                                );
                                return ExitCode::from(1);
                            }
                        };
                        // Re-sync continuation binding after downstream dispatch chain advances the run-graph.
                        // Downstream execution inside execute_downstream_dispatch_chain updates run-graph status
                        // via execute_and_record_dispatch_receipt, but the root-level continuation binding must
                        // be refreshed to reflect the final downstream target.
                        if !consume_final_mode.is_read_only() && direct_consumption_ready {
                            if let Some(run_id) = run_graph_bootstrap
                                .get("run_id")
                                .and_then(serde_json::Value::as_str)
                                .filter(|value| !value.is_empty())
                            {
                                if let Ok(status) = store.run_graph_status(run_id).await {
                                    if let Err(error) = crate::taskflow_continuation::sync_run_graph_continuation_binding(
                                        &store,
                                        &status,
                                        "consume_after_downstream_chain",
                                    )
                                    .await
                                    {
                                        eprintln!("Failed to re-sync continuation binding after downstream dispatch chain: {error}");
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                        }
                        let dispatch_receipt_json = serde_json::to_value(&dispatch_receipt)
                            .unwrap_or(serde_json::Value::Null);
                        if !consume_final_mode.is_read_only() {
                            if let Err(error) = store
                                .record_run_graph_dispatch_receipt(&dispatch_receipt)
                                .await
                            {
                                eprintln!("Failed to record run-graph dispatch receipt: {error}");
                                return ExitCode::from(1);
                            }
                        }
                        let generated_at = time::OffsetDateTime::now_utc()
                            .format(&super::Rfc3339)
                            .expect("rfc3339 timestamp should render");
                        let payload = super::TaskflowDirectConsumptionPayload {
                            artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                            artifact_type: "runtime_consumption".to_string(),
                            generated_at: generated_at.clone(),
                            closure_authority: "taskflow".to_string(),
                            consume_final_mode: consume_final_mode.as_str().to_string(),
                            role_selection,
                            request_text: request_text.clone(),
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
                                    "receipt_evidence": docflow_receipt_evidence,
                                }),
                            },
                            docflow_verdict,
                            closure_admission: closure_admission.clone(),
                            closure_admission_artifact:
                                crate::runtime_consumption_surface::canonical_closure_admission_artifact_json(
                                    &generated_at,
                                    "taskflow",
                                    &request_text,
                                    &closure_admission,
                                ),
                            taskflow_handoff_plan,
                            run_graph_bootstrap,
                            dispatch_receipt: dispatch_receipt_json,
                            dispatch_packet_preview,
                        };
                        if as_json {
                            if let Err(error) =
                                super::emit_taskflow_consume_final_json(&store, &payload)
                            {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                            if let Err(error) =
                                ensure_runtime_consumption_final_task_reconciliation_summary(
                                    &store, None,
                                )
                                .await
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
                            if let Err(error) =
                                ensure_runtime_consumption_final_task_reconciliation_summary(
                                    &store,
                                    Some(snapshot_path.clone()),
                                )
                                .await
                            {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                            super::print_surface_header(
                                super::RenderMode::Plain,
                                "vida taskflow consume final",
                            );
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "mode",
                                payload.consume_final_mode.as_str(),
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
                                    .map(display_lane_label)
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
                            if let Some(preview) = payload.dispatch_packet_preview.as_ref() {
                                if let Some(packet_template_kind) = preview
                                    .get("packet_template_kind")
                                    .and_then(serde_json::Value::as_str)
                                {
                                    super::print_surface_line(
                                        super::RenderMode::Plain,
                                        "packet template",
                                        packet_template_kind,
                                    );
                                }
                                let missing_fields = preview["packet_contract_missing_fields"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(serde_json::Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                if !missing_fields.is_empty() {
                                    super::print_surface_line(
                                        super::RenderMode::Plain,
                                        "missing packet fields",
                                        &missing_fields,
                                    );
                                }
                            }
                            super::print_surface_line(
                                super::RenderMode::Plain,
                                "snapshot path",
                                &snapshot_path,
                            );
                        }

                        match consume_final_mode {
                            ConsumeFinalMode::Preview => ExitCode::SUCCESS,
                            ConsumeFinalMode::Execute | ConsumeFinalMode::ValidateOnly => {
                                if payload.direct_consumption_ready {
                                    ExitCode::SUCCESS
                                } else {
                                    ExitCode::from(1)
                                }
                            }
                        }
                    }
                    Err(error) => {
                        if as_json {
                            let runtime_bundle = super::blocking_runtime_bundle(&error);
                            let bundle_check =
                                super::taskflow_consume_bundle_check(&runtime_bundle);
                            let mut docflow_verdict = super::RuntimeConsumptionDocflowVerdict {
                                status: "blocked".to_string(),
                                ready: false,
                                blockers: vec![
                                    crate::release_contract_adapters::blocker_code(
                                        BlockerCode::MissingDocflowActivation,
                                    )
                                    .expect(
                                        "missing docflow activation blocker should be canonical",
                                    ),
                                    crate::release_contract_adapters::blocker_code(
                                        BlockerCode::MissingReadinessVerdict,
                                    )
                                    .expect(
                                        "missing readiness verdict blocker should be canonical",
                                    ),
                                    crate::release_contract_adapters::blocker_code(
                                        BlockerCode::MissingProofVerdict,
                                    )
                                    .expect("missing proof verdict blocker should be canonical"),
                                ],
                                proof_surfaces: vec![],
                            };
                            let role_selection =
                                super::blocking_lane_selection(&request_text, &error);
                            let mut closure_admission = super::build_runtime_closure_admission(
                                &bundle_check,
                                &docflow_verdict,
                                &role_selection,
                            );
                            normalize_runtime_consumption_statuses(
                                &mut docflow_verdict,
                                &mut closure_admission,
                            );
                            let readiness = super::RuntimeConsumptionEvidence {
                                surface: "vida docflow readiness-check --profile active-canon"
                                    .to_string(),
                                ok: false,
                                row_count: 0,
                                verdict: Some("blocked".to_string()),
                                artifact_path: Some(
                                    "vida/config/docflow-readiness.current.jsonl".to_string(),
                                ),
                                output: error.clone(),
                            };
                            let proof = super::RuntimeConsumptionEvidence {
                                surface: "vida docflow proofcheck --profile active-canon"
                                    .to_string(),
                                ok: false,
                                row_count: 0,
                                verdict: Some("blocked".to_string()),
                                artifact_path: Some(
                                    crate::runtime_consumption_surface::DOCFLOW_PROOF_CURRENT_PATH
                                        .to_string(),
                                ),
                                output: error.clone(),
                            };
                            let docflow_receipt_evidence =
                                crate::runtime_consumption_surface::build_docflow_receipt_evidence(
                                    &readiness, &proof,
                                );
                            let dispatch_receipt = blocked_dispatch_receipt(
                                "docflow_activation_failed",
                                &bundle_check,
                                &runtime_bundle,
                            );
                            let mut docflow_activation = super::blocking_docflow_activation(&error);
                            if let Some(evidence) = docflow_activation.evidence.as_object_mut() {
                                evidence.insert(
                                    "receipt_evidence".to_string(),
                                    docflow_receipt_evidence,
                                );
                            }
                            let generated_at = time::OffsetDateTime::now_utc()
                                .format(&super::Rfc3339)
                                .expect("rfc3339 timestamp should render");
                            let payload = super::TaskflowDirectConsumptionPayload {
                                artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                                artifact_type: "runtime_consumption".to_string(),
                                generated_at: generated_at.clone(),
                                closure_authority: "taskflow".to_string(),
                                consume_final_mode: consume_final_mode.as_str().to_string(),
                                request_text: request_text.clone(),
                                role_selection,
                                runtime_bundle,
                                bundle_check,
                                docflow_activation,
                                docflow_verdict,
                                closure_admission: closure_admission.clone(),
                                closure_admission_artifact:
                                    crate::runtime_consumption_surface::canonical_closure_admission_artifact_json(
                                        &generated_at,
                                        "taskflow",
                                        &request_text,
                                        &closure_admission,
                                    ),
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
                                dispatch_receipt,
                                dispatch_packet_preview: None,
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
            eprintln!(
                "Usage: vida taskflow consume final <request_text> [--preview | --validate-only] [--json]"
            );
            ExitCode::from(2)
        }
        _ => ExitCode::from(2),
    }
}

async fn ensure_runtime_consumption_final_task_reconciliation_summary(
    store: &super::StateStore,
    snapshot_path_hint: Option<String>,
) -> Result<(), String> {
    if store
        .latest_task_reconciliation_summary()
        .await
        .map_err(|error| format!("Failed to load latest task reconciliation summary: {error}"))?
        .is_some()
    {
        return Ok(());
    }

    let snapshot_path = match snapshot_path_hint {
        Some(snapshot_path) => snapshot_path,
        None => super::runtime_consumption_state::latest_final_runtime_consumption_snapshot_path(
            store.root(),
        )
        .map_err(|error| {
            format!("Failed to locate runtime consumption final snapshot path: {error}")
        })?
        .ok_or_else(|| {
            "Failed to locate runtime consumption final snapshot path after consume final"
                .to_string()
        })?,
    };

    let _ = store
        .record_runtime_consumption_final_task_reconciliation_summary(Some(snapshot_path))
        .await
        .map_err(|error| {
            format!(
                "Failed to record runtime consumption final task reconciliation summary: {error}"
            )
        })?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExecutionPreparationEvidenceGate {
    missing_evidence_or_handoff_packet: bool,
}

impl ExecutionPreparationEvidenceGate {
    fn blocker_code(self) -> Option<&'static str> {
        if self.missing_evidence_or_handoff_packet {
            Some(super::blocker_code_str(
                super::BlockerCode::PendingExecutionPreparationEvidence,
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ApprovalDelegationEvidenceGate {
    missing_approval_or_delegation_evidence: bool,
}

impl ApprovalDelegationEvidenceGate {
    fn blocker_code(self) -> Option<&'static str> {
        if self.missing_approval_or_delegation_evidence {
            Some(super::blocker_code_str(
                super::BlockerCode::PendingApprovalDelegationEvidence,
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetrievalPolicyDecisionGate {
    blocker_code: Option<String>,
}

impl RetrievalPolicyDecisionGate {
    fn blocker_code(&self) -> Option<&str> {
        self.blocker_code.as_deref()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DeveloperHandoffPacketArtifact {
    path: Option<String>,
    ready: bool,
    status: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExecutionPreparationEvidenceArtifact {
    ready: bool,
    status: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExecutionPreparationArtifacts {
    handoff_ready: bool,
    developer_handoff_packet: DeveloperHandoffPacketArtifact,
    execution_preparation_evidence: ExecutionPreparationEvidenceArtifact,
}

fn nonempty_json_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .and_then(|value| {
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
}

fn decode_execution_preparation_artifacts(
    taskflow_handoff_plan: &serde_json::Value,
    run_graph_bootstrap: &serde_json::Value,
) -> ExecutionPreparationArtifacts {
    let artifact_json = run_graph_bootstrap
        .get("execution_preparation_artifacts")
        .filter(|value| value.is_object());
    let packet_json = artifact_json.and_then(|value| value.get("developer_handoff_packet"));
    let evidence_json = artifact_json.and_then(|value| value.get("execution_preparation_evidence"));

    let handoff_ready = super::json_bool(taskflow_handoff_plan.get("handoff_ready"), false)
        && (artifact_json
            .map(|value| super::json_bool(value.get("handoff_ready"), false))
            .unwrap_or_else(|| super::json_bool(run_graph_bootstrap.get("handoff_ready"), false)));
    let developer_handoff_packet = DeveloperHandoffPacketArtifact {
        path: nonempty_json_string(packet_json.and_then(|value| value.get("path"))).or_else(|| {
            nonempty_json_string(run_graph_bootstrap.get("execution_preparation_packet_path"))
        }),
        ready: packet_json
            .map(|value| super::json_bool(value.get("ready"), false))
            .unwrap_or_else(|| {
                super::json_bool(
                    run_graph_bootstrap.get("execution_preparation_handoff_packet_ready"),
                    false,
                ) || run_graph_bootstrap
                    .get("execution_preparation_packet_path")
                    .and_then(serde_json::Value::as_str)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
            }),
        status: nonempty_json_string(packet_json.and_then(|value| value.get("status"))),
    };
    let execution_preparation_evidence = ExecutionPreparationEvidenceArtifact {
        ready: evidence_json
            .map(|value| super::json_bool(value.get("ready"), false))
            .unwrap_or_else(|| {
                super::json_bool(
                    run_graph_bootstrap.get("execution_preparation_evidence_ready"),
                    false,
                ) || run_graph_bootstrap["evidence"]["execution_preparation"]["status"].as_str()
                    == Some("ready")
                    || run_graph_bootstrap["evidence"]["execution_preparation"]["ready"]
                        .as_bool()
                        .unwrap_or(false)
            }),
        status: nonempty_json_string(evidence_json.and_then(|value| value.get("status"))).or_else(
            || {
                nonempty_json_string(
                    run_graph_bootstrap["evidence"]["execution_preparation"].get("status"),
                )
            },
        ),
    };

    ExecutionPreparationArtifacts {
        handoff_ready,
        developer_handoff_packet,
        execution_preparation_evidence,
    }
}

fn build_execution_preparation_evidence_gate(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    taskflow_handoff_plan: &serde_json::Value,
    run_graph_bootstrap: &serde_json::Value,
) -> ExecutionPreparationEvidenceGate {
    let execution_plan = &role_selection.execution_plan;
    let dispatch_contract = &execution_plan["development_flow"]["dispatch_contract"];
    let execution_preparation_required = super::json_bool(
        dispatch_contract.get("execution_preparation_required"),
        false,
    ) || dispatch_contract["lane_catalog"]
        .get("execution_preparation")
        .is_some()
        || dispatch_contract["lane_sequence"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .any(|target| target == "execution_preparation");
    if !execution_preparation_required {
        return ExecutionPreparationEvidenceGate {
            missing_evidence_or_handoff_packet: false,
        };
    }

    let artifacts =
        decode_execution_preparation_artifacts(taskflow_handoff_plan, run_graph_bootstrap);
    let packet_ready = artifacts.developer_handoff_packet.ready
        && artifacts
            .developer_handoff_packet
            .path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    let evidence_ready = artifacts.execution_preparation_evidence.ready;

    ExecutionPreparationEvidenceGate {
        missing_evidence_or_handoff_packet: !(artifacts.handoff_ready
            && packet_ready
            && evidence_ready),
    }
}

async fn build_approval_delegation_evidence_gate(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    run_graph_bootstrap: &serde_json::Value,
) -> ApprovalDelegationEvidenceGate {
    let execution_plan = &role_selection.execution_plan;
    let delegated_mode = execution_plan["orchestration_contract"]["mode"].as_str()
        == Some("delegated_orchestration_cycle");
    if !delegated_mode {
        return ApprovalDelegationEvidenceGate {
            missing_approval_or_delegation_evidence: false,
        };
    }

    let Some(latest_status) = run_graph_bootstrap.get("latest_status") else {
        return ApprovalDelegationEvidenceGate {
            missing_approval_or_delegation_evidence: false,
        };
    };
    if !approval_delegation_latest_status_requires_receipt(latest_status) {
        return ApprovalDelegationEvidenceGate {
            missing_approval_or_delegation_evidence: false,
        };
    }
    let Some(run_id) = latest_status
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return ApprovalDelegationEvidenceGate {
            missing_approval_or_delegation_evidence: true,
        };
    };

    let receipt = match store.run_graph_approval_delegation_receipt(run_id).await {
        Ok(Some(receipt)) => receipt,
        Ok(None) => {
            return ApprovalDelegationEvidenceGate {
                missing_approval_or_delegation_evidence: true,
            };
        }
        Err(_) => {
            return ApprovalDelegationEvidenceGate {
                missing_approval_or_delegation_evidence: true,
            };
        }
    };

    ApprovalDelegationEvidenceGate {
        missing_approval_or_delegation_evidence: !approval_delegation_receipt_matches_latest_status(
            &receipt,
            latest_status,
        ),
    }
}

fn approval_delegation_latest_status_requires_receipt(latest_status: &serde_json::Value) -> bool {
    let status_field = |key: &str| latest_status.get(key).and_then(serde_json::Value::as_str);
    let status = status_field("status");
    let lifecycle_stage = status_field("lifecycle_stage");
    let policy_gate = status_field("policy_gate");
    let handoff_state = status_field("handoff_state");
    let resume_target = status_field("resume_target");
    let next_node = status_field("next_node");

    matches!(status, Some("awaiting_approval"))
        || matches!(
            lifecycle_stage,
            Some("approval_wait") | Some("implementation_review_wait")
        )
        || matches!(policy_gate, Some("approval_required"))
        || matches!(
            handoff_state,
            Some("awaiting_approval") | Some("awaiting_delegation")
        )
        || matches!(resume_target, Some("dispatch.approval"))
        || matches!(next_node, Some("approval"))
        || (matches!(status, Some("completed"))
            && matches!(lifecycle_stage, Some("implementation_complete"))
            && matches!(policy_gate, Some("not_required"))
            && matches!(handoff_state, Some("none"))
            && matches!(resume_target, Some("none"))
            && next_node.is_none())
}

fn approval_delegation_receipt_matches_latest_status(
    receipt: &super::state_store::RunGraphApprovalDelegationReceipt,
    latest_status: &serde_json::Value,
) -> bool {
    if receipt.transition_kind != "approval_complete" {
        return false;
    }

    let status_field = |key: &str| latest_status.get(key).and_then(serde_json::Value::as_str);
    receipt.run_id == status_field("run_id").unwrap_or_default()
        && receipt.task_id == status_field("task_id").unwrap_or_default()
        && receipt.task_class == status_field("task_class").unwrap_or_default()
        && receipt.route_task_class == status_field("route_task_class").unwrap_or_default()
        && receipt.active_node == status_field("active_node").unwrap_or_default()
        && receipt.status == status_field("status").unwrap_or_default()
        && receipt.lifecycle_stage == status_field("lifecycle_stage").unwrap_or_default()
        && receipt.policy_gate == status_field("policy_gate").unwrap_or_default()
        && receipt.handoff_state == status_field("handoff_state").unwrap_or_default()
        && receipt.resume_target == status_field("resume_target").unwrap_or_default()
}

fn build_retrieval_policy_decision_gate(
    bundle_check: &super::TaskflowConsumeBundleCheck,
) -> RetrievalPolicyDecisionGate {
    let missing_protocol_binding_receipt = crate::contract_profile_adapter::blocker_code_str(
        crate::contract_profile_adapter::BlockerCode::MissingProtocolBindingReceipt,
    );
    let protocol_binding_not_runtime_ready = crate::contract_profile_adapter::blocker_code_str(
        crate::contract_profile_adapter::BlockerCode::ProtocolBindingNotRuntimeReady,
    );
    let has_protocol_binding_receipt = !bundle_check
        .blockers
        .iter()
        .any(|code| code == missing_protocol_binding_receipt);
    let protocol_binding_runtime_ready = !bundle_check
        .blockers
        .iter()
        .any(|code| code == protocol_binding_not_runtime_ready);

    let blocker_code = crate::contract_profile_adapter::evaluate_policy_gate_protocol_binding(
        "retrieval_evidence",
        if has_protocol_binding_receipt {
            Some("bundle_check_protocol_binding_receipt")
        } else {
            None
        },
        protocol_binding_runtime_ready,
    );

    RetrievalPolicyDecisionGate { blocker_code }
}

fn blocked_dispatch_receipt(
    reason: &str,
    bundle_check: &super::TaskflowConsumeBundleCheck,
    runtime_bundle: &super::TaskflowConsumeBundlePayload,
) -> serde_json::Value {
    let mut downstream_dispatch_blockers = bundle_check.blockers.clone();
    if !downstream_dispatch_blockers.iter().any(|row| row == reason) {
        downstream_dispatch_blockers.insert(0, reason.to_string());
    }

    serde_json::json!({
        "status": "blocked",
        "dispatch_status": "blocked",
        "dispatch_kind": "none",
        "dispatch_target": "none",
        "dispatch_surface": "vida taskflow consume final",
        "blocker_code": reason,
        "downstream_dispatch_blockers": downstream_dispatch_blockers,
        "artifact_refs": {
            "root_artifact_id": bundle_check.root_artifact_id,
            "bundle_artifact_name": runtime_bundle.artifact_name,
            "cache_delivery_contract": {
                "cache_key_inputs_present": runtime_bundle.cache_delivery_contract["cache_key_inputs"].is_object(),
                "invalidation_tuple_present": runtime_bundle.cache_delivery_contract["invalidation_tuple"].is_object(),
            },
        },
    })
}

fn normalize_runtime_consumption_statuses(
    docflow_verdict: &mut super::RuntimeConsumptionDocflowVerdict,
    closure_admission: &mut super::RuntimeConsumptionClosureAdmission,
) {
    docflow_verdict.status =
        crate::release_contract_adapters::release_contract_status(docflow_verdict.ready)
            .to_string();
    closure_admission.status =
        crate::release_contract_adapters::release_contract_status(closure_admission.admitted)
            .to_string();
}

pub(crate) fn build_runtime_consumption_dispatch_receipt(
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
    let dispatch_target =
        canonical_dispatch_target_from_latest_status(role_selection, &latest_status)
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
                    super::runtime_assignment_from_execution_plan(&role_selection.execution_plan)
                        ["activation_agent_type"]
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
                    super::runtime_assignment_from_execution_plan(&role_selection.execution_plan)
                        ["activation_runtime_role"]
                        .as_str()
                        .map(str::to_string)
                })
        }
    });
    let selected_backend = super::downstream_selected_backend(
        role_selection,
        &dispatch_target,
        activation_agent_type.as_deref(),
        None,
    )
    .filter(|value| !value.is_empty());
    let dispatch_command =
        super::runtime_dispatch_command_for_target(role_selection, &dispatch_target);
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

fn canonical_dispatch_target_from_latest_status(
    role_selection: &super::RuntimeConsumptionLaneSelection,
    latest_status: &serde_json::Value,
) -> Option<String> {
    let next_node =
        super::json_string(latest_status.get("next_node")).filter(|value| !value.is_empty());
    next_node
        .as_deref()
        .and_then(|next_node| {
            super::dispatch_target_for_runtime_role(&role_selection.execution_plan, next_node)
                .or_else(|| Some(next_node.to_string()))
        })
        .or_else(|| {
            super::dispatch_target_for_runtime_role(
                &role_selection.execution_plan,
                &role_selection.selected_role,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{
        build_approval_delegation_evidence_gate, build_execution_preparation_evidence_gate,
        build_retrieval_policy_decision_gate, build_runtime_consumption_dispatch_receipt,
        normalize_runtime_consumption_statuses, parse_taskflow_consume_final_args,
        ApprovalDelegationEvidenceGate, ConsumeFinalMode, ExecutionPreparationEvidenceGate,
        RetrievalPolicyDecisionGate,
    };

    #[test]
    fn parse_taskflow_consume_final_args_supports_preview_and_validate_only_modes() {
        let preview_args = vec![
            "ship".to_string(),
            "this".to_string(),
            "--preview".to_string(),
            "--json".to_string(),
        ];
        let validate_args = vec![
            "ship".to_string(),
            "this".to_string(),
            "--validate-only".to_string(),
        ];

        let (preview_json, preview_mode, preview_request) =
            parse_taskflow_consume_final_args(&preview_args).expect("preview args should parse");
        let (validate_json, validate_mode, validate_request) =
            parse_taskflow_consume_final_args(&validate_args)
                .expect("validate-only args should parse");

        assert!(preview_json);
        assert_eq!(preview_mode, ConsumeFinalMode::Preview);
        assert_eq!(preview_request, "ship this");

        assert!(!validate_json);
        assert_eq!(validate_mode, ConsumeFinalMode::ValidateOnly);
        assert_eq!(validate_request, "ship this");
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_prefers_route_executor_backend() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string(), "development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "qwen_cli",
                        "subagents": "legacy_hint_should_not_win"
                    },
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-coach",
            "latest_status": {
                "next_node": "coach"
            }
        });

        let receipt =
            build_runtime_consumption_dispatch_receipt(&role_selection, &run_graph_bootstrap);

        assert_eq!(receipt.dispatch_target, "coach");
        assert_eq!(receipt.activation_agent_type.as_deref(), Some("middle"));
        assert_eq!(receipt.activation_runtime_role.as_deref(), Some("coach"));
        assert_eq!(receipt.selected_backend.as_deref(), Some("qwen_cli"));
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_canonicalizes_specification_target_from_business_analyst_alias(
    ) {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-specification",
            "latest_status": {
                "active_node": "planning",
                "next_node": "business_analyst",
                "route_task_class": "spec-pack",
                "task_class": "scope_discussion"
            }
        });

        let receipt =
            build_runtime_consumption_dispatch_receipt(&role_selection, &run_graph_bootstrap);

        assert_eq!(receipt.dispatch_target, "specification");
        assert_eq!(receipt.activation_agent_type.as_deref(), Some("middle"));
        assert_eq!(
            receipt.activation_runtime_role.as_deref(),
            Some("business_analyst")
        );
        assert_eq!(receipt.dispatch_command.as_deref(), Some("vida agent-init"));
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_keeps_agent_init_command_for_mixed_implementer_route() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue implementation".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["implementation".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementation": {
                        "executor_backend": "qwen_cli",
                        "fallback_executor_backend": "internal_subagents",
                        "activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-mixed-implementer",
            "latest_status": {
                "next_node": "implementer",
                "dispatch_command": "qwen --auth-type qwen-oauth -y -o json"
            }
        });

        let receipt =
            build_runtime_consumption_dispatch_receipt(&role_selection, &run_graph_bootstrap);

        assert_eq!(receipt.dispatch_target, "implementer");
        assert_eq!(receipt.dispatch_surface.as_deref(), Some("vida agent-init"));
        assert_eq!(receipt.selected_backend.as_deref(), Some("qwen_cli"));
        assert_eq!(receipt.dispatch_command.as_deref(), Some("vida agent-init"));
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_canonicalizes_real_bootstrap_shape_with_spec_pack_route_task_class(
    ) {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "lane_catalog": {
                            "specification": {
                                "activation_runtime_role": "business_analyst",
                                "activation_agent_type": "middle"
                            }
                        },
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-spec-bootstrap-shape",
            "latest_status": {
                "active_node": "planning",
                "next_node": "business_analyst",
                "route_task_class": "spec-pack",
                "task_class": "scope_discussion"
            }
        });

        let receipt =
            build_runtime_consumption_dispatch_receipt(&role_selection, &run_graph_bootstrap);

        assert_eq!(receipt.dispatch_target, "specification");
        assert_eq!(
            receipt.activation_runtime_role.as_deref(),
            Some("business_analyst")
        );
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_canonicalizes_specification_target_without_next_node() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "test".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue specification".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["research".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-specification-no-next-node",
            "latest_status": {
                "active_node": "planning",
                "route_task_class": "spec-pack",
                "task_class": "scope_discussion"
            }
        });

        let receipt =
            build_runtime_consumption_dispatch_receipt(&role_selection, &run_graph_bootstrap);

        assert_eq!(receipt.dispatch_target, "specification");
        assert_eq!(
            receipt.activation_runtime_role.as_deref(),
            Some("business_analyst")
        );
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_keeps_non_alias_dispatch_target() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue review".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["review".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "coach_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach",
                            "selected_agent_id": "middle"
                        }
                    }
                },
                "runtime_assignment": {
                    "selected_tier": "middle",
                    "activation_agent_type": "middle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-coach-stable",
            "latest_status": {
                "next_node": "coach",
                "route_task_class": "implementation"
            }
        });

        let receipt =
            build_runtime_consumption_dispatch_receipt(&role_selection, &run_graph_bootstrap);

        assert_eq!(receipt.dispatch_target, "coach");
        assert_eq!(receipt.activation_runtime_role.as_deref(), Some("coach"));
    }

    #[test]
    fn execution_preparation_gate_blocks_when_required_and_handoff_or_evidence_missing() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "architecture refactor implementation".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_preparation_required": true,
                        "lane_sequence": ["execution_preparation", "implementer"],
                        "lane_catalog": {
                            "execution_preparation": {
                                "completion_blocker": "pending_execution_preparation_evidence"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let taskflow_handoff_plan = serde_json::json!({
            "handoff_ready": false,
        });
        let run_graph_bootstrap = serde_json::json!({
            "handoff_ready": false,
            "execution_preparation_packet_path": "",
            "execution_preparation_evidence_ready": false,
        });

        let gate = build_execution_preparation_evidence_gate(
            &role_selection,
            &taskflow_handoff_plan,
            &run_graph_bootstrap,
        );

        assert_eq!(
            gate.blocker_code(),
            Some("pending_execution_preparation_evidence")
        );
    }

    #[test]
    fn execution_preparation_gate_passes_when_required_with_handoff_packet_and_evidence() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "architecture refactor implementation".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_preparation_required": true,
                        "lane_sequence": ["execution_preparation", "implementer"],
                        "lane_catalog": {
                            "execution_preparation": {
                                "completion_blocker": "pending_execution_preparation_evidence"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let taskflow_handoff_plan = serde_json::json!({
            "handoff_ready": true,
        });
        let run_graph_bootstrap = serde_json::json!({
            "handoff_ready": true,
            "execution_preparation_artifacts": {
                "handoff_ready": true,
                "developer_handoff_packet": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/packet.json"
                },
                "execution_preparation_evidence": {
                    "ready": true,
                    "status": "ready"
                }
            },
            "evidence": {
                "execution_preparation": {
                    "status": "ready",
                    "ready": true
                }
            }
        });

        let gate = build_execution_preparation_evidence_gate(
            &role_selection,
            &taskflow_handoff_plan,
            &run_graph_bootstrap,
        );

        assert_eq!(
            gate,
            ExecutionPreparationEvidenceGate {
                missing_evidence_or_handoff_packet: false
            }
        );
    }

    #[test]
    fn execution_preparation_gate_supports_legacy_bootstrap_fields_for_backward_compatibility() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "architecture refactor implementation".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "dispatch_contract": {
                        "execution_preparation_required": true,
                        "lane_sequence": ["execution_preparation", "implementer"],
                        "lane_catalog": {
                            "execution_preparation": {
                                "completion_blocker": "pending_execution_preparation_evidence"
                            }
                        }
                    }
                }
            }),
            reason: "test".to_string(),
        };

        let taskflow_handoff_plan = serde_json::json!({
            "handoff_ready": true,
        });
        let run_graph_bootstrap = serde_json::json!({
            "handoff_ready": true,
            "execution_preparation_packet_path": "/tmp/packet.json",
            "execution_preparation_handoff_packet_ready": true,
            "execution_preparation_evidence_ready": true,
            "evidence": {
                "execution_preparation": {
                    "status": "ready",
                    "ready": true
                }
            }
        });

        let gate = build_execution_preparation_evidence_gate(
            &role_selection,
            &taskflow_handoff_plan,
            &run_graph_bootstrap,
        );

        assert_eq!(
            gate,
            ExecutionPreparationEvidenceGate {
                missing_evidence_or_handoff_packet: false
            }
        );
    }

    #[tokio::test]
    async fn approval_delegation_gate_blocks_when_wait_branch_lacks_structured_receipt() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-approval-delegation-gate-block-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("open store");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implementation".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {
                    "mode": "delegated_orchestration_cycle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "latest_status": {
                "run_id": "run-approval-delegation",
                "handoff_state": "awaiting_approval",
                "policy_gate": "approval_required",
                "lifecycle_stage": "implementation_review_wait",
                "status": "awaiting_approval",
                "task_id": "run-approval-delegation",
                "task_class": "implementation",
                "route_task_class": "implementation",
                "active_node": "verification",
                "resume_target": "dispatch.approval"
            }
        });

        let gate =
            build_approval_delegation_evidence_gate(&store, &role_selection, &run_graph_bootstrap)
                .await;
        assert_eq!(
            gate.blocker_code(),
            Some("pending_approval_delegation_evidence")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn approval_delegation_gate_passes_when_latest_status_is_absent_for_fresh_consume_final_bootstrap(
    ) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-approval-delegation-gate-fresh-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("open store");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implementation".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {
                    "mode": "delegated_orchestration_cycle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "seed": {
                "run_id": "run-fresh-bootstrap"
            },
            "status": "seeded",
            "handoff_ready": true
        });

        let gate =
            build_approval_delegation_evidence_gate(&store, &role_selection, &run_graph_bootstrap)
                .await;
        assert_eq!(
            gate,
            ApprovalDelegationEvidenceGate {
                missing_approval_or_delegation_evidence: false
            }
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn approval_delegation_gate_passes_when_completion_receipt_is_route_bound() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-approval-delegation-gate-pass-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("open store");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "implementation".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {
                    "mode": "delegated_orchestration_cycle"
                }
            }),
            reason: "test".to_string(),
        };
        let status = crate::state_store::RunGraphStatus {
            run_id: "run-approval-delegation".to_string(),
            task_id: "run-approval-delegation".to_string(),
            task_class: "implementation".to_string(),
            active_node: "verification".to_string(),
            next_node: None,
            status: "completed".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "codex".to_string(),
            lane_id: "verification_lane".to_string(),
            lifecycle_stage: "implementation_complete".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            recovery_ready: false,
        };
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist completion receipt");

        let run_graph_bootstrap = serde_json::json!({
            "latest_status": {
                "run_id": status.run_id,
                "task_id": status.task_id,
                "task_class": status.task_class,
                "route_task_class": status.route_task_class,
                "active_node": status.active_node,
                "status": status.status,
                "lifecycle_stage": status.lifecycle_stage,
                "policy_gate": status.policy_gate,
                "handoff_state": status.handoff_state,
                "resume_target": status.resume_target,
            }
        });

        let gate =
            build_approval_delegation_evidence_gate(&store, &role_selection, &run_graph_bootstrap)
                .await;
        assert_eq!(
            gate,
            ApprovalDelegationEvidenceGate {
                missing_approval_or_delegation_evidence: false
            }
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn approval_delegation_gate_blocks_when_receipt_drift_breaks_governance_match() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-approval-delegation-gate-drift-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::StateStore::open(root.clone())
            .await
            .expect("open store");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "identity policy change".to_string(),
            selected_role: "orchestrator".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "orchestration_contract": {
                    "mode": "delegated_orchestration_cycle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "latest_status": {
                "run_id": "run-identity-policy",
                "handoff_state": "awaiting_approval",
                "policy_gate": "approval_required",
                "lifecycle_stage": "implementation_review_wait",
                "status": "awaiting_approval",
                "task_id": "run-identity-policy",
                "task_class": "identity_or_policy_change",
                "route_task_class": "identity_or_policy_change",
                "active_node": "approval",
                "resume_target": "dispatch.approval"
            }
        });

        store
            .record_run_graph_approval_delegation_receipt(
                &crate::state_store::RunGraphApprovalDelegationReceipt {
                    receipt_id: "run-graph-approval-delegation-run-identity-policy-stale"
                        .to_string(),
                    run_id: "run-identity-policy".to_string(),
                    task_id: "run-identity-policy".to_string(),
                    task_class: "implementation".to_string(),
                    route_task_class: "implementation".to_string(),
                    active_node: "approval".to_string(),
                    next_node: None,
                    status: "completed".to_string(),
                    lifecycle_stage: "implementation_complete".to_string(),
                    policy_gate: "not_required".to_string(),
                    handoff_state: "none".to_string(),
                    resume_target: "none".to_string(),
                    transition_kind: "approval_complete".to_string(),
                    recorded_at: "2026-04-20T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist stale approval/delegation receipt");

        let gate =
            build_approval_delegation_evidence_gate(&store, &role_selection, &run_graph_bootstrap)
                .await;
        assert_eq!(
            gate.blocker_code(),
            Some("pending_approval_delegation_evidence"),
            "stale governance receipts must fail closed for identity/policy-changing workflows"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn release1_runtime_consumption_statuses_are_emitted_as_pass_or_blocked() {
        let mut docflow_verdict = crate::RuntimeConsumptionDocflowVerdict {
            status: "blocked".to_string(),
            ready: false,
            blockers: vec![crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::MissingProofVerdict,
            )
            .expect("missing proof verdict blocker should be canonical")],
            proof_surfaces: vec![],
        };
        let mut closure_admission = crate::RuntimeConsumptionClosureAdmission {
            status: "blocked".to_string(),
            admitted: false,
            blockers: vec![crate::release1_contracts::blocker_code_value(
                crate::release1_contracts::BlockerCode::MissingClosureProof,
            )
            .expect("missing closure proof blocker should be canonical")],
            proof_surfaces: vec![],
        };

        normalize_runtime_consumption_statuses(&mut docflow_verdict, &mut closure_admission);
        assert_eq!(docflow_verdict.status, "blocked");
        assert_eq!(closure_admission.status, "blocked");

        docflow_verdict.ready = true;
        closure_admission.admitted = true;
        normalize_runtime_consumption_statuses(&mut docflow_verdict, &mut closure_admission);
        assert_eq!(docflow_verdict.status, "pass");
        assert_eq!(closure_admission.status, "pass");
    }

    #[test]
    fn retrieval_policy_gate_blocks_when_protocol_binding_is_not_ready() {
        let bundle_check = crate::TaskflowConsumeBundleCheck {
            ok: false,
            blockers: vec![
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::MissingProtocolBindingReceipt,
                )
                .to_string(),
                crate::release1_contracts::blocker_code_str(
                    crate::release1_contracts::BlockerCode::ProtocolBindingNotRuntimeReady,
                )
                .to_string(),
            ],
            root_artifact_id: "artifact-1".to_string(),
            artifact_count: 1,
            boot_classification: "compatible".to_string(),
            migration_state: "stable".to_string(),
            activation_status: "ready".to_string(),
        };

        let gate = build_retrieval_policy_decision_gate(&bundle_check);
        assert_eq!(
            gate,
            RetrievalPolicyDecisionGate {
                blocker_code: Some(
                    crate::release1_contracts::blocker_code_str(
                        crate::release1_contracts::BlockerCode::MissingProtocolBindingReceipt,
                    )
                    .to_string()
                )
            }
        );
    }

    #[test]
    fn parse_taskflow_consume_final_args_separates_preview_flags_from_request_text() {
        let args = vec![
            "fix".to_string(),
            "dispatch".to_string(),
            "--preview".to_string(),
            "--json".to_string(),
        ];
        let (as_json, mode, request_text) =
            super::parse_taskflow_consume_final_args(&args).expect("final args should parse");

        assert!(as_json);
        assert_eq!(mode, super::ConsumeFinalMode::Preview);
        assert_eq!(request_text, "fix dispatch");
    }

    #[test]
    fn parse_taskflow_consume_final_args_supports_validate_only_mode() {
        let args = vec![
            "--validate-only".to_string(),
            "shape".to_string(),
            "packet".to_string(),
        ];
        let (as_json, mode, request_text) = super::parse_taskflow_consume_final_args(&args)
            .expect("validate-only args should parse");

        assert!(!as_json);
        assert_eq!(mode, super::ConsumeFinalMode::ValidateOnly);
        assert_eq!(request_text, "shape packet");
    }
}
