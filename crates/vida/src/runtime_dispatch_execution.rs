use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

use crate::runtime_contract_vocab::canonical_dispatch_target_name;
use crate::runtime_lane_summary::summarize_execution_truth_for_route;
use crate::{yaml_lookup, RuntimeConsumptionLaneSelection, StateStore};

fn canonical_dispatch_target_for_admissibility(dispatch_target: &str) -> String {
    match canonical_dispatch_target_name(dispatch_target).as_str() {
        "implementer" => "implementation".to_string(),
        "execution_preparation" => "architecture".to_string(),
        other => other.to_string(),
    }
}

fn dispatch_target_requires_strict_admissibility(dispatch_target: &str) -> bool {
    matches!(
        canonical_dispatch_target_for_admissibility(dispatch_target).as_str(),
        "implementation" | "architecture"
    )
}

/// Check whether a backend is admissible for a given dispatch target (lane).
/// When no admissibility matrix is present, keep fail-open behavior for backward
/// compatibility. Once a matrix exists, write-producing lanes fail closed if the
/// backend row, lane mapping, or canonical lane key is missing.
fn backend_is_admissible_for_dispatch_target(
    execution_plan: &serde_json::Value,
    backend_id: &str,
    dispatch_target: &str,
) -> bool {
    let canonical_target = canonical_dispatch_target_for_admissibility(dispatch_target);
    let strict_required = dispatch_target_requires_strict_admissibility(dispatch_target);
    let Some(matrix) = execution_plan["backend_admissibility_matrix"].as_array() else {
        return !strict_required;
    };
    let Some(row) = matrix
        .iter()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))
    else {
        return !strict_required;
    };
    let Some(lane_admissibility) = row["lane_admissibility"].as_object() else {
        return !strict_required;
    };
    lane_admissibility
        .get(canonical_target.as_str())
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(!strict_required)
}

fn execution_plan_backend_class(
    role_selection: &RuntimeConsumptionLaneSelection,
    backend_id: &str,
) -> Option<String> {
    role_selection.execution_plan["backend_admissibility_matrix"]
        .as_array()?
        .iter()
        .find(|entry| entry["backend_id"].as_str() == Some(backend_id))?
        .get("backend_class")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn configured_backend_class(
    overlay: Option<&serde_yaml::Value>,
    backend_id: &str,
) -> Option<String> {
    let entry =
        overlay.and_then(|overlay| configured_subagent_backend_entry(overlay, backend_id))?;
    crate::yaml_string(yaml_lookup(entry, &["subagent_backend_class"]))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn backend_is_internal_host_bridge(
    role_selection: &RuntimeConsumptionLaneSelection,
    overlay: Option<&serde_yaml::Value>,
    backend_id: &str,
) -> bool {
    execution_plan_backend_class(role_selection, backend_id)
        .or_else(|| configured_backend_class(overlay, backend_id))
        .as_deref()
        .is_some_and(|backend_class| matches!(backend_class, "internal" | "internal_cli"))
}

fn backend_is_external_cli_bridge(
    role_selection: &RuntimeConsumptionLaneSelection,
    overlay: Option<&serde_yaml::Value>,
    backend_id: &str,
) -> bool {
    execution_plan_backend_class(role_selection, backend_id)
        .or_else(|| configured_backend_class(overlay, backend_id))
        .as_deref()
        .is_some_and(|backend_class| backend_class == "external_cli")
}

fn default_activation_view(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    serde_json::json!({
        "selection": {
            "mode": "dispatch_packet",
            "selected_role": receipt
                .activation_runtime_role
                .as_deref()
                .unwrap_or(&role_selection.selected_role),
        },
        "activation_semantics": {
            "activation_kind": "activation_view",
            "view_only": true,
        },
    })
}

const DEFAULT_DISPATCH_TIMEOUT_KILL_AFTER_GRACE_SECONDS: u64 = 1;
const DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS: u64 = 2;

async fn bounded_activation_view(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
) -> serde_json::Value {
    let open_store = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await;
    let Ok(Ok(store)) = open_store else {
        return default_activation_view(receipt, role_selection);
    };

    let rendered = tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_ACTIVATION_VIEW_RENDER_TIMEOUT_SECONDS),
        crate::init_surfaces::render_agent_init_packet_activation_with_store(
            &store,
            project_root,
            dispatch_packet_path,
            dispatch_packet_path_should_render_as_downstream(dispatch_packet_path),
        ),
    )
    .await;
    drop(store);

    match rendered {
        Ok(Ok(view)) => view,
        _ => default_activation_view(receipt, role_selection),
    }
}

fn dispatch_packet_path_should_render_as_downstream(dispatch_packet_path: &str) -> bool {
    let Ok(body) = std::fs::read_to_string(dispatch_packet_path) else {
        return false;
    };
    let Ok(packet) = serde_json::from_str::<serde_json::Value>(&body) else {
        return false;
    };
    packet["packet_kind"].as_str() == Some("runtime_downstream_dispatch_packet")
        || packet["downstream_dispatch_target"]
            .as_str()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn readiness_fallback_internal_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    blocked_backend_id: &str,
) -> Option<String> {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        dispatch_target,
    )?;
    let fallback_backend = crate::taskflow_routing::fallback_executor_backend_from_route(route)?
        .trim()
        .to_string();
    if fallback_backend.is_empty()
        || fallback_backend == blocked_backend_id
        || !backend_is_internal_host_bridge(role_selection, None, &fallback_backend)
    {
        return None;
    }
    backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        &fallback_backend,
        dispatch_target,
    )
    .then_some(fallback_backend)
}

fn push_unique_backend_candidate(candidates: &mut Vec<String>, candidate: Option<String>) {
    let Some(candidate) = candidate
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

pub(crate) fn internal_codex_external_fallback_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    blocked_backend_id: &str,
    overlay: &serde_yaml::Value,
) -> Option<String> {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        dispatch_target,
    )?;
    let mut candidates = Vec::new();
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::fallback_executor_backend_from_route(route),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::runtime_assignment_backend_for_route(
            &role_selection.execution_plan,
            route,
        ),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::route_primary_backend_hint_from_route(route),
    );
    for candidate in crate::taskflow_routing::fanout_executor_backends_from_route(route) {
        push_unique_backend_candidate(&mut candidates, Some(candidate));
    }

    candidates.into_iter().find(|candidate| {
        if candidate == blocked_backend_id {
            return false;
        }
        if !backend_is_external_cli_bridge(role_selection, Some(overlay), candidate) {
            return false;
        }
        if !backend_is_admissible_for_dispatch_target(
            &role_selection.execution_plan,
            candidate,
            dispatch_target,
        ) {
            return false;
        }
        let Some(backend_entry) =
            crate::runtime_dispatch_state::configured_external_backend_entry(overlay, candidate)
        else {
            return false;
        };
        if crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            candidate,
            backend_entry,
        )
        .is_some()
        {
            return false;
        }
        let selected_model_profile_id =
            crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
                role_selection,
                dispatch_target,
                Some(candidate),
            );
        let readiness =
            crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
                candidate,
                backend_entry,
                selected_model_profile_id.as_deref(),
            );
        !readiness["blocked"].as_bool().unwrap_or(false)
    })
}

fn ready_external_readiness_fallback_backend(
    role_selection: &RuntimeConsumptionLaneSelection,
    dispatch_target: &str,
    blocked_backend_id: &str,
    overlay: &serde_yaml::Value,
    inherited_selected_backend: Option<&str>,
) -> Option<String> {
    let route = crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
        &role_selection.execution_plan,
        dispatch_target,
    )?;
    let mut candidates = Vec::new();
    push_unique_backend_candidate(
        &mut candidates,
        inherited_selected_backend.map(str::to_string),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::runtime_assignment_backend_for_route(
            &role_selection.execution_plan,
            route,
        ),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::route_primary_backend_hint_from_route(route),
    );
    push_unique_backend_candidate(
        &mut candidates,
        crate::taskflow_routing::fallback_executor_backend_from_route(route),
    );
    for candidate in crate::taskflow_routing::fanout_executor_backends_from_route(route) {
        push_unique_backend_candidate(&mut candidates, Some(candidate));
    }

    candidates.into_iter().find(|candidate| {
        if candidate == blocked_backend_id {
            return false;
        }
        if !backend_is_external_cli_bridge(role_selection, Some(overlay), candidate) {
            return false;
        }
        if !backend_is_admissible_for_dispatch_target(
            &role_selection.execution_plan,
            candidate,
            dispatch_target,
        ) {
            return false;
        }
        let Some(backend_entry) =
            crate::runtime_dispatch_state::configured_external_backend_entry(overlay, candidate)
        else {
            return false;
        };
        if crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            candidate,
            backend_entry,
        )
        .is_some()
        {
            return false;
        }
        let selected_model_profile_id =
            crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
                role_selection,
                dispatch_target,
                Some(candidate),
            );
        let readiness =
            crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
                candidate,
                backend_entry,
                selected_model_profile_id.as_deref(),
            );
        !readiness["blocked"].as_bool().unwrap_or(false)
    })
}

fn configured_external_dispatch_wall_timeout_seconds(
    backend_entry: &serde_yaml::Value,
) -> Option<u64> {
    let dispatch = yaml_lookup(backend_entry, &["dispatch"])?;
    yaml_lookup(backend_entry, &["max_runtime_seconds"])
        .and_then(serde_yaml::Value::as_u64)
        .or_else(|| {
            yaml_lookup(dispatch, &["no_output_timeout_seconds"])
                .and_then(serde_yaml::Value::as_u64)
        })
        .filter(|seconds| *seconds > 0)
}

fn configured_internal_host_dispatch_wall_timeout_seconds(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> u64 {
    crate::runtime_dispatch_state::internal_host_runtime_window_seconds(
        project_root,
        role_selection,
        receipt,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandTimeoutWrapper {
    timeout_seconds: u64,
    kill_after_grace_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WrappedCommand {
    command: String,
    args: Vec<String>,
    timeout_wrapper: Option<CommandTimeoutWrapper>,
}

#[derive(Debug)]
struct ObservedCommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

#[cfg(test)]
fn test_exit_status(code: i32) -> ExitStatus {
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
}

#[cfg(test)]
fn emulated_test_shell_output(wrapped_command: &WrappedCommand) -> Option<ObservedCommandOutput> {
    if matches!(
        wrapped_command.command.as_str(),
        "qwen" | "hermes" | "opencode"
    ) {
        let stdout = serde_json::json!({
            "type": "result",
            "result": format!("external-dispatch:{}", wrapped_command.args.join(" ")),
            "is_error": false
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(0),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    let all_args = wrapped_command.args.join(" ");
    if all_args.contains("sleep 30") || all_args.contains("trap") {
        return Some(ObservedCommandOutput {
            status: test_exit_status(124),
            stdout: Vec::new(),
            stderr: b"test shell command timed out".to_vec(),
            timed_out: true,
        });
    }
    if wrapped_command.command != "sh" {
        return None;
    }
    let script = wrapped_command
        .args
        .windows(2)
        .find_map(|pair| (pair[0] == "-lc").then(|| pair[1].as_str()))
        .unwrap_or_default();
    if script.contains("sleep 30") || script.contains("trap") {
        return Some(ObservedCommandOutput {
            status: test_exit_status(124),
            stdout: Vec::new(),
            stderr: b"test shell command timed out".to_vec(),
            timed_out: true,
        });
    }
    if script.contains("external-dispatch:%s") {
        let prompt_args = wrapped_command
            .args
            .iter()
            .position(|arg| arg == "vida-dispatch")
            .map(|index| wrapped_command.args[index + 1..].to_vec())
            .unwrap_or_default();
        let rendered = if script.contains("\"$*\"") {
            prompt_args.join(" ")
        } else {
            prompt_args.first().cloned().unwrap_or_default()
        };
        let stdout = serde_json::json!({
            "type": "result",
            "result": format!("external-dispatch:{rendered}"),
            "is_error": false
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(0),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    if script.contains("input=$(cat)") {
        let stdout = serde_json::json!({
            "type": "result",
            "result": "STDIN_OK",
            "is_error": false
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(0),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    if script.contains("adapter boom") {
        let stdout = serde_json::json!({
            "type": "result",
            "subtype": "error_during_execution",
            "is_error": true,
            "error": {
                "message": "adapter boom"
            }
        })
        .to_string()
        .into_bytes();
        return Some(ObservedCommandOutput {
            status: test_exit_status(1),
            stdout,
            stderr: Vec::new(),
            timed_out: false,
        });
    }
    None
}

#[derive(Debug)]
enum TimeoutProgress {
    WaitingForDeadline(Instant),
    WaitingForKill(Instant),
    TimedOut,
}

#[cfg(unix)]
fn signal_process_group(process_group_id: u32, signal: libc::c_int) -> Result<(), String> {
    let result = unsafe { libc::killpg(process_group_id as libc::pid_t, signal) };
    if result == 0 {
        return Ok(());
    }

    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(code) if code == libc::ESRCH => Ok(()),
        _ => Err(format!(
            "failed to signal process group {process_group_id} with signal {signal}: {error}"
        )),
    }
}

fn spawn_reader_thread<T>(stream: Option<T>) -> std::thread::JoinHandle<Vec<u8>>
where
    T: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = stream {
            let _ = stream.read_to_end(&mut bytes);
        }
        bytes
    })
}

fn try_complete_reader(
    slot: &mut Option<Vec<u8>>,
    receiver: &mpsc::Receiver<Vec<u8>>,
) -> Result<(), String> {
    if slot.is_some() {
        return Ok(());
    }

    match receiver.try_recv() {
        Ok(bytes) => {
            *slot = Some(bytes);
            Ok(())
        }
        Err(TryRecvError::Empty) => Ok(()),
        Err(TryRecvError::Disconnected) => Err("command output reader disconnected".to_string()),
    }
}

fn execute_wrapped_command(
    mut process: std::process::Command,
    wrapped_command: &WrappedCommand,
    stdin_payload: Option<Vec<u8>>,
) -> Result<ObservedCommandOutput, String> {
    process
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if stdin_payload.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    #[cfg(unix)]
    if wrapped_command.timeout_wrapper.is_some() {
        process.process_group(0);
    }

    let mut child = process
        .spawn()
        .map_err(|error| format!("spawn failed for `{}`: {error}", wrapped_command.command))?;
    if let Some(bytes) = stdin_payload {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&bytes).map_err(|error| {
                format!(
                    "failed to write stdin for `{}`: {error}",
                    wrapped_command.command
                )
            })?;
        }
    }
    #[cfg(unix)]
    let process_group_id = child.id();
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();
    let (stdout_tx, stdout_rx) = mpsc::channel();
    let (stderr_tx, stderr_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = stdout_tx.send(spawn_reader_thread(child_stdout).join().unwrap_or_default());
    });
    std::thread::spawn(move || {
        let _ = stderr_tx.send(spawn_reader_thread(child_stderr).join().unwrap_or_default());
    });

    let mut status = None;
    let mut stdout = None;
    let mut stderr = None;
    let mut timed_out = false;
    let mut timeout_progress = wrapped_command.timeout_wrapper.as_ref().map(|wrapper| {
        TimeoutProgress::WaitingForDeadline(
            Instant::now() + Duration::from_secs(wrapper.timeout_seconds),
        )
    });

    loop {
        if status.is_none() {
            status = child.try_wait().map_err(|error| {
                format!("failed to wait on `{}`: {error}", wrapped_command.command)
            })?;
        }
        try_complete_reader(&mut stdout, &stdout_rx)?;
        try_complete_reader(&mut stderr, &stderr_rx)?;

        if status.is_some() && stdout.is_some() && stderr.is_some() {
            return Ok(ObservedCommandOutput {
                status: status.expect("status checked above"),
                stdout: stdout.take().expect("stdout checked above"),
                stderr: stderr.take().expect("stderr checked above"),
                timed_out,
            });
        }
        match timeout_progress.take() {
            Some(TimeoutProgress::WaitingForDeadline(deadline)) => {
                if Instant::now() >= deadline {
                    #[cfg(unix)]
                    signal_process_group(process_group_id, libc::SIGTERM)?;
                    #[cfg(not(unix))]
                    child
                        .kill()
                        .map_err(|error| format!("failed to kill timed out process: {error}"))?;
                    timed_out = true;
                    let kill_deadline = Instant::now()
                        + Duration::from_secs(
                            wrapped_command
                                .timeout_wrapper
                                .as_ref()
                                .map(|wrapper| wrapper.kill_after_grace_seconds)
                                .unwrap_or_default(),
                        );
                    timeout_progress = Some(TimeoutProgress::WaitingForKill(kill_deadline));
                } else {
                    timeout_progress = Some(TimeoutProgress::WaitingForDeadline(deadline));
                }
            }
            Some(TimeoutProgress::WaitingForKill(kill_deadline)) => {
                if Instant::now() >= kill_deadline {
                    #[cfg(unix)]
                    signal_process_group(process_group_id, libc::SIGKILL)?;
                    timeout_progress = Some(TimeoutProgress::TimedOut);
                } else {
                    timeout_progress = Some(TimeoutProgress::WaitingForKill(kill_deadline));
                }
            }
            Some(TimeoutProgress::TimedOut) => {
                return Ok(ObservedCommandOutput {
                    status: synthetic_timeout_exit_status(),
                    stdout: stdout.take().unwrap_or_default(),
                    stderr: stderr.take().unwrap_or_default(),
                    timed_out: true,
                });
            }
            None => {}
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

fn internal_host_activation_only_blocker_code(
    project_root: &Path,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    timed_out: bool,
) -> String {
    if timed_out {
        crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT.to_string()
    } else {
        crate::runtime_dispatch_state::internal_host_activation_view_only_blocker_code(
            project_root,
            role_selection,
            receipt,
        )
        .to_string()
    }
}

#[cfg(unix)]
fn synthetic_timeout_exit_status() -> ExitStatus {
    ExitStatus::from_raw(libc::SIGKILL)
}

#[cfg(not(unix))]
fn synthetic_timeout_exit_status() -> ExitStatus {
    synthetic_timeout_exit_status_non_unix()
}

#[cfg(windows)]
fn synthetic_timeout_exit_status_non_unix() -> ExitStatus {
    ExitStatus::from_raw(124)
}

#[cfg(all(not(unix), not(windows)))]
fn synthetic_timeout_exit_status_non_unix() -> ExitStatus {
    panic!("synthetic timeout exit status is unsupported on this platform")
}

async fn execute_wrapped_command_async(
    process: std::process::Command,
    wrapped_command: WrappedCommand,
    stdin_payload: Option<Vec<u8>>,
) -> Result<ObservedCommandOutput, String> {
    tokio::task::spawn_blocking(move || {
        execute_wrapped_command(process, &wrapped_command, stdin_payload)
    })
    .await
    .map_err(|error| format!("wrapped command task join failed: {error}"))?
}

fn wrap_command_with_optional_timeout(
    command: String,
    args: Vec<String>,
    timeout_seconds: Option<u64>,
) -> WrappedCommand {
    if let Some(timeout_seconds) = timeout_seconds.filter(|seconds| *seconds > 0) {
        let kill_after_grace_seconds =
            DEFAULT_DISPATCH_TIMEOUT_KILL_AFTER_GRACE_SECONDS.min(timeout_seconds.max(1));
        WrappedCommand {
            command,
            args,
            timeout_wrapper: Some(CommandTimeoutWrapper {
                timeout_seconds,
                kill_after_grace_seconds,
            }),
        }
    } else {
        WrappedCommand {
            command,
            args,
            timeout_wrapper: None,
        }
    }
}

#[derive(Debug)]
struct ParsedExternalProviderOutput {
    raw_json: serde_json::Value,
    result_text: Option<String>,
    usage: Option<serde_json::Value>,
    is_error: Option<bool>,
    error_message: Option<String>,
}

fn external_provider_output_indicates_error(output: &ParsedExternalProviderOutput) -> bool {
    if output.is_error.unwrap_or(false) {
        return true;
    }

    if external_provider_scope_guard_indicates_violation(&output.raw_json) {
        return true;
    }

    if external_provider_result_text_declares_blocker(output) {
        return true;
    }

    if output
        .error_message
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return true;
    }

    if output.is_error == Some(false)
        && output
            .raw_json
            .pointer("/raw_provider/provider")
            .and_then(serde_json::Value::as_str)
            == Some("pi")
        && output
            .raw_json
            .pointer("/raw_provider/terminal_event")
            .and_then(serde_json::Value::as_str)
            == Some("agent_end")
    {
        return false;
    }

    let Some(result_text) = output.result_text.as_ref() else {
        return false;
    };

    let normalized = result_text.trim().to_ascii_lowercase();
    if normalized.starts_with('[') && normalized.ends_with(']') {
        return normalized.contains("error") || normalized.contains("exception");
    }

    [
        "quota exceeded",
        "daily quota has been reached",
        "oauth quota exceeded",
        "auth failure",
        "authentication failed",
        "unauthorized",
        "invalid access token",
        "token expired",
        "invalid api key",
        "rate limit exceeded",
        "too many requests",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn external_provider_result_text_declares_blocker(output: &ParsedExternalProviderOutput) -> bool {
    output
        .result_text
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|normalized| {
            normalized.contains("dispatch blocked")
                || normalized.contains("blocked by vida pi write-scope guard")
                || normalized.contains("no execution receipt")
                || normalized.contains("no execution receipt/result artifact")
                || normalized.contains("refused in bash guarded-write mode")
        })
}

fn external_provider_scope_guard_indicates_violation(raw_json: &serde_json::Value) -> bool {
    external_provider_scope_guard(raw_json).is_some_and(|scope_guard| {
        scope_guard
            .get("valid")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
            || scope_guard
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| {
                    matches!(
                        status.trim().to_ascii_lowercase().as_str(),
                        "violation"
                            | "scope_violation"
                            | "owned_path_invalid"
                            | "missing_owned_paths"
                    )
                })
    })
}

fn external_provider_scope_guard(raw_json: &serde_json::Value) -> Option<&serde_json::Value> {
    match raw_json {
        serde_json::Value::Object(_) => raw_json.get("scope_guard"),
        serde_json::Value::Array(rows) => rows.iter().rev().find_map(|row| row.get("scope_guard")),
        _ => None,
    }
}

fn external_provider_reported_paths(raw_json: &serde_json::Value) -> Option<serde_json::Value> {
    let mut touched_paths = std::collections::BTreeSet::new();
    let mut changed_files = std::collections::BTreeSet::new();
    collect_external_provider_reported_paths(raw_json, &mut touched_paths, &mut changed_files);
    let mut body = serde_json::Map::new();
    if !touched_paths.is_empty() {
        body.insert(
            "touched_paths".to_string(),
            serde_json::json!(touched_paths.into_iter().collect::<Vec<_>>()),
        );
    }
    if !changed_files.is_empty() {
        body.insert(
            "changed_files".to_string(),
            serde_json::json!(changed_files.into_iter().collect::<Vec<_>>()),
        );
    }
    if body.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(body))
    }
}

fn collect_external_provider_reported_paths(
    raw_json: &serde_json::Value,
    touched_paths: &mut std::collections::BTreeSet<String>,
    changed_files: &mut std::collections::BTreeSet<String>,
) {
    match raw_json {
        serde_json::Value::Object(entries) => {
            if let Some(paths) = entries.get("touched_paths") {
                collect_external_provider_path_values(paths, touched_paths);
            }
            if let Some(paths) = entries.get("changed_files") {
                collect_external_provider_path_values(paths, changed_files);
            }
            for value in entries.values() {
                collect_external_provider_reported_paths(value, touched_paths, changed_files);
            }
        }
        serde_json::Value::Array(rows) => {
            for row in rows {
                collect_external_provider_reported_paths(row, touched_paths, changed_files);
            }
        }
        _ => {}
    }
}

fn collect_external_provider_path_values(
    value: &serde_json::Value,
    paths: &mut std::collections::BTreeSet<String>,
) {
    match value {
        serde_json::Value::String(path) => {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                paths.insert(trimmed.to_string());
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_external_provider_path_values(value, paths);
            }
        }
        serde_json::Value::Object(entries) => {
            for key in ["path", "file", "filename"] {
                if let Some(path) = entries.get(key).and_then(serde_json::Value::as_str) {
                    let trimmed = path.trim();
                    if !trimmed.is_empty() {
                        paths.insert(trimmed.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

fn external_provider_output_confirms_execution(
    output: Option<&ParsedExternalProviderOutput>,
) -> bool {
    output.is_some_and(|parsed| !external_provider_output_indicates_error(parsed))
}

fn external_provider_error_message(output: &ParsedExternalProviderOutput) -> Option<String> {
    if output
        .error_message
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return output.error_message.clone();
    }

    if output
        .result_text
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return output.result_text.clone();
    }

    None
}

fn external_provider_output_indicates_agent_end_timeout(
    output: &ParsedExternalProviderOutput,
) -> bool {
    let provider_is_pi = output
        .raw_json
        .pointer("/raw_provider/provider")
        .and_then(serde_json::Value::as_str)
        == Some("pi");
    if !provider_is_pi {
        return false;
    }
    let returned_agent_end = output
        .raw_json
        .pointer("/raw_provider/terminal_event")
        .and_then(serde_json::Value::as_str)
        == Some("agent_end");
    if returned_agent_end {
        return false;
    }
    external_provider_error_message(output)
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .is_some_and(|message| message.contains("timed out waiting for pi agent_end event"))
}

fn parse_external_provider_output(stdout: &str) -> Option<ParsedExternalProviderOutput> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
    let raw_json = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
        parsed
    } else {
        let parsed_lines = trimmed
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>();
        match parsed_lines {
            Ok(rows) if !rows.is_empty() => serde_json::Value::Array(rows),
            _ => return None,
        }
    };
    let result_row = match &raw_json {
        serde_json::Value::Array(rows) => rows
            .iter()
            .rev()
            .find(|row| row.get("type").and_then(serde_json::Value::as_str) == Some("result")),
        serde_json::Value::Object(_) => Some(&raw_json),
        _ => None,
    }?;
    Some(ParsedExternalProviderOutput {
        result_text: result_row
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        usage: result_row.get("usage").cloned(),
        is_error: result_row
            .get("is_error")
            .and_then(serde_json::Value::as_bool),
        error_message: result_row
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        raw_json,
    })
}

#[derive(Debug)]
struct ParsedInternalCodexOutput {
    raw_json: serde_json::Value,
    result_text: Option<String>,
    error_messages: Vec<String>,
}

fn parse_internal_codex_exec_output(stdout: &str) -> ParsedInternalCodexOutput {
    let mut rows = Vec::new();
    let mut result_text = None;
    let mut error_messages = Vec::new();

    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if row.get("type").and_then(serde_json::Value::as_str) == Some("item.completed") {
            if let Some(item) = row.get("item") {
                match item.get("type").and_then(serde_json::Value::as_str) {
                    Some("agent_message") => {
                        if let Some(text) = item
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                        {
                            result_text = Some(text.to_string());
                        }
                    }
                    Some("error") => {
                        if let Some(message) = item
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                        {
                            if !internal_codex_message_is_benign_warning(message) {
                                error_messages.push(message.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        rows.push(row);
    }

    ParsedInternalCodexOutput {
        raw_json: serde_json::Value::Array(rows),
        result_text,
        error_messages,
    }
}

fn internal_codex_message_is_benign_warning(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.starts_with("under-development features enabled:")
        || normalized.contains("to suppress this warning")
}

fn internal_codex_stderr_is_benign_warning(stderr: &str) -> bool {
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .all(|line| line.starts_with("WARN ") || line.contains(" WARN "))
}

fn internal_codex_output_confirms_execution(
    parsed_output: &ParsedInternalCodexOutput,
    stderr: &str,
    exit_success: bool,
) -> bool {
    exit_success
        && parsed_output.error_messages.is_empty()
        && internal_codex_stderr_is_benign_warning(stderr)
        && parsed_output
            .result_text
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn internal_codex_provider_failure_blocker_code(
    selected_cli_system: &str,
    stderr: &str,
    error_messages: &[String],
) -> Option<&'static str> {
    if selected_cli_system != "codex" {
        return None;
    }
    let windows_sandbox_spawn_failed = stderr.contains("windows sandbox: spawn setup refresh")
        || error_messages
            .iter()
            .any(|message| message.contains("windows sandbox: spawn setup refresh"));
    windows_sandbox_spawn_failed.then_some("internal_codex_windows_sandbox_unavailable")
}

fn internal_codex_provider_failure_blocker_reason(
    selected_cli_system: &str,
    blocker_code: &str,
    fallback_reason: String,
) -> String {
    if selected_cli_system == "codex"
        && blocker_code == "internal_codex_windows_sandbox_unavailable"
    {
        return "Internal Codex carrier reached `codex exec`, but the Windows sandbox failed while spawning worker shell commands. Retry with a configured backend/runtime profile whose sandbox is supported on this host, or route through a configured external CLI backend before claiming receipt-backed execution.".to_string();
    }
    fallback_reason
}

fn should_render_store_backed_activation_view_for_internal_failure(
    activation_only: bool,
    success: bool,
) -> bool {
    !activation_only || success
}

fn dispatch_packet_prompt(dispatch_packet_path: &str) -> String {
    std::fs::read_to_string(dispatch_packet_path)
        .ok()
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .and_then(|packet| {
            packet
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| {
            format!(
                "Read and execute the VIDA dispatch packet at {}. Return one bounded result that follows the packet.",
                dispatch_packet_path
            )
        })
}

fn configured_subagent_backend_entry<'a>(
    overlay: &'a serde_yaml::Value,
    backend_id: &str,
) -> Option<&'a serde_yaml::Value> {
    yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|entries| {
            entries.iter().find_map(|(key, value)| {
                (key.as_str()?.trim() == backend_id
                    && crate::yaml_bool(yaml_lookup(value, &["enabled"]), false))
                .then_some(value)
            })
        })
}

fn exact_model_profile_from_backend_entry(
    backend_id: &str,
    backend_entry: &serde_yaml::Value,
    profile_id: &str,
) -> Option<serde_json::Value> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return None;
    }
    let fallback_rate = crate::yaml_string(yaml_lookup(backend_entry, &["budget_cost_units"]))
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| {
            crate::yaml_string(yaml_lookup(backend_entry, &["normalized_cost_units"]))
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .or_else(|| {
            crate::yaml_string(yaml_lookup(backend_entry, &["rate"]))
                .and_then(|raw| raw.parse::<u64>().ok())
        });
    let fallback_runtime_roles =
        crate::yaml_string_list(crate::yaml_lookup(backend_entry, &["runtime_roles"]));
    let fallback_task_classes =
        crate::yaml_string_list(crate::yaml_lookup(backend_entry, &["task_classes"]));
    let projection = crate::model_profile_contract::normalize_profile_projection_from_yaml(
        backend_id,
        backend_entry,
        fallback_rate,
        &fallback_runtime_roles,
        &fallback_task_classes,
    );
    projection["model_profiles"]
        .get(profile_id)
        .cloned()
        .filter(|profile| !profile.is_null())
}

fn apply_internal_subagent_profile_overlay(
    carrier: &serde_json::Value,
    backend_id: &str,
    backend_entry: Option<&serde_yaml::Value>,
    profile_id: Option<&str>,
) -> serde_json::Value {
    let Some(profile_id) = profile_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return carrier.clone();
    };
    let Some(profile) = backend_entry
        .and_then(|entry| exact_model_profile_from_backend_entry(backend_id, entry, profile_id))
    else {
        return carrier.clone();
    };
    let mut patched = carrier.clone();
    let object = patched
        .as_object_mut()
        .expect("internal carrier row should serialize to an object");
    object.insert(
        "selected_model_profile_id".to_string(),
        serde_json::json!(profile_id),
    );
    object.insert(
        "internal_subagent_backend_id".to_string(),
        serde_json::json!(backend_id),
    );
    object.insert(
        "internal_subagent_model_profile_id".to_string(),
        serde_json::json!(profile_id),
    );
    for (target_key, profile_key) in [
        ("model", "model_ref"),
        ("selected_model_ref", "model_ref"),
        ("model_provider", "provider"),
        ("selected_model_provider", "provider"),
        ("selected_reasoning_effort", "reasoning_effort"),
        (
            "selected_plan_mode_reasoning_effort",
            "plan_mode_reasoning_effort",
        ),
        ("selected_sandbox_mode", "sandbox_mode"),
        ("normalized_cost_units", "normalized_cost_units"),
        ("speed_tier", "speed_tier"),
        ("quality_tier", "quality_tier"),
        ("write_scope", "write_scope"),
    ] {
        if profile[profile_key]
            .as_str()
            .is_some_and(|value| value.trim().is_empty())
        {
            continue;
        }
        if !profile[profile_key].is_null() {
            object.insert(target_key.to_string(), profile[profile_key].clone());
        }
    }
    if let Some(reasoning) = profile["reasoning_effort"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        object.insert(
            "model_reasoning_effort".to_string(),
            serde_json::json!(reasoning),
        );
    }
    if let Some(sandbox) = profile["sandbox_mode"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| internal_profile_sandbox_from_write_scope(profile["write_scope"].as_str()))
    {
        object.insert(
            "selected_sandbox_mode".to_string(),
            serde_json::json!(sandbox),
        );
        object.insert("sandbox_mode".to_string(), serde_json::json!(sandbox));
    }
    patched
}

fn internal_profile_sandbox_from_write_scope(write_scope: Option<&str>) -> Option<&'static str> {
    match write_scope?.trim() {
        "orchestrator_native" | "workspace-write" | "scoped_write" => Some("workspace-write"),
        "read-only" | "read_or_review" | "none" => Some("read-only"),
        _ => None,
    }
}

fn selected_internal_host_carrier(
    selected_cli_entry: Option<&serde_yaml::Value>,
    preferred_backend: Option<&str>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    role_selection: &RuntimeConsumptionLaneSelection,
    overlay: Option<&serde_yaml::Value>,
) -> Option<serde_json::Value> {
    let carriers =
        crate::host_runtime_materialization::host_runtime_entry_carrier_catalog(selected_cli_entry);
    let find_carrier = |candidate_id: &str| {
        carriers
            .iter()
            .find(|row| row["role_id"].as_str() == Some(candidate_id))
            .cloned()
    };
    let effective_backend = preferred_backend.or(receipt.selected_backend.as_deref());
    let preferred_profile_id =
        crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
            role_selection,
            &receipt.dispatch_target,
            effective_backend,
        );

    let direct_ids = [preferred_backend, receipt.selected_backend.as_deref()];
    for candidate_id in direct_ids.into_iter().flatten() {
        if let Some(carrier) = find_carrier(candidate_id) {
            return Some(
                crate::model_profile_contract::apply_selected_model_profile_to_row(
                    &carrier,
                    preferred_profile_id.as_deref(),
                ),
            );
        }
    }

    let prefers_internal_backend = direct_ids
        .into_iter()
        .flatten()
        .any(|backend_id| backend_is_internal_host_bridge(role_selection, overlay, backend_id));
    if !prefers_internal_backend {
        return None;
    }

    let internal_backend_id = effective_backend?;
    let internal_bridge_ids = [
        receipt.activation_agent_type.as_deref(),
        role_selection
            .execution_plan
            .get("runtime_assignment")
            .and_then(|value| value.get("activation_agent_type"))
            .and_then(serde_json::Value::as_str),
        role_selection
            .execution_plan
            .get("runtime_assignment")
            .and_then(|value| value.get("selected_tier"))
            .and_then(serde_json::Value::as_str),
        Some(role_selection.selected_role.as_str()),
    ];
    let selected_backend_entry =
        overlay.and_then(|overlay| configured_subagent_backend_entry(overlay, internal_backend_id));
    internal_bridge_ids
        .into_iter()
        .flatten()
        .find_map(find_carrier)
        .map(|carrier| {
            let host_profile_carrier =
                crate::model_profile_contract::apply_selected_model_profile_to_row(
                    &carrier,
                    preferred_profile_id.as_deref(),
                );
            apply_internal_subagent_profile_overlay(
                &host_profile_carrier,
                internal_backend_id,
                selected_backend_entry,
                preferred_profile_id.as_deref(),
            )
        })
}

fn configured_internal_host_runtime_env(
    project_root: &Path,
    selected_cli_system: &str,
    carrier_id: &str,
) -> Result<Vec<(String, String)>, String> {
    let runtime_root = project_root
        .join(".vida")
        .join("data")
        .join("internal-host")
        .join(selected_cli_system)
        .join(carrier_id);
    let xdg_config_home = runtime_root.join("config");
    let xdg_data_home = runtime_root.join("data");
    let xdg_state_home = runtime_root.join("state");
    let xdg_cache_home = runtime_root.join("cache");
    let tmpdir = runtime_root.join("tmp");
    for dir in [
        &xdg_config_home,
        &xdg_data_home,
        &xdg_state_home,
        &xdg_cache_home,
        &tmpdir,
    ] {
        std::fs::create_dir_all(dir).map_err(|error| {
            format!(
                "Failed to prepare internal host runtime dir `{}`: {error}",
                dir.display()
            )
        })?;
    }

    Ok(vec![
        (
            "XDG_CONFIG_HOME".to_string(),
            xdg_config_home.display().to_string(),
        ),
        (
            "XDG_DATA_HOME".to_string(),
            xdg_data_home.display().to_string(),
        ),
        (
            "XDG_STATE_HOME".to_string(),
            xdg_state_home.display().to_string(),
        ),
        (
            "XDG_CACHE_HOME".to_string(),
            xdg_cache_home.display().to_string(),
        ),
        ("TMPDIR".to_string(), tmpdir.display().to_string()),
    ])
}

fn configured_internal_host_activation_parts(
    system_entry: Option<&serde_yaml::Value>,
    project_root: &Path,
    dispatch_packet_path: &str,
    carrier: &serde_json::Value,
) -> Result<(String, Vec<String>, Option<String>), String> {
    let dispatch = system_entry
        .and_then(|entry| yaml_lookup(entry, &["dispatch"]))
        .ok_or_else(|| "Configured internal host system is missing `dispatch`".to_string())?;
    let command = yaml_lookup(dispatch, &["command"])
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Configured internal host system is missing non-empty `dispatch.command`".to_string()
        })?
        .to_string();
    let model = carrier["model"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Configured internal host carrier is missing model".to_string())?;
    let sandbox_mode = carrier["sandbox_mode"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Configured internal host carrier is missing sandbox_mode".to_string())?;
    if sandbox_mode == "danger-full-access" {
        return Err(
            "Configured internal host carrier uses forbidden sandbox_mode `danger-full-access`"
                .to_string(),
        );
    }
    let reasoning_effort = carrier["model_reasoning_effort"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("medium");
    let prompt = dispatch_packet_prompt(dispatch_packet_path);
    let mut args = crate::yaml_string_list(yaml_lookup(dispatch, &["static_args"]));
    args.extend(crate::yaml_string_list(yaml_lookup(
        dispatch,
        &["feature_args"],
    )));
    let mut stdin_payload = None;
    if let Some(workdir_flag) = crate::yaml_string(yaml_lookup(dispatch, &["workdir_flag"])) {
        args.push(workdir_flag);
        args.push(project_root.display().to_string());
    }
    if let Some(sandbox_flag) = crate::yaml_string(yaml_lookup(dispatch, &["sandbox_flag"])) {
        args.push(sandbox_flag);
        args.push(sandbox_mode.to_string());
    }
    if let Some(model_flag) = crate::yaml_string(yaml_lookup(dispatch, &["model_flag"])) {
        args.push(model_flag);
        args.push(model.to_string());
    }
    if let Some(reasoning_effort_flag) =
        crate::yaml_string(yaml_lookup(dispatch, &["reasoning_effort_flag"]))
    {
        let rendered_value =
            crate::yaml_string(yaml_lookup(dispatch, &["reasoning_effort_value_template"]))
                .map(|template| template.replace("{value}", reasoning_effort))
                .unwrap_or_else(|| reasoning_effort.to_string());
        args.push(reasoning_effort_flag);
        args.push(rendered_value);
    }
    let prompt_mode = crate::yaml_string(yaml_lookup(dispatch, &["prompt_mode"]))
        .unwrap_or_else(|| "positional".to_string());
    match prompt_mode.as_str() {
        "positional" => args.push(prompt),
        "stdin" => {
            args.push("-".to_string());
            stdin_payload = Some(prompt);
        }
        other => {
            return Err(format!(
                "Configured internal host system uses unsupported prompt_mode `{other}`"
            ));
        }
    }
    Ok((command, args, stdin_payload))
}

fn command_name(command: &str) -> String {
    let trimmed = command.trim().trim_matches('"').trim_matches('\'');
    Path::new(trimmed)
        .file_stem()
        .or_else(|| Path::new(trimmed).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed)
        .trim()
        .to_ascii_lowercase()
}

fn configured_codex_cli_fallback_enabled(overlay: &serde_yaml::Value) -> bool {
    configured_subagent_backend_entry(overlay, "codex_cli").is_some_and(|entry| {
        crate::yaml_string(yaml_lookup(entry, &["subagent_backend_class"])).as_deref()
            == Some("external_cli")
    })
}

fn internal_host_receipt_backed_completion_supported(
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> serde_json::Value {
    selected_cli_entry
        .and_then(|entry| {
            yaml_lookup(entry, &["dispatch", "receipt_backed_completion_supported"])
                .and_then(serde_yaml::Value::as_bool)
        })
        .map_or(serde_json::Value::Null, serde_json::Value::Bool)
}

fn internal_host_receipt_backed_completion_is_enabled(
    selected_cli_entry: Option<&serde_yaml::Value>,
) -> bool {
    selected_cli_entry.and_then(|entry| {
        yaml_lookup(entry, &["dispatch", "receipt_backed_completion_supported"])
            .and_then(serde_yaml::Value::as_bool)
    }) == Some(true)
}

fn annotate_internal_host_completion_capability(
    dispatch: &mut serde_json::Map<String, serde_json::Value>,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    execution_evidence_available: bool,
) {
    dispatch.insert(
        "receipt_backed_completion_supported".to_string(),
        internal_host_receipt_backed_completion_supported(selected_cli_entry),
    );
    dispatch.insert(
        "receipt_backed_completion_source_path".to_string(),
        serde_json::json!(format!(
            "vida.config.yaml:host_environment.systems.{selected_cli_system}.dispatch.receipt_backed_completion_supported"
        )),
    );
    dispatch.insert(
        "execution_evidence_required".to_string(),
        serde_json::json!(true),
    );
    dispatch.insert(
        "execution_evidence_available".to_string(),
        serde_json::json!(execution_evidence_available),
    );
    dispatch.insert(
        "activation_view_is_execution_evidence".to_string(),
        serde_json::json!(false),
    );
}

fn internal_codex_app_bridge_requires_fail_closed(
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    overlay: &serde_yaml::Value,
    command: &str,
    args: &[String],
) -> Option<&'static str> {
    if selected_cli_system != "codex" {
        return None;
    }
    if command_name(command) != "codex" {
        return None;
    }
    if !args.iter().any(|arg| arg.trim() == "exec") {
        return None;
    }
    if internal_host_receipt_backed_completion_is_enabled(selected_cli_entry) {
        return None;
    }
    if configured_codex_cli_fallback_enabled(overlay) {
        return Some("internal Codex carrier unavailable; refusing external codex_cli bridge for internal backend");
    }
    Some("internal Codex carrier unavailable; external codex_cli fallback disabled")
}

fn internal_codex_windows_sandbox_preflight_blocker(
    is_windows: bool,
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    command: &str,
    args: &[String],
    sandbox_mode: Option<&str>,
) -> Option<(&'static str, String)> {
    if !is_windows || selected_cli_system != "codex" || command_name(command) != "codex" {
        return None;
    }
    if !args.iter().any(|arg| arg.trim() == "exec") {
        return None;
    }
    let sandbox_mode = sandbox_mode
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if sandbox_mode != "workspace-write" {
        return None;
    }
    let windows_sandbox_spawn_supported = selected_cli_entry
        .and_then(|entry| yaml_lookup(entry, &["dispatch", "windows_sandbox_spawn_supported"]))
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false);
    if windows_sandbox_spawn_supported {
        return None;
    }
    Some((
        "internal_codex_windows_sandbox_unavailable",
        format!(
            "Internal Codex carrier is configured for `codex exec` with sandbox_mode `{sandbox_mode}` on Windows, but this host has not declared `dispatch.windows_sandbox_spawn_supported=true`; failing before process launch avoids a long no-receipt timeout. Route through a configured backend/runtime profile whose sandbox is supported on this host, or enable the support flag only after proving receipt-backed execution."
        ),
    ))
}

fn configured_external_cli_backend_ids(
    overlay: &serde_yaml::Value,
    enabled: Option<bool>,
) -> Vec<String> {
    let mut ids = yaml_lookup(overlay, &["agent_system", "subagents"])
        .and_then(serde_yaml::Value::as_mapping)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(key, value)| {
                    let backend_id = key.as_str()?.trim();
                    if backend_id.is_empty() {
                        return None;
                    }
                    if crate::yaml_string(yaml_lookup(value, &["subagent_backend_class"]))
                        .as_deref()
                        != Some("external_cli")
                    {
                        return None;
                    }
                    if let Some(expected_enabled) = enabled {
                        let actual_enabled =
                            yaml_lookup(value, &["enabled"]).and_then(serde_yaml::Value::as_bool);
                        if actual_enabled != Some(expected_enabled) {
                            return None;
                        }
                    }
                    Some(backend_id.to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

fn internal_codex_windows_sandbox_recovery_actions(
    overlay: &serde_yaml::Value,
    selected_cli_system: &str,
    dispatch_target: &str,
    sandbox_mode: Option<&str>,
) -> Vec<String> {
    let disabled_external = configured_external_cli_backend_ids(overlay, Some(false));
    let enabled_external = configured_external_cli_backend_ids(overlay, Some(true));
    let sandbox_mode = sandbox_mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let mut actions = vec![
        format!(
            "Preferred: enable a configured external CLI backend that is admissible for dispatch target `{dispatch_target}` (`agent_system.subagents.<backend>.enabled=true`, `subagent_backend_class=external_cli`, readiness satisfied), then route this lane to that backend through the configured runtime assignment/fallback fields."
        ),
        format!(
            "Alternative only after proof: if `{selected_cli_system}` has verified receipt-backed `codex exec` support for sandbox `{sandbox_mode}` on this Windows host, set `host_environment.systems.{selected_cli_system}.dispatch.windows_sandbox_spawn_supported=true` in `vida.config.yaml`."
        ),
        "Do not continue root-local implementation from this blocker; restore a receipt-backed backend route or record a separate configuration/readiness defect for the missing backend.".to_string(),
    ];
    if !disabled_external.is_empty() {
        actions.insert(
            1,
            format!(
                "Configured external CLI backends currently disabled in `agent_system.subagents`: {}.",
                disabled_external.join(", ")
            ),
        );
    } else if enabled_external.is_empty() {
        actions.insert(
            1,
            "No enabled external CLI backend is configured under `agent_system.subagents`; add or enable one before expecting external fallback dispatch.".to_string(),
        );
    }
    actions
}

pub(crate) fn agent_lane_dispatch_result(
    mut activation_view: serde_json::Value,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> serde_json::Value {
    let effective_selected_backend = preferred_backend
        .map(str::to_string)
        .or_else(|| receipt.selected_backend.clone())
        .or_else(|| {
            crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
                role_selection,
                receipt,
            )
        });
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let blocker_code =
        crate::runtime_dispatch_state::internal_host_activation_view_only_blocker_code(
            &project_root,
            role_selection,
            receipt,
        );
    let lane_dispatch = crate::runtime_dispatch_state::runtime_agent_lane_dispatch_for_root(
        &project_root,
        dispatch_packet_path,
        preferred_backend,
        crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
            role_selection,
            &receipt.dispatch_target,
            preferred_backend,
        )
        .as_deref(),
    );
    let effective_execution_posture =
        crate::runtime_dispatch_state::effective_execution_posture_summary(
            &role_selection.execution_plan,
            &receipt.dispatch_target,
            effective_selected_backend.as_deref(),
            receipt.activation_agent_type.as_deref(),
            Some(&host_runtime),
            false,
            None,
        );
    let execution_truth = summarize_execution_truth_for_route(
        &role_selection.execution_plan,
        crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
            &role_selection.execution_plan,
            &receipt.dispatch_target,
        ),
        host_runtime["selected_cli_execution_class"].as_str(),
        effective_selected_backend.as_deref(),
        Some("activation_view"),
        Some("missing"),
    );
    let body = activation_view
        .as_object_mut()
        .expect("agent-init activation view should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(lane_dispatch.surface),
    );
    body.insert("status".to_string(), serde_json::json!("blocked"));
    body.insert("execution_state".to_string(), serde_json::json!("blocked"));
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(lane_dispatch.activation_command),
    );
    body.insert(
        "dispatch_packet_path".to_string(),
        serde_json::json!(dispatch_packet_path),
    );
    body.insert("host_runtime".to_string(), host_runtime);
    body.insert(
        "effective_execution_posture".to_string(),
        effective_execution_posture,
    );
    body.insert("execution_truth".to_string(), execution_truth);
    body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
    body.insert(
        "blocker_reason".to_string(),
        serde_json::json!(
            "selected host/backend returned only an activation view without execution evidence"
        ),
    );
    body.insert(
        "backend_dispatch".to_string(),
        lane_dispatch.backend_dispatch,
    );
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        let runtime_assignment = role_selection
            .execution_plan
            .get("runtime_assignment")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        for key in [
            "selected_carrier_id",
            "selected_backend_id",
            "selected_model_profile_id",
            "selected_model_ref",
            "selected_model_provider",
            "selected_reasoning_effort",
            "selected_sandbox_mode",
        ] {
            let dispatch_has_value = dispatch.get(key).is_some_and(|value| !value.is_null());
            if !dispatch_has_value && !runtime_assignment[key].is_null() {
                dispatch.insert(key.to_string(), runtime_assignment[key].clone());
            }
        }
    }
    body.insert(
        "role_selection".to_string(),
        serde_json::to_value(role_selection).expect("lane selection should serialize"),
    );
    activation_view
}

fn refresh_execution_truth(
    body: &mut serde_json::Map<String, serde_json::Value>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    effective_selected_backend: Option<&str>,
    execution_evidence_status: &str,
) {
    let host_runtime = body
        .get("host_runtime")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let activation_kind = body
        .get("activation_semantics")
        .and_then(|value| value.get("activation_kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    body.insert(
        "execution_truth".to_string(),
        summarize_execution_truth_for_route(
            &role_selection.execution_plan,
            crate::runtime_dispatch_state::execution_plan_route_for_dispatch_target(
                &role_selection.execution_plan,
                &receipt.dispatch_target,
            ),
            host_runtime["selected_cli_execution_class"].as_str(),
            effective_selected_backend,
            Some(activation_kind),
            Some(execution_evidence_status),
        ),
    );
}

fn mark_dispatch_result_execution_evidence(
    body: &mut serde_json::Map<String, serde_json::Value>,
    evidence_kind: &str,
    backend_id: &str,
) {
    let activation_semantics = body
        .entry("activation_semantics".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let activation_semantics = activation_semantics
        .as_object_mut()
        .expect("activation_semantics should serialize to an object");
    activation_semantics.insert(
        "activation_kind".to_string(),
        serde_json::json!("execution_evidence"),
    );
    activation_semantics.insert("view_only".to_string(), serde_json::json!(false));
    activation_semantics.insert("executes_packet".to_string(), serde_json::json!(true));
    activation_semantics.insert(
        "records_completion_receipt".to_string(),
        serde_json::json!(true),
    );
    activation_semantics.insert(
        "transfers_root_session_write_authority".to_string(),
        serde_json::json!(false),
    );
    activation_semantics.insert(
        "root_session_write_guard_remains_authoritative".to_string(),
        serde_json::json!(true),
    );
    activation_semantics.insert(
        "next_lawful_action".to_string(),
        serde_json::json!(
            "treat this result as receipt-backed delegated-lane execution evidence and continue through runtime downstream progression"
        ),
    );
    body.insert(
        "execution_evidence".to_string(),
        serde_json::json!({
            "status": "recorded",
            "evidence_kind": evidence_kind,
            "backend_id": backend_id,
            "receipt_backed": true,
            "records_dispatch_result": true,
        }),
    );
    if let Some(posture) = body
        .get_mut("effective_execution_posture")
        .and_then(serde_json::Value::as_object_mut)
    {
        posture.insert(
            "activation_evidence_state".to_string(),
            serde_json::json!("execution_evidence"),
        );
        posture.insert(
            "receipt_backed_execution_evidence".to_string(),
            serde_json::json!(true),
        );
        posture.insert(
            "selected_backend".to_string(),
            serde_json::json!(backend_id),
        );
    }
    if let Some(posture) = body
        .get_mut("execution_truth")
        .and_then(serde_json::Value::as_object_mut)
    {
        posture.insert(
            "effective_selected_backend".to_string(),
            serde_json::json!(backend_id),
        );
        if let Some(activation_evidence) = posture
            .get_mut("activation_evidence")
            .and_then(serde_json::Value::as_object_mut)
        {
            activation_evidence.insert(
                "activation_kind".to_string(),
                serde_json::json!("execution_evidence"),
            );
            activation_evidence.insert(
                "execution_evidence_status".to_string(),
                serde_json::json!("recorded"),
            );
            activation_evidence.insert("receipt_backed".to_string(), serde_json::json!(true));
        }
    }
}

pub(crate) async fn execute_internal_agent_lane_dispatch(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    execute_internal_agent_lane_dispatch_with_fallback_policy(
        state_root,
        project_root,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        receipt,
        host_runtime,
        true,
    )
    .await
}

async fn execute_internal_agent_lane_dispatch_with_fallback_policy(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
    allow_internal_codex_external_fallback: bool,
) -> Result<Option<serde_json::Value>, String> {
    let Some(backend_id) = preferred_backend.or(receipt.selected_backend.as_deref()) else {
        return Err(format!(
            "Dispatch target `{}` is routed to an internal agent lane but no backend id was resolved",
            receipt.dispatch_target
        ));
    };
    if !backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        backend_id,
        &receipt.dispatch_target,
    ) {
        return Err(format!(
            "Backend `{backend_id}` is not admissible for dispatch target `{}`",
            receipt.dispatch_target
        ));
    }

    let overlay = crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root)?;
    let (selected_cli_system, selected_cli_entry) =
        crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let execution_class = selected_cli_entry
        .as_ref()
        .and_then(|entry| yaml_lookup(entry, &["execution_class"]))
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            host_runtime["selected_cli_execution_class"]
                .as_str()
                .unwrap_or("unknown")
        });
    if execution_class != "internal" {
        return Ok(None);
    }

    let Some(carrier) = selected_internal_host_carrier(
        selected_cli_entry.as_ref(),
        preferred_backend,
        receipt,
        role_selection,
        Some(&overlay),
    ) else {
        return Ok(None);
    };

    let carrier_id = carrier["role_id"]
        .as_str()
        .unwrap_or(selected_cli_system.as_str());
    let (command, args, stdin_payload) = configured_internal_host_activation_parts(
        selected_cli_entry.as_ref(),
        project_root,
        dispatch_packet_path,
        &carrier,
    )?;
    let preflight_blocker = internal_codex_app_bridge_requires_fail_closed(
        &selected_cli_system,
        selected_cli_entry.as_ref(),
        &overlay,
        &command,
        &args,
    )
    .map(|reason| ("internal_codex_carrier_unavailable", reason.to_string()))
    .or_else(|| {
        internal_codex_windows_sandbox_preflight_blocker(
            cfg!(windows),
            &selected_cli_system,
            selected_cli_entry.as_ref(),
            &command,
            &args,
            carrier["sandbox_mode"].as_str(),
        )
    });
    if let Some((blocker_code, blocker_reason)) = preflight_blocker {
        let preflight_recovery_actions =
            if blocker_code == "internal_codex_windows_sandbox_unavailable" {
                internal_codex_windows_sandbox_recovery_actions(
                    &overlay,
                    &selected_cli_system,
                    &receipt.dispatch_target,
                    carrier["sandbox_mode"].as_str(),
                )
            } else {
                vec![blocker_reason.clone()]
            };
        if allow_internal_codex_external_fallback {
            if let Some(fallback_backend) = internal_codex_external_fallback_backend(
                role_selection,
                &receipt.dispatch_target,
                backend_id,
                &overlay,
            ) {
                let mut result = Box::pin(execute_external_agent_lane_dispatch(
                    state_root,
                    project_root,
                    dispatch_packet_path,
                    Some(&fallback_backend),
                    role_selection,
                    receipt,
                    host_runtime.clone(),
                ))
                .await?;
                if let Some(body) = result.as_object_mut() {
                    body.insert(
                        "internal_codex_external_fallback".to_string(),
                        serde_json::json!({
                            "blocked_backend": backend_id,
                            "blocker_code": blocker_code,
                            "blocker_reason": blocker_reason,
                            "fallback_backend": fallback_backend,
                            "fallback_source": "route_admissible_external_backend",
                            "selected_cli_system": selected_cli_system,
                        }),
                    );
                }
                return Ok(Some(result));
            }
        }
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            preferred_backend,
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("internal agent lane dispatch result should serialize to an object");
        body.insert("surface".to_string(), serde_json::json!("vida agent-init"));
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        body.insert(
            "activation_command".to_string(),
            serde_json::json!(
                crate::runtime_dispatch_state::agent_init_command_for_packet_path(
                    dispatch_packet_path
                )
            ),
        );
        body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason.clone()),
        );
        body.insert(
            "next_actions".to_string(),
            serde_json::json!(preflight_recovery_actions.clone()),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert("backend_class".to_string(), serde_json::json!("internal"));
            dispatch.insert("backend_id".to_string(), serde_json::json!(backend_id));
            dispatch.insert(
                "carrier_id".to_string(),
                serde_json::json!(carrier["role_id"].clone()),
            );
            dispatch.insert(
                "sandbox_mode".to_string(),
                serde_json::json!(carrier["sandbox_mode"].clone()),
            );
            dispatch.insert(
                "preflight_blocker_code".to_string(),
                serde_json::json!(blocker_code),
            );
            dispatch.insert(
                "preflight_blocker_reason".to_string(),
                serde_json::json!(blocker_reason),
            );
            dispatch.insert(
                "preflight_recovery_actions".to_string(),
                serde_json::json!(preflight_recovery_actions),
            );
            dispatch.insert(
                "configured_external_cli_candidates".to_string(),
                serde_json::json!({
                    "enabled": configured_external_cli_backend_ids(&overlay, Some(true)),
                    "disabled": configured_external_cli_backend_ids(&overlay, Some(false)),
                }),
            );
            dispatch.insert(
                "executor_backend".to_string(),
                serde_json::json!("internal"),
            );
            dispatch.insert(
                "external_cli_fallback_enabled".to_string(),
                serde_json::json!(configured_codex_cli_fallback_enabled(&overlay)),
            );
            annotate_internal_host_completion_capability(
                dispatch,
                &selected_cli_system,
                selected_cli_entry.as_ref(),
                false,
            );
        }
        refresh_execution_truth(body, role_selection, receipt, Some(backend_id), "missing");
        return Ok(Some(result));
    }
    let wall_timeout_seconds = Some(configured_internal_host_dispatch_wall_timeout_seconds(
        project_root,
        role_selection,
        receipt,
    ));
    let wrapped_command =
        wrap_command_with_optional_timeout(command.clone(), args.clone(), wall_timeout_seconds);
    let activation_command = crate::runtime_dispatch_state::render_command_display(
        &wrapped_command.command,
        &wrapped_command.args,
    );
    let runtime_env =
        configured_internal_host_runtime_env(project_root, &selected_cli_system, carrier_id)?;

    let mut process = std::process::Command::new(&wrapped_command.command);
    process
        .args(&wrapped_command.args)
        .current_dir(project_root);
    for (key, value) in runtime_env {
        process.env(key, value);
    }
    process.env("VIDA_DISPATCH_PACKET_PATH", dispatch_packet_path);
    process.env("VIDA_DISPATCH_TARGET", &receipt.dispatch_target);
    process.env("VIDA_SELECTED_CLI_SYSTEM", &selected_cli_system);
    process.env("VIDA_SELECTED_BACKEND", carrier_id);
    if let Some(profile_id) = carrier["selected_model_profile_id"].as_str() {
        process.env("VIDA_SELECTED_MODEL_PROFILE", profile_id);
    }
    if let Some(runtime_role) = receipt.activation_runtime_role.as_deref() {
        process.env("VIDA_RUNTIME_ROLE", runtime_role);
    }

    let output = execute_wrapped_command_async(
        process,
        wrapped_command.clone(),
        stdin_payload.map(String::into_bytes),
    )
    .await
    .map_err(|error| {
        format!(
            "Failed to execute internal host carrier `{carrier_id}` for `{selected_cli_system}` via `{}`: {error}",
            wrapped_command.command
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed_output = parse_internal_codex_exec_output(&stdout);
    let exit_code = output.status.code();
    let timed_out = output.timed_out;
    let success =
        internal_codex_output_confirms_execution(&parsed_output, &stderr, output.status.success());
    let activation_only = timed_out
        || (output.status.success()
            && parsed_output.result_text.is_none()
            && parsed_output.error_messages.is_empty()
            && stderr.is_empty());
    let activation_view = if should_render_store_backed_activation_view_for_internal_failure(
        activation_only,
        success,
    ) {
        bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await
    } else {
        default_activation_view(receipt, role_selection)
    };
    let mut result = agent_lane_dispatch_result(
        activation_view,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        receipt,
        host_runtime,
    );
    let body = result
        .as_object_mut()
        .expect("internal agent lane dispatch result should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(format!("internal_cli:{selected_cli_system}")),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(activation_command),
    );
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch.insert("backend_class".to_string(), serde_json::json!("internal"));
        dispatch.insert("backend_id".to_string(), serde_json::json!(carrier_id));
        dispatch.insert(
            "carrier_id".to_string(),
            serde_json::json!(carrier["role_id"].clone()),
        );
        dispatch.insert(
            "model".to_string(),
            serde_json::json!(carrier["model"].clone()),
        );
        dispatch.insert(
            "model_reasoning_effort".to_string(),
            serde_json::json!(carrier["model_reasoning_effort"].clone()),
        );
        dispatch.insert(
            "sandbox_mode".to_string(),
            serde_json::json!(carrier["sandbox_mode"].clone()),
        );
        for key in [
            "selected_model_profile_id",
            "selected_model_ref",
            "selected_model_provider",
            "selected_reasoning_effort",
            "selected_sandbox_mode",
            "internal_subagent_backend_id",
            "internal_subagent_model_profile_id",
        ] {
            if !carrier[key].is_null() {
                dispatch.insert(key.to_string(), carrier[key].clone());
            }
        }
        annotate_internal_host_completion_capability(
            dispatch,
            &selected_cli_system,
            selected_cli_entry.as_ref(),
            success,
        );
    }

    body.insert(
        "status".to_string(),
        serde_json::json!(if success { "pass" } else { "blocked" }),
    );
    body.insert(
        "execution_state".to_string(),
        serde_json::json!(if success { "executed" } else { "blocked" }),
    );
    body.insert("provider_output".to_string(), serde_json::json!(stdout));
    body.insert("provider_error".to_string(), serde_json::json!(stderr));
    body.insert("exit_code".to_string(), serde_json::json!(exit_code));
    if let Some(timeout_wrapper) = &wrapped_command.timeout_wrapper {
        body.insert(
            "timeout_wrapper".to_string(),
            serde_json::json!({
                "command": wrapped_command.command,
                "timeout_seconds": timeout_wrapper.timeout_seconds,
                "kill_after_grace_seconds": timeout_wrapper.kill_after_grace_seconds,
                "timed_out": timed_out,
                "timeout_exit_code": exit_code,
            }),
        );
    }
    body.insert(
        "provider_output_json".to_string(),
        parsed_output.raw_json.clone(),
    );
    body.insert(
        "provider_result".to_string(),
        parsed_output
            .result_text
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    body.insert(
        "provider_error_items".to_string(),
        serde_json::to_value(parsed_output.error_messages.clone())
            .expect("internal host error items should serialize"),
    );
    if success {
        body.insert("blocker_code".to_string(), serde_json::Value::Null);
        body.insert("blocker_reason".to_string(), serde_json::Value::Null);
        mark_dispatch_result_execution_evidence(body, "internal_carrier_completion", carrier_id);
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "recorded");
    } else if activation_only {
        if timed_out {
            let timeout_seconds = wrapped_command
                .timeout_wrapper
                .as_ref()
                .map(|wrapper| wrapper.timeout_seconds)
                .unwrap_or_default();
            let kill_after_grace_seconds = wrapped_command
                .timeout_wrapper
                .as_ref()
                .map(|wrapper| wrapper.kill_after_grace_seconds)
                .unwrap_or_default();
            body.insert(
                "provider_error".to_string(),
                serde_json::json!(format!(
                    "internal host carrier for `{selected_cli_system}` timed out after {timeout_seconds}s and kill-after grace {kill_after_grace_seconds}s without receipt-backed completion"
                )),
            );
        }
        let blocker_reason = if timed_out {
            format!(
                "internal host carrier for `{selected_cli_system}` exceeded the bounded runtime window before returning execution evidence"
            )
        } else {
            format!(
                "internal host carrier for `{selected_cli_system}` completed without returning an agent_message result"
            )
        };
        let blocker_code = internal_host_activation_only_blocker_code(
            project_root,
            role_selection,
            receipt,
            timed_out,
        );
        body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "missing");
    } else {
        let blocker_reason = if !stderr.is_empty() {
            stderr.clone()
        } else if !parsed_output.error_messages.is_empty() {
            parsed_output.error_messages.join("\n")
        } else if output.status.success() {
            format!(
                "internal host carrier for `{selected_cli_system}` completed without returning an agent_message result"
            )
        } else {
            format!(
                "internal host carrier for `{selected_cli_system}` exited without returning receipt-backed completion"
            )
        };
        let blocker_code = internal_codex_provider_failure_blocker_code(
            &selected_cli_system,
            &stderr,
            &parsed_output.error_messages,
        )
        .unwrap_or("configured_backend_dispatch_failed");
        let blocker_reason = internal_codex_provider_failure_blocker_reason(
            &selected_cli_system,
            blocker_code,
            blocker_reason,
        );
        body.insert("blocker_code".to_string(), serde_json::json!(blocker_code));
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(blocker_reason),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(carrier_id), "missing");
    }

    Ok(Some(result))
}

pub(crate) async fn execute_external_agent_lane_dispatch(
    state_root: &Path,
    project_root: &Path,
    dispatch_packet_path: &str,
    preferred_backend: Option<&str>,
    role_selection: &RuntimeConsumptionLaneSelection,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    host_runtime: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let overlay = crate::runtime_dispatch_state::load_project_overlay_yaml_for_root(project_root)?;
    let (selected_cli_system, _) =
        crate::runtime_dispatch_state::selected_host_cli_system_for_runtime_dispatch(&overlay);
    let preferred_external_backend = preferred_backend.and_then(|backend_id| {
        crate::runtime_dispatch_state::configured_external_backend_entry_any(&overlay, backend_id)
            .map(|entry| (backend_id.to_string(), entry.clone()))
    });
    let (backend_id, backend_entry, backend_class) = if let Some((backend_id, backend_entry)) =
        preferred_external_backend
    {
        (backend_id, backend_entry, "external_cli".to_string())
    } else {
        let backend_class = crate::runtime_dispatch_state::configured_dispatch_backend_class(
            &overlay,
            &selected_cli_system,
        );
        let (backend_id, backend_entry) =
            crate::runtime_dispatch_state::selected_external_backend_for_system(
                &overlay,
                &selected_cli_system,
                preferred_backend,
            )
            .ok_or_else(|| {
                format!(
                    "Configured host CLI system `{selected_cli_system}` has no enabled external backend dispatch adapter"
                )
            })?;
        (backend_id, backend_entry, backend_class)
    };

    if let Some(dispatch_blocker) =
        crate::runtime_dispatch_state::configured_external_backend_dispatch_blocker(
            &backend_id,
            &backend_entry,
        )
    {
        let readiness_verdict = serde_json::json!({
            "backend_id": backend_id,
            "status": "external_backend_dispatch_blocked",
            "blocked": true,
            "blocker_code": "configured_backend_dispatch_failed",
            "blocker_reason": dispatch_blocker,
            "next_actions": [
                format!("Enable and repair external backend `{backend_id}` in `vida.config.yaml`, or reroute this lane to a receipt-backed backend before dispatch.")
            ],
        });
        if let Some(fallback_backend) = ready_external_readiness_fallback_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
            &overlay,
            receipt.selected_backend.as_deref(),
        ) {
            let mut result = Box::pin(execute_external_agent_lane_dispatch(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
            ))
            .await?;
            if let Some(body) = result.as_object_mut() {
                body.insert(
                    "external_dispatch_blocker_external_fallback".to_string(),
                    serde_json::json!({
                        "blocked_backend": backend_id,
                        "fallback_backend": fallback_backend,
                        "readiness": readiness_verdict,
                    }),
                );
            }
            return Ok(result);
        }
        if let Some(fallback_backend) = readiness_fallback_internal_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
        ) {
            if let Some(mut result) = execute_internal_agent_lane_dispatch_with_fallback_policy(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
                false,
            )
            .await?
            {
                if let Some(body) = result.as_object_mut() {
                    body.insert(
                        "external_dispatch_blocker_internal_fallback".to_string(),
                        serde_json::json!({
                            "blocked_backend": backend_id,
                            "fallback_backend": fallback_backend,
                            "readiness": readiness_verdict,
                        }),
                    );
                }
                return Ok(result);
            }
        }
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            Some(&backend_id),
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("agent lane dispatch result should serialize to an object");
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(dispatch_blocker),
        );
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        body.insert(
            "external_backend_readiness".to_string(),
            readiness_verdict.clone(),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert(
                "backend_class".to_string(),
                serde_json::json!(backend_class.clone()),
            );
            dispatch.insert("external_backend_readiness".to_string(), readiness_verdict);
        }
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
        return Ok(result);
    }

    // Admissibility gate: refuse to dispatch to an external backend that is not
    // admissible for the target lane (e.g. a read-only backend for an implementer lane).
    if !backend_is_admissible_for_dispatch_target(
        &role_selection.execution_plan,
        &backend_id,
        &receipt.dispatch_target,
    ) {
        let activation_view = match StateStore::open_existing(state_root.to_path_buf()).await {
            Ok(store) => {
                let rendered =
                    crate::init_surfaces::render_agent_init_packet_activation_with_store(
                        &store,
                        project_root,
                        dispatch_packet_path,
                        dispatch_packet_path_should_render_as_downstream(dispatch_packet_path),
                    )
                    .await
                    .unwrap_or_else(|_| default_activation_view(receipt, role_selection));
                drop(store);
                rendered
            }
            Err(_) => default_activation_view(receipt, role_selection),
        };
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            Some(&backend_id),
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("agent lane dispatch result should serialize to an object");
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("backend_inadmissible_for_lane"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(format!(
                "Backend `{backend_id}` is not admissible for dispatch target `{}` (lane_admissibility denies this lane); an implementation-capable backend is required",
                receipt.dispatch_target
            )),
        );
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
        return Ok(result);
    }

    let selected_model_profile_id =
        crate::runtime_dispatch_state::preferred_selected_model_profile_for_dispatch_target(
            role_selection,
            &receipt.dispatch_target,
            Some(&backend_id),
        );
    let readiness_verdict =
        crate::status_surface_external_cli::external_cli_backend_readiness_verdict_for_profile(
            &backend_id,
            &backend_entry,
            selected_model_profile_id.as_deref(),
        );
    if readiness_verdict["blocked"].as_bool().unwrap_or(false) {
        if let Some(fallback_backend) = ready_external_readiness_fallback_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
            &overlay,
            receipt.selected_backend.as_deref(),
        ) {
            let mut result = Box::pin(execute_external_agent_lane_dispatch(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
            ))
            .await?;
            if let Some(body) = result.as_object_mut() {
                body.insert(
                    "external_readiness_external_fallback".to_string(),
                    serde_json::json!({
                        "blocked_backend": backend_id,
                        "fallback_backend": fallback_backend,
                        "readiness": readiness_verdict,
                    }),
                );
            }
            return Ok(result);
        }
        if let Some(fallback_backend) = readiness_fallback_internal_backend(
            role_selection,
            &receipt.dispatch_target,
            &backend_id,
        ) {
            if let Some(mut result) = execute_internal_agent_lane_dispatch_with_fallback_policy(
                state_root,
                project_root,
                dispatch_packet_path,
                Some(&fallback_backend),
                role_selection,
                receipt,
                host_runtime.clone(),
                false,
            )
            .await?
            {
                if let Some(body) = result.as_object_mut() {
                    body.insert(
                        "external_readiness_fallback".to_string(),
                        serde_json::json!({
                            "blocked_backend": backend_id,
                            "fallback_backend": fallback_backend,
                            "readiness": readiness_verdict,
                        }),
                    );
                }
                return Ok(result);
            }
        }
        let readiness_status = readiness_verdict["status"]
            .as_str()
            .unwrap_or("external_backend_blocked");
        let selected_model_profile = selected_model_profile_id
            .as_deref()
            .or_else(|| readiness_verdict["selected_model_profile"].as_str())
            .unwrap_or("unknown");
        let next_action = readiness_verdict["next_actions"]
            .as_array()
            .and_then(|actions| actions.iter().filter_map(serde_json::Value::as_str).next())
            .unwrap_or("Repair the external backend readiness blocker before dispatch.");
        let blocker_code = readiness_verdict
            .get("blocker_code")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or_else(|| serde_json::json!("external_backend_not_ready"));
        let activation_view = bounded_activation_view(
            state_root,
            project_root,
            dispatch_packet_path,
            receipt,
            role_selection,
        )
        .await;
        let mut result = agent_lane_dispatch_result(
            activation_view,
            dispatch_packet_path,
            Some(&backend_id),
            role_selection,
            receipt,
            host_runtime,
        );
        let body = result
            .as_object_mut()
            .expect("agent lane dispatch result should serialize to an object");
        body.insert("blocker_code".to_string(), blocker_code);
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(format!(
                "External backend `{backend_id}` is not dispatch-ready before launch: {readiness_status}; selected_model_profile={selected_model_profile}. {next_action}"
            )),
        );
        body.insert("status".to_string(), serde_json::json!("blocked"));
        body.insert("execution_state".to_string(), serde_json::json!("blocked"));
        body.insert(
            "external_backend_readiness".to_string(),
            readiness_verdict.clone(),
        );
        if let Some(dispatch) = body
            .get_mut("backend_dispatch")
            .and_then(serde_json::Value::as_object_mut)
        {
            dispatch.insert(
                "backend_class".to_string(),
                serde_json::json!(backend_class.clone()),
            );
            dispatch.insert("external_backend_readiness".to_string(), readiness_verdict);
        }
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
        return Ok(result);
    }
    let (command, args) = crate::runtime_dispatch_state::configured_external_activation_parts(
        &backend_id,
        &backend_entry,
        project_root,
        dispatch_packet_path,
        selected_model_profile_id.as_deref(),
    )?;
    let stdin_payload =
        crate::runtime_dispatch_state::configured_external_activation_stdin_payload(
            &backend_entry,
            dispatch_packet_path,
        )?;
    let wall_timeout_seconds = configured_external_dispatch_wall_timeout_seconds(&backend_entry);
    let wrapped_command =
        wrap_command_with_optional_timeout(command.clone(), args.clone(), wall_timeout_seconds);
    let activation_command = crate::runtime_dispatch_state::render_command_display(
        &wrapped_command.command,
        &wrapped_command.args,
    );

    let mut process = std::process::Command::new(&wrapped_command.command);
    process
        .args(&wrapped_command.args)
        .current_dir(project_root)
        .stdin(Stdio::null());
    if let Some(serde_yaml::Value::Mapping(env_map)) =
        yaml_lookup(&backend_entry, &["dispatch", "env"])
    {
        for (key, value) in env_map {
            if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                process.env(key, value);
            }
        }
    }
    process.env("VIDA_DISPATCH_PACKET_PATH", dispatch_packet_path);
    process.env("VIDA_DISPATCH_TARGET", &receipt.dispatch_target);
    process.env("VIDA_SELECTED_CLI_SYSTEM", &selected_cli_system);
    if let Some(profile_id) = selected_model_profile_id.as_deref() {
        process.env("VIDA_SELECTED_MODEL_PROFILE", profile_id);
    }
    if let Some(runtime_role) = receipt.activation_runtime_role.as_deref() {
        process.env("VIDA_RUNTIME_ROLE", runtime_role);
    }
    let effective_selected_backend =
        crate::runtime_dispatch_state::canonical_selected_backend_for_receipt(
            role_selection,
            receipt,
        )
        .or_else(|| receipt.selected_backend.clone());
    if let Some(selected_backend) = effective_selected_backend.as_deref() {
        process.env("VIDA_SELECTED_BACKEND", selected_backend);
    }

    #[cfg(test)]
    let output = if let Some(output) = emulated_test_shell_output(&wrapped_command) {
        output
    } else {
        execute_wrapped_command_async(
            process,
            wrapped_command.clone(),
            stdin_payload.clone().map(String::into_bytes),
        )
        .await
        .map_err(|error| {
            format!(
                "Failed to execute configured external backend `{backend_id}` via `{}`: {error}",
                wrapped_command.command
            )
        })?
    };
    #[cfg(not(test))]
    let output = execute_wrapped_command_async(
        process,
        wrapped_command.clone(),
        stdin_payload.map(String::into_bytes),
    )
    .await
    .map_err(|error| {
        format!(
            "Failed to execute configured external backend `{backend_id}` via `{}`: {error}",
            wrapped_command.command
        )
    })?;
    let activation_view = bounded_activation_view(
        state_root,
        project_root,
        dispatch_packet_path,
        receipt,
        role_selection,
    )
    .await;
    let mut result = agent_lane_dispatch_result(
        activation_view,
        dispatch_packet_path,
        preferred_backend,
        role_selection,
        receipt,
        host_runtime,
    );
    let body = result
        .as_object_mut()
        .expect("agent lane dispatch result should serialize to an object");
    body.insert(
        "surface".to_string(),
        serde_json::json!(format!("{backend_class}:{backend_id}")),
    );
    body.insert(
        "activation_command".to_string(),
        serde_json::json!(activation_command),
    );
    if let Some(dispatch) = body
        .get_mut("backend_dispatch")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch.insert(
            "backend_class".to_string(),
            serde_json::json!(backend_class),
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed_output = parse_external_provider_output(&stdout);
    let success = output.status.success()
        && external_provider_output_confirms_execution(parsed_output.as_ref());
    let exit_code = output.status.code();
    let timed_out = output.timed_out;
    body.insert(
        "status".to_string(),
        serde_json::json!(if success { "pass" } else { "blocked" }),
    );
    body.insert(
        "execution_state".to_string(),
        serde_json::json!(if success { "executed" } else { "blocked" }),
    );
    body.insert("provider_output".to_string(), serde_json::json!(stdout));
    body.insert("provider_error".to_string(), serde_json::json!(stderr));
    body.insert("exit_code".to_string(), serde_json::json!(exit_code));
    if let Some(timeout_wrapper) = &wrapped_command.timeout_wrapper {
        body.insert(
            "timeout_wrapper".to_string(),
            serde_json::json!({
                "command": wrapped_command.command,
                "timeout_seconds": timeout_wrapper.timeout_seconds,
                "kill_after_grace_seconds": timeout_wrapper.kill_after_grace_seconds,
                "timed_out": timed_out,
                "timeout_exit_code": exit_code,
            }),
        );
    }
    if let Some(parsed_output) = parsed_output.as_ref() {
        body.insert(
            "provider_output_json".to_string(),
            parsed_output.raw_json.clone(),
        );
        body.insert(
            "provider_result".to_string(),
            parsed_output
                .result_text
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
        body.insert(
            "provider_usage".to_string(),
            parsed_output
                .usage
                .clone()
                .unwrap_or(serde_json::Value::Null),
        );
        if let Some(scope_guard) = external_provider_scope_guard(&parsed_output.raw_json) {
            body.insert("provider_scope_guard".to_string(), scope_guard.clone());
        }
        if let Some(paths) = external_provider_reported_paths(&parsed_output.raw_json) {
            body.insert("provider_reported_paths".to_string(), paths);
        }
        body.insert(
            "provider_is_error".to_string(),
            parsed_output
                .is_error
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null),
        );
        body.insert(
            "provider_error_message".to_string(),
            parsed_output
                .error_message
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    if success {
        body.insert("blocker_code".to_string(), serde_json::Value::Null);
        body.insert("blocker_reason".to_string(), serde_json::Value::Null);
        mark_dispatch_result_execution_evidence(body, "external_backend_completion", &backend_id);
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "recorded");
    } else if timed_out
        || parsed_output
            .as_ref()
            .is_some_and(external_provider_output_indicates_agent_end_timeout)
    {
        let timeout_seconds = wrapped_command
            .timeout_wrapper
            .as_ref()
            .map(|wrapper| wrapper.timeout_seconds)
            .unwrap_or_default();
        let kill_after_grace_seconds = wrapped_command
            .timeout_wrapper
            .as_ref()
            .map(|wrapper| wrapper.kill_after_grace_seconds)
            .unwrap_or_default();
        body.insert(
            "provider_error".to_string(),
            serde_json::json!(
                parsed_output
                    .as_ref()
                    .and_then(external_provider_error_message)
                    .unwrap_or_else(|| format!(
                        "configured external backend timed out after {timeout_seconds}s and kill-after grace {kill_after_grace_seconds}s without receipt-backed completion"
                    ))
            ),
        );
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!(crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::TimeoutWithoutTakeoverAuthority
            )),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(
                "configured external backend exceeded the bounded runtime window before returning execution evidence"
            ),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
    } else {
        let provider_error_message = parsed_output
            .as_ref()
            .and_then(external_provider_error_message)
            .or_else(|| {
                output.status.success().then(|| {
                    "configured external backend exited successfully but did not return a parseable success payload"
                        .to_string()
                })
            })
            .or_else(|| {
                body.get("provider_error_message")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
            });
        body.insert(
            "blocker_code".to_string(),
            serde_json::json!("configured_backend_dispatch_failed"),
        );
        body.insert(
            "blocker_reason".to_string(),
            serde_json::json!(provider_error_message.unwrap_or_else(|| {
                "configured external backend exited without returning receipt-backed completion"
                    .to_string()
            })),
        );
        refresh_execution_truth(body, role_selection, receipt, Some(&backend_id), "missing");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::execute_wrapped_command;
    use super::{
        agent_lane_dispatch_result, configured_external_dispatch_wall_timeout_seconds,
        configured_internal_host_activation_parts, configured_internal_host_runtime_env,
        dispatch_packet_path_should_render_as_downstream, dispatch_packet_prompt,
        execute_external_agent_lane_dispatch, external_provider_output_confirms_execution,
        internal_codex_app_bridge_requires_fail_closed, internal_codex_output_confirms_execution,
        internal_codex_windows_sandbox_preflight_blocker,
        internal_codex_windows_sandbox_recovery_actions,
        internal_host_activation_only_blocker_code, mark_dispatch_result_execution_evidence,
        parse_external_provider_output, parse_internal_codex_exec_output,
        ready_external_readiness_fallback_backend,
        should_render_store_backed_activation_view_for_internal_failure,
        wrap_command_with_optional_timeout, CommandTimeoutWrapper,
    };
    use crate::RuntimeConsumptionLaneSelection;
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    use std::process::Stdio;
    #[cfg(unix)]
    use std::time::{Duration, Instant};

    #[test]
    fn parse_external_provider_output_extracts_qwen_json_success_result() {
        let parsed = parse_external_provider_output(
            r#"[{"type":"system"},{"type":"result","subtype":"success","is_error":false,"result":"OK","usage":{"total_tokens":42}}]"#,
        )
        .expect("qwen json output should parse");

        assert_eq!(parsed.result_text.as_deref(), Some("OK"));
        assert_eq!(parsed.is_error, Some(false));
        assert_eq!(
            parsed.usage.expect("usage should exist")["total_tokens"],
            42
        );
        assert_eq!(parsed.error_message, None);
    }

    #[test]
    fn parse_external_provider_output_extracts_qwen_json_error_message() {
        let parsed = parse_external_provider_output(
            r#"[{"type":"result","subtype":"error_during_execution","is_error":true,"error":{"message":"Missing API key"}}]"#,
        )
        .expect("qwen json error output should parse");

        assert_eq!(parsed.is_error, Some(true));
        assert_eq!(parsed.error_message.as_deref(), Some("Missing API key"));
        assert_eq!(parsed.result_text, None);
    }

    #[test]
    fn parse_external_provider_output_detects_bracketed_api_error() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","is_error":false,"result":"[API Error: 401 invalid access token or token expired]"}"#,
        )
        .expect("qwen json error output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert_eq!(
            super::external_provider_error_message(&parsed).as_deref(),
            Some("[API Error: 401 invalid access token or token expired]")
        );
    }

    #[test]
    fn parse_external_provider_output_with_success_stays_success() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"OK"}"#,
        )
        .expect("qwen json error output should parse");

        assert!(!super::external_provider_output_indicates_error(&parsed));
        assert!(external_provider_output_confirms_execution(Some(&parsed)));
    }

    #[test]
    fn parse_external_provider_output_trusts_pi_agent_end_success_even_when_result_mentions_auth_text(
    ) {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"raw_provider":{"mode":"rpc","provider":"pi","terminal_event":"agent_end"},"result":"packet text mentions authentication failed and invalid api key as configuration examples"}"#,
        )
        .expect("pi adapter json output should parse");

        assert!(!super::external_provider_output_indicates_error(&parsed));
        assert!(external_provider_output_confirms_execution(Some(&parsed)));
    }

    #[test]
    fn parse_external_provider_output_blocks_pi_agent_end_when_result_declares_blocked_dispatch() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"raw_provider":{"mode":"rpc","provider":"pi","terminal_event":"agent_end"},"result":"Thinking mode: STC.\nBounded result: dispatch blocked by VIDA Pi write-scope guard; both the packet's `vida agent-init --execute-dispatch` path and the verification path are refused in bash guarded-write mode, so no execution receipt/result artifact was produced."}"#,
        )
        .expect("pi adapter blocked dispatch output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(!external_provider_output_confirms_execution(Some(&parsed)));
    }

    #[test]
    fn parse_external_provider_output_classifies_pi_agent_end_timeout_as_runtime_timeout() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"raw_provider":{"mode":"rpc","provider":"pi"},"error":{"message":"Timed out waiting for Pi agent_end event"}}"#,
        )
        .expect("pi timeout output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(super::external_provider_output_indicates_agent_end_timeout(
            &parsed
        ));
        assert_eq!(
            super::external_provider_error_message(&parsed).as_deref(),
            Some("Timed out waiting for Pi agent_end event")
        );
    }

    #[test]
    fn configured_external_dispatch_wall_timeout_prefers_max_runtime_over_no_output_window() {
        let backend_entry = serde_yaml::from_str(
            r#"
max_runtime_seconds: 420
dispatch:
  no_output_timeout_seconds: 180
"#,
        )
        .expect("backend entry should parse");

        assert_eq!(
            configured_external_dispatch_wall_timeout_seconds(&backend_entry),
            Some(420)
        );
    }

    #[test]
    fn parse_external_provider_output_blocks_scope_guard_violation() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"OK","scope_guard":{"status":"violation","valid":false,"touched_paths":["docs/spec.md"]}}"#,
        )
        .expect("adapter json output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(!external_provider_output_confirms_execution(Some(&parsed)));
        assert_eq!(
            super::external_provider_scope_guard(&parsed.raw_json)
                .expect("scope guard should be preserved")["status"],
            "violation"
        );
    }

    #[test]
    fn parse_external_provider_output_exposes_reported_paths() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"OK","raw_provider":{"provider_result_json":{"touched_paths":["src/lib.rs"],"changed_files":["src/main.rs"]}}}"#,
        )
        .expect("adapter json output should parse");

        let paths = super::external_provider_reported_paths(&parsed.raw_json)
            .expect("reported paths should be preserved");
        assert_eq!(paths["touched_paths"][0], "src/lib.rs");
        assert_eq!(paths["changed_files"][0], "src/main.rs");
    }

    #[test]
    fn parse_external_provider_output_detects_quota_exceeded_semantic_failure() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Qwen OAuth quota exceeded: Your free daily quota has been reached."}"#,
        )
        .expect("qwen json quota output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        assert!(!external_provider_output_confirms_execution(Some(&parsed)));
        assert_eq!(
            super::external_provider_error_message(&parsed).as_deref(),
            Some("Qwen OAuth quota exceeded: Your free daily quota has been reached.")
        );
    }

    #[test]
    fn parse_external_provider_output_bracketed_api_error_cannot_be_treated_as_executed() {
        let parsed = parse_external_provider_output(
            r#"{"type":"result","is_error":false,"result":"[API Error: 401 invalid access token or token expired]"}"#,
        )
        .expect("qwen json error output should parse");

        assert!(super::external_provider_output_indicates_error(&parsed));
        let status_code_success = true;
        let execution_succeeded =
            status_code_success && !super::external_provider_output_indicates_error(&parsed);
        assert!(!execution_succeeded);
    }

    #[test]
    fn parse_external_provider_output_plain_text_success_stays_unparsable() {
        let parsed = parse_external_provider_output("external-dispatch:implemented");
        assert!(parsed.is_none());
        assert!(!external_provider_output_confirms_execution(
            parsed.as_ref()
        ));
    }

    #[test]
    fn parse_external_provider_output_plain_text_auth_failure_stays_unparsable() {
        let parsed =
            parse_external_provider_output("Authentication failed: invalid API key provided");
        assert!(parsed.is_none());
        assert!(!external_provider_output_confirms_execution(
            parsed.as_ref()
        ));
    }

    #[test]
    fn unparsable_external_provider_stdout_cannot_confirm_execution() {
        assert!(!external_provider_output_confirms_execution(None));
    }

    #[test]
    fn parse_internal_codex_exec_output_extracts_last_agent_message() {
        let parsed = parse_internal_codex_exec_output(
            r#"{"type":"thread.started","thread_id":"abc"}
{"type":"item.completed","item":{"id":"1","type":"error","message":"warning"}}
{"type":"item.completed","item":{"id":"2","type":"agent_message","text":"first"}}
{"type":"item.completed","item":{"id":"3","type":"agent_message","text":"final"}}"#,
        );

        assert_eq!(parsed.result_text.as_deref(), Some("final"));
        assert_eq!(parsed.error_messages, vec!["warning".to_string()]);
        assert_eq!(parsed.raw_json.as_array().map(Vec::len), Some(4));
    }

    #[test]
    fn internal_codex_output_requires_clean_error_streams() {
        let parsed_with_error = parse_internal_codex_exec_output(
            r#"{"type":"item.completed","item":{"id":"1","type":"error","message":"warning"}}
{"type":"item.completed","item":{"id":"2","type":"agent_message","text":"final"}}"#,
        );
        assert!(!internal_codex_output_confirms_execution(
            &parsed_with_error,
            "",
            true
        ));

        let parsed_with_unstable_feature_warning = parse_internal_codex_exec_output(
            r#"{"type":"item.completed","item":{"id":"1","type":"error","message":"Under-development features enabled: memories. Under-development features are incomplete and may behave unpredictably. To suppress this warning, set `suppress_unstable_features_warning = true` in config.toml."}}
{"type":"item.completed","item":{"id":"2","type":"agent_message","text":"final"}}"#,
        );
        assert!(parsed_with_unstable_feature_warning
            .error_messages
            .is_empty());
        assert!(internal_codex_output_confirms_execution(
            &parsed_with_unstable_feature_warning,
            "",
            true
        ));

        let parsed_clean = parse_internal_codex_exec_output(
            r#"{"type":"item.completed","item":{"id":"1","type":"agent_message","text":"final"}}"#,
        );
        assert!(internal_codex_output_confirms_execution(
            &parsed_clean,
            "",
            true
        ));
        assert!(internal_codex_output_confirms_execution(
            &parsed_clean,
            "2026-05-12T20:35:57Z WARN codex_core::features: unknown feature key in config: hooks",
            true
        ));
        assert!(!internal_codex_output_confirms_execution(
            &parsed_clean,
            "sandbox denied write to /workspace/secret",
            true
        ));
        assert!(!internal_codex_output_confirms_execution(
            &parsed_clean,
            "",
            false
        ));
    }

    #[test]
    fn internal_codex_windows_sandbox_spawn_failure_gets_specific_blocker() {
        let stderr =
            "2026-05-22T02:27:30Z ERROR codex_core::exec: exec error: windows sandbox: spawn setup refresh";

        assert_eq!(
            super::internal_codex_provider_failure_blocker_code("codex", stderr, &[]),
            Some("internal_codex_windows_sandbox_unavailable")
        );
        assert_eq!(
            super::internal_codex_provider_failure_blocker_code("opencode", stderr, &[]),
            None
        );
        assert!(super::internal_codex_provider_failure_blocker_reason(
            "codex",
            "internal_codex_windows_sandbox_unavailable",
            stderr.to_string()
        )
        .contains("configured backend/runtime profile whose sandbox is supported"));
    }

    #[test]
    fn internal_activation_only_failure_skips_store_backed_activation_render() {
        assert!(!should_render_store_backed_activation_view_for_internal_failure(true, false));
        assert!(should_render_store_backed_activation_view_for_internal_failure(false, false));
    }

    #[test]
    fn configured_internal_host_runtime_env_uses_selected_system_segment() {
        let harness = std::env::temp_dir().join(format!(
            "vida-runtime-dispatch-execution-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&harness).expect("create harness dir");
        let env = configured_internal_host_runtime_env(&harness, "qwen", "worker-a")
            .expect("internal host env");
        let xdg_config_home = env
            .iter()
            .find(|(key, _)| key == "XDG_CONFIG_HOME")
            .map(|(_, value)| value.clone())
            .expect("xdg config home");

        let expected = harness
            .join(".vida")
            .join("data")
            .join("internal-host")
            .join("qwen")
            .join("worker-a")
            .join("config");
        assert_eq!(PathBuf::from(xdg_config_home), expected);
        let _ = std::fs::remove_dir_all(&harness);
    }

    #[test]
    fn downstream_agent_init_backend_truth_detects_downstream_packet_path_for_activation_render() {
        let harness = std::env::temp_dir().join(format!(
            "vida-runtime-dispatch-downstream-detect-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&harness).expect("create harness dir");
        let downstream_path = harness.join("downstream.json");
        let dispatch_path = harness.join("dispatch.json");
        std::fs::write(
            &downstream_path,
            serde_json::json!({
                "packet_kind": "runtime_downstream_dispatch_packet",
                "downstream_dispatch_target": "implementer"
            })
            .to_string(),
        )
        .expect("downstream packet should write");
        std::fs::write(
            &dispatch_path,
            serde_json::json!({
                "packet_kind": "runtime_dispatch_packet",
                "dispatch_target": "specification"
            })
            .to_string(),
        )
        .expect("dispatch packet should write");

        assert!(dispatch_packet_path_should_render_as_downstream(
            downstream_path
                .to_str()
                .expect("downstream path should render")
        ));
        assert!(!dispatch_packet_path_should_render_as_downstream(
            dispatch_path.to_str().expect("dispatch path should render")
        ));
        assert!(!dispatch_packet_path_should_render_as_downstream(
            harness
                .join("missing.json")
                .to_str()
                .expect("missing path should render")
        ));

        let _ = std::fs::remove_dir_all(&harness);
    }

    #[test]
    fn configured_internal_host_activation_parts_use_system_dispatch_config() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  feature_args: ["--enable", "multi_agent"]
  workdir_flag: -C
  sandbox_flag: -s
  model_flag: -m
  reasoning_effort_flag: -c
  reasoning_effort_value_template: 'model_reasoning_effort="{value}"'
  prompt_mode: positional
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.4",
            "model_reasoning_effort": "high",
            "sandbox_mode": "workspace-write"
        });

        let (command, args, stdin_payload) = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect("internal host activation parts");

        assert_eq!(command, "codex");
        assert_eq!(
            args,
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "--enable".to_string(),
                "multi_agent".to_string(),
                "-C".to_string(),
                "/tmp/project".to_string(),
                "-s".to_string(),
                "workspace-write".to_string(),
                "-m".to_string(),
                "gpt-5.4".to_string(),
                "-c".to_string(),
                "model_reasoning_effort=\"high\"".to_string(),
                dispatch_packet_prompt("/tmp/project/.vida/dispatch.json"),
            ]
        );
        assert_eq!(stdin_payload, None);
    }

    #[test]
    fn configured_internal_host_activation_parts_support_stdin_prompt_mode() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  workdir_flag: -C
  sandbox_flag: -s
  model_flag: -m
  reasoning_effort_flag: -c
  reasoning_effort_value_template: 'model_reasoning_effort="{value}"'
  prompt_mode: stdin
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.4",
            "model_reasoning_effort": "high",
            "sandbox_mode": "workspace-write"
        });

        let (command, args, stdin_payload) = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect("internal host activation parts");

        assert_eq!(command, "codex");
        assert_eq!(
            args,
            vec![
                "exec".to_string(),
                "--json".to_string(),
                "-C".to_string(),
                "/tmp/project".to_string(),
                "-s".to_string(),
                "workspace-write".to_string(),
                "-m".to_string(),
                "gpt-5.4".to_string(),
                "-c".to_string(),
                "model_reasoning_effort=\"high\"".to_string(),
                "-".to_string(),
            ]
        );
        assert_eq!(
            stdin_payload.as_deref(),
            Some(dispatch_packet_prompt("/tmp/project/.vida/dispatch.json").as_str())
        );
    }

    #[test]
    fn internal_codex_app_bridge_fail_closes_before_codex_exec_when_cli_fallback_disabled() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      role: codex_internal_primary
      default_model: gpt-5.5
    codex_cli:
      enabled: false
      subagent_backend_class: external_cli
      role: bridge_fallback
"#,
        )
        .expect("overlay should parse");
        let args = vec![
            "exec".to_string(),
            "--json".to_string(),
            "-m".to_string(),
            "gpt-5.5".to_string(),
        ];

        assert_eq!(
            internal_codex_app_bridge_requires_fail_closed("codex", None, &overlay, "codex", &args),
            Some("internal Codex carrier unavailable; external codex_cli fallback disabled")
        );
        assert_eq!(
            internal_codex_app_bridge_requires_fail_closed("qwen", None, &overlay, "codex", &args),
            None
        );
        assert_eq!(
            internal_codex_app_bridge_requires_fail_closed(
                "codex",
                None,
                &overlay,
                "fake-codex",
                &args
            ),
            None
        );
    }

    #[test]
    fn internal_codex_app_bridge_allows_codex_exec_when_receipt_backed_completion_is_configured() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");
        let selected_cli_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  receipt_backed_completion_supported: true
"#,
        )
        .expect("selected cli entry should parse");
        let args = vec!["exec".to_string(), "--json".to_string()];

        assert_eq!(
            internal_codex_app_bridge_requires_fail_closed(
                "codex",
                Some(&selected_cli_entry),
                &overlay,
                "codex",
                &args
            ),
            None
        );
    }

    #[test]
    fn internal_codex_windows_workspace_write_preflight_fails_fast_without_support_flag() {
        let selected_cli_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
"#,
        )
        .expect("selected cli entry should parse");
        let args = vec!["exec".to_string(), "--json".to_string()];

        let blocker = internal_codex_windows_sandbox_preflight_blocker(
            true,
            "codex",
            Some(&selected_cli_entry),
            "codex",
            &args,
            Some("workspace-write"),
        )
        .expect("workspace-write codex exec should fail closed on Windows");

        assert_eq!(blocker.0, "internal_codex_windows_sandbox_unavailable");
        assert!(blocker.1.contains("failing before process launch"));
        assert_eq!(
            internal_codex_windows_sandbox_preflight_blocker(
                false,
                "codex",
                Some(&selected_cli_entry),
                "codex",
                &args,
                Some("workspace-write"),
            ),
            None
        );
        assert_eq!(
            internal_codex_windows_sandbox_preflight_blocker(
                true,
                "codex",
                Some(&selected_cli_entry),
                "codex",
                &args,
                Some("read-only"),
            ),
            None
        );
    }

    #[test]
    fn internal_codex_windows_workspace_write_preflight_honors_support_flag() {
        let selected_cli_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  windows_sandbox_spawn_supported: true
"#,
        )
        .expect("selected cli entry should parse");
        let args = vec!["exec".to_string(), "--json".to_string()];

        assert_eq!(
            internal_codex_windows_sandbox_preflight_blocker(
                true,
                "codex",
                Some(&selected_cli_entry),
                "codex",
                &args,
                Some("workspace-write"),
            ),
            None
        );
    }

    #[test]
    fn internal_codex_windows_sandbox_recovery_actions_are_actionable() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    disabled_external_fixture:
      enabled: false
      subagent_backend_class: external_cli
    internal_fixture:
      enabled: true
      subagent_backend_class: internal
"#,
        )
        .expect("overlay should parse");

        let actions = internal_codex_windows_sandbox_recovery_actions(
            &overlay,
            "codex",
            "implementation",
            Some("workspace-write"),
        );

        assert!(
            actions
                .iter()
                .any(|action| action.contains("agent_system.subagents.<backend>.enabled=true")),
            "actions should name the external backend enablement path"
        );
        assert!(
            actions
                .iter()
                .any(|action| action.contains("disabled_external_fixture")),
            "actions should list configured disabled external candidates"
        );
        assert!(
            actions.iter().any(|action| action.contains(
                "host_environment.systems.codex.dispatch.windows_sandbox_spawn_supported=true"
            )),
            "actions should name the exact Windows support flag path"
        );
        assert!(
            actions
                .iter()
                .any(|action| action.contains("receipt-backed")),
            "actions should keep receipt-backed execution as the recovery target"
        );
    }

    #[test]
    fn internal_codex_exec_falls_back_to_ready_admissible_external_route_backend() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-internal-codex-external-fallback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("dispatch.json"),
            r#"{"prompt":"FALLBACK_OK"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            internal_codex_external_fallback_overlay(),
        )
        .expect("write overlay");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "analysis": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "qwen_cli",
                    "fanout_executor_backends": ["qwen_cli"]
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle"
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    receipt
                        .dispatch_packet_path
                        .as_deref()
                        .expect("receipt dispatch packet path"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("internal dispatch should return fallback result");

        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["surface"], "external_cli:qwen_cli");
        assert_eq!(result["blocker_code"], serde_json::Value::Null);
        assert_eq!(result["backend_dispatch"]["backend_id"], "qwen_cli");
        assert_eq!(
            result["internal_codex_external_fallback"]["blocked_backend"],
            "internal_subagents"
        );
        assert_eq!(
            result["internal_codex_external_fallback"]["fallback_backend"],
            "qwen_cli"
        );
        assert_ne!(result["blocker_code"], "internal_codex_carrier_unavailable");

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn external_readiness_fallback_prefers_ready_inherited_external_backend() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: vida-missing-hermes-test-command
        prompt_mode: positional
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: cargo
        static_args: ["--version"]
        prompt_mode: positional
"#,
        )
        .expect("overlay should parse");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "internal_subagents"
                }
            }
        }));

        assert_eq!(
            ready_external_readiness_fallback_backend(
                &role_selection,
                "coach",
                "hermes_cli",
                &overlay,
                Some("qwen_cli")
            )
            .as_deref(),
            Some("qwen_cli")
        );
    }

    #[test]
    fn external_readiness_fallback_prefers_ready_runtime_assignment_backend() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: vida-missing-hermes-test-command
        prompt_mode: positional
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: cargo
        static_args: ["--version"]
        prompt_mode: positional
"#,
        )
        .expect("overlay should parse");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "runtime_assignment": {
                "selected_backend_id": "qwen_cli",
                "selected_tier": "external_write_guarded",
                "activation_agent_type": "qwen_cli"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "internal_subagents"
                }
            }
        }));

        assert_eq!(
            ready_external_readiness_fallback_backend(
                &role_selection,
                "coach",
                "hermes_cli",
                &overlay,
                None
            )
            .as_deref(),
            Some("qwen_cli")
        );
    }

    #[test]
    fn external_readiness_fallback_rejects_disabled_external_candidate() {
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: vida-missing-hermes-test-command
        prompt_mode: positional
    qwen_cli:
      enabled: false
      subagent_backend_class: external_cli
      dispatch:
        command: cargo
        static_args: ["--version"]
        prompt_mode: positional
"#,
        )
        .expect("overlay should parse");
        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "runtime_assignment": {
                "selected_backend_id": "qwen_cli",
                "selected_tier": "external_write_guarded",
                "activation_agent_type": "qwen_cli"
            },
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "coach": true,
                        "review": true
                    }
                }
            ],
            "development_flow": {
                "coach": {
                    "executor_backend": "hermes_cli",
                    "fallback_executor_backend": "qwen_cli"
                }
            }
        }));

        assert!(
            ready_external_readiness_fallback_backend(
                &role_selection,
                "coach",
                "hermes_cli",
                &overlay,
                Some("qwen_cli")
            )
            .is_none(),
            "dispatch-blocked external fallback candidate must not be selected"
        );
    }

    #[test]
    fn internal_codex_exec_preserves_blocker_when_no_admissible_external_fallback_exists() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-internal-codex-no-external-fallback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("dispatch.json"),
            r#"{"prompt":"NO_FALLBACK"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            internal_codex_external_fallback_overlay(),
        )
        .expect("write overlay");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                },
                {
                    "backend_id": "qwen_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "analysis": false,
                        "implementation": false
                    }
                }
            ],
            "development_flow": {
                "analysis": {
                    "executor_backend": "internal_subagents",
                    "fallback_executor_backend": "qwen_cli",
                    "fanout_executor_backends": ["qwen_cli"]
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle"
            }
        }));
        let receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_internal_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    receipt
                        .dispatch_packet_path
                        .as_deref()
                        .expect("receipt dispatch packet path"),
                    Some("internal_subagents"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return")
            .expect("internal dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "internal_codex_carrier_unavailable");
        assert!(result["internal_codex_external_fallback"].is_null());
        assert_eq!(
            result["backend_dispatch"]["receipt_backed_completion_supported"],
            false
        );
        assert_eq!(
            result["backend_dispatch"]["receipt_backed_completion_source_path"],
            "vida.config.yaml:host_environment.systems.codex.dispatch.receipt_backed_completion_supported"
        );
        assert_eq!(
            result["backend_dispatch"]["execution_evidence_required"],
            true
        );
        assert_eq!(
            result["backend_dispatch"]["execution_evidence_available"],
            false
        );
        assert_eq!(
            result["backend_dispatch"]["activation_view_is_execution_evidence"],
            false
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn external_readiness_internal_fallback_preserves_internal_codex_blocker() {
        let blocked_external_backend = "disabled_external_fixture";
        let internal_backend = "internal_fixture";
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-internal-codex-blocker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("dispatch.json"),
            r#"{"prompt":"EXTERNAL_INTERNAL_BLOCKER"}"#,
        )
        .expect("write dispatch packet");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            internal_codex_disabled_external_primary_overlay(),
        )
        .expect("write overlay");

        let role_selection = internal_codex_fallback_role_selection(serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": blocked_external_backend,
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                },
                {
                    "backend_id": internal_backend,
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "analysis": true,
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "analysis": {
                    "executor_backend": blocked_external_backend,
                    "fallback_executor_backend": internal_backend,
                    "fanout_executor_backends": []
                }
            },
            "runtime_assignment": {
                "activation_agent_type": "middle",
                "selected_tier": "middle"
            }
        }));
        let mut receipt = internal_codex_fallback_receipt(
            project_root
                .join("dispatch.json")
                .to_str()
                .expect("dispatch path should render"),
        );
        receipt.selected_backend = Some(blocked_external_backend.to_string());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                super::execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    receipt
                        .dispatch_packet_path
                        .as_deref()
                        .expect("receipt dispatch packet path"),
                    Some(blocked_external_backend),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "internal_codex_carrier_unavailable");
        assert_eq!(result["backend_dispatch"]["backend_id"], internal_backend);
        assert_eq!(
            result["external_dispatch_blocker_internal_fallback"]["blocked_backend"],
            blocked_external_backend
        );
        assert_eq!(
            result["external_dispatch_blocker_internal_fallback"]["fallback_backend"],
            internal_backend
        );
        assert!(result["internal_codex_external_fallback"].is_null());

        let _ = std::fs::remove_dir_all(&project_root);
    }

    fn internal_codex_external_fallback_overlay() -> &'static str {
        r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch:
        command: codex
        receipt_backed_completion_supported: false
        static_args: ["exec", "--json"]
        prompt_mode: positional
      carriers:
        middle:
          model: gpt-5.4
          model_reasoning_effort: medium
          sandbox_mode: workspace-write
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: gpt-5.4
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
    qwen_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args:
          - -lc
          - |
            printf '{"type":"result","is_error":false,"result":"external-dispatch:%s"}' "$*"
          - vida-dispatch
        prompt_mode: positional
        prompt_template: "FALLBACK_OK"
"#
    }

    fn internal_codex_disabled_external_primary_overlay() -> &'static str {
        r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
      dispatch:
        command: codex
        static_args: ["exec", "--json"]
        prompt_mode: positional
      carriers:
        middle:
          model: fixture-model
          model_reasoning_effort: medium
          sandbox_mode: workspace-write
agent_system:
  subagents:
    disabled_external_fixture:
      enabled: false
      subagent_backend_class: external_cli
      dispatch:
        command: qwen
        prompt_mode: positional
    internal_fixture:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: internal
          reasoning_effort: medium
          normalized_cost_units: 4
          write_scope: orchestrator_native
          runtime_roles: [business_analyst]
          task_classes: [analysis]
"#
    }

    fn internal_codex_fallback_role_selection(
        execution_plan: serde_json::Value,
    ) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Analyze the bounded handoff".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan,
            reason: "test".to_string(),
        }
    }

    fn internal_codex_fallback_receipt(
        dispatch_packet_path: &str,
    ) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-codex-fallback".to_string(),
            dispatch_target: "analysis".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some(dispatch_packet_path.to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn internal_host_activation_only_timeout_uses_timeout_blocker() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({}),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-timeout-code".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("middle".to_string()),
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };

        assert_eq!(
            internal_host_activation_only_blocker_code(
                Path::new("/tmp/project"),
                &role_selection,
                &receipt,
                true,
            ),
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
        assert_eq!(
            internal_host_activation_only_blocker_code(
                Path::new("/tmp/project"),
                &role_selection,
                &receipt,
                false,
            ),
            crate::runtime_dispatch_state::INTERNAL_DISPATCH_TIMEOUT_WITHOUT_RECEIPT
        );
    }

    #[test]
    fn configured_internal_host_activation_parts_rejects_danger_full_access_sandbox() {
        let system_entry = serde_yaml::from_str(
            r#"
dispatch:
  command: codex
  static_args: ["exec", "--json"]
  sandbox_flag: -s
  model_flag: -m
  prompt_mode: positional
"#,
        )
        .expect("system entry should parse");
        let carrier = serde_json::json!({
            "model": "gpt-5.4",
            "sandbox_mode": "danger-full-access"
        });

        let error = configured_internal_host_activation_parts(
            Some(&system_entry),
            Path::new("/tmp/project"),
            "/tmp/project/.vida/dispatch.json",
            &carrier,
        )
        .expect_err("danger-full-access should be rejected");
        assert!(error.contains("forbidden sandbox_mode"));
    }

    #[test]
    fn mark_dispatch_result_execution_evidence_reclassifies_activation_view() {
        let mut body = serde_json::Map::from_iter([(
            "activation_semantics".to_string(),
            serde_json::json!({
                "activation_kind": "activation_view",
                "view_only": true,
                "executes_packet": false,
                "records_completion_receipt": false,
            }),
        )]);

        mark_dispatch_result_execution_evidence(&mut body, "internal_carrier_completion", "junior");

        assert_eq!(
            body["activation_semantics"]["activation_kind"],
            "execution_evidence"
        );
        assert_eq!(body["activation_semantics"]["view_only"], false);
        assert_eq!(body["activation_semantics"]["executes_packet"], true);
        assert_eq!(
            body["activation_semantics"]["records_completion_receipt"],
            true
        );
        assert_eq!(body["execution_evidence"]["status"], "recorded");
        assert_eq!(
            body["execution_evidence"]["evidence_kind"],
            "internal_carrier_completion"
        );
        assert_eq!(body["execution_evidence"]["backend_id"], "junior");
        assert_eq!(body["execution_evidence"]["receipt_backed"], true);
    }

    #[test]
    fn agent_lane_dispatch_result_emits_execution_truth() {
        let result = agent_lane_dispatch_result(
            serde_json::json!({
                "activation_semantics": {
                    "activation_kind": "activation_view",
                    "view_only": true
                }
            }),
            "/tmp/dispatch-packet.json",
            Some("internal_subagents"),
            &RuntimeConsumptionLaneSelection {
                ok: true,
                activation_source: "test".to_string(),
                selection_mode: "fixed".to_string(),
                fallback_role: "orchestrator".to_string(),
                request: "Implement the task".to_string(),
                selected_role: "worker".to_string(),
                conversational_mode: None,
                single_task_only: false,
                tracked_flow_entry: None,
                allow_freeform_chat: false,
                confidence: "high".to_string(),
                matched_terms: vec![],
                compiled_bundle: serde_json::Value::Null,
                execution_plan: serde_json::json!({
                    "backend_admissibility_matrix": [
                        {
                            "backend_id": "opencode_cli",
                            "backend_class": "external_cli"
                        },
                        {
                            "backend_id": "internal_subagents",
                            "backend_class": "internal"
                        }
                    ],
                    "development_flow": {
                        "implementer": {
                            "executor_backend": "opencode_cli",
                            "fallback_executor_backend": "internal_subagents"
                        }
                    }
                }),
                reason: "test".to_string(),
            },
            &crate::state_store::RunGraphDispatchReceipt {
                run_id: "run-1".to_string(),
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
                activation_agent_type: Some("worker".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-11T00:00:00Z".to_string(),
            },
            serde_json::json!({
                "selected_cli_execution_class": "internal"
            }),
        );

        assert_eq!(
            result["execution_truth"]["effective_execution_posture"],
            "hybrid"
        );
        assert_eq!(
            result["execution_truth"]["route_primary_backend"],
            "opencode_cli"
        );
        assert_eq!(
            result["execution_truth"]["effective_selected_backend"],
            "internal_subagents"
        );
        assert_eq!(
            result["execution_truth"]["selected_backend_source"],
            "route_fallback_hint"
        );
        assert_eq!(
            result["execution_truth"]["activation_evidence"]["execution_evidence_status"],
            "missing"
        );
    }

    #[test]
    fn selected_internal_host_carrier_maps_internal_backend_alias_to_activation_tier() {
        let system_entry = serde_yaml::from_str(
            r#"
carriers:
  junior:
    model: gpt-5.4
    model_reasoning_effort: low
    sandbox_mode: workspace-write
  middle:
    model: gpt-5.4
    model_reasoning_effort: medium
    sandbox_mode: workspace-write
"#,
        )
        .expect("system entry should parse");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "selected_tier": "middle"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "coach": true,
                            "review": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-carrier-bridge".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-19T00:00:00Z".to_string(),
        };

        let carrier = super::selected_internal_host_carrier(
            Some(&system_entry),
            Some("internal_subagents"),
            &receipt,
            &role_selection,
            None,
        )
        .expect("internal backend alias should bridge to activation tier");

        assert_eq!(carrier["role_id"].as_str(), Some("middle"));
    }

    #[test]
    fn selected_internal_host_carrier_applies_selected_model_profile_fields() {
        let system_entry = serde_yaml::from_str(
            r#"
carriers:
  middle:
    model: gpt-5.4
    model_reasoning_effort: medium
    sandbox_mode: workspace-write
    default_model_profile: codex_gpt54_medium
    model_profiles:
      codex_gpt54_medium:
        model_ref: gpt-5.4
        reasoning_effort: medium
        sandbox_mode: workspace-write
        normalized_cost_units: 4
        runtime_roles: [coach]
        task_classes: [review]
      codex_spark_high_review:
        model_ref: gpt-5.3-codex-spark
        reasoning_effort: high
        sandbox_mode: read-only
        normalized_cost_units: 16
        runtime_roles: [coach]
        task_classes: [review]
"#,
        )
        .expect("system entry should parse");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "selected_tier": "middle",
                    "selected_model_profile_id": "codex_spark_high_review"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "coach": true,
                            "review": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-profile-bridge".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let carrier = super::selected_internal_host_carrier(
            Some(&system_entry),
            Some("internal_subagents"),
            &receipt,
            &role_selection,
            None,
        )
        .expect("internal backend alias should bridge to activation tier");

        assert_eq!(carrier["role_id"].as_str(), Some("middle"));
        assert_eq!(
            carrier["selected_model_profile_id"].as_str(),
            Some("codex_spark_high_review")
        );
        assert_eq!(carrier["model"].as_str(), Some("gpt-5.3-codex-spark"));
        assert_eq!(carrier["model_reasoning_effort"].as_str(), Some("high"));
        assert_eq!(carrier["sandbox_mode"].as_str(), Some("read-only"));
    }

    #[test]
    fn selected_internal_host_carrier_applies_internal_subagent_route_profile_overlay() {
        let system_entry = serde_yaml::from_str(
            r#"
carriers:
  middle:
    model: gpt-5.4
    model_reasoning_effort: high
    sandbox_mode: workspace-write
"#,
        )
        .expect("system entry should parse");
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      default_model_profile: internal_fast
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: internal_fast
          reasoning_effort: low
          normalized_cost_units: 6
          speed_tier: fast
          quality_tier: medium_high
          write_scope: orchestrator_native
          runtime_roles: [worker]
          task_classes: [implementation]
        internal_review:
          provider: internal
          model_ref: internal_review
          reasoning_effort: medium
          normalized_cost_units: 8
          speed_tier: medium
          quality_tier: high
          write_scope: read_or_review
          runtime_roles: [coach]
          task_classes: [review]
"#,
        )
        .expect("overlay should parse");
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Continue development".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["continue".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "internal_subagents",
                        "profiles": {
                            "internal_subagents": "internal_review"
                        }
                    }
                },
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "selected_tier": "middle",
                    "selected_model_profile_id": "codex_gpt54_medium"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-internal-route-profile-bridge".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch.json".to_string()),
            dispatch_result_path: None,
            blocker_code: Some("internal_activation_view_only".to_string()),
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
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };

        let carrier = super::selected_internal_host_carrier(
            Some(&system_entry),
            Some("internal_subagents"),
            &receipt,
            &role_selection,
            Some(&overlay),
        )
        .expect("internal route profile should bridge through host carrier");

        assert_eq!(carrier["role_id"].as_str(), Some("middle"));
        assert_eq!(carrier["model"].as_str(), Some("internal_review"));
        assert_eq!(
            carrier["selected_model_profile_id"].as_str(),
            Some("internal_review")
        );
        assert_eq!(
            carrier["internal_subagent_model_profile_id"].as_str(),
            Some("internal_review")
        );
        assert_eq!(
            carrier["selected_model_ref"].as_str(),
            Some("internal_review")
        );
        assert_eq!(carrier["model_reasoning_effort"].as_str(), Some("medium"));
    }

    #[test]
    fn selected_internal_host_carrier_applies_internal_subagent_write_scope_sandbox() {
        let carrier = serde_json::json!({
            "role_id": "senior",
            "model": "gpt-5.4",
            "model_reasoning_effort": "high",
            "sandbox_mode": "read-only"
        });
        let overlay = serde_yaml::from_str(
            r#"
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      write_scope: orchestrator_native
      model_profiles:
        internal_fast:
          provider: internal
          model_ref: internal_fast
          reasoning_effort: low
          normalized_cost_units: 1
          write_scope: orchestrator_native
          runtime_roles: [worker]
          task_classes: [implementation]
"#,
        )
        .expect("overlay should parse");
        let backend_entry =
            super::configured_subagent_backend_entry(&overlay, "internal_subagents");

        let patched = super::apply_internal_subagent_profile_overlay(
            &carrier,
            "internal_subagents",
            backend_entry,
            Some("internal_fast"),
        );

        assert_eq!(patched["sandbox_mode"].as_str(), Some("workspace-write"));
        assert_eq!(
            patched["selected_sandbox_mode"].as_str(),
            Some("workspace-write")
        );
        assert_eq!(patched["write_scope"].as_str(), Some("orchestrator_native"));
    }

    #[test]
    fn wrap_command_with_optional_timeout_adds_kill_after_grace() {
        let wrapped = wrap_command_with_optional_timeout(
            "codex".to_string(),
            vec!["exec".to_string()],
            Some(5),
        );

        assert_eq!(wrapped.command, "codex");
        assert_eq!(wrapped.args, vec!["exec".to_string()]);
        assert_eq!(
            wrapped.timeout_wrapper,
            Some(CommandTimeoutWrapper {
                timeout_seconds: 5,
                kill_after_grace_seconds: 1,
            })
        );
    }

    #[test]
    fn internal_host_dispatch_wall_timeout_uses_configured_route_window() {
        let wrapped = wrap_command_with_optional_timeout(
            "codex".to_string(),
            vec!["exec".to_string()],
            Some(420),
        );

        assert_eq!(
            wrapped.timeout_wrapper,
            Some(CommandTimeoutWrapper {
                timeout_seconds: 420,
                kill_after_grace_seconds: 1,
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn execute_wrapped_command_times_out_when_descendant_keeps_pipe_open() {
        let wrapped = wrap_command_with_optional_timeout(
            "sh".to_string(),
            vec!["-c".to_string(), "(sleep 30) & exit 0".to_string()],
            Some(1),
        );
        let mut process = std::process::Command::new(&wrapped.command);
        process.args(&wrapped.args).stdin(Stdio::null());

        let started = Instant::now();
        let output = execute_wrapped_command(process, &wrapped, None)
            .expect("timed command should complete");

        assert!(output.timed_out);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[test]
    fn execute_wrapped_command_times_out_when_detached_descendant_keeps_pipe_open() {
        let wrapped = wrap_command_with_optional_timeout(
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "setsid sh -c 'sleep 30' & exit 0".to_string(),
            ],
            Some(1),
        );
        let mut process = std::process::Command::new(&wrapped.command);
        process.args(&wrapped.args).stdin(Stdio::null());

        let started = Instant::now();
        let output = execute_wrapped_command(process, &wrapped, None)
            .expect("detached timed command should complete");

        assert!(output.timed_out);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "expected detached descendant timeout wrapper to return within a bounded window, got {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_denies_read_only_backend_for_implementer() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "backend_class": "external_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "coach": true,
                        "execution_preparation": true,
                        "implementation": false,
                        "review": true,
                        "verification": false,
                        "policy_flags": {
                            "read_only_backend": true,
                            "review_only_backend": true,
                            "scoped_write_backend": false,
                            "internal_only_backend": false
                        }
                    }
                },
                {
                    "backend_id": "internal_subagents",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "analysis": true,
                        "coach": true,
                        "execution_preparation": true,
                        "implementation": true,
                        "review": true,
                        "verification": true,
                        "policy_flags": {
                            "read_only_backend": false,
                            "review_only_backend": false,
                            "scoped_write_backend": false,
                            "internal_only_backend": true
                        }
                    }
                }
            ]
        });

        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "hermes_cli should be inadmissible for implementer alias lane"
        );
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementation"
            ),
            "hermes_cli should be inadmissible for implementation lane"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "analysis"
            ),
            "hermes_cli should be admissible for analysis lane"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "internal_subagents",
                "implementation"
            ),
            "internal_subagents should be admissible for implementation lane"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_open_without_matrix() {
        let execution_plan = serde_json::json!({});
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "write-producing implementer lane should fail closed when no admissibility matrix is present"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "analysis"
            ),
            "read-only lanes should still fail open when no admissibility matrix is present"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_open_for_unknown_backend() {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "other_backend",
                    "lane_admissibility": {
                        "implementation": false
                    }
                }
            ]
        });
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "implementer lane should fail closed when backend row is missing from the matrix"
        );
        assert!(
            super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "analysis"
            ),
            "read-only lanes should continue failing open when backend is not in the matrix"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_closed_for_implementer_when_lane_key_missing(
    ) {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "lane_admissibility": {
                        "analysis": true,
                        "coach": true
                    }
                }
            ]
        });
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "implementer"
            ),
            "implementer lane should fail closed when canonical implementation key is absent"
        );
    }

    #[test]
    fn backend_is_admissible_for_dispatch_target_fails_closed_for_execution_preparation_when_canonical_lane_key_missing(
    ) {
        let execution_plan = serde_json::json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "hermes_cli",
                    "lane_admissibility": {
                        "execution_preparation": false
                    }
                }
            ]
        });
        assert!(
            !super::backend_is_admissible_for_dispatch_target(
                &execution_plan,
                "hermes_cli",
                "execution_preparation"
            ),
            "execution_preparation lane should fail closed when canonical architecture key is absent"
        );
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_inadmissible_implementer_backend_before_launch()
    {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-admissibility-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: qwen
  systems:
    qwen:
      enabled: true
      execution_class: external
      external_backend_id: hermes_cli
agent_system:
  subagents:
    hermes_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
"#,
        )
        .expect("write overlay");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the task".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true,
                            "coach": true,
                            "implementation": false
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": "hermes_cli"
                    }
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
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
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("hermes_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "backend_inadmissible_for_lane");
        assert_eq!(result["backend_dispatch"]["backend_id"], "hermes_cli");
        assert_eq!(
            result["backend_dispatch"]["provider_error"],
            serde_json::Value::Null
        );

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_known_readiness_failure_before_launch() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-readiness-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        let missing_auth = project_root.join("missing-provider-auth.json");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            format!(
                r#"
host_environment:
  cli_system: opencode
  systems:
    opencode:
      enabled: true
      execution_class: external
      external_backend_id: opencode_cli
agent_system:
  subagents:
    opencode_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
      readiness:
        auth:
          mode: file_present
          path: "{}"
"#,
                missing_auth.display().to_string().replace('\\', "/")
            ),
        )
        .expect("write overlay");

        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the task".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "opencode_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "analysis": true,
                            "coach": true,
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": "opencode_cli"
                    }
                },
                "runtime_assignment": {
                    "selected_backend_id": "opencode_cli",
                    "selected_model_profile_id": "opencode_minimax_free_review"
                }
            }),
            reason: "test".to_string(),
        };
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
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
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("opencode_cli".to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("opencode_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "interactive_auth_required");
        assert_eq!(result["backend_dispatch"]["backend_id"], "opencode_cli");
        assert_eq!(
            result["external_backend_readiness"]["status"],
            "interactive_auth_required"
        );
        assert_eq!(
            result["backend_dispatch"]["external_backend_readiness"]["blocked"],
            true
        );
        assert_eq!(
            result["backend_dispatch"]["provider_error"],
            serde_json::Value::Null
        );
        assert!(!result["blocker_reason"]
            .as_str()
            .expect("blocker reason should render")
            .contains("SHOULD_NOT_LAUNCH"));

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_executes_stdin_prompt_success_result() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-stdin-success-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: pi
  systems:
    pi:
      enabled: true
      execution_class: external
      external_backend_id: pi_cli
agent_system:
  subagents:
    pi_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args:
          - -lc
          - |
            input=$(cat)
            printf '{"type":"result","is_error":false,"result":"%s"}' "$input"
        prompt_mode: stdin
        prompt_template: "STDIN_OK"
"#,
        )
        .expect("write overlay");

        let role_selection = external_test_role_selection("pi_cli");
        let receipt = external_test_receipt("pi_cli");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("pi_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should execute");

        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["provider_result"], "STDIN_OK");
        assert_eq!(result["blocker_code"], serde_json::Value::Null);
        let activation_command = result["activation_command"]
            .as_str()
            .expect("activation command should render");
        assert!(!activation_command.contains("STDIN_OK"));

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_parseable_adapter_error_json() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-stdin-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: pi
  systems:
    pi:
      enabled: true
      execution_class: external
      external_backend_id: pi_cli
agent_system:
  subagents:
    pi_cli:
      enabled: true
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args:
          - -lc
          - |
            cat >/dev/null
            printf '{"type":"result","subtype":"error_during_execution","is_error":true,"error":{"message":"adapter boom"}}'
            exit 1
        prompt_mode: stdin
        prompt_template: "STDIN_ERROR"
"#,
        )
        .expect("write overlay");

        let role_selection = external_test_role_selection("pi_cli");
        let receipt = external_test_receipt("pi_cli");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("pi_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "external"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "configured_backend_dispatch_failed");
        assert_eq!(result["provider_is_error"], true);
        assert_eq!(result["provider_error_message"], "adapter boom");
        assert_eq!(result["blocker_reason"], "adapter boom");

        let _ = std::fs::remove_dir_all(&project_root);
    }

    #[test]
    fn execute_external_agent_lane_dispatch_blocks_disabled_external_backend_before_launch() {
        let project_root = std::env::temp_dir().join(format!(
            "vida-external-dispatch-disabled-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&project_root).expect("create project root");
        std::fs::write(
            project_root.join("vida.config.yaml"),
            r#"
host_environment:
  cli_system: codex
  systems:
    codex:
      enabled: true
      execution_class: internal
agent_system:
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
    hermes_cli:
      enabled: false
      subagent_backend_class: external_cli
      dispatch:
        command: sh
        static_args: ["-c", "echo SHOULD_NOT_LAUNCH >&2; exit 99"]
        prompt_mode: positional
"#,
        )
        .expect("write overlay");

        let role_selection = external_test_role_selection("hermes_cli");
        let receipt = external_test_receipt("hermes_cli");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime
            .block_on(async {
                execute_external_agent_lane_dispatch(
                    project_root.join("missing-state").as_path(),
                    &project_root,
                    "/tmp/dispatch-packet.json",
                    Some("hermes_cli"),
                    &role_selection,
                    &receipt,
                    serde_json::json!({
                        "selected_cli_execution_class": "internal"
                    }),
                )
                .await
            })
            .expect("dispatch should return blocked result");

        assert_eq!(result["status"], "blocked");
        assert_eq!(result["execution_state"], "blocked");
        assert_eq!(result["blocker_code"], "configured_backend_dispatch_failed");
        assert_eq!(
            result["external_backend_readiness"]["status"],
            "external_backend_dispatch_blocked"
        );
        assert!(result["blocker_reason"]
            .as_str()
            .expect("blocker reason should render")
            .contains("disabled"));
        assert!(!result["blocker_reason"]
            .as_str()
            .expect("blocker reason should render")
            .contains("SHOULD_NOT_LAUNCH"));

        let _ = std::fs::remove_dir_all(&project_root);
    }

    fn external_test_role_selection(backend_id: &str) -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Run the bounded external dispatch".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "backend_admissibility_matrix": [
                    {
                        "backend_id": backend_id,
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": backend_id
                    }
                },
                "runtime_assignment": {
                    "selected_backend_id": backend_id
                }
            }),
            reason: "test".to_string(),
        }
    }

    fn external_test_receipt(backend_id: &str) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-1".to_string(),
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
            activation_agent_type: Some("worker".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some(backend_id.to_string()),
            recorded_at: "2026-04-11T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn readiness_fallback_internal_backend_uses_admissible_internal_fallback() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Review the bounded implementation".to_string(),
            selected_role: "coach".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "coach": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "coach": true
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "coach": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            super::readiness_fallback_internal_backend(&role_selection, "coach", "hermes_cli"),
            Some("internal_subagents".to_string())
        );
    }

    #[test]
    fn readiness_fallback_internal_backend_rejects_inadmissible_internal_fallback() {
        let role_selection = RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Verify the bounded implementation".to_string(),
            selected_role: "verifier".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "verification": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "verification": false
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        };

        assert_eq!(
            super::readiness_fallback_internal_backend(
                &role_selection,
                "verification",
                "hermes_cli"
            ),
            None
        );
    }
}
