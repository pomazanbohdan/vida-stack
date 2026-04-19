use std::process::ExitCode;

use crate::{
    print_surface_header, print_surface_line, state_store::StateStore,
    taskflow_task_bridge::proxy_state_dir, RenderMode,
};

fn read_packet_body(path: &str) -> Result<serde_json::Value, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted packet `{path}`: {error}"))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to decode persisted packet `{path}`: {error}"))
}

async fn resolve_packet_render_run_id(
    store: &StateStore,
    requested_run_id: &str,
) -> Result<String, String> {
    let binding = store
        .run_graph_continuation_binding(requested_run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read explicit continuation binding for `{requested_run_id}`: {error}"
            )
        })?;
    let Some(binding) = binding else {
        return Ok(requested_run_id.to_string());
    };
    if binding.status != "bound"
        || binding.active_bounded_unit["kind"].as_str() != Some("task_graph_task")
    {
        return Ok(requested_run_id.to_string());
    }

    let bound_task_id = binding.active_bounded_unit["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(binding.task_id.as_str());
    if bound_task_id == requested_run_id {
        return Ok(requested_run_id.to_string());
    }

    let bound_receipt = store
        .run_graph_dispatch_receipt(bound_task_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read fresh dispatch receipt for bound task `{bound_task_id}`: {error}"
            )
        })?;
    if bound_receipt.is_none() {
        return Err(format!(
            "Run `{requested_run_id}` has explicit continuation binding to task_graph_task `{bound_task_id}`, but no fresh persisted dispatch receipt exists for the bound task. Run `vida taskflow run-graph dispatch-init {bound_task_id} --json` first."
        ));
    }

    Ok(bound_task_id.to_string())
}

async fn resolve_latest_packet_run_id(store: &StateStore) -> Result<String, String> {
    let Some(receipt) = store
        .latest_run_graph_dispatch_receipt()
        .await
        .map_err(|error| format!("Failed to read latest persisted dispatch receipt: {error}"))?
    else {
        return Err(
            "No latest persisted run-graph dispatch receipt exists; run `vida taskflow run-graph dispatch-init <task-id> --json` first."
                .to_string(),
        );
    };
    Ok(receipt.run_id)
}

fn build_taskflow_packet_render_payload(
    requested_run_id: &str,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: &str,
    dispatch_packet_body: serde_json::Value,
    downstream_packet: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida taskflow packet render",
        "requested_run_id": requested_run_id,
        "run_id": run_id,
        "dispatch_receipt": receipt,
        "dispatch_packet": {
            "path": dispatch_packet_path,
            "body": dispatch_packet_body,
        },
        "downstream_dispatch_packet": downstream_packet,
        "lawful_resume_inputs": {
            "run_id": run_id,
            "dispatch_packet_path": dispatch_packet_path,
            "downstream_dispatch_packet_path": receipt.downstream_dispatch_packet_path,
            "continue_command": format!("vida taskflow consume continue --run-id {} --json", receipt.run_id),
        }
    })
}

fn preview_value<'a>(body: &'a serde_json::Value, section: &str, key: &str) -> &'a str {
    body.get(section)
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("none")
}

fn preview_bool(body: &serde_json::Value, section: &str, key: &str) -> bool {
    body.get(section)
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) async fn run_taskflow_packet(args: &[String]) -> ExitCode {
    match args {
        [head] if head == "packet" => {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("packet"));
            return ExitCode::SUCCESS;
        }
        [head, flag] if head == "packet" && matches!(flag.as_str(), "--help" | "-h") => {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("packet"));
            return ExitCode::SUCCESS;
        }
        _ => {}
    }

    let (requested_run_id, as_json, latest_mode) = match args {
        [head, subcommand, run_id] if head == "packet" && subcommand == "render" => {
            (run_id.clone(), false, false)
        }
        [head, subcommand, run_id, flag]
            if head == "packet" && subcommand == "render" && matches!(flag.as_str(), "--json") =>
        {
            (run_id.clone(), true, false)
        }
        [head, subcommand] if head == "packet" && subcommand == "latest" => {
            ("latest".to_string(), false, true)
        }
        [head, subcommand, flag]
            if head == "packet" && subcommand == "latest" && matches!(flag.as_str(), "--json") =>
        {
            ("latest".to_string(), true, true)
        }
        [head, subcommand, flag]
            if head == "packet"
                && matches!(subcommand.as_str(), "render" | "latest")
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("packet"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!(
                "Usage: vida taskflow packet render <run-id> [--json]\n       vida taskflow packet latest [--json]"
            );
            return ExitCode::from(2);
        }
    };

    let store = match StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let run_id = if latest_mode {
        match resolve_latest_packet_run_id(&store).await {
            Ok(run_id) => run_id,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    } else {
        requested_run_id.clone()
    };
    let effective_run_id = match resolve_packet_render_run_id(&store, &run_id).await {
        Ok(run_id) => run_id,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let Some(receipt) = (match store.run_graph_dispatch_receipt(&effective_run_id).await {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!(
                "Failed to read persisted dispatch receipt for `{effective_run_id}`: {error}"
            );
            return ExitCode::from(1);
        }
    }) else {
        eprintln!(
            "No persisted run-graph dispatch receipt exists for run_id `{effective_run_id}`; run `vida taskflow run-graph dispatch-init {run_id} --json` first."
        );
        return ExitCode::from(1);
    };

    let dispatch_packet_path = match receipt.dispatch_packet_path.as_deref() {
        Some(path) if !path.trim().is_empty() => path,
        _ => {
            eprintln!(
                "Persisted dispatch receipt for `{effective_run_id}` is missing dispatch_packet_path."
            );
            return ExitCode::from(1);
        }
    };
    let dispatch_packet_body = match read_packet_body(dispatch_packet_path) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let downstream_packet = match receipt.downstream_dispatch_packet_path.as_deref() {
        Some(path) if !path.trim().is_empty() => match read_packet_body(path) {
            Ok(body) => Some(serde_json::json!({
                "path": path,
                "body": body,
            })),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        },
        _ => None,
    };

    let payload = build_taskflow_packet_render_payload(
        &requested_run_id,
        &effective_run_id,
        &receipt,
        dispatch_packet_path,
        dispatch_packet_body.clone(),
        downstream_packet,
    );

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow packet render");
        if latest_mode {
            print_surface_line(RenderMode::Plain, "requested", "latest");
        }
        print_surface_line(RenderMode::Plain, "run", &receipt.run_id);
        print_surface_line(
            RenderMode::Plain,
            "dispatch_target",
            &receipt.dispatch_target,
        );
        print_surface_line(
            RenderMode::Plain,
            "selected_backend",
            preview_value(&dispatch_packet_body, "route_policy", "effective_selected_backend"),
        );
        print_surface_line(
            RenderMode::Plain,
            "route_policy",
            &format!(
                "primary_backend={} backend_source={} posture={}",
                preview_value(&dispatch_packet_body, "route_policy", "route_primary_backend"),
                preview_value(&dispatch_packet_body, "route_policy", "selected_backend_source"),
                preview_value(
                    &dispatch_packet_body,
                    "effective_execution_posture",
                    "effective_posture_kind"
                ),
            ),
        );
        print_surface_line(
            RenderMode::Plain,
            "execution_posture",
            &format!(
                "selected_execution_class={} mixed_route_backends={} activation_evidence_state={}",
                preview_value(
                    &dispatch_packet_body,
                    "effective_execution_posture",
                    "selected_execution_class"
                ),
                preview_bool(
                    &dispatch_packet_body,
                    "effective_execution_posture",
                    "mixed_route_backends"
                ),
                preview_value(
                    &dispatch_packet_body,
                    "effective_execution_posture",
                    "activation_evidence_state"
                ),
            ),
        );
        print_surface_line(RenderMode::Plain, "dispatch_packet", dispatch_packet_path);
        if let Some(path) = receipt.downstream_dispatch_packet_path.as_deref() {
            print_surface_line(RenderMode::Plain, "downstream_packet", path);
        }
        print_surface_line(
            RenderMode::Plain,
            "continue_command",
            &format!(
                "vida taskflow consume continue --run-id {} --json",
                receipt.run_id
            ),
        );
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        build_taskflow_packet_render_payload, resolve_latest_packet_run_id,
        resolve_packet_render_run_id,
    };
    use crate::state_store::StateStore;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn packet_render_payload_preserves_persisted_selected_backend_truth() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-impl".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec![],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };

        let payload = build_taskflow_packet_render_payload(
            "run-impl",
            "run-impl",
            &receipt,
            "/tmp/dispatch-packet.json",
            serde_json::json!({
                "dispatch_target": "implementer",
                "selected_backend": "opencode_cli"
            }),
            None,
        );

        assert_eq!(
            payload["dispatch_receipt"]["selected_backend"],
            "opencode_cli"
        );
        assert_eq!(
            payload["dispatch_packet"]["body"]["selected_backend"],
            "opencode_cli"
        );
    }

    #[tokio::test]
    async fn packet_render_redirects_explicit_task_binding_to_fresh_bound_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-packet-render-explicit-task-binding-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: "run-old".to_string(),
                    task_id: "task-new".to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "task_graph_task",
                        "task_id": "task-new",
                        "run_id": "run-old",
                        "task_status": "in_progress",
                        "issue_type": "task"
                    }),
                    binding_source: "explicit_continuation_bind_task".to_string(),
                    why_this_unit: "reseed onto task-new".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_explicit_task_bound"
                        .to_string(),
                    request_text: Some("continue".to_string()),
                    recorded_at: "2026-04-16T09:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist binding");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "task-new".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "ready".to_string(),
                lane_status: "lane_ready".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida taskflow run-graph dispatch-init".to_string()),
                dispatch_command: Some(
                    "vida taskflow consume continue --run-id task-new --json".to_string(),
                ),
                dispatch_packet_path: Some("/tmp/task-new-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("opencode_cli".to_string()),
                recorded_at: "2026-04-16T10:00:00Z".to_string(),
            })
            .await
            .expect("persist bound receipt");

        let effective_run_id = resolve_packet_render_run_id(&store, "run-old")
            .await
            .expect("packet render should redirect to bound task receipt");

        assert_eq!(effective_run_id, "task-new");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_packet_run_id_reads_latest_receipt_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-packet-render-latest-run-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .record_run_graph_status(&crate::state_store::RunGraphStatus {
                run_id: "run-latest".to_string(),
                task_id: "task-latest".to_string(),
                task_class: "delivery_task".to_string(),
                active_node: "implementer".to_string(),
                next_node: None,
                status: "in_progress".to_string(),
                route_task_class: "delivery_task".to_string(),
                selected_backend: "opencode_cli".to_string(),
                lane_id: "lane-latest".to_string(),
                lifecycle_stage: "implementer_ready".to_string(),
                policy_gate: "none".to_string(),
                handoff_state: "none".to_string(),
                context_state: "sealed".to_string(),
                checkpoint_kind: "dispatch".to_string(),
                resume_target: "dispatch.implementer".to_string(),
                recovery_ready: true,
            })
            .await
            .expect("persist status");
        store
            .record_run_graph_dispatch_receipt(&crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-latest".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "ready".to_string(),
                lane_status: "lane_ready".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida taskflow run-graph dispatch-init".to_string()),
                dispatch_command: Some(
                    "vida taskflow consume continue --run-id run-latest --json".to_string(),
                ),
                dispatch_packet_path: Some("/tmp/run-latest-packet.json".to_string()),
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec![],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: None,
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("opencode_cli".to_string()),
                recorded_at: "2026-04-19T12:00:02Z".to_string(),
            })
            .await
            .expect("persist latest receipt");

        let resolved = resolve_latest_packet_run_id(&store)
            .await
            .expect("resolve latest run");
        assert_eq!(resolved, "run-latest");

        let _ = fs::remove_dir_all(&root);
    }
}
