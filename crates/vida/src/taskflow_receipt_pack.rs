use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Serialize;

use crate::release1_operator_output::{
    RELEASE1_OPERATOR_CONTRACT_SPEC, finalize_operator_surface_verdict,
};
use crate::state_store::{StateStore, StateStoreError, TaskRecord, work_item_is_program_container};

#[derive(Debug, Clone)]
struct TaskflowReceiptPackCommand {
    since: Option<String>,
    json: bool,
    fields: Option<String>,
    state_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
struct ReceiptPackTaskRef {
    id: String,
    title: String,
    status: String,
    issue_type: String,
    closed_at: Option<String>,
    close_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReceiptPackLatestReceipts {
    dispatch_run_id: Option<String>,
    dispatch_status: Option<String>,
    exception_receipt_ids: Vec<String>,
    verification_receipts: usize,
    task_reconciliation_receipt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReceiptPackGitRefs {
    status: String,
    since: Option<String>,
    head: Option<String>,
    branch: Option<String>,
    commits_since: Vec<String>,
    changed_files_since: Vec<String>,
    dirty_files_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReceiptPackPayload {
    surface: &'static str,
    status: &'static str,
    since: Option<String>,
    closed_tasks: Vec<ReceiptPackTaskRef>,
    closed_epics: Vec<ReceiptPackTaskRef>,
    exception_receipts: Vec<String>,
    verification_receipts: usize,
    quality_gates: Vec<String>,
    artifacts: serde_json::Value,
    git_refs: ReceiptPackGitRefs,
    latest_receipts: ReceiptPackLatestReceipts,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: serde_json::Value,
    shared_fields: serde_json::Value,
    operator_contracts: serde_json::Value,
}

pub(crate) async fn run_taskflow_receipt_pack(args: &[String]) -> ExitCode {
    let command = match parse_taskflow_receipt_pack_args(args) {
        Ok(command) => command,
        Err(message) if message == "help" => {
            print_taskflow_receipt_pack_help();
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let store = match StateStore::open_existing_read_only(command.state_dir.clone()).await {
        Ok(store) => store,
        Err(error) => {
            let payload = blocked_payload(
                command.since,
                "state_store_unavailable",
                format!("open authoritative state store before receipt pack: {error}"),
            );
            print_receipt_pack_payload(payload, command.json, command.fields.as_deref());
            return ExitCode::from(1);
        }
    };

    match build_taskflow_receipt_pack_payload(&store, &command).await {
        Ok(payload) => {
            print_receipt_pack_payload(payload, command.json, command.fields.as_deref());
            ExitCode::SUCCESS
        }
        Err(error) => {
            let payload =
                blocked_payload(command.since, "receipt_pack_unavailable", error.to_string());
            print_receipt_pack_payload(payload, command.json, command.fields.as_deref());
            ExitCode::from(1)
        }
    }
}

fn parse_taskflow_receipt_pack_args(args: &[String]) -> Result<TaskflowReceiptPackCommand, String> {
    let mut since = None;
    let mut json = false;
    let mut fields = None;
    let mut state_dir = None;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" => return Err("help".to_string()),
            "--json" => json = true,
            "--since" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("missing value for --since".to_string());
                };
                since = Some(value.clone());
            }
            "--fields" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("missing value for --fields".to_string());
                };
                fields = Some(value.clone());
            }
            "--state-dir" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("missing value for --state-dir".to_string());
                };
                state_dir = Some(PathBuf::from(value));
            }
            value => {
                return Err(format!(
                    "unexpected vida taskflow receipt-pack arg `{value}`"
                ));
            }
        }
        index += 1;
    }

    Ok(TaskflowReceiptPackCommand {
        since,
        json,
        fields,
        state_dir: state_dir.unwrap_or_else(crate::taskflow_task_bridge::proxy_state_dir),
    })
}

fn print_taskflow_receipt_pack_help() {
    println!("VIDA TaskFlow help: receipt-pack");
    println!();
    println!("Purpose:");
    println!("  Build a compact read-only evidence pack for final operator reports.");
    println!(
        "  It aggregates closed tasks, closure receipts, verification counts, quality gate command refs, artifacts, and git refs."
    );
    println!();
    println!("Canonical command:");
    println!(
        "  vida taskflow receipt-pack --since <commit-or-time> [--fields <field,...>] [--state-dir <path>] [--json]"
    );
    println!("  Default human output is compact TOON/plain.");
    println!("  Use --json only when a machine-readable payload is required.");
    println!();
    println!("Returned fields:");
    println!(
        "  closed_tasks, closed_epics, exception_receipts, verification_receipts, quality_gates, artifacts, git_refs"
    );
}

async fn build_taskflow_receipt_pack_payload(
    store: &StateStore,
    command: &TaskflowReceiptPackCommand,
) -> Result<ReceiptPackPayload, StateStoreError> {
    let tasks = store.all_tasks().await?;
    let closed_tasks = closed_task_refs(&tasks, false);
    let closed_epics = closed_task_refs(&tasks, true);
    let latest_status = store.latest_run_graph_status().await?;
    let latest_dispatch = store.latest_run_graph_dispatch_receipt_summary().await?;
    let latest_recovery = store.latest_run_graph_recovery_summary().await?;
    let latest_reconciliation = store.latest_task_reconciliation_summary().await?;
    let migration_receipts = store.migration_receipt_summary().await?;
    let exception_receipts = exception_receipt_ids(latest_dispatch.as_ref());
    let project_root =
        crate::taskflow_task_bridge::infer_project_root_from_state_root(store.root())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
    let git_refs = git_refs(&project_root, command.since.as_deref());
    let artifacts = serde_json::json!({
        "latest_run_graph_status": latest_status.as_ref().map(|status| serde_json::json!({
            "run_id": status.run_id,
            "task_id": status.task_id,
            "status": status.status,
            "active_node": status.active_node,
            "lifecycle_stage": status.lifecycle_stage,
        })),
        "latest_run_graph_recovery": latest_recovery.as_ref().map(|recovery| serde_json::json!({
            "run_id": recovery.run_id,
            "task_id": recovery.task_id,
            "recovery_ready": recovery.recovery_ready,
            "resume_target": recovery.resume_target,
        })),
        "state_dir": command.state_dir.display().to_string(),
    });
    let latest_receipts = ReceiptPackLatestReceipts {
        dispatch_run_id: latest_dispatch
            .as_ref()
            .map(|receipt| receipt.run_id.clone()),
        dispatch_status: latest_dispatch
            .as_ref()
            .map(|receipt| receipt.dispatch_status.clone()),
        exception_receipt_ids: exception_receipts.clone(),
        verification_receipts: migration_receipts.verification_receipts,
        task_reconciliation_receipt_id: latest_reconciliation
            .as_ref()
            .map(|receipt| receipt.receipt_id.clone()),
    };
    let artifact_refs = serde_json::json!({
        "state_dir": command.state_dir.display().to_string(),
        "latest_run_graph_run_id": latest_status.as_ref().map(|status| status.run_id.clone()),
        "latest_dispatch_run_id": latest_receipts.dispatch_run_id.clone(),
        "latest_task_reconciliation_receipt_id": latest_receipts.task_reconciliation_receipt_id.clone(),
    });
    let blocker_codes = Vec::<String>::new();
    let next_actions = Vec::<String>::new();
    let verdict = finalize_operator_surface_verdict(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        "pass",
        blocker_codes.clone(),
        next_actions.clone(),
        artifact_refs,
    );

    Ok(ReceiptPackPayload {
        surface: "vida taskflow receipt-pack",
        status: "pass",
        since: command.since.clone(),
        closed_tasks,
        closed_epics,
        exception_receipts,
        verification_receipts: migration_receipts.verification_receipts,
        quality_gates: vec!["vida quality gate --prepush".to_string()],
        artifacts,
        git_refs,
        latest_receipts,
        blocker_codes,
        next_actions,
        artifact_refs: verdict.artifact_refs,
        shared_fields: verdict.shared_fields,
        operator_contracts: verdict.operator_contracts,
    })
}

fn closed_task_refs(tasks: &[TaskRecord], epics_only: bool) -> Vec<ReceiptPackTaskRef> {
    tasks
        .iter()
        .filter(|task| StateStore::task_status_is_closed_like(&task.status))
        .filter(|task| work_item_is_program_container(&task.issue_type) == epics_only)
        .map(task_ref)
        .collect()
}

fn task_ref(task: &TaskRecord) -> ReceiptPackTaskRef {
    ReceiptPackTaskRef {
        id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        issue_type: task.issue_type.clone(),
        closed_at: task.closed_at.clone(),
        close_reason: task.close_reason.clone(),
    }
}

fn exception_receipt_ids(
    dispatch: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(dispatch) = dispatch {
        if let Some(id) = dispatch.exception_path_receipt_id.as_deref() {
            ids.push(id.to_string());
        }
        if let Some(id) = dispatch.supersedes_receipt_id.as_deref() {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

fn git_refs(project_root: &Path, since: Option<&str>) -> ReceiptPackGitRefs {
    let head = git_output(project_root, &["rev-parse", "HEAD"]);
    let branch = git_output(project_root, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let dirty_files_count = git_output(project_root, &["status", "--porcelain", "-uall"])
        .map(|output| output.lines().count())
        .unwrap_or(0);
    let commits_since = since
        .and_then(|since| {
            git_output(
                project_root,
                &["log", "--oneline", &format!("{since}..HEAD")],
            )
        })
        .map(|output| output.lines().map(str::to_string).collect())
        .unwrap_or_default();
    let changed_files_since = since
        .and_then(|since| {
            git_output(
                project_root,
                &["diff", "--name-only", &format!("{since}..HEAD")],
            )
        })
        .map(|output| output.lines().map(str::to_string).collect())
        .unwrap_or_default();
    let status = if head.is_some() {
        "pass"
    } else {
        "unavailable"
    }
    .to_string();

    ReceiptPackGitRefs {
        status,
        since: since.map(str::to_string),
        head,
        branch,
        commits_since,
        changed_files_since,
        dirty_files_count,
    }
}

fn git_output(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn blocked_payload(since: Option<String>, blocker_code: &str, reason: String) -> serde_json::Value {
    let blocker_codes = vec![blocker_code.to_string()];
    let next_actions =
        vec!["Run `vida taskflow closeout` before building a receipt pack.".to_string()];
    let artifact_refs = serde_json::json!({
        "reason": reason,
    });
    let verdict = finalize_operator_surface_verdict(
        &RELEASE1_OPERATOR_CONTRACT_SPEC,
        "blocked",
        blocker_codes.clone(),
        next_actions.clone(),
        artifact_refs,
    );
    serde_json::json!({
        "surface": "vida taskflow receipt-pack",
        "status": "blocked",
        "since": since,
        "closed_tasks": [],
        "closed_epics": [],
        "exception_receipts": [],
        "verification_receipts": 0,
        "quality_gates": [],
        "artifacts": {},
        "git_refs": {
            "status": "unavailable",
            "since": null,
            "head": null,
            "branch": null,
            "commits_since": [],
            "changed_files_since": [],
            "dirty_files_count": 0,
        },
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": verdict.artifact_refs,
        "shared_fields": verdict.shared_fields,
        "operator_contracts": verdict.operator_contracts,
    })
}

fn print_receipt_pack_payload<T: Serialize>(payload: T, json: bool, fields: Option<&str>) {
    let payload = serde_json::to_value(payload).expect("receipt pack payload should serialize");
    let payload = operator_output::toon_report::select_fields(payload, fields);
    if json {
        crate::print_json_pretty(&payload);
    } else {
        println!(
            "{}",
            operator_output::toon_report::render_value("vida taskflow receipt-pack", payload)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exception_receipt_ids_deduplicates_receipt_scope() {
        let mut receipt = crate::state_store::RunGraphDispatchReceiptSummary {
            run_id: "run-1".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_completed".to_string(),
            dispatch_target: "worker".to_string(),
            exception_path_receipt_id: Some("exception-1".to_string()),
            supersedes_receipt_id: Some("exception-1".to_string()),
            dispatch_kind: "agent_init".to_string(),
            dispatch_surface: None,
            dispatch_command: None,
            downstream_dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: None,
            downstream_dispatch_target: None,
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
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: None,
            effective_execution_posture: serde_json::json!({}),
            route_policy: serde_json::json!({}),
            activation_evidence: serde_json::json!({}),
            recorded_at: "2026-06-06T00:00:00Z".to_string(),
        };
        assert_eq!(exception_receipt_ids(Some(&receipt)), vec!["exception-1"]);
        receipt.supersedes_receipt_id = Some("exception-2".to_string());
        assert_eq!(
            exception_receipt_ids(Some(&receipt)),
            vec!["exception-1".to_string(), "exception-2".to_string()]
        );
    }
}
