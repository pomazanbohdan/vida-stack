use std::{path::Path, process::ExitCode, time::Duration};

use crate::{
    print_surface_header, print_surface_line,
    state_store::{StateStore, TaskRecord},
    taskflow_task_bridge::proxy_state_dir,
    RenderMode,
};

const TASKFLOW_PACKET_RECENT_PROJECTION_MAX_AGE: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq)]
struct PacketRenderRequest {
    requested_run_id: String,
    requested_task_id: Option<String>,
    as_json: bool,
    latest_mode: bool,
    fields: Option<String>,
}

fn projection_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn packet_projection_name(
    requested_run_id: &str,
    requested_task_id: Option<&str>,
    latest_mode: bool,
) -> String {
    if latest_mode {
        "taskflow-packet-latest".to_string()
    } else if let Some(task_id) = requested_task_id {
        format!("taskflow-packet-task-{}", projection_component(task_id))
    } else {
        format!(
            "taskflow-packet-render-{}",
            projection_component(requested_run_id)
        )
    }
}

fn packet_repair_projection_name(run_id: &str, task_id: &str) -> String {
    format!(
        "taskflow-packet-repair-{}-from-{}",
        projection_component(run_id),
        projection_component(task_id)
    )
}

fn usage() -> &'static str {
    "Usage: vida taskflow packet render <run-id> [--fields <field,...>] [--json]\n       vida taskflow packet task <task-id> [--fields <field,...>] [--json]\n       vida taskflow packet latest [--fields <field,...>] [--json]\n       vida taskflow packet repair --run-id <run-id> --from-task <task-id> [--json]"
}

fn read_packet_body(path: &str) -> Result<serde_json::Value, String> {
    let resolved_path = canonicalize_packet_path(path)?;
    let display_path = resolved_path.display().to_string();
    let body = std::fs::read_to_string(&resolved_path)
        .map_err(|error| format!("Failed to read persisted packet `{display_path}`: {error}"))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to decode persisted packet `{display_path}`: {error}"))
}

fn canonicalize_packet_path(path: &str) -> Result<std::path::PathBuf, String> {
    let candidate = Path::new(path);
    let resolved_path = candidate
        .canonicalize()
        .map_err(|error| format!("Failed to resolve persisted packet path `{path}`: {error}"))?;
    let state_root = proxy_state_dir();
    let resolved_state_root = state_root.canonicalize().map_err(|error| {
        format!(
            "Failed to resolve authoritative state root `{}`: {error}",
            state_root.display()
        )
    })?;
    if !resolved_path.starts_with(&resolved_state_root) {
        return Err(format!(
            "Persisted packet path `{}` resolves outside authoritative state root `{}`.",
            resolved_path.display(),
            resolved_state_root.display()
        ));
    }
    Ok(resolved_path)
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
        return Err("No latest persisted run-graph dispatch receipt exists; run `vida taskflow run-graph dispatch-init <task-id> --json` first.".to_string());
    };
    Ok(receipt.run_id)
}

async fn resolve_task_packet_run_id(store: &StateStore, task_id: &str) -> Result<String, String> {
    let Some(run_id) = store
        .latest_run_graph_run_id_for_task(task_id)
        .await
        .map_err(|error| {
            format!("Failed to read latest run-graph status for task `{task_id}`: {error}")
        })?
    else {
        return Err(format!(
            "No persisted run-graph status exists for task `{task_id}`; run `vida taskflow run-graph dispatch-init {task_id} --json` first."
        ));
    };
    Ok(run_id)
}

fn build_taskflow_packet_render_payload(
    requested_run_id: &str,
    run_id: &str,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    task_identity: Option<crate::state_store::RunGraphDispatchTaskIdentity>,
    dispatch_packet_path: &str,
    dispatch_packet_body: serde_json::Value,
    downstream_packet: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida taskflow packet render",
        "requested_run_id": requested_run_id,
        "run_id": run_id,
        "dispatch_receipt": receipt,
        "task_identity": task_identity,
        "dispatch_packet": {
            "path": dispatch_packet_path,
            "body": dispatch_packet_body,
        },
        "execution_preparation_artifacts": execution_preparation_artifacts_from_packet_body(
            &dispatch_packet_body
        ),
        "downstream_dispatch_packet": downstream_packet,
        "lawful_resume_inputs": {
            "run_id": run_id,
            "dispatch_packet_path": dispatch_packet_path,
            "downstream_dispatch_packet_path": receipt.downstream_dispatch_packet_path,
            "continue_command": format!("vida taskflow consume continue --run-id {} --json", receipt.run_id),
        }
    })
}

fn execution_preparation_artifacts_from_packet_body(
    dispatch_packet_body: &serde_json::Value,
) -> serde_json::Value {
    dispatch_packet_body
        .get("execution_preparation_artifacts")
        .or_else(|| {
            dispatch_packet_body
                .get("run_graph_bootstrap")
                .and_then(|value| value.get("execution_preparation_artifacts"))
        })
        .or_else(|| {
            dispatch_packet_body
                .get("taskflow_handoff_plan")
                .and_then(|value| value.get("execution_preparation_artifacts"))
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn hydrate_dispatch_packet_owned_paths_from_task(
    dispatch_packet_body: &mut serde_json::Value,
    task: &TaskRecord,
) -> bool {
    let owned_paths = &task.planner_metadata.owned_paths;
    if owned_paths.is_empty() {
        return false;
    }
    let mut hydrated = crate::runtime_dispatch_state::apply_owned_paths_if_missing(
        dispatch_packet_body,
        owned_paths,
    );
    if let Some(delivery_task_packet) = dispatch_packet_body.get_mut("delivery_task_packet") {
        hydrated |= crate::runtime_dispatch_state::apply_owned_paths_if_missing(
            delivery_task_packet,
            owned_paths,
        );
    }
    hydrated
}

fn reconcile_dispatch_packet_owned_paths_from_task(
    dispatch_packet_body: &mut serde_json::Value,
    task: &TaskRecord,
) -> bool {
    let owned_paths = &task.planner_metadata.owned_paths;
    if owned_paths.is_empty() {
        return false;
    }
    let mut repaired =
        crate::runtime_dispatch_state::apply_owned_paths(dispatch_packet_body, owned_paths);
    let Some(applied_owned_paths) = packet_string_array(dispatch_packet_body, "owned_paths") else {
        return repaired;
    };
    let packet_template_kind = packet_trimmed_string(dispatch_packet_body, "packet_template_kind")
        .unwrap_or("delivery_task_packet")
        .to_string();
    if let Some(active_packet) = dispatch_packet_body.get_mut(&packet_template_kind) {
        repaired |= crate::runtime_dispatch_state::apply_owned_paths(active_packet, owned_paths);
        let handoff_task_class =
            packet_trimmed_string(active_packet, "handoff_task_class").unwrap_or("implementation");
        let implementation_isolation =
            crate::runtime_dispatch_packets::implementation_isolation_contract(
                handoff_task_class,
                &applied_owned_paths,
            );
        if !implementation_isolation.is_null() {
            if let Some(object) = active_packet.as_object_mut() {
                if object.get("implementation_isolation") != Some(&implementation_isolation) {
                    object.insert(
                        "implementation_isolation".to_string(),
                        implementation_isolation.clone(),
                    );
                    repaired = true;
                }
            }
            if let Some(object) = dispatch_packet_body.as_object_mut() {
                if object.get("implementation_isolation") != Some(&implementation_isolation) {
                    object.insert(
                        "implementation_isolation".to_string(),
                        implementation_isolation,
                    );
                    repaired = true;
                }
            }
        }
    }
    repaired
}

fn packet_string_array(packet: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    Some(
        packet
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

fn repair_delivery_task_packet_identity(dispatch_packet_body: &mut serde_json::Value) -> bool {
    let Some(delivery_task_packet) = dispatch_packet_body
        .get_mut("delivery_task_packet")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return false;
    };
    let backlog_id = delivery_task_packet
        .get("backlog_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let Some(backlog_id) = backlog_id else {
        return false;
    };
    let mut repaired = false;
    for key in ["task_id", "id"] {
        let mismatched = delivery_task_packet
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value != backlog_id);
        if mismatched {
            delivery_task_packet.remove(key);
            repaired = true;
        }
    }
    repaired
}

#[derive(Debug)]
struct PacketRepairMutation {
    dispatch_packet_path: String,
    repaired: bool,
    contract_validated: bool,
}

fn packet_trimmed_string<'a>(packet: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    packet
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn active_repair_packet<'a>(
    packet: &'a serde_json::Value,
) -> Result<(&'a str, &'a serde_json::Value), String> {
    let packet_template_kind = packet_trimmed_string(packet, "packet_template_kind")
        .ok_or_else(|| "Persisted dispatch packet is missing packet_template_kind.".to_string())?;
    let active_packet = packet
        .get(packet_template_kind)
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            format!("Persisted dispatch packet is missing active `{packet_template_kind}` body.")
        })?;
    Ok((packet_template_kind, active_packet))
}

fn validate_packet_repair_binding(
    run_id: &str,
    task: &TaskRecord,
    status: &crate::state_store::RunGraphStatus,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    packet: &serde_json::Value,
) -> Result<(), String> {
    if receipt.run_id != run_id {
        return Err(format!(
            "Persisted dispatch receipt key `{run_id}` contains mismatched run_id `{}`.",
            receipt.run_id
        ));
    }
    if status.run_id != run_id {
        return Err(format!(
            "Persisted run-graph status for `{run_id}` contains mismatched run_id `{}`.",
            status.run_id
        ));
    }
    if status.task_id != task.id {
        return Err(format!(
            "packet repair task binding mismatch: run_id `{run_id}` is bound to task `{}`, not --from-task `{}`.",
            status.task_id, task.id
        ));
    }
    if packet_trimmed_string(packet, "run_id") != Some(receipt.run_id.as_str()) {
        return Err(format!(
            "Persisted dispatch packet run_id does not match receipt run_id `{}`.",
            receipt.run_id
        ));
    }
    if packet_trimmed_string(packet, "dispatch_target") != Some(receipt.dispatch_target.as_str()) {
        return Err(format!(
            "Persisted dispatch packet dispatch_target does not match receipt dispatch_target `{}`.",
            receipt.dispatch_target
        ));
    }
    let (_packet_template_kind, active_packet) = active_repair_packet(packet)?;
    if packet_trimmed_string(active_packet, "backlog_id") != Some(receipt.run_id.as_str()) {
        return Err(format!(
            "Persisted dispatch packet active body backlog_id does not match receipt run_id `{}`.",
            receipt.run_id
        ));
    }
    let expected_packet_prefix = format!("{}::{}::", receipt.run_id, receipt.dispatch_target);
    let packet_id = packet_trimmed_string(active_packet, "packet_id")
        .ok_or_else(|| "Persisted dispatch packet active body is missing packet_id.".to_string())?;
    if !packet_id.starts_with(&expected_packet_prefix) {
        return Err(format!(
            "Persisted dispatch packet packet_id `{packet_id}` is not bound to `{expected_packet_prefix}*`."
        ));
    }
    Ok(())
}

async fn repair_persisted_dispatch_packet_from_task(
    store: &StateStore,
    run_id: &str,
    task: &TaskRecord,
) -> Result<PacketRepairMutation, String> {
    let receipt = store
        .run_graph_dispatch_receipt(run_id)
        .await
        .map_err(|error| {
            format!("Failed to read persisted dispatch receipt for `{run_id}`: {error}")
        })?
        .ok_or_else(|| {
            format!("No persisted run-graph dispatch receipt exists for run_id `{run_id}`.")
        })?;
    let dispatch_packet_path = receipt
        .dispatch_packet_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| {
            format!("Persisted dispatch receipt for `{run_id}` is missing dispatch_packet_path.")
        })?;
    let status = store.run_graph_status(run_id).await.map_err(|error| {
        format!("Failed to read persisted run-graph status for `{run_id}`: {error}")
    })?;
    let resolved_path = canonicalize_packet_path(dispatch_packet_path)?;
    let display_path = resolved_path.display().to_string();
    let mut packet = read_packet_body(dispatch_packet_path)?;
    validate_packet_repair_binding(run_id, task, &status, &receipt, &packet)?;
    let mut repaired = repair_delivery_task_packet_identity(&mut packet);
    repaired |= reconcile_dispatch_packet_owned_paths_from_task(&mut packet, task);
    crate::validate_runtime_dispatch_packet_contract(&packet, "Repaired dispatch packet").map_err(
        |error| {
            format!("execution_preparation_gate_blocked: {error}; dispatch packet `{display_path}`")
        },
    )?;
    if repaired {
        std::fs::write(
            &resolved_path,
            serde_json::to_vec_pretty(&packet).map_err(|error| {
                format!("Failed to encode repaired packet `{display_path}`: {error}")
            })?,
        )
        .map_err(|error| format!("Failed to write repaired packet `{display_path}`: {error}"))?;
    }
    Ok(PacketRepairMutation {
        dispatch_packet_path: display_path,
        repaired,
        contract_validated: true,
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

fn parse_packet_repair_args(args: &[String]) -> Result<Option<(String, String, bool)>, String> {
    let [head, subcommand, rest @ ..] = args else {
        return Ok(None);
    };
    if head != "packet" || subcommand != "repair" {
        return Ok(None);
    }
    if rest
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        return Err(usage().to_string());
    }

    let mut run_id = None;
    let mut task_id = None;
    let mut as_json = false;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--run-id" => {
                index += 1;
                let Some(value) = rest.get(index) else {
                    return Err("Missing value for --run-id.".to_string());
                };
                run_id = Some(value.clone());
            }
            "--from-task" => {
                index += 1;
                let Some(value) = rest.get(index) else {
                    return Err("Missing value for --from-task.".to_string());
                };
                task_id = Some(value.clone());
            }
            "--json" => as_json = true,
            other => return Err(format!("Unsupported packet repair argument `{other}`.")),
        }
        index += 1;
    }

    let run_id = run_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "packet repair requires --run-id <id>.".to_string())?;
    let task_id = task_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "packet repair requires --from-task <task-id>.".to_string())?;

    Ok(Some((run_id, task_id, as_json)))
}

fn parse_packet_render_args(args: &[String]) -> Result<Option<PacketRenderRequest>, String> {
    let [head, subcommand, rest @ ..] = args else {
        return Ok(None);
    };
    if head != "packet" || !matches!(subcommand.as_str(), "render" | "task" | "latest") {
        return Ok(None);
    }
    if rest
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        return Err(usage().to_string());
    }

    let mut as_json = false;
    let mut fields = None;
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--json" => as_json = true,
            "--fields" => {
                index += 1;
                let Some(value) = rest.get(index) else {
                    return Err("Missing value for --fields.".to_string());
                };
                let value = value.trim();
                if value.is_empty() {
                    return Err("Missing value for --fields.".to_string());
                }
                fields = Some(value.to_string());
            }
            other if other.starts_with("--") => {
                return Err(format!("Unsupported packet render argument `{other}`."));
            }
            other => positionals.push(other.to_string()),
        }
        index += 1;
    }

    let (requested_run_id, requested_task_id, latest_mode) = match subcommand.as_str() {
        "render" => {
            if positionals.len() != 1 {
                return Err("packet render requires exactly one <run-id>.".to_string());
            }
            (positionals[0].clone(), None, false)
        }
        "task" => {
            if positionals.len() != 1 {
                return Err("packet task requires exactly one <task-id>.".to_string());
            }
            (positionals[0].clone(), Some(positionals[0].clone()), false)
        }
        "latest" => {
            if !positionals.is_empty() {
                return Err("packet latest does not accept a positional id.".to_string());
            }
            ("latest".to_string(), None, true)
        }
        _ => unreachable!("subcommand checked above"),
    };

    Ok(Some(PacketRenderRequest {
        requested_run_id,
        requested_task_id,
        as_json,
        latest_mode,
        fields,
    }))
}

fn packet_repair_args_request_json(args: &[String]) -> bool {
    matches!(args, [head, subcommand, ..] if head == "packet" && subcommand == "repair")
        && args.iter().any(|arg| arg == "--json")
}

fn packet_repair_parse_error_payload(error: &str) -> serde_json::Value {
    let blocker_code =
        if error.contains("requires --run-id") || error.contains("Missing value for --run-id") {
            "packet_repair_run_id_missing"
        } else if error.contains("requires --from-task")
            || error.contains("Missing value for --from-task")
        {
            "packet_repair_from_task_missing"
        } else {
            "packet_repair_argument_invalid"
        };
    serde_json::json!({
        "surface": "vida taskflow packet repair",
        "status": "blocked",
        "blocker_codes": [blocker_code],
        "error": error,
        "usage": usage(),
        "next_actions": [
            "Rerun as `vida taskflow packet repair --run-id <run-id> --from-task <task-id> --json`."
        ],
    })
}

async fn load_task_for_packet_repair(
    store: &StateStore,
    task_id: &str,
) -> Result<TaskRecord, String> {
    let tasks = store
        .all_tasks()
        .await
        .map_err(|error| format!("Failed to read canonical task metadata: {error}"))?;
    tasks
        .into_iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| format!("No canonical task `{task_id}` exists."))
}

fn task_has_packet_repair_metadata(task: &TaskRecord) -> bool {
    !task.planner_metadata.owned_paths.is_empty()
        && (!task.planner_metadata.proof_targets.is_empty()
            || !task.planner_metadata.acceptance_targets.is_empty())
}

fn build_taskflow_packet_repair_payload(
    run_id: &str,
    task: Option<&TaskRecord>,
    load_error: Option<&str>,
) -> serde_json::Value {
    let dispatch_init_command =
        task.map(|task| format!("vida taskflow run-graph dispatch-init {} --json", task.id));
    let render_command = format!("vida taskflow packet render {run_id} --json");
    let repair_command = task.map(|task| {
        format!(
            "vida taskflow packet repair --run-id {run_id} --from-task {} --json",
            task.id
        )
    });

    let mut blocker_codes = Vec::new();
    if load_error.is_some() {
        blocker_codes.push("task_metadata_not_found");
    } else if let Some(task) = task {
        if task.planner_metadata.owned_paths.is_empty() {
            blocker_codes.push("task_metadata_missing_owned_paths");
        }
        if task.planner_metadata.proof_targets.is_empty()
            && task.planner_metadata.acceptance_targets.is_empty()
        {
            blocker_codes.push("task_metadata_missing_proof_or_acceptance_targets");
        }
    }

    let status = if blocker_codes.is_empty() {
        "repair_ready"
    } else {
        "blocked"
    };

    let task_metadata = task.map(|task| {
        serde_json::json!({
            "task_id": task.id,
            "title": task.title,
            "status": task.status,
            "issue_type": task.issue_type,
            "planner_metadata": task.planner_metadata,
        })
    });

    let next_actions = if status == "repair_ready" {
        serde_json::json!([
            dispatch_init_command.clone().unwrap_or_default(),
            render_command,
        ])
    } else if load_error.is_some() {
        serde_json::json!([
            format!(
                "vida task show {}",
                task.map(|task| task.id.as_str()).unwrap_or("<task-id>")
            ),
            "If the task is missing and metadata is known, create it in one command: vida task create <task-id> <title> --parent-id <parent-id> --owned-path <path> --acceptance-target \"...\" --proof-target \"...\".".to_string(),
        ])
    } else {
        serde_json::json!([
            format!(
                "vida task update {} --owned-path <path> --proof-target <command> --json",
                task.map(|task| task.id.as_str()).unwrap_or("<task-id>")
            ),
            repair_command.clone().unwrap_or_default(),
        ])
    };

    serde_json::json!({
        "surface": "vida taskflow packet repair",
        "status": status,
        "run_id": run_id,
        "from_task": task.map(|task| task.id.as_str()),
        "task_metadata": task_metadata,
        "metadata_complete": task.map(task_has_packet_repair_metadata).unwrap_or(false),
        "blocker_codes": blocker_codes,
        "load_error": load_error,
        "repair_model": "rebind_from_canonical_task_metadata",
        "repair_command": repair_command,
        "dispatch_init_command": dispatch_init_command,
        "bind_command": dispatch_init_command,
        "render_command": render_command,
        "next_actions": next_actions,
    })
}

async fn run_taskflow_packet_repair(run_id: &str, task_id: &str, as_json: bool) -> ExitCode {
    let store = match StateStore::open_existing(proxy_state_dir()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    let loaded_task = load_task_for_packet_repair(&store, task_id).await;
    let (task, load_error) = match loaded_task {
        Ok(task) => (Some(task), None),
        Err(error) => (None, Some(error)),
    };
    let mut payload =
        build_taskflow_packet_repair_payload(run_id, task.as_ref(), load_error.as_deref());
    if payload["status"].as_str() == Some("repair_ready") {
        if let Some(task) = task.as_ref() {
            match repair_persisted_dispatch_packet_from_task(&store, run_id, task).await {
                Ok(mutation) => {
                    payload["repair_applied"] = serde_json::json!(mutation.repaired);
                    payload["contract_validated"] = serde_json::json!(mutation.contract_validated);
                    payload["dispatch_packet_path"] =
                        serde_json::json!(mutation.dispatch_packet_path);
                }
                Err(error) => {
                    payload["status"] = serde_json::json!("blocked");
                    payload["repair_error"] = serde_json::json!(error);
                    if let Some(blockers) = payload["blocker_codes"].as_array_mut() {
                        blockers.push(serde_json::json!("dispatch_packet_repair_failed"));
                    }
                }
            }
        }
    }

    if as_json {
        crate::print_json_pretty(&payload);
        crate::operator_projection_cache::write_json_projection(
            &proxy_state_dir(),
            &packet_repair_projection_name(run_id, task_id),
            &payload,
        );
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow packet repair");
        print_surface_line(
            RenderMode::Plain,
            "status",
            payload["status"].as_str().unwrap_or("blocked"),
        );
        print_surface_line(RenderMode::Plain, "run", run_id);
        print_surface_line(RenderMode::Plain, "from_task", task_id);
        if let Some(command) = payload["dispatch_init_command"].as_str() {
            print_surface_line(RenderMode::Plain, "bind_command", command);
        }
        if let Some(blockers) = payload["blocker_codes"].as_array() {
            let blocker_codes = blockers
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            if !blocker_codes.is_empty() {
                print_surface_line(RenderMode::Plain, "blocker_codes", &blocker_codes.join(","));
            }
        }
        if let Some(error) = payload["load_error"].as_str() {
            print_surface_line(RenderMode::Plain, "blocker", error);
        }
        if let Some(error) = payload["repair_error"].as_str() {
            print_surface_line(RenderMode::Plain, "repair_error", error);
        }
        if let Some(actions) = payload["next_actions"].as_array() {
            for (index, action) in actions
                .iter()
                .filter_map(serde_json::Value::as_str)
                .enumerate()
            {
                let label = format!("next_action_{}", index + 1);
                print_surface_line(RenderMode::Plain, &label, action);
            }
        }
    }

    if payload["status"].as_str() == Some("repair_ready") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
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

    match parse_packet_repair_args(args) {
        Ok(Some((run_id, task_id, as_json))) => {
            return run_taskflow_packet_repair(&run_id, &task_id, as_json).await;
        }
        Ok(None) => {}
        Err(error) if error.starts_with("Usage:") => {
            println!("{error}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            if packet_repair_args_request_json(args) {
                crate::print_json_pretty(&packet_repair_parse_error_payload(&error));
                return ExitCode::from(2);
            }
            eprintln!("{error}");
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
    }

    let render_request = match parse_packet_render_args(args) {
        Ok(Some(request)) => request,
        Ok(None) => {
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
        Err(error) if error.starts_with("Usage:") => {
            println!("{error}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
    };
    let requested_run_id = render_request.requested_run_id;
    let requested_task_id = render_request.requested_task_id;
    let as_json = render_request.as_json;
    let latest_mode = render_request.latest_mode;
    let field_selection = render_request.fields;
    let selected_output = field_selection.is_some();

    let state_root = proxy_state_dir();
    let projection_name =
        packet_projection_name(&requested_run_id, requested_task_id.as_deref(), latest_mode);
    if as_json {
        // Security hardening: packet JSON must come from authoritative state validation
        // (state store + dispatch receipt + canonical packet path checks), not raw cache.
    }

    let store = match StateStore::open_existing(state_root.clone()).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };
    let run_id = if let Some(task_id) = requested_task_id.as_deref() {
        match resolve_task_packet_run_id(&store, task_id).await {
            Ok(run_id) => run_id,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    } else if latest_mode {
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
    let receipt_from_store = match store.run_graph_dispatch_receipt(&effective_run_id).await {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!(
                "Failed to read persisted dispatch receipt for `{effective_run_id}`: {error}"
            );
            return ExitCode::from(1);
        }
    };
    let Some(receipt) = receipt_from_store else {
        eprintln!(
            "No persisted run-graph dispatch receipt exists for run_id `{effective_run_id}`. Run `vida taskflow run-graph dispatch-init {run_id} --json` first."
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
    let mut dispatch_packet_body = match read_packet_body(dispatch_packet_path) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let task_identity = match store
        .run_graph_dispatch_task_identity(&effective_run_id)
        .await
    {
        Ok(identity) => identity,
        Err(error) => {
            eprintln!("Failed to read dispatch task identity for `{effective_run_id}`: {error}");
            return ExitCode::from(1);
        }
    };
    let metadata_task_id = task_identity
        .as_ref()
        .and_then(|identity| {
            identity
                .dev_task_id
                .as_deref()
                .or(identity.work_pool_task_id.as_deref())
                .or(identity.spec_task_id.as_deref())
                .or(identity.feature_epic_id.as_deref())
        })
        .unwrap_or(&effective_run_id);
    if let Ok(task) = load_task_for_packet_repair(&store, metadata_task_id).await {
        hydrate_dispatch_packet_owned_paths_from_task(&mut dispatch_packet_body, &task);
    }
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
        task_identity,
        dispatch_packet_path,
        dispatch_packet_body.clone(),
        downstream_packet,
    );

    if let Some(fields) = field_selection.as_deref() {
        let selected_payload =
            operator_output::toon_report::select_fields(payload.clone(), Some(fields));
        if as_json {
            crate::print_json_pretty(&selected_payload);
        } else {
            println!(
                "{}",
                operator_output::toon_report::render_value(
                    "vida taskflow packet fields",
                    selected_payload,
                )
            );
        }
    } else if as_json {
        crate::print_json_pretty(&payload);
        if !selected_output {
            crate::operator_projection_cache::write_json_projection(
                &proxy_state_dir(),
                &projection_name,
                &payload,
            );
        }
    } else {
        print_surface_header(RenderMode::Plain, "vida taskflow packet render");
        if latest_mode {
            print_surface_line(RenderMode::Plain, "requested", "latest");
        }
        if let Some(task_id) = requested_task_id.as_deref() {
            print_surface_line(RenderMode::Plain, "requested_task", task_id);
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
            preview_value(
                &dispatch_packet_body,
                "route_policy",
                "effective_selected_backend",
            ),
        );
        print_surface_line(
            RenderMode::Plain,
            "route_policy",
            &format!(
                "primary_backend={} backend_source={} posture={}",
                preview_value(
                    &dispatch_packet_body,
                    "route_policy",
                    "route_primary_backend"
                ),
                preview_value(
                    &dispatch_packet_body,
                    "route_policy",
                    "selected_backend_source"
                ),
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
            &format!("vida taskflow consume continue --run-id {}", receipt.run_id),
        );
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        build_taskflow_packet_render_payload, build_taskflow_packet_repair_payload,
        hydrate_dispatch_packet_owned_paths_from_task, packet_repair_projection_name,
        parse_packet_render_args, parse_packet_repair_args, repair_delivery_task_packet_identity,
        repair_persisted_dispatch_packet_from_task, resolve_latest_packet_run_id,
        resolve_packet_render_run_id, run_taskflow_packet,
    };
    use crate::state_store::{
        CreateTaskRequest, ExecutionPlanStateRow, RunGraphDispatchReceiptStored, StateStore,
        TaskExecutionSemantics, TaskPlannerMetadata, TaskRecord, STATE_DATABASE, STATE_NAMESPACE,
    };
    use std::fs;
    use std::process::ExitCode;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn packet_render_args_accept_field_selection_with_json() {
        let args = vec![
            "packet".to_string(),
            "render".to_string(),
            "run-impl".to_string(),
            "--fields".to_string(),
            "dispatch_packet.path,dispatch_receipt.dispatch_target".to_string(),
            "--json".to_string(),
        ];

        let request = parse_packet_render_args(&args)
            .expect("render args parse")
            .expect("render request");

        assert_eq!(request.requested_run_id, "run-impl");
        assert_eq!(request.requested_task_id, None);
        assert!(request.as_json);
        assert!(!request.latest_mode);
        assert_eq!(
            request.fields.as_deref(),
            Some("dispatch_packet.path,dispatch_receipt.dispatch_target")
        );
    }

    #[test]
    fn packet_render_field_selection_extracts_compact_dispatch_fields() {
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };
        let payload = build_taskflow_packet_render_payload(
            "run-impl",
            "run-impl",
            &receipt,
            None,
            "/tmp/dispatch-packet.json",
            serde_json::json!({
                "route_policy": {
                    "effective_selected_backend": "internal_subagents",
                    "owner_function": "write_runtime_dispatch_packet"
                },
                "large_runtime_artifact": "x".repeat(4096),
            }),
            None,
        );

        let selected = operator_output::toon_report::select_fields(
            payload,
            Some(
                "run_id,dispatch_packet.path,dispatch_packet.body.route_policy.effective_selected_backend",
            ),
        );

        assert_eq!(selected["run_id"], "run-impl");
        assert_eq!(
            selected["dispatch_packet"]["path"],
            "/tmp/dispatch-packet.json"
        );
        assert_eq!(
            selected["dispatch_packet"]["body"]["route_policy"]["effective_selected_backend"],
            "internal_subagents"
        );
        assert!(selected["dispatch_packet"]["body"]
            .get("large_runtime_artifact")
            .is_none());
    }

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
            None,
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

    #[test]
    fn packet_render_payload_surfaces_execution_preparation_artifacts() {
        let receipt = crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-prep".to_string(),
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
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-14T00:00:00Z".to_string(),
        };

        let payload = build_taskflow_packet_render_payload(
            "run-prep",
            "run-prep",
            &receipt,
            None,
            "/tmp/dispatch-packet.json",
            serde_json::json!({
                "run_graph_bootstrap": {
                    "execution_preparation_artifacts": {
                        "handoff_ready": true,
                        "developer_handoff_packet": {
                            "ready": true,
                            "path": "/tmp/handoff.json"
                        },
                        "architecture_preparation_report": {
                            "ready": true,
                            "path": "/tmp/architecture.json"
                        }
                    }
                }
            }),
            None,
        );

        assert_eq!(
            payload["execution_preparation_artifacts"]["developer_handoff_packet"]["path"],
            "/tmp/handoff.json"
        );
        assert_eq!(
            payload["execution_preparation_artifacts"]["architecture_preparation_report"]["ready"],
            true
        );
    }

    fn packet_repair_task_with_metadata() -> TaskRecord {
        TaskRecord {
            id: "task-with-metadata".to_string(),
            display_id: None,
            title: "Task with metadata".to_string(),
            description: String::new(),
            status: "open".to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: String::new(),
            created_by: "test".to_string(),
            updated_at: String::new(),
            closed_at: None,
            close_reason: None,
            source_repo: "vida-stack".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: vec![],
            execution_semantics: Default::default(),
            planner_metadata: TaskPlannerMetadata {
                owned_paths: vec!["crates/vida/src/taskflow_packet.rs".to_string()],
                acceptance_targets: vec!["repair command is discoverable".to_string()],
                proof_targets: vec!["cargo test -p vida packet_repair -- --nocapture".to_string()],
                risk: None,
                estimate: None,
                lane_hint: None,
            },
            provider_mapping: None,
            dependencies: vec![],
        }
    }

    fn packet_repair_temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "vida-packet-repair-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn packet_repair_status(run_id: &str, task_id: &str) -> crate::state_store::RunGraphStatus {
        crate::state_store::RunGraphStatus {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            task_class: "delivery_task".to_string(),
            active_node: "implementer".to_string(),
            next_node: None,
            status: "in_progress".to_string(),
            route_task_class: "delivery_task".to_string(),
            selected_backend: "opencode_cli".to_string(),
            lane_id: format!("lane-{run_id}"),
            lifecycle_stage: "implementer_ready".to_string(),
            policy_gate: "none".to_string(),
            handoff_state: "none".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "none".to_string(),
            resume_target: "dispatch.implementer".to_string(),
            recovery_ready: false,
        }
    }

    fn packet_repair_receipt(
        run_id: &str,
        packet_path: &std::path::Path,
    ) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: run_id.to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "ready".to_string(),
            lane_status: "lane_ready".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida taskflow run-graph dispatch-init".to_string()),
            dispatch_command: Some(format!(
                "vida taskflow consume continue --run-id {run_id} --json"
            )),
            dispatch_packet_path: Some(packet_path.display().to_string()),
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
        }
    }

    fn packet_repair_dispatch_packet(run_id: &str) -> serde_json::Value {
        let mut delivery_task_packet =
            crate::runtime_dispatch_packets::runtime_delivery_task_packet(
                run_id,
                "implementer",
                "worker",
                "implementation",
                "delivery",
                "bounded repair test",
            );
        delivery_task_packet["owned_paths"] = serde_json::json!([]);
        serde_json::json!({
            "packet_kind": "runtime_dispatch_packet",
            "packet_template_kind": "delivery_task_packet",
            "run_id": run_id,
            "dispatch_target": "implementer",
            "owned_paths": [],
            "delivery_task_packet": delivery_task_packet,
        })
    }

    async fn packet_repair_binding_assert_rejection(
        label: &str,
        lookup_run_id: &str,
        status_row_run_id: &str,
        receipt_row_run_id: &str,
        task_id: &str,
        packet: serde_json::Value,
        expected_fragment: &str,
    ) {
        let root = packet_repair_temp_root(label);
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));
        let result = async {
            fs::create_dir_all(&root).expect("create state root");
            let mut task = packet_repair_task_with_metadata();
            task.id = task_id.to_string();
            let packet_path = root
                .join("runtime-consumption")
                .join(format!("{lookup_run_id}.json"));
            fs::create_dir_all(packet_path.parent().expect("packet parent"))
                .expect("create packet parent");
            let original_packet = packet.clone();
            fs::write(
                &packet_path,
                serde_json::to_vec_pretty(&original_packet).expect("encode packet"),
            )
            .expect("write packet");
            {
                let raw_db: surrealdb::Surreal<surrealdb::engine::local::Db> =
                    surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(root.clone())
                        .await
                        .expect("open raw db");
                raw_db
                    .use_ns(STATE_NAMESPACE)
                    .use_db(STATE_DATABASE)
                    .await
                    .expect("bind raw db");
                let _: Option<ExecutionPlanStateRow> = raw_db
                    .upsert(("execution_plan_state", lookup_run_id))
                    .content(ExecutionPlanStateRow {
                        run_id: status_row_run_id.to_string(),
                        task_id: task.id.clone(),
                        task_class: "delivery_task".to_string(),
                        active_node: "implementer".to_string(),
                        next_node: None,
                        status: "in_progress".to_string(),
                        updated_at: "2026-04-19T12:00:01Z".to_string(),
                    })
                    .await
                    .expect("persist status row");
                let mut receipt_row = RunGraphDispatchReceiptStored::from(packet_repair_receipt(
                    receipt_row_run_id,
                    &packet_path,
                ));
                receipt_row.run_id = receipt_row_run_id.to_string();
                let _: Option<RunGraphDispatchReceiptStored> = raw_db
                    .upsert(("run_graph_dispatch_receipt", lookup_run_id))
                    .content(receipt_row)
                    .await
                    .expect("persist receipt row");
            }
            let store = StateStore::open(root.clone()).await.expect("open store");

            let error = repair_persisted_dispatch_packet_from_task(&store, lookup_run_id, &task)
                .await
                .expect_err("expected rejection");

            assert!(error.contains(expected_fragment), "{error}");
            let persisted: serde_json::Value =
                serde_json::from_slice(&fs::read(&packet_path).expect("read packet"))
                    .expect("decode packet");
            assert_eq!(persisted, original_packet);
        }
        .await;
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        let _ = fs::remove_dir_all(&root);
        result
    }

    fn packet_repair_binding_apply(packet: &mut serde_json::Value, mutation: &PacketMutation) {
        match mutation {
            PacketMutation::None => {}
            PacketMutation::TopLevelRunId(value) => packet["run_id"] = serde_json::json!(value),
            PacketMutation::DispatchTarget(value) => {
                packet["dispatch_target"] = serde_json::json!(value)
            }
            PacketMutation::BacklogId(value) => {
                packet["delivery_task_packet"]["backlog_id"] = serde_json::json!(value)
            }
            PacketMutation::PacketId(value) => {
                packet["delivery_task_packet"]["packet_id"] = serde_json::json!(value)
            }
            PacketMutation::InvalidTemplateKind(value) => {
                packet["packet_template_kind"] = serde_json::json!(value)
            }
            PacketMutation::MissingTemplateKind => {
                packet
                    .as_object_mut()
                    .expect("packet object")
                    .remove("packet_template_kind");
            }
            PacketMutation::MissingActiveBody => {
                packet
                    .as_object_mut()
                    .expect("packet object")
                    .remove("delivery_task_packet");
            }
        }
    }

    enum PacketMutation {
        None,
        TopLevelRunId(&'static str),
        DispatchTarget(&'static str),
        BacklogId(&'static str),
        PacketId(&'static str),
        InvalidTemplateKind(usize),
        MissingTemplateKind,
        MissingActiveBody,
    }

    #[test]
    fn packet_repair_args_require_run_id_and_from_task() {
        let args = vec![
            "packet".to_string(),
            "repair".to_string(),
            "--run-id".to_string(),
            "run-1".to_string(),
            "--from-task".to_string(),
            "task-1".to_string(),
            "--json".to_string(),
        ];

        let parsed = parse_packet_repair_args(&args)
            .expect("valid repair args")
            .expect("repair args");

        assert_eq!(parsed, ("run-1".to_string(), "task-1".to_string(), true));
    }

    #[test]
    fn packet_repair_payload_reports_rebind_plan_from_task_metadata() {
        let task = packet_repair_task_with_metadata();

        let payload = build_taskflow_packet_repair_payload("run-1", Some(&task), None);

        assert_eq!(payload["status"], "repair_ready");
        assert_eq!(payload["metadata_complete"], true);
        assert_eq!(
            payload["bind_command"],
            "vida taskflow run-graph dispatch-init task-with-metadata --json"
        );
        assert_eq!(
            payload["task_metadata"]["planner_metadata"]["owned_paths"][0],
            "crates/vida/src/taskflow_packet.rs"
        );
    }

    #[tokio::test]
    async fn packet_repair_rejects_binding_mismatches_without_mutating_packet() {
        struct Case {
            label: &'static str,
            status_row_run_id: &'static str,
            receipt_row_run_id: &'static str,
            expected_fragment: &'static str,
            mutation: PacketMutation,
        }

        let cases = [
            Case {
                label: "receipt-run-id-mismatch",
                status_row_run_id: "run-packet-binding",
                receipt_row_run_id: "run-receipt-mismatch",
                expected_fragment: "Persisted dispatch receipt key",
                mutation: PacketMutation::None,
            },
            Case {
                label: "status-run-id-mismatch",
                status_row_run_id: "run-status-mismatch",
                receipt_row_run_id: "run-packet-binding",
                expected_fragment: "Persisted run-graph status",
                mutation: PacketMutation::None,
            },
            Case {
                label: "packet-run-id-mismatch",
                status_row_run_id: "run-packet-binding",
                receipt_row_run_id: "run-packet-binding",
                expected_fragment: "Persisted dispatch packet run_id does not match",
                mutation: PacketMutation::TopLevelRunId("run-packet-mismatch"),
            },
            Case {
                label: "dispatch-target-mismatch",
                status_row_run_id: "run-packet-binding",
                receipt_row_run_id: "run-packet-binding",
                expected_fragment: "Persisted dispatch packet dispatch_target does not match",
                mutation: PacketMutation::DispatchTarget("orchestrator"),
            },
            Case {
                label: "backlog-id-mismatch",
                status_row_run_id: "run-packet-binding",
                receipt_row_run_id: "run-packet-binding",
                expected_fragment: "backlog_id does not match receipt run_id",
                mutation: PacketMutation::BacklogId("run-other"),
            },
            Case {
                label: "packet-id-prefix-mismatch",
                status_row_run_id: "run-packet-binding",
                receipt_row_run_id: "run-packet-binding",
                expected_fragment: "is not bound to `run-packet-binding::implementer::",
                mutation: PacketMutation::PacketId("run-other::implementer::delivery"),
            },
        ];

        for case in cases {
            let mut packet = packet_repair_dispatch_packet("run-packet-binding");
            packet_repair_binding_apply(&mut packet, &case.mutation);
            packet_repair_binding_assert_rejection(
                case.label,
                "run-packet-binding",
                case.status_row_run_id,
                case.receipt_row_run_id,
                "task-binding",
                packet,
                case.expected_fragment,
            )
            .await;
        }
    }

    #[tokio::test]
    async fn packet_repair_rejects_missing_or_invalid_template_kind_and_active_body() {
        struct Case {
            label: &'static str,
            expected_fragment: &'static str,
            mutation: PacketMutation,
        }

        let cases = [
            Case {
                label: "missing-template-kind",
                expected_fragment: "missing packet_template_kind",
                mutation: PacketMutation::MissingTemplateKind,
            },
            Case {
                label: "invalid-template-kind",
                expected_fragment: "missing packet_template_kind",
                mutation: PacketMutation::InvalidTemplateKind(123),
            },
            Case {
                label: "missing-active-body",
                expected_fragment: "missing active `delivery_task_packet` body",
                mutation: PacketMutation::MissingActiveBody,
            },
        ];

        for case in cases {
            let mut packet = packet_repair_dispatch_packet("run-template");
            packet_repair_binding_apply(&mut packet, &case.mutation);
            packet_repair_binding_assert_rejection(
                case.label,
                "run-template",
                "run-template",
                "run-template",
                "task-template",
                packet,
                case.expected_fragment,
            )
            .await;
        }
    }

    #[tokio::test]
    async fn packet_repair_hydrates_only_receipt_task_bound_packet() {
        let root = packet_repair_temp_root("bound-success");
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));
        let result = async {
            fs::create_dir_all(&root).expect("create state root");
            let store = StateStore::open(root.clone()).await.expect("open store");
            let mut task = packet_repair_task_with_metadata();
            task.id = "task-bound".to_string();
            let run_id = "run-bound";
            store
                .record_run_graph_status(&packet_repair_status(run_id, &task.id))
                .await
                .expect("persist status");
            let packet_path = root.join("runtime-consumption").join("run-bound.json");
            fs::create_dir_all(packet_path.parent().expect("packet parent"))
                .expect("create packet parent");
            fs::write(
                &packet_path,
                serde_json::to_vec_pretty(&packet_repair_dispatch_packet(run_id))
                    .expect("encode packet"),
            )
            .expect("write packet");
            store
                .record_run_graph_dispatch_receipt(&packet_repair_receipt(run_id, &packet_path))
                .await
                .expect("persist receipt");

            let mutation = repair_persisted_dispatch_packet_from_task(&store, run_id, &task)
                .await
                .expect("repair bound packet");

            assert!(mutation.repaired);
            assert!(mutation.contract_validated);
            let repaired: serde_json::Value =
                serde_json::from_slice(&fs::read(&packet_path).expect("read repaired packet"))
                    .expect("decode repaired packet");
            assert_eq!(
                repaired["owned_paths"],
                serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
            );
            assert_eq!(
                repaired["delivery_task_packet"]["owned_paths"],
                serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
            );
        }
        .await;
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        let _ = fs::remove_dir_all(&root);
        result
    }

    #[tokio::test]
    async fn packet_repair_reconciles_stale_owned_path_mirrors_from_task_metadata() {
        let root = packet_repair_temp_root("stale-owned-path-mirrors");
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));
        let result = async {
            fs::create_dir_all(&root).expect("create state root");
            let store = StateStore::open(root.clone()).await.expect("open store");
            let mut task = packet_repair_task_with_metadata();
            task.id = "task-stale-owned-paths".to_string();
            task.planner_metadata.owned_paths = vec![
                "crates/vida/src/taskflow_packet.rs".to_string(),
                "crates/vida/src/agent_dispatch_surface.rs".to_string(),
            ];
            let run_id = "run-stale-owned-paths";
            store
                .record_run_graph_status(&packet_repair_status(run_id, &task.id))
                .await
                .expect("persist status");
            let packet_path = root
                .join("runtime-consumption")
                .join("run-stale-owned-paths.json");
            fs::create_dir_all(packet_path.parent().expect("packet parent"))
                .expect("create packet parent");
            let mut packet = packet_repair_dispatch_packet(run_id);
            packet["owned_paths"] = serde_json::json!(["crates/vida/src/taskflow_packet.rs"]);
            packet["delivery_task_packet"]["owned_paths"] =
                serde_json::json!(["crates/vida/src/taskflow_packet.rs"]);
            fs::write(
                &packet_path,
                serde_json::to_vec_pretty(&packet).expect("encode packet"),
            )
            .expect("write packet");
            store
                .record_run_graph_dispatch_receipt(&packet_repair_receipt(run_id, &packet_path))
                .await
                .expect("persist receipt");

            let mutation = repair_persisted_dispatch_packet_from_task(&store, run_id, &task)
                .await
                .expect("repair stale owned-path mirrors");

            assert!(mutation.repaired);
            assert!(mutation.contract_validated);
            let repaired: serde_json::Value =
                serde_json::from_slice(&fs::read(&packet_path).expect("read repaired packet"))
                    .expect("decode repaired packet");
            let expected_paths = serde_json::json!([
                "crates/vida/src/taskflow_packet.rs",
                "crates/vida/src/agent_dispatch_surface.rs"
            ]);
            assert_eq!(repaired["owned_paths"], expected_paths);
            assert_eq!(
                repaired["delivery_task_packet"]["owned_paths"],
                expected_paths
            );
            assert_eq!(
                repaired["implementation_isolation"]["owned_paths"],
                expected_paths
            );
            assert_eq!(
                repaired["delivery_task_packet"]["implementation_isolation"]["owned_paths"],
                expected_paths
            );
        }
        .await;
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        let _ = fs::remove_dir_all(&root);
        result
    }

    #[tokio::test]
    async fn packet_repair_json_cli_rejects_binding_mismatch_without_mutating_packet() {
        let root = packet_repair_temp_root("json-cli-mismatch");
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));
        let result = async {
            fs::create_dir_all(&root).expect("create state root");
            let store = StateStore::open(root.clone()).await.expect("open store");
            let mut task = packet_repair_task_with_metadata();
            task.id = "task-cli".to_string();
            let run_id = "run-cli";
            store
                .create_task(CreateTaskRequest {
                    task_id: &task.id,
                    title: &task.title,
                    display_id: None,
                    description: &task.description,
                    issue_type: "epic",
                    status: &task.status,
                    priority: task.priority,
                    parent_id: None,
                    labels: &task.labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: task.planner_metadata.clone(),
                    created_by: "test",
                    source_repo: &task.source_repo,
                })
                .await
                .expect("create canonical task");
            store
                .record_run_graph_status(&packet_repair_status(run_id, &task.id))
                .await
                .expect("persist status");
            let packet_path = root.join("runtime-consumption").join("run-cli.json");
            fs::create_dir_all(packet_path.parent().expect("packet parent"))
                .expect("create packet parent");
            let mut original_packet = packet_repair_dispatch_packet(run_id);
            original_packet["run_id"] = serde_json::json!("run-cli-mismatch");
            fs::write(
                &packet_path,
                serde_json::to_vec_pretty(&original_packet).expect("encode packet"),
            )
            .expect("write packet");
            store
                .record_run_graph_dispatch_receipt(&packet_repair_receipt(run_id, &packet_path))
                .await
                .expect("persist receipt");
            drop(store);

            let args = vec![
                "packet".to_string(),
                "repair".to_string(),
                "--run-id".to_string(),
                run_id.to_string(),
                "--from-task".to_string(),
                task.id.clone(),
                "--json".to_string(),
            ];
            let exit_code = run_taskflow_packet(&args).await;

            assert_eq!(exit_code, ExitCode::from(1));
            let projection = crate::operator_projection_cache::read_state_recent_json_projection(
                &root,
                &packet_repair_projection_name(run_id, &task.id),
                Duration::from_secs(60),
            )
            .expect("read packet repair projection");
            let payload: serde_json::Value =
                serde_json::from_str(&projection).expect("decode projection");
            assert_eq!(payload["status"], "blocked");
            assert_eq!(payload["from_task"], task.id);
            assert!(payload["blocker_codes"]
                .as_array()
                .expect("blocker codes")
                .iter()
                .any(|code| code == "dispatch_packet_repair_failed"));
            let repair_error = payload["repair_error"].as_str().expect("repair error");
            assert!(
                repair_error.contains("Persisted dispatch packet run_id does not match"),
                "{repair_error}"
            );
            let persisted: serde_json::Value =
                serde_json::from_slice(&fs::read(&packet_path).expect("read packet"))
                    .expect("decode packet");
            assert_eq!(persisted, original_packet);
        }
        .await;
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        let _ = fs::remove_dir_all(&root);
        result
    }

    #[tokio::test]
    async fn packet_repair_rejects_from_task_mismatch_without_mutating_packet() {
        let root = packet_repair_temp_root("task-mismatch");
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));
        let result = async {
            fs::create_dir_all(&root).expect("create state root");
            let store = StateStore::open(root.clone()).await.expect("open store");
            let mut task = packet_repair_task_with_metadata();
            task.id = "attacker-task".to_string();
            let run_id = "run-victim";
            store
                .record_run_graph_status(&packet_repair_status(run_id, "victim-task"))
                .await
                .expect("persist status");
            let packet_path = root.join("runtime-consumption").join("run-victim.json");
            fs::create_dir_all(packet_path.parent().expect("packet parent"))
                .expect("create packet parent");
            let original_packet = packet_repair_dispatch_packet(run_id);
            fs::write(
                &packet_path,
                serde_json::to_vec_pretty(&original_packet).expect("encode packet"),
            )
            .expect("write packet");
            store
                .record_run_graph_dispatch_receipt(&packet_repair_receipt(run_id, &packet_path))
                .await
                .expect("persist receipt");

            let error = repair_persisted_dispatch_packet_from_task(&store, run_id, &task)
                .await
                .expect_err("mismatched from-task must be rejected");

            assert!(error.contains("task binding mismatch"), "{error}");
            let persisted: serde_json::Value =
                serde_json::from_slice(&fs::read(&packet_path).expect("read packet"))
                    .expect("decode packet");
            assert_eq!(persisted, original_packet);
        }
        .await;
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        let _ = fs::remove_dir_all(&root);
        result
    }

    #[tokio::test]
    async fn packet_repair_validates_repaired_contract_before_persisting_owned_paths() {
        let root = packet_repair_temp_root("contract-before-write");
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(root.clone()));
        let result = async {
            fs::create_dir_all(&root).expect("create state root");
            let store = StateStore::open(root.clone()).await.expect("open store");
            let mut task = packet_repair_task_with_metadata();
            task.id = "task-invalid-packet".to_string();
            let run_id = "run-invalid-packet";
            store
                .record_run_graph_status(&packet_repair_status(run_id, &task.id))
                .await
                .expect("persist status");
            let packet_path = root
                .join("runtime-consumption")
                .join("run-invalid-packet.json");
            fs::create_dir_all(packet_path.parent().expect("packet parent"))
                .expect("create packet parent");
            let mut original_packet = packet_repair_dispatch_packet(run_id);
            original_packet["delivery_task_packet"]["proof_target"] = serde_json::Value::Null;
            fs::write(
                &packet_path,
                serde_json::to_vec_pretty(&original_packet).expect("encode packet"),
            )
            .expect("write packet");
            store
                .record_run_graph_dispatch_receipt(&packet_repair_receipt(run_id, &packet_path))
                .await
                .expect("persist receipt");

            let error = repair_persisted_dispatch_packet_from_task(&store, run_id, &task)
                .await
                .expect_err("invalid repaired packet must not be persisted");

            assert!(
                error.contains("execution_preparation_gate_blocked"),
                "{error}"
            );
            let persisted: serde_json::Value =
                serde_json::from_slice(&fs::read(&packet_path).expect("read packet"))
                    .expect("decode packet");
            assert_eq!(persisted, original_packet);
        }
        .await;
        crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        let _ = fs::remove_dir_all(&root);
        result
    }

    #[test]
    fn packet_render_hydrates_empty_owned_paths_from_task_metadata() {
        let task = packet_repair_task_with_metadata();
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": [],
            "delivery_task_packet": {
                "handoff_task_class": "implementation",
                "owned_paths": []
            }
        });

        assert!(hydrate_dispatch_packet_owned_paths_from_task(
            &mut packet,
            &task
        ));
        assert_eq!(
            packet["owned_paths"],
            serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
        );
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
        );
    }

    #[test]
    fn packet_repair_canonicalizes_stale_delivery_task_identity_before_validation() {
        let task = packet_repair_task_with_metadata();
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": [],
            "delivery_task_packet": {
                "packet_id": "run-1::orchestrator::delivery",
                "backlog_id": "run-1",
                "task_id": "task-with-metadata",
                "id": "task-with-metadata",
                "goal": "Execute bounded orchestrator handoff",
                "scope_in": ["dispatch_target:orchestrator", "runtime_role:worker"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "definition_of_done": ["bounded runtime result artifact"],
                "verification_command": "vida taskflow consume continue --run-id run-1",
                "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
                "stop_rules": ["stop after writing bounded dispatch result or explicit blocker"],
                "blocking_question": "What is the next bounded action required for orchestrator?"
            }
        });

        assert!(repair_delivery_task_packet_identity(&mut packet));
        assert!(hydrate_dispatch_packet_owned_paths_from_task(
            &mut packet,
            &task
        ));
        assert!(packet["delivery_task_packet"].get("task_id").is_none());
        assert!(packet["delivery_task_packet"].get("id").is_none());
        assert_eq!(packet["delivery_task_packet"]["backlog_id"], "run-1");
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
        );
        crate::validate_runtime_dispatch_packet_contract(&packet, "test packet")
            .expect("canonicalized repair packet should validate");
    }

    #[test]
    fn packet_render_hydration_rejects_unsafe_task_metadata_owned_paths() {
        let mut task = packet_repair_task_with_metadata();
        task.planner_metadata.owned_paths = vec![
            "../outside".to_string(),
            "/home/user/.ssh".to_string(),
            ".vida/data/state".to_string(),
            "windows\\path".to_string(),
            "C:/Users/admin/.ssh".to_string(),
            "crates/vida/src/taskflow_packet.rs".to_string(),
        ];
        let mut packet = serde_json::json!({
            "packet_template_kind": "delivery_task_packet",
            "owned_paths": [],
            "delivery_task_packet": {
                "handoff_task_class": "implementation",
                "owned_paths": []
            }
        });

        assert!(hydrate_dispatch_packet_owned_paths_from_task(
            &mut packet,
            &task
        ));
        assert_eq!(
            packet["owned_paths"],
            serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
        );
        assert_eq!(
            packet["delivery_task_packet"]["owned_paths"],
            serde_json::json!(["crates/vida/src/taskflow_packet.rs"])
        );
    }

    #[test]
    fn packet_repair_payload_blocks_missing_metadata_with_actionable_command() {
        let mut task = packet_repair_task_with_metadata();
        task.planner_metadata.owned_paths.clear();
        task.planner_metadata.proof_targets.clear();
        task.planner_metadata.acceptance_targets.clear();

        let payload = build_taskflow_packet_repair_payload("run-1", Some(&task), None);

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            "task_metadata_missing_owned_paths"
        );
        assert!(payload["next_actions"][0]
            .as_str()
            .expect("next action")
            .contains("vida task update task-with-metadata --owned-path"));
    }

    #[test]
    fn packet_repair_payload_guides_missing_task_to_one_shot_create() {
        let payload =
            build_taskflow_packet_repair_payload("run-1", None, Some("No canonical task exists."));

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["blocker_codes"][0], "task_metadata_not_found");
        let next_action = payload["next_actions"][1]
            .as_str()
            .expect("one-shot create next action");
        assert!(next_action.contains("vida task create <task-id> <title>"));
        assert!(next_action.contains("--owned-path <path>"));
        assert!(next_action.contains("--acceptance-target"));
        assert!(next_action.contains("--proof-target"));
    }

    #[tokio::test]
    async fn latest_packet_resolution_fails_closed_without_persisted_dispatch_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-packet-latest-no-cache-fallback-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone())
            .await
            .expect("state store should open");
        let cache_dir = root.join("runtime-consumption").join("dispatch-init-cache");
        fs::create_dir_all(&cache_dir).expect("cache dir should exist");
        fs::write(
            cache_dir.join("forged-run.json"),
            serde_json::json!({
                "run_id": "forged-run",
                "dispatch_receipt": {
                    "run_id": "forged-run",
                    "dispatch_status": "forged_not_routed"
                }
            })
            .to_string(),
        )
        .expect("cache file should write");

        let error = resolve_latest_packet_run_id(&store)
            .await
            .expect_err("cache-only packet latest should fail closed");

        assert!(error.contains("No latest persisted run-graph dispatch receipt exists"));

        let _ = fs::remove_dir_all(&root);
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
