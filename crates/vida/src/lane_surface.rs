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
    root_local_write_allowed_for_only_these_paths: Vec<String>,
}

#[derive(Serialize)]
struct LaneReclaimEnvelope {
    surface: &'static str,
    status: &'static str,
    reclaim_mode: &'static str,
    completed: bool,
    host_agents: bool,
    stale_scheduler_reservations_reclaimed: usize,
    host_agent_reclaim_api_available: bool,
    host_agent_reclaim_status: &'static str,
    next_actions: Vec<String>,
    blocker_codes: Vec<String>,
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
    Retire {
        run_id: &'a str,
        receipt_id: &'a str,
        reason: &'a str,
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
    Reclaim {
        completed: bool,
        host_agents: bool,
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
    "Usage: vida lane show <run-id> [--json]\n       vida lane show --latest [--json]\n       vida lane complete <run-id> --receipt-id <id> [--json]\n       vida lane retire <run-id> --receipt-id <id> --reason <text> [--json]\n       vida lane exception-takeover <run-id> --receipt-id <id> --reason-class <class> --active-bounded-unit <unit> --owned-write-scope <path> [--owned-write-scope <path> ...] --why-delegated-path-not-lawful <text> --why-local-write-safe <text> --return-to-normal-when <text> --verification-step <text> [--verification-step <text> ...] [--json]\n       vida lane supersede <run-id> --receipt-id <id> [--json]\n       vida lane reclaim --completed --host-agents [--json]"
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
        [head, run_id, rest @ ..] if head == "retire" => {
            let mut as_json = false;
            let mut receipt_id = None;
            let mut reason = None;
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
                    "--reason" => {
                        let Some(value) = rest.get(index + 1) else {
                            return Err(lane_usage().to_string());
                        };
                        reason = Some(value.as_str());
                        index += 2;
                    }
                    _ => return Err(lane_usage().to_string()),
                }
            }
            let Some(receipt_id) = receipt_id.filter(|value| !value.trim().is_empty()) else {
                return Err(lane_usage().to_string());
            };
            let Some(reason) = reason.filter(|value| !value.trim().is_empty()) else {
                return Err(lane_usage().to_string());
            };
            Ok(LaneCommand::Retire {
                run_id,
                receipt_id,
                reason,
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
        [head, rest @ ..] if head == "reclaim" => {
            let mut as_json = false;
            let mut completed = false;
            let mut host_agents = false;
            for arg in rest {
                match arg.as_str() {
                    "--json" => as_json = true,
                    "--completed" => completed = true,
                    "--host-agents" => host_agents = true,
                    _ => return Err(lane_usage().to_string()),
                }
            }
            if !completed || !host_agents {
                return Err(lane_usage().to_string());
            }
            Ok(LaneCommand::Reclaim {
                completed,
                host_agents,
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
    let root_local_write_allowed_for_only_these_paths =
        active_exception_write_scope(&summary, exception_path_metadata.as_ref());
    let selected_backend = status
        .as_ref()
        .map(|status| status.selected_backend.clone())
        .or(summary.selected_backend.clone());
    let artifact_refs = serde_json::json!({
        "latest_run_graph_dispatch_receipt_id": run_id.clone(),
        "exception_path_receipt_id": exception_path_receipt_id.clone(),
        "exception_path_metadata_path": exception_path_metadata_path.clone(),
        "root_local_write_allowed_for_only_these_paths": exception_path_metadata
            .as_ref()
            .map(|_| root_local_write_allowed_for_only_these_paths.clone())
            .unwrap_or_default(),
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
        root_local_write_allowed_for_only_these_paths,
        exception_path_metadata,
    }
}

fn active_exception_write_scope(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    exception_path_metadata: Option<&ExceptionTakeoverMetadata>,
) -> Vec<String> {
    if summary.lane_status != crate::LaneStatus::LaneExceptionTakeover.as_str()
        || !crate::release1_contracts::has_evidence_id(summary.exception_path_receipt_id.as_deref())
        || !crate::release1_contracts::has_evidence_id(summary.supersedes_receipt_id.as_deref())
    {
        return Vec::new();
    }
    exception_path_metadata
        .map(|metadata| metadata.owned_write_scope.clone())
        .unwrap_or_default()
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

fn lane_summary_dispatch_is_blocked(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> bool {
    let dispatch_status = summary.dispatch_status.trim().to_ascii_lowercase();
    let lane_status = summary.lane_status.trim().to_ascii_lowercase();
    let has_downstream_blockers = summary
        .downstream_dispatch_blockers
        .iter()
        .any(|value| !value.trim().is_empty());
    matches!(dispatch_status.as_str(), "blocked" | "failed")
        || matches!(lane_status.as_str(), "lane_blocked" | "lane_failed")
        || has_downstream_blockers
}

fn recovery_delegated_cycle_open(
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> bool {
    recovery.is_some_and(|recovery| {
        recovery.delegation_gate.local_exception_takeover_gate == "blocked_open_delegated_cycle"
            || recovery.delegation_gate.delegated_cycle_open
    })
}

fn lane_summary_raw_blocker_codes(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    include_downstream: bool,
) -> Vec<String> {
    let mut blocker_codes = Vec::new();
    if let Some(blocker_code) = summary
        .blocker_code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        blocker_codes.push(blocker_code.to_string());
    }
    if include_downstream {
        blocker_codes.extend(
            summary
                .downstream_dispatch_blockers
                .iter()
                .filter(|value| !value.trim().is_empty())
                .cloned(),
        );
    }
    if blocker_codes.is_empty() && lane_summary_dispatch_is_blocked(summary) {
        blocker_codes.push(
            crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::ToolExecutionFailed,
            )
            .to_string(),
        );
    }
    blocker_codes
}

fn canonical_lane_show_blocker_codes(blocker_codes: &[String]) -> Vec<String> {
    let mut canonical_codes = crate::release1_contracts::canonical_blocker_code_list(blocker_codes);
    let has_uncanonical_dispatch_blocker = blocker_codes.iter().any(|value| {
        !value.trim().is_empty()
            && crate::release1_contracts::canonical_blocker_code_list([value.as_str()]).is_empty()
    });
    if has_uncanonical_dispatch_blocker
        && !canonical_codes
            .iter()
            .any(|code| code == "tool_execution_failed")
    {
        canonical_codes.push(
            crate::release1_contracts::blocker_code_str(
                crate::release1_contracts::BlockerCode::ToolExecutionFailed,
            )
            .to_string(),
        );
        canonical_codes.sort();
        canonical_codes.dedup();
    }
    canonical_codes
}

fn blocked_lane_show_next_action(
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    recovery: Option<&crate::state_store::RunGraphRecoverySummary>,
) -> String {
    let run_id = crate::shell_quote(summary.run_id.trim());
    let dispatch_target = summary.dispatch_target.trim();
    let lane = if dispatch_target.is_empty() {
        "the blocked delegated lane".to_string()
    } else {
        format!("the blocked `{dispatch_target}` lane")
    };
    let mut action = format!(
        "Inspect {lane} for run `{}` with `vida taskflow recovery status {} --json` and keep the blocked dispatch result from `vida lane show {} --json` as evidence.",
        summary.run_id, run_id, run_id
    );
    if recovery_delegated_cycle_open(recovery) {
        action.push_str(&format!(
            " If no receipt-backed delegated completion exists, record structured exception takeover for run `{}` with a concrete receipt id, active bounded unit, and owned write scope, then supersede the lane with the same receipt id before local recovery work.",
            summary.run_id
        ));
    } else {
        action.push_str(&format!(
            " If the dispatch blocker has already been resolved, rerun `vida taskflow consume continue --run-id {} --json` to refresh continuation evidence.",
            run_id
        ));
    }
    action
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
        let recovery_open = recovery_delegated_cycle_open(recovery);
        if recovery_open {
            let mut blocker_codes = lane_summary_raw_blocker_codes(summary, true);
            blocker_codes.push("open_delegated_cycle".to_string());
            return LaneShowTruth {
                blocked: true,
                blocker_codes: canonical_lane_show_blocker_codes(&blocker_codes),
                next_actions: vec![format!(
                    "Active exception takeover for lane `{}` still has unresolved dispatch evidence; finish the bounded exception unit before retrying continuation.",
                    summary.run_id
                )],
            };
        }
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

    let completed_has_blocked_downstream = summary.lane_status
        == crate::LaneStatus::LaneCompleted.as_str()
        && summary
            .downstream_dispatch_blockers
            .iter()
            .any(|value| !value.trim().is_empty());
    if summary.lane_status == crate::LaneStatus::LaneCompleted.as_str()
        && !completed_has_blocked_downstream
    {
        return LaneShowTruth {
            blocked: false,
            blocker_codes: Vec::new(),
            next_actions: Vec::new(),
        };
    }

    let recovery_open = recovery_delegated_cycle_open(recovery);
    let mut blocked = lane_summary_dispatch_is_blocked(summary) || recovery_open;
    let mut blocker_codes = lane_summary_raw_blocker_codes(summary, blocked);
    let mut next_actions = Vec::new();
    if recovery_open {
        blocker_codes.push("open_delegated_cycle".to_string());
    }

    if summary.lane_status == crate::LaneStatus::LaneExceptionRecorded.as_str() {
        blocked = true;
        if recovery_open {
            next_actions.push(
                "Exception-path receipt recorded; delegated cycle is still open, so root-local write remains blocked."
                    .to_string(),
            );
        } else {
            blocker_codes.push("supersession_without_receipt".to_string());
            let receipt_id = summary
                .exception_path_receipt_id
                .as_deref()
                .unwrap_or_default();
            let run_id = crate::shell_quote(summary.run_id.trim());
            next_actions.push(if receipt_id.trim().is_empty() {
                format!(
                    "Exception-path receipt recorded for lane `{}` but no concrete receipt id is available; inspect `vida lane show {} --json` and recover the recorded receipt before supersession.",
                    summary.run_id, run_id
                )
            } else {
                let receipt_id = crate::shell_quote(receipt_id.trim());
                format!(
                    "Exception-path receipt recorded; record explicit supersession with `vida lane supersede {} --receipt-id {} --json` before local write becomes active.",
                    run_id, receipt_id
                )
            });
        }
    }
    if blocked && next_actions.is_empty() {
        next_actions.push(blocked_lane_show_next_action(summary, recovery));
    }

    LaneShowTruth {
        blocked,
        blocker_codes: canonical_lane_show_blocker_codes(&blocker_codes),
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
    if !envelope
        .root_local_write_allowed_for_only_these_paths
        .is_empty()
    {
        crate::print_surface_line(
            crate::RenderMode::Plain,
            "root_local_write_allowed_for_only_these_paths",
            &envelope
                .root_local_write_allowed_for_only_these_paths
                .join(", "),
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

fn exception_takeover_metadata_filename(run_id: &str) -> Result<String, String> {
    if run_id.is_empty() {
        return Err("Run id cannot be empty for exception takeover metadata.".to_string());
    }
    if !run_id
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
    {
        return Err(format!(
            "Run id `{run_id}` contains unsupported characters for exception takeover metadata filename."
        ));
    }
    Ok(format!("{run_id}.json"))
}

fn exception_takeover_metadata_path(state_root: &Path, run_id: &str) -> Result<PathBuf, String> {
    let file_name = exception_takeover_metadata_filename(run_id)?;
    Ok(exception_takeover_metadata_dir(state_root).join(file_name))
}

fn read_exception_takeover_metadata(
    state_root: &Path,
    run_id: &str,
) -> Result<Option<ExceptionTakeoverMetadata>, String> {
    let path = exception_takeover_metadata_path(state_root, run_id)?;
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
    let path = exception_takeover_metadata_path(state_root, run_id)?;
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
        let next_action =
            crate::status_surface_signals::terminal_next_action_requires_authoritative_run_state(
                Some(run_id),
            );
        return Err(format!(
            "Lane `{run_id}` is no longer active for mutation because run-graph status is terminal (`{}` / `{}`). Inspect `vida lane show {run_id} --json` for the persisted lane envelope and continuation evidence. {next_action}",
            status.status, status.lifecycle_stage,
        ));
    }
    Ok(())
}

fn retired_closed_task_run_graph_status(
    mut status: crate::state_store::RunGraphStatus,
) -> crate::state_store::RunGraphStatus {
    status.active_node = "closure".to_string();
    status.next_node = None;
    status.status = "completed".to_string();
    status.lifecycle_stage = "closure_complete".to_string();
    status.policy_gate = "closed_task_stale_run_retired".to_string();
    status.handoff_state = "none".to_string();
    status.context_state = "sealed".to_string();
    status.checkpoint_kind = "none".to_string();
    status.resume_target = "none".to_string();
    status.recovery_ready = false;
    status
}

fn read_lane_packet(path: &str) -> Result<serde_json::Value, String> {
    let normalized_path = crate::runtime_dispatch_state::normalize_persisted_runtime_path(path);
    let raw = std::fs::read_to_string(&normalized_path)
        .map_err(|error| format!("Failed to read persisted lane packet `{path}`: {error}"))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to decode persisted lane packet `{path}`: {error}"))
}

fn canonicalize_for_lane_packet_validation(
    path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    }) {
        return Err(format!(
            "Failed to canonicalize lane packet path `{}`: dot-segment traversal is not admissible",
            path.display()
        ));
    }
    if path.exists() {
        return std::fs::canonicalize(path).map_err(|error| {
            format!(
                "Failed to canonicalize lane packet path `{}`: {error}",
                path.display()
            )
        });
    }
    Err(format!(
        "Failed to canonicalize lane packet path `{}`: packet file does not exist",
        path.display()
    ))
}

fn validate_lane_packet_path(
    state_root: &std::path::Path,
    run_id: &str,
    packet_path: &str,
    takeover_active: bool,
) -> Result<std::path::PathBuf, String> {
    let normalized = crate::runtime_dispatch_state::normalize_persisted_runtime_path(packet_path);
    let canonical_packet_path = canonicalize_for_lane_packet_validation(&normalized)?;
    let parent_name = canonical_packet_path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let grandparent_name = canonical_packet_path
        .parent()
        .and_then(|parent| parent.parent())
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let under_state_root =
        canonical_packet_path.starts_with(std::fs::canonicalize(state_root).map_err(|error| {
            format!(
                "Failed to canonicalize VIDA state root `{}`: {error}",
                state_root.display()
            )
        })?);
    if under_state_root
        && grandparent_name == "runtime-consumption"
        && parent_name == "downstream-dispatch-packets"
    {
        return Ok(canonical_packet_path);
    }
    if takeover_active {
        if under_state_root
            && grandparent_name == "runtime-consumption"
            && parent_name == "dispatch-packets"
        {
            return Ok(canonical_packet_path);
        }
    }
    Err(format!(
        "Lane `{run_id}` packet path `{}` is outside VIDA runtime packet directories.",
        canonical_packet_path.display()
    ))
}

fn lane_completion_packet_path(
    receipt: &crate::state_store::RunGraphDispatchReceipt,
) -> Option<(String, bool)> {
    if let Some(packet_path) = receipt.downstream_dispatch_packet_path.clone() {
        return Some((packet_path, false));
    }
    receipt
        .dispatch_packet_path
        .clone()
        .map(|packet_path| (packet_path, true))
}

fn write_lane_packet(path: &str, packet: &serde_json::Value) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(packet)
        .map_err(|error| format!("Failed to encode persisted lane packet `{path}`: {error}"))?;
    std::fs::write(path, encoded)
        .map_err(|error| format!("Failed to write persisted lane packet `{path}`: {error}"))
}

fn decode_lane_completion_packet_context(
    packet: &serde_json::Value,
) -> Result<Option<(crate::RuntimeConsumptionLaneSelection, serde_json::Value)>, String> {
    let Some(role_selection_value) = packet
        .get("role_selection_full")
        .filter(|value| !value.is_null())
        .cloned()
    else {
        return Ok(None);
    };
    let Some(run_graph_bootstrap) = packet
        .get("run_graph_bootstrap")
        .filter(|value| !value.is_null())
        .cloned()
    else {
        return Ok(None);
    };
    let role_selection =
        serde_json::from_value::<crate::RuntimeConsumptionLaneSelection>(role_selection_value)
            .map_err(|error| {
                format!("Failed to decode role_selection_full from persisted lane packet: {error}")
            })?;
    Ok(Some((role_selection, run_graph_bootstrap)))
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
                match exception_takeover_metadata_path(store.root(), &summary.run_id) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
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
                match exception_takeover_metadata_path(store.root(), run_id) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
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
            let mut recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let mut status = store.run_graph_status(run_id).await.ok();
            if let Err(error) =
                lane_mutation_status_guard(run_id, status.as_ref(), recovery.as_ref(), &receipt)
            {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
            let exception_path_metadata =
                match read_exception_takeover_metadata(store.root(), run_id) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let takeover_active = lane_takeover_state(&receipt, recovery.as_ref()).is_active();
            let Some((packet_path, allow_dispatch_packet)) = lane_completion_packet_path(&receipt)
            else {
                eprintln!(
                    "Lane `{run_id}` has no persisted dispatch packet evidence for bounded completion."
                );
                return ExitCode::from(2);
            };
            let validated_packet_path = match validate_lane_packet_path(
                store.root(),
                run_id,
                &packet_path,
                allow_dispatch_packet || takeover_active,
            ) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(2);
                }
            };
            let validated_packet_path = validated_packet_path.display().to_string();
            let mut packet = match read_lane_packet(&validated_packet_path) {
                Ok(packet) => packet,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            };
            if packet.get("run_id").and_then(serde_json::Value::as_str) != Some(run_id) {
                eprintln!(
                    "Lane `{run_id}` packet `{validated_packet_path}` does not belong to the requested run."
                );
                return ExitCode::from(2);
            }
            if let Err(error) =
                crate::validate_runtime_dispatch_packet_contract(&packet, "Lane completion packet")
            {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
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
                    &validated_packet_path,
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            receipt.downstream_dispatch_ready = true;
            receipt.downstream_dispatch_blockers.clear();
            receipt.downstream_dispatch_status = Some("packet_ready".to_string());
            receipt.downstream_dispatch_result_path = Some(completion_result_path.clone());
            receipt.downstream_dispatch_active_target = Some(completed_target.clone());
            receipt.downstream_dispatch_last_target = Some(completed_target);
            receipt.dispatch_status = "executed".to_string();
            receipt.blocker_code = None;
            receipt.exception_path_receipt_id = None;
            receipt.supersedes_receipt_id = None;
            receipt.dispatch_result_path = Some(completion_result_path);
            receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
            match decode_lane_completion_packet_context(&packet) {
                Ok(Some((role_selection, run_graph_bootstrap))) => {
                    if let Err(error) =
                        crate::runtime_dispatch_state::refresh_downstream_dispatch_preview(
                            &store,
                            &role_selection,
                            &run_graph_bootstrap,
                            &mut receipt,
                        )
                        .await
                    {
                        eprintln!(
                            "Failed to refresh downstream dispatch preview after lane completion: {error}"
                        );
                        return ExitCode::from(1);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
            }
            let downstream_dispatch_status = receipt
                .downstream_dispatch_status
                .clone()
                .unwrap_or_else(|| "packet_ready".to_string());
            packet["downstream_dispatch_ready"] =
                serde_json::json!(receipt.downstream_dispatch_ready);
            packet["downstream_dispatch_blockers"] =
                serde_json::json!(receipt.downstream_dispatch_blockers.clone());
            packet["downstream_dispatch_status"] =
                serde_json::json!(downstream_dispatch_status.clone());
            packet["downstream_dispatch_result_path"] =
                serde_json::json!(receipt.downstream_dispatch_result_path.clone());
            packet["downstream_lane_status"] = serde_json::json!(downstream_dispatch_status);
            packet["downstream_dispatch_active_target"] =
                serde_json::json!(receipt.downstream_dispatch_active_target.clone());
            if let Err(error) = write_lane_packet(&validated_packet_path, &packet) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            if receipt.dispatch_status == "executed" {
                if let Some(current_status) = status.as_ref() {
                    let executed_status = crate::runtime_dispatch_state::apply_first_handoff_execution_to_run_graph_status(
                        current_status,
                        &receipt,
                    );
                    if let Err(error) = store.record_run_graph_status(&executed_status).await {
                        eprintln!(
                            "Failed to persist run-graph status after lane completion: {error}"
                        );
                        return ExitCode::from(1);
                    }
                    if let Err(error) =
                        crate::taskflow_continuation::sync_run_graph_continuation_binding(
                            &store,
                            &executed_status,
                            "lane_complete",
                        )
                        .await
                    {
                        eprintln!(
                            "Failed to synchronize continuation binding after lane completion: {error}"
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist lane completion evidence: {error}");
                return ExitCode::from(1);
            }
            status = store.run_graph_status(run_id).await.ok();
            recovery = store.run_graph_recovery_summary(run_id).await.ok();

            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let truth = derive_lane_show_truth(&updated_summary, recovery.as_ref());
            let exception_path_metadata_path =
                match exception_takeover_metadata_path(store.root(), run_id) {
                    Ok(path) => path,
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
        LaneCommand::Retire {
            run_id,
            receipt_id,
            reason: _reason,
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
            let Some(status) = (match store.run_graph_status(run_id).await {
                Ok(status) => Some(status),
                Err(error) => {
                    eprintln!("Failed to read run-graph status `{run_id}`: {error}");
                    return ExitCode::from(1);
                }
            }) else {
                eprintln!("Missing run-graph status for `{run_id}`.");
                return ExitCode::from(2);
            };
            match store.show_task(&status.task_id).await {
                Ok(task) if task.status == "closed" => {}
                Ok(task) => {
                    eprintln!(
                        "Lane `{run_id}` can only be retired after task `{}` is closed; current task status is `{}`.",
                        status.task_id, task.status
                    );
                    return ExitCode::from(2);
                }
                Err(error) => {
                    eprintln!(
                        "Failed to verify closed task `{}` before retiring lane `{run_id}`: {error}",
                        status.task_id
                    );
                    return ExitCode::from(1);
                }
            }
            let Some(packet_path) = receipt
                .downstream_dispatch_packet_path
                .clone()
                .or_else(|| receipt.dispatch_packet_path.clone())
            else {
                eprintln!("Lane `{run_id}` has no packet evidence for stale-run retirement.");
                return ExitCode::from(2);
            };
            let validated_packet_path =
                match validate_lane_packet_path(store.root(), run_id, &packet_path, true) {
                    Ok(path) => path.display().to_string(),
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
            let completion_result_path =
                match crate::runtime_dispatch_state::write_runtime_lane_completion_result(
                    store.root(),
                    run_id,
                    "closure",
                    receipt_id,
                    &validated_packet_path,
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            let retired_status = retired_closed_task_run_graph_status(status);
            if let Err(error) = store.record_run_graph_status(&retired_status).await {
                eprintln!("Failed to persist retired run-graph status `{run_id}`: {error}");
                return ExitCode::from(1);
            }
            if let Err(error) = crate::taskflow_continuation::sync_run_graph_continuation_binding(
                &store,
                &retired_status,
                "lane_retire_closed_task_stale_run",
            )
            .await
            {
                eprintln!("Failed to clear retired run continuation binding `{run_id}`: {error}");
                return ExitCode::from(1);
            }
            receipt.dispatch_status = "executed".to_string();
            receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
            receipt.blocker_code = None;
            receipt.exception_path_receipt_id = None;
            receipt.supersedes_receipt_id = None;
            receipt.downstream_dispatch_target = None;
            receipt.downstream_dispatch_command = None;
            receipt.downstream_dispatch_packet_path = None;
            receipt.downstream_dispatch_ready = false;
            receipt.downstream_dispatch_blockers.clear();
            receipt.downstream_dispatch_status = Some("retired_closed_task_run".to_string());
            receipt.downstream_dispatch_result_path = Some(completion_result_path.clone());
            receipt.downstream_dispatch_active_target = Some("closure".to_string());
            receipt.downstream_dispatch_last_target = Some("closure".to_string());
            receipt.dispatch_result_path = Some(completion_result_path);
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist retired lane receipt `{run_id}`: {error}");
                return ExitCode::from(1);
            }

            let updated_summary =
                crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
            let recovery = store.run_graph_recovery_summary(run_id).await.ok();
            let truth = derive_lane_show_truth(&updated_summary, recovery.as_ref());
            let exception_path_metadata_path =
                match exception_takeover_metadata_path(store.root(), run_id) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
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
                Some(retired_status),
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
            let metadata_path =
                match write_exception_takeover_metadata(store.root(), run_id, &metadata) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
            receipt.exception_path_receipt_id = Some(receipt_id.to_string());
            receipt.lane_status = explicit_lane_status_for_receipt(&receipt, recovery.as_ref());
            if let Err(error) = store.record_run_graph_dispatch_receipt(&receipt).await {
                eprintln!("Failed to persist exception takeover receipt: {error}");
                return ExitCode::from(1);
            }
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
                match exception_takeover_metadata_path(store.root(), run_id) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
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
        LaneCommand::Reclaim {
            completed,
            host_agents,
            as_json,
        } => {
            let reclaimed = match store.expire_stale_scheduler_dispatch_reservations().await {
                Ok(count) => count,
                Err(error) => {
                    eprintln!("Failed to reclaim stale scheduler reservations: {error}");
                    return ExitCode::from(1);
                }
            };
            let envelope = LaneReclaimEnvelope {
                surface: "vida lane reclaim",
                status: "pass",
                reclaim_mode: "completed_host_agents",
                completed,
                host_agents,
                stale_scheduler_reservations_reclaimed: reclaimed,
                host_agent_reclaim_api_available: false,
                host_agent_reclaim_status: "runtime_reclaimed_state_only",
                next_actions: vec![
                    "Runtime state was reclaimed where VIDA owns the reservation state; close any completed Codex App UI agent handles through the host app when the app still displays them."
                        .to_string(),
                ],
                blocker_codes: Vec::new(),
            };
            if crate::surface_render::print_surface_json(
                &envelope,
                as_json,
                "lane reclaim surface should serialize",
            ) {
                return ExitCode::SUCCESS;
            }
            crate::print_surface_header(crate::RenderMode::Plain, envelope.surface);
            crate::print_surface_line(crate::RenderMode::Plain, "status", envelope.status);
            crate::print_surface_line(
                crate::RenderMode::Plain,
                "stale_scheduler_reservations_reclaimed",
                &envelope.stale_scheduler_reservations_reclaimed.to_string(),
            );
            crate::print_surface_line(
                crate::RenderMode::Plain,
                "host_agent_reclaim_status",
                envelope.host_agent_reclaim_status,
            );
            ExitCode::SUCCESS
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

    fn acquire_lane_surface_test_lock() -> std::sync::MutexGuard<'static, ()> {
        lane_surface_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lane_complete_role_selection(dev_task_id: &str) -> crate::RuntimeConsumptionLaneSelection {
        crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "pm".to_string(),
            conversational_mode: Some("development".to_string()),
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "tracked_flow_bootstrap": {
                    "dev_task": {
                        "task_id": dev_task_id,
                        "ensure_command": "vida task ensure feature-x-dev \"Dev pack\" --type task --status open --json"
                    }
                },
                "development_flow": {
                    "dispatch_contract": {
                        "execution_lane_sequence": ["implementer", "coach", "verification"],
                        "implementer_activation": {
                            "completion_blocker": "pending_implementation_evidence",
                            "activation_agent_type": "junior",
                            "activation_runtime_role": "worker"
                        },
                        "coach_activation": {
                            "completion_blocker": "pending_review_clean_evidence",
                            "activation_agent_type": "middle",
                            "activation_runtime_role": "coach"
                        },
                        "verifier_activation": {
                            "completion_blocker": "pending_verification_evidence",
                            "activation_agent_type": "senior",
                            "activation_runtime_role": "verifier"
                        }
                    }
                },
                "orchestration_contract": {}
            }),
            reason: "test".to_string(),
        }
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
    fn parse_lane_retire_supports_receipt_id_reason_and_json() {
        let args = vec![
            "retire".to_string(),
            "run-1".to_string(),
            "--receipt-id".to_string(),
            "receipt-1".to_string(),
            "--reason".to_string(),
            "closed stale run".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane retire should parse");
        assert!(matches!(
            command,
            LaneCommand::Retire {
                run_id: "run-1",
                receipt_id: "receipt-1",
                reason: "closed stale run",
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
    fn parse_lane_reclaim_supports_completed_host_agents_json() {
        let args = vec![
            "reclaim".to_string(),
            "--completed".to_string(),
            "--host-agents".to_string(),
            "--json".to_string(),
        ];
        let command = parse_lane_args(&args).expect("lane reclaim should parse");
        assert!(matches!(
            command,
            LaneCommand::Reclaim {
                completed: true,
                host_agents: true,
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
        assert!(truth
            .next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery status run-lane-test --json")));
        assert!(truth
            .next_actions
            .iter()
            .any(|action| action.contains("vida lane show run-lane-test --json")));
    }

    #[test]
    fn derive_lane_show_truth_marks_downstream_blockers_as_blocked() {
        let mut receipt = sample_receipt("executed");
        receipt.blocker_code = None;
        receipt
            .downstream_dispatch_blockers
            .push("missing_owned_write_scope".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);

        let truth = derive_lane_show_truth(&summary, None);

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"tool_execution_failed".to_string()));
    }

    #[test]
    fn derive_lane_show_truth_marks_completed_downstream_blockers_as_blocked() {
        let mut receipt = sample_receipt("executed");
        receipt.blocker_code = None;
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        receipt
            .downstream_dispatch_blockers
            .push("missing_owned_write_scope".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);

        let truth = derive_lane_show_truth(&summary, None);

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"tool_execution_failed".to_string()));
        assert!(truth
            .next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery status run-lane-test --json")));
    }

    #[test]
    fn lane_completed_recovery_pass_does_not_surface_open_delegated_cycle() {
        let mut receipt = sample_receipt("executed");
        receipt.blocker_code = None;
        receipt.lane_status = crate::LaneStatus::LaneCompleted.as_str().to_string();
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-test".to_string(),
            task_id: "task-lane-test".to_string(),
            active_node: "coach".to_string(),
            lifecycle_stage: "coach_active".to_string(),
            resume_node: None,
            resume_status: "none".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: false,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "coach".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "delegated_lane_active".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: None,
                lifecycle_stage: "coach_active".to_string(),
            },
        };

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(!truth.blocked);
        assert!(!truth
            .blocker_codes
            .contains(&"open_delegated_cycle".to_string()));
        assert!(truth.next_actions.is_empty());
    }

    #[test]
    fn derive_lane_show_truth_blocks_running_open_delegated_cycle() {
        let mut receipt = sample_receipt("executing");
        receipt.lane_status = crate::LaneStatus::LaneRunning.as_str().to_string();
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let recovery = crate::state_store::RunGraphRecoverySummary {
            run_id: "run-lane-test".to_string(),
            task_id: "task-lane-test".to_string(),
            active_node: "analysis".to_string(),
            lifecycle_stage: "analysis_active".to_string(),
            resume_node: None,
            resume_status: "running".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "none".to_string(),
            policy_gate: "targeted_verification".to_string(),
            handoff_state: "none".to_string(),
            recovery_ready: true,
            delegation_gate: crate::state_store::RunGraphDelegationGateSummary {
                active_node: "analysis".to_string(),
                delegated_cycle_open: true,
                delegated_cycle_state: "open".to_string(),
                local_exception_takeover_gate: "blocked_open_delegated_cycle".to_string(),
                reporting_pause_gate: "delegated_cycle_open".to_string(),
                continuation_signal: "continue_delegated_cycle".to_string(),
                blocker_code: None,
                lifecycle_stage: "analysis_active".to_string(),
            },
        };

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"open_delegated_cycle".to_string()));
        assert!(truth
            .next_actions
            .iter()
            .any(|action| action.contains("vida taskflow recovery status run-lane-test --json")));
    }

    #[test]
    fn build_lane_envelope_exposes_root_scope_only_for_active_takeover() {
        let metadata = ExceptionTakeoverMetadata {
            reason_class: "test".to_string(),
            active_bounded_unit: "taskflow-timeout-actionability".to_string(),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful: "blocked".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround: "bounded".to_string(),
            return_to_normal_posture_condition: "verified".to_string(),
            verification_plan: vec!["test".to_string()],
            recorded_at: "2026-05-13T00:00:00Z".to_string(),
        };
        let mut stale_receipt = sample_receipt("executed");
        stale_receipt.blocker_code = None;
        stale_receipt.lane_status = crate::LaneStatus::LaneRunning.as_str().to_string();
        let stale_envelope = build_lane_envelope(
            crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(stale_receipt),
            None,
            Some("/tmp/exception.json".to_string()),
            Some(metadata.clone()),
            false,
            Vec::new(),
            Vec::new(),
        );
        assert!(stale_envelope
            .root_local_write_allowed_for_only_these_paths
            .is_empty());
        assert_eq!(
            stale_envelope.artifact_refs["root_local_write_allowed_for_only_these_paths"]
                .as_array()
                .map(Vec::len),
            Some(0)
        );

        let mut active_receipt = sample_receipt("executed");
        active_receipt.blocker_code = None;
        active_receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        active_receipt.exception_path_receipt_id = Some("exception-1".to_string());
        active_receipt.supersedes_receipt_id = Some("exception-1".to_string());
        let active_envelope = build_lane_envelope(
            crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(active_receipt),
            None,
            Some("/tmp/exception.json".to_string()),
            Some(metadata),
            false,
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            active_envelope.root_local_write_allowed_for_only_these_paths,
            vec!["crates/vida/src/lane_surface.rs".to_string()]
        );
    }

    #[test]
    fn derive_lane_show_truth_blocks_active_exception_takeover_with_dispatch_evidence() {
        let mut receipt = sample_receipt("blocked");
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("exception-0".to_string());
        receipt.blocker_code = Some("internal_dispatch_timeout_without_receipt".to_string());
        receipt
            .downstream_dispatch_blockers
            .push("internal_dispatch_timeout_without_receipt".to_string());
        let summary = crate::state_store::RunGraphDispatchReceiptSummary::from_receipt(receipt);
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "implementation",
            "coach",
        );
        status.status = "blocked".to_string();
        status.lifecycle_stage = "implementation_blocked".to_string();
        status.active_node = "coach".to_string();
        let mut recovery = crate::state_store::RunGraphRecoverySummary::from_status(status);
        recovery.delegation_gate.local_exception_takeover_gate =
            "blocked_open_delegated_cycle".to_string();
        recovery.delegation_gate.delegated_cycle_open = true;

        let truth = derive_lane_show_truth(&summary, Some(&recovery));

        assert!(truth.blocked);
        assert!(truth
            .blocker_codes
            .contains(&"open_delegated_cycle".to_string()));
        assert!(truth
            .blocker_codes
            .contains(&"tool_execution_failed".to_string()));
        assert!(truth
            .next_actions
            .iter()
            .any(|value| value.contains("finish the bounded exception unit")));
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
        assert!(truth.next_actions.iter().any(|value| value
            .contains("vida lane supersede run-lane-test --receipt-id exception-1 --json")));
    }

    #[test]
    fn lane_mutation_status_guard_reports_actionable_guidance_for_terminal_closed_lane() {
        let mut receipt = sample_receipt("executed");
        receipt.lane_status = crate::LaneStatus::LaneRunning.as_str().to_string();
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            "run-lane-test",
            "implementation",
            "closure",
        );
        status.status = "completed".to_string();
        status.lifecycle_stage = "closure_complete".to_string();
        status.next_node = None;
        let recovery = crate::state_store::RunGraphRecoverySummary::from_status(status.clone());

        let error =
            lane_mutation_status_guard("run-lane-test", Some(&status), Some(&recovery), &receipt)
                .expect_err("terminal lane should fail closed");

        assert!(error.contains("vida lane show run-lane-test --json"));
        assert!(error.contains("vida taskflow run-graph status run-lane-test --json"));
        assert!(error.contains("vida taskflow continuation bind"));
    }

    #[tokio::test]
    async fn lane_show_run_fails_closed_for_exception_recorded_open_cycle() {
        let _guard = acquire_lane_surface_test_lock();
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
        let _guard = acquire_lane_surface_test_lock();
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
        let metadata_path =
            exception_takeover_metadata_path(&root, run_id).expect("exception path metadata path");
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
    async fn lane_retire_marks_closed_task_stale_run_terminal() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-retire-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-retire";
        let task_id = "task-lane-retire";

        store
            .create_task(crate::state_store::CreateTaskRequest {
                task_id,
                title: "Retire closed task run",
                display_id: None,
                description: "",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: crate::state_store::TaskExecutionSemantics::default(),
                planner_metadata: crate::state_store::TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "",
            })
            .await
            .expect("create closed task");

        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            task_id,
            "implementation",
            "implementation",
        );
        status.run_id = run_id.to_string();
        status.active_node = "analysis".to_string();
        status.status = "blocked".to_string();
        status.lifecycle_stage = "analysis_blocked".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "awaiting_analysis".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist blocked run graph status");

        let packet_dir = root.join("runtime-consumption").join("dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("run-lane-retire.json");
        std::fs::write(&packet_path, "{\"run_id\":\"run-lane-retire\"}")
            .expect("write dispatch packet");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.lane_status = crate::LaneStatus::LaneRunning.as_str().to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked receipt");
        store
            .record_run_graph_continuation_binding(
                &crate::state_store::RunGraphContinuationBinding {
                    run_id: run_id.to_string(),
                    task_id: task_id.to_string(),
                    status: "bound".to_string(),
                    active_bounded_unit: serde_json::json!({
                        "kind": "run_graph_task",
                        "task_id": task_id,
                        "run_id": run_id,
                        "active_node": "analysis"
                    }),
                    binding_source: "test".to_string(),
                    why_this_unit: "test binding".to_string(),
                    primary_path: "normal_delivery_path".to_string(),
                    sequential_vs_parallel_posture: "sequential_only_open_cycle".to_string(),
                    request_text: None,
                    recorded_at: "2026-05-13T00:00:00Z".to_string(),
                },
            )
            .await
            .expect("persist continuation binding");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "retire".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "retire-1".to_string(),
                "--reason".to_string(),
                "closed stale run".to_string(),
                "--json".to_string(),
            ],
        };
        assert_eq!(run_lane(args).await, ExitCode::SUCCESS);

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store after retire");
        let retired = store
            .run_graph_status(run_id)
            .await
            .expect("read retired status");
        assert_eq!(retired.status, "completed");
        assert_eq!(retired.lifecycle_stage, "closure_complete");
        assert_eq!(retired.resume_target, "none");
        assert!(!retired.recovery_ready);
        let receipt = store
            .run_graph_dispatch_receipt(run_id)
            .await
            .expect("read retired receipt")
            .expect("receipt should exist");
        assert_eq!(
            receipt.lane_status,
            crate::LaneStatus::LaneCompleted.as_str()
        );
        assert_eq!(
            receipt.downstream_dispatch_status.as_deref(),
            Some("retired_closed_task_run")
        );
        assert!(!receipt.downstream_dispatch_ready);
        assert!(receipt.downstream_dispatch_target.is_none());
        assert!(receipt.downstream_dispatch_command.is_none());
        assert!(receipt.downstream_dispatch_packet_path.is_none());
        assert!(store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read continuation binding")
            .is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_exception_takeover_stays_recorded_until_explicit_supersession_exists() {
        let _guard = acquire_lane_surface_test_lock();
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
        let _guard = acquire_lane_surface_test_lock();
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
            !exception_takeover_metadata_path(&root, run_id)
                .expect("exception path metadata path")
                .exists(),
            "superseded mutation must not persist exception takeover metadata"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn lane_supersede_activates_exception_takeover_for_recorded_exception_receipt() {
        let _guard = acquire_lane_surface_test_lock();
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
        let _guard = acquire_lane_surface_test_lock();
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
        let _guard = acquire_lane_surface_test_lock();
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
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");
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
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lane_surface.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "delivery_task_packet": {
                    "goal": "Complete implementer lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["lane completion is receipt-backed"],
                    "verification_command": "cargo test -p vida lane_complete",
                    "proof_target": "lane completion receipt",
                    "stop_rules": ["stop if packet contract is invalid"],
                    "blocking_question": "none"
                },
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
        let advanced_status = store
            .run_graph_status(run_id)
            .await
            .expect("read advanced run graph status");
        let binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read run graph continuation binding")
            .expect("continuation binding should exist");
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
        assert_eq!(advanced_status.active_node, "implementer");
        assert_eq!(advanced_status.next_node.as_deref(), Some("coach"));
        assert_eq!(advanced_status.status, "ready");
        assert_eq!(advanced_status.lifecycle_stage, "implementer_active");
        assert_eq!(advanced_status.handoff_state, "awaiting_coach");
        assert_eq!(advanced_status.resume_target, "dispatch.coach");
        assert!(advanced_status.recovery_ready);
        assert_eq!(binding.binding_source, "lane_complete");
        assert_eq!(binding.active_bounded_unit["kind"], "run_graph_task");
        assert_eq!(binding.active_bounded_unit["active_node"], "implementer");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lane_complete_uses_current_dispatch_packet_when_downstream_packet_absent() {
        let mut receipt = sample_receipt("blocked");
        receipt.downstream_dispatch_packet_path = None;
        receipt.dispatch_packet_path =
            Some("runtime-consumption/dispatch-packets/current-writer.json".to_string());

        let (packet_path, allow_dispatch_packet) =
            lane_completion_packet_path(&receipt).expect("current dispatch packet should resolve");

        assert_eq!(
            packet_path,
            "runtime-consumption/dispatch-packets/current-writer.json"
        );
        assert!(allow_dispatch_packet);
    }

    #[tokio::test]
    async fn lane_complete_accepts_active_exception_takeover_with_root_dispatch_packet_evidence() {
        let _guard = acquire_lane_surface_test_lock();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-lane-surface-complete-exception-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let _state_override = ProxyStateDirOverrideGuard::install(root.clone());
        let run_id = "run-lane-complete-exception";
        let mut status = crate::taskflow_run_graph::default_run_graph_status(
            run_id,
            "implementation",
            "implementation",
        );
        status.task_id = run_id.to_string();
        status.active_node = "implementer".to_string();
        status.next_node = Some("implementer".to_string());
        status.status = "ready".to_string();
        status.lifecycle_stage = "implementer_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "awaiting_implementer".to_string();
        status.context_state = "sealed".to_string();
        status.checkpoint_kind = "execution_cursor".to_string();
        status.resume_target = "dispatch.implementer_lane".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let packet_path =
            root.join("runtime-consumption/dispatch-packets/run-lane-complete-exception.json");
        std::fs::create_dir_all(
            packet_path
                .parent()
                .expect("dispatch packet path should have parent"),
        )
        .expect("create dispatch packet dir");
        std::fs::write(
            &packet_path,
            serde_json::json!({
                "run_id": run_id,
                "dispatch_target": "implementer",
                "activation_runtime_role": "worker",
                "packet_template_kind": "delivery_task_packet",
                "owned_paths": ["crates/vida/src/lane_surface.rs"],
                "read_only_paths": [".vida/data/state/runtime-consumption"],
                "delivery_task_packet": {
                    "goal": "Complete exception-backed implementer lane evidence.",
                    "scope_in": ["dispatch_target:implementer"],
                    "handoff_task_class": "implementation",
                    "handoff_runtime_role": "worker",
                    "owned_paths": ["crates/vida/src/lane_surface.rs"],
                    "read_only_paths": [".vida/data/state/runtime-consumption"],
                    "definition_of_done": ["lane completion is receipt-backed"],
                    "verification_command": "cargo test -p vida lane_complete",
                    "proof_target": "lane completion receipt",
                    "stop_rules": ["stop if packet contract is invalid"],
                    "blocking_question": "none"
                },
                "role_selection_full": lane_complete_role_selection(run_id),
                "run_graph_bootstrap": {
                    "run_id": run_id
                },
                "downstream_dispatch_active_target": "implementer",
                "downstream_dispatch_ready": false,
                "downstream_dispatch_blockers": ["pending_implementation_evidence"],
                "downstream_dispatch_status": "blocked",
                "downstream_lane_status": "lane_exception_takeover"
            })
            .to_string(),
        )
        .expect("write root dispatch packet");
        let activation_result_path = root.join(
            "runtime-consumption/dispatch-results/run-lane-complete-exception-activation.json",
        );
        std::fs::create_dir_all(
            activation_result_path
                .parent()
                .expect("activation result path should have parent"),
        )
        .expect("create activation result dir");
        std::fs::write(
            &activation_result_path,
            serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "execution_state": "blocked",
                "blocker_code": "internal_dispatch_timeout_without_receipt",
                "activation_semantics": {
                    "view_only": true,
                    "activation_kind": "activation_view"
                }
            })
            .to_string(),
        )
        .expect("write activation-view result");

        let mut receipt = sample_receipt("blocked");
        receipt.run_id = run_id.to_string();
        receipt.dispatch_target = "implementer".to_string();
        receipt.dispatch_kind = "agent_lane".to_string();
        receipt.dispatch_surface = Some("vida agent-init".to_string());
        receipt.dispatch_command = Some("vida agent-init".to_string());
        receipt.dispatch_packet_path = Some(packet_path.display().to_string());
        receipt.dispatch_result_path = Some(activation_result_path.display().to_string());
        receipt.downstream_dispatch_target = Some("coach".to_string());
        receipt.downstream_dispatch_command = Some("vida agent-init".to_string());
        receipt.downstream_dispatch_note =
            Some("after `implementer` evidence is recorded, activate `coach`".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers = vec!["pending_implementation_evidence".to_string()];
        receipt.downstream_dispatch_packet_path = None;
        receipt.downstream_dispatch_status = None;
        receipt.downstream_dispatch_active_target = Some("implementer".to_string());
        receipt.exception_path_receipt_id = Some("exception-1".to_string());
        receipt.supersedes_receipt_id = Some("superseded-1".to_string());
        receipt.lane_status = crate::LaneStatus::LaneExceptionTakeover
            .as_str()
            .to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        let metadata = ExceptionTakeoverMetadata {
            reason_class: "runtime_lane_complete_exception_followup".to_string(),
            active_bounded_unit: format!("{run_id}:implementer:lane-complete-followup"),
            owned_write_scope: vec!["crates/vida/src/lane_surface.rs".to_string()],
            why_delegated_or_rerouted_path_is_not_currently_lawful:
                "active exception takeover is already the lawful execution path".to_string(),
            why_local_write_is_the_smallest_safe_bounded_workaround:
                "lane complete only needs bounded operator-surface correction".to_string(),
            return_to_normal_posture_condition:
                "lane complete succeeds from root dispatch packet evidence".to_string(),
            verification_plan: vec!["cargo test -p vida lane_complete".to_string()],
            recorded_at: "2026-04-22T15:01:35Z".to_string(),
        };
        write_exception_takeover_metadata(store.root(), run_id, &metadata)
            .expect("persist exception takeover metadata");
        drop(store);
        wait_for_state_unlock(&root);

        let args = ProxyArgs {
            args: vec![
                "complete".to_string(),
                run_id.to_string(),
                "--receipt-id".to_string(),
                "completion-exception-1".to_string(),
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
        let advanced_status = store
            .run_graph_status(run_id)
            .await
            .expect("read advanced run graph status");
        let binding = store
            .run_graph_continuation_binding(run_id)
            .await
            .expect("read run graph continuation binding")
            .expect("continuation binding should exist");
        assert!(after.downstream_dispatch_ready);
        assert!(after.downstream_dispatch_blockers.is_empty());
        assert_eq!(
            after.downstream_dispatch_status.as_deref(),
            Some("packet_ready")
        );
        assert_eq!(after.dispatch_status, "executed");
        assert_eq!(after.downstream_dispatch_target.as_deref(), Some("coach"));
        assert!(
            after
                .downstream_dispatch_packet_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
            "active exception takeover completion should materialize the next downstream packet when packet context is available"
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
        assert_eq!(
            result_json["completion_receipt_id"],
            "completion-exception-1"
        );
        let source_dispatch_packet_path = result_json["source_dispatch_packet_path"]
            .as_str()
            .expect("completion result should record source dispatch packet path");
        assert_eq!(
            std::path::PathBuf::from(source_dispatch_packet_path),
            std::fs::canonicalize(&packet_path)
                .expect("source dispatch packet should canonicalize")
        );
        assert_eq!(
            after.dispatch_result_path.as_deref(),
            Some(result_path.as_str())
        );

        let packet = std::fs::read_to_string(&packet_path).expect("read updated packet");
        let packet_json: serde_json::Value =
            serde_json::from_str(&packet).expect("updated packet should be json");
        assert_eq!(packet_json["downstream_dispatch_ready"], true);
        assert_eq!(packet_json["downstream_dispatch_status"], "packet_ready");
        assert_eq!(packet_json["downstream_lane_status"], "packet_ready");
        assert_eq!(packet_json["downstream_dispatch_result_path"], result_path);
        assert_eq!(advanced_status.active_node, "implementer");
        assert_eq!(advanced_status.next_node.as_deref(), Some("coach"));
        assert_eq!(advanced_status.status, "ready");
        assert_eq!(advanced_status.lifecycle_stage, "implementer_active");
        assert_eq!(advanced_status.handoff_state, "awaiting_coach");
        assert_eq!(advanced_status.resume_target, "dispatch.coach");
        assert!(advanced_status.recovery_ready);
        assert_eq!(binding.binding_source, "lane_complete");
        assert_eq!(binding.active_bounded_unit["kind"], "run_graph_task");
        assert_eq!(binding.active_bounded_unit["active_node"], "implementer");

        let _ = std::fs::remove_dir_all(&root);
    }
}
