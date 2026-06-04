use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

use crate::operator_contracts::{
    finalize_release1_operator_truth, shared_operator_output_contract_parity_error,
    FinalizedRelease1OperatorTruth,
};
use crate::release1_contracts::{blocker_code_str, BlockerCode};
use crate::surface_render::print_surface_json;
use crate::{
    print_surface_header, print_surface_line, RuntimeArgs, RuntimeCommand, RuntimeWebCommand,
    RuntimeWebRestartArgs, RuntimeWebStatusArgs,
};

const RUNTIME_WEB_STATUS_SURFACE: &str = "vida runtime web status";
const RUNTIME_WEB_RESTART_SURFACE: &str = "vida runtime web restart";
const RESTART_EXECUTOR_BLOCKER: BlockerCode = BlockerCode::ToolContractMissing;
const STATUS_STALE_LISTENER_BLOCKER: BlockerCode = BlockerCode::OwnerSurfaceContradiction;
const RESTART_EXECUTOR_NEXT_ACTION: &str = "VIDA does not execute project-local runtime web restart adapters automatically; rerun with --dry-run to inspect the restart plan, then manually run only trusted reviewed scripts outside VIDA.";
const STATUS_STALE_LISTENER_NEXT_ACTION: &str = "Run `vida runtime web restart --scope current-repo --include-edge-proxy --dry-run --json` to inspect current-repo web proof listeners, then manually stop only reviewed stale processes.";
const RUNTIME_WEB_SAFE_RESTART_COMMAND: &str =
    "vida runtime web restart --scope current-repo --include-edge-proxy --dry-run --json";
const LOCAL_WEB_ADAPTER: &str = "scripts/windows/Start-WebDevServer.ps1";
const EDGE_PROXY_ADAPTER: &str = "scripts/windows/Start-WebCloudflareEdgeProxy.ps1";
const PROJECT_ADAPTER_EXECUTION_DISABLED_REASON: &str =
    "automatic execution of project-local runtime web restart adapters is disabled for safety";
const PROCESS_SNAPSHOT_ENV: &str = "VIDA_RUNTIME_WEB_STATUS_PROCESS_SNAPSHOT";

pub(crate) async fn run_runtime(args: RuntimeArgs) -> ExitCode {
    match args.command {
        RuntimeCommand::Web(args) => match args.command {
            RuntimeWebCommand::Status(args) => run_runtime_web_status(args),
            RuntimeWebCommand::Restart(args) => run_runtime_web_restart(args),
        },
    }
}

fn run_runtime_web_status(args: RuntimeWebStatusArgs) -> ExitCode {
    let payload = build_runtime_web_status_payload(&args);
    let printed_json = print_surface_json(
        &payload,
        args.json,
        "runtime web status payload should render as json",
    );

    if !printed_json {
        print_runtime_web_status_plain(&payload);
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn build_runtime_web_status_payload(args: &RuntimeWebStatusArgs) -> Value {
    match crate::resolve_runtime_project_root() {
        Ok(project_root) => build_runtime_web_status_payload_for_project_root(args, &project_root),
        Err(error) => {
            build_runtime_web_status_payload_for_unresolved_root(args, &error.to_string())
        }
    }
}

fn build_runtime_web_status_payload_for_unresolved_root(
    args: &RuntimeWebStatusArgs,
    error: &str,
) -> Value {
    let project_root = format!("unresolved: {error}");
    let blocker_codes = vec![blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string()];
    let next_actions =
        vec!["Resolve the VIDA project root before inspecting runtime web services.".to_string()];
    let artifact_refs = serde_json::json!({
        "surface": RUNTIME_WEB_STATUS_SURFACE,
        "scope": args.scope,
        "include_edge_proxy": args.include_edge_proxy,
        "project_root": project_root,
    });
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)
        .expect("runtime web status operator truth should finalize");
    runtime_web_status_payload_from_parts(
        args,
        project_root,
        "blocked_project_root_unresolved",
        finalized,
        serde_json::json!([]),
        serde_json::json!([]),
        serde_json::json!([]),
        serde_json::json!({
            "status": "unavailable",
            "stale_process_count": 0,
            "safe_restart_command": RUNTIME_WEB_SAFE_RESTART_COMMAND,
            "runtime_owner_evidence": {
                "status": "unavailable",
                "reason": "project_root_unresolved",
                "persist_current": false,
            },
        }),
    )
}

fn build_runtime_web_status_payload_for_project_root(
    args: &RuntimeWebStatusArgs,
    project_root: &Path,
) -> Value {
    let adapter_plan = RuntimeWebRestartAdapterPlan::discover(project_root);
    let processes = discover_runtime_web_processes(project_root);
    let components =
        runtime_web_status_components(args.include_edge_proxy, &adapter_plan, &processes);
    let stale_processes = runtime_web_stale_processes(&components);
    let process_conflict_diagnostics =
        runtime_web_process_conflict_diagnostics(project_root, &stale_processes);
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mode = if stale_processes
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        blocker_codes.push(blocker_code_str(STATUS_STALE_LISTENER_BLOCKER).to_string());
        next_actions.push(STATUS_STALE_LISTENER_NEXT_ACTION.to_string());
        "stale_listener_conflict"
    } else {
        "inspected"
    };
    let artifact_refs = serde_json::json!({
        "surface": RUNTIME_WEB_STATUS_SURFACE,
        "scope": args.scope,
        "include_edge_proxy": args.include_edge_proxy,
        "project_root": project_root.display().to_string(),
    });
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)
        .expect("runtime web status operator truth should finalize");
    runtime_web_status_payload_from_parts(
        args,
        project_root.display().to_string(),
        mode,
        finalized,
        components,
        stale_processes,
        serde_json::json!(processes),
        process_conflict_diagnostics,
    )
}

fn runtime_web_status_payload_from_parts(
    args: &RuntimeWebStatusArgs,
    project_root: String,
    mode: &str,
    finalized: FinalizedRelease1OperatorTruth,
    components: Value,
    stale_processes: Value,
    process_snapshot: Value,
    process_conflict_diagnostics: Value,
) -> Value {
    let mut payload = serde_json::json!({
        "surface": RUNTIME_WEB_STATUS_SURFACE,
        "status": finalized.status,
        "trace_id": finalized.operator_contracts["trace_id"].clone(),
        "workflow_class": finalized.operator_contracts["workflow_class"].clone(),
        "risk_tier": finalized.operator_contracts["risk_tier"].clone(),
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
        "web_status": {
            "scope": args.scope,
            "include_edge_proxy": args.include_edge_proxy,
            "project_root": project_root,
            "mode": mode,
            "components": components,
            "stale_processes": stale_processes,
            "process_snapshot": process_snapshot,
            "safe_restart_command": RUNTIME_WEB_SAFE_RESTART_COMMAND,
            "process_conflict_diagnostics": process_conflict_diagnostics,
        },
        "components": components,
        "stale_processes": stale_processes,
        "process_conflict_diagnostics": process_conflict_diagnostics,
    });
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
    assert_eq!(
        shared_operator_output_contract_parity_error(&payload),
        None,
        "runtime web status payload should keep release-1 parity"
    );
    payload
}

fn runtime_web_process_conflict_diagnostics(project_root: &Path, stale_processes: &Value) -> Value {
    let stale_process_count = stale_processes
        .as_array()
        .map(|items| items.len())
        .unwrap_or(0);
    let runtime_owner_evidence = crate::orchestrator_session_surface::build_runtime_owner_evidence(
        &project_root.join(".vida/data/state"),
        false,
    )
    .map(crate::orchestrator_session_surface::compact_runtime_owner_evidence_for_operator)
    .unwrap_or_else(|error| {
        serde_json::json!({
            "status": "unavailable",
            "error": error,
            "persist_current": false,
        })
    });
    serde_json::json!({
        "status": if stale_process_count == 0 { "pass" } else { "stale_process_conflict" },
        "stale_process_count": stale_process_count,
        "safe_restart_command": RUNTIME_WEB_SAFE_RESTART_COMMAND,
        "runtime_owner_evidence": runtime_owner_evidence,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
struct RuntimeWebProcessSnapshot {
    process_id: u32,
    command_line: String,
    component_id: Option<&'static str>,
    ownership: String,
    owner_class: String,
    owner_root: Option<String>,
    owner_root_source: String,
    working_directory: Option<String>,
    working_directory_source: String,
    ports: Vec<u16>,
    safe_restart_command: String,
}

fn discover_runtime_web_processes(project_root: &Path) -> Vec<RuntimeWebProcessSnapshot> {
    if let Ok(raw) = std::env::var(PROCESS_SNAPSHOT_ENV) {
        return parse_runtime_web_process_snapshot(&raw, project_root);
    }
    if !cfg!(windows) {
        return Vec::new();
    }
    let script = "$pattern = '(-File\\s+.*(Start-WebDevServer|Start-WebOdooProxy|Start-WebCloudflareEdgeProxy)\\.ps1|flutter-wrapper\\.cmd|flutter.*web-server)'; Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and ($_.CommandLine -match $pattern) } | Select-Object @{Name='process_id';Expression={$_.ProcessId}},@{Name='command_line';Expression={$_.CommandLine}} | ConvertTo-Json -Depth 3";
    let Ok(output) = Command::new(trusted_windows_powershell_executable())
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_runtime_web_process_snapshot(&String::from_utf8_lossy(&output.stdout), project_root)
}

fn trusted_windows_powershell_executable() -> PathBuf {
    PathBuf::from(trusted_windows_powershell_executable_from_system_root(
        std::env::var("SystemRoot").ok().as_deref(),
    ))
}

fn trusted_windows_powershell_executable_from_system_root(system_root: Option<&str>) -> String {
    let system_root = system_root
        .filter(|path| is_windows_drive_absolute_path(path))
        .unwrap_or(r"C:\Windows");
    format!(r"{system_root}\System32\WindowsPowerShell\v1.0\powershell.exe")
}

fn is_windows_drive_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn parse_runtime_web_process_snapshot(
    raw: &str,
    project_root: &Path,
) -> Vec<RuntimeWebProcessSnapshot> {
    let Ok(value) = serde_json::from_str::<Value>(raw.trim()) else {
        return Vec::new();
    };
    let values = match value {
        Value::Array(values) => values,
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    let project_root = normalize_process_path(&project_root.display().to_string());
    values
        .into_iter()
        .filter_map(|value| {
            let process_id = value
                .get("process_id")
                .or_else(|| value.get("ProcessId"))
                .and_then(serde_json::Value::as_u64)? as u32;
            let command_line = value
                .get("command_line")
                .or_else(|| value.get("CommandLine"))
                .and_then(serde_json::Value::as_str)?
                .to_string();
            let normalized = normalize_process_path(&command_line);
            let component_id = runtime_web_component_for_command_line(&normalized);
            let owner_root = infer_runtime_web_owner_root(&command_line);
            let normalized_owner_root = owner_root
                .as_deref()
                .map(normalize_process_path)
                .unwrap_or_default();
            let ownership =
                if normalized_owner_root == project_root || normalized.contains(&project_root) {
                    "current_repo"
                } else {
                    "stale_foreign_repo"
                };
            let owner_class = runtime_web_owner_class(ownership, &normalized_owner_root);
            let ports = infer_runtime_web_ports(&command_line);
            component_id.map(|component_id| RuntimeWebProcessSnapshot {
                process_id,
                command_line,
                component_id: Some(component_id),
                ownership: ownership.to_string(),
                owner_class: owner_class.to_string(),
                owner_root: owner_root.clone(),
                owner_root_source: if owner_root.is_some() {
                    "command_line_script_path".to_string()
                } else {
                    "unresolved".to_string()
                },
                working_directory: owner_root,
                working_directory_source: "inferred_from_command_line_script_path".to_string(),
                ports,
                safe_restart_command: RUNTIME_WEB_SAFE_RESTART_COMMAND.to_string(),
            })
        })
        .collect()
}

fn runtime_web_owner_class(ownership: &str, normalized_owner_root: &str) -> &'static str {
    if ownership == "current_repo" {
        "current_repo"
    } else if normalized_owner_root.is_empty() {
        "unresolved_owner"
    } else if normalized_owner_root.contains("/.codex/worktrees/")
        || normalized_owner_root.contains("/.vida/worktrees/")
    {
        "stale_worktree"
    } else {
        "foreign_project_root"
    }
}

fn normalize_process_path(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

fn runtime_web_component_for_command_line(command_line: &str) -> Option<&'static str> {
    let command_line = command_line
        .replace('\'', "")
        .replace('"', "")
        .to_ascii_lowercase();
    if command_line.contains("get-ciminstance win32_process") {
        return None;
    }
    let file_script = command_line.contains("-file ");
    if file_script && command_line.contains("start-webcloudflareedgeproxy.ps1") {
        Some("edge_proxy")
    } else if file_script && command_line.contains("start-webodooproxy.ps1") {
        Some("local_proxy")
    } else if (file_script && command_line.contains("start-webdevserver.ps1"))
        || command_line.contains("flutter-wrapper")
        || (command_line.contains("flutter") && command_line.contains("web-server"))
    {
        Some("local_web_upstream")
    } else {
        None
    }
}

fn infer_runtime_web_owner_root(command_line: &str) -> Option<String> {
    let normalized = command_line.replace('\\', "/");
    for marker in [
        "/scripts/windows/Start-WebDevServer.ps1",
        "/scripts/windows/Start-WebOdooProxy.ps1",
        "/scripts/windows/Start-WebCloudflareEdgeProxy.ps1",
        "/scripts/windows/flutter-wrapper.cmd",
    ] {
        if let Some(index) = normalized
            .to_ascii_lowercase()
            .find(&marker.to_ascii_lowercase())
        {
            return Some(trim_to_runtime_web_path_start(&normalized[..index]));
        }
    }
    None
}

fn trim_to_runtime_web_path_start(value: &str) -> String {
    let trimmed = value.trim_matches('"').trim_matches('\'').trim();
    let bytes = trimmed.as_bytes();
    let mut path_start = None;
    for index in 0..bytes.len().saturating_sub(2) {
        if bytes[index].is_ascii_alphabetic()
            && bytes[index + 1] == b':'
            && bytes[index + 2] == b'/'
        {
            path_start = Some(index);
        }
    }
    path_start
        .map(|index| trimmed[index..].to_string())
        .unwrap_or_else(|| trimmed.to_string())
}

fn infer_runtime_web_ports(command_line: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for flag in [
        "-WebPort",
        "-ProxyPort",
        "-Port",
        "-FlutterPort",
        "-OdooProxyPort",
        "--web-port",
    ] {
        if let Some(port) = infer_port_after_flag(command_line, flag) {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
    ports
}

fn infer_port_after_flag(command_line: &str, flag: &str) -> Option<u16> {
    let lower = command_line.to_ascii_lowercase();
    let flag_lower = flag.to_ascii_lowercase();
    let index = lower.find(&flag_lower)?;
    let after = &command_line[index + flag.len()..];
    let digits = after
        .trim_start_matches([' ', '=', ':'])
        .chars()
        .skip_while(|ch| *ch == '"' || *ch == '\'')
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u16>().ok()
}

fn runtime_web_status_components(
    include_edge_proxy: bool,
    adapter_plan: &RuntimeWebRestartAdapterPlan,
    processes: &[RuntimeWebProcessSnapshot],
) -> Value {
    serde_json::json!([
        runtime_web_status_component(
            "local_web_upstream",
            "web_upstream",
            adapter_plan.local_web_script.as_ref(),
            vec![51235, 51237],
            processes,
            true,
        ),
        runtime_web_status_component(
            "local_proxy",
            "proxy",
            adapter_plan.local_web_script.as_ref(),
            vec![51236],
            processes,
            true,
        ),
        runtime_web_status_component(
            "edge_proxy",
            "edge_proxy",
            adapter_plan.edge_proxy_script.as_ref(),
            vec![51235],
            processes,
            include_edge_proxy,
        ),
    ])
}

fn runtime_web_status_component(
    component_id: &'static str,
    kind: &'static str,
    adapter_path: Option<&PathBuf>,
    expected_ports: Vec<u16>,
    processes: &[RuntimeWebProcessSnapshot],
    included: bool,
) -> Value {
    let current_repo_processes = processes
        .iter()
        .filter(|process| {
            process.component_id == Some(component_id) && process.ownership == "current_repo"
        })
        .cloned()
        .collect::<Vec<_>>();
    let stale_processes = processes
        .iter()
        .filter(|process| {
            process.component_id == Some(component_id) && process.ownership != "current_repo"
        })
        .cloned()
        .collect::<Vec<_>>();
    let health = if !included {
        "skipped"
    } else if !stale_processes.is_empty() {
        "stale_conflict"
    } else if !current_repo_processes.is_empty() {
        "running"
    } else if adapter_path.is_none() {
        "not_configured"
    } else {
        "stopped"
    };
    serde_json::json!({
        "id": component_id,
        "kind": kind,
        "included": included,
        "health": health,
        "adapter_path": adapter_path.map(|path| path.display().to_string()),
        "expected_ports": expected_ports,
        "current_repo_processes": current_repo_processes,
        "stale_processes": stale_processes,
    })
}

fn runtime_web_stale_processes(components: &Value) -> Value {
    let mut stale = Vec::new();
    if let Some(components) = components.as_array() {
        for component in components {
            if let Some(processes) = component["stale_processes"].as_array() {
                stale.extend(processes.iter().cloned());
            }
        }
    }
    serde_json::json!(stale)
}

fn run_runtime_web_restart(args: RuntimeWebRestartArgs) -> ExitCode {
    let payload = build_runtime_web_restart_payload(&args);
    let printed_json = print_surface_json(
        &payload,
        args.json,
        "runtime web restart payload should render as json",
    );

    if !printed_json {
        print_runtime_web_restart_plain(&payload);
    }

    if payload["status"].as_str() == Some("pass") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn build_runtime_web_restart_payload(args: &RuntimeWebRestartArgs) -> Value {
    match crate::resolve_runtime_project_root() {
        Ok(project_root) => build_runtime_web_restart_payload_for_project_root(args, &project_root),
        Err(error) => {
            build_runtime_web_restart_payload_for_unresolved_root(args, &error.to_string())
        }
    }
}

fn build_runtime_web_restart_payload_for_unresolved_root(
    args: &RuntimeWebRestartArgs,
    error: &str,
) -> Value {
    let project_root = format!("unresolved: {error}");
    let blocker_codes = vec![blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string()];
    let next_actions =
        vec!["Resolve the VIDA project root before restarting runtime web services.".to_string()];
    let component_results =
        unresolved_root_component_results(args.include_edge_proxy, args.dry_run);
    let actions = runtime_web_restart_actions_from_results(&component_results);
    let blocked_components =
        runtime_web_restart_blocked_components_from_results(&component_results);
    let components = runtime_web_restart_components(
        args.include_edge_proxy,
        args.dry_run,
        &RuntimeWebRestartAdapterPlan::missing(Path::new("")),
        &component_results,
    );
    runtime_web_restart_payload_from_parts(
        args,
        project_root,
        if args.dry_run {
            "plan_only"
        } else {
            "blocked_project_root_unresolved"
        },
        blocker_codes,
        next_actions,
        actions,
        blocked_components,
        components,
        serde_json::json!([]),
    )
}

fn build_runtime_web_restart_payload_for_project_root(
    args: &RuntimeWebRestartArgs,
    project_root: &Path,
) -> Value {
    let adapter_plan = RuntimeWebRestartAdapterPlan::discover(project_root);
    let dry_run = args.dry_run;
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let mut execution_receipts = Vec::new();
    let mut component_results = Vec::new();
    if dry_run {
        component_results.push(ComponentRestartResult::planned("local_web_upstream"));
        component_results.push(ComponentRestartResult::planned("local_proxy"));
        component_results.push(if args.include_edge_proxy {
            ComponentRestartResult::planned("edge_proxy")
        } else {
            ComponentRestartResult::skipped(
                "edge_proxy",
                "edge proxy restart requires --include-edge-proxy",
            )
        });
    } else {
        let local_result = blocked_project_adapter_execution_result(
            "local_web",
            adapter_plan.local_web_script.as_deref(),
            LOCAL_WEB_ADAPTER,
        );
        execution_receipts.push(local_result.receipt.clone());
        component_results.push(local_result.for_component("local_web_upstream"));
        component_results.push(local_result.for_component("local_proxy"));
        if args.include_edge_proxy {
            let edge_result = blocked_project_adapter_execution_result(
                "edge_proxy",
                adapter_plan.edge_proxy_script.as_deref(),
                EDGE_PROXY_ADAPTER,
            );
            execution_receipts.push(edge_result.receipt.clone());
            component_results.push(edge_result.for_component("edge_proxy"));
        } else {
            component_results.push(ComponentRestartResult::skipped(
                "edge_proxy",
                "edge proxy restart requires --include-edge-proxy",
            ));
        }
        blocker_codes.push(blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string());
        next_actions.push(RESTART_EXECUTOR_NEXT_ACTION.to_string());
    }

    let actions = runtime_web_restart_actions_from_results(&component_results);
    let blocked_components =
        runtime_web_restart_blocked_components_from_results(&component_results);
    let components = runtime_web_restart_components(
        args.include_edge_proxy,
        dry_run,
        &adapter_plan,
        &component_results,
    );
    let mode = if dry_run {
        "plan_only"
    } else if blocker_codes.is_empty() {
        "executed_project_adapter_restart"
    } else {
        "blocked_project_adapter_restart"
    };
    runtime_web_restart_payload_from_parts(
        args,
        project_root.display().to_string(),
        mode,
        blocker_codes,
        next_actions,
        actions,
        blocked_components,
        components,
        serde_json::json!(execution_receipts),
    )
}

#[allow(clippy::too_many_arguments)]
fn runtime_web_restart_payload_from_parts(
    args: &RuntimeWebRestartArgs,
    project_root: String,
    mode: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    actions: Value,
    blocked_components: Value,
    components: Value,
    execution_receipts: Value,
) -> Value {
    let artifact_refs = serde_json::json!({
        "surface": RUNTIME_WEB_RESTART_SURFACE,
        "scope": args.scope,
        "include_edge_proxy": args.include_edge_proxy,
        "dry_run": args.dry_run,
        "project_root": project_root,
    });
    let finalized = finalize_release1_operator_truth(blocker_codes, next_actions, artifact_refs)
        .expect("runtime web restart operator truth should finalize");
    let mut payload = serde_json::json!({
        "surface": RUNTIME_WEB_RESTART_SURFACE,
        "status": finalized.status,
        "trace_id": finalized.operator_contracts["trace_id"].clone(),
        "workflow_class": finalized.operator_contracts["workflow_class"].clone(),
        "risk_tier": finalized.operator_contracts["risk_tier"].clone(),
        "blocker_codes": finalized.blocker_codes,
        "next_actions": finalized.next_actions,
        "artifact_refs": finalized.artifact_refs,
        "shared_fields": finalized.shared_fields,
        "operator_contracts": finalized.operator_contracts,
        "restart": {
            "scope": args.scope,
            "include_edge_proxy": args.include_edge_proxy,
            "dry_run": args.dry_run,
            "project_root": project_root,
            "mode": mode,
            "actions": actions,
            "blocked_components": blocked_components,
            "components": components,
            "execution_receipts": execution_receipts,
        },
        "actions": actions,
        "blocked_components": blocked_components,
        "execution_receipts": execution_receipts,
    });
    for key in ["trace_id", "workflow_class", "risk_tier"] {
        payload["shared_fields"][key] = payload["operator_contracts"][key].clone();
    }
    assert_eq!(
        shared_operator_output_contract_parity_error(&payload),
        None,
        "runtime web restart payload should keep release-1 parity"
    );
    payload
}

#[derive(Debug, Clone)]
struct RuntimeWebRestartAdapterPlan {
    project_root: PathBuf,
    local_web_script: Option<PathBuf>,
    edge_proxy_script: Option<PathBuf>,
}

impl RuntimeWebRestartAdapterPlan {
    fn discover(project_root: &Path) -> Self {
        let local_web_script = project_root.join(LOCAL_WEB_ADAPTER);
        let edge_proxy_script = project_root.join(EDGE_PROXY_ADAPTER);
        Self {
            project_root: project_root.to_path_buf(),
            local_web_script: local_web_script.exists().then_some(local_web_script),
            edge_proxy_script: edge_proxy_script.exists().then_some(edge_proxy_script),
        }
    }

    fn missing(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            local_web_script: Some(project_root.join(LOCAL_WEB_ADAPTER)).filter(|_| false),
            edge_proxy_script: Some(project_root.join(EDGE_PROXY_ADAPTER)).filter(|_| false),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct RuntimeWebRestartExecutionReceipt {
    component_group: String,
    status: String,
    adapter_path: Option<String>,
    command: Vec<String>,
    exit_code: Option<i32>,
    stderr: Option<String>,
    stdout: Option<String>,
    simulation: bool,
}

#[derive(Debug, Clone)]
struct AdapterExecutionResult {
    component_group: &'static str,
    action: &'static str,
    reason: Option<String>,
    receipt: RuntimeWebRestartExecutionReceipt,
}

impl AdapterExecutionResult {
    fn for_component(&self, component_id: &'static str) -> ComponentRestartResult {
        ComponentRestartResult {
            component_id,
            action: self.action,
            reason: self.reason.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct ComponentRestartResult {
    component_id: &'static str,
    action: &'static str,
    reason: Option<String>,
}

impl ComponentRestartResult {
    fn planned(component_id: &'static str) -> Self {
        Self {
            component_id,
            action: "planned",
            reason: None,
        }
    }

    fn skipped(component_id: &'static str, reason: impl Into<String>) -> Self {
        Self {
            component_id,
            action: "skipped",
            reason: Some(reason.into()),
        }
    }

    fn blocked(component_id: &'static str, reason: impl Into<String>) -> Self {
        Self {
            component_id,
            action: "blocked",
            reason: Some(reason.into()),
        }
    }
}

fn blocked_project_adapter_execution_result(
    component_group: &'static str,
    script: Option<&Path>,
    expected_adapter: &'static str,
) -> AdapterExecutionResult {
    let Some(script) = script else {
        return missing_adapter_result(component_group, expected_adapter);
    };
    AdapterExecutionResult {
        component_group,
        action: "blocked",
        reason: Some(PROJECT_ADAPTER_EXECUTION_DISABLED_REASON.to_string()),
        receipt: RuntimeWebRestartExecutionReceipt {
            component_group: component_group.to_string(),
            status: "blocked".to_string(),
            adapter_path: Some(script.display().to_string()),
            command: Vec::new(),
            exit_code: None,
            stderr: Some(PROJECT_ADAPTER_EXECUTION_DISABLED_REASON.to_string()),
            stdout: None,
            simulation: false,
        },
    }
}

fn missing_adapter_result(
    component_group: &'static str,
    expected_adapter: &'static str,
) -> AdapterExecutionResult {
    AdapterExecutionResult {
        component_group,
        action: "blocked",
        reason: Some(format!(
            "missing project-local adapter `{expected_adapter}`"
        )),
        receipt: RuntimeWebRestartExecutionReceipt {
            component_group: component_group.to_string(),
            status: "blocked".to_string(),
            adapter_path: None,
            command: Vec::new(),
            exit_code: None,
            stderr: Some(format!(
                "missing project-local adapter `{expected_adapter}`"
            )),
            stdout: None,
            simulation: false,
        },
    }
}

fn unresolved_root_component_results(
    include_edge_proxy: bool,
    dry_run: bool,
) -> Vec<ComponentRestartResult> {
    if dry_run {
        return vec![
            ComponentRestartResult::planned("local_web_upstream"),
            ComponentRestartResult::planned("local_proxy"),
            if include_edge_proxy {
                ComponentRestartResult::planned("edge_proxy")
            } else {
                ComponentRestartResult::skipped(
                    "edge_proxy",
                    "edge proxy restart requires --include-edge-proxy",
                )
            },
        ];
    }
    vec![
        ComponentRestartResult::blocked("local_web_upstream", "project root unresolved"),
        ComponentRestartResult::blocked("local_proxy", "project root unresolved"),
        if include_edge_proxy {
            ComponentRestartResult::blocked("edge_proxy", "project root unresolved")
        } else {
            ComponentRestartResult::skipped(
                "edge_proxy",
                "edge proxy restart requires --include-edge-proxy",
            )
        },
    ]
}

fn runtime_web_restart_actions_from_results(results: &[ComponentRestartResult]) -> Value {
    serde_json::json!(results
        .iter()
        .map(|result| {
            serde_json::json!({
                "component_id": result.component_id,
                "action": result.action,
            })
        })
        .collect::<Vec<_>>())
}

fn runtime_web_restart_blocked_components_from_results(
    results: &[ComponentRestartResult],
) -> Value {
    serde_json::json!(results
        .iter()
        .filter(|result| result.action == "blocked")
        .map(|result| result.component_id)
        .collect::<Vec<_>>())
}

fn runtime_web_restart_components(
    include_edge_proxy: bool,
    dry_run: bool,
    adapter_plan: &RuntimeWebRestartAdapterPlan,
    results: &[ComponentRestartResult],
) -> Value {
    let action_for = |component_id: &str| {
        results
            .iter()
            .find(|result| result.component_id == component_id)
            .map(|result| result.action)
            .unwrap_or("blocked")
    };
    let reason_for = |component_id: &str| {
        results
            .iter()
            .find(|result| result.component_id == component_id)
            .and_then(|result| result.reason.clone())
    };
    let blocker_for = |component_id: &str| {
        if action_for(component_id) == "blocked" {
            Value::String(blocker_code_str(RESTART_EXECUTOR_BLOCKER).to_string())
        } else {
            Value::Null
        }
    };
    serde_json::json!([
        {
            "id": "local_web_upstream",
            "kind": "web_upstream",
            "action": action_for("local_web_upstream"),
            "ownership": "current_repo_required",
            "adapter_path": adapter_plan
                .local_web_script
                .as_ref()
                .map(|path| path.display().to_string()),
            "reason": reason_for("local_web_upstream"),
            "ports": [],
            "blocker_code": blocker_for("local_web_upstream"),
        },
        {
            "id": "local_proxy",
            "kind": "proxy",
            "action": action_for("local_proxy"),
            "ownership": "current_repo_required",
            "adapter_path": adapter_plan
                .local_web_script
                .as_ref()
                .map(|path| path.display().to_string()),
            "reason": reason_for("local_proxy"),
            "ports": [],
            "blocker_code": blocker_for("local_proxy"),
        },
        {
            "id": "edge_proxy",
            "kind": "edge_proxy",
            "action": action_for("edge_proxy"),
            "ownership": "explicit_include_required",
            "adapter_path": adapter_plan
                .edge_proxy_script
                .as_ref()
                .map(|path| path.display().to_string()),
            "reason": reason_for("edge_proxy"),
            "ports": [],
            "blocker_code": if !include_edge_proxy || dry_run {
                Value::Null
            } else {
                blocker_for("edge_proxy")
            },
        }
    ])
}

fn print_runtime_web_restart_plain(payload: &Value) {
    print_surface_header(crate::RenderMode::Plain, "Runtime web restart");
    print_surface_line(
        crate::RenderMode::Plain,
        "status",
        payload["status"].as_str().unwrap_or("blocked"),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "scope",
        payload["restart"]["scope"].as_str().unwrap_or(""),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "mode",
        payload["restart"]["mode"].as_str().unwrap_or("unknown"),
    );
    if let Some(next_action) = payload["next_actions"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
    {
        print_surface_line(crate::RenderMode::Plain, "next action", next_action);
    }
}

fn print_runtime_web_status_plain(payload: &Value) {
    print_surface_header(crate::RenderMode::Plain, "Runtime web status");
    print_surface_line(
        crate::RenderMode::Plain,
        "status",
        payload["status"].as_str().unwrap_or("blocked"),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "scope",
        payload["web_status"]["scope"].as_str().unwrap_or(""),
    );
    print_surface_line(
        crate::RenderMode::Plain,
        "mode",
        payload["web_status"]["mode"].as_str().unwrap_or("unknown"),
    );
    if let Some(next_action) = payload["next_actions"]
        .as_array()
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
    {
        print_surface_line(crate::RenderMode::Plain, "next action", next_action);
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::{Cli, Command};
    use std::ffi::OsString;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn runtime_web_test_env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn temp_runtime_web_project(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-web-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("scripts/windows")).expect("create script root");
        root
    }

    fn write_fake_runtime_web_adapter(root: &Path, relative_path: &str) {
        let path = root.join(relative_path);
        std::fs::create_dir_all(path.parent().expect("adapter parent"))
            .expect("create adapter parent");
        std::fs::write(&path, "Write-Output 'fake adapter'\n").expect("write fake adapter");
    }

    #[test]
    fn runtime_web_restart_cli_accepts_current_repo_edge_proxy_dry_run_json() {
        let cli = Cli::try_parse_from([
            "vida",
            "runtime",
            "web",
            "restart",
            "--scope",
            "current-repo",
            "--include-edge-proxy",
            "--dry-run",
            "--json",
        ])
        .expect("runtime web restart should parse");

        let Some(Command::Runtime(args)) = cli.command else {
            panic!("runtime command should parse as root runtime command");
        };
        let RuntimeCommand::Web(web) = args.command;
        let RuntimeWebCommand::Restart(restart) = web.command else {
            panic!("runtime web restart command should parse");
        };
        assert_eq!(restart.scope, "current-repo");
        assert!(restart.include_edge_proxy);
        assert!(restart.dry_run);
        assert!(restart.json);
    }

    #[test]
    fn runtime_web_status_cli_accepts_current_repo_edge_proxy_json() {
        let cli = Cli::try_parse_from([
            "vida",
            "runtime",
            "web",
            "status",
            "--scope",
            "current-repo",
            "--include-edge-proxy",
            "--json",
        ])
        .expect("runtime web status should parse");

        let Some(Command::Runtime(args)) = cli.command else {
            panic!("runtime command should parse as root runtime command");
        };
        let RuntimeCommand::Web(web) = args.command;
        let RuntimeWebCommand::Status(status) = web.command else {
            panic!("runtime web status command should parse");
        };
        assert_eq!(status.scope, "current-repo");
        assert!(status.include_edge_proxy);
        assert!(status.json);
    }

    #[test]
    fn runtime_web_restart_help_documents_options() {
        let help = Cli::try_parse_from(["vida", "runtime", "web", "restart", "--help"])
            .expect_err("help should render clap display error")
            .to_string();

        for expected in [
            "restart current-repo web proof listeners with fail-closed ownership checks",
            "--scope <SCOPE>",
            "current-repo",
            "--include-edge-proxy",
            "--dry-run",
            "--json",
        ] {
            assert!(
                help.contains(expected),
                "runtime web restart help should document `{expected}`:\n{help}"
            );
        }
    }

    #[test]
    fn runtime_web_status_help_documents_options() {
        let help = Cli::try_parse_from(["vida", "runtime", "web", "status", "--help"])
            .expect_err("help should render clap display error")
            .to_string();

        for expected in [
            "inspect current-repo web proof listeners and proxy health",
            "--scope <SCOPE>",
            "current-repo",
            "--include-edge-proxy",
            "--json",
        ] {
            assert!(
                help.contains(expected),
                "runtime web status help should document `{expected}`:\n{help}"
            );
        }
    }

    #[test]
    fn runtime_web_status_without_processes_reports_pass_not_configured() {
        let _lock = runtime_web_test_env_lock();
        let _snapshot = EnvGuard::set(PROCESS_SNAPSHOT_ENV, "[]");
        let project_root = temp_runtime_web_project("status-clean");

        let payload = build_runtime_web_status_payload_for_project_root(
            &RuntimeWebStatusArgs {
                scope: "current-repo".to_string(),
                include_edge_proxy: true,
                json: true,
            },
            &project_root,
        );

        assert_eq!(payload["surface"], RUNTIME_WEB_STATUS_SURFACE);
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["web_status"]["mode"], "inspected");
        assert_eq!(
            payload["web_status"]["components"][0]["health"],
            "not_configured"
        );
        assert_eq!(payload["stale_processes"].as_array().unwrap().len(), 0);
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn runtime_web_status_reports_stale_foreign_listener_conflict() {
        let _lock = runtime_web_test_env_lock();
        let project_root = temp_runtime_web_project("status-stale");
        write_fake_runtime_web_adapter(&project_root, LOCAL_WEB_ADAPTER);
        write_fake_runtime_web_adapter(&project_root, EDGE_PROXY_ADAPTER);
        let snapshot = serde_json::json!([
            {
                "process_id": 1001,
                "command_line": format!(
                    "pwsh -File {}/scripts/windows/Start-WebDevServer.ps1",
                    project_root.display()
                )
            },
            {
                "process_id": 1002,
                "command_line": "pwsh -File C:/foreign/repo/scripts/windows/Start-WebCloudflareEdgeProxy.ps1"
            }
        ])
        .to_string();
        let _snapshot = EnvGuard::set(PROCESS_SNAPSHOT_ENV, &snapshot);

        let payload = build_runtime_web_status_payload_for_project_root(
            &RuntimeWebStatusArgs {
                scope: "current-repo".to_string(),
                include_edge_proxy: true,
                json: true,
            },
            &project_root,
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            blocker_code_str(STATUS_STALE_LISTENER_BLOCKER)
        );
        assert_eq!(payload["web_status"]["mode"], "stale_listener_conflict");
        assert_eq!(
            payload["web_status"]["safe_restart_command"],
            RUNTIME_WEB_SAFE_RESTART_COMMAND
        );
        assert_eq!(payload["web_status"]["components"][0]["health"], "running");
        assert_eq!(
            payload["web_status"]["components"][2]["health"],
            "stale_conflict"
        );
        assert_eq!(payload["stale_processes"].as_array().unwrap().len(), 1);
        assert_eq!(
            payload["stale_processes"][0]["owner_root"],
            "C:/foreign/repo"
        );
        assert_eq!(
            payload["stale_processes"][0]["working_directory_source"],
            "inferred_from_command_line_script_path"
        );
        assert_eq!(
            payload["stale_processes"][0]["safe_restart_command"],
            RUNTIME_WEB_SAFE_RESTART_COMMAND
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn runtime_web_status_extracts_owner_root_and_ports_from_command_line() {
        let command_line = "pwsh -File C:/project/vida_mobile/scripts/windows/Start-WebDevServer.ps1 -WebPort 51237 -ProxyPort 51236";

        assert_eq!(
            infer_runtime_web_owner_root(command_line),
            Some("C:/project/vida_mobile".to_string())
        );
        assert_eq!(infer_runtime_web_ports(command_line), vec![51237, 51236]);
    }

    #[test]
    fn runtime_web_status_extracts_owner_root_after_powershell_executable_path() {
        let command_line = "\"C:/Program Files/WindowsApps/Microsoft.PowerShell_7.6.2.0_x64__8wekyb3d8bbwe/pwsh.exe\" -NoProfile -ExecutionPolicy Bypass -File C:/Users/pomaz/.codex/worktrees/88f0/vida_mobile/scripts/windows/Start-WebDevServer.ps1 -WebPort 51237";

        assert_eq!(
            infer_runtime_web_owner_root(command_line),
            Some("C:/Users/pomaz/.codex/worktrees/88f0/vida_mobile".to_string())
        );
        assert_eq!(
            runtime_web_owner_class(
                "stale_foreign_repo",
                &normalize_process_path("C:/Users/pomaz/.codex/worktrees/88f0/vida_mobile")
            ),
            "stale_worktree"
        );
    }

    #[test]
    fn runtime_web_status_ignores_own_process_discovery_command() {
        let command_line = "pwsh -NoProfile -Command \"$pattern = 'flutter.*web-server'; Get-CimInstance Win32_Process\"";

        assert_eq!(runtime_web_component_for_command_line(command_line), None);
    }

    #[test]
    fn runtime_web_status_process_discovery_uses_trusted_powershell_path() {
        assert_eq!(
            trusted_windows_powershell_executable_from_system_root(Some(r"D:\Windows")),
            r"D:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
        );
        assert_eq!(
            trusted_windows_powershell_executable_from_system_root(Some("pwsh")),
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
        );
        assert_eq!(
            trusted_windows_powershell_executable_from_system_root(Some(r"..\repo")),
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
        );
    }

    #[test]
    fn runtime_web_restart_json_dry_run_returns_success() {
        let exit = run_runtime_web_restart(RuntimeWebRestartArgs {
            scope: "current-repo".to_string(),
            include_edge_proxy: true,
            dry_run: true,
            json: true,
        });

        assert_eq!(exit, ExitCode::SUCCESS);
    }

    #[test]
    fn runtime_web_restart_dry_run_payload_is_standardized_pass_plan() {
        let project_root = temp_runtime_web_project("dry-run");
        let payload = build_runtime_web_restart_payload_for_project_root(
            &RuntimeWebRestartArgs {
                scope: "current-repo".to_string(),
                include_edge_proxy: true,
                dry_run: true,
                json: true,
            },
            &project_root,
        );

        assert_eq!(payload["surface"], RUNTIME_WEB_RESTART_SURFACE);
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["restart"]["mode"], "plan_only");
        assert_eq!(
            payload["restart"]["blocked_components"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(payload["actions"][0]["action"], "planned");
        assert_eq!(payload["restart"]["components"][2]["action"], "planned");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn runtime_web_restart_without_project_adapters_fails_closed_for_mutation() {
        let project_root = temp_runtime_web_project("missing-adapters");
        let payload = build_runtime_web_restart_payload_for_project_root(
            &RuntimeWebRestartArgs {
                scope: "current-repo".to_string(),
                include_edge_proxy: false,
                dry_run: false,
                json: true,
            },
            &project_root,
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            blocker_code_str(RESTART_EXECUTOR_BLOCKER)
        );
        assert_eq!(payload["blocked_components"][0], "local_web_upstream");
        assert_eq!(payload["restart"]["components"][0]["action"], "blocked");
        assert_eq!(payload["restart"]["components"][2]["action"], "skipped");
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn runtime_web_restart_project_adapters_are_blocked_without_execution() {
        let project_root = temp_runtime_web_project("adapters-present");
        write_fake_runtime_web_adapter(&project_root, LOCAL_WEB_ADAPTER);
        write_fake_runtime_web_adapter(&project_root, EDGE_PROXY_ADAPTER);

        let payload = build_runtime_web_restart_payload_for_project_root(
            &RuntimeWebRestartArgs {
                scope: "current-repo".to_string(),
                include_edge_proxy: true,
                dry_run: false,
                json: true,
            },
            &project_root,
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            blocker_code_str(RESTART_EXECUTOR_BLOCKER)
        );
        assert_eq!(
            payload["restart"]["mode"],
            "blocked_project_adapter_restart"
        );
        assert_eq!(payload["blocked_components"].as_array().unwrap().len(), 3);
        assert_eq!(payload["actions"][0]["action"], "blocked");
        assert_eq!(payload["actions"][1]["action"], "blocked");
        assert_eq!(payload["actions"][2]["action"], "blocked");
        assert_eq!(
            payload["restart"]["execution_receipts"][0]["status"],
            "blocked"
        );
        assert!(payload["restart"]["execution_receipts"][0]["command"]
            .as_array()
            .expect("command should render")
            .is_empty());
        assert_eq!(
            payload["restart"]["execution_receipts"][0]["stderr"],
            PROJECT_ADAPTER_EXECUTION_DISABLED_REASON
        );
        assert_eq!(shared_operator_output_contract_parity_error(&payload), None);
        let _ = std::fs::remove_dir_all(&project_root);
    }
}
