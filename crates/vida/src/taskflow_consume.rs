use std::process::ExitCode;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;

use crate::display_lane_label;
use crate::runtime_consumption_surface::RuntimeConsumptionClosureAdmissionEvidence;
use crate::BlockerCode;

const CONSUME_FINAL_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

async fn fail_fast_state_store_open_with_timeout(
    state_root: std::path::PathBuf,
    label: &str,
    timeout: Duration,
) -> Result<super::StateStore, String> {
    match tokio::time::timeout(timeout, super::StateStore::open_existing(state_root)).await {
        Ok(result) => {
            result.map_err(|error| format!("consume final failed fast: {label}: {error}"))
        }
        Err(_) => Err(format!(
            "consume final failed fast: {label} timed out while waiting for authoritative datastore lock"
        )),
    }
}

async fn fail_fast_state_store_open(
    state_root: std::path::PathBuf,
    label: &str,
) -> Result<super::StateStore, String> {
    fail_fast_state_store_open_with_timeout(state_root, label, CONSUME_FINAL_LOCK_TIMEOUT).await
}

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

pub(crate) fn consume_final_command_usage() -> &'static str {
    "vida taskflow consume final <request_text> [--task-id <task-id>] [--owned-path <path>] [--from-task-metadata] [--preview | --validate-only] [--json]"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConsumeFinalArgs {
    as_json: bool,
    mode: ConsumeFinalMode,
    request_text: String,
    task_id: Option<String>,
    owned_paths: Vec<String>,
    from_task_metadata: bool,
}

fn consume_final_usage() -> String {
    format!("Usage: {}", consume_final_command_usage())
}

fn parse_consume_final_value<'a>(
    iter: &mut std::slice::Iter<'a, String>,
    flag: &str,
) -> Result<String, String> {
    iter.next()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "{flag} requires a non-empty value. {}",
                consume_final_usage()
            )
        })
}

fn push_consume_final_owned_path_values(target: &mut Vec<String>, raw: &str) {
    for value in raw.split(',') {
        let value = value.trim();
        if !value.is_empty() && !target.iter().any(|existing| existing == value) {
            target.push(value.to_string());
        }
    }
}

fn parse_taskflow_consume_final_args(request: &[String]) -> Result<ConsumeFinalArgs, String> {
    let mut as_json = false;
    let mut mode = ConsumeFinalMode::Execute;
    let mut request_parts = Vec::new();
    let mut task_id = None;
    let mut owned_paths = Vec::new();
    let mut from_task_metadata = false;
    let mut iter = request.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => as_json = true,
            "--preview" => {
                if mode == ConsumeFinalMode::ValidateOnly {
                    return Err(format!(
                        "--preview conflicts with --validate-only. {}",
                        consume_final_usage()
                    ));
                }
                mode = ConsumeFinalMode::Preview;
            }
            "--validate-only" => {
                if mode == ConsumeFinalMode::Preview {
                    return Err(format!(
                        "--validate-only conflicts with --preview. {}",
                        consume_final_usage()
                    ));
                }
                mode = ConsumeFinalMode::ValidateOnly;
            }
            "--task-id" => task_id = Some(parse_consume_final_value(&mut iter, "--task-id")?),
            "--owned-path" => {
                let value = parse_consume_final_value(&mut iter, "--owned-path")?;
                push_consume_final_owned_path_values(&mut owned_paths, &value);
            }
            "--from-task-metadata" => from_task_metadata = true,
            "--help" | "-h" => return Err(consume_final_usage()),
            _ => request_parts.push(arg.clone()),
        }
    }
    let mut request_text = request_parts.join(" ").trim().to_string();
    if request_text.is_empty() {
        if let Some(task_id) = task_id.as_deref() {
            request_text = task_id.to_string();
        }
    }
    if request_text.is_empty() {
        return Err(consume_final_usage());
    }
    Ok(ConsumeFinalArgs {
        as_json,
        mode,
        request_text,
        task_id,
        owned_paths,
        from_task_metadata,
    })
}

fn requested_help(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "help"))
}

fn print_consume_final_help() {
    println!("VIDA TaskFlow consume final");
    println!();
    println!("Purpose:");
    println!("  Create or validate the bounded final runtime-consumption handoff for one request.");
    println!();
    println!("Usage:");
    println!("  {}", consume_final_command_usage());
    println!("  vida taskflow consume final --task-id <task-id> --from-task-metadata [--json]");
    println!();
    println!("Output:");
    println!("  default                  Emit compact TOON operator output.");
    println!("  --json                   Emit machine-readable JSON output.");
    println!();
    println!("Options:");
    println!("  --task-id <id>          Use a canonical TaskFlow task id as the bounded request.");
    println!(
        "  --owned-path <path>     Add packet owned path override. Accepts comma-separated values and repeated flags."
    );
    println!(
        "  --from-task-metadata    Pull owned paths from the task planner_metadata.owned_paths."
    );
    println!("  --preview              Render the handoff preview without execute-mode mutation.");
    println!("  --validate-only        Validate the handoff path without execute-mode mutation.");
    println!("  --json                 Emit machine-readable output.");
    println!("  -h, --help             Print help.");
    println!();
    println!("Remediation:");
    println!(
        "  If closure is blocked, run `vida taskflow consume bundle check` and follow its next actions."
    );
}

fn print_consume_continue_help() {
    println!("VIDA TaskFlow consume continue");
    println!();
    println!("Usage:");
    println!(
        "  vida taskflow consume continue [--run-id <run_id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]"
    );
    println!();
    println!("Output:");
    println!("  default              Emit compact TOON operator output.");
    println!("  --json               Emit machine-readable JSON output.");
    println!();
    println!("Options:");
    println!("  --run-id <run_id>    Resume or refresh a concrete run.");
    println!("  --dispatch-packet <path>");
    println!("                       Resume from an explicit dispatch packet.");
    println!("  --downstream-packet <path>");
    println!("                       Resume from an explicit downstream packet.");
    println!("  --json               Emit machine-readable output.");
    println!("  -h, --help           Print help.");
    println!();
    println!("Remediation:");
    println!("  If resume is blocked, inspect `vida taskflow recovery latest`.");
}

fn print_consume_advance_help() {
    println!("VIDA TaskFlow consume advance");
    println!();
    println!("Usage:");
    println!("  vida taskflow consume advance [--run-id <run_id>] [--max-rounds <n>] [--json]");
    println!();
    println!("Output:");
    println!("  default              Emit compact TOON operator output.");
    println!("  --json               Emit machine-readable JSON output.");
    println!();
    println!("Options:");
    println!("  --run-id <run_id>    Advance a concrete run.");
    println!("  --max-rounds <n>     Limit automatic resume rounds.");
    println!("  --json               Emit machine-readable output.");
    println!("  -h, --help           Print help.");
    println!();
    println!("Remediation:");
    println!("  If advance is blocked, inspect `vida taskflow recovery latest`.");
}

fn consume_final_operator_command_text(command: &str) -> String {
    operator_output::command_text::human_command(command)
}

fn consume_final_toon_line(label: &str, value: &str) -> String {
    format!(
        "{}: {}",
        taskflow_format_toon::sanitize_toon_scalar(label),
        taskflow_format_toon::sanitize_toon_scalar(value)
    )
}

fn consume_final_toon_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn consume_final_design_first_delegated_lanes(execution_plan: &serde_json::Value) -> String {
    let required_lanes = execution_plan["orchestration_contract"]["delegation_policy"]
        ["required_lanes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(display_lane_label)
        .collect::<Vec<_>>();
    if !required_lanes.is_empty() {
        return required_lanes.join(", ");
    }

    let active_cycle = execution_plan["orchestration_contract"]["active_cycle"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let mut lanes = Vec::new();
    if active_cycle
        .iter()
        .any(|step| *step == "delegate_specification_or_research_lane")
    {
        lanes.push("specification");
    }
    if active_cycle
        .iter()
        .any(|step| *step == "delegate_implementer_lane")
    {
        lanes.push("implementer");
    }
    lanes.join(", ")
}

fn consume_final_toon_text(
    payload: &super::TaskflowDirectConsumptionPayload,
    snapshot_path: &str,
) -> String {
    let mut lines = vec![
        consume_final_toon_line("mode", payload.consume_final_mode.as_str()),
        consume_final_toon_line("request", &payload.request_text),
        consume_final_toon_line(
            "bundle_ready",
            consume_final_toon_bool(payload.bundle_check.ok),
        ),
        consume_final_toon_line(
            "docflow_ready",
            consume_final_toon_bool(payload.docflow_verdict.ready),
        ),
        consume_final_toon_line(
            "closure_admitted",
            consume_final_toon_bool(payload.closure_admission.admitted),
        ),
    ];
    if !payload.requested_owned_paths.is_empty() {
        lines.push(consume_final_toon_line(
            "requested_owned_paths_count",
            &payload.requested_owned_paths.len().to_string(),
        ));
    }
    if let Some(mode) =
        payload.role_selection.execution_plan["orchestration_contract"]["mode"].as_str()
    {
        lines.push(consume_final_toon_line("execution_mode", mode));
    }
    if let Some(message) = payload.role_selection.execution_plan["orchestration_contract"]
        ["initial_response"]["operator_message"]
        .as_str()
    {
        lines.push(consume_final_toon_line("first_step", message));
    }
    let replanning = payload.role_selection.execution_plan["orchestration_contract"]["replanning"]
        ["checkpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if !replanning.is_empty() {
        lines.push(consume_final_toon_line("replan_checkpoints", &replanning));
    }
    if payload.role_selection.execution_plan["status"] == "design_first" {
        if let Some(feature_slug) =
            payload.role_selection.execution_plan["tracked_flow_bootstrap"]["feature_slug"].as_str()
        {
            lines.push(consume_final_toon_line(
                "tracked_flow",
                &format!("spec-first bootstrap for `{feature_slug}`"),
            ));
        }
        if let Some(command) = payload.role_selection.execution_plan["tracked_flow_bootstrap"]
            ["bootstrap_command"]
            .as_str()
        {
            lines.push(consume_final_toon_line(
                "next_tracked_command",
                &consume_final_operator_command_text(command),
            ));
        }
        let required_lanes =
            consume_final_design_first_delegated_lanes(&payload.role_selection.execution_plan);
        if !required_lanes.is_empty() {
            lines.push(consume_final_toon_line("delegated_lanes", &required_lanes));
        }
    } else if let Some(agent_type) = payload.taskflow_handoff_plan["activation_chain"]
        ["implementer"]["activation_agent_type"]
        .as_str()
    {
        lines.push(consume_final_toon_line("implementer_carrier", agent_type));
    }
    if let Some(preview) = payload.dispatch_packet_preview.as_ref() {
        if let Some(packet_template_kind) = preview
            .get("packet_template_kind")
            .and_then(serde_json::Value::as_str)
        {
            lines.push(consume_final_toon_line(
                "packet_template",
                packet_template_kind,
            ));
        }
        let missing_fields = preview["packet_contract_missing_fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !missing_fields.is_empty() {
            lines.push(consume_final_toon_line(
                "missing_packet_fields",
                &missing_fields,
            ));
        }
    }
    lines.push(consume_final_toon_line("snapshot_path", snapshot_path));
    taskflow_format_toon::render_section("vida taskflow consume final", &lines.join("\n  "))
}

async fn consume_final_owned_paths_override(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    dispatch_receipt: &crate::state_store::RunGraphDispatchReceipt,
    args: &ConsumeFinalArgs,
) -> Vec<String> {
    let mut owned_paths = super::implementation_owned_paths_for_dispatch_context(
        store,
        role_selection,
        dispatch_receipt,
    )
    .await;
    if args.from_task_metadata {
        if let Some(task_id) = args.task_id.as_deref() {
            for path in super::planner_metadata_owned_paths_from_task(store, task_id).await {
                if !owned_paths.iter().any(|existing| existing == &path) {
                    owned_paths.push(path);
                }
            }
        }
    }
    for path in &args.owned_paths {
        if !owned_paths.iter().any(|existing| existing == path) {
            owned_paths.push(path.clone());
        }
    }
    owned_paths
}

async fn consume_final_requested_owned_paths(
    store: &super::StateStore,
    args: &ConsumeFinalArgs,
) -> Vec<String> {
    let mut owned_paths = Vec::new();
    if args.from_task_metadata {
        if let Some(task_id) = args.task_id.as_deref() {
            for path in super::planner_metadata_owned_paths_from_task(store, task_id).await {
                if !owned_paths.iter().any(|existing| existing == &path) {
                    owned_paths.push(path);
                }
            }
        }
    }
    for path in &args.owned_paths {
        if !owned_paths.iter().any(|existing| existing == path) {
            owned_paths.push(path.clone());
        }
    }
    owned_paths
}

async fn validate_consume_final_explicit_task_id(
    store: &super::StateStore,
    task_id: &str,
) -> Result<(), String> {
    store
        .show_task(task_id)
        .await
        .map(|_| ())
        .map_err(|error| {
            format!(
                "consume final explicit --task-id `{task_id}` does not resolve in the authoritative task store; refusing to create a stale run graph: {error}"
            )
        })
}

fn bind_consume_final_explicit_task_id(
    role_selection: &mut crate::RuntimeConsumptionLaneSelection,
    task_id: &str,
) {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return;
    }
    if !role_selection.execution_plan.is_object() {
        role_selection.execution_plan = serde_json::json!({});
    }
    if let Some(plan) = role_selection.execution_plan.as_object_mut() {
        plan.insert(
            "runtime_consumption_explicit_task_id".to_string(),
            serde_json::Value::String(task_id.to_string()),
        );
    }
}

fn apply_consume_final_downstream_dispatch_contract(
    dispatch_receipt: &mut crate::state_store::RunGraphDispatchReceipt,
    direct_consumption_ready: bool,
    docflow_ready: bool,
    conversational_mode: bool,
    blocker_code: Option<&str>,
) {
    if direct_consumption_ready {
        return;
    }
    let docflow_verdict_block = super::blocker_code_str(super::BlockerCode::DocflowVerdictBlock);
    let should_clear_downstream = (!docflow_ready && !conversational_mode)
        || dispatch_receipt.downstream_dispatch_target.as_deref() == Some("closure")
        || blocker_code == Some(docflow_verdict_block);
    if !should_clear_downstream {
        return;
    }
    dispatch_receipt.downstream_dispatch_target = None;
    dispatch_receipt.downstream_dispatch_command = None;
    dispatch_receipt.downstream_dispatch_note = None;
    dispatch_receipt.downstream_dispatch_ready = false;
    dispatch_receipt.downstream_dispatch_packet_path = None;
    dispatch_receipt.downstream_dispatch_status = None;
    dispatch_receipt.downstream_dispatch_result_path = None;
    dispatch_receipt.downstream_dispatch_trace_path = None;
    dispatch_receipt.downstream_dispatch_executed_count = 0;
    dispatch_receipt.downstream_dispatch_active_target = None;
    dispatch_receipt.downstream_dispatch_last_target = None;
}

pub(crate) fn try_print_taskflow_consume_nested_help(args: &[String]) -> bool {
    match args {
        [head] if head == "consume" => {
            super::print_taskflow_proxy_help(Some("consume"));
            true
        }
        [head, flag] if head == "consume" && matches!(flag.as_str(), "--help" | "-h") => {
            super::print_taskflow_proxy_help(Some("consume"));
            true
        }
        [head, subcommand, request @ ..] if head == "consume" && subcommand == "final" => {
            if requested_help(request) {
                print_consume_final_help();
                return true;
            }
            false
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "continue" => {
            if requested_help(args) {
                print_consume_continue_help();
                return true;
            }
            false
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "advance" => {
            if requested_help(args) {
                print_consume_advance_help();
                return true;
            }
            false
        }
        _ => false,
    }
}

pub(crate) async fn run_taskflow_consume(args: &[String]) -> ExitCode {
    if try_print_taskflow_consume_nested_help(args) {
        return ExitCode::SUCCESS;
    }

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
            if requested_help(args) {
                print_consume_continue_help();
                return ExitCode::SUCCESS;
            }
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
            if requested_help(args) {
                print_consume_advance_help();
                return ExitCode::SUCCESS;
            }
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
            if requested_help(request) {
                print_consume_final_help();
                return ExitCode::SUCCESS;
            }
            let consume_final_args = match parse_taskflow_consume_final_args(request) {
                Ok(parsed) => parsed,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let as_json = consume_final_args.as_json;
            let consume_final_mode = consume_final_args.mode;
            let request_text = consume_final_args.request_text.clone();
            if request_text.is_empty() {
                eprintln!("{}", consume_final_usage());
                return ExitCode::from(2);
            }

            let state_dir = super::taskflow_task_bridge::proxy_state_dir();
            match fail_fast_state_store_open(state_dir, "opening authoritative state store").await {
                Ok(store) => {
                    let explicit_task_id = consume_final_args.task_id.as_deref();
                    if let Some(task_id) = explicit_task_id {
                        if let Err(error) =
                            validate_consume_final_explicit_task_id(&store, task_id).await
                        {
                            if as_json {
                                crate::print_json_pretty(&serde_json::json!({
                                    "surface": "vida taskflow consume final",
                                    "status": "blocked",
                                    "blocker_codes": ["consume_final_explicit_task_id_missing"],
                                    "next_actions": [
                                        format!("Create or restore task `{task_id}` before rerunning consume final with --task-id.")
                                    ],
                                    "artifact_refs": {
                                        "surface": "vida taskflow consume final",
                                        "task_id": task_id,
                                    },
                                    "error": error,
                                }));
                            } else {
                                eprintln!("{error}");
                            }
                            return ExitCode::from(1);
                        }
                    }
                    match super::build_taskflow_consume_bundle_payload(&store).await {
                        Ok(runtime_bundle) => {
                            let bundle_check =
                                super::taskflow_consume_bundle_check(&runtime_bundle);
                            let (registry, check, readiness, proof, overview) =
                                super::build_docflow_runtime_evidence();
                            let docflow_receipt_evidence =
                                crate::runtime_consumption_surface::build_docflow_receipt_evidence(
                                    &readiness, &proof,
                                );
                            let mut docflow_verdict = super::build_docflow_runtime_verdict(
                                &registry, &check, &readiness, &proof,
                            );
                            let mut role_selection =
                                match super::build_runtime_lane_selection_with_store(
                                    &store,
                                    &request_text,
                                )
                                .await
                                {
                                    Ok(selection) => selection,
                                    Err(error) => {
                                        if as_json {
                                            let mut blocking_role_selection =
                                                super::blocking_lane_selection(
                                                    &request_text,
                                                    &error,
                                                );
                                            if let Some(task_id) = explicit_task_id {
                                                bind_consume_final_explicit_task_id(
                                                    &mut blocking_role_selection,
                                                    task_id,
                                                );
                                            }
                                            let blocked_run_id = super::runtime_consumption_run_id(
                                                &blocking_role_selection,
                                            );
                                            let blocked_status = crate::runtime_dispatch_status::blocking_runtime_consumption_run_graph_status(
                                        &blocking_role_selection,
                                        &blocked_run_id,
                                    );
                                            let blocked_status_json =
                                                serde_json::to_value(&blocked_status)
                                                    .unwrap_or(serde_json::Value::Null);
                                            let run_graph_bootstrap = serde_json::json!({
                                                "status": "blocked",
                                                "handoff_ready": false,
                                                "reason": "unresolved_lane_selection",
                                                "run_id": blocked_run_id,
                                                "latest_status": blocked_status_json,
                                            });
                                            let dispatch_receipt = blocked_dispatch_receipt(
                                                "unresolved_lane_selection",
                                                &bundle_check,
                                                &runtime_bundle,
                                                Some(blocked_run_id.as_str()),
                                            );
                                            let mut closure_admission =
                                                super::RuntimeConsumptionClosureAdmission {
                                                    status: "blocked".to_string(),
                                                    admitted: false,
                                                    blockers: vec![
                                                        "unresolved_lane_selection".to_string()
                                                    ],
                                                    proof_surfaces: vec![
                                                        "vida taskflow consume bundle check"
                                                            .to_string(),
                                                    ],
                                                    evidence_table: vec![
                                                    RuntimeConsumptionClosureAdmissionEvidence {
                                                        requirement: "lane_selection".to_string(),
                                                        status: "blocked".to_string(),
                                                        evidence_refs: vec![
                                                            "vida taskflow consume bundle check"
                                                                .to_string(),
                                                        ],
                                                        blockers: vec![
                                                            "unresolved_lane_selection".to_string()
                                                        ],
                                                    },
                                                ],
                                                };
                                            normalize_runtime_consumption_statuses(
                                                &mut docflow_verdict,
                                                &mut closure_admission,
                                            );
                                            let generated_at = time::OffsetDateTime::now_utc()
                                                .format(&super::Rfc3339)
                                                .expect("rfc3339 timestamp should render");
                                            let requested_owned_paths =
                                                consume_final_requested_owned_paths(
                                                    &store,
                                                    &consume_final_args,
                                                )
                                                .await;
                                            let mut payload = super::TaskflowDirectConsumptionPayload {
                                        artifact_name: "taskflow_direct_runtime_consumption"
                                            .to_string(),
                                        artifact_type: "runtime_consumption".to_string(),
                                        generated_at: generated_at.clone(),
                                        closure_authority: "taskflow".to_string(),
                                        consume_final_mode: consume_final_mode.as_str().to_string(),
                                        role_selection: blocking_role_selection,
                                        request_text: request_text.clone(),
                                        requested_owned_paths,
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
                                        run_graph_bootstrap,
                                        dispatch_receipt,
                                        dispatch_packet_preview: None,
                                    };
                                            if should_record_blocked_dispatch_receipt(
                                                consume_final_mode,
                                            ) {
                                                if let Err(error) =
                                                    persist_blocked_consume_final_resume_evidence(
                                                        &store,
                                                        &payload.role_selection,
                                                        &blocked_status,
                                                        &request_text,
                                                        &payload.taskflow_handoff_plan,
                                                        &payload.run_graph_bootstrap,
                                                        &mut payload.dispatch_receipt,
                                                    )
                                                    .await
                                                {
                                                    eprintln!(
                                                "Failed to record blocked consume-final resume evidence: {error}"
                                            );
                                                }
                                            }
                                            if let Err(snapshot_error) =
                                                super::emit_taskflow_consume_final_json(
                                                    &store, &payload,
                                                )
                                                .map(|_| ())
                                            {
                                                eprintln!("{snapshot_error}");
                                            }
                                            return ExitCode::from(1);
                                        }
                                        eprintln!("{error}");
                                        return ExitCode::from(1);
                                    }
                                };
                            if let Some(task_id) = explicit_task_id {
                                bind_consume_final_explicit_task_id(&mut role_selection, task_id);
                            }
                            let run_graph_bootstrap =
                            crate::runtime_dispatch_bootstrap::build_runtime_consumption_run_graph_bootstrap(
                                &store,
                                &role_selection,
                            )
                            .await;
                            if let Err(error) =
                                crate::apply_run_graph_runtime_assignment_to_selection(
                                    &mut role_selection,
                                    &runtime_bundle.activation_bundle,
                                    &run_graph_bootstrap,
                                    "run-graph role selection execution_plan is not an object",
                                )
                            {
                                eprintln!("{error}");
                                return ExitCode::from(1);
                            }
                            let taskflow_handoff_plan =
                                super::build_taskflow_handoff_plan(&role_selection);
                            let mut closure_admission = super::build_runtime_closure_admission(
                                &bundle_check,
                                &docflow_verdict,
                                &role_selection,
                            );
                            normalize_runtime_consumption_statuses(
                                &mut docflow_verdict,
                                &mut closure_admission,
                            );
                            let execution_preparation_gate =
                                build_execution_preparation_evidence_gate(
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
                            let dispatch_packet_preview = {
                                let owned_paths_override = consume_final_owned_paths_override(
                                    &store,
                                    &role_selection,
                                    &dispatch_receipt,
                                    &consume_final_args,
                                )
                                .await;
                                let ctx = crate::RuntimeDispatchPacketContext::new(
                                    store.root(),
                                    &role_selection,
                                    &dispatch_receipt,
                                    &taskflow_handoff_plan,
                                    &run_graph_bootstrap,
                                )
                                .with_owned_paths_override(owned_paths_override);
                                match super::runtime_dispatch_packet_preview(&ctx) {
                                    Ok(preview) => Some(preview),
                                    Err(error) => {
                                        eprintln!(
                                        "Failed to build runtime dispatch packet preview: {error}"
                                    );
                                        return ExitCode::from(1);
                                    }
                                }
                            };
                            let pending_design_packet =
                                super::blocker_code_str(super::BlockerCode::PendingDesignPacket);
                            let pending_execution_preparation_evidence = super::blocker_code_str(
                                super::BlockerCode::PendingExecutionPreparationEvidence,
                            );
                            let direct_consumption_ready = bundle_check.ok
                                && docflow_verdict.ready
                                && closure_admission.admitted
                                && !closure_admission.blockers.iter().any(|row| {
                                    row == pending_design_packet
                                        || row == pending_execution_preparation_evidence
                                })
                                && dispatch_packet_preview
                                    .as_ref()
                                    .and_then(|preview| preview.get("status"))
                                    .and_then(serde_json::Value::as_str)
                                    != Some("blocked");
                            let consume_final_blocker_code = if !direct_consumption_ready {
                                dispatch_packet_preview
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
                                    .or_else(|| {
                                        Some(
                                        super::blocker_code_str(
                                            super::BlockerCode::PendingExecutionPreparationEvidence,
                                        )
                                        .to_string(),
                                    )
                                    })
                            } else {
                                None
                            };
                            if let Some(blocker_code) = consume_final_blocker_code.clone() {
                                dispatch_receipt.dispatch_status = "blocked".to_string();
                                dispatch_receipt.lane_status =
                                    super::LaneStatus::LaneBlocked.as_str().to_string();
                                dispatch_receipt.blocker_code = Some(blocker_code);
                            }
                            apply_consume_final_downstream_dispatch_contract(
                                &mut dispatch_receipt,
                                direct_consumption_ready,
                                docflow_verdict.ready,
                                role_selection.conversational_mode.is_some(),
                                consume_final_blocker_code.as_deref(),
                            );
                            if !consume_final_mode.is_read_only() {
                                let owned_paths_override = consume_final_owned_paths_override(
                                    &store,
                                    &role_selection,
                                    &dispatch_receipt,
                                    &consume_final_args,
                                )
                                .await;
                                let ctx = crate::RuntimeDispatchPacketContext::new(
                                    store.root(),
                                    &role_selection,
                                    &dispatch_receipt,
                                    &taskflow_handoff_plan,
                                    &run_graph_bootstrap,
                                )
                                .with_owned_paths_override(owned_paths_override);
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
                            let allow_automatic_dispatch_execution =
                            super::taskflow_task_bridge::infer_project_root_from_state_root(
                                store.root(),
                            )
                            .map(|project_root| {
                                super::runtime_dispatch_state::runtime_host_execution_contract_allows_automatic_dispatch_execution(&project_root)
                            })
                            .unwrap_or(true);
                            let state_root = store.root().to_path_buf();
                            drop(store);
                            if !consume_final_mode.is_read_only()
                                && direct_consumption_ready
                                && dispatch_receipt.dispatch_status == "routed"
                                && allow_taskflow_pack_execution
                                && allow_automatic_dispatch_execution
                            {
                                if let Err(error) = super::execute_and_record_dispatch_receipt(
                                    &state_root,
                                    &role_selection,
                                    &run_graph_bootstrap,
                                    &mut dispatch_receipt,
                                )
                                .await
                                {
                                    super::taskflow_consume_resume::emit_consume_continue_resume_error(
                                    &error,
                                    "vida taskflow consume final",
                                    as_json,
                                );
                                    return ExitCode::from(1);
                                }
                            }
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
                            let store = match fail_fast_state_store_open(
                                state_root.clone(),
                                "reopening authoritative state store before receipt persistence",
                            )
                            .await
                            {
                                Ok(store) => store,
                                Err(error) => {
                                    eprintln!(
                                    "Failed to reopen authoritative state store before receipt persistence: {error}"
                                );
                                    return ExitCode::from(1);
                                }
                            };
                            if !consume_final_mode.is_read_only() {
                                if let Err(error) = store
                                    .record_run_graph_dispatch_receipt(&dispatch_receipt)
                                    .await
                                {
                                    eprintln!(
                                        "Failed to record run-graph dispatch receipt: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                            // Re-sync continuation binding after downstream dispatch chain advances the run-graph.
                            // Downstream execution inside execute_downstream_dispatch_chain updates run-graph status
                            // via execute_and_record_dispatch_receipt, but the root-level continuation binding must
                            // be refreshed after the final receipt is persisted so reconciled status sees blocked
                            // downstream truth rather than stale upstream status.
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
                                        crate::taskflow_continuation::CONSUME_AFTER_DOWNSTREAM_CHAIN_BINDING_SOURCE,
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
                            let generated_at = time::OffsetDateTime::now_utc()
                                .format(&super::Rfc3339)
                                .expect("rfc3339 timestamp should render");
                            let requested_owned_paths =
                                consume_final_requested_owned_paths(&store, &consume_final_args)
                                    .await;
                            let payload = super::TaskflowDirectConsumptionPayload {
                            artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                            artifact_type: "runtime_consumption".to_string(),
                            generated_at: generated_at.clone(),
                            closure_authority: "taskflow".to_string(),
                            consume_final_mode: consume_final_mode.as_str().to_string(),
                            role_selection,
                            request_text: request_text.clone(),
                            requested_owned_paths,
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
                                let snapshot_path =
                                    match super::emit_taskflow_consume_final_json(&store, &payload)
                                    {
                                        Ok(snapshot_path) => snapshot_path,
                                        Err(error) => {
                                            eprintln!("{error}");
                                            return ExitCode::from(1);
                                        }
                                    };
                                if let Err(error) =
                                    ensure_runtime_consumption_final_task_reconciliation_summary(
                                        &store,
                                        Some(snapshot_path),
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
                                println!("{}", consume_final_toon_text(&payload, &snapshot_path));
                            }

                            match consume_final_mode {
                                ConsumeFinalMode::Preview => ExitCode::SUCCESS,
                                ConsumeFinalMode::Execute | ConsumeFinalMode::ValidateOnly => {
                                    if payload.closure_admission.admitted {
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
                                let mut role_selection =
                                    super::blocking_lane_selection(&request_text, &error);
                                if let Some(task_id) = explicit_task_id {
                                    bind_consume_final_explicit_task_id(
                                        &mut role_selection,
                                        task_id,
                                    );
                                }
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
                                let blocked_run_id =
                                    super::json_string(run_graph_bootstrap.get("run_id"))
                                        .unwrap_or_else(|| {
                                            super::runtime_consumption_run_id(&role_selection)
                                        });
                                let dispatch_receipt = blocked_dispatch_receipt(
                                    "docflow_activation_failed",
                                    &bundle_check,
                                    &runtime_bundle,
                                    Some(blocked_run_id.as_str()),
                                );
                                if should_record_blocked_dispatch_receipt(consume_final_mode) {
                                    if let Ok(receipt) = serde_json::from_value::<
                                        crate::state_store::RunGraphDispatchReceipt,
                                    >(
                                        dispatch_receipt.clone()
                                    ) {
                                        if let Err(error) =
                                            store.record_run_graph_dispatch_receipt(&receipt).await
                                        {
                                            eprintln!(
                                            "Failed to record blocked run-graph dispatch receipt: {error}"
                                        );
                                        }
                                    }
                                }
                                let mut docflow_activation =
                                    super::blocking_docflow_activation(&error);
                                if let Some(evidence) = docflow_activation.evidence.as_object_mut()
                                {
                                    evidence.insert(
                                        "receipt_evidence".to_string(),
                                        docflow_receipt_evidence,
                                    );
                                }
                                let generated_at = time::OffsetDateTime::now_utc()
                                    .format(&super::Rfc3339)
                                    .expect("rfc3339 timestamp should render");
                                let requested_owned_paths = consume_final_requested_owned_paths(
                                    &store,
                                    &consume_final_args,
                                )
                                .await;
                                let payload = super::TaskflowDirectConsumptionPayload {
                                artifact_name: "taskflow_direct_runtime_consumption".to_string(),
                                artifact_type: "runtime_consumption".to_string(),
                                generated_at: generated_at.clone(),
                                closure_authority: "taskflow".to_string(),
                                consume_final_mode: consume_final_mode.as_str().to_string(),
                                request_text: request_text.clone(),
                                requested_owned_paths,
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
                                run_graph_bootstrap,
                                dispatch_receipt,
                                dispatch_packet_preview: None,
                                direct_consumption_ready: false,
                            };
                                if let Err(snapshot_error) =
                                    super::emit_taskflow_consume_final_json(&store, &payload)
                                        .map(|_| ())
                                {
                                    eprintln!("{snapshot_error}");
                                    return ExitCode::from(1);
                                }
                                return ExitCode::from(1);
                            }
                            eprintln!("{error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Failed to open authoritative state store: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [head, subcommand, ..] if head == "consume" && subcommand == "final" => {
            eprintln!("{}", consume_final_usage());
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
struct ExecutionPreparationArtifact {
    ready: bool,
    status: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExecutionPreparationArtifacts {
    handoff_ready: bool,
    developer_handoff_packet: DeveloperHandoffPacketArtifact,
    architecture_preparation_report: ExecutionPreparationArtifact,
    change_boundary: ExecutionPreparationArtifact,
    dependency_impact_summary: ExecutionPreparationArtifact,
    spec_alignment_summary: ExecutionPreparationArtifact,
    execution_preparation_evidence: ExecutionPreparationEvidenceArtifact,
    structured_artifacts_present: bool,
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
    let run_graph_artifact_json = run_graph_bootstrap
        .get("execution_preparation_artifacts")
        .filter(|value| value.is_object());
    let artifact_json = run_graph_artifact_json
        .or_else(|| taskflow_handoff_plan.get("execution_preparation_artifacts"))
        .filter(|value| value.is_object());
    let packet_json = artifact_json.and_then(|value| value.get("developer_handoff_packet"));
    let evidence_json = artifact_json.and_then(|value| value.get("execution_preparation_evidence"));
    let structured_artifacts_present = run_graph_artifact_json.is_some();

    let handoff_ready = super::json_bool(taskflow_handoff_plan.get("handoff_ready"), false)
        && (artifact_json
            .map(|value| super::json_bool(value.get("handoff_ready"), false))
            .unwrap_or_else(|| super::json_bool(run_graph_bootstrap.get("handoff_ready"), false)));
    let legacy_packet_ready = super::json_bool(
        run_graph_bootstrap.get("execution_preparation_handoff_packet_ready"),
        false,
    ) || run_graph_bootstrap
        .get("execution_preparation_packet_path")
        .and_then(serde_json::Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let legacy_evidence_ready =
        super::json_bool(
            run_graph_bootstrap.get("execution_preparation_evidence_ready"),
            false,
        ) || run_graph_bootstrap["evidence"]["execution_preparation"]["status"].as_str()
            == Some("ready")
            || run_graph_bootstrap["evidence"]["execution_preparation"]["ready"]
                .as_bool()
                .unwrap_or(false);

    let developer_handoff_packet = DeveloperHandoffPacketArtifact {
        path: nonempty_json_string(packet_json.and_then(|value| value.get("path"))).or_else(|| {
            nonempty_json_string(run_graph_bootstrap.get("execution_preparation_packet_path"))
        }),
        ready: packet_json
            .map(|value| super::json_bool(value.get("ready"), false) || legacy_packet_ready)
            .unwrap_or(legacy_packet_ready),
        status: nonempty_json_string(packet_json.and_then(|value| value.get("status"))),
    };
    let execution_preparation_evidence = ExecutionPreparationEvidenceArtifact {
        ready: evidence_json
            .map(|value| super::json_bool(value.get("ready"), false) || legacy_evidence_ready)
            .unwrap_or(legacy_evidence_ready),
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
        architecture_preparation_report: decode_execution_preparation_artifact(
            artifact_json,
            "architecture_preparation_report",
        ),
        change_boundary: decode_execution_preparation_artifact(artifact_json, "change_boundary"),
        dependency_impact_summary: decode_execution_preparation_artifact(
            artifact_json,
            "dependency_impact_summary",
        ),
        spec_alignment_summary: decode_execution_preparation_artifact(
            artifact_json,
            "spec_alignment_summary",
        ),
        execution_preparation_evidence,
        structured_artifacts_present,
    }
}

fn decode_execution_preparation_artifact(
    artifact_json: Option<&serde_json::Value>,
    key: &str,
) -> ExecutionPreparationArtifact {
    let value = artifact_json.and_then(|value| value.get(key));
    ExecutionPreparationArtifact {
        ready: value
            .map(|value| super::json_bool(value.get("ready"), false))
            .unwrap_or(false),
        status: nonempty_json_string(value.and_then(|value| value.get("status"))),
        path: nonempty_json_string(value.and_then(|value| value.get("path"))),
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
    let required_artifacts_ready = !artifacts.structured_artifacts_present
        || (artifacts.architecture_preparation_report.ready
            && artifacts.change_boundary.ready
            && artifacts.dependency_impact_summary.ready
            && artifacts.spec_alignment_summary.ready);

    ExecutionPreparationEvidenceGate {
        missing_evidence_or_handoff_packet: !(artifacts.handoff_ready
            && packet_ready
            && evidence_ready
            && required_artifacts_ready),
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

fn should_record_blocked_dispatch_receipt(consume_final_mode: ConsumeFinalMode) -> bool {
    !consume_final_mode.is_read_only()
}

async fn persist_blocked_consume_final_resume_evidence(
    store: &super::StateStore,
    role_selection: &super::RuntimeConsumptionLaneSelection,
    status: &crate::state_store::RunGraphStatus,
    request_text: &str,
    taskflow_handoff_plan: &serde_json::Value,
    run_graph_bootstrap: &serde_json::Value,
    dispatch_receipt: &mut serde_json::Value,
) -> Result<(), String> {
    store
        .record_run_graph_status(status)
        .await
        .map_err(|error| format!("Failed to record blocked run-graph status: {error}"))?;

    let recorded_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    store
        .record_run_graph_dispatch_context(&crate::state_store::RunGraphDispatchContext {
            run_id: status.run_id.clone(),
            task_id: status.task_id.clone(),
            request_text: request_text.to_string(),
            role_selection: serde_json::to_value(role_selection)
                .unwrap_or_else(|_| serde_json::Value::Null),
            recorded_at,
        })
        .await
        .map_err(|error| format!("Failed to record blocked run-graph dispatch context: {error}"))?;

    let mut receipt = serde_json::from_value::<crate::state_store::RunGraphDispatchReceipt>(
        dispatch_receipt.clone(),
    )
    .map_err(|error| format!("Failed to decode blocked dispatch receipt: {error}"))?;
    let ctx = crate::RuntimeDispatchPacketContext::new(
        store.root(),
        role_selection,
        &receipt,
        taskflow_handoff_plan,
        run_graph_bootstrap,
    );
    let dispatch_packet_path = super::write_runtime_dispatch_packet(&ctx)?;
    receipt.dispatch_packet_path = Some(dispatch_packet_path);
    *dispatch_receipt = serde_json::to_value(&receipt)
        .map_err(|error| format!("Failed to encode blocked dispatch receipt: {error}"))?;

    store
        .record_run_graph_dispatch_receipt(&receipt)
        .await
        .map_err(|error| format!("Failed to record blocked run-graph dispatch receipt: {error}"))?;
    crate::taskflow_continuation::sync_run_graph_continuation_binding(
        store,
        status,
        "consume_final_blocked_resume_evidence",
    )
    .await
    .map_err(|error| format!("Failed to sync blocked continuation binding: {error}"))?;
    Ok(())
}

fn blocked_dispatch_receipt(
    reason: &str,
    bundle_check: &super::TaskflowConsumeBundleCheck,
    runtime_bundle: &super::TaskflowConsumeBundlePayload,
    run_id: Option<&str>,
) -> serde_json::Value {
    let mut downstream_dispatch_blockers = bundle_check.blockers.clone();
    if !downstream_dispatch_blockers.iter().any(|row| row == reason) {
        downstream_dispatch_blockers.insert(0, reason.to_string());
    }
    let run_id = run_id.map(str::trim).filter(|value| !value.is_empty());

    let mut receipt = serde_json::json!({
        "status": "blocked",
        "dispatch_status": "blocked",
        "lane_status": super::LaneStatus::LaneBlocked.as_str(),
        "dispatch_kind": "none",
        "dispatch_target": "none",
        "dispatch_surface": "vida taskflow consume final",
        "dispatch_command": null,
        "dispatch_packet_path": null,
        "dispatch_result_path": null,
        "blocker_code": reason,
        "supersedes_receipt_id": null,
        "exception_path_receipt_id": null,
        "downstream_dispatch_target": null,
        "downstream_dispatch_command": null,
        "downstream_dispatch_note": null,
        "downstream_dispatch_ready": false,
        "downstream_dispatch_blockers": downstream_dispatch_blockers,
        "downstream_dispatch_packet_path": null,
        "downstream_dispatch_status": null,
        "downstream_dispatch_result_path": null,
        "downstream_dispatch_trace_path": null,
        "downstream_dispatch_executed_count": 0,
        "downstream_dispatch_active_target": null,
        "downstream_dispatch_last_target": null,
        "activation_agent_type": null,
        "activation_runtime_role": null,
        "selected_backend": null,
        "recorded_at": time::OffsetDateTime::now_utc()
            .format(&super::Rfc3339)
            .expect("rfc3339 timestamp should render"),
        "artifact_refs": {
            "root_artifact_id": bundle_check.root_artifact_id,
            "bundle_artifact_name": runtime_bundle.artifact_name,
            "cache_delivery_contract": {
                "cache_key_inputs_present": runtime_bundle.cache_delivery_contract["cache_key_inputs"].is_object(),
                "invalidation_tuple_present": runtime_bundle.cache_delivery_contract["invalidation_tuple"].is_object(),
            },
        },
    });
    if let Some(run_id) = run_id {
        receipt["run_id"] = serde_json::Value::String(run_id.to_string());
    }
    receipt
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
    let dispatch_kind = if dispatch_surface.as_deref() == Some("vida taskflow bootstrap-spec") {
        "agent_lane".to_string()
    } else {
        dispatch_kind
    };
    let dispatch_surface = if dispatch_kind == "agent_lane"
        || dispatch_surface.as_deref() == Some("vida taskflow bootstrap-spec")
    {
        Some("vida agent-init".to_string())
    } else {
        dispatch_surface
    };
    let activation_agent_type = activation_agent_type.or_else(|| {
        if role_selection.conversational_mode.is_some() {
            role_selection.execution_plan["default_route"]["activation_agent_type"]
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    super::runtime_assignment_from_execution_plan(&role_selection.execution_plan)
                        ["activation_agent_type"]
                        .as_str()
                        .map(str::to_string)
                })
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
                .or_else(|| {
                    super::runtime_assignment_from_execution_plan(&role_selection.execution_plan)
                        ["activation_runtime_role"]
                        .as_str()
                        .map(str::to_string)
                })
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
    let downstream_dispatch_ready =
        dispatch_ready || (dispatch_target == "closure" && dispatch_blockers.is_empty());
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
        downstream_dispatch_ready,
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
    if next_node.as_deref() == Some("spec-pack")
        && super::execution_plan_agent_only_development_required(&role_selection.execution_plan)
    {
        return Some("specification".to_string());
    }
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
        consume_final_command_usage, fail_fast_state_store_open_with_timeout,
        normalize_runtime_consumption_statuses, parse_taskflow_consume_final_args,
        should_record_blocked_dispatch_receipt, try_print_taskflow_consume_nested_help,
        ApprovalDelegationEvidenceGate, ConsumeFinalMode, ExecutionPreparationEvidenceGate,
        RetrievalPolicyDecisionGate,
    };
    use std::time::Duration;

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

        let preview =
            parse_taskflow_consume_final_args(&preview_args).expect("preview args should parse");
        let validate = parse_taskflow_consume_final_args(&validate_args)
            .expect("validate-only args should parse");

        assert!(preview.as_json);
        assert_eq!(preview.mode, ConsumeFinalMode::Preview);
        assert_eq!(preview.request_text, "ship this");

        assert!(!validate.as_json);
        assert_eq!(validate.mode, ConsumeFinalMode::ValidateOnly);
        assert_eq!(validate.request_text, "ship this");
    }

    #[test]
    fn parse_taskflow_consume_final_args_rejects_conflicting_readonly_modes() {
        let preview_then_validate = vec![
            "ship".to_string(),
            "--preview".to_string(),
            "--validate-only".to_string(),
        ];
        let validate_then_preview = vec![
            "ship".to_string(),
            "--validate-only".to_string(),
            "--preview".to_string(),
        ];

        let preview_error = parse_taskflow_consume_final_args(&preview_then_validate)
            .expect_err("preview plus validate-only should be rejected");
        let validate_error = parse_taskflow_consume_final_args(&validate_then_preview)
            .expect_err("validate-only plus preview should be rejected");

        assert!(preview_error.contains("--validate-only conflicts with --preview"));
        assert!(validate_error.contains("--preview conflicts with --validate-only"));
        assert!(preview_error.contains(consume_final_command_usage()));
        assert!(validate_error.contains(consume_final_command_usage()));
    }

    #[test]
    fn blocked_dispatch_receipts_only_persist_in_execute_mode() {
        assert!(should_record_blocked_dispatch_receipt(
            ConsumeFinalMode::Execute
        ));
        assert!(!should_record_blocked_dispatch_receipt(
            ConsumeFinalMode::Preview
        ));
        assert!(!should_record_blocked_dispatch_receipt(
            ConsumeFinalMode::ValidateOnly
        ));
    }

    #[test]
    fn nested_consume_help_is_handled_before_async_final_surface() {
        let final_help = vec![
            "consume".to_string(),
            "final".to_string(),
            "--help".to_string(),
        ];
        let continue_help = vec![
            "consume".to_string(),
            "continue".to_string(),
            "--help".to_string(),
        ];

        assert!(try_print_taskflow_consume_nested_help(&final_help));
        assert!(try_print_taskflow_consume_nested_help(&continue_help));
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
                        "executor_backend": "hermes_cli",
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
        assert_eq!(receipt.selected_backend.as_deref(), Some("hermes_cli"));
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
    fn conversational_dispatch_receipt_uses_runtime_assignment_activation_fallback() {
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "clarify spec scope".to_string(),
            selected_role: "business_analyst".to_string(),
            conversational_mode: Some("scope_discussion".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("spec-pack".to_string()),
            allow_freeform_chat: true,
            confidence: "high".to_string(),
            matched_terms: vec![
                "clarify".to_string(),
                "spec".to_string(),
                "scope".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "runtime_assignment": {
                    "activation_agent_type": "middle",
                    "activation_runtime_role": "business_analyst",
                    "selected_tier": "middle"
                }
            }),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = serde_json::json!({
            "run_id": "run-scope-discussion",
            "latest_status": {
                "next_node": "business_analyst",
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
        assert_eq!(receipt.selected_backend.as_deref(), Some("middle"));
    }

    #[test]
    fn runtime_consumption_dispatch_receipt_keeps_spec_pack_before_agent_only_execution_sequence() {
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
            matched_terms: vec!["scope".to_string(), "specification".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "autonomous_execution": {
                    "agent_only_development": true
                },
                "development_flow": {
                    "dispatch_contract": {
                        "lane_sequence": ["junior", "coach", "tester"],
                        "lane_catalog": {
                            "middle": {
                                "activation": {
                                    "activation_agent_type": "middle",
                                    "activation_runtime_role": "business_analyst",
                                    "selected_agent_id": "middle"
                                }
                            },
                            "junior": {
                                "activation": {
                                    "activation_agent_type": "junior",
                                    "activation_runtime_role": "worker",
                                    "selected_agent_id": "junior"
                                }
                            }
                        },
                        "specification_activation": {
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "business_analyst",
                            "selected_agent_id": "middle"
                        },
                        "implementer_activation": {
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker",
                            "selected_agent_id": "junior"
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
            "run_id": "run-spec-pack",
            "latest_status": {
                "active_node": "planning",
                "next_node": "spec-pack"
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
        assert_ne!(receipt.dispatch_target, "junior");
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
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ],
                "development_flow": {
                    "implementation": {
                        "executor_backend": "hermes_cli",
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
        assert_eq!(receipt.selected_backend.as_deref(), Some("hermes_cli"));
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
                "architecture_preparation_report": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/architecture-preparation-report.json"
                },
                "developer_handoff_packet": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/packet.json"
                },
                "change_boundary": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/change-boundary.json"
                },
                "dependency_impact_summary": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/dependency-impact-summary.json"
                },
                "spec_alignment_summary": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/spec-alignment-summary.json"
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
    fn execution_preparation_gate_blocks_when_structured_artifact_is_missing() {
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
                "architecture_preparation_report": {
                    "ready": true,
                    "status": "ready"
                },
                "developer_handoff_packet": {
                    "ready": true,
                    "status": "ready",
                    "path": "/tmp/packet.json"
                },
                "change_boundary": {
                    "ready": false,
                    "status": "pending_change_boundary"
                },
                "dependency_impact_summary": {
                    "ready": true,
                    "status": "ready"
                },
                "spec_alignment_summary": {
                    "ready": true,
                    "status": "ready"
                },
                "execution_preparation_evidence": {
                    "ready": true,
                    "status": "ready"
                }
            }
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
            "execution_preparation_artifacts": {
                "handoff_ready": true,
                "developer_handoff_packet": {
                    "ready": false,
                    "status": "pending_developer_handoff_packet",
                    "path": null
                },
                "execution_preparation_evidence": {
                    "ready": false,
                    "status": "pending_execution_preparation_evidence"
                }
            }
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
            evidence_table: vec![],
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
    fn consume_final_design_first_delegated_lanes_prefers_required_lanes() {
        let execution_plan = serde_json::json!({
            "status": "design_first",
            "orchestration_contract": {
                "active_cycle": [
                    "publish_initial_execution_plan",
                    "delegate_specification_or_research_lane",
                    "replan_after_design_gate",
                    "shape_work_pool_and_dev_packets",
                    "delegate_implementer_lane"
                ],
                "delegation_policy": {
                    "required_lanes": [
                        "specification",
                        "implementer",
                        "coach",
                        "verifier",
                        "execution_preparation"
                    ]
                }
            }
        });

        assert_eq!(
            super::consume_final_design_first_delegated_lanes(&execution_plan),
            "specification, implementer, coach, verifier, execution preparation"
        );
    }

    #[test]
    fn consume_final_design_first_delegated_lanes_falls_back_to_active_cycle() {
        let execution_plan = serde_json::json!({
            "status": "design_first",
            "orchestration_contract": {
                "active_cycle": [
                    "delegate_specification_or_research_lane",
                    "delegate_implementer_lane"
                ],
                "delegation_policy": {
                    "required_lanes": []
                }
            }
        });

        assert_eq!(
            super::consume_final_design_first_delegated_lanes(&execution_plan),
            "specification, implementer"
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
        let parsed =
            super::parse_taskflow_consume_final_args(&args).expect("final args should parse");

        assert!(parsed.as_json);
        assert_eq!(parsed.mode, super::ConsumeFinalMode::Preview);
        assert_eq!(parsed.request_text, "fix dispatch");
    }

    #[test]
    fn parse_taskflow_consume_final_args_supports_validate_only_mode() {
        let args = vec![
            "--validate-only".to_string(),
            "shape".to_string(),
            "packet".to_string(),
        ];
        let parsed = super::parse_taskflow_consume_final_args(&args)
            .expect("validate-only args should parse");

        assert!(!parsed.as_json);
        assert_eq!(parsed.mode, super::ConsumeFinalMode::ValidateOnly);
        assert_eq!(parsed.request_text, "shape packet");
    }

    #[test]
    fn parse_taskflow_consume_final_args_supports_task_metadata_and_owned_path_options() {
        let args = vec![
            "--task-id".to_string(),
            "universal-surfaces-menu-route-binding-mismatch".to_string(),
            "--from-task-metadata".to_string(),
            "--owned-path".to_string(),
            "lib/features/menu/menu_route_registry.dart,test/features/menu/menu_route_registry_test.dart"
                .to_string(),
            "--owned-path".to_string(),
            "lib/features/menu/menu_route_registry.dart".to_string(),
            "--json".to_string(),
        ];

        let parsed = super::parse_taskflow_consume_final_args(&args)
            .expect("task metadata args should parse");

        assert!(parsed.as_json);
        assert_eq!(
            parsed.task_id.as_deref(),
            Some("universal-surfaces-menu-route-binding-mismatch")
        );
        assert!(parsed.from_task_metadata);
        assert_eq!(
            parsed.request_text,
            "universal-surfaces-menu-route-binding-mismatch"
        );
        assert_eq!(
            parsed.owned_paths,
            vec![
                "lib/features/menu/menu_route_registry.dart".to_string(),
                "test/features/menu/menu_route_registry_test.dart".to_string()
            ]
        );
    }

    #[test]
    fn consume_final_explicit_task_id_binding_overrides_request_slug_run_identity() {
        let mut selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "auto".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Use meeting specific event fields when".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: false,
            tracked_flow_entry: None,
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::Value::Null,
            reason: "test".to_string(),
        };

        super::bind_consume_final_explicit_task_id(
            &mut selection,
            "activity-meeting-event-form-fields",
        );

        assert_eq!(
            selection.execution_plan["runtime_consumption_explicit_task_id"],
            "activity-meeting-event-form-fields"
        );
        assert_eq!(
            crate::runtime_consumption_run_id(&selection),
            "activity-meeting-event-form-fields"
        );
    }

    #[test]
    fn consume_final_explicit_task_id_validation_fails_before_stale_run_creation() {
        let root = std::env::temp_dir().join(format!(
            "vida-consume-final-missing-task-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let store = crate::StateStore::open(root.clone())
                .await
                .expect("open store");
            let error =
                super::validate_consume_final_explicit_task_id(&store, "missing-task-run").await;
            assert!(error
                .expect_err("missing explicit task id should fail")
                .contains("refusing to create a stale run graph"));
            store.close().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn consume_final_fail_fast_open_returns_prompt_lock_error() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-consume-final-lock-{}-{}",
            std::process::id(),
            nanos
        ));
        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        runtime.block_on(async {
            let store = crate::StateStore::open(root.clone())
                .await
                .expect("open store");
            let result = tokio::time::timeout(
                Duration::from_secs(3),
                fail_fast_state_store_open_with_timeout(
                    root.clone(),
                    "opening authoritative state store",
                    Duration::from_secs(3),
                ),
            )
            .await
            .expect("helper should return inside the bounded window");
            if let Err(error) = result {
                assert!(
                    error.contains("consume final failed fast: opening authoritative state store"),
                    "expected contextual fail-fast error, got {error}"
                );
            }
            drop(store);
            let _ = std::fs::remove_dir_all(&root);
        });
    }
}
