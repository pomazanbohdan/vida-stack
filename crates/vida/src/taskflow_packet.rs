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

fn build_taskflow_packet_render_payload(
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    dispatch_packet_path: &str,
    dispatch_packet_body: serde_json::Value,
    downstream_packet: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida taskflow packet render",
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

    let (run_id, as_json) = match args {
        [head, subcommand, run_id] if head == "packet" && subcommand == "render" => {
            (run_id.clone(), false)
        }
        [head, subcommand, run_id, flag]
            if head == "packet" && subcommand == "render" && matches!(flag.as_str(), "--json") =>
        {
            (run_id.clone(), true)
        }
        [head, subcommand, flag]
            if head == "packet"
                && subcommand == "render"
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            crate::taskflow_layer4::print_taskflow_proxy_help(Some("packet"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("Usage: vida taskflow packet render <run-id> [--json]");
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
    let Some(receipt) = (match store.run_graph_dispatch_receipt(&run_id).await {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("Failed to read persisted dispatch receipt for `{run_id}`: {error}");
            return ExitCode::from(1);
        }
    }) else {
        eprintln!(
            "No persisted run-graph dispatch receipt exists for run_id `{run_id}`; run `vida taskflow run-graph dispatch-init {run_id} --json` first."
        );
        return ExitCode::from(1);
    };

    let dispatch_packet_path = match receipt.dispatch_packet_path.as_deref() {
        Some(path) if !path.trim().is_empty() => path,
        _ => {
            eprintln!("Persisted dispatch receipt for `{run_id}` is missing dispatch_packet_path.");
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
        &run_id,
        &receipt,
        dispatch_packet_path,
        dispatch_packet_body,
        downstream_packet,
    );

    if as_json {
        crate::print_json_pretty(&payload);
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow packet render");
        print_surface_line(RenderMode::Plain, "run", &receipt.run_id);
        print_surface_line(
            RenderMode::Plain,
            "dispatch_target",
            &receipt.dispatch_target,
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
    use super::build_taskflow_packet_render_payload;

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
            &receipt,
            "/tmp/dispatch-packet.json",
            serde_json::json!({
                "dispatch_target": "implementer",
                "selected_backend": "opencode_cli"
            }),
            None,
        );

        assert_eq!(payload["dispatch_receipt"]["selected_backend"], "opencode_cli");
        assert_eq!(
            payload["dispatch_packet"]["body"]["selected_backend"],
            "opencode_cli"
        );
    }
}
