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

    let payload = serde_json::json!({
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
    });

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
