use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;

use crate::host_runtime_registry::looks_like_host_runtime_source_root;
use crate::state_store::StateStore;
use crate::surface_render::print_compact_command_families;

use super::{
    build_runtime_lane_selection_with_store, ensure_launcher_bootstrap, normalize_root_arg,
    print_surface_header, print_surface_line, role_exists_in_lane_bundle, state_store,
    sync_launcher_activation_snapshot, AgentInitArgs, BootArgs, InitArgs, RenderMode,
};
use crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload;

const DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS: u64 = 10;
const AGENT_INIT_EXECUTE_DISPATCH_RECONCILIATION_GRACE_SECONDS: u64 = 20;
const AGENT_INIT_EXECUTE_DISPATCH_OPERATOR_HANDOFF_SECONDS: u64 = 2;
pub(crate) const AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV: &str =
    "VIDA_AGENT_INIT_EXECUTE_DISPATCH_WORKER";
const COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS: u64 = 30;
const BOOT_RELEASE_VERIFICATION_RETRY_DELAY_MS: u64 = 25;
const INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS: u64 = 45;
const LAUNCHER_BOOTSTRAP_MUTATION_TIMEOUT_SECONDS: u64 = 30;
const AGENT_INIT_EXECUTE_DISPATCH_MISSING_PACKET_ERROR: &str =
    "Agent init execute-dispatch requires either `--dispatch-packet` or `--downstream-packet`.";
static AGENT_INIT_READ_SURFACE_GUARD: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn agent_init_execute_dispatch_timeout_seconds(dispatch_handoff_timeout_seconds: u64) -> u64 {
    dispatch_handoff_timeout_seconds
        .saturating_add(AGENT_INIT_EXECUTE_DISPATCH_RECONCILIATION_GRACE_SECONDS)
}

fn agent_init_execute_dispatch_handoff_threshold_seconds() -> u64 {
    AGENT_INIT_EXECUTE_DISPATCH_OPERATOR_HANDOFF_SECONDS
}

fn agent_init_receipt_timeout_seconds(
    dispatch_handoff_timeout_seconds: u64,
    execute_dispatch_timeout_seconds: u64,
) -> u64 {
    dispatch_handoff_timeout_seconds.min(execute_dispatch_timeout_seconds)
}

fn agent_init_execute_dispatch_worker_active() -> bool {
    std::env::var_os(AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV).is_some()
}

fn agent_init_execute_dispatch_window_requires_operator_handoff(
    dispatch_handoff_timeout_seconds: u64,
) -> bool {
    dispatch_handoff_timeout_seconds >= AGENT_INIT_EXECUTE_DISPATCH_OPERATOR_HANDOFF_SECONDS
}

fn agent_init_execute_dispatch_should_handoff(
    dispatch_handoff_timeout_seconds: u64,
    uses_internal_host: bool,
) -> bool {
    #[cfg(test)]
    {
        let _ = dispatch_handoff_timeout_seconds;
        let _ = uses_internal_host;
        false
    }
    #[cfg(not(test))]
    {
        if agent_init_execute_dispatch_worker_active() {
            return false;
        }
        let _ = uses_internal_host;
        agent_init_execute_dispatch_window_requires_operator_handoff(
            dispatch_handoff_timeout_seconds,
        )
    }
}

fn orchestrator_init_bundle_timeout_payload(state_dir: &Path) -> serde_json::Value {
    let blocker_codes = vec!["taskflow_consume_bundle_timeout"];
    let next_actions = vec![
        "Retry `vida orchestrator-init --json` after concurrent VIDA state readers finish.",
        "Run `vida status --json` and `vida taskflow recovery latest --json` to inspect current runtime state if the timeout repeats.",
    ];
    let artifact_refs = serde_json::json!({
        "state_dir": state_dir.display().to_string(),
        "timeout_seconds": INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS,
        "timed_out_surface": "build_taskflow_consume_bundle_payload",
    });
    serde_json::json!({
        "surface": "vida orchestrator-init",
        "status": "blocked",
        "degraded": true,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "state_read": {
            "mode": "authoritative_open",
            "lock_resilient": true,
            "fallback": "degraded_timeout_surface",
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
        },
    })
}

fn emit_orchestrator_init_bundle_timeout(state_dir: &Path, as_json: bool) -> ExitCode {
    if as_json {
        crate::print_json_pretty(&orchestrator_init_bundle_timeout_payload(state_dir));
    } else {
        eprintln!(
            "Timed out building taskflow consume bundle for `vida orchestrator-init` after {INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS}s"
        );
    }
    ExitCode::from(1)
}

fn agent_init_bundle_timeout_payload(state_dir: &Path) -> serde_json::Value {
    let blocker_codes = vec!["taskflow_consume_bundle_timeout"];
    let next_actions = vec![
        "Retry `vida agent-init --json` after concurrent VIDA state readers finish.",
        "Run `vida status --json` and `vida taskflow recovery latest --json` to inspect current runtime state if the timeout repeats.",
    ];
    let artifact_refs = serde_json::json!({
        "state_dir": state_dir.display().to_string(),
        "timeout_seconds": INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS,
        "timed_out_surface": "build_taskflow_consume_bundle_payload",
    });
    serde_json::json!({
        "surface": "vida agent-init",
        "status": "blocked",
        "degraded": true,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "state_read": {
            "mode": "authoritative_open",
            "lock_resilient": true,
            "fallback": "degraded_timeout_surface",
        },
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
        },
    })
}

fn emit_agent_init_bundle_timeout(state_dir: &Path, as_json: bool) -> ExitCode {
    if as_json {
        crate::print_json_pretty(&agent_init_bundle_timeout_payload(state_dir));
    } else {
        eprintln!(
            "Timed out building taskflow consume bundle for `vida agent-init` after {INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS}s"
        );
    }
    ExitCode::from(1)
}

async fn verify_authoritative_state_store_released_after_boot(
    state_root: PathBuf,
) -> Result<(), String> {
    let timeout = std::time::Duration::from_secs(COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS);
    let retry_delay = std::time::Duration::from_millis(BOOT_RELEASE_VERIFICATION_RETRY_DELAY_MS);
    let deadline = std::time::Instant::now() + timeout;
    let mut last_lock_contention = None;

    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return Err(match last_lock_contention {
                Some(error) => format!(
                    "Timed out verifying authoritative state store release after `vida boot` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s; last lock-contention error: {error}"
                ),
                None => format!(
                    "Timed out verifying authoritative state store release after `vida boot` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s"
                ),
            });
        }

        match tokio::time::timeout(
            deadline.saturating_duration_since(now),
            StateStore::open_existing(state_root.clone()),
        )
        .await
        {
            Ok(Ok(reopened_store)) => {
                drop(reopened_store);
                return Ok(());
            }
            Ok(Err(error)) if StateStore::error_is_lock_contention(&error) => {
                last_lock_contention = Some(error.to_string());
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    continue;
                }
                tokio::time::sleep(retry_delay.min(remaining)).await;
            }
            Ok(Err(error)) => {
                return Err(format!(
                    "Failed to verify authoritative state store release after `vida boot`: {error}"
                ));
            }
            Err(_) => {
                return Err(match last_lock_contention {
                    Some(error) => format!(
                        "Timed out verifying authoritative state store release after `vida boot` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s; last lock-contention error: {error}"
                    ),
                    None => format!(
                        "Timed out verifying authoritative state store release after `vida boot` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s"
                    ),
                });
            }
        }
    }
}

fn agent_init_dispatch_timeout_artifact_refs(
    run_id: &str,
    dispatch_result_path: Option<&str>,
    timeout_seconds: u64,
) -> serde_json::Value {
    serde_json::json!({
        "run_id": run_id,
        "surface": "vida agent-init",
        "dispatch_result_path": dispatch_result_path,
        "timeout_seconds": timeout_seconds,
    })
}

fn agent_init_dispatch_timeout_blocker_codes(result_json: &serde_json::Value) -> Vec<String> {
    result_json
        .get("blocker_code")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| vec![value.to_string()])
        .unwrap_or_else(|| vec!["timeout_without_takeover_authority".to_string()])
}

fn agent_init_dispatch_timeout_next_actions() -> Vec<&'static str> {
    vec![
        "Inspect continuation evidence with `vida status --json` and `vida taskflow recovery latest --json`.",
        "Keep the timeout dispatch result as blocked evidence before retrying `vida agent-init --execute-dispatch --json`.",
    ]
}

fn agent_init_dispatch_timeout_operator_envelope(
    mut result_json: serde_json::Value,
    dispatch_mode: &serde_json::Value,
    run_id: &str,
    dispatch_result_path: Option<&str>,
    timeout_seconds: u64,
    warning: Option<&str>,
) -> serde_json::Value {
    let blocker_codes = agent_init_dispatch_timeout_blocker_codes(&result_json);
    let next_actions = agent_init_dispatch_timeout_next_actions();
    let artifact_refs =
        agent_init_dispatch_timeout_artifact_refs(run_id, dispatch_result_path, timeout_seconds);
    if let Some(object) = result_json.as_object_mut() {
        object.insert("dispatch_mode".to_string(), dispatch_mode.clone());
        object.insert(
            "blocker_codes".to_string(),
            serde_json::json!(blocker_codes),
        );
        object.insert("next_actions".to_string(), serde_json::json!(next_actions));
        object.insert("artifact_refs".to_string(), artifact_refs.clone());
        if let Some(warning) = warning {
            object.insert("timeout_reconciliation_warning".to_string(), warning.into());
        }
        object.insert(
            "operator_contracts".to_string(),
            serde_json::json!({
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "blocked",
                "blocker_codes": blocker_codes,
                "next_actions": next_actions,
                "artifact_refs": artifact_refs,
                "risk_tier": null,
                "trace_id": null,
                "workflow_class": null,
            }),
        );
        object.insert(
            "shared_fields".to_string(),
            serde_json::json!({
                "status": "blocked",
                "blocker_codes": blocker_codes,
                "next_actions": next_actions,
                "artifact_refs": artifact_refs,
            }),
        );
    }
    result_json
}

fn agent_init_dispatch_timeout_fallback_payload(
    dispatch_mode: &serde_json::Value,
    run_id: &str,
    dispatch_result_path: Option<&str>,
    timeout_seconds: u64,
    error: Option<&str>,
) -> serde_json::Value {
    let blocker_codes = vec!["timeout_without_takeover_authority"];
    let next_actions = agent_init_dispatch_timeout_next_actions();
    let artifact_refs =
        agent_init_dispatch_timeout_artifact_refs(run_id, dispatch_result_path, timeout_seconds);
    let mut payload = serde_json::json!({
        "surface": "vida agent-init",
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_mode": dispatch_mode,
        "blocker_code": "timeout_without_takeover_authority",
        "blocker_codes": blocker_codes,
        "provider_error": format!(
            "Timed out executing agent-init dispatch packet after {timeout_seconds}s total without receipt-backed completion"
        ),
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null,
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
        },
    });
    if let Some(error) = error {
        if let Some(object) = payload.as_object_mut() {
            object.insert("materialization_error".to_string(), error.into());
        }
    }
    payload
}

fn render_agent_init_dispatch_result_from_receipt(
    dispatch_mode: &serde_json::Value,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    json_output: bool,
    timeout_seconds: Option<u64>,
    warning: Option<&str>,
) -> Result<ExitCode, String> {
    let Some(dispatch_result_path) = dispatch_receipt.dispatch_result_path.as_deref() else {
        return Err(
            "Agent init execute-dispatch did not produce a dispatch result artifact.".to_string(),
        );
    };
    let result_body = std::fs::read_to_string(dispatch_result_path).map_err(|error| {
        format!("Failed to read agent-init dispatch result `{dispatch_result_path}`: {error}")
    })?;
    let mut result_json =
        serde_json::from_str::<serde_json::Value>(&result_body).map_err(|error| {
            format!("Failed to parse agent-init dispatch result `{dispatch_result_path}`: {error}")
        })?;
    if let Some(timeout_seconds) = timeout_seconds {
        result_json = agent_init_dispatch_timeout_operator_envelope(
            result_json,
            dispatch_mode,
            &dispatch_receipt.run_id,
            Some(dispatch_result_path),
            timeout_seconds,
            warning,
        );
    } else if let Some(object) = result_json.as_object_mut() {
        object.insert("dispatch_mode".to_string(), dispatch_mode.clone());
    }
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&result_json)
                .expect("agent-init dispatch result json should render")
        );
    } else {
        crate::print_json_pretty(&result_json);
    }
    Ok(if dispatch_receipt.dispatch_status == "blocked" {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn safe_dispatch_worker_id(run_id: &str, dispatch_target: &str) -> String {
    let ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let raw = format!("{run_id}-{dispatch_target}-{ts}");
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn dispatch_packet_flag_for_packet_path(packet_path: &str) -> &'static str {
    let is_downstream = super::taskflow_consume_resume::read_dispatch_packet(packet_path)
        .ok()
        .is_some_and(|packet| {
            packet
                .get("packet_kind")
                .and_then(serde_json::Value::as_str)
                == Some("runtime_downstream_dispatch_packet")
                || packet
                    .get("downstream_dispatch_target")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
        });
    if is_downstream {
        "--downstream-packet"
    } else {
        "--dispatch-packet"
    }
}

fn spawn_agent_init_execute_dispatch_worker(
    state_root: &Path,
    resume_inputs: &super::taskflow_consume_resume::ResumeInputs,
) -> Result<serde_json::Value, String> {
    let project_root = super::runtime_dispatch_project_root_from_state_root(state_root);
    let worker_dir = state_root
        .join("runtime-consumption")
        .join("dispatch-workers");
    std::fs::create_dir_all(&worker_dir)
        .map_err(|error| format!("Failed to create dispatch worker directory: {error}"))?;
    let worker_id = safe_dispatch_worker_id(
        &resume_inputs.dispatch_receipt.run_id,
        &resume_inputs.dispatch_receipt.dispatch_target,
    );
    let stdout_path = worker_dir.join(format!("{worker_id}.stdout.jsonl"));
    let stderr_path = worker_dir.join(format!("{worker_id}.stderr.log"));
    let executable = std::env::current_exe()
        .map_err(|error| format!("Failed to resolve current VIDA executable: {error}"))?;
    let packet_flag = dispatch_packet_flag_for_packet_path(&resume_inputs.dispatch_packet_path);
    #[cfg(windows)]
    {
        return spawn_agent_init_execute_dispatch_worker_windows(
            &executable,
            project_root.as_ref(),
            state_root,
            resume_inputs,
            packet_flag,
            &stdout_path,
            &stderr_path,
            &worker_id,
        );
    }
    #[cfg(not(windows))]
    let stdout = std::fs::File::create(&stdout_path)
        .map_err(|error| format!("Failed to create dispatch worker stdout log: {error}"))?;
    #[cfg(not(windows))]
    let stderr = std::fs::File::create(&stderr_path)
        .map_err(|error| format!("Failed to create dispatch worker stderr log: {error}"))?;
    #[cfg(not(windows))]
    let mut command = std::process::Command::new(executable);
    #[cfg(not(windows))]
    command
        .arg("agent-init")
        .arg(packet_flag)
        .arg(&resume_inputs.dispatch_packet_path)
        .arg("--execute-dispatch")
        .arg("--json")
        .arg("--state-dir")
        .arg(state_root)
        .current_dir(project_root.as_ref())
        .env(AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV, "1")
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(stderr);
    #[cfg(not(windows))]
    let child = command
        .spawn()
        .map_err(|error| format!("Failed to spawn agent-init dispatch worker: {error}"))?;
    #[cfg(not(windows))]
    {
        Ok(serde_json::json!({
        "worker_id": worker_id,
        "worker_pid": child.id(),
        "operator_handoff": "background_dispatch_worker",
        "operator_handoff_wait_seconds": AGENT_INIT_EXECUTE_DISPATCH_OPERATOR_HANDOFF_SECONDS,
        "stdout_path": stdout_path.display().to_string(),
        "stderr_path": stderr_path.display().to_string(),
        "packet_arg": packet_flag,
        "packet_path": resume_inputs.dispatch_packet_path,
        }))
    }
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn spawn_agent_init_execute_dispatch_worker_windows(
    executable: &Path,
    project_root: &Path,
    state_root: &Path,
    resume_inputs: &super::taskflow_consume_resume::ResumeInputs,
    packet_flag: &str,
    stdout_path: &Path,
    stderr_path: &Path,
    worker_id: &str,
) -> Result<serde_json::Value, String> {
    let worker_args = [
        "agent-init".to_string(),
        packet_flag.to_string(),
        resume_inputs.dispatch_packet_path.clone(),
        "--execute-dispatch".to_string(),
        "--json".to_string(),
        "--state-dir".to_string(),
        state_root.display().to_string(),
    ];
    let worker_pid =
        windows_spawn_detached_process(executable, project_root, &worker_args)?.to_string();
    Ok(serde_json::json!({
        "worker_id": worker_id,
        "worker_pid": worker_pid,
        "operator_handoff": "background_dispatch_worker",
        "operator_handoff_wait_seconds": AGENT_INIT_EXECUTE_DISPATCH_OPERATOR_HANDOFF_SECONDS,
        "stdout_path": stdout_path.display().to_string(),
        "stderr_path": stderr_path.display().to_string(),
        "packet_arg": packet_flag,
        "packet_path": resume_inputs.dispatch_packet_path,
        "launcher": "win32_create_process",
    }))
}

#[cfg(windows)]
fn windows_dispatch_worker_creation_flags() -> u32 {
    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;

    DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB
}

#[cfg(windows)]
fn windows_spawn_detached_process(
    executable: &Path,
    working_dir: &Path,
    args: &[String],
) -> Result<u32, String> {
    windows_create_detached_process(executable, working_dir, args)
}

#[cfg(windows)]
fn windows_quote_command_arg(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\\'))
    {
        let mut quoted = String::from("\"");
        let mut backslashes = 0;
        for ch in value.chars() {
            match ch {
                '\\' => backslashes += 1,
                '"' => {
                    quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                    quoted.push('"');
                    backslashes = 0;
                }
                _ => {
                    if backslashes > 0 {
                        quoted.push_str(&"\\".repeat(backslashes));
                        backslashes = 0;
                    }
                    quoted.push(ch);
                }
            }
        }
        if backslashes > 0 {
            quoted.push_str(&"\\".repeat(backslashes * 2));
        }
        quoted.push('"');
        quoted
    } else {
        value.to_string()
    }
}

#[cfg(windows)]
fn windows_command_line(executable: &Path, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(windows_quote_command_arg(&executable.display().to_string()));
    parts.extend(args.iter().map(|arg| windows_quote_command_arg(arg)));
    parts.join(" ")
}

#[cfg(windows)]
fn windows_wide_null(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
#[allow(non_snake_case)]
#[repr(C)]
struct WindowsStartupInfoW {
    cb: u32,
    lpReserved: *mut u16,
    lpDesktop: *mut u16,
    lpTitle: *mut u16,
    dwX: u32,
    dwY: u32,
    dwXSize: u32,
    dwYSize: u32,
    dwXCountChars: u32,
    dwYCountChars: u32,
    dwFillAttribute: u32,
    dwFlags: u32,
    wShowWindow: u16,
    cbReserved2: u16,
    lpReserved2: *mut u8,
    hStdInput: *mut std::ffi::c_void,
    hStdOutput: *mut std::ffi::c_void,
    hStdError: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[allow(non_snake_case)]
#[repr(C)]
struct WindowsStartupInfoExW {
    StartupInfo: WindowsStartupInfoW,
    lpAttributeList: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[allow(non_snake_case)]
#[repr(C)]
struct WindowsProcessInformation {
    hProcess: *mut std::ffi::c_void,
    hThread: *mut std::ffi::c_void,
    dwProcessId: u32,
    dwThreadId: u32,
}

#[cfg(windows)]
#[allow(non_snake_case)]
#[repr(C)]
struct WindowsProcessEntry32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

#[cfg(windows)]
extern "system" {
    fn CreateProcessW(
        lpApplicationName: *const u16,
        lpCommandLine: *mut u16,
        lpProcessAttributes: *mut std::ffi::c_void,
        lpThreadAttributes: *mut std::ffi::c_void,
        bInheritHandles: i32,
        dwCreationFlags: u32,
        lpEnvironment: *mut std::ffi::c_void,
        lpCurrentDirectory: *const u16,
        lpStartupInfo: *mut WindowsStartupInfoW,
        lpProcessInformation: *mut WindowsProcessInformation,
    ) -> i32;
    fn InitializeProcThreadAttributeList(
        lpAttributeList: *mut std::ffi::c_void,
        dwAttributeCount: u32,
        dwFlags: u32,
        lpSize: *mut usize,
    ) -> i32;
    fn UpdateProcThreadAttribute(
        lpAttributeList: *mut std::ffi::c_void,
        dwFlags: u32,
        Attribute: usize,
        lpValue: *mut std::ffi::c_void,
        cbSize: usize,
        lpPreviousValue: *mut std::ffi::c_void,
        lpReturnSize: *mut usize,
    ) -> i32;
    fn DeleteProcThreadAttributeList(lpAttributeList: *mut std::ffi::c_void);
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut std::ffi::c_void;
    fn Process32FirstW(hSnapshot: *mut std::ffi::c_void, lppe: *mut WindowsProcessEntry32W) -> i32;
    fn Process32NextW(hSnapshot: *mut std::ffi::c_void, lppe: *mut WindowsProcessEntry32W) -> i32;
    fn OpenProcess(
        dwDesiredAccess: u32,
        bInheritHandle: i32,
        dwProcessId: u32,
    ) -> *mut std::ffi::c_void;
    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
fn windows_find_explorer_parent_process() -> Option<*mut std::ffi::c_void> {
    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    const PROCESS_CREATE_PROCESS: u32 = 0x0080;
    let invalid_handle = (-1isize) as *mut std::ffi::c_void;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot == invalid_handle {
            return None;
        }
        let mut entry = std::mem::zeroed::<WindowsProcessEntry32W>();
        entry.dwSize = std::mem::size_of::<WindowsProcessEntry32W>() as u32;
        let mut found = None;
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            let len = entry
                .szExeFile
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(entry.szExeFile.len());
            let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if exe.eq_ignore_ascii_case("explorer.exe") {
                let handle = OpenProcess(PROCESS_CREATE_PROCESS, 0, entry.th32ProcessID);
                if !handle.is_null() {
                    found = Some(handle);
                    break;
                }
            }
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        found
    }
}

#[cfg(windows)]
fn windows_create_detached_process(
    executable: &Path,
    working_dir: &Path,
    args: &[String],
) -> Result<u32, String> {
    const EXTENDED_STARTUPINFO_PRESENT: u32 = 0x00080000;
    const PROC_THREAD_ATTRIBUTE_PARENT_PROCESS: usize = 0x00020000;
    let mut command_line = windows_wide_null(&windows_command_line(executable, args));
    let working_dir = windows_wide_null(&working_dir.display().to_string());
    let old_worker_env = std::env::var_os(AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV);
    std::env::set_var(AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV, "1");

    let parent = windows_find_explorer_parent_process();
    let mut attribute_storage = Vec::<usize>::new();
    let mut startup = unsafe { std::mem::zeroed::<WindowsStartupInfoExW>() };
    startup.StartupInfo.cb = std::mem::size_of::<WindowsStartupInfoExW>() as u32;

    let mut creation_flags = windows_dispatch_worker_creation_flags();
    if let Some(parent_handle) = parent {
        let mut size = 0usize;
        unsafe {
            InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size);
        }
        attribute_storage.resize(
            (size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>(),
            0,
        );
        let attribute_list = attribute_storage.as_mut_ptr() as *mut std::ffi::c_void;
        let initialized =
            unsafe { InitializeProcThreadAttributeList(attribute_list, 1, 0, &mut size) };
        if initialized != 0 {
            let mut parent_value = parent_handle;
            let updated = unsafe {
                UpdateProcThreadAttribute(
                    attribute_list,
                    0,
                    PROC_THREAD_ATTRIBUTE_PARENT_PROCESS,
                    (&mut parent_value as *mut *mut std::ffi::c_void).cast(),
                    std::mem::size_of::<*mut std::ffi::c_void>(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if updated != 0 {
                startup.lpAttributeList = attribute_list;
                creation_flags |= EXTENDED_STARTUPINFO_PRESENT;
            }
        }
    }

    let mut process_info = unsafe { std::mem::zeroed::<WindowsProcessInformation>() };
    let created = unsafe {
        CreateProcessW(
            std::ptr::null(),
            command_line.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            creation_flags,
            std::ptr::null_mut(),
            working_dir.as_ptr(),
            (&mut startup.StartupInfo) as *mut WindowsStartupInfoW,
            &mut process_info,
        )
    };
    let last_error = std::io::Error::last_os_error();
    if !startup.lpAttributeList.is_null() {
        unsafe {
            DeleteProcThreadAttributeList(startup.lpAttributeList);
        }
    }
    if let Some(parent_handle) = parent {
        unsafe {
            CloseHandle(parent_handle);
        }
    }
    if let Some(old) = old_worker_env {
        std::env::set_var(AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV, old);
    } else {
        std::env::remove_var(AGENT_INIT_EXECUTE_DISPATCH_WORKER_ENV);
    }
    if created == 0 {
        return Err(format!(
            "Failed to create detached Windows dispatch worker `{}`: {last_error}",
            executable.display()
        ));
    }
    unsafe {
        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);
    }
    Ok(process_info.dwProcessId)
}

fn agent_init_dispatch_started_payload(
    mut result_json: serde_json::Value,
    dispatch_mode: &serde_json::Value,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    worker: serde_json::Value,
) -> serde_json::Value {
    let dispatch_result_path = dispatch_receipt.dispatch_result_path.as_deref();
    let artifact_refs = serde_json::json!({
        "run_id": dispatch_receipt.run_id,
        "surface": "vida agent-init",
        "dispatch_result_path": dispatch_result_path,
        "dispatch_packet_path": dispatch_receipt.dispatch_packet_path,
        "worker": worker,
    });
    let next_actions = vec![
        format!(
            "Poll lane state with `vida lane show {} --json`.",
            dispatch_receipt.run_id
        ),
        format!(
            "Poll run-graph state with `vida taskflow run-graph status {} --json`.",
            dispatch_receipt.run_id
        ),
        "Do not rerun execute-dispatch while execution_state is executing; wait for the worker receipt or timeout evidence.".to_string(),
    ];
    if let Some(object) = result_json.as_object_mut() {
        object.insert("dispatch_mode".to_string(), dispatch_mode.clone());
        object.insert("async_dispatch".to_string(), serde_json::json!(true));
        object.insert(
            "operator_handoff".to_string(),
            serde_json::json!("background_dispatch_worker"),
        );
        object.insert("next_actions".to_string(), serde_json::json!(next_actions));
        object.insert("artifact_refs".to_string(), artifact_refs.clone());
        object.insert(
            "operator_contracts".to_string(),
            serde_json::json!({
                "contract_id": "release-1-operator-contracts",
                "schema_version": "release-1-v1",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": next_actions,
                "artifact_refs": artifact_refs,
                "risk_tier": null,
                "trace_id": null,
                "workflow_class": null,
            }),
        );
        object.insert(
            "shared_fields".to_string(),
            serde_json::json!({
                "status": "pass",
                "blocker_codes": [],
                "next_actions": next_actions,
                "artifact_refs": artifact_refs,
            }),
        );
    }
    result_json
}

async fn start_agent_init_dispatch_worker_and_return(
    json_output: bool,
    dispatch_mode: &serde_json::Value,
    state_root: &Path,
    resume_inputs: &mut super::taskflow_consume_resume::ResumeInputs,
) -> Result<ExitCode, String> {
    let project_root = super::runtime_dispatch_project_root_from_state_root(state_root);
    let stale_after_seconds =
        super::runtime_dispatch_state::dispatch_execution_started_stale_after_seconds(
            project_root.as_ref(),
            &resume_inputs.role_selection,
            &resume_inputs.dispatch_receipt,
        );
    let in_flight_result = super::runtime_dispatch_state::runtime_dispatch_execution_started_result(
        &resume_inputs.dispatch_receipt,
        stale_after_seconds,
    );
    let dispatch_result_path = super::runtime_dispatch_state::write_runtime_dispatch_result(
        state_root,
        &resume_inputs.dispatch_receipt,
        &in_flight_result,
    )?;
    resume_inputs.dispatch_receipt.dispatch_result_path = Some(dispatch_result_path.clone());
    resume_inputs.dispatch_receipt.dispatch_status = "executing".to_string();
    resume_inputs.dispatch_receipt.lane_status = "lane_running".to_string();
    resume_inputs.dispatch_receipt.blocker_code = None;
    let worker = spawn_agent_init_execute_dispatch_worker(state_root, resume_inputs)?;
    let result_body = std::fs::read_to_string(&dispatch_result_path).map_err(|error| {
        format!(
            "Failed to read in-flight agent-init dispatch result `{dispatch_result_path}`: {error}"
        )
    })?;
    let result_json = serde_json::from_str::<serde_json::Value>(&result_body).map_err(|error| {
        format!("Failed to parse in-flight agent-init dispatch result `{dispatch_result_path}`: {error}")
    })?;
    let result_json = agent_init_dispatch_started_payload(
        result_json,
        dispatch_mode,
        &resume_inputs.dispatch_receipt,
        worker,
    );
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&result_json)
                .expect("agent-init async dispatch json should render")
        );
    } else {
        crate::print_json_pretty(&result_json);
    }
    Ok(ExitCode::SUCCESS)
}

fn agent_init_execute_dispatch_resume_error_run_id(error: &str) -> Option<String> {
    for marker in [
        "Run-graph resume gate denied for `",
        "Stale missing-task run graph `",
        "run `",
        "run_id `",
    ] {
        let Some(start) = error.find(marker).map(|index| index + marker.len()) else {
            continue;
        };
        let rest = &error[start..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let run_id = rest[..end].trim();
        if !run_id.is_empty() {
            return Some(run_id.to_string());
        }
    }
    None
}

fn agent_init_execute_dispatch_resume_error_blocker_code(error: &str) -> &'static str {
    if error.contains("recovery_ready is false") {
        "run_graph_recovery_not_ready"
    } else if error.contains("Stale missing-task run graph")
        || error.contains("references missing TaskFlow task")
    {
        "stale_missing_task_run_graph"
    } else if error.contains("No persisted run-graph dispatch receipt exists")
        || error.contains("missing receipt recovery could not load dispatch context")
    {
        "missing_run_graph_dispatch_receipt"
    } else {
        "agent_init_execute_dispatch_resume_blocked"
    }
}

fn agent_init_execute_dispatch_resume_error_next_actions(
    blocker_code: &str,
    run_id: Option<&str>,
) -> Vec<String> {
    match (blocker_code, run_id) {
        ("run_graph_recovery_not_ready", Some(run_id)) => vec![
            format!(
                "Inspect the blocked run with `vida taskflow recovery status {} --json`.",
                crate::shell_quote(run_id)
            ),
            format!(
                "Inspect run-graph gate fields with `vida taskflow run-graph status {} --json` and make recovery_ready=true through the canonical recovery/continue path before retrying execute-dispatch.",
                crate::shell_quote(run_id)
            ),
            "Inspect route freshness with `vida taskflow route explain --json`; if it reports model_not_pinned or catalog drift, refresh or reseed the route assignment before creating another packet.".to_string(),
        ],
        ("run_graph_recovery_not_ready", None) => vec![
            "Inspect the blocked run with `vida taskflow recovery latest --json`.".to_string(),
            "Inspect run-graph gate fields with `vida taskflow run-graph status <run-id> --json` and make recovery_ready=true through the canonical recovery/continue path before retrying execute-dispatch.".to_string(),
            "Inspect route freshness with `vida taskflow route explain --json`; if it reports model_not_pinned or catalog drift, refresh or reseed the route assignment before creating another packet.".to_string(),
        ],
        ("stale_missing_task_run_graph", Some(run_id)) => vec![
            format!(
                "Retire the stale missing-task run with `vida lane retire {} --receipt-id {} --reason \"missing TaskFlow task stale run\" --json`.",
                crate::shell_quote(run_id),
                crate::shell_quote(run_id)
            ),
            "Refresh continuation evidence with `vida status --json` and `vida taskflow recovery latest --json` before retrying execute-dispatch.".to_string(),
        ],
        ("missing_run_graph_dispatch_receipt", Some(run_id)) => vec![
            format!(
                "Inspect recovery for missing receipt repair with `vida taskflow recovery status {} --json`.",
                crate::shell_quote(run_id)
            ),
            "Regenerate a fresh dispatch packet only after the recovery surface reports receipt-backed dispatch context.".to_string(),
        ],
        _ => vec![
            "Inspect continuation evidence with `vida status --json` and `vida taskflow recovery latest --json`.".to_string(),
            "Do not retry `vida agent-init --execute-dispatch --json` until the recovery surface reports recovery_ready=true and a dispatch resume target.".to_string(),
        ],
    }
}

fn agent_init_execute_dispatch_resume_error_payload(
    dispatch_mode: &serde_json::Value,
    error: &str,
) -> serde_json::Value {
    let blocker_code = agent_init_execute_dispatch_resume_error_blocker_code(error);
    let run_id = agent_init_execute_dispatch_resume_error_run_id(error);
    let next_actions =
        agent_init_execute_dispatch_resume_error_next_actions(blocker_code, run_id.as_deref());
    let artifact_refs = serde_json::json!({
        "surface": "vida agent-init",
        "run_id": run_id,
    });
    let blocker_codes = vec![blocker_code];
    serde_json::json!({
        "surface": "vida agent-init",
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_mode": dispatch_mode,
        "error": error,
        "run_id": run_id,
        "blocker_code": blocker_code,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs
        }
    })
}

fn agent_init_execute_dispatch_resume_error_payload_with_receipt_evidence(
    dispatch_mode: &serde_json::Value,
    error: &str,
    receipt: Option<&crate::state_store::RunGraphDispatchReceipt>,
    result_artifact: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut payload = agent_init_execute_dispatch_resume_error_payload(dispatch_mode, error);
    let Some(receipt) = receipt else {
        return payload;
    };
    let dispatch_result_path = receipt.dispatch_result_path.as_deref();
    let receipt_status = if receipt.dispatch_status == "blocked" || receipt.blocker_code.is_some() {
        "blocked"
    } else {
        "pass"
    };
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "underlying_dispatch_status".to_string(),
            serde_json::json!(receipt.dispatch_status),
        );
        object.insert(
            "underlying_lane_status".to_string(),
            serde_json::json!(receipt.lane_status),
        );
        object.insert(
            "underlying_dispatch_blocker_code".to_string(),
            serde_json::json!(receipt.blocker_code),
        );
        object.insert(
            "dispatch_result_path".to_string(),
            serde_json::json!(dispatch_result_path),
        );
        object.insert(
            "receipt_status".to_string(),
            serde_json::json!(receipt_status),
        );
        object.insert(
            "receipt_path".to_string(),
            serde_json::json!(dispatch_result_path),
        );
        object.insert(
            "lane_execution_receipt_path".to_string(),
            serde_json::json!(dispatch_result_path),
        );
        if let Some(artifact) = result_artifact {
            for key in [
                "activation_evidence",
                "activation_vs_execution_evidence",
                "lane_execution_receipt_artifact",
            ] {
                if let Some(value) = artifact.get(key) {
                    object
                        .entry(key.to_string())
                        .or_insert_with(|| value.clone());
                }
            }
            if !object.contains_key("activation_evidence") {
                if let Some(value) = artifact.get("activation_vs_execution_evidence") {
                    object.insert("activation_evidence".to_string(), value.clone());
                }
            }
        }
        for key in ["artifact_refs", "operator_contracts", "shared_fields"] {
            if let Some(section) = object.get_mut(key) {
                insert_dispatch_receipt_evidence_into_operator_section(
                    section,
                    dispatch_result_path,
                    receipt,
                );
            }
        }
    }
    payload
}

fn insert_dispatch_receipt_evidence_into_operator_section(
    section: &mut serde_json::Value,
    dispatch_result_path: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) {
    if section
        .as_object()
        .is_some_and(|object| object.contains_key("artifact_refs"))
    {
        if let Some(artifact_refs) = section.get_mut("artifact_refs") {
            insert_dispatch_receipt_evidence_into_operator_section(
                artifact_refs,
                dispatch_result_path,
                receipt,
            );
        }
        return;
    }
    let Some(object) = section.as_object_mut() else {
        return;
    };
    object.insert(
        "dispatch_result_path".to_string(),
        serde_json::json!(dispatch_result_path),
    );
    object.insert(
        "receipt_path".to_string(),
        serde_json::json!(dispatch_result_path),
    );
    object.insert(
        "lane_execution_receipt_path".to_string(),
        serde_json::json!(dispatch_result_path),
    );
    object.insert(
        "underlying_dispatch_blocker_code".to_string(),
        serde_json::json!(receipt.blocker_code),
    );
}

async fn agent_init_execute_dispatch_resume_error_receipt_evidence(
    store: &StateStore,
    error: &str,
    requested_dispatch_packet_path: Option<&str>,
) -> (
    Option<crate::state_store::RunGraphDispatchReceipt>,
    Option<serde_json::Value>,
) {
    let run_id = agent_init_execute_dispatch_resume_error_run_id(error).or_else(|| {
        requested_dispatch_packet_path
            .and_then(|path| super::taskflow_consume_resume::read_dispatch_packet(path).ok())
            .and_then(|packet| string_field(&packet, "run_id"))
    });
    let Some(run_id) = run_id else {
        return (None, None);
    };
    let receipt = store
        .run_graph_dispatch_receipt(&run_id)
        .await
        .ok()
        .flatten();
    let result_artifact = receipt
        .as_ref()
        .and_then(|receipt| receipt.dispatch_result_path.as_deref())
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok());
    (receipt, result_artifact)
}

fn orchestrator_init_projection_name(full: bool) -> &'static str {
    if full {
        "orchestrator-init-full-latest"
    } else {
        "orchestrator-init-summary-latest"
    }
}

fn compact_project_activation_summary(init_view: &serde_json::Value) -> serde_json::Value {
    let project_activation = &init_view["project_activation"];
    serde_json::json!({
        "status": project_activation["status"],
        "activation_pending": project_activation["activation_pending"],
        "project_shape": project_activation["project_shape"],
        "next_steps": project_activation["next_steps"],
        "host_environment": {
            "selected_cli_system": project_activation["host_environment"]["selected_cli_system"],
            "selected_cli_execution_class": project_activation["host_environment"]["selected_cli_execution_class"],
            "template_materialized": project_activation["host_environment"]["template_materialized"],
            "materialization_required": project_activation["host_environment"]["materialization_required"],
        }
    })
}

fn compact_dev_team_readiness_summary(dev_team_readiness: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "status": dev_team_readiness["status"],
        "sequence": dev_team_readiness["sequence"],
        "role_count": dev_team_readiness["roles"].as_array().map(|roles| roles.len()).unwrap_or(0),
        "flow_count": dev_team_readiness["flows"].as_array().map(|flows| flows.len()).unwrap_or(0),
        "active_selection": dev_team_readiness["active_selection"],
        "source_paths": dev_team_readiness["source_paths"],
    })
}

fn orchestrator_runtime_bundle_summary(
    bundle: &crate::TaskflowConsumeBundlePayload,
    state_dir: &Path,
) -> serde_json::Value {
    serde_json::json!({
        "bundle_id": bundle.metadata["bundle_id"],
        "root_artifact_id": bundle.control_core["root_artifact_id"],
        "activation_source": &bundle.activation_source,
        "vida_root": &bundle.vida_root,
        "state_dir": state_dir.display().to_string(),
        "launcher_runtime_paths": &bundle.launcher_runtime_paths,
    })
}

fn build_orchestrator_init_full_payload(
    init_view: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    orchestrator_runtime_contract: &serde_json::Value,
    bundle: &crate::TaskflowConsumeBundlePayload,
    state_dir: &Path,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida orchestrator-init",
        "view": "full",
        "state_read": {
            "mode": "authoritative_open",
            "lock_resilient": true,
            "fallback": "degraded_lock_contention_surface"
        },
        "init": init_view,
        "dev_team_readiness": dev_team_readiness,
        "orchestrator_runtime_contract": orchestrator_runtime_contract,
        "continuation_binding": init_view["continuation_binding"],
        "active_bounded_unit": init_view["continuation_binding"]["active_bounded_unit"],
        "why_this_unit": init_view["continuation_binding"]["why_this_unit"],
        "sequential_vs_parallel_posture": init_view["continuation_binding"]["sequential_vs_parallel_posture"],
        "runtime_bundle_summary": orchestrator_runtime_bundle_summary(bundle, state_dir),
    })
}

fn build_orchestrator_init_summary_payload(
    init_view: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
    orchestrator_runtime_contract: &serde_json::Value,
    bundle: &crate::TaskflowConsumeBundlePayload,
    state_dir: &Path,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida orchestrator-init",
        "view": "summary",
        "status": init_view["status"],
        "full_output_available": true,
        "full_output_command": "vida orchestrator-init --full --json",
        "state_read": {
            "mode": "authoritative_open",
            "lock_resilient": true,
            "fallback": "degraded_lock_contention_surface"
        },
        "init": {
            "status": init_view["status"],
            "local_runtime_surface": init_view["local_runtime_surface"],
            "boot_surface": init_view["boot_surface"],
            "continuation_binding": init_view["continuation_binding"],
            "project_activation": compact_project_activation_summary(init_view),
            "project_root": init_view["project_root"],
            "root_artifact_id": init_view["root_artifact_id"],
        },
        "continuation_binding": init_view["continuation_binding"],
        "active_bounded_unit": init_view["continuation_binding"]["active_bounded_unit"],
        "why_this_unit": init_view["continuation_binding"]["why_this_unit"],
        "sequential_vs_parallel_posture": init_view["continuation_binding"]["sequential_vs_parallel_posture"],
        "next_lawful_dispatch_action": orchestrator_runtime_contract["next_lawful_dispatch_action"],
        "dev_team_readiness_summary": compact_dev_team_readiness_summary(dev_team_readiness),
        "runtime_bundle_summary": orchestrator_runtime_bundle_summary(bundle, state_dir),
    })
}

fn cached_orchestrator_init_payload_has_top_level_continuation_fields(cached: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(cached)
        .ok()
        .is_some_and(|payload| {
            !payload
                .get("active_bounded_unit")
                .unwrap_or(&serde_json::Value::Null)
                .is_null()
                && !payload
                    .get("why_this_unit")
                    .unwrap_or(&serde_json::Value::Null)
                    .is_null()
                && payload
                    .get("sequential_vs_parallel_posture")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        })
}

fn build_orchestrator_runtime_contract(
    init_view: &serde_json::Value,
    dev_team_readiness: &serde_json::Value,
) -> serde_json::Value {
    let activation_pending = init_view["project_activation"]["activation_pending"]
        .as_bool()
        .unwrap_or(false);
    let default_topology =
        init_view["project_activation"]["normal_work_defaults"]["default_agent_topology"].clone();
    let configured_flows = dev_team_readiness["flows"].clone();
    let configured_roles = dev_team_readiness["roles"].clone();
    let next_lawful_dispatch_action = if activation_pending {
        serde_json::json!({
            "status": "blocked_pending_activation",
            "surface": "vida project-activator",
            "command": "vida project-activator --json",
            "reason": "project activation must complete before normal dispatch"
        })
    } else {
        serde_json::json!({
            "status": "preview_required",
            "surface": "vida agent dispatch-next",
            "command": "vida agent dispatch-next --dev-team --json",
            "reason": "review configured carrier/model/cost truth before any `vida agent-init` execution dispatch"
        })
    };
    serde_json::json!({
        "sticky_user_execution_intent": {
            "agent_first_or_parallel_agent_execution_is_sticky": true,
            "host_local_write_capability_is_not_authority": true,
            "source_surfaces": [
                "AGENTS.md",
                "AGENTS.sidecar.md",
                "vida status --json.root_session_write_guard"
            ]
        },
        "allowed_topology": {
            "default_agent_topology": default_topology,
            "flows": configured_flows,
            "roles": configured_roles,
            "topology_source": "vida.config.yaml via dev_team_readiness"
        },
        "next_lawful_dispatch_action": next_lawful_dispatch_action,
        "execution_evidence_contract": {
            "agent_init_without_execute_dispatch": "activation_view_only",
            "activation_view_is_execution_evidence": false,
            "activation_view_completes_delegated_work": false,
            "delegated_work_completion_requires": "receipt_backed_execution_evidence",
            "missing_execution_evidence_semantics": "non_executing_bridge_blocker"
        },
        "write_and_continuation_authority_contract": {
            "root_local_write_allowed_is_blanket_authority": false,
            "exception_takeover_authority": "path_scoped_owned_write_scope_only",
            "root_write_scope_field": "vida status --json.root_session_write_guard.root_local_write_allowed_for_only_these_paths",
            "continuation_binding_is_independent_of_exception_write_scope": true,
            "continuation_requires_explicit_fields": [
                "active_bounded_unit",
                "why_this_unit",
                "sequential_vs_parallel_posture"
            ],
            "scoped_exception_takeover_does_not_select_next_work": true
        },
        "hard_warnings": [
            "User requested agent-first earlier; root-local implementation is currently a policy violation unless explicitly superseded.",
            "`vida agent-init` without `--execute-dispatch` is activation/view-only and is not delegated work completion.",
            "`root_local_write_allowed=true` is path-scoped exception authority only; continuation still requires an explicit bounded-unit binding."
        ]
    })
}

fn agent_init_dispatch_mode(
    args: &AgentInitArgs,
    selection: &serde_json::Value,
) -> serde_json::Value {
    let has_packet = args.dispatch_packet.is_some() || args.downstream_packet.is_some();
    let mode = if args.execute_dispatch {
        "execution_dispatch"
    } else if has_packet {
        "packet_activation_view_only"
    } else {
        "activation_view_only"
    };
    serde_json::json!({
        "mode": mode,
        "requested_execute_dispatch": args.execute_dispatch,
        "has_packet_source": has_packet,
        "auto_dispatch_packet": args.auto_dispatch_packet,
        "selection_mode": selection["mode"].clone(),
        "activation_view_only": !args.execute_dispatch,
        "execution_dispatch": args.execute_dispatch,
        "activation_view_is_execution_evidence": false,
        "activation_view_completes_delegated_work": false,
        "execution_evidence_required_for_completion": true,
        "completion_requires_receipt_backed_execution": true,
        "required_completion_evidence": "receipt_backed_execution_evidence",
        "missing_execution_evidence_semantics": "non_executing_bridge_blocker",
        "root_session_write_authority_granted": false,
        "continuation_authority_granted": false,
    })
}

fn agent_init_execute_dispatch_missing_packet_payload(
    dispatch_mode: &serde_json::Value,
) -> serde_json::Value {
    let blocker_codes = vec!["agent_init_execute_dispatch_missing_packet"];
    let next_actions = vec![
        "Create or refresh a scheduler dispatch packet with `vida taskflow run-graph dispatch-init <task-id> --json`.",
        "Retry execution with `vida agent-init --dispatch-packet <path> --execute-dispatch --json` or `vida agent-init --downstream-packet <path> --execute-dispatch --json`.",
        "Do not treat packetless `vida agent-init --execute-dispatch` as activation, execution, or completion evidence.",
    ];
    let artifact_refs = serde_json::json!({
        "surface": "vida agent-init",
        "required_packet_flags": ["--dispatch-packet", "--downstream-packet"],
        "receipt_backed_execution_required": true,
    });
    serde_json::json!({
        "surface": "vida agent-init",
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_mode": dispatch_mode,
        "error": AGENT_INIT_EXECUTE_DISPATCH_MISSING_PACKET_ERROR,
        "blocker_code": "agent_init_execute_dispatch_missing_packet",
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs
        }
    })
}

fn validate_agent_init_auto_dispatch_packet_args(
    args: &AgentInitArgs,
    packet_arg_count: usize,
) -> Result<(), String> {
    if !args.auto_dispatch_packet {
        return Ok(());
    }
    if !args.execute_dispatch {
        return Err(
            "`--auto-dispatch-packet` is only valid with `--execute-dispatch`.".to_string(),
        );
    }
    if packet_arg_count > 0 {
        return Err(
            "`--auto-dispatch-packet` is exclusive with `--dispatch-packet` and `--downstream-packet`."
                .to_string(),
        );
    }
    if args.role.is_some() || args.request_text.is_some() {
        return Err(
            "`--auto-dispatch-packet` uses the active bounded runtime unit; do not combine it with `--role` or request text."
                .to_string(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentInitAutoDispatchActiveUnitError {
    blocker_code: &'static str,
    detail: String,
    active_task_id: Option<String>,
    resolved_run_id: String,
    lineage_task_ids: Vec<String>,
}

fn agent_init_auto_dispatch_lineage_task_ids(
    resume_inputs: &super::taskflow_consume_resume::ResumeInputs,
) -> Vec<String> {
    let mut ids = std::collections::BTreeSet::new();
    let run_id = resume_inputs.dispatch_receipt.run_id.trim();
    if !run_id.is_empty() {
        ids.insert(run_id.to_string());
    }
    for key in ["run_id", "task_id"] {
        if let Some(value) = resume_inputs
            .run_graph_bootstrap
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            ids.insert(value.to_string());
        }
    }
    ids.into_iter().collect()
}

fn validate_agent_init_auto_dispatch_active_unit_ids(
    active_task_ids: Vec<String>,
    lineage_task_ids: Vec<String>,
    resolved_run_id: &str,
) -> Result<(), AgentInitAutoDispatchActiveUnitError> {
    match active_task_ids.as_slice() {
        [] => Err(AgentInitAutoDispatchActiveUnitError {
            blocker_code: "auto_dispatch_packet_active_unit_missing",
            detail: "`--auto-dispatch-packet` requires one active non-container task.".to_string(),
            active_task_id: None,
            resolved_run_id: resolved_run_id.to_string(),
            lineage_task_ids,
        }),
        [active_task_id] => {
            if lineage_task_ids
                .iter()
                .any(|task_id| task_id == active_task_id)
            {
                Ok(())
            } else {
                Err(AgentInitAutoDispatchActiveUnitError {
                    blocker_code: "auto_dispatch_packet_active_unit_mismatch",
                    detail: format!(
                        "`--auto-dispatch-packet` resolved run `{resolved_run_id}` but active bounded unit is `{active_task_id}`."
                    ),
                    active_task_id: Some(active_task_id.clone()),
                    resolved_run_id: resolved_run_id.to_string(),
                    lineage_task_ids,
                })
            }
        }
        _ => Err(AgentInitAutoDispatchActiveUnitError {
            blocker_code: "auto_dispatch_packet_active_unit_ambiguous",
            detail: "`--auto-dispatch-packet` requires exactly one active non-container task."
                .to_string(),
            active_task_id: None,
            resolved_run_id: resolved_run_id.to_string(),
            lineage_task_ids,
        }),
    }
}

async fn validate_agent_init_auto_dispatch_active_unit(
    store: &state_store::StateStore,
    resume_inputs: &super::taskflow_consume_resume::ResumeInputs,
) -> Result<(), AgentInitAutoDispatchActiveUnitError> {
    let active_task_ids = store
        .list_tasks(Some("in_progress"), false)
        .await
        .map_err(|error| AgentInitAutoDispatchActiveUnitError {
            blocker_code: "auto_dispatch_packet_active_unit_unavailable",
            detail: format!("Failed to read active task state for auto dispatch: {error}"),
            active_task_id: None,
            resolved_run_id: resume_inputs.dispatch_receipt.run_id.clone(),
            lineage_task_ids: agent_init_auto_dispatch_lineage_task_ids(resume_inputs),
        })?
        .into_iter()
        .filter(|task| !crate::state_store::work_item_is_program_container(&task.issue_type))
        .map(|task| task.id)
        .collect::<Vec<_>>();
    validate_agent_init_auto_dispatch_active_unit_ids(
        active_task_ids,
        agent_init_auto_dispatch_lineage_task_ids(resume_inputs),
        &resume_inputs.dispatch_receipt.run_id,
    )
}

fn agent_init_auto_dispatch_active_unit_blocked_payload(
    dispatch_mode: &serde_json::Value,
    error: &AgentInitAutoDispatchActiveUnitError,
    dispatch_packet_path: &str,
) -> serde_json::Value {
    let blocker_codes = vec![error.blocker_code];
    let next_actions = vec![
        "Bind the intended bounded unit explicitly or pass the exact dispatch packet path for that run.".to_string(),
        "Do not execute a stale latest dispatch packet through `--auto-dispatch-packet`.".to_string(),
    ];
    let artifact_refs = serde_json::json!({
        "surface": "vida agent-init",
        "dispatch_packet_path": dispatch_packet_path,
        "resolved_run_id": error.resolved_run_id,
        "active_task_id": error.active_task_id,
        "lineage_task_ids": error.lineage_task_ids,
        "auto_dispatch_packet": true,
    });
    serde_json::json!({
        "surface": "vida agent-init",
        "status": "blocked",
        "execution_state": "blocked",
        "dispatch_mode": dispatch_mode,
        "error": error.detail,
        "blocker_code": error.blocker_code,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
        "operator_contracts": {
            "contract_id": "release-1-operator-contracts",
            "schema_version": "release-1-v1",
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs,
            "risk_tier": null,
            "trace_id": null,
            "workflow_class": null
        },
        "shared_fields": {
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
            "artifact_refs": artifact_refs
        }
    })
}

fn emit_agent_init_execute_dispatch_missing_packet(args: &AgentInitArgs) -> ExitCode {
    if args.json {
        let dispatch_mode = agent_init_dispatch_mode(args, &serde_json::Value::Null);
        crate::print_json_pretty(&agent_init_execute_dispatch_missing_packet_payload(
            &dispatch_mode,
        ));
    } else {
        eprintln!("{AGENT_INIT_EXECUTE_DISPATCH_MISSING_PACKET_ERROR}");
    }
    ExitCode::from(2)
}

fn agent_init_packet_execute_command(selection: &serde_json::Value) -> Option<String> {
    selection
        .get("dispatch_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(|path| {
            format!(
                "vida agent-init --dispatch-packet {} --execute-dispatch --json",
                crate::shell_quote(path)
            )
        })
        .or_else(|| {
            selection
                .get("downstream_packet_path")
                .and_then(serde_json::Value::as_str)
                .map(|path| {
                    format!(
                        "vida agent-init --downstream-packet {} --execute-dispatch --json",
                        crate::shell_quote(path)
                    )
                })
        })
}

fn agent_init_operator_guidance(
    selection: &serde_json::Value,
    activation_semantics: &serde_json::Value,
    dispatch_mode: &serde_json::Value,
) -> serde_json::Value {
    let execute_command = agent_init_packet_execute_command(selection);
    let current_mode = dispatch_mode
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("activation_view_only");
    let execution_dispatch = dispatch_mode["execution_dispatch"]
        .as_bool()
        .unwrap_or(false);
    let next_lawful_execution_action = if execution_dispatch {
        "wait for the execute-dispatch result artifact and require receipt-backed worker evidence before claiming completion".to_string()
    } else if let Some(command) = execute_command.as_deref() {
        format!(
            "run `{command}` for a packet-backed execution attempt; do not treat this activation view as completion"
        )
    } else {
        "create or refresh a scheduler dispatch packet first, then run `vida agent-init --dispatch-packet <path> --execute-dispatch --json` for execution evidence".to_string()
    };

    serde_json::json!({
        "current_surface_contract": {
            "mode": current_mode,
            "activation_kind": activation_semantics
                .get("activation_kind")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("activation_view")),
            "view_only": activation_semantics
                .get("view_only")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(true)),
            "executes_packet": execution_dispatch,
            "is_completion_evidence": false,
            "records_completion_receipt": activation_semantics
                .get("records_completion_receipt")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(false)),
            "transfers_root_session_write_authority": activation_semantics
                .get("transfers_root_session_write_authority")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(false)),
        },
        "flow_distinctions": [
            {
                "stage": "startup_activation_view",
                "surface": "vida agent-init --role <runtime-role> <task-id> --json",
                "executes_packet": false,
                "records_completion_receipt": false,
                "meaning": "bounded lane startup/context only"
            },
            {
                "stage": "packet_backed_execution_attempt",
                "surface": execute_command
                    .as_deref()
                    .unwrap_or("vida agent-init --dispatch-packet <path> --execute-dispatch --json"),
                "executes_packet": true,
                "records_completion_receipt": "only_on_success",
                "meaning": "attempts the bounded delegated packet and must return execution evidence"
            },
            {
                "stage": "receipt_backed_worker_execution",
                "surface": "dispatch result / run-graph dispatch receipt",
                "executes_packet": true,
                "records_completion_receipt": true,
                "meaning": "the only completion evidence that can support delegated work completion or write-authority decisions"
            }
        ],
        "next_lawful_execution_action": next_lawful_execution_action,
    })
}

async fn best_effort_record_agent_init_dispatch_timeout_receipt(
    state_root: &Path,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    execute_dispatch_timeout_seconds: u64,
) -> Option<String> {
    let store = match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    {
        Ok(Ok(store)) => store,
        Ok(Err(error)) => {
            return Some(format!(
                "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to reopen authoritative state store: {error}"
            ));
        }
        Err(_) => {
            return Some(format!(
                "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: timed out reopening authoritative state store after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
            ));
        }
    };
    if let Err(error) = store.record_run_graph_dispatch_receipt(receipt).await {
        return Some(format!(
            "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to persist timeout-blocked dispatch receipt: {error}"
        ));
    }
    if let Some(run_id) = run_graph_bootstrap
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    {
        match store.run_graph_status(run_id).await {
            Ok(status) => {
                if let Err(error) =
                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                        &store,
                        &status,
                        "agent_init_execute_dispatch_timeout",
                    )
                    .await
                {
                    return Some(format!(
                        "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to synchronize continuation binding after timeout: {error}"
                    ));
                }
            }
            Err(error) => {
                return Some(format!(
                    "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to read run-graph status after timeout: {error}"
                ));
            }
        }
    }
    None
}

pub(crate) fn resolve_init_bootstrap_source_root() -> PathBuf {
    if let Some(installed_root) = resolve_installed_runtime_root() {
        for candidate in installed_runtime_source_root_candidates(&installed_root) {
            if looks_like_init_bootstrap_source_root(&candidate) {
                return candidate;
            }
        }
    }
    super::repo_runtime_root()
}

pub(crate) fn resolve_installed_runtime_root() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let root = bin_dir.parent()?;
    taskflow_binary_candidates_for_root(root)
        .into_iter()
        .next()
        .map(|_| root.to_path_buf())
}

pub(crate) fn installed_runtime_source_root_candidates(root: &Path) -> Vec<PathBuf> {
    let current_root = root.join("current");
    if current_root == root {
        vec![root.to_path_buf()]
    } else {
        vec![current_root, root.to_path_buf()]
    }
}

pub(crate) fn taskflow_binary_candidates_for_root(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let bin_dir = root.join("bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_binary = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_file() && is_taskflow_binary {
                candidates.push(path);
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_runtime_dir = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_dir() && is_taskflow_runtime_dir {
                let candidate = path.join("src/vida");
                if candidate.exists() {
                    candidates.push(candidate);
                }
            }
        }
    }

    candidates
}

pub(crate) fn looks_like_init_bootstrap_source_root(root: &Path) -> bool {
    resolve_init_agents_source(root).is_ok()
        && resolve_init_sidecar_source(root).is_ok()
        && resolve_init_config_template_source(root).is_ok()
        && root
            .join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .is_dir()
        && root
            .join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT)
            .is_dir()
        && looks_like_host_runtime_source_root(root)
}

pub(crate) fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

pub(crate) fn resolve_init_agents_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.scaffold.md"),
        root.join("AGENTS.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve generated AGENTS scaffold. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_sidecar_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.sidecar.scaffold.md"),
        root.join("AGENTS.sidecar.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve project sidecar source. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_config_template_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/vida.config.yaml.template"),
        root.join("docs/framework/templates/vida.config.yaml.template"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve vida.config.yaml template. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_feature_design_template_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/feature-design-document.template.md"),
        root.join("docs/framework/templates/feature-design-document.template.md"),
        root.join("docs/product/spec/templates/feature-design-document.template.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve framework feature-design template source. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run;
    use crate::runtime_dispatch_state::{
        write_runtime_dispatch_packet, RuntimeDispatchPacketContext,
    };
    use crate::state_store::{RunGraphDispatchReceipt, RunGraphStatus, StateStore};
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir, EnvVarGuard};
    use clap::CommandFactory;
    use serde_json::json;
    use std::fs;
    use std::process::ExitCode;
    use std::time::{Duration, Instant};

    fn wait_for_state_unlock(state_dir: &Path) {
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn agent_init_execute_dispatch_timeout_honors_dispatch_window_with_reconciliation_grace() {
        assert_eq!(agent_init_execute_dispatch_timeout_seconds(1), 21);
        assert!(
            agent_init_execute_dispatch_timeout_seconds(45) > 45,
            "agent-init outer timeout must leave room for dispatch timeout reconciliation"
        );
        assert!(
            agent_init_execute_dispatch_timeout_seconds(90) > 90,
            "agent-init outer timeout must outlive the external CLI wall timeout"
        );
        assert_eq!(agent_init_execute_dispatch_timeout_seconds(240), 260);
        assert_eq!(
            agent_init_receipt_timeout_seconds(
                240,
                agent_init_execute_dispatch_timeout_seconds(240)
            ),
            240
        );
        assert_eq!(agent_init_execute_dispatch_handoff_threshold_seconds(), 2);
    }

    #[test]
    fn agent_init_execute_dispatch_window_handoff_starts_at_operator_target() {
        assert!(
            !agent_init_execute_dispatch_window_requires_operator_handoff(1),
            "sub-target handoffs can complete synchronously"
        );
        assert!(
            agent_init_execute_dispatch_window_requires_operator_handoff(2),
            "2s dispatch windows must return operator-visible in-flight evidence instead of waiting for terminal agent output"
        );
    }

    #[test]
    fn orchestrator_init_summary_payload_projects_top_level_continuation_fields() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let bundle = crate::taskflow_runtime_bundle::blocking_runtime_bundle("test");
        let init_view = json!({
            "status": "ready_enough_for_normal_work",
            "local_runtime_surface": "vida orchestrator-init",
            "boot_surface": "vida boot",
            "project_activation": {
                "activation_pending": false
            },
            "project_root": harness.path().display().to_string(),
            "root_artifact_id": "root",
            "continuation_binding": {
                "status": "bound",
                "active_bounded_unit": {
                    "kind": "task_graph_task",
                    "task_id": "active-task"
                },
                "why_this_unit": "Active task is authoritative.",
                "sequential_vs_parallel_posture": "sequential_only_taskflow_active"
            }
        });
        let payload = build_orchestrator_init_summary_payload(
            &init_view,
            &json!({ "flows": [], "roles": [] }),
            &json!({ "next_lawful_dispatch_action": {} }),
            &bundle,
            harness.path(),
        );

        assert_eq!(payload["active_bounded_unit"]["task_id"], "active-task");
        assert_eq!(payload["why_this_unit"], "Active task is authoritative.");
        assert_eq!(
            payload["sequential_vs_parallel_posture"],
            "sequential_only_taskflow_active"
        );
        assert_eq!(
            payload["init"]["continuation_binding"]["active_bounded_unit"]["task_id"],
            "active-task"
        );
        assert!(
            cached_orchestrator_init_payload_has_top_level_continuation_fields(
                &payload.to_string()
            )
        );
        assert!(
            !cached_orchestrator_init_payload_has_top_level_continuation_fields(
                &json!({
                    "surface": "vida orchestrator-init",
                    "status": "ready_enough_for_normal_work",
                    "init": {
                        "continuation_binding": {
                            "active_bounded_unit": {
                                "task_id": "nested-only"
                            }
                        }
                    }
                })
                .to_string()
            )
        );
    }

    #[cfg(windows)]
    #[test]
    fn agent_init_dispatch_worker_uses_detached_windows_process_flags() {
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;

        let flags = windows_dispatch_worker_creation_flags();
        assert_eq!(flags & DETACHED_PROCESS, DETACHED_PROCESS);
        assert_eq!(flags & CREATE_NEW_PROCESS_GROUP, CREATE_NEW_PROCESS_GROUP);
        assert_eq!(flags & CREATE_NO_WINDOW, CREATE_NO_WINDOW);
        assert_eq!(flags & CREATE_BREAKAWAY_FROM_JOB, CREATE_BREAKAWAY_FROM_JOB);
    }

    #[cfg(windows)]
    #[test]
    fn agent_init_dispatch_worker_command_line_quotes_windows_args() {
        let command_line = windows_command_line(
            std::path::Path::new(r"C:\Program Files\VIDA\vida.exe"),
            &[
                "agent-init".to_string(),
                "--downstream-packet".to_string(),
                r"C:\project\vida-stack\.vida\data\packet path.json".to_string(),
                r#"quote"inside"#.to_string(),
                r#"trail\"#.to_string(),
            ],
        );

        assert!(command_line.starts_with("\"C:\\Program Files\\VIDA\\vida.exe\""));
        assert!(command_line.contains("--downstream-packet"));
        assert!(command_line.contains("\"C:\\project\\vida-stack\\.vida\\data\\packet path.json\""));
        assert!(command_line.contains("\"quote\\\"inside\""));
        assert!(
            command_line.ends_with("\"trail\\\\\""),
            "trailing backslashes must be doubled before the closing quote"
        );
    }

    #[test]
    fn orchestrator_init_cache_policy_rejects_state_marker_stale_summary_projection() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let payload = json!({
            "surface": "vida orchestrator-init",
            "view": "summary",
            "status": "pass",
            "init": {
                "continuation_binding": {
                    "status": "ambiguous",
                    "active_bounded_unit": serde_json::Value::Null,
                    "why_this_unit": serde_json::Value::Null,
                    "sequential_vs_parallel_posture": "unknown_until_explicit_taskflow_binding"
                }
            }
        });
        crate::operator_projection_cache::write_json_projection(
            harness.path(),
            orchestrator_init_projection_name(false),
            &payload,
        );

        assert!(
            crate::operator_projection_cache::read_fresh_json_projection(
                harness.path(),
                orchestrator_init_projection_name(false)
            )
            .is_some()
        );

        std::thread::sleep(Duration::from_millis(10));
        crate::operator_projection_cache::touch_state_mutation_marker(harness.path());

        assert!(
            crate::operator_projection_cache::read_fresh_json_projection(
                harness.path(),
                orchestrator_init_projection_name(false)
            )
            .is_none()
        );
    }

    fn run_on_cli_runtime_stack(name: &str, test: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .name(name.to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(test)
            .expect("cli-stack test thread should spawn");
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
    }

    fn agent_lane_test_execution_plan(executor_backend: &str) -> serde_json::Value {
        json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "junior",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "implementer": {
                    "executor_backend": executor_backend
                }
            }
        })
    }

    #[test]
    fn dispatch_packet_execute_resume_inputs_decode_without_store() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let packet_path = harness.path().join("dispatch-packet.json");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "packet".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "coach review".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("internal_subagents"),
            reason: "test".to_string(),
        };
        fs::write(
            &packet_path,
            serde_json::to_string_pretty(&json!({
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "coach_review_packet": {},
                "run_id": "run-fast-dispatch",
                "dispatch_target": "coach",
                "dispatch_status": "packet_ready",
                "lane_status": "packet_ready",
                "dispatch_kind": "agent_lane",
                "dispatch_surface": "vida agent-init",
                "dispatch_command": "vida agent-init --dispatch-packet packet --execute-dispatch --json",
                "activation_agent_type": "middle",
                "activation_runtime_role": "coach",
                "selected_backend": "internal_subagents",
                "role_selection_full": role_selection,
                "run_graph_bootstrap": {
                    "run_id": "run-fast-dispatch"
                }
            }))
            .expect("packet should serialize"),
        )
        .expect("packet should write");

        let inputs = resume_inputs_from_dispatch_packet_without_store(
            packet_path.to_str().expect("packet path should render"),
        )
        .expect("dispatch packet should decode without state store");

        assert_eq!(inputs.dispatch_receipt.run_id, "run-fast-dispatch");
        assert_eq!(inputs.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(
            inputs.dispatch_receipt.selected_backend.as_deref(),
            Some("internal_subagents")
        );
        assert_eq!(
            inputs.dispatch_receipt.dispatch_packet_path.as_deref(),
            packet_path.to_str()
        );
        assert_eq!(inputs.role_selection.selected_role, "coach");
        assert_eq!(inputs.run_graph_bootstrap["run_id"], "run-fast-dispatch");

        let quoted_path = format!("'{}'", packet_path.display());
        let quoted_inputs = resume_inputs_from_dispatch_packet_without_store(&quoted_path)
            .expect("quoted dispatch packet path should normalize before read");
        assert_eq!(quoted_inputs.dispatch_receipt.run_id, "run-fast-dispatch");
        assert_eq!(
            quoted_inputs
                .dispatch_receipt
                .dispatch_packet_path
                .as_deref(),
            packet_path.to_str()
        );
    }

    #[test]
    fn boot_command_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        assert_eq!(
            runtime.block_on(run(super::super::Cli {
                command: Some(super::super::Command::Boot(BootArgs {
                    state_dir: Some(harness.path().to_path_buf()),
                    render: RenderMode::Plain,
                    instruction_source_root: None,
                    framework_memory_source_root: None,
                    extra_args: Vec::new(),
                })),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn boot_with_extra_argument_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(run(cli(&["boot", "unexpected"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn clap_help_lists_project_activator() {
        let mut command = crate::Cli::command();
        let help = command.render_long_help().to_string();
        assert!(
            help.contains("project-activator"),
            "project-activator should be present in help"
        );
    }

    #[test]
    fn init_bootstrap_source_requires_bootstrap_markers() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join("bin")).expect("bin dir should exist");
        fs::write(root.join("bin/taskflow"), "#!/bin/sh\n").expect("taskflow marker should exist");
        assert!(
            !looks_like_init_bootstrap_source_root(root),
            "taskflow binary alone should not qualify as an init bootstrap source"
        );

        fs::create_dir_all(root.join("install/assets")).expect("install assets dir should exist");
        fs::create_dir_all(root.join(".codex")).expect(".codex dir should exist");
        fs::write(
            root.join("install/assets/AGENTS.scaffold.md"),
            "# scaffold\n",
        )
        .expect("generated AGENTS scaffold should exist");
        fs::write(root.join("AGENTS.sidecar.md"), "# sidecar\n")
            .expect("project sidecar should exist");
        fs::write(
            root.join("install/assets/vida.config.yaml.template"),
            concat!(
                "project:\n",
                "  id: demo\n",
                "host_environment:\n",
                "  systems:\n",
                "    codex:\n",
                "      template_root: .codex\n",
                "      runtime_root: .codex\n",
            ),
        )
        .expect("config template should exist");
        assert!(
            !looks_like_init_bootstrap_source_root(root),
            "bootstrap source should not qualify until framework instruction bundles are present"
        );
        fs::create_dir_all(root.join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT))
            .expect("instruction bundle source should exist");
        fs::create_dir_all(root.join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT))
            .expect("framework memory source should exist");
        assert!(
            looks_like_init_bootstrap_source_root(root),
            "bootstrap source should require init assets and framework-owned bundle sources"
        );
    }

    #[test]
    fn installed_runtime_source_root_candidates_prefer_current_layout() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let install_root = harness.path().join("vida-install");
        let current_root = install_root.join("current");
        fs::create_dir_all(current_root.join("bin")).expect("current bin dir should exist");
        fs::write(current_root.join("bin/taskflow"), "#!/bin/sh\n")
            .expect("current taskflow marker should exist");
        fs::create_dir_all(current_root.join("install/assets"))
            .expect("current install assets dir should exist");
        fs::create_dir_all(current_root.join(".codex")).expect("current .codex dir should exist");
        fs::write(
            current_root.join("install/assets/AGENTS.scaffold.md"),
            "# scaffold\n",
        )
        .expect("current generated AGENTS scaffold should exist");
        fs::write(current_root.join("AGENTS.sidecar.md"), "# sidecar\n")
            .expect("current sidecar should exist");
        fs::write(
            current_root.join("install/assets/vida.config.yaml.template"),
            concat!(
                "project:\n",
                "  id: demo\n",
                "host_environment:\n",
                "  systems:\n",
                "    codex:\n",
                "      template_root: .codex\n",
                "      runtime_root: .codex\n",
            ),
        )
        .expect("current config template should exist");
        fs::create_dir_all(current_root.join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT))
            .expect("current instruction bundle source should exist");
        fs::create_dir_all(current_root.join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT))
            .expect("current framework memory source should exist");

        let candidates = installed_runtime_source_root_candidates(&install_root);
        assert_eq!(candidates[0], current_root);
        assert!(
            looks_like_init_bootstrap_source_root(&candidates[0]),
            "installed `current/` layout should be recognized as the bootstrap source root"
        );
    }

    #[test]
    fn init_preserves_existing_agents_as_sidecar_when_missing() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");
        fs::write(
            harness.path().join("AGENTS.md"),
            "project documentation: docs/\n",
        )
        .expect("existing agents should be written");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        let sidecar = fs::read_to_string(harness.path().join("AGENTS.sidecar.md"))
            .expect("sidecar should exist");
        assert!(
            sidecar.contains("Project Agent Instructions"),
            "sidecar should use the project instruction overlay scaffold"
        );
        assert!(
            sidecar.contains("Migrated Project Instructions"),
            "pre-init project instructions should be embedded, not used as the whole sidecar"
        );
        assert!(sidecar.contains("project documentation: docs/"));
        let framework_agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("framework agents should exist");
        assert!(
            framework_agents.contains("VIDA Project Bootstrap Carrier"),
            "generated bootstrap carrier should replace root AGENTS.md"
        );
    }

    #[test]
    fn init_replaces_agents_template_and_keeps_existing_sidecar_with_backup() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");

        fs::write(
            harness.path().join("AGENTS.md"),
            "project-specific bootstrap notes\n",
        )
        .expect("existing agents should be written");
        fs::write(
            harness.path().join("AGENTS.sidecar.md"),
            "current sidecar content\n",
        )
        .expect("existing sidecar should be written");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let framework_agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("framework agents should exist");
        assert!(
            framework_agents.contains("VIDA Project Bootstrap Carrier"),
            "generated bootstrap carrier should replace root AGENTS.md"
        );

        let sidecar = fs::read_to_string(harness.path().join("AGENTS.sidecar.md"))
            .expect("sidecar should still exist");
        assert_eq!(sidecar, "current sidecar content\n");

        let backup = fs::read_to_string(
            harness
                .path()
                .join(".vida/receipts/AGENTS.pre-init.backup.md"),
        )
        .expect("agents backup should be written");
        assert!(
            backup.contains("archived legacy snapshot"),
            "backup should be explicitly inactive"
        );
        assert!(backup.contains("project-specific bootstrap notes"));
    }

    #[test]
    fn init_with_extra_argument_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(run(cli(&["init", "unexpected"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn init_materializes_and_refreshes_framework_instruction_bundles_from_explicit_sources() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");
        let source_root = harness.path().join("source");
        let instruction_source = source_root.join("framework-source");
        let memory_source = source_root.join("framework-memory-source");
        fs::create_dir_all(instruction_source.join("framework"))
            .expect("instruction source should exist");
        fs::create_dir_all(&memory_source).expect("memory source should exist");
        fs::write(
            instruction_source.join("framework/agent-definition.md"),
            "agent definition v1\n",
        )
        .expect("instruction file should write");
        fs::write(memory_source.join("framework-memory.md"), "memory v1\n")
            .expect("memory file should write");

        assert_eq!(
            runtime.block_on(run(cli(&[
                "init",
                "--instruction-source-root",
                instruction_source.to_str().expect("path should be utf8"),
                "--framework-memory-source-root",
                memory_source.to_str().expect("path should be utf8"),
            ]))),
            ExitCode::SUCCESS
        );
        assert_eq!(
            fs::read_to_string(harness.path().join(
                "vida/config/instructions/bundles/framework-source/framework/agent-definition.md"
            ))
            .expect("materialized instruction file should exist"),
            "agent definition v1\n"
        );
        assert_eq!(
            fs::read_to_string(harness.path().join(
                "vida/config/instructions/bundles/framework-memory-source/framework-memory.md"
            ))
            .expect("materialized memory file should exist"),
            "memory v1\n"
        );

        fs::write(
            instruction_source.join("framework/agent-definition.md"),
            "agent definition v2\n",
        )
        .expect("instruction file should update");
        fs::write(memory_source.join("framework-memory.md"), "memory v2\n")
            .expect("memory file should update");
        assert_eq!(
            runtime.block_on(run(cli(&[
                "init",
                "--instruction-source-root",
                instruction_source.to_str().expect("path should be utf8"),
                "--framework-memory-source-root",
                memory_source.to_str().expect("path should be utf8"),
            ]))),
            ExitCode::SUCCESS
        );

        assert_eq!(
            fs::read_to_string(harness.path().join(
                "vida/config/instructions/bundles/framework-source/framework/agent-definition.md"
            ))
            .expect("refreshed instruction file should exist"),
            "agent definition v2\n"
        );
        assert_eq!(
            fs::read_to_string(harness.path().join(
                "vida/config/instructions/bundles/framework-memory-source/framework-memory.md"
            ))
            .expect("refreshed memory file should exist"),
            "memory v2\n"
        );
    }

    #[test]
    fn project_activator_repair_restores_framework_instruction_bundles() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        fs::remove_dir_all(
            harness
                .path()
                .join("vida/config/instructions/bundles/framework-source"),
        )
        .expect("framework-source bundle should be removable");
        fs::remove_dir_all(
            harness
                .path()
                .join("vida/config/instructions/bundles/framework-memory-source"),
        )
        .expect("framework-memory-source bundle should be removable");

        assert_eq!(
            runtime.block_on(run(cli(&["project-activator", "--repair", "--json"]))),
            ExitCode::SUCCESS
        );
        assert!(harness
            .path()
            .join("vida/config/instructions/bundles/framework-source/framework/agent-definition.md")
            .is_file());
        assert!(harness
            .path()
            .join("vida/config/instructions/bundles/framework-memory-source/framework-memory.md")
            .is_file());
    }

    #[test]
    fn agent_init_timeout_reconciliation_defers_when_reopen_is_contended() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-agent-init-timeout-reopen-{}-{}",
            std::process::id(),
            nanos
        ));
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = StateStore::open(root.clone()).await.expect("open store");
            let receipt = RunGraphDispatchReceipt {
                run_id: "run-agent-init-timeout-reopen".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: Some("/tmp/packet.json".to_string()),
                dispatch_result_path: Some("/tmp/result.json".to_string()),
                blocker_code: Some("internal_dispatch_timeout_without_receipt".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-22T00:00:00Z".to_string(),
            };
            let warning = best_effort_record_agent_init_dispatch_timeout_receipt(
                &root,
                &json!({ "run_id": "run-agent-init-timeout-reopen" }),
                &receipt,
                12,
            )
            .await
            .expect("reopen contention should return a deferral warning");
            assert!(
                warning.contains(
                    "authoritative timeout reconciliation deferred until next safe reopen"
                ),
                "expected deferred reconciliation warning, got {warning}"
            );
            store.close().await;
            let _ = fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn agent_init_execute_dispatch_materializes_internal_host_bridge_request() {
        run_on_cli_runtime_stack(
            "agent_init_execute_dispatch_timeout_materializes_internal_timeout_receipt",
            || {
                let runtime =
                    tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
                let harness =
                    TempStateHarness::new().expect("temp state harness should initialize");
                let _cwd = guard_current_dir(harness.path());
                let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");

                assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());
                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "project-activator",
                        "--project-id",
                        "test-project",
                        "--language",
                        "english",
                        "--host-cli-system",
                        "codex",
                        "--json"
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());
                assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
                wait_for_state_unlock(harness.path());

                let config_path = harness.path().join("vida.config.yaml");
                let config = fs::read_to_string(&config_path).expect("config should exist");
                let updated = config.replace(
                    "      execution_class: internal\n",
                    "      execution_class: internal\n      max_runtime_seconds: 1\n",
                );
                let updated = updated.replace(
                    "      max_runtime_seconds: 420\n      verification_gate: targeted_verification\n",
                    "      max_runtime_seconds: 1\n      verification_gate: targeted_verification\n",
                );
                fs::write(&config_path, updated).expect("config should update");

                let fake_bin = harness.path().join("fake-bin");
                fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
                let fake_codex = if cfg!(windows) {
                    let fake_codex = fake_bin.join("codex.cmd");
                    fs::write(
                &fake_codex,
                "@echo off\r\nping -n 12 127.0.0.1 >nul\r\necho {\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"too-late\"}}\r\n",
            )
            .expect("fake codex should write");
                    fake_codex
                } else {
                    let fake_codex = fake_bin.join("codex");
                    fs::write(
                &fake_codex,
                "#!/bin/sh\nsleep 11\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"too-late\"}}'\n",
            )
            .expect("fake codex should write");
                    fake_codex
                };
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&fake_codex)
                        .expect("fake codex metadata should load")
                        .permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&fake_codex, perms)
                        .expect("fake codex should be executable");
                }
                let config = fs::read_to_string(&config_path).expect("config should reload");
                let fake_codex_command = fake_codex.to_string_lossy().replace('\\', "/");
                let updated = if cfg!(windows) {
                    let replacement = format!(
                        "        command: cmd\n        receipt_backed_completion_supported: true\n        windows_sandbox_spawn_supported: true\n        no_output_timeout_seconds: 1\n        static_args:\n          - /C\n          - '{fake_codex_command}'\n"
                    );
                    let with_windows_flag = "        command: codex\n        receipt_backed_completion_supported: true\n        windows_sandbox_spawn_supported: true\n        no_output_timeout_seconds: 2\n        static_args:\n          - exec\n          - --json\n";
                    let without_windows_flag = "        command: codex\n        receipt_backed_completion_supported: true\n        no_output_timeout_seconds: 2\n        static_args:\n          - exec\n          - --json\n";
                    let updated = config.replacen(with_windows_flag, &replacement, 1);
                    if updated == config {
                        config.replacen(without_windows_flag, &replacement, 1)
                    } else {
                        updated
                    }
                } else {
                    config.replacen(
                        "        command: codex\n",
                        &format!("        command: '{fake_codex_command}'\n"),
                        1,
                    )
                };
                fs::write(&config_path, updated).expect("config should point at fake codex");

                let original_path = std::env::var("PATH").ok();
                let mut path_entries = vec![fake_bin.clone()];
                if let Some(original_path) = original_path.as_deref() {
                    path_entries.extend(std::env::split_paths(original_path));
                }
                let patched_path =
                    std::env::join_paths(path_entries).expect("test PATH should join for platform");
                std::env::set_var("PATH", &patched_path);

                let state_root = harness.path().join(".vida").join("data").join("state");
                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should open");
                runtime
                    .block_on(store.create_task(crate::state_store::CreateTaskRequest {
                        task_id: "run-agent-init-timeout-epic",
                        title: "Timeout dispatch fixture epic",
                        display_id: None,
                        description: "Parent epic for the timeout dispatch fixture",
                        issue_type: "epic",
                        status: "open",
                        priority: 1,
                        parent_id: None,
                        labels: &[],
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by: "test",
                        source_repo: "",
                    }))
                    .expect("timeout fixture parent should exist");
                runtime
                    .block_on(store.create_task(crate::state_store::CreateTaskRequest {
                        task_id: "run-agent-init-timeout",
                        title: "Timeout dispatch fixture",
                        display_id: None,
                        description: "TaskFlow task backing the timeout dispatch fixture",
                        issue_type: "defect",
                        status: "open",
                        priority: 1,
                        parent_id: Some("run-agent-init-timeout-epic"),
                        labels: &[],
                        execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                        planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                        created_by: "test",
                        source_repo: "",
                    }))
                    .expect("timeout fixture task should exist");
                let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/init_surfaces.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/init_surfaces.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
                let run_graph_bootstrap = json!({
                    "run_id": "run-agent-init-timeout"
                });
                let status = RunGraphStatus {
                    run_id: "run-agent-init-timeout".to_string(),
                    task_id: "run-agent-init-timeout".to_string(),
                    task_class: "implementation".to_string(),
                    active_node: "planning".to_string(),
                    next_node: Some("worker".to_string()),
                    status: "ready".to_string(),
                    route_task_class: "implementation".to_string(),
                    selected_backend: "junior".to_string(),
                    lane_id: "worker_lane".to_string(),
                    lifecycle_stage: "dispatch_ready".to_string(),
                    policy_gate: "single_task_scope_required".to_string(),
                    handoff_state: "awaiting_worker".to_string(),
                    context_state: "sealed".to_string(),
                    checkpoint_kind: "conversation_cursor".to_string(),
                    resume_target: "dispatch.worker_lane".to_string(),
                    recovery_ready: true,
                };
                runtime
                    .block_on(store.record_run_graph_status(&status))
                    .expect("run graph status should record");
                let receipt = RunGraphDispatchReceipt {
                    run_id: "run-agent-init-timeout".to_string(),
                    dispatch_target: "implementer".to_string(),
                    dispatch_status: "routed".to_string(),
                    lane_status: "lane_running".to_string(),
                    supersedes_receipt_id: None,
                    exception_path_receipt_id: None,
                    dispatch_kind: "agent_lane".to_string(),
                    dispatch_surface: Some("vida agent-init".to_string()),
                    dispatch_command: None,
                    dispatch_packet_path: None,
                    dispatch_result_path: None,
                    blocker_code: None,
                    downstream_dispatch_target: None,
                    downstream_dispatch_command: None,
                    downstream_dispatch_note: None,
                    downstream_dispatch_ready: false,
                    downstream_dispatch_blockers: Vec::new(),
                    downstream_dispatch_packet_path: None,
                    downstream_dispatch_status: None,
                    downstream_dispatch_result_path: None,
                    downstream_dispatch_trace_path: None,
                    downstream_dispatch_executed_count: 0,
                    downstream_dispatch_active_target: None,
                    downstream_dispatch_last_target: None,
                    activation_agent_type: Some("junior".to_string()),
                    activation_runtime_role: Some("worker".to_string()),
                    selected_backend: Some("junior".to_string()),
                    recorded_at: "2026-04-17T00:00:00Z".to_string(),
                };
                let handoff_plan = json!({});
                let ctx = RuntimeDispatchPacketContext::new(
                    &state_root,
                    &role_selection,
                    &receipt,
                    &handoff_plan,
                    &run_graph_bootstrap,
                );
                let dispatch_packet_path =
                    write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
                let mut persisted_receipt = receipt.clone();
                persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
                runtime
                    .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
                    .expect("dispatch receipt should record");
                drop(store);

                assert_eq!(
                    runtime.block_on(run(cli(&[
                        "agent-init",
                        "--dispatch-packet",
                        dispatch_packet_path.as_str(),
                        "--execute-dispatch",
                        "--json",
                    ]))),
                    ExitCode::SUCCESS
                );
                wait_for_state_unlock(harness.path());

                let store = runtime
                    .block_on(StateStore::open(state_root.clone()))
                    .expect("state store should reopen");
                let recorded_receipt = runtime
                    .block_on(store.latest_run_graph_dispatch_receipt())
                    .expect("latest dispatch receipt should load")
                    .expect("latest dispatch receipt should exist");
                assert_eq!(recorded_receipt.dispatch_status, "bridge_request_pending");
                assert_eq!(recorded_receipt.blocker_code.as_deref(), None);
                let recorded_status = runtime
                    .block_on(store.run_graph_status("run-agent-init-timeout"))
                    .expect("run graph status should load after timeout");
                assert_eq!(recorded_status.status, "running");
                assert_eq!(recorded_status.lifecycle_stage, "implementer_active");
                let dispatch_result_path = recorded_receipt
                    .dispatch_result_path
                    .as_deref()
                    .expect("dispatch result path should record");
                let rendered = fs::read_to_string(dispatch_result_path)
                    .expect("dispatch result artifact should load");
                let parsed: serde_json::Value =
                    serde_json::from_str(&rendered).expect("execute-dispatch json should parse");
                assert_eq!(parsed["status"], "blocked");
                assert_eq!(parsed["execution_state"], "bridge_request_pending");
                assert_eq!(parsed["blocker_code"], "host_tool_bridge_adapter_required");
                assert!(parsed["blocker_reason"]
                    .as_str()
                    .expect("blocker reason should render")
                    .contains("parent host-agent bridge"));

                if let Some(original_path) = original_path {
                    std::env::set_var("PATH", original_path);
                } else {
                    std::env::remove_var("PATH");
                }
            },
        );
    }
}

pub(crate) fn ensure_runtime_home(project_root: &Path) -> Result<(), String> {
    for relative in [
        ".vida/config",
        ".vida/db",
        ".vida/cache",
        ".vida/framework",
        ".vida/project",
        ".vida/project/agent-extensions",
        ".vida/receipts",
        ".vida/runtime",
        ".vida/scratchpad",
    ] {
        super::ensure_dir(&project_root.join(relative))?;
    }
    Ok(())
}

fn copy_file_if_missing(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    std::fs::copy(source, target).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

pub(crate) fn copy_tree_replace(source_root: &Path, target_root: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "Expected directory source for tree materialization: {}",
            source_root.display()
        ));
    }
    if target_root.exists() {
        std::fs::remove_dir_all(target_root)
            .map_err(|error| format!("Failed to replace {}: {error}", target_root.display()))?;
    }
    copy_tree_recursive(source_root, target_root)
}

pub(crate) fn copy_tree_recursive(source_root: &Path, target_root: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(source_root)
        .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?;
    if metadata.is_dir() {
        std::fs::create_dir_all(target_root)
            .map_err(|error| format!("Failed to create {}: {error}", target_root.display()))?;
        for entry in std::fs::read_dir(source_root)
            .map_err(|error| format!("Failed to read {}: {error}", source_root.display()))?
        {
            let entry = entry
                .map_err(|error| format!("Failed to iterate {}: {error}", source_root.display()))?;
            copy_tree_recursive(&entry.path(), &target_root.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = target_root.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    std::fs::copy(source_root, target_root).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source_root.display(),
            target_root.display()
        )
    })?;
    Ok(())
}

fn write_file_if_missing(target: &Path, contents: &str) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        super::ensure_dir(parent)?;
    }
    std::fs::write(target, contents)
        .map_err(|error| format!("Failed to write {}: {error}", target.display()))
}

pub(crate) fn default_init_instruction_bundle_source_roots(
    bootstrap_source_root: &Path,
) -> (PathBuf, PathBuf) {
    (
        bootstrap_source_root.join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT),
        bootstrap_source_root.join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT),
    )
}

pub(crate) fn materialize_framework_instruction_bundles(
    project_root: &Path,
    instruction_source_root: &Path,
    framework_memory_source_root: &Path,
) -> Result<(), String> {
    copy_tree_replace(
        instruction_source_root,
        &project_root.join(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT),
    )?;
    copy_tree_replace(
        framework_memory_source_root,
        &project_root.join(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT),
    )?;
    Ok(())
}

pub(crate) fn write_runtime_agent_extension_projections(project_root: &Path) -> Result<(), String> {
    let root = super::project_activator_surface::runtime_agent_extensions_root(project_root);
    super::ensure_dir(&root)?;
    write_file_if_missing(
        &root.join("README.md"),
        super::DEFAULT_RUNTIME_AGENT_EXTENSIONS_README,
    )?;
    write_file_if_missing(
        &root.join("roles.yaml"),
        super::DEFAULT_AGENT_EXTENSION_ROLES_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.yaml"),
        super::DEFAULT_AGENT_EXTENSION_SKILLS_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.yaml"),
        super::DEFAULT_AGENT_EXTENSION_PROFILES_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.yaml"),
        super::DEFAULT_AGENT_EXTENSION_FLOWS_YAML,
    )?;
    write_file_if_missing(
        &root.join("dispatch-aliases.yaml"),
        super::DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_YAML,
    )?;
    write_file_if_missing(
        &root.join("hook-templates.yaml"),
        super::DEFAULT_AGENT_EXTENSION_HOOK_TEMPLATES_YAML,
    )?;
    write_file_if_missing(
        &root.join("roles.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_ROLES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_SKILLS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_PROFILES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_FLOWS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("dispatch-aliases.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("hook-templates.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_HOOK_TEMPLATES_SIDECAR_YAML,
    )?;

    let receipt_path = project_root.join(".vida/receipts/agent-extensions-bootstrap.json");
    if !receipt_path.exists() {
        let generated_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render");
        let receipt = serde_json::json!({
            "receipt_kind": "agent_extensions_bootstrap",
            "generated_at": generated_at,
            "project_root": project_root.display().to_string(),
            "runtime_projection_root": root.display().to_string(),
            "base_projection_files": [
                ".vida/project/agent-extensions/README.md",
                ".vida/project/agent-extensions/roles.yaml",
                ".vida/project/agent-extensions/skills.yaml",
                ".vida/project/agent-extensions/profiles.yaml",
                ".vida/project/agent-extensions/flows.yaml",
                ".vida/project/agent-extensions/dispatch-aliases.yaml",
                ".vida/project/agent-extensions/hook-templates.yaml"
            ],
            "sidecar_projection_files": [
                ".vida/project/agent-extensions/roles.sidecar.yaml",
                ".vida/project/agent-extensions/skills.sidecar.yaml",
                ".vida/project/agent-extensions/profiles.sidecar.yaml",
                ".vida/project/agent-extensions/flows.sidecar.yaml",
                ".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml",
                ".vida/project/agent-extensions/hook-templates.sidecar.yaml"
            ],
            "source": "vida init default runtime projection bootstrap"
        });
        write_file_if_missing(
            &receipt_path,
            &serde_json::to_string_pretty(&receipt)
                .expect("agent extension bootstrap receipt should render"),
        )?;
    }

    Ok(())
}

pub(crate) async fn run_init(args: super::BootArgs) -> ExitCode {
    if let Some(arg) = args.extra_args.first() {
        eprintln!("Unsupported `vida init` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    let project_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let bootstrap_source_root = resolve_init_bootstrap_source_root();
    let (default_instruction_source_root, default_framework_memory_source_root) =
        default_init_instruction_bundle_source_roots(&bootstrap_source_root);
    let instruction_source_root = args
        .instruction_source_root
        .unwrap_or(default_instruction_source_root);
    let framework_memory_source_root = args
        .framework_memory_source_root
        .unwrap_or(default_framework_memory_source_root);
    let framework_agents = match resolve_init_agents_source(&bootstrap_source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let sidecar_scaffold = match resolve_init_sidecar_source(&bootstrap_source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let config_template = match resolve_init_config_template_source(&bootstrap_source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if !framework_agents.is_file() {
        eprintln!(
            "Missing framework bootstrap carrier: {}",
            framework_agents.display()
        );
        return ExitCode::from(1);
    }

    if let Err(error) = materialize_framework_agents_and_sidecar(
        &project_root,
        &framework_agents,
        &sidecar_scaffold,
    )
    .and_then(|()| {
        materialize_framework_instruction_bundles(
            &project_root,
            &instruction_source_root,
            &framework_memory_source_root,
        )
    })
    .and_then(|()| copy_file_if_missing(&config_template, &project_root.join("vida.config.yaml")))
    .and_then(|()| materialize_project_docs_scaffold(&project_root))
    .and_then(|()| ensure_runtime_home(&project_root))
    .and_then(|()| write_runtime_agent_extension_projections(&project_root))
    {
        eprintln!("{error}");
        return ExitCode::from(1);
    }

    let activation_view =
        super::project_activator_surface::build_project_activator_view(&project_root);
    print_init_summary(&project_root, &activation_view);
    ExitCode::SUCCESS
}

pub(crate) fn materialize_project_docs_scaffold(project_root: &Path) -> Result<(), String> {
    let project_id = super::project_activator_surface::inferred_project_id_candidate(project_root);
    let project_title = super::inferred_project_title(&project_id, None);
    let source_root = resolve_init_bootstrap_source_root();
    let feature_template_source = resolve_feature_design_template_source(&source_root)?;
    let feature_template = std::fs::read_to_string(&feature_template_source).map_err(|error| {
        format!(
            "Failed to read framework feature-design template source {}: {error}",
            feature_template_source.display()
        )
    })?;

    let generated_files = vec![
        (
            project_root.join("README.md"),
            render_project_readme(&project_title),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ROOT_MAP),
            render_project_root_map(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PRODUCT_INDEX),
            render_project_product_index(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PRODUCT_SPEC_README),
            render_project_product_spec_readme(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE),
            feature_template,
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ARCHITECTURE_DOC),
            render_project_architecture_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PROCESS_README),
            render_project_process_readme(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_DECISIONS_DOC),
            with_scaffold_footer(
                "# Decisions\n\nRecord bounded architecture and product decisions here.\n",
                "process/decisions",
                "process_doc",
                "docs/process/decisions.md",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ENVIRONMENTS_DOC),
            render_project_environments_doc(project_root),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_OPERATIONS_DOC),
            render_project_operations_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_AGENT_SYSTEM_DOC),
            render_project_agent_system_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_DOC_TOOLING_DOC),
            render_project_doc_tooling_map(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ORCHESTRATOR_STARTUP_BUNDLE),
            render_project_runtime_projection_doc(
                "Project Orchestrator Startup Bundle",
                "process/project-orchestrator-startup-bundle",
                super::DEFAULT_PROJECT_ORCHESTRATOR_STARTUP_BUNDLE,
                "Compact orchestrator startup bundle scaffold used by `vida orchestrator-init`.",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PACKET_AND_LANE_RUNTIME_CAPSULE),
            render_project_runtime_projection_doc(
                "Project Packet And Lane Runtime Capsule",
                "process/project-packet-and-lane-runtime-capsule",
                super::DEFAULT_PROJECT_PACKET_AND_LANE_RUNTIME_CAPSULE,
                "Compact packet and lane runtime capsule scaffold used by launcher startup projection.",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_START_READINESS_RUNTIME_CAPSULE),
            render_project_runtime_projection_doc(
                "Project Start Readiness Runtime Capsule",
                "process/project-start-readiness-runtime-capsule",
                super::DEFAULT_PROJECT_START_READINESS_RUNTIME_CAPSULE,
                "Compact project start-readiness runtime capsule scaffold used by launcher startup projection.",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PACKET_RENDERING_RUNTIME_CAPSULE),
            render_project_runtime_projection_doc(
                "Project Packet Rendering Runtime Capsule",
                "process/project-packet-rendering-runtime-capsule",
                super::DEFAULT_PROJECT_PACKET_RENDERING_RUNTIME_CAPSULE,
                "Compact packet rendering runtime capsule scaffold used by launcher startup projection.",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC),
            render_project_host_agent_guide(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_RESEARCH_README),
            render_project_research_readme(),
        ),
    ];

    for (path, content) in generated_files {
        write_file_if_missing(&path, &content)?;
        if let Ok(relative_source_path) = path.strip_prefix(project_root) {
            write_scaffold_changelog_if_missing(
                &path,
                relative_source_path,
                scaffold_artifact_path_for(relative_source_path),
                scaffold_artifact_type_for(relative_source_path),
            )?;
        }
    }

    Ok(())
}

pub(crate) fn render_project_readme(project_title: &str) -> String {
    with_scaffold_footer(
        &format!(
            "# {project_title}\n\n\
This repository contains a VIDA-initialized project scaffold.\n\n\
Use `AGENTS.md` for framework bootstrap, `AGENTS.sidecar.md` for project agent instructions and docs routing, and `docs/` for project-owned operating context.\n"
        ),
        "project/readme",
        "document",
        "README.md",
    )
}

pub(crate) fn render_project_root_map() -> String {
    with_scaffold_footer(
        &format!(
            "# Project Root Map\n\n\
This project uses the following canonical documentation roots:\n\n\
- `docs/product/` for product-facing intent and architecture notes\n\
- `docs/process/` for project operations and working agreements\n\
- `docs/research/` for research notes and discovery artifacts\n\n\
Primary pointers:\n\n\
- Product index: `{}`\n\
- Product spec/readiness guide: `{}`\n\
- Feature design template: `{}`\n\
- Process index: `{}`\n\
- Documentation tooling: `{}`\n\
- Host agent guide: `{}`\n\
- Research index: `{}`\n\
- Repository overview: `README.md`\n",
            super::DEFAULT_PROJECT_PRODUCT_INDEX,
            super::DEFAULT_PROJECT_PRODUCT_SPEC_README,
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            super::DEFAULT_PROJECT_PROCESS_README,
            super::DEFAULT_PROJECT_DOC_TOOLING_DOC,
            super::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC,
            super::DEFAULT_PROJECT_RESEARCH_README
        ),
        "project/root-map",
        "document",
        "docs/project-root-map.md",
    )
}

pub(crate) fn render_project_product_index() -> String {
    with_scaffold_footer(
        &format!(
            "# Product Index\n\n\
Product documentation currently contains:\n\n\
- `{}` for the initial project architecture outline\n\
- `{}` for bounded feature/change design and ADR routing\n",
            super::DEFAULT_PROJECT_ARCHITECTURE_DOC,
            super::DEFAULT_PROJECT_PRODUCT_SPEC_README
        ),
        "product/index",
        "product_index",
        "docs/product/index.md",
    )
}

pub(crate) fn render_project_product_spec_readme() -> String {
    with_scaffold_footer(
        &format!(
            "# Product Spec Guide\n\n\
Use this directory for bounded product-facing feature/change design documents and linked ADRs.\n\n\
Default rule:\n\n\
1. If a request asks for research, detailed specifications, implementation planning, and then code, create or update one bounded design document before implementation.\n\
2. Start from the local template at `{}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before normal implementation work begins.\n\
4. Use `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check` to keep the document canonical.\n\
5. Close the spec-pack task only after the design artifact is finalized and validated, then hand off through the next TaskFlow packet.\n\
6. When one major decision needs durable standalone recording, add a linked ADR instead of overloading the design document.\n\
\n\
Suggested homes:\n\n\
- `docs/product/spec/<feature>-design.md` for committed feature/change designs\n\
- `docs/research/<topic>.md` for exploratory research before design closure\n",
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "product/spec/readme",
        "product_spec",
        "docs/product/spec/README.md",
    )
}

pub(crate) fn render_project_architecture_doc() -> String {
    with_scaffold_footer(
        "# Architecture\n\nCurrent project posture:\n\n- VIDA bootstrap scaffold is initialized\n- project documentation roots are materialized\n- project-specific implementation modules are not yet defined\n",
        "product/architecture",
        "document",
        "docs/product/architecture.md",
    )
}

pub(crate) fn render_project_process_readme() -> String {
    with_scaffold_footer(
        "# Process Docs\n\nThis directory contains the minimum process documentation expected by VIDA activation.\n\nAvailable process docs:\n\n- `decisions.md`\n- `environments.md`\n- `project-operations.md`\n- `agent-system.md`\n- `documentation-tooling-map.md`\n- `codex-agent-configuration-guide.md` (current host-agent guide filename)\n",
        "process/readme",
        "process_doc",
        "docs/process/README.md",
    )
}

pub(crate) fn render_project_decisions_doc(answers: &super::ProjectActivationAnswers) -> String {
    with_scaffold_footer(
        &format!(
            "# Decisions\n\n\
Initial activation decisions:\n\n\
- project id: `{}`\n\
- host CLI system: selected through `vida project-activator`\n\
- language policy:\n  - user communication: `{}`\n  - reasoning: `{}`\n  - documentation: `{}`\n  - todo protocol: `{}`\n",
            answers.project_id,
            answers.user_communication_language,
            answers.reasoning_language,
            answers.documentation_language,
            answers.todo_protocol_language
        ),
        "process/decisions",
        "process_doc",
        "docs/process/decisions.md",
    )
}

pub(crate) fn render_project_environments_doc(project_root: &Path) -> String {
    with_scaffold_footer(
        &format!(
            "# Environments\n\n\
Initial environment assumptions:\n\n\
- local project root: `{}`\n\
- VIDA runtime directories are managed under `.vida/`\n\
- host CLI agent template is selected through `vida project-activator`\n",
            project_root.display()
        ),
        "process/environments",
        "process_doc",
        "docs/process/environments.md",
    )
}

pub(crate) fn render_project_operations_doc() -> String {
    with_scaffold_footer(
        &format!(
            "# Project Operations\n\n\
Current operating baseline:\n\n\
- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces\n\
- use `AGENTS.sidecar.md` as the project agent-instructions overlay and project documentation map\n\
- while project activation is pending, do not enter TaskFlow execution; use `vida project-activator` and `vida docflow`\n\
\n\
Default feature-delivery flow:\n\n\
1. If the request asks for research, specifications, a plan, and then implementation, start with a bounded design document.\n\
2. Use the local template at `{}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before code execution.\n\
4. Keep the design artifact canonical through `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check`.\n\
5. Close the spec-pack task and shape the next work-pool/dev packet in `vida taskflow` after the design document names the bounded file set, proof targets, and rollout.\n\
6. When the selected host runtime surface is materialized, use the delegated host team surface instead of collapsing the root session directly into coding.\n\
7. Treat `vida.config.yaml` as the owner of carrier tiers, host-system inventory, and any optional internal aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.\n\
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.\n\
9. For normal write-producing work, treat project agent-first execution as the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the canonical project control surface.\n\
10. Keep the root session in orchestration posture unless an explicit exception path is recorded.\n\
11. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; if the root-session write guard is still active, continue through packet shaping or `vida agent-init` dispatch instead of local coding.\n\
12. Host-local shell/edit capability is not a lane-change receipt and does not authorize root-session coding.\n\
13. If the user explicitly orders agent-first or parallel-agent execution, keep that routing intent sticky; do not silently substitute root-session coding.\n\
14. Finding the patch location, reproducing a runtime defect, hitting a worker timeout, or tripping a thread-limit/`not_found` lane failure does not authorize root-session coding; recover delegated lanes, wait, reroute, or record the exception path first.\n\
15. If delegated execution returns only an activation view without execution evidence and a bounded read-only diagnostic path still exists, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.\n\
16. Saturation recovery means: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.\n\
17. Under continued-development intent, stay in commentary/progress mode until the user explicitly asks to stop; do not emit final closure wording while a next lawful TaskFlow continuation item is already known.\n\
18. Do not treat commentary, status output, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\n\
19. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.\n\
20. After any bounded result, green test, successful build, runtime handoff, or delegated handoff, immediately bind the next lawful continuation item in the same cycle instead of pausing at a summary.\n\
21. Sticky continuation intent is not permission to self-select `ready_head[0]`, the first ready backlog item, or any adjacent slice; fail closed unless the active bounded unit is explicit from user wording or runtime evidence.\n\
22. If continued-development intent is active but `vida status --json` or `vida orchestrator-init --json` cannot state `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, publish an ambiguity report instead of continuing implementation.\n\
23. When recording progress into the backlog from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\n",
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "process/project-operations",
        "process_doc",
        "docs/process/project-operations.md",
    )
}

pub(crate) fn render_project_agent_system_doc() -> String {
    with_scaffold_footer(
        "# Agent System\n\nProject activation owns host CLI agent-template selection and runtime admission.\n\n- default framework host templates become available only after the selected host CLI template is materialized\n- supported and active host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`\n- framework template inventory may be broader than the enabled active list in project config\n- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers`; compatibility projections such as `host_environment.codex.agents` may exist but must not become a second canonical source\n- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model\n- the selected runtime surface is rendered under the configured runtime root and is not the owner of tier/rate/task-class policy\n- project activation materializes the selected host template using the configured `materialization_mode`; the current internal Codex adapter renders the configured TOML catalog root, while external CLI systems use their configured runtime roots\n- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`\n- project-local agent extensions remain under `.vida/project/agent-extensions/`\n- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists\n- project \"agent-first\" development means the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional carrier mechanics and not the canonical execution contract\n- host-local shell/edit capability is an executor affordance only and must not be interpreted as lawful root-session write ownership\n- when the selected host execution class is internal, optional external CLI subagents remain auxiliary carrier details and do not make the whole session externally gated by default\n- patch localization, runtime-defect diagnosis, or other read-only findings feed the next delegated packet and do not transfer write ownership back to the root session\n",
        "process/agent-system",
        "process_doc",
        "docs/process/agent-system.md",
    )
}

pub(crate) fn render_project_doc_tooling_map() -> String {
    with_scaffold_footer(
        &format!(
            "# Documentation Tooling Map\n\n\
Use `vida docflow` for documentation inventory, mutation, validation, and readiness checks.\n\n\
Design-document rule:\n\n\
1. For bounded feature/change work that requires research, detailed specifications, planning, and implementation, begin with one design document before code execution.\n\
2. Start from `{}`.\n\
3. Open one epic and one spec-pack task in `vida taskflow` before writing code.\n\
4. Suggested command sequence:\n\
   - `vida docflow init docs/product/spec/<feature>-design.md product/spec/<feature>-design product_spec \"initialize feature design\"`\n\
   - edit the document using the local template shape\n\
   - `vida docflow finalize-edit docs/product/spec/<feature>-design.md \"record bounded feature design\"`\n\
   - `vida docflow check --root . docs/product/spec/<feature>-design.md`\n\
   - `vida task close <spec-task-id> --reason \"design packet finalized and handed off\" --json`\n\
\n\
Activation rule:\n\n\
1. During project activation, `vida project-activator` owns bounded config/doc materialization.\n\
2. `vida taskflow` and any non-canonical external TaskFlow runtime are not lawful activation-entry surfaces while activation is pending.\n\
3. After activation writes, prefer `vida docflow` for documentation-oriented inspection and proof before multi-step implementation.\n",
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "process/documentation-tooling-map",
        "process_doc",
        "docs/process/documentation-tooling-map.md",
    )
}

pub(crate) fn render_project_runtime_projection_doc(
    title: &str,
    artifact_path: &str,
    source_path: &str,
    purpose: &str,
) -> String {
    with_scaffold_footer(
        &format!(
            "# {title}\n\n\
Purpose: {purpose}\n\n\
Runtime projection status:\n\n\
- registered: true\n\
- mapped: true\n\
- bound: true\n\
- compiled: true\n\
- validated: true\n\
- executable: true\n\n\
This scaffold gives `vida init` a ready-enough project runtime projection. Projects may replace this body with richer local protocol text while preserving canonical footer metadata.\n"
        ),
        artifact_path,
        "process_doc",
        source_path,
    )
}

pub(crate) fn render_project_research_readme() -> String {
    with_scaffold_footer(
        "# Research Notes\n\nUse this directory for research artifacts, discovery notes, and external references that support future project work.\n",
        "research/readme",
        "document",
        "docs/research/README.md",
    )
}

pub(crate) fn render_project_host_agent_guide() -> String {
    with_scaffold_footer(
        "# Host Agent Configuration Guide\n\nThis project uses framework-materialized host runtime surfaces; the active internal Codex surface currently renders under `.codex/**`.\n\nSource-of-truth rule:\n\n- `vida.config.yaml -> host_environment.systems.<system>.carriers` owns carrier-tier metadata, rates, runtime-role fit, and task-class fit\n- `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` owns the dispatch-alias registry for executor-local overlays\n- `.codex/**` is the rendered executor surface used by the current internal Codex adapter after activation\n- `.codex/config.toml` should expose the carrier tiers materialized from the selected host-system carrier catalog\n\nCarrier rule:\n\n- the primary visible agent model is the configured carrier catalog rendered from `vida.config.yaml`, not a Rust-hardcoded role list\n- compatibility projections such as `host_environment.codex.agents` may exist for older consumers but must not be treated as a second canonical source\n- runtime role remains explicit activation state such as `worker`, `coach`, `verifier`, or `solution_architect`\n- internal alias ids may exist in registry state, but they must not replace the carrier-tier model at the project surface\n\nWorking rule:\n\n1. The root session stays the orchestrator.\n2. Documentation/specification work should complete the bounded design document first.\n3. Before delegated implementation starts, open the feature epic/spec task in `vida taskflow` and close the spec task only after the design artifact is finalized.\n4. After a bounded packet exists, route research, specification, planning, implementation, review, and verification through the configured carrier catalog instead of collapsing into root-session coding.\n5. Let runtime choose the cheapest capable configured carrier tier with a healthy local score from `.vida/state/worker-strategy.json` and pass the lawful runtime role explicitly.\n6. Canonical delegated execution still dispatches through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the primary project delegation surface.\n7. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; an active root-session write guard still means orchestration-only.\n8. If the user explicitly orders agent-first or parallel-agent execution, keep that routing sticky; do not silently substitute root-session coding because a host tool offers local write access.\n9. Finding the patch location, reproducing a runtime defect, hitting a worker timeout, or tripping a thread-limit/`not_found` lane failure is not a lane-change receipt and does not authorize root-session coding.\n10. Recover delegated-lane saturation first: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.\n11. Under continued-development intent, stay in commentary/progress mode and continue routing; do not emit final closure wording while a next lawful continuation item is already known.\n12. Do not treat commentary, status output, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\n13. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.\n14. After any bounded result, successful build, runtime handoff, or delegated handoff, immediately bind the next lawful continuation item in the same cycle instead of pausing at a summary.\n15. Sticky continuation intent does not authorize choosing the first ready task or an adjacent slice by plausibility; continue only when the active bounded unit is explicit from user wording or runtime evidence.\n16. If `vida status --json` or `vida orchestrator-init --json` does not expose explicit `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, fail closed to an ambiguity report instead of continuing implementation.\n17. When recording task progress from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\n18. Use `.vida/project/agent-extensions/**` for project-local role and skill overlays; do not treat `.codex/**` as the owner of framework or product law.\n",
        "process/codex-agent-configuration-guide",
        "process_doc",
        "docs/process/codex-agent-configuration-guide.md",
    )
}

fn with_scaffold_footer(
    body: &str,
    artifact_path: &str,
    artifact_type: &str,
    source_path: &str,
) -> String {
    let changelog_ref = scaffold_changelog_ref_for(source_path);
    format!(
        "{body}\n-----\nartifact_path: {artifact_path}\nartifact_type: {artifact_type}\nartifact_version: '1'\nartifact_revision: '2026-04-04'\nschema_version: '1'\nstatus: scaffold\nsource_path: {source_path}\ncreated_at: '2026-04-04T00:00:00Z'\nupdated_at: '2026-04-04T00:00:00Z'\nchangelog_ref: {changelog_ref}\n"
    )
}

fn scaffold_changelog_ref_for(source_path: &str) -> String {
    let source_path = Path::new(source_path);
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    format!("{stem}.changelog.jsonl")
}

fn scaffold_artifact_path_for(relative_source_path: &Path) -> &'static str {
    match relative_source_path.to_string_lossy().as_ref() {
        "README.md" => "project/readme",
        "docs/project-root-map.md" => "project/root-map",
        "docs/product/index.md" => "product/index",
        "docs/product/architecture.md" => "product/architecture",
        "docs/product/spec/README.md" => "product/spec/readme",
        "docs/product/spec/templates/feature-design-document.template.md" => {
            "product/spec/templates/feature-design-document.template"
        }
        "docs/process/README.md" => "process/readme",
        "docs/process/agent-system.md" => "process/agent-system",
        "docs/process/codex-agent-configuration-guide.md" => {
            "process/codex-agent-configuration-guide"
        }
        "docs/process/decisions.md" => "process/decisions",
        "docs/process/documentation-tooling-map.md" => "process/documentation-tooling-map",
        "docs/process/environments.md" => "process/environments",
        "docs/process/project-orchestrator-startup-bundle.md" => {
            "process/project-orchestrator-startup-bundle"
        }
        "docs/process/project-packet-and-lane-runtime-capsule.md" => {
            "process/project-packet-and-lane-runtime-capsule"
        }
        "docs/process/project-start-readiness-runtime-capsule.md" => {
            "process/project-start-readiness-runtime-capsule"
        }
        "docs/process/project-packet-rendering-runtime-capsule.md" => {
            "process/project-packet-rendering-runtime-capsule"
        }
        "docs/process/project-operations.md" => "process/project-operations",
        "docs/research/README.md" => "research/readme",
        _ => "project/scaffold-doc",
    }
}

fn scaffold_artifact_type_for(relative_source_path: &Path) -> &'static str {
    match relative_source_path.to_string_lossy().as_ref() {
        "docs/process/README.md"
        | "docs/process/agent-system.md"
        | "docs/process/codex-agent-configuration-guide.md"
        | "docs/process/decisions.md"
        | "docs/process/documentation-tooling-map.md"
        | "docs/process/environments.md"
        | "docs/process/project-orchestrator-startup-bundle.md"
        | "docs/process/project-packet-and-lane-runtime-capsule.md"
        | "docs/process/project-start-readiness-runtime-capsule.md"
        | "docs/process/project-packet-rendering-runtime-capsule.md"
        | "docs/process/project-operations.md" => "process_doc",
        "docs/product/index.md" => "product_index",
        "docs/product/spec/README.md"
        | "docs/product/spec/templates/feature-design-document.template.md" => "product_spec",
        _ => "document",
    }
}

fn write_scaffold_changelog_if_missing(
    absolute_source_path: &Path,
    relative_source_path: &Path,
    artifact_path: &str,
    artifact_type: &str,
) -> Result<(), String> {
    let parent = absolute_source_path.parent().ok_or_else(|| {
        format!(
            "Failed to determine scaffold parent directory for {}",
            absolute_source_path.display()
        )
    })?;
    let stem = absolute_source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    let changelog_path = parent.join(format!("{stem}.changelog.jsonl"));
    let entry = format!(
        "{{\"ts\":\"2026-04-04T00:00:00Z\",\"event\":\"metadata_initialized\",\"artifact_path\":\"{artifact_path}\",\"artifact_type\":\"{artifact_type}\",\"artifact_version\":\"1\",\"artifact_revision\":\"2026-04-04\",\"source_path\":\"{}\",\"reason\":\"initialize scaffold metadata for docflow-ready project bootstrap\",\"actor\":\"vida\",\"scope\":\"scaffold-init\",\"tags\":[\"scaffold\",\"docflow\"]}}\n",
        relative_source_path.display()
    );
    write_file_if_missing(&changelog_path, &entry)
}

fn existing_agents_looks_like_framework_bootstrap(contents: &str) -> bool {
    let lower = contents.to_ascii_lowercase();
    lower.contains("vida project bootstrap carrier")
        || lower.contains("generated downstream bootstrap carrier")
        || lower.contains("framework bootstrap carrier")
        || lower.contains("vida orchestrator-init")
        || lower.contains("vida agent-init")
        || lower.contains("root-session write guard")
}

fn strip_markdown_footer(contents: &str) -> &str {
    contents
        .split_once("\n-----\n")
        .map(|(body, _)| body.trim_end())
        .unwrap_or_else(|| contents.trim_end())
}

fn render_migrated_project_sidecar(scaffold: &str, existing_agents: &str) -> String {
    let migrated = strip_markdown_footer(existing_agents);
    let section = format!(
        "\n## Migrated Project Instructions\n\n\
The following project-local instructions were migrated from the pre-init root `AGENTS.md`.\n\
Treat them as active project instructions unless they conflict with the generated VIDA bootstrap carrier or runtime law.\n\n\
<migrated_project_instructions>\n{migrated}\n</migrated_project_instructions>\n"
    );
    if let Some((body, footer)) = scaffold.split_once("\n-----\n") {
        format!("{}{}\n-----\n{}", body.trim_end(), section, footer)
    } else {
        format!("{}{}\n", scaffold.trim_end(), section)
    }
}

fn write_legacy_agents_snapshot(
    project_root: &Path,
    file_name: &str,
    contents: &str,
    note: &str,
) -> Result<(), String> {
    let backup_path = project_root.join(".vida/receipts").join(file_name);
    if let Some(parent) = backup_path.parent() {
        super::ensure_dir(parent)?;
    }
    if !backup_path.exists() {
        let body = format!(
            "# Legacy AGENTS Snapshot\n\n\
Status: archived legacy snapshot, not active authority.\n\n\
{note}\n\n\
----- legacy content -----\n{contents}"
        );
        std::fs::write(&backup_path, body).map_err(|error| {
            format!("Failed to write {} backup: {error}", backup_path.display())
        })?;
    }
    Ok(())
}

fn materialize_framework_agents_and_sidecar(
    project_root: &Path,
    framework_agents: &Path,
    _sidecar_scaffold: &Path,
) -> Result<(), String> {
    let agents = project_root.join("AGENTS.md");
    let sidecar = project_root.join("AGENTS.sidecar.md");
    let framework_contents = std::fs::read_to_string(framework_agents)
        .map_err(|error| format!("Failed to read {}: {error}", framework_agents.display()))?;
    let project_id = super::project_activator_surface::inferred_project_id_candidate(project_root);
    let project_title = super::inferred_project_title(&project_id, None);
    let default_sidecar = super::project_activator_surface::render_project_sidecar(&project_title);

    if agents.is_file() {
        let existing_agents = std::fs::read_to_string(&agents)
            .map_err(|error| format!("Failed to read {}: {error}", agents.display()))?;
        if existing_agents != framework_contents {
            if !sidecar.is_file()
                || super::project_activator_surface::file_contains_placeholder(&sidecar)
            {
                if existing_agents_looks_like_framework_bootstrap(&existing_agents) {
                    write_legacy_agents_snapshot(
                        project_root,
                        "AGENTS.legacy.md",
                        &existing_agents,
                        "The pre-init root AGENTS.md looked like a legacy framework/bootstrap carrier, so init archived it instead of making it active sidecar law.",
                    )?;
                } else {
                    if let Some(parent) = sidecar.parent() {
                        super::ensure_dir(parent)?;
                    }
                    let migrated_sidecar =
                        render_migrated_project_sidecar(&default_sidecar, &existing_agents);
                    std::fs::write(&sidecar, migrated_sidecar).map_err(|error| {
                        format!(
                            "Failed to preserve existing {} as migrated {}: {error}",
                            agents.display(),
                            sidecar.display()
                        )
                    })?;
                }
            } else {
                write_legacy_agents_snapshot(
                    project_root,
                    "AGENTS.pre-init.backup.md",
                    &existing_agents,
                    "An existing non-placeholder AGENTS.sidecar.md was preserved; the pre-init root AGENTS.md is archived for manual review.",
                )?;
            }
        }
    }

    if !sidecar.is_file() || super::project_activator_surface::file_contains_placeholder(&sidecar) {
        if let Some(parent) = sidecar.parent() {
            super::ensure_dir(parent)?;
        }
        std::fs::write(&sidecar, &default_sidecar)
            .map_err(|error| format!("Failed to write {}: {error}", sidecar.display()))?;
    }
    std::fs::write(&agents, framework_contents)
        .map_err(|error| format!("Failed to write {}: {error}", agents.display()))
}

fn print_init_summary(project_root: &Path, activation_view: &serde_json::Value) {
    println!("vida init project bootstrap ready");
    println!("project root: {}", project_root.display());
    println!(
        "materialized: AGENTS.md, AGENTS.sidecar.md, vida.config.yaml, vida/config/instructions/bundles/**, README.md, docs/project-root-map.md, docs/product/**, docs/process/**, docs/research/README.md, .vida/config, .vida/db, .vida/cache, .vida/framework, .vida/project, .vida/project/agent-extensions/*, .vida/project/agent-extensions/*.sidecar.yaml, .vida/receipts, .vida/runtime, .vida/scratchpad"
    );
    println!(
        "activation status: {}",
        activation_view["status"].as_str().unwrap_or("unknown")
    );
    if let Ok(launcher) = super::doctor_launcher_summary_for_root(project_root) {
        println!("launcher status: {}", launcher.status);
        if let Some(layout) = launcher.install_layout.as_ref() {
            println!("install root: {}", layout.install_root);
            println!("runtime bin: {}", layout.runtime_bin_dir);
        }
        if launcher.path_resolution.status == "warn" {
            println!(
                "path resolution: command `{}` is not resolving to this active runtime in the current shell",
                launcher.path_resolution.command
            );
            for action in launcher.next_actions {
                println!("launcher next step: {action}");
            }
        }
    }
    if activation_view["activation_pending"]
        .as_bool()
        .unwrap_or(true)
    {
        println!("next step: vida project-activator --json");
        if let Some(example) = activation_view["interview"]["one_shot_example"].as_str() {
            println!("activation example: {example}");
        }
        if let Some(step) = activation_view["next_steps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .next()
        {
            println!("activation note: {step}");
        }
        println!(
            "activation rule: while activation is pending, use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime"
        );
    }
}

pub(crate) async fn run_boot(args: BootArgs) -> ExitCode {
    if let Some(arg) = args.extra_args.first() {
        eprintln!("Unsupported `vida boot` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    let render = args.render;
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = args
        .instruction_source_root
        .unwrap_or_else(|| PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT));
    let framework_memory_source_root = args
        .framework_memory_source_root
        .unwrap_or_else(|| PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT));

    match tokio::time::timeout(
        std::time::Duration::from_secs(COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS),
        StateStore::open(state_dir.clone()),
    )
    .await
    {
        Ok(Ok(store)) => {
            let state_root = store.root().to_path_buf();
            let exit_code = async {
                match store.seed_framework_instruction_bundle().await {
                    Ok(()) => match store.backend_summary().await {
                        Ok(summary) => match store.source_tree_summary().await {
                            Ok(source_tree) => match store
                                .ingest_instruction_source_tree(&normalize_root_arg(
                                    &instruction_source_root,
                                ))
                                .await
                            {
                                Ok(ingest) => {
                                    print_surface_header(render, "vida boot scaffold ready");
                                    print_surface_line(render, "authoritative state store", &summary);
                                    match store.state_spine_summary().await {
                                        Ok(state_spine) => print_surface_line(
                                            render,
                                            "authoritative state spine",
                                            &format!(
                                                "initialized (state-v{}, {} entity surfaces, mutation root {})",
                                                state_spine.state_schema_version,
                                                state_spine.entity_surface_count,
                                                state_spine.authoritative_mutation_root
                                            ),
                                        ),
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to read authoritative state spine summary: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                    print_surface_line(
                                        render,
                                        "framework instruction bundle",
                                        "seeded",
                                    );
                                    print_surface_line(
                                        render,
                                        "instruction source tree",
                                        &source_tree,
                                    );
                                    print_surface_line(
                                        render,
                                        "instruction ingest",
                                        &ingest.as_display(),
                                    );
                                    match store.evaluate_boot_compatibility().await {
                                        Ok(compatibility) => {
                                            print_surface_line(
                                                render,
                                                "boot compatibility",
                                                &format!(
                                                    "{} ({})",
                                                    compatibility.classification,
                                                    compatibility.next_step
                                                ),
                                            );
                                            if crate::release1_contracts::canonical_compatibility_class_str(
                                                &compatibility.classification,
                                            ) != Some(
                                                crate::release1_contracts::CompatibilityClass::BackwardCompatible
                                                    .as_str(),
                                            ) {
                                                eprintln!(
                                                    "Boot compatibility check failed: {}",
                                                    compatibility.reasons.join(", ")
                                                );
                                                return ExitCode::from(1);
                                            }
                                        }
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to evaluate boot compatibility: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                    match store.evaluate_migration_preflight().await {
                                        Ok(migration) => {
                                            print_surface_line(
                                                render,
                                                "migration preflight",
                                                &format!(
                                                    "{} / {} ({})",
                                                    migration.compatibility_classification,
                                                    migration.migration_state,
                                                    migration.next_step
                                                ),
                                            );
                                            if !migration.blockers.is_empty() {
                                                eprintln!(
                                                    "Migration preflight failed: {}",
                                                    migration.blockers.join(", ")
                                                );
                                                return ExitCode::from(1);
                                            }
                                        }
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to evaluate migration preflight: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                    match store.migration_receipt_summary().await {
                                        Ok(summary) => {
                                            print_surface_line(
                                                render,
                                                "migration receipts",
                                                &summary.as_display(),
                                            );
                                        }
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to read migration receipt summary: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                    match store.active_instruction_root().await {
                                        Ok(root_artifact_id) => match store
                                            .resolve_effective_instruction_bundle(&root_artifact_id)
                                            .await
                                        {
                                            Ok(bundle) => {
                                                print_surface_line(
                                                    render,
                                                    "effective instruction bundle",
                                                    &bundle.mandatory_chain_order.join(" -> "),
                                                );
                                                print_surface_line(
                                                    render,
                                                    "effective instruction bundle receipt",
                                                    &bundle.receipt_id,
                                                );
                                            }
                                            Err(error) => {
                                                eprintln!(
                                                    "Failed to resolve effective instruction bundle: {error}"
                                                );
                                                return ExitCode::from(1);
                                            }
                                        },
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to read active instruction root: {error}"
                                            );
                                            return ExitCode::from(1);
                                        }
                                    }
                                    match store
                                        .ingest_framework_memory_source_tree(&normalize_root_arg(
                                            &framework_memory_source_root,
                                        ))
                                        .await
                                    {
                                        Ok(framework_ingest) => {
                                            if let Err(error) =
                                                sync_launcher_activation_snapshot(&store).await
                                            {
                                                eprintln!(
                                                    "Failed to persist launcher activation snapshot: {error}"
                                                );
                                                return ExitCode::from(1);
                                            }
                                            print_surface_line(
                                                render,
                                                "framework memory ingest",
                                                &framework_ingest.as_display(),
                                            );
                                            print_surface_line(
                                                render,
                                                "state dir",
                                                &store.root().display().to_string(),
                                            );
                                            ExitCode::SUCCESS
                                        }
                                        Err(error) => {
                                            eprintln!(
                                                "Failed to ingest framework memory source tree: {error}"
                                            );
                                            ExitCode::from(1)
                                        }
                                    }
                                }
                                Err(error) => {
                                    eprintln!("Failed to ingest instruction source tree: {error}");
                                    ExitCode::from(1)
                                }
                            },
                            Err(error) => {
                                eprintln!("Failed to read source tree metadata: {error}");
                                ExitCode::from(1)
                            }
                        },
                        Err(error) => {
                            eprintln!("Failed to read storage metadata: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to seed framework instruction bundle: {error}");
                        ExitCode::from(1)
                    }
                }
            }
            .await;
            store.close().await;
            if exit_code == ExitCode::SUCCESS {
                if let Err(error) =
                    verify_authoritative_state_store_released_after_boot(state_root).await
                {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            }
            exit_code
        }
        Ok(Err(error)) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!(
                "Timed out opening authoritative state store for `vida boot` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s (cold authoritative state open timeout)"
            );
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_orchestrator_init(args: InitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    let framework_memory_source_root =
        PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT);

    if args.json {
        let projection_name = orchestrator_init_projection_name(args.full);
        if let Some(cached) = crate::operator_projection_cache::read_fresh_json_projection(
            &state_dir,
            projection_name,
        ) {
            let rendered = if let Some(overlay) =
                crate::operator_projection_cache::read_runtime_continuation_binding_overlay_newer_than_projection(
                    &state_dir,
                    projection_name,
                )
            {
                crate::operator_projection_cache::apply_runtime_continuation_binding_overlay_to_fresh_payload(
                    &state_dir,
                    &cached,
                    &overlay,
                )
                .unwrap_or_else(|| cached.clone())
            } else {
                cached.clone()
            };
            if cached_orchestrator_init_payload_has_top_level_continuation_fields(&rendered) {
                println!("{rendered}");
                return ExitCode::SUCCESS;
            }
        }
        if let Some(cached) =
            crate::operator_projection_cache::read_state_stale_recent_json_projection(
                &state_dir,
                orchestrator_init_projection_name(args.full),
                std::time::Duration::from_secs(300),
            )
        {
            if let Some(overlay) =
                crate::operator_projection_cache::read_runtime_continuation_binding_overlay(
                    &state_dir,
                )
            {
                if let Some(rendered) =
                    crate::operator_projection_cache::apply_runtime_continuation_binding_overlay_to_payload(
                        &state_dir,
                        &cached,
                        &overlay,
                    )
                {
                    println!("{rendered}");
                    return ExitCode::SUCCESS;
                }
            }
        }
    }

    // Security: orchestrator-init JSON may use only state-marker-fresh projections.
    // Stale projections still fail closed to authoritative recomputation.

    match tokio::time::timeout(
        std::time::Duration::from_secs(COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS),
        async {
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => Ok(store),
                Err(crate::state_store::StateStoreError::MissingStateDir(_)) => {
                    StateStore::open(state_dir.clone()).await
                }
                Err(error) => Err(error),
            }
        },
    )
    .await
    {
        Ok(Ok(store)) => {
            match store.read_launcher_activation_snapshot().await {
                Ok(_) => {}
                Err(crate::state_store::StateStoreError::MissingLauncherActivationSnapshot) => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(LAUNCHER_BOOTSTRAP_MUTATION_TIMEOUT_SECONDS),
                        ensure_launcher_bootstrap(
                            &store,
                            &instruction_source_root,
                            &framework_memory_source_root,
                        ),
                    )
                    .await
                    {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                        Err(_) => {
                            eprintln!(
                                "Timed out ensuring launcher bootstrap for `vida orchestrator-init` after {LAUNCHER_BOOTSTRAP_MUTATION_TIMEOUT_SECONDS}s"
                            );
                            return ExitCode::from(1);
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to read launcher activation snapshot: {error}");
                    return ExitCode::from(1);
                }
            }
            match tokio::time::timeout(
                std::time::Duration::from_secs(INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS),
                build_taskflow_consume_bundle_payload(&store),
            )
            .await
            {
                Ok(Ok(bundle)) => {
                    let project_activation_view = match std::env::current_dir() {
                        Ok(path) => {
                            super::project_activator_surface::build_project_activator_view(&path)
                        }
                        Err(error) => {
                            eprintln!("Failed to resolve current directory: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let init_view =
                        super::project_activator_surface::merge_project_activation_into_init_view(
                            bundle.orchestrator_init_view.clone(),
                            &project_activation_view,
                        );
                    let dev_team_readiness =
                        super::taskflow_consume_bundle::build_dev_team_readiness(
                            &bundle.config_path,
                            &bundle.activation_bundle,
                        );
                    let orchestrator_runtime_contract =
                        build_orchestrator_runtime_contract(&init_view, &dev_team_readiness);
                    if args.json {
                        let payload = if args.full {
                            build_orchestrator_init_full_payload(
                                &init_view,
                                &dev_team_readiness,
                                &orchestrator_runtime_contract,
                                &bundle,
                                store.root(),
                            )
                        } else {
                            build_orchestrator_init_summary_payload(
                                &init_view,
                                &dev_team_readiness,
                                &orchestrator_runtime_contract,
                                &bundle,
                                store.root(),
                            )
                        };
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&payload)
                                .expect("orchestrator-init json should render")
                        );
                        crate::operator_projection_cache::write_json_projection(
                            store.root(),
                            orchestrator_init_projection_name(args.full),
                            &payload,
                        );
                    } else {
                        print_surface_header(RenderMode::Plain, "vida orchestrator-init");
                        print_surface_line(
                            RenderMode::Plain,
                            "status",
                            init_view["status"].as_str().unwrap_or("unknown"),
                        );
                        print_surface_line(RenderMode::Plain, "boot surface", "vida boot");
                        print_surface_line(
                            RenderMode::Plain,
                            "bundle id",
                            bundle.metadata["bundle_id"].as_str().unwrap_or(""),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "state dir",
                            &store.root().display().to_string(),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "next lawful dispatch",
                            orchestrator_runtime_contract["next_lawful_dispatch_action"]["command"]
                                .as_str()
                                .unwrap_or("vida agent dispatch-next --dev-team --json"),
                        );
                        print_compact_command_families(RenderMode::Plain, "vida orchestrator-init");
                        if init_view["project_activation"]["activation_pending"]
                            .as_bool()
                            .unwrap_or(false)
                        {
                            print_surface_line(
                                RenderMode::Plain,
                                "next step",
                                "vida project-activator --json",
                            );
                            if let Some(example) = init_view["project_activation"]["interview"]
                                ["one_shot_example"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "activation example",
                                    example,
                                );
                            }
                            print_surface_line(
                                RenderMode::Plain,
                                "activation runtime",
                                "use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is pending",
                            );
                        } else if init_view["project_activation"]["normal_work_defaults"]
                            ["documentation_first_for_feature_requests"]
                            .as_bool()
                            .unwrap_or(false)
                        {
                            print_surface_line(
                                RenderMode::Plain,
                                "feature flow",
                                "for requests that combine research/specification/planning and implementation, start with one bounded design document before code execution",
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "feature intake",
                                init_view["project_activation"]["normal_work_defaults"]
                                    ["intake_runtime"]
                                    .as_str()
                                    .unwrap_or("vida taskflow consume final <request> --json"),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "design template",
                                init_view["project_activation"]["normal_work_defaults"]
                                    ["local_feature_design_template"]
                                    .as_str()
                                    .unwrap_or("docs/product/spec/templates/feature-design-document.template.md"),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "documentation runtime",
                                "open one feature epic and one spec-pack task in `vida taskflow`, then use `vida docflow` to initialize, finalize, and validate the design document before shaping the execution packet",
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "execution posture",
                                "after the bounded design document is ready, delegate normal write-producing work through the configured development team instead of collapsing directly into root-session coding",
                            );
                            if let Some(rule) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["selection_rule"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "agent model",
                                    "agent=execution carrier; role=runtime activation state",
                                );
                                print_surface_line(RenderMode::Plain, "carrier selection", rule);
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["snapshot"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "agent snapshot cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["carrier_catalog"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "carrier catalog cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["runtime_roles"]
                                .as_str()
                            {
                                print_surface_line(RenderMode::Plain, "runtime roles cmd", command);
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["scores"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "carrier scores cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["selection_preview"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "selection preview cmd",
                                    command,
                                );
                            }
                        }
                    }
                    ExitCode::SUCCESS
                }
                Ok(Err(error)) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
                Err(_) => emit_orchestrator_init_bundle_timeout(&state_dir, args.json),
            }
        }
        Ok(Err(error)) => {
            if StateStore::message_is_lock_contention(&error.to_string()) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    "vida orchestrator-init",
                    &state_dir,
                    RenderMode::Plain,
                    args.json,
                    &error.to_string(),
                );
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => crate::status_surface::emit_degraded_read_lock_surface(
            "vida orchestrator-init",
            &state_dir,
            RenderMode::Plain,
            args.json,
            &format!(
                "Timed out opening authoritative state store for `vida orchestrator-init` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s (cold authoritative state open timeout)"
            ),
        ),
    }
}

async fn execute_agent_init_dispatch_from_resume_inputs(
    json_output: bool,
    dispatch_mode: &serde_json::Value,
    state_root: std::path::PathBuf,
    mut resume_inputs: super::taskflow_consume_resume::ResumeInputs,
) -> ExitCode {
    if let Some(exit_code) = execute_agent_init_prelaunch_blocker_without_store_reopen(
        json_output,
        dispatch_mode,
        &state_root,
        &mut resume_inputs,
    )
    .await
    {
        return exit_code;
    }

    if resume_inputs.dispatch_receipt.dispatch_status == "executed"
        && resume_inputs.dispatch_receipt.lane_status == super::LaneStatus::LaneCompleted.as_str()
        && resume_inputs.dispatch_receipt.blocker_code.is_none()
        && super::runtime_dispatch_state::dispatch_receipt_has_execution_evidence(
            &resume_inputs.dispatch_receipt,
        )
    {
        let warning =
            super::runtime_dispatch_state::reconcile_executed_dispatch_result_state_best_effort(
                &state_root,
                &resume_inputs.run_graph_bootstrap,
                &resume_inputs.dispatch_receipt,
            )
            .await;
        return match render_agent_init_dispatch_result_from_receipt(
            dispatch_mode,
            &resume_inputs.dispatch_receipt,
            json_output,
            None,
            warning.as_deref(),
        ) {
            Ok(exit_code) => exit_code,
            Err(render_error) => {
                eprintln!("{render_error}");
                ExitCode::from(1)
            }
        };
    }

    let dispatch_handoff_timeout_seconds = super::dispatch_handoff_timeout_seconds_for_state_root(
        &state_root,
        &resume_inputs.role_selection,
        &resume_inputs.dispatch_receipt,
    );
    let uses_internal_host =
        super::runtime_dispatch_state::dispatch_handoff_uses_internal_host_for_state_root(
            &state_root,
            &resume_inputs.role_selection,
            &resume_inputs.dispatch_receipt,
        );
    if agent_init_execute_dispatch_should_handoff(
        dispatch_handoff_timeout_seconds,
        uses_internal_host,
    ) {
        return match start_agent_init_dispatch_worker_and_return(
            json_output,
            dispatch_mode,
            &state_root,
            &mut resume_inputs,
        )
        .await
        {
            Ok(exit_code) => exit_code,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(1)
            }
        };
    }
    let execute_dispatch_timeout_seconds =
        agent_init_execute_dispatch_timeout_seconds(dispatch_handoff_timeout_seconds);
    let receipt_timeout_seconds = agent_init_receipt_timeout_seconds(
        dispatch_handoff_timeout_seconds,
        execute_dispatch_timeout_seconds,
    );
    match tokio::time::timeout(
        std::time::Duration::from_secs(execute_dispatch_timeout_seconds),
        super::execute_and_record_dispatch_receipt(
            &state_root,
            &resume_inputs.role_selection,
            &resume_inputs.run_graph_bootstrap,
            &mut resume_inputs.dispatch_receipt,
        ),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if resume_inputs.dispatch_receipt.dispatch_status == "blocked"
                && resume_inputs.dispatch_receipt.blocker_code.as_deref()
                    == Some("internal_dispatch_timeout_without_receipt")
            {
                match render_agent_init_dispatch_result_from_receipt(
                    dispatch_mode,
                    &resume_inputs.dispatch_receipt,
                    json_output,
                    Some(execute_dispatch_timeout_seconds),
                    None,
                ) {
                    Ok(exit_code) => return exit_code,
                    Err(render_error) => {
                        eprintln!("{render_error}");
                        return ExitCode::from(1);
                    }
                }
            }
            eprintln!("Failed to execute agent-init dispatch packet: {error}");
            return ExitCode::from(1);
        }
        Err(_) => {
            match super::runtime_dispatch_state::apply_existing_executed_dispatch_result_to_receipt(
                &state_root,
                &mut resume_inputs.dispatch_receipt,
            ) {
                Ok(true) => {
                    let warning =
                        super::runtime_dispatch_state::reconcile_executed_dispatch_result_state_best_effort(
                            &state_root,
                            &resume_inputs.run_graph_bootstrap,
                            &resume_inputs.dispatch_receipt,
                        )
                        .await;
                    return match render_agent_init_dispatch_result_from_receipt(
                        dispatch_mode,
                        &resume_inputs.dispatch_receipt,
                        json_output,
                        None,
                        warning.as_deref(),
                    ) {
                        Ok(exit_code) => exit_code,
                        Err(render_error) => {
                            eprintln!("{render_error}");
                            ExitCode::from(1)
                        }
                    };
                }
                Ok(false) => {}
                Err(error) => {
                    eprintln!(
                        "Failed to inspect existing executed dispatch result before timeout materialization: {error}"
                    );
                }
            }
            if let Err(error) = super::apply_dispatch_handoff_timeout_to_receipt_for_state_root(
                &state_root,
                &resume_inputs.role_selection,
                &mut resume_inputs.dispatch_receipt,
                receipt_timeout_seconds,
            ) {
                if json_output {
                    crate::print_json_pretty(&agent_init_dispatch_timeout_fallback_payload(
                        dispatch_mode,
                        &resume_inputs.dispatch_receipt.run_id,
                        resume_inputs
                            .dispatch_receipt
                            .dispatch_result_path
                            .as_deref(),
                        execute_dispatch_timeout_seconds,
                        Some(&error.to_string()),
                    ));
                    return ExitCode::from(1);
                }
                eprintln!(
                    "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion, and failed to materialize timeout receipt: {error}"
                );
                return ExitCode::from(1);
            }
            let timeout_warning = best_effort_record_agent_init_dispatch_timeout_receipt(
                &state_root,
                &resume_inputs.run_graph_bootstrap,
                &resume_inputs.dispatch_receipt,
                execute_dispatch_timeout_seconds,
            )
            .await;
            if json_output {
                let dispatch_result_path = resume_inputs
                    .dispatch_receipt
                    .dispatch_result_path
                    .as_deref();
                let result_json = dispatch_result_path
                    .and_then(|path| {
                        std::fs::read_to_string(path)
                            .ok()
                            .and_then(|body| serde_json::from_str(&body).ok())
                    })
                    .map(|result_json| {
                        agent_init_dispatch_timeout_operator_envelope(
                            result_json,
                            dispatch_mode,
                            &resume_inputs.dispatch_receipt.run_id,
                            dispatch_result_path,
                            execute_dispatch_timeout_seconds,
                            timeout_warning.as_deref(),
                        )
                    })
                    .unwrap_or_else(|| {
                        agent_init_dispatch_timeout_fallback_payload(
                            dispatch_mode,
                            &resume_inputs.dispatch_receipt.run_id,
                            dispatch_result_path,
                            execute_dispatch_timeout_seconds,
                            timeout_warning.as_deref(),
                        )
                    });
                crate::print_json_pretty(&result_json);
                return ExitCode::from(1);
            }
            if let Some(warning) = timeout_warning {
                eprintln!("{warning}");
            }
            eprintln!(
                "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion"
            );
            return ExitCode::from(1);
        }
    }
    match render_agent_init_dispatch_result_from_receipt(
        dispatch_mode,
        &resume_inputs.dispatch_receipt,
        json_output,
        None,
        None,
    ) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

async fn execute_agent_init_prelaunch_blocker_without_store_reopen(
    json_output: bool,
    dispatch_mode: &serde_json::Value,
    state_root: &Path,
    resume_inputs: &mut super::taskflow_consume_resume::ResumeInputs,
) -> Option<ExitCode> {
    if resume_inputs.dispatch_receipt.dispatch_kind != "agent_lane" {
        return None;
    }
    let project_root = super::runtime_dispatch_project_root_from_state_root(state_root);
    resume_inputs.dispatch_receipt.selected_backend =
        super::runtime_dispatch_state::preferred_selected_backend_for_receipt(
            &resume_inputs.role_selection,
            &resume_inputs.dispatch_receipt,
        );
    super::runtime_dispatch_state::sync_receipt_dispatch_handoff_surface(
        project_root.as_ref(),
        &resume_inputs.role_selection,
        &mut resume_inputs.dispatch_receipt,
    );
    if !super::runtime_dispatch_state::internal_host_dispatch_requires_prelaunch_blocker(
        project_root.as_ref(),
        &resume_inputs.role_selection,
        &resume_inputs.dispatch_receipt,
    ) {
        return None;
    }
    if let Err(error) =
        super::runtime_dispatch_state::apply_internal_activation_view_only_to_receipt(
            state_root,
            project_root.as_ref(),
            &resume_inputs.role_selection,
            &mut resume_inputs.dispatch_receipt,
        )
    {
        eprintln!("Failed to materialize agent-init prelaunch blocker result: {error}");
        return Some(ExitCode::from(1));
    }
    let persist_warning = persist_agent_init_prelaunch_blocked_dispatch_receipt(
        state_root,
        &resume_inputs.dispatch_receipt,
    )
    .await;
    let Some(dispatch_result_path) = resume_inputs
        .dispatch_receipt
        .dispatch_result_path
        .as_deref()
    else {
        eprintln!("Agent init prelaunch blocker did not produce a dispatch result artifact.");
        return Some(ExitCode::from(1));
    };
    let result_body = match std::fs::read_to_string(dispatch_result_path) {
        Ok(body) => body,
        Err(error) => {
            eprintln!(
                "Failed to read agent-init prelaunch dispatch result `{dispatch_result_path}`: {error}"
            );
            return Some(ExitCode::from(1));
        }
    };
    let mut result_json = match serde_json::from_str::<serde_json::Value>(&result_body) {
        Ok(json) => json,
        Err(error) => {
            eprintln!(
                "Failed to parse agent-init prelaunch dispatch result `{dispatch_result_path}`: {error}"
            );
            return Some(ExitCode::from(1));
        }
    };
    if let Some(object) = result_json.as_object_mut() {
        object.insert("dispatch_mode".to_string(), dispatch_mode.clone());
        if let Some(warning) = persist_warning {
            object.insert(
                "prelaunch_reconciliation_warning".to_string(),
                serde_json::json!(warning),
            );
        }
    }
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&result_json)
                .expect("agent-init prelaunch result json should render")
        );
    } else {
        crate::print_json_pretty(&result_json);
    }
    Some(ExitCode::from(1))
}

async fn persist_agent_init_prelaunch_blocked_dispatch_receipt(
    state_root: &Path,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<String> {
    let store = match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    {
        Ok(Ok(store)) => store,
        Ok(Err(error)) => {
            return Some(format!(
                "authoritative prelaunch-blocker receipt persistence deferred until next safe reopen: failed to reopen state store: {error}"
            ));
        }
        Err(_) => {
            return Some(format!(
                "authoritative prelaunch-blocker receipt persistence deferred until next safe reopen: timed out reopening state store after {}s",
                DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS
            ));
        }
    };
    store
        .record_run_graph_dispatch_receipt(receipt)
        .await
        .err()
        .map(|error| {
            format!(
                "authoritative prelaunch-blocker receipt persistence deferred until next safe reopen: failed to persist blocked dispatch receipt: {error}"
            )
        })
}

fn string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalized_packet_arg_path(packet_path: &str) -> std::path::PathBuf {
    let trimmed = packet_path.trim();
    let unquoted = trimmed
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .or_else(|| {
            trimmed
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .unwrap_or(trimmed);
    super::runtime_dispatch_state::normalize_persisted_runtime_path(unquoted)
}

fn resume_inputs_from_downstream_packet_without_store(
    packet_path: &str,
) -> Result<super::taskflow_consume_resume::ResumeInputs, String> {
    let normalized_packet_path = normalized_packet_arg_path(packet_path);
    let normalized_packet_path = normalized_packet_path.display().to_string();
    let packet = super::taskflow_consume_resume::read_dispatch_packet(&normalized_packet_path)?;
    let run_id = string_field(&packet, "run_id")
        .ok_or_else(|| "Persisted downstream dispatch packet is missing run_id".to_string())?;
    let dispatch_target = string_field(&packet, "downstream_dispatch_target").ok_or_else(|| {
        "Persisted downstream dispatch packet is missing downstream_dispatch_target".to_string()
    })?;
    let role_selection: super::RuntimeConsumptionLaneSelection = serde_json::from_value(
        packet
            .get("role_selection_full")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| {
        format!("Failed to decode role_selection from downstream dispatch packet: {error}")
    })?;
    let downstream_dispatch_ready = packet
        .get("downstream_dispatch_ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let downstream_dispatch_blockers = packet
        .get("downstream_dispatch_blockers")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if downstream_dispatch_ready && !downstream_dispatch_blockers.is_empty() {
        return Err(
            "Persisted downstream dispatch packet has packet_ready status but also blocker evidence"
                .to_string(),
        );
    }
    let dispatch_status = if downstream_dispatch_ready && downstream_dispatch_blockers.is_empty() {
        "packet_ready".to_string()
    } else {
        string_field(&packet, "downstream_dispatch_status").unwrap_or_else(|| "blocked".to_string())
    };
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status,
        lane_status: string_field(&packet, "downstream_lane_status")
            .unwrap_or_else(|| "packet_ready".to_string()),
        supersedes_receipt_id: string_field(&packet, "downstream_supersedes_receipt_id"),
        exception_path_receipt_id: string_field(&packet, "downstream_exception_path_receipt_id"),
        dispatch_kind: "agent_lane".to_string(),
        dispatch_surface: Some("vida agent-init".to_string()),
        dispatch_command: string_field(&packet, "downstream_dispatch_command"),
        dispatch_packet_path: Some(normalized_packet_path.clone()),
        dispatch_result_path: None,
        blocker_code: None,
        downstream_dispatch_target: None,
        downstream_dispatch_command: None,
        downstream_dispatch_note: None,
        downstream_dispatch_ready: false,
        downstream_dispatch_blockers: Vec::new(),
        downstream_dispatch_packet_path: None,
        downstream_dispatch_status: None,
        downstream_dispatch_result_path: None,
        downstream_dispatch_trace_path: None,
        downstream_dispatch_executed_count: packet
            .get("downstream_dispatch_executed_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        downstream_dispatch_active_target: None,
        downstream_dispatch_last_target: string_field(&packet, "source_dispatch_target")
            .or_else(|| string_field(&packet, "downstream_dispatch_last_target")),
        activation_agent_type: string_field(&packet, "activation_agent_type"),
        activation_runtime_role: string_field(&packet, "activation_runtime_role"),
        selected_backend: string_field(&packet, "selected_backend"),
        recorded_at,
    };
    let run_graph_bootstrap = packet
        .get("run_graph_bootstrap")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(super::taskflow_consume_resume::ResumeInputs {
        dispatch_receipt: receipt,
        dispatch_packet_path: normalized_packet_path,
        role_selection,
        run_graph_bootstrap,
    })
}

fn resume_inputs_from_dispatch_packet_without_store(
    packet_path: &str,
) -> Result<super::taskflow_consume_resume::ResumeInputs, String> {
    let normalized_packet_path = normalized_packet_arg_path(packet_path);
    let normalized_packet_path = normalized_packet_path.display().to_string();
    let body = std::fs::read_to_string(&normalized_packet_path)
        .map_err(|error| format!("Failed to read dispatch packet `{packet_path}`: {error}"))?;
    let packet = serde_json::from_str::<serde_json::Value>(&body)
        .map_err(|error| format!("Failed to parse dispatch packet `{packet_path}`: {error}"))?;
    let run_id = string_field(&packet, "run_id")
        .ok_or_else(|| "Persisted dispatch packet is missing run_id".to_string())?;
    let dispatch_target = string_field(&packet, "dispatch_target")
        .ok_or_else(|| "Persisted dispatch packet is missing dispatch_target".to_string())?;
    let role_selection: super::RuntimeConsumptionLaneSelection = serde_json::from_value(
        packet
            .get("role_selection_full")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| format!("Failed to decode role_selection from dispatch packet: {error}"))?;
    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("rfc3339 timestamp should render");
    let receipt = crate::state_store::RunGraphDispatchReceipt {
        run_id: run_id.clone(),
        dispatch_target: dispatch_target.clone(),
        dispatch_status: string_field(&packet, "dispatch_status")
            .unwrap_or_else(|| "packet_ready".to_string()),
        lane_status: string_field(&packet, "lane_status")
            .unwrap_or_else(|| "packet_ready".to_string()),
        supersedes_receipt_id: string_field(&packet, "supersedes_receipt_id"),
        exception_path_receipt_id: string_field(&packet, "exception_path_receipt_id"),
        dispatch_kind: string_field(&packet, "dispatch_kind")
            .unwrap_or_else(|| "agent_lane".to_string()),
        dispatch_surface: string_field(&packet, "dispatch_surface")
            .or_else(|| Some("vida agent-init".to_string())),
        dispatch_command: string_field(&packet, "dispatch_command"),
        dispatch_packet_path: Some(normalized_packet_path.clone()),
        dispatch_result_path: None,
        blocker_code: string_field(&packet, "blocker_code"),
        downstream_dispatch_target: string_field(&packet, "downstream_dispatch_target"),
        downstream_dispatch_command: string_field(&packet, "downstream_dispatch_command"),
        downstream_dispatch_note: string_field(&packet, "downstream_dispatch_note"),
        downstream_dispatch_ready: packet
            .get("downstream_dispatch_ready")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        downstream_dispatch_blockers: packet
            .get("downstream_dispatch_blockers")
            .and_then(serde_json::Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        downstream_dispatch_packet_path: string_field(&packet, "downstream_dispatch_packet_path"),
        downstream_dispatch_status: string_field(&packet, "downstream_dispatch_status"),
        downstream_dispatch_result_path: string_field(&packet, "downstream_dispatch_result_path"),
        downstream_dispatch_trace_path: string_field(&packet, "downstream_dispatch_trace_path"),
        downstream_dispatch_executed_count: packet
            .get("downstream_dispatch_executed_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        downstream_dispatch_active_target: string_field(
            &packet,
            "downstream_dispatch_active_target",
        ),
        downstream_dispatch_last_target: string_field(&packet, "downstream_dispatch_last_target"),
        activation_agent_type: string_field(&packet, "activation_agent_type"),
        activation_runtime_role: string_field(&packet, "activation_runtime_role"),
        selected_backend: string_field(&packet, "selected_backend"),
        recorded_at,
    };
    let run_graph_bootstrap = packet
        .get("run_graph_bootstrap")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "run_id": run_id }));
    Ok(super::taskflow_consume_resume::ResumeInputs {
        dispatch_receipt: receipt,
        dispatch_packet_path: normalized_packet_path,
        role_selection,
        run_graph_bootstrap,
    })
}

async fn merge_persisted_dispatch_receipt_without_resume_gate(
    store: &StateStore,
    mut inputs: super::taskflow_consume_resume::ResumeInputs,
) -> Result<super::taskflow_consume_resume::ResumeInputs, String> {
    let Some(receipt) = store
        .run_graph_dispatch_receipt(&inputs.dispatch_receipt.run_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to read active dispatch receipt for packet-backed worker resume: {error}"
            )
        })?
    else {
        return Ok(inputs);
    };
    let active_packet_path = receipt
        .dispatch_packet_path
        .as_deref()
        .map(normalized_packet_arg_path);
    let requested_packet_path = normalized_packet_arg_path(&inputs.dispatch_packet_path);
    let packet_matches = active_packet_path
        .as_ref()
        .is_some_and(|path| path == &requested_packet_path);
    if receipt.dispatch_target == inputs.dispatch_receipt.dispatch_target && packet_matches {
        inputs.dispatch_receipt = receipt;
        inputs.dispatch_packet_path = requested_packet_path.display().to_string();
        inputs.dispatch_receipt.dispatch_packet_path = Some(inputs.dispatch_packet_path.clone());
    }
    Ok(inputs)
}

pub(crate) async fn run_agent_init(args: AgentInitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .clone()
        .unwrap_or_else(state_store::default_state_dir);
    let packet_arg_count =
        usize::from(args.dispatch_packet.is_some()) + usize::from(args.downstream_packet.is_some());
    if let Err(error) = validate_agent_init_auto_dispatch_packet_args(&args, packet_arg_count) {
        eprintln!("{error}");
        return ExitCode::from(2);
    }

    if !args.execute_dispatch && packet_arg_count > 0 {
        if packet_arg_count > 1 {
            eprintln!(
                "Agent init accepts at most one packet source: use either `--dispatch-packet` or `--downstream-packet`."
            );
            return ExitCode::from(2);
        }
        if args.role.is_some() || args.request_text.is_some() {
            eprintln!(
                "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
            );
            return ExitCode::from(2);
        }
        let (packet_path, downstream) = if let Some(packet_path) = args.dispatch_packet.as_deref() {
            (packet_path, false)
        } else {
            (
                args.downstream_packet
                    .as_deref()
                    .expect("packet source count checked above"),
                true,
            )
        };
        let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path) {
            Ok(packet) => packet,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        };
        let project_root = match std::env::current_dir() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("Failed to resolve current directory: {error}");
                return ExitCode::from(1);
            }
        };
        let surface_payload = match build_fast_agent_init_packet_activation_payload(
            &project_root,
            packet_path,
            packet,
            downstream,
            &args,
        ) {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
        };
        emit_fast_agent_init_packet_activation_payload(&surface_payload, args.json);
        return ExitCode::SUCCESS;
    }

    if args.execute_dispatch && args.downstream_packet.is_some() && args.dispatch_packet.is_none() {
        let packet_path = args
            .downstream_packet
            .as_deref()
            .expect("downstream packet checked above");
        let resume_inputs = match resume_inputs_from_downstream_packet_without_store(packet_path) {
            Ok(inputs) => inputs,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        };
        let selection_value =
            serde_json::to_value(&resume_inputs.role_selection).unwrap_or(serde_json::Value::Null);
        let dispatch_mode = agent_init_dispatch_mode(&args, &selection_value);
        if !agent_init_execute_dispatch_worker_active() {
            let mut resume_inputs = resume_inputs;
            return match start_agent_init_dispatch_worker_and_return(
                args.json,
                &dispatch_mode,
                &state_dir,
                &mut resume_inputs,
            )
            .await
            {
                Ok(exit_code) => exit_code,
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            };
        }
        return execute_agent_init_dispatch_from_resume_inputs(
            args.json,
            &dispatch_mode,
            state_dir,
            resume_inputs,
        )
        .await;
    }

    let _read_surface_guard = if args.execute_dispatch {
        None
    } else {
        Some(
            AGENT_INIT_READ_SURFACE_GUARD
                .get_or_init(|| tokio::sync::Mutex::new(()))
                .lock()
                .await,
        )
    };

    match tokio::time::timeout(
        std::time::Duration::from_secs(COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS),
        async {
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => Ok(store),
                Err(crate::state_store::StateStoreError::MissingStateDir(_)) => {
                    StateStore::open(state_dir.clone()).await
                }
                Err(error) => Err(error),
            }
        },
    )
    .await
    {
        Ok(Ok(store)) => {
            let store_state_root = store.root().to_path_buf();
            if args.execute_dispatch {
                if packet_arg_count == 0 && !args.auto_dispatch_packet {
                    return emit_agent_init_execute_dispatch_missing_packet(&args);
                }
                if packet_arg_count > 1 {
                    eprintln!(
                        "Agent init accepts at most one packet source: use either `--dispatch-packet` or `--downstream-packet`."
                    );
                    return ExitCode::from(2);
                }
                let resume_inputs = if agent_init_execute_dispatch_worker_active()
                    && packet_arg_count > 0
                {
                    let packet_path = args
                        .dispatch_packet
                        .as_deref()
                        .or(args.downstream_packet.as_deref())
                        .expect("packet_arg_count > 0 should provide a packet path");
                    match resume_inputs_from_dispatch_packet_without_store(packet_path) {
                        Ok(inputs) => {
                            match merge_persisted_dispatch_receipt_without_resume_gate(
                                &store, inputs,
                            )
                            .await
                            {
                                Ok(inputs) => inputs,
                                Err(error) => {
                                    if args.json {
                                        let dispatch_mode = agent_init_dispatch_mode(
                                            &args,
                                            &serde_json::Value::Null,
                                        );
                                        crate::print_json_pretty(
                                            &agent_init_execute_dispatch_resume_error_payload(
                                                &dispatch_mode,
                                                &error,
                                            ),
                                        );
                                        return ExitCode::from(1);
                                    }
                                    eprintln!("{error}");
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        Err(error) => {
                            if args.json {
                                let dispatch_mode =
                                    agent_init_dispatch_mode(&args, &serde_json::Value::Null);
                                crate::print_json_pretty(
                                    &agent_init_execute_dispatch_resume_error_payload(
                                        &dispatch_mode,
                                        &error,
                                    ),
                                );
                                return ExitCode::from(1);
                            }
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    }
                } else {
                    match super::taskflow_consume_resume::resolve_runtime_consumption_resume_inputs(
                        &store,
                        None,
                        args.dispatch_packet.as_deref(),
                        args.downstream_packet.as_deref(),
                    )
                    .await
                    {
                        Ok(inputs) => inputs,
                        Err(error) => {
                            if args.json {
                                let dispatch_mode =
                                    agent_init_dispatch_mode(&args, &serde_json::Value::Null);
                                let (receipt_evidence, result_artifact) =
                                    agent_init_execute_dispatch_resume_error_receipt_evidence(
                                        &store,
                                        &error,
                                        args.dispatch_packet.as_deref(),
                                    )
                                    .await;
                                crate::print_json_pretty(
                                    &agent_init_execute_dispatch_resume_error_payload_with_receipt_evidence(
                                        &dispatch_mode,
                                        &error,
                                        receipt_evidence.as_ref(),
                                        result_artifact.as_ref(),
                                    ),
                                );
                                return ExitCode::from(1);
                            }
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    }
                };
                let selection_value = serde_json::to_value(&resume_inputs.role_selection)
                    .unwrap_or(serde_json::Value::Null);
                let dispatch_mode = agent_init_dispatch_mode(&args, &selection_value);
                if args.auto_dispatch_packet {
                    if let Err(error) =
                        validate_agent_init_auto_dispatch_active_unit(&store, &resume_inputs).await
                    {
                        if args.json {
                            crate::print_json_pretty(
                                &agent_init_auto_dispatch_active_unit_blocked_payload(
                                    &dispatch_mode,
                                    &error,
                                    &resume_inputs.dispatch_packet_path,
                                ),
                            );
                        } else {
                            eprintln!("{}", error.detail);
                        }
                        return ExitCode::from(1);
                    }
                }
                drop(store);
                return execute_agent_init_dispatch_from_resume_inputs(
                    args.json,
                    &dispatch_mode,
                    store_state_root,
                    resume_inputs,
                )
                .await;
            }
            let bundle = match tokio::time::timeout(
                std::time::Duration::from_secs(INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS),
                build_taskflow_consume_bundle_payload(&store),
            )
            .await
            {
                Ok(Ok(bundle)) => bundle,
                Ok(Err(error)) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
                Err(_) => return emit_agent_init_bundle_timeout(&state_dir, args.json),
            };
            if packet_arg_count > 1 {
                eprintln!(
                    "Agent init accepts at most one packet source: use either `--dispatch-packet` or `--downstream-packet`."
                );
                return ExitCode::from(2);
            }
            let selection = if let Some(packet_path) = args.dispatch_packet.as_deref() {
                if args.role.is_some() || args.request_text.is_some() {
                    eprintln!(
                        "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
                    );
                    return ExitCode::from(2);
                }
                let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path)
                {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                match agent_init_packet_selection(packet_path, packet, false) {
                    Ok(selection) => selection,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                }
            } else if let Some(packet_path) = args.downstream_packet.as_deref() {
                if args.role.is_some() || args.request_text.is_some() {
                    eprintln!(
                        "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
                    );
                    return ExitCode::from(2);
                }
                let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path)
                {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                match agent_init_packet_selection(packet_path, packet, true) {
                    Ok(selection) => selection,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                }
            } else if let Some(role) = args.role.clone() {
                let compiled_bundle = &bundle.activation_bundle;
                if !role_exists_in_lane_bundle(compiled_bundle, &role) || role == "orchestrator" {
                    eprintln!(
                        "Agent init requires a non-orchestrator lane role present in the compiled activation bundle."
                    );
                    return ExitCode::from(2);
                }
                serde_json::json!({
                    "mode": "explicit_role",
                    "selected_role": role,
                    "request_text": args.request_text.clone().unwrap_or_default(),
                })
            } else {
                let request = match args.request_text.as_deref() {
                    Some(request) if !request.trim().is_empty() => request,
                    _ => {
                        eprintln!(
                            "Agent init requires either a non-orchestrator `--role` or a bounded request text."
                        );
                        return ExitCode::from(2);
                    }
                };
                match build_runtime_lane_selection_with_store(&store, request).await {
                    Ok(selection) => {
                        if selection.selected_role == "orchestrator" {
                            eprintln!(
                                "Agent init resolved to orchestrator posture; provide a non-orchestrator `--role` or a bounded worker request."
                            );
                            return ExitCode::from(2);
                        }
                        serde_json::to_value(selection).expect("lane selection should serialize")
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };

            let project_root = match std::env::current_dir() {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("Failed to resolve current directory: {error}");
                    return ExitCode::from(1);
                }
            };
            let project_activation_view =
                super::project_activator_surface::build_project_activator_view(&project_root);
            let init_view =
                super::project_activator_surface::merge_project_activation_into_init_view(
                    bundle.agent_init_view,
                    &project_activation_view,
                );
            let dev_team_readiness = super::taskflow_consume_bundle::build_dev_team_readiness(
                &bundle.config_path,
                &bundle.activation_bundle,
            );
            let activation_semantics = agent_init_activation_semantics(&selection);
            let dispatch_mode = agent_init_dispatch_mode(&args, &selection);
            let surface_payload = build_agent_init_surface_payload(
                &project_root,
                &bundle.config_path,
                init_view.clone(),
                selection.clone(),
                activation_semantics.clone(),
                dispatch_mode.clone(),
                serde_json::json!({
                    "bundle_id": bundle.metadata["bundle_id"],
                    "activation_source": bundle.activation_source,
                    "vida_root": bundle.vida_root,
                    "state_dir": store.root().display().to_string(),
                    "launcher_runtime_paths": bundle.launcher_runtime_paths,
                }),
                &bundle.activation_bundle,
                dev_team_readiness,
            );

            if surface_payload["backend_truth"]["assignment_blocker"]["authoritative"]
                .as_bool()
                .unwrap_or(false)
            {
                let blocker_code = surface_payload["backend_truth"]["assignment_blocker"]
                    ["blocker_code"]
                    .as_str()
                    .unwrap_or("runtime_assignment_truth_required");
                eprintln!(
                    "Agent init requires runtime assignment truth for `{}` mode: {}.",
                    selection["mode"].as_str().unwrap_or("unknown"),
                    blocker_code
                );
                return ExitCode::from(1);
            }

            if args.execute_dispatch {
                let dispatch_setup = {
                    if packet_arg_count == 0 && !args.auto_dispatch_packet {
                        return emit_agent_init_execute_dispatch_missing_packet(&args);
                    }

                    let resume_inputs = if agent_init_execute_dispatch_worker_active()
                        && packet_arg_count > 0
                    {
                        let packet_path = args
                            .dispatch_packet
                            .as_deref()
                            .or(args.downstream_packet.as_deref())
                            .expect("packet_arg_count > 0 should provide a packet path");
                        match resume_inputs_from_dispatch_packet_without_store(packet_path) {
                            Ok(inputs) => {
                                match merge_persisted_dispatch_receipt_without_resume_gate(
                                    &store, inputs,
                                )
                                .await
                                {
                                    Ok(inputs) => inputs,
                                    Err(error) => {
                                        if args.json {
                                            crate::print_json_pretty(
                                                &agent_init_execute_dispatch_resume_error_payload(
                                                    &dispatch_mode,
                                                    &error,
                                                ),
                                            );
                                            return ExitCode::from(1);
                                        }
                                        eprintln!("{error}");
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            Err(error) => {
                                if args.json {
                                    crate::print_json_pretty(
                                        &agent_init_execute_dispatch_resume_error_payload(
                                            &dispatch_mode,
                                            &error,
                                        ),
                                    );
                                    return ExitCode::from(1);
                                }
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        }
                    } else {
                        match super::taskflow_consume_resume::resolve_runtime_consumption_resume_inputs(
                            &store,
                            None,
                            args.dispatch_packet.as_deref(),
                            args.downstream_packet.as_deref(),
                        )
                        .await
                        {
                            Ok(inputs) => {
                                if args.auto_dispatch_packet {
                                    if let Err(error) =
                                        validate_agent_init_auto_dispatch_active_unit(
                                            &store,
                                            &inputs,
                                        )
                                        .await
                                    {
                                        if args.json {
                                            crate::print_json_pretty(
                                                &agent_init_auto_dispatch_active_unit_blocked_payload(
                                                    &dispatch_mode,
                                                    &error,
                                                    &inputs.dispatch_packet_path,
                                                ),
                                            );
                                        } else {
                                            eprintln!("{}", error.detail);
                                        }
                                        return ExitCode::from(1);
                                    }
                                }
                                inputs
                            }
                            Err(error) => {
                                if args.json {
                                    let (receipt_evidence, result_artifact) =
                                        agent_init_execute_dispatch_resume_error_receipt_evidence(
                                            &store,
                                            &error,
                                            args.dispatch_packet.as_deref(),
                                        )
                                        .await;
                                    crate::print_json_pretty(
                                        &agent_init_execute_dispatch_resume_error_payload_with_receipt_evidence(
                                            &dispatch_mode,
                                            &error,
                                            receipt_evidence.as_ref(),
                                            result_artifact.as_ref(),
                                        ),
                                    );
                                    return ExitCode::from(1);
                                }
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                        }
                    };
                    (store_state_root.clone(), resume_inputs)
                };
                drop(store);
                let (state_root, resume_inputs) = dispatch_setup;
                return execute_agent_init_dispatch_from_resume_inputs(
                    args.json,
                    &dispatch_mode,
                    state_root,
                    resume_inputs,
                )
                .await;
            }

            drop(store);

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&surface_payload)
                        .expect("agent-init json should render")
                );
            } else {
                print_surface_header(RenderMode::Plain, "vida agent-init");
                print_surface_line(
                    RenderMode::Plain,
                    "status",
                    init_view["status"].as_str().unwrap_or("unknown"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "selected role",
                    selection["selected_role"].as_str().unwrap_or("unknown"),
                );
                if let Some(mode) = selection["mode"].as_str() {
                    print_surface_line(RenderMode::Plain, "mode", mode);
                }
                if let Some(mode) = surface_payload["dispatch_mode"]["mode"].as_str() {
                    print_surface_line(RenderMode::Plain, "dispatch_mode", mode);
                }
                if let Some(path) = selection["dispatch_packet_path"].as_str() {
                    print_surface_line(RenderMode::Plain, "dispatch packet", path);
                }
                if let Some(path) = selection["downstream_packet_path"].as_str() {
                    print_surface_line(RenderMode::Plain, "downstream packet", path);
                }
                print_compact_command_families(RenderMode::Plain, "vida agent-init");
                if let Some(backend) = surface_payload["backend_truth"]["selected_backend"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "selected backend", backend);
                }
                if let Some(source) =
                    surface_payload["backend_truth"]["selected_backend_source"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "backend source", source);
                }
                if let Some(carrier_id) =
                    surface_payload["backend_truth"]["selected_carrier_id"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "selected carrier", carrier_id);
                }
                if let Some(profile_id) =
                    surface_payload["backend_truth"]["selected_model_profile_id"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "selected model profile", profile_id);
                }
                if let Some(backend) =
                    surface_payload["backend_truth"]["route_primary_backend"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "route primary backend", backend);
                }
                if let Some(backend) =
                    surface_payload["backend_truth"]["route_fallback_backend"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "fallback backend", backend);
                }
                if let Some(posture) =
                    surface_payload["backend_truth"]["effective_execution_posture"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "execution posture", posture);
                }
                if let Some(status) = surface_payload["backend_truth"]["override_status"].as_str() {
                    print_surface_line(RenderMode::Plain, "lawful override", status);
                }
                print_surface_line(
                    RenderMode::Plain,
                    "activation semantics",
                    activation_semantics["activation_kind"]
                        .as_str()
                        .unwrap_or("activation_view"),
                );
                if let Some(next_step) = activation_semantics["next_lawful_action"].as_str() {
                    print_surface_line(RenderMode::Plain, "next lawful action", next_step);
                }
                if let Some(next_execution_action) =
                    surface_payload["operator_guidance"]["next_lawful_execution_action"].as_str()
                {
                    print_surface_line(
                        RenderMode::Plain,
                        "next execution action",
                        next_execution_action,
                    );
                }
                if let Some(stage) = surface_payload["operator_guidance"]["flow_distinctions"]
                    .as_array()
                    .and_then(|rows| rows.first())
                    .and_then(|row| row.get("stage"))
                    .and_then(serde_json::Value::as_str)
                {
                    print_surface_line(RenderMode::Plain, "agent-init stage", stage);
                }
                if let Some(fallback_surface) = init_view["source_mode_fallback_surface"].as_str() {
                    print_surface_line(RenderMode::Plain, "fallback surface", fallback_surface);
                }
                if init_view["project_activation"]["activation_pending"]
                    .as_bool()
                    .unwrap_or(false)
                {
                    print_surface_line(
                        RenderMode::Plain,
                        "next step",
                        "vida project-activator --json",
                    );
                    if let Some(example) =
                        init_view["project_activation"]["interview"]["one_shot_example"].as_str()
                    {
                        print_surface_line(RenderMode::Plain, "activation example", example);
                    }
                    print_surface_line(
                        RenderMode::Plain,
                        "activation runtime",
                        "use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is pending",
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Ok(Err(error)) => {
            if StateStore::message_is_lock_contention(&error.to_string()) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    "vida agent-init",
                    &state_dir,
                    RenderMode::Plain,
                    args.json,
                    &error.to_string(),
                );
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!(
                "Timed out opening authoritative state store for `vida agent-init` after {COLD_AUTHORITATIVE_STATE_OPEN_TIMEOUT_SECONDS}s (cold authoritative state open timeout)"
            );
            ExitCode::from(1)
        }
    }
}

fn agent_init_activation_semantics(selection: &serde_json::Value) -> serde_json::Value {
    let mode = selection["mode"].as_str().unwrap_or("unknown");
    let packet_template_kind = selection["packet_template_kind"]
        .as_str()
        .unwrap_or_default();
    let tracked_flow_shaping_only = packet_template_kind == "tracked_flow_packet";
    let next_lawful_action = match mode {
        "dispatch_packet" | "downstream_packet" if tracked_flow_shaping_only => {
            "complete only the tracked-flow/task-shaping handoff; this activation does not itself execute implementation and does not authorize root-session writing"
        }
        "dispatch_packet" | "downstream_packet" => {
            "use this activation view to execute only the bounded packet owned by the selected lane; completion still requires receipt-backed evidence and does not transfer root-session write authority; if execution evidence is still missing, continue bounded diagnosis/reroute rather than treating this view as completion"
        }
        "explicit_role" => {
            "use this bounded startup view to initialize the selected non-orchestrator lane; execution still requires a lawful packet or bounded worker request"
        }
        _ => {
            "treat this surface as activation/view-only runtime context; it does not by itself execute work or transfer root-session write authority"
        }
    };

    serde_json::json!({
        "activation_kind": "activation_view",
        "view_only": true,
        "executes_packet": false,
        "records_completion_receipt": false,
        "transfers_root_session_write_authority": false,
        "root_session_write_guard_remains_authoritative": true,
        "tracked_flow_shaping_only": tracked_flow_shaping_only,
        "next_lawful_action": next_lawful_action,
    })
}

fn agent_init_packet_selection(
    packet_path: &str,
    packet: serde_json::Value,
    downstream: bool,
) -> Result<serde_json::Value, String> {
    let selected_role = packet
        .get("activation_runtime_role")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            packet
                .get("role_selection")
                .and_then(|value| value.get("selected_role"))
                .and_then(serde_json::Value::as_str)
        })
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    if selected_role == "orchestrator" || selected_role == "unknown" {
        return Err(
            "Packet activation requires a non-orchestrator runtime role in the dispatch packet."
                .to_string(),
        );
    }

    let dispatch_target_key = if downstream {
        "downstream_dispatch_target"
    } else {
        "dispatch_target"
    };
    let packet_path_key = if downstream {
        "downstream_packet_path"
    } else {
        "dispatch_packet_path"
    };

    Ok(serde_json::json!({
        "mode": if downstream { "downstream_packet" } else { "dispatch_packet" },
        "selected_role": selected_role,
        "request_text": packet.get("request_text").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "dispatch_target": packet.get(dispatch_target_key).and_then(serde_json::Value::as_str).unwrap_or_default(),
        packet_path_key: packet_path,
        "packet_kind": packet.get("packet_kind").cloned().unwrap_or(serde_json::Value::Null),
        "packet_template_kind": packet.get("packet_template_kind").cloned().unwrap_or(serde_json::Value::Null),
        "packet": packet,
    }))
}

fn recomputed_agent_init_execution_truth(
    selection: &serde_json::Value,
    packet: &serde_json::Value,
) -> Option<serde_json::Value> {
    let role_selection = packet
        .get("role_selection_full")
        .cloned()
        .and_then(|value| {
            serde_json::from_value::<super::RuntimeConsumptionLaneSelection>(value).ok()
        })?;
    let dispatch_target = agent_init_selection_dispatch_target(selection);
    Some(
        super::runtime_dispatch_state::dispatch_execution_route_summary(
            &role_selection,
            dispatch_target,
            packet
                .get("selected_backend")
                .and_then(serde_json::Value::as_str),
            packet
                .get("selected_backend_override")
                .and_then(serde_json::Value::as_str),
        ),
    )
}

fn packet_execution_truth_conflicts_with_runtime_assignment(
    selection: &serde_json::Value,
    packet: &serde_json::Value,
    packet_execution_truth: &serde_json::Value,
) -> bool {
    let Some(role_selection) = packet
        .get("role_selection_full")
        .cloned()
        .and_then(|value| {
            serde_json::from_value::<super::RuntimeConsumptionLaneSelection>(value).ok()
        })
    else {
        return false;
    };
    let dispatch_target = agent_init_selection_dispatch_target(selection);
    let (runtime_assignment, _) = super::runtime_dispatch_state::dispatch_target_runtime_assignment(
        &role_selection.execution_plan,
        dispatch_target,
    );
    let assignment_backend = runtime_assignment
        .get("selected_backend_id")
        .or_else(|| runtime_assignment.get("selected_carrier_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let packet_backend = packet_execution_truth
        .get("effective_selected_backend")
        .or_else(|| packet_execution_truth.get("selected_backend"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    matches!(
        (assignment_backend, packet_backend),
        (Some(assignment), Some(packet)) if assignment != packet
    )
}

fn agent_init_execution_truth(selection: &serde_json::Value) -> serde_json::Value {
    let packet = selection
        .get("packet")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if packet.is_null() {
        return serde_json::Value::Null;
    }

    let recomputed = recomputed_agent_init_execution_truth(selection, &packet);
    let packet_execution_truth = packet.get("execution_truth").cloned();
    match (packet_execution_truth, recomputed) {
        (Some(packet_truth), Some(recomputed_truth))
            if packet_execution_truth_conflicts_with_runtime_assignment(
                selection,
                &packet,
                &packet_truth,
            ) =>
        {
            recomputed_truth
        }
        (Some(packet_truth), _) => packet_truth,
        (None, Some(recomputed_truth)) => recomputed_truth,
        (None, None) => serde_json::Value::Null,
    }
}

fn agent_init_role_selection(
    selection: &serde_json::Value,
) -> Option<super::RuntimeConsumptionLaneSelection> {
    selection
        .get("packet")
        .and_then(|packet| packet.get("role_selection_full"))
        .cloned()
        .or_else(|| {
            selection
                .get("execution_plan")
                .map(|_| selection.clone())
                .filter(|value| value.get("selected_role").is_some())
        })
        .and_then(|value| {
            serde_json::from_value::<super::RuntimeConsumptionLaneSelection>(value).ok()
        })
}

fn task_class_for_runtime_role(runtime_role: &str) -> &'static str {
    match runtime_role {
        "solution_architect" => "architecture",
        "verifier" | "prover" => "verification",
        "coach" => "coach",
        "business_analyst" => "specification",
        _ => "implementation",
    }
}

fn explicit_mode_runtime_assignment(
    selection: &serde_json::Value,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let Some(selected_role) = selection
        .get("selected_role")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return serde_json::Value::Null;
    };
    crate::build_runtime_assignment_from_resolved_constraints(
        activation_bundle,
        "orchestrator",
        task_class_for_runtime_role(selected_role),
        selected_role,
    )
}

fn activation_bundle_has_carrier_runtime_roles(activation_bundle: &serde_json::Value) -> bool {
    crate::carrier_runtime_section(activation_bundle)["roles"]
        .as_array()
        .is_some_and(|roles| !roles.is_empty())
}

fn agent_init_effective_activation_bundle(
    project_root: &Path,
    config_path: &str,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    if activation_bundle_has_carrier_runtime_roles(activation_bundle) {
        return activation_bundle.clone();
    }
    let Ok(config) =
        crate::project_activator_surface::read_yaml_file_checked(Path::new(config_path))
    else {
        return activation_bundle.clone();
    };
    match crate::build_compiled_agent_extension_bundle_for_root(&config, project_root) {
        Ok(projected_bundle) if activation_bundle_has_carrier_runtime_roles(&projected_bundle) => {
            projected_bundle
        }
        _ => {
            let registry = crate::project_activator_surface::host_cli_system_registry_with_fallback(
                Some(&config),
            );
            let selected_host_cli_system = crate::yaml_string(crate::yaml_lookup(
                &config,
                &["host_environment", "cli_system"],
            ))
            .and_then(|system| {
                registry
                    .get(&system)
                    .map(|entry| (system, entry))
                    .or_else(|| None)
            })
            .or_else(|| {
                let mut enabled_entries = registry
                    .iter()
                    .filter(|(_, entry)| {
                        crate::project_activator_surface::host_cli_system_enabled(entry)
                    })
                    .collect::<Vec<_>>();
                enabled_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                enabled_entries
                    .into_iter()
                    .next()
                    .map(|(system, entry)| (system.clone(), entry))
            })
            .or_else(|| {
                let mut entries = registry.iter().collect::<Vec<_>>();
                entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                entries
                    .into_iter()
                    .next()
                    .map(|(system, entry)| (system.clone(), entry))
            });
            let Some((_, selected_entry)) = selected_host_cli_system else {
                return activation_bundle.clone();
            };
            let carrier_roles = crate::project_activator_surface::host_cli_entry_carrier_catalog(
                Some(selected_entry),
            );
            if carrier_roles.is_empty() {
                return activation_bundle.clone();
            }
            let mut effective_bundle = activation_bundle.clone();
            effective_bundle["carrier_runtime"]["roles"] = serde_json::Value::Array(carrier_roles);
            effective_bundle
        }
    }
}

fn task_class_for_dispatch_target(dispatch_target: &str, selected_role: &str) -> &'static str {
    match dispatch_target {
        "specification" | "analysis" => "specification",
        "coach" => "coach",
        "verification" => "verification",
        "execution_preparation" => "architecture",
        "implementer" | "" => task_class_for_runtime_role(selected_role),
        _ => task_class_for_runtime_role(selected_role),
    }
}

fn rebuilt_embedded_runtime_assignment(
    selection: &serde_json::Value,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let selected_role = selection
        .get("selected_role")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(role_selection.selected_role.as_str());
    let dispatch_target = agent_init_selection_dispatch_target(selection);
    let task_class = task_class_for_dispatch_target(dispatch_target, selected_role);
    crate::build_runtime_assignment_from_resolved_constraints(
        activation_bundle,
        &role_selection.selected_role,
        task_class,
        selected_role,
    )
}

fn packet_level_runtime_assignment(
    selection: &serde_json::Value,
) -> Option<(serde_json::Value, &'static str)> {
    let packet = selection.get("packet")?;
    let top_level_assignment = packet
        .get("runtime_assignment")
        .or_else(|| packet.get("carrier_runtime_assignment"))
        .filter(|assignment| !assignment.is_null());
    if let Some(assignment) = top_level_assignment {
        let has_carrier = assignment
            .get("selected_carrier_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        let has_model_profile = assignment
            .get("selected_model_profile_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        let disabled = assignment
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            == Some(false);
        if disabled || (has_carrier && has_model_profile) {
            return Some((assignment.clone(), "packet_runtime_assignment"));
        }
    }

    let dispatch_target = agent_init_selection_dispatch_target(selection);
    let role_selection = agent_init_role_selection(selection)?;
    let (assignment, source) = super::runtime_dispatch_state::dispatch_target_runtime_assignment(
        &role_selection.execution_plan,
        dispatch_target,
    );
    (!assignment.is_null()).then_some((assignment, source))
}

fn agent_init_runtime_assignment_resolution(
    selection: &serde_json::Value,
    activation_bundle: &serde_json::Value,
) -> (serde_json::Value, &'static str) {
    if matches!(
        selection.get("mode").and_then(serde_json::Value::as_str),
        Some("dispatch_packet" | "downstream_packet")
    ) {
        if let Some((runtime_assignment, assignment_source)) =
            packet_level_runtime_assignment(selection)
        {
            return (runtime_assignment, assignment_source);
        }
    }

    if let Some(role_selection) = agent_init_role_selection(selection) {
        let runtime_assignment =
            super::runtime_assignment_from_execution_plan(&role_selection.execution_plan).clone();
        if !runtime_assignment.is_null() {
            return (runtime_assignment, "embedded_runtime_assignment");
        }
        if matches!(
            selection.get("mode").and_then(serde_json::Value::as_str),
            Some(
                "dispatch_packet"
                    | "downstream_packet"
                    | "runtime"
                    | "fixed"
                    | "auto"
                    | "compiled"
                    | "test"
            )
        ) {
            return (
                rebuilt_embedded_runtime_assignment(selection, &role_selection, activation_bundle),
                "rebuilt_legacy_embedded_selection",
            );
        }
    }

    match selection.get("mode").and_then(serde_json::Value::as_str) {
        Some("explicit_role") => (
            explicit_mode_runtime_assignment(selection, activation_bundle),
            "provisional_explicit_role",
        ),
        _ => (serde_json::Value::Null, "none"),
    }
}

fn agent_init_selection_dispatch_target(selection: &serde_json::Value) -> &str {
    selection
        .get("dispatch_target")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            selection
                .get("packet")
                .and_then(|packet| packet.get("dispatch_target"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            selection
                .get("packet")
                .and_then(|packet| packet.get("downstream_dispatch_target"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn agent_init_missing_assignment_blocker(
    selection: &serde_json::Value,
    activation_bundle: &serde_json::Value,
    runtime_assignment: &serde_json::Value,
) -> serde_json::Value {
    let mode = selection
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    if !matches!(
        mode,
        "explicit_role"
            | "runtime"
            | "fixed"
            | "auto"
            | "compiled"
            | "test"
            | "dispatch_packet"
            | "downstream_packet"
    ) {
        return serde_json::Value::Null;
    }

    let carrier_runtime = crate::carrier_runtime_section(activation_bundle);
    let model_selection_enabled = carrier_runtime["model_selection"]["enabled"]
        .as_bool()
        .unwrap_or(false);
    let has_carrier_roles = carrier_runtime["roles"]
        .as_array()
        .is_some_and(|roles| !roles.is_empty());
    let runtime_assignment_expected = model_selection_enabled && has_carrier_roles;
    let has_embedded_selection = agent_init_role_selection(selection).is_some();
    let authoritative = match mode {
        "dispatch_packet" | "downstream_packet" => has_embedded_selection,
        "explicit_role" => false,
        _ => true,
    };
    if matches!(mode, "dispatch_packet" | "downstream_packet") && !has_embedded_selection {
        return serde_json::Value::Null;
    }

    if !runtime_assignment_expected && !authoritative {
        return serde_json::Value::Null;
    }

    let has_carrier = runtime_assignment
        .get("selected_carrier_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .is_some();
    let has_model_profile = runtime_assignment
        .get("selected_model_profile_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .is_some();
    if has_carrier && has_model_profile {
        return serde_json::Value::Null;
    }

    let blocker_code = if runtime_assignment["enabled"].as_bool() == Some(false) {
        "runtime_assignment_unavailable"
    } else {
        "runtime_assignment_truth_required"
    };
    let reason = if runtime_assignment["enabled"].as_bool() == Some(false) {
        runtime_assignment
            .get("reason")
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    } else if !has_carrier {
        serde_json::Value::String("selected_carrier_id_missing".to_string())
    } else {
        serde_json::Value::String("selected_model_profile_id_missing".to_string())
    };

    serde_json::json!({
        "status": if authoritative { "blocked" } else { "advisory" },
        "authoritative": authoritative,
        "warning": !authoritative,
        "blocker_code": blocker_code,
        "reason": reason,
        "mode": mode,
    })
}

fn agent_init_backend_truth(
    selection: &serde_json::Value,
    execution_truth: &serde_json::Value,
    activation_bundle: &serde_json::Value,
) -> serde_json::Value {
    let execution_truth = execution_truth.as_object();
    let role_selection = agent_init_role_selection(selection);
    let (runtime_assignment, assignment_source) =
        agent_init_runtime_assignment_resolution(selection, activation_bundle);
    let assignment_blocker =
        agent_init_missing_assignment_blocker(selection, activation_bundle, &runtime_assignment);
    let selected_backend = execution_truth
        .as_ref()
        .and_then(|truth| {
            truth
                .get("effective_selected_backend")
                .or_else(|| truth.get("selected_backend"))
        })
        .cloned()
        .or_else(|| runtime_assignment.get("selected_backend_id").cloned())
        .unwrap_or(serde_json::Value::Null);
    let route_primary_backend = execution_truth
        .as_ref()
        .and_then(|truth| truth.get("route_primary_backend"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_fallback_backend = execution_truth
        .as_ref()
        .and_then(|truth| truth.get("route_fallback_backend"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_backend_source = execution_truth
        .as_ref()
        .and_then(|truth| truth.get("selected_backend_source"))
        .cloned()
        .or_else(|| runtime_assignment.get("selection_rule").cloned())
        .unwrap_or(serde_json::Value::Null);
    let effective_execution_posture = execution_truth
        .as_ref()
        .and_then(|truth| truth.get("effective_execution_posture"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_backend_class = execution_truth
        .as_ref()
        .and_then(|truth| truth.get("selected_backend_class"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_carrier_id = runtime_assignment
        .get("selected_carrier_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_model_profile_id = runtime_assignment
        .get("selected_model_profile_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let selected_backend_str = selected_backend.as_str().filter(|value| !value.is_empty());
    let route_primary_backend_str = route_primary_backend
        .as_str()
        .filter(|value| !value.is_empty());
    let selected_carrier_str = selected_carrier_id
        .as_str()
        .filter(|value| !value.is_empty());
    let override_active = matches!(
        (selected_backend_str, route_primary_backend_str),
        (Some(selected), Some(primary)) if selected != primary
    );
    let dynamic_carrier_matches_effective_backend =
        match (selected_carrier_str, selected_backend_str) {
            (Some(selected_carrier), Some(effective_backend)) => {
                serde_json::Value::Bool(selected_carrier == effective_backend)
            }
            _ => serde_json::Value::Null,
        };
    let dynamic_carrier_matches_route_primary_backend =
        match (selected_carrier_str, route_primary_backend_str) {
            (Some(selected_carrier), Some(route_primary_backend)) => {
                serde_json::Value::Bool(selected_carrier == route_primary_backend)
            }
            _ => serde_json::Value::Null,
        };

    let lawful_override = if override_active {
        role_selection
            .as_ref()
            .and_then(|role_selection| {
                let dispatch_target = agent_init_selection_dispatch_target(selection);
                selected_backend_str.map(|backend_id| {
                    super::runtime_dispatch_state::backend_is_admissible_for_dispatch_target(
                        &role_selection.execution_plan,
                        backend_id,
                        dispatch_target,
                    )
                })
            })
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Bool(false)
    };
    let override_status = if !override_active {
        "not_needed"
    } else if lawful_override == serde_json::Value::Bool(true) {
        "lawful"
    } else if lawful_override == serde_json::Value::Bool(false) {
        "inadmissible"
    } else {
        "unknown"
    };

    serde_json::json!({
        "selected_backend": selected_backend,
        "selected_backend_source": selected_backend_source,
        "backend_selection_source": selected_backend_source,
        "selected_backend_class": selected_backend_class,
        "selected_carrier_id": selected_carrier_id,
        "selected_model_profile_id": selected_model_profile_id,
        "dynamic_carrier_matches_effective_backend": dynamic_carrier_matches_effective_backend,
        "dynamic_carrier_matches_route_primary_backend": dynamic_carrier_matches_route_primary_backend,
        "route_primary_backend": route_primary_backend,
        "route_fallback_backend": route_fallback_backend,
        "effective_execution_posture": effective_execution_posture,
        "override_active": override_active,
        "lawful_override": lawful_override,
        "override_status": override_status,
        "assignment_source": assignment_source,
        "runtime_assignment": runtime_assignment,
        "assignment_blocker": assignment_blocker,
    })
}

fn build_agent_init_surface_payload(
    project_root: &Path,
    config_path: &str,
    init_view: serde_json::Value,
    selection: serde_json::Value,
    activation_semantics: serde_json::Value,
    dispatch_mode: serde_json::Value,
    runtime_bundle_summary: serde_json::Value,
    activation_bundle: &serde_json::Value,
    dev_team_readiness: serde_json::Value,
) -> serde_json::Value {
    let effective_activation_bundle =
        agent_init_effective_activation_bundle(project_root, config_path, activation_bundle);
    let execution_truth = agent_init_execution_truth(&selection);
    let backend_truth =
        agent_init_backend_truth(&selection, &execution_truth, &effective_activation_bundle);
    let packet_activation_evidence = if activation_semantics["view_only"].as_bool() == Some(true) {
        serde_json::Value::Null
    } else {
        selection
            .get("packet")
            .and_then(|packet| packet.get("activation_evidence"))
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    let dev_team_readiness = enrich_dev_team_readiness_with_agent_selection(
        dev_team_readiness,
        &selection,
        &backend_truth,
    );
    let operator_guidance =
        agent_init_operator_guidance(&selection, &activation_semantics, &dispatch_mode);

    serde_json::json!({
        "surface": "vida agent-init",
        "init": init_view,
        "selection": selection,
        "dispatch_mode": dispatch_mode,
        "activation_semantics": activation_semantics,
        "operator_guidance": operator_guidance,
        "execution_truth": execution_truth,
        "backend_truth": backend_truth,
        "dev_team_readiness": dev_team_readiness,
        "packet_activation_evidence": packet_activation_evidence,
        "runtime_bundle_summary": runtime_bundle_summary,
    })
}

fn enrich_dev_team_readiness_with_agent_selection(
    mut dev_team_readiness: serde_json::Value,
    selection: &serde_json::Value,
    backend_truth: &serde_json::Value,
) -> serde_json::Value {
    let Some(dev_team_object) = dev_team_readiness.as_object_mut() else {
        return dev_team_readiness;
    };
    let selected_runtime_role = selection
        .get("selected_role")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty());
    let selected_dev_team_role = dev_team_object
        .get("roles")
        .and_then(serde_json::Value::as_array)
        .and_then(|roles| {
            roles
                .iter()
                .find(|role| role["runtime_role"].as_str() == selected_runtime_role)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_model = selected_dev_team_role
        .get("selected_model")
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "model_ref": selected_dev_team_role
                    .get("default_model")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            })
        });
    let selected_cost_units = selected_dev_team_role
        .get("cost_policy")
        .and_then(|cost| cost.get("budget_units"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    dev_team_object.insert(
        "active_selection".to_string(),
        serde_json::json!({
            "selected_runtime_role": selected_runtime_role,
            "selected_dev_team_role": selected_dev_team_role.get("role_id").cloned().unwrap_or(serde_json::Value::Null),
            "selected_backend": backend_truth.get("selected_backend").cloned().unwrap_or(serde_json::Value::Null),
            "selected_carrier_id": backend_truth.get("selected_carrier_id").cloned().unwrap_or(serde_json::Value::Null),
            "selected_model_profile_id": backend_truth.get("selected_model_profile_id").cloned().unwrap_or(serde_json::Value::Null),
            "selected_model": selected_model,
            "selected_cost_units": selected_cost_units,
        }),
    );
    dev_team_readiness
}

fn build_fast_agent_init_packet_activation_payload(
    project_root: &Path,
    packet_path: &str,
    packet: serde_json::Value,
    downstream: bool,
    args: &AgentInitArgs,
) -> Result<serde_json::Value, String> {
    let selection = agent_init_packet_selection(packet_path, packet, downstream)?;
    let activation_semantics = agent_init_activation_semantics(&selection);
    let dispatch_mode = agent_init_dispatch_mode(args, &selection);
    Ok(build_agent_init_surface_payload(
        project_root,
        &project_root.join("vida.config.yaml").display().to_string(),
        serde_json::json!({
            "status": "pass",
            "activation_source": "packet_activation_fast_path",
            "view_only": true,
        }),
        selection,
        activation_semantics,
        dispatch_mode,
        serde_json::json!({
            "activation_source": "packet_activation_fast_path",
            "state_dir": serde_json::Value::Null,
            "launcher_runtime_paths": serde_json::Value::Null,
        }),
        &serde_json::Value::Null,
        serde_json::json!({
            "status": "not_required",
            "reason": "packet_activation_view_only_fast_path",
        }),
    ))
}

fn emit_fast_agent_init_packet_activation_payload(surface_payload: &serde_json::Value, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(surface_payload).expect("agent-init json should render")
        );
        return;
    }
    print_surface_header(RenderMode::Plain, "vida agent-init");
    print_surface_line(
        RenderMode::Plain,
        "status",
        surface_payload["init"]["status"]
            .as_str()
            .unwrap_or("unknown"),
    );
    print_surface_line(
        RenderMode::Plain,
        "selected role",
        surface_payload["selection"]["selected_role"]
            .as_str()
            .unwrap_or("unknown"),
    );
    if let Some(mode) = surface_payload["selection"]["mode"].as_str() {
        print_surface_line(RenderMode::Plain, "mode", mode);
    }
    if let Some(mode) = surface_payload["dispatch_mode"]["mode"].as_str() {
        print_surface_line(RenderMode::Plain, "dispatch_mode", mode);
    }
    if let Some(path) = surface_payload["selection"]["dispatch_packet_path"].as_str() {
        print_surface_line(RenderMode::Plain, "dispatch packet", path);
    }
    if let Some(path) = surface_payload["selection"]["downstream_packet_path"].as_str() {
        print_surface_line(RenderMode::Plain, "downstream packet", path);
    }
    print_compact_command_families(RenderMode::Plain, "vida agent-init");
}

pub(crate) async fn render_agent_init_packet_activation_with_store(
    store: &super::StateStore,
    project_root: &Path,
    packet_path: &str,
    downstream: bool,
) -> Result<serde_json::Value, String> {
    let instruction_source_root =
        PathBuf::from(super::state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    let framework_memory_source_root =
        PathBuf::from(super::state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT);
    super::ensure_launcher_bootstrap(
        store,
        &instruction_source_root,
        &framework_memory_source_root,
    )
    .await?;
    let bundle = build_taskflow_consume_bundle_payload(store).await?;
    let packet = super::taskflow_consume_resume::read_dispatch_packet(packet_path)?;
    let selection = agent_init_packet_selection(packet_path, packet, downstream)?;

    let project_activation_view =
        super::project_activator_surface::build_project_activator_view(project_root);
    let init_view = super::project_activator_surface::merge_project_activation_into_init_view(
        bundle.agent_init_view,
        &project_activation_view,
    );
    let activation_semantics = agent_init_activation_semantics(&selection);

    Ok(build_agent_init_surface_payload(
        project_root,
        &bundle.config_path,
        init_view,
        selection,
        activation_semantics,
        serde_json::json!({
            "mode": "packet_activation_view_only",
            "requested_execute_dispatch": false,
            "has_packet_source": true,
            "activation_view_only": true,
            "execution_dispatch": false,
            "execution_evidence_required_for_completion": true,
            "completion_requires_receipt_backed_execution": true,
        }),
        serde_json::json!({
            "bundle_id": bundle.metadata["bundle_id"],
            "activation_source": bundle.activation_source,
            "vida_root": bundle.vida_root,
            "state_dir": store.root().display().to_string(),
            "launcher_runtime_paths": bundle.launcher_runtime_paths,
        }),
        &bundle.activation_bundle,
        super::taskflow_consume_bundle::build_dev_team_readiness(
            &bundle.config_path,
            &bundle.activation_bundle,
        ),
    ))
}

#[cfg(test)]
mod agent_init_surface_tests {
    use super::*;
    use crate::run;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir, EnvVarGuard};
    use crate::RuntimeConsumptionLaneSelection;
    use std::fs;
    use std::path::Path;
    use std::process::ExitCode;
    use std::time::{Duration, Instant};

    fn wait_for_state_unlock(state_dir: &Path) {
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn test_role_selection() -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementer": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "runtime_assignment": {
                    "selected_carrier_id": "junior",
                    "selected_backend_id": "junior",
                    "selected_model_profile_id": "codex_gpt54_mini_impl",
                    "selected_tier": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        }
    }

    fn test_activation_bundle() -> serde_json::Value {
        serde_json::json!({
            "carrier_runtime": {
                "selection_rule": "role_task_then_readiness_then_score_then_cost_quality",
                "model_selection": {
                    "enabled": true,
                    "default_strategy": "balanced_cost_quality",
                    "candidate_scope": "unified_carrier_model_profiles"
                },
                "roles": [
                    {
                        "role_id": "junior",
                        "tier": "junior",
                        "enabled": true,
                        "rate": 1,
                        "normalized_cost_units": 1,
                        "default_runtime_role": "worker",
                        "runtime_roles": ["worker"],
                        "task_classes": ["implementation"],
                        "reasoning_band": "medium",
                        "default_model_profile": "codex_gpt54_mini_impl",
                        "model_profiles": {
                            "codex_gpt54_mini_impl": {
                                "profile_id": "codex_gpt54_mini_impl",
                                "model_ref": "gpt-5.4-mini",
                                "reasoning_effort": "medium",
                                "provider": "openai",
                                "normalized_cost_units": 1,
                                "speed_tier": "fast",
                                "quality_tier": "medium",
                                "write_scope": "workspace-write",
                                "runtime_roles": ["worker"],
                                "task_classes": ["implementation"],
                                "readiness": { "required": true, "ready": true },
                                "budget": {
                                    "max_task_price_units": 10
                                }
                            }
                        }
                    }
                ]
            }
        })
    }

    fn test_project_root() -> &'static Path {
        Path::new("/tmp")
    }

    fn test_config_path() -> &'static str {
        "/tmp/vida.test.config.yaml"
    }

    fn default_agent_init_args() -> AgentInitArgs {
        AgentInitArgs {
            request_text: None,
            role: None,
            dispatch_packet: None,
            downstream_packet: None,
            execute_dispatch: false,
            auto_dispatch_packet: false,
            state_dir: None,
            json: true,
        }
    }

    #[test]
    fn agent_init_auto_dispatch_packet_args_fail_closed() {
        let mut args = default_agent_init_args();
        args.auto_dispatch_packet = true;
        assert_eq!(
            validate_agent_init_auto_dispatch_packet_args(&args, 0)
                .expect_err("auto dispatch without execution must fail"),
            "`--auto-dispatch-packet` is only valid with `--execute-dispatch`."
        );

        args.execute_dispatch = true;
        args.dispatch_packet = Some("/tmp/dispatch.json".to_string());
        assert_eq!(
            validate_agent_init_auto_dispatch_packet_args(&args, 1)
                .expect_err("auto dispatch must not combine with manual packet"),
            "`--auto-dispatch-packet` is exclusive with `--dispatch-packet` and `--downstream-packet`."
        );

        args.dispatch_packet = None;
        args.role = Some("worker".to_string());
        assert_eq!(
            validate_agent_init_auto_dispatch_packet_args(&args, 0)
                .expect_err("auto dispatch must use active bounded runtime unit"),
            "`--auto-dispatch-packet` uses the active bounded runtime unit; do not combine it with `--role` or request text."
        );

        args.role = None;
        args.request_text = Some("implement this".to_string());
        assert_eq!(
            validate_agent_init_auto_dispatch_packet_args(&args, 0)
                .expect_err("auto dispatch must not accept request text"),
            "`--auto-dispatch-packet` uses the active bounded runtime unit; do not combine it with `--role` or request text."
        );

        args.request_text = None;
        validate_agent_init_auto_dispatch_packet_args(&args, 0)
            .expect("execute-only auto dispatch should be admissible");
    }

    #[test]
    fn agent_init_auto_dispatch_packet_mode_is_execution_without_packet_source() {
        let mut args = default_agent_init_args();
        args.execute_dispatch = true;
        args.auto_dispatch_packet = true;

        let dispatch_mode =
            agent_init_dispatch_mode(&args, &serde_json::json!({ "mode": "active_runtime_unit" }));

        assert_eq!(dispatch_mode["mode"], "execution_dispatch");
        assert_eq!(dispatch_mode["requested_execute_dispatch"], true);
        assert_eq!(dispatch_mode["has_packet_source"], false);
        assert_eq!(dispatch_mode["auto_dispatch_packet"], true);
        assert_eq!(dispatch_mode["selection_mode"], "active_runtime_unit");
        assert_eq!(dispatch_mode["activation_view_only"], false);
        assert_eq!(
            dispatch_mode["completion_requires_receipt_backed_execution"],
            true
        );
        assert_eq!(dispatch_mode["root_session_write_authority_granted"], false);
        assert_eq!(dispatch_mode["continuation_authority_granted"], false);
    }

    #[test]
    fn agent_init_auto_dispatch_active_unit_ids_fail_closed_for_stale_lineage() {
        let error = validate_agent_init_auto_dispatch_active_unit_ids(
            vec!["active-task".to_string()],
            vec!["stale-run".to_string()],
            "stale-run",
        )
        .expect_err("stale latest dispatch must not execute");

        assert_eq!(
            error.blocker_code,
            "auto_dispatch_packet_active_unit_mismatch"
        );
        assert_eq!(error.active_task_id.as_deref(), Some("active-task"));
        assert_eq!(error.resolved_run_id, "stale-run");
    }

    #[test]
    fn agent_init_auto_dispatch_active_unit_ids_accept_matching_lineage() {
        validate_agent_init_auto_dispatch_active_unit_ids(
            vec!["active-task".to_string()],
            vec!["active-task".to_string(), "run-123".to_string()],
            "run-123",
        )
        .expect("matching active task lineage should execute");
    }

    #[test]
    fn agent_init_auto_dispatch_active_unit_ids_fail_closed_without_single_active_task() {
        let missing = validate_agent_init_auto_dispatch_active_unit_ids(
            Vec::new(),
            vec!["run-123".to_string()],
            "run-123",
        )
        .expect_err("missing active unit should block auto dispatch");
        assert_eq!(
            missing.blocker_code,
            "auto_dispatch_packet_active_unit_missing"
        );

        let ambiguous = validate_agent_init_auto_dispatch_active_unit_ids(
            vec!["task-a".to_string(), "task-b".to_string()],
            vec!["task-a".to_string()],
            "task-a",
        )
        .expect_err("ambiguous active unit should block auto dispatch");
        assert_eq!(
            ambiguous.blocker_code,
            "auto_dispatch_packet_active_unit_ambiguous"
        );
    }

    #[test]
    fn orchestrator_runtime_contract_exposes_sticky_intent_topology_and_next_action() {
        let contract = build_orchestrator_runtime_contract(
            &serde_json::json!({
                "project_activation": {
                    "activation_pending": false,
                    "normal_work_defaults": {
                        "default_agent_topology": ["junior", "middle", "senior"]
                    }
                }
            }),
            &serde_json::json!({
                "flows": [{"flow_id": "default_delivery"}],
                "roles": [{"role_id": "developer", "runtime_role": "worker"}]
            }),
        );

        assert_eq!(
            contract["sticky_user_execution_intent"]
                ["agent_first_or_parallel_agent_execution_is_sticky"],
            true
        );
        assert_eq!(
            contract["allowed_topology"]["default_agent_topology"],
            serde_json::json!(["junior", "middle", "senior"])
        );
        assert_eq!(
            contract["next_lawful_dispatch_action"]["command"],
            "vida agent dispatch-next --dev-team --json"
        );
        assert_eq!(
            contract["execution_evidence_contract"]["activation_view_is_execution_evidence"],
            false
        );
        assert_eq!(
            contract["execution_evidence_contract"]["delegated_work_completion_requires"],
            "receipt_backed_execution_evidence"
        );
        assert_eq!(
            contract["write_and_continuation_authority_contract"]
                ["root_local_write_allowed_is_blanket_authority"],
            false
        );
        assert_eq!(
            contract["write_and_continuation_authority_contract"]
                ["continuation_binding_is_independent_of_exception_write_scope"],
            true
        );
    }

    #[test]
    fn agent_init_surface_payload_exposes_execution_truth_selected_backend() {
        let role_selection = test_role_selection();
        let selection = agent_init_packet_selection(
            "/tmp/dispatch.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "fix runtime handoff",
                "dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "selected_backend": "internal_subagents",
                "role_selection_full": role_selection,
            }),
            false,
        )
        .expect("packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({
                "mode": "activation_view_only",
                "activation_view_is_execution_evidence": false,
                "required_completion_evidence": "receipt_backed_execution_evidence",
                "root_session_write_authority_granted": false,
                "continuation_authority_granted": false
            }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({
                "status": "ready",
                "roles": [
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "default_model": "gpt-5.4",
                        "cost_policy": {
                            "budget_units": 1
                        }
                    }
                ]
            }),
        );

        assert_eq!(
            payload["execution_truth"]["effective_selected_backend"],
            "internal_subagents"
        );
        assert_eq!(
            payload["backend_truth"]["selected_backend"],
            "internal_subagents"
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "junior");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "codex_gpt54_mini_impl"
        );
        assert_eq!(
            payload["backend_truth"]["backend_selection_source"],
            "dynamic_runtime_selection"
        );
        assert_eq!(
            payload["backend_truth"]["dynamic_carrier_matches_effective_backend"],
            false
        );
        assert_eq!(
            payload["backend_truth"]["dynamic_carrier_matches_route_primary_backend"],
            false
        );
        assert_eq!(
            payload["backend_truth"]["route_primary_backend"],
            "hermes_cli"
        );
        assert_eq!(
            payload["backend_truth"]["route_fallback_backend"],
            "internal_subagents"
        );
        assert_eq!(payload["backend_truth"]["override_active"], true);
        assert_eq!(payload["backend_truth"]["lawful_override"], true);
        assert_eq!(payload["backend_truth"]["override_status"], "lawful");
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_dev_team_role"],
            "developer"
        );
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_carrier_id"],
            "junior"
        );
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_cost_units"],
            1
        );
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_model"]["model_ref"],
            "gpt-5.4"
        );
        assert_eq!(
            payload["operator_guidance"]["current_surface_contract"]["mode"],
            "activation_view_only"
        );
        assert_eq!(
            payload["operator_guidance"]["current_surface_contract"]["executes_packet"],
            false
        );
        assert_eq!(
            payload["operator_guidance"]["flow_distinctions"][0]["stage"],
            "startup_activation_view"
        );
        assert_eq!(
            payload["operator_guidance"]["flow_distinctions"][1]["surface"],
            format!(
                "vida agent-init --dispatch-packet {} --execute-dispatch --json",
                crate::shell_quote("/tmp/dispatch.json")
            )
        );
        assert!(payload["operator_guidance"]["next_lawful_execution_action"]
            .as_str()
            .is_some_and(|value| value.contains("--execute-dispatch")));
    }

    #[test]
    fn agent_init_surface_payload_exposes_route_fallback_backend_for_downstream_packet() {
        let selection = agent_init_packet_selection(
            "/tmp/downstream.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "fix runtime handoff",
                "downstream_dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "role_selection_full": test_role_selection(),
            }),
            true,
        )
        .expect("downstream packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({
                "mode": "activation_view_only",
                "activation_view_is_execution_evidence": false,
                "required_completion_evidence": "receipt_backed_execution_evidence",
                "root_session_write_authority_granted": false,
                "continuation_authority_granted": false
            }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(payload["selection"]["mode"], "downstream_packet");
        assert_eq!(
            payload["execution_truth"]["route_primary_backend"],
            "hermes_cli"
        );
        assert_eq!(
            payload["execution_truth"]["route_fallback_backend"],
            "internal_subagents"
        );
        assert_eq!(payload["backend_truth"]["override_status"], "lawful");
    }

    #[test]
    fn downstream_packet_resume_preserves_source_dispatch_target_as_previous_lane_context() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let packet_path = harness.path().join("downstream-packet.json");
        fs::write(
            &packet_path,
            serde_json::to_string(&serde_json::json!({
                "run_id": "run-duplicate-flow",
                "source_dispatch_target": "implementer",
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_ready": true,
                "downstream_dispatch_blockers": [],
                "downstream_dispatch_status": "packet_ready",
                "downstream_lane_status": "packet_ready",
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "delivery_task_packet": {
                    "goal": "Verify downstream source context",
                    "scope_in": ["downstream packet resume"],
                    "definition_of_done": ["source dispatch target is preserved"],
                    "verification_command": "vida agent-init --downstream-packet packet.json --execute-dispatch --json",
                    "proof_target": "decoded dispatch receipt",
                    "stop_rules": ["stop after decode"],
                    "blocking_question": "Does the downstream packet carry previous-lane context?"
                },
                "activation_runtime_role": "coach",
                "activation_agent_type": "middle",
                "selected_backend": "middle",
                "role_selection_full": test_role_selection(),
                "run_graph_bootstrap": {
                    "run_id": "run-duplicate-flow"
                }
            }))
            .expect("packet should serialize"),
        )
        .expect("packet should write");

        let inputs = resume_inputs_from_downstream_packet_without_store(
            packet_path.to_str().expect("packet path should be utf-8"),
        )
        .expect("downstream packet should decode");

        assert_eq!(inputs.dispatch_receipt.dispatch_target, "coach");
        assert_eq!(
            inputs
                .dispatch_receipt
                .downstream_dispatch_last_target
                .as_deref(),
            Some("implementer")
        );
    }

    #[test]
    fn downstream_agent_init_backend_truth_prefers_downstream_target_over_stale_plan_assignment() {
        let mut role_selection = test_role_selection();
        role_selection.selected_role = "business_analyst".to_string();
        role_selection.execution_plan = serde_json::json!({
            "runtime_assignment": {
                "enabled": false,
                "reason": "no_carrier_declares_runtime_role_and_task_class",
                "runtime_role": "business_analyst",
                "task_class": "verification"
            },
            "development_flow": {
                "dispatch_contract": {
                    "implementer_activation": {
                        "enabled": true,
                        "selected_carrier_id": "junior",
                        "selected_backend_id": "junior",
                        "selected_model_profile_id": "codex_gpt54_mini_impl",
                        "selected_tier": "junior",
                        "activation_agent_type": "junior",
                        "activation_runtime_role": "worker",
                        "runtime_role": "worker",
                        "task_class": "implementation",
                        "selection_rule": "role_task_then_readiness_then_score_then_cost_quality"
                    }
                }
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                },
                {
                    "backend_id": "junior",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                }
            ]
        });
        let selection = agent_init_packet_selection(
            "/tmp/downstream.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "fix runtime handoff",
                "downstream_dispatch_target": "implementer",
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "selected_backend": "internal_subagents",
                "role_selection_full": role_selection,
            }),
            true,
        )
        .expect("downstream packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({
                "mode": "activation_view_only",
                "activation_view_is_execution_evidence": false,
                "required_completion_evidence": "receipt_backed_execution_evidence",
                "root_session_write_authority_granted": false,
                "continuation_authority_granted": false
            }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(payload["selection"]["mode"], "downstream_packet");
        assert_eq!(payload["selection"]["dispatch_target"], "implementer");
        assert_eq!(
            payload["backend_truth"]["assignment_source"],
            "dispatch_contract_implementer_activation"
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "junior");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "codex_gpt54_mini_impl"
        );
        assert_eq!(
            payload["backend_truth"]["runtime_assignment"]["task_class"],
            "implementation"
        );
        assert!(payload["backend_truth"]["assignment_blocker"].is_null());
    }

    #[test]
    fn downstream_agent_init_backend_truth_uses_runtime_assignment_over_stale_selected_backend() {
        let mut role_selection = test_role_selection();
        role_selection.selected_role = "coach".to_string();
        role_selection.execution_plan = serde_json::json!({
            "development_flow": {
                "coach": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "internal_subagents",
                    "fanout_executor_backends": ["internal_subagents"],
                    "carrier_runtime_assignment": {
                        "enabled": true,
                        "selected_backend_id": "pi_cli",
                        "selected_carrier_id": "pi_cli",
                        "selected_model_profile_id": "pi_gpt55_medium_guarded",
                        "activation_agent_type": "pi_cli",
                        "activation_runtime_role": "coach",
                        "task_class": "coach",
                        "runtime_role": "coach"
                    }
                }
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true
                    }
                },
                {
                    "backend_id": "pi_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true
                    }
                }
            ]
        });
        let selection = agent_init_packet_selection(
            "/tmp/downstream.json",
            serde_json::json!({
                "activation_runtime_role": "coach",
                "request_text": "review bounded runtime fix",
                "downstream_dispatch_target": "coach",
                "packet_kind": "runtime_downstream_dispatch_packet",
                "packet_template_kind": "coach_review_packet",
                "selected_backend": "internal_subagents",
                "execution_truth": {
                    "effective_selected_backend": "internal_subagents",
                    "selected_backend_source": "stale_packet_selected_backend",
                    "route_primary_backend": "internal_subagents",
                    "route_fallback_backend": "internal_subagents"
                },
                "role_selection_full": role_selection,
            }),
            true,
        )
        .expect("downstream packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({
                "mode": "activation_view_only",
                "activation_view_is_execution_evidence": false,
                "required_completion_evidence": "receipt_backed_execution_evidence",
                "root_session_write_authority_granted": false,
                "continuation_authority_granted": false
            }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(
            payload["execution_truth"]["effective_selected_backend"],
            "pi_cli"
        );
        assert_eq!(payload["backend_truth"]["selected_backend"], "pi_cli");
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "pi_cli");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "pi_gpt55_medium_guarded"
        );
        assert_eq!(
            payload["backend_truth"]["assignment_source"],
            "route_carrier_runtime_assignment"
        );
    }

    #[test]
    fn agent_init_surface_payload_exposes_explicit_role_cost_policy_truth() {
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            serde_json::json!({
                "mode": "explicit_role",
                "selected_role": "worker",
                "request_text": "repair"
            }),
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({
                "mode": "activation_view_only",
                "activation_view_is_execution_evidence": false,
                "required_completion_evidence": "receipt_backed_execution_evidence",
                "root_session_write_authority_granted": false,
                "continuation_authority_granted": false
            }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({
                "status": "ready",
                "roles": [
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "default_model": "gpt-5.4-mini",
                        "cost_policy": {
                            "budget_units": 1
                        }
                    }
                ]
            }),
        );

        assert!(payload["execution_truth"].is_null());
        assert_eq!(payload["dispatch_mode"]["mode"], "activation_view_only");
        assert_eq!(
            payload["dispatch_mode"]["activation_view_is_execution_evidence"],
            false
        );
        assert_eq!(
            payload["dispatch_mode"]["required_completion_evidence"],
            "receipt_backed_execution_evidence"
        );
        assert_eq!(
            payload["dispatch_mode"]["root_session_write_authority_granted"],
            false
        );
        assert_eq!(
            payload["dispatch_mode"]["continuation_authority_granted"],
            false
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "junior");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "codex_gpt54_mini_impl"
        );
        assert_eq!(payload["backend_truth"]["selected_backend"], "junior");
        assert_eq!(
            payload["backend_truth"]["assignment_source"],
            "provisional_explicit_role"
        );
        assert!(payload["backend_truth"]["assignment_blocker"].is_null());
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_cost_units"],
            1
        );
        assert_eq!(
            payload["operator_guidance"]["current_surface_contract"]["view_only"],
            true
        );
        assert_eq!(
            payload["operator_guidance"]["flow_distinctions"][2]["stage"],
            "receipt_backed_worker_execution"
        );
        assert!(payload["operator_guidance"]["next_lawful_execution_action"]
            .as_str()
            .is_some_and(|value| value.contains("scheduler dispatch packet")));
    }

    #[test]
    fn agent_init_surface_payload_keeps_packet_mode_compatible_without_runtime_assignment() {
        let selection = agent_init_packet_selection(
            "/tmp/dispatch.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "repair",
                "dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet"
            }),
            false,
        )
        .expect("packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "mode": "activation_view_only" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert!(payload["backend_truth"]["selected_carrier_id"].is_null());
        assert!(payload["backend_truth"]["assignment_blocker"].is_null());
    }

    #[test]
    fn agent_init_surface_payload_rebuilds_legacy_embedded_packet_runtime_assignment() {
        let selection = agent_init_packet_selection(
            "/tmp/dispatch.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "repair",
                "dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "fixed",
                    "fallback_role": "orchestrator",
                    "request": "repair",
                    "selected_role": "worker",
                    "conversational_mode": null,
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["repair"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "implementer": {
                                "executor_backend": "junior"
                            }
                        }
                    },
                    "reason": "test"
                }
            }),
            false,
        )
        .expect("packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "mode": "activation_view_only" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(
            payload["backend_truth"]["assignment_source"],
            "rebuilt_legacy_embedded_selection"
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "junior");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "codex_gpt54_mini_impl"
        );
        assert!(payload["backend_truth"]["assignment_blocker"].is_null());
    }

    #[test]
    fn agent_init_surface_payload_rebuilds_from_host_environment_carrier_catalog() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let config_path = harness.path().join("vida.config.yaml");
        fs::write(
            &config_path,
            concat!(
                "project:\n",
                "  id: demo\n",
                "host_environment:\n",
                "  systems:\n",
                "    codex:\n",
                "      enabled: true\n",
                "      runtime_root: .codex\n",
                "      template_root: .codex\n",
                "      carriers:\n",
                "        junior:\n",
                "          orchestration_tier: junior\n",
                "          default_runtime_role: worker\n",
                "          runtime_roles: [worker]\n",
                "          task_classes: [implementation]\n",
                "          budget_cost_units: 1\n",
                "          reasoning_band: medium\n",
                "          model: gpt-5.4-mini\n",
                "          model_provider: openai\n",
                "          model_reasoning_effort: medium\n",
                "          plan_mode_reasoning_effort: medium\n",
                "          sandbox_mode: workspace-write\n",
                "          write_scope: workspace-write\n",
                "agent_extensions:\n",
                "  enabled: false\n",
                "agent_system:\n",
                "  model_selection:\n",
                "    enabled: true\n",
                "    candidate_scope: unified_carrier_model_profiles\n",
                "    default_strategy: balanced_cost_quality\n",
            ),
        )
        .expect("config should write");

        let selection = agent_init_packet_selection(
            "/tmp/dispatch.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "repair",
                "dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "role_selection_full": {
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "fixed",
                    "fallback_role": "orchestrator",
                    "request": "repair",
                    "selected_role": "worker",
                    "conversational_mode": null,
                    "single_task_only": true,
                    "tracked_flow_entry": "dev-pack",
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": ["repair"],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "implementer": {
                                "executor_backend": "junior"
                            }
                        }
                    },
                    "reason": "test"
                }
            }),
            false,
        )
        .expect("packet selection should build");
        let payload = build_agent_init_surface_payload(
            harness.path(),
            config_path.to_str().expect("config path should render"),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "mode": "activation_view_only" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &serde_json::json!({
                "agent_system": {
                    "model_selection": {
                        "enabled": true,
                        "candidate_scope": "unified_carrier_model_profiles",
                        "default_strategy": "balanced_cost_quality"
                    }
                },
                "carrier_runtime": {
                    "model_selection": {
                        "enabled": true,
                        "candidate_scope": "unified_carrier_model_profiles",
                        "default_strategy": "balanced_cost_quality"
                    },
                    "roles": []
                }
            }),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(
            payload["backend_truth"]["assignment_source"],
            "rebuilt_legacy_embedded_selection"
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "junior");
        assert!(payload["backend_truth"]["assignment_blocker"].is_null());
    }

    #[test]
    fn agent_init_surface_payload_prefers_dispatch_target_assignment_over_stale_plan_assignment() {
        let mut role_selection = test_role_selection();
        role_selection.selected_role = "business_analyst".to_string();
        role_selection.execution_plan = serde_json::json!({
            "runtime_assignment": {
                "enabled": false,
                "reason": "no_carrier_declares_runtime_role_and_task_class",
                "runtime_role": "business_analyst",
                "task_class": "verification"
            },
            "development_flow": {
                "dispatch_contract": {
                    "lane_catalog": {
                        "specification": {
                            "activation": {
                                "selected_tier": "middle",
                                "activation_agent_type": "middle",
                                "activation_runtime_role": "business_analyst"
                            },
                            "task_class": "specification",
                            "runtime_role": "business_analyst"
                        }
                    },
                    "specification_activation": {
                        "enabled": true,
                        "selected_carrier_id": "middle",
                        "selected_backend_id": "middle",
                        "selected_model_profile_id": "codex_gpt55_medium_write",
                        "selected_tier": "middle",
                        "activation_agent_type": "middle",
                        "activation_runtime_role": "business_analyst",
                        "runtime_role": "business_analyst",
                        "task_class": "specification",
                        "selection_rule": "role_task_then_readiness_then_score_then_cost_quality"
                    }
                }
            }
        });
        let selection = agent_init_packet_selection(
            "/tmp/dispatch.json",
            serde_json::json!({
                "activation_runtime_role": "business_analyst",
                "request_text": "write docs/product/spec/github-114-design.md",
                "dispatch_target": "specification",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "role_selection_full": role_selection,
            }),
            false,
        )
        .expect("packet selection should build");
        let payload = build_agent_init_surface_payload(
            test_project_root(),
            test_config_path(),
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "mode": "activation_view_only" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            &test_activation_bundle(),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(
            payload["backend_truth"]["assignment_source"],
            "dispatch_contract_specification_activation"
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "middle");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "codex_gpt55_medium_write"
        );
        assert_eq!(
            payload["backend_truth"]["runtime_assignment"]["task_class"],
            "specification"
        );
        assert!(payload["backend_truth"]["assignment_blocker"].is_null());
    }

    #[test]
    fn agent_init_missing_assignment_blocker_is_advisory_for_bare_explicit_role() {
        let explicit_blocker = agent_init_missing_assignment_blocker(
            &serde_json::json!({
                "mode": "explicit_role",
                "selected_role": "worker"
            }),
            &test_activation_bundle(),
            &serde_json::json!({
                "enabled": false,
                "reason": "carrier_runtime_roles_missing"
            }),
        );
        assert_eq!(
            explicit_blocker["blocker_code"],
            "runtime_assignment_unavailable"
        );
        assert_eq!(explicit_blocker["status"], "advisory");
        assert_eq!(explicit_blocker["authoritative"], false);

        let packet_blocker = agent_init_missing_assignment_blocker(
            &serde_json::json!({
                "mode": "dispatch_packet",
                "selected_role": "worker"
            }),
            &test_activation_bundle(),
            &serde_json::Value::Null,
        );
        assert!(packet_blocker.is_null());
    }

    #[test]
    fn agent_init_missing_assignment_blocker_skips_minimal_bootstrap_explicit_role() {
        let blocker = agent_init_missing_assignment_blocker(
            &serde_json::json!({
                "mode": "explicit_role",
                "selected_role": "worker"
            }),
            &serde_json::json!({
                "carrier_runtime": {
                    "model_selection": {
                        "enabled": false
                    },
                    "roles": []
                }
            }),
            &serde_json::json!({
                "enabled": false,
                "reason": "carrier_runtime_roles_missing"
            }),
        );

        assert!(blocker.is_null());
    }

    #[test]
    fn agent_init_missing_assignment_blocker_blocks_authoritative_runtime_mode() {
        let blocker = agent_init_missing_assignment_blocker(
            &serde_json::json!({
                "mode": "runtime",
                "selected_role": "worker"
            }),
            &test_activation_bundle(),
            &serde_json::json!({
                "enabled": false,
                "reason": "carrier_runtime_roles_missing"
            }),
        );

        assert_eq!(blocker["blocker_code"], "runtime_assignment_unavailable");
        assert_eq!(blocker["status"], "blocked");
        assert_eq!(blocker["authoritative"], true);
    }

    #[test]
    fn agent_init_missing_assignment_blocker_blocks_authoritative_embedded_selection() {
        let blocker = agent_init_missing_assignment_blocker(
            &serde_json::json!({
                "mode": "dispatch_packet",
                "selected_role": "worker",
                "packet": {
                    "role_selection_full": {
                        "ok": true,
                        "activation_source": "test",
                        "selection_mode": "fixed",
                        "fallback_role": "orchestrator",
                        "request": "repair",
                        "selected_role": "worker",
                        "conversational_mode": null,
                        "single_task_only": true,
                        "tracked_flow_entry": "dev-pack",
                        "allow_freeform_chat": false,
                        "confidence": "high",
                        "matched_terms": ["repair"],
                        "compiled_bundle": null,
                        "execution_plan": {
                            "development_flow": {
                                "implementer": {
                                    "executor_backend": "junior"
                                }
                            }
                        },
                        "reason": "test"
                    }
                }
            }),
            &test_activation_bundle(),
            &serde_json::Value::Null,
        );

        assert_eq!(blocker["blocker_code"], "runtime_assignment_truth_required");
        assert_eq!(blocker["status"], "blocked");
        assert_eq!(blocker["authoritative"], true);
    }

    #[test]
    fn agent_init_missing_assignment_blocker_blocks_unrebuildable_embedded_selection() {
        let blocker = agent_init_missing_assignment_blocker(
            &serde_json::json!({
                "mode": "dispatch_packet",
                "selected_role": "worker",
                "packet": {
                    "role_selection_full": {
                        "ok": true,
                        "activation_source": "test",
                        "selection_mode": "fixed",
                        "fallback_role": "orchestrator",
                        "request": "repair",
                        "selected_role": "worker",
                        "conversational_mode": null,
                        "single_task_only": true,
                        "tracked_flow_entry": "dev-pack",
                        "allow_freeform_chat": false,
                        "confidence": "high",
                        "matched_terms": ["repair"],
                        "compiled_bundle": null,
                        "execution_plan": {
                            "development_flow": {
                                "implementer": {
                                    "executor_backend": "junior"
                                }
                            }
                        },
                        "reason": "test"
                    }
                }
            }),
            &serde_json::json!({
                "carrier_runtime": {
                    "model_selection": {
                        "enabled": true,
                        "default_strategy": "balanced_cost_quality",
                        "candidate_scope": "unified_carrier_model_profiles"
                    },
                    "roles": []
                }
            }),
            &serde_json::json!({
                "enabled": false,
                "reason": "carrier_runtime_roles_missing"
            }),
        );

        assert_eq!(blocker["blocker_code"], "runtime_assignment_unavailable");
        assert_eq!(blocker["status"], "blocked");
        assert_eq!(blocker["authoritative"], true);
    }

    #[test]
    fn orchestrator_init_succeeds_after_init_scaffold() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&["orchestrator-init", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn orchestrator_init_bundle_timeout_payload_is_json_operator_envelope() {
        let payload = orchestrator_init_bundle_timeout_payload(Path::new(".vida/data/state"));

        assert_eq!(payload["surface"], "vida orchestrator-init");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["degraded"], true);
        assert_eq!(
            payload["blocker_codes"][0],
            "taskflow_consume_bundle_timeout"
        );
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["operator_contracts"]["blocker_codes"][0],
            "taskflow_consume_bundle_timeout"
        );
        assert!(payload["next_actions"][0]
            .as_str()
            .unwrap()
            .contains("vida orchestrator-init --json"));
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["timed_out_surface"],
            "build_taskflow_consume_bundle_payload"
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["timeout_seconds"],
            INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS
        );
    }

    #[test]
    fn agent_init_bundle_timeout_payload_is_json_operator_envelope() {
        let payload = agent_init_bundle_timeout_payload(Path::new(".vida/data/state"));

        assert_eq!(payload["surface"], "vida agent-init");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["degraded"], true);
        assert_eq!(
            payload["blocker_codes"][0],
            "taskflow_consume_bundle_timeout"
        );
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(
            payload["operator_contracts"]["blocker_codes"][0],
            "taskflow_consume_bundle_timeout"
        );
        assert!(payload["next_actions"][0]
            .as_str()
            .unwrap()
            .contains("vida agent-init --json"));
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["timed_out_surface"],
            "build_taskflow_consume_bundle_payload"
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["timeout_seconds"],
            INIT_SURFACE_CONSUME_BUNDLE_PAYLOAD_TIMEOUT_SECONDS
        );
    }

    #[test]
    fn agent_init_dispatch_timeout_operator_envelope_adds_contract_fields() {
        let dispatch_mode = serde_json::json!({
            "mode": "execution_dispatch",
            "execution_dispatch": true
        });
        let payload = agent_init_dispatch_timeout_operator_envelope(
            serde_json::json!({
                "surface": "vida agent-init",
                "status": "blocked",
                "execution_state": "blocked",
                "blocker_code": "internal_dispatch_timeout_without_receipt",
            }),
            &dispatch_mode,
            "run-timeout",
            Some("dispatch-result.json"),
            12,
            Some("deferred reconciliation"),
        );

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["dispatch_mode"]["mode"], "execution_dispatch");
        assert_eq!(
            payload["blocker_codes"][0],
            "internal_dispatch_timeout_without_receipt"
        );
        assert_eq!(payload["operator_contracts"]["status"], "blocked");
        assert_eq!(payload["shared_fields"]["status"], "blocked");
        assert_eq!(payload["artifact_refs"]["run_id"], "run-timeout");
        assert_eq!(
            payload["timeout_reconciliation_warning"],
            "deferred reconciliation"
        );
    }

    #[test]
    fn agent_init_execute_dispatch_resume_error_payload_names_recovery_gate() {
        let payload = agent_init_execute_dispatch_resume_error_payload(
            &serde_json::json!({
                "mode": "dispatch_packet",
                "execution_dispatch": true
            }),
            "Run-graph resume gate denied for `epic-2-run`: recovery_ready is false",
        );

        assert_eq!(payload["surface"], "vida agent-init");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["run_id"], "epic-2-run");
        assert_eq!(payload["blocker_code"], "run_graph_recovery_not_ready");
        assert_eq!(
            payload["operator_contracts"]["blocker_codes"][0],
            "run_graph_recovery_not_ready"
        );
        assert!(payload["next_actions"][0]
            .as_str()
            .expect("first action should render")
            .contains("vida taskflow recovery status epic-2-run --json"));
        assert!(payload["next_actions"][1]
            .as_str()
            .expect("second action should render")
            .contains("recovery_ready=true"));
        assert!(payload["next_actions"][2]
            .as_str()
            .expect("third action should render")
            .contains("vida taskflow route explain --json"));
    }

    #[test]
    fn agent_init_resume_gate_error_payload_preserves_existing_blocked_dispatch_evidence() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "epic-2-run".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init --execute-dispatch --json".to_string()),
            dispatch_packet_path: Some("dispatch-packet.json".to_string()),
            dispatch_result_path: Some("dispatch-result.json".to_string()),
            blocker_code: Some("internal_codex_carrier_unavailable".to_string()),
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("senior".to_string()),
            activation_runtime_role: Some("verifier".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-05-21T00:00:00Z".to_string(),
        };
        let payload = agent_init_execute_dispatch_resume_error_payload_with_receipt_evidence(
            &serde_json::json!({
                "mode": "dispatch_packet",
                "execution_dispatch": true
            }),
            "Run-graph resume gate denied for `epic-2-run`: recovery_ready is false",
            Some(&receipt),
            Some(&serde_json::json!({
                "activation_vs_execution_evidence": {
                    "evidence_state": "activation_view_only",
                    "receipt_backed": false
                }
            })),
        );

        assert_eq!(payload["blocker_code"], "run_graph_recovery_not_ready");
        assert_eq!(
            payload["underlying_dispatch_blocker_code"],
            "internal_codex_carrier_unavailable"
        );
        assert_eq!(payload["dispatch_result_path"], "dispatch-result.json");
        assert_eq!(payload["receipt_status"], "blocked");
        assert_eq!(payload["receipt_path"], "dispatch-result.json");
        assert_eq!(
            payload["lane_execution_receipt_path"],
            "dispatch-result.json"
        );
        assert_eq!(
            payload["artifact_refs"]["dispatch_result_path"],
            "dispatch-result.json"
        );
        assert_eq!(
            payload["operator_contracts"]["artifact_refs"]["dispatch_result_path"],
            "dispatch-result.json"
        );
        assert_eq!(
            payload["activation_evidence"]["evidence_state"],
            "activation_view_only"
        );
        assert_eq!(
            payload["activation_vs_execution_evidence"]["evidence_state"],
            "activation_view_only"
        );
    }

    #[test]
    fn agent_init_packet_execute_command_quotes_shell_unsafe_dispatch_packet_path() {
        let unsafe_path = "/tmp/packet' ; echo injected ; #.json";
        let selection = serde_json::json!({
            "dispatch_packet_path": unsafe_path,
        });

        let command = agent_init_packet_execute_command(&selection)
            .expect("dispatch packet command should render");

        assert_eq!(
            command,
            format!(
                "vida agent-init --dispatch-packet {} --execute-dispatch --json",
                crate::shell_quote(unsafe_path)
            )
        );
        assert!(!command.contains("--dispatch-packet '/tmp/packet' ; echo injected ; #.json'"));
    }

    #[test]
    fn agent_init_packet_execute_command_quotes_shell_unsafe_downstream_packet_path() {
        let unsafe_path = "/tmp/downstream' ; echo injected ; #.json";
        let selection = serde_json::json!({
            "downstream_packet_path": unsafe_path,
        });

        let command = agent_init_packet_execute_command(&selection)
            .expect("downstream packet command should render");

        assert_eq!(
            command,
            format!(
                "vida agent-init --downstream-packet {} --execute-dispatch --json",
                crate::shell_quote(unsafe_path)
            )
        );
        assert!(
            !command.contains("--downstream-packet '/tmp/downstream' ; echo injected ; #.json'")
        );
    }

    #[test]
    fn agent_init_succeeds_after_init_scaffold() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&["agent-init", "--role", "worker", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn parallel_agent_init_role_views_do_not_contend_on_write_open() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        let _state_dir_env = EnvVarGuard::unset("VIDA_STATE_DIR");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let results = runtime.block_on(async {
            let handles = (0..4)
                .map(|_| tokio::spawn(run(cli(&["agent-init", "--role", "worker", "--json"]))))
                .collect::<Vec<_>>();
            let mut results = Vec::with_capacity(handles.len());
            for handle in handles {
                results.push(handle.await.expect("agent-init task should not panic"));
            }
            results
        });

        assert_eq!(results, vec![ExitCode::SUCCESS; 4]);
    }
}
