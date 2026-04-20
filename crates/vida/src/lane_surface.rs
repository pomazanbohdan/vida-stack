use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

use crate::contract_profile_adapter::render_operator_contract_envelope;
use crate::taskflow_task_bridge::proxy_state_dir;
use crate::{state_store::StateStore, ProxyArgs};

#[derive(Serialize)]
struct LaneEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    run_id: String,
    lane_id: Option<String>,
    runtime_role: Option<String>,
    lane_status: String,
    selected_backend: Option<String>,
    dispatch_status: String,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
    exception_path_metadata_path: Option<String>,
    exception_path_metadata: Option<ExceptionTakeoverMetadata>,
}

#[derive(Serialize)]
struct BlockedLaneEnvelope {
    surface: &'static str,
    status: &'static str,
    trace_id: Option<String>,
    workflow_class: Option<String>,
    risk_tier: Option<String>,
    artifact_refs: serde_json::Value,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
    reason: String,
}

enum LaneCommand<'a> {
    ShowLatest {
        as_json: bool,
    },
    ShowRun {
        run_id: &'a str,
        as_json: bool,
    },
    Complete {
        run_id: &'a str,
        receipt_id: &'a str,
        as_json: bool,
    },
    ExceptionTakeover {
        run_id: &'a str,
        receipt_id: &'a str,
        metadata: ExceptionTakeoverMetadata,
        as_json: bool,
    },
    Supersede {
        run_id: &'a str,
        receipt_id: &'a str,
        as_json: bool,
    },
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
struct ExceptionTakeoverMetadata {
    reason_class: String,
    active_bounded_unit: String,
    owned_write_scope: Vec<String>,
    why_delegated_or_rerouted_path_is_not_currently_lawful: String,
    why_local_write_is_the_smallest_safe_bounded_workaround: String,
    return_to_normal_posture_condition: String,
    verification_plan: Vec<String>,
    recorded_at: String,
}

impl ExceptionTakeoverMetadata {
    fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("reason_class", self.reason_class.trim()),
            ("active_bounded_unit", self.active_bounded_unit.trim()),
            (
                "why_delegated_or_rerouted_path_is_not_currently_lawful",
                self.why_delegated_or_rerouted_path_is_not_currently_lawful
                    .trim(),
            ),
            (
                "why_local_write_is_the_smallest_safe_bounded_workaround",
                self.why_local_write_is_the_smallest_safe_bounded_workaround
                    .trim(),
            ),
            (
                "return_to_normal_posture_condition",
                self.return_to_normal_posture_condition.trim(),
            ),
            ("recorded_at", self.recorded_at.trim()),
        ] {
            if value.is_empty() {
                return Err(format!(
                    "exception takeover metadata field `{field}` must be non-empty"
                ));
            }
        }
        if self.owned_write_scope.is_empty()
            || self
                .owned_write_scope
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(
                "exception takeover metadata requires at least one non-empty `owned_write_scope` entry"
                    .to_string(),
            );
        }
        if self.verification_plan.is_empty()
            || self
                .verification_plan
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(
                "exception takeover metadata requires at least one non-empty `verification_plan` entry"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn lane_usage() -> &'static str {
    "Usage: vida lane show <run-id> [--json]\n       vida lane show --latest [--json]\n       vida lane complete <run-id> --receipt-id <id> [--json]\n       vida lane exception-takeover <run-id> --receipt-id <id> --reason-class <class> --active-bounded-unit <unit> --owned-write-scope <path> [--owned-write-scope <path> ...] --why-delegated-path-not-lawful <text> --why-local-write-safe <text> --return-to-normal-when <text> --verification-step <text> [--verification-step <text> ...] [--json]\n       vida lane supersede <run-id> --receipt-id <id> [--json]"
}

fn parse_lane_args<'a>(args: &'a [String]) -> Result<LaneCommand<'a>, String> {
    match args {
        [] => Err(lane_usage().to_string()),
        [flag] => {
            if matches!(flag.as_str(), "-h" | "--help") {
                Err(lane_usage().to_string())
            } else {
                Err(lane_usage().to_string())
            }
        }
        [head, rest @ ..] if head == "show" => {
            let mut as_json = false;
            let mut latest = false;
            let mut run_id = None;
            for arg in rest {
                match arg.as_str() {
                    "--json" => as_json = true,
                    "--latest" => latest = true,
                    value if !value.starts_with('-') && run_id.is_none() => run_id = Some(value),
                    _ => return Err(lane_usage().to_string()),
                }
            }
            if latest {
                if run_id.is_some() {
                    return Err(lane_usage().to_string());
                }
                return Ok(LaneCommand::ShowLatest { as_json });
            }
            let Some(run_id) = run_id else {
                return Err(lane_usage().to_string());
            };
            Ok(LaneCommand::ShowRun { run_id, as_json })
        }
        [head, run_id, rest @ ..] if head == "complete" => {
            let mut as_json = false;
            let mut receipt_id = None;
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--json" => {
                        as_json = true;
                        index += 1;
                    }
                    "--receipt-id" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        receipt_id = Some(value.as_str());
                        index += 2;
                    }
                    _ => return Err(lane_usage().to_string()),
                }
            }
            let Some(receipt_id) = receipt_id else {
                return Err(lane_usage().to_string());
            };
            Ok(LaneCommand::Complete {
                run_id,
                receipt_id,
                as_json,
            })
        }
        [head, run_id, rest @ ..] if head == "exception-takeover" => {
            let mut as_json = false;
            let mut receipt_id = None;
            let mut reason_class = None;
            let mut active_bounded_unit = None;
            let mut owned_write_scope = Vec::new();
            let mut why_delegated_path_not_lawful = None;
            let mut why_local_write_safe = None;
            let mut return_to_normal_when = None;
            let mut verification_plan = Vec::new();
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--json" => {
                        as_json = true;
                        index += 1;
                    }
                    "--receipt-id" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        receipt_id = Some(value.as_str());
                        index += 2;
                    }
                    "--reason-class" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        reason_class = Some(value.as_str());
                        index += 2;
                    }
                    "--active-bounded-unit" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        active_bounded_unit = Some(value.as_str());
                        index += 2;
                    }
                    "--owned-write-scope" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        owned_write_scope.push(value.to_string());
                        index += 2;
                    }
                    "--why-delegated-path-not-lawful" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        why_delegated_path_not_lawful = Some(value.as_str());
                        index += 2;
                    }
                    "--why-local-write-safe" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        why_local_write_safe = Some(value.as_str());
                        index += 2;
                    }
                    "--return-to-normal-when" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        return_to_normal_when = Some(value.as_str());
                        index += 2;
                    }
                    "--verification-step" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        verification_plan.push(value.to_string());
                        index += 2;
                    }
                    _ => return Err(lane_usage().to_string()),
                }
            }
            let Some(receipt_id) = receipt_id else {
                return Err(lane_usage().to_string());
            };
            let metadata = ExceptionTakeoverMetadata {
                reason_class: reason_class.unwrap_or_default().to_string(),
                active_bounded_unit: active_bounded_unit.unwrap_or_default().to_string(),
                owned_write_scope,
                why_delegated_or_rerouted_path_is_not_currently_lawful:
                    why_delegated_path_not_lawful
                        .unwrap_or_default()
                        .to_string(),
                why_local_write_is_the_smallest_safe_bounded_workaround: why_local_write_safe
                    .unwrap_or_default()
                    .to_string(),
                return_to_normal_posture_condition: return_to_normal_when
                    .unwrap_or_default()
                    .to_string(),
                verification_plan,
                recorded_at: time::OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .expect("rfc3339 timestamp should render"),
            };
            metadata.validate()?;
            Ok(LaneCommand::ExceptionTakeover {
                run_id,
                receipt_id,
                metadata,
                as_json,
            })
        }
        [head, run_id, rest @ ..] if head == "supersede" => {
            let mut as_json = false;
            let mut receipt_id = None;
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--json" => {
                        as_json = true;
                        index += 1;
                    }
                    "--receipt-id" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        receipt_id = Some(value.as_str());
                        index += 2;
                    }
                    _ => return Err(lane_usage().to_string()),
                }
            }
            let Some(receipt_id) = receipt_id else {
                return Err(lane_usage().to_string());
            };
            Ok(LaneCommand::Supersede {
                run_id,
                receipt_id,
                as_json,
            })
        }
        _ => Err(lane_usage().to_string()),
    }
}

#[cfg(test)]
fn exception_takeover_allowed(
    receipt: &crate::state_store::RunGraphDispatchReceiptSummary,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    crate::release1_contracts::exception_takeover_state(
        Some("pending-exception-receipt"),
        receipt.supersedes_receipt_id.as_deref(),
        recovery.map(|recovery| {
            recovery
                .delegation_gate
                .local_exception_takeover_gate
                .as_str()
        }),
    )
    .is_active()
}

fn build_lane_envelope(
    summary: crate::state_store::RunGraphDispatchReceiptSummary,
    status: Option<crate::state_store::RunGraphStatus>,
    exception_path_metadata_path: Option<String>,
    exception_path_metadata: Option<ExceptionTakeoverMetadata>,
    blocked: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
) -> LaneEnvelope {
    let run_id = summary.run_id.clone();
    let dispatch_packet_path = summary.dispatch_packet_path.clone();
    let dispatch_result_path = summary.dispatch_result_path.clone();
    let downstream_dispatch_packet_path = summary.downstream_dispatch_packet_path.clone();
    let downstream_dispatch_result_path = summary.downstream_dispatch_result_path.clone();
    let exception_path_receipt_id = summary.exception_path_receipt_id.clone();
    let supersedes_receipt_id = summary.supersedes_receipt_id.clone();
    let lane_status = summary.lane_status.clone();
    let dispatch_status = summary.dispatch_status.clone();
    let selected_backend = status
        .as_ref()
        .map(|status| status.selected_backend.clone())
        .or(summary.selected_backend.clone());
    let artifact_refs = serde_json::json!({
        "latest_run_graph_dispatch_receipt_id": run_id.clone(),
        "exception_path_receipt_id": exception_path_receipt_id.clone(),
        "exception_path_metadata_path": exception_path_metadata_path.clone(),
        "dispatch_packet_path": dispatch_packet_path.clone(),
        "dispatch_result_path": dispatch_result_path.clone(),
        "downstream_dispatch_packet_path": downstream_dispatch_packet_path.clone(),
        "downstream_dispatch_result_path": downstream_dispatch_result_path.clone(),
    });
    let operator_contracts = render_operator_contract_envelope(
        if blocked { "blocked" } else { "pass" },
        blocker_codes.clone(),
        next_actions.clone(),
        artifact_refs,
    );
    let surface_status = if operator_contracts["status"].as_str() == Some("blocked") {
        "blocked"
    } else {
        "pass"
    };
    LaneEnvelope {
        surface: "vida lane",
        status: surface_status,
        trace_id: operator_contracts["trace_id"]
            .as_str()
            .map(ToOwned::to_owned),
        workflow_class: operator_contracts["workflow_class"]
            .as_str()
            .map(ToOwned::to_owned),
        risk_tier: operator_contracts["risk_tier"]
            .as_str()
            .map(ToOwned::to_owned),
        artifact_refs: operator_contracts["artifact_refs"].clone(),
        next_actions,
        blocker_codes,
        run_id,
        lane_id: status.as_ref().map(|status| status.lane_id.clone()),
        runtime_role: status
            .as_ref()
            .map(|status| status.task_class.clone())
            .or(summary.activation_runtime_role.clone()),
        lane_status,
        selected_backend,
        dispatch_status,
        supersedes_receipt_id,
        exception_path_receipt_id,
        exception_path_metadata_path,
        exception_path_metadata,
    }
}

struct LaneShowTruth {
    blocked: bool,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
}

fn recovery_takeover_gate(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> Option<&str> {
    recovery.map(|recovery| {
        recovery
            .delegation_gate
            .local_exception_takeover_gate
            .as_str()
    })
}

fn derive_lane_show_truth(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> LaneShowTruth {
    let takeover_state = crate::release1_contracts::exception_takeover_state(
        summary.exception_path_receipt_id.as_deref(),
        summary.supersedes_receipt_id.as_deref(),
        recovery_takeover_gate(recovery),
    );

    if summary.lane_status == crate::LaneStatus::LaneExceptionTakeover.as_str()
        && takeover_state.is_active()
    {
        return LaneShowTruth {
            blocked: false,
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
        };
    }

    if summary.lane_status == crate::LaneStatus::LaneSuperseded.as_str() {
        return LaneShowTruth {
            blocked: false,
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
        };
    }

    let mut blocked = matches!(summary.dispatch_status.as_str(), "blocked" | "failed")
        || matches!(summary.lane_status.as_str(), "lane_blocked" | "lane_failed");
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();

    if let Some(blocker_code) = summary
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }

    if blocked {
        blocker_codes.extend(
            summary
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }

    if summary.lane_status == crate::LaneStatus::LaneExceptionRecorded.as_str() {
        blocked = true;
        if recovery.is_some_and(|recovery| {
            recovery.delegation_gate.local_exception_takeover_gate == "blocked_open_delegated_cycle"
                || recovery.delegation_gate.delegated_cycle_open
        }) {
            blocker_codes.push("open_delegated_cycle".to_string());
            next_actions.push(
                "Exception-path receipt recorded; delegated cycle is still open, so root-local write remains blocked."
                    .to_string(),
            );
        } else {
            blocker_codes.push("supersession_without_receipt".to_string());
            next_actions.push(
                format!(
                    "Exception-path receipt recorded; record explicit supersession with `vida lane supersede {} --receipt-id <id>` before local write becomes active.",
                    summary.run_id
                ),
            );
        }
    }

    LaneShowTruth {
        blocked,
        blocker_codes: crate::release1_contracts::canonical_blocker_code_list(&blocker_codes),
        next_actions,
    }
}

fn lane_takeover_state(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> crate::release1_contracts::ExceptionTakeoverState {
    crate::release1_contracts::exception_takeover_state(
        receipt.exception_path_receipt_id.as_deref(),
        receipt.supersedes_receipt_id.as_deref(),
        recovery_takeover_gate(recovery),
    )
}

fn explicit_lane_status_for_receipt(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> String {
    let takeover_active = lane_takeover_state(receipt, recovery).is_active();
    if receipt
        .exception_path_receipt_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && receipt
            .supersedes_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && takeover_active
    {
        return crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
    }
    if receipt
        .supersedes_receipt_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return crate::LaneStatus::LaneSuperseded.as_str().to_string();
    }
    crate::derive_lane_status(
        &receipt.dispatch_status,
        receipt.supersedes_receipt_id.as_deref(),
        receipt.exception_path_receipt_id.as_deref(),
    )
    .as_str()
    .to_string()
}

fn emit_lane_envelope(envelope: &LaneEnvelope, as_json: bool) -> ExitCode {
    if crate::surface_render::print_surface_json(envelope, as_json, "lane surface should serialize")
    {
        return if envelope.status == "pass" {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(2)
        };
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(crate::RenderMode::Plain, "run_id", &envelope.run_id);
    if let Some(trace_id) = envelope.trace_id.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "trace_id", trace_id);
    }
    if let Some(workflow_class) = envelope.workflow_class.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "workflow_class", workflow_class);
    }
    if let Some(risk_tier) = envelope.risk_tier.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "risk_tier", risk_tier);
    }
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "lane_status",
        &envelope.lane_status,
    );
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "dispatch_status",
        &envelope.dispatch_status,
    );
    if !envelope.blocker_codes.is_empty() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "blocker_codes",
            &envelope.blocker_codes.join(", "),
        );
    }
    if let Some(lane_id) = envelope.lane_id.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "lane_id", lane_id);
    }
    if let Some(runtime_role) = envelope.runtime_role.as_deref() {
        crate::print_surface_line(crate::RenderMode::Plain, "runtime_role", runtime_role);
    }
    if let Some(selected_backend) = envelope.selected_backend.as_deref() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "selected_backend",
            selected_backend,
        );
    }
    if let Some(receipt_id) = envelope.exception_path_receipt_id.as_deref() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "exception_path_receipt_id",
            receipt_id,
        );
    }
    if let Some(receipt_id) = envelope.supersedes_receipt_id.as_deref() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "supersedes_receipt_id",
            receipt_id,
        );
    }
    if let Some(path) = envelope.exception_path_metadata_path.as_deref() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "exception_path_metadata_path",
            path,
        );
    }
    if let Some(metadata) = envelope.exception_path_metadata.as_ref() {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "exception_reason_class",
            &metadata.reason_class,
        );
    }
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "artifact_refs",
        &envelope.artifact_refs.to_string(),
    );
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    if envelope.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn emit_blocked_lane_envelope(as_json: bool) -> ExitCode {
    let next_actions = vec![
        "Use `vida lane show --latest --json` or `vida lane show <run-id> --json` to inspect the current lane envelope, then record exception-path evidence with `vida lane exception-takeover` or explicit supersession with `vida lane supersede` as needed."
            .to_string(),
    ];
    let operator_contracts = render_operator_contract_envelope(
        "blocked",
        vec!["unsupported_blocker_code".to_string()],
        next_actions.clone(),
        serde_json::json!([]),
    );
    let status = if operator_contracts["status"].as_str() == Some("blocked") {
        "blocked"
    } else {
        "pass"
    };
    let envelope = BlockedLaneEnvelope {
        surface: "vida lane",
        status,
        trace_id: operator_contracts["trace_id"].as_str().map(ToOwned::to_owned),
        workflow_class: operator_contracts["workflow_class"]
            .as_str()
            .map(ToOwned::to_owned),
        risk_tier: operator_contracts["risk_tier"].as_str().map(ToOwned::to_owned),
        artifact_refs: operator_contracts["artifact_refs"].clone(),
        next_actions,
        blocker_codes: operator_contracts["blocker_codes"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        reason: "vida lane requires a bounded subcommand; the root surface fails closed instead of inferring one."
            .to_string(),
    };

    if crate::surface_render::print_surface_json(
        &envelope,
        as_json,
        "blocked lane surface should serialize",
    ) {
        return ExitCode::from(2);
    }

    crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
    crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
    crate::print_surface_line(
        crate::RenderMode::Plain,
        "blocker_codes",
        &envelope.blocker_codes.join(", "),
    );
    crate::print_surface_line(crate::RenderMode::Plain, "reason", &envelope.reason);
    if let Some(next_action) = envelope.next_actions.first() {
        crate::print_surface_line(crate::RenderMode::Plain, "next_action", next_action);
    }
    ExitCode::from(2)
}

fn exception_takeover_metadata_dir(state_root: &Path) -> PathBuf {
    state_root.join("lane-exception-path-metadata")
}

fn exception_takeover_metadata_path(state_root: &Path, run_id: &str) -> PathBuf {
    exception_takeover_metadata_dir(state_root).join(format!("{run_id}.json"))
}

fn read_exception_takeover_metadata(
    state_root: &Path,
    run_id: &str,
) -> Result<Option<ExceptionTakeoverMetadata>, String> {
    let path = exception_takeover_metadata_path(state_root, run_id);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "Failed to read persisted exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    let metadata: ExceptionTakeoverMetadata = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to decode persisted exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    metadata.validate()?;
    Ok(Some(metadata))
}

fn write_exception_takeover_metadata(
    state_root: &Path,
    run_id: &str,
    metadata: &ExceptionTakeoverMetadata,
) -> Result<String, String> {
    metadata.validate()?;
    let dir = exception_takeover_metadata_dir(state_root);
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "Failed to create exception takeover metadata directory `{}`: {error}",
            dir.display()
        )
    })?;
    let path = exception_takeover_metadata_path(state_root, run_id);
    let encoded = serde_json::to_string_pretty(metadata).map_err(|error| {
        format!(
            "Failed to encode exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    std::fs::write(&path, encoded).map_err(|error| {
        format!(
            "Failed to persist exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    Ok(path.display().to_string())
}

fn lane_mutation_status_guard(
    run_id: &str,
    status: Option<&crate::state_store::RunGraphStatus>,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Result<(), String> {
    let Some(status) = status else {
        return Err(format!(
            "Lane `{run_id}` has no authoritative run-graph status, so the lane surface cannot prove this run is still active for mutation."
        ));
    };
    if receipt.lane_status == crate::LaneStatus::LaneSuperseded.as_str() {
        return Err(format!(
            "Lane `{run_id}` is already superseded; record a new active lane instead of mutating superseded evidence."
        ));
    }
    let terminal_completed_without_next_unit = status.lifecycle_stage == "closure_complete"
        && status
            .next_node
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none();
    let recovery_terminal = recovery.is_some_and(|recovery| {
        recovery.resume_status == "completed" && recovery.lifecycle_stage == "closure_complete"
    });
    if status.status == "completed" || terminal_completed_without_next_unit || recovery_terminal {
        return Err(format!(
            "Lane `{run_id}` is no longer active for mutation because run-graph status is terminal (`{}` / `{}`).",
            status.status, status.lifecycle_stage
        ));
    }
    Ok(())
}

fn read_lane_packet(path: &str) -> Result<serde_json::Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read persisted lane packet `{path}`: {error}"))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to decode persisted lane packet `{path}`: {error}"))
}

fn write_lane_packet(path: &str, packet: &serde_json::Value) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(packet)
        .map_err(|error| format!("Failed to encode persisted lane packet `{path}`: {error}"))?;
    std::fs::write(path, encoded)
        .map_err(|error| format!("Failed to write persisted lane packet `{path}`: {error}"))
}

pub(crate) async fn run_lane(args: ProxyArgs) -> ExitCode {
    if args.args.is_empty() || args.args.iter().all(|arg| arg.starts_with('-')) {
        return emit_blocked_lane_envelope(args.args.iter().any(|arg| arg == "--json"));
    }

    let command = match parse_lane_args(&args.args) {
        Ok(command) => command,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    let state_dir = proxy_state_dir();
    let store = match StateStore::open_existing(state_dir).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Failed to open authoritative state store: {error}");
            return ExitCode::from(1);
        }
    };

    match command {
        LaneCommand::ShowLatest { as_json } => {
            let Some(summary) = (match store.latest_run_graph_dispatch_receipt_summary().await {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("Failed to read latest lane receipt summary: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("No lane receipt found.");
                return ExitCode::from(2);
            };
            let status = match store.run_graph_status(&summary.run_id).await {
                Ok(status) => Some(status),
                Err(_) => None,
            };
            let recovery = store.run_graph_recovery_summary(&summary.run_id).await.ok();
            let exception_path_metadata_path =
                exception_takeover_metadata_path(store.root(), &summary.run_id);
            let exception_path_metadata =
                match read_exception_takeover_metadata(store.root(), &summary.run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let truth = derive_lane_show_truth(&summary, recovery.as_ref());
            let envelope = build_lane_envelope(
                summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::ShowRun { run_id, as_json } => {
            let Some(receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing lane receipt for `{run_id}`.");
                return ExitCode::from(2);
            };
            let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let status = match store.run_graph_status(run_id).await {
                Ok(status) => Some(status),
                Err(_) => None,
            };
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let exception_path_metadata_path =
                exception_takeover_metadata_path(store.root(), run_id);
            let exception_path_metadata =
                match read_exception_takeover_metadata(store.root(), run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let truth = derive_lane_show_truth(&summary, recovery.as_ref());
            let envelope = build_lane_envelope(
                summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::Complete {
            run_id,
            receipt_id,
            as_json,
        } => {
            let Some(mut receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing lane receipt for `{run_id}`.");
                return ExitCode::from(2);
            };
            let Some(packet_path) = receipt.downstream_dispatch_packet_path.clone() else {
                eprintln!(
                    "Lane `{run_id}` has no persisted downstream dispatch packet for bounded completion evidence."
                );
                return ExitCode::from(2);
            };
            let mut packet = match read_lane_packet(&packet_path) {
                Ok(packet) => packet,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            let completed_target = packet
                .get("downstream_dispatch_active_target")
                .and_then(serde_json::Value::as_str)
                .or(receipt.downstream_dispatch_active_target.as_deref())
                .or(receipt.downstream_dispatch_last_target.as_deref())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(receipt.dispatch_target.as_str())
                .to_string();
            let completion_result_path =
                match crate::runtime_dispatch_state::write_runtime_lane_completion_result(
                    store.root(),
                    run_id,
                    &completed_target,
                    receipt_id,
                    &packet_path,
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            packet["downstream_dispatch_ready"] = serde_json::json!(true);
            packet["downstream_dispatch_blockers"] = serde_json::json!([]);
            packet["downstream_dispatch_status"] = serde_json::json!("packet_ready");
            packet["downstream_dispatch_result_path"] =
                serde_json::json!(completion_result_path.clone());
            packet["downstream_lane_status"] = serde_json::json!("packet_ready");
            packet["downstream_dispatch_active_target"] = serde_json::json!(completed_target);
            if let Err(error) = write_lane_packet(&packet_path, &packet) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }

            receipt.downstream_dispatch_ready = true;
            receipt.downstream_dispatch_blockers.clear();
            receipt.downstream_dispatch_status = Some("packet_ready".to_string());
            receipt.downstream_dispatch_result_path = Some(completion_result_path);
            receipt.downstream_dispatch_active_target = Some(completed_target.clone());
            receipt.downstream_dispatch_last_target = Some(completed_target);
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist lane completion evidence: {error}");
                return ExitCode::from(1);
            }

            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let status = store.run_graph_status(run_id).await.ok();
            let truth = derive_lane_show_truth(&updated_summary, None);
            let exception_path_metadata_path =
                exception_takeover_metadata_path(store.root(), run_id);
            let exception_path_metadata =
                match read_exception_takeover_metadata(store.root(), run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let envelope = build_lane_envelope(
                updated_summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::ExceptionTakeover {
            run_id,
            receipt_id,
            metadata,
            as_json,
        } => {
            let Some(mut receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing lane receipt for `{run_id}`.");
                return ExitCode::from(2);
            };
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let status = store.run_graph_status(run_id).await.ok();
            if let Err(error) =
                lane_mutation_status_guard(run_id, status.as_ref(), recovery.as_ref(), &receipt)
            {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
            receipt.exception_path_receipt_id = Some(receipt_id.to_string());
            receipt.lane_status = explicit_lane_status_for_receipt(&receipt, recovery.as_ref());
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist exception takeover receipt: {error}");
                return ExitCode::from(1);
            }
            let metadata_path =
                match write_exception_takeover_metadata(store.root(), run_id, &metadata) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let truth = derive_lane_show_truth(&updated_summary, recovery.as_ref());
            let envelope = build_lane_envelope(
                updated_summary,
                status,
                Some(metadata_path),
                Some(metadata),
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope(&envelope, as_json)
        }
        LaneCommand::Supersede {
            run_id,
            receipt_id,
            as_json,
        } => {
            let Some(mut receipt) = (match store.run_graph_dispatch_receipt(run_id).await {
                Ok(receipt) => receipt,
                Err(error) => {
                    eprintln!("Failed to read lane receipt `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing lane receipt for `{run_id}`.");
                return ExitCode::from(2);
            };
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let status = store.run_graph_status(run_id).await.ok();
            if let Err(error) =
                lane_mutation_status_guard(run_id, status.as_ref(), recovery.as_ref(), &receipt)
            {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
            receipt.supersedes_receipt_id = Some(receipt_id.to_string());
            receipt.lane_status = explicit_lane_status_for_receipt(&receipt, recovery.as_ref());
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist superseded lane receipt: {error}");
                return ExitCode::from(1);
            }
            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let exception_path_metadata_path =
                exception_takeover_metadata_path(store.root(), run_id);
            let exception_path_metadata =
                match read_exception_takeover_metadata(store.root(), run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let truth = derive_lane_show_truth(&updated_summary, recovery.as_ref());
            let envelope = build_lane_envelope(
                updated_summary,
                status,
                exception_path_metadata_path
                    .exists()
                    .then(|| exception_path_metadata_path.display().to_string()),
                exception_path_metadata,
                truth.blocked,
                truth.blocker_codes,
                truth.next_actions,
            );
            emit_lane_envelope(&envelope, as_json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn lane_surface_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn wait_for_state_unlock(state_dir: &std::path::Path) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let direct_lock_path = state_dir.join("LOCK");
        while direct_lock_path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    struct ProxyStateDirOverrideGuard;

    impl ProxyStateDirOverrideGuard {
        fn install(path: std::path::PathBuf) -> Self {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(Some(path));
            Self
        }
    }

    impl Drop for ProxyStateDirOverrideGuard {
        fn drop(&mut self) {
            crate::taskflow_task_bridge::set_test_proxy_state_dir_override(None);
        }
    }

    fn sample_receipt(dispatch_status: &str) -> crate::state_store::RunGraphDispatchReceipt {
        crate::state_store::RunGraphDispatchReceipt {
            run_id: "run-lane-test".to_string(),
            dispatch_target: "spec-pack".to_string(),
            dispatch_status: dispatch_status.to_string(),
            lane_status: String::new(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida taskflow bootstrap-spec".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
            blocker_code: Some("configured_backend_dispatch_failed".to_string()),
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
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("business_analyst".to_string()),
            selected_backend: Some("internal_subagents".to_string()),
            recorded_at: "2026-04-09T00:00:00Z".to_string(),
        }
    }

    fn sample_exception_takeover_args(run_id: &str, receipt_id: &str) -> Vec<String> {
        vec![
            "exception-takeover".to_string(),
            run_id.to_string(),
            "--receipt-id".to_string(),
            receipt_id.to_string(),
            "--reason-class".to_string(),
            "failed_lawful_reuse".to_string(),
            "--active-bounded-unit".to_string(),
            "feature-spec-compliant-exception-path-takeover-surface-dev".to_string(),
            "--owned-write-scope".to_string(),
            "crates/vida/src/lane_surface.rs".to_string(),
            "--why-delegated-path-not-lawful".to_string(),
            "delegated lane is blocked and cannot lawfully persist the required receipt"
                .to_string(),
            "--why-local-write-safe".to_string(),
            "mutation is bounded to the lane takeover surface and its targeted tests".to_string(),
            "--return-to-normal-when".to_string(),
            "return once canonical delegated execution is restored for the bounded unit"
                .to_string(),
            "--verification-step".to_string(),
            "cargo test -p vida lane_surface -- --nocapture".to_string(),
            "--json".to_string(),
        ]
    }

    #[test]
    fn parse_lane_show_latest_supports_json() {
        let args = vec![
            "show".to_string(),
            "--latest".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane show latest should parse");
        assert!(matches!(command, LaneCommand::ShowLatest { as_json: true }));
    }

    #[test]
    fn parse_lane_complete_supports_receipt_id_and_json() {
        let args = vec![
            "complete".to_string(),
            "run-1".to_string(),
            "--receipt-id".to_string(),
            "receipt-1".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane complete should parse");
        assert!(matches!(
            command,
            LaneCommand::Complete {
                run_id: "run-1",
                receipt_id: "receipt-1",
                as_json: true
            }
        ));
    }

    #[test]
    fn parse_lane_supersede_supports_receipt_id_and_json() {
        let args = vec![
            "supersede".to_string(),
            "run-1".to_string(),
            "--receipt-id".to_string(),
            "receipt-1".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane supersede should parse");
        assert!(matches!(
            command,
            LaneCommand::Supersede {
                run_id: "run-1",
                receipt_id: "receipt-1",
                as_json: true
            }
        ));
    }

    #[test]
    fn parse_lane_exception_takeover_requires_structured_metadata() {
        let args = sample_exception_takeover_args("run-1", "receipt-1");
        let command = parse_lane_args(&args).expect("lane exception takeover should parse");
        assert!(matches!(
            command,
            LaneCommand::ExceptionTakeover {
                run_id: "run-1",
                receipt_id: "receipt-1",
                as_json: true,
                ..
            }
        ));
    }

    #[test]
    fn exception_takeover_requires_more_than_a_recorded_receipt_when_recovery_is_missing() {
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(
            sample_receipt("executed"),
        );
        assert!(!exception_takeover_allowed(&summary, None));
    }

    #[test]
    fn derive_lane_show_truth_marks_blocked_dispatch_receipts_as_blocked() {
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(
            sample_receipt("blocked"),
        );

        let truth = derive_lane_show_truth(&summary, None);

        assert!(truth.blocked);
        assert!(truth.next_actions.is_empty());
    }

    #[test]
    fn derive_lane_show_truth_blocks_exception_recorded_open_cycle() {
        let mut receipt = sample_receipt("executed");
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "specification",
            "scope_discussion",
        );
        status.active_node = "implementer".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.status = "ready".to_string();
        let recovery = crate::state_store::RunGraphRecoverySummary::from_status(status);

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"open_delegated_cycle".to_string()));
    }

    #[test]
    fn derive_lane_show_truth_requires_supersession_after_exception_receipt_when_cycle_is_clear() {
        let mut receipt = sample_receipt("executed");
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "specification",
            "scope_discussion",
        );
        status.active_node = "closure".to_string();
        status.lifecycle_stage = "closure_pending".to_string();
        status.status = "blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        let recovery = crate::state_store::RunGraphRecoverySummary::from_status(status);

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"supersession_without_receipt".to_string()));
        assert!(truth
            .next_actions
            .iter()
            .any(|value| value.contains("vida lane supersede run-lane-test --receipt-id <id>")));
    }

    #[tokio::test]
    async fn lane_show_run_fails_closed_for_exception_recorded_open_cycle() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-show-run-open-cycle-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-show-open-cycle";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "implementer".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.status = "ready".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception-recorded receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec!["show".to_string(), run_id.to_string(), "--json".to_string()],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_exception_takeover_records_receipt_without_activating_local_write() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-test";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "spec-pack".to_string();
        status.status = "ready".to_string();
        status.lifecycle_stage = "spec_pack_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        store
            .record_run_graph_dispatch_receipt(&sample_receipt("executed"))
            .await
            .expect("persist dispatch receipt");

        let before = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt before");
        assert_eq!(
            before.and_then(|receipt| receipt.exception_path_receipt_id),
            None
        );

        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: sample_exception_takeover_args(run_id, "receipt-1"),
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after lane command");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(
            after.exception_path_receipt_id.as_deref(),
            Some("receipt-1")
        );
        assert_eq!(after.lane_status, "lane_exception_recorded");
        let metadata_path = exception_takeover_metadata_path(&root, run_id);
        let metadata = read_exception_takeover_metadata(&root, run_id)
            .expect("read persisted exception takeover metadata")
            .expect("exception takeover metadata should exist");
        assert!(metadata_path.exists());
        assert_eq!(metadata.reason_class, "failed_lawful_reuse");
        assert_eq!(
            metadata.active_bounded_unit,
            "feature-spec-compliant-exception-path-takeover-surface-dev"
        );
        assert_eq!(metadata.owned_write_scope.len(), 1);
        assert_eq!(metadata.verification_plan.len(), 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_exception_takeover_stays_recorded_until_explicit_supersession_exists() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-clear-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-clear";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "closure".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "closure_pending".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.handoff_state = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: sample_exception_takeover_args(run_id, "receipt-clear-1"),
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after lane command");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(
            after.exception_path_receipt_id.as_deref(),
            Some("receipt-clear-1")
        );
        assert_eq!(after.lane_status, "lane_exception_recorded");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_exception_takeover_rejects_superseded_lane_mutation() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-superseded-mutation-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-superseded";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "closure".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "closure_pending".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.lane_status = crate::LaneStatus::LaneSuperseded.as_str().to_string();
        receipt.supersedes_receipt_id = Some("supersede-1".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: sample_exception_takeover_args(run_id, "receipt-superseded-1"),
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after rejected mutation");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(after.exception_path_receipt_id, None);
        assert!(
            !exception_takeover_metadata_path(&root, run_id).exists(),
            "superseded mutation must not persist exception takeover metadata"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_supersede_activates_exception_takeover_for_recorded_exception_receipt() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-supersede-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-supersede";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "implementer".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.status = "ready".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception-recorded receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "supersede".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "supersede-1".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after lane command");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert_eq!(after.supersedes_receipt_id.as_deref(), Some("supersede-1"));
        assert_eq!(
            after.exception_path_receipt_id.as_deref(),
            Some("exception-1")
        );
        assert_eq!(after.lane_status, "lane_exception_takeover");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_show_run_blocks_admissible_takeover_until_supersession_receipt_exists() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-show-run-supersession-needed-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-show-supersession-needed";

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "specification",
            "scope_discussion",
        );
        status.active_node = "closure".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "closure_pending".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.context_state = "sealed".to_string();
        status.resume_target = "none".to_string();
        status.handoff_state = "none".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionRecorded
            .as_str()
            .to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist exception-recorded receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec!["show".to_string(), run_id.to_string(), "--json".to_string()],
        };
        assert_eq!(run_lane(args).await, ExitCode::from(2));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_complete_records_receipt_backed_downstream_completion_evidence() {
        let _guard = lane_surface_test_lock()
            .lock()
            .expect("lane surface test lock should acquire");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-complete-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-complete";
        let packet_path =
            root.join("runtime-consumption/downstream-dispatch-packets/run-lane-complete.json");
        std::fs::create_dir_all(
            packet_path
                .parent()
                .expect("downstream packet path should have parent"),
        )
        .expect("create downstream packet dir");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "downstream_dispatch_target": "coach",
                "downstream_dispatch_active_target": "implementer",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_implementation_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_blocked"
            })
            .to_string(),
        )
        .expect("write downstream packet");

        let mut receipt = sample_receipt("executed");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_note =
            Some("after `implementer` evidence is recorded, activate `coach`".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "completion-1".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after lane command");
        let after = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read receipt after")
            .expect("receipt should exist");
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());
        assert_eq!(
            after.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        let result_path = after
            .downstream_dispatch_result_path
            .clone()
            .expect("completion result path should be recorded");
        let result = std::fs::read_to_string(&result_path).expect("read completion result");
        let result_json: serde_json::Value =
            serde_json::from_str(&result).expect("completion result should be json");
        assert_eq!(
            result_json["artifact_kind"],
            "runtime_lane_completion_result"
        );
        assert_eq!(result_json["completion_receipt_id"], "completion-1");

        let packet = std::fs::read_to_string(&packet_path).expect("read updated packet");
        let packet_json: serde_json::Value =
            serde_json::from_str(&packet).expect("updated packet should be json");
        assert_eq!(packet_json["downstream_dispatch_ready"], true);
        assert_eq!(packet_json["downstream_dispatch_status"], "packet_ready");
        assert_eq!(packet_json["downstream_lane_status"], "packet_ready");
        assert_eq!(packet_json["downstream_dispatch_result_path"], result_path);

        let _ = std::fs::remove_dir_all(&root);
    }
}
